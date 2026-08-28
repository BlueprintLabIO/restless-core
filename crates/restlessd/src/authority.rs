//! Narrow durable truth for the Authority Plane.
//!
//! This is deliberately not another service, public API, command algebra, or
//! workflow engine. It is the daemon's private governance store: approvals,
//! effect receipts, replay suppression and the small amount of evidence needed
//! to explain those decisions. OrgIntel remains recoverable coordination state
//! and may disappear without taking this truth with it.

use std::path::Path;

use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Utc};
use restless_orgintel::OrgIntel;
use sha2::{Digest as _, Sha256};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row as _};

pub const GOVERNANCE_KINDS: &[&str] = &[
    "effect_intent",
    "effect",
    "inbound_effect",
    "effect_replayed",
    "effect_reconciled",
    "effect_repeat_party",
    "approval_required",
    "approval_granted",
    "approval_declined",
    "approval_revoked",
    "lifecycle",
    "mandate_revision",
];

const IMPORT_VERSION: i32 = 2;
const MAX_MANDATE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuthorityRecord {
    pub id: i64,
    pub actor_id: Option<String>,
    pub body: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ModelCooldown {
    pub model: String,
    pub kind: String,
    pub reason: String,
    pub retry_at: DateTime<Utc>,
}

/// The result of reserving a governed customer-contact send under its
/// owner-configured per-recipient ceiling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectIntentClaim {
    Claimed,
    AlreadyClaimed,
    PartyCapReached { maximum: u16, occupied: i64 },
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct MandateRevisionOutcome {
    message: String,
    runtime_projection: MandateProjectionOutcome,
    evidence_status: &'static str,
}

#[derive(Debug, serde::Serialize)]
struct MandateProjectionOutcome {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

/// One installation-wide pool; company is an indexed value, not a schema.
/// Authority owns very little data and does not inherit OrgIntel's
/// per-company recoverable schema lifecycle.
#[derive(Clone)]
pub struct AuthorityStore {
    pool: PgPool,
}

impl AuthorityStore {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(4)
            .connect(database_url)
            .await
            .context("connect Authority store")?;
        // Fixed identifiers only. No company or request data is interpolated.
        sqlx::query("CREATE SCHEMA IF NOT EXISTS restless_authority")
            .execute(&pool)
            .await
            .context("create Authority schema")?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS restless_authority.records (\
               id BIGSERIAL PRIMARY KEY, \
               company TEXT NOT NULL, \
               kind TEXT NOT NULL, \
               actor_id TEXT, \
               body JSONB NOT NULL, \
               created_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
               legacy_orgintel_event_id BIGINT, \
               UNIQUE (company, legacy_orgintel_event_id)\
             )",
        )
        .execute(&pool)
        .await
        .context("create Authority records")?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS authority_records_company_kind_id \
             ON restless_authority.records (company, kind, id)",
        )
        .execute(&pool)
        .await
        .context("index Authority records")?;
        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS authority_effect_execution_intent \
             ON restless_authority.records \
             (company, (body->>'idempotency_key'), ((body->>'execution_no')::integer)) \
             WHERE kind = 'effect_intent'",
        )
        .execute(&pool)
        .await
        .context("index effect execution intents")?;
        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS authority_effect_execution_receipt \
             ON restless_authority.records \
             (company, (body->>'idempotency_key'), ((body->>'execution_no')::integer)) \
             WHERE kind = 'effect' AND body ? 'execution_no'",
        )
        .execute(&pool)
        .await
        .context("index effect execution receipts")?;
        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS authority_inbound_provider_event \
             ON restless_authority.records (company, (body->>'provider_event_id')) \
             WHERE kind = 'inbound_effect'",
        )
        .execute(&pool)
        .await
        .context("index inbound provider events")?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS restless_authority.company_migrations (\
               company TEXT PRIMARY KEY, \
               version INTEGER NOT NULL DEFAULT 1, \
               imported_at TIMESTAMPTZ NOT NULL DEFAULT now()\
             )",
        )
        .execute(&pool)
        .await
        .context("create Authority migration markers")?;
        sqlx::query(
            "ALTER TABLE restless_authority.company_migrations \
             ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1",
        )
        .execute(&pool)
        .await
        .context("version Authority migration markers")?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS restless_authority.model_cooldowns (\
               company TEXT NOT NULL, model TEXT NOT NULL, kind TEXT NOT NULL, reason TEXT NOT NULL, \
               retry_at TIMESTAMPTZ NOT NULL, updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
               PRIMARY KEY (company, model)\
             )",
        )
        .execute(&pool)
        .await
        .context("create model cooldowns")?;
        crate::legal::ensure_schema(&pool).await?;
        crate::finance::ensure_schema(&pool).await?;
        crate::airwallex::ensure_schema(&pool).await?;
        Ok(Self { pool })
    }

    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn set_model_cooldown(
        &self,
        company: &str,
        model: &str,
        kind: &str,
        reason: &str,
        retry_at: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO restless_authority.model_cooldowns \
             (company, model, kind, reason, retry_at) VALUES ($1,$2,$3,$4,$5) \
             ON CONFLICT (company, model) DO UPDATE SET kind=EXCLUDED.kind, \
             reason=EXCLUDED.reason, retry_at=EXCLUDED.retry_at, updated_at=now()",
        )
        .bind(company)
        .bind(model)
        .bind(kind)
        .bind(reason)
        .bind(retry_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn active_model_cooldowns(&self, company: &str) -> Result<Vec<ModelCooldown>> {
        Ok(sqlx::query_as(
            "SELECT model, kind, reason, retry_at FROM restless_authority.model_cooldowns \
             WHERE company=$1 AND retry_at > now() ORDER BY retry_at, model",
        )
        .bind(company)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn clear_model_cooldown(&self, company: &str, model: &str) -> Result<()> {
        sqlx::query("DELETE FROM restless_authority.model_cooldowns WHERE company=$1 AND model=$2")
            .bind(company)
            .bind(model)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn emit(
        &self,
        company: &str,
        kind: &str,
        actor: Option<&str>,
        body: serde_json::Value,
    ) -> Result<i64> {
        let id = sqlx::query_scalar(
            "INSERT INTO restless_authority.records (company, kind, actor_id, body) \
             VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(company)
        .bind(kind)
        .bind(actor)
        .bind(body)
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("record Authority {kind} for {company}"))?;
        Ok(id)
    }

    /// Atomically reserve one execution number for a generic material effect.
    /// Two daemon requests may race; only the row that lands may start the
    /// child process.
    pub async fn claim_effect_intent(
        &self,
        company: &str,
        actor: &str,
        body: serde_json::Value,
    ) -> Result<bool> {
        let inserted = sqlx::query_scalar::<_, i64>(
            "INSERT INTO restless_authority.records (company, kind, actor_id, body) \
             VALUES ($1, 'effect_intent', $2, $3) \
             ON CONFLICT DO NOTHING RETURNING id",
        )
        .bind(company)
        .bind(actor)
        .bind(body)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("claim effect execution for {company}"))?;
        Ok(inserted.is_some())
    }

    /// Atomically reserve a `customer-contact.email` execution while applying
    /// the owner's per-recipient limit. Confirmed sends and intents without a
    /// matching receipt both occupy a slot: an interrupted send is not proof
    /// that no email reached the recipient.
    pub async fn claim_customer_contact_email_intent(
        &self,
        company: &str,
        actor: &str,
        body: serde_json::Value,
        party: &str,
        maximum: u16,
    ) -> Result<EffectIntentClaim> {
        let mut tx = self
            .pool
            .begin()
            .await
            .with_context(|| format!("begin customer-contact cap reservation for {company}"))?;
        // The existing uniqueness index makes one idempotency execution safe.
        // This transaction lock additionally serialises *different* keys for
        // the same company/recipient, which is what makes the cap real.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1::text), hashtext($2::text))")
            .bind(company)
            .bind(party)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("lock customer-contact cap for {company}/{party}"))?;

        let occupied = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)::BIGINT FROM ( \
               SELECT 1 \
               FROM restless_authority.records AS receipt \
               WHERE receipt.company = $1 \
                 AND receipt.kind = 'effect' \
                 AND receipt.body->>'effect_class' = 'customer-contact.email' \
                 AND receipt.body->>'party' = $2 \
                 AND receipt.body->>'success' = 'true' \
               UNION ALL \
               SELECT 1 \
               FROM restless_authority.records AS intent \
               WHERE intent.company = $1 \
                 AND intent.kind = 'effect_intent' \
                 AND intent.body->>'effect_class' = 'customer-contact.email' \
                 AND intent.body->>'party' = $2 \
                 AND NOT EXISTS ( \
                   SELECT 1 \
                   FROM restless_authority.records AS receipt \
                   WHERE receipt.company = intent.company \
                     AND receipt.kind = 'effect' \
                     AND receipt.body->>'idempotency_key' = intent.body->>'idempotency_key' \
                     AND COALESCE(receipt.body->>'execution_no', '1') = COALESCE(intent.body->>'execution_no', '1') \
                 ) \
             ) AS occupied",
        )
        .bind(company)
        .bind(party)
        .fetch_one(&mut *tx)
        .await
        .with_context(|| format!("count customer-contact sends for {company}/{party}"))?;
        if occupied >= i64::from(maximum) {
            tx.commit().await?;
            return Ok(EffectIntentClaim::PartyCapReached { maximum, occupied });
        }

        let inserted = sqlx::query_scalar::<_, i64>(
            "INSERT INTO restless_authority.records (company, kind, actor_id, body) \
             VALUES ($1, 'effect_intent', $2, $3) \
             ON CONFLICT DO NOTHING RETURNING id",
        )
        .bind(company)
        .bind(actor)
        .bind(body)
        .fetch_optional(&mut *tx)
        .await
        .with_context(|| format!("claim customer-contact execution for {company}"))?;
        tx.commit().await?;
        Ok(if inserted.is_some() {
            EffectIntentClaim::Claimed
        } else {
            EffectIntentClaim::AlreadyClaimed
        })
    }

    pub async fn records_of_kind(&self, company: &str, kind: &str) -> Result<Vec<AuthorityRecord>> {
        sqlx::query_as(
            "SELECT id, actor_id, body, created_at \
             FROM restless_authority.records \
             WHERE company = $1 AND kind = $2 ORDER BY id",
        )
        .bind(company)
        .bind(kind)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("read Authority {kind} records for {company}"))
    }

    pub async fn find_body(
        &self,
        company: &str,
        kind: &str,
        json_field: &str,
        value: &str,
    ) -> Result<Option<serde_json::Value>> {
        let row = sqlx::query(
            "SELECT body FROM restless_authority.records \
             WHERE company = $1 AND kind = $2 AND body->>$3 = $4 \
             ORDER BY id DESC LIMIT 1",
        )
        .bind(company)
        .bind(kind)
        .bind(json_field)
        .bind(value)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("find Authority {kind} record for {company}"))?;
        Ok(row.map(|row| row.get(0)))
    }

    /// Atomically record one provider event. Webhook deliveries can race; a
    /// read-then-insert dedupe would let both through. The partial unique index
    /// makes the provider's event id the deciding fact without imposing that
    /// shape on any other Authority record.
    pub async fn emit_inbound_once(&self, company: &str, body: serde_json::Value) -> Result<bool> {
        Ok(self.emit_inbound_once_with_id(company, body).await?.1)
    }

    /// Record or recover one authoritative inbound record id. The id is the
    /// stable cross-layer source reference; OrgIntel never owns provider
    /// delivery truth, and a redelivery can use this id to repair a lost
    /// projection without creating a second Authority fact.
    pub async fn emit_inbound_once_with_id(
        &self,
        company: &str,
        body: serde_json::Value,
    ) -> Result<(i64, bool)> {
        let provider_event_id = body
            .get("provider_event_id")
            .and_then(serde_json::Value::as_str)
            .context("inbound Authority body needs provider_event_id")?
            .to_string();
        let inserted = sqlx::query_scalar::<_, i64>(
            "INSERT INTO restless_authority.records (company, kind, actor_id, body) \
             VALUES ($1, 'inbound_effect', 'world', $2) \
             ON CONFLICT DO NOTHING RETURNING id",
        )
        .bind(company)
        .bind(body)
        .fetch_optional(&self.pool)
        .await
        .with_context(|| format!("record inbound Authority effect for {company}"))?;
        if let Some(id) = inserted {
            return Ok((id, true));
        }
        let id = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM restless_authority.records \
             WHERE company=$1 AND kind='inbound_effect' AND body->>'provider_event_id'=$2",
        )
        .bind(company)
        .bind(provider_event_id)
        .fetch_one(&self.pool)
        .await
        .with_context(|| format!("recover inbound Authority record for {company}"))?;
        Ok((id, false))
    }

    pub async fn inbound_companies(&self) -> Result<Vec<String>> {
        Ok(sqlx::query_scalar(
            "SELECT DISTINCT company FROM restless_authority.records \
             WHERE kind='inbound_effect' ORDER BY company",
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn inbound_after(
        &self,
        company: &str,
        after_id: i64,
        limit: i64,
    ) -> Result<Vec<AuthorityRecord>> {
        Ok(sqlx::query_as(
            "SELECT id,actor_id,body,created_at FROM restless_authority.records \
             WHERE company=$1 AND kind='inbound_effect' AND id>$2 ORDER BY id LIMIT $3",
        )
        .bind(company)
        .bind(after_id.max(0))
        .bind(limit.clamp(1, 1000))
        .fetch_all(&self.pool)
        .await?)
    }

    /// Import the governance events written before the Authority store
    /// existed. The marker and imported rows commit together; a crash can only
    /// cause a safe retry, and the legacy event id makes that retry idempotent.
    /// Config approvals are migration input, never a live second writer.
    pub async fn import_legacy_company(
        &self,
        company: &str,
        org: &OrgIntel,
        config_approvals: &[String],
    ) -> Result<usize> {
        let imported_version = sqlx::query_scalar::<_, Option<i32>>(
            "SELECT version FROM restless_authority.company_migrations WHERE company = $1",
        )
        .bind(company)
        .fetch_optional(&self.pool)
        .await?;
        if imported_version
            .flatten()
            .is_some_and(|version| version >= IMPORT_VERSION)
        {
            return Ok(0);
        }

        let mut legacy = Vec::new();
        for kind in GOVERNANCE_KINDS {
            legacy.extend(org.events_of_kind(kind).await.with_context(|| {
                format!("read legacy {kind} events for Authority migration of {company}")
            })?);
        }
        legacy.sort_by_key(|event| event.id);

        let mut tx = self.pool.begin().await?;
        let mut imported = 0;
        for event in legacy {
            let result = sqlx::query(
                "INSERT INTO restless_authority.records \
                 (company, kind, actor_id, body, created_at, legacy_orgintel_event_id) \
                 VALUES ($1, $2, $3, $4, $5, $6) \
                 ON CONFLICT (company, legacy_orgintel_event_id) DO NOTHING",
            )
            .bind(company)
            .bind(event.kind)
            .bind(event.actor_id)
            .bind(event.body)
            .bind(event.created_at)
            .bind(event.id)
            .execute(&mut *tx)
            .await?;
            imported += result.rows_affected() as usize;
        }

        // An old config approval may predate approval events. Give it an
        // explicit provenance so it cannot silently become constitutional
        // truth merely because a TOML entry happened to exist.
        for party in config_approvals {
            let party = party.trim().to_lowercase();
            if party.is_empty() {
                continue;
            }
            let exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS (SELECT 1 FROM restless_authority.records \
                 WHERE company = $1 AND kind = 'approval_granted' \
                   AND lower(body->>'party') = $2)",
            )
            .bind(company)
            .bind(&party)
            .fetch_one(&mut *tx)
            .await?;
            if !exists {
                sqlx::query(
                    "INSERT INTO restless_authority.records (company, kind, actor_id, body) \
                     VALUES ($1, 'approval_granted', 'owner', $2)",
                )
                .bind(company)
                .bind(serde_json::json!({
                    "party": party,
                    "principal": "owner",
                    "source": "legacy_company_config"
                }))
                .execute(&mut *tx)
                .await?;
                imported += 1;
            }
        }

        sqlx::query(
            "INSERT INTO restless_authority.company_migrations (company, version) VALUES ($1, $2) \
             ON CONFLICT (company) DO UPDATE SET version = EXCLUDED.version, imported_at = now()",
        )
        .bind(company)
        .bind(IMPORT_VERSION)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(imported)
    }

    /// Initialise a genuinely new company with no OrgIntel history.
    pub async fn initialise_company(
        &self,
        company: &str,
        config_approvals: &[String],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for party in config_approvals {
            let party = party.trim().to_lowercase();
            if party.is_empty() {
                continue;
            }
            sqlx::query(
                "INSERT INTO restless_authority.records (company, kind, actor_id, body) \
                 VALUES ($1, 'approval_granted', 'owner', $2)",
            )
            .bind(company)
            .bind(serde_json::json!({
                "party": party,
                "principal": "owner",
                "source": "initial_company_config"
            }))
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "INSERT INTO restless_authority.company_migrations (company, version) VALUES ($1, $2) \
             ON CONFLICT (company) DO UPDATE SET version = EXCLUDED.version, imported_at = now()",
        )
        .bind(company)
        .bind(IMPORT_VERSION)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Test-company teardown only. The daemon enforces `_test` before this is
    /// reachable; production governance truth has no convenience delete.
    pub async fn delete_test_company(&self, company: &str) -> Result<()> {
        if !crate::runtime::is_test_company(company) {
            anyhow::bail!("refusing to delete Authority records for non-test company {company}");
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM restless_authority.payment_intents WHERE company = $1")
            .bind(company)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM restless_authority.money_envelopes WHERE company = $1")
            .bind(company)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM restless_authority.legal_profiles WHERE company = $1")
            .bind(company)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM restless_authority.airwallex_connections WHERE company = $1")
            .bind(company)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM restless_authority.records WHERE company = $1")
            .bind(company)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM restless_authority.company_migrations WHERE company = $1")
            .bind(company)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

pub(crate) fn mandate_revision(markdown: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(markdown.as_bytes()))
}

pub(crate) fn validate_mandate(markdown: &str) -> Result<()> {
    if markdown.trim().is_empty() {
        bail!("charter must contain at least one non-whitespace character");
    }
    if markdown.len() > MAX_MANDATE_BYTES {
        bail!("charter must be at most {MAX_MANDATE_BYTES} UTF-8 bytes");
    }
    if markdown.contains('\0') {
        bail!("charter cannot contain a NUL character");
    }
    Ok(())
}

/// The one source-owned owner mandate mutation. CompanyConfig remains the
/// canonical host file; Authority evidence brackets the atomic replacement,
/// and the Runtime receives only a read-only projection.
pub(crate) async fn revise_mandate(
    authority: &AuthorityStore,
    root: &Path,
    mut config: crate::runtime::CompanyConfig,
    markdown: String,
) -> Result<MandateRevisionOutcome> {
    validate_mandate(&markdown)?;
    let previous_markdown = config.mission.clone();
    let previous_revision = mandate_revision(&previous_markdown);
    let revision = mandate_revision(&markdown);
    if revision == previous_revision {
        return Ok(MandateRevisionOutcome {
            message: "Charter is already current.".into(),
            runtime_projection: MandateProjectionOutcome {
                status: "unchanged",
                detail: None,
            },
            evidence_status: "unchanged",
        });
    }

    let requested_at = Utc::now();
    let request_record_id = authority
        .emit(
            &config.name,
            "mandate_revision",
            Some("owner"),
            serde_json::json!({
                "state": "requested",
                "previous_revision": previous_revision,
                "revision": revision,
                "previous_markdown": previous_markdown,
                "markdown": markdown,
                "requested_at": requested_at,
            }),
        )
        .await
        .context("record owner charter revision before changing the canonical mandate")?;

    config.mission = markdown;
    if let Err(error) = crate::runtime::CompanyConfig::save(root, &config) {
        authority
            .emit(
                &config.name,
                "mandate_revision",
                Some("owner"),
                serde_json::json!({
                    "state": "failed",
                    "request_record_id": request_record_id,
                    "previous_revision": previous_revision,
                    "revision": revision,
                    "requested_at": requested_at,
                    "observed_at": Utc::now(),
                    "error": format!("{error:#}"),
                }),
            )
            .await
            .context(
                "charter save failed and its Authority failure evidence could not be recorded",
            )?;
        return Err(error).context("save canonical owner charter");
    }

    let runtime_projection = match crate::runtime::sync_mission_projection(&config).await {
        Ok(status) => MandateProjectionOutcome {
            status,
            detail: None,
        },
        Err(error) => MandateProjectionOutcome {
            status: "failed",
            detail: Some(format!("{error:#}")),
        },
    };
    let evidence_status = match authority
        .emit(
            &config.name,
            "mandate_revision",
            Some("owner"),
            serde_json::json!({
                "state": "succeeded",
                "request_record_id": request_record_id,
                "previous_revision": previous_revision,
                "revision": revision,
                "requested_at": requested_at,
                "observed_at": Utc::now(),
                "runtime_projection": runtime_projection.status,
                "runtime_projection_detail": runtime_projection.detail.as_deref(),
            }),
        )
        .await
    {
        Ok(_) => "recorded",
        Err(error) => {
            tracing::error!(
                company = config.name,
                %error,
                "canonical charter changed but final Authority revision evidence is incomplete"
            );
            "incomplete"
        }
    };

    let message = match runtime_projection.status {
        "updated" => "Charter saved and the Company computer projection was refreshed.",
        "deferred" => "Charter saved. The Company computer will receive it when next started.",
        "failed" => "Charter saved, but the Company computer projection could not be refreshed.",
        _ => "Charter saved.",
    };
    Ok(MandateRevisionOutcome {
        message: message.into(),
        runtime_projection,
        evidence_status,
    })
}

#[cfg(test)]
mod mandate_tests {
    use super::*;

    #[test]
    fn revision_tracks_exact_owner_text_and_validation_is_bounded() {
        let original = "# Company\n\nDo useful work.\n";
        assert_eq!(mandate_revision(original), mandate_revision(original));
        assert_ne!(
            mandate_revision(original),
            mandate_revision(original.trim_end())
        );
        assert_ne!(
            mandate_revision(original),
            mandate_revision("# Company\n\nDo useful work!\n")
        );
        assert!(validate_mandate(original).is_ok());
        assert!(validate_mandate("  \n").is_err());
        assert!(validate_mandate("valid\0invalid").is_err());
        assert!(validate_mandate(&"a".repeat(MAX_MANDATE_BYTES + 1)).is_err());
    }

    #[tokio::test]
    async fn customer_contact_cap_counts_confirmed_and_unresolved_sends() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping customer-contact cap scenario");
            return;
        };
        let company = format!(
            "customer_contact_cap_{}_test",
            uuid::Uuid::new_v4().simple()
        );
        let party = "centre@example.test";
        let store = AuthorityStore::connect(&database_url).await.unwrap();
        store.delete_test_company(&company).await.unwrap();

        let intent = |key: &str| {
            serde_json::json!({
                "idempotency_key": key,
                "execution_no": 1,
                "effect_class": "customer-contact.email",
                "party": party,
            })
        };
        assert_eq!(
            store
                .claim_customer_contact_email_intent(&company, "tester", intent("first"), party, 3,)
                .await
                .unwrap(),
            EffectIntentClaim::Claimed
        );
        store
            .emit(
                &company,
                "effect",
                Some("tester"),
                serde_json::json!({
                    "idempotency_key": "first",
                    "execution_no": 1,
                    "effect_class": "customer-contact.email",
                    "party": party,
                    "success": true,
                }),
            )
            .await
            .unwrap();
        for key in ["second", "third"] {
            assert_eq!(
                store
                    .claim_customer_contact_email_intent(&company, "tester", intent(key), party, 3)
                    .await
                    .unwrap(),
                EffectIntentClaim::Claimed
            );
        }
        assert_eq!(
            store
                .claim_customer_contact_email_intent(
                    &company,
                    "tester",
                    intent("fourth"),
                    party,
                    3,
                )
                .await
                .unwrap(),
            EffectIntentClaim::PartyCapReached {
                maximum: 3,
                occupied: 3,
            }
        );
        store.delete_test_company(&company).await.unwrap();
    }
}
