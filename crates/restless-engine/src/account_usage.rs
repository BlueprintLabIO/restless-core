//! How much headroom each AI account has (S100): its rate-limit windows as the provider reports
//! them on calls companies already make, and which company used how much.
//!
//! Passive only: nothing here ever sends a model call to probe. A ChatGPT/Codex sign-in reports a
//! primary (about five hours) and a secondary (weekly) window on every response, as
//! `x-codex-{primary,secondary}-{used-percent,window-minutes,reset-at}` headers and as a
//! `codex.rate_limits` stream event. A Claude sign-in reports
//! `anthropic-ratelimit-unified-{5h,7d}-{utilization,reset}`. An account that reports nothing
//! stays unknown; nothing is guessed.

use std::sync::OnceLock;

use anyhow::{Context as _, Result};
use chrono::{DateTime, Duration, TimeZone as _, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};

static POOL: OnceLock<PgPool> = OnceLock::new();

/// Remember Authority's pool, so the model relay can record what it observes.
pub fn remember_pool(pool: PgPool) {
    let _ = POOL.set(pool);
}

/// Samples older than this are dropped; the forecast and burn need hours, history a couple weeks.
const SAMPLE_RETENTION_DAYS: i64 = 14;
/// At most one sample per account, window and company per this many seconds.
const SAMPLE_EVERY_SECONDS: i64 = 60;
/// The Inbox warns at this much of any window used.
pub const WARN_PERCENT: f64 = 80.0;

pub async fn ensure_schema(pool: &PgPool) -> Result<()> {
    for statement in [
        "CREATE TABLE IF NOT EXISTS restless_authority.account_usage (\
           account TEXT NOT NULL, window_name TEXT NOT NULL, used_percent DOUBLE PRECISION NOT NULL, \
           window_minutes BIGINT, resets_at TIMESTAMPTZ, observed_at TIMESTAMPTZ NOT NULL, source TEXT NOT NULL, \
           PRIMARY KEY (account, window_name))",
        "CREATE TABLE IF NOT EXISTS restless_authority.account_usage_samples (\
           account TEXT NOT NULL, window_name TEXT NOT NULL, company TEXT NOT NULL, \
           used_percent DOUBLE PRECISION NOT NULL, observed_at TIMESTAMPTZ NOT NULL)",
        "CREATE INDEX IF NOT EXISTS account_usage_samples_by_account \
           ON restless_authority.account_usage_samples (account, window_name, observed_at)",
    ] {
        sqlx::query(statement).execute(pool).await.context("create account usage tables")?;
    }
    Ok(())
}

/// One rate-limit window as a provider reported it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Window {
    /// `primary`/`secondary` (Codex) or `5h`/`7d` (Claude).
    pub name: String,
    pub used_percent: f64,
    pub window_minutes: Option<i64>,
    pub resets_at: Option<DateTime<Utc>>,
}

/// A reset given as epoch seconds (or milliseconds), or as seconds from now.
fn reset_time(value: &str, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let number: f64 = value.trim().parse().ok()?;
    if !number.is_finite() || number < 0.0 {
        return None;
    }
    let number = number as i64;
    match number {
        // Seconds from now: anything under a year.
        0..=31_536_000 => Some(now + Duration::seconds(number)),
        // Epoch milliseconds.
        n if n > 100_000_000_000 => Utc.timestamp_millis_opt(n).single(),
        n => Utc.timestamp_opt(n, 0).single(),
    }
}

fn window_minutes_label(minutes: Option<i64>) -> Option<&'static str> {
    match minutes? {
        280..=320 => Some("5h"),
        10_000..=10_200 => Some("7d"),
        _ => None,
    }
}

/// The windows in a Codex response's headers.
pub fn codex_header_windows(
    header: impl Fn(&str) -> Option<String>,
    now: DateTime<Utc>,
) -> Vec<Window> {
    ["primary", "secondary"]
        .into_iter()
        .filter_map(|name| {
            let used: f64 = header(&format!("x-codex-{name}-used-percent"))?
                .trim()
                .parse()
                .ok()?;
            if !used.is_finite() {
                return None;
            }
            let minutes = header(&format!("x-codex-{name}-window-minutes"))
                .and_then(|value| value.trim().parse().ok());
            let resets_at = header(&format!("x-codex-{name}-reset-at"))
                .and_then(|value| reset_time(&value, now))
                .or_else(|| {
                    header(&format!("x-codex-{name}-reset-after-seconds"))
                        .and_then(|value| reset_time(&value, now))
                });
            Some(Window {
                name: name.into(),
                used_percent: used.clamp(0.0, 100.0),
                window_minutes: minutes,
                resets_at,
            })
        })
        .collect()
}

