//! restlessd: starts the Restless daemon. The engine (restless-engine) and
//! the owner API (restless-owner) hold the code; this binary reads the plane
//! configuration, starts both, and runs until it is told to stop.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use restless_orgintel::OrgIntel;
use sqlx::{Connection as _, PgConnection};
use tokio::net::UnixListener;

use restless_engine::*;
use restless_owner::owner;

/// Reconcile one bounded attachment-stage batch per configured company. This
/// runs after startup inventory and periodically even when no browser uploads
/// arrive, so a crash cannot make linked receipts or abandoned stages live
/// forever. Runtime outages leave the tokenized DB claim for bounded retry.
async fn reconcile_owner_attachments(daemon: &Daemon, configs: &[runtime::CompanyConfig]) {
    for config in configs {
        let company = config.name.as_str();
        let outcome = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            let org = daemon.orgintel.get(company).await?;
            owner::collect_owner_attachments(&org, company).await
        })
        .await;
        match outcome {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                tracing::warn!(
                    company,
                    "owner attachment reconciliation deferred: {error:#}"
                )
            }
            Err(_) => tracing::warn!(company, "owner attachment reconciliation timed out"),
        }
    }
}

// Unoptimised builds (the README launcher's `cargo build`) keep large async
// frames on the stack; a live model turn overflowed Tokio's 2 MiB default.
const RUNTIME_THREAD_STACK_BYTES: usize = 16 * 1024 * 1024;

fn main() -> Result<()> {
    release::set_source_revision(env!("RESTLESS_SOURCE_REVISION"));
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(RUNTIME_THREAD_STACK_BYTES)
        .build()
        .context("start the async runtime")?
        .block_on(run())
}

