//! Durable admission and receipts for provider/model invocations.
//!
//! Work Attempts remain the productive-work authority and the host spend
//! ledger remains the money authority.  This module supplies the missing
//! cross-replica count fence at the exact provider-launch boundary, including
//! collaboration turns that have no Work Attempt of their own.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{OrgIntel, OrgIntelError, Result};

const RECONCILE_ON_ADMISSION: i64 = 64;
const MAX_RECONCILE_BATCH: i64 = 128;
const MAX_RECENT_RECEIPTS: i64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "model_invocation_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ModelInvocationKind {
    Work,
    OwnerConversation,
    RoomMention,
}

impl ModelInvocationKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Work => "work",
            Self::OwnerConversation => "owner_conversation",
            Self::RoomMention => "room_mention",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "model_invocation_state", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ModelInvocationState {
    Admitted,
    Settled,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "model_invocation_outcome", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ModelInvocationOutcome {
    Completed,
    Blocked,
    Failed,
    Cancelled,
}

impl ModelInvocationOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// The durable source claim that makes a provider invocation legitimate.
/// Constructors are represented as enum variants so an invalid combination
/// cannot reach SQL and accidentally become another launch class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelInvocationSource {
    Work {
        work_id: Uuid,
        attempt_id: Uuid,
    },
    OwnerConversation {
        cognitive_lease_token: Uuid,
    },
    RoomMention {
        cognitive_lease_token: Uuid,
        mention_id: Uuid,
    },
}

impl ModelInvocationSource {
    pub fn kind(self) -> ModelInvocationKind {
        match self {
            Self::Work { .. } => ModelInvocationKind::Work,
            Self::OwnerConversation { .. } => ModelInvocationKind::OwnerConversation,
            Self::RoomMention { .. } => ModelInvocationKind::RoomMention,
        }
    }

    pub fn subject_id(self) -> Uuid {
        match self {
            Self::Work { attempt_id, .. } => attempt_id,
            Self::OwnerConversation {
                cognitive_lease_token,
            }
            | Self::RoomMention {
                cognitive_lease_token,
                ..
            } => cognitive_lease_token,
        }
    }

    fn work_id(self) -> Option<Uuid> {
        match self {
            Self::Work { work_id, .. } => Some(work_id),
            Self::OwnerConversation { .. } | Self::RoomMention { .. } => None,
        }
    }

    fn attempt_id(self) -> Option<Uuid> {
        match self {
            Self::Work { attempt_id, .. } => Some(attempt_id),
            Self::OwnerConversation { .. } | Self::RoomMention { .. } => None,
        }
    }

    fn cognitive_lease_token(self) -> Option<Uuid> {
        match self {
            Self::Work { .. } => None,
            Self::OwnerConversation {
                cognitive_lease_token,
            }
            | Self::RoomMention {
                cognitive_lease_token,
                ..
            } => Some(cognitive_lease_token),
        }
    }

    fn mention_id(self) -> Option<Uuid> {
        match self {
            Self::RoomMention { mention_id, .. } => Some(mention_id),
            Self::Work { .. } | Self::OwnerConversation { .. } => None,
        }
    }
}

