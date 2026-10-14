//! Narrow, durable root email mandates and one-use Exec permits.
//!
//! Exec supplies the judgement and evidence for each recipient and message.
//! This module checks the verifiable subset: active root, sender/recipient and
//! payload binding, expiry, timezone-aware quota, and single consumption.

use std::collections::HashMap;
use std::str::FromStr;

use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Row as _, Transaction};
use uuid::Uuid;

const MAX_PERMIT_LIFETIME_SECS: i64 = 5 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMandate {
    pub id: Uuid,
    pub purpose: String,
    pub audience_guidance: String,
    pub sender: String,
    #[serde(default)]
    pub sender_name: Option<String>,
    pub max_per_day: u32,
    pub max_total: u32,
    pub timezone: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewEmailMandate {
    pub purpose: String,
    pub audience_guidance: String,
    pub sender: String,
    #[serde(default)]
    pub sender_name: Option<String>,
    pub max_per_day: u32,
    pub max_total: u32,
    pub timezone: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmailPermitProposal {
    pub sender: String,
    pub recipient: String,
    pub payload_sha256: String,
    pub effect_key: String,
    pub rationale: String,
    pub evidence_refs: Vec<String>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailPermit {
    pub id: Uuid,
    pub mandate_id: Uuid,
    pub mandate_version: i64,
    pub sender: String,
    pub recipient: String,
    pub payload_sha256: String,
    pub effect_key: String,
    pub rationale: String,
    pub evidence_refs: Vec<String>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailReservation {
    pub permit_id: Uuid,
    pub effect_key: String,
    pub reserved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMandateUsage {
    pub mandate_id: Uuid,
    pub usage_day: String,
    pub used_today: u32,
    pub used_total: u32,
    pub limit_per_day: u32,
    pub limit_total: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderOutcome {
    ConfirmedSent,
    ConfirmedNotSent,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReservationState {
    Reserved,
    ConfirmedSent,
    ConfirmedNotSent,
    Unknown,
}

fn normalize_mailbox(value: &str) -> Result<String> {
    let value = value.trim().to_ascii_lowercase();
    if value.len() > 320 {
        bail!("email address exceeds 320 bytes");
    }
    let (local, domain) = value
        .split_once('@')
        .context("email address must contain one @")?;
    if local.is_empty()
        || domain.is_empty()
        || domain.starts_with('.')
        || domain.ends_with('.')
        || value
            .bytes()
            .any(|b| b.is_ascii_whitespace() || b == b'<' || b == b'>')
    {
        bail!("email address is not a valid normalized mailbox");
    }
    if domain.contains('@') || local.contains('@') {
        bail!("email address must contain one @");
    }
    Ok(format!("{local}@{domain}"))
}

fn normalize_sender_name(value: Option<&str>) -> Result<Option<String>> {
    value.map(|name| {
        let name = name.trim();
        if name.is_empty()
            || name.chars().count() > 120
            || name.chars().any(|c| c.is_control() || matches!(c, '<' | '>'))
        {
            bail!("sender name must be at most 120 characters without control or address delimiters");
        }
        Ok(name.to_owned())
    }).transpose()
}

fn validate_digest(value: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("payload digest must be 64 hexadecimal SHA256 characters");
    }
    Ok(())
}

pub async fn lock_company(tx: &mut Transaction<'_, Postgres>, company: &str) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
        .bind(format!("email-authority:{company}"))
        .execute(&mut **tx)
        .await
        .context("serialize email authority operation")?;
    Ok(())
}

async fn current_root(
    tx: &mut Transaction<'_, Postgres>,
    company: &str,
    mandate_id: Uuid,
) -> Result<(EmailMandate, i64)> {
    let row = sqlx::query(
        "SELECT id, body FROM restless_authority.records \
         WHERE company=$1 AND kind='email_mandate_granted' AND body->>'id'=$2 \
         ORDER BY id DESC LIMIT 1",
    )
    .bind(company)
    .bind(mandate_id.to_string())
    .fetch_optional(&mut **tx)
    .await?;
    let row = row.context("email mandate does not exist")?;
    let version: i64 = row.try_get("id")?;
    let body: Value = row.try_get("body")?;
    let mandate: EmailMandate = serde_json::from_value(body).context("decode email mandate")?;
    let revoked: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM restless_authority.records \
         WHERE company=$1 AND kind='email_mandate_revoked' \
           AND body->>'mandate_id'=$2 AND (body->>'mandate_version')::bigint=$3)",
    )
    .bind(company)
    .bind(mandate_id.to_string())
    .bind(version)
    .fetch_one(&mut **tx)
    .await?;
    if revoked {
        bail!("email mandate has been revoked");
    }
    Ok((mandate, version))
}

pub async fn grant_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    company: &str,
    owner_actor_id: &str,
    mandate: NewEmailMandate,
) -> Result<EmailMandate> {
    if owner_actor_id.trim().is_empty() {
        bail!("owner attribution is required to grant an email mandate");
    }
    let mandate = canonicalize_new_mandate(mandate)?;
    let now = Utc::now();
    let mandate = EmailMandate {
        id: Uuid::new_v4(),
        purpose: mandate.purpose,
        audience_guidance: mandate.audience_guidance,
        sender: mandate.sender,
        sender_name: mandate.sender_name,
        max_per_day: mandate.max_per_day,
        max_total: mandate.max_total,
        timezone: mandate.timezone,
        expires_at: mandate.expires_at,
        created_at: now,
    };
    let grants = sqlx::query(
        "SELECT id,body FROM restless_authority.records \
         WHERE company=$1 AND kind='email_mandate_granted' ORDER BY id DESC",
    )
    .bind(company)
    .fetch_all(&mut **tx)
    .await?;
    for row in grants {
        let grant_id: i64 = row.try_get("id")?;
        let body: Value = row.try_get("body")?;
        let existing: EmailMandate = serde_json::from_value(body)?;
        if existing.expires_at <= now {
            continue;
        }
        let revoked: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM restless_authority.records \
             WHERE company=$1 AND kind='email_mandate_revoked' AND body->>'mandate_id'=$2 \
               AND (body->>'mandate_version')::bigint=$3)",
        )
        .bind(company)
        .bind(existing.id.to_string())
        .bind(grant_id)
        .fetch_one(&mut **tx)
        .await?;
        if !revoked {
            bail!("company already has an active email root mandate");
        }
    }
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM restless_authority.records \
         WHERE company=$1 AND kind='email_mandate_granted' AND body->>'id'=$2)",
    )
    .bind(company)
    .bind(mandate.id.to_string())
    .fetch_one(&mut **tx)
    .await?;
    if exists {
        bail!("email mandate ids are immutable and cannot be granted twice");
    }
    sqlx::query(
        "INSERT INTO restless_authority.records (company,kind,actor_id,body) \
         VALUES ($1,'email_mandate_granted',$2,$3)",
    )
    .bind(company)
    .bind(owner_actor_id)
    .bind(serde_json::to_value(&mandate)?)
    .execute(&mut **tx)
    .await?;
    Ok(mandate)
}

