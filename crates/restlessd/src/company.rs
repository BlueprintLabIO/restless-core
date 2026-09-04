//! Source-aware projection for the stable owner-facing Company area.
//!
//! This module deliberately owns no company state. It composes Authority,
//! OrgIntel and Runtime observations and invokes only the Runtime lifecycle
//! actions already owned by the daemon.

use std::collections::BTreeSet;

use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Utc};
use restless_orgintel::{ArtifactRefRow, ScheduleRow};
use serde::Serialize;

use crate::{airwallex, approval, credential, finance, legal, reconcile, runtime, Daemon};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RecoveryAction {
    Start,
    Restart,
    Reconcile,
}

impl RecoveryAction {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "start" => Some(Self::Start),
            "restart" => Some(Self::Restart),
            "reconcile" => Some(Self::Reconcile),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Restart => "restart",
            Self::Reconcile => "reconcile",
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct RecoveryOutcome {
    action: RecoveryAction,
    message: String,
    doctor: runtime::RuntimeDoctor,
}

#[derive(Debug, Serialize)]
pub(crate) struct CompanyView {
    company: CompanyIdentity,
    sources: Sources,
    charter: Charter,
    limits: Limits,
    harnesses: HarnessSettings,
    resources: Resources,
    external_actions: ExternalActions,
    computer: CompanyComputer,
    attention_href: String,
    refreshed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct CompanyIdentity {
    id: String,
    name: String,
    outcome_standard: restless_orgintel::OutcomeStandard,
}

#[derive(Debug, Serialize)]
struct Sources {
    authority: SourceObservation,
    orgintel: SourceObservation,
    runtime: SourceObservation,
}

#[derive(Debug, Clone, Serialize)]
struct SourceObservation {
    status: &'static str,
    observed_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

impl SourceObservation {
    fn available(observed_at: DateTime<Utc>) -> Self {
        Self {
            status: "available",
            observed_at,
            detail: None,
        }
    }

