//! Model gateway embedding (sprint 01 T2). restlessd owns the gateway as a
//! library, not a separate process: it holds the provider key host-side,
//! mints short-lived purpose tokens per agent wake, and enforces each
//! company's dollar ceiling. A company container only ever sees a base URL
//! and a ≤1h token — never the provider credential.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};
use chrono::{Duration, Utc};
use restless_model_gateway::{
    CeilingMap, FileAuditSink, FileUsageStore, GatewayConfig, GatewayState, ModelRate,
    PURPOSE_TOKEN_VERSION, PurposeTokenClaims, PurposeTokenCodec, PurposeTokenLimits, SecretBytes,
    SpendRecord, SpendStore, ceiling_map, load_owner_private_secret, router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::runtime::{self, CompanyConfig};

const TOKEN_AUDIENCE: &str = "model-gateway";
/// One agent wake (ARCHITECTURE.md §6: wake/sleep, not daemons-in-containers).
const TOKEN_LIFETIME_MINUTES: i64 = 60;
const TOKEN_MAX_REQUESTS: u32 = 1_024;
const TOKEN_MAX_REQUEST_BYTES: u64 = 16 * 1024 * 1024;
const TOKEN_MAX_RESPONSE_BYTES: u64 = 64 * 1024 * 1024;

/// Installation-owned gateway settings at `$RESTLESS_HOME/gateway.toml`.
/// Seeded with defaults on first boot; the owner edits the file, not code.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatewayFileConfig {
    /// Address the embedded gateway listens on. Containers reach it through
    /// the host gateway alias; the host reaches it on loopback.
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default = "default_upstream_origin")]
    pub upstream_origin: String,
    /// Fixed prefix between origin and the client-facing /v1 path
    /// (https://openrouter.ai + /api + /v1/responses).
    #[serde(default = "default_path_prefix")]
    pub upstream_path_prefix: String,
    /// Adapter model (what agents ask for) → upstream model (what is sent).
    #[serde(default = "default_routes")]
    pub routes: BTreeMap<String, String>,
    /// Per-call output-token ceiling. Providers pre-authorize against
    /// `max_output_tokens`; agent runtimes default high (codex: 64000) and a
    /// key with limited credit 402s every call. 16k covers any turn output
    /// this sprint produces and keeps pre-authorization small.
    #[serde(default = "default_max_output_tokens_cap")]
    pub max_output_tokens_cap: u64,
    /// Price list keyed by upstream model. Approximate by design (T2): the
    /// fuse only needs to bound runaway spend, not reconcile invoices.
    #[serde(default = "default_rates")]
    pub rates: BTreeMap<String, ModelRate>,
}

fn default_bind() -> String {
    "0.0.0.0:7790".to_string()
}
fn default_upstream_origin() -> String {
    "https://openrouter.ai".to_string()
}
fn default_path_prefix() -> String {
    "/api".to_string()
}
fn default_routes() -> BTreeMap<String, String> {
    BTreeMap::new()
}
fn default_max_output_tokens_cap() -> u64 {
    16_384
}
fn default_rates() -> BTreeMap<String, ModelRate> {
    BTreeMap::from([(
        "anthropic/claude-sonnet-4".to_string(),
        ModelRate { input_usd_per_mtok: 3.0, output_usd_per_mtok: 15.0 },
    )])
}

impl Default for GatewayFileConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            upstream_origin: default_upstream_origin(),
            upstream_path_prefix: default_path_prefix(),
            routes: default_routes(),
            max_output_tokens_cap: default_max_output_tokens_cap(),
            rates: default_rates(),
        }
    }
}

/// The running gateway: codec for minting, ceilings for the fuse.
pub struct GatewayHandle {
    codec: PurposeTokenCodec,
    ceilings: CeilingMap,
    /// Port the listener bound, for building base URLs.
    port: u16,
    /// The spend ledger, shared with the HTTP path. The fuse now also runs at
    /// the ACP session layer (see `over_ceiling` / `record_turn`), because the
    /// agent reports its own per-turn cost and the daemon already knows whose
    /// session it is.
    spend: std::sync::Arc<SpendStore>,
}

/// Writes turn costs into the shared spend ledger. Cloneable so supervised
/// staff processes can meter themselves without borrowing the daemon.
#[derive(Clone)]
pub struct TurnMeter {
    spend: std::sync::Arc<SpendStore>,
}

impl TurnMeter {
    /// A turn we cannot account for poisons the company fail-closed, exactly
    /// as an unaccountable HTTP response did: unaccounted spend and unbounded
    /// spend are indistinguishable.
    pub fn record(&self, company: &str, model: &str, used: u64, cost_usd: Option<f64>) {
        let Some(cost_usd) = cost_usd else {
            tracing::error!(
                company,
                used,
                "agent reported usage without a cost; poisoning fail-closed"
            );
            self.spend.poison(company);
            return;
        };
        let record = SpendRecord {
            request_id: Uuid::new_v4(),
            company_id: company.to_owned(),
            model: model.to_owned(),
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: used,
            cost_micro_usd: (cost_usd * 1_000_000.0).round().max(0.0) as u64,
            occurred_at: Utc::now(),
        };
        if self.spend.record(&record).is_err() {
            self.spend.poison(company);
            tracing::error!(company, "turn spend record failed; company poisoned fail-closed");
        }
    }
}

