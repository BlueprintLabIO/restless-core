//! The account broker (S98-02, S98-03, S98-06): companies use the owner's accounts without ever
//! holding them.
//!
//! An account granted to a company appears in its actors' tool gateway as one tool per account,
//! `acct_<name>__request`. A call names a method and a path; the broker decides its tier from the
//! kind of account and the method (an agent may only raise it), checks the grant's abilities and
//! limits, attaches the credential here on the host, and answers with the response, the credential
//! cut out of it. Calls that act go through the effect runner, so they carry a receipt and an
//! idempotency key, and a tier that asks first waits in the owner's Attention like any reserved
//! tool. Every call is recorded as a use, and a first use, a first spend, a new destination or a
//! burst raises one notice the owner settles with Fine or Freeze.

use std::time::Duration;

use anyhow::{bail, Context, Result};
use base64::Engine as _;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};

use crate::accounts::{Account, Grant, Who};

/// Kinds whose credential the broker attaches to HTTP calls.
pub const BROKERED_KINDS: &[&str] = &["api", "github"];
pub const TOOL_PREFIX: &str = "acct_";
const TOOL_SUFFIX: &str = "__request";
const GITHUB_API: &str = "https://api.github.com";
const CALL_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_BODY_CHARS: usize = 60_000;
/// Uses in one hour that count as a burst once they are also well above the usual rate.
const BURST_FLOOR: i64 = 30;
const BURST_FACTOR: f64 = 5.0;
pub const TIERS: &[&str] = &["read", "draft", "act", "spend", "admin"];
/// Request headers an agent may set; anything that could carry or replace a credential is dropped.
const ALLOWED_HEADERS: &[&str] = &["accept", "content-type", "x-github-api-version", "if-match", "if-none-match", "idempotency-key"];

pub async fn ensure_schema(pool: &PgPool) -> Result<()> {
    for statement in [
        "CREATE TABLE IF NOT EXISTS restless_authority.owner_account_notices (\
           id TEXT PRIMARY KEY, account_id TEXT NOT NULL, grant_id TEXT, company TEXT NOT NULL, \
           use_id BIGINT NOT NULL, kind TEXT NOT NULL, state TEXT NOT NULL DEFAULT 'open', \
           created_at TIMESTAMPTZ NOT NULL DEFAULT now(), decided_at TIMESTAMPTZ, decided_by TEXT)",
        "CREATE INDEX IF NOT EXISTS owner_account_notices_by_account \
           ON restless_authority.owner_account_notices (account_id, state)",
        "CREATE INDEX IF NOT EXISTS owner_account_uses_by_company \
           ON restless_authority.owner_account_uses (company, at DESC)",
        "CREATE TABLE IF NOT EXISTS restless_authority.owner_account_requests (\
           id TEXT PRIMARY KEY, company TEXT NOT NULL, account_id TEXT, kind TEXT, \
           abilities JSONB NOT NULL, reason TEXT NOT NULL DEFAULT '', requested_by TEXT NOT NULL, \
           state TEXT NOT NULL DEFAULT 'open', grant_id TEXT, \
           created_at TIMESTAMPTZ NOT NULL DEFAULT now(), decided_at TIMESTAMPTZ)",
    ] {
        sqlx::query(statement).execute(pool).await.context("create account broker tables")?;
    }
    Ok(())
}

fn rank(tier: &str) -> usize {
    TIERS.iter().position(|known| *known == tier).unwrap_or(2)
}

/// The tier a call implies from the account's kind, its method and its path.
pub fn implied_tier(kind: &str, method: &str, path: &str) -> &'static str {
    let method = method.to_ascii_uppercase();
    if matches!(method.as_str(), "GET" | "HEAD" | "OPTIONS") {
        return "read";
    }
    if kind == "github" {
        let path = path.split('?').next().unwrap_or_default().trim_end_matches('/');
        let parts = path.trim_start_matches('/').split('/').collect::<Vec<_>>();
        let repo_level = parts.first() == Some(&"repos") && parts.len() == 3;
        let admin_area = ["collaborators", "hooks", "keys", "secrets", "invitations", "transfer", "branches"]
            .iter()
            .any(|area| parts.iter().skip(3).any(|part| part == area))
            && !(method == "POST" && path.ends_with("/branches")); // creating a branch is ordinary work
        if (repo_level && matches!(method.as_str(), "PATCH" | "DELETE")) || admin_area {
            return "admin";
        }
        if path.ends_with("/merge") || path.ends_with("/merges") {
            return "act";
        }
        // Proposals: a pull request, an issue, a comment or review, a branch, a draft release.
        let last = parts.last().copied().unwrap_or_default();
        if method == "POST" && matches!(last, "pulls" | "issues" | "comments" | "reviews" | "gists") {
            return "draft";
        }
    }
    "act"
}

/// The higher of what the call implies and what the caller declared: declaring can only add care.
pub fn call_tier(kind: &str, method: &str, path: &str, declared: Option<&str>) -> &'static str {
    let implied = implied_tier(kind, method, path);
    match declared.and_then(|declared| TIERS.iter().find(|tier| **tier == declared)) {
        Some(declared) if rank(declared) > rank(implied) => declared,
        _ => implied,
    }
}

/// What a call touches, for scope, notices and the use log: `owner/repo` on GitHub, else the path.
pub fn call_target(kind: &str, path: &str) -> Option<String> {
    let path = path.split('?').next().unwrap_or_default();
    if kind == "github" {
        let parts = path.trim_start_matches('/').split('/').collect::<Vec<_>>();
        return (parts.first() == Some(&"repos") && parts.len() >= 3)
            .then(|| format!("{}/{}", parts[1], parts[2]).to_ascii_lowercase());
    }
    (!path.is_empty()).then(|| path.to_string())
}

