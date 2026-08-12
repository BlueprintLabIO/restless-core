//! Per-company dollar spend accounting — the fuse (sprint 01 T2).
//!
//! The legacy gateway bounded request *counts* only. This module adds the
//! missing dollar dimension: cost computed per completed upstream response
//! from a configured rate table, appended fsync'd to a JSONL spool, in-memory
//! per-company totals rebuilt from the spool on boot. The pre-flight check in
//! the proxy rejects at the ceiling and never fails open: an unreadable spool
//! or an unpriced model refuses service rather than guessing.

use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, OpenOptions},
    io::{BufRead, Write as _},
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
    pub cost_micro_usd: u64,
    pub occurred_at: DateTime<Utc>,
}

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
    pub fn open(root: &std::path::Path) -> GatewayResult<Self> {
        let root = root.canonicalize().map_err(|error| {
            GatewayError::Configuration(format!("spend root: {error}"))
        })?;
        let path = root.join("spend.jsonl");
        let mut totals: HashMap<String, u64> = HashMap::new();
        if path.exists() {
            let file = fs::File::open(&path).map_err(|error| {
                GatewayError::Configuration(format!("open spend spool: {error}"))
            })?;
            for line in std::io::BufReader::new(file).lines() {
                let line = line.map_err(|error| {
                    GatewayError::Configuration(format!("read spend spool: {error}"))
                })?;
                if line.trim().is_empty() {
                    continue;
                }
                let record: SpendRecord = serde_json::from_str(&line).map_err(|error| {
                    GatewayError::Configuration(format!("corrupt spend spool line: {error}"))
                })?;
                let total = totals.entry(record.company_id).or_default();
                *total = total.saturating_add(record.cost_micro_usd);
            }
        }
        let writer = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| GatewayError::Configuration(format!("append spend spool: {error}")))?;
        Ok(Self { path, writer: Mutex::new(writer), totals: Mutex::new(totals) })
    }

    #[must_use]
    pub fn spent_micro_usd(&self, company_id: &str) -> u64 {
        self.totals
            .lock()
            .map(|totals| totals.get(company_id).copied().unwrap_or(0))
            .unwrap_or(u64::MAX) // poisoned lock fails closed
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
            model: "poison-marker".to_owned(),
            input_tokens: 0,
            output_tokens: 0,
            cost_micro_usd: u64::MAX,
            occurred_at: Utc::now(),
        };
        if self.record(&marker).is_err() {
            // Spool is unwritable; at minimum stop this process from serving.
            if let Ok(mut totals) = self.totals.lock() {
                totals.insert(company_id.to_owned(), u64::MAX);
            }
        }
    }

    /// Append one accounted call, fsync, then update the in-memory total.
    pub fn record(&self, record: &SpendRecord) -> GatewayResult<()> {
        let mut line = serde_json::to_vec(record)
            .map_err(|error| GatewayError::Configuration(format!("encode spend record: {error}")))?;
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
        let Some(data) = line.strip_prefix("data:") else { continue };
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(data) else { continue };
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
    let input = usage.get("input_tokens").and_then(serde_json::Value::as_u64);
    let output = usage.get("output_tokens").and_then(serde_json::Value::as_u64);
    match (input, output) {
        (Some(input_tokens), Some(output_tokens)) => {
            Some(TokenUsage { input_tokens, output_tokens })
        }
        _ => None,
    }
}

/// Per-company ceilings in micro-USD, refreshed by the embedding daemon from
/// company config files. Unknown company = no ceiling entry = refuse (the
/// issuer only mints tokens for configured companies; a missing entry means
/// the config vanished mid-run, which is exactly when to stop spending).
pub type CeilingMap = std::sync::Arc<std::sync::RwLock<BTreeMap<String, u64>>>;

#[must_use]
pub fn ceiling_map() -> CeilingMap {
    std::sync::Arc::new(std::sync::RwLock::new(BTreeMap::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_cost_is_integer_exact() {
        let rate = ModelRate { input_usd_per_mtok: 3.0, output_usd_per_mtok: 15.0 };
        // 1000 in + 500 out = $0.003 + $0.0075 = $0.0105 = 10500 micro
        assert_eq!(rate.cost_micro_usd(1000, 500), 10_500);
    }

    #[test]
    fn parses_sse_completed_usage() {
        let tail = b"data: {\"type\":\"response.output_text.delta\"}\n\ndata: {\"type\":\"response.completed\",\"response\":{\"usage\":{\"input_tokens\":42,\"output_tokens\":17}}}\n\n";
        let usage = parse_token_usage(tail).expect("usage");
        assert_eq!(usage, TokenUsage { input_tokens: 42, output_tokens: 17 });
    }

    #[test]
    fn parses_plain_json_usage() {
        let body = br#"{"id":"r1","usage":{"input_tokens":5,"output_tokens":9}}"#;
        let usage = parse_token_usage(body).expect("usage");
        assert_eq!(usage, TokenUsage { input_tokens: 5, output_tokens: 9 });
    }

    #[test]
    fn missing_usage_is_none_not_zero() {
        assert_eq!(parse_token_usage(b"data: {\"type\":\"response.completed\",\"response\":{}}\n"), None);
    }
}
