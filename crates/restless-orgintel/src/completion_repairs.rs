//! Durable bounded admission for completion-envelope-only corrections.
//!
//! This ledger counts reservations before a correction provider call. It is
//! deliberately keyed to the Work Attempt so a resumed model session cannot
//! reset its repair allowance.

use serde::Serialize;
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

use crate::{OrgIntel, OrgIntelError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionRepairDecision {
    Admitted,
    AlreadyReserved,
    LimitReached,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompletionRepairBudget {
    pub attempt_id: Uuid,
    pub repairs_total: i32,
    pub repairs_for_fingerprint: i32,
}

impl OrgIntel {
    /// Reserve one envelope-only repair before model dispatch. The command id
    /// makes an uncertain retry detectable; `AlreadyReserved` must never be
    /// interpreted as permission to dispatch the correction again.
    pub async fn reserve_completion_protocol_repair(
        &self,
        attempt_id: Uuid,
        command_id: Uuid,
        failure_fingerprint: &str,
        max_per_fingerprint: i32,
        max_total: i32,
    ) -> Result<(CompletionRepairDecision, CompletionRepairBudget)> {
        if attempt_id.is_nil()
            || command_id.is_nil()
            || !(1..=8).contains(&max_per_fingerprint)
            || !(1..=16).contains(&max_total)
        {
            return Err(OrgIntelError::InvalidWork(
                "completion repair reservation has invalid identity or limits".into(),
            ));
        }
        let fingerprint = failure_fingerprint.trim();
        if fingerprint.len() != 64 || !fingerprint.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(OrgIntelError::InvalidWork(
                "completion repair fingerprint must be a SHA-256 hex digest".into(),
            ));
        }
        let fingerprint = fingerprint.to_ascii_lowercase();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO completion_protocol_repair_budgets (attempt_id) VALUES ($1) \
             ON CONFLICT (attempt_id) DO NOTHING",
        )
        .bind(attempt_id)
        .execute(&mut *tx)
        .await?;

        let total: i32 = sqlx::query_scalar(
            "SELECT repairs_total FROM completion_protocol_repair_budgets \
             WHERE attempt_id=$1 FOR UPDATE",
        )
        .bind(attempt_id)
        .fetch_one(&mut *tx)
        .await?;
        let prior_command: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT attempt_id,failure_fingerprint_sha256 \
             FROM completion_protocol_repair_reservations WHERE command_id=$1",
        )
        .bind(command_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((prior_attempt, prior_fingerprint)) = prior_command {
            if prior_attempt != attempt_id || prior_fingerprint != fingerprint {
                return Err(OrgIntelError::InvalidWork(
                    "completion repair command id was reused for a different reservation".into(),
                ));
            }
            let per_fingerprint =
                current_fingerprint_count(&mut tx, attempt_id, &fingerprint).await?;
            tx.commit().await?;
            return Ok((
                CompletionRepairDecision::AlreadyReserved,
                CompletionRepairBudget {
                    attempt_id,
                    repairs_total: total,
                    repairs_for_fingerprint: per_fingerprint,
                },
            ));
        }
        let per_fingerprint = current_fingerprint_count(&mut tx, attempt_id, &fingerprint).await?;
        if total >= max_total || per_fingerprint >= max_per_fingerprint {
            tx.commit().await?;
            return Ok((
                CompletionRepairDecision::LimitReached,
                CompletionRepairBudget {
                    attempt_id,
                    repairs_total: total,
                    repairs_for_fingerprint: per_fingerprint,
                },
            ));
        }
        sqlx::query(
            "INSERT INTO completion_protocol_repair_fingerprints \
             (attempt_id,failure_fingerprint_sha256,repairs) VALUES ($1,$2,1) \
             ON CONFLICT (attempt_id,failure_fingerprint_sha256) \
             DO UPDATE SET repairs=completion_protocol_repair_fingerprints.repairs+1,updated_at=now()",
        )
        .bind(attempt_id)
        .bind(&fingerprint)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE completion_protocol_repair_budgets SET repairs_total=repairs_total+1,updated_at=now() \
             WHERE attempt_id=$1",
        )
        .bind(attempt_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO completion_protocol_repair_reservations \
             (command_id,attempt_id,failure_fingerprint_sha256) VALUES ($1,$2,$3)",
        )
        .bind(command_id)
        .bind(attempt_id)
        .bind(&fingerprint)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok((
            CompletionRepairDecision::Admitted,
            CompletionRepairBudget {
                attempt_id,
                repairs_total: total + 1,
                repairs_for_fingerprint: per_fingerprint + 1,
            },
        ))
    }
}

async fn current_fingerprint_count(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    attempt_id: Uuid,
    fingerprint: &str,
) -> Result<i32> {
    Ok(sqlx::query_scalar(
        "SELECT repairs FROM completion_protocol_repair_fingerprints \
         WHERE attempt_id=$1 AND failure_fingerprint_sha256=$2",
    )
    .bind(attempt_id)
    .bind(fingerprint)
    .fetch_optional(&mut **tx)
    .await?
    .unwrap_or_default())
}

/// Stable protocol-failure fingerprint. Response text is hashed before it is
/// stored so ordinary OrgIntel rows never retain raw provider content.
pub fn completion_failure_fingerprint(raw_response: &str) -> String {
    format!("{:x}", Sha256::digest(raw_response.trim().as_bytes()))
}
