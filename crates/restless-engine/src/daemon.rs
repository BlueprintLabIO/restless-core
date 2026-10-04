//! The daemon's shared state and the plane configuration it is built from:
//! the per-company OrgIntel registry, local ports, plane database setup and
//! the standing actors every company starts with.

#[allow(unused_imports)]
use crate::*;

pub struct Daemon {
    pub root: PathBuf,
    pub capabilities: capability::CapabilityIssuer,
    pub spend: spend::SpendLedger,
    pub authority: authority::AuthorityStore,
    pub publication: publication::PublicationManager,
    pub launch: launch::LaunchBroker,
    pub orgintel: OrgIntelRegistry,
    pub staff: staff::StaffRegistry,
    /// Reconnectable live projections for agent turns. Completed messages,
    /// Work, and Attempts remain OrgIntel truth; this state is ephemeral.
    pub activities: activity::AgentActivityStreams,
    /// One reconnecting LISTEN/NOTIFY listener per company cell, shared by
    /// scheduler and realtime projections. Notifications are wake hints only;
    /// each consumer rereads its durable source of truth.
    pub cell_wakes: cell_wake::CellWakeHub,
    /// Exact outbound bridges from hosted Company Runtimes. Local appliance
    /// mode keeps its deliberately separate direct-Docker adapter.
    pub runtime_bridges: runtime_bridge::RuntimeBridgeRegistry,
    /// Crash-safe admission barrier for appliance replacement. The marker is
    /// host lifecycle state; live Work remains in the registries below and in
    /// OrgIntel rather than being copied into this gate.
    pub lifecycle: restless_contracts::appliance::LifecycleGate,
    /// One wake at a time per company, however the wake was requested —
    /// the scheduler (T6) and the owner-typed socket path share this set.
    pub in_flight: schedule::InFlight,
    /// Wake-only hints from launchd/systemd or the owner. The durable schedule
    /// ledger decides whether anything is due; this signal carries no work.
    pub schedule_wake: std::sync::Arc<tokio::sync::Notify>,
}

/// Lazily ensured per-cell OrgIntel handles (one pool per company, against
/// that company's **own** database and role — see `cell.rs`). `database_url`
/// is the account plane's admin connection, used only to provision cells and
/// to read a legacy shared schema during import; no company query runs on it.
pub struct OrgIntelRegistry {
    pub database_url: String,
    pub root: std::path::PathBuf,
    pub handles: std::sync::Mutex<HashMap<String, OrgIntel>>,
}

impl OrgIntelRegistry {
    /// This cell's own connection string, provisioning it if absent.
    /// Idempotent, and the only way anything reaches a cell's database.
    pub async fn cell_database_url(&self, company: &str) -> Result<String> {
        cell::ensure_database(&self.root, &self.database_url, company).await
    }

    pub async fn get(&self, company: &str) -> Result<OrgIntel> {
        let config = self.root.join("companies").join(format!("{company}.toml"));
        if !config.is_file() {
            anyhow::bail!(
                "company {company:?} is not configured; refusing to provision or reopen an OrgIntel cell"
            );
        }
        let cached = self
            .handles
            .lock()
            .expect("orgintel registry")
            .get(company)
            .cloned();
        if let Some(handle) = cached {
            // A cached handle can outlive its schema: a scenario reset, an
            // operator drop, or a restore removes the tables and every later
            // query fails with `relation "actors" does not exist`. Re-ensure
            // instead of serving a handle to nothing.
            if handle.is_live().await? {
                return Ok(handle);
            }
            tracing::warn!(
                company,
                "orgintel schema vanished under a cached handle; re-ensuring"
            );
        }
        let handle = ensure_cell_orgintel(&self.root, &self.database_url, company).await?;
        self.handles
            .lock()
            .expect("orgintel registry")
            .insert(company.to_string(), handle.clone());
        Ok(handle)
    }

    /// Drop a cached handle after its schema is destroyed (S04-T1). `get`
    /// already re-ensures a handle whose schema vanished, so this is an
    /// optimisation rather than a correctness fix — but leaving a handle to a
    /// dropped schema in the map means the next `up` of the same name pays a
    /// failed query first, and that noise is what made the sprint-02 reuse bug
    /// hard to read.
    pub fn forget(&self, company: &str) {
        self.handles
            .lock()
            .expect("orgintel registry")
            .remove(company);
    }
}