/// A scope entry matches a GitHub repo exactly or as `owner/*`, and anything else as a prefix.
pub fn in_scope(kind: &str, scope: &[String], target: Option<&str>, tier: &str) -> bool {
    if scope.is_empty() {
        return true;
    }
    let Some(target) = target else {
        // Account-wide reads (who am I, my repos) stay open; anything else must name its target.
        return tier == "read";
    };
    scope.iter().map(|entry| entry.trim().to_ascii_lowercase()).any(|entry| {
        if kind == "github" {
            entry == target || entry.strip_suffix("/*").is_some_and(|owner| target.split('/').next() == Some(owner))
        } else {
            target.to_ascii_lowercase().starts_with(&entry)
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Run,
    /// Runs, and the owner reads about it in the use log and the receipt.
    RunAndTell,
    /// Waits for the owner's approval in Attention.
    Ask(String),
    Refuse(String),
}

/// What the grant says about this call. Pure, so each rule is tested on its own.
#[allow(clippy::too_many_arguments)]
pub fn decide(
    account: &Account,
    grant: &Grant,
    actor: &str,
    tier: &str,
    in_scope: bool,
    cost_micros: Option<i64>,
    spent_in_period: i64,
    actions_today: i64,
    now: DateTime<Utc>,
) -> Decision {
    if account.frozen {
        return Decision::Refuse(format!("{} is frozen by the owner; nothing is sent until they unfreeze it.", account.label));
    }
    if grant.frozen {
        return Decision::Refuse(format!("This company's use of {} is frozen by the owner.", account.label));
    }
    if grant.limits.expires_at.is_some_and(|expiry| expiry <= now) {
        return Decision::Refuse(format!("This company's grant of {} has expired. Ask the owner to renew it.", account.label));
    }
    if let Who::Actors { actors } = &grant.who {
        if !actors.iter().any(|named| named == actor) {
            return Decision::Refuse(format!("{} is granted to named people and agents in this company, and you are not one of them.", account.label));
        }
    }
    if !in_scope {
        return Decision::Refuse(format!(
            "That is outside what this company may touch with {} ({}). Ask the owner to widen the grant.",
            account.label,
            grant.limits.scope.join(", ")
        ));
    }
    if tier != "read" {
        if let Some(quiet) = &grant.limits.quiet_hours {
            if in_quiet_hours(quiet, now) {
                return Decision::Refuse(format!(
                    "{} is in quiet hours ({} to {} {}); reads work, anything else waits until they end.",
                    account.label, quiet.start, quiet.end, quiet.tz
                ));
            }
        }
    }
    let abilities = &grant.abilities;
    let mode = match tier {
        "read" => &abilities.read,
        "draft" => &abilities.draft,
        "act" => &abilities.act,
        "spend" => &abilities.spend,
        _ => &abilities.admin,
    };
    if mode == "off" {
        return Decision::Refuse(format!(
            "This company may not {tier} with {}. Ask the owner for more (the company's Accounts page).",
            account.label
        ));
    }
    if tier != "read" {
        if let Some(cap) = grant.limits.actions_per_day {
            if actions_today >= cap {
                return Decision::Ask(format!("{} has used its {cap} actions today with {}; this one asks the owner.", grant.company, account.label));
            }
        }
    }
    if tier == "spend" {
        let Some(cost) = cost_micros else {
            return Decision::Ask("A spend that does not say what it costs asks the owner first.".into());
        };
        return match &grant.spend_cap {
            Some(cap) if spent_in_period.saturating_add(cost) <= cap.amount_micros => Decision::RunAndTell,
            Some(cap) => Decision::Ask(format!(
                "This spend would pass the {} cap of {:.2} per {}; it asks the owner.",
                account.label,
                cap.amount_micros as f64 / 1_000_000.0,
                cap.period
            )),
            None if mode == "allow" => Decision::RunAndTell,
            None => Decision::Ask(format!("Spending with {} asks the owner first.", account.label)),
        };
    }
    match mode.as_str() {
        "allow" => Decision::Run,
        "tell" => Decision::RunAndTell,
        _ => Decision::Ask(format!("To {tier} with {} this company asks the owner first.", account.label)),
    }
}

fn in_quiet_hours(quiet: &crate::accounts::QuietHours, now: DateTime<Utc>) -> bool {
    let parse = |value: &str| chrono::NaiveTime::parse_from_str(value.trim(), "%H:%M").ok();
    let (Some(start), Some(end)) = (parse(&quiet.start), parse(&quiet.end)) else {
        return false;
    };
    let local = match quiet.tz.parse::<chrono_tz::Tz>() {
        Ok(tz) => now.with_timezone(&tz).time(),
        Err(_) => now.time(),
    };
    if start <= end {
        local >= start && local < end
    } else {
        local >= start || local < end
    }
}

/// An account this company may call through the broker, with the credential the call will use.
#[derive(Clone)]
pub struct Usable {
    pub account: Account,
    pub grant: Grant,
    pub credential_id: String,
    settings: serde_json::Value,
    secrets: serde_json::Map<String, serde_json::Value>,
}

impl Usable {
    pub fn tool_name(&self) -> String {
        tool_name(&self.account)
    }
}

pub fn tool_name(account: &Account) -> String {
    let mut slug = account
        .label
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    while slug.contains("__") {
        slug = slug.replace("__", "_");
    }
    let slug = slug.trim_matches('_').chars().take(24).collect::<String>();
    let short = account.id.chars().take(6).collect::<String>();
    if slug.is_empty() {
        format!("{TOOL_PREFIX}{short}{TOOL_SUFFIX}")
    } else {
        format!("{TOOL_PREFIX}{slug}_{short}{TOOL_SUFFIX}")
    }
}

pub fn is_account_tool(name: &str) -> bool {
    name.starts_with(TOOL_PREFIX)
        && [TOOL_SUFFIX, GIT_SUFFIX, BROWSER_SUFFIX, FILL_SUFFIX].iter().any(|suffix| name.ends_with(suffix))
}

const BROWSER_SUFFIX: &str = "__browser";
const FILL_SUFFIX: &str = "__fill";

pub fn is_browser_tool(name: &str) -> bool {
    name.starts_with(TOOL_PREFIX) && (name.ends_with(BROWSER_SUFFIX) || name.ends_with(FILL_SUFFIX))
}

/// The browser tools a website-login or browser-profile account adds (S98-04).
pub fn browser_tool_definitions(usable: &Usable) -> Vec<(String, String, serde_json::Value)> {
    let account = &usable.account;
    if !crate::account_browser::BROWSER_KINDS.contains(&account.kind.as_str()) {
        return Vec::new();
    }
    let base = tool_name(account);
    let domains = crate::account_browser::account_domains(&usable.settings).join(", ");
    let mut tools = vec![(
        base.replace(TOOL_SUFFIX, BROWSER_SUFFIX),
        format!(
            "[{} · the owner's signed-in browser for {domains}, granted to this company] Borrow it: answers a DevTools endpoint to drive with Playwright \
             (`chromium.connectOverCDP(endpoint)`) or any CDP client, already signed in. One session at a time holds it for up to 30 minutes; \
             `release: true` hands it back. Cookies and site storage cannot be read through it. Needs `purpose`.",
            account.label
        ),
        serde_json::json!({
            "type": "object",
            "properties": {
                "purpose": {"type": "string", "description": "One sentence: why you need the account's browser."},
                "url": {"type": "string", "description": "Optional page to open first, on the account's own domains."},
                "release": {"type": "boolean", "description": "True to hand the browser back when you are done."}
            }
        }),
    )];
    if account.kind == "website_login" {
        tools.push((
            base.replace(TOOL_SUFFIX, FILL_SUFFIX),
            format!(
                "[{} · sign-in] Type the account's username, password or current one-time code into the focused field of the borrowed browser, \
                 only on {domains}. The value is typed by Restless and never shown to you. Click into the field first. \
                 A code the site sent to a phone or app comes from the owner: use field `code` with `ask_owner: true`.",
                account.label
            ),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "field": {"type": "string", "enum": ["username", "password", "code"]},
                    "target_id": {"type": "string", "description": "Optional DevTools target id of the page; defaults to the page you are attached to."},
                    "ask_owner": {"type": "boolean", "description": "For `code`: the site sent the code to a phone, email or app, so the owner must give it. They answer in their Inbox and Restless types it here."}
                },
                "required": ["field"]
            }),
        ));
    }
    tools
}

