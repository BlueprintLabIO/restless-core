//! Runtime-side half of the hosted Company Runtime bridge.
//!
//! The process is supervised inside the immutable company image. It makes one
//! outbound authenticated websocket, launches only the fixed ACP harnesses in
//! the bridge contract, and exposes the existing Restless coordination JSONL
//! protocol on loopback. It is not a remote shell.

use std::collections::HashMap;
use std::io::{Cursor, Read as _};
use std::os::unix::fs::{MetadataExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context as _, Result};
use base64::Engine as _;
use futures_util::{SinkExt as _, StreamExt as _};
use http::header::AUTHORIZATION;
use restless_runtime_bridge_protocol::{
    gate_definition_digest, ArtifactReceipt, ArtifactSelector, AttemptCleanupReceipt,
    CandidateIdentity, CoordinationMethod, Frame, GateCompletionStatus, GateDefinition,
    GateReceipt, GateResources, Message as BridgeMessage, PreflightStatus, PromotionReceipt,
    ReleaseIdentity, ReviewCopyReceipt, RuntimeIdentity, SequenceTracker, WorkspaceIdentity,
    WorkspaceObservation, PROTOCOL_VERSION,
};
use serde::Deserialize;
use sha2::{Digest as _, Sha256};
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, BufReader};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;
use tokio_tungstenite::tungstenite::Message as WebSocketMessage;
use tokio_tungstenite::Connector;
use uuid::Uuid;

const COORDINATION_ADDRESS: &str = "127.0.0.1:7791";
const HEALTH_URL: &str = "http://127.0.0.1:7789/health";
const MAX_COORDINATION_BYTES: usize = 128 * 1024;
const OUTBOUND_DEPTH: usize = 128;
const ACP_CHUNK_BYTES: usize = 48 * 1024;
const SEND_DEADLINE: Duration = Duration::from_secs(10);
const MAX_CA_BYTES: u64 = 64 * 1024;
const MAX_GIT_STDOUT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_GIT_STDERR_BYTES: u64 = 16 * 1024;
const MAX_GATE_STDOUT_BYTES: u64 = 512 * 1024;
const MAX_GATE_STDERR_BYTES: u64 = 512 * 1024;
const MAX_ARTIFACT_FILES: usize = 100_000;
const MAX_ARTIFACT_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const WORKSPACE_RESULT_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_WORKSPACE_RESULTS: usize = 1_024;
const BRIDGE_SECRET_GID: u32 = 10_002;
const OMP_CONFIG: &str = include_str!("../../restlessd/omp-runtime.yml");

#[derive(Clone)]
struct Config {
    url: String,
    identity: RuntimeIdentity,
    capability_file: PathBuf,
    tls_connector: Connector,
    /// The same bounded trust anchor used by the bridge connection. Hosted
    /// model traffic leaves from the uid-2000 ACP child, so it needs its own
    /// private, per-operation projection after the bridge drops privileges.
    agent_ca_pem: Option<Vec<u8>>,
}

#[derive(Debug)]
struct Outbound {
    message: BridgeMessage,
    response: Option<oneshot::Sender<BridgeMessage>>,
}

struct AgentControl {
    stdin: mpsc::Sender<Vec<u8>>,
    cancel: oneshot::Sender<String>,
}

#[derive(Clone, Default)]
struct WorkspaceOperations {
    state: Arc<Mutex<WorkspaceOperationState>>,
}

#[derive(Default)]
struct WorkspaceOperationState {
    active: HashMap<Uuid, ActiveWorkspaceOperation>,
    completed: HashMap<Uuid, CompletedWorkspaceOperation>,
}

struct ActiveWorkspaceOperation {
    request: BridgeMessage,
    connection_nonce: Uuid,
    cancel: oneshot::Sender<String>,
}

struct CompletedWorkspaceOperation {
    request: BridgeMessage,
    response: BridgeMessage,
    completed_at: Instant,
}

struct WorkspaceConnectionGuard {
    operations: WorkspaceOperations,
    connection_nonce: Uuid,
}

