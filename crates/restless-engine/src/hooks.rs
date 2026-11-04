//! Provider webhooks on the account plane (ADR 0017): one public door, `POST /hooks/<provider>/<token>`.
//!
//! The outside world reaches a company here: a reply to its email, a payout settling, later a
//! GitHub push or a form submission. The path names the provider and an opaque per-company token,
//! never the company. The token only routes; it is not the trust boundary. Each provider's own
//! signature is, checked with that company's own secret before anything is written, exactly as
//! the older per-provider listeners did (`ingress.rs`, `airwallex_ingress.rs`).
//!
//! The door does four things and nothing else: bound the request, find the company, verify the
//! provider's signature, and hand the verified event to the provider's existing receiver (which
//! records Authority before it answers). It reads no cookie, sets none, carries no session and
//! answers no other path, so it can sit outside the owner-browser boundary without becoming an
//! owner API.
//!
//! Per provider it also keeps a little health: when an event last arrived and when one last
//! failed, so a webhook that went quiet or broke is noticed (`incoming_email`, the Inbox).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row as _};

/// The public path prefix every provider shares.
pub const HOOKS_PREFIX: &str = "/hooks/";

/// A webhook is a small JSON document; Resend's and Airwallex's are a few KiB. Anything larger
/// is a bug or an attempt to exhaust memory.
pub const MAX_BODY: usize = 256 * 1024;

/// Deliveries per token per minute before the door answers 429. A provider retries a 429, so a
/// burst is delayed, never lost. Far above any real company's mail or payment rate.
const PER_TOKEN_PER_MINUTE: u32 = 120;
/// Deliveries per minute across every token, including tokens that name nobody: spraying random
/// tokens costs a lookup each, and this bounds that.
const ALL_TOKENS_PER_MINUTE: u32 = 1_200;
const RATE_WINDOW: Duration = Duration::from_secs(60);
/// The rate table forgets every window when it grows past this many tokens.
const RATE_TABLE_LIMIT: usize = 10_000;

/// A provider the door verifies. Adding one is a new arm here: its name in the path, its
/// signature check, and the receiver its verified events go to (ADR 0017 §Adding a provider).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    /// Email: Svix-signed (`svix-id`, `svix-timestamp`, `svix-signature`).
    Resend,
    /// Payouts: `x-timestamp` + `x-signature` HMAC over timestamp and body.
    Airwallex,
}

impl Provider {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "resend" => Some(Self::Resend),
            "airwallex" => Some(Self::Airwallex),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Resend => "resend",
            Self::Airwallex => "airwallex",
        }
    }
}

pub async fn ensure_schema(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.hook_endpoints (\
           company TEXT NOT NULL, provider TEXT NOT NULL, token TEXT NOT NULL UNIQUE, \
           provider_webhook_id TEXT, receiving_domains JSONB NOT NULL DEFAULT '[]'::jsonb, \
           registered_at TIMESTAMPTZ, registration_error TEXT, last_connect_at TIMESTAMPTZ, \
           last_event_at TIMESTAMPTZ, events BIGINT NOT NULL DEFAULT 0, \
           last_failure_at TIMESTAMPTZ, last_failure TEXT, failures BIGINT NOT NULL DEFAULT 0, \
           last_poll_at TIMESTAMPTZ, poll_cursor TIMESTAMPTZ, poll_error TEXT, \
           missed BIGINT NOT NULL DEFAULT 0, last_missed_at TIMESTAMPTZ, \
           check_nonce TEXT, check_sent_at TIMESTAMPTZ, check_passed_at TIMESTAMPTZ, \
           created_at TIMESTAMPTZ NOT NULL DEFAULT now(), \
           PRIMARY KEY (company, provider)\
         )",
    )
    .execute(pool)
    .await
    .context("create hook endpoints")?;
    Ok(())
}

