//! Per-company dollar spend accounting — the fuse (sprint 01 T2).
//!
//! Append fsync'd to a JSONL spool, per-company totals rebuilt from the spool
//! on boot, and never fails open: an unreadable spool refuses service rather
//! than guessing. Costs now arrive from the agent's own per-turn usage report
//! rather than being scraped from a proxied HTTP response, so the rate table
//! and usage parser below are retained only for records written by that
//! retired path.

use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::Write as _,
    path::PathBuf,
    sync::Mutex,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{GatewayError, GatewayResult};

/// USD per million tokens, stored as micro-dollars to stay integer-exact.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelRate {
    pub input_usd_per_mtok: f64,
    pub output_usd_per_mtok: f64,
}

impl ModelRate {
    /// Cost of one call in micro-USD (1 USD = 1_000_000 micro).
    #[must_use]
    pub fn cost_micro_usd(&self, input_tokens: u64, output_tokens: u64) -> u64 {
        let input = (input_tokens as f64 / 1_000_000.0) * self.input_usd_per_mtok;
        let output = (output_tokens as f64 / 1_000_000.0) * self.output_usd_per_mtok;
        ((input + output) * 1_000_000.0).round() as u64
    }
}

/// One accounted model call. Appended to the spool; the boot rebuild reads
/// exactly these lines back.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpendRecord {
    pub request_id: Uuid,
    pub company_id: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// Total tokens for the turn. The ACP `UsageUpdate` the agent reports is
    /// a single total, not an input/output split — so turn-metered records
    /// carry this and leave the split at zero. Defaulted so the 91 records
    /// written by the HTTP-proxy path still rebuild on boot.
    #[serde(default)]
    pub total_tokens: u64,
    pub cost_micro_usd: u64,
    /// S04-T9. Which actor's turn this was. Defaulted so the records written
    /// before this field existed still rebuild on boot — the spool is
    /// append-only history, and a schema change must not invalidate it.
    ///
    /// The **role** is deliberately not stored here: OrgIntel owns role
    /// (`cross-layer §3.1`) and is the single writer of it. Cost is joined to
    /// role at read time, so a renamed role does not leave the ledger
    /// disagreeing with the org.
    #[serde(default)]
    pub actor_id: String,
    /// Supervised Runtime session that produced this record. Older ACP-era
    /// records predate host-side admission and remain explicitly empty.
    #[serde(default)]
    pub session_id: String,
    pub occurred_at: DateTime<Utc>,
}

/// Model field marking a fail-closed poison, and its cancellation. Both are
/// records rather than mutations: the spool stays append-only and the incident
/// stays visible after recovery.
pub const POISON_MARKER: &str = "poison-marker";
pub const POISON_CLEARED: &str = "poison-cleared";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum SpendEntryType {
    #[serde(rename = "spendCorrection")]
    SpendCorrection,
}

/// An append-only correction to exact, already-recorded duplicate requests.
/// The original [`SpendRecord`] lines are never edited or removed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpendCorrection {
    #[serde(rename = "type")]
    entry_type: SpendEntryType,
    pub correction_id: Uuid,
    pub company_id: String,
    pub request_ids: Vec<Uuid>,
    /// Signed for audit clarity, but corrections are strictly subtractive.
    pub delta_micro_usd: i64,
    pub reason: String,
    pub corrected_by: String,
    pub occurred_at: DateTime<Utc>,
}

/// What a correction would do. Previewing this value never writes the spool.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpendCorrectionPreview {
    pub correction_id: Uuid,
    pub company_id: String,
    pub request_ids: Vec<Uuid>,
    pub delta_micro_usd: i64,
    pub current_total_micro_usd: u64,
    pub post_correction_total_micro_usd: u64,
    pub reason: String,
    pub corrected_by: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SpendEntry {
    Correction(SpendCorrection),
    Record(SpendRecord),
}

impl SpendEntry {
    fn company_id(&self) -> &str {
        match self {
            Self::Correction(correction) => &correction.company_id,
            Self::Record(record) => &record.company_id,
        }
    }
}

#[derive(Clone, Default)]
struct SpendState {
    accounted_totals: HashMap<String, u64>,
    poisons: HashSet<String>,
    records: HashMap<Uuid, SpendRecord>,
    corrections: HashMap<Uuid, SpendCorrection>,
    corrected_requests: HashSet<Uuid>,
    breakdowns: HashMap<String, HashMap<(String, String), u64>>,
}

struct CorrectionPlan {
    preview: SpendCorrectionPreview,
    records: Vec<SpendRecord>,
}

