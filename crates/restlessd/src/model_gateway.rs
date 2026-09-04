//! Host-side model credential isolation through OMP's imported auth broker
//! and auth gateway.
//!
//! Restless supervises the open-source proxy shipped by the ACP runtime for
//! mature pi-native traffic and exposes one narrow first-party Responses relay
//! for the pinned Codex runtime. Both place credentials only on the host and
//! give company processes a short-lived signed exact-model capability.
//! Provider keys, OMP's root bearer, and Infisical machine-identity credentials
//! never cross into the Company Runtime.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::Stdio;
use std::sync::{Arc, OnceLock, RwLock};
use std::task::{Context as TaskContext, Poll};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, State};
use axum::http::header::{ACCEPT, AUTHORIZATION, CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderMap, Response, StatusCode};
use axum::routing::{get, post};
use axum::Router;
use futures_util::{Stream, StreamExt as _};
use serde::{Deserialize, Serialize};
use tokio::process::{Child, Command};

use crate::runtime::CompanyConfig;

const BROKER_PROFILE: &str = "restless-model-broker";
const GATEWAY_PROFILE: &str = "restless-model-gateway";
/// OMP itself keeps the root provider bearer on this loopback-only listener.
#[cfg(test)]
const OMP_GATEWAY_HOST_URL: &str = "http://127.0.0.1:7796";
/// The Runtime-facing relay owns the established container route. It never
/// accepts OMP's root bearer.
#[cfg(test)]
const RELAY_RUNTIME_URL: &str = "http://host.docker.internal:7790";
const MODEL_CAPABILITY_ENV: &str = "RESTLESS_MODEL_CAPABILITY";
const DISABLED_LOCAL_DISCOVERY_URL: &str = "http://127.0.0.1:1/v1";
pub(crate) const RESPONSES_TARIFF_VERSION: &str = "omp-18.0.10-gpt-5.6-2026-08-30";

#[derive(Debug, Clone)]
struct GatewayEndpoints {
    broker_profile: String,
    gateway_profile: String,
    broker_url: String,
    broker_bind: String,
    gateway_host_url: String,
    gateway_bind: String,
    relay_runtime_url: String,
    relay_bind: String,
    relay_loopback_probe: String,
    relay_loopback_url: String,
}

impl GatewayEndpoints {
    fn from_env() -> Result<Self> {
        let offset = crate::port_offset()?;
        let broker_port = crate::port_with_offset(7789)?;
        let gateway_port = crate::port_with_offset(7796)?;
        let relay_port = crate::port_with_offset(7790)?;
        let entry_mode = std::env::var("RESTLESS_ENTRY_MODE").ok();
        let entry_host = std::env::var("RESTLESS_ENTRY_HOST").ok();
        let profile_suffix = (offset != 0).then(|| format!("-{offset}"));
        Ok(Self {
            broker_profile: format!(
                "{BROKER_PROFILE}{}",
                profile_suffix.as_deref().unwrap_or_default()
            ),
            gateway_profile: format!(
                "{GATEWAY_PROFILE}{}",
                profile_suffix.as_deref().unwrap_or_default()
            ),
            broker_url: format!("http://127.0.0.1:{broker_port}"),
            broker_bind: format!("127.0.0.1:{broker_port}"),
            gateway_host_url: format!("http://127.0.0.1:{gateway_port}"),
            gateway_bind: format!("127.0.0.1:{gateway_port}"),
            relay_runtime_url: runtime_relay_url(
                entry_mode.as_deref(),
                entry_host.as_deref(),
                relay_port,
            )?,
            relay_bind: format!("0.0.0.0:{relay_port}"),
            relay_loopback_probe: format!("127.0.0.1:{relay_port}"),
            relay_loopback_url: format!("http://127.0.0.1:{relay_port}"),
        })
    }
}

fn runtime_relay_url(
    entry_mode: Option<&str>,
    entry_host: Option<&str>,
    port: u16,
) -> Result<String> {
    match entry_mode.unwrap_or("local") {
        "local" => Ok(format!("http://host.docker.internal:{port}")),
        "network" => {
            let host = entry_host.context("RESTLESS_ENTRY_HOST is required in network mode")?;
            if host.is_empty()
                || host.len() > 253
                || host.split('.').any(|label| {
                    label.is_empty()
                        || label.len() > 63
                        || label.starts_with('-')
                        || label.ends_with('-')
                        || !label.bytes().all(|byte| {
                            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
                        })
                })
            {
                bail!("RESTLESS_ENTRY_HOST must be a bounded lowercase DNS hostname");
            }
            Ok(format!("https://{host}/internal/v1/model"))
        }
        _ => bail!("RESTLESS_ENTRY_MODE must be `local` or `network`"),
    }
}

pub(crate) fn hosted_relay_loopback_url() -> Result<String> {
    Ok(GatewayEndpoints::from_env()?.relay_loopback_url)
}

fn preflight_runtime_relay_port(loopback_bind: &str) -> Result<()> {
    // Docker Desktop can own 127.0.0.1:PORT while macOS still permits a
    // second process to bind 0.0.0.0:PORT. Company containers then resolve
    // host.docker.internal to Docker's listener and silently reach the wrong
    // service. Probe the exact loopback alias before spawning any broker
    // process so an isolated plane fails with an actionable offset error.
    let listener = std::net::TcpListener::bind(loopback_bind).with_context(|| {
        format!(
            "Runtime model relay port {loopback_bind} is already claimed on loopback; \
             choose a collision-free RESTLESS_PORT_OFFSET"
        )
    })?;
    drop(listener);
    Ok(())
}

static CLIENT: OnceLock<ClientConfig> = OnceLock::new();

/// Companies the account plane currently cannot admit a model route for.
/// Consulted before a company wakes so the refusal names the exact reason
/// instead of failing later inside the company's first Attempt.
static UNSTARTABLE: OnceLock<RwLock<BTreeMap<String, String>>> = OnceLock::new();

/// Why this company cannot start, if the plane could not admit it.
pub fn unstartable_reason(company: &str) -> Option<String> {
    UNSTARTABLE
        .get_or_init(|| RwLock::new(BTreeMap::new()))
        .read()
        .ok()?
        .get(company)
        .cloned()
}

pub fn is_ready() -> bool {
    CLIENT
        .get_or_init(ClientConfig::empty)
        .snapshot
        .read()
        .is_ok_and(|snapshot| snapshot.is_some())
}

#[derive(Clone)]
pub struct ClientConfig {
    snapshot: Arc<RwLock<Option<GatewayClientSnapshot>>>,
}

#[derive(Clone)]
struct GatewayClientSnapshot {
    providers: BTreeMap<String, ModelBilling>,
    runtime_url: String,
    primary_models: BTreeMap<String, String>,
    admitted_models: BTreeMap<String, BTreeSet<String>>,
}

impl ClientConfig {
    fn empty() -> Self {
        Self {
            snapshot: Arc::new(RwLock::new(None)),
        }
    }

    fn replace(&self, snapshot: Option<GatewayClientSnapshot>) -> Result<()> {
        *self
            .snapshot
            .write()
            .map_err(|_| anyhow::anyhow!("model gateway client state is unavailable"))? = snapshot;
        Ok(())
    }

    fn billing_for(&self, model: &str) -> Result<ModelBilling> {
        let (provider, _) = split_model(model)?;
        self.snapshot
            .read()
            .map_err(|_| anyhow::anyhow!("model gateway client state is unavailable"))?
            .as_ref()
            .and_then(|snapshot| snapshot.providers.get(provider).copied())
            .with_context(|| {
                format!("model provider {provider} is not admitted by the host gateway")
            })
    }

    fn company_primary_model_is_admitted(&self, company: &str, model: &str) -> bool {
        self.snapshot.read().is_ok_and(|snapshot| {
            snapshot.as_ref().is_some_and(|snapshot| {
                snapshot.primary_models.get(company).map(String::as_str) == Some(model)
                    && snapshot
                        .admitted_models
                        .get(company)
                        .is_some_and(|models| models.contains(model))
            })
        })
    }

    fn company_model_route_is_admitted(&self, company: &str, model: &str) -> bool {
        let Ok((provider, _)) = split_model(model) else {
            return false;
        };
        self.snapshot.read().is_ok_and(|snapshot| {
            snapshot.as_ref().is_some_and(|snapshot| {
                snapshot.admitted_models.get(company).is_some_and(|models| {
                    models.iter().any(|candidate| {
                        split_model(candidate)
                            .is_ok_and(|(candidate_provider, _)| candidate_provider == provider)
                    })
                })
            })
        })
    }