pub struct NewModelInvocationAdmission<'a> {
    pub client_command_id: &'a str,
    pub actor_id: &'a str,
    pub model: &'a str,
    pub harness: &'a str,
    pub configured_effort: &'a str,
    pub source: ModelInvocationSource,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ModelInvocationReceiptRow {
    pub id: Uuid,
    pub client_command_id: String,
    pub client_payload_sha256: String,
    pub actor_id: String,
    pub kind: ModelInvocationKind,
    pub subject_id: Uuid,
    pub model: String,
    pub harness: String,
    pub configured_effort: String,
    pub work_id: Option<Uuid>,
    pub attempt_id: Option<Uuid>,
    pub mention_id: Option<Uuid>,
    pub window_started_at: DateTime<Utc>,
    pub admitted_at: DateTime<Utc>,
    pub reclaim_after: DateTime<Utc>,
    pub state: ModelInvocationState,
    pub settlement_payload_sha256: Option<String>,
    pub outcome: Option<ModelInvocationOutcome>,
    pub evidence: Option<serde_json::Value>,
    pub settled_at: Option<DateTime<Utc>>,
    pub expired_at: Option<DateTime<Utc>>,
    pub expiry_reason: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct ModelInvocationAdmissionDbRow {
    id: Uuid,
    admission_token: Uuid,
    client_command_id: String,
    client_payload_sha256: String,
    actor_id: String,
    kind: ModelInvocationKind,
    subject_id: Uuid,
    model: String,
    harness: String,
    configured_effort: String,
    work_id: Option<Uuid>,
    attempt_id: Option<Uuid>,
    mention_id: Option<Uuid>,
    window_started_at: DateTime<Utc>,
    admitted_at: DateTime<Utc>,
    reclaim_after: DateTime<Utc>,
    state: ModelInvocationState,
    settlement_payload_sha256: Option<String>,
    outcome: Option<ModelInvocationOutcome>,
    evidence: Option<serde_json::Value>,
    settled_at: Option<DateTime<Utc>>,
    expired_at: Option<DateTime<Utc>>,
    expiry_reason: Option<String>,
}

impl ModelInvocationAdmissionDbRow {
    fn receipt(&self) -> ModelInvocationReceiptRow {
        ModelInvocationReceiptRow {
            id: self.id,
            client_command_id: self.client_command_id.clone(),
            client_payload_sha256: self.client_payload_sha256.clone(),
            actor_id: self.actor_id.clone(),
            kind: self.kind,
            subject_id: self.subject_id,
            model: self.model.clone(),
            harness: self.harness.clone(),
            configured_effort: self.configured_effort.clone(),
            work_id: self.work_id,
            attempt_id: self.attempt_id,
            mention_id: self.mention_id,
            window_started_at: self.window_started_at,
            admitted_at: self.admitted_at,
            reclaim_after: self.reclaim_after,
            state: self.state,
            settlement_payload_sha256: self.settlement_payload_sha256.clone(),
            outcome: self.outcome,
            evidence: self.evidence.clone(),
            settled_at: self.settled_at,
            expired_at: self.expired_at,
            expiry_reason: self.expiry_reason.clone(),
        }
    }
}

/// Unforgeable launch permission.  Its token is intentionally private: code
/// outside OrgIntel can carry the permit to the model boundary and settlement
/// API, but cannot construct or alter one.
#[derive(Debug)]
pub struct ModelInvocationAdmission {
    row: ModelInvocationAdmissionDbRow,
}

impl ModelInvocationAdmission {
    pub fn id(&self) -> Uuid {
        self.row.id
    }

    pub fn receipt(&self) -> ModelInvocationReceiptRow {
        self.row.receipt()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelInvocationLimitScope {
    Company,
    Actor,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelInvocationBudgetSnapshot {
    pub actor_id: String,
    pub observed_at: DateTime<Utc>,
    pub window_started_at: DateTime<Utc>,
    pub window_ends_at: DateTime<Utc>,
    pub window_seconds: i32,
    pub company_limit: i32,
    pub company_used: i64,
    pub company_remaining: i64,
    pub actor_limit: i32,
    pub actor_used: i64,
    pub actor_remaining: i64,
}

#[derive(Debug)]
pub enum ModelInvocationAdmissionDecision {
    /// The caller owns the one permit that may cross the provider boundary.
    Admitted(ModelInvocationAdmission),
    /// Exact lost-response replay. This receipt deliberately carries no
    /// settlement token and therefore cannot authorize another launch.
    AlreadyAdmitted(ModelInvocationReceiptRow),
    AlreadySettled(ModelInvocationReceiptRow),
    Expired(ModelInvocationReceiptRow),
    Denied {
        scope: ModelInvocationLimitScope,
        budget: ModelInvocationBudgetSnapshot,
    },
}

pub struct ModelInvocationSettlement {
    pub outcome: ModelInvocationOutcome,
    /// Bounded, canonicalized launch evidence. It must be an object and must
    /// describe observed execution, never an inferred generic health state.
    pub evidence: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelInvocationSettlementReceipt {
    pub invocation: ModelInvocationReceiptRow,
    /// False is an exact retry after a lost settlement response.
    pub created: bool,
}

#[derive(Debug, Clone, Copy, sqlx::FromRow)]
struct ModelInvocationPolicyRow {
    window_seconds: i32,
    company_limit: i32,
    actor_limit: i32,
    reclaim_after_seconds: i32,
}

const RECEIPT_COLUMNS: &str =
    "id,client_command_id,client_payload_sha256,actor_id,kind,subject_id,model,\
     harness,configured_effort,work_id,attempt_id,mention_id,window_started_at,admitted_at,reclaim_after,state,\
     settlement_payload_sha256,outcome,evidence,settled_at,expired_at,expiry_reason";

fn validate_admission(request: &NewModelInvocationAdmission<'_>) -> Result<()> {
    if request.client_command_id.trim().is_empty() || request.client_command_id.len() > 200 {
        return Err(OrgIntelError::InvalidWork(
            "model invocation command id must contain 1..=200 bytes".into(),
        ));
    }
    if request.model.trim().is_empty() || request.model.len() > 512 {
        return Err(OrgIntelError::InvalidWork(
            "model invocation provider model must contain 1..=512 bytes".into(),
        ));
    }
    if request.harness.trim().is_empty() || request.harness.len() > 64 {
        return Err(OrgIntelError::InvalidWork(
            "model invocation harness must contain 1..=64 bytes".into(),
        ));
    }
    if request.configured_effort.trim().is_empty() || request.configured_effort.len() > 64 {
        return Err(OrgIntelError::InvalidWork(
            "model invocation reasoning effort must contain 1..=64 bytes".into(),
        ));
    }
    Ok(())
}

fn admission_digest(request: &NewModelInvocationAdmission<'_>) -> String {
    let semantics = serde_json::json!({
        "actor": request.actor_id,
        "attempt": request.source.attempt_id(),
        "domain": "restless.model-invocation-admission.v1",
        "harness": request.harness,
        "kind": request.source.kind(),
        "lease": request.source.cognitive_lease_token(),
        "mention": request.source.mention_id(),
        "model": request.model,
        "reasoning_effort": request.configured_effort,
        "subject": request.source.subject_id(),
        "work": request.source.work_id(),
    });
    let mut canonical = Vec::new();
    canonical_json(&semantics, &mut canonical);
    format!("{:x}", Sha256::digest(canonical))
}

fn canonical_json(value: &serde_json::Value, output: &mut Vec<u8>) {
    match value {
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => {
            output.extend(serde_json::to_vec(value).expect("JSON scalar is serializable"));
        }
        serde_json::Value::Array(values) => {
            output.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                canonical_json(value, output);
            }
            output.push(b']');
        }
        serde_json::Value::Object(values) => {
            output.push(b'{');
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            for (index, key) in keys.into_iter().enumerate() {
                if index != 0 {
                    output.push(b',');
                }
                output.extend(serde_json::to_vec(key).expect("JSON object key is serializable"));
                output.push(b':');
                canonical_json(&values[key], output);
            }
            output.push(b'}');
        }
    }
}

fn settlement_digest(settlement: &ModelInvocationSettlement) -> String {
    let mut bytes = format!(
        "domain=restless.model-invocation-settlement.v1\noutcome={}\nevidence=",
        settlement.outcome.as_str()
    )
    .into_bytes();
    canonical_json(&settlement.evidence, &mut bytes);
    bytes.push(b'\n');
    format!("{:x}", Sha256::digest(bytes))
}

fn receipt_query(prefix: &str) -> String {
    format!("{prefix} {RECEIPT_COLUMNS} FROM model_invocation_admissions")
}

impl OrgIntel {
    /// Resolve the exact live cognitive source immediately before admission.
    /// Admission revalidates it under the Actor lock, so this convenience read
    /// cannot turn an expired or replaced lease into launch authority.
    pub async fn current_cognitive_model_invocation_source(
        &self,
        actor_id: &str,
        focused_mention: bool,
    ) -> Result<ModelInvocationSource> {
        let current: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "SELECT lease_token,focused_mention_id FROM actor_cognitive_leases \
             WHERE actor_id=$1 AND claimed_until>current_timestamp AND revoked_at IS NULL",
        )
        .bind(actor_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some((cognitive_lease_token, mention_id)) = current else {
            return Err(OrgIntelError::InvalidWork(
                "no exact live cognitive lease exists for model invocation admission".into(),
            ));
        };
        match (focused_mention, mention_id) {
            (true, Some(mention_id)) => Ok(ModelInvocationSource::RoomMention {
                cognitive_lease_token,
                mention_id,
            }),
            (false, None) => Ok(ModelInvocationSource::OwnerConversation {
                cognitive_lease_token,
            }),
            (true, None) => Err(OrgIntelError::InvalidWork(
                "focused mention invocation has no mention bound to its cognitive lease".into(),
            )),
            (false, Some(_)) => Err(OrgIntelError::InvalidWork(
                "owner conversation invocation cannot bypass a focused Room mention".into(),
            )),
        }
    }

    /// Atomically reserve one provider launch. Every replica contends on the
    /// company policy row, so neither the company nor Actor bound can race.
    pub async fn admit_model_invocation(
        &self,
        request: NewModelInvocationAdmission<'_>,
    ) -> Result<ModelInvocationAdmissionDecision> {
        validate_admission(&request)?;
        let payload_sha256 = admission_digest(&request);
        let mut tx = self.pool.begin().await?;
        let policy: ModelInvocationPolicyRow = sqlx::query_as(
            "SELECT window_seconds,company_limit,actor_limit,reclaim_after_seconds \
             FROM model_invocation_policy WHERE singleton=TRUE FOR UPDATE",
        )
        .fetch_one(&mut *tx)
        .await?;

        let prior_sql = format!(
            "{} WHERE client_command_id=$1 FOR UPDATE",
            receipt_query("SELECT admission_token,")
        );
        if let Some(prior) = sqlx::query_as::<_, ModelInvocationAdmissionDbRow>(&prior_sql)
            .bind(request.client_command_id)
            .fetch_optional(&mut *tx)
            .await?
        {
            if prior.client_payload_sha256 != payload_sha256 {
                return Err(OrgIntelError::InvalidWork(
                    "model invocation command id was reused with different launch semantics".into(),
                ));
            }
            // An idempotency receipt is not enduring launch authority. A
            // retry may reuse the permit only while the exact Work Attempt or
            // cognitive lease that originally justified it is still live.
            if prior.state == ModelInvocationState::Admitted {
                validate_source_claim_in_tx(&mut tx, request.actor_id, request.source).await?;
            }
            tx.commit().await?;
            return Ok(match prior.state {
                ModelInvocationState::Admitted => {
                    ModelInvocationAdmissionDecision::AlreadyAdmitted(prior.receipt())
                }
                ModelInvocationState::Settled => {
                    ModelInvocationAdmissionDecision::AlreadySettled(prior.receipt())
                }
                ModelInvocationState::Expired => {
                    ModelInvocationAdmissionDecision::Expired(prior.receipt())
                }
            });
        }

        expire_abandoned_in_tx(&mut tx, RECONCILE_ON_ADMISSION).await?;
        validate_source_claim_in_tx(&mut tx, request.actor_id, request.source).await?;

        let (observed_at, window_started_at): (DateTime<Utc>, DateTime<Utc>) = sqlx::query_as(
            "SELECT current_timestamp, \
                 to_timestamp(floor(extract(epoch FROM current_timestamp)/$1)*$1)",
        )
        .bind(policy.window_seconds as f64)
        .fetch_one(&mut *tx)
        .await?;
        let company_used: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM model_invocation_admissions \
             WHERE window_started_at=$1 AND state IN ('admitted','settled')",
        )
        .bind(window_started_at)
        .fetch_one(&mut *tx)
        .await?;
        let actor_used: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM model_invocation_admissions \
             WHERE actor_id=$1 AND window_started_at=$2 \
               AND state IN ('admitted','settled')",
        )
        .bind(request.actor_id)
        .bind(window_started_at)
        .fetch_one(&mut *tx)
        .await?;
        let budget = budget_snapshot(
            request.actor_id,
            observed_at,
            window_started_at,
            policy,
            company_used,
            actor_used,
        );
        let denied_scope = if company_used >= i64::from(policy.company_limit) {
            Some(ModelInvocationLimitScope::Company)
        } else if actor_used >= i64::from(policy.actor_limit) {
            Some(ModelInvocationLimitScope::Actor)
        } else {
            None
        };
        if let Some(scope) = denied_scope {
            tx.commit().await?;
            return Ok(ModelInvocationAdmissionDecision::Denied { scope, budget });
        }

        let id = Uuid::new_v4();
        let token = Uuid::new_v4();
        let insert_sql = format!(
            "INSERT INTO model_invocation_admissions \
             (id,admission_token,client_command_id,client_payload_sha256,actor_id,kind,\
              subject_id,model,harness,configured_effort,work_id,attempt_id,cognitive_lease_token,mention_id,\
              window_started_at,admitted_at,reclaim_after) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,\
                     $16+make_interval(secs=>$17)) RETURNING admission_token,{RECEIPT_COLUMNS}"
        );
        let row: ModelInvocationAdmissionDbRow = sqlx::query_as(&insert_sql)
            .bind(id)
            .bind(token)
            .bind(request.client_command_id)
            .bind(&payload_sha256)
            .bind(request.actor_id)
            .bind(request.source.kind())
            .bind(request.source.subject_id())
            .bind(request.model)
            .bind(request.harness)
            .bind(request.configured_effort)
            .bind(request.source.work_id())
            .bind(request.source.attempt_id())
            .bind(request.source.cognitive_lease_token())
            .bind(request.source.mention_id())
            .bind(window_started_at)
            .bind(observed_at)
            .bind(policy.reclaim_after_seconds as f64)
            .fetch_one(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO events (kind,actor_id,body) VALUES \
             ('model_invocation_admitted',$1,$2)",
        )
        .bind(request.actor_id)
        .bind(serde_json::json!({
            "invocation_id": row.id,
            "command_id": request.client_command_id,
            "payload_sha256": payload_sha256,
            "kind": request.source.kind(),
            "subject_id": request.source.subject_id(),
            "model": request.model,
            "harness": request.harness,
            "configured_effort": request.configured_effort,
            "window_started_at": window_started_at,
            "company_used": company_used + 1,
            "company_limit": policy.company_limit,
            "actor_used": actor_used + 1,
            "actor_limit": policy.actor_limit,
        }))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(ModelInvocationAdmissionDecision::Admitted(
            ModelInvocationAdmission { row },
        ))
    }

    /// Settle an exact admitted invocation once. A lost response can replay
    /// the same evidence and receive the original receipt; semantic drift is
    /// rejected and an expired permit can never manufacture completion.
    pub async fn settle_model_invocation(
        &self,
        admission: &ModelInvocationAdmission,
        settlement: ModelInvocationSettlement,
    ) -> Result<ModelInvocationSettlementReceipt> {
        if !settlement.evidence.is_object() {
            return Err(OrgIntelError::InvalidWork(
                "model invocation settlement evidence must be a JSON object".into(),
            ));
        }
        let mut canonical_evidence = Vec::new();
        canonical_json(&settlement.evidence, &mut canonical_evidence);
        if canonical_evidence.len() > 4096 {
            return Err(OrgIntelError::InvalidWork(
                "model invocation settlement evidence exceeds 4096 bytes".into(),
            ));
        }
        let payload_sha256 = settlement_digest(&settlement);
        let mut tx = self.pool.begin().await?;
        let select_sql = format!(
            "{} WHERE id=$1 FOR UPDATE",
            receipt_query("SELECT admission_token,")
        );
        let prior: ModelInvocationAdmissionDbRow = sqlx::query_as(&select_sql)
            .bind(admission.row.id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                OrgIntelError::InvalidWork(
                    "model invocation admission no longer exists in this company".into(),
                )
            })?;
        if prior.admission_token != admission.row.admission_token
            || prior.actor_id != admission.row.actor_id
            || prior.client_payload_sha256 != admission.row.client_payload_sha256
        {
            return Err(OrgIntelError::InvalidWork(
                "model invocation settlement permit does not match its durable admission".into(),
            ));
        }
        match prior.state {
            ModelInvocationState::Settled => {
                if prior.settlement_payload_sha256.as_deref() != Some(&payload_sha256) {
                    return Err(OrgIntelError::InvalidWork(
                        "model invocation settlement was replayed with different evidence".into(),
                    ));
                }
                tx.commit().await?;
                return Ok(ModelInvocationSettlementReceipt {
                    invocation: prior.receipt(),
                    created: false,
                });
            }
            ModelInvocationState::Expired => {
                return Err(OrgIntelError::InvalidWork(
                    "expired model invocation admission cannot be settled".into(),
                ));
            }
            ModelInvocationState::Admitted => {}
        }
        let update_sql = format!(
            "UPDATE model_invocation_admissions SET \
             state='settled',settlement_payload_sha256=$2,outcome=$3,evidence=$4,\
             settled_at=current_timestamp WHERE id=$1 RETURNING admission_token,{RECEIPT_COLUMNS}"
        );
        let settled: ModelInvocationAdmissionDbRow = sqlx::query_as(&update_sql)
            .bind(prior.id)
            .bind(&payload_sha256)
            .bind(settlement.outcome)
            .bind(&settlement.evidence)
            .fetch_one(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO events (kind,actor_id,body) VALUES \
             ('model_invocation_settled',$1,$2)",
        )
        .bind(&settled.actor_id)
        .bind(serde_json::json!({
            "invocation_id": settled.id,
            "admission_payload_sha256": settled.client_payload_sha256,
            "settlement_payload_sha256": payload_sha256,
            "kind": settled.kind,
            "subject_id": settled.subject_id,
            "model": settled.model,
            "harness": settled.harness,
            "configured_effort": settled.configured_effort,
            "outcome": settlement.outcome,
            "evidence": settlement.evidence,
        }))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(ModelInvocationSettlementReceipt {
            invocation: settled.receipt(),
            created: true,
        })
    }

    /// Reclaim only a bounded set of old, unsettled admissions whose durable
    /// source claim is no longer live. Merely reaching a wall-clock deadline
    /// is insufficient while a Work Attempt or cognitive lease remains live.
    pub async fn reconcile_expired_model_invocations(
        &self,
        limit: i64,
    ) -> Result<Vec<ModelInvocationReceiptRow>> {
        let limit = limit.clamp(1, MAX_RECONCILE_BATCH);
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "SELECT singleton FROM model_invocation_policy WHERE singleton=TRUE FOR UPDATE",
        )
        .fetch_one(&mut *tx)
        .await?;
        let rows = expire_abandoned_in_tx(&mut tx, limit).await?;
        tx.commit().await?;
        Ok(rows)
    }

    /// Exact current window use for one Actor. This is a bounded read and does
    /// not reclaim or infer state from a provider or daemon health endpoint.
    pub async fn model_invocation_budget_snapshot(
        &self,
        actor_id: &str,
    ) -> Result<ModelInvocationBudgetSnapshot> {
        let mut tx = self.pool.begin().await?;
        let policy: ModelInvocationPolicyRow = sqlx::query_as(
            "SELECT window_seconds,company_limit,actor_limit,reclaim_after_seconds \
             FROM model_invocation_policy WHERE singleton=TRUE FOR SHARE",
        )
        .fetch_one(&mut *tx)
        .await?;
        let (observed_at, window_started_at): (DateTime<Utc>, DateTime<Utc>) = sqlx::query_as(
            "SELECT current_timestamp, \
                 to_timestamp(floor(extract(epoch FROM current_timestamp)/$1)*$1)",
        )
        .bind(policy.window_seconds as f64)
        .fetch_one(&mut *tx)
        .await?;
        let company_used: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM model_invocation_admissions \
             WHERE window_started_at=$1 AND state IN ('admitted','settled')",
        )
        .bind(window_started_at)
        .fetch_one(&mut *tx)
        .await?;
        let actor_used: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM model_invocation_admissions \
             WHERE actor_id=$1 AND window_started_at=$2 \
               AND state IN ('admitted','settled')",
        )
        .bind(actor_id)
        .bind(window_started_at)
        .fetch_one(&mut *tx)
        .await?;
        let snapshot = budget_snapshot(
            actor_id,
            observed_at,
            window_started_at,
            policy,
            company_used,
            actor_used,
        );
        tx.commit().await?;
        Ok(snapshot)
    }

    /// Latest immutable admission/settlement evidence. Tokens are never part
    /// of this projection and the caller cannot request an unbounded history.
    pub async fn recent_model_invocations(
        &self,
        limit: i64,
    ) -> Result<Vec<ModelInvocationReceiptRow>> {
        let limit = limit.clamp(1, MAX_RECENT_RECEIPTS);
        let sql = format!(
            "{} ORDER BY admitted_at DESC,id DESC LIMIT $1",
            receipt_query("SELECT")
        );
        Ok(sqlx::query_as(&sql)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?)
    }
}

