//! Keeping one actor's OMP process between turns (Sprint 113 Part B).
//!
//! Starting OMP costs about two seconds of every turn, and it reads its system prompt at launch,
//! so it cannot start early. A warm process holds no authority of its own: the model and
//! coordination grants reach it through a private file rewritten at each turn start, and the
//! relays refuse every grant of a turn that has ended (`capability::open_turn`). Only the launch
//! contract survives a turn: the same actor, system prompt, tools, model and container.
//!
//! A parked process counts against the company's memory because it is really there. It leaves
//! after a short idle limit, first when memory is short, and with the company's Runtime.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use agent_client_protocol::{Agent, ConnectionTo};
use anyhow::{Context, Result};
use sha2::{Digest as _, Sha256};
use tokio_util::sync::CancellationToken;

use super::{
    clean_up_session_processes, read_session_marker, remove_and_verify_session_artifacts, TurnHooks,
};

/// Sprint 113 starts here. Long enough to carry a conversation, short enough that an idle
/// company gives its memory back within a minute.
const IDLE_LIMIT: Duration = Duration::from_secs(60);
/// A process serves many turns of one actor, and OMP keeps its sessions in memory.
const MAX_AGE: Duration = Duration::from_secs(30 * 60);
const MAX_TURNS: u32 = 100;
/// Parked processes one company may hold; the oldest idle one leaves first.
const MAX_PER_COMPANY: usize = 4;

pub(super) const MODEL_CAPABILITY_FILE_ENV: &str = "RESTLESS_MODEL_CAPABILITY_FILE";
pub(super) const SESSION_CAPABILITY_FILE_ENV: &str = "RESTLESS_SESSION_CAPABILITY_FILE";

/// Where a launch's model and coordination grants arrive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialDelivery {
    /// In the process environment, fixed for the life of the process.
    Environment,
    /// In a private file rewritten at each turn start. Requires a `restless` CLI that reads it.
    TurnFile,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct WarmKey {
    company: String,
    actor: String,
    container: String,
    contract: String,
}

impl WarmKey {
    /// One process per actor and launch contract. The container stands for the image: a new
    /// image is a new container, and a process cannot outlive its container.
    #[expect(
        clippy::too_many_arguments,
        reason = "the launch contract is exactly these fields, named at the one call site"
    )]
    pub(super) fn new(
        company: &str,
        actor: &str,
        container: &str,
        build: &str,
        model: &str,
        effort: &str,
        workdir: &str,
        system_prompt: &str,
        native_tools: &[&str],
        team_coordination_wake: bool,
    ) -> Self {
        let mut hash = Sha256::new();
        for field in [
            build,
            model,
            effort,
            workdir,
            system_prompt,
            &native_tools.join(","),
            if team_coordination_wake { "wake" } else { "" },
        ] {
            hash.update(field.as_bytes());
            hash.update([0]);
        }
        Self {
            company: company.to_string(),
            actor: actor.to_string(),
            container: container.to_string(),
            contract: format!("{:x}", hash.finalize()),
        }
    }
}

/// One running agent process and the connection that speaks to it.
pub(super) struct AgentProcess {
    pub(super) child: tokio::process::Child,
    pub(super) cx: ConnectionTo<Agent>,
    pub(super) hooks: Arc<Mutex<Option<TurnHooks>>>,
    pub(super) stop: CancellationToken,
    pub(super) stderr_task: tokio::task::JoinHandle<String>,
    pub(super) container: String,
    pub(super) actor: String,
    pub(super) launched_at: chrono::DateTime<chrono::Utc>,
    pub(super) session_marker: String,
    pub(super) system_prompt_path: String,
    pub(super) session_runtime: String,
    /// `None` unless this process receives its grants through files.
    pub(super) credentials_dir: Option<String>,
    pub(super) load_session_advertised: bool,
    /// The provider session held in this process, and whether MCP servers are attached to it.
    pub(super) live_session: Option<String>,
    pub(super) has_mcp: bool,
    pub(super) started: Instant,
    pub(super) turns: u32,
}

impl AgentProcess {
    pub(super) fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    pub(super) fn model_file(&self) -> Option<String> {
        self.credentials_dir
            .as_ref()
            .map(|dir| format!("{dir}/model"))
    }

    pub(super) fn coordination_file(&self) -> Option<String> {
        self.credentials_dir
            .as_ref()
            .map(|dir| format!("{dir}/coordination"))
    }

    fn is_due_for_retirement(&self) -> bool {
        self.started.elapsed() >= MAX_AGE || self.turns >= MAX_TURNS
    }