impl Drop for WorkspaceConnectionGuard {
    fn drop(&mut self) {
        self.operations.cancel_connection(self.connection_nonce);
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HealthEnvelope {
    status: String,
    release: ReleaseIdentity,
}

impl WorkspaceOperations {
    fn start(
        &self,
        config: Config,
        connection_nonce: Uuid,
        environment_fingerprint: String,
        request: BridgeMessage,
        outbound: mpsc::Sender<Outbound>,
    ) -> Result<()> {
        let operation_id = workspace_operation_id(&request)
            .context("workspace request has no operation identity")?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("workspace operations"))?;
        prune_workspace_results(&mut state);
        if let Some(completed) = state.completed.get(&operation_id) {
            let response = if same_workspace_request(&completed.request, &request) {
                completed.response.clone()
            } else {
                BridgeMessage::Error {
                    operation_id: Some(operation_id),
                    code: "operation_replay_conflict".into(),
                    message: "workspace operation id was replayed with different coordinates"
                        .into(),
                }
            };
            drop(state);
            tokio::spawn(async move {
                if let Err(error) = queue(&outbound, response).await {
                    eprintln!("Runtime workspace replay could not be returned: {error:#}");
                }
            });
            return Ok(());
        }
        if let Some(active) = state.active.get(&operation_id) {
            if same_workspace_request(&active.request, &request) {
                // The original operation owns the one terminal response. A
                // byte-identical replay while it is active is a no-op.
                return Ok(());
            }
            let response = BridgeMessage::Error {
                operation_id: Some(operation_id),
                code: "operation_replay_conflict".into(),
                message: "active workspace operation id names different coordinates".into(),
            };
            drop(state);
            tokio::spawn(async move {
                let _ = queue(&outbound, response).await;
            });
            return Ok(());
        }
        let (cancel_tx, mut cancel_rx) = oneshot::channel();
        state.active.insert(
            operation_id,
            ActiveWorkspaceOperation {
                request: request.clone(),
                connection_nonce,
                cancel: cancel_tx,
            },
        );
        drop(state);

        let operations = self.clone();
        tokio::spawn(async move {
            let deadline_ms = workspace_deadline_ms(&request).unwrap_or_default();
            let remaining_ms = deadline_ms.saturating_sub(now_ms().unwrap_or(i64::MAX));
            let deadline = tokio::time::Instant::now()
                + Duration::from_millis(u64::try_from(remaining_ms).unwrap_or_default());
            let execution =
                execute_workspace_operation(&config, &environment_fingerprint, request.clone());
            tokio::pin!(execution);
            let response = tokio::select! {
                biased;
                _ = &mut cancel_rx => None,
                _ = tokio::time::sleep_until(deadline) => Some(BridgeMessage::Error {
                    operation_id: Some(operation_id),
                    code: "workspace_timeout".into(),
                    message: "typed Runtime workspace operation exceeded its deadline".into(),
                }),
                result = &mut execution => Some(match result {
                    Ok(response) => response,
                    Err(error) => BridgeMessage::Error {
                        operation_id: Some(operation_id),
                        code: "workspace_refused".into(),
                        message: bounded_error(&error),
                    },
                }),
            };

            let Some(response) = response else {
                return;
            };
            let should_send = {
                let Ok(mut state) = operations.state.lock() else {
                    return;
                };
                let owned = state.active.get(&operation_id).is_some_and(|active| {
                    active.connection_nonce == connection_nonce && active.request == request
                });
                if owned {
                    state.active.remove(&operation_id);
                    if matches!(
                        response,
                        BridgeMessage::WorkspacePrepared { .. }
                            | BridgeMessage::WorkspaceObserved { .. }
                            | BridgeMessage::GateCompleted { .. }
                            | BridgeMessage::ArtifactObserved { .. }
                            | BridgeMessage::CandidatePromoted { .. }
                            | BridgeMessage::ReviewCopyPrepared { .. }
                            | BridgeMessage::AttemptCleaned { .. }
                    ) {
                        state.completed.insert(
                            operation_id,
                            CompletedWorkspaceOperation {
                                request,
                                response: response.clone(),
                                completed_at: Instant::now(),
                            },
                        );
                        prune_workspace_results(&mut state);
                    }
                }
                owned
            };
            if should_send {
                if let Err(error) = queue(&outbound, response).await {
                    eprintln!("Runtime workspace result could not be returned: {error:#}");
                }
            }
        });
        Ok(())
    }

    fn cancel(&self, operation_id: Uuid, reason: String) {
        let active = self
            .state
            .lock()
            .ok()
            .and_then(|mut state| state.active.remove(&operation_id));
        if let Some(active) = active {
            let _ = active.cancel.send(reason);
        }
    }

    fn cancel_connection(&self, connection_nonce: Uuid) {
        let active = self
            .state
            .lock()
            .map(|mut state| {
                let operation_ids = state
                    .active
                    .iter()
                    .filter_map(|(operation_id, active)| {
                        (active.connection_nonce == connection_nonce).then_some(*operation_id)
                    })
                    .collect::<Vec<_>>();
                operation_ids
                    .into_iter()
                    .filter_map(|operation_id| state.active.remove(&operation_id))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for active in active {
            let _ = active
                .cancel
                .send("Runtime bridge connection closed".into());
        }
    }
}

fn prune_workspace_results(state: &mut WorkspaceOperationState) {
    state
        .completed
        .retain(|_, result| result.completed_at.elapsed() <= WORKSPACE_RESULT_TTL);
    if state.completed.len() > MAX_WORKSPACE_RESULTS {
        let mut oldest = state
            .completed
            .iter()
            .map(|(operation_id, result)| (*operation_id, result.completed_at))
            .collect::<Vec<_>>();
        oldest.sort_unstable_by_key(|(_, completed_at)| *completed_at);
        for (operation_id, _) in oldest
            .into_iter()
            .take(state.completed.len() - MAX_WORKSPACE_RESULTS)
        {
            state.completed.remove(&operation_id);
        }
    }
}

fn workspace_operation_id(message: &BridgeMessage) -> Option<Uuid> {
    match message {
        BridgeMessage::PrepareWorkspace { operation_id, .. }
        | BridgeMessage::ObserveWorkspace { operation_id, .. }
        | BridgeMessage::RunGate { operation_id, .. }
        | BridgeMessage::ObserveArtifact { operation_id, .. }
        | BridgeMessage::PromoteCandidate { operation_id, .. }
        | BridgeMessage::PrepareReviewCopy { operation_id, .. }
        | BridgeMessage::CleanupAttempt { operation_id, .. } => Some(*operation_id),
        _ => None,
    }
}

fn workspace_deadline_ms(message: &BridgeMessage) -> Option<i64> {
    match message {
        BridgeMessage::PrepareWorkspace { deadline_ms, .. }
        | BridgeMessage::ObserveWorkspace { deadline_ms, .. }
        | BridgeMessage::RunGate { deadline_ms, .. }
        | BridgeMessage::ObserveArtifact { deadline_ms, .. }
        | BridgeMessage::PromoteCandidate { deadline_ms, .. }
        | BridgeMessage::PrepareReviewCopy { deadline_ms, .. }
        | BridgeMessage::CleanupAttempt { deadline_ms, .. } => Some(*deadline_ms),
        _ => None,
    }
}

/// Deadlines are transport freshness, not part of an operation's durable
/// identity. Core must be able to replay the same typed operation after a
/// lost response with a fresh bounded deadline, while every authority-bearing
/// coordinate remains byte-for-byte equal.
fn same_workspace_request(left: &BridgeMessage, right: &BridgeMessage) -> bool {
    match (left, right) {
        (
            BridgeMessage::PrepareWorkspace {
                operation_id: left_operation,
                workspace: left_workspace,
                requested_source_ref: left_ref,
                expected_source_commit: left_commit,
                expected_source_tree: left_tree,
                ..
            },
            BridgeMessage::PrepareWorkspace {
                operation_id: right_operation,
                workspace: right_workspace,
                requested_source_ref: right_ref,
                expected_source_commit: right_commit,
                expected_source_tree: right_tree,
                ..
            },
        ) => {
            left_operation == right_operation
                && left_workspace == right_workspace
                && left_ref == right_ref
                && left_commit == right_commit
                && left_tree == right_tree
        }
        (
            BridgeMessage::ObserveWorkspace {
                operation_id: left_operation,
                workspace: left_workspace,
                expected_environment_fingerprint: left_environment,
                ..
            },
            BridgeMessage::ObserveWorkspace {
                operation_id: right_operation,
                workspace: right_workspace,
                expected_environment_fingerprint: right_environment,
                ..
            },
        ) => {
            left_operation == right_operation
                && left_workspace == right_workspace
                && left_environment == right_environment
        }
        (
            BridgeMessage::RunGate {
                operation_id: left_operation,
                workspace: left_workspace,
                candidate: left_candidate,
                definition: left_definition,
                definition_digest: left_digest,
                resources: left_resources,
                ..
            },
            BridgeMessage::RunGate {
                operation_id: right_operation,
                workspace: right_workspace,
                candidate: right_candidate,
                definition: right_definition,
                definition_digest: right_digest,
                resources: right_resources,
                ..
            },
        ) => {
            left_operation == right_operation
                && left_workspace == right_workspace
                && left_candidate == right_candidate
                && left_definition == right_definition
                && left_digest == right_digest
                && left_resources == right_resources
        }
        (
            BridgeMessage::ObserveArtifact {
                operation_id: left_operation,
                workspace: left_workspace,
                candidate: left_candidate,
                selector: left_selector,
                ..
            },
            BridgeMessage::ObserveArtifact {
                operation_id: right_operation,
                workspace: right_workspace,
                candidate: right_candidate,
                selector: right_selector,
                ..
            },
        ) => {
            left_operation == right_operation
                && left_workspace == right_workspace
                && left_candidate == right_candidate
                && left_selector == right_selector
        }
        (
            BridgeMessage::PromoteCandidate {
                operation_id: left_operation,
                workspace: left_workspace,
                candidate: left_candidate,
                integration_branch: left_branch,
                ..
            },
            BridgeMessage::PromoteCandidate {
                operation_id: right_operation,
                workspace: right_workspace,
                candidate: right_candidate,
                integration_branch: right_branch,
                ..
            },
        ) => {
            left_operation == right_operation
                && left_workspace == right_workspace
                && left_candidate == right_candidate
                && left_branch == right_branch
        }
        (
            BridgeMessage::PrepareReviewCopy {
                operation_id: left_operation,
                workspace: left_workspace,
                candidate: left_candidate,
                ..
            },
            BridgeMessage::PrepareReviewCopy {
                operation_id: right_operation,
                workspace: right_workspace,
                candidate: right_candidate,
                ..
            },
        ) => {
            left_operation == right_operation
                && left_workspace == right_workspace
                && left_candidate == right_candidate
        }
        (
            BridgeMessage::CleanupAttempt {
                operation_id: left_operation,
                workspace: left_workspace,
                expected_environment_fingerprint: left_environment,
                ..
            },
            BridgeMessage::CleanupAttempt {
                operation_id: right_operation,
                workspace: right_workspace,
                expected_environment_fingerprint: right_environment,
                ..
            },
        ) => {
            left_operation == right_operation
                && left_workspace == right_workspace
                && left_environment == right_environment
        }
        _ => false,
    }
}

async fn execute_workspace_operation(
    config: &Config,
    environment_fingerprint: &str,
    request: BridgeMessage,
) -> Result<BridgeMessage> {
    match request {
        BridgeMessage::PrepareWorkspace {
            operation_id,
            workspace,
            requested_source_ref,
            expected_source_commit,
            expected_source_tree,
            ..
        } => {
            let (observation, reused) = prepare_workspace(
                config,
                &workspace,
                requested_source_ref.as_deref(),
                expected_source_commit.as_deref(),
                expected_source_tree.as_deref(),
                environment_fingerprint,
            )
            .await?;
            Ok(BridgeMessage::WorkspacePrepared {
                operation_id,
                workspace,
                requested_source_ref,
                observation,
                reused,
            })
        }
        BridgeMessage::ObserveWorkspace {
            operation_id,
            workspace,
            expected_environment_fingerprint,
            ..
        } => {
            if expected_environment_fingerprint != environment_fingerprint {
                bail!("Runtime environment differs from frozen Attempt coordinates");
            }
            let observation = observe_workspace(&workspace, environment_fingerprint).await?;
            Ok(BridgeMessage::WorkspaceObserved {
                operation_id,
                workspace,
                observation,
            })
        }
        BridgeMessage::RunGate {
            operation_id,
            workspace,
            candidate,
            definition,
            definition_digest,
            resources,
            ..
        } => {
            if candidate.environment_fingerprint != environment_fingerprint {
                bail!("Runtime environment differs from frozen gate candidate");
            }
            if definition_digest != gate_definition_digest(&definition) {
                bail!("frozen gate definition digest does not match its argv");
            }
            let receipt = execute_gate_operation(
                operation_id,
                &workspace,
                &candidate,
                &definition,
                &definition_digest,
                &resources,
                environment_fingerprint,
            )
            .await?;
            Ok(BridgeMessage::GateCompleted {
                operation_id,
                workspace,
                receipt,
            })
        }
        BridgeMessage::ObserveArtifact {
            operation_id,
            workspace,
            candidate,
            selector,
            ..
        } => {
            if candidate.environment_fingerprint != environment_fingerprint {
                bail!("Runtime environment differs from frozen artifact candidate");
            }
            let receipt = observe_artifact_operation(
                &workspace,
                &candidate,
                selector,
                environment_fingerprint,
            )
            .await?;
            Ok(BridgeMessage::ArtifactObserved {
                operation_id,
                workspace,
                receipt,
            })
        }
        BridgeMessage::PromoteCandidate {
            operation_id,
            workspace,
            candidate,
            integration_branch,
            ..
        } => {
            if candidate.environment_fingerprint != environment_fingerprint {
                bail!("Runtime environment differs from frozen promotion candidate");
            }
            let receipt = promote_candidate_operation(
                &workspace,
                &candidate,
                &integration_branch,
                environment_fingerprint,
            )
            .await?;
            Ok(BridgeMessage::CandidatePromoted {
                operation_id,
                workspace,
                receipt,
            })
        }
        BridgeMessage::PrepareReviewCopy {
            operation_id,
            workspace,
            candidate,
            ..
        } => {
            if candidate.environment_fingerprint != environment_fingerprint {
                bail!("Runtime environment differs from frozen review candidate");
            }
            let receipt =
                prepare_review_copy_operation(&workspace, &candidate, environment_fingerprint)
                    .await?;
            Ok(BridgeMessage::ReviewCopyPrepared {
                operation_id,
                workspace,
                receipt,
            })
        }
        BridgeMessage::CleanupAttempt {
            operation_id,
            workspace,
            expected_environment_fingerprint,
            ..
        } => {
            if expected_environment_fingerprint != environment_fingerprint {
                bail!("Runtime environment differs from frozen cleanup coordinates");
            }
            let receipt = cleanup_attempt_operation(&workspace).await?;
            Ok(BridgeMessage::AttemptCleaned {
                operation_id,
                workspace,
                receipt,
            })
        }
        _ => bail!("not a typed Runtime workspace request"),
    }
}

async fn prepare_workspace(
    _config: &Config,
    workspace: &WorkspaceIdentity,
    requested_source_ref: Option<&str>,
    expected_source_commit: Option<&str>,
    expected_source_tree: Option<&str>,
    environment_fingerprint: &str,
) -> Result<(WorkspaceObservation, bool)> {
    let (repo_path, workdir) = workspace_paths(workspace)?;
    ensure_plain_directory(Path::new("/company"))?;
    ensure_plain_directory(Path::new("/company/repos"))?;
    ensure_plain_directory(&repo_path)?;
    let directories = run_company_command(
        "mkdir",
        &[
            "-p".into(),
            "--".into(),
            "/company/worktrees".into(),
            format!("/company/run/attempts/{}", workspace.attempt_id),
        ],
    )
    .await?;
    require_success("prepare Attempt workspace directories", &directories)?;
    ensure_plain_directory(Path::new("/company/worktrees"))?;
    ensure_contained_directory(Path::new("/company/repos"), &repo_path)?;

    let (source_commit, source_tree) = match (expected_source_commit, expected_source_tree) {
        (Some(commit), Some(tree)) => {
            let observed_tree = resolve_tree(&repo_path, commit).await?;
            if observed_tree != tree {
                bail!("frozen source commit no longer resolves to its exact tree");
            }
            (commit.to_string(), tree.to_string())
        }
        (None, None) => {
            let requested = requested_source_ref.unwrap_or("HEAD");
            let commit = resolve_commit(&repo_path, requested).await?;
            let tree = resolve_tree(&repo_path, &commit).await?;
            (commit, tree)
        }
        _ => bail!("frozen commit and tree must be supplied together"),
    };

    let reused = match std::fs::symlink_metadata(&workdir) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                bail!("Attempt worktree path is not a plain directory");
            }
            ensure_contained_directory(Path::new("/company/worktrees"), &workdir)?;
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let output = run_company_command(
                "git",
                &[
                    "-C".into(),
                    path_text(&repo_path)?,
                    "worktree".into(),
                    "add".into(),
                    "--detach".into(),
                    path_text(&workdir)?,
                    source_commit.clone(),
                ],
            )
            .await?;
            require_success("create detached Attempt worktree", &output)?;
            ensure_contained_directory(Path::new("/company/worktrees"), &workdir)?;
            false
        }
        Err(error) => return Err(error).context("inspect Attempt worktree path"),
    };

    let observation = observe_workspace(workspace, environment_fingerprint).await?;
    if observation.source_commit != source_commit || observation.source_tree != source_tree {
        bail!("materialized workspace differs from frozen source {source_commit}/{source_tree}");
    }
    Ok((observation, reused))
}

async fn observe_workspace(
    workspace: &WorkspaceIdentity,
    environment_fingerprint: &str,
) -> Result<WorkspaceObservation> {
    let (repo_path, workdir) = workspace_paths(workspace)?;
    ensure_contained_directory(Path::new("/company/repos"), &repo_path)?;
    ensure_contained_directory(Path::new("/company/worktrees"), &workdir)?;
    let source_commit = resolve_commit(&workdir, "HEAD").await?;
    let source_tree = resolve_tree(&workdir, &source_commit).await?;
    let status = run_company_command(
        "git",
        &[
            "-C".into(),
            path_text(&workdir)?,
            "status".into(),
            "--porcelain=v1".into(),
            "-z".into(),
            "--untracked-files=all".into(),
        ],
    )
    .await?;
    require_success("observe Attempt worktree status", &status)?;
    let dirty_entries = u32::try_from(dirty_entry_count(&status.stdout)?)
        .context("Attempt status entry count exceeds protocol")?;
    Ok(WorkspaceObservation {
        repo_path: path_text(&repo_path)?,
        workdir: path_text(&workdir)?,
        source_commit,
        source_tree,
        status_digest: format!("{:x}", Sha256::digest(&status.stdout)),
        dirty_entries,
        environment_fingerprint: environment_fingerprint.to_string(),
        observed_at_ms: now_ms()?,
    })
}

