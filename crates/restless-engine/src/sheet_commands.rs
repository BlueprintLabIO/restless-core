//! Native Sheets use the upstream engine in a credential-free bounded child.
//! Rust remains the only writer of company metadata and accepted revisions.
use anyhow::{Context, Result};
use restless_orgintel::{OrgIntel, SheetState, SHEET_ENGINE_VERSION};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use chrono::Utc;
use uuid::Uuid;

struct Worker {
    child: tokio::process::Child,
    input: tokio::process::ChildStdin,
    output: BufReader<tokio::process::ChildStdout>,
}

// These integration tests use separate Tokio runtimes. A process/pipe belongs
// to the runtime that spawned it, so tests own and release it before that
// runtime shuts down, while production keeps its one long-lived runtime.
pub static SHEET_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
pub async fn reset_test_worker() {
    *WORKER.lock().await = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::FutureExt;
    use std::panic::AssertUnwindSafe;

    #[tokio::test]
    async fn native_sheets_durable_agent_browser_acl_and_history() {
        let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Sheets database integration");
            return;
        };
        let _owned_worker = SHEET_TEST_LOCK.lock().await;
        reset_test_worker().await;
        assert!(
            url::Url::parse(&url).unwrap().path().ends_with("_test"),
            "requires a disposable test database"
        );
        let company = format!("sheets_{}_test", &Uuid::new_v4().simple().to_string()[..12]);
        let other = format!("other_{}_test", &Uuid::new_v4().simple().to_string()[..12]);
        let org = OrgIntel::ensure(&url, &company).await.unwrap();
        let elsewhere = OrgIntel::ensure(&url, &other).await.unwrap();
        let outcome=AssertUnwindSafe(async {
            for (id,kind) in [("owner","owner"),("alice","human"),("bob","human"),("exec","exec")] {
                org.ensure_actor(id,kind,"test",id).await.unwrap();
            }
            elsewhere.ensure_actor("owner","owner","test","owner").await.unwrap();
            let sheet=Uuid::new_v4();
            let created=execute(&org,"owner",SheetOperation::Create{id:sheet,title:"Deals".into(),visibility:"participants".into()}).await.unwrap();
            assert!(org.list_sheets("alice").await.unwrap().is_empty());
            assert!(elsewhere.sheet_state(sheet,"owner",false,0).await.is_err());
            org.share_sheet(sheet,"owner","alice",Some("read")).await.unwrap();
            org.share_sheet(sheet,"owner","exec",Some("edit")).await.unwrap();
            assert_eq!(org.sheet_state(sheet,"alice",false,0).await.unwrap().access,"read");
            assert!(org.sheet_state(sheet,"alice",true,0).await.is_err());
            let base=created["head_revision"].as_str().unwrap().to_string();
            let action=json!({"action":"set_cells","values":[["record_id","Item","Buy","Sell","Fees","Profit"],["gfx","GFX 50S II",2100,3200,100,"=D2-C2-E2"]]});
            let key=Uuid::new_v4();
            let receipt=execute(&org,"exec",SheetOperation::Edit{sheet,expected_revision:base.clone(),key,action:action.clone()}).await.unwrap();
            let first=receipt["revision_id"].as_str().unwrap().to_string();
            let checkpoint=Uuid::new_v4();
            let saved=execute(&org,"owner",SheetOperation::Checkpoint{sheet,expected_revision:first.clone(),key:checkpoint,title:"Saturday".into()}).await.unwrap();
            let before=org.sheet_state(sheet,"owner",false,0).await.unwrap();
            assert!(execute(&org,"exec",SheetOperation::Edit{sheet,expected_revision:first.clone(),key:Uuid::new_v4(),action:json!({"action":"set_cells","values":[["must rollback",{"content":{"bad":true}}]]})}).await.is_err());
            assert_eq!(org.sheet_state(sheet,"owner",false,0).await.unwrap().sheet.sequence,before.sheet.sequence);
            let later=execute(&org,"exec",SheetOperation::Edit{sheet,expected_revision:first.clone(),key:Uuid::new_v4(),action:json!({"action":"update_record","range":"A1:F2","record_id":"gfx","values":{"Sell":3100}})}).await.unwrap();
            assert_eq!(execute(&org,"exec",SheetOperation::Edit{sheet,expected_revision:base.clone(),key,action}).await.unwrap(),receipt,"lost-response retry survives a later edit");
            assert_eq!(execute(&org,"owner",SheetOperation::Checkpoint{sheet,expected_revision:first.clone(),key:checkpoint,title:"Saturday".into()}).await.unwrap(),saved);
            assert!(execute(&org,"owner",SheetOperation::Checkpoint{sheet,expected_revision:first.clone(),key:checkpoint,title:"different".into()}).await.is_err());
            assert!(execute(&org,"exec",SheetOperation::Edit{sheet,expected_revision:first,key:Uuid::new_v4(),action:json!({"action":"set_cells","values":[["stale"]]})}).await.is_err());
            let read=execute(&org,"alice",SheetOperation::Read{sheet,query:Some(json!({"action":"get_range","range":"F2"}))}).await.unwrap();
            assert_eq!(read["result"]["rows"][0][0]["value"],900);
            let recovered=Uuid::new_v4();
            execute(&org,"owner",SheetOperation::RestoreCopy{sheet,version:checkpoint,id:recovered,title:"Recovered".into()}).await.unwrap();
            assert!(org.sheet_state(recovered,"alice",false,0).await.is_err());
            let old=execute(&org,"owner",SheetOperation::Read{sheet:recovered,query:Some(json!({"action":"get_range","range":"F2"}))}).await.unwrap();
            assert_eq!(old["result"]["rows"][0][0]["value"],1000);
            org.share_sheet(sheet,"owner","alice",Some("edit")).await.unwrap();
            org.share_sheet(sheet,"owner","bob",Some("edit")).await.unwrap();
            let a=org.claim_sheet_client(sheet,"alice",None).await.unwrap();
            let b=org.claim_sheet_client(sheet,"bob",None).await.unwrap();
            let a2=org.claim_sheet_client(sheet,"alice",None).await.unwrap();
            assert_ne!(a.client_id,b.client_id);assert_ne!(a.client_id,a2.client_id);
            assert!(org.claim_sheet_client(sheet,"bob",Some((a.client_id,&a.reconnect_token))).await.is_err());
            let resumed=org.claim_sheet_client(sheet,"alice",Some((a.client_id,&a.reconnect_token))).await.unwrap();
            assert!(!org.sheet_client_active(sheet,"alice",a.client_id,a.generation).await.unwrap());
            let state=org.sheet_state(sheet,"alice",true,0).await.unwrap();
            let mut input=model_input(&state);input["client_id"]=json!(resumed.client_id);input["operation"]=json!({"action":"set_cells","start":"G2","values":[["browser"]]});
            let output=model(input).await.unwrap();
            let message=output["messages"][0].clone();
            assert!(accept_browser(&org,sheet,"alice",&a,&message.to_string()).await.is_err(),"old reconnect generation cannot write");
            accept_browser(&org,sheet,"alice",&resumed,&message.to_string()).await.unwrap();
            let head=org.sheet_state(sheet,"owner",false,0).await.unwrap().sheet.head_revision;
            execute(&org,"exec",SheetOperation::Edit{sheet,expected_revision:head,key:Uuid::new_v4(),action:json!({"action":"set_cells","start":"H2","values":[["agent"]]})}).await.unwrap();
            accept_browser(&org,sheet,"alice",&resumed,&message.to_string()).await.unwrap();
            let state=org.sheet_state(sheet,"owner",false,0).await.unwrap();
            let mut reused=message.clone();reused["serverRevisionId"]=json!(state.sheet.head_revision);reused["nextRevisionId"]=state.snapshot["revisionId"].clone();
            let mut badworkbook=output["workbook"].clone();badworkbook["revisionId"]=reused["nextRevisionId"].clone();
            assert!(org.accept_sheet_messages(sheet,"alice",&state.sheet.head_revision,&[reused],&badworkbook,None,Some((resumed.client_id,resumed.generation))).await.is_err());
            let undo=json!({"version":1,"type":"REVISION_UNDONE","serverRevisionId":state.sheet.head_revision,"nextRevisionId":Uuid::new_v4(),"undoneRevisionId":later["revision_id"]});
            assert!(accept_browser(&org,sheet,"alice",&resumed,&undo.to_string()).await.is_err(),"cannot undo another Actor's revision");
            org.share_sheet(sheet,"owner","alice",None).await.unwrap();
            assert!(org.sheet_checkpoint(sheet,"alice",checkpoint).await.is_err(),"history obeys current ACL");
            // Replay from a new DB handle and a new process restores the same body.
            *WORKER.lock().await=None;
            let reopened=OrgIntel::ensure(&url,&company).await.unwrap();
            let finalread=execute(&reopened,"owner",SheetOperation::Read{sheet,query:Some(json!({"action":"get_range","range":"F2:H2"}))}).await.unwrap();
            assert_eq!(finalread["result"]["rows"][0][0]["value"],900);
            assert_eq!(finalread["result"]["rows"][0][1]["value"],"browser");
            assert_eq!(finalread["result"]["rows"][0][2]["value"],"agent");
            reopened.close().await;
            // A long-lived sheet opens from its newest checkpoint and replays a
            // bounded tail that rebuilds exactly what the whole log rebuilds.
            let long=Uuid::new_v4();
            let mut head=execute(&org,"owner",SheetOperation::Create{id:long,title:"Long".into(),visibility:"company".into()}).await.unwrap()["head_revision"].as_str().unwrap().to_string();
            for n in 0..270 {
                let receipt=execute(&org,"owner",SheetOperation::Edit{sheet:long,expected_revision:head,key:Uuid::new_v4(),action:json!({"action":"set_cells","start":format!("A{}",n%40+1),"values":[[n]]})}).await.unwrap();
                head=receipt["revision_id"].as_str().unwrap().to_string();
            }
            let fresh=org.sheet_state(long,"owner",false,0).await.unwrap();
            assert_eq!(fresh.messages.len(),220,"replays from the checkpoint at sequence 50");
            let pool=sqlx::PgPool::connect(&url).await.unwrap();
            let base:Value=sqlx::query_scalar(&format!("SELECT replay_base FROM {company}.native_sheets WHERE id=$1")).bind(long).fetch_one(&pool).await.unwrap();
            let log:Vec<Value>=sqlx::query_scalar(&format!("SELECT message FROM {company}.native_sheet_messages WHERE sheet_id=$1 ORDER BY sequence")).bind(long).fetch_all(&pool).await.unwrap();
            pool.close().await;
            assert_eq!(log.len(),270);
            let whole=model(json!({"engine_version":SHEET_ENGINE_VERSION,"snapshot":base,"messages":log})).await.unwrap();
            let tail=model(model_input(&fresh)).await.unwrap();
            assert_eq!(tail["workbook"],whole["workbook"]);
        }).catch_unwind().await;
        org.close().await;
        elsewhere.close().await;
        let cleanup = sqlx::PgPool::connect(&url).await.unwrap();
        for schema in [&company, &other] {
            sqlx::query(&format!("DROP SCHEMA {schema} CASCADE"))
                .execute(&cleanup)
                .await
                .unwrap();
        }
        cleanup.close().await;
        reset_test_worker().await;
        if let Err(panic) = outcome {
            std::panic::resume_unwind(panic);
        }
    }
}
static WORKER: tokio::sync::Mutex<Option<Worker>> = tokio::sync::Mutex::const_new(None);