/// One company's door for one provider, and what it has seen.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Endpoint {
    pub company: String,
    pub provider: String,
    #[serde(skip_serializing)]
    pub token: String,
    pub provider_webhook_id: Option<String>,
    pub receiving_domains: Vec<String>,
    pub registered_at: Option<DateTime<Utc>>,
    pub registration_error: Option<String>,
    pub last_connect_at: Option<DateTime<Utc>>,
    pub last_event_at: Option<DateTime<Utc>>,
    pub events: i64,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub last_failure: Option<String>,
    pub failures: i64,
    pub last_poll_at: Option<DateTime<Utc>>,
    pub poll_cursor: Option<DateTime<Utc>>,
    pub poll_error: Option<String>,
    pub missed: i64,
    pub last_missed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing)]
    pub check_nonce: Option<String>,
    pub check_sent_at: Option<DateTime<Utc>>,
    pub check_passed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

const ENDPOINT_COLUMNS: &str = "company, provider, token, provider_webhook_id, receiving_domains, \
     registered_at, registration_error, last_connect_at, last_event_at, events, last_failure_at, last_failure, \
     failures, last_poll_at, poll_cursor, poll_error, missed, last_missed_at, check_nonce, \
     check_sent_at, check_passed_at, created_at";

fn endpoint_from_row(row: &sqlx::postgres::PgRow) -> Result<Endpoint> {
    let domains: serde_json::Value = row.try_get("receiving_domains")?;
    Ok(Endpoint {
        company: row.try_get("company")?,
        provider: row.try_get("provider")?,
        token: row.try_get("token")?,
        provider_webhook_id: row.try_get("provider_webhook_id")?,
        receiving_domains: serde_json::from_value(domains).unwrap_or_default(),
        registered_at: row.try_get("registered_at")?,
        registration_error: row.try_get("registration_error")?,
        last_connect_at: row.try_get("last_connect_at")?,
        last_event_at: row.try_get("last_event_at")?,
        events: row.try_get("events")?,
        last_failure_at: row.try_get("last_failure_at")?,
        last_failure: row.try_get("last_failure")?,
        failures: row.try_get("failures")?,
        last_poll_at: row.try_get("last_poll_at")?,
        poll_cursor: row.try_get("poll_cursor")?,
        poll_error: row.try_get("poll_error")?,
        missed: row.try_get("missed")?,
        last_missed_at: row.try_get("last_missed_at")?,
        check_nonce: row.try_get("check_nonce")?,
        check_sent_at: row.try_get("check_sent_at")?,
        check_passed_at: row.try_get("check_passed_at")?,
        created_at: row.try_get("created_at")?,
    })
}

pub async fn endpoint(
    pool: &PgPool,
    company: &str,
    provider: Provider,
) -> Result<Option<Endpoint>> {
    let row = sqlx::query(&format!(
        "SELECT {ENDPOINT_COLUMNS} FROM restless_authority.hook_endpoints \
         WHERE company=$1 AND provider=$2"
    ))
    .bind(company)
    .bind(provider.name())
    .fetch_optional(pool)
    .await
    .context("read hook endpoint")?;
    row.as_ref().map(endpoint_from_row).transpose()
}

pub async fn endpoints_of(pool: &PgPool, provider: Provider) -> Result<Vec<Endpoint>> {
    let rows = sqlx::query(&format!(
        "SELECT {ENDPOINT_COLUMNS} FROM restless_authority.hook_endpoints \
         WHERE provider=$1 ORDER BY company"
    ))
    .bind(provider.name())
    .fetch_all(pool)
    .await
    .context("read hook endpoints")?;
    rows.iter().map(endpoint_from_row).collect()
}

/// The company's door for this provider, minted on first use. The token is the only thing a
/// provider is told; it never changes for the life of the endpoint, so a webhook registered with
/// it keeps working across releases and plane moves.
pub async fn ensure_endpoint(pool: &PgPool, company: &str, provider: Provider) -> Result<Endpoint> {
    crate::runtime::validate_company_name(company)?;
    sqlx::query(
        "INSERT INTO restless_authority.hook_endpoints (company, provider, token) \
         VALUES ($1,$2,$3) ON CONFLICT (company, provider) DO NOTHING",
    )
    .bind(company)
    .bind(provider.name())
    .bind(mint_token(plane_id().as_deref()))
    .execute(pool)
    .await
    .context("mint hook endpoint")?;
    endpoint(pool, company, provider)
        .await?
        .context("hook endpoint vanished after it was minted")
}