pub fn canonicalize_new_mandate(mut mandate: NewEmailMandate) -> Result<NewEmailMandate> {
    if mandate.purpose.trim().is_empty() || mandate.audience_guidance.trim().is_empty() {
        bail!("email mandate needs a purpose and audience guidance");
    }
    if mandate.purpose.trim().len() > 2_000 || mandate.audience_guidance.trim().len() > 8_000 {
        bail!("email mandate purpose or audience guidance exceeds its size limit");
    }
    mandate.sender = normalize_mailbox(&mandate.sender)?;
    mandate.sender_name = normalize_sender_name(mandate.sender_name.as_deref())?;
    mandate.purpose = mandate.purpose.trim().to_owned();
    mandate.audience_guidance = mandate.audience_guidance.trim().to_owned();
    if mandate.max_per_day == 0 || mandate.max_total == 0 || mandate.max_per_day > mandate.max_total
    {
        bail!("email mandate limits must be positive and daily limit cannot exceed total limit");
    }
    Tz::from_str(&mandate.timezone).context("timezone must be a valid IANA timezone")?;
    let now = Utc::now();
    if mandate.expires_at <= now {
        bail!("email mandate must expire in the future");
    }
    Ok(mandate)
}

pub async fn list(pool: &PgPool, company: &str) -> Result<Vec<EmailMandate>> {
    let rows = sqlx::query(
        "SELECT body FROM restless_authority.records \
         WHERE company=$1 AND kind='email_mandate_granted' ORDER BY id DESC",
    )
    .bind(company)
    .fetch_all(pool)
    .await?;
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for row in rows {
        let body: Value = row.try_get("body")?;
        let mandate: EmailMandate = serde_json::from_value(body)?;
        if seen.insert(mandate.id) {
            let revoked: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM restless_authority.records \
                 WHERE company=$1 AND kind='email_mandate_revoked' AND body->>'mandate_id'=$2)",
            )
            .bind(company)
            .bind(mandate.id.to_string())
            .fetch_one(pool)
            .await?;
            if !revoked {
                result.push(mandate);
            }
        }
    }
    Ok(result)
}

