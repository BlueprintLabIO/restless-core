//! Narrow durable truth for the Authority Plane.
//!
//! This is deliberately not another service, public API, command algebra, or
//! workflow engine. It is the daemon's private governance store: approvals,
//! effect receipts, replay suppression and the small amount of evidence needed
//! to explain those decisions. OrgIntel remains recoverable coordination state
//! and may disappear without taking this truth with it.

use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use restless_orgintel::OrgIntel;
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
];

const IMPORT_VERSION: i32 = 2;

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
        Ok(Self { pool })
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
        Ok(inserted.is_some())
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
