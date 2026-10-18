//! Accounts the owner holds, granted to companies (S98-01).
//!
//! An account is an identity the owner holds at account level (a website login, an API, a browser
//! profile, later GitHub and the rest); its credentials prove it; a grant lets one company use it
//! with abilities and limits; a use records every time one did. Credential values live in the
//! vault and never in these rows or in a response: a row keeps only the reference.
//!
//! AI connections and S57 tool connections already exist in their own stores. They are listed
//! here as accounts of kind `ai_provider` and `mcp_tool`, read from where they live, so their
//! behaviour cannot change until each is moved deliberately.

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::collections::BTreeMap;
use uuid::Uuid;

pub const KINDS: &[&str] = &["ai_provider", "api", "website_login", "browser_profile", "github", "mcp_tool"];
/// Kinds created through this model; the others are listed from their own stores.
const OWN_KINDS: &[&str] = &["api", "website_login", "browser_profile", "github"];
const MODES: &[&str] = &["allow", "tell", "ask", "off"];

pub async fn ensure_schema(pool: &PgPool) -> Result<()> {
    for statement in [
        "CREATE TABLE IF NOT EXISTS restless_authority.owner_accounts (\
           id TEXT PRIMARY KEY, kind TEXT NOT NULL, label TEXT NOT NULL, provider TEXT, \
           identity_display TEXT, identity_key TEXT, personal BOOLEAN NOT NULL DEFAULT FALSE, \
           frozen BOOLEAN NOT NULL DEFAULT FALSE, health TEXT NOT NULL DEFAULT 'unknown', \
           created_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_at TIMESTAMPTZ NOT NULL DEFAULT now())",
        "CREATE TABLE IF NOT EXISTS restless_authority.owner_account_credentials (\
           id TEXT PRIMARY KEY, account_id TEXT NOT NULL REFERENCES restless_authority.owner_accounts(id) ON DELETE CASCADE, \
           kind TEXT NOT NULL, label TEXT, pinned BOOLEAN NOT NULL DEFAULT FALSE, \
           settings JSONB NOT NULL DEFAULT '{}'::jsonb, secrets JSONB NOT NULL DEFAULT '{}'::jsonb, \
           health TEXT NOT NULL DEFAULT 'unknown', updated_at TIMESTAMPTZ NOT NULL DEFAULT now())",
        "CREATE TABLE IF NOT EXISTS restless_authority.owner_account_grants (\
           id TEXT PRIMARY KEY, account_id TEXT NOT NULL REFERENCES restless_authority.owner_accounts(id) ON DELETE CASCADE, \
           company TEXT NOT NULL, abilities JSONB NOT NULL, spend_cap JSONB, limits JSONB NOT NULL, \
           who JSONB NOT NULL, frozen BOOLEAN NOT NULL DEFAULT FALSE, \
           created_at TIMESTAMPTZ NOT NULL DEFAULT now(), created_by TEXT NOT NULL, \
           UNIQUE (account_id, company))",
        "CREATE TABLE IF NOT EXISTS restless_authority.owner_account_uses (\
           id BIGSERIAL PRIMARY KEY, account_id TEXT NOT NULL, grant_id TEXT, company TEXT NOT NULL, \
           actor TEXT NOT NULL, directed_by TEXT, action TEXT NOT NULL, tier TEXT NOT NULL, \
           target TEXT, result TEXT NOT NULL, cost_micros BIGINT, at TIMESTAMPTZ NOT NULL DEFAULT now())",
        "CREATE INDEX IF NOT EXISTS owner_account_uses_by_account \
           ON restless_authority.owner_account_uses (account_id, at DESC)",
    ] {
        sqlx::query(statement).execute(pool).await.context("create owner account tables")?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identity {
    pub display: Option<String>,
    pub key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Credential {
    pub id: String,
    pub kind: String,
    pub label: Option<String>,
    pub pinned: bool,
    pub health: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Abilities {
    pub read: String,
    pub draft: String,
    pub act: String,
    pub spend: String,
    pub admin: String,
}

impl Default for Abilities {
    /// Permissive by default (owner decision, 18 October): act then tell, spend asks above its cap,
    /// admin off until granted.
    fn default() -> Self {
        Self { read: "allow".into(), draft: "allow".into(), act: "tell".into(), spend: "ask".into(), admin: "off".into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpendCap {
    pub amount_micros: i64,
    pub period: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuietHours {
    pub start: String,
    pub end: String,
    pub tz: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Limits {
    #[serde(default)]
    pub scope: Vec<String>,
    #[serde(default)]
    pub actions_per_day: Option<i64>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub quiet_hours: Option<QuietHours>,
}

/// `"everyone"`, or the named actors.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Who {
    Everyone(String),
    Actors { actors: Vec<String> },
}

impl Default for Who {
    fn default() -> Self {
        Self::Everyone("everyone".into())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Grant {
    pub id: String,
    pub account_id: String,
    pub company: String,
    pub abilities: Abilities,
    pub spend_cap: Option<SpendCap>,
    pub limits: Limits,
    pub who: Who,
    pub frozen: bool,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Account {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub provider: Option<String>,
    pub identity: Identity,
    pub personal: bool,
    pub health: String,
    pub frozen: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub credentials: Vec<Credential>,
    pub grants: Vec<Grant>,
    /// Notices about its uses the owner has not answered (S98-10).
    pub open_notices: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Use {
    pub id: i64,
    pub account_id: String,
    pub grant_id: Option<String>,
    pub company: String,
    pub actor: String,
    pub directed_by: Option<String>,
    pub action: String,
    pub tier: String,
    pub target: Option<String>,
    pub result: String,
    pub cost_micros: Option<i64>,
    pub at: DateTime<Utc>,
    /// The notice this use raised, if any (S98-10).
    pub notice: Option<crate::account_broker::Notice>,
}

/// What the owner sends to add an account (the Settings › Accounts form).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewAccount {
    pub kind: String,
    pub label: String,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub personal: bool,
    pub credential: NewCredential,
}

/// A credential as entered. The secret fields go to the vault; the rest are settings.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewCredential {
    pub kind: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub pinned: bool,
    // website_login
    #[serde(default)]
    pub sign_in_url: Option<String>,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub totp_secret: Option<String>,
    // api
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub auth: Option<String>,
    #[serde(default)]
    pub auth_name: Option<String>,
    #[serde(default)]
    pub secret: Option<String>,
    // browser_profile
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct GrantInput {
    #[serde(default)]
    pub company: Option<String>,
    #[serde(default)]
    pub abilities: Option<Abilities>,
    #[serde(default, deserialize_with = "double_option")]
    pub spend_cap: Option<Option<SpendCap>>,
    #[serde(default)]
    pub limits: Option<Limits>,
    #[serde(default)]
    pub who: Option<Who>,
    #[serde(default)]
    pub frozen: Option<bool>,
}

fn double_option<'de, D, T>(deserializer: D) -> std::result::Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

fn new_id() -> String {
    Uuid::new_v4().simple().to_string()
}

/// A secret's place in the vault: under the account, never shared with another.
pub fn secret_reference(account: &str, credential: &str, field: &str) -> String {
    format!("infisical:/owner/accounts/{account}/{credential}/{}", field.to_ascii_uppercase())
}

fn present(value: &Option<String>) -> Option<&str> {
    value.as_deref().map(str::trim).filter(|value| !value.is_empty())
}

fn http_url(value: &str) -> Option<url::Url> {
    url::Url::parse(value.trim())
        .ok()
        .filter(|url| matches!(url.scheme(), "https" | "http") && url.host_str().is_some())
}

fn host_name(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() <= 253
        && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
        && value.contains('.')
}

/// Check an owner's new account and split its credential into settings and secrets.
pub fn validate_new(account: &NewAccount) -> Result<(serde_json::Value, BTreeMap<&'static str, String>)> {
    if !OWN_KINDS.contains(&account.kind.as_str()) {
        bail!("Add AI accounts and tool connections from their own pages; this form adds API, GitHub, website-login and browser-profile accounts.");
    }
    let label = account.label.trim();
    if label.is_empty() || label.chars().count() > 80 {
        bail!("Give the account a name of up to 80 characters.");
    }
    let credential = &account.credential;
    let mut secrets = BTreeMap::new();
    let settings = match (account.kind.as_str(), credential.kind.as_str()) {
        ("website_login", "password") => {
            let sign_in = present(&credential.sign_in_url)
                .and_then(http_url)
                .context("Enter the page where this account signs in (https://…).")?;
            let domains = credential.domains.iter().map(|domain| domain.trim().to_ascii_lowercase()).collect::<Vec<_>>();
            if domains.is_empty() || domains.len() > 10 || !domains.iter().all(|domain| host_name(domain)) {
                bail!("List the account's own domains (example.com); the password is filled only there.");
            }
            let username = present(&credential.username).context("Enter the username or email it signs in with.")?;
            let password = present(&credential.password).context("Enter the password.")?;
            secrets.insert("password", password.to_string());
            if let Some(totp) = present(&credential.totp_secret) {
                secrets.insert("totp_secret", totp.replace(' ', ""));
            }
            serde_json::json!({ "sign_in_url": sign_in.as_str(), "domains": domains, "username": username })
        }
        ("api", "api_key") => {
            let base = present(&credential.base_url)
                .and_then(http_url)
                .context("Enter the API's base URL (https://…).")?;
            let auth = credential.auth.as_deref().unwrap_or("bearer");
            if !matches!(auth, "bearer" | "header" | "query" | "basic") {
                bail!("Choose how the key is sent: bearer, header, query or basic.");
            }
            let auth_name = present(&credential.auth_name);
            if matches!(auth, "header" | "query") && auth_name.is_none() {
                bail!("Name the header or query parameter that carries the key.");
            }
            let secret = present(&credential.secret).context("Enter the key.")?;
            if secret.len() > 32768 {
                bail!("That key is too long.");
            }
            secrets.insert("secret", secret.to_string());
            serde_json::json!({ "base_url": base.as_str(), "auth": auth, "auth_name": auth_name })
        }
        ("github", "token") => {
            // Until the Restless GitHub App (S98-05): a token the broker attaches, never handed out.
            let secret = present(&credential.secret).context("Paste a GitHub token (fine-grained is best).")?;
            if secret.len() > 1024 || secret.chars().any(char::is_whitespace) {
                bail!("That does not look like a GitHub token.");
            }
            secrets.insert("secret", secret.to_string());
            serde_json::json!({})
        }
        ("browser_profile", "browser_profile") => {
            let url = present(&credential.url)
                .and_then(http_url)
                .context("Enter the page to sign in at (https://…).")?;
            serde_json::json!({ "url": url.as_str() })
        }
        _ => bail!("That kind of credential does not belong to this kind of account."),
    };
    Ok((settings, secrets))
}

fn validate_grant(input: &GrantInput) -> Result<()> {
    if let Some(abilities) = &input.abilities {
        for mode in [&abilities.read, &abilities.draft, &abilities.act, &abilities.spend, &abilities.admin] {
            if !MODES.contains(&mode.as_str()) {
                bail!("Each ability is allow, tell, ask or off.");
            }
        }
    }
    if let Some(Some(cap)) = &input.spend_cap {
        if cap.amount_micros < 0 || !matches!(cap.period.as_str(), "day" | "week" | "month") {
            bail!("A spend cap is an amount per day, week or month.");
        }
    }
    if let Some(limits) = &input.limits {
        if limits.scope.len() > 50 || limits.scope.iter().any(|item| item.trim().is_empty() || item.len() > 200) {
            bail!("Keep the scope to up to 50 short entries.");
        }
        if limits.actions_per_day.is_some_and(|count| count < 0) {
            bail!("Actions per day cannot be negative.");
        }
    }
    if let Some(Who::Everyone(value)) = &input.who {
        if value != "everyone" {
            bail!("Who is \"everyone\" or a list of actors.");
        }
    }
    Ok(())
}

fn row_grant(row: &sqlx::postgres::PgRow) -> Result<Grant> {
    Ok(Grant {
        id: row.try_get("id")?,
        account_id: row.try_get("account_id")?,
        company: row.try_get("company")?,
        abilities: serde_json::from_value(row.try_get("abilities")?)?,
        spend_cap: row
            .try_get::<Option<serde_json::Value>, _>("spend_cap")?
            .map(serde_json::from_value)
            .transpose()?,
        limits: serde_json::from_value(row.try_get("limits")?)?,
        who: serde_json::from_value(row.try_get("who")?)?,
        frozen: row.try_get("frozen")?,
        created_at: row.try_get("created_at")?,
        created_by: row.try_get("created_by")?,
    })
}

/// Every account the owner holds that this model stores, with its credentials and grants.
pub async fn list(pool: &PgPool) -> Result<Vec<Account>> {
    let accounts = sqlx::query("SELECT * FROM restless_authority.owner_accounts ORDER BY created_at")
        .fetch_all(pool)
        .await?;
    let credentials = sqlx::query(
        "SELECT id, account_id, kind, label, pinned, health, updated_at \
         FROM restless_authority.owner_account_credentials ORDER BY updated_at",
    )
    .fetch_all(pool)
    .await?;
    let grants = sqlx::query("SELECT * FROM restless_authority.owner_account_grants ORDER BY created_at")
        .fetch_all(pool)
        .await?;
    let open = crate::account_broker::open_notices(pool).await.unwrap_or_default();
    let mut out = Vec::with_capacity(accounts.len());
    for row in accounts {
        let id: String = row.try_get("id")?;
        out.push(Account {
            kind: row.try_get("kind")?,
            label: row.try_get("label")?,
            provider: row.try_get("provider")?,
            identity: Identity { display: row.try_get("identity_display")?, key: row.try_get("identity_key")? },
            personal: row.try_get("personal")?,
            health: row.try_get("health")?,
            frozen: row.try_get("frozen")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
            credentials: credentials
                .iter()
                .filter(|credential| credential.try_get::<String, _>("account_id").is_ok_and(|account| account == id))
                .map(|credential| -> Result<Credential> {
                    Ok(Credential {
                        id: credential.try_get("id")?,
                        kind: credential.try_get("kind")?,
                        label: credential.try_get("label")?,
                        pinned: credential.try_get("pinned")?,
                        health: credential.try_get("health")?,
                        updated_at: credential.try_get("updated_at")?,
                    })
                })
                .collect::<Result<_>>()?,
            grants: grants
                .iter()
                .filter(|grant| grant.try_get::<String, _>("account_id").is_ok_and(|account| account == id))
                .map(row_grant)
                .collect::<Result<_>>()?,
            open_notices: open.get(&id).copied().unwrap_or(0),
            id,
        });
    }
    Ok(out)
}

pub async fn get(pool: &PgPool, id: &str) -> Result<Option<Account>> {
    Ok(list(pool).await?.into_iter().find(|account| account.id == id))
}

/// Store a new account: its secrets in the vault first, then the rows that point at them.
pub async fn create(pool: &PgPool, input: NewAccount) -> Result<Account> {
    let (settings, secrets) = validate_new(&input)?;
    let github_login = match (input.kind.as_str(), secrets.get("secret")) {
        ("github", Some(token)) => Some(github_login(token).await?),
        _ => None,
    };
    let account = new_id();
    let credential = new_id();
    let mut references = serde_json::Map::new();
    for (field, value) in &secrets {
        let reference = secret_reference(&account, &credential, field);
        crate::credential::store_reference(&reference, value)
            .await
            .context("The vault did not accept the secret; nothing was saved.")?;
        references.insert((*field).to_string(), serde_json::Value::String(reference));
    }
    let provider = input.provider.as_deref().map(str::trim).filter(|value| !value.is_empty());
    let mut transaction = pool.begin().await?;
    sqlx::query(
        "INSERT INTO restless_authority.owner_accounts (id, kind, label, provider, identity_display, personal, health) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&account)
    .bind(&input.kind)
    .bind(input.label.trim())
    .bind(provider)
    .bind(github_login.as_deref().or_else(|| settings.get("username").and_then(|value| value.as_str())))
    .bind(input.personal)
    .bind(if input.kind == "browser_profile" { "unknown" } else { "signed_in" })
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO restless_authority.owner_account_credentials (id, account_id, kind, label, pinned, settings, secrets) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&credential)
    .bind(&account)
    .bind(&input.credential.kind)
    .bind(input.credential.label.as_deref())
    .bind(input.credential.pinned)
    .bind(&settings)
    .bind(serde_json::Value::Object(references))
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    get(pool, &account).await?.context("the new account was not found")
}

/// Who a GitHub token signs in as; a token GitHub refuses is not saved.
async fn github_login(token: &str) -> Result<String> {
    let response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Restless-Account-Broker")
        .build()?
        .get("https://api.github.com/user")
        .bearer_auth(token)
        .header("accept", "application/vnd.github+json")
        .send()
        .await
        .context("GitHub did not answer; try again in a moment.")?;
    if !response.status().is_success() {
        bail!("GitHub refused that token. Check it has not expired and try again.");
    }
    let user: serde_json::Value = response.json().await.context("GitHub answered oddly; try again.")?;
    user.get("login")
        .and_then(|login| login.as_str())
        .map(str::to_string)
        .context("GitHub did not say who the token belongs to.")
}

pub async fn update(pool: &PgPool, id: &str, label: Option<&str>, frozen: Option<bool>) -> Result<Option<Account>> {
    if let Some(label) = label {
        let label = label.trim();
        if label.is_empty() || label.chars().count() > 80 {
            bail!("Give the account a name of up to 80 characters.");
        }
    }
    let changed = sqlx::query(
        "UPDATE restless_authority.owner_accounts SET label = COALESCE($2, label), \
         frozen = COALESCE($3, frozen), updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(label.map(str::trim))
    .bind(frozen)
    .execute(pool)
    .await?
    .rows_affected();
    if changed == 0 {
        return Ok(None);
    }
    get(pool, id).await
}

/// Remove an account, its grants and its secrets. Returns false when there was none.
pub async fn delete(pool: &PgPool, id: &str) -> Result<bool> {
    let references = sqlx::query(
        "SELECT secrets FROM restless_authority.owner_account_credentials WHERE account_id = $1",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;
    let removed = sqlx::query("DELETE FROM restless_authority.owner_accounts WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected();
    for row in references {
        if let Ok(serde_json::Value::Object(secrets)) = row.try_get::<serde_json::Value, _>("secrets") {
            for reference in secrets.values().filter_map(|value| value.as_str()) {
                if !crate::credential::delete_reference(reference).await {
                    tracing::warn!(account = id, "a removed account's secret stayed in the vault");
                }
            }
        }
    }
    Ok(removed > 0)
}

pub async fn create_grant(pool: &PgPool, account: &str, input: GrantInput, by: &str) -> Result<Grant> {
    validate_grant(&input)?;
    let company = input
        .company
        .as_deref()
        .map(str::trim)
        .filter(|company| !company.is_empty())
        .context("Choose the company to grant it to.")?;
    let id = new_id();
    let row = sqlx::query(
        "INSERT INTO restless_authority.owner_account_grants \
         (id, account_id, company, abilities, spend_cap, limits, who, frozen, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, COALESCE($8, FALSE), $9) RETURNING *",
    )
    .bind(&id)
    .bind(account)
    .bind(company)
    .bind(serde_json::to_value(input.abilities.unwrap_or_default())?)
    .bind(input.spend_cap.flatten().map(serde_json::to_value).transpose()?)
    .bind(serde_json::to_value(input.limits.unwrap_or_default())?)
    .bind(serde_json::to_value(input.who.unwrap_or_default())?)
    .bind(input.frozen)
    .bind(by)
    .fetch_one(pool)
    .await
    .map_err(|error| match &error {
        sqlx::Error::Database(database) if database.is_unique_violation() => {
            anyhow::anyhow!("That company already has this account; change its grant instead.")
        }
        sqlx::Error::Database(database) if database.is_foreign_key_violation() => {
            anyhow::anyhow!("That account no longer exists.")
        }
        _ => anyhow::Error::from(error),
    })?;
    row_grant(&row)
}

pub async fn update_grant(pool: &PgPool, account: &str, grant: &str, input: GrantInput) -> Result<Option<Grant>> {
    validate_grant(&input)?;
    let row = sqlx::query(
        "UPDATE restless_authority.owner_account_grants SET \
           abilities = COALESCE($3, abilities), \
           spend_cap = CASE WHEN $4 THEN $5 ELSE spend_cap END, \
           limits = COALESCE($6, limits), who = COALESCE($7, who), frozen = COALESCE($8, frozen) \
         WHERE id = $2 AND account_id = $1 RETURNING *",
    )
    .bind(account)
    .bind(grant)
    .bind(input.abilities.map(serde_json::to_value).transpose()?)
    .bind(input.spend_cap.is_some())
    .bind(input.spend_cap.flatten().map(serde_json::to_value).transpose()?)
    .bind(input.limits.map(serde_json::to_value).transpose()?)
    .bind(input.who.map(serde_json::to_value).transpose()?)
    .bind(input.frozen)
    .fetch_optional(pool)
    .await?;
    row.as_ref().map(row_grant).transpose()
}

pub async fn delete_grant(pool: &PgPool, account: &str, grant: &str) -> Result<bool> {
    Ok(sqlx::query("DELETE FROM restless_authority.owner_account_grants WHERE id = $2 AND account_id = $1")
        .bind(account)
        .bind(grant)
        .execute(pool)
        .await?
        .rows_affected()
        > 0)
}

/// One company's grants, with the account each names, for its members to see.
pub async fn company_grants(pool: &PgPool, company: &str) -> Result<Vec<(Grant, Account)>> {
    let accounts = list(pool).await?;
    Ok(accounts
        .iter()
        .flat_map(|account| {
            account
                .grants
                .iter()
                .filter(|grant| grant.company == company)
                .map(|grant| (grant.clone(), account.clone()))
        })
        .collect())
}

/// Filters on a use log; every one is optional.
#[derive(Debug, Default, Clone)]
pub struct UseFilter<'a> {
    pub account: Option<&'a str>,
    pub company: Option<&'a str>,
    pub actor: Option<&'a str>,
    pub directed_by: Option<&'a str>,
    pub tier: Option<&'a str>,
    pub before: Option<DateTime<Utc>>,
}

pub fn use_from_row(row: &sqlx::postgres::PgRow) -> Result<Use> {
    Ok(Use {
        id: row.try_get("id")?,
        account_id: row.try_get("account_id")?,
        grant_id: row.try_get("grant_id")?,
        company: row.try_get("company")?,
        actor: row.try_get("actor")?,
        directed_by: row.try_get("directed_by")?,
        action: row.try_get("action")?,
        tier: row.try_get("tier")?,
        target: row.try_get("target")?,
        result: row.try_get("result")?,
        cost_micros: row.try_get("cost_micros")?,
        at: row.try_get("at")?,
        notice: None,
    })
}

/// Uses newest first, each with the notice it raised.
pub async fn uses(pool: &PgPool, filter: UseFilter<'_>, limit: i64) -> Result<Vec<Use>> {
    let rows = sqlx::query(
        "SELECT * FROM restless_authority.owner_account_uses WHERE ($1::text IS NULL OR account_id = $1) \
           AND ($2::text IS NULL OR company = $2) AND ($3::text IS NULL OR actor = $3) \
           AND ($4::text IS NULL OR directed_by = $4) AND ($5::text IS NULL OR tier = $5) \
           AND ($6::timestamptz IS NULL OR at < $6) \
         ORDER BY at DESC, id DESC LIMIT $7",
    )
    .bind(filter.account)
    .bind(filter.company)
    .bind(filter.actor)
    .bind(filter.directed_by)
    .bind(filter.tier)
    .bind(filter.before)
    .bind(limit.clamp(1, 200))
    .fetch_all(pool)
    .await?;
    let mut out = rows.iter().map(use_from_row).collect::<Result<Vec<_>>>()?;
    let ids = out.iter().map(|row| row.id).collect::<Vec<_>>();
    let mut notices = crate::account_broker::notices_for_uses(pool, &ids).await?;
    for row in &mut out {
        row.notice = notices.remove(&row.id);
    }
    Ok(out)
}

/// The owner's AI connections as accounts, read from their own store (no behaviour change).
pub fn ai_provider_accounts(root: &std::path::Path) -> Result<Vec<Account>> {
    let registry = crate::model_connections::load_owner_connections(root)?;
    let now = Utc::now();
    Ok(registry
        .connections
        .into_iter()
        .map(|connection| Account {
            id: connection.id.clone(),
            kind: "ai_provider".into(),
            label: connection.label,
            provider: Some(connection.provider),
            identity: Identity { display: None, key: connection.account_key },
            personal: connection.kind == "oauth",
            health: "unknown".into(),
            frozen: false,
            created_at: now,
            updated_at: now,
            credentials: vec![Credential {
                id: connection.id,
                kind: connection.kind,
                label: None,
                pinned: true,
                health: "unknown".into(),
                updated_at: now,
            }],
            grants: Vec::new(),
            open_notices: 0,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn login(credential: serde_json::Value) -> NewAccount {
        serde_json::from_value(serde_json::json!({
            "kind": "website_login", "label": "eBay", "provider": "ebay.com", "personal": true,
            "credential": credential
        }))
        .unwrap()
    }

    #[test]
    fn a_website_login_keeps_its_password_apart_from_its_settings() {
        let (settings, secrets) = validate_new(&login(serde_json::json!({
            "kind": "password", "sign_in_url": "https://signin.ebay.com/", "domains": ["ebay.com", "signin.ebay.com"],
            "username": "seller@example.com", "password": "hunter2", "totp_secret": "JBSW Y3DP"
        })))
        .unwrap();
        assert_eq!(secrets.get("password").map(String::as_str), Some("hunter2"));
        assert_eq!(secrets.get("totp_secret").map(String::as_str), Some("JBSWY3DP"));
        let text = settings.to_string();
        assert!(!text.contains("hunter2") && !text.contains("JBSW"));
        assert_eq!(settings["username"], "seller@example.com");
    }

    #[test]
    fn a_login_needs_its_own_domains_and_an_https_sign_in_page() {
        assert!(validate_new(&login(serde_json::json!({
            "kind": "password", "sign_in_url": "javascript:alert(1)", "domains": ["ebay.com"],
            "username": "u", "password": "p"
        })))
        .is_err());
        assert!(validate_new(&login(serde_json::json!({
            "kind": "password", "sign_in_url": "https://ebay.com", "domains": ["not a host"],
            "username": "u", "password": "p"
        })))
        .is_err());
    }

    #[test]
    fn an_api_key_names_how_it_is_sent() {
        let api = |auth: &str, name: Option<&str>| -> NewAccount {
            serde_json::from_value(serde_json::json!({
                "kind": "api", "label": "Shippo", "provider": "api.goshippo.com",
                "credential": { "kind": "api_key", "base_url": "https://api.goshippo.com", "auth": auth, "auth_name": name, "secret": "k" }
            }))
            .unwrap()
        };
        assert!(validate_new(&api("bearer", None)).is_ok());
        assert!(validate_new(&api("header", None)).is_err());
        assert!(validate_new(&api("header", Some("X-Api-Key"))).is_ok());
        assert!(validate_new(&api("cookie", None)).is_err());
    }

    #[test]
    fn ai_accounts_and_tools_are_not_added_through_this_form() {
        let ai: NewAccount = serde_json::from_value(serde_json::json!({
            "kind": "ai_provider", "label": "Claude", "credential": { "kind": "api_key", "secret": "k" }
        }))
        .unwrap();
        assert!(validate_new(&ai).is_err());
    }

    #[tokio::test]
    async fn a_grant_is_made_changed_frozen_and_removed_and_its_company_sees_it() {
        let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            return;
        };
        let pool = PgPool::connect(&url).await.unwrap();
        sqlx::query("CREATE SCHEMA IF NOT EXISTS restless_authority").execute(&pool).await.unwrap();
        ensure_schema(&pool).await.unwrap();
        // Rows straight in: the vault write is the credential module's own concern.
        let account = new_id();
        sqlx::query(
            "INSERT INTO restless_authority.owner_accounts (id, kind, label, provider) VALUES ($1, 'api', 'Shippo', 'api.goshippo.com')",
        )
        .bind(&account)
        .execute(&pool)
        .await
        .unwrap();
        let company = format!("accounts_{}_test", Uuid::new_v4().simple());
        let grant = create_grant(&pool, &account, GrantInput { company: Some(company.clone()), ..Default::default() }, "owner")
            .await
            .unwrap();
        assert_eq!(grant.abilities, Abilities::default());
        assert!(create_grant(&pool, &account, GrantInput { company: Some(company.clone()), ..Default::default() }, "owner")
            .await
            .unwrap_err()
            .to_string()
            .contains("already has this account"));
        let changed: GrantInput = serde_json::from_value(serde_json::json!({
            "spend_cap": { "amount_micros": 50_000_000, "period": "month" }, "frozen": true
        }))
        .unwrap();
        let changed = update_grant(&pool, &account, &grant.id, changed).await.unwrap().unwrap();
        assert!(changed.frozen);
        assert_eq!(changed.spend_cap.unwrap().amount_micros, 50_000_000);
        let seen = company_grants(&pool, &company).await.unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].1.label, "Shippo");
        assert!(delete_grant(&pool, &account, &grant.id).await.unwrap());
        assert!(company_grants(&pool, &company).await.unwrap().is_empty());
        assert!(delete(&pool, &account).await.unwrap());
    }

    #[test]
    fn grants_default_to_permissive_and_refuse_unknown_modes() {
        let defaults = Abilities::default();
        assert_eq!((defaults.read.as_str(), defaults.act.as_str(), defaults.admin.as_str()), ("allow", "tell", "off"));
        let bad: GrantInput = serde_json::from_value(serde_json::json!({
            "abilities": { "read": "allow", "draft": "allow", "act": "always", "spend": "ask", "admin": "off" }
        }))
        .unwrap();
        assert!(validate_grant(&bad).is_err());
        let who: Who = serde_json::from_value(serde_json::json!({ "actors": ["exec"] })).unwrap();
        assert_eq!(who, Who::Actors { actors: vec!["exec".into()] });
        let cleared: GrantInput = serde_json::from_value(serde_json::json!({ "spend_cap": null })).unwrap();
        assert_eq!(cleared.spend_cap, Some(None));
    }
}