pub async fn revoke(
    pool: &PgPool,
    company: &str,
    owner_actor_id: &str,
    mandate_id: Uuid,
    reason: &str,
) -> Result<()> {
    if owner_actor_id.trim().is_empty() || reason.trim().is_empty() {
        bail!("owner attribution and revocation reason are required");
    }
    if reason.len() > 2_000 {
        bail!("email mandate revocation reason exceeds its size limit");
    }
    let mut tx = pool.begin().await?;
    lock_company(&mut tx, company).await?;
    revoke_in_transaction(&mut tx, company, owner_actor_id, mandate_id, reason).await?;
    tx.commit().await?;
    Ok(())
}

/// Revoke one current mandate inside a transaction that already holds the company lock.
pub async fn revoke_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    company: &str,
    owner_actor_id: &str,
    mandate_id: Uuid,
    reason: &str,
) -> Result<()> {
    let (_, version) = current_root(tx, company, mandate_id).await?;
    sqlx::query(
        "INSERT INTO restless_authority.records (company,kind,actor_id,body) \
         VALUES ($1,'email_mandate_revoked',$2,$3)",
    )
    .bind(company)
    .bind(owner_actor_id)
    .bind(serde_json::json!({
        "mandate_id": mandate_id,
        "mandate_version": version,
        "reason": reason.trim(),
        "revoked_at": Utc::now(),
    }))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn issue(
    pool: &PgPool,
    company: &str,
    issuer_actor_id: &str,
    mandate_id: Uuid,
    mut proposal: EmailPermitProposal,
) -> Result<EmailPermit> {
    if issuer_actor_id != "exec" {
        bail!("only the authenticated Exec actor may issue an email permit");
    }
    proposal.recipient = normalize_mailbox(&proposal.recipient)?;
    proposal.sender = normalize_mailbox(&proposal.sender)?;
    validate_digest(&proposal.payload_sha256)?;
    if proposal.effect_key.trim().is_empty() || proposal.effect_key.len() > 200 {
        bail!("email effect key must contain 1 to 200 characters");
    }
    if proposal.effect_key.chars().any(char::is_control) {
        bail!("email effect key must not contain control characters");
    }
    if proposal.rationale.trim().is_empty() || proposal.rationale.len() > 4_000 {
        bail!("Exec must record a bounded, nonempty decision rationale");
    }
    if proposal.evidence_refs.is_empty()
        || proposal.evidence_refs.len() > 16
        || proposal
            .evidence_refs
            .iter()
            .any(|r| r.trim().is_empty() || r.len() > 1_000)
    {
        bail!("Exec must provide 1 to 16 bounded evidence references");
    }
    let now = Utc::now();
    if proposal.expires_at <= now
        || proposal.expires_at > now + chrono::Duration::seconds(MAX_PERMIT_LIFETIME_SECS)
    {
        bail!("email permit expiry must be within five minutes");
    }
    let mut tx = pool.begin().await?;
    lock_company(&mut tx, company).await?;
    let (root, version) = current_root(&mut tx, company, mandate_id).await?;
    if root.expires_at <= now {
        bail!("email mandate has expired");
    }
    if normalize_mailbox(&root.sender)? != root.sender {
        bail!("email mandate sender is not normalized");
    }
    if proposal.sender != root.sender {
        bail!("proposed sender is outside the email mandate");
    }
    let permit = EmailPermit {
        id: Uuid::new_v4(),
        mandate_id,
        mandate_version: version,
        sender: proposal.sender,
        recipient: proposal.recipient,
        payload_sha256: proposal.payload_sha256.to_ascii_lowercase(),
        effect_key: proposal.effect_key,
        rationale: proposal.rationale.trim().to_owned(),
        evidence_refs: proposal.evidence_refs,
        issued_at: now,
        expires_at: proposal.expires_at,
    };
    sqlx::query(
        "INSERT INTO restless_authority.records (company,kind,actor_id,body) \
         VALUES ($1,'email_permit_issued',$2,$3)",
    )
    .bind(company)
    .bind(issuer_actor_id)
    .bind(serde_json::to_value(&permit)?)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(permit)
}

pub async fn list_permits(
    pool: &PgPool,
    company: &str,
    mandate_id: Uuid,
) -> Result<Vec<EmailPermit>> {
    let rows = sqlx::query(
        "SELECT body FROM restless_authority.records \
         WHERE company=$1 AND kind='email_permit_issued' AND body->>'mandate_id'=$2 ORDER BY id DESC",
    )
    .bind(company)
    .bind(mandate_id.to_string())
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|row| -> Result<EmailPermit> { Ok(serde_json::from_value(row.try_get("body")?)?) })
        .collect()
}