    fn provider_billing(&self, provider: &str) -> Option<ModelBilling> {
        self.snapshot.read().ok().and_then(|snapshot| {
            snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.providers.get(provider).copied())
        })
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "model admission must bind the exact capability identity and productive coordinates in one visible call"
    )]
    pub fn auth_for(
        &self,
        model: &str,
        capabilities: &crate::capability::CapabilityIssuer,
        company: &str,
        actor: &str,
        session: &str,
        responsibility: &str,
        work_id: Option<uuid::Uuid>,
        attempt_id: Option<uuid::Uuid>,
    ) -> Result<AgentGatewayAuth> {
        let (provider, _) = split_model(model)?;
        let snapshot = self
            .snapshot
            .read()
            .map_err(|_| anyhow::anyhow!("model gateway client state is unavailable"))?;
        let snapshot = snapshot
            .as_ref()
            .context("host model gateway is not installed; restlessd did not finish booting")?;
        if !snapshot.admitted_models.get(company).is_some_and(|models| {
            models.iter().any(|candidate| {
                split_model(candidate)
                    .is_ok_and(|(candidate_provider, _)| candidate_provider == provider)
            })
        }) {
            bail!("company {company} is not admitted for exact model {model}");
        }
        let billing = snapshot.providers.get(provider).copied().with_context(|| {
            format!("model provider {provider} is not admitted by the host gateway")
        })?;
        Ok(AgentGatewayAuth {
            token_env: MODEL_CAPABILITY_ENV.to_string(),
            token: capabilities.issue_model_session(
                company,
                actor,
                session,
                provider,
                model,
                billing.as_str(),
                responsibility,
                work_id,
                attempt_id,
            )?,
            runtime_url: snapshot.runtime_url.clone(),
            billing,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HostedAdmissionOutcome {
    Admitted,
    Unstartable,
    IdentityDrift,
    Unavailable,
}

#[derive(Clone)]
pub(crate) struct HostedModelAdmission {
    sender: tokio::sync::mpsc::Sender<HostedAdmissionRequest>,
}

pub(crate) struct HostedAdmissionRequests {
    receiver: tokio::sync::mpsc::Receiver<HostedAdmissionRequest>,
}

struct HostedAdmissionRequest {
    config: CompanyConfig,
    response: tokio::sync::oneshot::Sender<HostedAdmissionOutcome>,
}

pub(crate) fn hosted_admission_channel() -> (HostedModelAdmission, HostedAdmissionRequests) {
    let (sender, receiver) = tokio::sync::mpsc::channel(32);
    (
        HostedModelAdmission { sender },
        HostedAdmissionRequests { receiver },
    )
}

impl HostedModelAdmission {
    /// Ask the plane-owned supervisor to make this exact, already-verified
    /// company/model policy live. The request carries references, never
    /// resolved provider material; credential resolution remains inside the
    /// model gateway process boundary.
    pub(crate) async fn admit(&self, config: CompanyConfig) -> HostedAdmissionOutcome {
        let (response, result) = tokio::sync::oneshot::channel();
        if self
            .sender
            .send(HostedAdmissionRequest { config, response })
            .await
            .is_err()
        {
            return HostedAdmissionOutcome::Unavailable;
        }
        result.await.unwrap_or(HostedAdmissionOutcome::Unavailable)
    }
}

impl HostedAdmissionRequests {
    pub(crate) async fn reconcile_next(
        &mut self,
        configs: &mut Vec<CompanyConfig>,
        processes: &mut Option<Processes>,
        root: &Path,
        capabilities: &crate::capability::CapabilityIssuer,
        spend: &crate::spend::SpendLedger,
    ) -> bool {
        let Some(request) = self.receiver.recv().await else {
            return false;
        };
        reconcile_hosted_admission(configs, processes, request, root, capabilities, spend).await;
        true
    }
}

/// Return the current billing contract for one exact model route without
/// issuing a session capability or contacting the provider.
pub fn billing_for_model(model: &str) -> Result<ModelBilling> {
    client()?.billing_for(model)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelBilling {
    MeteredApi,
    Subscription,
}

impl ModelBilling {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MeteredApi => "metered_api",
            Self::Subscription => "subscription",
        }
    }
}

pub struct AgentGatewayAuth {
    pub token_env: String,
    pub token: String,
    pub runtime_url: String,
    pub billing: ModelBilling,
}

/// Apply the installation-wide cooldown facts to one company's ordered model
/// policy. This is shared by Exec and Staff so a dead provider is not retried
/// once per actor and wake.
pub async fn available_candidates(
    config: &CompanyConfig,
    preferred: Option<&str>,
    authority: &crate::authority::AuthorityStore,
) -> Result<Vec<String>> {
    let ordered = ordered_candidates(config, preferred)?;
    let cooldowns = authority.active_model_cooldowns(&config.name).await?;
    filter_cooling_candidates(ordered, &cooldowns)
}

/// An explicit non-Exec actor preference is an exact next-wake contract, not
/// an invitation to inherit the company's fallback chain. Exec may cross that
/// chain for portfolio continuity; a lead or worker must instead block
/// visibly so role/model experiments and specialist assignments cannot change
/// capability silently.
pub async fn available_actor_candidates(
    config: &CompanyConfig,
    preferred: Option<&str>,
    authority: &crate::authority::AuthorityStore,
) -> Result<Vec<String>> {
    let ordered = actor_candidates(config, preferred)?;
    let cooldowns = authority.active_model_cooldowns(&config.name).await?;
    filter_cooling_candidates(ordered, &cooldowns)
}

/// A durable addressed message remains owed while its actor's exact policy is
/// unavailable. Schedulers use this read-only preflight to stay quiet until a
/// recorded cooldown expires instead of retrying the same conversation on
/// every database scan.
pub async fn actor_policy_is_cooling(
    config: &CompanyConfig,
    preferred: Option<&str>,
    authority: &crate::authority::AuthorityStore,
) -> Result<bool> {
    let ordered = actor_candidates(config, preferred)?;
    let cooldowns = authority.active_model_cooldowns(&config.name).await?;
    Ok(candidates_all_cooling(&ordered, &cooldowns))
}

fn actor_candidates(config: &CompanyConfig, preferred: Option<&str>) -> Result<Vec<String>> {
    match preferred.map(str::trim).filter(|model| !model.is_empty()) {
        Some(model) => Ok(vec![model.to_string()]),
        None => ordered_candidates(config, None),
    }
}

fn ordered_candidates(config: &CompanyConfig, preferred: Option<&str>) -> Result<Vec<String>> {
    let mut ordered = Vec::new();
    if let Some(model) = preferred.map(str::trim).filter(|model| !model.is_empty()) {
        ordered.push(model.to_string());
    }
    for model in config.model_candidates()? {
        if !ordered.iter().any(|candidate| candidate == model) {
            ordered.push(model.to_string());
        }
    }
    // A candidate not admitted for this exact company cannot be reached.
    // Hosted admission may change while the plane remains alive, so consult
    // the current snapshot rather than a boot-time provider list.
    if is_ready() {
        let client = CLIENT
            .get()
            .expect("model gateway readiness requires a client state");
        ordered.retain(|model| client.company_model_route_is_admitted(&config.name, model));
    }
    Ok(ordered)
}

fn filter_cooling_candidates(
    mut ordered: Vec<String>,
    cooldowns: &[crate::authority::ModelCooldown],
) -> Result<Vec<String>> {
    let configured = ordered.clone();
    ordered.retain(|model| !cooldowns.iter().any(|cooldown| &cooldown.model == model));
    if ordered.is_empty() {
        let next = cooldowns
            .iter()
            .filter(|cooldown| configured.iter().any(|model| model == &cooldown.model))
            .min_by_key(|cooldown| cooldown.retry_at)
            .map(|cooldown| {
                format!(
                    "{} until {} ({})",
                    cooldown.model,
                    cooldown.retry_at.to_rfc3339(),
                    cooldown.kind
                )
            })
            .unwrap_or_else(|| "no configured candidates".into());
        bail!("all model candidates are cooling down; next: {next}");
    }
    Ok(ordered)
}

fn candidates_all_cooling(
    ordered: &[String],
    cooldowns: &[crate::authority::ModelCooldown],
) -> bool {
    !ordered.is_empty()
        && ordered
            .iter()
            .all(|model| cooldowns.iter().any(|cooldown| &cooldown.model == model))
}

pub async fn record_cooldown(
    authority: &crate::authority::AuthorityStore,
    company: &str,
    model: &str,
    kind: crate::health::BlockKind,
    reason: &str,
) -> Result<()> {
    let duration = match kind {
        crate::health::BlockKind::Credential | crate::health::BlockKind::Model => {
            chrono::Duration::hours(24)
        }
        crate::health::BlockKind::Quota => chrono::Duration::hours(1),
        crate::health::BlockKind::NoOp => chrono::Duration::minutes(15),
        crate::health::BlockKind::Transport => chrono::Duration::minutes(2),
        _ => return Ok(()),
    };
    let retry_at = chrono::Utc::now() + duration;
    authority
        .set_model_cooldown(
            company,
            model,
            kind.as_str(),
            &reason.chars().take(500).collect::<String>(),
            retry_at,
        )
        .await?;
    authority
        .emit(
            company,
            "model_cooldown",
            None,
            serde_json::json!({
                "model": model, "kind": kind.as_str(), "retry_at": retry_at,
            }),
        )
        .await?;
    Ok(())
}

/// Child handles are deliberately ordinary supervised processes. Dropping the
/// daemon drops these handles and requests process termination; their durable
/// credential vault remains host-side in OMP's Restless-only profile.
pub struct Processes {
    broker: Option<Child>,
    gateway: Option<Child>,
    relay: Option<tokio::task::JoinHandle<()>>,
    marker: PathBuf,
}

impl Processes {
    /// Stop the replaceable model boundary and wait for its owned listeners to
    /// close before a hosted admission attempt binds the same ports again.
    pub(crate) async fn shutdown(mut self) {
        if let Some(relay) = self.relay.take() {
            relay.abort();
            let _ = relay.await;
        }
        if let Some(mut gateway) = self.gateway.take() {
            let _ = gateway.kill().await;
        }
        if let Some(mut broker) = self.broker.take() {
            let _ = broker.kill().await;
        }
        let _ = std::fs::remove_file(&self.marker);
    }
}

impl Drop for Processes {
    fn drop(&mut self) {
        if let Some(relay) = self.relay.take() {
            relay.abort();
        }
        if let Some(gateway) = &mut self.gateway {
            let _ = gateway.start_kill();
        }
        if let Some(broker) = &mut self.broker {
            let _ = broker.start_kill();
        }
        let _ = std::fs::remove_file(&self.marker);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManagedModelChild {
    pid: u32,
    program: String,
    args: Vec<String>,
}

fn model_children_path(root: &Path) -> PathBuf {
    root.join("machine/model-children.json")
}

fn resolved_program(program: &str) -> Result<PathBuf> {
    let candidate = Path::new(program);
    if candidate.components().count() > 1 {
        return std::fs::canonicalize(candidate)
            .with_context(|| format!("resolve model child program {program}"));
    }
    let path = std::env::var_os("PATH").context("PATH is unavailable")?;
    for directory in std::env::split_paths(&path) {
        let candidate = directory.join(program);
        if candidate.is_file() {
            return std::fs::canonicalize(&candidate)
                .with_context(|| format!("resolve {}", candidate.display()));
        }
    }
    bail!("model child program {program:?} is not on PATH")
}

fn read_model_children(path: &Path) -> Vec<ManagedModelChild> {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_model_children(path: &Path, children: &[ManagedModelChild]) -> Result<()> {
    let parent = path.parent().context("model child marker has no parent")?;
    std::fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".model-children-{}.tmp", std::process::id()));
    std::fs::write(&temporary, serde_json::to_vec_pretty(children)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(temporary, path)?;
    Ok(())
}

fn record_model_child(root: &Path, child: &Child, program: &Path, args: &[&str]) -> Result<()> {
    let pid = child.id().context("model child has no pid")?;
    let path = model_children_path(root);
    let mut children = read_model_children(&path);
    children.retain(|entry| entry.pid != pid);
    children.push(ManagedModelChild {
        pid,
        program: program.display().to_string(),
        args: args.iter().map(|value| (*value).to_string()).collect(),
    });
    write_model_children(&path, &children)
}

fn forget_model_child(root: &Path, pid: u32) -> Result<()> {
    let path = model_children_path(root);
    let mut children = read_model_children(&path);
    children.retain(|entry| entry.pid != pid);
    if children.is_empty() {
        if path.exists() {
            std::fs::remove_file(path)?;
        }
    } else {
        write_model_children(&path, &children)?;
    }
    Ok(())
}

fn model_child_matches(entry: &ManagedModelChild) -> bool {
    let output = std::process::Command::new("ps")
        .args(["-p", &entry.pid.to_string(), "-o", "command="])
        .output();
    let Ok(output) = output else { return false };
    if !output.status.success() {
        return false;
    }
    let observed = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let mut words = observed.split_whitespace();
    let Some(program) = words.next() else {
        return false;
    };
    let exact = Path::new(program).file_name() == Path::new(&entry.program).file_name()
        && words.eq(entry.args.iter().map(String::as_str));
    exact && allowed_model_child_args(&entry.args)
}

fn allowed_model_child_args(args: &[String]) -> bool {
    matches!(
        args,
        [command, token] if (command == "auth-broker" || command == "auth-gateway") && token == "token"
    ) || matches!(
        args,
        [command, serve, bind, _]
            if (command == "auth-broker" || command == "auth-gateway")
                && serve == "serve"
                && bind == "--bind"
    )
}

fn sweep_orphaned_model_children(root: &Path) -> Result<()> {
    let path = model_children_path(root);
    let children = read_model_children(&path);
    for child in children {
        if !model_child_matches(&child) {
            continue;
        }
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &child.pid.to_string()])
            .status();
        std::thread::sleep(Duration::from_millis(100));
        if model_child_matches(&child) {
            let _ = std::process::Command::new("kill")
                .args(["-KILL", &child.pid.to_string()])
                .status();
        }
    }
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn replace_client_snapshot(snapshot: Option<GatewayClientSnapshot>) -> Result<()> {
    CLIENT.get_or_init(ClientConfig::empty).replace(snapshot)
}

fn replace_unstartable(companies: BTreeMap<String, String>) {
    if let Ok(mut current) = UNSTARTABLE
        .get_or_init(|| RwLock::new(BTreeMap::new()))
        .write()
    {
        *current = companies;
    }
}

fn mark_unstartable(company: &str, reason: String) {
    if let Ok(mut current) = UNSTARTABLE
        .get_or_init(|| RwLock::new(BTreeMap::new()))
        .write()
    {
        current.insert(company.to_owned(), reason);
    }
}

fn admitted_models(
    configs: &[&CompanyConfig],
    credentials: &BTreeMap<String, ProviderCredential>,
) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let mut admitted = BTreeMap::new();
    for config in configs {
        let mut models = BTreeSet::new();
        for model in config.model_candidates()? {
            let (provider, _) = split_model(model)?;
            if credentials.contains_key(provider) {
                models.insert(model.to_owned());
            }
        }
        admitted.insert(config.name.clone(), models);
    }
    Ok(admitted)
}

fn same_gateway_policy(left: &CompanyConfig, right: &CompanyConfig) -> bool {
    left.name == right.name
        && left.model == right.model
        && left.model_failover == right.model_failover
        && left.credentials == right.credentials
        && left.worker_runtime == right.worker_runtime
}

pub(crate) fn company_model_is_admitted(company: &str, model: &str) -> bool {
    CLIENT
        .get_or_init(ClientConfig::empty)
        .company_primary_model_is_admitted(company, model)
}

async fn reconcile_hosted_admission(
    configs: &mut Vec<CompanyConfig>,
    processes: &mut Option<Processes>,
    request: HostedAdmissionRequest,
    root: &Path,
    capabilities: &crate::capability::CapabilityIssuer,
    spend: &crate::spend::SpendLedger,
) {
    let HostedAdmissionRequest { config, response } = request;
    let outcome =
        reconcile_hosted_admission_inner(configs, processes, config, root, capabilities, spend)
            .await;
    let _ = response.send(outcome);
}

async fn reconcile_hosted_admission_inner(
    configs: &mut Vec<CompanyConfig>,
    processes: &mut Option<Processes>,
    config: CompanyConfig,
    root: &Path,
    capabilities: &crate::capability::CapabilityIssuer,
    spend: &crate::spend::SpendLedger,
) -> HostedAdmissionOutcome {
    if let Some(existing) = configs.iter().find(|existing| existing.name == config.name) {
        if !same_gateway_policy(existing, &config) {
            tracing::warn!(
                company = %config.name,
                "refused hosted model admission because the company's gateway policy drifted"
            );
            return HostedAdmissionOutcome::IdentityDrift;
        }
    }
    if company_model_is_admitted(&config.name, &config.model) {
        return HostedAdmissionOutcome::Admitted;
    }

    let mut candidate = configs.clone();
    if !candidate
        .iter()
        .any(|existing| existing.name == config.name)
    {
        candidate.push(config.clone());
    }

    // Resolve references before interrupting a working gateway. An absent
    // first-company credential is scoped unstartable state, not a reason to
    // take another company's provider route down.
    let credentials = match provider_credentials(&candidate).await {
        Ok(credentials) => credentials,
        Err(error) => {
            tracing::warn!(company = %config.name, "hosted model admission preflight failed: {error:#}");
            return HostedAdmissionOutcome::IdentityDrift;
        }
    };
    let admission = match admit(&candidate, &credentials) {
        Ok(admission) => admission,
        Err(error) => {
            tracing::warn!(company = %config.name, "hosted model policy is invalid: {error:#}");
            return HostedAdmissionOutcome::IdentityDrift;
        }
    };
    if let Some(reason) = admission.unstartable.get(&config.name) {
        mark_unstartable(&config.name, reason.clone());
        tracing::warn!(company = %config.name, reason, "hosted company is waiting for a usable provider credential");
        return HostedAdmissionOutcome::Unstartable;
    }
    drop(credentials);

    replace_client_snapshot(None).ok();
    if let Some(running) = processes.take() {
        running.shutdown().await;
    }
    match start(&candidate, root, capabilities.clone(), spend.clone()).await {
        Ok(next) => {
            *configs = candidate;
            *processes = next;
            if company_model_is_admitted(&config.name, &config.model) {
                HostedAdmissionOutcome::Admitted
            } else {
                HostedAdmissionOutcome::Unstartable
            }
        }
        Err(error) => {
            tracing::error!(company = %config.name, "hosted model admission failed: {error:#}");
            // Restore the previously admitted set. A new company may fail
            // closed, but it must not strand an already-working sibling.
            match start(configs, root, capabilities.clone(), spend.clone()).await {
                Ok(previous) => *processes = previous,
                Err(recovery) => tracing::error!(
                    "model gateway recovery after refused hosted admission failed: {recovery:#}"
                ),
            }
            HostedAdmissionOutcome::Unavailable
        }
    }
}

/// Start the imported broker/gateway pair and install its narrow client
/// configuration for ACP and world-model processes.
pub async fn start(
    configs: &[CompanyConfig],
    root: &std::path::Path,
    capabilities: crate::capability::CapabilityIssuer,
    spend: crate::spend::SpendLedger,
) -> Result<Option<Processes>> {
    sweep_orphaned_model_children(root)?;
    let endpoints = GatewayEndpoints::from_env()?;
    let mut provider_credentials = provider_credentials(configs).await?;
    if provider_credentials.is_empty() {
        // The account plane is not a company. It serves the cockpit, holds the
        // owner's credentials and routes surfaces; it performs no company work,
        // so it must start with zero startable companies and report them
        // unstartable — the same admission rule S25-T1 established per company,
        // applied to the plane itself.
        //
        // Refusing to boot here would make a freshly provisioned hosted plane
        // unstartable until its first company existed, which inverts Cloud's
        // provisioning order: Fleet creates the plane, then the cell.
        tracing::warn!(
            companies = configs.len(),
            "no company model provider is available; the plane will serve the cockpit \
             and every company will report its own unstartable reason"
        );
        let admission = admit(configs, &provider_credentials)?;
        replace_unstartable(admission.unstartable);
        replace_client_snapshot(None)?;
        return Ok(None);
    }

    preflight_runtime_relay_port(&endpoints.relay_loopback_probe)?;

    let omp =
        resolved_program(&std::env::var("RESTLESS_OMP_BIN").unwrap_or_else(|_| "omp".to_string()))?;
    let discover_llama_cpp = provider_credentials.contains_key("llama.cpp")
        || std::env::var_os("LLAMA_CPP_BASE_URL").is_some();
    let broker_token = token(
        root,
        &omp,
        &endpoints.broker_profile,
        "auth-broker",
        discover_llama_cpp,
    )
    .await?;
    let broker_args = [
        "auth-broker",
        "serve",
        "--bind",
        endpoints.broker_bind.as_str(),
    ];
    let mut broker_command = Command::new(&omp);
    if !discover_llama_cpp {
        broker_command.env("LLAMA_CPP_BASE_URL", DISABLED_LOCAL_DISCOVERY_URL);
    }
    let mut broker = broker_command
        .current_dir(root)
        .env("OMP_PROFILE", &endpoints.broker_profile)
        .args(broker_args)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .context("start OMP model credential broker")?;
    record_model_child(root, &broker, &omp, &broker_args)?;
    wait_for_broker(&mut broker, &broker_token, &endpoints.broker_url).await?;

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .context("build model credential sync client")?;
    prune_unconfigured_credentials(
        &http,
        &broker_token,
        &endpoints.broker_url,
        provider_credentials.keys().map(String::as_str).collect(),
    )
    .await?;
    for (provider, credential) in &provider_credentials {
        let ProviderCredential::ApiKey(key) = credential else {
            continue;
        };
        let response = http
            .post(format!("{}/v1/credential", endpoints.broker_url))
            .bearer_auth(&broker_token)
            .json(&serde_json::json!({
                "provider": provider,
                "credential": { "type": "api_key", "key": key }
            }))
            .send()
            .await
            .with_context(|| format!("sync {provider} credential to host model broker"))?;
        if !response.status().is_success() {
            // Never include a provider response body here: a hostile or buggy
            // backend can reflect request material, including the secret.
            bail!(
                "host model broker refused the {provider} credential with HTTP {}",
                response.status()
            );
        }
    }
    let unadmitted = reconcile_credentials(
        &http,
        &broker_token,
        &endpoints.broker_url,
        &provider_credentials,
    )
    .await?;
    provider_credentials.retain(|provider, _| !unadmitted.contains(provider));
    if provider_credentials.is_empty() {
        bail!("no configured company model provider could be admitted by the host model broker");
    }
    let admission = admit(configs, &provider_credentials)?;
    for (company, reason) in &admission.unstartable {
        tracing::warn!(company, reason, "company cannot start: {reason}");
    }
    let startable = admission.startable(configs);
    replace_unstartable(admission.unstartable);

    let gateway_token = token(
        root,
        &omp,
        &endpoints.gateway_profile,
        "auth-gateway",
        discover_llama_cpp,
    )
    .await?;
    let mut gateway_command = Command::new(&omp);
    gateway_command
        .current_dir(root)
        .env("OMP_PROFILE", &endpoints.gateway_profile)
        .env("OMP_AUTH_BROKER_URL", &endpoints.broker_url)
        .env("OMP_AUTH_BROKER_TOKEN", &broker_token);
    // OMP probes llama.cpp's conventional localhost:8080 endpoint while it
    // refreshes its catalogue. Another desktop service can legitimately own
    // that port and answer without being llama.cpp, leaving OMP's discovery
    // open before the gateway binds. A Restless gateway may discover only
    // providers admitted by company policy, so route an unconfigured local
    // provider to an immediately closed loopback endpoint.
    if !discover_llama_cpp {
        gateway_command.env("LLAMA_CPP_BASE_URL", DISABLED_LOCAL_DISCOVERY_URL);
    }
    if provider_credentials.contains_key("litellm") {
        let base_url = std::env::var("GPT_BASE_URL")
            .context("litellm model route needs GPT_BASE_URL for OMP discovery")?;
        gateway_command.env("LITELLM_BASE_URL", base_url);
    }
    let gateway_args = [
        "auth-gateway",
        "serve",
        "--bind",
        endpoints.gateway_bind.as_str(),
    ];
    let mut gateway = gateway_command
        .args(gateway_args)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .context("start OMP model auth gateway")?;
    record_model_child(root, &gateway, &omp, &gateway_args)?;
    let required_providers = provider_credentials
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let required_models = runtime_pinned_models(&startable)?;
    wait_for_gateway(
        &mut gateway,
        &gateway_token,
        &endpoints.gateway_host_url,
        &required_providers,
        &required_models,
    )
    .await?;
    let responses_routes = direct_responses_routes(&provider_credentials)?;
    let primary_models = startable
        .iter()
        .map(|config| (config.name.clone(), config.model.clone()))
        .collect();
    let admitted_models = admitted_models(&startable, &provider_credentials)?;
    let providers = provider_credentials
        .into_iter()
        .map(|(provider, credential)| (provider, credential.billing()))
        .collect();

    let relay = start_runtime_relay(
        RelayState {
            root: root.to_path_buf(),
            capabilities,
            spend,
            upstream_token: gateway_token,
            upstream_url: endpoints.gateway_host_url,
            responses_routes,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(15 * 60))
                .build()
                .context("build Runtime model relay client")?,
        },
        &endpoints.relay_bind,
    )
    .await?;
    if let Err(error) = replace_client_snapshot(Some(GatewayClientSnapshot {
        providers,
        runtime_url: endpoints.relay_runtime_url.clone(),
        primary_models,
        admitted_models,
    })) {
        relay.abort();
        return Err(error);
    }
    Ok(Some(Processes {
        broker: Some(broker),
        gateway: Some(gateway),
        relay: Some(relay),
        marker: model_children_path(root),
    }))
}

#[derive(Deserialize)]
struct BrokerSnapshot {
    credentials: Vec<BrokerCredential>,
}

#[derive(Deserialize)]
struct BrokerCredential {
    id: i64,
    provider: String,
    credential: serde_json::Value,
}

impl BrokerCredential {
    fn api_key(&self) -> Option<&str> {
        (self.credential.get("type")?.as_str()? == "api_key")
            .then(|| self.credential.get("key")?.as_str())?
    }

    fn is_oauth(&self) -> bool {
        self.credential.get("type").and_then(|value| value.as_str()) == Some("oauth")
    }
}

async fn broker_snapshot(
    http: &reqwest::Client,
    broker_token: &str,
    broker_url: &str,
) -> Result<BrokerSnapshot> {
    let response = http
        .get(format!("{broker_url}/v1/snapshot"))
        .bearer_auth(broker_token)
        .send()
        .await
        .context("read host model broker credential snapshot")?;
    if !response.status().is_success() {
        bail!(
            "host model broker snapshot failed with HTTP {}",
            response.status()
        );
    }
    response
        .json::<BrokerSnapshot>()
        .await
        .context("parse redacted host model broker snapshot")
}

async fn disable_credential(
    http: &reqwest::Client,
    broker_token: &str,
    broker_url: &str,
    credential: &BrokerCredential,
    cause: &str,
) -> Result<()> {
    let response = http
        .post(format!(
            "{broker_url}/v1/credential/{}/disable",
            credential.id
        ))
        .bearer_auth(broker_token)
        .json(&serde_json::json!({ "cause": cause }))
        .send()
        .await
        .with_context(|| format!("disable stale {} model credential", credential.provider))?;
    if !response.status().is_success() {
        bail!(
            "host model broker refused stale {} credential removal with HTTP {}",
            credential.provider,
            response.status()
        );
    }
    Ok(())
}

/// The OMP profile survives daemon restarts, so changing a company from one
/// provider to another must not leave the old provider routable as a silent
/// fallback. Disable every active row not named by the current company set
/// before the gateway snapshots its catalogue.
async fn prune_unconfigured_credentials(
    http: &reqwest::Client,
    broker_token: &str,
    broker_url: &str,
    configured: BTreeSet<&str>,
) -> Result<()> {
    let snapshot = broker_snapshot(http, broker_token, broker_url).await?;
    for credential in snapshot
        .credentials
        .into_iter()
        .filter(|credential| !configured.contains(credential.provider.as_str()))
    {
        disable_credential(
            http,
            broker_token,
            broker_url,
            &credential,
            "provider is not configured by any Restless company at daemon boot",
        )
        .await?;
    }
    Ok(())
}

/// API-key rows have no broker identity key, so OMP intentionally treats a
/// changed key as a second account. Restless V0 has one credential per provider,
/// however: after uploading the current Infisical value, disable every other
/// active row for that provider. This both applies rotation and prevents daemon
/// restarts from accumulating equally privileged fallback keys.
/// Returns the providers that could **not** be canonicalised at the broker.
/// A configured credential reference can resolve and still have no usable
/// account behind it — an `omp-oauth:` provider the owner has not signed in to,
/// for instance. That is one provider's fact, not the plane's: the provider is
/// dropped and the companies that depend on it become unstartable, rather than
/// the whole plane refusing to boot (cross-layer contract §1.4.1).
async fn reconcile_credentials(
    http: &reqwest::Client,
    broker_token: &str,
    broker_url: &str,
    provider_credentials: &BTreeMap<String, ProviderCredential>,
) -> Result<BTreeSet<String>> {
    let snapshot = broker_snapshot(http, broker_token, broker_url).await?;
    let mut unadmitted = BTreeSet::new();
    for (provider, expected) in provider_credentials {
        let canonical = match expected {
            ProviderCredential::ApiKey(expected_key) => {
                canonical_api_key_credential(&snapshot, provider, expected_key)
            }
            ProviderCredential::OmpOauth => canonical_oauth_credential(&snapshot, provider),
        };
        let (keep_id, superseded_ids) = match canonical {
            Ok(canonical) => canonical,
            Err(error) => {
                tracing::warn!(provider, "provider not admitted: {error:#}");
                unadmitted.insert(provider.clone());
                continue;
            }
        };
        for credential in snapshot
            .credentials
            .iter()
            .filter(|credential| superseded_ids.contains(&credential.id))
        {
            disable_credential(
                http,
                broker_token,
                broker_url,
                credential,
                "superseded by the current Restless model credential reference",
            )
            .await?;
        }

        let verified = broker_snapshot(http, broker_token, broker_url).await?;
        let active = verified
            .credentials
            .iter()
            .filter(|credential| credential.provider == *provider)
            .collect::<Vec<_>>();
        let matches = active.len() == 1
            && active[0].id == keep_id
            && match expected {
                ProviderCredential::ApiKey(expected_key) => {
                    active[0].api_key() == Some(expected_key.as_str())
                }
                ProviderCredential::OmpOauth => active[0].is_oauth(),
            };
        if !matches {
            tracing::warn!(
                provider,
                "provider not admitted: host model broker did not converge it to one current credential"
            );
            unadmitted.insert(provider.clone());
        }
    }
    Ok(unadmitted)
}

fn canonical_api_key_credential(
    snapshot: &BrokerSnapshot,
    provider: &str,
    expected_key: &str,
) -> Result<(i64, Vec<i64>)> {
    let provider_rows = snapshot
        .credentials
        .iter()
        .filter(|credential| credential.provider == provider)
        .collect::<Vec<_>>();
    let matching = provider_rows
        .iter()
        .filter(|credential| credential.api_key() == Some(expected_key))
        .collect::<Vec<_>>();
    if matching.len() != 1 {
        bail!(
            "host model broker has {} current {provider} credentials after sync; expected exactly one",
            matching.len()
        );
    }
    let keep_id = matching[0].id;
    let superseded = provider_rows
        .into_iter()
        .filter_map(|credential| (credential.id != keep_id).then_some(credential.id))
        .collect();
    Ok((keep_id, superseded))
}

fn canonical_oauth_credential(
    snapshot: &BrokerSnapshot,
    provider: &str,
) -> Result<(i64, Vec<i64>)> {
    let provider_rows = snapshot
        .credentials
        .iter()
        .filter(|credential| credential.provider == provider)
        .collect::<Vec<_>>();
    let matching = provider_rows
        .iter()
        .filter(|credential| credential.is_oauth())
        .collect::<Vec<_>>();
    if matching.len() != 1 {
        bail!(
            "host model broker has {} OAuth credentials for {provider}; expected exactly one owner-authenticated account",
            matching.len()
        );
    }
    let keep_id = matching[0].id;
    let superseded = provider_rows
        .into_iter()
        .filter_map(|credential| (credential.id != keep_id).then_some(credential.id))
        .collect();
    Ok((keep_id, superseded))
}

#[derive(Clone)]
struct RelayState {
    root: std::path::PathBuf,
    capabilities: crate::capability::CapabilityIssuer,
    spend: crate::spend::SpendLedger,
    upstream_token: String,
    upstream_url: String,
    responses_routes: BTreeMap<String, DirectResponsesRoute>,
    http: reqwest::Client,
}

#[derive(Clone)]
struct DirectResponsesRoute {
    base_url: String,
    api_key: String,
}

fn direct_responses_routes(
    credentials: &BTreeMap<String, ProviderCredential>,
) -> Result<BTreeMap<String, DirectResponsesRoute>> {
    let mut routes = BTreeMap::new();
    if let Some(ProviderCredential::ApiKey(api_key)) = credentials.get("litellm") {
        let mut url = reqwest::Url::parse(
            &std::env::var("GPT_BASE_URL").context("litellm Responses route needs GPT_BASE_URL")?,
        )
        .context("parse GPT_BASE_URL for the Responses relay")?;
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            bail!("GPT_BASE_URL is not a credential-free HTTP(S) origin/path");
        }
        if url.path() == "/" || url.path().is_empty() {
            url.set_path("/v1");
        }
        routes.insert(
            "litellm".to_string(),
            DirectResponsesRoute {
                base_url: url.as_str().trim_end_matches('/').to_string(),
                api_key: api_key.clone(),
            },
        );
    }
    Ok(routes)
}

