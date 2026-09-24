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

fn validate_digest(value: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("payload digest must be 64 hexadecimal SHA256 characters");
    }
    Ok(())
}

async fn lock_company(tx: &mut Transaction<'_, Postgres>, company: &str) -> Result<()> {
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

pub(super) async fn grant(
    pool: &PgPool,
    company: &str,
    owner_actor_id: &str,
    mandate: NewEmailMandate,
) -> Result<EmailMandate> {
    if owner_actor_id.trim().is_empty() {
        bail!("owner attribution is required to grant an email mandate");
    }
    if mandate.purpose.trim().is_empty() || mandate.audience_guidance.trim().is_empty() {
        bail!("email mandate needs a purpose and audience guidance");
    }
    if mandate.purpose.trim().len() > 2_000 || mandate.audience_guidance.trim().len() > 8_000 {
        bail!("email mandate purpose or audience guidance exceeds its size limit");
    }
    let sender = normalize_mailbox(&mandate.sender)?;
    if mandate.max_per_day == 0 || mandate.max_total == 0 || mandate.max_per_day > mandate.max_total
    {
        bail!("email mandate limits must be positive and daily limit cannot exceed total limit");
    }
    Tz::from_str(&mandate.timezone).context("timezone must be a valid IANA timezone")?;
    let now = Utc::now();
    if mandate.expires_at <= now {
        bail!("email mandate must expire in the future");
    }
    let mandate = EmailMandate {
        id: Uuid::new_v4(),
        purpose: mandate.purpose.trim().to_owned(),
        audience_guidance: mandate.audience_guidance.trim().to_owned(),
        sender,
        max_per_day: mandate.max_per_day,
        max_total: mandate.max_total,
        timezone: mandate.timezone,
        expires_at: mandate.expires_at,
        created_at: now,
    };
    let mut tx = pool.begin().await?;
    lock_company(&mut tx, company).await?;
    let grants = sqlx::query(
        "SELECT id,body FROM restless_authority.records \
         WHERE company=$1 AND kind='email_mandate_granted' ORDER BY id DESC",
    )
    .bind(company)
    .fetch_all(&mut *tx)
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
        .fetch_one(&mut *tx)
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
    .fetch_one(&mut *tx)
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
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(mandate)
}

pub(super) async fn list(pool: &PgPool, company: &str) -> Result<Vec<EmailMandate>> {
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

pub(super) async fn revoke(
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
    let (_, version) = current_root(&mut tx, company, mandate_id).await?;
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
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn issue(
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

pub(super) async fn list_permits(
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

pub(super) async fn usage(
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

pub(super) async fn reserve(
    pool: &PgPool,
    company: &str,
    permit_id: Uuid,
    actual_sender: &str,
    recipient: &str,
    payload_sha256: &str,
    effect_key: &str,
) -> Result<EmailReservation> {
    let actual_sender = normalize_mailbox(actual_sender)?;
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
    let prior_recipient_reservations = sqlx::query(
        "SELECT body FROM restless_authority.records \
         WHERE company=$1 AND kind='email_send_reserved' AND lower(body->>'recipient')=$2",
    )
    .bind(company)
    .bind(recipient.as_str())
    .fetch_all(&mut *tx)
    .await?;
    for prior in prior_recipient_reservations {
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
        if last_outcome.as_deref() != Some("confirmed_not_sent") {
            bail!(
                "recipient already has a sent or uncertain outbound email under an email mandate"
            );
        }
    }
    for receipt in sqlx::query(
        "SELECT body FROM restless_authority.records WHERE company=$1 AND kind='effect'",
    )
    .bind(company)
    .fetch_all(&mut *tx)
    .await?
    {
        let body: Value = receipt.try_get("body")?;
        if body.get("effect_class").and_then(Value::as_str) == Some("customer-contact.email")
            && body
                .get("party")
                .and_then(Value::as_str)
                .is_some_and(|party| party.eq_ignore_ascii_case(&recipient))
            && body.get("success").and_then(Value::as_bool) == Some(true)
        {
            bail!("recipient already has a successful customer-contact.email receipt");
        }
    }
    let prior_generic_intent: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM restless_authority.records \
         WHERE company=$1 AND kind='effect_intent' \
           AND body->>'effect_class'='customer-contact.email' \
           AND lower(body->>'party')=$2)",
    )
    .bind(company)
    .bind(recipient.as_str())
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

pub(super) async fn record_status(
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
