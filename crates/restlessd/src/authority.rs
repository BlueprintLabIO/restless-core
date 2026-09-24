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
use sqlx::{PgPool, Postgres, Row as _, Transaction};
use uuid::Uuid;

#[path = "mandate.rs"]
mod mandate;

pub use mandate::{
    EmailMandate, EmailMandateUsage, EmailPermit, EmailPermitProposal, EmailReservation,
    NewEmailMandate, ProviderOutcome,
};

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
    "company_identity_decision",
    "provider_connection_requested",
    "provider_connection_enabled",
    "provider_connection_observed",
    "provider_connection_disabled",
    "publication_request",
    "publication_authorized",
    "publication_resource_grant",
    "publication_ready",
    "publication_recovered",
    "publication_failed",
    "publication_observation",
    "publication_invitation",
    "publication_invitation_revoked",
    "publication_stopped",
    "publication_cleanup",
    "email_mandate_granted",
    "email_mandate_proposal",
    "email_mandate_proposal_decision",
    "email_mandate_revoked",
    "email_permit_issued",
    "email_send_reserved",
    "email_send_status",
    "email_observation",
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
        // Several daemon/test callers may open the one installation-wide
        // Authority store concurrently. PostgreSQL's `IF NOT EXISTS` DDL is
        // not race-free: concurrent schema/type creation can still collide in
        // system catalogues. A transaction-scoped advisory lock serializes
        // only this short, idempotent bootstrap and releases automatically on
        // every error path.
        let mut bootstrap = pool.begin().await.context("begin Authority bootstrap")?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(0x5253_544c_4155_5448_i64)
            .execute(&mut *bootstrap)
            .await
            .context("serialize Authority bootstrap")?;
        // Fixed identifiers only. No company or request data is interpolated.
        // Runs on the locked transaction, not the bare pool -- an unlocked
        // connection here would race every other caller's unlocked DDL.
        sqlx::query("CREATE SCHEMA IF NOT EXISTS restless_authority")
            .execute(&mut *bootstrap)
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
        .execute(&mut *bootstrap)
        .await
        .context("create Authority records")?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS authority_records_company_kind_id \
             ON restless_authority.records (company, kind, id)",
        )
        .execute(&mut *bootstrap)
        .await
        .context("index Authority records")?;
        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS authority_effect_execution_intent \
             ON restless_authority.records \
             (company, (body->>'idempotency_key'), ((body->>'execution_no')::integer)) \
             WHERE kind = 'effect_intent'",
        )
        .execute(&mut *bootstrap)
        .await
        .context("index effect execution intents")?;
        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS authority_effect_execution_receipt \
             ON restless_authority.records \
             (company, (body->>'idempotency_key'), ((body->>'execution_no')::integer)) \
             WHERE kind = 'effect' AND body ? 'execution_no'",
        )
        .execute(&mut *bootstrap)
        .await
        .context("index effect execution receipts")?;
        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS authority_inbound_provider_event \
             ON restless_authority.records (company, (body->>'provider_event_id')) \
             WHERE kind = 'inbound_effect'",
        )
        .execute(&mut *bootstrap)
        .await
        .context("index inbound provider events")?;
        for (name, sql) in [
            (
                "publication request idempotency",
                "CREATE UNIQUE INDEX IF NOT EXISTS authority_publication_request_key \
                 ON restless_authority.records (company, (body->>'idempotency_key')) \
                 WHERE kind = 'publication_request'",
            ),
            (
                "publication authorization",
                "CREATE UNIQUE INDEX IF NOT EXISTS authority_publication_authorized_once \
                 ON restless_authority.records (company, (body->>'publication_id')) \
                 WHERE kind = 'publication_authorized'",
            ),
            (
                "publication resource grant",
                "CREATE UNIQUE INDEX IF NOT EXISTS authority_publication_resource_grant_once \
                 ON restless_authority.records (company, (body->>'publication_id')) \
                 WHERE kind = 'publication_resource_grant'",
            ),
            (
                "publication ready receipt",
                "CREATE UNIQUE INDEX IF NOT EXISTS authority_publication_ready_once \
                 ON restless_authority.records (company, (body->>'publication_id')) \
                 WHERE kind = 'publication_ready'",
            ),
            (
                "publication provider failure",
                "CREATE UNIQUE INDEX IF NOT EXISTS authority_publication_failure_key \
                 ON restless_authority.records (company, (body->>'failure_key')) \
                 WHERE kind = 'publication_failed'",
            ),
            (
                "publication invitation",
                "CREATE UNIQUE INDEX IF NOT EXISTS authority_publication_invitation_id \
                 ON restless_authority.records (company, (body->>'invitation_id')) \
                 WHERE kind = 'publication_invitation'",
            ),
            (
                "publication invitation revocation",
                "CREATE UNIQUE INDEX IF NOT EXISTS authority_publication_invitation_revoke_once \
                 ON restless_authority.records (company, (body->>'invitation_id')) \
                 WHERE kind = 'publication_invitation_revoked'",
            ),
            (
                "publication stop",
                "CREATE UNIQUE INDEX IF NOT EXISTS authority_publication_stop_once \
                 ON restless_authority.records (company, (body->>'publication_id')) \
                 WHERE kind = 'publication_stopped'",
            ),
            (
                "publication cleanup",
                "CREATE UNIQUE INDEX IF NOT EXISTS authority_publication_cleanup_once \
                 ON restless_authority.records (company, (body->>'publication_id')) \
                 WHERE kind = 'publication_cleanup'",
            ),
            (
                "email permit reservation once",
                "CREATE UNIQUE INDEX IF NOT EXISTS authority_email_permit_reservation_once \
                 ON restless_authority.records (company, (body->>'permit_id')) \
                 WHERE kind = 'email_send_reserved'",
            ),
            (
                "email mandate proposal decision once",
                "CREATE UNIQUE INDEX IF NOT EXISTS authority_email_mandate_proposal_decision_once \
                 ON restless_authority.records (company, (body->>'proposal_id')) \
                 WHERE kind = 'email_mandate_proposal_decision'",
            ),
            (
                "email effect key reservation once",
                "CREATE UNIQUE INDEX IF NOT EXISTS authority_email_effect_key_once \
                 ON restless_authority.records (company, (body->>'effect_key')) \
                 WHERE kind = 'email_send_reserved'",
            ),
        ] {
            sqlx::query(sql)
                .execute(&mut *bootstrap)
                .await
                .with_context(|| format!("index {name}"))?;
        }
        // Was a second transaction with its own advisory lock (7310026); merged
        // into the one above since a single caller always acquired both in order.
        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS authority_company_identity_decision \
             ON restless_authority.records \
             (company, kind, (body->>'proposal_id'), (body->>'decision')) \
             WHERE kind = 'company_identity_decision'",
        )
        .execute(&mut *bootstrap)
        .await
        .context("index company identity decisions")?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS restless_authority.company_migrations (\
               company TEXT PRIMARY KEY, \
               version INTEGER NOT NULL DEFAULT 1, \
               imported_at TIMESTAMPTZ NOT NULL DEFAULT now()\
             )",
        )
        .execute(&mut *bootstrap)
        .await
        .context("create Authority migration markers")?;
        sqlx::query(
            "ALTER TABLE restless_authority.company_migrations \
             ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1",
        )
        .execute(&mut *bootstrap)
        .await
        .context("version Authority migration markers")?;
        // Fleet's company-bootstrap operation is Authority truth, not an
        // OrgIntel event.  The reservation is committed before any cell or
        // filesystem provisioning starts, so a process crash leaves the exact
        // request fingerprint and company/cell custody available for retry.
        // The receipt bytes are retained verbatim: a lost HTTP response must
        // replay byte-for-byte rather than reconstructing a plausible answer.
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS restless_authority.company_bootstrap_operations (\
               operation_id UUID PRIMARY KEY, \
               request_fingerprint BYTEA NOT NULL CHECK (octet_length(request_fingerprint)=32), \
               owner_id UUID NOT NULL, \
               plane_id UUID NOT NULL, \
               plane_hostname TEXT NOT NULL CHECK (octet_length(plane_hostname) BETWEEN 1 AND 253), \
               plane_desired_revision BIGINT NOT NULL CHECK (plane_desired_revision > 0), \
               account_plane_image TEXT NOT NULL CHECK (octet_length(account_plane_image) BETWEEN 1 AND 512), \
               core_release TEXT NOT NULL CHECK (octet_length(core_release) BETWEEN 1 AND 64), \
               release_manifest_digest TEXT NOT NULL CHECK (octet_length(release_manifest_digest)=71), \
               company_id UUID NOT NULL UNIQUE, \
               cell_id UUID NOT NULL UNIQUE, \
               company_handle TEXT NOT NULL UNIQUE CHECK (octet_length(company_handle) BETWEEN 1 AND 63), \
               model TEXT NOT NULL CHECK (octet_length(model) BETWEEN 1 AND 160), \
               reasoning_effort TEXT NOT NULL CHECK (reasoning_effort IN ('none','low','medium','high','xhigh','max','ultra')), \
               config_fingerprint BYTEA NOT NULL CHECK (octet_length(config_fingerprint)=32), \
               status TEXT NOT NULL CHECK (status IN ('provisioning','ready')), \
               receipt_bytes BYTEA, \
               created_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
               ready_at TIMESTAMPTZ, \
               updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
               CHECK (operation_id <> '00000000-0000-0000-0000-000000000000'), \
               CHECK (owner_id <> '00000000-0000-0000-0000-000000000000'), \
               CHECK (plane_id <> '00000000-0000-0000-0000-000000000000'), \
               CHECK (company_id <> '00000000-0000-0000-0000-000000000000'), \
               CHECK (cell_id <> '00000000-0000-0000-0000-000000000000'), \
               CHECK ((status='provisioning' AND receipt_bytes IS NULL AND ready_at IS NULL) \
                   OR (status='ready' AND receipt_bytes IS NOT NULL AND ready_at IS NOT NULL))\
             )",
        )
        .execute(&mut *bootstrap)
        .await
        .context("create durable company-bootstrap operations")?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS company_bootstrap_plane_status \
             ON restless_authority.company_bootstrap_operations \
             (owner_id,plane_id,status,created_at)",
        )
        .execute(&mut *bootstrap)
        .await
        .context("index company-bootstrap operations")?;
        // Fleet's Runtime bootstrap advances one exact immutable generation
        // at a time. This survives Core restarts, so a stopped or compromised
        // old container cannot regain readiness by replaying a still-live
        // bridge capability after its replacement has been accepted.
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS restless_authority.runtime_bridge_generations (\
               cell_id UUID PRIMARY KEY, \
               owner_id UUID NOT NULL, \
               plane_id UUID NOT NULL, \
               company_id UUID NOT NULL UNIQUE, \
               company_handle TEXT NOT NULL, \
               runtime_id TEXT NOT NULL, \
               runtime_generation BIGINT NOT NULL CHECK (runtime_generation > 0), \
               runtime_image TEXT NOT NULL, \
               volume_name TEXT NOT NULL, \
               source_revision TEXT NOT NULL, \
               credential_operation_id UUID NOT NULL UNIQUE, \
               credential_id UUID NOT NULL, \
               credential_epoch BIGINT NOT NULL CHECK (credential_epoch > 0), \
               credential_expires_at TIMESTAMPTZ NOT NULL, \
               updated_at TIMESTAMPTZ NOT NULL DEFAULT now()\
             )",
        )
        .execute(&mut *bootstrap)
        .await
        .context("create durable Runtime-bridge generations")?;
        sqlx::query(
            "ALTER TABLE restless_authority.runtime_bridge_generations \
             DROP COLUMN IF EXISTS desired_revision",
        )
        .execute(&mut *bootstrap)
        .await
        .context("remove mutable desired revision from Runtime process identity")?;
        sqlx::query(
            "ALTER TABLE restless_authority.runtime_bridge_generations \
             ADD COLUMN IF NOT EXISTS credential_operation_id UUID NOT NULL DEFAULT gen_random_uuid(), \
             ADD COLUMN IF NOT EXISTS credential_id UUID NOT NULL DEFAULT gen_random_uuid(), \
             ADD COLUMN IF NOT EXISTS credential_epoch BIGINT NOT NULL DEFAULT 1 CHECK (credential_epoch > 0), \
             ADD COLUMN IF NOT EXISTS credential_expires_at TIMESTAMPTZ NOT NULL DEFAULT now()",
        )
        .execute(&mut *bootstrap)
        .await
        .context("add renewable Runtime-bridge credential lease")?;
        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS runtime_bridge_credential_operation \
             ON restless_authority.runtime_bridge_generations (credential_operation_id)",
        )
        .execute(&mut *bootstrap)
        .await
        .context("index Runtime-bridge credential refresh operations")?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS restless_authority.model_cooldowns (\
               company TEXT NOT NULL, model TEXT NOT NULL, kind TEXT NOT NULL, reason TEXT NOT NULL, \
               retry_at TIMESTAMPTZ NOT NULL, updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
               PRIMARY KEY (company, model)\
             )",
        )
        .execute(&mut *bootstrap)
        .await
        .context("create model cooldowns")?;
        // Commit before the other modules' ensure_schema(&pool) calls: they run
        // on a separate session that can't see these objects until it commits.
        bootstrap
            .commit()
            .await
            .context("complete Authority bootstrap")?;
        // Each ensure_schema below still runs its own unguarded `&pool` DDL, and
        // `CREATE TABLE IF NOT EXISTS` isn't race-free against pg_type catalogue
        // collisions -- serialize them with a session-level lock (no single
        // transaction spans these four independent calls to scope it to).
        let mut serializer = pool.acquire().await.context("acquire schema serializer")?;
        sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(0x5253_544c_4155_5448_i64)
            .execute(&mut *serializer)
            .await
            .context("serialize dependent-module Authority schema bootstrap")?;
        let ensure_result = async {
            crate::legal::ensure_schema(&pool).await?;
            crate::finance::ensure_schema(&pool).await?;
            crate::airwallex::ensure_schema(&pool).await?;
            crate::connected_tool::ensure_schema(&pool).await?;
            Ok::<_, anyhow::Error>(())
        }
        .await;
        sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(0x5253_544c_4155_5448_i64)
            .execute(&mut *serializer)
            .await
            .context("release dependent-module Authority schema bootstrap lock")?;
        ensure_result?;
        Ok(Self { pool })
    }

    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn grant_email_mandate(
        &self,
        company: &str,
        owner_actor_id: &str,
        mandate: NewEmailMandate,
    ) -> Result<EmailMandate> {
        mandate::grant(&self.pool, company, owner_actor_id, mandate).await
    }

    pub async fn propose_email_mandate(
        &self,
        company: &str,
        proposer: &str,
        proposal: NewEmailMandate,
        judgement_note: Option<&str>,
    ) -> Result<Uuid> {
        let proposal = mandate::canonicalize_new_mandate(proposal)?;
        let id = Uuid::new_v4();
        let body = serde_json::json!({"id":id,"proposal":proposal,"judgement_note":judgement_note});
        sqlx::query("INSERT INTO restless_authority.records (company,kind,actor_id,body) VALUES ($1,'email_mandate_proposal',$2,$3)")
            .bind(company).bind(proposer).bind(body).execute(&self.pool).await?;
        Ok(id)
    }

    pub async fn record_email_observation(
        &self,
        company: &str,
        actor: &str,
        mut snapshot: serde_json::Value,
    ) -> Result<i64> {
        if actor.trim().is_empty() || !snapshot.is_object() {
            anyhow::bail!("email observation needs an attributed actor and JSON object snapshot");
        }
        let observation_id = Uuid::new_v4();
        let observed_at = Utc::now();
        let object = snapshot.as_object_mut().expect("validated JSON object");
        object.insert("observation_id".into(), serde_json::json!(observation_id));
        object.insert("observed_at".into(), serde_json::json!(observed_at));
        let row = sqlx::query("INSERT INTO restless_authority.records (company,kind,actor_id,body) VALUES ($1,'email_observation',$2,$3) RETURNING id")
            .bind(company).bind(actor).bind(snapshot).fetch_one(&self.pool).await?;
        Ok(sqlx::Row::try_get(&row, "id")?)
    }

    pub async fn pending_email_mandate_proposals(&self, company: &str) -> Result<Vec<AuthorityRecord>> {
        Ok(sqlx::query_as("SELECT p.id,p.actor_id,p.body,p.created_at FROM restless_authority.records p WHERE p.company=$1 AND p.kind='email_mandate_proposal' AND NOT EXISTS (SELECT 1 FROM restless_authority.records d WHERE d.company=p.company AND d.kind='email_mandate_proposal_decision' AND d.body->>'proposal_id'=p.body->>'id') ORDER BY p.id")
            .bind(company).fetch_all(&self.pool).await?)
    }

    pub async fn decide_email_mandate_proposal(&self, company: &str, owner: &str, proposal_id: Uuid, approve: bool, owner_note: Option<&str>) -> Result<Option<EmailMandate>> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(format!("email-mandate:{company}:{proposal_id}"))
            .execute(&mut *tx).await?;
        let proposal_row = sqlx::query("SELECT body FROM restless_authority.records WHERE company=$1 AND kind='email_mandate_proposal' AND body->>'id'=$2 ORDER BY id DESC LIMIT 1 FOR UPDATE")
            .bind(company).bind(proposal_id.to_string()).fetch_optional(&mut *tx).await?;
        let Some(row) = proposal_row else { anyhow::bail!("email mandate proposal not found") };
        let body: serde_json::Value = sqlx::Row::try_get(&row, "body")?;
        if let Some(decision) = sqlx::query("SELECT body FROM restless_authority.records WHERE company=$1 AND kind='email_mandate_proposal_decision' AND body->>'proposal_id'=$2 ORDER BY id LIMIT 1")
            .bind(company).bind(proposal_id.to_string()).fetch_optional(&mut *tx).await? {
            let old: serde_json::Value = sqlx::Row::try_get(&decision, "body")?;
            if old["decision"] == if approve {"approve"} else {"decline"} {
                tx.commit().await?;
                if approve {
                    let mandate_id = old["mandate_id"].as_str().and_then(|s| Uuid::parse_str(s).ok());
                    let granted = match mandate_id {
                        Some(mandate_id) => self.records_of_kind(company, "email_mandate_granted").await?.into_iter().find_map(|record| {
                            let mandate: EmailMandate = serde_json::from_value(record.body).ok()?;
                            (mandate.id == mandate_id).then_some(mandate)
                        }),
                        None => None,
                    };
                    return Ok(granted);
                }
                return Ok(None);
            }
            anyhow::bail!("email mandate proposal was already decided")
        }
        let proposal: NewEmailMandate = serde_json::from_value(body["proposal"].clone())?;
        let mandate = if approve {
            mandate::lock_company(&mut tx, company).await?;
            Some(mandate::grant_in_transaction(&mut tx, company, owner, proposal).await?)
        } else { None };
        let decision_body = serde_json::json!({"proposal_id":proposal_id,"decision":if approve {"approve"} else {"decline"},"mandate_id":mandate.as_ref().map(|m|m.id),"owner_note":owner_note});
        sqlx::query("INSERT INTO restless_authority.records (company,kind,actor_id,body) VALUES ($1,'email_mandate_proposal_decision',$2,$3)")
            .bind(company).bind(owner).bind(decision_body).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(mandate)
    }

    pub async fn list_email_mandates(&self, company: &str) -> Result<Vec<EmailMandate>> {
        mandate::list(&self.pool, company).await
    }

    pub async fn revoke_email_mandate(
        &self,
        company: &str,
        owner_actor_id: &str,
        mandate_id: Uuid,
        reason: &str,
    ) -> Result<()> {
        mandate::revoke(&self.pool, company, owner_actor_id, mandate_id, reason).await
    }

    pub async fn issue_email_permit(
        &self,
        company: &str,
        issuer_actor_id: &str,
        mandate_id: Uuid,
        proposal: EmailPermitProposal,
    ) -> Result<EmailPermit> {
        mandate::issue(&self.pool, company, issuer_actor_id, mandate_id, proposal).await
    }

    pub async fn list_email_permits(
        &self,
        company: &str,
        mandate_id: Uuid,
    ) -> Result<Vec<EmailPermit>> {
        mandate::list_permits(&self.pool, company, mandate_id).await
    }

    pub async fn email_mandate_usage(
        &self,
        company: &str,
        mandate_id: Uuid,
    ) -> Result<EmailMandateUsage> {
        mandate::usage(&self.pool, company, mandate_id).await
    }

    pub async fn reserve_email_send(
        &self,
        company: &str,
        permit_id: Uuid,
        actual_sender: &str,
        actual_sender_name: Option<&str>,
        recipient: &str,
        payload_sha256: &str,
        effect_key: &str,
    ) -> Result<EmailReservation> {
        mandate::reserve(
            &self.pool,
            company,
            permit_id,
            actual_sender,
            actual_sender_name,
            recipient,
            payload_sha256,
            effect_key,
        )
        .await
    }

    pub async fn record_email_send_status(
        &self,
        company: &str,
        permit_id: Uuid,
        effect_key: &str,
        outcome: ProviderOutcome,
        provider_ref: Option<&str>,
        provider_detail: Option<&str>,
    ) -> Result<()> {
        mandate::record_status(
            &self.pool,
            company,
            permit_id,
            effect_key,
            outcome,
            provider_ref,
            provider_detail,
        )
        .await
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

    /// The durable human Actor who currently holds root Authority for this
    /// company, if that fact has ever been explicitly recorded (Sprint 45 /
    /// C45-T4). Absent for every company that has never transferred
    /// ownership away from its bootstrap default — callers combine this with
    /// their own bootstrap/membership-owner fallback, never assume `None`
    /// means "nobody owns Authority".
    pub async fn current_authority_owner(&self, company: &str) -> Result<Option<String>> {
        Ok(sqlx::query_scalar(
            "SELECT actor_id FROM restless_authority.records \
             WHERE company=$1 AND kind='authority_ownership_transfer' \
             ORDER BY id DESC LIMIT 1",
        )
        .bind(company)
        .fetch_optional(&self.pool)
        .await?)
    }

    /// Transfer root Authority ownership from one durable Actor to another.
    ///
    /// Membership ownership and root Authority ownership are separate facts
    /// (ARCHITECTURE.md decision #28): this never touches
    /// `human_principal_actor_bindings.membership_role`, which stays
    /// Cloud/Better-Auth-owned truth. `expected_current_owner` is the
    /// caller's own computation of who holds Authority right now — the
    /// recorded transfer chain if one exists, else the bootstrap membership
    /// owner (or the local singleton `"owner"`). This call re-checks that
    /// computation against the authoritative chain under an advisory lock so
    /// two racing transfers cannot both believe they were first, and so a
    /// caller cannot transfer away Authority it does not currently hold —
    /// once a chain exists. Before the first transfer, this store has no
    /// independent way to verify who the true bootstrap owner is; the caller
    /// (owner.rs's `effective_authority_owner` plus its
    /// `principal.actor_id() == current.actor_id` check) must derive
    /// `expected_current_owner` from server-side membership/bootstrap data,
    /// never from client input.
    pub async fn transfer_authority_owner(
        &self,
        company: &str,
        expected_current_owner: &str,
        to_actor_id: &str,
        rationale: &str,
    ) -> Result<i64> {
        if to_actor_id.trim().is_empty() {
            bail!("Authority ownership transfer needs a destination Actor");
        }
        if rationale.trim().is_empty() {
            bail!("Authority ownership transfer needs a rationale");
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .context("begin Authority ownership transfer")?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
            .bind(format!("authority-owner:{company}"))
            .execute(&mut *tx)
            .await
            .context("serialize Authority ownership transfer")?;
        let current: Option<String> = sqlx::query_scalar(
            "SELECT actor_id FROM restless_authority.records \
             WHERE company=$1 AND kind='authority_ownership_transfer' \
             ORDER BY id DESC LIMIT 1",
        )
        .bind(company)
        .fetch_optional(&mut *tx)
        .await
        .context("read current Authority owner before transfer")?;
        let effective_current = current.as_deref().unwrap_or(expected_current_owner);
        if effective_current != expected_current_owner {
            bail!("Authority ownership already changed to a different Actor; refresh and retry");
        }
        if effective_current == to_actor_id {
            bail!("that Actor already holds Authority ownership");
        }
        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO restless_authority.records (company,kind,actor_id,body) \
             VALUES ($1,'authority_ownership_transfer',$2,$3) RETURNING id",
        )
        .bind(company)
        .bind(to_actor_id)
        .bind(serde_json::json!({
            "from_actor_id": effective_current,
            "rationale": rationale.trim(),
        }))
        .fetch_one(&mut *tx)
        .await
        .context("record Authority ownership transfer")?;
        tx.commit()
            .await
            .context("commit Authority ownership transfer")?;
        Ok(id)
    }

    /// Record one owner identity decision idempotently. Authority owns the
    /// decision; OrgIntel projects the resulting release and can safely retry
    /// after a crash between the two stores.
    pub async fn record_company_identity_decision(
        &self,
        company: &str,
        proposal_id: Uuid,
        decision: &str,
        rationale: &str,
        actor_id: &str,
    ) -> Result<i64> {
        let body = serde_json::json!({
            "proposal_id": proposal_id,
            "decision": decision,
            "rationale": rationale,
        });
        let inserted = sqlx::query_scalar::<_, i64>(
            "INSERT INTO restless_authority.records (company,kind,actor_id,body) \
             VALUES ($1,'company_identity_decision',$3,$2) \
             ON CONFLICT (company,kind,(body->>'proposal_id'),(body->>'decision')) \
             WHERE kind='company_identity_decision' DO NOTHING RETURNING id",
        )
        .bind(company)
        .bind(body)
        .bind(actor_id)
        .fetch_optional(&self.pool)
        .await?;
        if let Some(id) = inserted {
            return Ok(id);
        }
        sqlx::query_scalar(
            "SELECT id FROM restless_authority.records WHERE company=$1 \
             AND kind='company_identity_decision' AND body->>'proposal_id'=$2 \
             AND body->>'decision'=$3 ORDER BY id LIMIT 1",
        )
        .bind(company)
        .bind(proposal_id.to_string())
        .bind(decision)
        .fetch_one(&self.pool)
        .await
        .context("recover company identity decision")
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

    pub async fn recent_records_of_kind(
        &self,
        company: &str,
        kind: &str,
        limit: i64,
    ) -> Result<Vec<AuthorityRecord>> {
        sqlx::query_as(
            "SELECT id, actor_id, body, created_at \
             FROM restless_authority.records \
             WHERE company = $1 AND kind = $2 ORDER BY id DESC LIMIT $3",
        )
        .bind(company)
        .bind(kind)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .with_context(|| format!("read recent Authority {kind} records for {company}"))
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
        Self::initialise_company_in_transaction(&mut tx, company, config_approvals).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Initialise Authority state while a higher-level lifecycle transaction
    /// already owns serialization. Hosted company bootstrap uses this form so
    /// callers queued on its advisory lock cannot consume every pool
    /// connection and starve the lock holder of the connection it needs to
    /// finish.
    pub(crate) async fn initialise_company_in_transaction(
        tx: &mut Transaction<'_, Postgres>,
        company: &str,
        config_approvals: &[String],
    ) -> Result<()> {
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
            .execute(&mut **tx)
            .await?;
        }
        sqlx::query(
            "INSERT INTO restless_authority.company_migrations (company, version) VALUES ($1, $2) \
             ON CONFLICT (company) DO UPDATE SET version = EXCLUDED.version, imported_at = now()",
        )
        .bind(company)
        .bind(IMPORT_VERSION)
        .execute(&mut **tx)
        .await?;
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
        sqlx::query("DELETE FROM restless_authority.provider_connections WHERE company = $1")
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
}

#[cfg(test)]
mod identity_decision_attribution_tests {
    use super::*;

    /// Sprint 45 / C45-T2 regression: `record_company_identity_decision` used
    /// to hard-code the literal string `"owner"` as `actor_id` regardless of
    /// who actually decided. In hosted/multiplayer mode the acting human's
    /// actor id is a durable `human-{uuid}`, never that literal, so the old
    /// behaviour silently mis-attributed every hosted decision to nobody real.
    #[tokio::test]
    async fn identity_decision_is_attributed_to_the_real_deciding_actor_not_a_literal() {
        let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!(
                "RESTLESS_TEST_DATABASE_URL unset; skipping identity decision attribution scenario"
            );
            return;
        };
        let store = AuthorityStore::connect(&url)
            .await
            .expect("connect Authority store");
        let company = format!("identitydecisionattr{}", Uuid::new_v4().simple());
        let proposal_id = Uuid::new_v4();
        let human_actor = format!("human-{}", Uuid::new_v4());

        let record_id = store
            .record_company_identity_decision(
                &company,
                proposal_id,
                "promote",
                "Established one grammar with product truth.",
                &human_actor,
            )
            .await
            .expect("record identity decision");

        let stored: (Option<String>,) = sqlx::query_as(
            "SELECT actor_id FROM restless_authority.records WHERE id=$1 AND company=$2",
        )
        .bind(record_id)
        .bind(&company)
        .fetch_one(&store.pool)
        .await
        .expect("read back identity decision record");

        assert_eq!(
            stored.0.as_deref(),
            Some(human_actor.as_str()),
            "identity decision must attribute the real deciding actor, not a hard-coded literal"
        );
        assert_ne!(stored.0.as_deref(), Some("owner"));

        // Idempotent recovery of the same (company, proposal, decision) still
        // returns the original attributed row rather than silently rewriting it.
        let recovered_id = store
            .record_company_identity_decision(
                &company,
                proposal_id,
                "promote",
                "Established one grammar with product truth.",
                "a-different-later-caller",
            )
            .await
            .expect("recover identity decision");
        assert_eq!(recovered_id, record_id);
    }
}

#[cfg(test)]
mod authority_ownership_transfer_tests {
    use super::*;

    async fn store() -> Option<AuthorityStore> {
        let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
        Some(
            AuthorityStore::connect(&url)
                .await
                .expect("connect Authority store"),
        )
    }

    /// C45-T4: membership ownership and root Authority ownership are
    /// separate facts. A company that has never explicitly transferred
    /// Authority reports no owner from the Authority store at all — callers
    /// must supply their own bootstrap fallback, never assume the absence of
    /// a transfer record means Authority is unowned.
    #[tokio::test]
    async fn a_company_with_no_transfer_reports_no_authority_owner() {
        let Some(store) = store().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Authority ownership scenario");
            return;
        };
        let company = format!("authorityowner{}", Uuid::new_v4().simple());
        assert_eq!(store.current_authority_owner(&company).await.unwrap(), None);
    }

    #[tokio::test]
    async fn transfer_moves_ownership_and_is_attributed_and_ordered() {
        let Some(store) = store().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Authority ownership scenario");
            return;
        };
        let company = format!("authorityowner{}", Uuid::new_v4().simple());
        let bootstrap_owner = "human-bootstrap";
        let successor = "human-successor";

        store
            .transfer_authority_owner(&company, bootstrap_owner, successor, "planned handover")
            .await
            .expect("first transfer from the bootstrap default");
        assert_eq!(
            store.current_authority_owner(&company).await.unwrap(),
            Some(successor.to_string())
        );

        // A second, unrelated company's Authority ownership is untouched.
        let other_company = format!("authorityowner{}", Uuid::new_v4().simple());
        assert_eq!(
            store.current_authority_owner(&other_company).await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn transfer_refuses_a_stale_expected_owner_once_a_transfer_chain_exists() {
        let Some(store) = store().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Authority ownership scenario");
            return;
        };
        let company = format!("authorityowner{}", Uuid::new_v4().simple());
        let bootstrap_owner = "human-bootstrap";

        // Before any transfer exists, the store cannot independently verify
        // who the true bootstrap owner is — that verification is the
        // caller's job, checking `expected_current_owner` against
        // server-derived membership/bootstrap data (owner.rs's
        // `effective_authority_owner`) before ever reaching this call. What
        // the store alone guarantees is race/staleness safety once a chain
        // exists: the exact resulting owner is the only one who can move it
        // again.
        store
            .transfer_authority_owner(&company, bootstrap_owner, "human-successor", "handover")
            .await
            .expect("legitimate first transfer");

        // The bootstrap default no longer holds Authority; claiming it still
        // does must fail even though it was correct a moment ago.
        assert!(store
            .transfer_authority_owner(&company, bootstrap_owner, "human-third", "stale retry")
            .await
            .is_err());
        assert_eq!(
            store.current_authority_owner(&company).await.unwrap(),
            Some("human-successor".to_string())
        );
    }

    #[tokio::test]
    async fn transfer_refuses_an_empty_destination_or_rationale_and_a_self_transfer() {
        let Some(store) = store().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Authority ownership scenario");
            return;
        };
        let company = format!("authorityowner{}", Uuid::new_v4().simple());
        assert!(store
            .transfer_authority_owner(&company, "human-bootstrap", "", "rationale")
            .await
            .is_err());
        assert!(store
            .transfer_authority_owner(&company, "human-bootstrap", "human-successor", "")
            .await
            .is_err());
        assert!(store
            .transfer_authority_owner(
                &company,
                "human-bootstrap",
                "human-bootstrap",
                "transfer to self"
            )
            .await
            .is_err());
        assert_eq!(store.current_authority_owner(&company).await.unwrap(), None);
    }
}
