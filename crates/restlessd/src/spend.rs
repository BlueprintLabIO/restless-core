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

use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;
use restless_model_gateway::{SpendRecord, SpendStore};
use uuid::Uuid;

use crate::runtime::CompanyConfig;

/// The company's spend ledger. One per installation, shared by every company;
/// the store keys by company id.
pub struct SpendLedger {
    store: std::sync::Arc<SpendStore>,
}

/// Writes turn costs into the ledger. Cloneable so supervised staff processes
/// can meter themselves without borrowing the daemon.
#[derive(Clone)]
pub struct TurnMeter {
    store: std::sync::Arc<SpendStore>,
}

impl TurnMeter {
    /// A turn we cannot account for poisons the company fail-closed:
    /// unaccounted spend and unbounded spend are indistinguishable.
    pub fn record(&self, company: &str, model: &str, used: u64, cost_usd: Option<f64>) {
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
            occurred_at: Utc::now(),
        };
        if self.store.record(&record).is_err() {
            self.store.poison(company);
            tracing::error!(company, "turn spend record failed; company poisoned fail-closed");
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
            builder.create(&dir).with_context(|| format!("create {}", dir.display()))?;
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
        let store = SpendStore::open(&dir).map_err(|error| anyhow::anyhow!("spend store: {error}"))?;
        Ok(Self { store: std::sync::Arc::new(store) })
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
        let ceiling = (company.spend_ceiling_usd * 1_000_000.0).round().max(0.0) as u64;
        (spent >= ceiling).then(|| (spent as f64 / 1_000_000.0, ceiling as f64 / 1_000_000.0))
    }

    /// A cheap cloneable handle for turns that outlive the borrow — staff run
    /// in spawned tasks but spend the same budget.
    #[must_use]
    pub fn meter(&self) -> TurnMeter {
        TurnMeter { store: std::sync::Arc::clone(&self.store) }
    }

    /// Clear a fail-closed poison once an operator has looked. The spool keeps
    /// both records, so the incident stays legible after recovery.
    pub fn clear_poison(&self, company: &str) -> Result<()> {
        self.store
            .clear_poison(company)
            .map_err(|error| anyhow::anyhow!("clear poison: {error}"))
    }

    /// Record what one turn cost, from the agent's own ACP usage report.
    pub fn record_turn(&self, company: &str, model: &str, used: u64, cost_usd: Option<f64>) {
        self.meter().record(company, model, used, cost_usd);
    }
}