async fn start_runtime_relay(
    state: RelayState,
    relay_bind: &str,
) -> Result<tokio::task::JoinHandle<()>> {
    let listener = tokio::net::TcpListener::bind(relay_bind)
        .await
        .with_context(|| format!("bind Runtime model relay {relay_bind}"))?;
    let app = Router::new()
        .route("/v1/models", get(relay_models))
        .route("/v1/pi/stream", post(relay_pi_stream))
        .route("/v1/responses", post(relay_responses))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(state);
    Ok(tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, app).await {
            tracing::error!("Runtime model relay stopped: {error}");
        }
    }))
}

/// Give a Runtime only the exact model named by its signed session grant.
///
/// Dynamic OpenAI-compatible providers need a catalogue before ACP can select
/// a model. Letting them query the provider directly would bypass both host
/// credential custody and exact-model authority. A static custom model is not
/// equivalent: OMP currently drops the provider's `pi-native` transport while
/// constructing that custom entry. This narrow catalogue preserves OMP's
/// ordinary discovery/metadata merge and therefore its native gateway route.
async fn relay_models(State(state): State<RelayState>, headers: HeaderMap) -> Response<Body> {
    let token = match bearer_capability(&headers) {
        Ok(token) => token,
        Err(error) => return relay_error(StatusCode::UNAUTHORIZED, &error),
    };
    let grant = match state.capabilities.verify_model(token) {
        Ok(grant) => grant,
        Err(error) => {
            return relay_error(
                StatusCode::UNAUTHORIZED,
                &format!("invalid model capability: {error:#}"),
            );
        }
    };
    let (_, model_id) = match split_model(&grant.model) {
        Ok(parts) => parts,
        Err(error) => return relay_error(StatusCode::UNAUTHORIZED, &format!("{error:#}")),
    };
    let body = serde_json::json!({
        "object": "list",
        "data": [{
            "id": model_id,
            "object": "model",
            "owned_by": grant.provider,
        }],
    })
    .to_string();
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/json")
        .header(CACHE_CONTROL, "no-store")
        .body(Body::from(body))
        .expect("static model catalogue response")
}

