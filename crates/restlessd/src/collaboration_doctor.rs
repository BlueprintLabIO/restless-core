//! Explicit local workflow probes. All writes belong to one disposable cell.
use crate::{runtime, Daemon};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{process::Stdio, time::Duration};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

struct Probe<'a> {
    daemon: &'a Daemon,
    company: String,
    container: String,
    image: String,
}
impl Probe<'_> {
    async fn cli(&self, actor: &str, args: &[&str], input: &str, succeeds: bool) -> Result<Value> {
        let token = self.daemon.capabilities.issue_actor_session(
            &self.company,
            actor,
            "collaboration-doctor",
            None,
            None,
        )?;
        let operation = async {
            let mut child = tokio::process::Command::new("docker")
                .args(["run", "--rm", "--name", &self.container, "--network", "host", "--read-only", "--cpus", "1", "--memory", "256m", "--pids-limit", "64", "-i", "--entrypoint", "sh", "-e"])
                .arg(format!("RESTLESS_COMPANY={}", self.company))
                .arg("-e").arg(format!("RESTLESS_COORDINATOR=127.0.0.1:{}", crate::port_with_offset(crate::COORD_TCP_PORT)?))
                .arg(&self.image)
                // Session authority travels over stdin, never command arguments.
                .args(["-c", "IFS= read -r RESTLESS_SESSION_CAPABILITY; export RESTLESS_SESSION_CAPABILITY; exec restless \"$@\"", "doctor"])
                .args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
                .kill_on_drop(true).spawn().context("launch installed collaboration CLI")?;
            let mut stdin = child.stdin.take().context("CLI stdin unavailable")?;
            stdin
                .write_all(format!("{token}\n{input}").as_bytes())
                .await?;
            drop(stdin);
            child
                .wait_with_output()
                .await
                .context("wait for installed collaboration CLI")
        };
        let output = tokio::time::timeout(Duration::from_secs(45), operation).await;
        // Also reap a container whose docker client timed out or lost its connection.
        let _ =
            runtime::docker_bounded(&["rm", "-f", &self.container], Duration::from_secs(10)).await;
        let output = output.context("installed collaboration CLI timed out")??;
        anyhow::ensure!(
            output.status.success() == succeeds,
            "installed CLI {} returned an unexpected outcome",
            args.join(" ")
        );
        if succeeds {
            serde_json::from_slice(&output.stdout).context("decode installed CLI response")
        } else {
            let expected = match (args.first(), args.get(1)) {
                (Some(&"document"), _) => "native document is unavailable",
                (Some(&"room"), Some(&"remove")) => "only the Room owner may remove participants",
                (Some(&"room"), _) => "not an active participant in this Room",
                _ => anyhow::bail!("probe has no expected denial for this operation"),
            };
            anyhow::ensure!(
                String::from_utf8_lossy(&output.stderr).contains(expected),
                "installed CLI failed without the expected permission denial: {expected}"
            );
            Ok(Value::Null)
        }
    }
    async fn run(&self, checks: &mut Vec<Value>) -> Result<()> {
        self.cli(
            "exec",
            &[
                "people",
                "create",
                "--id",
                "probe-reader",
                "--role",
                "reader",
                "--reason",
                "Disposable collaboration permission probe",
            ],
            "",
            true,
        )
        .await?;
        let key = Uuid::new_v4().to_string();
        let create = [
            "document",
            "create",
            "--title",
            "Doctor document",
            "--markdown-file",
            "/dev/stdin",
            "--key",
            &key,
        ];
        let doc = self
            .cli("exec", &create, "# Doctor\n\nOriginal paragraph.\n", true)
            .await?;
        anyhow::ensure!(
            self.cli("exec", &create, "# Doctor\n\nOriginal paragraph.\n", true)
                .await?
                == doc,
            "document retry changed identity"
        );
        let id = doc["document_id"]
            .as_str()
            .context("created document identity")?;
        self.cli("probe-reader", &["document", "read", id], "", false)
            .await?;
        let live = self
            .cli("exec", &["document", "read", id], "", true)
            .await?;
        let block = live["blocks"]
            .as_array()
            .and_then(|v| v.last())
            .context("live block guards")?;
        let block_id = block["block_id"].as_str().context("block identity")?;
        let operations = json!([{"op":"replace","block_id":block_id,"expected_hash":block["hash"],"block":{"type":"paragraph","attrs":{"block_id":block_id},"content":[{"type":"text","text":"Verified live edit."}]}}]).to_string();
        let edit_key = Uuid::new_v4().to_string();
        let edit = [
            "document",
            "edit",
            id,
            "--operations-file",
            "/dev/stdin",
            "--key",
            &edit_key,
        ];
        let edited = self.cli("exec", &edit, &operations, true).await?;
        anyhow::ensure!(
            self.cli("exec", &edit, &operations, true).await? == edited,
            "live edit retry changed receipt"
        );
        let observed = self
            .cli("exec", &["document", "read", id], "", true)
            .await?;
        anyhow::ensure!(
            observed["content_json"]
                .to_string()
                .contains("Verified live edit."),
            "live edit did not persist"
        );
        self.cli(
            "exec",
            &[
                "document",
                "checkpoint",
                id,
                "--snapshot-file",
                "/dev/stdin",
                "--reason",
                "Doctor checkpoint",
                "--key",
                &Uuid::new_v4().to_string(),
            ],
            &observed.to_string(),
            true,
        )
        .await?;
        checks.push(json!({"id":"document_edit","status":"verified","detail":"Installed actor CLI creates, reads, edits and checkpoints; exact retries preserve identity."}));
        let snapshot = self
            .cli("exec", &["document", "snapshot", id], "", true)
            .await?;
        let revision = snapshot["document"]["version"].to_string();
        self.cli(
            "exec",
            &[
                "document",
                "share",
                id,
                "probe-reader",
                "--access",
                "comment",
                "--revision",
                &revision,
                "--key",
                &Uuid::new_v4().to_string(),
            ],
            "",
            true,
        )
        .await?;
        self.cli("probe-reader", &["document", "read", id], "", true)
            .await?;
        let forbidden = json!([{"op":"insert","after":null,"block":{"type":"paragraph","attrs":{"block_id":"doctor-forbidden"},"content":[{"type":"text","text":"Must not be inserted by a comment-only actor."}]}}]).to_string();
        self.cli(
            "probe-reader",
            &[
                "document",
                "edit",
                id,
                "--operations-file",
                "/dev/stdin",
                "--key",
                &Uuid::new_v4().to_string(),
            ],
            &forbidden,
            false,
        )
        .await?;
        let thread = self
            .cli(
                "probe-reader",
                &[
                    "document",
                    "comment",
                    id,
                    "Doctor comment",
                    "--key",
                    &Uuid::new_v4().to_string(),
                ],
                "",
                true,
            )
            .await?;
        let thread_id = thread["thread"]["thread"]["id"]
            .as_str()
            .context("comment thread identity")?;
        self.cli(
            "exec",
            &[
                "document",
                "reply",
                id,
                thread_id,
                "Doctor reply",
                "--key",
                &Uuid::new_v4().to_string(),
            ],
            "",
            true,
        )
        .await?;
        let comments = self
            .cli(
                "probe-reader",
                &["document", "comments", id, thread_id],
                "",
                true,
            )
            .await?;
        anyhow::ensure!(
            comments["items"].as_array().is_some_and(|v| v.len() == 2),
            "comment thread reply missing"
        );
        let snapshot = self
            .cli("exec", &["document", "snapshot", id], "", true)
            .await?;
        self.cli(
            "exec",
            &[
                "document",
                "unshare",
                id,
                "probe-reader",
                "--revision",
                &snapshot["document"]["version"].to_string(),
                "--key",
                &Uuid::new_v4().to_string(),
            ],
            "",
            true,
        )
        .await?;
        self.cli("probe-reader", &["document", "read", id], "", false)
            .await?;
        checks.push(json!({"id":"document_access_comments","status":"verified","detail":"Private access, comment-only editing denial, exact-thread replies and revoked reads are enforced."}));
        let room = self
            .cli(
                "exec",
                &[
                    "room",
                    "create",
                    "--title",
                    "Doctor Room",
                    "--member",
                    "probe-reader",
                    "--key",
                    &Uuid::new_v4().to_string(),
                ],
                "",
                true,
            )
            .await?;
        let room_id = room["id"].as_str().context("Room identity")?;
        let message = self
            .cli(
                "probe-reader",
                &[
                    "room",
                    "send",
                    room_id,
                    "Doctor root",
                    "--key",
                    &Uuid::new_v4().to_string(),
                ],
                "",
                true,
            )
            .await?;
        let parent = message["message"]["id"].to_string();
        self.cli(
            "exec",
            &[
                "room",
                "send",
                room_id,
                "Doctor thread reply",
                "--parent",
                &parent,
                "--key",
                &Uuid::new_v4().to_string(),
            ],
            "",
            true,
        )
        .await?;
        let thread = self
            .cli(
                "probe-reader",
                &["room", "thread", room_id, &parent],
                "",
                true,
            )
            .await?;
        anyhow::ensure!(
            thread.to_string().contains("Doctor thread reply"),
            "Room thread reply missing"
        );
        self.cli(
            "probe-reader",
            &["room", "remove", room_id, "exec"],
            "",
            false,
        )
        .await?;
        self.cli(
            "exec",
            &["room", "remove", room_id, "probe-reader"],
            "",
            true,
        )
        .await?;
        self.cli(
            "probe-reader",
            &[
                "room",
                "send",
                room_id,
                "Denied",
                "--key",
                &Uuid::new_v4().to_string(),
            ],
            "",
            false,
        )
        .await?;
        checks.push(json!({"id":"room_threads_membership","status":"verified","detail":"Installed CLI creates Rooms, posts threaded replies and enforces membership management/removal."}));
        Ok(())
    }
}