fn candidate_from_observation(observation: &WorkspaceObservation) -> CandidateIdentity {
    CandidateIdentity {
        source_commit: observation.source_commit.clone(),
        source_tree: observation.source_tree.clone(),
        status_digest: observation.status_digest.clone(),
        environment_fingerprint: observation.environment_fingerprint.clone(),
    }
}

async fn require_exact_candidate(
    workspace: &WorkspaceIdentity,
    candidate: &CandidateIdentity,
    environment_fingerprint: &str,
) -> Result<WorkspaceObservation> {
    if candidate.environment_fingerprint != environment_fingerprint {
        bail!("candidate belongs to another Runtime environment");
    }
    let observation = observe_workspace(workspace, environment_fingerprint).await?;
    if observation.dirty_entries != 0 || candidate_from_observation(&observation) != *candidate {
        bail!("Attempt workspace is dirty, divergent, or stale relative to the frozen candidate");
    }
    Ok(observation)
}

struct GateProcessEvidence {
    status: GateCompletionStatus,
    exit_code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    duration_ms: u64,
    leaked_processes: u32,
}

async fn execute_gate_operation(
    operation_id: Uuid,
    workspace: &WorkspaceIdentity,
    candidate: &CandidateIdentity,
    definition: &GateDefinition,
    definition_digest: &str,
    resources: &GateResources,
    environment_fingerprint: &str,
) -> Result<GateReceipt> {
    require_exact_candidate(workspace, candidate, environment_fingerprint).await?;
    prepare_gate_runtime(operation_id, workspace, resources).await?;
    let result = async {
        let (_, workdir) = workspace_paths(workspace)?;
        let cwd = gate_cwd(definition, &workdir)?;
        let mut argv = definition.argv.clone();
        for value in &mut argv {
            *value = value
                .replace(
                    "{RESTLESS_GATE_PORT}",
                    &resources
                        .port
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                )
                .replace(
                    "{RESTLESS_GATE_DISPLAY}",
                    &resources
                        .display
                        .map(|value| format!(":{value}"))
                        .unwrap_or_default(),
                )
                .replace("{RESTLESS_GATE_TMPDIR}", &resources.tempdir);
        }
        let evidence = run_gate_process(
            &cwd,
            &argv,
            resources,
            Duration::from_secs(u64::from(definition.timeout_seconds)),
        )
        .await?;
        require_exact_candidate(workspace, candidate, environment_fingerprint).await?;
        let mut combined = Vec::with_capacity(
            evidence
                .stdout
                .len()
                .saturating_add(evidence.stderr.len())
                .saturating_add(1),
        );
        combined.extend_from_slice(&evidence.stdout);
        combined.push(0);
        combined.extend_from_slice(&evidence.stderr);
        let excerpt = bounded_bytes(&combined);
        let engine_error = excerpt.lines().any(|line| {
            let normalized = line.trim().to_ascii_lowercase();
            normalized.starts_with("error:")
                || normalized.contains("script error")
                || normalized.contains("fatal runtime error")
        });
        let passed = evidence.status == GateCompletionStatus::Conclusive
            && evidence.exit_code == Some(0)
            && evidence.leaked_processes == 0
            && !engine_error;
        anyhow::Ok(GateReceipt {
            candidate: candidate.clone(),
            gate_id: definition.gate_id,
            definition_digest: definition_digest.to_string(),
            resources: resources.clone(),
            output_digest: format!("{:x}", Sha256::digest(&combined)),
            output_excerpt: excerpt,
            exit_code: evidence.exit_code,
            status: evidence.status,
            duration_ms: evidence.duration_ms,
            leaked_processes: evidence.leaked_processes,
            passed,
            observed_at_ms: now_ms()?,
        })
    }
    .await;
    let cleanup = cleanup_gate_runtime(resources);
    match (result, cleanup) {
        (Ok(receipt), Ok(())) => Ok(receipt),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => {
            Err(error.context("gate completed but its exact Runtime state remained"))
        }
        (Err(error), Err(cleanup)) => Err(error.context(format!(
            "gate failed and exact Runtime cleanup also failed: {cleanup:#}"
        ))),
    }
}

async fn prepare_gate_runtime(
    operation_id: Uuid,
    workspace: &WorkspaceIdentity,
    resources: &GateResources,
) -> Result<()> {
    let root = format!(
        "/company/run/gates/{}/{}",
        workspace.attempt_id.simple(),
        operation_id.simple()
    );
    if resources.holder_token != operation_id.simple().to_string()
        || resources.tempdir != format!("{root}/tmp")
        || resources.process_group_marker != format!("{root}/process-group.pid")
    {
        bail!("gate resource coordinates do not belong to this exact operation");
    }
    let output = run_company_command(
        "mkdir",
        &["-p".into(), "--".into(), resources.tempdir.clone()],
    )
    .await?;
    require_success("prepare exact governed gate directory", &output)?;
    for path in [
        "/company",
        "/company/run",
        "/company/run/gates",
        &format!("/company/run/gates/{}", workspace.attempt_id.simple()),
        &root,
        &resources.tempdir,
    ] {
        ensure_plain_directory(Path::new(path))?;
    }
    Ok(())
}

fn gate_cwd(definition: &GateDefinition, workdir: &Path) -> Result<PathBuf> {
    let declared = if definition.cwd == "@attempt" {
        workdir.to_path_buf()
    } else {
        PathBuf::from(&definition.cwd)
    };
    let canonical = std::fs::canonicalize(&declared)
        .with_context(|| format!("canonicalize governed gate cwd {}", declared.display()))?;
    if canonical != Path::new("/company") && !canonical.starts_with("/company/") {
        bail!("governed gate cwd escapes the company Runtime");
    }
    Ok(canonical)
}

async fn read_bounded<R>(reader: R, maximum: u64) -> Result<Vec<u8>>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut bytes = Vec::new();
    reader.take(maximum + 1).read_to_end(&mut bytes).await?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum {
        bail!("governed gate exceeded its output bound");
    }
    Ok(bytes)
}

async fn run_gate_process(
    cwd: &Path,
    argv: &[String],
    resources: &GateResources,
    timeout: Duration,
) -> Result<GateProcessEvidence> {
    let (program, args) = argv
        .split_first()
        .context("governed gate definition has no program")?;
    let mut command = tokio::process::Command::new(program);
    command
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("HOME", "/company/home")
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .env("LC_ALL", "C")
        .env("TMPDIR", &resources.tempdir)
        .env("RESTLESS_GATE_TOKEN", &resources.holder_token)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .kill_on_drop(true);
    if let Some(port) = resources.port {
        command.env("RESTLESS_GATE_PORT", port.to_string());
    }
    if let Some(display) = resources.display {
        command.env("DISPLAY", format!(":{display}"));
    }
    unsafe {
        command.pre_exec(drop_agent_privileges);
    }
    let started = Instant::now();
    let mut child = command.spawn().context("spawn frozen governed gate argv")?;
    let process_group = child
        .id()
        .context("governed gate child has no process id")?;
    let mut guard = ProcessGroupGuard {
        process_group,
        armed: true,
    };
    write_private(
        Path::new(&resources.process_group_marker),
        process_group.to_string().as_bytes(),
    )?;
    let stdout = child.stdout.take().context("open governed gate stdout")?;
    let stderr = child.stderr.take().context("open governed gate stderr")?;
    let stdout_task = tokio::spawn(read_bounded(stdout, MAX_GATE_STDOUT_BYTES));
    let stderr_task = tokio::spawn(read_bounded(stderr, MAX_GATE_STDERR_BYTES));
    let timed = tokio::time::timeout(timeout, child.wait()).await;
    let (status, exit_code) = match timed {
        Ok(Ok(status)) => (GateCompletionStatus::Conclusive, status.code()),
        Ok(Err(error)) => {
            kill_process_group(process_group, nix::sys::signal::Signal::SIGKILL);
            let _ = child.wait().await;
            let _ = stdout_task.await;
            let _ = stderr_task.await;
            guard.armed = false;
            return Err(error).context("wait for governed gate process");
        }
        Err(_) => {
            kill_process_group(process_group, nix::sys::signal::Signal::SIGTERM);
            tokio::time::sleep(Duration::from_millis(250)).await;
            kill_process_group(process_group, nix::sys::signal::Signal::SIGKILL);
            let _ = child.wait().await;
            (GateCompletionStatus::Timeout, None)
        }
    };
    // A successful parent may have left grandchildren. The process-group
    // identity, not the parent status, owns the complete gate lifetime.
    kill_process_group(process_group, nix::sys::signal::Signal::SIGKILL);
    tokio::time::sleep(Duration::from_millis(50)).await;
    let leaked_processes = u32::from(process_group_exists(process_group));
    guard.armed = false;
    let stdout = stdout_task
        .await
        .context("join governed gate stdout reader")??;
    let stderr = stderr_task
        .await
        .context("join governed gate stderr reader")??;
    Ok(GateProcessEvidence {
        status,
        exit_code,
        stdout,
        stderr,
        duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        leaked_processes,
    })
}

fn process_group_exists(process_group: u32) -> bool {
    i32::try_from(process_group).is_ok_and(|process_group| {
        let result = unsafe { nix::libc::kill(-process_group, 0) };
        result == 0 || std::io::Error::last_os_error().raw_os_error() != Some(nix::libc::ESRCH)
    })
}

fn cleanup_gate_runtime(resources: &GateResources) -> Result<()> {
    let tempdir = Path::new(&resources.tempdir);
    let root = tempdir
        .parent()
        .context("gate tempdir has no operation root")?;
    if !root.starts_with("/company/run/gates/") {
        bail!("refusing cleanup outside the exact governed gate root");
    }
    match std::fs::symlink_metadata(root) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            bail!("governed gate root was replaced before cleanup")
        }
        Ok(_) => std::fs::remove_dir_all(root).context("remove exact governed gate root")?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("inspect exact governed gate root"),
    }
    if std::fs::symlink_metadata(root).is_ok() {
        bail!("exact governed gate root remains after cleanup");
    }
    Ok(())
}

async fn git_tree_manifest(
    workspace: &WorkspaceIdentity,
    candidate: &CandidateIdentity,
    relative_path: Option<&str>,
) -> Result<(Vec<u8>, u32, u64)> {
    let (repo_path, _) = workspace_paths(workspace)?;
    let mut args = vec![
        "-C".into(),
        path_text(&repo_path)?,
        "ls-tree".into(),
        "-r".into(),
        "-l".into(),
        "-z".into(),
        "--full-tree".into(),
        candidate.source_commit.clone(),
    ];
    if let Some(relative_path) = relative_path {
        args.push("--".into());
        args.push(relative_path.to_string());
    }
    let output = run_company_command("git", &args).await?;
    require_success("enumerate exact candidate artifact", &output)?;
    if output.stdout.is_empty() {
        bail!("declared candidate artifact path does not exist in the exact commit");
    }
    let (file_count, total_bytes) = summarize_git_tree_manifest(&output.stdout)?;
    Ok((output.stdout, file_count, total_bytes))
}