/// The company a token names, for this provider only: a Resend token never routes an Airwallex
/// delivery.
async fn company_for_token(
    pool: &PgPool,
    provider: Provider,
    token: &str,
) -> Result<Option<String>> {
    if !is_token_shaped(token) {
        return Ok(None);
    }
    sqlx::query_scalar(
        "SELECT company FROM restless_authority.hook_endpoints WHERE provider=$1 AND token=$2",
    )
    .bind(provider.name())
    .bind(token)
    .fetch_optional(pool)
    .await
    .context("resolve hook token")
}

/// `p<plane>-<64 hex>` on a plane Cloud routes (the edge router finds the plane from the token,
/// and the plane id is already public in its own hostname), else `<64 hex>`. 256 random bits.
pub fn mint_token(plane: Option<&str>) -> String {
    let random = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    match plane {
        Some(plane) => format!("p{plane}-{random}"),
        None => random,
    }
}

fn is_token_shaped(token: &str) -> bool {
    let random = match token.strip_prefix('p') {
        Some(rest) => match rest.split_once('-') {
            Some((plane, random))
                if plane.len() == 32 && plane.bytes().all(|b| b.is_ascii_hexdigit()) =>
            {
                random
            }
            _ => return false,
        },
        None => token,
    };
    random.len() == 64
        && random
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// This plane's id as 32 hex digits, when Cloud runs it.
fn plane_id() -> Option<String> {
    let raw = std::env::var("RESTLESS_ENTRY_PLANE_ID").ok()?;
    let id = uuid::Uuid::parse_str(raw.trim()).ok()?;
    Some(id.simple().to_string())
}

/// Where providers reach this plane from the internet, without a trailing slash:
/// `RESTLESS_HOOKS_PUBLIC_BASE` when set (a self-hosted plane behind its own tunnel, a dev
/// tunnel), else the one public address Cloud routes this plane under, else the plane's own
/// network hostname. `None` for a local-only install: nothing on the internet can call it, and
/// incoming email arrives by polling instead.
pub fn public_base() -> Option<String> {
    let trimmed = |name: &str| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())
    };
    if let Some(base) = trimmed("RESTLESS_HOOKS_PUBLIC_BASE") {
        return Some(base);
    }
    if std::env::var("RESTLESS_ENTRY_MODE").as_deref() != Ok("network") {
        return None;
    }
    if let Some(origin) = trimmed("RESTLESS_PUBLIC_ORIGIN") {
        return Some(origin);
    }
    trimmed("RESTLESS_ENTRY_HOST").map(|host| format!("https://{host}"))
}

pub fn public_url(base: &str, provider: Provider, token: &str) -> String {
    format!("{base}{HOOKS_PREFIX}{}/{token}", provider.name())
}

/// Whether a path is the door, `/hooks/<provider>/<token>` and nothing deeper.
pub fn is_hooks_path(path: &str) -> bool {
    path.strip_prefix(HOOKS_PREFIX)
        .is_some_and(|rest| matches!(rest.split('/').collect::<Vec<_>>().as_slice(), [p, t] if !p.is_empty() && !t.is_empty()))
}

// ----- health -------------------------------------------------------------------------------

/// A verified event arrived.
pub async fn record_event(pool: &PgPool, company: &str, provider: Provider) -> Result<()> {
    sqlx::query(
        "UPDATE restless_authority.hook_endpoints SET last_event_at=now(), events=events+1 \
         WHERE company=$1 AND provider=$2",
    )
    .bind(company)
    .bind(provider.name())
    .execute(pool)
    .await
    .context("record hook event")?;
    Ok(())
}

/// A delivery for this company was refused or could not be recorded. `reason` is ours, never the
/// sender's text, and is shown to the owner.
pub async fn record_failure(
    pool: &PgPool,
    company: &str,
    provider: Provider,
    reason: &str,
) -> Result<()> {
    sqlx::query(
        "UPDATE restless_authority.hook_endpoints SET last_failure_at=now(), last_failure=$3, \
         failures=failures+1 WHERE company=$1 AND provider=$2",
    )
    .bind(company)
    .bind(provider.name())
    .bind(reason.chars().take(300).collect::<String>())
    .execute(pool)
    .await
    .context("record hook failure")?;
    Ok(())
}

// ----- the door -----------------------------------------------------------------------------