/// A freshly minted purpose token plus how to reach the gateway.
#[derive(Debug, Serialize)]
pub struct MintedToken {
    pub token: String,
    pub expires_at: String,
    /// Base URL from the host (acceptance tests, dogfooding).
    pub base_url_host: String,
    /// Base URL from inside a company container (agent process env).
    pub base_url_container: String,
}

impl GatewayHandle {
    /// Mint a ≤1h purpose token for one company's configured model. This is
    /// the only credential an agent process ever receives (T2, T4).
    pub fn mint_token(&self, company: &CompanyConfig, actor: &str) -> Result<MintedToken> {
        self.refresh_ceiling(company);
        let now = Utc::now();
        let claims = PurposeTokenClaims {
            schema_version: PURPOSE_TOKEN_VERSION,
            token_id: Uuid::new_v4(),
            company_id: company.name.clone(),
            actor_id: actor.to_string(),
            execution_id: Uuid::new_v4(),
            audience: TOKEN_AUDIENCE.to_string(),
            issued_at: now,
            not_before: now,
            expires_at: now + Duration::minutes(TOKEN_LIFETIME_MINUTES),
            allowed_paths: BTreeSet::from([
                "/v1/responses".to_string(),
                "/v1/responses/compact".to_string(),
            ]),
            allowed_models: BTreeSet::from([company.model.clone()]),
            limits: PurposeTokenLimits {
                maximum_requests: TOKEN_MAX_REQUESTS,
                maximum_request_bytes: TOKEN_MAX_REQUEST_BYTES,
                maximum_response_bytes: TOKEN_MAX_RESPONSE_BYTES,
            },
        };
        let token = self
            .codec
            .issue_at(&claims, now)
            .map_err(|error| anyhow::anyhow!("mint purpose token: {error}"))?;
        Ok(MintedToken {
            token,
            expires_at: claims.expires_at.to_rfc3339(),
            // OpenAI-style API roots: codex appends /responses (and /models)
            // to whatever base it is given, so the base must include /v1.
            base_url_host: format!("http://127.0.0.1:{}/v1", self.port),
            base_url_container: format!("http://host.docker.internal:{}/v1", self.port),
        })
    }

    /// Pre-turn fuse check: has this company already spent its ceiling?
    ///
    /// The HTTP path checked before each *request*; metering per turn means
    /// checking before each *turn*, which bounds overshoot to one turn rather
    /// than one request. On a $10 ceiling with turns costing cents that is a
    /// rounding error, and it buys the deletion of the whole proxy path.
    /// Returns (spent, ceiling) in USD when the company must not start.
    #[must_use]
    pub fn over_ceiling(&self, company: &CompanyConfig) -> Option<(f64, f64)> {
        self.refresh_ceiling(company);
        let spent = self.spend.spent_micro_usd(&company.name);
        let ceiling = (company.spend_ceiling_usd * 1_000_000.0).round().max(0.0) as u64;
        (spent >= ceiling)
            .then(|| (spent as f64 / 1_000_000.0, ceiling as f64 / 1_000_000.0))
    }

    /// What this company has spent so far, in USD. The agent is told this so
    /// it can size its own ambition instead of discovering it is broke by
    /// being stopped mid-turn.
    #[must_use]
    pub fn spent_usd(&self, company: &str) -> f64 {
        self.spend.spent_micro_usd(company) as f64 / 1_000_000.0
    }

    /// A cheap cloneable handle to the ledger, for turns that outlive the
    /// borrow — staff run in spawned tasks but spend the same budget.
    #[must_use]
    pub fn meter(&self) -> TurnMeter {
        TurnMeter { spend: std::sync::Arc::clone(&self.spend) }
    }

    /// Record what one turn cost, from the agent's own ACP usage report.
    pub fn record_turn(&self, company: &str, model: &str, used: u64, cost_usd: Option<f64>) {
        self.meter().record(company, model, used, cost_usd);
    }

    /// (Re)load one company's ceiling into the fuse map. Unknown companies
    /// have no entry and are refused — the fuse fails closed by absence.
    pub fn refresh_ceiling(&self, company: &CompanyConfig) {
        let micro = (company.spend_ceiling_usd * 1_000_000.0).round().max(0.0) as u64;
        if let Ok(mut map) = self.ceilings.write() {
            map.insert(company.name.clone(), micro);
        }
    }
}