async fn relay_pi_stream(
    State(state): State<RelayState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    let token = match bearer_capability(&headers) {
        Ok(token) => token,
        Err(error) => return relay_error(StatusCode::UNAUTHORIZED, &error),
    };
    let grant = match state.capabilities.verify_model(token) {
        Ok(grant) => grant,
        Err(error) => {
            return relay_error(
                StatusCode::UNAUTHORIZED,
                &format!("invalid model capability: {error:#}"),
            );
        }
    };
    let request = match serde_json::from_slice::<serde_json::Value>(&body) {
        Ok(request) => request,
        Err(_) => return relay_error(StatusCode::BAD_REQUEST, "model request body is not JSON"),
    };
    let model = match requested_model(&request) {
        Some(model) => model,
        None => return relay_error(StatusCode::BAD_REQUEST, "model request has no modelId"),
    };
    let provider = match split_model(model) {
        Ok((provider, _)) => provider,
        Err(_) => {
            return relay_error(
                StatusCode::BAD_REQUEST,
                "model request must use a provider-qualified model id",
            );
        }
    };
    if provider != grant.provider || model != grant.model {
        return relay_error(
            StatusCode::FORBIDDEN,
            "model capability does not permit this exact model",
        );
    }
    let config = match CompanyConfig::load(&state.root, &grant.company) {
        Ok(config) => config,
        Err(_) => {
            return relay_error(
                StatusCode::FORBIDDEN,
                "model capability company is unavailable",
            );
        }
    };
    let billing = match grant.billing.as_str() {
        "metered_api" => ModelBilling::MeteredApi,
        "subscription" => ModelBilling::Subscription,
        _ => {
            return relay_error(
                StatusCode::UNAUTHORIZED,
                "model capability has an invalid billing policy",
            );
        }
    };
    if billing == ModelBilling::MeteredApi {
        let budget = state.spend.budget_state(&config);
        if !budget.is_available() {
            return relay_error(
                StatusCode::PAYMENT_REQUIRED,
                &budget.owner_message(&config.name),
            );
        }
    }

    let mut upstream_request = state
        .http
        .post(format!("{}/v1/pi/stream", state.upstream_url))
        .bearer_auth(&state.upstream_token)
        .header(CONTENT_TYPE, "application/json")
        .body(body);
    if let Some(accept) = headers.get(ACCEPT) {
        upstream_request = upstream_request.header(ACCEPT, accept);
    }
    let upstream = match upstream_request.send().await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(company = %grant.company, actor = %grant.actor, "model relay upstream transport: {error}");
            return relay_error(StatusCode::BAD_GATEWAY, "host model gateway is unavailable");
        }
    };
    if !upstream.status().is_success() {
        let status = upstream.status();
        tracing::warn!(
            company = %grant.company,
            actor = %grant.actor,
            upstream_status = %status,
            "host model gateway refused Runtime request"
        );
        return relay_error(status, "host model gateway refused the model request");
    }

    let status = upstream.status();
    let upstream_headers = upstream.headers().clone();
    let request_id = uuid::Uuid::new_v4();
    let stream = MeteredStream::new(
        upstream.bytes_stream(),
        state.spend.meter(),
        MeteredRequest {
            request_id,
            company: grant.company,
            actor: grant.actor,
            session: grant.session,
            responsibility: grant.responsibility,
            work_id: grant.work_id,
            attempt_id: grant.attempt_id,
            model: model.to_string(),
            billing,
        },
    );
    // The Runtime client and the provider's accounting stream have different
    // lifetimes. A deliberate ACP cancellation may close the former while the
    // provider is still producing its terminal charged-usage event. Keep a
    // host-owned drain alive so interruption can be prompt without turning a
    // known request into permanently unknown spend. The bounded channel still
    // provides ordinary downstream backpressure while connected; after a
    // disconnect the task stops forwarding bytes but continues to the exact
    // semantic terminal.
    let (stream, _drain) = detach_metered_stream(stream);
    let mut response = Response::builder()
        .status(status)
        .header("x-restless-request-id", request_id.to_string());
    for name in [CONTENT_TYPE, CACHE_CONTROL] {
        if let Some(value) = upstream_headers.get(&name) {
            response = response.header(name, value);
        }
    }
    response
        .body(Body::from_stream(stream))
        .unwrap_or_else(|_| relay_error(StatusCode::BAD_GATEWAY, "could not relay model stream"))
}

/// Relay the documented OpenAI Responses wire used by the pinned first-party
/// Codex app-server. The Runtime still receives only its signed, exact-model
/// capability: the host-side OMP gateway retains the provider credential and
/// the relay rewrites the unqualified Codex model id to the provider-qualified
/// route only after verifying that capability.
async fn relay_responses(
    State(state): State<RelayState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    let token = match bearer_capability(&headers) {
        Ok(token) => token,
        Err(error) => return relay_error(StatusCode::UNAUTHORIZED, &error),
    };
    let grant = match state.capabilities.verify_model(token) {
        Ok(grant) => grant,
        Err(error) => {
            return relay_error(
                StatusCode::UNAUTHORIZED,
                &format!("invalid model capability: {error:#}"),
            );
        }
    };
    let (provider, model_id) = match split_model(&grant.model) {
        Ok(parts) => parts,
        Err(error) => return relay_error(StatusCode::UNAUTHORIZED, &format!("{error:#}")),
    };
    let mut request = match serde_json::from_slice::<serde_json::Value>(&body) {
        Ok(request) => request,
        Err(_) => return relay_error(StatusCode::BAD_REQUEST, "model request body is not JSON"),
    };
    let requested = match request.get("model").and_then(serde_json::Value::as_str) {
        Some(model) if !model.is_empty() => model,
        _ => return relay_error(StatusCode::BAD_REQUEST, "Responses request has no model"),
    };
    if requested != model_id && requested != grant.model {
        return relay_error(
            StatusCode::FORBIDDEN,
            "model capability does not permit this exact model",
        );
    }
    if request.get("stream").and_then(serde_json::Value::as_bool) != Some(true) {
        return relay_error(
            StatusCode::BAD_REQUEST,
            "Runtime Responses requests must stream for terminal accounting",
        );
    }
    // The host gateway catalogue names custom routes by provider-qualified id;
    // Codex correctly uses the provider-local id on the OpenAI wire.
    request["model"] = serde_json::Value::String(grant.model.clone());
    if response_tariff_micro_usd(model_id, 0, 0, 0).is_none() {
        return relay_error(
            StatusCode::BAD_REQUEST,
            "exact Responses tariff is not pinned for this model",
        );
    }
    let config = match CompanyConfig::load(&state.root, &grant.company) {
        Ok(config) => config,
        Err(_) => {
            return relay_error(
                StatusCode::FORBIDDEN,
                "model capability company is unavailable",
            );
        }
    };
    let billing = match grant.billing.as_str() {
        "metered_api" => ModelBilling::MeteredApi,
        "subscription" => ModelBilling::Subscription,
        _ => {
            return relay_error(
                StatusCode::UNAUTHORIZED,
                "model capability has an invalid billing policy",
            );
        }
    };
    if billing == ModelBilling::MeteredApi {
        let budget = state.spend.budget_state(&config);
        if !budget.is_available() {
            return relay_error(
                StatusCode::PAYMENT_REQUIRED,
                &budget.owner_message(&config.name),
            );
        }
    }
    let Some(route) = state.responses_routes.get(provider) else {
        return relay_error(
            StatusCode::BAD_REQUEST,
            "provider has no admitted first-party Responses contract",
        );
    };

    let encoded = match serde_json::to_vec(&request) {
        Ok(encoded) => encoded,
        Err(_) => return relay_error(StatusCode::BAD_REQUEST, "could not encode model request"),
    };
    let mut upstream_request = state
        .http
        .post(format!("{}/responses", route.base_url))
        .bearer_auth(&route.api_key)
        .header(CONTENT_TYPE, "application/json")
        .body(encoded);
    if let Some(accept) = headers.get(ACCEPT) {
        upstream_request = upstream_request.header(ACCEPT, accept);
    }
    let upstream = match upstream_request.send().await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(company = %grant.company, actor = %grant.actor, "Responses relay upstream transport: {error}");
            return relay_error(StatusCode::BAD_GATEWAY, "host model gateway is unavailable");
        }
    };
    if !upstream.status().is_success() {
        let status = upstream.status();
        tracing::warn!(
            company = %grant.company,
            actor = %grant.actor,
            upstream_status = %status,
            "host model gateway refused Runtime Responses request"
        );
        return relay_error(status, "host model gateway refused the model request");
    }

    let status = upstream.status();
    let upstream_headers = upstream.headers().clone();
    let request_id = uuid::Uuid::new_v4();
    let stream = MeteredStream::new_responses(
        upstream.bytes_stream(),
        state.spend.meter(),
        MeteredRequest {
            request_id,
            company: grant.company,
            actor: grant.actor,
            session: grant.session,
            responsibility: grant.responsibility,
            work_id: grant.work_id,
            attempt_id: grant.attempt_id,
            model: grant.model,
            billing,
        },
    );
    let (stream, _drain) = detach_metered_stream(stream);
    let mut response = Response::builder()
        .status(status)
        .header("x-restless-request-id", request_id.to_string());
    for name in [CONTENT_TYPE, CACHE_CONTROL] {
        if let Some(value) = upstream_headers.get(&name) {
            response = response.header(name, value);
        }
    }
    response
        .body(Body::from_stream(stream))
        .unwrap_or_else(|_| relay_error(StatusCode::BAD_GATEWAY, "could not relay model stream"))
}

fn detach_metered_stream(
    mut upstream: MeteredStream,
) -> (
    impl Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Send,
    tokio::task::JoinHandle<()>,
) {
    let (sender, receiver) = tokio::sync::mpsc::channel(32);
    let drain = tokio::spawn(async move {
        let mut downstream_open = true;
        while let Some(item) = upstream.next().await {
            if downstream_open && sender.send(item).await.is_err() {
                downstream_open = false;
            }
        }
    });
    let stream = futures_util::stream::unfold(receiver, |mut receiver| async move {
        receiver.recv().await.map(|item| (item, receiver))
    });
    (stream, drain)
}

fn bearer_capability(headers: &HeaderMap) -> std::result::Result<&str, String> {
    let raw = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| "model request has no bearer capability".to_string())?;
    raw.strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
        .ok_or_else(|| "model request bearer capability is malformed".to_string())
}

fn requested_model(request: &serde_json::Value) -> Option<&str> {
    request
        .get("modelId")
        .and_then(serde_json::Value::as_str)
        .filter(|model| !model.is_empty())
        .or_else(|| {
            request
                .get("model")
                .and_then(serde_json::Value::as_str)
                .filter(|model| !model.is_empty())
        })
        .or_else(|| {
            request
                .get("model")
                .and_then(serde_json::Value::as_object)
                .and_then(|model| model.get("id"))
                .and_then(serde_json::Value::as_str)
                .filter(|model| !model.is_empty())
        })
}

fn relay_error(status: StatusCode, message: &str) -> Response<Body> {
    let body = serde_json::json!({ "error": { "message": message } }).to_string();
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .expect("static relay error response")
}

struct MeteredRequest {
    request_id: uuid::Uuid,
    company: String,
    actor: String,
    session: String,
    responsibility: String,
    work_id: Option<uuid::Uuid>,
    attempt_id: Option<uuid::Uuid>,
    model: String,
    billing: ModelBilling,
}

impl MeteredRequest {
    fn spend_record(
        &self,
        input_tokens: u64,
        output_tokens: u64,
        total_tokens: u64,
        cached_input_tokens: Option<u64>,
        cost_micro_usd: u64,
        settlement: restless_model_gateway::SpendSettlement,
    ) -> restless_model_gateway::SpendRecord {
        restless_model_gateway::SpendRecord {
            request_id: self.request_id,
            company_id: self.company.clone(),
            model: self.model.clone(),
            input_tokens,
            output_tokens,
            total_tokens,
            cached_input_tokens,
            cost_micro_usd,
            actor_id: self.actor.clone(),
            session_id: self.session.clone(),
            responsibility: self.responsibility.clone(),
            work_id: self.work_id,
            attempt_id: self.attempt_id,
            settlement,
            occurred_at: chrono::Utc::now(),
        }
    }
}

#[derive(Clone, Copy)]
enum MeteringProtocol {
    PiNative,
    OpenAiResponses,
}

/// The relay forwards chunks unchanged but observes enough pi-native SSE to
/// make the terminal charged usage a host-side record. A provider error is a
/// valid terminal only when its canonical error message carries a provider
/// cost (including zero). A semantic pi-native `done` or `error` message is
/// the accounting boundary. The `[DONE]` sentinel is transport cleanup, and
/// a partial, Drop, or EOF without a semantic terminal remains ambiguous and
/// therefore records one request-local metering uncertainty.
struct MeteredStream {
    inner: Pin<Box<dyn Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Send>>,
    meter: crate::spend::TurnMeter,
    request: MeteredRequest,
    protocol: MeteringProtocol,
    frame_buffer: Vec<u8>,
    settled: bool,
    failed: bool,
}

impl MeteredStream {
    fn new<S>(inner: S, meter: crate::spend::TurnMeter, request: MeteredRequest) -> Self
    where
        S: Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Send + 'static,
    {
        Self {
            inner: Box::pin(inner),
            meter,
            request,
            protocol: MeteringProtocol::PiNative,
            frame_buffer: Vec::new(),
            settled: false,
            failed: false,
        }
    }

    fn new_responses<S>(inner: S, meter: crate::spend::TurnMeter, request: MeteredRequest) -> Self
    where
        S: Stream<Item = std::result::Result<Bytes, reqwest::Error>> + Send + 'static,
    {
        Self {
            inner: Box::pin(inner),
            meter,
            request,
            protocol: MeteringProtocol::OpenAiResponses,
            frame_buffer: Vec::new(),
            settled: false,
            failed: false,
        }
    }

    fn observe(&mut self, bytes: &Bytes) {
        if self.settled || self.failed {
            return;
        }
        self.frame_buffer.extend_from_slice(bytes);
        if self.frame_buffer.len() > 256 * 1024 {
            self.failed = true;
            return;
        }
        while let Some((end, separator)) = sse_frame_end(&self.frame_buffer) {
            let frame = self.frame_buffer.drain(..end).collect::<Vec<_>>();
            self.frame_buffer.drain(..separator);
            self.observe_frame(&frame);
            if self.settled || self.failed {
                return;
            }
        }
    }