/// What the door answers. Bodies are fixed strings: a caller probing the endpoint learns nothing
/// about which check failed; the reason goes to the log and, when a company is known, its health.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    Accepted,
    NotFound,
    MethodNotAllowed,
    Unauthorized,
    TooLarge,
    BadRequest,
    RateLimited,
    Unavailable,
}

impl Answer {
    pub fn status(self) -> u16 {
        match self {
            Self::Accepted => 202,
            Self::NotFound => 404,
            Self::MethodNotAllowed => 405,
            Self::Unauthorized => 401,
            Self::TooLarge => 413,
            Self::BadRequest => 400,
            Self::RateLimited => 429,
            Self::Unavailable => 503,
        }
    }

    pub fn body(self) -> &'static str {
        match self {
            Self::Accepted => r#"{"status":"accepted"}"#,
            Self::NotFound => r#"{"error":"not found"}"#,
            Self::MethodNotAllowed => r#"{"error":"only POST is accepted"}"#,
            Self::Unauthorized => r#"{"error":"signature verification failed"}"#,
            Self::TooLarge => r#"{"error":"body too large"}"#,
            Self::BadRequest => r#"{"error":"invalid event"}"#,
            Self::RateLimited => r#"{"error":"slow down"}"#,
            Self::Unavailable => r#"{"error":"event not recorded"}"#,
        }
    }
}

/// Fixed-window counters per token and for the whole door.
#[derive(Default)]
pub struct RateLimiter {
    inner: Mutex<RateState>,
}

#[derive(Default)]
struct RateState {
    all: Option<(Instant, u32)>,
    tokens: HashMap<String, (Instant, u32)>,
}

impl RateLimiter {
    pub fn admit(&self, token: &str) -> bool {
        self.admit_at(token, Instant::now())
    }

    fn admit_at(&self, token: &str, now: Instant) -> bool {
        let mut state = self.inner.lock().expect("hook rate table poisoned");
        let bump = |slot: &mut Option<(Instant, u32)>, limit: u32| -> bool {
            match slot {
                Some((start, count)) if now.duration_since(*start) < RATE_WINDOW => {
                    if *count >= limit {
                        return false;
                    }
                    *count += 1;
                    true
                }
                _ => {
                    *slot = Some((now, 1));
                    true
                }
            }
        };
        if !bump(&mut state.all, ALL_TOKENS_PER_MINUTE) {
            return false;
        }
        if state.tokens.len() >= RATE_TABLE_LIMIT && !state.tokens.contains_key(token) {
            state.tokens.clear();
        }
        let mut slot = state.tokens.get(token).copied();
        let admitted = bump(&mut slot, PER_TOKEN_PER_MINUTE);
        if let Some(slot) = slot {
            state.tokens.insert(token.to_string(), slot);
        }
        admitted
    }
}

/// One request at the door. `header` looks a request header up by its lowercase name.
pub async fn receive(
    daemon: &Arc<crate::Daemon>,
    limiter: &RateLimiter,
    method: &str,
    path: &str,
    header: impl Fn(&str) -> Option<String>,
    body: &[u8],
) -> Answer {
    if method != "POST" {
        return Answer::MethodNotAllowed;
    }
    let Some((provider, token)) = path
        .strip_prefix(HOOKS_PREFIX)
        .and_then(|rest| rest.split_once('/'))
        .filter(|(_, token)| !token.contains('/'))
    else {
        return Answer::NotFound;
    };
    let Some(provider) = Provider::parse(provider) else {
        return Answer::NotFound;
    };
    if body.len() > MAX_BODY {
        return Answer::TooLarge;
    }
    if !limiter.admit(token) {
        return Answer::RateLimited;
    }
    let pool = daemon.authority.pool();
    let company = match company_for_token(pool, provider, token).await {
        Ok(Some(company)) => company,
        Ok(None) => return Answer::NotFound,
        Err(error) => {
            tracing::warn!(
                provider = provider.name(),
                "hook token lookup failed: {error:#}"
            );
            return Answer::Unavailable;
        }
    };
    let answer = match provider {
        Provider::Resend => receive_resend(daemon, &company, &header, body).await,
        Provider::Airwallex => receive_airwallex(daemon, &company, &header, body).await,
    };
    match &answer {
        Ok(()) => {
            if let Err(error) = record_event(pool, &company, provider).await {
                tracing::warn!(company, "hook event health not recorded: {error:#}");
            }
            Answer::Accepted
        }
        Err((answer, reason)) => {
            tracing::warn!(
                company,
                provider = provider.name(),
                reason,
                "hook delivery refused"
            );
            if let Err(error) = record_failure(pool, &company, provider, reason).await {
                tracing::warn!(company, "hook failure health not recorded: {error:#}");
            }
            *answer
        }
    }
}

