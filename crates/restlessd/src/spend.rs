//! The spend fuse (sprint 01 T2, relocated).
//!
//! This began as an embedded HTTP model gateway: provider key held host-side,
//! a purpose token minted per wake, requests proxied upstream so token usage
//! could be scraped off the response and charged against a per-company
//! ceiling. All of that existed to answer one question — *how much did that
//! turn cost?* — and the agent turned out to answer it directly: `omp` reports
//! tokens and dollars per turn on the ACP session stream.
//!
//! So the fuse moved to the session layer, where the daemon already knows
//! whose turn it is, and the proxy became dead weight: no agent has routed a
//! request through it since the runtime swap. What remains here is the part
//! that was always load-bearing — a crash-durable ledger of what each company
//! has spent, and the check that stops a company at its ceiling.
//!
//! The trade-off, recorded honestly: the ceiling is now checked per *turn*
//! rather than per *request*, so a single turn can overshoot. The watchdog
//! also checks it mid-turn against reported usage, which bounds the overshoot
//! to whatever one turn burns between ticks — cents, against a ceiling in
//! dollars.

use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use chrono::Utc;
use restless_model_gateway::{SpendCorrection, SpendCorrectionPreview, SpendRecord, SpendStore};
use uuid::Uuid;

use crate::{
    model_gateway::ModelBilling,
    runtime::{CompanyConfig, SpendCeiling},
};

/// The company's spend ledger. One per installation, shared by every company;
/// the store keys by company id.
///
/// Metered sessions also share a one-per-company in-process lane. It is not a
/// reservation or a scheduler: it has no durable state, queue, priority, or
/// retry policy. It merely prevents two charged ACP sessions from using the
/// same stale pre-turn remaining balance. A single active provider session can
/// still overshoot between its own usage reports.
#[derive(Clone)]
pub struct SpendLedger {
    store: Arc<SpendStore>,
    metered_turn_lanes: Arc<Mutex<HashMap<String, Arc<tokio::sync::Semaphore>>>>,
}

/// Writes turn costs into the ledger. Cloneable so supervised staff processes
/// can meter themselves without borrowing the daemon.
#[derive(Clone)]
pub struct TurnMeter {
    store: Arc<SpendStore>,
}

impl TurnMeter {
    /// A turn we cannot account for poisons the company fail-closed:
    /// unaccounted spend and unbounded spend are indistinguishable.
    pub fn record(
        &self,
        company: &str,
        actor: &str,
        model: &str,
        used: u64,
        cost_usd: Option<f64>,
    ) {
        let Some(cost_usd) = cost_usd else {
            tracing::error!(
                company,
                used,
                "agent reported usage without a cost; poisoning fail-closed"
            );
            self.store.poison(company);
            return;
        };
        let record = SpendRecord {
            request_id: Uuid::new_v4(),
            company_id: company.to_owned(),
            model: model.to_owned(),
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: used,
            cost_micro_usd: (cost_usd * 1_000_000.0).round().max(0.0) as u64,
            actor_id: actor.to_owned(),
            occurred_at: Utc::now(),
        };
        if self.store.record(&record).is_err() {
            self.store.poison(company);
            tracing::error!(
                company,
                "turn spend record failed; company poisoned fail-closed"
            );
        }
    }
}