impl SpendState {
    fn apply_record(&mut self, record: SpendRecord) -> GatewayResult<()> {
        if record.request_id == Uuid::nil() {
            match record.model.as_str() {
                POISON_MARKER => {
                    self.poisons.insert(record.company_id);
                    return Ok(());
                }
                POISON_CLEARED => {
                    self.poisons.remove(&record.company_id);
                    return Ok(());
                }
                _ => {
                    return Err(GatewayError::Configuration(
                        "nil spend request id is reserved for poison audit markers".into(),
                    ))
                }
            }
        }
        if self.records.contains_key(&record.request_id) {
            return Err(GatewayError::Configuration(format!(
                "duplicate spend request id {}",
                record.request_id
            )));
        }
        let current = self
            .accounted_totals
            .get(&record.company_id)
            .copied()
            .unwrap_or_default();
        let next = current.checked_add(record.cost_micro_usd).ok_or_else(|| {
            GatewayError::Configuration(format!(
                "spend total overflow for company {:?}",
                record.company_id
            ))
        })?;
        let actor = if record.actor_id.is_empty() {
            "unattributed".to_string()
        } else {
            record.actor_id.clone()
        };
        let key = (actor, record.model.clone());
        let breakdown = self
            .breakdowns
            .entry(record.company_id.clone())
            .or_default();
        let current_key = breakdown.get(&key).copied().unwrap_or_default();
        let next_key = current_key
            .checked_add(record.cost_micro_usd)
            .ok_or_else(|| {
                GatewayError::Configuration(format!(
                    "spend breakdown overflow for company {:?}",
                    record.company_id
                ))
            })?;
        self.accounted_totals
            .insert(record.company_id.clone(), next);
        breakdown.insert(key, next_key);
        self.records.insert(record.request_id, record);
        Ok(())
    }

    fn correction_plan(&self, correction: &SpendCorrection) -> GatewayResult<CorrectionPlan> {
        if correction.correction_id == Uuid::nil() {
            return Err(GatewayError::InvalidRequest(
                "correction id must be a non-nil stable UUID".into(),
            ));
        }
        if self.corrections.contains_key(&correction.correction_id) {
            return Err(GatewayError::InvalidRequest(format!(
                "correction id {} is already applied",
                correction.correction_id
            )));
        }
        if correction.company_id.trim().is_empty()
            || correction.reason.trim().is_empty()
            || correction.corrected_by.trim().is_empty()
        {
            return Err(GatewayError::InvalidRequest(
                "correction needs company, reason and correcting principal".into(),
            ));
        }
        if correction.delta_micro_usd >= 0 {
            return Err(GatewayError::InvalidRequest(
                "a spend correction must have a negative micro-USD delta".into(),
            ));
        }
        if correction.request_ids.is_empty() {
            return Err(GatewayError::InvalidRequest(
                "a spend correction must reference at least one exact request id".into(),
            ));
        }

        let mut unique = HashSet::new();
        let mut records = Vec::with_capacity(correction.request_ids.len());
        let mut referenced_total = 0_u64;
        for request_id in &correction.request_ids {
            if !unique.insert(*request_id) {
                return Err(GatewayError::InvalidRequest(format!(
                    "request id {request_id} is duplicated within the correction"
                )));
            }
            let record = self.records.get(request_id).ok_or_else(|| {
                GatewayError::InvalidRequest(format!(
                    "correction references unknown request id {request_id}"
                ))
            })?;
            if record.company_id != correction.company_id {
                return Err(GatewayError::InvalidRequest(format!(
                    "request id {request_id} belongs to company {:?}, not {:?}",
                    record.company_id, correction.company_id
                )));
            }
            if self.corrected_requests.contains(request_id) {
                return Err(GatewayError::InvalidRequest(format!(
                    "request id {request_id} was already corrected"
                )));
            }
            referenced_total = referenced_total
                .checked_add(record.cost_micro_usd)
                .ok_or_else(|| {
                    GatewayError::InvalidRequest("referenced spend total overflows u64".into())
                })?;
            records.push(record.clone());
        }

        let subtraction = correction.delta_micro_usd.unsigned_abs();
        if subtraction > referenced_total {
            return Err(GatewayError::InvalidRequest(format!(
                "over-correction: requested {subtraction} micro-USD but referenced records total {referenced_total}"
            )));
        }
        if subtraction != referenced_total {
            return Err(GatewayError::InvalidRequest(format!(
                "correction must remove the exact referenced duplicate records: delta is -{subtraction}, referenced total is {referenced_total} micro-USD"
            )));
        }
        let current_total = self
            .accounted_totals
            .get(&correction.company_id)
            .copied()
            .unwrap_or_default();
        let post_correction_total = current_total.checked_sub(subtraction).ok_or_else(|| {
            GatewayError::InvalidRequest(format!(
                "over-correction: company total is {current_total} micro-USD, subtraction is {subtraction}"
            ))
        })?;
        Ok(CorrectionPlan {
            preview: SpendCorrectionPreview {
                correction_id: correction.correction_id,
                company_id: correction.company_id.clone(),
                request_ids: correction.request_ids.clone(),
                delta_micro_usd: correction.delta_micro_usd,
                current_total_micro_usd: current_total,
                post_correction_total_micro_usd: post_correction_total,
                reason: correction.reason.clone(),
                corrected_by: correction.corrected_by.clone(),
            },
            records,
        })
    }

