//! Deterministic authority for one deliberately bounded operating-money rail.
//!
//! Business legitimacy stays with OrgIntel judgement. This module answers the
//! enumerable question: may this exact account/beneficiary/currency/amount be
//! reserved, and what did an authenticated provider later confirm?

use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use sqlx::{PgPool, Row as _};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MoneyEnvelopeInput {
    pub source_account_ref: String,
    pub currency: String,
    pub beneficiary_refs: Vec<String>,
    pub per_payment_limit_minor: i64,
    pub aggregate_limit_minor: i64,
    #[serde(default)]
    pub frozen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MoneyEnvelope {
    #[serde(flatten)]
    pub limits: MoneyEnvelopeInput,
    pub period_started_at: DateTime<Utc>,
    pub updated_by: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PaymentIntentInput {
    /// Cross-plane identifiers only. OrgIntel remains the writer of both rows;
    /// Authority binds this exact consequence to the outcome and owner step
    /// that caused it.
    pub work_id: Uuid,
    pub owner_handoff_id: Uuid,
    pub source_account_ref: String,
    pub provider_beneficiary_ref: String,
    pub amount_minor: i64,
    pub currency: String,
    pub purpose: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    pub idempotency_key: String,
    pub requesting_actor: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentState {
    Reserved,
    Submitted,
    InApproval,
    Scheduled,
    Processing,
    Blocked,
    Unknown,
    Settled,
    Rejected,
    Cancelled,
    Failed,
}

impl PaymentState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reserved => "reserved",
            Self::Submitted => "submitted",
            Self::InApproval => "in_approval",
            Self::Scheduled => "scheduled",
            Self::Processing => "processing",
            Self::Blocked => "blocked",
            Self::Unknown => "unknown",
            Self::Settled => "settled",
            Self::Rejected => "rejected",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }

    fn from_db(value: &str) -> Result<Self> {
        Ok(match value {
            "reserved" => Self::Reserved,
            "submitted" => Self::Submitted,
            "in_approval" => Self::InApproval,
            "scheduled" => Self::Scheduled,
            "processing" => Self::Processing,
            "blocked" => Self::Blocked,
            "unknown" => Self::Unknown,
            "settled" => Self::Settled,
            "rejected" => Self::Rejected,
            "cancelled" => Self::Cancelled,
            "failed" => Self::Failed,
            other => bail!("unknown stored payment state {other:?}"),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PaymentIntent {
    #[serde(flatten)]
    pub request: PaymentIntentInput,
    pub state: PaymentState,
    pub provider: String,
    #[serde(default)]
    pub provider_transfer_id: Option<String>,
    #[serde(default)]
    pub raw_provider_status: Option<String>,
    #[serde(default)]
    pub provider_approval_url: Option<String>,
    #[serde(default)]
    pub settled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reservation {
    pub intent: PaymentIntent,
    pub replayed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderObservation {
    pub payment: PaymentIntent,
    pub changed: bool,
}

pub async fn ensure_schema(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.money_envelopes (\
           company TEXT NOT NULL, currency TEXT NOT NULL, source_account_ref TEXT NOT NULL, \
           beneficiary_refs JSONB NOT NULL, per_payment_limit_minor BIGINT NOT NULL, \
           aggregate_limit_minor BIGINT NOT NULL, frozen BOOLEAN NOT NULL DEFAULT false, \
           period_started_at TIMESTAMPTZ NOT NULL, updated_by TEXT NOT NULL, \
           updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), PRIMARY KEY (company,currency), \
           CHECK (per_payment_limit_minor > 0), CHECK (aggregate_limit_minor > 0)\
         )",
    )
    .execute(pool)
    .await
    .context("create Authority money envelopes")?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.payment_intents (\
           company TEXT NOT NULL, idempotency_key TEXT NOT NULL, fingerprint TEXT NOT NULL, \
           source_account_ref TEXT NOT NULL, provider_beneficiary_ref TEXT NOT NULL, \
           amount_minor BIGINT NOT NULL, currency TEXT NOT NULL, purpose TEXT NOT NULL, \
           work_id UUID NOT NULL, owner_handoff_id UUID NOT NULL, evidence_refs JSONB NOT NULL, \
           requesting_actor TEXT NOT NULL, provider TEXT NOT NULL, \
           state TEXT NOT NULL, provider_transfer_id TEXT, raw_provider_status TEXT, \
           provider_approval_url TEXT, settled_at TIMESTAMPTZ, \
           created_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
           updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
           PRIMARY KEY (company,idempotency_key), UNIQUE (company,provider_transfer_id), \
           CHECK (amount_minor > 0)\
         )",
    )
    .execute(pool)
    .await
    .context("create Authority payment intents")?;
    // Additive repair for a development database created by the pre-link
    // Sprint 08 branch. New reservations always supply both identifiers;
    // historical unlinked rows remain visible but cannot be submitted.
    sqlx::query(
        "ALTER TABLE restless_authority.payment_intents \
         ADD COLUMN IF NOT EXISTS work_id UUID, \
         ADD COLUMN IF NOT EXISTS owner_handoff_id UUID, \
         ADD COLUMN IF NOT EXISTS settled_at TIMESTAMPTZ",
    )
    .execute(pool)
    .await
    .context("add Work links to Authority payment intents")?;
    sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS payment_intents_owner_handoff_unique \
         ON restless_authority.payment_intents (company,owner_handoff_id) \
         WHERE owner_handoff_id IS NOT NULL",
    )
    .execute(pool)
    .await
    .context("make each owner handoff identify at most one payment")?;
    Ok(())
}

/// Check cross-plane identifiers at the orchestration seam without making
/// Authority a second writer of Work. The payment must be caused by current,
/// pending OrgIntel Work and its exact payment-confirmation handoff.
pub async fn validate_work_link(
    org: &restless_orgintel::OrgIntel,
    input: &PaymentIntentInput,
) -> Result<()> {
    let graph = org.work_graph_snapshot().await?;
    if !graph.work.iter().any(|work| work.id == input.work_id) {
        bail!("payment Work does not exist in OrgIntel");
    }
    let handoff = graph
        .handoffs
        .iter()
        .find(|handoff| handoff.id == input.owner_handoff_id)
        .context("payment owner handoff does not exist in OrgIntel")?;
    if handoff.work_id != input.work_id
        || handoff.category != restless_orgintel::OwnerHandoffCategory::PaymentConfirmation
        || handoff.state != restless_orgintel::OwnerHandoffState::Pending
        || handoff.assigned_to.is_some()
    {
        bail!("payment must link its exact pending owner-level payment_confirmation handoff");
    }
    Ok(())
}

pub async fn set_envelope(
    store: &crate::authority::AuthorityStore,
    company: &str,
    mut input: MoneyEnvelopeInput,
    owner: &str,
) -> Result<MoneyEnvelope> {
    validate_envelope(&input)?;
    input.currency = normalise_currency(&input.currency)?;
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO restless_authority.money_envelopes \
         (company,currency,source_account_ref,beneficiary_refs,per_payment_limit_minor,\
          aggregate_limit_minor,frozen,period_started_at,updated_by) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) \
         ON CONFLICT (company,currency) DO UPDATE SET \
          source_account_ref=EXCLUDED.source_account_ref, beneficiary_refs=EXCLUDED.beneficiary_refs, \
          per_payment_limit_minor=EXCLUDED.per_payment_limit_minor, \
          aggregate_limit_minor=EXCLUDED.aggregate_limit_minor, frozen=EXCLUDED.frozen, \
          period_started_at=EXCLUDED.period_started_at, updated_by=EXCLUDED.updated_by, updated_at=now()",
    )
    .bind(company)
    .bind(&input.currency)
    .bind(&input.source_account_ref)
    .bind(serde_json::to_value(&input.beneficiary_refs)?)
    .bind(input.per_payment_limit_minor)
    .bind(input.aggregate_limit_minor)
    .bind(input.frozen)
    .bind(now)
    .bind(owner)
    .execute(store.pool())
    .await?;
    store
        .emit(
            company,
            "money_envelope_changed",
            Some(owner),
            serde_json::json!({
                "source_account_ref": input.source_account_ref,
                "currency": input.currency,
                "beneficiary_refs": input.beneficiary_refs,
                "per_payment_limit_minor": input.per_payment_limit_minor,
                "aggregate_limit_minor": input.aggregate_limit_minor,
                "frozen": input.frozen,
                "period_started_at": now,
            }),
        )
        .await?;
    Ok(MoneyEnvelope {
        limits: input,
        period_started_at: now,
        updated_by: owner.to_string(),
        updated_at: now,
    })
}

pub async fn set_frozen(
    store: &crate::authority::AuthorityStore,
    company: &str,
    currency: &str,
    frozen: bool,
    owner: &str,
) -> Result<MoneyEnvelope> {
    let currency = normalise_currency(currency)?;
    let row = sqlx::query(
        "UPDATE restless_authority.money_envelopes SET frozen=$3, updated_by=$4, updated_at=now() \
         WHERE company=$1 AND currency=$2 RETURNING *",
    )
    .bind(company)
    .bind(&currency)
    .bind(frozen)
    .bind(owner)
    .fetch_optional(store.pool())
    .await?
    .context("no money envelope exists for that company/currency")?;
    let envelope = envelope_from_row(&row)?;
    store
        .emit(
            company,
            if frozen {
                "financial_effects_frozen"
            } else {
                "financial_effects_unfrozen"
            },
            Some(owner),
            serde_json::json!({"currency": currency}),
        )
        .await?;
    Ok(envelope)
}

/// Replace the payees one allowance may pay, keeping its limits and running total. Each is an
/// existing beneficiary in the payment provider; Authority checks every payment against the list.
pub async fn set_payees(
    store: &crate::authority::AuthorityStore,
    company: &str,
    currency: &str,
    payees: Vec<String>,
    owner: &str,
) -> Result<MoneyEnvelope> {
    let currency = normalise_currency(currency)?;
    let mut payees = payees
        .into_iter()
        .map(|payee| payee.trim().to_string())
        .filter(|payee| !payee.is_empty())
        .collect::<Vec<_>>();
    payees.dedup();
    if payees.is_empty() {
        bail!("a payment allowance needs at least one payee; freeze it to stop payments instead");
    }
    if payees.iter().any(|payee| payee.len() > 200 || payee.chars().any(char::is_control)) {
        bail!("a payee is one provider beneficiary reference of at most 200 characters");
    }
    let row = sqlx::query(
        "UPDATE restless_authority.money_envelopes SET beneficiary_refs=$3, updated_by=$4, updated_at=now() \
         WHERE company=$1 AND currency=$2 RETURNING *",
    )
    .bind(company)
    .bind(&currency)
    .bind(serde_json::to_value(&payees)?)
    .bind(owner)
    .fetch_optional(store.pool())
    .await?
    .context("no payment allowance exists for that currency")?;
    let envelope = envelope_from_row(&row)?;
    store
        .emit(
            company,
            "money_envelope_changed",
            Some(owner),
            serde_json::json!({"currency": currency, "beneficiary_refs": payees}),
        )
        .await?;
    Ok(envelope)
}

pub const PAYEE_PROPOSED_KIND: &str = "payee_proposed";
pub const PAYEE_DECIDED_KIND: &str = "payee_decided";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayeeProposalInput {
    pub currency: String,
    /// The beneficiary as the payment provider names it; it must already exist there.
    pub beneficiary_ref: String,
    /// Who they are and why the company needs to pay them.
    pub reason: String,
    #[serde(default)]
    pub work_id: Option<Uuid>,
}

/// A company actor asks the owner to let an allowance pay one more payee. The owner answers in
/// the Inbox; nothing is payable until then.
pub async fn propose_payee(
    store: &crate::authority::AuthorityStore,
    company: &str,
    actor: &str,
    mut input: PayeeProposalInput,
) -> Result<i64> {
    input.currency = normalise_currency(&input.currency)?;
    input.beneficiary_ref = input.beneficiary_ref.trim().to_string();
    input.reason = input.reason.trim().to_string();
    if input.beneficiary_ref.is_empty()
        || input.beneficiary_ref.len() > 200
        || input.beneficiary_ref.chars().any(char::is_control)
    {
        bail!("name the payee by its provider beneficiary reference (at most 200 characters)");
    }
    if input.reason.is_empty() || input.reason.len() > 2_000 {
        bail!("say who the payee is and why the company needs to pay them (at most 2,000 characters)");
    }
    let envelope = envelopes(store, company)
        .await?
        .into_iter()
        .find(|envelope| envelope.limits.currency == input.currency)
        .with_context(|| format!("there is no {} payment allowance; ask the owner to set one up first", input.currency))?;
    if envelope.limits.beneficiary_refs.contains(&input.beneficiary_ref) {
        bail!("{} is already a payee of the {} allowance", input.beneficiary_ref, input.currency);
    }
    if pending_payee_proposals(store, company).await?.iter().any(|(_, pending)| {
        pending.currency == input.currency && pending.beneficiary_ref == input.beneficiary_ref
    }) {
        bail!("this payee is already waiting for the owner's answer");
    }
    store
        .emit(company, PAYEE_PROPOSED_KIND, Some(actor), serde_json::to_value(&input)?)
        .await
}

/// Payee proposals the owner has not answered, by record.
pub async fn pending_payee_proposals(
    store: &crate::authority::AuthorityStore,
    company: &str,
) -> Result<Vec<(crate::authority::AuthorityRecord, PayeeProposalInput)>> {
    let decided = store
        .records_of_kind(company, PAYEE_DECIDED_KIND)
        .await?
        .into_iter()
        .filter_map(|record| record.body.get("proposal").and_then(serde_json::Value::as_i64))
        .collect::<std::collections::HashSet<_>>();
    Ok(store
        .records_of_kind(company, PAYEE_PROPOSED_KIND)
        .await?
        .into_iter()
        .filter(|record| !decided.contains(&record.id))
        .filter_map(|record| {
            let input = serde_json::from_value(record.body.clone()).ok()?;
            Some((record, input))
        })
        .collect())
}

/// The owner's answer: approving adds the payee to the allowance, keeping its limits.
pub async fn decide_payee(
    store: &crate::authority::AuthorityStore,
    company: &str,
    owner: &str,
    proposal: i64,
    approve: bool,
    note: Option<&str>,
) -> Result<()> {
    let (_, pending) = pending_payee_proposals(store, company)
        .await?
        .into_iter()
        .find(|(record, _)| record.id == proposal)
        .context("this payee request was already answered or no longer exists")?;
    if approve {
        let envelope = envelopes(store, company)
            .await?
            .into_iter()
            .find(|envelope| envelope.limits.currency == pending.currency)
            .context("the payment allowance for this currency no longer exists")?;
        let mut payees = envelope.limits.beneficiary_refs;
        if !payees.contains(&pending.beneficiary_ref) {
            payees.push(pending.beneficiary_ref.clone());
        }
        set_payees(store, company, &pending.currency, payees, owner).await?;
    }
    store
        .emit(
            company,
            PAYEE_DECIDED_KIND,
            Some(owner),
            serde_json::json!({
                "proposal": proposal,
                "approve": approve,
                "currency": pending.currency,
                "beneficiary_ref": pending.beneficiary_ref,
                "note": note.map(str::trim).filter(|note| !note.is_empty()),
            }),
        )
        .await?;
    Ok(())
}

pub async fn envelopes(
    store: &crate::authority::AuthorityStore,
    company: &str,
) -> Result<Vec<MoneyEnvelope>> {
    let rows = sqlx::query(
        "SELECT * FROM restless_authority.money_envelopes WHERE company=$1 ORDER BY currency",
    )
    .bind(company)
    .fetch_all(store.pool())
    .await?;
    rows.iter().map(envelope_from_row).collect()
}

pub async fn reserve(
    store: &crate::authority::AuthorityStore,
    company: &str,
    mut input: PaymentIntentInput,
) -> Result<Reservation> {
    validate_intent(&mut input)?;
    let fingerprint = intent_fingerprint(&input)?;
    crate::outside_actions::ensure_running(store, company).await?;
    let mut tx = store.pool().begin().await?;

    if let Some(row) = sqlx::query(
        "SELECT * FROM restless_authority.payment_intents \
         WHERE company=$1 AND idempotency_key=$2 FOR UPDATE",
    )
    .bind(company)
    .bind(&input.idempotency_key)
    .fetch_optional(&mut *tx)
    .await?
    {
        if row.get::<String, _>("fingerprint") != fingerprint {
            bail!(
                "payment idempotency key {:?} was already used with different authority fields",
                input.idempotency_key
            );
        }
        return Ok(Reservation {
            intent: intent_from_row(&row)?,
            replayed: true,
        });
    }

    let envelope = sqlx::query(
        "SELECT * FROM restless_authority.money_envelopes \
         WHERE company=$1 AND currency=$2 FOR UPDATE",
    )
    .bind(company)
    .bind(&input.currency)
    .fetch_optional(&mut *tx)
    .await?
    .context("no owner-set money envelope exists for this currency")?;
    let envelope = envelope_from_row(&envelope)?;
    if envelope.limits.frozen {
        bail!("financial effects are frozen for {}", input.currency);
    }
    if input.source_account_ref != envelope.limits.source_account_ref {
        bail!("payment source account is outside the owner-set envelope");
    }
    if !envelope
        .limits
        .beneficiary_refs
        .iter()
        .any(|beneficiary| beneficiary == &input.provider_beneficiary_ref)
    {
        bail!("payment beneficiary is outside the owner-set envelope");
    }
    if input.amount_minor > envelope.limits.per_payment_limit_minor {
        bail!("payment exceeds the per-payment envelope");
    }
    let committed: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount_minor),0)::bigint FROM restless_authority.payment_intents \
         WHERE company=$1 AND currency=$2 AND (\
           state IN ('reserved','submitted','in_approval','scheduled','processing','blocked','unknown') \
           OR (state='settled' AND settled_at >= $3))",
    )
    .bind(company)
    .bind(&input.currency)
    .bind(envelope.period_started_at)
    .fetch_one(&mut *tx)
    .await?;
    if committed.saturating_add(input.amount_minor) > envelope.limits.aggregate_limit_minor {
        bail!("payment would exceed the aggregate reserved/settled envelope");
    }

    let row = sqlx::query(
        "INSERT INTO restless_authority.payment_intents \
         (company,idempotency_key,fingerprint,source_account_ref,provider_beneficiary_ref,\
          amount_minor,currency,purpose,work_id,owner_handoff_id,evidence_refs,requesting_actor,provider,state) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,'airwallex','reserved') RETURNING *",
    )
    .bind(company)
    .bind(&input.idempotency_key)
    .bind(fingerprint)
    .bind(&input.source_account_ref)
    .bind(&input.provider_beneficiary_ref)
    .bind(input.amount_minor)
    .bind(&input.currency)
    .bind(&input.purpose)
    .bind(input.work_id)
    .bind(input.owner_handoff_id)
    .bind(serde_json::to_value(&input.evidence_refs)?)
    .bind(&input.requesting_actor)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    let intent = intent_from_row(&row)?;
    store
        .emit(
            company,
            "payment_reserved",
            Some(&intent.request.requesting_actor),
            serde_json::to_value(&intent)?,
        )
        .await?;
    Ok(Reservation {
        intent,
        replayed: false,
    })
}