async fn run() -> Result<()> {
    let machine_profile = restless_contracts::appliance::MachineProfile::from_env()?;
    // Local source checkouts conventionally keep bootstrap credentials in an
    // ignored `.env`. Load it before any subsystem reads configuration, while
    // preserving explicitly inherited service-manager variables. Infisical is
    // the durable backend; this is the one-time/local migration source.
    match dotenvy::dotenv() {
        Ok(path) => eprintln!("loaded local environment from {}", path.display()),
        Err(dotenvy::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("load local .env"),
    }
    restless_contracts::appliance::load_profile_environment(&machine_profile)?;
    restless_contracts::appliance::load_release_environment()?;
    if matches!(std::env::args().nth(1).as_deref(), Some("--help" | "-h")) {
        println!(
            "restlessd\n\nThe supervised Restless account plane. Run it without arguments; use `restless appliance status` for lifecycle status."
        );
        return Ok(());
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    // The test issuer for network entry (S27-T5). Restless Cloud is the real
    // issuer; this exists so an end-to-end run can mint against the same wire
    // format the verifier reads.
    if std::env::args().nth(1).as_deref() == Some("mint-entry-assertion") {
        println!("{}", entry::mint_from_env()?);
        return Ok(());
    }

    if std::env::args().nth(1).as_deref() == Some("appliance-preflight") {
        if machine_profile.kind != restless_contracts::appliance::ProfileKind::Stable {
            anyhow::bail!("appliance preflight requires the stable profile");
        }
        let cockpit = std::env::var_os("RESTLESS_COCKPIT_DIR")
            .map(PathBuf::from)
            .context("RESTLESS_COCKPIT_DIR is required for appliance preflight")?;
        if !cockpit.join("index.html").is_file() {
            anyhow::bail!("the staged Cockpit has no index.html");
        }
        for company in configured_companies(&machine_profile.state_root)? {
            runtime::CompanyConfig::load(&machine_profile.state_root, &company)
                .with_context(|| format!("preflight company configuration {company}"))?;
        }
        let orgintel = OrgIntelConfig::read_only(&machine_profile)?;
        OrgIntel::probe(&orgintel.database_url)
            .await
            .context("preflight could not reach OrgIntel")?;
        // An older build cannot open a cell a newer one has migrated: boot
        // would quarantine it while the plane still reports ready. Refuse
        // activation (a rollback, or an install of an older build) instead. A
        // cell this preflight cannot read is left to boot, which reports it.
        for company in configured_companies(&machine_profile.state_root)? {
            let Some(cell_url) = cell::recorded_database_url(&machine_profile.state_root, &company)
            else {
                continue;
            };
            match OrgIntel::unknown_applied_migrations(&cell_url, &company).await {
                Ok(unknown) => {
                    if let Some(newest) = unknown.iter().max() {
                        anyhow::bail!(
                            "{company} holds data a newer release has upgraded (OrgIntel migration {newest}), and this release cannot open it; keep a release that carries that migration, or restore a backup taken before the upgrade"
                        );
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        company,
                        "preflight could not read company migrations: {error:#}"
                    )
                }
            }
        }
        println!(
            "{}",
            serde_json::json!({
                "status": "ready_to_activate",
                "profile": "stable",
                "state_root": machine_profile.state_root,
                "cockpit": cockpit,
            })
        );
        return Ok(());
    }

    // Resolve and lock the machine profile before any migration, provider
    // listener or schedule reconciliation can mutate state. The Unix socket
    // is a transport and may be stale; it is not a singleton primitive.
    let root = machine_profile.state_root.clone();
    std::fs::create_dir_all(root.join("companies"))
        .with_context(|| format!("create state root {}", root.display()))?;
    std::fs::create_dir_all(machine_profile.log_dir())?;
    std::fs::create_dir_all(machine_profile.launch_cache_dir())?;
    let _singleton = restless_contracts::appliance::SingletonGuard::acquire(&machine_profile)?;
    tracing::info!(
        profile = machine_profile.kind.as_str(),
        root = %machine_profile.state_root.display(),
        lock = %_singleton.path().display(),
        "machine profile locked"
    );
    if std::env::args().nth(1).as_deref() == Some("rotate-native-documents-credential") {
        let args = std::env::args().collect::<Vec<_>>();
        anyhow::ensure!(
            args.len() == 3,
            "usage: restlessd rotate-native-documents-credential <company>"
        );
        let company = &args[2];
        runtime::CompanyConfig::load(&root, company)
            .with_context(|| format!("rotation company {company} is not configured"))?;
        anyhow::ensure!(
            owner_config::OwnerConfig::from_env()?
                .local_documents_issuer()
                .is_some(),
            "native Documents credential rotation is supported for the local Docs service only"
        );
        anyhow::ensure!(
            root.join("cells")
                .join(company)
                .join("database.url")
                .is_file()
                && cell::native_documents_store_credential_path(&root, company).is_file(),
            "native Documents rotation requires an existing cell and sidecar credential"
        );
        let orgintel = OrgIntelConfig::read_only(&machine_profile)?;
        OrgIntel::probe(&orgintel.database_url).await?;
        let cell_url = cell::ensure_database(&root, &orgintel.database_url, company).await?;
        let mut cell_connection = PgConnection::connect(&cell_url).await?;
        let cell_id: uuid::Uuid = sqlx::query_scalar(
            format!("SELECT cell_id FROM {company}.company_access_identity WHERE singleton=TRUE")
                .as_str(),
        )
        .fetch_one(&mut cell_connection)
        .await
        .context("read existing native Documents cell identity")?;
        cell_connection.close().await?;
        local_documents::stop_for_credential_rotation(&root, cell_id).await?;
        cell::rotate_native_documents_store(&root, &orgintel.database_url, company).await?;
        println!(
            "{}",
            serde_json::json!({"status":"credential_rotated_restart_required","company":company})
        );
        return Ok(());
    }
    let capabilities = capability::CapabilityIssuer::open(&root)?;
    // Two supported topologies (ADR 0007): direct loopback, or a network
    // entry that verifies a signed assertion. Resolve and validate the entry
    // configuration before starting provider or scheduler work, so a plane
    // that cannot describe how it verifies fails here rather than serving.
    let owner_config = owner_config::OwnerConfig::from_env()?;
    runtime::validate_company_image_config(owner_config.is_network())?;
    // A plane on a pinned release (Cloud, or an installed appliance after an upgrade) rolls its
    // companies' computers forward with that release (see below). A development profile does not.
    let release_plane = runtime::company_image_is_pinned();

    // Open authoritative charged-use accounting before the model relay. The
    // relay receives this exact ledger and is the only model path permitted to
    // append charged records.
    let spend = spend::SpendLedger::open(&root)?;

    // Model access is a host authority boundary. OMP's imported broker and
    // gateway hold the provider credential; company processes receive only a
    // signed, scoped relay capability. Its network/provider startup is kept
    // off the owner-surface critical path below.
    let company_configs = configured_companies(&root)?
        .into_iter()
        .map(|company| runtime::CompanyConfig::load(&root, &company))
        .collect::<Result<Vec<_>>>()?;
    // T5: coordination state. The database must answer at boot — probe,
    // never guess that it will be there when a company wakes.
    let orgintel_config = OrgIntelConfig::load_or_seed(&machine_profile)?;
    // A plane's database container can still be starting when this process does (a server boot
    // on 21 October exited here). Wait for it, up to a bound, rather than exiting on the first
    // refusal.
    let database_deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
    loop {
        let attempt = async {
            ensure_profile_database(&machine_profile, &orgintel_config).await?;
            OrgIntel::probe(&orgintel_config.database_url).await?;
            anyhow::Ok(())
        }
        .await;
        match attempt {
            Ok(_) => break,
            Err(error) if std::time::Instant::now() < database_deadline => {
                tracing::warn!(error = %format!("{error:#}"), "orgintel database not ready yet; waiting");
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            Err(error) => return Err(error).context("orgintel database is not reachable at boot"),
        }
    }

    let authority = authority::AuthorityStore::connect(&orgintel_config.database_url).await?;

    // One-time custody transfer from the old recoverable event stream. Do it
    // before listeners open so no effect can race its own migration.
    for company in configured_companies(&root)? {
        let bootstrap = async {
            let mut config = runtime::CompanyConfig::load(&root, &company)?;
            // Bootstrap is a serial migration pass, not the live handle registry.
            // Caching one pool per historical test company here exhausted
            // PostgreSQL before the daemon could finish booting. Keep only the
            // current company's pool alive; the runtime registry below remains
            // lazy and caches only companies that are actually used.
            let org = ensure_cell_orgintel(&root, &orgintel_config.database_url, &company).await?;
            ensure_standing_actors(&org, config.configured_model()).await?;
            let imported = authority
                .import_legacy_company(&company, &org, &approval::legacy_config_approvals(&config))
                .await?;
            if imported > 0 {
                tracing::info!(
                    company,
                    imported,
                    "migrated governance truth into Authority"
                );
            }
            approval::purge_legacy_config_approvals(&root, &mut config)?;
            drop(org);
            Ok::<_, anyhow::Error>(())
        }
        .await;
        if let Err(error) = bootstrap {
            // One historical or experimental cell must not take every live
            // company and every schedule offline. Keep the exact company
            // unavailable and visible through its own failing cell probe,
            // while the rest of the account plane continues to operate.
            tracing::error!(company, "company cell quarantined during boot: {error:#}");
        }
    }

    let publication = publication::PublicationManager::new(&root, authority.clone())?;
    let launch = launch::LaunchBroker::new(&root)?;
    let daemon = std::sync::Arc::new(Daemon {
        root: root.clone(),
        capabilities,
        spend,
        authority,
        publication,
        launch,
        orgintel: OrgIntelRegistry {
            database_url: orgintel_config.database_url,
            root: root.clone(),
            handles: std::sync::Mutex::new(HashMap::new()),
        },
        staff: staff::StaffRegistry::default(),
        activities: activity::AgentActivityStreams::default(),
        cell_wakes: cell_wake::CellWakeHub::default(),
        runtime_bridges: if owner_config.hosted_runtime() {
            runtime_bridge::RuntimeBridgeRegistry::hosted()
        } else {
            runtime_bridge::RuntimeBridgeRegistry::default()
        },
        lifecycle: restless_contracts::appliance::LifecycleGate::new(
            restless_contracts::appliance::drain_marker_exists(&root),
        ),
        in_flight: std::sync::Arc::new(std::sync::Mutex::new(schedule::WakeClaims::default())),
        schedule_wake: std::sync::Arc::new(tokio::sync::Notify::new()),
    });
    if daemon.runtime_bridges.is_hosted() {
        runtime_bridge::remember_identity_pool(daemon.authority.pool().clone());
    } else if let Err(error) =
        restless_engine::machine_router::start(daemon.authority.pool().clone()).await
    {
        // Companies on the plane run as before; only paired machines need the router.
        tracing::warn!(error = %format!("{error:#}"), "machine router not started");
    }
    restless_engine::account_usage::remember_pool(daemon.authority.pool().clone());
    daemon.lifecycle.begin_recovery();
    runtime::begin_startup_recovery();

    // Start the provider boundary independently of startup recovery. Large
    // historical stores can make orphan and publication repair expensive;
    // model readiness must not queue behind those reads. The scheduler still
    // waits on the explicit recovery barrier below, so no autonomous work can
    // race cleanup from a previous daemon generation.
    let test_scheduler_disabled =
        std::env::var("RESTLESS_TEST_DISABLE_SCHEDULER").is_ok_and(|value| value == "1");
    if test_scheduler_disabled
        && company_configs
            .iter()
            .any(|config| !config.name.ends_with("_test"))
    {
        anyhow::bail!("RESTLESS_TEST_DISABLE_SCHEDULER is allowed only on an all-test plane");
    }
    let (recovery_ready_tx, mut recovery_ready_rx) = tokio::sync::watch::channel(false);
    let model_configs = company_configs.clone();
    let model_root = root.clone();
    tokio::spawn(local_documents::maintain_history_task(
        std::sync::Arc::clone(&daemon),
    ));

    let model_capabilities = daemon.capabilities.clone();
    let model_spend = daemon.spend.clone();
    let tool_gateway_daemon =
        (!daemon.runtime_bridges.is_hosted()).then(|| std::sync::Arc::clone(&daemon));
    let schedule_daemon = std::sync::Arc::clone(&daemon);
    let idle_daemon = std::sync::Arc::clone(&daemon);
    let mut idle_recovery_ready_rx = recovery_ready_rx.clone();
    tokio::spawn(async move {
        while !*idle_recovery_ready_rx.borrow() {
            if idle_recovery_ready_rx.changed().await.is_err() {
                return;
            }
        }
        runtime_sleep::run_task(idle_daemon).await;
    });

    tokio::spawn(async move {
        let load_configs = |root: &std::path::Path| -> Result<Vec<runtime::CompanyConfig>> {
            configured_companies(root)?
                .into_iter()
                .map(|company| runtime::CompanyConfig::load(root, &company))
                .collect()
        };
        let mut model_configs = model_configs;
        loop {
            match model_gateway::start(
                &model_configs,
                &model_root,
                model_capabilities.clone(),
                model_spend.clone(),
                tool_gateway_daemon.clone(),
            )
            .await
            {
                Ok(mut processes) => {
                    if model_gateway::is_ready() {
                        tracing::info!("model gateway ready");
                    } else {
                        tracing::info!(
                            "account model broker ready; no direct model route is admitted yet"
                        );
                    }
                    // Providers load only when the gateway starts, so restart it
                    // when a company's model route or credential references
                    // change instead of asking the owner to restart Restless.
                    // Restart only when what the gateway loads changes. A new
                    // company or a model switch within loaded providers is
                    // re-admitted in place: the relay also carries every
                    // company's tool gateway, so a restart drops their calls.
                    let started_from = processes.fingerprint().to_owned();
                    let mut admitted_from = model_gateway::provider_fingerprint(&model_configs);
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        if let Err(error) = processes.check_running() {
                            tracing::warn!("model service stopped; restarting: {error:#}");
                            if let Ok(current) = load_configs(&model_root) {
                                model_configs = current;
                            }
                            break;
                        }
                        if let Ok(current) = load_configs(&model_root) {
                            let unchanged = model_connections::account_oauth_providers(&model_root)
                                .is_ok_and(|accounts| {
                                    model_gateway::gateway_fingerprint(&current, &accounts)
                                        == started_from
                                });
                            if !unchanged {
                                model_configs = current;
                                break;
                            }
                            let admission = model_gateway::provider_fingerprint(&current);
                            if admission != admitted_from {
                                match model_gateway::readmit(&current) {
                                    Ok(()) => {
                                        tracing::info!("company model routes changed; re-admitted without restarting the gateway");
                                        admitted_from = admission;
                                    }
                                    Err(error) => {
                                        tracing::warn!("re-admission failed; reloading the model gateway: {error:#}");
                                        model_configs = current;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    tracing::info!("reloading account model services");
                    model_gateway::uninstall();
                    drop(processes);
                    // Let the stopped broker and gateway release their ports.
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
                Err(error) => {
                    model_gateway::uninstall();
                    tracing::error!(
                        "model gateway unavailable; owner plane remains ready and retry is scheduled: {error:#}"
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    if let Ok(current) = load_configs(&model_root) {
                        model_configs = current;
                    }
                }
            }
        }
    });

    tokio::spawn(async move {
        while !*recovery_ready_rx.borrow() {
            if recovery_ready_rx.changed().await.is_err() {
                return;
            }
        }
        if test_scheduler_disabled {
            tracing::warn!("automatic scheduler disabled for an isolated test plane");
        } else {
            tokio::spawn(schedule::run_task(std::sync::Arc::clone(&schedule_daemon)));
        }
    });

    // Runtime recovery is safety-critical for new work but is not availability-
    // critical for the durable control plane. Docker Desktop can legitimately
    // take tens of seconds to answer while many unrelated containers are busy.
    // Keep admission closed, expose an honest recovering state, and retry the
    // bounded observation asynchronously. The scheduler shares the same barrier
    // through `recovery_ready_rx`, so no work races orphan cleanup.
    let recovery_daemon = std::sync::Arc::clone(&daemon);
    let recovery_configs = company_configs.clone();
    tokio::spawn(async move {
        let recovery_started = std::time::Instant::now();
        let hosted_runtime = recovery_daemon.runtime_bridges.is_hosted();
        if release_plane {
            company::note_roll_forward_stage("waiting_for_startup_recovery", None);
        }
        let running_companies = if hosted_runtime {
            Vec::new()
        } else {
            loop {
                let attempt_started = std::time::Instant::now();
                match runtime::running_configured_companies(&recovery_configs).await {
                    Ok(companies) => {
                        tracing::info!(
                            elapsed_ms = attempt_started.elapsed().as_millis(),
                            running = companies.len(),
                            "runtime recovery inventory complete"
                        );
                        break companies;
                    }
                    Err(error) => {
                        if release_plane {
                            company::note_roll_forward_stage(
                                "waiting_for_startup_recovery",
                                Some(format!("Docker inventory deferred: {error:#}")),
                            );
                        }
                        tracing::warn!(
                            elapsed_ms = attempt_started.elapsed().as_millis(),
                            "runtime recovery inventory deferred: {error:#}"
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            }
        };

        // Each company observes capacity and completes its own orphan sweep.
        // A busy or unobservable company stays held while other companies recover.
        let sweep_started = std::time::Instant::now();
        if !hosted_runtime {
            let companies = recovery_configs
                .iter()
                .map(|config| config.name.clone())
                .collect::<Vec<_>>();
            // Install every hold before spawning any sweep or reopening the
            // scheduler; one slow company cannot delay a healthy peer.
            for company in &companies {
                recovery_daemon.staff.set_recovery_pending(company, true);
            }
            let recovery_slots = std::sync::Arc::new(tokio::sync::Semaphore::new(4));
            for company in companies {
                let daemon = std::sync::Arc::clone(&recovery_daemon);
                let slots = std::sync::Arc::clone(&recovery_slots);
                let was_running = running_companies.contains(&company);
                tokio::spawn(async move {
                    loop {
                        let Ok(permit) = slots.acquire().await else { break; };
                        let companies = vec![company.clone()];
                        let running = if was_running { companies.clone() } else { Vec::new() };
                        let complete = staff::sweep_orphans(&daemon.orgintel, &daemon.staff, &companies, &running).await.is_empty();
                        drop(permit);
                        if complete {
                            daemon.schedule_wake.notify_one();
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                });
            }
        }
        tracing::info!(
            elapsed_ms = sweep_started.elapsed().as_millis(),
            "runtime orphan recovery scheduled"
        );

        if !hosted_runtime {
            reconcile_owner_attachments(&recovery_daemon, &recovery_configs).await;
        }

        // Published services are provider-owned processes, not Runtime
        // children. Reconcile only isolated test companies after orphan repair;
        // one broken fixture remains a degraded publication, never a plane
        // startup failure.
        let publication_started = std::time::Instant::now();
        for company in recovery_configs
            .iter()
            .map(|config| config.name.as_str())
            .filter(|company| company.ends_with("_test"))
        {
            let outcome = async {
                let org = recovery_daemon.orgintel.get(company).await?;
                recovery_daemon
                    .publication
                    .reconcile_company(&org, company)
                    .await
            }
            .await;
            if let Err(error) = outcome {
                tracing::error!(
                    company,
                    "published-service reconciliation failed: {error:#}"
                );
            }
        }
        tracing::info!(
            elapsed_ms = publication_started.elapsed().as_millis(),
            "published-service recovery complete"
        );

        runtime::finish_startup_recovery();
        recovery_daemon.lifecycle.finish_recovery();
        recovery_ready_tx.send_replace(true);
        tracing::info!(
            elapsed_ms = recovery_started.elapsed().as_millis(),
            "startup recovery barrier opened"
        );
        if release_plane && !hosted_runtime {
            company::note_roll_forward_waiting();
            // In its own task: a rebuild can take a while and must not hold up startup.
            let roll_daemon = std::sync::Arc::clone(&recovery_daemon);
            let roll_configs = recovery_configs.clone();
            tokio::spawn(async move {
                company::roll_forward_runtimes(&roll_daemon, &roll_configs).await;
            });
        }
        if !hosted_runtime && !test_scheduler_disabled {
            for config in &recovery_configs {
                match recovery_daemon.orgintel.get(&config.name).await {
                    Ok(org) => {
                        tokio::spawn(native_harness::startup_doctor_task(
                            recovery_daemon.root.clone(),
                            config.clone(),
                            recovery_daemon.capabilities.clone(),
                            org,
                        ));
                    }
                    Err(error) => {
                        tracing::error!(company=%config.name, %error, "automatic company setup unavailable")
                    }
                }
            }
        }
        loop {
            tokio::time::sleep(owner::OWNER_ATTACHMENT_RECONCILE_INTERVAL).await;
            if !hosted_runtime {
                reconcile_owner_attachments(&recovery_daemon, &recovery_configs).await;
            }
        }
    });

    // The owner API is useful during recovery for diagnosis and read-only
    // inspection. Mutation paths share LifecycleGate and remain closed until
    // the asynchronous recovery task opens admission. It is a required
    // listener: if it cannot start (network-entry keys unavailable, port in
    // use) or stops, the daemon exits non-zero like any other required boot
    // failure, so a supervisor restarts it instead of a plane that answers
    // nobody.
    let owner_daemon = std::sync::Arc::clone(&daemon);
    let owner_gateway = tokio::spawn(owner::serve_task(owner_daemon, owner_config));
    tokio::pin!(owner_gateway);
    // T6: the scheduler is what makes the company act without the owner
    // typing — time triggers (exec-set schedules + periodic tick) and
    // OrgIntel LISTEN/NOTIFY events share one loop. Product integration tests
    // may drive the exact semantic loop themselves; a narrowly named escape
    // hatch prevents the resident scheduler racing that controller. It is
    // refused if any real company is configured.
    let sock = root.join("restlessd.sock");
    if sock.exists() {
        std::fs::remove_file(&sock)?;
    }
    let listener = UnixListener::bind(&sock).with_context(|| format!("bind {}", sock.display()))?;
    tracing::info!(socket = %sock.display(), "restlessd listening");

    // Publish this plane so a CLI pointed at another home can name the live
    // planes instead of reporting that none is running. Dropped on exit.
    let _plane_registration = match plane::register(
        &root,
        &sock,
        port_offset()?,
        company_configs
            .iter()
            .map(|config| config.name.clone())
            .collect(),
    ) {
        Ok(registration) => Some(registration),
        Err(error) => {
            tracing::warn!("could not publish this plane for CLI discovery: {error:#}");
            None
        }
    };

    // Unix sockets do not cross the Docker Desktop file share (probed: the
    // mount hangs), so containers reach the daemon over TCP on the same
    // proven path as the model relay. TCP is capability-authenticated before
    // dispatch; company identity is never trusted as JSON sent by the caller.
    let coord_addr = format!("0.0.0.0:{}", port_with_offset(COORD_TCP_PORT)?);
    match tokio::net::TcpListener::bind(&coord_addr).await {
        Ok(tcp) => {
            tracing::info!(addr = %coord_addr, "coordination TCP listening (company containers)");
            let tcp_daemon = std::sync::Arc::clone(&daemon);
            tokio::spawn(async move {
                loop {
                    match tcp.accept().await {
                        Ok((stream, peer)) => {
                            // Containers only: the listener is on every interface so each Docker
                            // network's gateway reaches it, but never answers the internet.
                            if !restless_engine::coordination::local_peer(peer.ip()) {
                                tracing::warn!(%peer, "coordination TCP refused a public peer");
                                continue;
                            }
                            let daemon = std::sync::Arc::clone(&tcp_daemon);
                            tokio::spawn(async move {
                                if let Err(error) = coordination::serve_tcp(
                                    stream,
                                    &daemon,
                                    ConnectionOrigin::RuntimeTcp,
                                )
                                .await
                                {
                                    tracing::warn!("tcp connection error: {error:#}");
                                }
                            });
                        }
                        Err(error) => tracing::warn!("tcp accept: {error:#}"),
                    }
                }
            });
        }
        Err(error) => {
            tracing::error!(addr = %coord_addr, "coordination TCP bind failed: {error:#} — \
                agents in containers will have no coordination channel");
        }
    }

    // A company's home sites go out through its owner's own computer (S99 step 2). The proxy
    // answers only company container addresses.
    let home_addr = format!(
        "0.0.0.0:{}",
        port_with_offset(restless_engine::home_egress::HOME_EGRESS_PORT)?
    );
    match tokio::net::TcpListener::bind(&home_addr).await {
        Ok(tcp) => {
            tracing::info!(addr = %home_addr, "home egress listening (company containers)");
            tokio::spawn(restless_engine::home_egress::serve(
                daemon.authority.pool().clone(),
                tcp,
            ));
        }
        Err(error) => tracing::warn!(addr = %home_addr, "home egress bind failed: {error:#}"),
    }

    // S03-T2: the world's front door, on its OWN listener and its own task.
    // The failure boundary is the point (AC6): a slow, malformed or flooded
    // inbound request must not be able to stall the scheduler — F12's lesson,
    // where one company's hung Docker took down all three. The port is never
    // unauthenticated: each request is verified with its own company's signing
    // secret, and a company without one refuses every event rather than
    // accepting it unsigned.
    let sink = std::sync::Arc::new(inbound::AuthoritySink {
        daemon: std::sync::Arc::clone(&daemon),
    });
    tokio::spawn(async move {
        let port = match port_with_offset(ingress::INGRESS_PORT) {
            Ok(port) => port,
            Err(error) => {
                tracing::error!("event ingress port is invalid: {error:#}");
                return;
            }
        };
        if let Err(error) = ingress::serve(port, sink).await {
            tracing::error!("event ingress stopped: {error:#}");
        }
    });
    // Repair the only durable seam after Authority custody. The immediate
    // projection is event-driven; this bounded cursor scan exists solely for
    // restart/crash recovery and is idempotent at the OrgIntel source ref.
    let inbound_daemon = std::sync::Arc::clone(&daemon);
    tokio::spawn(async move {
        loop {
            match inbound::reconcile_pending(&inbound_daemon).await {
                Ok(count) if count > 0 => {
                    tracing::info!(count, "reconciled pending inbound projections")
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!("inbound projection reconciliation deferred: {error:#}")
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    });

    // Finance webhooks use Airwallex's own timestamp+body HMAC and trigger a
    // direct authenticated provider read. The listener is independent of the
    // Resend/Svix rail so one provider's credential outage cannot disable the
    // other or the scheduler.
    let finance_daemon = std::sync::Arc::clone(&daemon);
    tokio::spawn(async move {
        if let Err(error) = airwallex_ingress::serve_task(finance_daemon).await {
            tracing::error!("Airwallex event ingress stopped: {error:#}");
        }
    });

    // Supervisors stop the daemon with SIGTERM: Docker Compose when a hosted
    // plane moves to a new release, systemd or launchd on an appliance, and
    // `restless-dev`. Observe it inside the runtime so in-flight work drains
    // first and owned model-broker/gateway child handles are dropped and
    // killed before the process exits; an abrupt default signal exit leaves
    // those children holding 7789/7790 and the next daemon half-attached to
    // stale supervision. The socket keeps accepting during the drain, so an
    // owner or `restless appliance` call gets the clean `draining` answer
    // instead of hanging until exit.
    let stop = async {
        shutdown_signal().await;
        drain_before_exit(&daemon).await;
    };
    tokio::pin!(stop);
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, _) = accepted?;
                let daemon = std::sync::Arc::clone(&daemon);
                tokio::spawn(async move {
                    if let Err(error) =
                        coordination::serve_unix(stream, &daemon, ConnectionOrigin::LocalOwner).await
                    {
                        tracing::warn!("connection error: {error:#}");
                    }
                });
            }
            () = &mut stop => {
                tracing::info!("shutdown requested; stopping supervised daemon children");
                break;
            }
            stopped = &mut owner_gateway => {
                return match stopped {
                    Ok(Ok(())) => Err(anyhow::anyhow!("owner gateway stopped")),
                    Ok(Err(error)) => Err(error.context("owner gateway stopped")),
                    Err(error) => Err(anyhow::Error::new(error).context("owner gateway task failed")),
                };
            }
        }
    }
    Ok(())
}

/// How long a stop waits for in-flight work: the window `restless appliance
/// upgrade` gives its drain. A hosted plane's Compose `stop_grace_period` must
/// be longer, or Docker kills the daemon mid-drain.
const STOP_DRAIN_WINDOW: std::time::Duration = std::time::Duration::from_secs(30);

/// Close work admission as an appliance drain does and give in-flight Exec
/// turns, Staff and governed effects up to [`STOP_DRAIN_WINDOW`] to finish,
/// so a release that replaces the plane does not cut off an external effect
/// mid-flight. An appliance operation that already drained, or chose to
/// force, exits at once, and so does a second signal. Nothing is persisted:
/// the next daemon opens admission as usual.
async fn drain_before_exit(daemon: &Daemon) {
    if daemon.lifecycle.is_draining()
        || restless_contracts::appliance::drain_marker_exists(&daemon.root)
    {
        return;
    }
    let again = shutdown_signal();
    tokio::pin!(again);
    let window = task_drain_window();
    if !wait_for_running_work(
        &daemon.lifecycle,
        || running_work(daemon),
        window,
        std::time::Duration::from_secs(1),
        again.as_mut(),
    )
    .await
    {
        return;
    }
    daemon.lifecycle.begin_drain();
    tracing::info!("stop requested; draining in-flight work before exit");
    let started = std::time::Instant::now();
    loop {
        let status = coordination::appliance_drain_status(daemon);
        // Recovery admits no work, so unlike a replacement it does not hold
        // the exit.
        if status.active_requests == 0
            && status.exec_wakes.is_empty()
            && status.staff.is_empty()
            && status.native_clients.is_empty()
        {
            return;
        }
        if started.elapsed() >= STOP_DRAIN_WINDOW {
            tracing::warn!(
                active_requests = status.active_requests,
                exec_wakes = ?status.exec_wakes,
                staff = ?status.staff,
                native_clients = ?status.native_clients,
                "stopping with work still in flight after the drain window"
            );
            return;
        }
        tokio::select! {
            () = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
            () = &mut again => return,
        }
    }
}

/// How long a stop first waits for running Work to finish, from
/// `RESTLESS_STOP_TASK_DRAIN_SECONDS` (default 0: an appliance upgrade already
/// refuses to replace a busy daemon). A hosted plane sets it so a release does
/// not cut off Work mid-run: its Compose `stop_grace_period` must cover this
/// window plus [`STOP_DRAIN_WINDOW`].
fn task_drain_window() -> std::time::Duration {
    std::env::var("RESTLESS_STOP_TASK_DRAIN_SECONDS")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(|seconds| std::time::Duration::from_secs(seconds.min(3_600)))
        .unwrap_or_default()
}

/// Staff and Work running now, not counting the Exec: owner chat goes on while
/// the stop waits, and an interrupted Exec turn is replayed by the next daemon.
fn running_work(daemon: &Daemon) -> Vec<String> {
    daemon
        .staff
        .running_staff()
        .into_iter()
        .filter(|actor| !actor.ends_with("/exec"))
        .collect()
}

/// Before the admission drain, stop starting Work and Staff turns and wait up
/// to `window` for the running ones to finish. Owner chat, reads and effects
/// stay open meanwhile. False when a second signal asks to exit now.
async fn wait_for_running_work(
    lifecycle: &restless_contracts::appliance::LifecycleGate,
    running_work: impl Fn() -> Vec<String>,
    window: std::time::Duration,
    poll: std::time::Duration,
    mut again: std::pin::Pin<&mut impl std::future::Future<Output = ()>>,
) -> bool {
    if window.is_zero() || running_work().is_empty() {
        return true;
    }
    lifecycle.pause_tasks();
    tracing::info!(
        running = ?running_work(),
        window_seconds = window.as_secs(),
        "stop requested; new Work paused while running Work finishes"
    );
    let started = std::time::Instant::now();
    loop {
        let running = running_work();
        if running.is_empty() {
            tracing::info!(
                waited_seconds = started.elapsed().as_secs(),
                "running Work finished before the stop"
            );
            return true;
        }
        if started.elapsed() >= window {
            tracing::warn!(
                ?running,
                "stopping with Work still running after the task window"
            );
            return true;
        }
        tokio::select! {
            () = tokio::time::sleep(poll) => {}
            () = again.as_mut() => return false,
        }
    }
}

async fn shutdown_signal() {
    let interrupt = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("install SIGTERM handler");
        tokio::select! {
            _ = interrupt => {}
            _ = terminate.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = interrupt.await;
    }
}

#[cfg(test)]
mod stop_tests {
    use super::wait_for_running_work;
    use restless_contracts::appliance::LifecycleGate;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    const POLL: Duration = Duration::from_millis(10);

    #[tokio::test]
    async fn a_stop_waits_for_running_work_with_new_work_paused() {
        let gate = LifecycleGate::default();
        let polls = AtomicUsize::new(0);
        let running = || {
            if polls.fetch_add(1, Ordering::SeqCst) < 5 {
                vec!["reno/designer".into()]
            } else {
                vec![]
            }
        };
        let never = std::future::pending::<()>();
        tokio::pin!(never);
        let window = Duration::from_secs(60);
        assert!(wait_for_running_work(&gate, running, window, POLL, never.as_mut()).await);
        assert!(gate.tasks_paused());
        // Owner chat and effects are still admitted while Work finishes.
        assert!(gate.try_enter().is_some());
        assert!(polls.load(Ordering::SeqCst) > 5);
    }

    #[tokio::test]
    async fn an_idle_plane_or_no_window_stops_at_once_without_pausing() {
        let gate = LifecycleGate::default();
        let never = std::future::pending::<()>();
        tokio::pin!(never);
        let window = Duration::from_secs(60);
        assert!(wait_for_running_work(&gate, Vec::new, window, POLL, never.as_mut()).await);
        let busy = || vec!["reno/designer".to_string()];
        assert!(wait_for_running_work(&gate, busy, Duration::ZERO, POLL, never.as_mut()).await);
        assert!(!gate.tasks_paused());
    }

    #[tokio::test]
    async fn the_window_bounds_the_wait_and_a_second_signal_ends_it() {
        let gate = LifecycleGate::default();
        let busy = || vec!["reno/designer".to_string()];
        let never = std::future::pending::<()>();
        tokio::pin!(never);
        let started = std::time::Instant::now();
        let short = Duration::from_millis(50);
        assert!(wait_for_running_work(&gate, busy, short, POLL, never.as_mut()).await);
        assert!(started.elapsed() < Duration::from_secs(5));

        let now = std::future::ready(());
        tokio::pin!(now);
        let window = Duration::from_secs(60);
        assert!(!wait_for_running_work(&gate, busy, window, POLL, now.as_mut()).await);
    }
}
