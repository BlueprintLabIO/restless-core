//! The spend ledger and host-side model admission fuse.
//!
//! Provider credentials remain behind OMP's loopback gateway. The Runtime sees
//! a scoped relay capability, and the relay records terminal charged usage here
//! in exact micro-USD. ACP usage remains useful telemetry and an in-session
//! early-stop hint, but it is intentionally not a second charging path.

use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use chrono::Utc;
use restless_model_gateway::{
    CompanySpendState, SpendCorrection, SpendCorrectionPreview, SpendRecord, SpendStore,
};
use uuid::Uuid;

use crate::{
    model_gateway::ModelBilling,
    runtime::{CompanyConfig, SpendCeiling},
};

/// The company's spend ledger. One per installation, shared by every company;
/// the store keys by company id.
///
/// Metered sessions share a small per-company in-process admission pool. Its
/// reservations are deliberately ephemeral: Work scheduling stays in
/// OrgIntel and exact terminal charges stay in the durable store. The pool
/// only prevents concurrent charged ACP sessions from each treating the same
/// uncommitted balance as entirely theirs. A provider session can still
/// overshoot its own reservation between usage reports, so this remains the
/// Authority Plane's coarse outer fuse rather than perfect per-task costing.
#[derive(Clone)]
pub struct SpendLedger {
    /// `$RESTLESS_HOME`. Each cell's ledger lives under `cells/<company>/spend/`.
    root: Arc<Path2>,
    /// One store per cell, opened on first use. Cross-layer contract §1.4: a
    /// store that spans companies is never the source of truth, so the owner
    /// total is a sum over these rather than a shared file with a company
    /// column.
    stores: Arc<Mutex<HashMap<String, Arc<SpendStore>>>>,
    metered_turn_lanes: Arc<Mutex<HashMap<String, Arc<MeteredTurnLane>>>>,
}

type Path2 = std::path::PathBuf;

/// The first product workload that needed same-company parallel model work
/// was EXP-05's four independently closing Staff units. Keep the outer limit
/// small and explicit; provider capacity above this is still unproved.
const METERED_TURN_CONCURRENCY: usize = 4;

struct MeteredTurnLane {
    permits: Arc<tokio::sync::Semaphore>,
    reserved_micro_usd: Mutex<u64>,
}

impl MeteredTurnLane {
    fn new() -> Self {
        Self {
            permits: Arc::new(tokio::sync::Semaphore::new(METERED_TURN_CONCURRENCY)),
            reserved_micro_usd: Mutex::new(0),
        }
    }
}

/// An ephemeral share of the company's currently uncommitted model envelope.
/// Dropping it returns unused headroom and releases one concurrency slot. The
/// provider's exact terminal charge is recorded separately before callers
/// drop this guard.
pub struct MeteredTurnPermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
    lane: Arc<MeteredTurnLane>,
    reserved_micro_usd: u64,
}

impl MeteredTurnPermit {
    #[must_use]
    pub fn allowance_micro_usd(&self) -> u64 {
        self.reserved_micro_usd
    }

    #[must_use]
    pub fn allowance_usd(&self) -> f64 {
        self.reserved_micro_usd as f64 / 1_000_000.0
    }
}

impl Drop for MeteredTurnPermit {
    fn drop(&mut self) {
        let mut reserved = self
            .lane
            .reserved_micro_usd
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *reserved = reserved.saturating_sub(self.reserved_micro_usd);
    }
}

/// Writes turn costs into the ledger. Cloneable so supervised staff processes
/// can meter themselves without borrowing the daemon. Holds the ledger rather
/// than one store, because each cell keeps its own.
#[derive(Clone)]
pub struct TurnMeter {
    ledger: SpendLedger,
}

/// The one authoritative answer to whether a charged model turn may begin.
///
/// `MeteringUnknown` is deliberately neither an exhausted budget nor a zero
/// balance. The ledger has preserved every exact charge it knows, but a prior
/// provider stream lacked an exact terminal charge. Charged admission pauses
/// until that discrepancy is reconciled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelBudgetState {
    Available {
        accounted_micro_usd: u64,
        ceiling_micro_usd: u64,
        remaining_micro_usd: u64,
    },
    Exhausted {
        accounted_micro_usd: u64,
        ceiling_micro_usd: u64,
    },
    MeteringUnknown {
        accounted_micro_usd: u64,
        ceiling_micro_usd: u64,
    },
}

