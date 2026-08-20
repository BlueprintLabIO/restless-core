//! Source-aware projection for the stable owner-facing Company area.
//!
//! This module deliberately owns no company state. It composes Authority,
//! OrgIntel and Runtime observations and invokes only the Runtime lifecycle
//! actions already owned by the daemon.

use std::collections::BTreeSet;

use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Utc};
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
    poisoned: bool,
}

#[derive(Debug, Serialize)]
struct Resources {
    status: &'static str,
    items: Vec<ResourceRow>,
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
}

pub(crate) async fn project(
    daemon: &Daemon,
    config: &runtime::CompanyConfig,
    probe_credentials: bool,
) -> CompanyView {
    let observed_at = Utc::now();

    let org_result = match daemon.orgintel.get(&config.name).await {
        Ok(org) => org.list_goals().await.map_err(|error| format!("{error:#}")),
        Err(error) => Err(format!("{error:#}")),
    };
    let (goals, orgintel_source) = match org_result {
        Ok(goals) => (goals, SourceObservation::available(observed_at)),
        Err(error) => (
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

    let spend_breakdown = daemon.spend.breakdown(&config.name);
    let accounted_usd: f64 = spend_breakdown.iter().map(|(_, _, usd)| usd).sum();
    let poisoned = daemon.spend.spent_usd(&config.name) > 1_000_000_000.0;
    let spend = SpendLimit {
        model: config.model.clone(),
        accounted_usd: round_usd(accounted_usd),
        ceiling_usd: config.spend_ceiling_usd,
        remaining_usd: (!poisoned)
            .then(|| round_usd((config.spend_ceiling_usd - accounted_usd).max(0.0))),
        poisoned,
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
        probe_credentials,
    )
    .await;
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
    })
}

async fn resources(
    config: &runtime::CompanyConfig,
    authority: Option<&AuthorityProjection>,
    doctor: Option<&runtime::RuntimeDoctor>,
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
        })),
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
            });
        }
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
            consequence: "Rebuilds the current shell and replaces it when needed while preserving the named company volume.",
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

    let result = match action {
        RecoveryAction::Start => runtime::up(config, false).await,
        RecoveryAction::Restart => match runtime::down(&config.name).await {
            Ok(_) => runtime::up(config, false).await,
            Err(error) => Err(error),
        },
        RecoveryAction::Reconcile => runtime::up(config, true).await,
    };
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
            )),
            vec![RecoveryAction::Start]
        );
        assert_eq!(
            recommended_actions(&doctor(
                runtime::ContainerStatus::Running,
                runtime::ReconciliationStatus::Required,
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
            )),
            vec![RecoveryAction::Restart]
        );
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