/// What one browser account keeps: its settings, and the vault reference for a secret field.
impl Usable {
    pub fn settings(&self) -> &serde_json::Value {
        &self.settings
    }

    pub fn secret_reference(&self, field: &str) -> Option<&str> {
        self.secrets.get(field).and_then(|value| value.as_str())
    }
}

const GIT_SUFFIX: &str = "__git_remote";

/// The git tool a GitHub account adds: it answers a remote for one repository.
pub fn git_tool_name(account: &Account) -> Option<String> {
    (account.kind == "github").then(|| tool_name(account).replace(TOOL_SUFFIX, GIT_SUFFIX))
}

pub fn git_tool_definition(usable: &Usable) -> Option<(String, String, serde_json::Value)> {
    let name = git_tool_name(&usable.account)?;
    let scope = if usable.grant.limits.scope.is_empty() {
        String::new()
    } else {
        format!(" Only: {}.", usable.grant.limits.scope.join(", "))
    };
    let description = format!(
        "[{} · the owner's GitHub account, granted to this company] A git remote URL for one repository, to clone, fetch and push as this account. \
         The GitHub token never reaches this computer; the URL holds only this session's pass and stops working when the session ends (ask again). \
         Pushing a branch drafts; pushing the default branch, a tag, or deleting acts.{scope}",
        usable.account.label
    );
    let schema = serde_json::json!({
        "type": "object",
        "properties": { "repo": {"type": "string", "description": "owner/repo, such as yail259/frankensurf"} },
        "required": ["repo"]
    });
    Some((name, description, schema))
}

/// The brokered accounts granted to this company that this actor may name. Frozen and expired
/// grants are still listed, so a call explains itself instead of the tool vanishing.
pub async fn usable(pool: &PgPool, company: &str, actor: &str) -> Result<Vec<Usable>> {
    let mut out = Vec::new();
    for (grant, account) in crate::accounts::company_grants(pool, company).await? {
        if !BROKERED_KINDS.contains(&account.kind.as_str())
            && !crate::account_browser::BROWSER_KINDS.contains(&account.kind.as_str())
        {
            continue;
        }
        if let Who::Actors { actors } = &grant.who {
            if !actors.iter().any(|named| named == actor) {
                continue;
            }
        }
        let credential = sqlx::query(
            "SELECT id, settings, secrets FROM restless_authority.owner_account_credentials \
             WHERE account_id = $1 ORDER BY pinned DESC, updated_at DESC LIMIT 1",
        )
        .bind(&account.id)
        .fetch_optional(pool)
        .await?;
        let Some(credential) = credential else { continue };
        let secrets = match credential.try_get::<serde_json::Value, _>("secrets")? {
            serde_json::Value::Object(secrets) => secrets,
            _ => serde_json::Map::new(),
        };
        out.push(Usable {
            credential_id: credential.try_get("id")?,
            settings: credential.try_get("settings")?,
            secrets,
            account,
            grant,
        });
    }
    Ok(out)
}

/// The tool an actor sees for one granted account.
pub fn tool_definition(usable: &Usable) -> (String, String, serde_json::Value) {
    let account = &usable.account;
    let abilities = &usable.grant.abilities;
    let base = match account.kind.as_str() {
        "github" => GITHUB_API.to_string(),
        _ => usable.settings.get("base_url").and_then(|value| value.as_str()).unwrap_or_default().to_string(),
    };
    let scope = if usable.grant.limits.scope.is_empty() {
        String::new()
    } else {
        format!(" Only: {}.", usable.grant.limits.scope.join(", "))
    };
    let description = format!(
        "[{} · the owner's {} account, granted to this company] Call {base} as this account. \
         The credential is attached for you and never shown. Reads {}; drafts {}; acting {}; spending {}; admin {}.{scope} \
         Anything but a read needs `purpose`. Say `tier: \"spend\"` and `cost` for anything that costs money.",
        account.label,
        if account.kind == "github" { "GitHub" } else { "API" },
        mode_words(&abilities.read),
        mode_words(&abilities.draft),
        mode_words(&abilities.act),
        mode_words(&abilities.spend),
        mode_words(&abilities.admin),
    );
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "method": {"type": "string", "enum": ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD"]},
            "path": {"type": "string", "description": format!("Path under {base}, starting with /, with any query string.")},
            "body": {"description": "JSON body, or a string sent as is."},
            "headers": {"type": "object", "additionalProperties": {"type": "string"}, "description": "Optional: accept, content-type and similar. Credentials are never accepted here."},
            "purpose": {"type": "string", "description": "One sentence: why. Required for anything but a read."},
            "tier": {"type": "string", "enum": ["draft", "act", "spend", "admin"], "description": "Raise the tier when the call does more than its method suggests, such as a POST that pays."},
            "cost": {"type": "number", "description": "What the call spends, in the account's currency (for a spend)."},
            "key": {"type": "string", "description": "Reuse to make a retry the same call; omit to derive one."}
        },
        "required": ["method", "path"]
    });
    (usable.tool_name(), description, schema)
}

fn mode_words(mode: &str) -> &'static str {
    match mode {
        "allow" => "run",
        "tell" => "run and the owner is told",
        "ask" => "ask the owner first",
        _ => "are not allowed",
    }
}