type Refusal = (Answer, String);

async fn receive_resend(
    daemon: &Arc<crate::Daemon>,
    company: &str,
    header: &impl Fn(&str) -> Option<String>,
    body: &[u8],
) -> std::result::Result<(), Refusal> {
    let config = crate::runtime::CompanyConfig::load(&daemon.root, company)
        .map_err(|_| (Answer::NotFound, "the company no longer exists".to_string()))?;
    let secret = crate::credential::resolve_resend_webhook(&config)
        .await
        .map_err(|_| {
            (
                Answer::Unavailable,
                "the webhook signing secret could not be read from the Vault".to_string(),
            )
        })?;
    let svix_id = header("svix-id").unwrap_or_default();
    crate::ingress::verify_signature(
        &secret,
        &svix_id,
        &header("svix-timestamp").unwrap_or_default(),
        &header("svix-signature").unwrap_or_default(),
        body,
        Utc::now().timestamp(),
    )
    .map_err(|error| {
        (
            Answer::Unauthorized,
            format!("a delivery failed its signature check: {error}"),
        )
    })?;
    let parsed: serde_json::Value = serde_json::from_slice(body).map_err(|_| {
        (
            Answer::BadRequest,
            "a signed delivery was not JSON".to_string(),
        )
    })?;
    let event = crate::ingress::InboundEvent {
        provider_event_id: svix_id,
        company: company.to_string(),
        body: parsed,
    };
    crate::inbound::accept_resend(daemon, event, crate::inbound::Via::Webhook)
        .await
        .map(|_| ())
        .map_err(|error| {
            tracing::warn!(company, "verified Resend event not recorded: {error:#}");
            (
                Answer::Unavailable,
                "a verified event could not be recorded; Resend will retry it".to_string(),
            )
        })
}