pub async fn payment(
    store: &crate::authority::AuthorityStore,
    company: &str,
    key: &str,
) -> Result<Option<PaymentIntent>> {
    let row = sqlx::query(
        "SELECT * FROM restless_authority.payment_intents WHERE company=$1 AND idempotency_key=$2",
    )
    .bind(company)
    .bind(key)
    .fetch_optional(store.pool())
    .await?;
    row.as_ref().map(intent_from_row).transpose()
}

pub async fn payments(
    store: &crate::authority::AuthorityStore,
    company: &str,
) -> Result<Vec<PaymentIntent>> {
    let rows = sqlx::query(
        "SELECT * FROM restless_authority.payment_intents WHERE company=$1 ORDER BY created_at DESC",
    )
    .bind(company)
    .fetch_all(store.pool())
    .await?;
    rows.iter().map(intent_from_row).collect()
}

pub async fn payment_by_provider_id(
    store: &crate::authority::AuthorityStore,
    company: &str,
    provider_transfer_id: &str,
) -> Result<Option<PaymentIntent>> {
    let row = sqlx::query(
        "SELECT * FROM restless_authority.payment_intents \
         WHERE company=$1 AND provider_transfer_id=$2",
    )
    .bind(company)
    .bind(provider_transfer_id)
    .fetch_optional(store.pool())
    .await?;
    row.as_ref().map(intent_from_row).transpose()
}