fn budget_snapshot(
    actor_id: &str,
    observed_at: DateTime<Utc>,
    window_started_at: DateTime<Utc>,
    policy: ModelInvocationPolicyRow,
    company_used: i64,
    actor_used: i64,
) -> ModelInvocationBudgetSnapshot {
    ModelInvocationBudgetSnapshot {
        actor_id: actor_id.to_string(),
        observed_at,
        window_started_at,
        window_ends_at: window_started_at
            + chrono::Duration::seconds(i64::from(policy.window_seconds)),
        window_seconds: policy.window_seconds,
        company_limit: policy.company_limit,
        company_used,
        company_remaining: i64::from(policy.company_limit)
            .saturating_sub(company_used)
            .max(0),
        actor_limit: policy.actor_limit,
        actor_used,
        actor_remaining: i64::from(policy.actor_limit)
            .saturating_sub(actor_used)
            .max(0),
    }
}

async fn validate_source_claim_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: &str,
    source: ModelInvocationSource,
) -> Result<()> {
    match source {
        ModelInvocationSource::Work {
            work_id,
            attempt_id,
        } => {
            let accountability = crate::actors::lock_work_accountability_in_tx(tx, work_id).await?;
            let valid: bool = accountability.owner_id == actor_id
                && sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM work_attempts \
                     WHERE id=$1 AND work_id=$2 AND actor_id=$3 AND state='running')",
                )
                .bind(attempt_id)
                .bind(work_id)
                .bind(actor_id)
                .fetch_one(&mut **tx)
                .await?;
            if !valid {
                return Err(OrgIntelError::InvalidWork(
                    "model invocation is not bound to this Actor's live Work Attempt".into(),
                ));
            }
        }
        ModelInvocationSource::OwnerConversation {
            cognitive_lease_token,
        } => {
            validate_cognitive_source_in_tx(tx, actor_id, cognitive_lease_token, None).await?;
        }
        ModelInvocationSource::RoomMention {
            cognitive_lease_token,
            mention_id,
        } => {
            validate_cognitive_source_in_tx(tx, actor_id, cognitive_lease_token, Some(mention_id))
                .await?;
        }
    }
    Ok(())
}