impl SpendLedger {
    /// Open the ledger under `$RESTLESS_HOME/spend/`, rebuilding per-company
    /// totals from the spool. Failure to open is fatal: a daemon that cannot
    /// account does not run companies.
    pub fn open(root: &Path) -> Result<Self> {
        let dir = root.join("spend");
        if !dir.exists() {
            let mut builder = std::fs::DirBuilder::new();
            builder.recursive(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt as _;
                builder.mode(0o700);
            }
            builder
                .create(&dir)
                .with_context(|| format!("create {}", dir.display()))?;
        }
        // The spool used to live under gateway/spend/ when this was a proxy.
        // Carry it across rather than silently starting a company's accounting
        // from zero — the ceiling is enforced against this history.
        let legacy = root.join("gateway").join("spend").join("spend.jsonl");
        let current = dir.join("spend.jsonl");
        if legacy.exists() && !current.exists() {
            std::fs::rename(&legacy, &current)
                .with_context(|| format!("migrate spend spool from {}", legacy.display()))?;
            tracing::info!(from = %legacy.display(), "migrated spend spool out of the retired gateway directory");
        }
        let store =
            SpendStore::open(&dir).map_err(|error| anyhow::anyhow!("spend store: {error}"))?;
        Ok(Self {
            store: Arc::new(store),
            metered_turn_lanes: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Wait for this company's one charged-model session lane. Subscription
    /// routes have no authoritative charged cost and deliberately remain
    /// concurrent. The returned permit drops on cancellation, failure, or
    /// normal completion.
    pub async fn acquire_metered_turn(
        &self,
        company: &str,
        billing: ModelBilling,
    ) -> Option<tokio::sync::OwnedSemaphorePermit> {
        if billing == ModelBilling::Subscription {
            return None;
        }
        let lane = {
            let mut lanes = self
                .metered_turn_lanes
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            Arc::clone(
                lanes
                    .entry(company.to_string())
                    .or_insert_with(|| Arc::new(tokio::sync::Semaphore::new(1))),
            )
        };
        Some(
            lane.acquire_owned()
                .await
                .expect("metered turn lane is never closed"),
        )
    }

    /// What this company has spent so far, in USD. Shown to the agent so it can
    /// size its own ambition rather than discovering it is broke mid-turn.
    #[must_use]
    pub fn spent_usd(&self, company: &str) -> f64 {
        self.store.spent_micro_usd(company) as f64 / 1_000_000.0
    }

    /// Pre-turn check: has this company already spent its ceiling? Returns
    /// (spent, ceiling) in USD when it must not start.
    #[must_use]
    pub fn over_ceiling(&self, company: &CompanyConfig) -> Option<(f64, f64)> {
        let spent = self.store.spent_micro_usd(&company.name);
        let ceiling = company.spend_ceiling_usd.micro_usd();
        (spent >= ceiling).then(|| (spent as f64 / 1_000_000.0, ceiling as f64 / 1_000_000.0))
    }

    /// Remaining company envelope calculated in exact micro-USD. Convert to a
    /// display float only at the caller's presentation boundary.
    #[must_use]
    pub fn remaining_micro_usd(&self, company: &CompanyConfig) -> u64 {
        self.remaining_micro_usd_for(&company.name, company.spend_ceiling_usd)
    }

    #[must_use]
    pub fn remaining_usd(&self, company: &CompanyConfig) -> f64 {
        self.remaining_micro_usd(company) as f64 / 1_000_000.0
    }

    #[must_use]
    pub fn remaining_micro_usd_for(&self, company: &str, ceiling: SpendCeiling) -> u64 {
        ceiling
            .micro_usd()
            .saturating_sub(self.store.spent_micro_usd(company))
    }

    /// A cheap cloneable handle for turns that outlive the borrow — staff run
    /// in spawned tasks but spend the same budget.
    #[must_use]
    pub fn meter(&self) -> TurnMeter {
        TurnMeter {
            store: Arc::clone(&self.store),
        }
    }

    /// Clear a fail-closed poison once an operator has looked. The spool keeps
    /// both records, so the incident stays legible after recovery.
    pub fn clear_poison(&self, company: &str) -> Result<()> {
        self.store
            .clear_poison(company)
            .map_err(|error| anyhow::anyhow!("clear poison: {error}"))
    }

    /// S04-T9. Spend by `(actor, model)` for one company.
    #[must_use]
    pub fn breakdown(&self, company: &str) -> Vec<(String, String, f64)> {
        self.store
            .breakdown_micro_usd(company)
            .into_iter()
            .map(|(actor, model, micro)| (actor, model, micro as f64 / 1_000_000.0))
            .collect()
    }

    /// Owner recovery preview: validate exact duplicate request ids and show
    /// the post-correction total without appending anything.
    pub fn preview_correction(
        &self,
        correction_id: Uuid,
        company: &str,
        request_ids: &[Uuid],
        delta_micro_usd: i64,
        reason: &str,
        corrected_by: &str,
    ) -> Result<SpendCorrectionPreview> {
        self.store
            .preview_correction(
                correction_id,
                company,
                request_ids,
                delta_micro_usd,
                reason,
                corrected_by,
            )
            .map_err(|error| anyhow::anyhow!("preview spend correction: {error}"))
    }

    /// Append a subtractive, owner-attributed correction. This does not alter
    /// the configured ceiling or create any new authority.
    pub fn correct(
        &self,
        correction_id: Uuid,
        company: &str,
        request_ids: &[Uuid],
        delta_micro_usd: i64,
        reason: &str,
        corrected_by: &str,
    ) -> Result<(SpendCorrection, SpendCorrectionPreview)> {
        self.store
            .correct(
                correction_id,
                company,
                request_ids,
                delta_micro_usd,
                reason,
                corrected_by,
            )
            .map_err(|error| anyhow::anyhow!("apply spend correction: {error}"))
    }

    /// S04-T1. Drop a destroyed company's accounted spend.
    pub fn forget(&self, company: &str) -> Result<()> {
        self.store
            .forget(company)
            .map_err(|error| anyhow::anyhow!("forget spend: {error}"))
    }

    /// Record what one turn cost, from the agent's own ACP usage report.
    pub fn record_turn(
        &self,
        company: &str,
        actor: &str,
        model: &str,
        used: u64,
        cost_usd: Option<f64>,
    ) {
        self.meter().record(company, actor, model, used, cost_usd);
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use super::*;

    #[tokio::test]
    async fn charged_turns_share_one_company_lane_without_blocking_other_companies() {
        let root = std::env::temp_dir().join(format!(
            "restless-metered-turn-lane-test-{}",
            uuid::Uuid::new_v4()
        ));
        assert!(!root.exists(), "test spend root must be fresh");
        let ledger = Arc::new(SpendLedger::open(&root).unwrap());

        let first = ledger
            .acquire_metered_turn("same-company", ModelBilling::MeteredApi)
            .await
            .expect("metered routes acquire a lane");
        assert!(
            ledger
                .acquire_metered_turn("same-company", ModelBilling::Subscription)
                .await
                .is_none(),
            "subscription routes do not take the charged lane"
        );

        let another_company = tokio::time::timeout(
            Duration::from_millis(100),
            ledger.acquire_metered_turn("other-company", ModelBilling::MeteredApi),
        )
        .await
        .expect("a different company is not blocked")
        .expect("metered routes acquire a lane");

        let waiting_ledger = Arc::clone(&ledger);
        let mut waiting_turn = tokio::spawn(async move {
            waiting_ledger
                .acquire_metered_turn("same-company", ModelBilling::MeteredApi)
                .await
                .expect("metered routes acquire a lane")
        });
        assert!(
            tokio::time::timeout(Duration::from_millis(50), &mut waiting_turn)
                .await
                .is_err(),
            "a second charged turn waits for the active company turn"
        );

        drop(first);
        let second = tokio::time::timeout(Duration::from_millis(100), waiting_turn)
            .await
            .expect("the waiting turn proceeds after release")
            .expect("the waiting task did not fail");

        drop(second);
        drop(another_company);
        drop(ledger);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