fn worker_path() -> Result<PathBuf> {
    let configured = std::env::var_os("RESTLESS_SHEETS_WORKER").map(PathBuf::from);
    let path = configured.unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../services/native-sheets/src/worker.mjs")
    });
    anyhow::ensure!(
        path.is_absolute() && path.is_file(),
        "Native Sheets model worker is not installed"
    );
    Ok(path)
}
async fn start_worker() -> Result<Worker> {
    let node = std::env::var_os("RESTLESS_NODE_BIN").unwrap_or_else(|| "node".into());
    let mut cmd = tokio::process::Command::new(node);
    cmd.arg("--max-old-space-size=512")
        .arg(worker_path()?)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("NODE_ENV", "production")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = cmd.spawn().context("start Native Sheets engine")?;
    let input = child.stdin.take().context("Sheets worker stdin")?;
    let output = BufReader::new(child.stdout.take().context("Sheets worker stdout")?);
    Ok(Worker {
        child,
        input,
        output,
    })
}
pub async fn model(input: Value) -> Result<Value> {
    let bytes = serde_json::to_vec(&input)?;
    anyhow::ensure!(
        bytes.len() <= 32 * 1024 * 1024,
        "Workbook replay exceeds the supported 32 MiB limit"
    );
    let mut guard = WORKER.lock().await;
    if guard
        .as_mut()
        .is_some_and(|w| w.child.try_wait().ok().flatten().is_some())
    {
        *guard = None;
    }
    if guard.is_none() {
        *guard = Some(start_worker().await?);
    }
    let w = guard.as_mut().unwrap();
    // Large, long-lived workbooks can legitimately take longer than fifteen
    // seconds to replay in the deterministic local worker. Keep the guard
    // bounded while allowing those workbooks to complete.
    let result = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        w.input.write_all(&bytes).await?;
        w.input.write_all(b"\n").await?;
        w.input.flush().await?;
        let mut line = Vec::new();
        loop {
            let available = w.output.fill_buf().await?;
            anyhow::ensure!(!available.is_empty(), "Sheets worker stopped");
            let n = available
                .iter()
                .position(|b| *b == b'\n')
                .map_or(available.len(), |n| n + 1);
            anyhow::ensure!(
                line.len() + n <= 24 * 1024 * 1024,
                "Sheets response exceeds 24 MiB"
            );
            let done = available[n - 1] == b'\n';
            line.extend_from_slice(&available[..n]);
            w.output.consume(n);
            if done {
                break;
            }
        }
        let value: Value = serde_json::from_slice(&line)?;
        anyhow::ensure!(
            value["ok"] == true,
            "{}",
            value["error"].as_str().unwrap_or("Sheets model failed")
        );
        Ok::<Value, anyhow::Error>(value)
    })
    .await;
    match result {
        Ok(Ok(v)) => Ok(v),
        Ok(Err(e)) => {
            *guard = None;
            Err(e)
        }
        Err(_) => {
            *guard = None;
            anyhow::bail!("Sheets model timed out")
        }
    }
}
pub fn model_input(state: &SheetState) -> Value {
    json!({"engine_version":state.sheet.engine_version,"snapshot":state.snapshot,
        "messages":state.messages.iter().map(|m| &m.message).collect::<Vec<_>>()})
}

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum SheetOperation {
    List,
    /// Archived sheets, for restoring.
    Archived,
    /// Archive a sheet out of the Library, or restore it.
    Archive {
        sheet: Uuid,
        archived: bool,
    },
    Create {
        id: Uuid,
        title: String,
        #[serde(default = "company_visibility")]
        visibility: String,
    },
    Read {
        sheet: Uuid,
        #[serde(default)]
        query: Option<Value>,
    },
    Edit {
        sheet: Uuid,
        expected_revision: String,
        key: Uuid,
        action: Value,
    },
    Share {
        sheet: Uuid,
        actor: String,
        access: Option<String>,
    },
    Versions {
        sheet: Uuid,
    },
    Version {
        sheet: Uuid,
        version: Uuid,
    },
    RestoreCopy {
        sheet: Uuid,
        version: Uuid,
        id: Uuid,
        title: String,
    },
    Checkpoint {
        sheet: Uuid,
        expected_revision: String,
        key: Uuid,
        title: String,
    },
}
fn company_visibility() -> String {
    "company".into()
}
pub fn is_read_action(action: &Value) -> bool {
    matches!(
        action["action"].as_str(),
        Some("get_range" | "filter" | "export_csv" | "snapshot")
    )
}
pub async fn execute(
    org: &OrgIntel,
    actor: &str,
    operation: SheetOperation,
) -> Result<Value> {
    match operation {
        SheetOperation::List => Ok(json!(org.list_sheets(actor).await?)),
        SheetOperation::Archived => Ok(json!(org.list_archived_sheets(actor).await?)),
        SheetOperation::Archive { sheet, archived } => {
            org.set_sheet_archived(sheet, actor, archived).await?;
            Ok(json!({ "sheet": sheet, "archived": archived }))
        }
        SheetOperation::Create {
            id,
            title,
            visibility,
        } => {
            let output = model(json!({"engine_version":SHEET_ENGINE_VERSION})).await?;
            Ok(json!(
                org.create_sheet(id, actor, &title, &visibility, &output["workbook"])
                    .await?
            ))
        }
        SheetOperation::Read { sheet, query } => {
            let state = org.sheet_state(sheet, actor, false, 0).await?;
            let mut input = model_input(&state);
            if let Some(query) = query {
                anyhow::ensure!(is_read_action(&query), "Read only sheet action required");
                input["operation"] = query;
            }
            let output = model(input).await?;
            Ok(
                json!({"sheet":state.sheet,"access":state.access,"workbook":output["workbook"],"result":output["result"]}),
            )
        }
        SheetOperation::Edit {
            sheet,
            expected_revision,
            key,
            action,
        } => {
            anyhow::ensure!(!is_read_action(&action), "Use a sheet read for this action");
            let request = json!({"expected_revision":expected_revision,"action":action});
            if let Some(result) = org
                .sheet_command_result(sheet, actor, key, &request)
                .await?
            {
                return Ok(result);
            }
            let state = org.sheet_state(sheet, actor, true, 0).await?;
            anyhow::ensure!(
                state.sheet.head_revision == expected_revision,
                "sheet conflict: workbook changed; reread its revision"
            );
            let mut input = model_input(&state);
            input["operation"] = action;
            let output = model(input).await?;
            let mut messages: Vec<Value> = serde_json::from_value(output["messages"].clone())?;
            for message in &mut messages {
                message["timestamp"] = json!(chrono::Utc::now().timestamp_millis());
            }
            let result =
                json!({"revision_id":output["workbook"]["revisionId"],"result":output["result"]});
            org.accept_sheet_messages(
                sheet,
                actor,
                &expected_revision,
                &messages,
                &output["workbook"],
                Some((key, &request, &result)),
                None,
            )
            .await?;
            Ok(org
                .sheet_command_result(sheet, actor, key, &request)
                .await?
                .context("accepted sheet receipt missing")?)
        }
        SheetOperation::Share {
            sheet,
            actor: target,
            access,
        } => {
            org.share_sheet(sheet, actor, &target, access.as_deref())
                .await?;
            Ok(json!({"shared":true}))
        }
        SheetOperation::Versions { sheet } => Ok(json!(org.sheet_checkpoints(sheet, actor).await?)),
        SheetOperation::Version { sheet, version } => {
            Ok(org.sheet_checkpoint(sheet, actor, version).await?)
        }
        SheetOperation::RestoreCopy {
            sheet,
            version,
            id,
            title,
        } => {
            let checkpoint = org.sheet_checkpoint(sheet, actor, version).await?;
            // Recovery creates a private copy; it cannot erase live pending OT
            // edits or widen the source workbook's audience.
            Ok(json!(
                org.create_sheet(id, actor, &title, "participants", &checkpoint["workbook"])
                    .await?
            ))
        }
        SheetOperation::Checkpoint {
            sheet,
            expected_revision,
            key,
            title,
        } => {
            if let Some(receipt) = org
                .sheet_checkpoint_receipt(sheet, actor, key, &title, &expected_revision)
                .await?
            {
                return Ok(receipt);
            }
            let state = org.sheet_state(sheet, actor, true, 0).await?;
            anyhow::ensure!(
                state.sheet.head_revision == expected_revision,
                "sheet conflict: workbook changed"
            );
            let output = model(model_input(&state)).await?;
            Ok(org
                .checkpoint_sheet(
                    sheet,
                    actor,
                    key,
                    &title,
                    &expected_revision,
                    &output["workbook"],
                )
                .await?)
        }
    }
}