pub async fn usage(
    pool: &PgPool,
    company: &str,
    mandate_id: Uuid,
) -> Result<EmailMandateUsage> {
    let row = sqlx::query(
        "SELECT body FROM restless_authority.records WHERE company=$1 \
         AND kind='email_mandate_granted' AND body->>'id'=$2 ORDER BY id DESC LIMIT 1",
    )
    .bind(company)
    .bind(mandate_id.to_string())
    .fetch_optional(pool)
    .await?
    .context("email mandate does not exist")?;
    let root: EmailMandate = serde_json::from_value(row.try_get("body")?)?;
    let timezone = Tz::from_str(&root.timezone).context("mandate timezone is invalid")?;
    let usage_day = Utc::now().with_timezone(&timezone).date_naive().to_string();
    let mandate_id_text = mandate_id.to_string();
    let mut states: HashMap<String, (String, ReservationState)> = HashMap::new();
    let rows = sqlx::query(
        "SELECT kind,body FROM restless_authority.records WHERE company=$1 \
         AND kind IN ('email_send_reserved','email_send_status') ORDER BY id",
    )
    .bind(company)
    .fetch_all(pool)
    .await?;
    for row in rows {
        let kind: String = row.try_get("kind")?;
        let body: Value = row.try_get("body")?;
        let key = body
            .get("effect_key")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if key.is_empty() {
            continue;
        }
        if kind == "email_send_reserved"
            && body.get("mandate_id").and_then(Value::as_str) == Some(mandate_id_text.as_str())
        {
            let day = body
                .get("usage_day")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            states.insert(key.to_owned(), (day, ReservationState::Reserved));
        } else if kind == "email_send_status" {
            let next = match body.get("outcome").and_then(Value::as_str) {
                Some("confirmed_sent") => ReservationState::ConfirmedSent,
                Some("confirmed_not_sent") => ReservationState::ConfirmedNotSent,
                Some("unknown") => ReservationState::Unknown,
                _ => continue,
            };
            if let Some((_, state)) = states.get_mut(key) {
                if !matches!(
                    *state,
                    ReservationState::ConfirmedSent | ReservationState::ConfirmedNotSent
                ) {
                    *state = next;
                }
            }
        }
    }
    let mut used_total = 0_u32;
    let mut used_today = 0_u32;
    for (day, state) in states.values() {
        if *state == ReservationState::ConfirmedNotSent {
            continue;
        }
        used_total = used_total.saturating_add(1);
        if day == &usage_day {
            used_today = used_today.saturating_add(1);
        }
    }
    Ok(EmailMandateUsage {
        mandate_id,
        usage_day,
        used_today,
        used_total,
        limit_per_day: root.max_per_day,
        limit_total: root.max_total,
    })
}