async fn validate_cognitive_source_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: &str,
    lease_token: Uuid,
    mention_id: Option<Uuid>,
) -> Result<()> {
    let active_actor: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM actors \
         WHERE id=$1 AND retired_at IS NULL AND actor_class='agent' FOR UPDATE)",
    )
    .bind(actor_id)
    .fetch_one(&mut **tx)
    .await?;
    let valid_lease: bool = active_actor
        && sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actor_cognitive_leases \
             WHERE actor_id=$1 AND lease_token=$2 AND claimed_until>current_timestamp \
               AND revoked_at IS NULL AND focused_mention_id IS NOT DISTINCT FROM $3)",
        )
        .bind(actor_id)
        .bind(lease_token)
        .bind(mention_id)
        .fetch_one(&mut **tx)
        .await?;
    if !valid_lease {
        return Err(OrgIntelError::InvalidWork(
            "model invocation is not bound to this Actor's exact live cognitive lease and focus"
                .into(),
        ));
    }
    if let Some(mention_id) = mention_id {
        let valid_mention: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM message_mentions \
             WHERE id=$1 AND mentioned_actor_id=$2 AND claim_token=$3 \
               AND resolution_message_id IS NULL AND cancelled_event_id IS NULL \
               AND claimed_until>current_timestamp)",
        )
        .bind(mention_id)
        .bind(actor_id)
        .bind(lease_token)
        .fetch_one(&mut **tx)
        .await?;
        if !valid_mention {
            return Err(OrgIntelError::InvalidWork(
                "model invocation mention is no longer owed to this cognitive lease".into(),
            ));
        }
    }
    Ok(())
}