pub(crate) async fn run(daemon: &Daemon) -> Result<Value> {
    anyhow::ensure!(
        !daemon.runtime_bridges.is_hosted() && cfg!(target_os = "linux"),
        "Collaboration workflow probes currently require a local Linux installation."
    );
    static PROBE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _guard = PROBE
        .try_lock()
        .context("a collaboration workflow probe is already running")?;
    let company = format!("doctor_{}_test", Uuid::new_v4().simple());
    let config: runtime::CompanyConfig = toml::from_str(&format!(
        "name = {company:?}\nmodel = \"fake/test\"\nspend_ceiling_usd = 0\n"
    ))?;
    let probe = Probe {
        daemon,
        container: format!("restless-{company}-cli"),
        company: company.clone(),
        image: std::env::var("RESTLESS_COMPANY_IMAGE")
            .unwrap_or_else(|_| runtime::COMPANY_IMAGE.into()),
    };
    let mut checks = Vec::new();
    let outcome = async {
        runtime::CompanyConfig::save(&daemon.root, &config)?;
        daemon.authority.initialise_company(&company, &[]).await?;
        let org = daemon.orgintel.get(&company).await?;
        crate::ensure_standing_actors(&org, config.configured_model()).await?;
        let issuer = crate::owner::OwnerConfig::from_env()?
            .local_documents_issuer()
            .context("local Docs issuer unavailable")?;
        crate::local_documents::ensure(&daemon.root, &company, &org, &issuer).await?;
        probe.run(&mut checks).await
    }
    .await;
    let cleanup = async {
        let remaining = runtime::docker_bounded(
            &[
                "ps",
                "-aq",
                "--filter",
                &format!("name=^/{}$", probe.container),
            ],
            Duration::from_secs(10),
        )
        .await;
        let org = daemon.orgintel.get(&company).await?;
        runtime::destroy(
            &daemon.root,
            &daemon.orgintel.database_url,
            &company,
            &org,
            &daemon.spend,
        )
        .await?;
        daemon.orgintel.forget(&company);
        daemon.cell_wakes.remove_company(&company);
        daemon.authority.delete_test_company(&company).await?;
        let remaining = remaining?;
        anyhow::ensure!(
            remaining.status.success() && remaining.stdout.is_empty(),
            "could not verify probe CLI container removal"
        );
        Ok::<_, anyhow::Error>(())
    }
    .await;
    let verified = outcome.is_ok() && cleanup.is_ok();
    Ok(
        json!({"status":if verified {"verified"} else {"failed"},"test_company":company,"checks":checks,"failure":outcome.err().map(|e|format!("{e:#}")),"cleanup":{"status":if cleanup.is_ok(){"complete"}else{"failed"},"failure":cleanup.err().map(|e|format!("{e:#}"))},"scope":"Installed CLI, Core, Docs persistence, comments and Room permissions. Browser rendering and model replies require separate verification."}),
    )
}