pub async fn reserve(
    pool: &PgPool,
    company: &str,
    permit_id: Uuid,
    actual_sender: &str,
    actual_sender_name: Option<&str>,
    recipient: &str,
    payload_sha256: &str,
    effect_key: &str,
) -> Result<EmailReservation> {
    let actual_sender = normalize_mailbox(actual_sender)?;
    let actual_sender_name = normalize_sender_name(actual_sender_name)?;
    let recipient = normalize_mailbox(recipient)?;
    validate_digest(payload_sha256)?;
    let now = Utc::now();
    let mut tx = pool.begin().await?;
    lock_company(&mut tx, company).await?;
    let row = sqlx::query(
        "SELECT actor_id,body FROM restless_authority.records \
         WHERE company=$1 AND kind='email_permit_issued' AND body->>'id'=$2 ORDER BY id DESC LIMIT 1",
    )
    .bind(company)
    .bind(permit_id.to_string())
    .fetch_optional(&mut *tx)
    .await?
    .context("email permit does not exist")?;
    let issuer: Option<String> = row.try_get("actor_id")?;
    if issuer.as_deref() != Some("exec") {
        bail!("email permit is not attributed to Exec");
    }
    let permit: EmailPermit = serde_json::from_value(row.try_get("body")?)?;
    let (root, version) = current_root(&mut tx, company, permit.mandate_id).await?;
    if version != permit.mandate_version || root.expires_at <= now {
        bail!("email mandate is no longer active at the permit version");
    }
    if permit.expires_at <= now {
        bail!("email permit has expired");
    }
    if normalize_mailbox(&root.sender)? != actual_sender
        || root.sender_name != actual_sender_name
        || permit.sender != actual_sender
        || recipient != permit.recipient
        || payload_sha256.to_ascii_lowercase() != permit.payload_sha256
        || effect_key != permit.effect_key
    {
        bail!("actual email does not match the exact permit");
    }
    let prior: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM restless_authority.records \
         WHERE company=$1 AND kind='email_send_reserved' \
           AND (body->>'permit_id'=$2 OR body->>'effect_key'=$3))",
    )
    .bind(company)
    .bind(permit_id.to_string())
    .bind(effect_key)
    .fetch_one(&mut *tx)
    .await?;
    if prior {
        bail!("email permit or effect key has already been consumed");
    }
    // A reply is not outreach: once this correspondent has written in after an earlier email
    // to them, Exec may answer in that thread. Only an outcome nobody knows still refuses.
    let latest_inbound: Option<i64> = sqlx::query_scalar(
        "SELECT max(id) FROM restless_authority.records \
         WHERE company=$1 AND kind='inbound_effect' \
           AND body->>'type' IN ('email.received','email.inbound') \
           AND (lower(body->>'party')=$2 OR lower(body->>'party') LIKE '%<' || $2 || '>')",
    )
    .bind(company)
    .bind(recipient.as_str())
    .fetch_one(&mut *tx)
    .await?;
    let answered = |id: i64| latest_inbound.is_some_and(|inbound| inbound > id);
    let prior_recipient_reservations = sqlx::query(
        "SELECT id, body FROM restless_authority.records \
         WHERE company=$1 AND kind='email_send_reserved' AND lower(body->>'recipient')=$2",
    )
    .bind(company)
    .bind(recipient.as_str())
    .fetch_all(&mut *tx)
    .await?;
    for prior in prior_recipient_reservations {
        let prior_id: i64 = prior.try_get("id")?;
        let body: Value = prior.try_get("body")?;
        let prior_key = body
            .get("effect_key")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let last_outcome: Option<String> = sqlx::query_scalar(
            "SELECT body->>'outcome' FROM restless_authority.records \
             WHERE company=$1 AND kind='email_send_status' AND body->>'effect_key'=$2 \
             ORDER BY id DESC LIMIT 1",
        )
        .bind(company)
        .bind(prior_key)
        .fetch_optional(&mut *tx)
        .await?;
        match last_outcome.as_deref() {
            Some("confirmed_not_sent") => {}
            Some("confirmed_sent") if answered(prior_id) => {}
            Some("confirmed_sent") => bail!(
                "recipient already has a sent outbound email under an email mandate and has not written back since"
            ),
            _ => bail!(
                "recipient already has an uncertain outbound email under an email mandate; reconcile it before replying"
            ),
        }
    }
    for receipt in sqlx::query(
        "SELECT id, body FROM restless_authority.records WHERE company=$1 AND kind='effect'",
    )
    .bind(company)
    .fetch_all(&mut *tx)
    .await?
    {
        let body: Value = receipt.try_get("body")?;
        if !answered(receipt.try_get("id")?)
            && body.get("effect_class").and_then(Value::as_str) == Some("customer-contact.email")
            && body
                .get("party")
                .and_then(Value::as_str)
                .is_some_and(|party| party.eq_ignore_ascii_case(&recipient))
            && body.get("success").and_then(Value::as_bool) == Some(true)
        {
            bail!("recipient already has a successful customer-contact.email receipt");
        }
    }
    // A generic attempt the correspondent has since answered is settled only if it has a result.
    let prior_generic_intent: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM restless_authority.records intent \
         WHERE intent.company=$1 AND intent.kind='effect_intent' \
           AND intent.body->>'effect_class'='customer-contact.email' \
           AND lower(intent.body->>'party')=$2 \
           AND NOT (intent.id < COALESCE($3, 0) AND EXISTS(SELECT 1 FROM restless_authority.records receipt \
             WHERE receipt.company=$1 AND receipt.kind='effect' \
               AND receipt.body->>'idempotency_key'=intent.body->>'idempotency_key' \
               AND receipt.body->>'execution_no'=intent.body->>'execution_no')))",
    )
    .bind(company)
    .bind(recipient.as_str())
    .bind(latest_inbound)
    .fetch_one(&mut *tx)
    .await?;
    if prior_generic_intent {
        bail!("recipient has a prior generic email attempt; reconcile its provider outcome before delegated outreach");
    }
    let timezone = Tz::from_str(&root.timezone).context("mandate timezone is invalid")?;
    let usage_day = now.with_timezone(&timezone).date_naive().to_string();
    let rows = sqlx::query(
        "SELECT kind,body FROM restless_authority.records \
         WHERE company=$1 AND kind IN ('email_send_reserved','email_send_status') ORDER BY id",
    )
    .bind(company)
    .fetch_all(&mut *tx)
    .await?;
    let mut reservations: HashMap<String, (String, ReservationState)> = HashMap::new();
    let mandate_id_text = permit.mandate_id.to_string();
    for row in rows {
        let body: Value = row.try_get("body")?;
        let key = body
            .get("effect_key")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        if key.is_empty() {
            continue;
        }
        let kind: String = row.try_get("kind")?;
        if kind == "email_send_reserved"
            && body.get("mandate_id").and_then(Value::as_str) == Some(mandate_id_text.as_str())
        {
            let day = body
                .get("usage_day")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            reservations.insert(key, (day, ReservationState::Reserved));
        } else if kind == "email_send_status" {
            let state = match body.get("outcome").and_then(Value::as_str) {
                Some("confirmed_sent") => ReservationState::ConfirmedSent,
                Some("confirmed_not_sent") => ReservationState::ConfirmedNotSent,
                Some("unknown") => ReservationState::Unknown,
                _ => continue,
            };
            if let Some((day, old)) = reservations.get_mut(&key) {
                if *old != ReservationState::ConfirmedSent
                    && *old != ReservationState::ConfirmedNotSent
                {
                    *old = state;
                }
                let _ = day;
            }
        }
    }
    let mut total = 0_u32;
    let mut daily = 0_u32;
    for (day, state) in reservations.values() {
        if *state == ReservationState::ConfirmedNotSent {
            continue;
        }
        total = total.saturating_add(1);
        if day == &usage_day {
            daily = daily.saturating_add(1);
        }
    }
    if total >= root.max_total || daily >= root.max_per_day {
        bail!("email mandate quota is exhausted");
    }
    let reserved_at = now;
    sqlx::query(
        "INSERT INTO restless_authority.records (company,kind,actor_id,body) \
         VALUES ($1,'email_send_reserved','exec',$2)",
    )
    .bind(company)
    .bind(serde_json::json!({
        "permit_id": permit_id,
        "mandate_id": permit.mandate_id,
        "mandate_version": permit.mandate_version,
        "recipient": recipient,
        "sender": actual_sender,
        "sender_name": actual_sender_name,
        "payload_sha256": payload_sha256.to_ascii_lowercase(),
        "effect_key": effect_key,
        "usage_day": usage_day,
        "status": "reserved",
        "reserved_at": reserved_at,
    }))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(EmailReservation {
        permit_id,
        effect_key: effect_key.to_owned(),
        reserved_at,
    })
}