/// One call an actor asked for, validated.
#[derive(Debug, Clone)]
pub struct Call {
    pub method: reqwest::Method,
    pub path: String,
    pub body: Option<serde_json::Value>,
    pub headers: Vec<(String, String)>,
    pub purpose: String,
    pub declared_tier: Option<String>,
    pub cost_micros: Option<i64>,
    pub key: Option<String>,
}

pub fn parse_call(arguments: &serde_json::Map<String, serde_json::Value>) -> Result<Call> {
    let text = |name: &str| arguments.get(name).and_then(|value| value.as_str()).map(str::trim).filter(|value| !value.is_empty());
    let method = text("method").context("`method` is required: GET, POST, PUT, PATCH, DELETE or HEAD")?;
    let method = reqwest::Method::from_bytes(method.to_ascii_uppercase().as_bytes()).context("unknown method")?;
    let path = text("path").context("`path` is required, starting with /")?;
    if !path.starts_with('/') || path.starts_with("//") || path.contains("..") || path.contains('\\') || path.contains('@') {
        bail!("`path` is a path under the account's address, starting with / (not a full URL)");
    }
    let mut headers = Vec::new();
    if let Some(serde_json::Value::Object(given)) = arguments.get("headers") {
        for (name, value) in given {
            let name = name.to_ascii_lowercase();
            if !ALLOWED_HEADERS.contains(&name.as_str()) {
                bail!("the header {name} is not accepted; the broker attaches the credential itself");
            }
            headers.push((name, value.as_str().unwrap_or_default().to_string()));
        }
    }
    let cost_micros = arguments
        .get("cost")
        .and_then(|value| value.as_f64())
        .filter(|cost| cost.is_finite() && *cost >= 0.0)
        .map(|cost| (cost * 1_000_000.0).round() as i64);
    Ok(Call {
        method,
        path: path.to_string(),
        body: arguments.get("body").cloned().filter(|body| !body.is_null()),
        headers,
        purpose: text("purpose").unwrap_or_default().to_string(),
        declared_tier: text("tier").map(str::to_string),
        cost_micros,
        key: text("key").map(str::to_string),
    })
}

/// Usage this grant has already had, for its caps.
pub async fn grant_usage(pool: &PgPool, grant: &Grant, now: DateTime<Utc>) -> Result<(i64, i64)> {
    let period = match grant.spend_cap.as_ref().map(|cap| cap.period.as_str()) {
        Some("day") => chrono::Duration::days(1),
        Some("week") => chrono::Duration::weeks(1),
        _ => chrono::Duration::days(30),
    };
    let row = sqlx::query(
        "SELECT COALESCE(SUM(cost_micros) FILTER (WHERE tier = 'spend' AND at > $2), 0)::BIGINT AS spent, \
                COUNT(*) FILTER (WHERE tier <> 'read' AND at > $3)::BIGINT AS today \
         FROM restless_authority.owner_account_uses WHERE grant_id = $1 AND result = 'ok'",
    )
    .bind(&grant.id)
    .bind(now - period)
    .bind(now - chrono::Duration::days(1))
    .fetch_one(pool)
    .await?;
    Ok((row.try_get("spent")?, row.try_get("today")?))
}

/// What the upstream answered, the credential already cut out.
#[derive(Debug, Serialize)]
pub struct Answer {
    pub status: u16,
    pub headers: serde_json::Map<String, serde_json::Value>,
    pub body: serde_json::Value,
}

/// The account's key, read from the vault here and only here, before anything is sent.
pub async fn secret(usable: &Usable) -> Result<String> {
    let reference = usable
        .secrets
        .get("secret")
        .and_then(|value| value.as_str())
        .context("this account has no key to send")?;
    crate::credential::resolve_reference(reference)
        .await
        .context("the vault did not give the account's key; nothing was sent")
}

/// Send the call as the account.
pub async fn send(usable: &Usable, call: &Call, secret: &str) -> Result<Answer> {
    let secret = secret.to_string();
    let (base, auth, auth_name) = match usable.account.kind.as_str() {
        "github" => (GITHUB_API.to_string(), "bearer".to_string(), None),
        _ => (
            usable.settings.get("base_url").and_then(|value| value.as_str()).unwrap_or_default().to_string(),
            usable.settings.get("auth").and_then(|value| value.as_str()).unwrap_or("bearer").to_string(),
            usable.settings.get("auth_name").and_then(|value| value.as_str()).map(str::to_string),
        ),
    };
    let base = url::Url::parse(&base).context("the account's address is not a URL")?;
    let mut url = base.clone();
    let (path, query) = call.path.split_once('?').unwrap_or((&call.path, ""));
    let joined = format!("{}{}", base.path().trim_end_matches('/'), path);
    url.set_path(&joined);
    url.set_query((!query.is_empty()).then_some(query));
    if url.host_str() != base.host_str() || url.scheme() != base.scheme() {
        bail!("the call left the account's own address");
    }
    if auth == "query" {
        url.query_pairs_mut().append_pair(auth_name.as_deref().unwrap_or("key"), &secret);
    }
    let client = reqwest::Client::builder()
        .timeout(CALL_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("Restless-Account-Broker")
        .build()?;
    let mut request = client.request(call.method.clone(), url);
    request = match auth.as_str() {
        "bearer" => request.bearer_auth(&secret),
        "header" => request.header(auth_name.as_deref().unwrap_or("x-api-key"), &secret),
        "basic" => {
            let (user, password) = secret.split_once(':').unwrap_or((secret.as_str(), ""));
            request.basic_auth(user, Some(password))
        }
        _ => request,
    };
    if usable.account.kind == "github" {
        request = request.header("accept", "application/vnd.github+json").header("x-github-api-version", "2022-11-28");
    }
    for (name, value) in &call.headers {
        request = request.header(name, value);
    }
    request = match &call.body {
        Some(serde_json::Value::String(text)) => request.body(text.clone()),
        Some(body) => request.json(body),
        None => request,
    };
    let response = request.send().await.context("the account's service did not answer")?;
    let status = response.status().as_u16();
    let mut headers = serde_json::Map::new();
    for name in ["content-type", "link", "location", "x-ratelimit-remaining", "retry-after"] {
        if let Some(value) = response.headers().get(name).and_then(|value| value.to_str().ok()) {
            headers.insert(name.into(), serde_json::Value::String(redact(value, &secret)));
        }
    }
    let text = redact(&response.text().await.unwrap_or_default(), &secret);
    let body = match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(json) if text.len() <= MAX_BODY_CHARS => json,
        _ => {
            let total = text.chars().count();
            let mut cut = text.chars().take(MAX_BODY_CHARS).collect::<String>();
            if total > MAX_BODY_CHARS {
                cut.push_str(&format!("\n[Cut to {MAX_BODY_CHARS} of {total} characters. Ask for less: a narrower path or a page.]"));
            }
            serde_json::Value::String(cut)
        }
    };
    Ok(Answer { status, headers, body })
}