async fn expire_abandoned_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    limit: i64,
) -> Result<Vec<ModelInvocationReceiptRow>> {
    let sql = format!(
        "WITH candidates AS (\
           SELECT admission.id FROM model_invocation_admissions AS admission \
           WHERE admission.state='admitted' AND admission.reclaim_after<=current_timestamp AND (\
             (admission.kind='work' AND NOT EXISTS (\
               SELECT 1 FROM work_attempts AS attempt \
               WHERE attempt.id=admission.attempt_id \
                 AND attempt.work_id=admission.work_id \
                 AND attempt.actor_id=admission.actor_id \
                 AND attempt.state='running'\
             )) OR\
             (admission.kind='owner_conversation' AND NOT EXISTS (\
               SELECT 1 FROM actor_cognitive_leases AS lease \
               WHERE lease.actor_id=admission.actor_id \
                 AND lease.lease_token=admission.cognitive_lease_token \
                 AND lease.claimed_until>current_timestamp \
                 AND lease.revoked_at IS NULL \
                 AND lease.focused_mention_id IS NULL\
             )) OR\
             (admission.kind='room_mention' AND (\
               NOT EXISTS (\
                 SELECT 1 FROM actor_cognitive_leases AS lease \
                 WHERE lease.actor_id=admission.actor_id \
                   AND lease.lease_token=admission.cognitive_lease_token \
                   AND lease.claimed_until>current_timestamp \
                   AND lease.revoked_at IS NULL \
                   AND lease.focused_mention_id=admission.mention_id\
               ) OR NOT EXISTS (\
                 SELECT 1 FROM message_mentions AS mention \
                 WHERE mention.id=admission.mention_id \
                   AND mention.mentioned_actor_id=admission.actor_id \
                   AND mention.claim_token=admission.cognitive_lease_token \
                   AND mention.resolution_message_id IS NULL \
                   AND mention.cancelled_event_id IS NULL \
                   AND mention.claimed_until>current_timestamp\
               )\
             ))\
           ) \
           ORDER BY admission.reclaim_after,admission.id \
           FOR UPDATE SKIP LOCKED LIMIT $1\
         ), expired AS (\
           UPDATE model_invocation_admissions AS admission \
           SET state='expired',expired_at=current_timestamp,\
               expiry_reason='source claim no longer live after bounded reclaim delay' \
           WHERE admission.id IN (SELECT id FROM candidates) \
             AND admission.state='admitted' \
           RETURNING {RECEIPT_COLUMNS}\
         ) SELECT * FROM expired ORDER BY admitted_at,id"
    );
    let rows: Vec<ModelInvocationReceiptRow> = sqlx::query_as(&sql)
        .bind(limit)
        .fetch_all(&mut **tx)
        .await?;
    for row in &rows {
        sqlx::query(
            "INSERT INTO events (kind,actor_id,body) VALUES \
             ('model_invocation_expired',$1,$2)",
        )
        .bind(&row.actor_id)
        .bind(serde_json::json!({
            "invocation_id": row.id,
            "admission_payload_sha256": row.client_payload_sha256,
            "kind": row.kind,
            "subject_id": row.subject_id,
            "model": row.model,
            "harness": row.harness,
            "configured_effort": row.configured_effort,
            "reason": row.expiry_reason,
        }))
        .execute(&mut **tx)
        .await?;
    }
    Ok(rows)
}