pub async fn record_status(
    pool: &PgPool,
    company: &str,
    permit_id: Uuid,
    effect_key: &str,
    outcome: ProviderOutcome,
    provider_ref: Option<&str>,
    provider_detail: Option<&str>,
) -> Result<()> {
    if provider_detail.is_some_and(|detail| detail.len() > 500) {
        bail!("email provider detail exceeds 500 bytes");
    }
    let mut tx = pool.begin().await?;
    lock_company(&mut tx, company).await?;
    let row = sqlx::query(
        "SELECT body FROM restless_authority.records WHERE company=$1 AND kind='email_send_reserved' \
         AND body->>'permit_id'=$2 ORDER BY id DESC LIMIT 1",
    )
    .bind(company)
    .bind(permit_id.to_string())
    .fetch_optional(&mut *tx)
    .await?
    .context("email permit has no send reservation")?;
    let reserved: Value = row.try_get("body")?;
    if reserved.get("effect_key").and_then(Value::as_str) != Some(effect_key) {
        bail!("email status effect key does not match reservation");
    }
    let latest = sqlx::query_scalar::<_, Value>(
        "SELECT body FROM restless_authority.records WHERE company=$1 AND kind='email_send_status' \
         AND body->>'effect_key'=$2 ORDER BY id DESC LIMIT 1",
    )
    .bind(company)
    .bind(effect_key)
    .fetch_optional(&mut *tx)
    .await?;
    let previous = latest.and_then(|b| b.get("outcome").and_then(Value::as_str).map(str::to_owned));
    if matches!(
        previous.as_deref(),
        Some("confirmed_sent" | "confirmed_not_sent")
    ) {
        let desired = match outcome {
            ProviderOutcome::ConfirmedSent => "confirmed_sent",
            ProviderOutcome::ConfirmedNotSent => "confirmed_not_sent",
            ProviderOutcome::Unknown => "unknown",
        };
        if previous.as_deref() != Some(desired) {
            bail!("terminal email provider outcome cannot be overwritten");
        }
        tx.commit().await?;
        return Ok(());
    }
    if let Some(reference) = provider_ref {
        if outcome != ProviderOutcome::ConfirmedSent || reference.trim().is_empty() {
            bail!("provider reference is valid only for a confirmed send");
        }
    }
    if outcome == ProviderOutcome::ConfirmedSent && provider_ref.is_none() {
        bail!("a provider-accepted email needs its provider message ID");
    }
    let value = match outcome {
        ProviderOutcome::ConfirmedSent => "confirmed_sent",
        ProviderOutcome::ConfirmedNotSent => "confirmed_not_sent",
        ProviderOutcome::Unknown => "unknown",
    };
    sqlx::query(
        "INSERT INTO restless_authority.records (company,kind,actor_id,body) \
         VALUES ($1,'email_send_status','company/exec',$2)",
    )
    .bind(company)
    .bind(serde_json::json!({
        "permit_id": permit_id,
        "effect_key": effect_key,
        "outcome": value,
        "provider_ref": provider_ref,
        "provider_detail": provider_detail,
        "recorded_at": Utc::now(),
    }))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod replace_tests {
    use crate::authority::AuthorityStore;

    use super::*;

    fn proposal(per_day: u32) -> NewEmailMandate {
        NewEmailMandate {
            purpose: "Invite past clients to the open day".into(),
            audience_guidance: "People who bought in the last two years".into(),
            sender: "hello@example.com".into(),
            sender_name: None,
            max_per_day: per_day,
            max_total: 100,
            timezone: "UTC".into(),
            expires_at: Utc::now() + chrono::Duration::days(30),
        }
    }

    /// Changing a mandate is a proposal like any other: until approved the current mandate
    /// holds; approving replaces it, so exactly one mandate is ever current.
    #[tokio::test]
    async fn an_approved_change_replaces_the_current_mandate_and_nothing_else() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping mandate replacement scenario");
            return;
        };
        let authority = AuthorityStore::connect(&database_url).await.unwrap();
        let company = format!("mandate_replace_{}", Uuid::new_v4().simple());
        let first = authority
            .propose_email_mandate(&company, "exec", proposal(5), None, None)
            .await
            .unwrap();
        let current = authority
            .decide_email_mandate_proposal(&company, "owner", first, true, None)
            .await
            .unwrap()
            .unwrap();
        // A second new mandate is still refused while one is current.
        let rival = authority
            .propose_email_mandate(&company, "exec", proposal(6), None, None)
            .await
            .unwrap();
        assert!(authority
            .decide_email_mandate_proposal(&company, "owner", rival, true, None)
            .await
            .is_err());
        let change = authority
            .propose_email_mandate(&company, "owner", proposal(10), None, Some(current.id))
            .await
            .unwrap();
        assert_eq!(list(authority.pool(), &company).await.unwrap()[0].id, current.id);
        let replaced = authority
            .decide_email_mandate_proposal(&company, "owner", change, true, None)
            .await
            .unwrap()
            .unwrap();
        let listed = list(authority.pool(), &company).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, replaced.id);
        assert_eq!(listed[0].max_per_day, 10);
    }

    /// A reply is not new outreach: after a send, the same recipient is refused until they
    /// write back, and each message they send allows one answer in that thread.
    #[tokio::test]
    async fn a_correspondent_who_writes_back_can_be_answered_once_per_message() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping mandate reply scenario");
            return;
        };
        let authority = AuthorityStore::connect(&database_url).await.unwrap();
        let company = format!("mandate_reply_{}", Uuid::new_v4().simple());
        let proposed = authority
            .propose_email_mandate(&company, "exec", proposal(10), None, None)
            .await
            .unwrap();
        let mandate = authority
            .decide_email_mandate_proposal(&company, "owner", proposed, true, None)
            .await
            .unwrap()
            .unwrap();
        let send = |n: u32| {
            let authority = &authority;
            let company = &company;
            async move {
                let digest = format!("{n:064x}");
                let key = format!("reply-{n}");
                let permit = authority
                    .issue_email_permit(
                        company,
                        "exec",
                        mandate.id,
                        EmailPermitProposal {
                            sender: "hello@example.com".into(),
                            recipient: "customer@example.com".into(),
                            payload_sha256: digest.clone(),
                            effect_key: key.clone(),
                            rationale: "They asked a question.".into(),
                            evidence_refs: vec!["inbound".into()],
                            expires_at: Utc::now() + chrono::Duration::minutes(4),
                        },
                    )
                    .await?;
                authority
                    .reserve_email_send(
                        company,
                        permit.id,
                        "hello@example.com",
                        None,
                        "customer@example.com",
                        &digest,
                        &key,
                    )
                    .await?;
                authority
                    .record_email_send_status(
                        company,
                        permit.id,
                        &key,
                        ProviderOutcome::ConfirmedSent,
                        Some(&format!("re_{n}")),
                        None,
                    )
                    .await
            }
        };
        let inbound = |n: u32| {
            let authority = &authority;
            let company = &company;
            async move {
                authority
                    .emit_inbound_once_with_id(
                        company,
                        serde_json::json!({
                            "provider_event_id": format!("evt-{n}"),
                            "type": "email.received",
                            "party": "Customer <Customer@example.com>",
                        }),
                    )
                    .await
                    .unwrap();
            }
        };
        send(1).await.unwrap();
        assert!(send(2).await.is_err(), "a second email before they write back is outreach");
        inbound(1).await;
        send(3).await.unwrap();
        assert!(send(4).await.is_err(), "one answer per message they sent");
        inbound(2).await;
        send(5).await.unwrap();
    }
}