pub async fn mark_unknown(
    store: &crate::authority::AuthorityStore,
    company: &str,
    key: &str,
) -> Result<PaymentIntent> {
    let row = sqlx::query(
        "UPDATE restless_authority.payment_intents SET state='unknown',updated_at=now() \
         WHERE company=$1 AND idempotency_key=$2 \
           AND state IN ('reserved','submitted','unknown') RETURNING *",
    )
    .bind(company)
    .bind(key)
    .fetch_optional(store.pool())
    .await?
    .context("payment cannot be marked unknown from its current state")?;
    let intent = intent_from_row(&row)?;
    store
        .emit(
            company,
            "payment_outcome_unknown",
            Some("daemon"),
            serde_json::json!({
                "idempotency_key": key,
                "amount_minor": intent.request.amount_minor,
                "currency": intent.request.currency,
                "reservation_retained": true,
            }),
        )
        .await?;
    Ok(intent)
}

pub async fn confirm_provider_state(
    store: &crate::authority::AuthorityStore,
    company: &str,
    key: &str,
    provider_transfer_id: &str,
    raw_status: &str,
    approval_url: Option<&str>,
) -> Result<ProviderObservation> {
    if provider_transfer_id.trim().is_empty() || raw_status.trim().is_empty() {
        bail!("provider confirmation needs transfer id and raw status");
    }
    let state = map_airwallex_status(raw_status);
    let mut tx = store.pool().begin().await?;
    let current_row = sqlx::query(
        "SELECT * FROM restless_authority.payment_intents \
         WHERE company=$1 AND idempotency_key=$2 FOR UPDATE",
    )
    .bind(company)
    .bind(key)
    .fetch_optional(&mut *tx)
    .await?
    .context("payment intent missing")?;
    let current = intent_from_row(&current_row)?;
    if current
        .provider_transfer_id
        .as_deref()
        .is_some_and(|existing| existing != provider_transfer_id)
    {
        bail!("payment provider transfer id changed");
    }
    let effective_approval_url = approval_url
        .map(str::to_string)
        .or_else(|| current.provider_approval_url.clone());
    let changed = current.provider_transfer_id.as_deref() != Some(provider_transfer_id)
        || current.raw_provider_status.as_deref() != Some(raw_status)
        || current.state != state
        || current.provider_approval_url != effective_approval_url;
    if !changed {
        tx.commit().await?;
        return Ok(ProviderObservation {
            payment: current,
            changed: false,
        });
    }
    let row = sqlx::query(
        "UPDATE restless_authority.payment_intents SET \
           provider_transfer_id=$3, raw_provider_status=$4, \
           provider_approval_url=COALESCE($5,provider_approval_url), \
           state=$6, \
           settled_at=CASE WHEN $6='settled' THEN COALESCE(settled_at,now()) ELSE settled_at END, \
           updated_at=now() \
         WHERE company=$1 AND idempotency_key=$2 RETURNING *",
    )
    .bind(company)
    .bind(key)
    .bind(provider_transfer_id)
    .bind(raw_status)
    .bind(approval_url)
    .bind(state.as_str())
    .fetch_one(&mut *tx)
    .await?;
    let intent = intent_from_row(&row)?;
    sqlx::query(
        "INSERT INTO restless_authority.records (company,kind,actor_id,body) \
         VALUES ($1,'payment_provider_observed','daemon',$2)",
    )
    .bind(company)
    .bind(serde_json::json!({
        "idempotency_key": key,
        "provider": "airwallex",
        "provider_transfer_id": provider_transfer_id,
        "raw_status": raw_status,
        "state": intent.state,
        "observed_at": Utc::now(),
    }))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(ProviderObservation {
        payment: intent,
        changed: true,
    })
}