/// OrgIntel connection settings at `$RESTLESS_HOME/orgintel.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrgIntelConfig {
    pub database_url: String,
}

impl OrgIntelConfig {
    pub fn default_for_profile(profile: &restless_contracts::appliance::MachineProfile) -> Self {
        let user = std::env::var("USER").unwrap_or_else(|_| "postgres".to_string());
        let database = match profile.kind {
            restless_contracts::appliance::ProfileKind::Stable => "restless".to_string(),
            restless_contracts::appliance::ProfileKind::Dev | restless_contracts::appliance::ProfileKind::Test => {
                format!(
                    "restless_plane_{}",
                    profile.resource_namespace.replace('-', "_")
                )
            }
        };
        Self {
            database_url: format!("postgres://{user}@localhost/{database}"),
        }
    }

    pub fn read_only(profile: &restless_contracts::appliance::MachineProfile) -> Result<Self> {
        if let Some(config) = Self::from_plane_environment()? {
            config.ensure_profile_isolation(profile)?;
            return Ok(config);
        }
        let config = Self::read_from_root(&profile.state_root)?
            .unwrap_or_else(|| Self::default_for_profile(profile));
        config.ensure_profile_isolation(profile)?;
        Ok(config)
    }

    pub fn load_or_seed(profile: &restless_contracts::appliance::MachineProfile) -> Result<Self> {
        if let Some(config) = Self::from_plane_environment()? {
            config.ensure_profile_isolation(profile)?;
            return Ok(config);
        }
        let path = profile.state_root.join("orgintel.toml");
        if path.exists() {
            return Self::read_only(profile);
        }
        let config = Self::default_for_profile(profile);
        let rendered = toml::to_string_pretty(&config).context("render orgintel.toml")?;
        std::fs::write(&path, rendered).with_context(|| format!("seed {}", path.display()))?;
        config.ensure_profile_isolation(profile)?;
        Ok(config)
    }

    pub fn read_from_root(root: &Path) -> Result<Option<Self>> {
        let path = root.join("orgintel.toml");
        if !path.exists() {
            return Ok(None);
        }
        let raw =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        toml::from_str(&raw)
            .map(Some)
            .with_context(|| format!("parse {}", path.display()))
    }

    /// Cloud supplies the account-plane database credential as deployment
    /// state. Keep it in memory: persisting this value into `orgintel.toml`
    /// would copy a rotated secret into the long-lived plane volume.
    pub fn from_plane_environment() -> Result<Option<Self>> {
        let Some(raw) = std::env::var_os("RESTLESS_PLANE_DATABASE_URL") else {
            return Ok(None);
        };
        let raw = raw
            .into_string()
            .map_err(|_| anyhow::anyhow!("RESTLESS_PLANE_DATABASE_URL must be valid UTF-8"))?;
        validate_plane_database_url(&raw)?;
        Ok(Some(Self { database_url: raw }))
    }

    pub fn ensure_profile_isolation(
        &self,
        profile: &restless_contracts::appliance::MachineProfile,
    ) -> Result<()> {
        if profile.kind == restless_contracts::appliance::ProfileKind::Stable {
            return Ok(());
        }
        let home = std::env::var("HOME").context("HOME is not set")?;
        let stable = restless_contracts::appliance::MachineProfile::stable(Path::new(&home))?;
        let stable_config = Self::read_from_root(&stable.state_root)?
            .unwrap_or_else(|| Self::default_for_profile(&stable));
        if database_target(&self.database_url)? == database_target(&stable_config.database_url)? {
            anyhow::bail!(
                "the {} profile resolves the stable OrgIntel/Authority database; use its own database in {}",
                profile.kind.as_str(),
                profile.state_root.join("orgintel.toml").display()
            );
        }
        Ok(())
    }
}