async fn receive_airwallex(
    daemon: &Arc<crate::Daemon>,
    company: &str,
    header: &impl Fn(&str) -> Option<String>,
    body: &[u8],
) -> std::result::Result<(), Refusal> {
    let config = crate::runtime::CompanyConfig::load(&daemon.root, company)
        .map_err(|_| (Answer::NotFound, "the company no longer exists".to_string()))?;
    let observation = crate::airwallex::receive_webhook(
        &config,
        &daemon.authority,
        &header("x-timestamp").unwrap_or_default(),
        &header("x-signature").unwrap_or_default(),
        body,
    )
    .await
    .map_err(|error| {
        (
            Answer::Unauthorized,
            format!("an Airwallex delivery was not accepted: {error}"),
        )
    })?;
    if let Some(observation) = observation {
        crate::continue_after_payment_observation(
            daemon,
            company,
            &observation.payment,
            observation.changed,
        )
        .await
        .map_err(|error| {
            tracing::warn!(company, "Airwallex observation not continued: {error:#}");
            (
                Answer::Unavailable,
                "a verified payout event could not be applied; Airwallex will retry it".to_string(),
            )
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_opaque_and_shaped() {
        let plane = "0123456789abcdef0123456789abcdef";
        let routed = mint_token(Some(plane));
        assert!(routed.starts_with(&format!("p{plane}-")));
        assert!(is_token_shaped(&routed));
        let local = mint_token(None);
        assert_eq!(local.len(), 64);
        assert!(is_token_shaped(&local));
        assert_ne!(mint_token(None), mint_token(None));
        // A company name, a short guess, a path trick, upper case: never looked up.
        for bad in ["reno", "p123-abc", "../../etc", &local.to_uppercase(), ""] {
            assert!(!is_token_shaped(bad), "{bad}");
        }
    }

    #[test]
    fn only_the_door_itself_is_a_hooks_path() {
        assert!(is_hooks_path("/hooks/resend/abc"));
        assert!(!is_hooks_path("/hooks/resend"));
        assert!(!is_hooks_path("/hooks/resend/abc/more"));
        assert!(!is_hooks_path("/hooks//abc"));
        assert!(!is_hooks_path("/api/hooks/resend/abc"));
    }

    #[test]
    fn a_flood_on_one_token_is_slowed_without_starving_another() {
        let limiter = RateLimiter::default();
        let start = Instant::now();
        for _ in 0..PER_TOKEN_PER_MINUTE {
            assert!(limiter.admit_at("a", start));
        }
        assert!(!limiter.admit_at("a", start));
        assert!(limiter.admit_at("b", start));
        // The next window admits again.
        assert!(limiter.admit_at("a", start + RATE_WINDOW));
    }

    #[test]
    fn the_whole_door_is_bounded_too() {
        let limiter = RateLimiter::default();
        let start = Instant::now();
        for n in 0..ALL_TOKENS_PER_MINUTE {
            assert!(limiter.admit_at(&format!("t{n}"), start));
        }
        assert!(!limiter.admit_at("fresh", start));
    }

    const SECRET_A: &str = "whsec_dGVzdC1zZWNyZXQtZm9yLXNpZ25pbmctb25seQ==";

    fn secret_b() -> String {
        use base64::Engine as _;
        format!(
            "whsec_{}",
            base64::engine::general_purpose::STANDARD.encode(b"another-company-secret")
        )
    }

    fn sign(secret: &str, id: &str, timestamp: i64, body: &[u8]) -> String {
        use base64::Engine as _;
        use hmac::Mac as _;
        let key = base64::engine::general_purpose::STANDARD
            .decode(secret.strip_prefix("whsec_").unwrap())
            .unwrap();
        let mut mac = <hmac::Hmac<sha2::Sha256>>::new_from_slice(&key).unwrap();
        mac.update(format!("{id}.{timestamp}.").as_bytes());
        mac.update(body);
        format!(
            "v1,{}",
            base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
        )
    }

    fn received(email_id: &str, to: &str) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({
            "type": "email.received",
            "created_at": "2026-11-04T01:00:00.000Z",
            "data": {
                "email_id": email_id,
                "from": "Alex <admin@aleon.example>",
                "to": [to],
                "subject": "Re: Aleon Electrical's website",
            }
        }))
        .unwrap()
    }

    struct Door {
        daemon: Arc<crate::Daemon>,
        limiter: RateLimiter,
        root: std::path::PathBuf,
        companies: Vec<String>,
    }

    impl Door {
        async fn post(&self, path: &str, svix: (&str, i64, &str), body: &[u8]) -> Answer {
            let (id, timestamp, secret) = svix;
            let signature = sign(secret, id, timestamp, body);
            let headers: HashMap<&str, String> = [
                ("svix-id", id.to_string()),
                ("svix-timestamp", timestamp.to_string()),
                ("svix-signature", signature),
            ]
            .into();
            receive(
                &self.daemon,
                &self.limiter,
                "POST",
                path,
                |name| headers.get(name).cloned(),
                body,
            )
            .await
        }

        async fn records(&self, company: &str) -> Vec<serde_json::Value> {
            self.daemon
                .authority
                .inbound_after(company, 0, 1_000)
                .await
                .unwrap()
                .into_iter()
                .map(|record| record.body)
                .collect()
        }

        async fn close(self) {
            for company in &self.companies {
                sqlx::query("DELETE FROM restless_authority.hook_endpoints WHERE company=$1")
                    .bind(company)
                    .execute(self.daemon.authority.pool())
                    .await
                    .unwrap();
                self.daemon
                    .authority
                    .delete_test_company(company)
                    .await
                    .unwrap();
            }
            std::fs::remove_dir_all(&self.root).ok();
        }
    }

    async fn door() -> Option<Door> {
        let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
        let suffix = uuid::Uuid::new_v4().simple().to_string();
        let root = std::env::temp_dir().join(format!("restless-hooks-{suffix}"));
        std::fs::create_dir_all(root.join("companies")).unwrap();
        let mut companies = Vec::new();
        for (label, secret) in [("alpha", SECRET_A.to_string()), ("beta", secret_b())] {
            let company = format!("hk{label}{}_test", &suffix[..10]);
            let variable = format!("HOOKS_TEST_SECRET_{}", company.to_uppercase());
            // SAFETY: a variable unique to this test company, read only by its resolver.
            unsafe { std::env::set_var(&variable, &secret) };
            let mut config: crate::runtime::CompanyConfig =
                toml::from_str(&format!("name = '{company}'")).unwrap();
            config.credentials.insert(
                crate::credential::RESEND_WEBHOOK_BINDING.into(),
                format!("env:{variable}"),
            );
            crate::runtime::CompanyConfig::save(&root, &config).unwrap();
            companies.push(company);
        }
        let authority = crate::authority::AuthorityStore::connect(&database_url)
            .await
            .unwrap();
        let daemon = Arc::new(crate::Daemon {
            root: root.clone(),
            capabilities: crate::capability::CapabilityIssuer::open(&root).unwrap(),
            spend: crate::spend::SpendLedger::open(&root).unwrap(),
            publication: crate::publication::PublicationManager::new(&root, authority.clone())
                .unwrap(),
            launch: crate::launch::LaunchBroker::new(&root).unwrap(),
            authority,
            orgintel: crate::OrgIntelRegistry {
                database_url,
                root: root.clone(),
                handles: std::sync::Mutex::new(HashMap::new()),
            },
            staff: crate::staff::StaffRegistry::default(),
            activities: crate::activity::AgentActivityStreams::default(),
            cell_wakes: crate::cell_wake::CellWakeHub::default(),
            runtime_bridges: crate::runtime_bridge::RuntimeBridgeRegistry::default(),
            lifecycle: restless_contracts::appliance::LifecycleGate::default(),
            in_flight: Arc::new(std::sync::Mutex::new(crate::schedule::WakeClaims::default())),
            schedule_wake: Arc::new(tokio::sync::Notify::new()),
        });
        Some(Door {
            daemon,
            limiter: RateLimiter::default(),
            root,
            companies,
        })
    }

    /// Signatures, replays, the wrong company, unknown tokens, size, method, and that a received
    /// email is one record whichever delivery or path brought it.
    #[tokio::test]
    async fn the_door_records_only_verified_events_under_the_company_its_token_names() {
        let Some(door) = door().await else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping the hooks door scenario");
            return;
        };
        let pool = door.daemon.authority.pool().clone();
        let (alpha, beta) = (door.companies[0].clone(), door.companies[1].clone());
        let a = ensure_endpoint(&pool, &alpha, Provider::Resend)
            .await
            .unwrap();
        let b = ensure_endpoint(&pool, &beta, Provider::Resend)
            .await
            .unwrap();
        assert_eq!(
            ensure_endpoint(&pool, &alpha, Provider::Resend)
                .await
                .unwrap()
                .token,
            a.token
        );
        let path_a = format!("/hooks/resend/{}", a.token);
        let path_b = format!("/hooks/resend/{}", b.token);
        let now = Utc::now().timestamp();
        let body = received("em_1", "hi@alpha.example");

        assert_eq!(
            door.post(&path_a, ("msg_1", now, SECRET_A), &body).await,
            Answer::Accepted
        );
        let records = door.records(&alpha).await;
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0]["provider_event_id"],
            "resend:email.received:em_1"
        );
        assert_eq!(records[0]["provider_delivery_id"], "msg_1");

        // A replay of the same delivery, and Resend's redelivery under a new message id, are the
        // same received email: one record.
        assert_eq!(
            door.post(&path_a, ("msg_1", now, SECRET_A), &body).await,
            Answer::Accepted
        );
        assert_eq!(
            door.post(&path_a, ("msg_2", now, SECRET_A), &body).await,
            Answer::Accepted
        );
        assert_eq!(door.records(&alpha).await.len(), 1);
        // The poll finding it later is the same record too.
        let polled = crate::inbound::accept_resend(
            &door.daemon,
            crate::ingress::InboundEvent {
                provider_event_id: "poll:em_1".into(),
                company: alpha.clone(),
                body: serde_json::from_slice(&body).unwrap(),
            },
            crate::inbound::Via::Poll,
        )
        .await
        .unwrap();
        assert!(matches!(
            polled,
            crate::inbound::Accepted::Recorded {
                inserted: false,
                ..
            }
        ));

        // A captured delivery replayed an hour later carries a genuine signature and is refused.
        let stale = received("em_2", "hi@alpha.example");
        assert_eq!(
            door.post(&path_a, ("msg_3", now - 3_600, SECRET_A), &stale)
                .await,
            Answer::Unauthorized
        );
        // Another company's secret never files an event here, nor does a forged one.
        assert_eq!(
            door.post(&path_a, ("msg_4", now, &secret_b()), &stale)
                .await,
            Answer::Unauthorized
        );
        assert_eq!(
            door.post(&path_b, ("msg_5", now, SECRET_A), &stale).await,
            Answer::Unauthorized
        );
        assert_eq!(door.records(&alpha).await.len(), 1);
        assert!(door.records(&beta).await.is_empty());
        assert_eq!(
            door.post(&path_b, ("msg_6", now, &secret_b()), &stale)
                .await,
            Answer::Accepted
        );
        assert_eq!(door.records(&beta).await.len(), 1);

        // Names, guesses and the other provider's door find nothing.
        for path in [
            format!("/hooks/resend/{alpha}"),
            format!("/hooks/resend/{}", mint_token(None)),
            format!("/hooks/airwallex/{}", a.token),
            format!("/hooks/stripe/{}", a.token),
            format!("{path_a}/more"),
        ] {
            assert_eq!(
                door.post(&path, ("msg_7", now, SECRET_A), &stale).await,
                Answer::NotFound,
                "{path}"
            );
        }
        let oversize = vec![b' '; MAX_BODY + 1];
        assert_eq!(
            door.post(&path_a, ("msg_8", now, SECRET_A), &oversize)
                .await,
            Answer::TooLarge
        );
        assert_eq!(
            receive(&door.daemon, &door.limiter, "GET", &path_a, |_| None, b"").await,
            Answer::MethodNotAllowed
        );

        // Health: deliveries counted, the refusals remembered in the company's own words.
        let health = endpoint(&pool, &alpha, Provider::Resend)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(health.events, 3);
        assert!(health.last_event_at.is_some());
        assert_eq!(health.failures, 2);
        assert!(health.last_failure.unwrap().contains("signature check"));

        // Mail for another company's domain on a shared Resend account is answered, not recorded.
        sqlx::query(
            "UPDATE restless_authority.hook_endpoints SET receiving_domains=$2 WHERE company=$1",
        )
        .bind(&alpha)
        .bind(serde_json::json!(["alpha.example"]))
        .execute(&pool)
        .await
        .unwrap();
        let elsewhere = received("em_3", "Someone <someone@other.example>");
        assert_eq!(
            door.post(&path_a, ("msg_9", now, SECRET_A), &elsewhere)
                .await,
            Answer::Accepted
        );
        assert_eq!(door.records(&alpha).await.len(), 1);
        door.close().await;
    }

    #[test]
    fn the_public_base_prefers_the_explicit_then_the_one_address_then_the_plane() {
        // Pure on the values the function reads; set and restore in one test so no other test
        // in this binary observes them.
        let names = [
            "RESTLESS_HOOKS_PUBLIC_BASE",
            "RESTLESS_ENTRY_MODE",
            "RESTLESS_PUBLIC_ORIGIN",
            "RESTLESS_ENTRY_HOST",
        ];
        let saved: Vec<_> = names.iter().map(|name| std::env::var_os(name)).collect();
        // SAFETY: test-local variables that only `public_base` reads.
        unsafe {
            for name in names {
                std::env::remove_var(name);
            }
            assert_eq!(public_base(), None);
            std::env::set_var("RESTLESS_ENTRY_MODE", "network");
            std::env::set_var("RESTLESS_ENTRY_HOST", "owner-ab.restless.run");
            assert_eq!(
                public_base().as_deref(),
                Some("https://owner-ab.restless.run")
            );
            std::env::set_var("RESTLESS_PUBLIC_ORIGIN", "https://restless.run/");
            assert_eq!(public_base().as_deref(), Some("https://restless.run"));
            std::env::set_var("RESTLESS_HOOKS_PUBLIC_BASE", "https://tunnel.example.test");
            assert_eq!(
                public_base().as_deref(),
                Some("https://tunnel.example.test")
            );
            for (name, value) in names.iter().zip(saved) {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
        assert_eq!(
            public_url("https://restless.run", Provider::Resend, "abc"),
            "https://restless.run/hooks/resend/abc"
        );
    }
}