impl ModelBudgetState {
    #[must_use]
    pub fn accounted_micro_usd(self) -> u64 {
        match self {
            Self::Available {
                accounted_micro_usd,
                ..
            }
            | Self::Exhausted {
                accounted_micro_usd,
                ..
            }
            | Self::MeteringUnknown {
                accounted_micro_usd,
                ..
            } => accounted_micro_usd,
        }
    }

    #[must_use]
    pub fn ceiling_micro_usd(self) -> u64 {
        match self {
            Self::Available {
                ceiling_micro_usd, ..
            }
            | Self::Exhausted {
                ceiling_micro_usd, ..
            }
            | Self::MeteringUnknown {
                ceiling_micro_usd, ..
            } => ceiling_micro_usd,
        }
    }

    #[must_use]
    pub fn remaining_micro_usd(self) -> Option<u64> {
        match self {
            Self::Available {
                remaining_micro_usd,
                ..
            } => Some(remaining_micro_usd),
            Self::Exhausted { .. } | Self::MeteringUnknown { .. } => None,
        }
    }

    #[must_use]
    pub fn is_available(self) -> bool {
        matches!(self, Self::Available { .. })
    }

    #[must_use]
    pub fn owner_message(self, company: &str) -> String {
        let accounted = self.accounted_micro_usd() as f64 / 1_000_000.0;
        let ceiling = self.ceiling_micro_usd() as f64 / 1_000_000.0;
        match self {
            Self::Available { .. } => format!(
                "{company} has ${accounted:.2} accounted against its ${ceiling:.2} ceiling"
            ),
            Self::Exhausted { .. } => format!(
                "{company} has spent ${accounted:.2} of its ${ceiling:.2} ceiling; the owner must raise it before charged work continues"
            ),
            Self::MeteringUnknown { .. } => format!(
                "{company} has ${accounted:.2} exactly accounted, but a provider stream ended without an exact charge; charged work is paused until model metering is reconciled"
            ),
        }
    }
}

impl TurnMeter {
    /// The host-side relay has normalised the provider charge upward into the
    /// ledger's micro-USD unit and attributed it to one supervised session.
    /// Preserve that integer charge through the spool rather than converting a
    /// provider decimal back through `f64`.
    pub fn record_exact(
        &self,
        company: &str,
        actor: &str,
        session: &str,
        model: &str,
        used: u64,
        cost_micro_usd: u64,
    ) {
        let record = SpendRecord {
            request_id: Uuid::new_v4(),
            company_id: company.to_owned(),
            model: model.to_owned(),
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: used,
            cost_micro_usd,
            actor_id: actor.to_owned(),
            session_id: session.to_owned(),
            occurred_at: Utc::now(),
        };
        let Ok(store) = self.ledger.store(company) else {
            tracing::error!(
                company,
                "cell spend ledger unavailable; turn charge not recorded"
            );
            return;
        };
        if store.record(&record).is_err() {
            store.poison(company);
            tracing::error!(
                company,
                "turn spend record failed; company poisoned fail-closed"
            );
        }
    }

    pub fn poison(&self, company: &str) {
        if let Ok(store) = self.ledger.store(company) {
            store.poison(company);
        }
    }
}


/// A ledger directory is owner-private: it records money.
fn create_private_dir(dir: &Path) -> Result<()> {
    if dir.exists() {
        return Ok(());
    }
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt as _;
        builder.mode(0o700);
    }
    builder
        .create(dir)
        .with_context(|| format!("create {}", dir.display()))
}