    fn apply_correction(
        &mut self,
        correction: SpendCorrection,
        plan: CorrectionPlan,
    ) -> GatewayResult<()> {
        self.accounted_totals.insert(
            correction.company_id.clone(),
            plan.preview.post_correction_total_micro_usd,
        );
        let breakdown = self
            .breakdowns
            .entry(correction.company_id.clone())
            .or_default();
        for record in &plan.records {
            let actor = if record.actor_id.is_empty() {
                "unattributed".to_string()
            } else {
                record.actor_id.clone()
            };
            let key = (actor, record.model.clone());
            let current = breakdown.get(&key).copied().unwrap_or_default();
            let next = current.checked_sub(record.cost_micro_usd).ok_or_else(|| {
                GatewayError::Configuration(format!(
                    "correction {} exceeds its actor/model breakdown",
                    correction.correction_id
                ))
            })?;
            if next == 0 {
                breakdown.remove(&key);
            } else {
                breakdown.insert(key, next);
            }
            self.corrected_requests.insert(record.request_id);
        }
        self.corrections
            .insert(correction.correction_id, correction);
        Ok(())
    }
}

/// Crash-durable per-company spend counter. Append-only JSONL spool +
/// in-memory totals rebuilt on boot. Open failure is fatal to the gateway
/// (fail closed): a gateway that cannot account does not serve.
pub struct SpendStore {
    path: PathBuf,
    writer: Mutex<fs::File>,
    state: Mutex<SpendState>,
}