/// The windows in a `codex.rate_limits` stream event.
pub fn codex_event_windows(event: &serde_json::Value, now: DateTime<Utc>) -> Vec<Window> {
    if event.get("type").and_then(serde_json::Value::as_str) != Some("codex.rate_limits") {
        return Vec::new();
    }
    let Some(limits) = event.get("rate_limits") else {
        return Vec::new();
    };
    ["primary", "secondary"]
        .into_iter()
        .filter_map(|name| {
            let window = limits.get(name)?;
            let used = window
                .get("used_percent")?
                .as_f64()
                .filter(|used| used.is_finite())?;
            let minutes = window
                .get("window_minutes")
                .and_then(serde_json::Value::as_i64);
            let resets_at = window
                .get("reset_at")
                .or_else(|| window.get("resets_at"))
                .and_then(|value| reset_time(&value.to_string(), now));
            Some(Window {
                name: name.into(),
                used_percent: used.clamp(0.0, 100.0),
                window_minutes: minutes,
                resets_at,
            })
        })
        .collect()
}

/// The windows in a Claude subscription response's headers (utilization is a fraction).
pub fn claude_header_windows(
    header: impl Fn(&str) -> Option<String>,
    now: DateTime<Utc>,
) -> Vec<Window> {
    [("5h", 300), ("7d", 10_080)]
        .into_iter()
        .filter_map(|(name, minutes)| {
            let utilization: f64 =
                header(&format!("anthropic-ratelimit-unified-{name}-utilization"))?
                    .trim()
                    .parse()
                    .ok()?;
            if !utilization.is_finite() {
                return None;
            }
            let percent = if utilization <= 1.0 {
                utilization * 100.0
            } else {
                utilization
            };
            let resets_at = header(&format!("anthropic-ratelimit-unified-{name}-reset"))
                .and_then(|value| reset_time(&value, now));
            Some(Window {
                name: name.into(),
                used_percent: percent.clamp(0.0, 100.0),
                window_minutes: Some(minutes),
                resets_at,
            })
        })
        .collect()
}

/// One account sign-in's limits as the model broker last read them from its provider.
#[derive(Debug, Clone, PartialEq)]
pub struct BrokerReport {
    pub provider: String,
    /// The provider's account id or email, when the report names one.
    pub account_key: Option<String>,
    pub windows: Vec<Window>,
}

/// The windows in the broker's `GET /v1/usage` body. The broker asks each provider itself (ChatGPT's
/// usage endpoint, Claude's), so this works when no response header ever reaches the relay: OMP's
/// gateway reads the providers' rate-limit headers itself and does not pass them on, which left
/// Settings › Usage "Unknown" for accounts in daily use (20 October). Only an account's main
/// windows are taken, under the names the header path uses: Codex `primary`/`secondary`, Claude
/// `5h`/`7d`.
pub fn broker_reports(body: &serde_json::Value) -> Vec<BrokerReport> {
    let Some(reports) = body.get("reports").and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    reports
        .iter()
        .filter_map(|report| {
            let provider = report.get("provider")?.as_str()?.to_string();
            let metadata = report.get("metadata");
            let account_key = ["accountId", "email"].iter().find_map(|field| {
                metadata?
                    .get(*field)?
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
            });
            let windows = report
                .get("limits")?
                .as_array()?
                .iter()
                .filter_map(|limit| {
                    let id = limit.get("id")?.as_str()?;
                    let name = id.strip_prefix(&format!("{provider}:"))?;
                    if !matches!(name, "primary" | "secondary" | "5h" | "7d") {
                        return None;
                    }
                    let amount = limit.get("amount")?;
                    let number =
                        |field: &str| amount.get(field).and_then(serde_json::Value::as_f64);
                    let used = number("usedFraction")
                        .map(|fraction| fraction * 100.0)
                        .or_else(|| {
                            (amount.get("unit").and_then(serde_json::Value::as_str)
                                == Some("percent"))
                            .then(|| number("used"))
                            .flatten()
                        })
                        .or_else(|| match (number("used"), number("limit")) {
                            (Some(used), Some(limit)) if limit > 0.0 => Some(used / limit * 100.0),
                            _ => None,
                        })
                        .filter(|used| used.is_finite())?;
                    let window = limit.get("window");
                    let window_minutes = window
                        .and_then(|window| window.get("durationMs")?.as_f64())
                        .map(|ms| (ms / 60_000.0).round() as i64);
                    let resets_at = window
                        .and_then(|window| window.get("resetsAt")?.as_f64())
                        .and_then(|ms| Utc.timestamp_millis_opt(ms as i64).single());
                    Some(Window {
                        name: name.to_string(),
                        used_percent: used.clamp(0.0, 100.0),
                        window_minutes,
                        resets_at,
                    })
                })
                .collect::<Vec<_>>();
            (!windows.is_empty()).then_some(BrokerReport {
                provider,
                account_key,
                windows,
            })
        })
        .collect()
}