/// The credential, and its encoded forms, never leave in an answer.
pub fn redact(text: &str, secret: &str) -> String {
    if secret.len() < 4 {
        return text.to_string();
    }
    let mut out = text.replace(secret, "[credential]");
    let encoded = base64::engine::general_purpose::STANDARD.encode(secret);
    if encoded.len() >= 8 {
        out = out.replace(&encoded, "[credential]");
    }
    out
}

/// Record one use and raise at most one notice about it.
#[allow(clippy::too_many_arguments)]
pub async fn record_use(
    pool: &PgPool,
    usable: &Usable,
    company: &str,
    actor: &str,
    directed_by: Option<&str>,
    action: &str,
    tier: &str,
    target: Option<&str>,
    result: &str,
    cost_micros: Option<i64>,
) -> Result<i64> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO restless_authority.owner_account_uses \
         (account_id, grant_id, company, actor, directed_by, action, tier, target, result, cost_micros) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING id",
    )
    .bind(&usable.account.id)
    .bind(&usable.grant.id)
    .bind(company)
    .bind(actor)
    .bind(directed_by)
    .bind(action.chars().take(300).collect::<String>())
    .bind(tier)
    .bind(target)
    .bind(result)
    .bind(cost_micros)
    .fetch_one(pool)
    .await?;
    if result == "ok" {
        if let Some(kind) = notice_kind(pool, &usable.account.id, &usable.grant.id, company, tier, target, id).await? {
            sqlx::query(
                "INSERT INTO restless_authority.owner_account_notices (id, account_id, grant_id, company, use_id, kind) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(uuid::Uuid::new_v4().simple().to_string())
            .bind(&usable.account.id)
            .bind(&usable.grant.id)
            .bind(company)
            .bind(id)
            .bind(kind)
            .execute(pool)
            .await?;
        }
    }
    Ok(id)
}

async fn notice_kind(
    pool: &PgPool,
    account: &str,
    grant: &str,
    company: &str,
    tier: &str,
    target: Option<&str>,
    this_use: i64,
) -> Result<Option<&'static str>> {
    let row = sqlx::query(
        "SELECT \
           COUNT(*) FILTER (WHERE result = 'ok')::BIGINT AS before, \
           COUNT(*) FILTER (WHERE result = 'ok' AND tier = 'spend')::BIGINT AS spends, \
           COUNT(*) FILTER (WHERE result = 'ok' AND target IS NOT DISTINCT FROM $4 AND tier <> 'read')::BIGINT AS same_target, \
           COUNT(*) FILTER (WHERE at > now() - interval '1 hour')::BIGINT AS last_hour, \
           COUNT(*) FILTER (WHERE at > now() - interval '7 days' AND at <= now() - interval '1 hour')::BIGINT AS week \
         FROM restless_authority.owner_account_uses WHERE account_id = $1 AND company = $2 AND id <> $3",
    )
    .bind(account)
    .bind(company)
    .bind(this_use)
    .bind(target)
    .fetch_one(pool)
    .await?;
    let before: i64 = row.try_get("before")?;
    let spends: i64 = row.try_get("spends")?;
    let same_target: i64 = row.try_get("same_target")?;
    let last_hour: i64 = row.try_get::<i64, _>("last_hour")? + 1;
    let week: i64 = row.try_get("week")?;
    let open_burst: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM restless_authority.owner_account_notices \
         WHERE grant_id = $1 AND kind = 'burst' AND created_at > now() - interval '1 day')",
    )
    .bind(grant)
    .fetch_one(pool)
    .await?;
    Ok(notice_for(before, tier, spends, target.is_some() && same_target == 0, last_hour, week, open_burst))
}

/// Which notice, if any, one use raises. First use outranks everything; a burst is rare by design.
pub fn notice_for(
    uses_before: i64,
    tier: &str,
    spends_before: i64,
    new_destination: bool,
    last_hour: i64,
    week_before: i64,
    burst_noticed_today: bool,
) -> Option<&'static str> {
    if uses_before == 0 {
        return Some("first_use");
    }
    if tier == "spend" && spends_before == 0 {
        return Some("first_spend");
    }
    if matches!(tier, "act" | "spend" | "admin") && new_destination {
        return Some("new_destination");
    }
    let usual_hour = week_before as f64 / (7.0 * 24.0 - 1.0);
    if !burst_noticed_today && last_hour >= BURST_FLOOR && last_hour as f64 >= BURST_FACTOR * usual_hour.max(1.0) {
        return Some("burst");
    }
    None
}

#[derive(Debug, Clone, Serialize)]
pub struct Notice {
    pub id: String,
    pub kind: String,
    pub state: String,
}

/// The owner's answer to a notice: `fine` closes it; `freeze` also freezes that company's grant.
pub async fn decide_notice(pool: &PgPool, account: &str, notice: &str, decision: &str, by: &str) -> Result<Option<i64>> {
    if !matches!(decision, "fine" | "freeze") {
        bail!("Answer a notice with fine or freeze.");
    }
    let state = if decision == "fine" { "fine" } else { "frozen" };
    let row = sqlx::query(
        "UPDATE restless_authority.owner_account_notices SET state = $3, decided_at = now(), decided_by = $4 \
         WHERE id = $2 AND account_id = $1 RETURNING use_id, grant_id",
    )
    .bind(account)
    .bind(notice)
    .bind(state)
    .bind(by)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else { return Ok(None) };
    if decision == "freeze" {
        if let Some(grant) = row.try_get::<Option<String>, _>("grant_id")? {
            sqlx::query("UPDATE restless_authority.owner_account_grants SET frozen = TRUE WHERE id = $1")
                .bind(grant)
                .execute(pool)
                .await?;
        }
    }
    Ok(Some(row.try_get("use_id")?))
}