impl SpendStore {
    /// Open (creating if absent) the spool under `root`, rebuilding totals.
    ///
    /// Corruption policy: a corrupt line with only blank space after it is a
    /// torn tail — a crash or full disk mid-append (observed this sprint:
    /// the host disk filled once). The fsync'd prefix is accounted truth;
    /// the fragment is truncated away, LOUDLY, and the gateway still boots.
    /// A corrupt line with more content after it is real damage and stays
    /// fatal — a gateway that cannot account does not serve.
    pub fn open(root: &std::path::Path) -> GatewayResult<Self> {
        let root = root
            .canonicalize()
            .map_err(|error| GatewayError::Configuration(format!("spend root: {error}")))?;
        let path = root.join("spend.jsonl");
        let mut state = SpendState::default();
        if path.exists() {
            let text = fs::read_to_string(&path).map_err(|error| {
                GatewayError::Configuration(format!("read spend spool: {error}"))
            })?;
            let lines: Vec<&str> = text.lines().collect();
            for (index, line) in lines.iter().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                let entry: SpendEntry =
                    match serde_json::from_str(line) {
                        Ok(entry) => entry,
                        Err(error) => {
                            let torn_tail =
                                lines[index + 1..].iter().all(|rest| rest.trim().is_empty());
                            if torn_tail {
                                let truncate_at: u64 = lines[..index]
                                    .iter()
                                    .map(|line| line.len() as u64 + 1)
                                    .sum();
                                tracing::error!(
                                    line = index + 1,
                                    %error,
                                    "truncating torn spend-spool tail (crash or full disk \
                                     mid-append); the fsync'd prefix is accounted truth"
                                );
                                let file = OpenOptions::new().write(true).open(&path).map_err(
                                    |error| {
                                        GatewayError::Configuration(format!(
                                            "open spend spool for tail repair: {error}"
                                        ))
                                    },
                                )?;
                                file.set_len(truncate_at).map_err(|error| {
                                    GatewayError::Configuration(format!(
                                        "truncate torn spend-spool tail: {error}"
                                    ))
                                })?;
                                file.sync_all()?;
                                break;
                            }
                            return Err(GatewayError::Configuration(format!(
                                "corrupt spend spool line {}: {error}",
                                index + 1
                            )));
                        }
                    };
                match entry {
                    SpendEntry::Record(record) => state.apply_record(record)?,
                    SpendEntry::Correction(correction) => {
                        let plan = state.correction_plan(&correction).map_err(|error| {
                            GatewayError::Configuration(format!(
                                "invalid spend correction on line {}: {error}",
                                index + 1
                            ))
                        })?;
                        state.apply_correction(correction, plan)?;
                    }
                }
            }
        }
        let writer = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| GatewayError::Configuration(format!("append spend spool: {error}")))?;
        Ok(Self {
            path,
            writer: Mutex::new(writer),
            state: Mutex::new(state),
        })
    }

    #[must_use]
    pub fn spent_micro_usd(&self, company_id: &str) -> u64 {
        self.state
            .lock()
            .map(|state| {
                if state.poisons.contains(company_id) {
                    u64::MAX
                } else {
                    state
                        .accounted_totals
                        .get(company_id)
                        .copied()
                        .unwrap_or_default()
                }
            })
            .unwrap_or(u64::MAX) // poisoned lock fails closed
    }

    /// S04-T9. Spend broken down by `(actor, model)` for one company, read
    /// from the spool rather than from memory — the in-memory totals are a
    /// per-company sum and cannot answer "what did the critic cost".
    ///
    /// Reads the file each call. That is fine at this size and honest: the
    /// alternative is a second aggregate to keep in sync with the first.
    #[must_use]
    pub fn breakdown_micro_usd(&self, company_id: &str) -> Vec<(String, String, u64)> {
        let Ok(state) = self.state.lock() else {
            return Vec::new();
        };
        let mut rows: Vec<(String, String, u64)> = state
            .breakdowns
            .get(company_id)
            .into_iter()
            .flat_map(|breakdown| breakdown.iter())
            .map(|((actor, model), cost)| (actor.clone(), model.clone(), *cost))
            .collect();
        rows.sort_by(|a, b| b.2.cmp(&a.2));
        rows
    }

    /// S04-T1. Forget one company entirely: its spool records, its total and
    /// its poison.
    ///
    /// The spool is **one shared file**, not one per company, so destroying a
    /// throwaway cannot be done by deleting a path — and a `--destroy` that
    /// silently left spend behind is precisely the sprint-02 defect this was
    /// meant to close, where three "identical" comparison arms ran with
    /// $2.45 / $10.51 / $12.85 of headroom against a nominal $15 ceiling.
    pub fn forget(&self, company_id: &str) -> GatewayResult<()> {
        let mut writer = self.writer.lock().map_err(|_| GatewayError::Upstream)?;
        let mut state = self.state.lock().map_err(|_| GatewayError::Upstream)?;
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(GatewayError::Configuration(format!(
                    "read spend spool: {error}"
                )))
            }
        };
        let kept: Vec<&str> = text
            .lines()
            .filter(|line| {
                serde_json::from_str::<SpendEntry>(line)
                    .map(|entry| entry.company_id() != company_id)
                    .unwrap_or(true)
            })
            .collect();
        let mut rewritten = kept.join("\n");
        if !rewritten.is_empty() {
            rewritten.push('\n');
        }
        // Write-then-rename: a crash mid-rewrite must not lose other companies'
        // accounted spend.
        let temporary = self.path.with_extension("jsonl.tmp");
        fs::write(&temporary, rewritten)
            .map_err(|error| GatewayError::Configuration(format!("write spend spool: {error}")))?;
        fs::rename(&temporary, &self.path).map_err(|error| {
            GatewayError::Configuration(format!("replace spend spool: {error}"))
        })?;
        *writer = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| GatewayError::Configuration(format!("reopen spend spool: {error}")))?;
        state.accounted_totals.remove(company_id);
        state.poisons.remove(company_id);
        state.breakdowns.remove(company_id);
        state
            .records
            .retain(|_, record| record.company_id != company_id);
        state
            .corrections
            .retain(|_, correction| correction.company_id != company_id);
        state.corrected_requests = state
            .corrections
            .values()
            .flat_map(|correction| correction.request_ids.iter().copied())
            .collect();
        Ok(())
    }

    /// Pin a company's total at the maximum so every later pre-flight check
    /// fails closed. Used when a completed upstream call could not be
    /// accounted: an unaccountable spend stream is indistinguishable from
    /// unbounded spend, so the company stops until an operator inspects.
    /// The marker is appended to the spool (nil request id) so the poisoned
    /// state survives a daemon restart via the boot rebuild.
    pub fn poison(&self, company_id: &str) {
        let marker = SpendRecord {
            request_id: Uuid::nil(),
            company_id: company_id.to_owned(),
            model: POISON_MARKER.to_owned(),
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            cost_micro_usd: u64::MAX,
            actor_id: "daemon".into(),
            session_id: "system".into(),
            occurred_at: Utc::now(),
        };
        if self.record(&marker).is_err() {
            // Spool is unwritable; at minimum stop this process from serving.
            if let Ok(mut state) = self.state.lock() {
                state.poisons.insert(company_id.to_owned());
            }
        }
    }

    /// Clear a fail-closed poison after an operator has inspected. Appends a
    /// cancelling record; the original poison stays in the spool.
    pub fn clear_poison(&self, company_id: &str) -> GatewayResult<()> {
        let marker = SpendRecord {
            request_id: Uuid::nil(),
            company_id: company_id.to_owned(),
            model: POISON_CLEARED.to_owned(),
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            cost_micro_usd: 0,
            actor_id: "daemon".into(),
            session_id: "system".into(),
            occurred_at: Utc::now(),
        };
        self.record(&marker)?;
        Ok(())
    }

    /// Append one accounted call, fsync, then update the in-memory total.
    pub fn record(&self, record: &SpendRecord) -> GatewayResult<()> {
        let mut writer = self.writer.lock().map_err(|_| GatewayError::Upstream)?;
        let mut state = self.state.lock().map_err(|_| GatewayError::Upstream)?;
        let mut candidate = state.clone();
        candidate.apply_record(record.clone())?;
        self.append(&mut writer, record, "spend record")?;
        *state = candidate;
        Ok(())
    }

    /// Calculate the exact new accounted total without writing the spool.
    pub fn preview_correction(
        &self,
        correction_id: Uuid,
        company_id: &str,
        request_ids: &[Uuid],
        delta_micro_usd: i64,
        reason: &str,
        corrected_by: &str,
    ) -> GatewayResult<SpendCorrectionPreview> {
        let correction = SpendCorrection {
            entry_type: SpendEntryType::SpendCorrection,
            correction_id,
            company_id: company_id.to_owned(),
            request_ids: request_ids.to_vec(),
            delta_micro_usd,
            reason: reason.trim().to_owned(),
            corrected_by: corrected_by.to_owned(),
            occurred_at: Utc::now(),
        };
        let state = self.state.lock().map_err(|_| GatewayError::Upstream)?;
        Ok(state.correction_plan(&correction)?.preview)
    }

    /// Append one owner-attributed correction after revalidating it against
    /// the current spool state. The delta can only remove exact referenced
    /// records, so this recovery act can never mint additional spend authority.
    pub fn correct(
        &self,
        correction_id: Uuid,
        company_id: &str,
        request_ids: &[Uuid],
        delta_micro_usd: i64,
        reason: &str,
        corrected_by: &str,
    ) -> GatewayResult<(SpendCorrection, SpendCorrectionPreview)> {
        let correction = SpendCorrection {
            entry_type: SpendEntryType::SpendCorrection,
            correction_id,
            company_id: company_id.to_owned(),
            request_ids: request_ids.to_vec(),
            delta_micro_usd,
            reason: reason.trim().to_owned(),
            corrected_by: corrected_by.to_owned(),
            occurred_at: Utc::now(),
        };
        let mut writer = self.writer.lock().map_err(|_| GatewayError::Upstream)?;
        let mut state = self.state.lock().map_err(|_| GatewayError::Upstream)?;
        let plan = state.correction_plan(&correction)?;
        let preview = plan.preview.clone();
        self.append(&mut writer, &correction, "spend correction")?;
        state.apply_correction(correction.clone(), plan)?;
        Ok((correction, preview))
    }

    fn append<T: Serialize>(
        &self,
        writer: &mut fs::File,
        value: &T,
        label: &str,
    ) -> GatewayResult<()> {
        let mut line = serde_json::to_vec(value)
            .map_err(|error| GatewayError::Configuration(format!("encode {label}: {error}")))?;
        line.push(b'\n');
        writer.write_all(&line)?;
        writer.sync_all()?;
        // fsync the directory so the append survives a crash on all filesystems.
        if let Some(parent) = self.path.parent() {
            if let Ok(dir) = fs::File::open(parent) {
                let _ = dir.sync_all();
            }
        }
        Ok(())
    }
}