/// Copy this company's rows out of the installation-wide spool into its own
/// cell ledger, once. Non-destructive: the shared spool is left in place so an
/// operator can verify the split before removing it. The ceiling is enforced
/// against accumulated history, so starting a migrated company from zero would
/// silently raise its budget.
fn extract_legacy_company_spool(root: &Path, cell_dir: &Path, company: &str) -> Result<()> {
    let cell_spool = cell_dir.join("spend.jsonl");
    if cell_spool.exists() {
        return Ok(());
    }
    let shared = root.join("spend").join("spend.jsonl");
    let Ok(text) = std::fs::read_to_string(&shared) else {
        return Ok(());
    };
    let mut mine = String::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            // A torn or corrupt shared line is the shared spool's problem;
            // SpendStore::open repairs its own tail. Never guess ownership.
            continue;
        };
        if value["companyId"].as_str() == Some(company) {
            mine.push_str(line);
            mine.push('\n');
        }
    }
    if mine.is_empty() {
        return Ok(());
    }
    std::fs::write(&cell_spool, &mine)
        .with_context(|| format!("write {}", cell_spool.display()))?;
    tracing::info!(
        company,
        rows = mine.lines().count(),
        "extracted this company's spend history into its own cell ledger"
    );
    Ok(())
}

impl SpendLedger {
    /// Open the ledger under `$RESTLESS_HOME/spend/`, rebuilding per-company
    /// totals from the spool. Failure to open is fatal: a daemon that cannot
    /// account does not run companies.
    pub fn open(root: &Path) -> Result<Self> {
        // The spool used to live under gateway/spend/ when this was a proxy.
        // Normalise that to the installation spool first so per-cell extraction
        // below has exactly one legacy source to read.
        let dir = root.join("spend");
        let legacy = root.join("gateway").join("spend").join("spend.jsonl");
        let current = dir.join("spend.jsonl");
        if legacy.exists() && !current.exists() {
            create_private_dir(&dir)?;
            std::fs::rename(&legacy, &current)
                .with_context(|| format!("migrate spend spool from {}", legacy.display()))?;
            tracing::info!(from = %legacy.display(), "migrated spend spool out of the retired gateway directory");
        }
        Ok(Self {
            root: Arc::new(root.to_path_buf()),
            stores: Arc::new(Mutex::new(HashMap::new())),
            metered_turn_lanes: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// This cell's ledger, opened on first use. Failure is not silently
    /// tolerated by callers that record money: a cell that cannot account does
    /// not get charged turns.
    fn store(&self, company: &str) -> Result<Arc<SpendStore>> {
        if let Some(store) = self
            .stores
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(company)
        {
            return Ok(Arc::clone(store));
        }
        let dir = self.root.join("cells").join(company).join("spend");
        create_private_dir(&dir)?;
        extract_legacy_company_spool(&self.root, &dir, company)?;
        let store = Arc::new(
            SpendStore::open(&dir).map_err(|error| anyhow::anyhow!("cell spend store: {error}"))?,
        );
        self.stores
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(company.to_string(), Arc::clone(&store));
        Ok(store)
    }

    /// Admit one of at most four same-company charged sessions and reserve it
    /// an equal share of the uncommitted envelope. Sequential callers retain
    /// all four slots over time; concurrent callers cannot each inherit the
    /// same full remaining balance. Subscription routes have no authoritative
    /// charged cost and deliberately bypass this pool.
    pub async fn acquire_metered_turn(
        &self,
        company: &str,
        billing: ModelBilling,
        ceiling: SpendCeiling,
    ) -> Option<MeteredTurnPermit> {
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
                    .or_insert_with(|| Arc::new(MeteredTurnLane::new())),
            )
        };
        let permit = Arc::clone(&lane)
            .permits
            .clone()
            .acquire_owned()
            .await
            .expect("metered turn lane is never closed");
        let remaining = self
            .budget_state_for(company, ceiling)
            .remaining_micro_usd()
            .unwrap_or_default();
        let reserved_micro_usd = {
            let mut reserved = lane
                .reserved_micro_usd
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let uncommitted = remaining.saturating_sub(*reserved);
            // Each slot may hold at most one quarter of the currently
            // remaining envelope. Cap again by uncommitted headroom so races
            // between concurrent acquirers cannot duplicate a share (for
            // example 8 -> 2+2+2+2 regardless of lock acquisition order).
            let slots = METERED_TURN_CONCURRENCY as u64;
            let fair_share = remaining.saturating_add(slots.saturating_sub(1)) / slots;
            let share = fair_share.min(uncommitted);
            *reserved = reserved.saturating_add(share);
            share
        };
        Some(MeteredTurnPermit {
            _permit: permit,
            lane,
            reserved_micro_usd,
        })
    }

    /// What this company has spent so far, in USD. Shown to the agent so it can
    /// size its own ambition rather than discovering it is broke mid-turn.
    #[must_use]
    pub fn spent_usd(&self, company: &str) -> f64 {
        match self.store(company) {
            Ok(store) => store.accounted_micro_usd(company) as f64 / 1_000_000.0,
            Err(_) => 0.0,
        }
    }

    /// Classify charged-model admission from exact accounting and an explicit
    /// metering-certainty state. Unknown is not converted into arithmetic.
    #[must_use]
    pub fn budget_state(&self, company: &CompanyConfig) -> ModelBudgetState {
        self.budget_state_for(&company.name, company.spend_ceiling_usd)
    }

    #[must_use]
    pub fn budget_state_for(&self, company: &str, ceiling: SpendCeiling) -> ModelBudgetState {
        let ceiling_micro_usd = ceiling.micro_usd();
        // A cell whose ledger cannot be opened must not be treated as having
        // spent nothing: that would silently grant it a full budget. Refuse
        // charged admission until accounting is available again.
        let Ok(store) = self.store(company) else {
            return ModelBudgetState::MeteringUnknown {
                accounted_micro_usd: 0,
                ceiling_micro_usd,
            };
        };
        match store.company_state(company) {
            CompanySpendState::MeteringUnknown {
                accounted_micro_usd,
            } => ModelBudgetState::MeteringUnknown {
                accounted_micro_usd,
                ceiling_micro_usd,
            },
            CompanySpendState::Accounted {
                accounted_micro_usd,
            } if accounted_micro_usd >= ceiling_micro_usd => ModelBudgetState::Exhausted {
                accounted_micro_usd,
                ceiling_micro_usd,
            },
            CompanySpendState::Accounted {
                accounted_micro_usd,
            } => ModelBudgetState::Available {
                accounted_micro_usd,
                ceiling_micro_usd,
                remaining_micro_usd: ceiling_micro_usd.saturating_sub(accounted_micro_usd),
            },
        }
    }

    /// A cheap cloneable handle for relay streams that outlive the daemon
    /// borrow while writing to the same company ledger.
    #[must_use]
    pub fn meter(&self) -> TurnMeter {
        TurnMeter {
            ledger: self.clone(),
        }
    }

    /// Clear a fail-closed poison once an operator has looked. The spool keeps
    /// both records, so the incident stays legible after recovery.
    pub fn clear_poison(&self, company: &str) -> Result<()> {
        self.store(company)?
            .clear_poison(company)
            .map_err(|error| anyhow::anyhow!("clear poison: {error}"))
    }

    /// S04-T9. Spend by `(actor, model)` for one company.
    #[must_use]
    pub fn breakdown(&self, company: &str) -> Vec<(String, String, f64)> {
        let Ok(store) = self.store(company) else {
            return Vec::new();
        };
        store
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
        self.store(company)?
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
        self.store(company)?
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
        self.store(company)?
            .forget(company)
            .map_err(|error| anyhow::anyhow!("forget spend: {error}"))?;
        // A destroyed company's ledger goes with it; leaving the handle cached
        // would serve a store for a cell that no longer exists.
        self.stores
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(company);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use super::*;

    /// Cross-layer contract §1.4: no shared store with a company column. Each
    /// cell's ledger holds only its own rows, and a company migrated off the
    /// installation-wide spool keeps its accumulated history — starting it
    /// from zero would silently grant it a fresh budget.
    #[test]
    fn each_cell_ledger_holds_only_its_own_history() {
        let root = std::env::temp_dir().join(format!("restless-cell-spend-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("spend")).unwrap();
        let row = |company: &str, cost: u64, session: &str| {
            serde_json::json!({
                "requestId": Uuid::new_v4(),
                "companyId": company,
                "model": "zai/glm-5.3",
                "inputTokens": 0,
                "outputTokens": 0,
                "totalTokens": 10,
                "costMicroUsd": cost,
                "actorId": "exec",
                "sessionId": session,
                "occurredAt": Utc::now(),
            })
            .to_string()
        };
        // One installation-wide spool holding two companies — the legacy shape.
        std::fs::write(
            root.join("spend").join("spend.jsonl"),
            format!(
                "{}\n{}\n{}\n",
                row("alpha_test", 7, "s1"),
                row("beta_test", 500, "s2"),
                row("alpha_test", 3, "s3")
            ),
        )
        .unwrap();

        let ledger = SpendLedger::open(&root).unwrap();
        // alpha keeps exactly its own accumulated spend, not the shared total.
        assert_eq!(ledger.spent_usd("alpha_test"), 10.0 / 1_000_000.0);
        assert_eq!(ledger.spent_usd("beta_test"), 500.0 / 1_000_000.0);

        let alpha = std::fs::read_to_string(root.join("cells/alpha_test/spend/spend.jsonl")).unwrap();
        assert_eq!(alpha.lines().count(), 2, "alpha takes only its own rows");
        assert!(
            !alpha.contains("beta_test"),
            "a cell ledger must never contain another company's rows: {alpha}"
        );
        // Non-destructive: the shared spool stays until an operator removes it.
        assert!(root.join("spend").join("spend.jsonl").exists());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[tokio::test]
    async fn charged_turns_reserve_one_envelope_across_four_company_slots() {
        let root = std::env::temp_dir().join(format!(
            "restless-metered-turn-lane-test-{}",
            uuid::Uuid::new_v4()
        ));
        assert!(!root.exists(), "test spend root must be fresh");
        let ledger = Arc::new(SpendLedger::open(&root).unwrap());
        let ceiling = SpendCeiling::from_micro_usd(8_000_000);

        let first = ledger
            .acquire_metered_turn("same-company", ModelBilling::MeteredApi, ceiling)
            .await
            .expect("metered routes acquire a lane");
        assert_eq!(first.allowance_micro_usd(), 2_000_000);
        assert!(
            ledger
                .acquire_metered_turn("same-company", ModelBilling::Subscription, ceiling)
                .await
                .is_none(),
            "subscription routes do not take the charged lane"
        );

        let another_company = tokio::time::timeout(
            Duration::from_millis(100),
            ledger.acquire_metered_turn("other-company", ModelBilling::MeteredApi, ceiling),
        )
        .await
        .expect("a different company is not blocked")
        .expect("metered routes acquire a lane");

        let second = ledger
            .acquire_metered_turn("same-company", ModelBilling::MeteredApi, ceiling)
            .await
            .expect("second metered route acquires a lane");
        let third = ledger
            .acquire_metered_turn("same-company", ModelBilling::MeteredApi, ceiling)
            .await
            .expect("third metered route acquires a lane");
        let fourth = ledger
            .acquire_metered_turn("same-company", ModelBilling::MeteredApi, ceiling)
            .await
            .expect("fourth metered route acquires a lane");
        assert_eq!(
            [
                first.allowance_micro_usd(),
                second.allowance_micro_usd(),
                third.allowance_micro_usd(),
                fourth.allowance_micro_usd(),
            ]
            .into_iter()
            .sum::<u64>(),
            ceiling.micro_usd(),
            "concurrent reservations never duplicate company headroom"
        );

        let waiting_ledger = Arc::clone(&ledger);
        let mut waiting_turn = tokio::spawn(async move {
            waiting_ledger
                .acquire_metered_turn("same-company", ModelBilling::MeteredApi, ceiling)
                .await
                .expect("metered routes acquire a lane")
        });
        assert!(
            tokio::time::timeout(Duration::from_millis(50), &mut waiting_turn)
                .await
                .is_err(),
            "a fifth charged turn waits for an active company slot"
        );

        drop(first);
        let replacement = tokio::time::timeout(Duration::from_millis(100), waiting_turn)
            .await
            .expect("the waiting turn proceeds after release")
            .expect("the waiting task did not fail");
        assert_eq!(
            replacement.allowance_micro_usd(),
            2_000_000,
            "an unused reservation returns to the next admitted turn"
        );

        drop(second);
        drop(third);
        drop(fourth);
        drop(replacement);
        drop(another_company);
        drop(ledger);
        std::fs::remove_dir_all(&root).unwrap();
    }
}