    fn unavailable(observed_at: DateTime<Utc>, error: impl std::fmt::Display) -> Self {
        Self {
            status: "unavailable",
            observed_at,
            detail: Some(error.to_string()),
        }
    }
}

#[derive(Debug, Serialize)]
struct Charter {
    purpose: String,
    source: &'static str,
    revision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    effective_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    legal_identity: Option<LegalIdentity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_direction: Option<CurrentDirection>,
    current_direction_status: &'static str,
}

#[derive(Debug, Serialize)]
struct LegalIdentity {
    legal_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    trading_name: Option<String>,
    entity_type: String,
    jurisdiction: String,
    owner_asserted_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct CurrentDirection {
    id: uuid::Uuid,
    title: String,
    body: String,
    href: String,
    observed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct Limits {
    status: &'static str,
    independently: Vec<LimitStatement>,
    asks_owner: Vec<LimitStatement>,
    cannot: Vec<LimitStatement>,
    approved_parties: Vec<String>,
    spend: SpendLimit,
    money_envelopes: Vec<finance::MoneyEnvelope>,
}

#[derive(Debug, Serialize)]
struct LimitStatement {
    title: &'static str,
    explanation: &'static str,
}

#[derive(Debug, Serialize)]
struct SpendLimit {
    model: String,
    accounted_usd: f64,
    ceiling_usd: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    remaining_usd: Option<f64>,
    status: &'static str,
}

#[derive(Debug, Serialize)]
struct Resources {
    status: &'static str,
    items: Vec<ResourceRow>,
}

#[derive(Debug, Serialize)]
struct HarnessSettings {
    coordination: runtime::AgentHarness,
    worker: runtime::AgentHarness,
    options: Vec<HarnessOption>,
}

#[derive(Debug, Serialize)]
struct HarnessOption {
    id: runtime::AgentHarness,
    label: &'static str,
    transport: &'static str,
    expected_build: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_build: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    native_agent_build: Option<String>,
    status: &'static str,
    detail: String,
    authentication: &'static str,
    limitations: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct ResourceRow {
    id: String,
    label: String,
    kind: &'static str,
    source: &'static str,
    status: String,
    observed_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    launch: Option<crate::launch::ArtifactLaunchDescriptor>,
}

#[derive(Debug, Serialize)]
struct ExternalActions {
    status: &'static str,
    items: Vec<ExternalActionRow>,
}

#[derive(Debug, Serialize)]
struct ExternalActionRow {
    id: String,
    title: String,
    effect_class: String,
    source: &'static str,
    state: String,
    evidence: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    actor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    party: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    receipt_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
    observed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct CompanyComputer {
    doctor: CompanyDoctor,
    #[serde(skip_serializing_if = "Option::is_none")]
    runtime: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation: Option<String>,
}

#[derive(Debug, Serialize)]
struct CompanyDoctor {
    status: &'static str,
    observed_at: DateTime<Utc>,
    checks: Vec<DoctorCheck>,
    actions: Vec<DoctorAction>,
}

#[derive(Debug, Serialize)]
struct DoctorCheck {
    id: &'static str,
    label: &'static str,
    source: &'static str,
    status: &'static str,
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

#[derive(Debug, Serialize)]
struct DoctorAction {
    id: RecoveryAction,
    label: &'static str,
    consequence: &'static str,
    confirmation: &'static str,
}

struct AuthorityProjection {
    approved_parties: Vec<String>,
    effect_receipts: Vec<crate::authority::AuthorityRecord>,
    effect_intents: Vec<crate::authority::AuthorityRecord>,
    reconciliations: Vec<crate::authority::AuthorityRecord>,
    legal_profile: Option<legal::LegalProfile>,
    provider: Option<airwallex::Connection>,
    envelopes: Vec<finance::MoneyEnvelope>,
    payments: Vec<finance::PaymentIntent>,
    cooldowns: Vec<crate::authority::ModelCooldown>,
    publications: Vec<crate::authority::AuthorityRecord>,
}

pub(crate) async fn project(
    daemon: &Daemon,
    config: &runtime::CompanyConfig,
    probe_credentials: bool,
) -> CompanyView {
    let observed_at = Utc::now();

    let org_result = match daemon.orgintel.get(&config.name).await {
        Ok(org) => tokio::try_join!(
            org.list_goals(),
            org.list_artifact_refs(None),
            org.list_schedules(None, true),
        )
        .map_err(|error| format!("{error:#}")),
        Err(error) => Err(format!("{error:#}")),
    };
    let (goals, artifacts, schedules, orgintel_source) = match org_result {
        Ok((goals, artifacts, schedules)) => (
            goals,
            artifacts,
            schedules,
            SourceObservation::available(observed_at),
        ),
        Err(error) => (
            Vec::new(),
            Vec::new(),
            Vec::new(),
            SourceObservation::unavailable(observed_at, error),
        ),
    };

    let authority_result = read_authority(daemon, &config.name).await;
    let (authority, authority_source) = match authority_result {
        Ok(authority) => (Some(authority), SourceObservation::available(observed_at)),
        Err(error) => (
            None,
            SourceObservation::unavailable(observed_at, format!("{error:#}")),
        ),
    };

    let runtime_result = runtime::doctor(&config.name).await;
    let (runtime_doctor, runtime_source) = match runtime_result {
        Ok(doctor) => (Some(doctor), SourceObservation::available(observed_at)),
        Err(error) => (
            None,
            SourceObservation::unavailable(observed_at, format!("{error:#}")),
        ),
    };

    let budget = daemon.spend.budget_state(config);
    let accounted_usd = budget.accounted_micro_usd() as f64 / 1_000_000.0;
    let spend = SpendLimit {
        model: config.model.clone(),
        accounted_usd: round_usd(accounted_usd),
        ceiling_usd: config.spend_ceiling_usd.as_usd(),
        remaining_usd: budget
            .remaining_micro_usd()
            .map(|remaining| round_usd(remaining as f64 / 1_000_000.0)),
        status: match budget {
            crate::spend::ModelBudgetState::Available { .. } => "available",
            crate::spend::ModelBudgetState::Exhausted { .. } => "exhausted",
            crate::spend::ModelBudgetState::MeteringUnknown { .. } => "metering_unknown",
        },
    };

    let legal_identity = authority
        .as_ref()
        .and_then(|value| value.legal_profile.as_ref())
        .map(|profile| LegalIdentity {
            legal_name: profile.safe.legal_name.clone(),
            trading_name: profile.safe.trading_name.clone(),
            entity_type: profile.safe.entity_type.clone(),
            jurisdiction: profile.safe.jurisdiction.clone(),
            owner_asserted_at: profile.owner_asserted_at,
        });
    let current_direction = goals
        .iter()
        .filter(|goal| goal.closed_at.is_none())
        .min_by_key(|goal| goal.created_at)
        .map(|goal| CurrentDirection {
            id: goal.id,
            title: goal.title.clone(),
            body: goal.body.clone(),
            href: format!("/{}/work", config.name),
            observed_at,
        });
    let effective_at = std::fs::metadata(
        daemon
            .root
            .join("companies")
            .join(format!("{}.toml", config.name)),
    )
    .ok()
    .and_then(|metadata| metadata.modified().ok())
    .map(DateTime::<Utc>::from);

    let limits = Limits {
        status: if authority.is_some() {
            "available"
        } else {
            "unavailable"
        },
        independently: vec![
            LimitStatement {
                title: "Internal company work",
                explanation: "Exec and Staff may plan, research, edit company files, build and coordinate inside the Company computer.",
            },
            LimitStatement {
                title: "Model use inside the spend ceiling",
                explanation: "Configured models may be used while metered spend remains inside the owner-set company ceiling.",
            },
            LimitStatement {
                title: "Already granted external parties",
                explanation: "Governed effects may proceed only through their source-owned checks and existing grants.",
            },
        ],
        asks_owner: vec![
            LimitStatement {
                title: "First consequential contact",
                explanation: "A new external party is prepared and brought to Attention before the first real effect.",
            },
            LimitStatement {
                title: "Irreducible human steps",
                explanation: "Identity, legal attestation, provider approval, payment confirmation and owner judgement remain owner actions in Attention.",
            },
        ],
        cannot: vec![
            LimitStatement {
                title: "Expand its own mandate",
                explanation: "The company cannot grant itself new authority, rewrite the owner mandate or approve its own expansion.",
            },
            LimitStatement {
                title: "Reach host or provider-root custody",
                explanation: "Host control, raw secrets, provider-root credentials and owner authentication stay outside the Company computer.",
            },
            LimitStatement {
                title: "Rewrite external history",
                explanation: "Receipts and unknown outcomes are reconciled forward; they are never edited into a more convenient result.",
            },
        ],
        approved_parties: authority
            .as_ref()
            .map(|value| value.approved_parties.clone())
            .unwrap_or_default(),
        spend,
        money_envelopes: authority
            .as_ref()
            .map(|value| value.envelopes.clone())
            .unwrap_or_default(),
    };

    let resources = resources(
        config,
        authority.as_ref(),
        runtime_doctor.as_ref(),
        &artifacts,
        &schedules,
        probe_credentials,
    )
    .await;
    let harnesses = harness_settings(config, runtime_doctor.as_ref());
    let external_actions = actions(authority.as_ref());
    let company_doctor = company_doctor(
        authority_source.clone(),
        orgintel_source.clone(),
        runtime_source.clone(),
        runtime_doctor.as_ref(),
        observed_at,
    );
    let generation = runtime::generation(&config.name).await.ok().flatten();
    let runtime_json = runtime_doctor
        .as_ref()
        .and_then(|doctor| serde_json::to_value(doctor).ok());

    CompanyView {
        company: CompanyIdentity {
            id: config.name.clone(),
            name: display_name(&config.name),
            outcome_standard: config.outcome_standard,
        },
        sources: Sources {
            authority: authority_source,
            orgintel: orgintel_source.clone(),
            runtime: runtime_source,
        },
        charter: Charter {
            purpose: config.mission.clone(),
            source: "authority_config",
            revision: crate::authority::mandate_revision(&config.mission),
            effective_at,
            legal_identity,
            current_direction,
            current_direction_status: orgintel_source.status,
        },
        limits,
        harnesses,
        resources,
        external_actions,
        computer: CompanyComputer {
            doctor: company_doctor,
            runtime: runtime_json,
            generation,
        },
        attention_href: format!("/{}/attention", config.name),
        refreshed_at: observed_at,
    }
}

fn harness_settings(
    config: &runtime::CompanyConfig,
    doctor: Option<&runtime::RuntimeDoctor>,
) -> HarnessSettings {
    let coordination_available = doctor
        .and_then(|doctor| doctor.coordination.as_ref())
        .is_some_and(|coordination| coordination.status == "available");
    let unstartable = crate::model_gateway::unstartable_reason(&config.name);
    let options = [
        runtime::AgentHarness::RestlessManaged,
        runtime::AgentHarness::Codex,
        runtime::AgentHarness::ClaudeAgent,
    ]
    .into_iter()
    .map(|harness| {
        let expected_build = harness.build();
        let observed_build = doctor
            .and_then(|doctor| doctor.release.as_ref())
            .and_then(|release| release.harnesses.get(harness.as_str()))
            .cloned();
        let expected_native_agent = harness.native_agent_build();
        let observed_native_agent = expected_native_agent.and_then(|_| {
            doctor
                .and_then(|doctor| doctor.release.as_ref())
                .and_then(|release| release.harness_agents.get(harness.as_str()))
                .cloned()
        });
        let package_matches = observed_build.as_deref() == Some(expected_build)
            && expected_native_agent
                .is_none_or(|expected| observed_native_agent.as_deref() == Some(expected));
        let mut candidate = config.clone();
        candidate.coordination_harness = harness;
        candidate.worker_harness = harness;
        let incompatibility = candidate
            .validate_harness_models()
            .err()
            .map(|error| error.to_string());
        let (status, detail) = if let Some(reason) = incompatibility {
            (
                "incompatible",
                format!("The current model/effort policy is incompatible: {reason}"),
            )
        } else if !package_matches {
            (
                "not_ready",
                match observed_build.as_deref() {
                    Some(observed) => format!(
                        "Runtime reported {observed}; this release requires {expected_build}. Reconcile the Company computer."
                    ),
                    None => "The pinned build was not observed from the running Company computer. Start or reconcile it before selecting this harness.".into(),
                },
            )
        } else if let Some(reason) = unstartable.as_deref() {
            ("not_ready", format!("Provider access is not ready: {reason}"))
        } else if !coordination_available {
            (
                "not_ready",
                "The Runtime coordination path is not ready; start or reconcile the Company computer."
                    .into(),
            )
        } else {
            (
                "ready",
                "Pinned build, model policy, provider admission, and Runtime coordination are ready."
                    .into(),
            )
        };
        let (label, transport, authentication, limitations) = match harness {
            runtime::AgentHarness::RestlessManaged => (
                "Restless Managed",
                "ACP",
                "Host-managed provider route; Runtime receives only a scoped session capability.",
                vec!["Input submitted during a running turn is queued for the next turn."],
            ),
            runtime::AgentHarness::Codex => (
                "Codex",
                "Native App Server",
                "Host OpenAI-compatible route; Runtime receives only a scoped session capability.",
                vec![
                    "Optional native events appear only when the App Server reports them.",
                    "Input submitted during a running turn is queued unless native acknowledgement is observed.",
                ],
            ),
            runtime::AgentHarness::ClaudeAgent => (
                "Claude Agent",
                "ACP",
                "Host Anthropic API key through the scoped relay; Claude subscription login is unsupported.",
                vec![
                    "API authentication only; claude.ai subscription login is not supported.",
                    "Input submitted during a running turn is queued for the next turn.",
                ],
            ),
        };
        HarnessOption {
            id: harness,
            label,
            transport,
            expected_build,
            observed_build,
            native_agent_build: observed_native_agent,
            status,
            detail,
            authentication,
            limitations,
        }
    })
    .collect();
    HarnessSettings {
        coordination: config.coordination_harness,
        worker: config.worker_harness,
        options,
    }
}

async fn read_authority(daemon: &Daemon, company: &str) -> Result<AuthorityProjection> {
    let (
        approved_parties,
        effect_receipts,
        effect_intents,
        reconciliations,
        legal_profile,
        provider,
        envelopes,
        payments,
        cooldowns,
        publications,
    ) = tokio::try_join!(
        approval::approved_parties(&daemon.authority, company),
        daemon.authority.records_of_kind(company, "effect"),
        daemon.authority.records_of_kind(company, "effect_intent"),
        daemon
            .authority
            .records_of_kind(company, "effect_reconciled"),
        legal::get_profile(&daemon.authority, company),
        airwallex::connection(&daemon.authority, company),
        finance::envelopes(&daemon.authority, company),
        finance::payments(&daemon.authority, company),
        daemon.authority.active_model_cooldowns(company),
        publication_records(&daemon.authority, company),
    )?;
    Ok(AuthorityProjection {
        approved_parties,
        effect_receipts,
        effect_intents,
        reconciliations,
        legal_profile,
        provider,
        envelopes,
        payments,
        cooldowns,
        publications,
    })
}

async fn publication_records(
    authority: &crate::authority::AuthorityStore,
    company: &str,
) -> Result<Vec<crate::authority::AuthorityRecord>> {
    let mut records = Vec::new();
    for kind in [
        "publication_request",
        "publication_authorized",
        "publication_resource_grant",
        "publication_ready",
        "publication_recovered",
        "publication_failed",
        "publication_stopped",
        "publication_cleanup",
    ] {
        records.extend(authority.records_of_kind(company, kind).await?);
    }
    records.sort_by_key(|record| record.id);
    Ok(records)
}

async fn resources(
    config: &runtime::CompanyConfig,
    authority: Option<&AuthorityProjection>,
    doctor: Option<&runtime::RuntimeDoctor>,
    artifacts: &[ArtifactRefRow],
    schedules: &[ScheduleRow],
    probe_credentials: bool,
) -> Resources {
    let observed_at = Utc::now();
    let mut items = Vec::new();
    let model_cooldown = authority.and_then(|value| {
        value
            .cooldowns
            .iter()
            .find(|cooldown| cooldown.model == config.model)
    });
    items.push(ResourceRow {
        id: "model:primary".into(),
        label: config.model.clone(),
        kind: "model_access",
        source: "authority_config",
        status: if model_cooldown.is_some() {
            "degraded"
        } else {
            "configured_unprobed"
        }
        .into(),
        observed_at,
        detail: model_cooldown.map(|cooldown| {
            format!(
                "{}; retry after {}",
                cooldown.reason, cooldown.retry_at
            )
        }).or_else(|| Some("Configured model route; this read does not spend tokens to claim that generation works.".into())),
        metadata: Some(serde_json::json!({
            "fallbacks": config.model_failover,
            "coordination_harness": config.coordination_harness,
            "worker_harness": config.worker_harness,
            "reasoning_effort": config.reasoning_effort,
        })),
        launch: None,
    });

    for (binding, reference) in &config.credentials {
        let (status, detail) = if probe_credentials {
            let probe = credential::probe_reference(reference).await;
            (probe.status.as_str().to_string(), probe.detail)
        } else {
            (
                "configured_unprobed".into(),
                Some("A governed reference exists; its target was not probed by this read.".into()),
            )
        };
        items.push(ResourceRow {
            id: format!("credential:{binding}"),
            label: binding.clone(),
            kind: "credential_reference",
            source: "authority_config",
            status,
            observed_at,
            detail,
            metadata: None,
            launch: None,
        });
    }

    if let Some(provider) = authority.and_then(|value| value.provider.as_ref()) {
        let status = if provider.configured.observed_at.is_some() {
            "observed"
        } else {
            "configured_unprobed"
        };
        items.push(ResourceRow {
            id: "provider:airwallex".into(),
            label: "Airwallex".into(),
            kind: "provider_account",
            source: "authority",
            status: status.into(),
            observed_at: provider.configured.observed_at.unwrap_or(provider.updated_at),
            detail: Some(if status == "observed" {
                "Provider configuration carries a timestamped account observation; current authentication is not inferred.".into()
            } else {
                "Provider account is configured but has no current live observation.".into()
            }),
            metadata: Some(serde_json::json!({
                "environment": provider.configured.environment,
                "account_ref": provider.configured.account_ref,
                "read_scopes": provider.configured.read_scopes,
                "submit_scopes": provider.configured.submit_scopes,
            })),
            launch: None,
        });
    }

    if let Some(supervisor) = doctor.and_then(|value| value.supervisor.as_ref()) {
        for service in &supervisor.services {
            items.push(ResourceRow {
                id: format!("runtime-service:{}", service.name),
                label: service.name.clone(),
                kind: "runtime_service",
                source: "runtime",
                status: service.state.clone(),
                observed_at,
                detail: Some("Observed from the Company computer process supervisor.".into()),
                metadata: None,
                launch: None,
            });
        }
    }

    if let Some(authority) = authority {
        for requested in authority
            .publications
            .iter()
            .filter(|record| record.body.get("request").is_some())
        {
            let Some(publication_id) = requested
                .body
                .get("publication_id")
                .and_then(serde_json::Value::as_str)
            else {
                continue;
            };
            let related = |record: &&crate::authority::AuthorityRecord| {
                record
                    .body
                    .get("publication_id")
                    .and_then(serde_json::Value::as_str)
                    == Some(publication_id)
            };
            let latest = authority
                .publications
                .iter()
                .filter(related)
                .max_by_key(|record| record.id)
                .unwrap_or(requested);
            let status = if latest.body.get("cleaned_at").is_some() {
                "cleaned"
            } else if latest.body.get("stopped_at").is_some() {
                "stopped"
            } else if latest.body.get("error").is_some() {
                "failed"
            } else if latest.body.get("receipt").is_some() {
                "ready"
            } else if latest.body.get("authorized_at").is_some()
                || latest.body.get("resources").is_some() && latest.body.get("provider").is_some()
            {
                "authorized"
            } else {
                "awaiting_owner_authorization"
            };
            let request = requested.body.get("request").cloned().unwrap_or_default();
            let profile = request
                .pointer("/candidate/manifest/profile")
                .and_then(serde_json::Value::as_str);
            let expires_at = request
                .get("expires_at")
                .and_then(serde_json::Value::as_str)
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&Utc));
            let availability = if expires_at.is_some_and(|expiry| expiry <= observed_at) {
                "expired"
            } else {
                match status {
                    "ready" if profile == Some("https-websocket-demo") => "ready",
                    "ready" => "unavailable",
                    "authorized" => "preparing",
                    "stopped" | "cleaned" => "stopped",
                    _ => "unavailable",
                }
            };
            let launch = profile.map(|profile| crate::launch::ArtifactLaunchDescriptor {
                contract_version: crate::launch::CONTRACT_VERSION,
                shape: if profile == "https-websocket-demo" {
                    crate::launch::LaunchShape::EmbeddedWeb
                } else {
                    crate::launch::LaunchShape::NativeClient
                },
                availability: availability.into(),
                detail: if profile == "https-websocket-demo" {
                    match availability {
                        "ready" => "Open the exact released web artifact inside Restless.",
                        "preparing" => "The released web artifact is still preparing.",
                        "expired" => "The publication access window has expired.",
                        "stopped" => "The publication has been stopped.",
                        _ => "The released web artifact is not currently reachable.",
                    }
                } else {
                    "This is the authoritative native server. Open its matching verified client artifact."
                }
                .into(),
                open_endpoint: format!(
                    "/api/companies/{}/resources/published-service:{publication_id}/open",
                    config.name
                ),
                artifact_digest: request
                    .pointer("/candidate/image")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|image| image.rsplit_once('@').map(|(_, digest)| digest.to_string())),
                candidate_digest: request
                    .pointer("/candidate/manifest_digest")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                work_id: request
                    .pointer("/candidate/work_id")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                attempt_id: request
                    .pointer("/candidate/attempt_id")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                audience: request
                    .get("audience")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                expires_at,
                platform: (profile == "godot-enet-udp").then(|| "macos-arm64".into()),
                publication_id: Some(publication_id.to_string()),
                runtime_generation: request
                    .pointer("/candidate/runtime_generation")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
            });
            items.push(ResourceRow {
                id: format!("published-service:{publication_id}"),
                label: publication_id.to_string(),
                kind: "published_service",
                source: "authority",
                status: status.into(),
                observed_at: latest.created_at,
                detail: Some(if status == "awaiting_owner_authorization" {
                    "Prepared by the company; the exact audience, expiry, resource envelope and immutable build await owner authorization.".into()
                } else {
                    "Bounded provider state; the company Runtime itself has no public route.".into()
                }),
                metadata: Some(serde_json::json!({
                    "image": request.pointer("/candidate/image"),
                    "manifest_digest": request.pointer("/candidate/manifest_digest"),
                    "work_id": request.pointer("/candidate/work_id"),
                    "attempt_id": request.pointer("/candidate/attempt_id"),
                    "runtime_generation": request.pointer("/candidate/runtime_generation"),
                    "profile": request.pointer("/candidate/manifest/profile"),
                    "audience": request.get("audience"),
                    "expires_at": request.get("expires_at"),
                    "resources": request.get("resources"),
                })),
                launch,
            });
        }

        for artifact in artifacts {
            if artifact.kind != "native_client_release"
                || artifact.state != restless_orgintel::ArtifactRefState::Available
            {
                continue;
            }
            let manifest = match crate::launch::NativeClientRelease::parse(artifact) {
                Ok(manifest) => manifest,
                Err(error) => {
                    items.push(ResourceRow {
                        id: format!("artifact:{}", artifact.id),
                        label: artifact.label.clone(),
                        kind: "native_client",
                        source: "orgintel",
                        status: "unavailable".into(),
                        observed_at: artifact.created_at,
                        detail: Some(format!("Native client release is invalid: {error}")),
                        metadata: Some(serde_json::json!({
                            "digest": artifact.digest,
                            "work_id": artifact.work_id,
                            "attempt_id": artifact.attempt_id,
                        })),
                        launch: None,
                    });
                    continue;
                }
            };
            let related = |record: &&crate::authority::AuthorityRecord| {
                record
                    .body
                    .get("publication_id")
                    .and_then(serde_json::Value::as_str)
                    == Some(manifest.publication_id.as_str())
            };
            let request = authority
                .publications
                .iter()
                .find(|record| related(record) && record.body.get("request").is_some());
            let latest = authority
                .publications
                .iter()
                .filter(related)
                .max_by_key(|row| row.id);
            let expires_at = request
                .and_then(|row| row.body.pointer("/request/expires_at"))
                .and_then(serde_json::Value::as_str)
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .map(|value| value.with_timezone(&Utc));
            let availability = if expires_at.is_some_and(|expiry| expiry <= observed_at) {
                "expired"
            } else if latest.is_some_and(|row| {
                row.body.get("stopped_at").is_some() || row.body.get("cleaned_at").is_some()
            }) {
                "stopped"
            } else if latest.is_some_and(|row| row.body.get("receipt").is_some()) {
                "ready"
            } else if request.is_some() {
                "preparing"
            } else {
                "unavailable"
            };
            items.push(ResourceRow {
                id: format!("artifact:{}", artifact.id),
                label: artifact.label.clone(),
                kind: "native_client",
                source: "orgintel",
                status: availability.into(),
                observed_at: artifact.created_at,
                detail: Some(match availability {
                    "ready" => "Verified native client for the matching authoritative session.",
                    "preparing" => "The matching authoritative session is still preparing.",
                    "expired" => "The matching session has expired.",
                    "stopped" => "The matching session was stopped.",
                    _ => "No matching authoritative session is available.",
                }.into()),
                metadata: Some(serde_json::json!({
                    "digest": artifact.digest,
                    "work_id": artifact.work_id,
                    "attempt_id": artifact.attempt_id,
                    "platform": manifest.platform,
                    "publication_id": manifest.publication_id,
                })),
                launch: Some(crate::launch::ArtifactLaunchDescriptor {
                    contract_version: crate::launch::CONTRACT_VERSION,
                    shape: crate::launch::LaunchShape::NativeClient,
                    availability: availability.into(),
                    detail: "Verify the exact archive, launch locally and exchange one short-lived session handle.".into(),
                    open_endpoint: format!(
                        "/api/companies/{}/resources/artifact:{}/open",
                        config.name, artifact.id
                    ),
                    artifact_digest: artifact.digest.clone(),
                    candidate_digest: request
                        .and_then(|row| row.body.pointer("/request/candidate/manifest_digest"))
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string),
                    work_id: artifact.work_id.map(|id| id.to_string()),
                    attempt_id: artifact.attempt_id.map(|id| id.to_string()),
                    audience: request
                        .and_then(|row| row.body.pointer("/request/audience"))
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string),
                    expires_at,
                    platform: Some(manifest.platform),
                    publication_id: Some(manifest.publication_id),
                    runtime_generation: artifact.runtime_generation.clone(),
                }),
            });
        }
    }