pub fn map_airwallex_status(raw: &str) -> PaymentState {
    match raw.trim().to_ascii_uppercase().as_str() {
        "IN_APPROVAL" => PaymentState::InApproval,
        "SCHEDULED" => PaymentState::Scheduled,
        "PROCESSING" | "SENT" | "OVERDUE" => PaymentState::Processing,
        "APPROVAL_BLOCKED" => PaymentState::Blocked,
        "PAID" => PaymentState::Settled,
        "APPROVAL_REJECTED" => PaymentState::Rejected,
        "APPROVAL_RECALLED" | "CANCELLED" => PaymentState::Cancelled,
        "FAILED" => PaymentState::Failed,
        "CREATED" => PaymentState::Submitted,
        _ => PaymentState::Unknown,
    }
}

fn validate_envelope(input: &MoneyEnvelopeInput) -> Result<()> {
    normalise_currency(&input.currency)?;
    if input.source_account_ref.trim().is_empty() {
        bail!("money envelope needs an exact source account reference");
    }
    if input.beneficiary_refs.is_empty()
        || input
            .beneficiary_refs
            .iter()
            .any(|value| value.trim().is_empty())
    {
        bail!("money envelope needs at least one existing provider beneficiary reference");
    }
    if input.per_payment_limit_minor <= 0
        || input.aggregate_limit_minor <= 0
        || input.per_payment_limit_minor > input.aggregate_limit_minor
    {
        bail!("money limits must be positive and per-payment cannot exceed aggregate");
    }
    Ok(())
}