/// Token usage extracted from an upstream response body (SSE tail or JSON).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// Parse token usage from the tail of an upstream response. Two shapes:
/// SSE (`data: {"type":"response.completed","response":{"usage":{...}}}`) and
/// a plain JSON body with a top-level `usage`. Deterministic envelope
/// parsing — this reads a structured field, never content.
#[must_use]
pub fn parse_token_usage(tail: &[u8]) -> Option<TokenUsage> {
    let text = std::str::from_utf8(tail).ok()?;
    // SSE: scan data lines from the end; the completed event is last.
    for line in text.lines().rev() {
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(data) else {
            continue;
        };
        if let Some(usage) = extract_usage(&value) {
            return Some(usage);
        }
    }
    // Plain JSON body.
    let value = serde_json::from_str::<serde_json::Value>(text).ok()?;
    extract_usage(&value)
}

fn extract_usage(value: &serde_json::Value) -> Option<TokenUsage> {
    let usage = value
        .pointer("/response/usage")
        .or_else(|| value.pointer("/usage"))?;
    let input = usage
        .get("input_tokens")
        .and_then(serde_json::Value::as_u64);
    let output = usage
        .get("output_tokens")
        .and_then(serde_json::Value::as_u64);
    match (input, output) {
        (Some(input_tokens), Some(output_tokens)) => Some(TokenUsage {
            input_tokens,
            output_tokens,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_cost_is_integer_exact() {
        let rate = ModelRate {
            input_usd_per_mtok: 3.0,
            output_usd_per_mtok: 15.0,
        };
        // 1000 in + 500 out = $0.003 + $0.0075 = $0.0105 = 10500 micro
        assert_eq!(rate.cost_micro_usd(1000, 500), 10_500);
    }

    #[test]
    fn parses_sse_completed_usage() {
        let tail = b"data: {\"type\":\"response.output_text.delta\"}\n\ndata: {\"type\":\"response.completed\",\"response\":{\"usage\":{\"input_tokens\":42,\"output_tokens\":17}}}\n\n";
        let usage = parse_token_usage(tail).expect("usage");
        assert_eq!(
            usage,
            TokenUsage {
                input_tokens: 42,
                output_tokens: 17
            }
        );
    }

    #[test]
    fn parses_plain_json_usage() {
        let body = br#"{"id":"r1","usage":{"input_tokens":5,"output_tokens":9}}"#;
        let usage = parse_token_usage(body).expect("usage");
        assert_eq!(
            usage,
            TokenUsage {
                input_tokens: 5,
                output_tokens: 9
            }
        );
    }

    #[test]
    fn missing_usage_is_none_not_zero() {
        assert_eq!(
            parse_token_usage(b"data: {\"type\":\"response.completed\",\"response\":{}}\n"),
            None
        );
    }

    fn spend_record(company: &str, cost: u64) -> SpendRecord {
        SpendRecord {
            request_id: Uuid::new_v4(),
            company_id: company.to_owned(),
            model: "m".to_owned(),
            input_tokens: 1,
            output_tokens: 1,
            total_tokens: 0,
            cost_micro_usd: cost,
            actor_id: "exec".to_owned(),
            session_id: String::new(),
            occurred_at: Utc::now(),
        }
    }

    /// A crash/ENOSPC mid-append leaves a torn last line: the boot rebuild
    /// must truncate it and keep serving — and the repair must be real,
    /// i.e. the next append doesn't strand the fragment mid-file.
    #[test]
    fn torn_tail_is_truncated_and_spending_continues() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        let mut content = String::new();
        content.push_str(&serde_json::to_string(&spend_record("acme", 100)).unwrap());
        content.push('\n');
        content.push_str(&serde_json::to_string(&spend_record("acme", 200)).unwrap());
        content.push('\n');
        content.push_str("{\"requestId\":\"partial");
        fs::write(root.join("spend.jsonl"), content).unwrap();

        let store = SpendStore::open(root).expect("torn tail must not be fatal");
        assert_eq!(store.spent_micro_usd("acme"), 300);
        store.record(&spend_record("acme", 50)).unwrap();
        drop(store);
        let reopened = SpendStore::open(root).expect("repaired spool reopens");
        assert_eq!(reopened.spent_micro_usd("acme"), 350);
    }

    /// A corrupt line with good lines after it is real damage, not a torn
    /// tail: refuse to serve rather than guess at the accounting.
    #[test]
    fn mid_file_corruption_stays_fatal() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        let good = serde_json::to_string(&spend_record("acme", 100)).unwrap();
        fs::write(root.join("spend.jsonl"), format!("garbage\n{good}\n")).unwrap();
        assert!(SpendStore::open(root).is_err());
    }

    /// S04-T1. Destroying a throwaway must clear its spend, and must not touch
    /// anyone else's.
    ///
    /// This exists because the first implementation deleted
    /// `spend/<company>.jsonl` — a path that has never existed, since the spool
    /// is one shared file. It "passed" every observable check while silently
    /// leaving spend accounted, which is the sprint-02 comparison defect
    /// verbatim: three arms that looked identical and ran with different
    /// headroom because only three of four states were reset.
    #[test]
    fn forgetting_one_company_leaves_the_others_accounted() {
        let dir = std::env::temp_dir().join(format!("restless-forget-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let store = SpendStore::open(&dir).unwrap();
        store.record(&spend_record("keeper", 500)).unwrap();
        store.record(&spend_record("throwaway", 900)).unwrap();
        store.record(&spend_record("keeper", 250)).unwrap();
        assert_eq!(store.spent_micro_usd("throwaway"), 900);

        store.forget("throwaway").unwrap();

        assert_eq!(
            store.spent_micro_usd("throwaway"),
            0,
            "destroyed spend must be gone"
        );
        assert_eq!(
            store.spent_micro_usd("keeper"),
            750,
            "another company must be untouched"
        );

        // And it survives a restart: the spool itself was rewritten, not just
        // the in-memory total.
        let reopened = SpendStore::open(&dir).unwrap();
        assert_eq!(reopened.spent_micro_usd("throwaway"), 0);
        assert_eq!(reopened.spent_micro_usd("keeper"), 750);
        let _ = fs::remove_dir_all(&dir);
    }

    /// A poisoned company's total is a sentinel, not money. The breakdown must
    /// keep reporting real accounted spend so the owner can still see what was
    /// actually burned before the fuse blew.
    #[test]
    fn a_poison_does_not_contaminate_the_accounted_breakdown() {
        let dir = std::env::temp_dir().join(format!("restless-poison-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let store = SpendStore::open(&dir).unwrap();
        store.record(&spend_record("acme", 1_500)).unwrap();
        store.poison("acme");

        assert_eq!(
            store.spent_micro_usd("acme"),
            u64::MAX,
            "the fuse must fail closed"
        );
        let breakdown = store.breakdown_micro_usd("acme");
        let accounted: u64 = breakdown.iter().map(|(_, _, cost)| cost).sum();
        assert_eq!(
            accounted, 1_500,
            "real spend must survive the poison, un-inflated"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn correction_preview_is_read_only_and_application_is_exact_once() {
        let temporary = tempfile::tempdir().unwrap();
        let store = SpendStore::open(temporary.path()).unwrap();
        let mut duplicate_a = spend_record("acme", 470_000);
        duplicate_a.actor_id = "lead".into();
        duplicate_a.model = "kimi-k3".into();
        let mut duplicate_b = spend_record("acme", 1_110_000);
        duplicate_b.actor_id = "lead".into();
        duplicate_b.model = "kimi-k3".into();
        let mut truthful_final = spend_record("acme", 2_380_000);
        truthful_final.actor_id = "lead".into();
        truthful_final.model = "kimi-k3".into();
        store.record(&duplicate_a).unwrap();
        store.record(&duplicate_b).unwrap();
        store.record(&truthful_final).unwrap();
        let spool = temporary.path().join("spend.jsonl");
        let before = fs::read(&spool).unwrap();
        let correction_id = Uuid::new_v4();
        let request_ids = [duplicate_a.request_id, duplicate_b.request_id];

        let preview = store
            .preview_correction(
                correction_id,
                "acme",
                &request_ids,
                -1_580_000,
                "ACP emitted cumulative snapshots as separate records",
                "owner",
            )
            .unwrap();
        assert_eq!(preview.current_total_micro_usd, 3_960_000);
        assert_eq!(preview.post_correction_total_micro_usd, 2_380_000);
        assert_eq!(fs::read(&spool).unwrap(), before, "preview must not write");
        assert_eq!(store.spent_micro_usd("acme"), 3_960_000);

        let (correction, applied) = store
            .correct(
                correction_id,
                "acme",
                &request_ids,
                -1_580_000,
                "ACP emitted cumulative snapshots as separate records",
                "owner",
            )
            .unwrap();
        assert_eq!(applied, preview);
        assert_eq!(correction.correction_id, correction_id);
        assert_eq!(correction.delta_micro_usd, -1_580_000);
        assert_eq!(correction.corrected_by, "owner");
        assert_eq!(store.spent_micro_usd("acme"), 2_380_000);
        assert_eq!(
            store.breakdown_micro_usd("acme"),
            vec![("lead".into(), "kimi-k3".into(), 2_380_000)]
        );

        let text = fs::read_to_string(&spool).unwrap();
        assert_eq!(
            text.lines().count(),
            4,
            "three originals plus one correction"
        );
        for request_id in request_ids {
            assert!(
                text.contains(&request_id.to_string()),
                "the original duplicate and exact reference both remain"
            );
        }
        let correction_line: serde_json::Value =
            serde_json::from_str(text.lines().last().unwrap()).unwrap();
        assert_eq!(correction_line["type"], "spendCorrection");
        assert_eq!(correction_line["correctionId"], correction_id.to_string());
        assert_eq!(correction_line["deltaMicroUsd"], -1_580_000);
        assert_eq!(correction_line["correctedBy"], "owner");
        assert!(correction_line["occurredAt"].is_string());

        assert!(store
            .correct(
                correction_id,
                "acme",
                &request_ids,
                -1_580_000,
                "retry",
                "owner",
            )
            .is_err());
        assert!(store
            .correct(
                Uuid::new_v4(),
                "acme",
                &request_ids,
                -1_580_000,
                "same records under another id",
                "owner",
            )
            .is_err());
        assert_eq!(store.spent_micro_usd("acme"), 2_380_000);
        assert_eq!(text, fs::read_to_string(&spool).unwrap());

        drop(store);
        let reopened = SpendStore::open(temporary.path()).unwrap();
        assert_eq!(reopened.spent_micro_usd("acme"), 2_380_000);
        assert_eq!(
            reopened.breakdown_micro_usd("acme"),
            vec![("lead".into(), "kimi-k3".into(), 2_380_000)]
        );
        assert!(reopened
            .preview_correction(
                Uuid::new_v4(),
                "acme",
                &request_ids,
                -1_580_000,
                "cannot apply twice after restart",
                "owner",
            )
            .is_err());
    }

    #[test]
    fn correction_rejects_ambiguous_or_authority_expanding_inputs_without_writing() {
        let temporary = tempfile::tempdir().unwrap();
        let store = SpendStore::open(temporary.path()).unwrap();
        let acme = spend_record("acme", 100);
        let other = spend_record("other", 50);
        store.record(&acme).unwrap();
        store.record(&other).unwrap();
        let spool = temporary.path().join("spend.jsonl");
        let before = fs::read(&spool).unwrap();

        let invalid = [
            (vec![Uuid::new_v4()], -100, "unknown request"),
            (vec![other.request_id], -50, "cross-company request"),
            (
                vec![acme.request_id, acme.request_id],
                -200,
                "duplicate request reference",
            ),
            (vec![acme.request_id], 100, "positive correction"),
            (vec![acme.request_id], 0, "zero correction"),
            (vec![acme.request_id], -101, "over correction"),
            (vec![acme.request_id], -99, "partial ambiguous correction"),
        ];
        for (request_ids, delta, label) in invalid {
            assert!(
                store
                    .preview_correction(
                        Uuid::new_v4(),
                        "acme",
                        &request_ids,
                        delta,
                        label,
                        "owner",
                    )
                    .is_err(),
                "{label} must fail"
            );
        }
        assert!(store
            .preview_correction(
                Uuid::nil(),
                "acme",
                &[acme.request_id],
                -100,
                "nil correction id",
                "owner",
            )
            .is_err());
        assert_eq!(store.spent_micro_usd("acme"), 100);
        assert_eq!(store.spent_micro_usd("other"), 50);
        assert_eq!(fs::read(&spool).unwrap(), before);
    }

    #[test]
    fn legacy_spool_and_torn_correction_tail_recover_without_guessing() {
        let temporary = tempfile::tempdir().unwrap();
        let old_request = Uuid::new_v4();
        let old_line = serde_json::json!({
            "requestId": old_request,
            "companyId": "legacy",
            "model": "old-model",
            "inputTokens": 2,
            "outputTokens": 3,
            "costMicroUsd": 75,
            "occurredAt": Utc::now(),
        });
        fs::write(
            temporary.path().join("spend.jsonl"),
            format!("{old_line}\n{{\"type\":\"spendCorrection\",\"correctionId\":\"partial"),
        )
        .unwrap();

        let store = SpendStore::open(temporary.path()).expect("torn tail is discarded loudly");
        assert_eq!(store.spent_micro_usd("legacy"), 75);
        assert_eq!(
            store.breakdown_micro_usd("legacy"),
            vec![("unattributed".into(), "old-model".into(), 75)]
        );
        store.record(&spend_record("legacy", 25)).unwrap();
        drop(store);
        assert_eq!(
            SpendStore::open(temporary.path())
                .unwrap()
                .spent_micro_usd("legacy"),
            100
        );
    }

    #[test]
    fn invalid_or_mid_file_corrections_fail_closed() {
        let invalid_cases = [
            "{\"type\":\"spendCorrection\",\"correctionId\":\"not-a-uuid\"}",
            "{\"type\":\"spendCorrection\",\"correctionId\":\"partial",
        ];
        for invalid in invalid_cases {
            let temporary = tempfile::tempdir().unwrap();
            let first = serde_json::to_string(&spend_record("acme", 100)).unwrap();
            let after = serde_json::to_string(&spend_record("acme", 25)).unwrap();
            fs::write(
                temporary.path().join("spend.jsonl"),
                format!("{first}\n{invalid}\n{after}\n"),
            )
            .unwrap();
            assert!(
                SpendStore::open(temporary.path()).is_err(),
                "invalid correction before valid history must be fatal"
            );
        }

        let temporary = tempfile::tempdir().unwrap();
        let record = spend_record("acme", 100);
        let positive = SpendCorrection {
            entry_type: SpendEntryType::SpendCorrection,
            correction_id: Uuid::new_v4(),
            company_id: "acme".into(),
            request_ids: vec![record.request_id],
            delta_micro_usd: 100,
            reason: "would create authority".into(),
            corrected_by: "owner".into(),
            occurred_at: Utc::now(),
        };
        fs::write(
            temporary.path().join("spend.jsonl"),
            format!(
                "{}\n{}\n",
                serde_json::to_string(&record).unwrap(),
                serde_json::to_string(&positive).unwrap()
            ),
        )
        .unwrap();
        assert!(
            SpendStore::open(temporary.path()).is_err(),
            "a well-formed but authority-expanding correction is not a torn tail"
        );
    }
}