    /// Stop the process and remove everything it left, as a turn end does for a cold process.
    pub(super) async fn terminate(mut self) -> Result<String> {
        self.stop.cancel();
        let owned = read_session_marker(&self.container, &self.session_marker).await;
        let _ = self.child.kill().await;
        let stderr = self.stderr_task.await.unwrap_or_default();
        let (processes, keep_marker) = clean_up_session_processes(
            &self.container,
            &self.session_marker,
            owned,
            self.launched_at,
            &self.actor,
        )
        .await;
        let mut artifacts = vec![self.system_prompt_path.clone(), self.session_runtime.clone()];
        if !keep_marker {
            artifacts.push(self.session_marker.clone());
        }
        if let Some(dir) = &self.credentials_dir {
            artifacts.push(dir.clone());
        }
        let paths = artifacts.iter().map(String::as_str).collect::<Vec<_>>();
        processes.and(remove_and_verify_session_artifacts(&self.container, &paths).await)?;
        Ok(stderr)
    }
}

struct Parked {
    process: AgentProcess,
    idle_since: Instant,
    generation: u64,
}

fn parked() -> &'static Mutex<HashMap<WarmKey, Parked>> {
    static PARKED: OnceLock<Mutex<HashMap<WarmKey, Parked>>> = OnceLock::new();
    PARKED.get_or_init(Mutex::default)
}

static GENERATION: AtomicU64 = AtomicU64::new(0);

/// Whether the container's `restless` reads its grant from a file. An older CLI ignores the file
/// variable and, finding no environment grant, would fall back to the Runtime's bridge identity,
/// which is the Exec: so a launch must know before it leaves the grant out of the environment.
fn cli_support() -> &'static Mutex<HashMap<String, bool>> {
    static SUPPORT: OnceLock<Mutex<HashMap<String, bool>>> = OnceLock::new();
    SUPPORT.get_or_init(Mutex::default)
}

async fn cli_reads_capability_file(container: &str) -> bool {
    if let Some(known) = cli_support()
        .lock()
        .ok()
        .and_then(|known| known.get(container).copied())
    {
        return known;
    }
    let script = format!("grep -aqF {SESSION_CAPABILITY_FILE_ENV} \"$(command -v restless)\"");
    let supported = crate::runtime::docker_bounded(
        &["exec", container, "sh", "-c", &script],
        Duration::from_secs(8),
    )
    .await
    .is_ok_and(|output| output.status.success());
    if let Ok(mut known) = cli_support().lock() {
        known.insert(container.to_string(), supported);
    }
    supported
}

/// File delivery needs a turn-bound managed launch and a CLI that understands it.
pub(super) async fn credential_delivery(
    harness: crate::runtime::AgentHarness,
    model: &str,
    turn_bound: bool,
    container: &str,
) -> CredentialDelivery {
    if turn_bound
        && harness == crate::runtime::AgentHarness::RestlessManaged
        && !model.starts_with("native-")
        && cli_reads_capability_file(container).await
    {
        CredentialDelivery::TurnFile
    } else {
        CredentialDelivery::Environment
    }
}

/// Take this actor's parked process, if it is still running and within its limits.
pub(super) async fn take(key: &WarmKey) -> Option<AgentProcess> {
    let entry = parked().lock().ok()?.remove(key)?;
    let mut process = entry.process;
    if process.is_running() && !process.is_due_for_retirement() {
        return Some(process);
    }
    retire(process, "no longer usable").await;
    None
}

/// Keep a process for this actor's next turn, or stop it when the company has no room for it.
pub(super) async fn park(key: WarmKey, mut process: AgentProcess) {
    process.turns += 1;
    if process.is_due_for_retirement() {
        retire(process, "reached its age or turn limit").await;
        return;
    }
    if !memory_allows_parking(&key.company).await {
        retire(process, "the company needs its memory").await;
        return;
    }
    let generation = GENERATION.fetch_add(1, Ordering::Relaxed);
    let oldest = {
        let Ok(mut parked) = parked().lock() else {
            retire(process, "the registry is unavailable").await;
            return;
        };
        let oldest = oldest_over_limit(&parked, &key.company);
        let displaced = parked.insert(
            key.clone(),
            Parked {
                process,
                idle_since: Instant::now(),
                generation,
            },
        );
        (oldest, displaced)
    };
    let (oldest, displaced) = oldest;
    if let Some(displaced) = displaced {
        retire(displaced.process, "replaced by a newer process").await;
    }
    if let Some(oldest) = oldest {
        retire_key(&oldest, "the company's oldest idle process").await;
    }
    tokio::spawn(async move {
        tokio::time::sleep(IDLE_LIMIT).await;
        let expired = parked().lock().ok().and_then(|mut parked| {
            let current = parked.get(&key).is_some_and(|p| p.generation == generation);
            current.then(|| parked.remove(&key)).flatten()
        });
        if let Some(entry) = expired {
            retire(entry.process, "idle limit reached").await;
        }
    });
}

/// The key of the company's longest-idle process, when it already holds the most it may.
fn oldest_over_limit(parked: &HashMap<WarmKey, Parked>, company: &str) -> Option<WarmKey> {
    let held = parked.keys().filter(|key| key.company == company).count();
    (held >= MAX_PER_COMPANY)
        .then(|| {
            parked
                .iter()
                .filter(|(key, _)| key.company == company)
                .min_by_key(|(_, entry)| entry.idle_since)
                .map(|(key, _)| key.clone())
        })
        .flatten()
}