pub fn configured_companies(root: &Path) -> Result<Vec<String>> {
    let directory = root.join("companies");
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut companies = Vec::new();
    for entry in
        std::fs::read_dir(&directory).with_context(|| format!("read {}", directory.display()))?
    {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) == Some("toml") {
            if let Some(name) = path.file_stem().and_then(|value| value.to_str()) {
                companies.push(name.to_string());
            }
        }
    }
    companies.sort();
    Ok(companies)
}

/// Namespace every host listener owned by one daemon with a single bounded
/// offset. The default remains the established port map. A second isolated
/// daemon can set `RESTLESS_PORT_OFFSET` and receive a coherent model relay,
/// coordination plane, owner surface, and ingress set without borrowing or
/// terminating the first daemon's processes.
pub fn port_offset() -> Result<u16> {
    let raw = std::env::var("RESTLESS_PORT_OFFSET").unwrap_or_else(|_| "0".to_string());
    raw.parse::<u16>()
        .with_context(|| format!("parse RESTLESS_PORT_OFFSET {raw:?} as a non-negative integer"))
}

pub fn port_with_offset(base: u16) -> Result<u16> {
    base.checked_add(port_offset()?).with_context(|| {
        format!("RESTLESS_PORT_OFFSET places base port {base} outside the TCP port range")
    })
}

/// Base TCP port the company containers reach the daemon on (T10). Next to the
/// model gateway's 7790; reachable as host.docker.internal from containers.
pub const COORD_TCP_PORT: u16 = 7791;

/// A Runtime is only ready for coordination after the host-issued bridge grant
/// is materialised inside its persistent computer. Keep the issuer and the
/// Runtime file write together here: neither the Runtime nor OrgIntel owns
/// that authority boundary.
pub async fn materialize_runtime_bridge(daemon: &Daemon, company: &str) -> Result<()> {
    let bridge = daemon
        .capabilities
        .issue_runtime_bridge(company)
        .context("issue Runtime bridge capability")?;
    runtime::install_runtime_bridge_capability(company, &bridge)
        .await
        .context("install Runtime bridge capability")
}

pub fn runtime_coordinator() -> Result<String> {
    Ok(format!(
        "host.docker.internal:{}",
        port_with_offset(COORD_TCP_PORT)?
    ))
}

/// Execute one Runtime-originated coordination JSONL request through the
/// existing capability-authenticated dispatcher. The hosted bridge calls this
/// instead of acquiring a Docker-network TCP route. Streaming `watch` is not
/// part of the one-request bridge contract.
pub async fn proxy_runtime_coordination(
    daemon: std::sync::Arc<Daemon>,
    request: serde_json::Value,
) -> Result<serde_json::Value> {
    if request.get("cmd").and_then(serde_json::Value::as_str) == Some("watch") {
        anyhow::bail!("streaming watch is not supported by Runtime coordination proxy");
    }
    let mut encoded =
        serde_json::to_vec(&request).context("encode Runtime coordination request")?;
    if encoded.is_empty() || encoded.len() > 128 * 1024 {
        anyhow::bail!("Runtime coordination request exceeds its bound");
    }
    encoded.push(b'\n');
    let (mut client, server) = tokio::io::duplex(256 * 1024);
    let task =
        tokio::spawn(async move { serve(server, &daemon, ConnectionOrigin::RuntimeTcp).await });
    client.write_all(&encoded).await?;
    let mut response = String::new();
    let mut reader = BufReader::new(client);
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        reader.read_line(&mut response),
    )
    .await
    .context("Runtime coordination response timed out")??;
    task.abort();
    if response.is_empty() || response.len() > 128 * 1024 || !response.ends_with('\n') {
        anyhow::bail!("Runtime coordination response exceeds its bound");
    }
    serde_json::from_str(response.trim_end_matches('\n'))
        .context("decode Runtime coordination response")
}