fn summarize_git_tree_manifest(manifest: &[u8]) -> Result<(u32, u64)> {
    let mut file_count = 0_usize;
    let mut total_bytes = 0_u64;
    for record in manifest
        .split(|byte| *byte == 0)
        .filter(|row| !row.is_empty())
    {
        let separator = record
            .iter()
            .position(|byte| *byte == b'\t')
            .context("Git tree record omitted its path")?;
        let (metadata, path_with_tab) = record.split_at(separator);
        let path = &path_with_tab[1..];
        if path.is_empty() {
            bail!("Git tree record has an empty path");
        }
        let fields = metadata.split(|byte| *byte == b' ').collect::<Vec<_>>();
        if fields.len() != 4 || fields[0] == b"120000" {
            if fields.first().is_some_and(|mode| *mode == b"120000") {
                bail!("candidate artifact contains a symbolic link");
            }
            bail!("Git returned a malformed candidate tree record");
        }
        let size = std::str::from_utf8(fields[3])
            .context("Git tree size is not UTF-8")?
            .parse::<u64>()
            .context("Git tree size is not numeric")?;
        file_count = file_count.saturating_add(1);
        total_bytes = total_bytes.saturating_add(size);
        if file_count > MAX_ARTIFACT_FILES || total_bytes > MAX_ARTIFACT_BYTES {
            bail!("candidate artifact exceeds its file-count or byte bound");
        }
    }
    Ok((
        u32::try_from(file_count).context("candidate artifact file count exceeds protocol")?,
        total_bytes,
    ))
}

async fn observe_artifact_operation(
    workspace: &WorkspaceIdentity,
    candidate: &CandidateIdentity,
    selector: ArtifactSelector,
    environment_fingerprint: &str,
) -> Result<ArtifactReceipt> {
    require_exact_candidate(workspace, candidate, environment_fingerprint).await?;
    let relative_path = match &selector {
        ArtifactSelector::RepositoryTree => None,
        ArtifactSelector::WorkspacePath { relative_path } => Some(relative_path.as_str()),
    };
    let (manifest, file_count, total_bytes) =
        git_tree_manifest(workspace, candidate, relative_path).await?;
    require_exact_candidate(workspace, candidate, environment_fingerprint).await?;
    let uri = match &selector {
        ArtifactSelector::RepositoryTree => format!(
            "git:/company/repos/{}#{}",
            workspace.repo, candidate.source_commit
        ),
        ArtifactSelector::WorkspacePath { relative_path } => format!(
            "/company/worktrees/{}/{}",
            workspace.worktree, relative_path
        ),
    };
    let mut digest_source = serde_json::to_vec(&(workspace, candidate, &selector))?;
    digest_source.extend_from_slice(&manifest);
    Ok(ArtifactReceipt {
        candidate: candidate.clone(),
        selector,
        uri,
        content_digest: format!("{:x}", Sha256::digest(digest_source)),
        file_count,
        total_bytes,
        observed_at_ms: now_ms()?,
    })
}

#[derive(Debug)]
struct GitSnapshot {
    commit: String,
    tree: String,
    status_digest: String,
    dirty_entries: u32,
}

async fn observe_git_snapshot(path: &Path) -> Result<GitSnapshot> {
    ensure_plain_directory(path)?;
    let commit = resolve_commit(path, "HEAD").await?;
    let tree = resolve_tree(path, &commit).await?;
    let status = run_company_command(
        "git",
        &[
            "-C".into(),
            path_text(path)?,
            "status".into(),
            "--porcelain=v1".into(),
            "-z".into(),
            "--untracked-files=all".into(),
        ],
    )
    .await?;
    require_success("observe exact Git checkout status", &status)?;
    Ok(GitSnapshot {
        commit,
        tree,
        status_digest: format!("{:x}", Sha256::digest(&status.stdout)),
        dirty_entries: u32::try_from(dirty_entry_count(&status.stdout)?)
            .context("Git status entry count exceeds protocol")?,
    })
}

async fn promote_candidate_operation(
    workspace: &WorkspaceIdentity,
    candidate: &CandidateIdentity,
    integration_branch: &str,
    environment_fingerprint: &str,
) -> Result<PromotionReceipt> {
    require_exact_candidate(workspace, candidate, environment_fingerprint).await?;
    let (repo_path, _) = workspace_paths(workspace)?;
    ensure_contained_directory(Path::new("/company/repos"), &repo_path)?;
    let before = observe_git_snapshot(&repo_path).await?;
    if before.dirty_entries != 0 {
        bail!("integration checkout is dirty; refusing to overwrite shared state");
    }
    let branch = run_company_command(
        "git",
        &[
            "-C".into(),
            path_text(&repo_path)?,
            "symbolic-ref".into(),
            "--quiet".into(),
            "--short".into(),
            "HEAD".into(),
        ],
    )
    .await?;
    require_success("read exact integration branch", &branch)?;
    if std::str::from_utf8(&branch.stdout)?.trim() != integration_branch {
        bail!("integration checkout is not on the frozen branch");
    }
    let candidate_tree = resolve_tree(&repo_path, &candidate.source_commit).await?;
    if candidate_tree != candidate.source_tree {
        bail!("integration repository resolves the candidate commit to another tree");
    }
    let reused = before.commit == candidate.source_commit;
    if reused {
        if before.tree != candidate.source_tree {
            bail!("integrated candidate commit no longer resolves to its frozen tree");
        }
    } else {
        let ancestor = run_company_command(
            "git",
            &[
                "-C".into(),
                path_text(&repo_path)?,
                "merge-base".into(),
                "--is-ancestor".into(),
                before.commit.clone(),
                candidate.source_commit.clone(),
            ],
        )
        .await?;
        require_success("require fast-forward integration history", &ancestor)?;
        let promotion = run_company_command(
            "git",
            &[
                "-C".into(),
                path_text(&repo_path)?,
                "merge".into(),
                "--ff-only".into(),
                "--no-edit".into(),
                candidate.source_commit.clone(),
            ],
        )
        .await?;
        require_success("fast-forward exact candidate", &promotion)?;
    }
    let after = observe_git_snapshot(&repo_path).await?;
    if after.commit != candidate.source_commit
        || after.tree != candidate.source_tree
        || after.dirty_entries != 0
    {
        bail!("integration promotion did not leave the exact clean candidate");
    }
    require_exact_candidate(workspace, candidate, environment_fingerprint).await?;
    Ok(PromotionReceipt {
        candidate: candidate.clone(),
        integration_branch: integration_branch.to_string(),
        previous_commit: before.commit,
        previous_tree: before.tree,
        promoted_commit: after.commit,
        promoted_tree: after.tree,
        status_digest: after.status_digest,
        reused,
        observed_at_ms: now_ms()?,
    })
}

fn review_paths(
    workspace: &WorkspaceIdentity,
    candidate: &CandidateIdentity,
) -> (PathBuf, PathBuf) {
    (
        PathBuf::from(format!("/company/reviews/git/{}", candidate.source_commit)),
        PathBuf::from(format!(
            "/company/reviews/by-attempt/{}",
            workspace.attempt_id.simple()
        )),
    )
}

fn review_tree_is_hardened(root: &Path) -> Result<()> {
    for path in walkdir(root)? {
        let metadata = std::fs::symlink_metadata(&path)
            .with_context(|| format!("inspect review path {}", path.display()))?;
        if metadata.file_type().is_symlink() {
            bail!("review copy contains a symbolic link");
        }
        if metadata.uid() != 0 || metadata.gid() != 2000 {
            bail!("review copy is not owned by the immutable Runtime boundary");
        }
        let mode = metadata.permissions().mode() & 0o777;
        if metadata.is_dir() {
            if mode != 0o550 {
                bail!("review directory is not read-only to the reviewer");
            }
        } else if !metadata.is_file() || mode != 0o440 {
            bail!("review file is not an immutable regular file");
        }
    }
    Ok(())
}

fn harden_review_tree(root: &Path) -> Result<()> {
    let mut paths = walkdir(root)?;
    // Reject links before changing any ownership. A committed link could
    // otherwise turn a supposedly bounded review snapshot into host-volume
    // traversal for the reviewer process.
    for path in &paths {
        let metadata = std::fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() || (!metadata.is_dir() && !metadata.is_file()) {
            bail!("review copy contains a non-regular path");
        }
    }
    paths.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for path in &paths {
        let metadata = std::fs::symlink_metadata(path)?;
        std::os::unix::fs::chown(path, Some(0), Some(2000))?;
        let mode = if metadata.is_dir() { 0o550 } else { 0o440 };
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    }
    review_tree_is_hardened(root)
}

async fn prepare_review_copy_operation(
    workspace: &WorkspaceIdentity,
    candidate: &CandidateIdentity,
    environment_fingerprint: &str,
) -> Result<ReviewCopyReceipt> {
    require_exact_candidate(workspace, candidate, environment_fingerprint).await?;
    let (manifest, file_count, _) = git_tree_manifest(workspace, candidate, None).await?;
    let (repo_path, _) = workspace_paths(workspace)?;
    let (review_workdir, alias) = review_paths(workspace, candidate);
    let reviews_root = Path::new("/company/reviews");
    let git_root = reviews_root.join("git");
    let alias_root = reviews_root.join("by-attempt");
    match std::fs::symlink_metadata(reviews_root) {
        Ok(_) => ensure_plain_directory(reviews_root)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir(reviews_root)?;
            std::os::unix::fs::chown(reviews_root, Some(0), Some(2000))?;
        }
        Err(error) => return Err(error).context("inspect review custody root"),
    }
    std::fs::set_permissions(reviews_root, std::fs::Permissions::from_mode(0o770))?;
    for path in [git_root.as_path(), alias_root.as_path()] {
        match std::fs::symlink_metadata(path) {
            Ok(_) => ensure_plain_directory(path)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir(path)?;
                std::os::unix::fs::chown(path, Some(0), Some(2000))?;
            }
            Err(error) => return Err(error).context("inspect review custody child root"),
        }
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o550))?;
    }
    std::fs::set_permissions(reviews_root, std::fs::Permissions::from_mode(0o550))?;
    let reused = match std::fs::symlink_metadata(&review_workdir) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                bail!("commit-addressed review path was replaced");
            }
            review_tree_is_hardened(&review_workdir)?;
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            // Temporarily open only the exact parent while the uid-2000 Git
            // process creates its detached worktree. The finished copy is
            // root-owned and sealed before any receipt is returned.
            std::fs::set_permissions(&git_root, std::fs::Permissions::from_mode(0o770))?;
            let created = run_company_command(
                "git",
                &[
                    "-C".into(),
                    path_text(&repo_path)?,
                    "worktree".into(),
                    "add".into(),
                    "--detach".into(),
                    path_text(&review_workdir)?,
                    candidate.source_commit.clone(),
                ],
            )
            .await;
            std::fs::set_permissions(&git_root, std::fs::Permissions::from_mode(0o550))?;
            let created = created?;
            require_success("prepare commit-addressed review copy", &created)?;
            harden_review_tree(&review_workdir)?;
            false
        }
        Err(error) => return Err(error).context("inspect commit-addressed review path"),
    };
    let review = observe_git_snapshot(&review_workdir).await?;
    if review.commit != candidate.source_commit
        || review.tree != candidate.source_tree
        || review.dirty_entries != 0
    {
        bail!("review copy differs from the exact clean terminal candidate");
    }
    match std::fs::symlink_metadata(&alias) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            if std::fs::read_link(&alias)? != review_workdir {
                bail!("Attempt review alias already names another candidate");
            }
        }
        Ok(_) => bail!("Attempt review alias is not a symbolic link"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::set_permissions(&alias_root, std::fs::Permissions::from_mode(0o770))?;
            let linked = std::os::unix::fs::symlink(&review_workdir, &alias);
            std::fs::set_permissions(&alias_root, std::fs::Permissions::from_mode(0o550))?;
            linked?;
        }
        Err(error) => return Err(error).context("inspect Attempt review alias"),
    }
    // The exact source must still be identical after worktree preparation,
    // ownership sealing, reviewer-readable Git probes and alias creation.
    require_exact_candidate(workspace, candidate, environment_fingerprint).await?;
    let mut digest_source = serde_json::to_vec(&(workspace, candidate, "review-copy-v1"))?;
    digest_source.extend_from_slice(&manifest);
    Ok(ReviewCopyReceipt {
        candidate: candidate.clone(),
        uri: path_text(&review_workdir)?,
        alias_uri: path_text(&alias)?,
        content_digest: format!("{:x}", Sha256::digest(digest_source)),
        file_count,
        access_probed: true,
        reused,
        observed_at_ms: now_ms()?,
    })
}

