//! Per-company dollar spend accounting — the fuse (sprint 01 T2).
//!
//! Append fsync'd to a JSONL spool, per-company totals rebuilt from the spool
//! on boot, and never fails open: an unreadable spool refuses service rather
//! than guessing. Costs now arrive from the agent's own per-turn usage report
//! rather than being scraped from a proxied HTTP response, so the rate table
//! and usage parser below are retained only for records written by that
//! retired path.

use std::{
    collections::HashMap,
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
    pub occurred_at: DateTime<Utc>,
}

/// Model field marking a fail-closed poison, and its cancellation. Both are
/// records rather than mutations: the spool stays append-only and the incident
/// stays visible after recovery.
pub const POISON_MARKER: &str = "poison-marker";
pub const POISON_CLEARED: &str = "poison-cleared";

/// Crash-durable per-company spend counter. Append-only JSONL spool +
/// in-memory totals rebuilt on boot. Open failure is fatal to the gateway
/// (fail closed): a gateway that cannot account does not serve.
pub struct SpendStore {
    path: PathBuf,
    writer: Mutex<fs::File>,
    totals: Mutex<HashMap<String, u64>>,
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
        let mut totals: HashMap<String, u64> = HashMap::new();
        let mut poisons: std::collections::HashSet<String> = std::collections::HashSet::new();
        if path.exists() {
            let text = fs::read_to_string(&path).map_err(|error| {
                GatewayError::Configuration(format!("read spend spool: {error}"))
            })?;
            let lines: Vec<&str> = text.lines().collect();
            for (index, line) in lines.iter().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                let record: SpendRecord =
                    match serde_json::from_str(line) {
                        Ok(record) => record,
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
                // A cleared poison is cancelled, not erased: both records stay
                // in the spool so the incident is still legible, but the
                // company's total returns to its real spend. Without this a
                // fail-closed poison is permanent and a company is bricked by a
                // provider outage it did not cause — observed live when a
                // credit exhaustion produced usage with no cost, poisoning two
                // companies that were otherwise healthy.
                if record.model == POISON_CLEARED {
                    poisons.remove(&record.company_id);
                    continue;
                }
                if record.model == POISON_MARKER {
                    poisons.insert(record.company_id.clone());
                    continue;
                }
                let total = totals.entry(record.company_id).or_default();
                *total = total.saturating_add(record.cost_micro_usd);
            }
        }
        for company in poisons {
            totals.insert(company, u64::MAX);
        }
        let writer = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| GatewayError::Configuration(format!("append spend spool: {error}")))?;
        Ok(Self {
            path,
            writer: Mutex::new(writer),
            totals: Mutex::new(totals),
        })
    }

    #[must_use]
    pub fn spent_micro_usd(&self, company_id: &str) -> u64 {
        self.totals
            .lock()
            .map(|totals| totals.get(company_id).copied().unwrap_or(0))
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
        let mut by_key: HashMap<(String, String), u64> = HashMap::new();
        let Ok(text) = fs::read_to_string(&self.path) else {
            return Vec::new();
        };
        for line in text.lines() {
            let Ok(record) = serde_json::from_str::<SpendRecord>(line) else {
                continue;
            };
            if record.company_id != company_id || record.request_id == Uuid::nil() {
                continue;
            }
            let actor = if record.actor_id.is_empty() {
                // Records written before the actor dimension existed. Named for
                // what they are rather than blamed on a guess.
                "unattributed".to_string()
            } else {
                record.actor_id.clone()
            };
            *by_key.entry((actor, record.model.clone())).or_default() += record.cost_micro_usd;
        }
        let mut rows: Vec<(String, String, u64)> =
            by_key.into_iter().map(|((a, m), c)| (a, m, c)).collect();
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
                serde_json::from_str::<SpendRecord>(line)
                    .map(|record| record.company_id != company_id)
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

        if let Ok(mut totals) = self.totals.lock() {
            totals.remove(company_id);
        }
        if let Ok(mut writer) = self.writer.lock() {
            let _ = writer.flush();
        }
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
            occurred_at: Utc::now(),
        };
        if self.record(&marker).is_err() {
            // Spool is unwritable; at minimum stop this process from serving.
            if let Ok(mut totals) = self.totals.lock() {
                totals.insert(company_id.to_owned(), u64::MAX);
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
            occurred_at: Utc::now(),
        };
        self.record(&marker)?;
        if let Ok(mut totals) = self.totals.lock() {
            totals.remove(company_id);
        }
        Ok(())
    }

    /// Append one accounted call, fsync, then update the in-memory total.
    pub fn record(&self, record: &SpendRecord) -> GatewayResult<()> {
        let mut line = serde_json::to_vec(record).map_err(|error| {
            GatewayError::Configuration(format!("encode spend record: {error}"))
        })?;
        line.push(b'\n');
        let mut writer = self.writer.lock().map_err(|_| GatewayError::Upstream)?;
        writer.write_all(&line)?;
        writer.sync_all()?;
        // fsync the directory so the append survives a crash on all filesystems.
        if let Some(parent) = self.path.parent() {
            if let Ok(dir) = fs::File::open(parent) {
                let _ = dir.sync_all();
            }
        }
        drop(writer);
        let mut totals = self.totals.lock().map_err(|_| GatewayError::Upstream)?;
        let total = totals.entry(record.company_id.clone()).or_default();
        *total = total.saturating_add(record.cost_micro_usd);
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
}