    if let Some(doctor) = doctor {
        let ready = doctor.container == runtime::ContainerStatus::Running;
        items.push(ResourceRow {
            id: "company-computer".into(),
            label: "Company Computer".into(),
            kind: "company_computer",
            source: "runtime",
            status: if ready { "ready" } else { "unavailable" }.into(),
            observed_at,
            detail: Some(
                if ready {
                    "Private streamed fallback for visual or non-packaged work."
                } else {
                    "The Company Computer is not running."
                }
                .into(),
            ),
            metadata: None,
            launch: Some(crate::launch::ArtifactLaunchDescriptor::computer(
                &config.name,
                ready,
            )),
        });
    }

    for schedule in schedules
        .iter()
        .filter(|schedule| schedule.cancelled_at.is_none())
    {
        let always_on = schedule.machine_requirement == "always_on";
        items.push(ResourceRow {
            id: format!("schedule:{}", schedule.id),
            label: schedule.reason.clone(),
            kind: "schedule",
            source: "orgintel",
            status: if always_on { "requires_always_on_runner" } else { "mac_must_remain_awake" }.into(),
            observed_at: schedule.last_considered_at.unwrap_or(schedule.created_at),
            detail: Some(if always_on {
                "This timing guarantee is not claimed by the local Mac; attach an always-on runner."
            } else {
                "This schedule catches up according to policy after resume, but the Mac cannot work while powered off."
            }.into()),
            metadata: Some(serde_json::json!({
                "next_fire_at": schedule.fire_at,
                "missed_policy": schedule.missed_policy,
                "maximum_lateness_seconds": schedule.catch_up_grace_seconds,
            })),
            launch: None,
        });
    }