async fn cleanup_attempt_operation(workspace: &WorkspaceIdentity) -> Result<AttemptCleanupReceipt> {
    let attempt = workspace.attempt_id.to_string();
    let removed_paths = vec![
        format!("/company/run/attempts/{attempt}"),
        format!("/company/run/gates/{attempt}"),
    ];
    for value in &removed_paths {
        let path = Path::new(value);
        match std::fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                bail!("Attempt Runtime path was replaced before cleanup")
            }
            Ok(_) => std::fs::remove_dir_all(path)
                .with_context(|| format!("remove exact Attempt Runtime path {value}"))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("inspect exact Attempt Runtime path"),
        }
    }
    let residue_count = removed_paths
        .iter()
        .filter(|path| std::fs::symlink_metadata(path).is_ok())
        .count();
    if residue_count != 0 {
        bail!("exact Attempt Runtime residue remains after cleanup");
    }
    Ok(AttemptCleanupReceipt {
        attempt_id: workspace.attempt_id,
        removed_paths,
        residue_count: 0,
        observed_at_ms: now_ms()?,
    })
}

fn workspace_paths(workspace: &WorkspaceIdentity) -> Result<(PathBuf, PathBuf)> {
    if !runtime_slug(&workspace.repo) || !runtime_slug(&workspace.worktree) {
        bail!("workspace contains an invalid repository or worktree slug");
    }
    Ok((
        PathBuf::from(format!("/company/repos/{}", workspace.repo)),
        PathBuf::from(format!("/company/worktrees/{}", workspace.worktree)),
    ))
}

fn runtime_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn ensure_plain_directory(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("inspect Runtime directory {}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        bail!("Runtime path {} is not a plain directory", path.display());
    }
    if std::fs::canonicalize(path)? != path {
        bail!("Runtime directory {} is redirected", path.display());
    }
    Ok(())
}

fn ensure_contained_directory(root: &Path, candidate: &Path) -> Result<()> {
    ensure_plain_directory(root)?;
    ensure_plain_directory(candidate)?;
    let root = std::fs::canonicalize(root)?;
    let candidate = std::fs::canonicalize(candidate)?;
    if candidate.parent() != Some(root.as_path()) {
        bail!("Runtime workspace path escapes its exact parent");
    }
    Ok(())
}

async fn resolve_commit(path: &Path, reference: &str) -> Result<String> {
    let output = run_company_command(
        "git",
        &[
            "-C".into(),
            path_text(path)?,
            "rev-parse".into(),
            "--verify".into(),
            "--end-of-options".into(),
            format!("{reference}^{{commit}}"),
        ],
    )
    .await?;
    require_success("resolve exact source commit", &output)?;
    canonical_oid(&output.stdout).context("source commit is not one canonical Git object id")
}

async fn resolve_tree(path: &Path, commit: &str) -> Result<String> {
    let output = run_company_command(
        "git",
        &[
            "-C".into(),
            path_text(path)?,
            "rev-parse".into(),
            "--verify".into(),
            "--end-of-options".into(),
            format!("{commit}^{{tree}}"),
        ],
    )
    .await?;
    require_success("resolve exact source tree", &output)?;
    canonical_oid(&output.stdout).context("source tree is not one canonical Git object id")
}

fn canonical_oid(bytes: &[u8]) -> Option<String> {
    let value = std::str::from_utf8(bytes).ok()?.trim();
    (matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f')))
    .then(|| value.to_string())
}

fn dirty_entry_count(status: &[u8]) -> Result<usize> {
    let fields = status
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    let mut cursor = 0;
    let mut entries = 0_usize;
    while cursor < fields.len() {
        let field = fields[cursor];
        if field.len() < 3 || field[2] != b' ' {
            bail!("Git returned malformed porcelain status");
        }
        let renamed = matches!(field[0], b'R' | b'C') || matches!(field[1], b'R' | b'C');
        cursor += 1;
        if renamed {
            if cursor >= fields.len() {
                bail!("Git omitted the source path for a renamed status entry");
            }
            cursor += 1;
        }
        entries = entries.saturating_add(1);
    }
    Ok(entries)
}

struct BoundedCommandOutput {
    success: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

struct ProcessGroupGuard {
    process_group: u32,
    armed: bool,
}

impl Drop for ProcessGroupGuard {
    fn drop(&mut self) {
        if self.armed {
            kill_process_group(self.process_group, nix::sys::signal::Signal::SIGKILL);
        }
    }
}

async fn run_company_command(
    program: &'static str,
    args: &[String],
) -> Result<BoundedCommandOutput> {
    if !matches!(program, "git" | "mkdir") {
        bail!("program is outside the typed Runtime workspace allowlist");
    }
    let mut command = tokio::process::Command::new(program);
    if program == "git" {
        // Repository/global configuration is data, not authority to execute
        // preparation hooks or fsmonitor processes in the bridge operation.
        command.args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.pager=cat",
            "-c",
            "safe.directory=*",
        ]);
    }
    command
        .args(args)
        .current_dir("/company")
        .env_clear()
        .env("HOME", "/company/home")
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .env("LC_ALL", "C")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .kill_on_drop(true);
    unsafe {
        command.pre_exec(drop_agent_privileges);
    }
    let mut child = command
        .spawn()
        .with_context(|| format!("spawn typed Runtime {program} operation"))?;
    let process_group = child.id().context("workspace child has no process id")?;
    let mut guard = ProcessGroupGuard {
        process_group,
        armed: true,
    };
    let stdout = child.stdout.take().context("open workspace stdout")?;
    let stderr = child.stderr.take().context("open workspace stderr")?;
    let read_stdout = async move {
        let mut bytes = Vec::new();
        stdout
            .take(MAX_GIT_STDOUT_BYTES + 1)
            .read_to_end(&mut bytes)
            .await?;
        Ok::<_, std::io::Error>(bytes)
    };
    let read_stderr = async move {
        let mut bytes = Vec::new();
        stderr
            .take(MAX_GIT_STDERR_BYTES + 1)
            .read_to_end(&mut bytes)
            .await?;
        Ok::<_, std::io::Error>(bytes)
    };
    let (status, stdout, stderr) = tokio::try_join!(child.wait(), read_stdout, read_stderr)?;
    guard.armed = false;
    if u64::try_from(stdout.len()).unwrap_or(u64::MAX) > MAX_GIT_STDOUT_BYTES
        || u64::try_from(stderr.len()).unwrap_or(u64::MAX) > MAX_GIT_STDERR_BYTES
    {
        bail!("typed Runtime workspace command exceeded its output bound");
    }
    Ok(BoundedCommandOutput {
        success: status.success(),
        stdout,
        stderr,
    })
}

fn require_success(action: &str, output: &BoundedCommandOutput) -> Result<()> {
    if !output.success {
        bail!("{action} failed: {}", bounded_bytes(&output.stderr));
    }
    Ok(())
}

fn bounded_bytes(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(2_000)
        .collect::<String>()
        .trim()
        .to_string()
}

fn bounded_error(error: &anyhow::Error) -> String {
    let message = bounded_bytes(format!("{error:#}").as_bytes());
    if message.is_empty() {
        "typed Runtime workspace operation was refused".into()
    } else {
        message
    }
}

fn environment_fingerprint(
    identity: &RuntimeIdentity,
    release: &ReleaseIdentity,
) -> Result<String> {
    let encoded = serde_json::to_vec(&(identity, release))?;
    Ok(format!("{:x}", Sha256::digest(encoded)))
}

fn path_text(path: &Path) -> Result<String> {
    path.to_str()
        .map(str::to_string)
        .context("Runtime workspace path is not UTF-8")
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_environment()?;
    let workspace_operations = WorkspaceOperations::default();
    let (outbound_tx, mut outbound_rx) = mpsc::channel(OUTBOUND_DEPTH);
    tokio::spawn(serve_coordination(outbound_tx.clone()));

    let mut delay = Duration::from_secs(1);
    loop {
        match run_connection(
            &config,
            &workspace_operations,
            &outbound_tx,
            &mut outbound_rx,
        )
        .await
        {
            Ok(()) => delay = Duration::from_secs(1),
            Err(error) => eprintln!("Runtime bridge disconnected: {error:#}"),
        }
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(Duration::from_secs(30));
    }
}

impl Config {
    fn from_environment() -> Result<Self> {
        let url = required("RESTLESS_RUNTIME_BRIDGE_URL")?;
        let parsed = reqwest::Url::parse(&url).context("parse RESTLESS_RUNTIME_BRIDGE_URL")?;
        if parsed.scheme() != "wss"
            || parsed.path() != "/internal/v1/runtime-bridge"
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
            bail!("RESTLESS_RUNTIME_BRIDGE_URL must be the exact credential-free wss endpoint");
        }
        let identity = RuntimeIdentity {
            owner_id: required_uuid("RESTLESS_RUNTIME_OWNER_ID")?,
            plane_id: required_uuid("RESTLESS_RUNTIME_PLANE_ID")?,
            company_id: required_uuid("RESTLESS_RUNTIME_COMPANY_ID")?,
            cell_id: required_uuid("RESTLESS_RUNTIME_CELL_ID")?,
            company: required("RESTLESS_COMPANY")?,
            runtime_id: required("RESTLESS_RUNTIME_ID")?,
            runtime_generation: required_i64("RESTLESS_RUNTIME_GENERATION")?,
            runtime_image: required("RESTLESS_RUNTIME_IMAGE")?,
            volume_name: required("RESTLESS_RUNTIME_VOLUME_NAME")?,
            source_revision: required("RESTLESS_SOURCE_REVISION")?,
        };
        let capability_file = PathBuf::from(required("RESTLESS_RUNTIME_BRIDGE_CAPABILITY_FILE")?);
        let ca_path = std::env::var_os("RESTLESS_RUNTIME_BRIDGE_CA_FILE").map(PathBuf::from);
        let agent_ca_pem = ca_path.as_deref().map(read_ca_pem).transpose()?;
        let tls_connector = tls_connector(agent_ca_pem.as_deref())?;
        // Fail before supervision reports this process started, but never
        // cache the value: Fleet rotates the root-owned projection in place
        // and every reconnect must pick up the newest bounded grant.
        read_secret(&capability_file)?;
        // Reuse protocol validation before any network connection.
        Frame {
            protocol_version: PROTOCOL_VERSION,
            sequence: 1,
            identity: identity.clone(),
            message: BridgeMessage::Heartbeat {
                connection_nonce: Uuid::new_v4(),
                sent_at_ms: now_ms()?,
            },
        }
        .validate()
        .context("validate Runtime bridge identity")?;
        Ok(Self {
            url,
            identity,
            capability_file,
            tls_connector,
            agent_ca_pem,
        })
    }
}

