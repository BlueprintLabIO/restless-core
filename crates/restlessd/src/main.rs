//! restlessd — the stable coordination core (ARCHITECTURE.md §4.4).
//!
//! Sprint 01 slice: company environment lifecycle over a unix socket and the
//! stable coordination core. JSON-lines protocol: one request line, one
//! response line.

mod acp;
mod activity;
mod airwallex;
mod airwallex_ingress;
mod approval;
mod attention;
mod authority;
mod capability;
mod capability_sourcing;
mod cell;
mod cell_wake;
mod codex;
mod collaboration_doctor;
mod company;
mod company_bootstrap;
mod company_projection;
mod connected_tool;
mod coordination;
mod context;
mod credential;
mod daemon;
mod custom_harness;
mod document_collaboration_token;
mod document_commands;
mod documents_service;
mod email;
mod effect;
mod entry;
mod exec;
mod finance;
mod health;
mod inbound;
mod ingress;
mod launch;
mod legal;
mod local_documents;
mod sheet_commands;
use crate::authority as mandate;
mod mcp_gateway;
mod stdio_mcp;
mod mentions;
mod model_catalog;
mod model_connections;
mod model_gateway;
mod native_harness;
mod owner;
mod owner_config;
mod owner_brief;
mod owner_cell_readiness;
mod plane;
mod publication;
mod reconcile;
mod release;
mod room_commands;
mod runtime;
mod runtime_bridge;
mod runtime_sleep;
mod runtime_mode;
mod runtime_usage;
mod schedule;
mod schedule_test;
mod schedule_test_proxy;
mod skills;
mod spend;
mod staff;
mod telemetry;
mod transcript;
mod wire;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use restless_orgintel::OrgIntel;
use serde::{Deserialize, Serialize};
use sqlx::{Connection as _, Executor as _, PgConnection};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

#[cfg(test)]
use wire::OWNER_ONLY;
use wire::{authorize, Principal, Request, Response};
pub(crate) use coordination::*;
pub(crate) use daemon::*;















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
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(RUNTIME_THREAD_STACK_BYTES)
        .build()
        .context("start the async runtime")?
        .block_on(run())
}