/// Start the embedded gateway: load or seed state under
/// `$RESTLESS_HOME/gateway/`, then serve until the process exits.
pub async fn start(root: &Path) -> Result<GatewayHandle> {
    let dir = root.join("gateway");
    for sub in ["", "usage", "audit", "spend"] {
        create_private_dir(&dir.join(sub))?;
    }
    let signing_key = load_or_create_signing_key(&dir.join("signing.key"))?;
    let provider_key = load_or_create_provider_key(&dir.join("provider.key"))?;
    let file_config = load_or_seed_config(&dir.join("gateway.toml"))?;

    let codec = PurposeTokenCodec::new(signing_key, TOKEN_AUDIENCE)
        .map_err(|error| anyhow::anyhow!("token codec: {error}"))?;
    let usage = FileUsageStore::new(&dir.join("usage"))
        .map_err(|error| anyhow::anyhow!("usage store: {error}"))?;
    let audit = FileAuditSink::new(&dir.join("audit"))
        .map_err(|error| anyhow::anyhow!("audit sink: {error}"))?;
    let spend = std::sync::Arc::new(
        SpendStore::open(&dir.join("spend"))
            .map_err(|error| anyhow::anyhow!("spend store: {error}"))?,
    );
    let ceilings = ceiling_map();
    seed_ceilings(root, &ceilings);

    let config = GatewayConfig {
        upstream_origin: url::Url::parse(&file_config.upstream_origin)
            .with_context(|| format!("parse upstream_origin {}", file_config.upstream_origin))?,
        upstream_path_prefix: file_config.upstream_path_prefix.clone(),
        model_routes: file_config.routes.clone(),
        max_output_tokens_cap: file_config.max_output_tokens_cap,
        rates: file_config.rates.clone(),
        provider_key,
        token_codec: codec.clone(),
    };
    let state = GatewayState::new(
        config,
        std::sync::Arc::new(usage),
        std::sync::Arc::new(audit),
        std::sync::Arc::clone(&spend),
        ceilings.clone(),
    )
    .map_err(|error| anyhow::anyhow!("gateway state: {error}"))?;

    let listener = tokio::net::TcpListener::bind(&file_config.bind)
        .await
        .with_context(|| format!("bind gateway {}", file_config.bind))?;
    let port = listener.local_addr()?.port();
    tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, router(state)).await {
            tracing::error!("model gateway serve failed: {error}");
        }
    });
    tracing::info!(bind = %file_config.bind, "model gateway listening");
    Ok(GatewayHandle { codec, ceilings, port, spend })
}

/// Load every company config's ceiling into the fuse map at boot.
fn seed_ceilings(root: &Path, ceilings: &CeilingMap) {
    let companies = root.join("companies");
    let Ok(entries) = std::fs::read_dir(&companies) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else { continue };
        match CompanyConfig::load(root, name) {
            Ok(config) => {
                let micro = (config.spend_ceiling_usd * 1_000_000.0).round().max(0.0) as u64;
                if let Ok(mut map) = ceilings.write() {
                    map.insert(config.name.clone(), micro);
                }
            }
            Err(error) => tracing::warn!(company = name, "skipping ceiling seed: {error:#}"),
        }
    }
}

fn load_or_seed_config(path: &Path) -> Result<GatewayFileConfig> {
    if path.exists() {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("read {}", path.display()))?;
        return toml::from_str(&raw).with_context(|| format!("parse {}", path.display()));
    }
    let config = GatewayFileConfig::default();
    let rendered = toml::to_string_pretty(&config).context("render default gateway.toml")?;
    std::fs::write(path, rendered).with_context(|| format!("seed {}", path.display()))?;
    Ok(config)
}

fn load_or_create_signing_key(path: &Path) -> Result<SecretBytes> {
    if path.exists() {
        return load_owner_private_secret(path)
            .map_err(|error| anyhow::anyhow!("load {}: {error}", path.display()));
    }
    // 32 bytes of OS randomness (two v4 UUIDs), written owner-private.
    let mut bytes = Uuid::new_v4().as_bytes().to_vec();
    bytes.extend_from_slice(Uuid::new_v4().as_bytes());
    write_private_file(path, &bytes)?;
    SecretBytes::new(bytes).map_err(|error| anyhow::anyhow!("signing key: {error}"))
}

fn load_or_create_provider_key(path: &Path) -> Result<SecretBytes> {
    if path.exists() {
        return load_owner_private_secret(path)
            .map_err(|error| anyhow::anyhow!("load {}: {error}", path.display()));
    }
    let key = std::env::var("RESTLESS_PROVIDER_KEY")
        .or_else(|_| std::env::var("OPENROUTER_API_KEY"))
        .context(
            "no gateway provider key: set RESTLESS_PROVIDER_KEY (or OPENROUTER_API_KEY) once; \
             it is stored owner-private at gateway/provider.key and never enters a container",
        )?;
    let key = key.trim();
    if key.is_empty() {
        bail!("gateway provider key env var is empty");
    }
    write_private_file(path, key.as_bytes())?;
    SecretBytes::new(key.as_bytes().to_vec())
        .map_err(|error| anyhow::anyhow!("provider key: {error}"))
}

fn create_private_dir(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        builder.mode(0o700);
    }
    builder.create(path).with_context(|| format!("create {}", path.display()))
}

fn write_private_file(path: &Path, contents: &[u8]) -> Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(path).with_context(|| format!("create {}", path.display()))?;
    use std::io::Write as _;
    file.write_all(contents)?;
    file.sync_all()?;
    Ok(())
}