async fn run_connection(
    config: &Config,
    workspace_operations: &WorkspaceOperations,
    outbound_tx: &mpsc::Sender<Outbound>,
    outbound_rx: &mut mpsc::Receiver<Outbound>,
) -> Result<()> {
    let health = health().await?;
    verify_release(config, &health.release)?;
    let capability = read_secret(&config.capability_file)?;
    // Build from the URL so tungstenite supplies the complete Upgrade
    // handshake (method, Host, Connection, version and random key), then add
    // the single bridge bearer. A raw http::Request is not enriched by the
    // tungstenite client and would fail every real proxy handshake.
    let mut request = config
        .url
        .as_str()
        .into_client_request()
        .context("build Runtime bridge websocket request")?;
    request.headers_mut().insert(
        AUTHORIZATION,
        format!("Bearer {capability}")
            .parse()
            .context("encode Runtime bridge bearer")?,
    );
    let (socket, _) = tokio_tungstenite::connect_async_tls_with_config(
        request,
        None,
        true,
        Some(config.tls_connector.clone()),
    )
    .await
    .context("connect Runtime bridge websocket")?;
    let (mut socket_tx, mut socket_rx) = socket.split();
    let nonce = Uuid::new_v4();
    let _workspace_guard = WorkspaceConnectionGuard {
        operations: workspace_operations.clone(),
        connection_nonce: nonce,
    };
    let environment_fingerprint = environment_fingerprint(&config.identity, &health.release)?;
    let register = Frame {
        protocol_version: PROTOCOL_VERSION,
        sequence: 1,
        identity: config.identity.clone(),
        message: BridgeMessage::Register {
            connection_nonce: nonce,
            release: health.release,
        },
    };
    send_frame(&mut socket_tx, register).await?;
    let ack = tokio::time::timeout(Duration::from_secs(10), socket_rx.next())
        .await
        .context("Runtime bridge registration timed out")?
        .context("Runtime bridge closed before registration")??;
    let WebSocketMessage::Binary(bytes) = ack else {
        bail!("Runtime bridge registration returned a non-binary frame");
    };
    let ack = Frame::decode(&bytes).context("decode Runtime bridge registration")?;
    if ack.identity != config.identity {
        bail!("Runtime bridge registration acknowledged another identity");
    }
    let BridgeMessage::RegisterAck {
        connection_nonce,
        heartbeat_interval_ms,
        ..
    } = ack.message
    else {
        bail!("Runtime bridge did not acknowledge registration");
    };
    if connection_nonce != nonce {
        bail!("Runtime bridge registration nonce mismatch");
    }
    let mut incoming_sequence = SequenceTracker::default();
    incoming_sequence.observe(ack.sequence)?;
    let mut outgoing_sequence = 2_u64;
    let mut heartbeat = tokio::time::interval(Duration::from_millis(heartbeat_interval_ms));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut coordination = HashMap::<Uuid, oneshot::Sender<BridgeMessage>>::new();
    let mut agents = HashMap::<Uuid, AgentControl>::new();

    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                let frame = Frame {
                    protocol_version: PROTOCOL_VERSION,
                    sequence: outgoing_sequence,
                    identity: config.identity.clone(),
                    message: BridgeMessage::Heartbeat { connection_nonce: nonce, sent_at_ms: now_ms()? },
                };
                outgoing_sequence = outgoing_sequence.saturating_add(1);
                send_frame(&mut socket_tx, frame).await?;
            }
            outbound = outbound_rx.recv() => {
                let Some(outbound) = outbound else { bail!("Runtime bridge outbound queue closed"); };
                if let Some(response) = outbound.response {
                    let operation_id = message_operation_id(&outbound.message)
                        .context("local Runtime request has no operation id")?;
                    if coordination.insert(operation_id, response).is_some() {
                        bail!("duplicate local Runtime operation {operation_id}");
                    }
                }
                if let BridgeMessage::Exit { operation_id, .. } = &outbound.message {
                    agents.remove(operation_id);
                }
                let frame = Frame {
                    protocol_version: PROTOCOL_VERSION,
                    sequence: outgoing_sequence,
                    identity: config.identity.clone(),
                    message: outbound.message,
                };
                outgoing_sequence = outgoing_sequence.saturating_add(1);
                send_frame(&mut socket_tx, frame).await?;
            }
            incoming = socket_rx.next() => {
                let Some(incoming) = incoming else { bail!("Runtime bridge websocket closed"); };
                match incoming? {
                    WebSocketMessage::Binary(bytes) => {
                        let frame = Frame::decode(&bytes)?;
                        if frame.identity != config.identity {
                            bail!("Runtime bridge frame changed identity");
                        }
                        incoming_sequence.observe(frame.sequence)?;
                        handle_incoming(
                            config,
                            frame.message,
                            outbound_tx,
                            &mut coordination,
                            &mut agents,
                            workspace_operations,
                            nonce,
                            &environment_fingerprint,
                        ).await?;
                    }
                    WebSocketMessage::Ping(payload) => {
                        tokio::time::timeout(
                            SEND_DEADLINE,
                            socket_tx.send(WebSocketMessage::Pong(payload)),
                        )
                        .await
                        .context("Runtime bridge pong timed out")??;
                    }
                    WebSocketMessage::Pong(_) => {}
                    WebSocketMessage::Close(_) => bail!("Runtime bridge websocket closed"),
                    WebSocketMessage::Text(_) | WebSocketMessage::Frame(_) => bail!("Runtime bridge accepts binary protocol frames only"),
                }
            }
        }
    }
}

async fn handle_incoming(
    config: &Config,
    message: BridgeMessage,
    outbound: &mpsc::Sender<Outbound>,
    coordination: &mut HashMap<Uuid, oneshot::Sender<BridgeMessage>>,
    agents: &mut HashMap<Uuid, AgentControl>,
    workspace_operations: &WorkspaceOperations,
    connection_nonce: Uuid,
    environment_fingerprint: &str,
) -> Result<()> {
    match message {
        BridgeMessage::PreflightRequest { operation_id, .. } => {
            let observed = health().await;
            let (status, release, disk_available_bytes, error) = match observed {
                Ok(health) if verify_release(config, &health.release).is_ok() => (
                    PreflightStatus::Ready,
                    health.release,
                    disk_available_bytes().await.unwrap_or_default(),
                    None,
                ),
                Ok(health) => (
                    PreflightStatus::Refused,
                    health.release,
                    0,
                    Some("Runtime release does not match its immutable deployment".into()),
                ),
                Err(error) => (
                    PreflightStatus::Unavailable,
                    fallback_release(config),
                    0,
                    Some(format!("Runtime health unavailable: {error}")),
                ),
            };
            queue(
                outbound,
                BridgeMessage::PreflightResult {
                    operation_id,
                    status,
                    release,
                    disk_available_bytes,
                    observed_at_ms: now_ms()?,
                    error,
                },
            )
            .await?;
        }
        launch @ BridgeMessage::LaunchAgent { operation_id, .. } => {
            if agents.contains_key(&operation_id) {
                queue(
                    outbound,
                    BridgeMessage::Error {
                        operation_id: Some(operation_id),
                        code: "duplicate_operation".into(),
                        message: "this Runtime agent operation is already active".into(),
                    },
                )
                .await?;
            } else {
                let control = launch_agent(config, launch, outbound.clone()).await?;
                agents.insert(operation_id, control);
            }
        }
        request @ (BridgeMessage::PrepareWorkspace { .. }
        | BridgeMessage::ObserveWorkspace { .. }
        | BridgeMessage::RunGate { .. }
        | BridgeMessage::ObserveArtifact { .. }
        | BridgeMessage::PromoteCandidate { .. }
        | BridgeMessage::PrepareReviewCopy { .. }
        | BridgeMessage::CleanupAttempt { .. }) => {
            workspace_operations.start(
                config.clone(),
                connection_nonce,
                environment_fingerprint.to_string(),
                request,
                outbound.clone(),
            )?;
        }
        BridgeMessage::AcpStdin {
            operation_id,
            data_base64,
        } => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(data_base64)
                .context("decode ACP stdin")?;
            agents
                .get(&operation_id)
                .context("ACP stdin names no active operation")?
                .stdin
                .send(bytes)
                .await
                .context("send ACP stdin")?;
        }
        BridgeMessage::Cancel {
            operation_id,
            reason,
        } => {
            if let Some(control) = agents.remove(&operation_id) {
                let _ = control.cancel.send(reason);
            } else {
                workspace_operations.cancel(operation_id, reason);
            }
        }
        response @ (BridgeMessage::CoordinationResponse { operation_id, .. }
        | BridgeMessage::Error {
            operation_id: Some(operation_id),
            ..
        }) => {
            if let Some(waiter) = coordination.remove(&operation_id) {
                let _ = waiter.send(response);
            }
        }
        BridgeMessage::Error {
            operation_id: None,
            message,
            ..
        } => bail!("Core refused Runtime bridge: {message}"),
        _ => bail!("Core sent a message invalid for the Runtime side"),
    }
    Ok(())
}