/// The account a company's model uses, as `provider@connection`, from its credential reference
/// (`omp-oauth:openai-codex@<connection id>`).
pub fn account_of(reference: &str) -> Option<String> {
    let rest = reference
        .split_once(':')
        .map_or(reference, |(_, rest)| rest);
    let (provider, connection) = rest.split_once('@')?;
    (!provider.is_empty() && !connection.is_empty()).then(|| format!("{provider}@{connection}"))
}

/// Record what a call observed, if Authority is reachable. Never fails the call.
pub async fn observe(account: &str, company: &str, windows: &[Window], source: &str) {
    let Some(pool) = POOL.get() else { return };
    if let Err(error) = record(pool, account, company, windows, source, Utc::now()).await {
        tracing::warn!(%account, error = %format!("{error:#}"), "account usage not recorded");
    }
}

pub async fn record(
    pool: &PgPool,
    account: &str,
    company: &str,
    windows: &[Window],
    source: &str,
    now: DateTime<Utc>,
) -> Result<()> {
    for window in windows {
        sqlx::query(
            "INSERT INTO restless_authority.account_usage \
               (account, window_name, used_percent, window_minutes, resets_at, observed_at, source) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (account, window_name) DO UPDATE SET used_percent = EXCLUDED.used_percent, \
               window_minutes = COALESCE(EXCLUDED.window_minutes, account_usage.window_minutes), \
               resets_at = COALESCE(EXCLUDED.resets_at, account_usage.resets_at), \
               observed_at = EXCLUDED.observed_at, source = EXCLUDED.source \
             WHERE account_usage.observed_at <= EXCLUDED.observed_at",
        )
        .bind(account)
        .bind(&window.name)
        .bind(window.used_percent)
        .bind(window.window_minutes)
        .bind(window.resets_at)
        .bind(now)
        .bind(source)
        .execute(pool)
        .await?;
        let recent = sqlx::query(
            "SELECT 1 FROM restless_authority.account_usage_samples \
             WHERE account = $1 AND window_name = $2 AND company = $3 AND observed_at > $4",
        )
        .bind(account)
        .bind(&window.name)
        .bind(company)
        .bind(now - Duration::seconds(SAMPLE_EVERY_SECONDS))
        .fetch_optional(pool)
        .await?
        .is_some();
        if !recent {
            sqlx::query(
                "INSERT INTO restless_authority.account_usage_samples \
                   (account, window_name, company, used_percent, observed_at) VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(account)
            .bind(&window.name)
            .bind(company)
            .bind(window.used_percent)
            .bind(now)
            .execute(pool)
            .await?;
        }
    }
    sqlx::query("DELETE FROM restless_authority.account_usage_samples WHERE observed_at < $1")
        .bind(now - Duration::days(SAMPLE_RETENTION_DAYS))
        .execute(pool)
        .await?;
    Ok(())
}

/// A window as the owner sees it: how full, when it resets, how fresh, who used it, and when it
/// runs out at the current pace.
#[derive(Debug, Clone, Serialize)]
pub struct WindowView {
    pub name: String,
    /// "5-hour" or "Weekly", when the length says which.
    pub label: String,
    pub used_percent: f64,
    pub window_minutes: Option<i64>,
    pub resets_at: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
    /// When it reaches 100% at the pace of the last hour, if that is before the reset.
    pub runs_out_at: Option<DateTime<Utc>>,
    /// Percentage points each company used in this window so far, largest first.
    pub by_company: Vec<(String, f64)>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountView {
    pub account: String,
    pub windows: Vec<WindowView>,
}

fn label_for(name: &str, minutes: Option<i64>) -> String {
    match (name, window_minutes_label(minutes)) {
        (_, Some("5h")) | ("5h", _) => "5-hour".into(),
        (_, Some("7d")) | ("7d", _) => "Weekly".into(),
        ("primary", _) => "Short window".into(),
        ("secondary", _) => "Long window".into(),
        (other, _) => other.to_string(),
    }
}

/// When `used` reaches 100 at the pace of `samples` (time, percent), oldest first.
pub fn forecast(
    samples: &[(DateTime<Utc>, f64)],
    now: DateTime<Utc>,
    resets_at: Option<DateTime<Utc>>,
) -> Option<DateTime<Utc>> {
    let recent: Vec<&(DateTime<Utc>, f64)> = samples
        .iter()
        .filter(|(at, _)| *at >= now - Duration::hours(1))
        .collect();
    let (first, last) = (recent.first()?, recent.last()?);
    let minutes = (last.0 - first.0).num_seconds() as f64 / 60.0;
    let gained = last.1 - first.1;
    if minutes < 5.0 || gained <= 0.0 || last.1 >= 100.0 {
        return None;
    }
    let per_minute = gained / minutes;
    let at = last.0 + Duration::seconds(((100.0 - last.1) / per_minute * 60.0) as i64);
    match resets_at {
        Some(reset) if at >= reset => None,
        _ => Some(at),
    }
}

/// Points each company added, from consecutive samples of one window (oldest first). A drop is a
/// reset and starts over.
pub fn burn_by_company(samples: &[(DateTime<Utc>, f64, String)]) -> Vec<(String, f64)> {
    let mut totals: std::collections::BTreeMap<String, f64> = std::collections::BTreeMap::new();
    let mut previous: Option<f64> = None;
    for (_, used, company) in samples {
        if let Some(before) = previous {
            if *used >= before {
                *totals.entry(company.clone()).or_default() += used - before;
            } else {
                totals.clear();
            }
        }
        previous = Some(*used);
    }
    let mut ranked: Vec<(String, f64)> = totals
        .into_iter()
        .filter(|(company, points)| *points > 0.0 && !company.is_empty())
        .collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    ranked
}

/// Every account's windows, as last observed.
pub async fn accounts(pool: &PgPool, now: DateTime<Utc>) -> Result<Vec<AccountView>> {
    let rows = sqlx::query(
        "SELECT account, window_name, used_percent, window_minutes, resets_at, observed_at \
         FROM restless_authority.account_usage ORDER BY account, window_name",
    )
    .fetch_all(pool)
    .await?;
    let mut views: Vec<AccountView> = Vec::new();
    for row in rows {
        let account: String = row.get("account");
        let name: String = row.get("window_name");
        let minutes: Option<i64> = row.get("window_minutes");
        let mut resets_at: Option<DateTime<Utc>> = row.get("resets_at");
        let mut used: f64 = row.get("used_percent");
        // A window whose reset has passed since it was seen has started over.
        if resets_at.is_some_and(|reset| reset <= now) {
            used = 0.0;
            resets_at = None;
        }
        let samples = sqlx::query(
            "SELECT observed_at, used_percent, company FROM restless_authority.account_usage_samples \
             WHERE account = $1 AND window_name = $2 AND observed_at > $3 ORDER BY observed_at",
        )
        .bind(&account)
        .bind(&name)
        .bind(now - Duration::minutes(minutes.unwrap_or(300).clamp(60, 10_080)))
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(|sample| (sample.get::<DateTime<Utc>, _>("observed_at"), sample.get::<f64, _>("used_percent"), sample.get::<String, _>("company")))
        .collect::<Vec<_>>();
        let pace: Vec<(DateTime<Utc>, f64)> =
            samples.iter().map(|(at, used, _)| (*at, *used)).collect();
        let window = WindowView {
            label: label_for(&name, minutes),
            runs_out_at: forecast(&pace, now, resets_at),
            by_company: burn_by_company(&samples),
            used_percent: used,
            window_minutes: minutes,
            resets_at,
            observed_at: row.get("observed_at"),
            name,
        };
        match views.last_mut() {
            Some(view) if view.account == account => view.windows.push(window),
            _ => views.push(AccountView {
                account,
                windows: vec![window],
            }),
        }
    }
    Ok(views)
}

/// The fullest window of one account, for a warning.
pub async fn fullest(
    pool: &PgPool,
    account: &str,
    now: DateTime<Utc>,
) -> Result<Option<WindowView>> {
    Ok(accounts(pool, now)
        .await?
        .into_iter()
        .find(|view| view.account == account)
        .and_then(|view| {
            view.windows
                .into_iter()
                .max_by(|a, b| a.used_percent.total_cmp(&b.used_percent))
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 19, 1, 0, 0).unwrap()
    }

    #[test]
    fn the_brokers_usage_report_gives_each_sign_ins_main_windows() {
        let body = serde_json::json!({
            "generatedAt": 1_760_900_000_000_i64,
            "reports": [
                {
                    "provider": "openai-codex",
                    "fetchedAt": 1_760_900_000_000_i64,
                    "metadata": { "accountId": "acct_1", "email": "a@b.test" },
                    "limits": [
                        { "id": "openai-codex:primary", "label": "5 Hour", "scope": { "provider": "openai-codex" },
                          "window": { "id": "5h", "durationMs": 18_000_000, "resetsAt": 1_760_910_000_000_i64 },
                          "amount": { "usedFraction": 0.42, "unit": "percent" } },
                        { "id": "openai-codex:secondary", "label": "7 Day", "scope": { "provider": "openai-codex" },
                          "window": { "id": "7d", "durationMs": 604_800_000 },
                          "amount": { "used": 17, "unit": "percent" } },
                        { "id": "openai-codex:gpt-6:primary", "label": "5 Hour (GPT-6)", "scope": { "provider": "openai-codex" },
                          "amount": { "usedFraction": 0.9, "unit": "percent" } }
                    ]
                },
                { "provider": "anthropic", "fetchedAt": 0, "limits": [
                    { "id": "anthropic:5h", "label": "Claude 5 Hour", "scope": { "provider": "anthropic" },
                      "amount": { "used": 30, "limit": 120, "unit": "tokens" } },
                    { "id": "anthropic:extra", "label": "Extra", "scope": { "provider": "anthropic" },
                      "amount": { "usedFraction": 0.1, "unit": "usd" } }
                ] }
            ]
        });
        let reports = broker_reports(&body);
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].account_key.as_deref(), Some("acct_1"));
        let names: Vec<_> = reports[0]
            .windows
            .iter()
            .map(|window| window.name.as_str())
            .collect();
        assert_eq!(
            names,
            ["primary", "secondary"],
            "a model's own window is not the account's"
        );
        assert!((reports[0].windows[0].used_percent - 42.0).abs() < 1e-9);
        assert_eq!(reports[0].windows[0].window_minutes, Some(300));
        assert!(reports[0].windows[0].resets_at.is_some());
        assert!((reports[0].windows[1].used_percent - 17.0).abs() < 1e-9);
        assert_eq!(reports[1].windows.len(), 1);
        assert!((reports[1].windows[0].used_percent - 25.0).abs() < 1e-9);
        assert!(broker_reports(&serde_json::json!({})).is_empty());
    }

    #[test]
    fn an_account_wide_reading_is_nobodys_burn() {
        let samples = vec![
            (now(), 10.0, String::new()),
            (now(), 15.0, "reno".to_string()),
            (now(), 18.0, String::new()),
        ];
        assert_eq!(burn_by_company(&samples), vec![("reno".to_string(), 5.0)]);
    }

    #[test]
    fn codex_headers_give_both_windows() {
        let headers = [
            ("x-codex-primary-used-percent", "84.5"),
            ("x-codex-primary-window-minutes", "300"),
            ("x-codex-primary-reset-at", "1792371600"),
            ("x-codex-secondary-used-percent", "31"),
            ("x-codex-secondary-window-minutes", "10080"),
        ];
        let header = |name: &str| {
            headers
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        };
        let windows = codex_header_windows(header, now());
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].used_percent, 84.5);
        assert_eq!(
            windows[0].resets_at,
            Utc.timestamp_opt(1_792_371_600, 0).single()
        );
        assert_eq!(label_for("primary", windows[0].window_minutes), "5-hour");
        assert_eq!(label_for("secondary", windows[1].window_minutes), "Weekly");
        assert!(
            codex_header_windows(|_| None, now()).is_empty(),
            "no headers, nothing known"
        );
    }

    #[test]
    fn the_codex_stream_event_carries_the_same() {
        let event = serde_json::json!({
            "type": "codex.rate_limits",
            "rate_limits": { "primary": { "used_percent": 100.0, "window_minutes": 300, "reset_at": 1792371600 } }
        });
        let windows = codex_event_windows(&event, now());
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].used_percent, 100.0);
        assert!(
            codex_event_windows(&serde_json::json!({ "type": "response.completed" }), now())
                .is_empty()
        );
    }

    #[test]
    fn claude_utilization_is_a_fraction() {
        let header = |name: &str| match name {
            "anthropic-ratelimit-unified-5h-utilization" => Some("0.42".to_string()),
            "anthropic-ratelimit-unified-5h-reset" => Some("1792371600".to_string()),
            _ => None,
        };
        let windows = claude_header_windows(header, now());
        assert_eq!(windows.len(), 1);
        assert!((windows[0].used_percent - 42.0).abs() < 1e-9);
    }

    #[test]
    fn the_account_is_the_connection_behind_the_model() {
        assert_eq!(
            account_of("omp-oauth:openai-codex@0123abcd").as_deref(),
            Some("openai-codex@0123abcd")
        );
        assert_eq!(account_of("hosted-model-relay:v1"), None);
    }

    #[test]
    fn the_forecast_says_when_it_runs_out_before_the_reset() {
        let at = |minutes: i64| now() - Duration::minutes(minutes);
        let climbing = [(at(40), 60.0), (at(20), 70.0), (at(0), 80.0)];
        let runs_out = forecast(&climbing, now(), Some(now() + Duration::hours(3))).unwrap();
        assert_eq!((runs_out - now()).num_minutes(), 40);
        assert_eq!(
            forecast(&climbing, now(), Some(now() + Duration::minutes(10))),
            None,
            "resets first"
        );
        assert_eq!(
            forecast(&[(at(2), 70.0), (at(0), 80.0)], now(), None),
            None,
            "too little to go on"
        );
        assert_eq!(
            forecast(&[(at(30), 80.0), (at(0), 80.0)], now(), None),
            None,
            "not climbing"
        );
    }

    #[test]
    fn burn_is_split_by_the_company_that_moved_it() {
        let at = now();
        let samples = vec![
            (at, 10.0, "reno".to_string()),
            (at, 30.0, "frankensurf".to_string()),
            (at, 35.0, "reno".to_string()),
            (at, 60.0, "frankensurf".to_string()),
        ];
        assert_eq!(
            burn_by_company(&samples),
            vec![("frankensurf".to_string(), 45.0), ("reno".to_string(), 5.0)]
        );
        // A reset starts the count again.
        let reset = vec![
            (at, 90.0, "reno".to_string()),
            (at, 5.0, "reno".to_string()),
            (at, 9.0, "frankensurf".to_string()),
        ];
        assert_eq!(
            burn_by_company(&reset),
            vec![("frankensurf".to_string(), 4.0)]
        );
    }

    #[test]
    fn resets_read_as_epochs_or_seconds_from_now() {
        assert_eq!(reset_time("3600", now()), Some(now() + Duration::hours(1)));
        assert_eq!(
            reset_time("1792371600000", now()),
            Utc.timestamp_millis_opt(1_792_371_600_000).single()
        );
        assert_eq!(reset_time("soon", now()), None);
    }
}