/// Notices by use, for the use log.
pub async fn notices_for_uses(pool: &PgPool, uses: &[i64]) -> Result<std::collections::HashMap<i64, Notice>> {
    let rows = sqlx::query(
        "SELECT id, use_id, kind, state FROM restless_authority.owner_account_notices WHERE use_id = ANY($1)",
    )
    .bind(uses)
    .fetch_all(pool)
    .await?;
    rows.iter()
        .map(|row| -> Result<(i64, Notice)> {
            Ok((row.try_get("use_id")?, Notice { id: row.try_get("id")?, kind: row.try_get("kind")?, state: row.try_get("state")? }))
        })
        .collect()
}

/// Open notices per account, for the "to look at" count on each account row.
pub async fn open_notices(pool: &PgPool) -> Result<std::collections::HashMap<String, i64>> {
    let rows = sqlx::query(
        "SELECT account_id, COUNT(*)::BIGINT AS open FROM restless_authority.owner_account_notices \
         WHERE state = 'open' GROUP BY account_id",
    )
    .fetch_all(pool)
    .await?;
    rows.iter().map(|row| Ok((row.try_get("account_id")?, row.try_get("open")?))).collect()
}

/// The short form of a use wherever one shows up: a notice, an approval, a receipt.
pub fn use_block(account: &Account, company: &str, grant: Option<&str>, row: &crate::accounts::Use) -> serde_json::Value {
    serde_json::json!({
        "account": { "id": account.id, "label": account.label, "kind": account.kind, "provider": account.provider },
        "grant_id": grant,
        "company": company,
        "tier": row.tier,
        "action": row.action,
        "target": row.target,
        "actor_display": row.actor,
        "directed_by_display": row.directed_by,
        "cost_micros": row.cost_micros,
    })
}

/// A company asking for an account, or for more on one (S98-08).
#[derive(Debug, Clone, Serialize)]
pub struct AccountRequest {
    pub id: String,
    pub company: String,
    pub account_id: Option<String>,
    pub kind: Option<String>,
    pub abilities: crate::accounts::Abilities,
    pub reason: String,
    pub requested_by: String,
    pub state: String,
    pub grant_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

fn row_request(row: &sqlx::postgres::PgRow) -> Result<AccountRequest> {
    Ok(AccountRequest {
        id: row.try_get("id")?,
        company: row.try_get("company")?,
        account_id: row.try_get("account_id")?,
        kind: row.try_get("kind")?,
        abilities: serde_json::from_value(row.try_get("abilities")?)?,
        reason: row.try_get("reason")?,
        requested_by: row.try_get("requested_by")?,
        state: row.try_get("state")?,
        grant_id: row.try_get("grant_id")?,
        created_at: row.try_get("created_at")?,
    })
}

pub async fn create_request(
    pool: &PgPool,
    company: &str,
    account_id: Option<&str>,
    kind: Option<&str>,
    abilities: Option<crate::accounts::Abilities>,
    reason: &str,
    by: &str,
) -> Result<AccountRequest> {
    if account_id.is_none() && kind.is_none_or(|kind| !crate::accounts::KINDS.contains(&kind)) {
        bail!("Name the account, or the kind of account the company needs.");
    }
    if reason.chars().count() > 1000 {
        bail!("Keep the reason to a short paragraph.");
    }
    let abilities = abilities.unwrap_or_default();
    for mode in [&abilities.read, &abilities.draft, &abilities.act, &abilities.spend, &abilities.admin] {
        if !matches!(mode.as_str(), "allow" | "tell" | "ask" | "off") {
            bail!("Each ability is allow, tell, ask or off.");
        }
    }
    if let Some(account) = account_id {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM restless_authority.owner_accounts WHERE id = $1)")
            .bind(account)
            .fetch_one(pool)
            .await?;
        if !exists {
            bail!("That account no longer exists.");
        }
    }
    let row = sqlx::query(
        "INSERT INTO restless_authority.owner_account_requests (id, company, account_id, kind, abilities, reason, requested_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
    )
    .bind(uuid::Uuid::new_v4().simple().to_string())
    .bind(company)
    .bind(account_id)
    .bind(kind)
    .bind(serde_json::to_value(&abilities)?)
    .bind(reason.trim())
    .bind(by)
    .fetch_one(pool)
    .await?;
    row_request(&row)
}

pub async fn open_requests(pool: &PgPool, company: &str) -> Result<Vec<AccountRequest>> {
    let rows = sqlx::query(
        "SELECT * FROM restless_authority.owner_account_requests WHERE company = $1 AND state = 'open' ORDER BY created_at",
    )
    .bind(company)
    .fetch_all(pool)
    .await?;
    rows.iter().map(row_request).collect()
}