async fn launch_agent(
    config: &Config,
    message: BridgeMessage,
    outbound: mpsc::Sender<Outbound>,
) -> Result<AgentControl> {
    let BridgeMessage::LaunchAgent {
        operation_id,
        session_id: _,
        actor,
        harness,
        model,
        reasoning_effort,
        workdir,
        prompt,
        coordination_capability,
        model_capability,
        model_url,
        deadline_ms,
        ..
    } = message
    else {
        bail!("launch_agent requires LaunchAgent");
    };
    let canonical_workdir =
        std::fs::canonicalize(&workdir).context("canonicalize agent workdir")?;
    if canonical_workdir != Path::new("/company") && !canonical_workdir.starts_with("/company/") {
        bail!("agent workdir must stay inside /company");
    }
    validate_model_url(&config.url, &model_url)?;
    let session_root = create_session_root(operation_id)?;
    let system_prompt = session_root.join("system.md");
    write_private(&system_prompt, prompt.as_bytes())?;
    let model_ca_file = config
        .agent_ca_pem
        .as_deref()
        .map(|pem| {
            let path = session_root.join("model-gateway-ca.pem");
            write_private(&path, pem)?;
            anyhow::Ok(path)
        })
        .transpose()?;
    let (program, args, profile_dir) = match harness.as_str() {
        "restless-managed" => {
            let profile = session_root.join("omp-profile");
            std::fs::create_dir_all(&profile)?;
            let provider = model.split_once('/').map(|pair| pair.0).context("model must include provider")?;
            let models = format!("providers:\n  {provider}:\n    baseUrl: {model_url}\n    apiKey: RESTLESS_MODEL_CAPABILITY\n    transport: pi-native\n    api: openai-responses\n");
            write_private(&profile.join("models.yml"), models.as_bytes())?;
            let runtime_config = profile.join("restless-runtime.yml");
            write_private(&runtime_config, OMP_CONFIG.as_bytes())?;
            (
                "omp",
                vec![
                    "--model".to_string(),
                    model,
                    "--thinking".to_string(),
                    reasoning_effort,
                    "--system-prompt".to_string(),
                    system_prompt
                        .to_str()
                        .context("system prompt path is not UTF-8")?
                        .to_string(),
                    "--config".to_string(),
                    runtime_config
                        .to_str()
                        .context("OMP profile path is not UTF-8")?
                        .to_string(),
                    "--no-extensions".to_string(),
                    "--no-rules".to_string(),
                    "--tools".to_string(),
                    "read,bash,edit,write,grep".to_string(),
                    "acp".to_string(),
                ],
                profile,
            )
        }
        _ => bail!("hosted Runtime bridge currently accepts only the certified restless-managed ACP harness"),
    };
    make_company_owned(&session_root)?;
    let mut command = tokio::process::Command::new(program);
    command
        .args(args)
        .current_dir(&canonical_workdir)
        .process_group(0)
        .env("HOME", "/company/home")
        .env("PI_CODING_AGENT_DIR", &profile_dir)
        .env("RESTLESS_ACTOR", actor)
        .env("RESTLESS_COORDINATOR", COORDINATION_ADDRESS)
        .env("RESTLESS_SESSION_CAPABILITY", coordination_capability)
        .env("RESTLESS_MODEL_CAPABILITY", model_capability)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    if let Some(ca_file) = model_ca_file {
        // omp is a Bun executable. NODE_EXTRA_CA_CERTS augments, rather than
        // replaces, the image's public WebPKI roots and makes the exact
        // private plane hostname trusted by the hosted model relay.
        command.env("NODE_EXTRA_CA_CERTS", ca_file);
    }
    // Hosted Runtime runs the bridge with a dedicated credential boundary.
    // Clear supplementary groups before dropping the child to the ordinary
    // company identity; `.uid()`/`.gid()` alone would retain the bridge's
    // groups and let an ACP process read bridge-only mounts.
    unsafe {
        command.pre_exec(drop_agent_privileges);
    }
    let mut child = command
        .spawn()
        .context("spawn certified Runtime ACP harness")?;
    let process_group = child.id().context("Runtime ACP child has no process id")?;
    let mut child_stdin = child.stdin.take().context("open ACP stdin")?;
    let mut child_stdout = child.stdout.take().context("open ACP stdout")?;
    let (stdin_tx, mut stdin_rx) = mpsc::channel::<Vec<u8>>(32);
    let (cancel_tx, mut cancel_rx) = oneshot::channel::<String>();
    tokio::spawn(async move {
        let outcome: Result<Option<i32>> = async {
            let deadline = tokio::time::Instant::now()
                + Duration::from_millis(deadline_ms.saturating_sub(now_ms()?).try_into().unwrap_or(0));
            let mut buffer = vec![0_u8; ACP_CHUNK_BYTES];
            loop {
                tokio::select! {
                    read = child_stdout.read(&mut buffer) => {
                        let read = read?;
                        if read == 0 { return Ok(child.wait().await?.code()); }
                        queue(&outbound, BridgeMessage::AcpStdout {
                            operation_id,
                            data_base64: base64::engine::general_purpose::STANDARD.encode(&buffer[..read]),
                        }).await?;
                    }
                    input = stdin_rx.recv() => {
                        let Some(input) = input else {
                            terminate_process_group(process_group).await;
                            return Ok(child.wait().await?.code());
                        };
                        child_stdin.write_all(&input).await?;
                        child_stdin.flush().await?;
                    }
                    _ = &mut cancel_rx => {
                        terminate_process_group(process_group).await;
                        return Ok(child.wait().await?.code());
                    }
                    _ = tokio::time::sleep_until(deadline) => {
                        terminate_process_group(process_group).await;
                        return Ok(child.wait().await?.code());
                    }
                }
            }
        }.await;
        // The ACP parent may exit before tool grandchildren. The Linux
        // process group is the ownership record; reap it on every terminal
        // path before reporting Exit to Core.
        kill_process_group(process_group, nix::sys::signal::Signal::SIGKILL);
        let (code, error) = match outcome {
            Ok(code) => (code, None),
            Err(error) => (
                None,
                Some(format!("Runtime ACP supervision failed: {error}")),
            ),
        };
        let _ = queue(
            &outbound,
            BridgeMessage::Exit {
                operation_id,
                code,
                signal: None,
                error,
            },
        )
        .await;
        let _ = std::fs::remove_dir_all(session_root);
    });
    Ok(AgentControl {
        stdin: stdin_tx,
        cancel: cancel_tx,
    })
}

fn validate_model_url(bridge_url: &str, model_url: &str) -> Result<()> {
    let bridge = reqwest::Url::parse(bridge_url).context("parse configured Runtime bridge URL")?;
    let model = reqwest::Url::parse(model_url).context("parse hosted model relay URL")?;
    if model.scheme() != "https"
        || model.path() != "/internal/v1/model-gateway"
        || model.query().is_some()
        || model.fragment().is_some()
        || !model.username().is_empty()
        || model.password().is_some()
        || model.host_str() != bridge.host_str()
        || model.port_or_known_default() != bridge.port_or_known_default()
    {
        bail!("hosted model relay must be the exact HTTPS account-plane endpoint");
    }
    Ok(())
}

fn kill_process_group(process_group: u32, signal: nix::sys::signal::Signal) {
    if let Ok(pid) = i32::try_from(process_group) {
        let _ = nix::sys::signal::killpg(nix::unistd::Pid::from_raw(pid), signal);
    }
}

async fn terminate_process_group(process_group: u32) {
    kill_process_group(process_group, nix::sys::signal::Signal::SIGTERM);
    tokio::time::sleep(Duration::from_secs(1)).await;
    kill_process_group(process_group, nix::sys::signal::Signal::SIGKILL);
}

async fn serve_coordination(outbound: mpsc::Sender<Outbound>) {
    let listener = match tokio::net::TcpListener::bind(COORDINATION_ADDRESS).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("Runtime coordination proxy could not bind: {error}");
            return;
        }
    };
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        let outbound = outbound.clone();
        tokio::spawn(async move {
            if let Err(error) = proxy_coordination(stream, outbound).await {
                eprintln!("Runtime coordination proxy failed: {error:#}");
            }
        });
    }
}

async fn proxy_coordination(
    stream: tokio::net::TcpStream,
    outbound: mpsc::Sender<Outbound>,
) -> Result<()> {
    let (reader, mut writer) = tokio::io::split(stream);
    let mut bytes = Vec::new();
    BufReader::new(reader).read_until(b'\n', &mut bytes).await?;
    if bytes.is_empty() || bytes.len() > MAX_COORDINATION_BYTES || bytes.last() != Some(&b'\n') {
        bail!("coordination request is outside its bound");
    }
    bytes.pop();
    let body: serde_json::Value =
        serde_json::from_slice(&bytes).context("decode coordination request")?;
    if body.get("cmd").and_then(serde_json::Value::as_str) == Some("watch") {
        bail!("streaming watch is not available through the one-request coordination proxy");
    }
    let operation_id = Uuid::new_v4();
    let (response_tx, response_rx) = oneshot::channel();
    outbound
        .send(Outbound {
            message: BridgeMessage::CoordinationRequest {
                operation_id,
                method: CoordinationMethod::Post,
                path: "/coordination".into(),
                body: Some(body),
                deadline_ms: now_ms()? + 30_000,
            },
            response: Some(response_tx),
        })
        .await
        .context("queue coordination request")?;
    let response = tokio::time::timeout(Duration::from_secs(30), response_rx)
        .await
        .context("coordination response timed out")??;
    let body = match response {
        BridgeMessage::CoordinationResponse {
            status_code: 200,
            body: Some(body),
            ..
        } => body,
        BridgeMessage::CoordinationResponse {
            status_code, body, ..
        } => {
            serde_json::json!({"ok": false, "error": format!("coordination proxy returned HTTP {status_code}"), "detail": body})
        }
        BridgeMessage::Error { message, .. } => serde_json::json!({"ok": false, "error": message}),
        _ => bail!("coordination proxy returned the wrong response type"),
    };
    let mut encoded = serde_json::to_vec(&body)?;
    encoded.push(b'\n');
    writer.write_all(&encoded).await?;
    Ok(())
}

async fn health() -> Result<HealthEnvelope> {
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?
        .get(HEALTH_URL)
        .send()
        .await
        .context("read Runtime release health")?;
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|size| size > 64 * 1024)
    {
        bail!("Runtime release health is unavailable");
    }
    let health = response.json::<HealthEnvelope>().await?;
    if health.status != "ok" {
        bail!("Runtime release health is not ready");
    }
    Ok(health)
}

fn verify_release(config: &Config, release: &ReleaseIdentity) -> Result<()> {
    if release.source_revision != config.identity.source_revision {
        bail!("Runtime source revision mismatch");
    }
    Ok(())
}

fn fallback_release(config: &Config) -> ReleaseIdentity {
    ReleaseIdentity {
        core_version: "unavailable".into(),
        source_revision: config.identity.source_revision.clone(),
        api_contract_version: 1,
        assertion_contract_version: 1,
        schema_version: 1,
        harnesses: std::collections::BTreeMap::new(),
        harness_agents: std::collections::BTreeMap::new(),
        harness_dependencies: std::collections::BTreeMap::new(),
    }
}

async fn disk_available_bytes() -> Result<u64> {
    let output = tokio::process::Command::new("df")
        .args(["-Pk", "/company"])
        .output()
        .await?;
    let line = String::from_utf8(output.stdout)?
        .lines()
        .nth(1)
        .context("df omitted company volume")?
        .to_string();
    let blocks = line
        .split_whitespace()
        .nth(3)
        .context("df omitted available blocks")?
        .parse::<u64>()?;
    Ok(blocks.saturating_mul(1024))
}

async fn queue(sender: &mpsc::Sender<Outbound>, message: BridgeMessage) -> Result<()> {
    tokio::time::timeout(
        SEND_DEADLINE,
        sender.send(Outbound {
            message,
            response: None,
        }),
    )
    .await
    .context("Runtime bridge outbound queue timed out")?
    .context("queue Runtime bridge frame")
}

async fn send_frame<S>(sender: &mut S, frame: Frame) -> Result<()>
where
    S: futures_util::Sink<WebSocketMessage, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    tokio::time::timeout(
        SEND_DEADLINE,
        sender.send(WebSocketMessage::Binary(frame.encode()?.into())),
    )
    .await
    .context("Runtime bridge websocket write timed out")??;
    Ok(())
}

fn message_operation_id(message: &BridgeMessage) -> Option<Uuid> {
    match message {
        BridgeMessage::CoordinationRequest { operation_id, .. } => Some(*operation_id),
        _ => None,
    }
}

fn required(name: &'static str) -> Result<String> {
    let value = std::env::var(name).with_context(|| format!("{name} is required"))?;
    if value.is_empty() || value.trim() != value || value.contains(['\r', '\n']) {
        bail!("{name} is invalid");
    }
    Ok(value)
}

fn required_uuid(name: &'static str) -> Result<Uuid> {
    let value = Uuid::parse_str(&required(name)?).with_context(|| format!("parse {name}"))?;
    if value.is_nil() {
        bail!("{name} must be non-nil");
    }
    Ok(value)
}

fn required_i64(name: &'static str) -> Result<i64> {
    let value = required(name)?
        .parse::<i64>()
        .with_context(|| format!("parse {name}"))?;
    if value < 1 {
        bail!("{name} must be positive");
    }
    Ok(value)
}

fn read_secret(path: &Path) -> Result<String> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 16_384 {
        bail!("Runtime bridge capability file is invalid");
    }
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
    if metadata.permissions().mode() & 0o777 != 0o640 || metadata.gid() != BRIDGE_SECRET_GID {
        bail!("Runtime bridge capability file must be 0640 and owned by the bridge secret group");
    }
    let file = std::fs::File::open(path)?;
    let opened = file.metadata()?;
    if opened.dev() != metadata.dev()
        || opened.ino() != metadata.ino()
        || opened.len() != metadata.len()
    {
        bail!("Runtime bridge capability file changed while opening");
    }
    let mut value = String::new();
    std::io::Read::take(file, 16_385).read_to_string(&mut value)?;
    let value = value.strip_suffix('\n').unwrap_or(&value).to_string();
    if value.is_empty() || value.len() > 16_384 || value.contains(['\r', '\n']) {
        bail!("Runtime bridge capability is invalid");
    }
    Ok(value)
}

