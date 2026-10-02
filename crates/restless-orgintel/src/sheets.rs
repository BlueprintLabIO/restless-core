//! Cell-local native workbooks. Upstream JSON and accepted OT messages are the
//! only body truth. Checkpoints do not compact the replay log or destroy undo.
use crate::OrgIntel;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub const SHEET_ENGINE_VERSION: &str = "19.0.51";
pub const MAX_SHEET_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum SheetError {
    #[error("sheet is unavailable")]
    Unavailable,
    #[error("sheet conflict: {0}")]
    Conflict(String),
    #[error("invalid sheet: {0}")]
    Invalid(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}
type Result<T> = std::result::Result<T, SheetError>;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SheetRow {
    pub id: Uuid,
    pub title: String,
    pub owner_actor_id: String,
    pub visibility: String,
    pub engine_version: String,
    pub head_revision: String,
    pub sequence: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SheetMessage {
    pub sequence: i64,
    pub actor_id: String,
    pub message: Value,
}
#[derive(Debug, Clone, Serialize)]
pub struct SheetState {
    pub sheet: SheetRow,
    pub access: String,
    pub snapshot: Value,
    pub messages: Vec<SheetMessage>,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SheetCheckpoint {
    pub id: Uuid,
    pub sequence: i64,
    pub revision_id: String,
    pub title: Option<String>,
    pub actor_id: String,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize)]
pub struct SheetClient {
    pub client_id: Uuid,
    pub reconnect_token: String,
    pub generation: Uuid,
}
const ROW: &str = "id,title,owner_actor_id,visibility,engine_version,head_revision,sequence,created_at,updated_at";

async fn access(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    actor: &str,
    edit: bool,
) -> Result<String> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT owner_actor_id,visibility FROM native_sheets WHERE id=$1 FOR SHARE")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?;
    let Some((owner, visibility)) = row else {
        return Err(SheetError::Unavailable);
    };
    let class: Option<String> = sqlx::query_scalar(
        "SELECT actor_class FROM actors WHERE id=$1 AND retired_at IS NULL FOR SHARE",
    )
    .bind(actor)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(class) = class else {
        return Err(SheetError::Unavailable);
    };
    // A suspended external member loses access even if the durable Actor is
    // retained for attribution. Local humans have no external binding.
    let denied: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM human_principal_actor_bindings WHERE actor_id=$1 AND membership_status<>'active')")
        .bind(actor).fetch_one(&mut **tx).await?;
    if denied {
        return Err(SheetError::Unavailable);
    }
    let explicit: Option<String> = sqlx::query_scalar(
        "SELECT access FROM native_sheet_participants WHERE sheet_id=$1 AND actor_id=$2",
    )
    .bind(id)
    .bind(actor)
    .fetch_optional(&mut **tx)
    .await?;
    let grant = if owner == actor {
        Some("edit".to_string())
    } else {
        explicit.or_else(|| (visibility == "company" && class == "human").then(|| "read".into()))
    };
    let grant = grant.ok_or(SheetError::Unavailable)?;
    if edit && grant != "edit" {
        return Err(SheetError::Unavailable);
    }
    Ok(grant)
}
fn bounded(value: &Value) -> Result<()> {
    if serde_json::to_vec(value)
        .map_err(|e| SheetError::Invalid(e.to_string()))?
        .len()
        > MAX_SHEET_BYTES
    {
        return Err(SheetError::Invalid("workbook exceeds 8 MiB".into()));
    }
    Ok(())
}
fn revision(message: &Value, field: &str) -> Result<String> {
    let value = message[field]
        .as_str()
        .filter(|v| !v.is_empty() && v.len() <= 100)
        .ok_or_else(|| SheetError::Invalid(format!("missing {field}")))?;
    Ok(value.into())
}