fn validate_intent(input: &mut PaymentIntentInput) -> Result<()> {
    input.currency = normalise_currency(&input.currency)?;
    if input.amount_minor <= 0 {
        bail!("payment amount_minor must be positive");
    }
    for (label, value, max) in [
        ("source account", input.source_account_ref.as_str(), 200),
        (
            "provider beneficiary",
            input.provider_beneficiary_ref.as_str(),
            200,
        ),
        ("purpose", input.purpose.as_str(), 500),
        ("idempotency key", input.idempotency_key.as_str(), 100),
        ("requesting actor", input.requesting_actor.as_str(), 100),
    ] {
        if value.trim().is_empty() || value.chars().count() > max {
            bail!("{label} must contain between 1 and {max} characters");
        }
    }
    if input.evidence_refs.len() > 32
        || input
            .evidence_refs
            .iter()
            .any(|value| value.trim().is_empty() || value.chars().count() > 2_048)
    {
        bail!("payment may carry at most 32 bounded evidence references");
    }
    Ok(())
}

fn normalise_currency(value: &str) -> Result<String> {
    let currency = value.trim().to_ascii_uppercase();
    if currency.len() != 3 || !currency.bytes().all(|byte| byte.is_ascii_uppercase()) {
        bail!("currency must be an explicit three-letter ISO code");
    }
    Ok(currency)
}