    /// The pi-native client accepts a final SSE event without the customary
    /// blank-line delimiter. Mirror that tolerant protocol behaviour here,
    /// but keep strict JSON parsing so a genuinely truncated terminal frame is
    /// still unaccountable and therefore fail-closed.
    fn finish(&mut self) {
        if self.settled || self.failed || self.frame_buffer.is_empty() {
            return;
        }
        let frame = std::mem::take(&mut self.frame_buffer);
        self.observe_frame(&frame);
    }

    fn observe_frame(&mut self, frame: &[u8]) {
        let Ok(frame) = std::str::from_utf8(frame) else {
            self.failed = true;
            return;
        };
        let data = frame
            .lines()
            .filter_map(|line| line.trim_end_matches('\r').strip_prefix("data:"))
            .map(str::trim_start)
            .collect::<Vec<_>>()
            .join("\n");
        if data.is_empty() {
            return;
        }
        if data == "[DONE]" {
            // A normal semantic `done` or `error` should already have settled
            // the request. A bare sentinel cannot prove spend and remains
            // fail-closed.
            self.record_terminal_usage(None);
            return;
        }
        let Ok(event) = serde_json::from_str::<serde_json::Value>(&data) else {
            self.failed = true;
            return;
        };
        match self.protocol {
            MeteringProtocol::PiNative => {
                match event.get("type").and_then(serde_json::Value::as_str) {
                    Some("done") => self.record_terminal_usage(event.pointer("/message/usage")),
                    Some("error") => {
                        // pi-native error events are terminal messages too. A provider
                        // refusal may legitimately cost $0, but accepting it requires
                        // the same exact usage envelope as a success.
                        self.record_terminal_usage(event.pointer("/error/usage"));
                    }
                    _ => {}
                }
            }
            MeteringProtocol::OpenAiResponses => {
                match event.get("type").and_then(serde_json::Value::as_str) {
                    Some("response.completed") => {
                        self.record_responses_terminal(event.pointer("/response/usage"));
                    }
                    Some("response.failed") | Some("response.incomplete") => {
                        // A provider may have consumed tokens before a failed
                        // response. Settle only when the terminal usage is
                        // present; otherwise Drop poisons metered accounting.
                        self.record_responses_terminal(event.pointer("/response/usage"));
                    }
                    _ => {}
                }
            }
        }
    }

    fn record_terminal_usage(&mut self, usage: Option<&serde_json::Value>) {
        if self.settled || self.failed {
            return;
        }
        let tokens = usage.map(total_tokens).unwrap_or_default();
        let micro_usd = match self.request.billing {
            ModelBilling::MeteredApi => usage
                .and_then(|usage| usage.pointer("/cost/total"))
                .and_then(ceiling_micro_usd),
            ModelBilling::Subscription => Some(0),
        };
        let Some(micro_usd) = micro_usd else {
            self.failed = true;
            return;
        };
        self.meter.record_exact(self.request.spend_record(
            0,
            0,
            tokens,
            None,
            micro_usd,
            restless_model_gateway::SpendSettlement::Accounted,
        ));
        self.settled = true;
    }

    fn record_responses_terminal(&mut self, usage: Option<&serde_json::Value>) {
        if self.settled || self.failed {
            return;
        }
        let Some(usage) = usage else {
            self.failed = true;
            return;
        };
        let Some(input) = usage
            .get("input_tokens")
            .and_then(serde_json::Value::as_u64)
        else {
            self.failed = true;
            return;
        };
        let Some(output) = usage
            .get("output_tokens")
            .and_then(serde_json::Value::as_u64)
        else {
            self.failed = true;
            return;
        };
        let cached = usage
            .pointer("/input_tokens_details/cached_tokens")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_default();
        let tokens = usage
            .get("total_tokens")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or_else(|| input.saturating_add(output));
        let micro_usd = match self.request.billing {
            ModelBilling::MeteredApi => split_model(&self.request.model)
                .ok()
                .and_then(|(_, model)| response_tariff_micro_usd(model, input, output, cached)),
            ModelBilling::Subscription => Some(0),
        };
        let Some(micro_usd) = micro_usd else {
            self.failed = true;
            return;
        };
        self.meter.record_exact(self.request.spend_record(
            input,
            output,
            tokens,
            Some(cached),
            micro_usd,
            restless_model_gateway::SpendSettlement::Accounted,
        ));
        self.settled = true;
    }

    fn fail_closed(&mut self, detail: &str) {
        if self.request.billing == ModelBilling::MeteredApi && !self.settled {
            self.meter.record_unknown(self.request.spend_record(
                0,
                0,
                0,
                None,
                0,
                restless_model_gateway::SpendSettlement::MeteringUnknown,
            ));
            tracing::error!(
                company = %self.request.company,
                actor = %self.request.actor,
                session = %self.request.session,
                request_id = %self.request.request_id,
                "metered model response had no terminal charged usage; only this request is uncertain: {detail}"
            );
            self.settled = true;
        }
    }
}

impl Stream for MeteredStream {
    type Item = std::result::Result<Bytes, reqwest::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().get_mut();
        match this.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                this.observe(&bytes);
                Poll::Ready(Some(Ok(bytes)))
            }
            Poll::Ready(Some(Err(error))) => {
                this.fail_closed("upstream stream failed");
                Poll::Ready(Some(Err(error)))
            }
            Poll::Ready(None) => {
                this.finish();
                if !this.settled || this.failed {
                    this.fail_closed("stream ended before a valid done event");
                }
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for MeteredStream {
    fn drop(&mut self) {
        if !self.settled || self.failed {
            self.fail_closed("relay body dropped before a valid done event");
        }
    }
}

fn sse_frame_end(buffer: &[u8]) -> Option<(usize, usize)> {
    buffer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|end| (end, 4))
        .or_else(|| {
            buffer
                .windows(2)
                .position(|window| window == b"\n\n")
                .map(|end| (end, 2))
        })
}

fn total_tokens(usage: &serde_json::Value) -> u64 {
    if let Some(tokens) = usage.get("totalTokens").and_then(serde_json::Value::as_u64) {
        return tokens;
    }
    ["input", "output", "cacheRead", "cacheWrite"]
        .into_iter()
        .filter_map(|field| usage.get(field).and_then(serde_json::Value::as_u64))
        .fold(0_u64, u64::saturating_add)
}

/// Convert the provider's decimal charge into the ledger's coarser
/// micro-USD unit without ever understating spend. Providers may report
/// hundredths of a micro-dollar (for example `$0.01104072`); an upward
/// quantisation of less than one micro-dollar preserves the hard ceiling even
/// though the ledger does not store a fractional micro-dollar.
fn ceiling_micro_usd(value: &serde_json::Value) -> Option<u64> {
    let raw = value.as_number()?.to_string();
    let exponent_at = raw.find(['e', 'E']);
    let (significand, exponent) = match exponent_at {
        Some(index) => {
            let (significand, exponent) = raw.split_at(index);
            let exponent = exponent.get(1..)?.parse::<i64>().ok()?;
            if raw[index + 1..].contains(['e', 'E']) {
                return None;
            }
            (significand, exponent)
        }
        None => (raw.as_str(), 0),
    };
    let (whole, fraction) = significand.split_once('.').unwrap_or((significand, ""));
    if whole.is_empty()
        || whole.starts_with('-')
        || whole.starts_with('+')
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let digits = format!("{whole}{fraction}").parse::<u64>().ok()?;
    if digits == 0 {
        return Some(0);
    }
    // A JSON number might serialise in exponent form (for example 0.000001 as
    // 1e-6); normalise it with integer powers only. The ledger uses whole
    // micro-USD, so retain an exact value when possible and otherwise round
    // *up* rather than leave real spend outside the hard envelope.
    let shift = exponent
        .checked_sub(fraction.len() as i64)?
        .checked_add(6)?;
    if shift >= 0 {
        let power: u32 = shift.try_into().ok()?;
        digits.checked_mul(10_u64.checked_pow(power)?)
    } else {
        let scale = shift.checked_abs()?;
        // Any positive value smaller than 10^-19 USD still consumes one whole
        // micro-USD in a conservative integer ledger. Large positive shifts
        // cannot fit the ledger and stay fail-closed.
        if scale > 19 {
            return Some(u64::from(digits != 0));
        }
        let divisor = 10_u64.checked_pow(scale.try_into().ok()?)?;
        let whole = digits / divisor;
        whole.checked_add(u64::from(digits % divisor != 0))
    }
}

/// Pinned tariff used only for the first-party Responses relay. Values are
/// hundredths of a micro-USD per token and match the GPT-5.6 entries shipped
/// in the pinned OMP 18.0.10 catalogue in the Company image. Integer math and
/// upward rounding preserve the existing hard-spend invariant.
fn response_tariff_micro_usd(
    model: &str,
    input_tokens: u64,
    output_tokens: u64,
    cached_input_tokens: u64,
) -> Option<u64> {
    let long_context = input_tokens > 272_000;
    let (input_rate, output_rate, cached_rate): (u64, u64, u64) = match (model, long_context) {
        ("gpt-5.6-sol", false) => (500, 3_000, 50),
        ("gpt-5.6-sol", true) => (1_000, 4_500, 100),
        ("gpt-5.6-terra", false) => (200, 1_200, 20),
        ("gpt-5.6-terra", true) => (400, 1_800, 40),
        ("gpt-5.6-luna", false) => (20, 120, 2),
        ("gpt-5.6-luna", true) => (40, 180, 4),
        _ => return None,
    };
    let uncached = input_tokens.checked_sub(cached_input_tokens)?;
    let hundredth_micro_usd = uncached
        .checked_mul(input_rate)?
        .checked_add(cached_input_tokens.checked_mul(cached_rate)?)?
        .checked_add(output_tokens.checked_mul(output_rate)?)?;
    hundredth_micro_usd.checked_add(99).map(|value| value / 100)
}

pub fn client() -> Result<&'static ClientConfig> {
    let client = CLIENT.get_or_init(ClientConfig::empty);
    if client
        .snapshot
        .read()
        .is_ok_and(|snapshot| snapshot.is_some())
    {
        Ok(client)
    } else {
        bail!("host model gateway is not installed; restlessd did not finish booting")
    }
}

pub fn oauth_is_loaded(provider: &str) -> Result<bool> {
    Ok(matches!(
        client()?.provider_billing(provider),
        Some(ModelBilling::Subscription)
    ))
}

pub fn models_config(model: &str, runtime_url: &str, token_env: &str) -> Result<String> {
    let (provider, model_id) = split_model(model)?;
    if !provider
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        bail!("invalid model provider identifier {provider:?}");
    }
    let entry_mode = std::env::var("RESTLESS_ENTRY_MODE").ok();
    let entry_host = std::env::var("RESTLESS_ENTRY_HOST").ok();
    let recognised_runtime_relay =
        recognised_runtime_relay(runtime_url, entry_mode.as_deref(), entry_host.as_deref());
    if !recognised_runtime_relay || token_env != MODEL_CAPABILITY_ENV {
        bail!("refusing an unrecognised model gateway route");
    }
    let api = if matches!(provider, "moonshot" | "zai" | "zhipu-coding-plan") {
        // Moonshot and GLM expose these ACP-compatible routes as Chat
        // Completions. Advertising Responses makes OMP send a valid model to
        // the wrong upstream API; Moonshot surfaced that mismatch as a 500 and
        // GLM can return a bare 404 as assistant content.
        "openai-completions"
    } else {
        "openai-responses"
    };
    let catalogue = if provider == "litellm" {
        // The relay's capability-filtered `/v1/models` returns only this
        // session's exact grant. Discovery keeps OMP's provider-level
        // `pi-native` transport intact; a custom `models:` entry does not.
        "    discovery:\n      type: proxy\n"
    } else if matches!(provider, "zai" | "zhipu-coding-plan") && model_id == "glm-5.3-flash" {
        // Discover the one model authorised by the scoped relay and preserve
        // its exact text+image contract across OMP catalogue versions. A
        // static custom model would discard `pi-native` and bypass the only
        // Runtime route Restless exposes, producing a false HTTP 404.
        "    discovery:\n      type: proxy\n    modelOverrides:\n      glm-5.3-flash:\n        name: GLM 5.3 Flash\n        reasoning: true\n        supportsTools: true\n        input: [text, image]\n        cost: {input: 0, output: 0, cacheRead: 0, cacheWrite: 0}\n        contextWindow: 131072\n        maxTokens: 32768\n"
    } else {
        ""
    };
    Ok(format!(
        "# Managed by Restless. Contains a gateway route, never a provider credential.\n\
providers:\n  {provider}:\n    baseUrl: {runtime_url}\n    apiKey: {token_env}\n    transport: pi-native\n    api: {api}\n"
    ) + catalogue)
}

fn recognised_runtime_relay(
    runtime_url: &str,
    entry_mode: Option<&str>,
    entry_host: Option<&str>,
) -> bool {
    let Ok(url) = reqwest::Url::parse(runtime_url) else {
        return false;
    };
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return false;
    }
    match entry_mode.unwrap_or("local") {
        "local" => {
            url.scheme() == "http"
                && url.host_str() == Some("host.docker.internal")
                && url.port().is_some()
                && url.path() == "/"
        }
        "network" => {
            url.scheme() == "https"
                && entry_host.is_some_and(|host| url.host_str() == Some(host))
                && url.port().is_none()
                && url.path() == "/internal/v1/model"
        }
        _ => false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProviderCredential {
    ApiKey(String),
    OmpOauth,
}

impl ProviderCredential {
    fn billing(&self) -> ModelBilling {
        match self {
            Self::ApiKey(_) => ModelBilling::MeteredApi,
            Self::OmpOauth => ModelBilling::Subscription,
        }
    }
}

/// The account plane's model admission for this owner's companies.
///
/// Cross-layer contract §1.4.1: one company's configuration must never prevent
/// another company from starting. The plane validates its own credentials and
/// records the companies it cannot admit, with the reason, instead of refusing
/// to boot for all of them.
pub struct Admission {
    /// company name → why that company cannot start.
    pub unstartable: BTreeMap<String, String>,
}

impl Admission {
    /// Companies this plane can start, in configuration order.
    fn startable<'a>(&self, configs: &'a [CompanyConfig]) -> Vec<&'a CompanyConfig> {
        configs
            .iter()
            .filter(|config| !self.unstartable.contains_key(&config.name))
            .collect()
    }
}