/// Provision this company's cell storage, import a legacy shared schema if one
/// is still the only copy, and return a handle bound to the cell's own
/// database and role. This is the single path to an OrgIntel handle — there is
/// no shared-database fallback, because two ways to reach company state is
/// exactly the split brain the cell boundary exists to remove.
pub async fn ensure_cell_orgintel(
    root: &std::path::Path,
    admin_url: &str,
    company: &str,
) -> Result<OrgIntel> {
    let cell_url = cell::ensure_database(root, admin_url, company).await?;
    if cell::import_legacy_schema(admin_url, &cell_url, company).await? {
        tracing::info!(
            company,
            "imported the legacy shared OrgIntel schema into this cell's own database; \
             the legacy schema is left in place for verification"
        );
    }
    let org = OrgIntel::ensure(&cell_url, company)
        .await
        .with_context(|| format!("open cell OrgIntel for {company}"))?;
    cell::ensure_native_documents_store(root, admin_url, company)
        .await
        .with_context(|| format!("provision native Documents storage capability for {company}"))?;
    Ok(org)
}

pub async fn ensure_profile_database(
    profile: &restless_contracts::appliance::MachineProfile,
    config: &OrgIntelConfig,
) -> Result<()> {
    if profile.kind == restless_contracts::appliance::ProfileKind::Stable {
        return Ok(());
    }
    let mut target =
        url::Url::parse(&config.database_url).context("parse OrgIntel database_url")?;
    let database = target.path().trim_start_matches('/').to_string();
    if database.is_empty()
        || database.len() > 63
        || !database.bytes().enumerate().all(|(index, byte)| {
            byte == b'_' || byte.is_ascii_lowercase() || (index > 0 && byte.is_ascii_digit())
        })
    {
        anyhow::bail!(
            "non-stable OrgIntel database names must be lowercase SQL identifiers, got {database:?}"
        );
    }
    target.set_path("/postgres");
    let mut admin = PgConnection::connect(target.as_str())
        .await
        .context("connect to Postgres to provision the isolated profile database")?;
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname=$1)")
            .bind(&database)
            .fetch_one(&mut admin)
            .await?;
    if !exists {
        admin
            .execute(format!("CREATE DATABASE {database}").as_str())
            .await
            .with_context(|| format!("create isolated profile database {database}"))?;
    }
    admin.close().await.ok();
    Ok(())
}

pub fn database_target(database_url: &str) -> Result<(String, Option<u16>, String)> {
    let parsed = url::Url::parse(database_url).context("parse OrgIntel database_url")?;
    let host = parsed
        .host_str()
        .context("OrgIntel database_url has no host")?;
    let database = parsed.path().trim_start_matches('/');
    if database.is_empty() || database.contains('/') {
        anyhow::bail!("OrgIntel database_url must name exactly one database");
    }
    Ok((
        host.to_ascii_lowercase(),
        parsed.port_or_known_default(),
        database.to_string(),
    ))
}

pub fn validate_plane_database_url(raw: &str) -> Result<()> {
    if raw.is_empty() || raw.len() > 2_048 || raw.trim() != raw || raw.contains(['\r', '\n']) {
        anyhow::bail!("RESTLESS_PLANE_DATABASE_URL must be one bounded URL value");
    }
    // Never put the raw URL in diagnostics: it normally contains the plane's
    // database password.
    let parsed = url::Url::parse(raw).map_err(|_| {
        anyhow::anyhow!("RESTLESS_PLANE_DATABASE_URL must be a valid PostgreSQL URL")
    })?;
    let database = parsed.path().strip_prefix('/').unwrap_or_default();
    if !matches!(parsed.scheme(), "postgres" | "postgresql")
        || parsed.host_str().is_none()
        || parsed.username().is_empty()
        || parsed.password().is_none_or(str::is_empty)
        || database.is_empty()
        || database.contains('/')
        || parsed.fragment().is_some()
    {
        anyhow::bail!(
            "RESTLESS_PLANE_DATABASE_URL must identify one password-authenticated PostgreSQL database"
        );
    }
    Ok(())
}

/// Owner and Exec exist for the lifetime of a company, not only after its
/// first conversation or wake. Keeping that lifecycle fact here prevents a
/// freshly created company from presenting an empty People surface or making
/// its first staff creation depend on an unrelated `tell` command.
pub async fn ensure_standing_actors(org: &OrgIntel, model: Option<&str>) -> Result<()> {
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .context("ensure standing Owner")?;
    org.ensure_actor_with_model("exec", "exec", "exec", "The Exec", model)
        .await
        .context("ensure standing Exec")?;
    Ok(())
}