pub async fn accept_browser(
    org: &restless_orgintel::OrgIntel,
    id: Uuid,
    actor: &str,
    client: &restless_orgintel::SheetClient,
    text: &str,
) -> Result<()> {
    let mut message: Value = serde_json::from_str(text)?;
    let kind = message["type"].as_str().unwrap_or("").to_string();
    anyhow::ensure!(message["version"] == 1, "Unsupported Sheets protocol");
    if ["CLIENT_JOINED", "CLIENT_MOVED", "CLIENT_LEFT", "SNAPSHOT"].contains(&kind.as_str()) {
        return Ok(());
    }
    anyhow::ensure!(
        ["REMOTE_REVISION", "REVISION_UNDONE", "REVISION_REDONE"].contains(&kind.as_str()),
        "Unsupported sheet message"
    );
    if kind == "REMOTE_REVISION" {
        anyhow::ensure!(
            message["clientId"].as_str() == Some(&client.client_id.to_string()),
            "Invalid sheet client identity"
        );
    }
    let state = org.sheet_state(id, actor, true, 0).await?;
    if let Some(stored) = state
        .messages
        .iter()
        .find(|m| m.message["nextRevisionId"] == message["nextRevisionId"])
    {
        let mut original = stored.message.clone();
        original.as_object_mut().map(|o| o.remove("timestamp"));
        anyhow::ensure!(
            stored.actor_id == actor && original == message,
            "Sheet revision ID reused"
        );
        return Ok(());
    }
    // Stale messages are not accepted. The ordered catch-up pump delivers the
    // intervening revisions and upstream transforms/resubmits pending edits.
    if message["serverRevisionId"].as_str() != Some(&state.sheet.head_revision) {
        return Ok(());
    }
    if kind != "REMOTE_REVISION" {
        let field = if kind == "REVISION_UNDONE" {
            "undoneRevisionId"
        } else {
            "redoneRevisionId"
        };
        anyhow::ensure!(
            state
                .messages
                .iter()
                .any(|m| m.message["nextRevisionId"] == message[field] && m.actor_id == actor),
            "Only your own recent changes can be undone here"
        );
    }
    message["timestamp"] = json!(Utc::now().timestamp_millis());
    let mut input = model_input(&state);
    input["revision"] = message.clone();
    let output = model(input).await?;
    match org
        .accept_sheet_messages(
            id,
            actor,
            &state.sheet.head_revision,
            &[message],
            &output["workbook"],
            None,
            Some((client.client_id, client.generation)),
        )
        .await
    {
        Ok(_) => Ok(()),
        Err(restless_orgintel::SheetError::Conflict(_)) => Ok(()),
        Err(e) => Err(e.into()),
    }
}