async fn memory_allows_parking(company: &str) -> bool {
    match crate::runtime::actor_capacity(company).await {
        Ok(Some(capacity)) => !capacity.waiting_for_memory && capacity.company_memory_slots >= 2,
        // A host with no workload pool: the slots already follow what memory is available.
        Ok(None) => crate::runtime::host_actor_capacity().company_slots >= 2,
        Err(error) => {
            tracing::warn!(company, %error, "could not read memory to park an agent process");
            false
        }
    }
}

async fn retire_key(key: &WarmKey, reason: &str) {
    let entry = parked().lock().ok().and_then(|mut p| p.remove(key));
    if let Some(entry) = entry {
        retire(entry.process, reason).await;
    }
}

pub(super) async fn retire(process: AgentProcess, reason: &str) {
    tracing::info!(actor = %process.actor, reason, "stopping warm agent process");
    if let Err(error) = process.terminate().await {
        tracing::warn!(%error, "warm agent process cleanup was incomplete");
    }
}

/// An actor start is waiting for memory or a slot: the longest-idle warm process leaves first.
pub fn evict_idle_for_capacity(company: &str) {
    let key = parked().lock().ok().and_then(|parked| {
        parked
            .iter()
            .filter(|(key, _)| key.company == company)
            .min_by_key(|(_, entry)| entry.idle_since)
            .map(|(key, _)| key.clone())
    });
    if let Some(key) = key {
        tokio::spawn(async move { retire_key(&key, "an actor start needs the capacity").await });
    }
}

/// Stop every parked process of a company whose Runtime is stopping or being replaced.
pub async fn drain_company(company: &str, container: &str) {
    let entries = match parked().lock() {
        Ok(mut parked) => {
            let keys = parked
                .keys()
                .filter(|key| key.company == company)
                .cloned()
                .collect::<Vec<_>>();
            keys.iter().filter_map(|key| parked.remove(key)).collect()
        }
        Err(_) => Vec::new(),
    };
    for entry in entries {
        retire(entry.process, "the company's Runtime is stopping").await;
    }
    if let Ok(mut known) = cli_support().lock() {
        known.remove(container);
    }
}

/// Stop every parked process on the way out of the daemon.
pub async fn drain_all() {
    let entries = match parked().lock() {
        Ok(mut parked) => parked.drain().map(|(_, entry)| entry).collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };
    for entry in entries {
        retire(entry.process, "the daemon is stopping").await;
    }
}

/// Remove a process's grants from the container and every process it started during the turn,
/// leaving only the agent itself. The session leader is the agent: it `exec`s into OMP.
pub(super) async fn end_turn_in_process(process: &AgentProcess) -> Result<()> {
    if let Some(dir) = &process.credentials_dir {
        remove_and_verify_session_artifacts(
            &process.container,
            &[&format!("{dir}/model"), &format!("{dir}/coordination")],
        )
        .await
        .context("remove the turn's credential files")?;
    }
    if let super::SessionMarker::Owned(session_id) =
        read_session_marker(&process.container, &process.session_marker).await
    {
        reap_children(&process.container, &session_id).await;
    }
    Ok(())
}

/// Every process of this Linux session except its leader. A warm turn ends by reaping these, as
/// a cold turn's end reaps the whole session.
fn children_in_session(table: &str, session_id: &str) -> Vec<String> {
    super::pids_in_session(table, session_id)
        .into_iter()
        .filter(|pid| pid != session_id)
        .collect()
}

async fn reap_children(container: &str, session_id: &str) {
    let Ok(output) = crate::runtime::docker_bounded(
        &["exec", container, "ps", "-eo", "pid=,sid="],
        Duration::from_secs(8),
    )
    .await
    else {
        return;
    };
    let leaked = children_in_session(&String::from_utf8_lossy(&output.stdout), session_id);
    if leaked.is_empty() {
        return;
    }
    let mut args = vec!["exec", container, "kill", "-9"];
    args.extend(leaked.iter().map(String::as_str));
    let _ = crate::runtime::docker_bounded(&args, Duration::from_secs(8)).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(prompt: &str, model: &str) -> WarmKey {
        WarmKey::new(
            "acme_test",
            "delivery-lead",
            "restless-acme",
            "omp 1",
            model,
            "medium",
            "/company",
            prompt,
            &["read", "bash"],
            false,
        )
    }

    #[test]
    fn any_change_to_the_launch_contract_changes_the_key() {
        let original = key("a prompt", "anthropic/claude-sonnet-5-5");
        assert_eq!(original, key("a prompt", "anthropic/claude-sonnet-5-5"));
        assert_ne!(original, key("another prompt", "anthropic/claude-sonnet-5-5"));
        assert_ne!(original, key("a prompt", "anthropic/claude-opus-5-5"));
    }

    #[test]
    fn a_warm_turn_end_reaps_children_but_not_the_agent() {
        let table = "  7 7\n 12 7\n 13 7\n 20 9\n";
        assert_eq!(children_in_session(table, "7"), vec!["12", "13"]);
    }
}