fn intent_fingerprint(input: &PaymentIntentInput) -> Result<String> {
    let bytes = serde_json::to_vec(input)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn envelope_from_row(row: &sqlx::postgres::PgRow) -> Result<MoneyEnvelope> {
    Ok(MoneyEnvelope {
        limits: MoneyEnvelopeInput {
            source_account_ref: row.get("source_account_ref"),
            currency: row.get("currency"),
            beneficiary_refs: serde_json::from_value(row.get("beneficiary_refs"))?,
            per_payment_limit_minor: row.get("per_payment_limit_minor"),
            aggregate_limit_minor: row.get("aggregate_limit_minor"),
            frozen: row.get("frozen"),
        },
        period_started_at: row.get("period_started_at"),
        updated_by: row.get("updated_by"),
        updated_at: row.get("updated_at"),
    })
}

fn intent_from_row(row: &sqlx::postgres::PgRow) -> Result<PaymentIntent> {
    Ok(PaymentIntent {
        request: PaymentIntentInput {
            work_id: row
                .try_get("work_id")
                .context("payment intent predates required Work linkage")?,
            owner_handoff_id: row
                .try_get("owner_handoff_id")
                .context("payment intent predates required owner-handoff linkage")?,
            source_account_ref: row.get("source_account_ref"),
            provider_beneficiary_ref: row.get("provider_beneficiary_ref"),
            amount_minor: row.get("amount_minor"),
            currency: row.get("currency"),
            purpose: row.get("purpose"),
            evidence_refs: serde_json::from_value(row.get("evidence_refs"))?,
            idempotency_key: row.get("idempotency_key"),
            requesting_actor: row.get("requesting_actor"),
        },
        state: PaymentState::from_db(row.get::<String, _>("state").as_str())?,
        provider: row.get("provider"),
        provider_transfer_id: row.get("provider_transfer_id"),
        raw_provider_status: row.get("raw_provider_status"),
        provider_approval_url: row.get("provider_approval_url"),
        settled_at: row.get("settled_at"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent(key: &str, amount_minor: i64) -> PaymentIntentInput {
        PaymentIntentInput {
            work_id: Uuid::new_v4(),
            owner_handoff_id: Uuid::new_v4(),
            source_account_ref: "wallet-aud".into(),
            provider_beneficiary_ref: "beneficiary-1".into(),
            amount_minor,
            currency: "AUD".into(),
            purpose: "Bounded test invoice".into(),
            evidence_refs: vec!["/company/evidence/invoice.pdf".into()],
            idempotency_key: key.into(),
            requesting_actor: "exec".into(),
        }
    }

    /// A payee is added only by the owner's answer, and adding one keeps the allowance's limits
    /// and running total.
    #[tokio::test]
    async fn a_payee_joins_an_allowance_only_on_the_owners_approval() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping payee scenario");
            return;
        };
        let company = format!("payee_{}_test", Uuid::new_v4().simple());
        let store = crate::authority::AuthorityStore::connect(&database_url).await.unwrap();
        set_envelope(
            &store,
            &company,
            MoneyEnvelopeInput {
                source_account_ref: "wallet-aud".into(),
                currency: "AUD".into(),
                beneficiary_refs: vec!["beneficiary-1".into()],
                per_payment_limit_minor: 1_000,
                aggregate_limit_minor: 5_000,
                frozen: false,
            },
            "owner",
        )
        .await
        .unwrap();
        let before = envelopes(&store, &company).await.unwrap().remove(0);
        let proposal = |beneficiary: &str| PayeeProposalInput {
            currency: "aud".into(),
            beneficiary_ref: beneficiary.into(),
            reason: "The printer for the open day.".into(),
            work_id: None,
        };
        let declined = propose_payee(&store, &company, "exec", proposal("beneficiary-2")).await.unwrap();
        decide_payee(&store, &company, "owner", declined, false, None).await.unwrap();
        assert_eq!(envelopes(&store, &company).await.unwrap()[0].limits.beneficiary_refs, ["beneficiary-1"]);
        let asked = propose_payee(&store, &company, "exec", proposal("beneficiary-2")).await.unwrap();
        assert!(propose_payee(&store, &company, "exec", proposal("beneficiary-2")).await.is_err());
        decide_payee(&store, &company, "owner", asked, true, None).await.unwrap();
        assert!(decide_payee(&store, &company, "owner", asked, true, None).await.is_err());
        let after = &envelopes(&store, &company).await.unwrap()[0];
        assert_eq!(after.limits.beneficiary_refs, ["beneficiary-1", "beneficiary-2"]);
        assert_eq!(after.period_started_at, before.period_started_at);
        assert_eq!(after.limits.aggregate_limit_minor, 5_000);
        store.delete_test_company(&company).await.unwrap();
    }

    #[test]
    fn provider_statuses_remain_distinct_and_unknown_is_honest() {
        assert_eq!(
            map_airwallex_status("IN_APPROVAL"),
            PaymentState::InApproval
        );
        assert_eq!(map_airwallex_status("SCHEDULED"), PaymentState::Scheduled);
        assert_eq!(map_airwallex_status("PAID"), PaymentState::Settled);
        assert_eq!(map_airwallex_status("FAILED"), PaymentState::Failed);
        assert_eq!(map_airwallex_status("FUTURE_STATE"), PaymentState::Unknown);
    }

    #[test]
    fn payment_fingerprint_binds_every_consequential_field() {
        let base = PaymentIntentInput {
            work_id: Uuid::new_v4(),
            owner_handoff_id: Uuid::new_v4(),
            source_account_ref: "wallet-aud".into(),
            provider_beneficiary_ref: "beneficiary-1".into(),
            amount_minor: 100,
            currency: "AUD".into(),
            purpose: "Pay a real invoice".into(),
            evidence_refs: vec!["/company/invoice.pdf".into()],
            idempotency_key: "invoice-1".into(),
            requesting_actor: "exec".into(),
        };
        let first = intent_fingerprint(&base).unwrap();
        for changed in [
            PaymentIntentInput {
                amount_minor: 101,
                ..base.clone()
            },
            PaymentIntentInput {
                provider_beneficiary_ref: "beneficiary-2".into(),
                ..base.clone()
            },
            PaymentIntentInput {
                source_account_ref: "wallet-other".into(),
                ..base.clone()
            },
            PaymentIntentInput {
                currency: "USD".into(),
                ..base.clone()
            },
        ] {
            assert_ne!(first, intent_fingerprint(&changed).unwrap());
        }
    }

    #[tokio::test]
    async fn concurrent_reservation_idempotency_unknown_and_freeze_are_fail_closed() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping finance Authority scenario");
            return;
        };
        let company = format!("finance_{}_test", Uuid::new_v4().simple());
        let store = crate::authority::AuthorityStore::connect(&database_url)
            .await
            .unwrap();
        store.delete_test_company(&company).await.unwrap();
        set_envelope(
            &store,
            &company,
            MoneyEnvelopeInput {
                source_account_ref: "wallet-aud".into(),
                currency: "aud".into(),
                beneficiary_refs: vec!["beneficiary-1".into()],
                per_payment_limit_minor: 100,
                aggregate_limit_minor: 100,
                frozen: false,
            },
            "owner",
        )
        .await
        .unwrap();

        let first = intent("concurrent-a", 60);
        let second = intent("concurrent-b", 60);
        let (a, b) = tokio::join!(
            reserve(&store, &company, first.clone()),
            reserve(&store, &company, second.clone())
        );
        assert_ne!(
            a.is_ok(),
            b.is_ok(),
            "exactly one concurrent reservation must fit"
        );
        let accepted = if a.is_ok() { first } else { second };

        let replay = reserve(&store, &company, accepted.clone()).await.unwrap();
        assert!(replay.replayed);
        let changed = PaymentIntentInput {
            amount_minor: accepted.amount_minor + 1,
            ..accepted.clone()
        };
        assert!(reserve(&store, &company, changed).await.is_err());

        mark_unknown(&store, &company, &accepted.idempotency_key)
            .await
            .unwrap();
        let restarted = crate::authority::AuthorityStore::connect(&database_url)
            .await
            .unwrap();
        assert_eq!(
            payment(&restarted, &company, &accepted.idempotency_key)
                .await
                .unwrap()
                .unwrap()
                .state,
            PaymentState::Unknown
        );
        assert!(reserve(&restarted, &company, intent("over-retained", 41))
            .await
            .is_err());

        let observed = confirm_provider_state(
            &restarted,
            &company,
            &accepted.idempotency_key,
            "transfer-1",
            "APPROVAL_REJECTED",
            Some("https://www.airwallex.com/app/transfers"),
        )
        .await
        .unwrap();
        assert!(observed.changed);
        assert_eq!(observed.payment.state, PaymentState::Rejected);
        let resubmitted = confirm_provider_state(
            &restarted,
            &company,
            &accepted.idempotency_key,
            "transfer-1",
            "IN_APPROVAL",
            Some("https://www.airwallex.com/app/transfers"),
        )
        .await
        .unwrap();
        assert!(resubmitted.changed);
        assert_eq!(resubmitted.payment.state, PaymentState::InApproval);
        let duplicate = confirm_provider_state(
            &restarted,
            &company,
            &accepted.idempotency_key,
            "transfer-1",
            "IN_APPROVAL",
            Some("https://www.airwallex.com/app/transfers"),
        )
        .await
        .unwrap();
        assert!(!duplicate.changed);
        let paid = confirm_provider_state(
            &restarted,
            &company,
            &accepted.idempotency_key,
            "transfer-1",
            "PAID",
            Some("https://www.airwallex.com/app/transfers"),
        )
        .await
        .unwrap();
        assert_eq!(paid.payment.state, PaymentState::Settled);
        assert!(paid.payment.settled_at.is_some());
        let returned = confirm_provider_state(
            &restarted,
            &company,
            &accepted.idempotency_key,
            "transfer-1",
            "FAILED",
            Some("https://www.airwallex.com/app/transfers"),
        )
        .await
        .unwrap();
        assert_eq!(returned.payment.state, PaymentState::Failed);

        set_frozen(&restarted, &company, "AUD", true, "owner")
            .await
            .unwrap();
        assert!(reserve(&restarted, &company, intent("frozen", 1))
            .await
            .is_err());
        assert_eq!(payments(&restarted, &company).await.unwrap().len(), 1);
        restarted.delete_test_company(&company).await.unwrap();
    }
}