fn read_ca_pem(path: &Path) -> Result<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path)
        .with_context(|| format!("read Runtime bridge CA metadata at {}", path.display()))?;
    use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > MAX_CA_BYTES
        || metadata.permissions().mode() & 0o777 != 0o640
        || metadata.gid() != BRIDGE_SECRET_GID
    {
        bail!("Runtime bridge CA must be one bounded non-writable regular file");
    }
    let file = std::fs::File::open(path)?;
    let opened = file.metadata()?;
    if opened.dev() != metadata.dev()
        || opened.ino() != metadata.ino()
        || opened.len() != metadata.len()
    {
        bail!("Runtime bridge CA changed while opening");
    }
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or_default());
    file.take(MAX_CA_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.is_empty() || u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_CA_BYTES {
        bail!("Runtime bridge CA is outside its byte bound");
    }
    Ok(bytes)
}

fn tls_connector(ca_pem: Option<&[u8]>) -> Result<Connector> {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    if let Some(ca_pem) = ca_pem {
        let mut reader = Cursor::new(ca_pem);
        let items = rustls_pemfile::read_all(&mut reader)
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("parse Runtime bridge CA PEM")?;
        if items.is_empty() || items.len() > 16 {
            bail!("Runtime bridge CA must contain between one and sixteen certificates");
        }
        for item in items {
            let rustls_pemfile::Item::X509Certificate(certificate) = item else {
                bail!("Runtime bridge CA file may contain certificates only");
            };
            roots
                .add(certificate)
                .context("Runtime bridge CA contains an invalid certificate")?;
        }
    }
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(Connector::Rustls(std::sync::Arc::new(config)))
}

fn create_session_root(operation_id: Uuid) -> Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt as _;
    let bridge_root = Path::new("/tmp/restless-runtime-bridge");
    let sessions = bridge_root.join("agent-sessions");
    for path in [bridge_root, sessions.as_path()] {
        match std::fs::create_dir(path) {
            Ok(()) => std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o711))?,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let metadata = std::fs::symlink_metadata(path)?;
                use std::os::unix::fs::MetadataExt as _;
                if !metadata.is_dir()
                    || metadata.file_type().is_symlink()
                    || metadata.uid() != 0
                    || metadata.permissions().mode() & 0o777 != 0o711
                {
                    bail!("Runtime bridge session root is not the root-owned boundary");
                }
            }
            Err(error) => return Err(error).context("create Runtime bridge session root"),
        }
    }
    let session_root = sessions.join(operation_id.simple().to_string());
    std::fs::create_dir(&session_root).context("create unique agent session directory")?;
    std::fs::set_permissions(&session_root, std::fs::Permissions::from_mode(0o700))?;
    Ok(session_root)
}

#[cfg(target_os = "linux")]
fn drop_agent_privileges() -> std::io::Result<()> {
    // Drop the bounding set before uid transition. CAP_SETPCAP (8) must be
    // retained until every other bound is gone, then drops itself last.
    for capability in (0..=63).filter(|capability| *capability != 8) {
        let result = unsafe { nix::libc::prctl(nix::libc::PR_CAPBSET_DROP, capability, 0, 0, 0) };
        if result != 0 && std::io::Error::last_os_error().raw_os_error() != Some(nix::libc::EINVAL)
        {
            return Err(std::io::Error::last_os_error());
        }
    }
    if unsafe { nix::libc::prctl(nix::libc::PR_CAPBSET_DROP, 8, 0, 0, 0) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    if unsafe {
        nix::libc::prctl(
            nix::libc::PR_CAP_AMBIENT,
            nix::libc::PR_CAP_AMBIENT_CLEAR_ALL,
            0,
            0,
            0,
        )
    } != 0
        || unsafe { nix::libc::setgroups(0, std::ptr::null()) } != 0
        || unsafe { nix::libc::setgid(2000) } != 0
        || unsafe { nix::libc::setuid(2000) } != 0
        || unsafe { nix::libc::prctl(nix::libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) } != 0
    {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn drop_agent_privileges() -> std::io::Result<()> {
    if unsafe { nix::libc::setgroups(0, std::ptr::null()) } != 0
        || unsafe { nix::libc::setgid(2000) } != 0
        || unsafe { nix::libc::setuid(2000) } != 0
    {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

fn make_company_owned(root: &Path) -> Result<()> {
    for entry in walkdir(root)? {
        std::os::unix::fs::chown(&entry, Some(2000), Some(2000))
            .with_context(|| format!("assign agent session path {}", entry.display()))?;
    }
    Ok(())
}

fn walkdir(root: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = vec![root.to_path_buf()];
    let mut cursor = 0;
    while cursor < paths.len() {
        let path = paths[cursor].clone();
        cursor += 1;
        if path.is_dir() {
            for entry in std::fs::read_dir(&path)? {
                paths.push(entry?.path());
            }
        }
    }
    paths.reverse();
    Ok(paths)
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("private file has no parent")?;
    std::fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".runtime-bridge-{}.tmp", Uuid::new_v4()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?;
    std::io::Write::write_all(&mut file, bytes)?;
    file.sync_all()?;
    std::fs::rename(&temporary, path)?;
    Ok(())
}

fn now_ms() -> Result<i64> {
    i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis())
        .context("clock exceeds protocol")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime_identity(generation: i64) -> RuntimeIdentity {
        RuntimeIdentity {
            owner_id: Uuid::new_v4(),
            plane_id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            cell_id: Uuid::new_v4(),
            company: "company".into(),
            runtime_id: "runtime-1".into(),
            runtime_generation: generation,
            runtime_image: format!("runtime@sha256:{}", "a".repeat(64)),
            volume_name: "company-data".into(),
            source_revision: "b".repeat(40),
        }
    }

    fn runtime_release() -> ReleaseIdentity {
        ReleaseIdentity {
            core_version: "test".into(),
            source_revision: "b".repeat(40),
            api_contract_version: 1,
            assertion_contract_version: 1,
            schema_version: 1,
            harnesses: std::collections::BTreeMap::new(),
            harness_agents: std::collections::BTreeMap::new(),
            harness_dependencies: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn hosted_model_relay_is_exact_and_same_origin() {
        let bridge = "wss://plane.example/internal/v1/runtime-bridge";
        assert!(
            validate_model_url(bridge, "https://plane.example/internal/v1/model-gateway").is_ok()
        );
        for refused in [
            "http://plane.example/internal/v1/model-gateway",
            "https://other.example/internal/v1/model-gateway",
            "https://plane.example/internal/v1/model-gateway?route=other",
            "https://plane.example/v1/responses",
            "https://user@plane.example/internal/v1/model-gateway",
        ] {
            assert!(
                validate_model_url(bridge, refused).is_err(),
                "accepted {refused}"
            );
        }
    }

    #[test]
    fn git_evidence_parser_counts_renames_once_and_requires_canonical_oids() {
        let status = b" M src/main.rs\0R  new-name.rs\0old-name.rs\0?? notes.txt\0";
        assert_eq!(dirty_entry_count(status).unwrap(), 3);
        assert_eq!(
            canonical_oid(format!("{}\n", "a".repeat(40)).as_bytes()),
            Some("a".repeat(40))
        );
        assert!(canonical_oid("A".repeat(40).as_bytes()).is_none());
        assert!(dirty_entry_count(b"malformed\0").is_err());
    }

    #[test]
    fn artifact_manifest_refuses_committed_symlinks() {
        let ordinary = format!("100644 blob {} 12\tsrc/main.rs\0", "a".repeat(40));
        assert_eq!(
            summarize_git_tree_manifest(ordinary.as_bytes()).unwrap(),
            (1, 12)
        );
        let linked = format!("120000 blob {} 12\tsecrets\0", "b".repeat(40));
        assert!(summarize_git_tree_manifest(linked.as_bytes())
            .unwrap_err()
            .to_string()
            .contains("symbolic link"));
    }

    #[test]
    fn environment_fingerprint_changes_with_the_pinned_generation_and_release() {
        let first = environment_fingerprint(&runtime_identity(1), &runtime_release()).unwrap();
        let second = environment_fingerprint(&runtime_identity(2), &runtime_release()).unwrap();
        assert_ne!(first, second);
        assert_eq!(first.len(), 64);

        let mut release = runtime_release();
        release.core_version = "next".into();
        let changed_release = environment_fingerprint(&runtime_identity(1), &release).unwrap();
        assert_ne!(first, changed_release);
    }

    #[tokio::test]
    async fn completed_workspace_operation_replays_only_for_the_same_request() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let operations = WorkspaceOperations::default();
        let workspace = WorkspaceIdentity {
            attempt_id: Uuid::new_v4(),
            work_id: Uuid::new_v4(),
            work_revision: 1,
            repo: "game".into(),
            worktree: "work-1-r1".into(),
        };
        let operation_id = Uuid::new_v4();
        let deadline_ms = now_ms().unwrap() + 30_000;
        let request = BridgeMessage::ObserveWorkspace {
            operation_id,
            workspace: workspace.clone(),
            expected_environment_fingerprint: "c".repeat(64),
            deadline_ms,
        };
        let response = BridgeMessage::WorkspaceObserved {
            operation_id,
            workspace: workspace.clone(),
            observation: WorkspaceObservation {
                repo_path: "/company/repos/game".into(),
                workdir: "/company/worktrees/work-1-r1".into(),
                source_commit: "d".repeat(40),
                source_tree: "e".repeat(40),
                status_digest: "f".repeat(64),
                dirty_entries: 0,
                environment_fingerprint: "c".repeat(64),
                observed_at_ms: now_ms().unwrap(),
            },
        };
        operations.state.lock().unwrap().completed.insert(
            operation_id,
            CompletedWorkspaceOperation {
                request: request.clone(),
                response: response.clone(),
                completed_at: Instant::now(),
            },
        );
        let config = Config {
            url: "wss://plane.example/internal/v1/runtime-bridge".into(),
            identity: runtime_identity(1),
            capability_file: "/run/secrets/runtime-bridge".into(),
            tls_connector: tls_connector(None).unwrap(),
            agent_ca_pem: None,
        };
        let (outbound, mut queued) = mpsc::channel(2);
        operations
            .start(
                config.clone(),
                Uuid::new_v4(),
                "c".repeat(64),
                request.clone(),
                outbound.clone(),
            )
            .unwrap();
        assert_eq!(queued.recv().await.unwrap().message, response);

        let refreshed_deadline = BridgeMessage::ObserveWorkspace {
            operation_id,
            workspace: workspace.clone(),
            expected_environment_fingerprint: "c".repeat(64),
            deadline_ms: deadline_ms + 1_000,
        };
        operations
            .start(
                config.clone(),
                Uuid::new_v4(),
                "c".repeat(64),
                refreshed_deadline,
                outbound.clone(),
            )
            .unwrap();
        assert_eq!(queued.recv().await.unwrap().message, response);

        let conflicting = BridgeMessage::ObserveWorkspace {
            operation_id,
            workspace,
            expected_environment_fingerprint: "0".repeat(64),
            deadline_ms,
        };
        operations
            .start(
                config,
                Uuid::new_v4(),
                "c".repeat(64),
                conflicting,
                outbound,
            )
            .unwrap();
        assert!(matches!(
            queued.recv().await.unwrap().message,
            BridgeMessage::Error { code, .. } if code == "operation_replay_conflict"
        ));
    }
}