async fn run() -> Result<()> {
    let machine_profile = restlessd::appliance::MachineProfile::from_env()?;
    // Local source checkouts conventionally keep bootstrap credentials in an
    // ignored `.env`. Load it before any subsystem reads configuration, while
    // preserving explicitly inherited service-manager variables. Infisical is
    // the durable backend; this is the one-time/local migration source.
    match dotenvy::dotenv() {
        Ok(path) => eprintln!("loaded local environment from {}", path.display()),
        Err(dotenvy::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("load local .env"),
    }
    restlessd::appliance::load_profile_environment(&machine_profile)?;
    restlessd::appliance::load_release_environment()?;
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
        if machine_profile.kind != restlessd::appliance::ProfileKind::Stable {
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
    let _singleton = restlessd::appliance::SingletonGuard::acquire(&machine_profile)?;
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
            owner_config::OwnerConfig::from_env()?.local_documents_issuer().is_some(),
            "native Documents credential rotation is supported for the local Docs service only"
        );
        anyhow::ensure!(
            root.join("cells").join(company).join("database.url").is_file()
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
    ensure_profile_database(&machine_profile, &orgintel_config).await?;
    OrgIntel::probe(&orgintel_config.database_url)
        .await
        .context("orgintel database is not reachable at boot")?;

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
        lifecycle: restlessd::appliance::LifecycleGate::new(
            restlessd::appliance::drain_marker_exists(&root),
        ),
        in_flight: std::sync::Arc::new(std::sync::Mutex::new(schedule::WakeClaims::default())),
        schedule_wake: std::sync::Arc::new(tokio::sync::Notify::new()),
    });
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
    tokio::spawn(local_documents::maintain_history(std::sync::Arc::clone(
        &daemon,
    )));

    let model_capabilities = daemon.capabilities.clone();
    let model_spend = daemon.spend.clone();
    let local_mcp_daemon = (!daemon.runtime_bridges.is_hosted())
        .then(|| std::sync::Arc::clone(&daemon));
    let schedule_daemon = std::sync::Arc::clone(&daemon);
    let idle_daemon = std::sync::Arc::clone(&daemon);
    let mut idle_recovery_ready_rx = recovery_ready_rx.clone();
    tokio::spawn(async move {
        while !*idle_recovery_ready_rx.borrow() {
            if idle_recovery_ready_rx.changed().await.is_err() {
                return;
            }
        }
        runtime_sleep::run(idle_daemon).await;
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
                local_mcp_daemon.clone(),
            )
            .await
            {
                Ok(processes) => {
                    if model_gateway::is_ready() {
                        tracing::info!("model gateway ready");
                    } else {
                        tracing::info!("account model broker ready; no direct model route is admitted yet");
                    }
                    // Providers load only when the gateway starts, so restart it
                    // when a company's model route or credential references
                    // change instead of asking the owner to restart Restless.
                    let started_from = format!("{}|{:?}", model_gateway::provider_fingerprint(&model_configs), model_connections::account_oauth_providers(&model_root));
                    loop {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        if let Ok(current) = load_configs(&model_root) {
                            if format!("{}|{:?}", model_gateway::provider_fingerprint(&current), model_connections::account_oauth_providers(&model_root)) != started_from {
                                model_configs = current;
                                break;
                            }
                        }
                    }
                    tracing::info!("company model providers changed; reloading the model gateway");
                    model_gateway::uninstall();
                    drop(processes);
                    // Let the stopped broker and gateway release their ports.
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
                Err(error) => {
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
            tokio::spawn(schedule::run(std::sync::Arc::clone(&schedule_daemon)));
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
                        tracing::warn!(
                            elapsed_ms = attempt_started.elapsed().as_millis(),
                            "runtime recovery inventory deferred: {error:#}"
                        );
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            }
        };

        // Effect and cognitive orphan domains are disjoint. Sweep them in
        // parallel after the one shared inventory instead of multiplying slow
        // Docker round trips across the appliance readiness path.
        let sweep_started = std::time::Instant::now();
        if !hosted_runtime {
            tokio::join!(
                effect::sweep_orphans(&running_companies),
                staff::sweep_orphans(&recovery_daemon.orgintel, &running_companies)
            );
        }
        tracing::info!(
            elapsed_ms = sweep_started.elapsed().as_millis(),
            "runtime orphan recovery complete"
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
        if !hosted_runtime && !test_scheduler_disabled {
            for config in &recovery_configs {
                match recovery_daemon.orgintel.get(&config.name).await {
                    Ok(org) => {
                        tokio::spawn(native_harness::startup_doctor(
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
    // the asynchronous recovery task opens admission.
    let owner_daemon = std::sync::Arc::clone(&daemon);
    tokio::spawn(async move {
        if let Err(error) = owner::serve(owner_daemon, owner_config).await {
            tracing::error!("owner gateway stopped: {error:#}");
        }
    });
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
                        Ok((stream, _)) => {
                            let daemon = std::sync::Arc::clone(&tcp_daemon);
                            tokio::spawn(async move {
                                if let Err(error) =
                                    serve(stream, &daemon, ConnectionOrigin::RuntimeTcp).await
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

    // S03-T2: the world's front door, on its OWN listener and its own task.
    // The failure boundary is the point (AC6): a slow, malformed or flooded
    // inbound request must not be able to stall the scheduler — F12's lesson,
    // where one company's hung Docker took down all three. Absent secret means
    // absent rail: we do not open an unauthenticated public port, ever, and a
    // company that cannot receive is honest about it rather than silently open.
    let webhook_reference = std::env::var("RESTLESS_RESEND_WEBHOOK_CREDENTIAL")
        .unwrap_or_else(|_| "env:RESEND_WEBHOOK_SECRET".to_string());
    match credential::resolve_reference(&webhook_reference).await {
        Ok(secret) => {
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
                if let Err(error) = ingress::serve(port, secret, sink).await {
                    tracing::error!("event ingress stopped: {error:#}");
                }
            });
        }
        Err(error) => tracing::warn!(
            reference = %webhook_reference,
            "event ingress is NOT listening because its credential did not resolve: {error:#}. \
             The company can send but cannot receive; inbound replies will not wake it"
        ),
    }
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
        if let Err(error) = airwallex_ingress::serve(finance_daemon).await {
            tracing::error!("Airwallex event ingress stopped: {error:#}");
        }
    });

    // `restless-dev` stops the daemon with SIGTERM. Observe it inside the
    // runtime so owned model-broker/gateway child handles are dropped and
    // killed before the process exits; an abrupt default signal exit leaves
    // those children holding 7789/7790 and the next daemon half-attached to
    // stale supervision.
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, _) = accepted?;
                let daemon = std::sync::Arc::clone(&daemon);
                tokio::spawn(async move {
                    if let Err(error) =
                        serve(stream, &daemon, ConnectionOrigin::LocalOwner).await
                    {
                        tracing::warn!("connection error: {error:#}");
                    }
                });
            }
            () = &mut shutdown => {
                tracing::info!("shutdown requested; stopping supervised daemon children");
                break;
            }
        }
    }
    Ok(())
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
mod tests {
    use super::*;

    #[test]
    fn one_time_schedule_accepts_legacy_cli_default_without_accepting_recurring_policy() {
        let mut payload = serde_json::json!({
            "cmd":"schedule-add", "company":"schedule_test", "as_actor":"exec",
            "reason":"Observe consent expiry", "fire_at":"2026-09-22T04:08:30Z",
            "execution_requirement":"local-mac",
        });
        assert!(!has_recurring_schedule_fields(
            &decoded_request(payload.clone()).orgintel
        ));
        payload["execution_requirement"] = serde_json::Value::Null;
        assert!(!has_recurring_schedule_fields(
            &decoded_request(payload.clone()).orgintel
        ));
        for (field, value) in [
            ("execution_requirement", serde_json::json!("always-on")),
            ("recurrence", serde_json::json!("weekdays")),
            ("local_time", serde_json::json!("09:00")),
            ("timezone", serde_json::json!("Australia/Sydney")),
            ("missed_policy", serde_json::json!("skip")),
            ("catch_up_grace_seconds", serde_json::json!(60)),
        ] {
            let mut recurring = payload.clone();
            recurring[field] = value;
            assert!(
                has_recurring_schedule_fields(&decoded_request(recurring).orgintel),
                "must not silently ignore {field}"
            );
        }
    }

    fn decoded_request(value: serde_json::Value) -> Request {
        Request::decode(&value.to_string()).expect("decode request through the transport boundary")
    }

    #[test]
    fn nonstable_profiles_default_to_distinct_authority_databases() {
        let stable =
            restlessd::appliance::MachineProfile::stable(Path::new("/Users/founder")).unwrap();
        let dev = restlessd::appliance::MachineProfile {
            kind: restlessd::appliance::ProfileKind::Dev,
            state_root: PathBuf::from("/tmp/restless-dev-profile"),
            port_offset: 1_200,
            resource_namespace: "checkout_alpha".into(),
        };
        let test = restlessd::appliance::MachineProfile {
            kind: restlessd::appliance::ProfileKind::Test,
            state_root: PathBuf::from("/tmp/restless-test-profile"),
            port_offset: 21_200,
            resource_namespace: "checkout_alpha_test".into(),
        };
        let stable = OrgIntelConfig::default_for_profile(&stable);
        let dev = OrgIntelConfig::default_for_profile(&dev);
        let test = OrgIntelConfig::default_for_profile(&test);
        assert_eq!(database_target(&stable.database_url).unwrap().2, "restless");
        assert_eq!(
            database_target(&dev.database_url).unwrap().2,
            "restless_plane_checkout_alpha"
        );
        assert_eq!(
            database_target(&test.database_url).unwrap().2,
            "restless_plane_checkout_alpha_test"
        );
        assert_ne!(
            database_target(&stable.database_url).unwrap(),
            database_target(&dev.database_url).unwrap()
        );
        assert_ne!(
            database_target(&dev.database_url).unwrap(),
            database_target(&test.database_url).unwrap()
        );
    }

    #[test]
    fn hosted_plane_database_url_is_exact_password_authenticated_postgres() {
        validate_plane_database_url(
            "postgresql://restless:secret@plane-database:5432/restless?sslmode=require",
        )
        .unwrap();
        for invalid in [
            " postgres://restless:secret@plane-database/restless",
            "postgres://restless@plane-database/restless",
            "postgres://restless:secret@plane-database/",
            "postgres://restless:secret@plane-database/one/two",
            "https://restless:secret@plane-database/restless",
            "postgres://restless:secret@plane-database/restless#credential-copy",
        ] {
            assert!(
                validate_plane_database_url(invalid).is_err(),
                "accepted invalid plane database URL shape"
            );
        }
    }

    #[tokio::test]
    async fn an_unconfigured_company_cannot_recreate_a_destroyed_cell() {
        let root =
            std::env::temp_dir().join(format!("restless-cell-tombstone-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("companies")).unwrap();
        let registry = OrgIntelRegistry {
            database_url: "postgres://must-not-connect.invalid/restless".into(),
            root: root.clone(),
            handles: std::sync::Mutex::new(HashMap::new()),
        };
        let error = match registry.get("destroyed_test").await {
            Ok(_) => panic!("an unconfigured company unexpectedly acquired an OrgIntel cell"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("not configured"));
        assert!(!root.join("cells/destroyed_test/database.url").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn local_company_creation_persists_an_unconfigured_exec_without_launching() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping local creation scenario");
            return;
        };
        let parsed = url::Url::parse(&database_url).unwrap();
        assert!(
            parsed.path().ends_with("_test"),
            "local creation scenario requires a disposable test database"
        );
        let company = format!(
            "unconfigured_create_{}_test",
            &uuid::Uuid::new_v4().simple().to_string()[..10]
        );
        let root = std::env::temp_dir().join(&company);
        std::fs::create_dir_all(root.join("companies")).unwrap();
        let authority = authority::AuthorityStore::connect(&database_url)
            .await
            .unwrap();
        let daemon = Daemon {
            root: root.clone(),
            capabilities: capability::CapabilityIssuer::open(&root).unwrap(),
            spend: spend::SpendLedger::open(&root).unwrap(),
            publication: publication::PublicationManager::new(&root, authority.clone()).unwrap(),
            launch: launch::LaunchBroker::new(&root).unwrap(),
            authority,
            orgintel: OrgIntelRegistry {
                database_url: database_url.clone(),
                root: root.clone(),
                handles: std::sync::Mutex::new(HashMap::new()),
            },
            staff: staff::StaffRegistry::default(),
            activities: activity::AgentActivityStreams::default(),
            cell_wakes: cell_wake::CellWakeHub::default(),
            runtime_bridges: runtime_bridge::RuntimeBridgeRegistry::default(),
            lifecycle: restlessd::appliance::LifecycleGate::default(),
            in_flight: std::sync::Arc::new(std::sync::Mutex::new(schedule::WakeClaims::default())),
            schedule_wake: std::sync::Arc::new(tokio::sync::Notify::new()),
        };
        let config: runtime::CompanyConfig = toml::from_str(&format!(
            "name = {company:?}\nmission = \"Choose intelligence later\"\n"
        ))
        .unwrap();

        create_local_company_inner(&daemon, config, false)
            .await
            .unwrap();
        let restored = runtime::CompanyConfig::load(&root, &company).unwrap();
        assert!(restored.model_candidates().unwrap().is_empty());
        assert!(!restored.has_configured_model_route());
        let org = daemon.orgintel.get(&company).await.unwrap();
        assert_eq!(org.active_actor("exec").await.unwrap().unwrap().model, None);
        assert!(!daemon.staff.is_actor_running(&company, "exec"));
        let wake_error = schedule::run_exec_turn(
            &daemon,
            &restored,
            &org,
            "test wake must remain closed",
            &tokio_util::sync::CancellationToken::new(),
        )
        .await
        .unwrap_err();
        assert!(wake_error.to_string().contains("Intelligence provider"));
        assert!(!daemon.staff.is_actor_running(&company, "exec"));

        org.close().await;
        daemon.orgintel.forget(&company);
        daemon
            .authority
            .delete_test_company(&company)
            .await
            .unwrap();
        crate::cell::destroy_database(&root, &database_url, &company)
            .await
            .unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn exec_fast_path_is_only_the_unambiguous_coherent_single_worker_route() {
        let one_worker = vec!["product-builder".to_string()];
        assert!(work_commission_is_admitted(
            "exec",
            "product-direction",
            restless_orgintel::ProducingTopology::CoherentSingleWorker,
            &one_worker,
            "product-builder",
        ));
        assert!(!work_commission_is_admitted(
            "exec",
            "product-direction",
            restless_orgintel::ProducingTopology::LocallyClosingParallelUnit,
            &one_worker,
            "product-builder",
        ));
        assert!(!work_commission_is_admitted(
            "exec",
            "product-direction",
            restless_orgintel::ProducingTopology::CoherentSingleWorker,
            &["product-builder".into(), "product-reviewer".into()],
            "product-builder",
        ));
        assert!(work_commission_is_admitted(
            "product-direction",
            "product-direction",
            restless_orgintel::ProducingTopology::LocallyClosingParallelUnit,
            &[],
            "product-builder",
        ));
    }

    /// The hole this ticket closes: an agent inside the container asking for
    /// the owner's authority act. `main.rs:215` accepted this with the expiry
    /// "before any real external effect" — sprint 03 sent real email.
    #[test]
    fn the_company_may_not_perform_an_owner_authority_act() {
        for cmd in OWNER_ONLY {
            let refusal = authorize(Principal::CompanyExec, cmd)
                .expect_err("company/exec must not perform {cmd}");
            assert!(refusal.contains("owner authority"), "{cmd}: {refusal}");
        }
    }

    /// Sprint 05's administrative-surface guard. The dispatcher remains an
    /// intentionally ordinary match rather than a universal command algebra,
    /// so this test reads that match and fails when a new coordination verb is
    /// added without an owner CLI spelling. Open-ended Linux/browser work is
    /// deliberately outside this enumeration (`attach` is its door).
    /// ...and the gate must not break the agents' ordinary channel, which
    /// would be a worse bug than the one it fixes.
    #[test]
    fn the_company_keeps_its_coordination_channel() {
        for cmd in ["work", "message", "work-handoff-resolve", "effect", "inbox"] {
            assert_eq!(
                authorize(Principal::CompanyExec, cmd).unwrap(),
                Principal::CompanyExec,
                "{cmd} must stay open to the company"
            );
        }
    }

    /// TCP has no principal fallback. Local Unix derives owner identity from
    /// its listener, but a Runtime must carry a valid capability.
    #[test]
    fn runtime_legacy_principal_spelling_cannot_claim_owner() {
        assert!(Principal::legacy_runtime_claim(None).is_ok());
        assert!(Principal::legacy_runtime_claim(Some("company/exec")).is_ok());
        assert!(Principal::legacy_runtime_claim(Some("owner")).is_err());
        assert!(Principal::legacy_runtime_claim(Some("root")).is_err());
    }

    #[test]
    fn tcp_capability_derives_company_and_actor_before_dispatch() {
        let root =
            std::env::temp_dir().join(format!("restless-tcp-auth-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let issuer = capability::CapabilityIssuer::open(&root).unwrap();
        let token = issuer
            .issue_actor_session("acme_test", "delivery-lead", "session_1", None, None)
            .unwrap();

        let mut valid = decoded_request(serde_json::json!({
            "cmd": "message",
            "company": "acme_test",
            "principal": "company/exec",
            "session_capability": token,
            "from": "delivery-lead",
            "to": "exec",
            "body": "native result is ready"
        }));
        assert_eq!(
            authenticate_request(&mut valid, &issuer, ConnectionOrigin::RuntimeTcp).unwrap(),
            Principal::CompanyExec
        );
        assert_eq!(valid.company.as_deref(), Some("acme_test"));
        assert_eq!(valid.common.from.as_deref(), Some("delivery-lead"));

        for (command, field) in [
            ("document-operation", "document_operation"),
            ("sheet-operation", "sheet_operation"),
            ("room-operation", "room_operation"),
        ] {
            let token = issuer
                .issue_actor_session(
                    "acme_test",
                    "delivery-lead",
                    "collaboration_session",
                    None,
                    None,
                )
                .unwrap();
            let value = serde_json::json!({"cmd":command,"company":"acme_test","session_capability":token,field:{"operation":"list"}});
            let mut collaboration = decoded_request(value.clone());
            authenticate_request(&mut collaboration, &issuer, ConnectionOrigin::RuntimeTcp)
                .unwrap();
            assert_eq!(
                collaboration.orgintel.actor.as_deref(),
                Some("delivery-lead")
            );
            let mut forged = value;
            forged["actor"] = serde_json::json!("owner");
            assert!(authenticate_request(
                &mut decoded_request(forged),
                &issuer,
                ConnectionOrigin::RuntimeTcp
            )
            .is_err());
        }

        let mut owner_claim = decoded_request(serde_json::json!({
            "cmd": "approve",
            "company": "acme_test",
            "principal": "owner",
            "session_capability": issuer
                .issue_actor_session("acme_test", "delivery-lead", "session_2", None, None)
                .unwrap()
        }));
        assert!(
            authenticate_request(&mut owner_claim, &issuer, ConnectionOrigin::RuntimeTcp)
                .unwrap_err()
                .contains("may not claim owner")
        );

        let mut foreign_company = decoded_request(serde_json::json!({
            "cmd": "message",
            "company": "other_test",
            "session_capability": issuer
                .issue_actor_session("acme_test", "delivery-lead", "session_3", None, None)
                .unwrap(),
            "from": "delivery-lead",
            "body": "forged"
        }));
        assert!(
            authenticate_request(&mut foreign_company, &issuer, ConnectionOrigin::RuntimeTcp)
                .is_err()
        );

        let mut foreign_actor = decoded_request(serde_json::json!({
            "cmd": "message",
            "company": "acme_test",
            "session_capability": issuer
                .issue_actor_session("acme_test", "delivery-lead", "session_4", None, None)
                .unwrap(),
            "from": "exec",
            "body": "forged"
        }));
        assert!(
            authenticate_request(&mut foreign_actor, &issuer, ConnectionOrigin::RuntimeTcp)
                .unwrap_err()
                .contains("cannot claim")
        );

        let work_id = uuid::Uuid::new_v4();
        let attempt_id = uuid::Uuid::new_v4();
        let document_id = uuid::Uuid::new_v4();
        let named_version_id = uuid::Uuid::new_v4();
        let command_id = uuid::Uuid::new_v4();
        let scoped_token = issuer
            .issue_actor_session(
                "acme_test",
                "delivery-lead",
                "session_document",
                Some(work_id),
                Some(attempt_id),
            )
            .unwrap();
        let document_request = |token: &str, requested_attempt: uuid::Uuid| {
            decoded_request(serde_json::json!({
                "cmd": "document-review-request",
                "company": "acme_test",
                "session_capability": token,
                "document_id": document_id,
                "expected_document_version": 3,
                "named_version_id": named_version_id,
                "document_work_id": work_id,
                "document_attempt_id": requested_attempt,
                "expected_work_revision": 1,
                "reviewer_actor_id": "alex",
                "review_summary": "Review this exact named version",
                "document_command_id": command_id,
            }))
        };
        let mut valid_document = document_request(&scoped_token, attempt_id);
        assert_eq!(
            authenticate_request(&mut valid_document, &issuer, ConnectionOrigin::RuntimeTcp,)
                .unwrap(),
            Principal::CompanyExec
        );
        assert_eq!(
            valid_document.orgintel.actor.as_deref(),
            Some("delivery-lead")
        );
        let mut mixed_attempt = document_request(&scoped_token, uuid::Uuid::new_v4());
        assert!(
            authenticate_request(&mut mixed_attempt, &issuer, ConnectionOrigin::RuntimeTcp,)
                .unwrap_err()
                .contains("do not match the signed ActorSession")
        );
        let bridge_token = issuer.issue_runtime_bridge("acme_test").unwrap();
        let mut ambient_exec = document_request(&bridge_token, attempt_id);
        assert!(
            authenticate_request(&mut ambient_exec, &issuer, ConnectionOrigin::RuntimeTcp,)
                .unwrap_err()
                .contains("Work-bound ActorSession")
        );
        let mut human_document = document_request("unused", attempt_id);
        assert!(
            authenticate_request(&mut human_document, &issuer, ConnectionOrigin::LocalOwner,)
                .unwrap_err()
                .contains("hosted Runtime Attempt operation")
        );

        let mut local = decoded_request(serde_json::json!({
            "cmd": "approve",
            "company": "acme_test",
            "principal": "company/exec"
        }));
        assert_eq!(
            authenticate_request(&mut local, &issuer, ConnectionOrigin::LocalOwner).unwrap(),
            Principal::Owner
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn the_owner_may_do_everything_the_company_may_and_more() {
        for cmd in OWNER_ONLY {
            assert_eq!(authorize(Principal::Owner, cmd).unwrap(), Principal::Owner);
        }
        assert_eq!(
            authorize(Principal::Owner, "goals").unwrap(),
            Principal::Owner
        );
    }

    #[test]
    fn runtime_attribution_fields_are_pinned_to_the_authenticated_actor() {
        let mut request = decoded_request(serde_json::json!({
            "cmd": "connected-tool-attach", "company": "acme_test",
            "tool_name": "crm", "work_id": uuid::Uuid::new_v4().to_string()
        }));
        bind_runtime_actor(&mut request, "lead").unwrap();
        assert_eq!(request.orgintel.actor.as_deref(), Some("lead"));
        assert!(bind_runtime_actor(&mut request, "impostor").is_err());

        let mut document = decoded_request(serde_json::json!({
            "cmd": "document-operation",
            "company": "acme_test",
            "document_operation": {"operation":"list", "cursor":null, "archived":false}
        }));
        bind_runtime_actor(&mut document, "writer").unwrap();
        assert_eq!(document.orgintel.actor.as_deref(), Some("writer"));
        assert!(bind_runtime_actor(&mut document, "impostor").is_err());
    }
}