impl OrgIntel {
    pub async fn claim_sheet_client(
        &self,
        id: Uuid,
        actor: &str,
        resume: Option<(Uuid, &str)>,
    ) -> Result<SheetClient> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT id FROM native_sheets WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(SheetError::Unavailable)?;
        access(&mut tx, id, actor, false).await?;
        let generation = Uuid::new_v4();
        let (client_id, reconnect_token) = if let Some((client, token)) = resume {
            let claim:Option<(String,Vec<u8>)>=sqlx::query_as("SELECT actor_id,reconnect_hash FROM native_sheet_clients WHERE sheet_id=$1 AND client_id=$2")
                .bind(id).bind(client).fetch_optional(&mut *tx).await?;
            let Some((owner, hash)) = claim else {
                return Err(SheetError::Unavailable);
            };
            if owner != actor || hash != Sha256::digest(token.as_bytes()).to_vec() {
                return Err(SheetError::Unavailable);
            }
            sqlx::query(
                "UPDATE native_sheet_clients SET generation=$3 WHERE sheet_id=$1 AND client_id=$2",
            )
            .bind(id)
            .bind(client)
            .bind(generation)
            .execute(&mut *tx)
            .await?;
            (client, token.to_string())
        } else {
            let client = Uuid::new_v4();
            let token = Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO native_sheet_clients(sheet_id,client_id,actor_id,reconnect_hash,generation) VALUES($1,$2,$3,$4,$5)")
                .bind(id).bind(client).bind(actor).bind(Sha256::digest(token.as_bytes()).to_vec()).bind(generation).execute(&mut *tx).await?;
            (client, token)
        };
        tx.commit().await?;
        Ok(SheetClient {
            client_id,
            reconnect_token,
            generation,
        })
    }
    pub async fn sheet_client_active(
        &self,
        id: Uuid,
        actor: &str,
        client: Uuid,
        generation: Uuid,
    ) -> Result<bool> {
        Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM native_sheet_clients WHERE sheet_id=$1 AND actor_id=$2 AND client_id=$3 AND generation=$4)")
            .bind(id).bind(actor).bind(client).bind(generation).fetch_one(&self.pool).await?)
    }
    pub async fn create_sheet(
        &self,
        id: Uuid,
        actor: &str,
        title: &str,
        visibility: &str,
        workbook: &Value,
    ) -> Result<SheetRow> {
        if title.trim().is_empty()
            || title.chars().count() > 200
            || !["company", "participants"].contains(&visibility)
        {
            return Err(SheetError::Invalid("title or visibility is invalid".into()));
        }
        bounded(workbook)?;
        let head = revision(workbook, "revisionId")?;
        let mut tx = self.pool.begin().await?;
        let active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL)",
        )
        .bind(actor)
        .fetch_one(&mut *tx)
        .await?;
        if !active {
            return Err(SheetError::Unavailable);
        }
        let denied:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM human_principal_actor_bindings WHERE actor_id=$1 AND membership_status<>'active')")
            .bind(actor).fetch_one(&mut *tx).await?;
        if denied {
            return Err(SheetError::Unavailable);
        }
        sqlx::query("INSERT INTO native_sheets(id,title,owner_actor_id,visibility,engine_version,replay_base,head_revision) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(id) DO NOTHING")
            .bind(id).bind(title.trim()).bind(actor).bind(visibility).bind(SHEET_ENGINE_VERSION).bind(workbook).bind(head).execute(&mut *tx).await?;
        let row: SheetRow = sqlx::query_as(&format!("SELECT {ROW} FROM native_sheets WHERE id=$1"))
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        if row.owner_actor_id != actor || row.title != title.trim() || row.visibility != visibility
        {
            return Err(SheetError::Conflict("creation ID already used".into()));
        }
        let original: Value =
            sqlx::query_scalar("SELECT replay_base FROM native_sheets WHERE id=$1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        if original != *workbook {
            return Err(SheetError::Conflict(
                "creation ID already used for different workbook".into(),
            ));
        }
        tx.commit().await?;
        Ok(row)
    }
    pub async fn list_sheets(&self, actor: &str) -> Result<Vec<SheetRow>> {
        let mut tx = self.pool.begin().await?;
        let rows: Vec<SheetRow> = sqlx::query_as(&format!(
            "SELECT {ROW} FROM native_sheets ORDER BY updated_at DESC LIMIT 100"
        ))
        .fetch_all(&mut *tx)
        .await?;
        let mut visible = Vec::new();
        for row in rows {
            if access(&mut tx, row.id, actor, false).await.is_ok() {
                visible.push(row);
            }
        }
        tx.commit().await?;
        Ok(visible)
    }
    pub async fn sheet_state(
        &self,
        id: Uuid,
        actor: &str,
        edit: bool,
        after: i64,
    ) -> Result<SheetState> {
        let mut tx = self.pool.begin().await?;
        let grant = access(&mut tx, id, actor, edit).await?;
        let sheet = sqlx::query_as(&format!("SELECT {ROW} FROM native_sheets WHERE id=$1"))
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        let snapshot = sqlx::query_scalar("SELECT replay_base FROM native_sheets WHERE id=$1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        let messages = sqlx::query_as("SELECT sequence,actor_id,message FROM native_sheet_messages WHERE sheet_id=$1 AND sequence>$2 ORDER BY sequence")
            .bind(id).bind(after).fetch_all(&mut *tx).await?;
        tx.commit().await?;
        Ok(SheetState {
            sheet,
            access: grant,
            snapshot,
            messages,
        })
    }
    pub async fn sheet_command_result(
        &self,
        id: Uuid,
        actor: &str,
        key: Uuid,
        request: &Value,
    ) -> Result<Option<Value>> {
        let mut tx = self.pool.begin().await?;
        access(&mut tx, id, actor, true).await?;
        let stored: Option<(String,Value,Value)> = sqlx::query_as("SELECT actor_id,request,result FROM native_sheet_commands WHERE sheet_id=$1 AND command_id=$2")
            .bind(id).bind(key).fetch_optional(&mut *tx).await?;
        tx.commit().await?;
        match stored {
            Some((owner, original, result)) if owner == actor && &original == request => {
                Ok(Some(result))
            }
            Some(_) => Err(SheetError::Conflict(
                "command key reused for different input".into(),
            )),
            None => Ok(None),
        }
    }
    /// Validated upstream messages enter the same ordered stream regardless of
    /// whether the writer is a browser or an agent. Commit precedes delivery.
    pub async fn accept_sheet_messages(
        &self,
        id: Uuid,
        actor: &str,
        expected: &str,
        messages: &[Value],
        workbook: &Value,
        command: Option<(Uuid, &Value, &Value)>,
        client: Option<(Uuid, Uuid)>,
    ) -> Result<i64> {
        bounded(workbook)?;
        let mut tx = self.pool.begin().await?;
        let (mut head, mut sequence): (String, i64) = sqlx::query_as(
            "SELECT head_revision,sequence FROM native_sheets WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(SheetError::Unavailable)?;
        access(&mut tx, id, actor, true).await?;
        if let Some((client, generation)) = client {
            let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM native_sheet_clients WHERE sheet_id=$1 AND actor_id=$2 AND client_id=$3 AND generation=$4)")
                .bind(id).bind(actor).bind(client).bind(generation).fetch_one(&mut *tx).await?;
            if !valid {
                return Err(SheetError::Unavailable);
            }
        }
        if let Some((key, request, _)) = command {
            let stored: Option<(String,Value)> = sqlx::query_as("SELECT actor_id,request FROM native_sheet_commands WHERE sheet_id=$1 AND command_id=$2")
                .bind(id).bind(key).fetch_optional(&mut *tx).await?;
            if let Some((owner, original)) = stored {
                if owner != actor || &original != request {
                    return Err(SheetError::Conflict("command key reused".into()));
                }
                tx.commit().await?;
                return Ok(sequence);
            }
        }
        // A lost acknowledgement may resend a previously accepted revision.
        if messages.len() == 1 {
            let next = revision(&messages[0], "nextRevisionId")?;
            let stored: Option<(String,Value)> = sqlx::query_as("SELECT actor_id,message FROM native_sheet_messages WHERE sheet_id=$1 AND revision_id=$2")
                .bind(id).bind(next).fetch_optional(&mut *tx).await?;
            if let Some((owner, original)) = stored {
                let mut submitted = messages[0].clone();
                submitted.as_object_mut().map(|o| o.remove("timestamp"));
                let mut original = original;
                original.as_object_mut().map(|o| o.remove("timestamp"));
                if owner != actor || submitted != original {
                    return Err(SheetError::Conflict("revision ID reused".into()));
                }
                tx.commit().await?;
                return Ok(sequence);
            }
        }
        if head != expected {
            return Err(SheetError::Conflict(
                "workbook changed; reread its current revision".into(),
            ));
        }
        if sequence + messages.len() as i64 > 20_000 {
            return Err(SheetError::Invalid("workbook exceeds the supported 20,000-revision history; recover a checkpoint into a new sheet".into()));
        }
        let replay_bytes:i64=sqlx::query_scalar("SELECT COALESCE(SUM(octet_length(message::text)),0)::bigint FROM native_sheet_messages WHERE sheet_id=$1")
            .bind(id).fetch_one(&mut *tx).await?;
        let added: usize = messages
            .iter()
            .map(|m| serde_json::to_vec(m).map_or(usize::MAX, |v| v.len()))
            .sum();
        if replay_bytes as usize
            + added
            + serde_json::to_vec(workbook).map_or(usize::MAX, |v| v.len())
            > 24 * 1024 * 1024
        {
            return Err(SheetError::Invalid("workbook replay exceeds the supported 24 MiB history limit; recover a checkpoint into a new sheet".into()));
        }
        for message in messages {
            bounded(message)?;
            if message["version"] != 1 || revision(message, "serverRevisionId")? != head {
                return Err(SheetError::Invalid("revision chain is invalid".into()));
            }
            let kind = message["type"].as_str().unwrap_or("");
            if !["REMOTE_REVISION", "REVISION_UNDONE", "REVISION_REDONE"].contains(&kind) {
                return Err(SheetError::Invalid(
                    "unsupported collaboration message".into(),
                ));
            }
            if kind != "REMOTE_REVISION" {
                let field = if kind == "REVISION_UNDONE" {
                    "undoneRevisionId"
                } else {
                    "redoneRevisionId"
                };
                let target = revision(message, field)?;
                let owner:Option<String>=sqlx::query_scalar("SELECT actor_id FROM native_sheet_messages WHERE sheet_id=$1 AND revision_id=$2 AND message->>'type'='REMOTE_REVISION'")
                    .bind(id).bind(target).fetch_optional(&mut *tx).await?;
                if owner.as_deref() != Some(actor) {
                    return Err(SheetError::Unavailable);
                }
            } else {
                let message_client = Uuid::parse_str(&revision(message, "clientId")?)
                    .map_err(|_| SheetError::Invalid("invalid client ID".into()))?;
                if let Some((bound, _)) = client {
                    if bound != message_client {
                        return Err(SheetError::Unavailable);
                    }
                }
                // Agent Models also reserve their IDs atomically. No two Actors
                // can cause upstream to mistake each other's operation for an ACK.
                let owner: Option<String> = sqlx::query_scalar(
                    "SELECT actor_id FROM native_sheet_clients WHERE sheet_id=$1 AND client_id=$2",
                )
                .bind(id)
                .bind(message_client)
                .fetch_optional(&mut *tx)
                .await?;
                if owner.as_deref().is_some_and(|o| o != actor) {
                    return Err(SheetError::Unavailable);
                }
                if owner.is_none() {
                    sqlx::query("INSERT INTO native_sheet_clients(sheet_id,client_id,actor_id,reconnect_hash,generation) VALUES($1,$2,$3,$4,$5)")
                        .bind(id).bind(message_client).bind(actor).bind(Vec::<u8>::new()).bind(Uuid::new_v4()).execute(&mut *tx).await?;
                }
            }
            let next = revision(message, "nextRevisionId")?;
            let next_uuid = Uuid::parse_str(&next)
                .map_err(|_| SheetError::Invalid("next revision must be a fresh UUID".into()))?;
            if next_uuid.is_nil() || next == head {
                return Err(SheetError::Invalid("next revision must be fresh".into()));
            }
            let reused:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM native_sheet_messages WHERE sheet_id=$1 AND revision_id=$2) OR EXISTS(SELECT 1 FROM native_sheets WHERE id=$1 AND replay_base->>'revisionId'=$2)")
                .bind(id).bind(&next).fetch_one(&mut *tx).await?;
            if reused {
                return Err(SheetError::Conflict("revision ID reused".into()));
            }
            head = next;
            sequence += 1;
            sqlx::query("INSERT INTO native_sheet_messages(sheet_id,sequence,revision_id,actor_id,message) VALUES($1,$2,$3,$4,$5)")
                .bind(id).bind(sequence).bind(&head).bind(actor).bind(message).execute(&mut *tx).await?;
        }
        if workbook["revisionId"].as_str() != Some(&head) {
            return Err(SheetError::Invalid(
                "checkpoint does not match accepted head".into(),
            ));
        }
        sqlx::query(
            "UPDATE native_sheets SET head_revision=$2,sequence=$3,updated_at=now() WHERE id=$1",
        )
        .bind(id)
        .bind(&head)
        .bind(sequence)
        .execute(&mut *tx)
        .await?;
        if !messages.is_empty() && sequence / 50 > (sequence - messages.len() as i64) / 50 {
            sqlx::query("INSERT INTO native_sheet_checkpoints(id,sheet_id,sequence,revision_id,workbook,actor_id) VALUES($1,$2,$3,$4,$5,$6)")
                .bind(Uuid::new_v4()).bind(id).bind(sequence).bind(&head).bind(workbook).bind(actor).execute(&mut *tx).await?;
        }
        if let Some((key, request, result)) = command {
            sqlx::query("INSERT INTO native_sheet_commands(sheet_id,command_id,actor_id,request,result) VALUES($1,$2,$3,$4,$5)")
                .bind(id).bind(key).bind(actor).bind(request).bind(result).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(sequence)
    }
    pub async fn share_sheet(
        &self,
        id: Uuid,
        actor: &str,
        participant: &str,
        grant: Option<&str>,
    ) -> Result<()> {
        if grant.is_some_and(|g| !["read", "edit"].contains(&g)) {
            return Err(SheetError::Invalid("invalid access".into()));
        }
        let mut tx = self.pool.begin().await?;
        let owner: Option<String> =
            sqlx::query_scalar("SELECT owner_actor_id FROM native_sheets WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        access(&mut tx, id, actor, true).await?;
        if owner.as_deref() != Some(actor) {
            return Err(SheetError::Unavailable);
        }
        if let Some(grant) = grant {
            let active: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL)",
            )
            .bind(participant)
            .fetch_one(&mut *tx)
            .await?;
            if !active {
                return Err(SheetError::Unavailable);
            }
            sqlx::query("INSERT INTO native_sheet_participants(sheet_id,actor_id,access) VALUES($1,$2,$3) ON CONFLICT(sheet_id,actor_id) DO UPDATE SET access=EXCLUDED.access")
                .bind(id).bind(participant).bind(grant).execute(&mut *tx).await?;
        } else {
            sqlx::query("DELETE FROM native_sheet_participants WHERE sheet_id=$1 AND actor_id=$2")
                .bind(id)
                .bind(participant)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
    pub async fn sheet_checkpoints(&self, id: Uuid, actor: &str) -> Result<Vec<SheetCheckpoint>> {
        let mut tx = self.pool.begin().await?;
        access(&mut tx, id, actor, false).await?;
        let rows=sqlx::query_as("SELECT id,sequence,revision_id,title,actor_id,created_at FROM native_sheet_checkpoints WHERE sheet_id=$1 ORDER BY sequence DESC,created_at DESC LIMIT 100")
            .bind(id).fetch_all(&mut *tx).await?;
        tx.commit().await?;
        Ok(rows)
    }
    pub async fn sheet_checkpoint(&self, id: Uuid, actor: &str, version: Uuid) -> Result<Value> {
        let mut tx = self.pool.begin().await?;
        access(&mut tx, id, actor, false).await?;
        let view:Option<Value>=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'sheet_id',sheet_id,'sequence',sequence,'revision_id',revision_id,'title',title,'actor_id',actor_id,'created_at',created_at,'workbook',workbook) FROM native_sheet_checkpoints WHERE sheet_id=$1 AND id=$2")
            .bind(id).bind(version).fetch_optional(&mut *tx).await?;
        tx.commit().await?;
        view.ok_or(SheetError::Unavailable)
    }
    pub async fn checkpoint_sheet(
        &self,
        id: Uuid,
        actor: &str,
        key: Uuid,
        title: &str,
        expected: &str,
        workbook: &Value,
    ) -> Result<Value> {
        bounded(workbook)?;
        if title.trim().is_empty() || title.chars().count() > 200 {
            return Err(SheetError::Invalid("checkpoint title is invalid".into()));
        }
        let mut tx = self.pool.begin().await?;
        let (head, sequence): (String, i64) = sqlx::query_as(
            "SELECT head_revision,sequence FROM native_sheets WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(SheetError::Unavailable)?;
        access(&mut tx, id, actor, true).await?;
        let existing:Option<(Uuid,String,Option<String>,String,i64)>=sqlx::query_as("SELECT sheet_id,actor_id,title,revision_id,sequence FROM native_sheet_checkpoints WHERE id=$1")
            .bind(key).fetch_optional(&mut *tx).await?;
        if let Some((sheet, owner, label, revision, sequence)) = existing {
            if sheet != id
                || owner != actor
                || label.as_deref() != Some(title.trim())
                || revision != expected
            {
                return Err(SheetError::Conflict(
                    "checkpoint ID reused for different input".into(),
                ));
            }
            tx.commit().await?;
            return Ok(json!({"id":key,"revision_id":revision,"sequence":sequence}));
        }
        if head != expected || workbook["revisionId"].as_str() != Some(&head) {
            return Err(SheetError::Conflict("checkpoint revision changed".into()));
        }
        sqlx::query("INSERT INTO native_sheet_checkpoints(id,sheet_id,sequence,revision_id,workbook,title,actor_id) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(id) DO NOTHING")
            .bind(key).bind(id).bind(sequence).bind(&head).bind(workbook).bind(title.trim()).bind(actor).execute(&mut *tx).await?;
        // The checkpoint key is global, so another sheet's transaction may
        // win its unique insertion while this sheet's lock is held.
        let stored:(Uuid,String,Option<String>,String,i64)=sqlx::query_as("SELECT sheet_id,actor_id,title,revision_id,sequence FROM native_sheet_checkpoints WHERE id=$1")
            .bind(key).fetch_one(&mut *tx).await?;
        if stored.0 != id
            || stored.1 != actor
            || stored.2.as_deref() != Some(title.trim())
            || stored.3 != expected
        {
            return Err(SheetError::Conflict(
                "checkpoint ID reused for different input".into(),
            ));
        }
        tx.commit().await?;
        Ok(json!({"id":key,"revision_id":head,"sequence":sequence}))
    }
    pub async fn sheet_checkpoint_receipt(
        &self,
        id: Uuid,
        actor: &str,
        key: Uuid,
        title: &str,
        expected: &str,
    ) -> Result<Option<Value>> {
        let mut tx = self.pool.begin().await?;
        access(&mut tx, id, actor, true).await?;
        let stored:Option<(Uuid,String,Option<String>,String,i64)>=sqlx::query_as("SELECT sheet_id,actor_id,title,revision_id,sequence FROM native_sheet_checkpoints WHERE id=$1")
            .bind(key).fetch_optional(&mut *tx).await?;
        let receipt = if let Some((sheet, owner, label, revision, sequence)) = stored {
            if sheet != id
                || owner != actor
                || label.as_deref() != Some(title.trim())
                || revision != expected
            {
                return Err(SheetError::Conflict(
                    "checkpoint ID reused for different input".into(),
                ));
            }
            Some(json!({"id":key,"revision_id":revision,"sequence":sequence}))
        } else {
            None
        };
        tx.commit().await?;
        Ok(receipt)
    }
}