/// The owner's answer. Approving a request that names an account grants it (or widens the grant
/// already there) with the abilities asked for; one that names only a kind is approved by saving a
/// grant from Settings, which passes its id.
pub async fn settle_request(
    pool: &PgPool,
    id: &str,
    decision: &str,
    grant_id: Option<&str>,
    by: &str,
) -> Result<Option<AccountRequest>> {
    let Some(row) = sqlx::query("SELECT * FROM restless_authority.owner_account_requests WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
    else {
        return Ok(None);
    };
    let request = row_request(&row)?;
    if request.state != "open" {
        bail!("That request was already answered.");
    }
    let grant = match (decision, grant_id, request.account_id.as_deref()) {
        ("denied", _, _) => None,
        ("approved", Some(grant), _) => Some(grant.to_string()),
        ("approved", None, Some(account)) => {
            let existing = crate::accounts::company_grants(pool, &request.company)
                .await?
                .into_iter()
                .find(|(grant, _)| grant.account_id == account);
            let input = crate::accounts::GrantInput { abilities: Some(request.abilities.clone()), ..Default::default() };
            Some(match existing {
                Some((grant, _)) => {
                    crate::accounts::update_grant(pool, account, &grant.id, input).await?;
                    grant.id
                }
                None => {
                    let input = crate::accounts::GrantInput { company: Some(request.company.clone()), ..input };
                    crate::accounts::create_grant(pool, account, input, by).await?.id
                }
            })
        }
        ("approved", None, None) => bail!("Choose which account to grant: approve it with limits from Settings."),
        _ => bail!("Answer a request with approved or denied."),
    };
    let row = sqlx::query(
        "UPDATE restless_authority.owner_account_requests SET state = $2, grant_id = $3, decided_at = now() \
         WHERE id = $1 AND state = 'open' RETURNING *",
    )
    .bind(id)
    .bind(decision)
    .bind(grant)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(row_request).transpose()
}

/// Open notices about one company's uses, each with its use and account, for its Inbox.
pub async fn open_company_notices(pool: &PgPool, company: &str) -> Result<Vec<(Notice, crate::accounts::Use, Account)>> {
    let rows = sqlx::query(
        "SELECT n.id AS notice_id, n.kind AS notice_kind, n.state AS notice_state, u.* \
         FROM restless_authority.owner_account_notices n \
         JOIN restless_authority.owner_account_uses u ON u.id = n.use_id \
         WHERE n.company = $1 AND n.state = 'open' ORDER BY n.created_at",
    )
    .bind(company)
    .fetch_all(pool)
    .await?;
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let accounts = crate::accounts::list(pool).await?;
    let mut out = Vec::new();
    for row in rows {
        let account_id: String = row.try_get("account_id")?;
        let Some(account) = accounts.iter().find(|account| account.id == account_id) else { continue };
        out.push((
            Notice { id: row.try_get("notice_id")?, kind: row.try_get("notice_kind")?, state: row.try_get("notice_state")? },
            crate::accounts::use_from_row(&row)?,
            account.clone(),
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{Abilities, Identity, Limits, QuietHours, SpendCap};

    fn account(kind: &str) -> Account {
        Account {
            id: "0123456789abcdef".into(),
            kind: kind.into(),
            label: "Owner's GitHub".into(),
            provider: None,
            identity: Identity { display: None, key: None },
            personal: false,
            health: "signed_in".into(),
            frozen: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            credentials: Vec::new(),
            grants: Vec::new(),
            open_notices: 0,
        }
    }

    fn grant() -> Grant {
        Grant {
            id: "g".into(),
            account_id: "0123456789abcdef".into(),
            company: "frankensurf".into(),
            abilities: Abilities::default(),
            spend_cap: None,
            limits: Limits::default(),
            who: Who::default(),
            frozen: false,
            created_at: Utc::now(),
            created_by: "owner".into(),
        }
    }

    fn run(grant: &Grant, tier: &str) -> Decision {
        decide(&account("github"), grant, "exec", tier, true, None, 0, 0, Utc::now())
    }

    #[test]
    fn github_calls_take_the_tier_their_effect_has() {
        assert_eq!(implied_tier("github", "GET", "/repos/a/b/pulls"), "read");
        assert_eq!(implied_tier("github", "POST", "/repos/a/b/pulls"), "draft");
        assert_eq!(implied_tier("github", "POST", "/repos/a/b/issues/3/comments"), "draft");
        assert_eq!(implied_tier("github", "PUT", "/repos/a/b/pulls/3/merge"), "act");
        assert_eq!(implied_tier("github", "POST", "/repos/a/b/releases"), "act");
        assert_eq!(implied_tier("github", "DELETE", "/repos/a/b"), "admin");
        assert_eq!(implied_tier("github", "PUT", "/repos/a/b/collaborators/x"), "admin");
        assert_eq!(implied_tier("api", "POST", "/v1/charges"), "act");
        // Declaring can raise the tier, never lower it.
        assert_eq!(call_tier("api", "POST", "/v1/charges", Some("spend")), "spend");
        assert_eq!(call_tier("github", "PUT", "/repos/a/b/pulls/3/merge", Some("draft")), "act");
    }

    #[test]
    fn scope_names_repos_and_owners() {
        let scope = vec!["yail259/frankensurf".to_string(), "blueprintlabio/*".to_string()];
        let target = |path| call_target("github", path);
        assert!(in_scope("github", &scope, target("/repos/Yail259/Frankensurf/pulls").as_deref(), "draft"));
        assert!(in_scope("github", &scope, target("/repos/blueprintlabio/restless-core").as_deref(), "read"));
        assert!(!in_scope("github", &scope, target("/repos/yail259/other/pulls").as_deref(), "draft"));
        assert!(in_scope("github", &scope, None, "read"));
        assert!(!in_scope("github", &scope, None, "act"));
    }

    #[test]
    fn each_mode_decides_its_tier() {
        let grant = grant();
        assert_eq!(run(&grant, "read"), Decision::Run);
        assert_eq!(run(&grant, "draft"), Decision::Run);
        assert_eq!(run(&grant, "act"), Decision::RunAndTell);
        assert!(matches!(run(&grant, "admin"), Decision::Refuse(_)));
        assert!(matches!(run(&grant, "spend"), Decision::Ask(_)));
    }

    #[test]
    fn spend_runs_under_its_cap_and_asks_above_it() {
        let mut grant = grant();
        grant.spend_cap = Some(SpendCap { amount_micros: 50_000_000, period: "month".into() });
        let spend = |spent, cost| decide(&account("api"), &grant, "exec", "spend", true, Some(cost), spent, 0, Utc::now());
        assert_eq!(spend(40_000_000, 10_000_000), Decision::RunAndTell);
        assert!(matches!(spend(40_000_000, 10_000_001), Decision::Ask(_)));
        assert!(matches!(
            decide(&account("api"), &grant, "exec", "spend", true, None, 0, 0, Utc::now()),
            Decision::Ask(_)
        ));
    }

    #[test]
    fn frozen_expired_unnamed_out_of_scope_and_capped_calls_do_not_run() {
        let now = Utc::now();
        let mut frozen = grant();
        frozen.frozen = true;
        assert!(matches!(run(&frozen, "read"), Decision::Refuse(_)));
        let mut expired = grant();
        expired.limits.expires_at = Some(now - chrono::Duration::minutes(1));
        assert!(matches!(run(&expired, "read"), Decision::Refuse(_)));
        let mut named = grant();
        named.who = Who::Actors { actors: vec!["baloo".into()] };
        assert!(matches!(run(&named, "read"), Decision::Refuse(_)));
        assert!(matches!(decide(&account("github"), &grant(), "exec", "act", false, None, 0, 0, now), Decision::Refuse(_)));
        let mut capped = grant();
        capped.limits.actions_per_day = Some(3);
        assert!(matches!(decide(&account("github"), &capped, "exec", "act", true, None, 0, 3, now), Decision::Ask(_)));
        assert_eq!(decide(&account("github"), &capped, "exec", "read", true, None, 0, 3, now), Decision::Run);
        let mut whole = account("github");
        whole.frozen = true;
        assert!(matches!(decide(&whole, &grant(), "exec", "read", true, None, 0, 0, now), Decision::Refuse(_)));
    }

    #[test]
    fn quiet_hours_hold_everything_but_reads() {
        let mut quiet = grant();
        quiet.limits.quiet_hours = Some(QuietHours { start: "22:00".into(), end: "07:00".into(), tz: "UTC".into() });
        let night = "2026-10-18T23:30:00Z".parse::<DateTime<Utc>>().unwrap();
        let day = "2026-10-18T12:00:00Z".parse::<DateTime<Utc>>().unwrap();
        assert!(matches!(decide(&account("github"), &quiet, "exec", "act", true, None, 0, 0, night), Decision::Refuse(_)));
        assert_eq!(decide(&account("github"), &quiet, "exec", "read", true, None, 0, 0, night), Decision::Run);
        assert_eq!(decide(&account("github"), &quiet, "exec", "act", true, None, 0, 0, day), Decision::RunAndTell);
    }

    #[test]
    fn the_credential_never_comes_back() {
        let secret = "ghp_abcdefghijklmnop";
        let encoded = base64::engine::general_purpose::STANDARD.encode(secret);
        let text = format!("token {secret} and basic {encoded}");
        let cut = redact(&text, secret);
        assert!(!cut.contains(secret) && !cut.contains(&encoded));
    }

    #[test]
    fn a_path_must_stay_under_the_account() {
        let call = |path: &str| {
            let arguments = serde_json::json!({"method": "GET", "path": path});
            parse_call(arguments.as_object().unwrap())
        };
        assert!(call("/repos/a/b").is_ok());
        for bad in ["https://evil.example/x", "//evil.example/x", "/a/../../x", "/x@evil"] {
            assert!(call(bad).is_err(), "{bad}");
        }
        let header = serde_json::json!({"method": "GET", "path": "/x", "headers": {"Authorization": "Bearer mine"}});
        assert!(parse_call(header.as_object().unwrap()).is_err());
    }

    #[test]
    fn one_notice_per_use_in_order_of_weight() {
        assert_eq!(notice_for(0, "read", 0, false, 1, 0, false), Some("first_use"));
        assert_eq!(notice_for(5, "spend", 0, false, 1, 0, false), Some("first_spend"));
        assert_eq!(notice_for(5, "act", 1, true, 1, 0, false), Some("new_destination"));
        assert_eq!(notice_for(5, "read", 0, false, 31, 7 * 24, false), Some("burst"));
        assert_eq!(notice_for(5, "read", 0, false, 31, 7 * 24 * 20, false), None);
        assert_eq!(notice_for(5, "read", 0, false, 31, 0, true), None);
        assert_eq!(notice_for(5, "act", 0, false, 2, 100, false), None);
    }

    #[tokio::test]
    async fn uses_raise_notices_freeze_stops_the_grant_and_requests_grant_as_asked() {
        let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            return;
        };
        let pool = PgPool::connect(&url).await.unwrap();
        sqlx::query("CREATE SCHEMA IF NOT EXISTS restless_authority").execute(&pool).await.unwrap();
        crate::accounts::ensure_schema(&pool).await.unwrap();
        ensure_schema(&pool).await.unwrap();
        let id = uuid::Uuid::new_v4().simple().to_string();
        sqlx::query("INSERT INTO restless_authority.owner_accounts (id, kind, label) VALUES ($1, 'github', 'GitHub')")
            .bind(&id)
            .execute(&pool)
            .await
            .unwrap();
        let company = format!("broker_{}_test", uuid::Uuid::new_v4().simple());
        let granted = crate::accounts::create_grant(
            &pool,
            &id,
            crate::accounts::GrantInput { company: Some(company.clone()), ..Default::default() },
            "owner",
        )
        .await
        .unwrap();
        let usable = Usable {
            account: crate::accounts::get(&pool, &id).await.unwrap().unwrap(),
            grant: granted.clone(),
            credential_id: "c".into(),
            settings: serde_json::json!({}),
            secrets: serde_json::Map::new(),
        };
        let first = record_use(&pool, &usable, &company, "exec", None, "GET /user", "read", None, "ok", None).await.unwrap();
        record_use(&pool, &usable, &company, "exec", None, "GET /user", "read", None, "ok", None).await.unwrap();
        let open = open_company_notices(&pool, &company).await.unwrap();
        assert_eq!(open.len(), 1, "only the first use is news");
        assert_eq!((open[0].0.kind.as_str(), open[0].1.id), ("first_use", first));
        assert_eq!(crate::accounts::get(&pool, &id).await.unwrap().unwrap().open_notices, 1);
        let logged = crate::accounts::uses(&pool, crate::accounts::UseFilter { company: Some(&company), ..Default::default() }, 10)
            .await
            .unwrap();
        assert_eq!(logged.len(), 2);
        assert_eq!(logged.iter().find(|row| row.id == first).unwrap().notice.as_ref().unwrap().kind, "first_use");
        decide_notice(&pool, &id, &open[0].0.id, "freeze", "owner").await.unwrap();
        assert!(open_company_notices(&pool, &company).await.unwrap().is_empty());
        let frozen = crate::accounts::company_grants(&pool, &company).await.unwrap();
        assert!(frozen[0].0.frozen, "freeze pauses this company's grant");
        assert!(matches!(
            decide(&frozen[0].1, &frozen[0].0, "exec", "read", true, None, 0, 0, Utc::now()),
            Decision::Refuse(_)
        ));

        let other = format!("broker_{}_test", uuid::Uuid::new_v4().simple());
        let asked = Abilities { act: "allow".into(), ..Abilities::default() };
        let request = create_request(&pool, &other, Some(&id), None, Some(asked.clone()), "To open release PRs.", "baloo")
            .await
            .unwrap();
        assert_eq!(open_requests(&pool, &other).await.unwrap().len(), 1);
        let settled = settle_request(&pool, &request.id, "approved", None, "owner").await.unwrap().unwrap();
        assert_eq!(settled.state, "approved");
        let granted = crate::accounts::company_grants(&pool, &other).await.unwrap();
        assert_eq!(granted[0].0.abilities, asked);
        assert!(open_requests(&pool, &other).await.unwrap().is_empty());
        assert!(settle_request(&pool, &request.id, "denied", None, "owner").await.is_err());
        crate::accounts::delete(&pool, &id).await.unwrap();
    }

    #[test]
    fn tool_names_are_stable_short_and_distinct() {
        let name = tool_name(&account("github"));
        assert_eq!(name, "acct_owner_s_github_012345__request");
        assert!(is_account_tool(&name) && name.len() <= 64);
    }
}