async fn provider_credentials(
    configs: &[CompanyConfig],
) -> Result<BTreeMap<String, ProviderCredential>> {
    let mut credentials = BTreeMap::<String, ProviderCredential>::new();
    // Resolve only explicit host references on the first pass. Throwaway
    // configs deliberately carry none; they may use an installation route
    // authorised by another company, but iteration order must never decide it.
    for config in configs {
        let (primary_provider, _) = split_model(&config.model)?;
        for model in config.model_candidates()? {
            let (provider, _) = split_model(model)?;
            let provider_capability = format!("model.inference.{provider}");
            let reference = config.credentials.get(&provider_capability).or_else(|| {
                (provider == primary_provider)
                    .then(|| config.credentials.get("model.inference"))
                    .flatten()
            });
            let Some(reference) = reference else {
                continue;
            };
            let credential = match crate::credential::omp_oauth_provider(reference)? {
                Some(referenced_provider) => {
                    if referenced_provider != provider {
                        bail!(
                            "company {} maps {provider_capability} to OAuth provider {referenced_provider}",
                            config.name
                        );
                    }
                    ProviderCredential::OmpOauth
                }
                None => match crate::credential::resolve_reference(reference).await {
                    Ok(value) => ProviderCredential::ApiKey(value),
                    Err(error) => {
                        // A missing optional failover must not take down the
                        // account plane. A missing primary is projected by
                        // `admit` as this company's unstartable state. Keep
                        // configuration-shape errors above fatal, but turn an
                        // unavailable secret backend/value into scoped health.
                        tracing::warn!(
                            company = %config.name,
                            provider,
                            error = %format!("{error:#}"),
                            "model credential is unavailable"
                        );
                        continue;
                    }
                },
            };
            if let Some(existing) = credentials.get(provider) {
                if existing != &credential {
                    bail!(
                        "V0 model gateway refuses different {provider} credentials across companies; separate provider custody before multi-account use"
                    );
                }
            } else {
                credentials.insert(provider.to_string(), credential);
            }
        }
    }
    Ok(credentials)
}

/// Decide which companies this plane can start, given the providers it managed
/// to admit. Called after broker reconciliation, because a credential that
/// resolves from configuration can still fail to canonicalise at the broker.
///
/// A company whose *primary* model has no admitted provider cannot think, so it
/// cannot start. That is this company's fact, not the plane's. An unadmitted
/// *failover* candidate is not fatal — the chain is a fallback, not a
/// requirement — so it is dropped with a warning and the company still starts
/// on the route it does have.
fn admit(
    configs: &[CompanyConfig],
    credentials: &BTreeMap<String, ProviderCredential>,
) -> Result<Admission> {
    let mut unstartable = BTreeMap::<String, String>::new();
    for config in configs {
        let (primary_provider, _) = split_model(&config.model)?;
        if !credentials.contains_key(primary_provider) {
            unstartable.insert(
                config.name.clone(),
                format!(
                    "no usable host credential for model {}; set credentials.model.inference.{primary_provider}",
                    config.model
                ),
            );
            continue;
        }
        for model in config.model_candidates()?.into_iter().skip(1) {
            let (provider, _) = split_model(model)?;
            if !credentials.contains_key(provider) {
                tracing::warn!(
                    company = %config.name,
                    model,
                    "dropping failover candidate: no usable host credential for provider {provider}"
                );
            }
        }
    }
    Ok(Admission { unstartable })
}

fn split_model(model: &str) -> Result<(&str, &str)> {
    let (provider, id) = model.split_once('/').with_context(|| {
        format!("model {model} must be provider-qualified, e.g. moonshot/kimi-k3")
    })?;
    if provider.is_empty() || id.is_empty() {
        bail!("model {model} must contain a provider and model id");
    }
    Ok((provider, id))
}

async fn token(
    root: &Path,
    omp: &Path,
    profile: &str,
    command: &str,
    discover_llama_cpp: bool,
) -> Result<String> {
    let args = [command, "token"];
    let mut token_command = Command::new(omp);
    if !discover_llama_cpp {
        token_command.env("LLAMA_CPP_BASE_URL", DISABLED_LOCAL_DISCOVERY_URL);
    }
    let child = token_command
        .current_dir(root)
        .env("OMP_PROFILE", profile)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .with_context(|| format!("start OMP {command} token command"))?;
    let pid = child.id().context("OMP token command has no pid")?;
    record_model_child(root, &child, omp, &args)?;
    let output = tokio::time::timeout(Duration::from_secs(10), child.wait_with_output()).await;
    forget_model_child(root, pid)?;
    let output = output
        .with_context(|| format!("OMP {command} token command exceeded 10 seconds"))?
        .with_context(|| format!("create OMP {command} bearer"))?;
    if !output.status.success() {
        bail!("OMP {command} token command failed");
    }
    let value = String::from_utf8(output.stdout)
        .context("OMP bearer was not UTF-8")?
        .trim()
        .to_string();
    if value.len() < 32 || value.bytes().any(|byte| byte.is_ascii_whitespace()) {
        bail!("OMP {command} returned an invalid bearer");
    }
    Ok(value)
}