    Resources {
        status: if authority.is_none() && doctor.is_none() {
            "unavailable"
        } else if authority.is_none() || doctor.is_none() {
            "partial"
        } else {
            "available"
        },
        items,
    }
}

fn actions(authority: Option<&AuthorityProjection>) -> ExternalActions {
    let Some(authority) = authority else {
        return ExternalActions {
            status: "unavailable",
            items: Vec::new(),
        };
    };
    let reconciled_receipts = authority
        .reconciliations
        .iter()
        .filter_map(|row| row.body.get("receipt_id")?.as_str().map(str::to_string))
        .collect::<BTreeSet<_>>();
    let completed = authority
        .effect_receipts
        .iter()
        .filter_map(|row| {
            Some((
                row.body.get("idempotency_key")?.as_str()?.to_string(),
                execution_no(&row.body),
            ))
        })
        .collect::<BTreeSet<_>>();
    let mut items = authority
        .effect_receipts
        .iter()
        .rev()
        .take(50)
        .map(|row| action_from_receipt(row, &reconciled_receipts))
        .collect::<Vec<_>>();
    items.extend(
        authority
            .effect_intents
            .iter()
            .filter(|row| {
                row.body
                    .get("idempotency_key")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|key| {
                        !completed.contains(&(key.to_string(), execution_no(&row.body)))
                    })
            })
            .map(|row| ExternalActionRow {
                id: format!("intent:{}", row.id),
                title: row
                    .body
                    .get("purpose")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("External effect awaiting reconciliation")
                    .to_string(),
                effect_class: row
                    .body
                    .get("effect_class")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("external_effect")
                    .to_string(),
                source: "authority",
                state: "unknown".into(),
                evidence: "unknown",
                actor: row.actor_id.clone(),
                party: row
                    .body
                    .get("party")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string),
                receipt_ref: None,
                detail: Some(format!(
                    "Execution {} has durable intent but no result receipt.",
                    execution_no(&row.body)
                )),
                observed_at: row.created_at,
            }),
    );
    items.extend(authority.payments.iter().map(|payment| {
        let confirmed =
            payment.provider_transfer_id.is_some() || payment.raw_provider_status.is_some();
        ExternalActionRow {
            id: format!("payment:{}", payment.request.idempotency_key),
            title: payment.request.purpose.clone(),
            effect_class: "payment.transfer".into(),
            source: "authority_provider",
            state: payment.state.as_str().into(),
            evidence: if confirmed {
                "provider_confirmed"
            } else {
                "authority_recorded"
            },
            actor: Some(payment.request.requesting_actor.clone()),
            party: Some(payment.request.provider_beneficiary_ref.clone()),
            receipt_ref: payment.provider_transfer_id.clone(),
            detail: Some(format!(
                "{} {} from {}",
                format_minor(payment.request.amount_minor),
                payment.request.currency,
                payment.request.source_account_ref
            )),
            observed_at: payment.updated_at,
        }
    }));
    items.sort_by_key(|item| std::cmp::Reverse(item.observed_at));
    ExternalActions {
        status: "available",
        items,
    }
}

fn action_from_receipt(
    row: &crate::authority::AuthorityRecord,
    reconciled_receipts: &BTreeSet<String>,
) -> ExternalActionRow {
    let governed = reconcile::is_governed_receipt(&row.body);
    let receipt_ref = row
        .body
        .get("id")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let reconciled = row
        .body
        .get("outcome")
        .and_then(|outcome| outcome.get("reconciled"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        || receipt_ref
            .as_ref()
            .is_some_and(|id| reconciled_receipts.contains(id));
    let state = if !governed {
        "unverified"
    } else if row.body.get("success").and_then(serde_json::Value::as_bool) == Some(false) {
        "failed"
    } else {
        match row.body.get("outcome").map(reconcile::outcome_of) {
            Some(reconcile::Outcome::Failed) => "failed",
            Some(reconcile::Outcome::Unknown) => "unknown",
            _ => "succeeded",
        }
    };
    ExternalActionRow {
        id: receipt_ref
            .clone()
            .unwrap_or_else(|| format!("legacy:{}", row.id)),
        title: row
            .body
            .get("purpose")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Recorded external effect")
            .to_string(),
        effect_class: row
            .body
            .get("effect_class")
            .or_else(|| row.body.get("capability"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("external_effect")
            .to_string(),
        source: "authority",
        state: state.into(),
        evidence: if !governed {
            "legacy_unverified"
        } else if reconciled {
            "reconciled"
        } else {
            "self_attested"
        },
        actor: row
            .body
            .get("actor")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .or_else(|| row.actor_id.clone()),
        party: row
            .body
            .get("party")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        receipt_ref,
        detail: row
            .body
            .get("outcome")
            .and_then(|outcome| outcome.get("status"))
            .and_then(serde_json::Value::as_str)
            .map(|status| format!("Reported outcome: {status}")),
        observed_at: row.created_at,
    }
}

fn company_doctor(
    authority: SourceObservation,
    orgintel: SourceObservation,
    runtime_source: SourceObservation,
    doctor: Option<&runtime::RuntimeDoctor>,
    observed_at: DateTime<Utc>,
) -> CompanyDoctor {
    let mut checks = vec![
        DoctorCheck {
            id: "authority",
            label: "Authority",
            source: "authority",
            status: source_check_status(&authority),
            summary: if authority.status == "available" {
                "Mandate, limits and governance records are readable.".into()
            } else {
                "Lifecycle changes and new external consequences must pause.".into()
            },
            detail: authority.detail,
        },
        DoctorCheck {
            id: "orgintel",
            label: "Organisation",
            source: "orgintel",
            status: source_check_status(&orgintel),
            summary: if orgintel.status == "available" {
                "Current direction and organisational state are readable.".into()
            } else {
                "Coordination may be stale; Authority and Runtime are checked independently.".into()
            },
            detail: orgintel.detail,
        },
    ];
    if let Some(doctor) = doctor {
        let container_status = match doctor.container {
            runtime::ContainerStatus::Running => "healthy",
            runtime::ContainerStatus::Stopped | runtime::ContainerStatus::Absent => "degraded",
        };
        checks.push(DoctorCheck {
            id: "runtime",
            label: "Company computer",
            source: "runtime",
            status: container_status,
            summary: format!("Container is {}.", container_name(doctor.container)),
            detail: None,
        });
        checks.push(DoctorCheck {
            id: "persistence",
            label: "Persistent company files",
            source: "runtime",
            status: if doctor.volume_exists {
                "healthy"
            } else {
                "degraded"
            },
            summary: if doctor.volume_exists {
                if doctor.container == runtime::ContainerStatus::Absent || doctor.volume_mounted {
                    "The named company volume is present.".into()
                } else {
                    "The company volume exists but is not mounted by the current container.".into()
                }
            } else {
                "The persistent company volume does not exist yet.".into()
            },
            detail: (!doctor.volume_exists
                || (doctor.container != runtime::ContainerStatus::Absent
                    && !doctor.volume_mounted))
                .then(|| {
                    "Reconcile before assuming the current container holds durable company work."
                        .into()
                }),
        });
        checks.push(DoctorCheck {
            id: "image",
            label: "Runtime image",
            source: "runtime",
            status: match doctor.reconciliation {
                runtime::ReconciliationStatus::Current => "healthy",
                runtime::ReconciliationStatus::Required => "degraded",
                runtime::ReconciliationStatus::Unknown => "unknown",
            },
            summary: match doctor.reconciliation {
                runtime::ReconciliationStatus::Current => {
                    "The running shell matches the current Restless source."
                }
                runtime::ReconciliationStatus::Required => {
                    "The replaceable shell needs reconciliation; the company volume is preserved."
                }
                runtime::ReconciliationStatus::Unknown => {
                    "The current image relationship could not be proved."
                }
            }
            .into(),
            detail: None,
        });
        checks.push(service_check(doctor));
        checks.push(browser_check(doctor));
        checks.push(coordination_check(doctor));
    } else {
        checks.push(DoctorCheck {
            id: "runtime",
            label: "Company computer",
            source: "runtime",
            status: "unavailable",
            summary: "The Runtime could not be inspected.".into(),
            detail: runtime_source.detail,
        });
    }
    let actions = doctor
        .map(recommended_actions)
        .unwrap_or_default()
        .into_iter()
        .map(action_copy)
        .collect::<Vec<_>>();
    let status = overall_doctor_status(&checks);
    CompanyDoctor {
        status,
        observed_at,
        checks,
        actions,
    }
}

fn service_check(doctor: &runtime::RuntimeDoctor) -> DoctorCheck {
    match doctor.supervisor.as_ref() {
        Some(supervisor) => DoctorCheck {
            id: "supervisor",
            label: "Company services",
            source: "runtime",
            status: if supervisor.status == "available" {
                "healthy"
            } else {
                "degraded"
            },
            summary: if supervisor.status == "available" {
                format!(
                    "{} supervised services are running.",
                    supervisor.services.len()
                )
            } else {
                "One or more supervised services are not healthy.".into()
            },
            detail: None,
        },
        None => DoctorCheck {
            id: "supervisor",
            label: "Company services",
            source: "runtime",
            status: "unavailable",
            summary: "Services are not observable while the Company computer is stopped.".into(),
            detail: None,
        },
    }
}

fn browser_check(doctor: &runtime::RuntimeDoctor) -> DoctorCheck {
    match doctor.browser.as_ref() {
        Some(browser) => DoctorCheck {
            id: "browser",
            label: "Browser and desktop",
            source: "runtime",
            status: if browser.status == "available" {
                "healthy"
            } else {
                "degraded"
            },
            summary: if browser.status == "available" {
                format!(
                    "Persistent browser and desktop are available; controller is {}.",
                    browser.controller
                )
            } else {
                "The persistent browser or desktop transport is degraded.".into()
            },
            detail: Some(format!(
                "desktop {}, browser {}, automation {}, transport {}",
                browser.desktop, browser.chromium, browser.automation, browser.web_transport
            )),
        },
        None => DoctorCheck {
            id: "browser",
            label: "Browser and desktop",
            source: "runtime",
            status: "unavailable",
            summary: "Browser state is not observable while the Company computer is stopped."
                .into(),
            detail: None,
        },
    }
}

fn coordination_check(doctor: &runtime::RuntimeDoctor) -> DoctorCheck {
    match doctor.coordination.as_ref() {
        Some(coordination) => DoctorCheck {
            id: "coordination",
            label: "Runtime coordination",
            source: "runtime",
            status: if coordination.status == "available" {
                "healthy"
            } else {
                "degraded"
            },
            summary: if coordination.status == "available" {
                "The Runtime completed an authenticated coordination status request.".into()
            } else {
                "The Runtime cannot currently use its bounded coordination path; files and already-running local work remain available.".into()
            },
            detail: coordination.detail.clone(),
        },
        None => DoctorCheck {
            id: "coordination",
            label: "Runtime coordination",
            source: "runtime",
            status: "unavailable",
            summary:
                "Runtime coordination is not observable while the Company computer is stopped."
                    .into(),
            detail: None,
        },
    }
}

fn source_check_status(source: &SourceObservation) -> &'static str {
    if source.status == "available" {
        "healthy"
    } else {
        "unavailable"
    }
}

fn overall_doctor_status(checks: &[DoctorCheck]) -> &'static str {
    if checks.iter().any(|check| {
        matches!(check.id, "authority" | "orgintel" | "runtime") && check.status == "unavailable"
    }) {
        "unavailable"
    } else if checks
        .iter()
        .any(|check| matches!(check.status, "degraded" | "unavailable"))
    {
        "degraded"
    } else if checks.iter().any(|check| check.status == "unknown") {
        "unknown"
    } else {
        "healthy"
    }
}

fn recommended_actions(doctor: &runtime::RuntimeDoctor) -> Vec<RecoveryAction> {
    if doctor.reconciliation == runtime::ReconciliationStatus::Required {
        return vec![RecoveryAction::Reconcile];
    }
    if doctor.container == runtime::ContainerStatus::Running
        && doctor
            .coordination
            .as_ref()
            .is_none_or(|value| value.status != "available")
    {
        return vec![RecoveryAction::Reconcile];
    }
    match doctor.container {
        runtime::ContainerStatus::Stopped => vec![RecoveryAction::Start],
        runtime::ContainerStatus::Absent => vec![RecoveryAction::Reconcile],
        runtime::ContainerStatus::Running
            if doctor
                .supervisor
                .as_ref()
                .is_some_and(|value| value.status != "available")
                || doctor
                    .browser
                    .as_ref()
                    .is_some_and(|value| value.status != "available") =>
        {
            vec![RecoveryAction::Restart]
        }
        runtime::ContainerStatus::Running => Vec::new(),
    }
}

fn action_copy(action: RecoveryAction) -> DoctorAction {
    match action {
        RecoveryAction::Start => DoctorAction {
            id: action,
            label: "Start company computer",
            consequence: "Starts the existing company shell and preserves its volume and browser profile.",
            confirmation: "Start the Company computer now?",
        },
        RecoveryAction::Restart => DoctorAction {
            id: action,
            label: "Restart company computer",
            consequence: "Stops and starts the replaceable shell. Company files and the persistent browser profile remain on the named volume.",
            confirmation: "Restart the Company computer and briefly interrupt its processes?",
        },
        RecoveryAction::Reconcile => DoctorAction {
            id: action,
            label: "Reconcile company computer",
            consequence: "Rebuilds the current shell and restores its Runtime coordination grant while preserving the named company volume.",
            confirmation: "Reconcile the Company computer with the current Restless source?",
        },
    }
}

pub(crate) async fn recover(
    daemon: &Daemon,
    config: &runtime::CompanyConfig,
    action: RecoveryAction,
) -> Result<RecoveryOutcome> {
    let before = runtime::doctor(&config.name)
        .await
        .context("inspect Company computer before recovery")?;
    if !recommended_actions(&before).contains(&action) {
        bail!(
            "{} is not a current doctor recommendation; refresh Company computer before acting",
            action.as_str()
        );
    }
    let requested_at = Utc::now();
    daemon
        .authority
        .emit(
            &config.name,
            "lifecycle",
            Some("owner"),
            serde_json::json!({
                "action": action,
                "state": "requested",
                "requested_at": requested_at,
            }),
        )
        .await
        .context("record owner recovery request before changing Runtime lifecycle")?;

    let result: Result<String> = async {
        let message = match action {
            RecoveryAction::Start => runtime::up(config, false).await,
            RecoveryAction::Restart => match runtime::down(&config.name).await {
                Ok(_) => runtime::up(config, false).await,
                Err(error) => Err(error),
            },
            RecoveryAction::Reconcile => runtime::up(config, true).await,
        }?;
        crate::materialize_runtime_bridge(daemon, &config.name).await?;
        Ok(message)
    }
    .await;
    let message = match result {
        Ok(message) => message,
        Err(error) => {
            daemon
                .authority
                .emit(
                    &config.name,
                    "lifecycle",
                    Some("owner"),
                    serde_json::json!({
                        "action": action,
                        "state": "failed",
                        "requested_at": requested_at,
                        "observed_at": Utc::now(),
                        "error": format!("{error:#}"),
                    }),
                )
                .await
                .context(
                    "Runtime recovery failed and its lifecycle receipt could not be recorded",
                )?;
            return Err(error).context("Company computer recovery failed");
        }
    };
    let after = runtime::doctor(&config.name)
        .await
        .context("re-probe Company computer after recovery")?;
    daemon
        .authority
        .emit(
            &config.name,
            "lifecycle",
            Some("owner"),
            serde_json::json!({
                "action": action,
                "state": "succeeded",
                "message": message,
                "requested_at": requested_at,
                "observed_at": Utc::now(),
                "observed": {
                    "container": after.container,
                    "volume_exists": after.volume_exists,
                    "volume_mounted": after.volume_mounted,
                    "reconciliation": after.reconciliation,
                    "coordination": after.coordination.as_ref().map(|value| value.status.as_str()),
                    "supervisor": after.supervisor.as_ref().map(|value| value.status.as_str()),
                    "browser": after.browser.as_ref().map(|value| value.status.as_str()),
                },
            }),
        )
        .await
        .context("Company computer recovered but its lifecycle receipt could not be recorded")?;
    Ok(RecoveryOutcome {
        action,
        message,
        doctor: after,
    })
}

fn execution_no(body: &serde_json::Value) -> i64 {
    body.get("execution_no")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(1)
}

fn container_name(status: runtime::ContainerStatus) -> &'static str {
    match status {
        runtime::ContainerStatus::Running => "running",
        runtime::ContainerStatus::Stopped => "stopped",
        runtime::ContainerStatus::Absent => "absent",
    }
}

fn display_name(name: &str) -> String {
    name.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn round_usd(value: f64) -> f64 {
    let rounded = (value * 10_000.0).round() / 10_000.0;
    if rounded == 0.0 {
        0.0
    } else {
        rounded
    }
}

fn format_minor(amount: i64) -> String {
    format!("{:.2}", amount as f64 / 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(body: serde_json::Value) -> crate::authority::AuthorityRecord {
        crate::authority::AuthorityRecord {
            id: 7,
            actor_id: Some("exec".into()),
            body,
            created_at: Utc::now(),
        }
    }

    fn doctor(
        container: runtime::ContainerStatus,
        reconciliation: runtime::ReconciliationStatus,
        supervisor: Option<&str>,
        browser: Option<&str>,
        coordination: Option<&str>,
    ) -> runtime::RuntimeDoctor {
        runtime::RuntimeDoctor {
            company: "company_test".into(),
            container,
            volume: "restless-vol-company_test".into(),
            volume_exists: true,
            volume_mounted: container != runtime::ContainerStatus::Absent,
            image: runtime::COMPANY_IMAGE.into(),
            container_image_id: None,
            target_image_id: None,
            source_digest: None,
            image_source_digest: None,
            reconciliation,
            action: None,
            release: None,
            coordination: coordination.map(|status| runtime::CoordinationDoctor {
                status: status.into(),
                detail: (status != "available").then(|| "coordination detail".into()),
            }),
            supervisor: supervisor.map(|status| runtime::SupervisorDoctor {
                status: status.into(),
                services: Vec::new(),
            }),
            browser: browser.map(|status| runtime::BrowserDoctor {
                status: status.into(),
                desktop: status.into(),
                chromium: status.into(),
                automation: status.into(),
                web_transport: status.into(),
                controller: "unclaimed".into(),
            }),
        }
    }

    #[test]
    fn doctor_recommends_only_the_smallest_source_owned_repair() {
        assert_eq!(
            recommended_actions(&doctor(
                runtime::ContainerStatus::Stopped,
                runtime::ReconciliationStatus::Current,
                None,
                None,
                None,
            )),
            vec![RecoveryAction::Start]
        );
        assert_eq!(
            recommended_actions(&doctor(
                runtime::ContainerStatus::Running,
                runtime::ReconciliationStatus::Required,
                Some("degraded"),
                Some("degraded"),
                Some("degraded"),
            )),
            vec![RecoveryAction::Reconcile]
        );
        assert_eq!(
            recommended_actions(&doctor(
                runtime::ContainerStatus::Running,
                runtime::ReconciliationStatus::Current,
                Some("available"),
                Some("degraded"),
                Some("available"),
            )),
            vec![RecoveryAction::Restart]
        );
        assert_eq!(
            recommended_actions(&doctor(
                runtime::ContainerStatus::Running,
                runtime::ReconciliationStatus::Current,
                Some("available"),
                Some("available"),
                Some("degraded"),
            )),
            vec![RecoveryAction::Reconcile]
        );
    }

    #[test]
    fn harness_settings_report_observed_builds_and_model_compatibility() {
        let config: runtime::CompanyConfig = toml::from_str(
            r#"name = "company_test"
model = "moonshot/kimi-k3"
"#,
        )
        .unwrap();
        let mut runtime_doctor = doctor(
            runtime::ContainerStatus::Running,
            runtime::ReconciliationStatus::Current,
            Some("available"),
            Some("available"),
            Some("available"),
        );
        runtime_doctor.release = Some(runtime::RuntimeReleaseIdentity {
            core_version: "0.0.0-test".into(),
            source_revision: "test-revision".into(),
            api_contract_version: 1,
            assertion_contract_version: 1,
            schema_version: crate::release::SCHEMA_VERSION,
            harnesses: [
                ("restless-managed".into(), "omp-18.0.10".into()),
                ("codex".into(), "codex-cli-0.151.0".into()),
                ("claude-agent".into(), "claude-agent-acp-0.73.0".into()),
            ]
            .into_iter()
            .collect(),
            harness_agents: [("claude-agent".into(), "claude-code-2.1.257".into())]
                .into_iter()
                .collect(),
            harness_dependencies: [("claude-agent-sdk".into(), "0.3.257".into())]
                .into_iter()
                .collect(),
        });

        let settings = harness_settings(&config, Some(&runtime_doctor));
        assert_eq!(
            settings.coordination,
            runtime::AgentHarness::RestlessManaged
        );
        assert_eq!(settings.worker, runtime::AgentHarness::RestlessManaged);
        let managed = settings
            .options
            .iter()
            .find(|option| option.id == runtime::AgentHarness::RestlessManaged)
            .unwrap();
        assert_eq!(managed.status, "ready");
        assert_eq!(managed.observed_build.as_deref(), Some("omp-18.0.10"));
        for harness in [
            runtime::AgentHarness::Codex,
            runtime::AgentHarness::ClaudeAgent,
        ] {
            let option = settings
                .options
                .iter()
                .find(|option| option.id == harness)
                .unwrap();
            assert_eq!(option.status, "incompatible");
            assert!(option.detail.contains("current model/effort policy"));
        }
    }

    #[test]
    fn doctor_exposes_a_degraded_runtime_coordination_path() {
        let observed_at = Utc::now();
        let source = SourceObservation::available(observed_at);
        let runtime_doctor = doctor(
            runtime::ContainerStatus::Running,
            runtime::ReconciliationStatus::Current,
            Some("available"),
            Some("available"),
            Some("degraded"),
        );

        let report = company_doctor(
            source.clone(),
            source.clone(),
            source,
            Some(&runtime_doctor),
            observed_at,
        );
        let coordination = report
            .checks
            .iter()
            .find(|check| check.id == "coordination")
            .expect("coordination check");
        assert_eq!(coordination.status, "degraded");
        assert_eq!(coordination.detail.as_deref(), Some("coordination detail"));
        assert_eq!(report.status, "degraded");
    }

    #[test]
    fn unknown_and_unavailable_doctor_states_never_become_healthy() {
        let checks = vec![
            DoctorCheck {
                id: "authority",
                label: "Authority",
                source: "authority",
                status: "healthy",
                summary: String::new(),
                detail: None,
            },
            DoctorCheck {
                id: "image",
                label: "Image",
                source: "runtime",
                status: "unknown",
                summary: String::new(),
                detail: None,
            },
        ];
        assert_eq!(overall_doctor_status(&checks), "unknown");
    }

    #[test]
    fn external_effect_evidence_is_never_upgraded_by_a_success_word() {
        let legacy = action_from_receipt(
            &record(serde_json::json!({
                "capability": "customer-contact.email",
                "outcome": {"status": "sent"}
            })),
            &BTreeSet::new(),
        );
        assert_eq!(legacy.state, "unverified");
        assert_eq!(legacy.evidence, "legacy_unverified");

        let governed = action_from_receipt(
            &record(serde_json::json!({
                "id": "00000000-0000-0000-0000-000000000007",
                "effect_class": "customer-contact.email",
                "command_digest": "digest",
                "tool": "mail-tool",
                "success": true,
                "execution_no": 1,
                "outcome": {"status": "sent"}
            })),
            &BTreeSet::new(),
        );
        assert_eq!(governed.state, "succeeded");
        assert_eq!(governed.evidence, "self_attested");

        let reconciled = action_from_receipt(
            &record(serde_json::json!({
                "id": "00000000-0000-0000-0000-000000000008",
                "effect_class": "browser.form.submit",
                "command_digest": "digest",
                "tool": "browser-tool",
                "success": false,
                "execution_no": 1,
                "outcome": {"status": "failed", "reconciled": true}
            })),
            &BTreeSet::new(),
        );
        assert_eq!(reconciled.state, "failed");
        assert_eq!(reconciled.evidence, "reconciled");
    }
}