async fn wait_for_broker(child: &mut Child, token: &str, broker_url: &str) -> Result<()> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(1))
        .build()?;
    for _ in 0..100 {
        if let Some(status) = child.try_wait().context("inspect OMP broker")? {
            bail!("OMP model credential broker exited during boot ({status})");
        }
        if http
            .get(format!("{broker_url}/v1/healthz"))
            .bearer_auth(token)
            .send()
            .await
            .is_ok_and(|response| response.status().is_success())
        {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    bail!("OMP model credential broker did not become ready")
}

#[derive(Deserialize)]
struct ModelList {
    data: Vec<ModelRow>,
}

#[derive(Deserialize)]
struct ModelRow {
    id: String,
}

fn missing_required_models(
    required: &BTreeSet<String>,
    observed: &BTreeSet<String>,
) -> Vec<String> {
    required.difference(observed).cloned().collect()
}

fn runtime_pinned_models(configs: &[&CompanyConfig]) -> Result<BTreeSet<String>> {
    let mut required = BTreeSet::new();
    for config in configs {
        for model in config.model_candidates()? {
            let (provider, _) = split_model(model)?;
            // OpenAI-compatible catalogues are provider-discovered. They are
            // also the models Restless must pin into the credential-free
            // Runtime config, so seeing some other model from the same
            // provider is not readiness for this exact launch contract.
            if matches!(provider, "litellm" | "zhipu-coding-plan") {
                required.insert(model.to_string());
            }
        }
    }
    Ok(required)
}

async fn wait_for_gateway(
    child: &mut Child,
    token: &str,
    gateway_host_url: &str,
    required_providers: &BTreeSet<String>,
    required_models: &BTreeSet<String>,
) -> Result<()> {
    let http = reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()?;
    let mut observed = BTreeSet::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    while tokio::time::Instant::now() < deadline {
        if let Some(status) = child.try_wait().context("inspect OMP gateway")? {
            bail!("OMP model auth gateway exited during boot ({status})");
        }
        if let Ok(response) = http
            .get(format!("{gateway_host_url}/v1/models"))
            .bearer_auth(token)
            .send()
            .await
        {
            if response.status().is_success() {
                if let Ok(list) = response.json::<ModelList>().await {
                    observed = list.data.into_iter().map(|model| model.id).collect();
                    let observed_providers = observed
                        .iter()
                        .filter_map(|model| model.split_once('/').map(|(provider, _)| provider))
                        .map(str::to_string)
                        .collect::<BTreeSet<_>>();
                    if required_providers.is_subset(&observed_providers)
                        && missing_required_models(required_models, &observed).is_empty()
                    {
                        return Ok(());
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let observed_providers = observed
        .iter()
        .filter_map(|model| model.split_once('/').map(|(provider, _)| provider))
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    let missing_providers = required_providers
        .difference(&observed_providers)
        .cloned()
        .collect::<Vec<_>>();
    let missing = missing_required_models(required_models, &observed);
    bail!(
        "OMP model auth gateway did not expose configured providers {:?} or pinned models {:?} (observed {:?})",
        missing_providers,
        missing,
        observed
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_root() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "restless-model-relay-test-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(root.join("companies")).unwrap();
        CompanyConfig::save(
            &root,
            &CompanyConfig {
                name: "acme_test".into(),
                mission: "relay test".into(),
                spend_ceiling_usd: crate::runtime::SpendCeiling::from_micro_usd(2),
                outcome_standard: Default::default(),
                model: "moonshot/kimi-k3".into(),
                worker_runtime: crate::runtime::WorkerRuntime::Omp,
                reasoning_effort: crate::acp::DEFAULT_REASONING_EFFORT.into(),
                model_failover: Vec::new(),
                credentials: BTreeMap::new(),
                approved_parties: Vec::new(),
            },
        )
        .unwrap();
        root
    }

    fn test_relay_state(
        root: &std::path::Path,
    ) -> (
        crate::capability::CapabilityIssuer,
        crate::spend::SpendLedger,
        RelayState,
    ) {
        let capabilities = crate::capability::CapabilityIssuer::open(root).unwrap();
        let spend = crate::spend::SpendLedger::open(root).unwrap();
        let state = RelayState {
            root: root.to_path_buf(),
            capabilities: capabilities.clone(),
            spend: spend.clone(),
            upstream_token: "host-only-root-bearer".into(),
            upstream_url: OMP_GATEWAY_HOST_URL.into(),
            responses_routes: BTreeMap::new(),
            http: reqwest::Client::new(),
        };
        (capabilities, spend, state)
    }

    fn bearer_headers(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
        headers
    }

    #[test]
    fn orphan_sweep_accepts_only_restless_owned_omp_shapes() {
        let allowed = [
            vec!["auth-broker".into(), "token".into()],
            vec![
                "auth-broker".into(),
                "serve".into(),
                "--bind".into(),
                "127.0.0.1:7789".into(),
            ],
            vec!["auth-gateway".into(), "token".into()],
            vec![
                "auth-gateway".into(),
                "serve".into(),
                "--bind".into(),
                "127.0.0.1:7796".into(),
            ],
        ];
        for args in allowed {
            assert!(allowed_model_child_args(&args));
        }

        for rejected in [
            vec!["auth-broker".into(), "delete".into()],
            vec!["auth-gateway".into(), "serve".into()],
            vec!["auth-gateway".into(), "serve".into(), "--all".into()],
            vec!["unrelated".into(), "token".into()],
        ] {
            assert!(!allowed_model_child_args(&rejected));
        }
    }

    fn metered_request(company: &str, actor: &str, session: &str, model: &str) -> MeteredRequest {
        MeteredRequest {
            request_id: uuid::Uuid::new_v4(),
            company: company.into(),
            actor: actor.into(),
            session: session.into(),
            responsibility: "test:model-relay".into(),
            work_id: None,
            attempt_id: None,
            model: model.into(),
            billing: ModelBilling::MeteredApi,
        }
    }

    #[test]
    fn runtime_model_config_contains_only_the_narrow_gateway_route() {
        let config =
            models_config("moonshot/kimi-k3", RELAY_RUNTIME_URL, MODEL_CAPABILITY_ENV).unwrap();
        assert!(config.contains("\n  moonshot:\n    baseUrl:"));
        assert!(config.contains("transport: pi-native"));
        assert!(config.contains("api: openai-completions"));
        assert!(config.contains("apiKey: RESTLESS_MODEL_CAPABILITY"));
        assert!(!config.contains("MOONSHOT_API_KEY"));
        assert!(!config.contains("api.kimi.com"));
    }

    #[test]
    fn runtime_relay_preflight_rejects_a_loopback_alias_collision() {
        let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = occupied.local_addr().unwrap().to_string();
        let error = preflight_runtime_relay_port(&address).unwrap_err();
        assert!(error.to_string().contains("RESTLESS_PORT_OFFSET"));

        drop(occupied);
        preflight_runtime_relay_port(&address).unwrap();
    }

    #[test]
    fn provider_and_route_are_not_open_ended_injection_points() {
        assert!(models_config(
            "moonshot\nheaders/kimi-k3",
            RELAY_RUNTIME_URL,
            MODEL_CAPABILITY_ENV
        )
        .is_err());
        assert!(models_config(
            "moonshot/kimi-k3",
            "https://example.invalid",
            MODEL_CAPABILITY_ENV
        )
        .is_err());
        assert!(models_config(
            "moonshot/kimi-k3",
            "http://host.docker.internal:17790/v1",
            MODEL_CAPABILITY_ENV
        )
        .is_err());
        assert!(models_config(
            "moonshot/kimi-k3",
            "http://host.docker.internal:17790",
            MODEL_CAPABILITY_ENV
        )
        .is_ok());
    }

    #[test]
    fn hosted_runtime_uses_the_exact_https_plane_relay_while_local_stays_unchanged() {
        assert_eq!(
            runtime_relay_url(Some("local"), None, 7790).unwrap(),
            RELAY_RUNTIME_URL
        );

        let hosted =
            runtime_relay_url(Some("network"), Some("owner-1.planes.example.test"), 7790).unwrap();
        assert_eq!(
            hosted,
            "https://owner-1.planes.example.test/internal/v1/model"
        );
        assert!(recognised_runtime_relay(
            &hosted,
            Some("network"),
            Some("owner-1.planes.example.test")
        ));

        for refused in [
            "http://owner-1.planes.example.test/internal/v1/model",
            "https://other.planes.example.test/internal/v1/model",
            "https://owner-1.planes.example.test/internal/v1/model?token=leak",
            "https://owner-1.planes.example.test:8443/internal/v1/model",
            "https://owner-1.planes.example.test/internal/v1/model/extra",
        ] {
            assert!(!recognised_runtime_relay(
                refused,
                Some("network"),
                Some("owner-1.planes.example.test")
            ));
        }
        assert!(runtime_relay_url(Some("network"), Some("owner.example.test/path"), 7790).is_err());
    }

    #[test]
    fn runtime_model_config_discovers_only_through_the_scoped_relay() {
        let config = models_config(
            "litellm/gpt-5.6-sol",
            RELAY_RUNTIME_URL,
            MODEL_CAPABILITY_ENV,
        )
        .unwrap();
        assert!(config.contains("\n  litellm:\n    baseUrl:"));
        assert!(config.contains("    discovery:\n      type: proxy\n"));
        assert!(!config.contains("    models:\n"));
        assert!(!config.contains("GPT_API_KEY"));
        assert!(!config.contains("GPT_BASE_URL"));
    }

    #[test]
    fn runtime_pins_an_admitted_model_missing_from_the_bundled_catalogue() {
        let config =
            models_config("zai/glm-5.3-flash", RELAY_RUNTIME_URL, MODEL_CAPABILITY_ENV).unwrap();
        assert!(config.contains("    discovery:\n      type: proxy\n"));
        assert!(config.contains("    modelOverrides:\n      glm-5.3-flash:\n"));
        assert!(config.contains("        input: [text, image]\n"));
        assert!(!config.contains("    models:\n"));
        assert!(config.contains("    api: openai-completions\n"));
        assert!(!config.contains("ZAI_API_KEY"));
        assert!(!config.contains("ZAI_BASE_URL"));
    }

    #[test]
    fn runtime_pins_glm_flash_on_the_bigmodel_coding_route() {
        let config = models_config(
            "zhipu-coding-plan/glm-5.3-flash",
            RELAY_RUNTIME_URL,
            MODEL_CAPABILITY_ENV,
        )
        .unwrap();
        assert!(config.contains("\n  zhipu-coding-plan:\n    baseUrl:"));
        assert!(config.contains("    discovery:\n      type: proxy\n"));
        assert!(config.contains("    modelOverrides:\n      glm-5.3-flash:\n"));
        assert!(config.contains("    api: openai-completions\n"));
        assert!(!config.contains("ZAI_API_KEY"));
    }

    #[tokio::test]
    async fn relay_catalogue_exposes_only_the_capability_model() {
        use axum::body::to_bytes;

        let root = test_root();
        let (issuer, _spend, state) = test_relay_state(&root);
        let token = issuer
            .issue_model_session(
                "acme_test",
                "delivery-lead",
                "session_123",
                "moonshot",
                "moonshot/kimi-k3",
                "metered_api",
                "work:delivery",
                None,
                None,
            )
            .unwrap();
        let response = relay_models(State(state), bearer_headers(&token)).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let catalogue: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(catalogue["data"].as_array().unwrap().len(), 1);
        assert_eq!(catalogue["data"][0]["id"], "kimi-k3");
        assert_eq!(catalogue["data"][0]["owned_by"], "moonshot");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn gateway_readiness_requires_every_exact_configured_model() {
        let required = BTreeSet::from([
            "litellm/gpt-5.6-sol".to_string(),
            "litellm/gpt-5.6-terra".to_string(),
        ]);
        let observed = BTreeSet::from(["litellm/gpt-5.6-terra".to_string()]);
        assert_eq!(
            missing_required_models(&required, &observed),
            vec!["litellm/gpt-5.6-sol".to_string()]
        );
    }

    #[test]
    fn dynamic_gateway_snapshot_is_company_and_primary_model_scoped() {
        let client = ClientConfig::empty();
        client
            .replace(Some(GatewayClientSnapshot {
                providers: BTreeMap::from([("moonshot".into(), ModelBilling::MeteredApi)]),
                runtime_url: "http://host.docker.internal:7790".into(),
                primary_models: BTreeMap::from([("acme_test".into(), "moonshot/kimi-k3".into())]),
                admitted_models: BTreeMap::from([(
                    "acme_test".into(),
                    BTreeSet::from(["moonshot/kimi-k3".into()]),
                )]),
            }))
            .unwrap();

        assert!(client.company_primary_model_is_admitted("acme_test", "moonshot/kimi-k3"));
        assert!(!client.company_primary_model_is_admitted("acme_test", "moonshot/kimi-k2"));
        assert!(!client.company_primary_model_is_admitted("other_test", "moonshot/kimi-k3"));
        assert!(client.company_model_route_is_admitted("acme_test", "moonshot/kimi-k2"));
        assert!(!client.company_model_route_is_admitted("acme_test", "anthropic/claude-sonnet-4-6"));
    }

    #[test]
    fn hosted_admission_identity_rejects_model_and_credential_drift() {
        let root = test_root();
        let base = CompanyConfig::load(&root, "acme_test").unwrap();
        let mut model_drift = base.clone();
        model_drift.model = "moonshot/kimi-k2".into();
        assert!(!same_gateway_policy(&base, &model_drift));

        let mut credential_drift = base.clone();
        credential_drift.credentials.insert(
            "model.inference.moonshot".into(),
            "infisical:/providers/moonshot/MOONSHOT_API_KEY".into(),
        );
        assert!(!same_gateway_policy(&base, &credential_drift));
        assert!(same_gateway_policy(&base, &base));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn zero_company_supervisor_channel_delivers_exact_admission_result() {
        let root = test_root();
        let config = CompanyConfig::load(&root, "acme_test").unwrap();
        let (admission, mut requests) = hosted_admission_channel();
        let pending = tokio::spawn({
            let config = config.clone();
            async move { admission.admit(config).await }
        });
        let request = requests.receiver.recv().await.unwrap();
        assert!(same_gateway_policy(&request.config, &config));
        request
            .response
            .send(HostedAdmissionOutcome::Admitted)
            .unwrap();
        assert_eq!(pending.await.unwrap(), HostedAdmissionOutcome::Admitted);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn hosted_reconciler_refuses_same_company_model_drift_before_process_changes() {
        let root = test_root();
        let base = CompanyConfig::load(&root, "acme_test").unwrap();
        let mut drifted = base.clone();
        drifted.model = "moonshot/kimi-k2".into();
        let (issuer, spend, relay) = test_relay_state(&root);
        let mut processes = None;
        let outcome = reconcile_hosted_admission_inner(
            &mut vec![base],
            &mut processes,
            drifted,
            &root,
            &issuer,
            &spend,
        )
        .await;
        assert_eq!(outcome, HostedAdmissionOutcome::IdentityDrift);
        drop(relay);
        drop(spend);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn first_hosted_company_without_a_credential_is_not_false_ready() {
        let root = test_root();
        let config = CompanyConfig::load(&root, "acme_test").unwrap();
        let (issuer, spend, relay) = test_relay_state(&root);
        let mut configs = Vec::new();
        let mut processes = None;
        let outcome = reconcile_hosted_admission_inner(
            &mut configs,
            &mut processes,
            config,
            &root,
            &issuer,
            &spend,
        )
        .await;
        assert_eq!(outcome, HostedAdmissionOutcome::Unstartable);
        assert!(configs.is_empty());
        drop(relay);
        drop(spend);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn exact_readiness_is_reserved_for_runtime_pinned_catalogues() {
        let config = CompanyConfig {
            name: "catalogue_test".into(),
            mission: String::new(),
            spend_ceiling_usd: crate::runtime::SpendCeiling::from_micro_usd(2),
            outcome_standard: Default::default(),
            model: "litellm/gpt-5.6-sol".into(),
            worker_runtime: crate::runtime::WorkerRuntime::Omp,
            reasoning_effort: crate::acp::DEFAULT_REASONING_EFFORT.into(),
            model_failover: vec!["zai/glm-5.3-flash".into()],
            credentials: BTreeMap::new(),
            approved_parties: Vec::new(),
        };
        assert_eq!(
            runtime_pinned_models(&[&config]).unwrap(),
            BTreeSet::from(["litellm/gpt-5.6-sol".to_string()])
        );
    }

    /// Cross-layer contract §1.4.1: one company's configuration must never
    /// prevent another company from starting. This is the invariant that
    /// distinguishes the account plane from the cells it serves — a plane that
    /// refuses to boot for one bad config has fused the two tiers.
    #[test]
    fn one_companys_unroutable_model_does_not_stop_the_others() {
        let company = |name: &str, model: &str, failover: Vec<String>| CompanyConfig {
            name: name.into(),
            mission: String::new(),
            spend_ceiling_usd: crate::runtime::SpendCeiling::from_micro_usd(2),
            outcome_standard: Default::default(),
            model: model.into(),
            worker_runtime: crate::runtime::WorkerRuntime::Omp,
            reasoning_effort: crate::acp::DEFAULT_REASONING_EFFORT.into(),
            model_failover: failover,
            credentials: BTreeMap::new(),
            approved_parties: Vec::new(),
        };
        let configs = vec![
            // Primary resolves; one failover candidate does not.
            company(
                "healthy_test",
                "zai/glm-5.3",
                vec!["anthropic/claude-haiku-4-5".into()],
            ),
            // Primary does not resolve at all.
            company("broken_test", "openai-codex/gpt-5.6-sol", Vec::new()),
        ];
        let credentials =
            BTreeMap::from([("zai".to_string(), ProviderCredential::ApiKey("k".into()))]);

        let admission = admit(&configs, &credentials).unwrap();

        // Only the company whose primary route is unusable is held back, and
        // the reason names the exact missing configuration key.
        assert_eq!(
            admission.unstartable.keys().collect::<Vec<_>>(),
            vec!["broken_test"]
        );
        assert!(
            admission.unstartable["broken_test"]
                .contains("credentials.model.inference.openai-codex"),
            "reason must name the exact key to set, got {:?}",
            admission.unstartable["broken_test"]
        );
        // A dead failover candidate is not fatal: the chain is a fallback, not
        // a requirement, so the company still starts on the route it has.
        assert!(!admission.unstartable.contains_key("healthy_test"));
        assert_eq!(
            admission
                .startable(&configs)
                .iter()
                .map(|config| config.name.as_str())
                .collect::<Vec<_>>(),
            vec!["healthy_test"]
        );
    }

    #[test]
    fn agent_model_access_is_a_signed_company_actor_session_exact_model_grant() {
        let root = test_root();
        let (issuer, _spend, _state) = test_relay_state(&root);
        let client = ClientConfig::empty();
        client
            .replace(Some(GatewayClientSnapshot {
                providers: BTreeMap::from([("moonshot".into(), ModelBilling::MeteredApi)]),
                runtime_url: RELAY_RUNTIME_URL.into(),
                primary_models: BTreeMap::from([("acme_test".into(), "moonshot/kimi-k3".into())]),
                admitted_models: BTreeMap::from([(
                    "acme_test".into(),
                    BTreeSet::from(["moonshot/kimi-k3".into()]),
                )]),
            }))
            .unwrap();
        let work_id = uuid::Uuid::new_v4();
        let attempt_id = uuid::Uuid::new_v4();
        let access = client
            .auth_for(
                "moonshot/kimi-k3",
                &issuer,
                "acme_test",
                "delivery-lead",
                "session_123",
                "work:delivery",
                Some(work_id),
                Some(attempt_id),
            )
            .unwrap();
        assert_eq!(access.token_env, MODEL_CAPABILITY_ENV);
        assert_eq!(access.runtime_url, RELAY_RUNTIME_URL);
        assert_eq!(
            issuer.verify_model(&access.token).unwrap(),
            crate::capability::ModelGrant {
                company: "acme_test".into(),
                actor: "delivery-lead".into(),
                session: "session_123".into(),
                provider: "moonshot".into(),
                model: "moonshot/kimi-k3".into(),
                billing: "metered_api".into(),
                responsibility: "work:delivery".into(),
                work_id: Some(work_id),
                attempt_id: Some(attempt_id),
            }
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn relay_refuses_a_provider_mismatch_and_exhausted_company_before_forwarding() {
        let root = test_root();
        let (issuer, spend, state) = test_relay_state(&root);
        let token = issuer
            .issue_model_session(
                "acme_test",
                "delivery-lead",
                "session_123",
                "moonshot",
                "moonshot/kimi-k3",
                "metered_api",
                "work:delivery",
                None,
                None,
            )
            .unwrap();
        let mismatched = relay_pi_stream(
            State(state.clone()),
            bearer_headers(&token),
            Bytes::from_static(br#"{"modelId":"openai/gpt-5"}"#),
        )
        .await;
        assert_eq!(mismatched.status(), StatusCode::FORBIDDEN);

        let same_provider_wrong_model = relay_pi_stream(
            State(state.clone()),
            bearer_headers(&token),
            Bytes::from_static(br#"{"modelId":"moonshot/another-model"}"#),
        )
        .await;
        assert_eq!(same_provider_wrong_model.status(), StatusCode::FORBIDDEN);

        spend.meter().record_exact(
            metered_request(
                "acme_test",
                "delivery-lead",
                "prior_session",
                "moonshot/kimi-k3",
            )
            .spend_record(
                0,
                0,
                0,
                None,
                2,
                restless_model_gateway::SpendSettlement::Accounted,
            ),
        );
        let exhausted = relay_pi_stream(
            State(state),
            bearer_headers(&token),
            Bytes::from_static(br#"{"modelId":"moonshot/kimi-k3"}"#),
        )
        .await;
        assert_eq!(exhausted.status(), StatusCode::PAYMENT_REQUIRED);

        drop(spend);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relay_records_one_exact_terminal_charge_and_poison_missing_terminal_usage() {
        let root = test_root();
        let (_issuer, ledger, _state) = test_relay_state(&root);
        let event = serde_json::json!({
            "type": "done",
            "message": {
                "usage": {
                    "input": 3,
                    "output": 5,
                    "cost": { "total": 0.000001 }
                }
            }
        });
        {
            let inner = futures_util::stream::empty::<std::result::Result<Bytes, reqwest::Error>>();
            let mut stream = MeteredStream::new(
                inner,
                ledger.meter(),
                metered_request(
                    "acme_test",
                    "delivery-lead",
                    "session_123",
                    "moonshot/kimi-k3",
                ),
            );
            let frame = Bytes::from(format!("data: {event}\n\n"));
            stream.observe(&frame);
            stream.observe(&frame);
        }
        assert_eq!(
            ledger
                .budget_state_for("acme_test", crate::runtime::SpendCeiling::from_micro_usd(2))
                .remaining_micro_usd(),
            Some(1),
            "one terminal event writes one exact micro-USD record"
        );
        // Each cell keeps its own ledger; there is no installation-wide spool
        // to read (cross-layer contract §1.4).
        let spool =
            std::fs::read_to_string(root.join("cells/acme_test/spend/spend.jsonl")).unwrap();
        let records = spool
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["actorId"], "delivery-lead");
        assert_eq!(records[0]["sessionId"], "session_123");
        assert_eq!(records[0]["totalTokens"], 8);
        assert_eq!(records[0]["costMicroUsd"], 1);

        {
            let inner = futures_util::stream::empty::<std::result::Result<Bytes, reqwest::Error>>();
            let _missing_terminal = MeteredStream::new(
                inner,
                ledger.meter(),
                metered_request(
                    "acme_test",
                    "delivery-lead",
                    "session_missing_usage",
                    "moonshot/kimi-k3",
                ),
            );
        }
        assert_eq!(
            ledger
                .budget_state_for("acme_test", crate::runtime::SpendCeiling::from_micro_usd(2))
                .remaining_micro_usd(),
            None,
            "a metered stream without terminal charged usage leaves metering unknown and blocks further charged requests"
        );

        drop(ledger);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn runtime_disconnect_does_not_abandon_terminal_metering() {
        let root = test_root();
        let (_issuer, ledger, _state) = test_relay_state(&root);
        let event = serde_json::json!({
            "type": "done",
            "message": {
                "usage": {
                    "input": 11,
                    "output": 7,
                    "cost": { "total": 0.000001 }
                }
            }
        });
        let frame = Bytes::from(format!("data: {event}\n\n"));
        let upstream = futures_util::stream::iter(vec![Ok(frame)]);
        let metered = MeteredStream::new(
            upstream,
            ledger.meter(),
            metered_request(
                "cancelled_test",
                "support-owner",
                "session_cancelled",
                "zai/glm-5.3",
            ),
        );
        let (downstream, drain) = detach_metered_stream(metered);

        // ACP cancellation closes this response body. The host still owns the
        // upstream request and must consume its terminal accounting event.
        drop(downstream);
        drain.await.unwrap();

        assert_eq!(
            ledger
                .budget_state_for(
                    "cancelled_test",
                    crate::runtime::SpendCeiling::from_micro_usd(2),
                )
                .remaining_micro_usd(),
            Some(1),
        );
        // Each cell keeps its own ledger; there is no installation-wide spool
        // to read (cross-layer contract §1.4).
        let spool =
            std::fs::read_to_string(root.join("cells/cancelled_test/spend/spend.jsonl")).unwrap();
        let record: serde_json::Value = serde_json::from_str(spool.trim()).unwrap();
        assert_eq!(record["sessionId"], "session_cancelled");
        assert_eq!(record["totalTokens"], 18);
        assert_eq!(record["costMicroUsd"], 1);

        drop(ledger);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relay_accepts_an_explicitly_accounted_provider_error_but_not_an_ambiguous_one() {
        let root = test_root();
        let (_issuer, ledger, _state) = test_relay_state(&root);
        let explicit_zero_cost_error = serde_json::json!({
            "type": "error",
            "reason": "error",
            "error": {
                "usage": {
                    "input": 0,
                    "output": 0,
                    "cost": { "total": 0 }
                }
            }
        });
        {
            let inner = futures_util::stream::empty::<std::result::Result<Bytes, reqwest::Error>>();
            let mut stream = MeteredStream::new(
                inner,
                ledger.meter(),
                metered_request(
                    "acme_test",
                    "delivery-lead",
                    "provider_error_with_zero_cost",
                    "moonshot/kimi-k3",
                ),
            );
            stream.observe(&Bytes::from(format!(
                "data: {explicit_zero_cost_error}\n\n"
            )));
        }
        assert_eq!(
            ledger
                .budget_state_for("acme_test", crate::runtime::SpendCeiling::from_micro_usd(2))
                .remaining_micro_usd(),
            Some(2),
            "an error with an exact $0 usage envelope does not create unknown spend"
        );

        {
            let inner = futures_util::stream::empty::<std::result::Result<Bytes, reqwest::Error>>();
            let mut stream = MeteredStream::new(
                inner,
                ledger.meter(),
                metered_request(
                    "acme_test",
                    "delivery-lead",
                    "ambiguous_provider_error",
                    "moonshot/kimi-k3",
                ),
            );
            stream.observe(&Bytes::from_static(
                b"data: {\"type\":\"error\",\"reason\":\"error\"}\n\n",
            ));
        }
        assert_eq!(
            ledger
                .budget_state_for("acme_test", crate::runtime::SpendCeiling::from_micro_usd(2))
                .remaining_micro_usd(),
            None,
            "an error with no exact usage remains fail-closed without a fake zero balance"
        );

        drop(ledger);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relay_poisoned_when_only_a_partial_and_wire_cleanup_arrive() {
        let root = test_root();
        let (_issuer, ledger, _state) = test_relay_state(&root);
        {
            let inner = futures_util::stream::empty::<std::result::Result<Bytes, reqwest::Error>>();
            let mut stream = MeteredStream::new(
                inner,
                ledger.meter(),
                metered_request(
                    "acme_test",
                    "delivery-lead",
                    "tool_use_partial",
                    "moonshot/kimi-k3",
                ),
            );
            let partial = serde_json::json!({
                "type": "toolcall_end",
                "partial": {
                    "stopReason": "toolUse",
                    "usage": {
                        "input": 3,
                        "output": 5,
                        "cost": { "total": 0.000001 }
                    }
                }
            });
            stream.observe(&Bytes::from(format!("data: {partial}\n\n")));
            assert_eq!(
                ledger
                    .budget_state_for("acme_test", crate::runtime::SpendCeiling::from_micro_usd(2))
                    .remaining_micro_usd(),
                Some(2),
                "a partial is not a provider-confirmed terminal usage record"
            );
            stream.observe(&Bytes::from_static(b"data: [DONE]\n\n"));
        }
        assert_eq!(
            ledger
                .budget_state_for("acme_test", crate::runtime::SpendCeiling::from_micro_usd(2))
                .remaining_micro_usd(),
            None,
            "a bare wire sentinel cannot turn a partial into accounted provider spend"
        );

        drop(ledger);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relay_flushes_a_valid_terminal_frame_that_arrives_without_a_final_blank_line() {
        let root = test_root();
        let (_issuer, ledger, _state) = test_relay_state(&root);
        let event = serde_json::json!({
            "type": "done",
            "message": {
                "usage": {
                    "input": 3,
                    "output": 5,
                    "cost": { "total": 0.000001 }
                }
            }
        });
        {
            let inner = futures_util::stream::empty::<std::result::Result<Bytes, reqwest::Error>>();
            let mut stream = MeteredStream::new(
                inner,
                ledger.meter(),
                metered_request(
                    "acme_test",
                    "delivery-lead",
                    "trailing_terminal",
                    "moonshot/kimi-k3",
                ),
            );
            stream.observe(&Bytes::from(format!("data: {event}\n")));
            assert_eq!(
                ledger
                    .budget_state_for("acme_test", crate::runtime::SpendCeiling::from_micro_usd(2))
                    .remaining_micro_usd(),
                Some(2),
                "a trailing frame is held until EOF"
            );
            stream.finish();
        }
        assert_eq!(
            ledger
                .budget_state_for("acme_test", crate::runtime::SpendCeiling::from_micro_usd(2))
                .remaining_micro_usd(),
            Some(1),
            "a valid trailing terminal frame records its exact cost"
        );

        drop(ledger);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn micro_usd_parser_preserves_or_conservatively_rounds_provider_decimals() {
        assert_eq!(
            ceiling_micro_usd(&serde_json::from_str("0.000001").unwrap()),
            Some(1)
        );
        assert_eq!(
            ceiling_micro_usd(&serde_json::from_str("12.34").unwrap()),
            Some(12_340_000)
        );
        assert_eq!(
            ceiling_micro_usd(&serde_json::from_str("0.0000001").unwrap()),
            Some(1),
            "never round real provider spend down below one ledger unit"
        );
        assert_eq!(
            ceiling_micro_usd(&serde_json::from_str("0.01104072").unwrap()),
            Some(11_041),
            "GLM's 8-place dollar prices are charged upward by less than one micro-dollar"
        );
        assert_eq!(
            ceiling_micro_usd(&serde_json::from_str("1e-6").unwrap()),
            Some(1)
        );
        assert_eq!(
            ceiling_micro_usd(&serde_json::from_str("1e-7").unwrap()),
            Some(1)
        );
        assert!(ceiling_micro_usd(&serde_json::from_str("-1").unwrap()).is_none());
    }

    #[test]
    fn responses_tariff_is_exact_cached_aware_and_conservative() {
        assert_eq!(
            response_tariff_micro_usd("gpt-5.6-sol", 1_000, 100, 100),
            Some(7_550)
        );
        assert_eq!(
            response_tariff_micro_usd("gpt-5.6-terra", 1, 0, 1),
            Some(1),
            "a positive sub-micro-dollar charge rounds upward"
        );
        assert_eq!(
            response_tariff_micro_usd("gpt-5.6-sol", 272_001, 1, 0),
            Some(2_720_055),
            "the pinned long-context tier starts above 272K input tokens"
        );
        assert!(response_tariff_micro_usd("gpt-5.6-sol", 2, 0, 3).is_none());
        assert!(response_tariff_micro_usd("unknown", 1, 1, 0).is_none());
    }

    #[test]
    fn responses_terminal_records_pinned_cost_and_tokens_once() {
        let root = test_root();
        let (_issuer, ledger, _state) = test_relay_state(&root);
        let event = serde_json::json!({
            "type": "response.completed",
            "response": {
                "usage": {
                    "input_tokens": 1_000,
                    "output_tokens": 100,
                    "total_tokens": 1_100,
                    "input_tokens_details": { "cached_tokens": 100 }
                }
            }
        });
        {
            let inner = futures_util::stream::empty::<std::result::Result<Bytes, reqwest::Error>>();
            let mut stream = MeteredStream::new_responses(
                inner,
                ledger.meter(),
                metered_request(
                    "acme_test",
                    "codex-worker",
                    "responses-session",
                    "litellm/gpt-5.6-sol",
                ),
            );
            let frame = Bytes::from(format!("data: {event}\n\n"));
            stream.observe(&frame);
            stream.observe(&frame);
        }
        let spool =
            std::fs::read_to_string(root.join("cells/acme_test/spend/spend.jsonl")).unwrap();
        let records = spool
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["totalTokens"], 1_100);
        assert_eq!(records[0]["costMicroUsd"], 7_550);

        drop(ledger);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relay_charges_a_glm_precision_terminal_without_poisoning() {
        let root = test_root();
        let (_issuer, ledger, _state) = test_relay_state(&root);
        let event = serde_json::json!({
            "type": "done",
            "message": {
                "usage": {
                    "input": 7_568,
                    "output": 71,
                    "cacheRead": 512,
                    "cost": { "total": 0.01104072 }
                }
            }
        });
        {
            let inner = futures_util::stream::empty::<std::result::Result<Bytes, reqwest::Error>>();
            let mut stream = MeteredStream::new(
                inner,
                ledger.meter(),
                metered_request("acme_test", "delivery-lead", "glm_precision", "zai/glm-5.3"),
            );
            stream.observe(&Bytes::from(format!("data: {event}\n\n")));
        }
        assert_eq!(
            ledger
                .budget_state_for(
                    "acme_test",
                    crate::runtime::SpendCeiling::from_micro_usd(12_000)
                )
                .remaining_micro_usd(),
            Some(959),
            "the micro-USD ledger uses a conservative 11,041-micro charge"
        );

        drop(ledger);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn credential_rotation_keeps_only_the_referenced_provider_key() {
        let snapshot: BrokerSnapshot = serde_json::from_value(serde_json::json!({
            "credentials": [
                {"id": 1, "provider": "moonshot", "credential": {"type": "api_key", "key": "old"}},
                {"id": 2, "provider": "moonshot", "credential": {"type": "api_key", "key": "current"}},
                {"id": 3, "provider": "openai", "credential": {"type": "api_key", "key": "unrelated"}}
            ]
        }))
        .unwrap();
        assert_eq!(
            canonical_api_key_credential(&snapshot, "moonshot", "current").unwrap(),
            (2, vec![1])
        );
        assert!(canonical_api_key_credential(&snapshot, "moonshot", "missing").is_err());
    }

    #[test]
    fn oauth_reference_keeps_exactly_one_authenticated_provider_row() {
        let snapshot: BrokerSnapshot = serde_json::from_value(serde_json::json!({
            "credentials": [
                {"id": 7, "provider": "anthropic", "credential": {"type": "oauth", "refresh": "<remote>", "access": "redacted", "expires": 1}},
                {"id": 8, "provider": "anthropic", "credential": {"type": "api_key", "key": "old"}}
            ]
        }))
        .unwrap();
        assert_eq!(
            canonical_oauth_credential(&snapshot, "anthropic").unwrap(),
            (7, vec![8])
        );
    }

    #[test]
    fn one_cooldown_read_drives_exec_and_staff_candidate_order() {
        let config = CompanyConfig {
            name: "continuity_test".into(),
            mission: String::new(),
            spend_ceiling_usd: crate::runtime::SpendCeiling::from_micro_usd(10_000_000),
            outcome_standard: Default::default(),
            model: "moonshot/kimi-k3".into(),
            worker_runtime: crate::runtime::WorkerRuntime::Omp,
            reasoning_effort: crate::acp::DEFAULT_REASONING_EFFORT.into(),
            model_failover: vec!["anthropic/claude-sonnet-4-5".into()],
            credentials: BTreeMap::new(),
            approved_parties: Vec::new(),
        };
        let ordered = ordered_candidates(&config, Some("moonshot/kimi-k3")).unwrap();
        assert_eq!(ordered, ["moonshot/kimi-k3", "anthropic/claude-sonnet-4-5"]);
        let available = filter_cooling_candidates(
            ordered,
            &[crate::authority::ModelCooldown {
                model: "moonshot/kimi-k3".into(),
                kind: "quota".into(),
                reason: "allowance exhausted".into(),
                retry_at: chrono::Utc::now() + chrono::Duration::hours(1),
            }],
        )
        .unwrap();
        assert_eq!(available, ["anthropic/claude-sonnet-4-5"]);
    }

    #[test]
    fn all_cooling_error_names_only_the_candidate_policy() {
        let now = chrono::Utc::now();
        let error = filter_cooling_candidates(
            vec!["zhipu-coding-plan/glm-5.3-flash".into()],
            &[
                crate::authority::ModelCooldown {
                    model: "litellm/gpt-5.6-terra".into(),
                    kind: "quota".into(),
                    reason: "unrelated old route".into(),
                    retry_at: now + chrono::Duration::minutes(10),
                },
                crate::authority::ModelCooldown {
                    model: "zhipu-coding-plan/glm-5.3-flash".into(),
                    kind: "quota".into(),
                    reason: "current exact route".into(),
                    retry_at: now + chrono::Duration::hours(1),
                },
            ],
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("zhipu-coding-plan/glm-5.3-flash"));
        assert!(!error.contains("litellm/gpt-5.6-terra"));
    }

    #[test]
    fn actor_conversation_waits_only_when_its_entire_policy_is_cooling() {
        let cooldown = crate::authority::ModelCooldown {
            model: "zai/glm-5.3".into(),
            kind: "quota".into(),
            reason: "allowance exhausted".into(),
            retry_at: chrono::Utc::now() + chrono::Duration::minutes(10),
        };
        assert!(candidates_all_cooling(
            &["zai/glm-5.3".into()],
            std::slice::from_ref(&cooldown)
        ));
        assert!(!candidates_all_cooling(
            &["zai/glm-5.3".into(), "anthropic/claude-sonnet-4-6".into()],
            &[cooldown]
        ));
    }

    #[test]
    fn explicit_staff_model_never_inherits_the_exec_fallback_chain() {
        let config = CompanyConfig {
            name: "exact_staff_test".into(),
            mission: String::new(),
            spend_ceiling_usd: crate::runtime::SpendCeiling::from_micro_usd(10_000_000),
            outcome_standard: Default::default(),
            model: "openai-codex/gpt-5.6-sol".into(),
            worker_runtime: crate::runtime::WorkerRuntime::Omp,
            reasoning_effort: crate::acp::DEFAULT_REASONING_EFFORT.into(),
            model_failover: vec!["anthropic/claude-sonnet-4-6".into()],
            credentials: BTreeMap::new(),
            approved_parties: Vec::new(),
        };
        assert_eq!(
            actor_candidates(&config, Some("openai-codex/gpt-5.6-terra")).unwrap(),
            ["openai-codex/gpt-5.6-terra"]
        );
        assert_eq!(
            actor_candidates(&config, None).unwrap(),
            ["openai-codex/gpt-5.6-sol", "anthropic/claude-sonnet-4-6"]
        );
    }
}
