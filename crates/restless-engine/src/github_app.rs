//! GitHub as an App, not a token (S98-05).
//!
//! The owner creates their own Restless GitHub App in one step through GitHub's manifest flow:
//! GitHub hands the App's key straight to the plane, so nobody pastes or holds it. Installing the
//! App on an organisation or account, and choosing its repositories, makes an owner account of
//! kind `github` whose credential names the installation. Each use then mints a one-hour
//! installation token from the App's key; nothing expires that a person has to renew, and an
//! organisation's limit on personal tokens does not apply.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use base64::Engine as _;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use uuid::Uuid;

const API: &str = "https://api.github.com";
/// How long a manifest or install hand-off may take before its state is forgotten.
const FLOW_TTL: Duration = Duration::from_secs(30 * 60);
/// A cached installation token is replaced this long before GitHub expires it.
const TOKEN_MARGIN: chrono::Duration = chrono::Duration::minutes(5);

pub async fn ensure_schema(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.github_apps (\
           id TEXT PRIMARY KEY, app_id BIGINT NOT NULL UNIQUE, slug TEXT NOT NULL, \
           owner_login TEXT NOT NULL, html_url TEXT NOT NULL, key_reference TEXT NOT NULL, \
           created_at TIMESTAMPTZ NOT NULL DEFAULT now())",
    )
    .execute(pool)
    .await
    .context("create github_apps")?;
    Ok(())
}

/// The owner's GitHub App, as the plane keeps it: never its key, only where the key lives.
#[derive(Debug, Clone, Serialize)]
pub struct App {
    pub id: String,
    pub app_id: i64,
    pub slug: String,
    pub owner_login: String,
    pub html_url: String,
    #[serde(skip)]
    pub key_reference: String,
    pub created_at: DateTime<Utc>,
}

impl App {
    /// Where the owner installs the App and picks repositories.
    pub fn install_url(&self, state: &str) -> String {
        format!(
            "https://github.com/apps/{}/installations/new?state={}",
            self.slug,
            urlencode(state)
        )
    }
}

fn urlencode(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

fn app_from(row: &sqlx::postgres::PgRow) -> Result<App> {
    Ok(App {
        id: row.try_get("id")?,
        app_id: row.try_get("app_id")?,
        slug: row.try_get("slug")?,
        owner_login: row.try_get("owner_login")?,
        html_url: row.try_get("html_url")?,
        key_reference: row.try_get("key_reference")?,
        created_at: row.try_get("created_at")?,
    })
}

pub async fn apps(pool: &PgPool) -> Result<Vec<App>> {
    sqlx::query("SELECT * FROM restless_authority.github_apps ORDER BY created_at")
        .fetch_all(pool)
        .await?
        .iter()
        .map(app_from)
        .collect()
}

/// The App for an organisation or account, or the owner's only App.
pub async fn app_for(pool: &PgPool, owner: Option<&str>) -> Result<Option<App>> {
    let all = apps(pool).await?;
    Ok(match owner {
        Some(owner) => all
            .into_iter()
            .find(|app| app.owner_login.eq_ignore_ascii_case(owner)),
        None => all.into_iter().next(),
    })
}

/// A plain GitHub login: what an organisation or user name may be.
pub fn valid_login(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 39
        && !value.starts_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

// ---- The hand-off with GitHub: one state per flow, so a returning browser is ours. ----

#[derive(Clone)]
struct Flow {
    started: Instant,
    owner: Option<String>,
}

static FLOWS: Mutex<Option<HashMap<String, Flow>>> = Mutex::new(None);

fn flows<T>(f: impl FnOnce(&mut HashMap<String, Flow>) -> T) -> T {
    let mut guard = FLOWS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    map.retain(|_, flow| flow.started.elapsed() < FLOW_TTL);
    f(map)
}

/// Begin a hand-off for an organisation (or the owner's own account when `None`).
pub fn start_flow(owner: Option<&str>) -> String {
    let state = Uuid::new_v4().simple().to_string();
    flows(|map| {
        map.insert(
            state.clone(),
            Flow {
                started: Instant::now(),
                owner: owner.map(str::to_string),
            },
        )
    });
    state
}

/// The organisation a returning hand-off was for, if it is one this plane started.
fn flow(state: &str) -> Option<Option<String>> {
    flows(|map| map.get(state).map(|flow| flow.owner.clone()))
}

fn finish_flow(state: &str) {
    flows(|map| map.remove(state));
}

/// The App the owner is asked to create: private, no webhook, and only what a company needs to
/// work on code: read and write code, pull requests, issues and workflow files; read checks.
pub fn manifest(page_url: &str, owner: Option<&str>) -> serde_json::Value {
    let base = page_url.split(['?', '#']).next().unwrap_or(page_url);
    let mut name = match owner {
        Some(owner) => format!("Restless {owner}"),
        None => "Restless".to_string(),
    };
    name.truncate(34);
    serde_json::json!({
        "name": name,
        "url": "https://restless.run",
        "description": "Your Restless companies, working on the repositories you choose.",
        "public": false,
        "redirect_url": format!("{base}?github_app=created"),
        "setup_url": format!("{base}?github_app=installed"),
        "setup_on_update": true,
        "request_oauth_on_install": false,
        "hook_attributes": { "url": "https://restless.run/github-app-webhook", "active": false },
        "default_permissions": {
            "contents": "write",
            "pull_requests": "write",
            "issues": "write",
            "workflows": "write",
            "checks": "read",
            "statuses": "read",
            "actions": "read",
            "metadata": "read"
        },
        "default_events": []
    })
}

/// Where the browser posts the manifest: the organisation's App settings, or the owner's own.
pub fn manifest_action(owner: Option<&str>, state: &str) -> String {
    match owner {
        Some(owner) => format!(
            "https://github.com/organizations/{owner}/settings/apps/new?state={}",
            urlencode(state)
        ),
        None => format!(
            "https://github.com/settings/apps/new?state={}",
            urlencode(state)
        ),
    }
}

fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("Restless-GitHub-App")
        .build()?)
}

/// GitHub came back with a code for the App it just created: keep the App, its key in the vault.
pub async fn created(pool: &PgPool, code: &str, state: &str) -> Result<App> {
    let Some(owner) = flow(state) else {
        bail!("That GitHub hand-off has expired. Start again from Accounts.");
    };
    if code.is_empty() || code.len() > 200 || !code.bytes().all(|b| b.is_ascii_alphanumeric()) {
        bail!("GitHub sent back something unexpected. Start again from Accounts.");
    }
    let response = client()?
        .post(format!("{API}/app-manifests/{code}/conversions"))
        .header("accept", "application/vnd.github+json")
        .send()
        .await
        .context("GitHub did not answer; try again in a moment.")?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let said = response
            .json::<serde_json::Value>()
            .await
            .ok()
            .and_then(|body| body.get("message")?.as_str().map(str::to_string))
            .unwrap_or_default();
        tracing::warn!(status, said, "GitHub refused the App manifest conversion");
        bail!("GitHub would not finish creating the App (it answered {status}). Start again from Accounts.");
    }
    let body: serde_json::Value = response
        .json()
        .await
        .context("GitHub answered oddly; try again.")?;
    let app_id = body
        .get("id")
        .and_then(|value| value.as_i64())
        .context("GitHub did not say which App it created.")?;
    let slug = body
        .get("slug")
        .and_then(|value| value.as_str())
        .context("GitHub did not name the App.")?
        .to_string();
    let pem = body
        .get("pem")
        .and_then(|value| value.as_str())
        .context("GitHub did not hand over the App's key.")?;
    signing_key(pem).context("The App's key from GitHub could not be read.")?;
    let owner_login = body
        .pointer("/owner/login")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or(owner)
        .unwrap_or_default();
    let html_url = body
        .get("html_url")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    let id = Uuid::new_v4().simple().to_string();
    let key_reference = format!("infisical:/owner/github-apps/{id}/PRIVATE_KEY");
    crate::credential::store_reference(&key_reference, &key_for_vault(pem))
        .await
        .context("The vault did not accept the App's key; nothing was saved.")?;
    sqlx::query(
        "INSERT INTO restless_authority.github_apps (id, app_id, slug, owner_login, html_url, key_reference) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&id)
    .bind(app_id)
    .bind(&slug)
    .bind(&owner_login)
    .bind(&html_url)
    .bind(&key_reference)
    .execute(pool)
    .await?;
    // The same state carries on to the install, so it stays open until then.
    apps(pool)
        .await?
        .into_iter()
        .find(|app| app.id == id)
        .context("the new App was not found")
}

/// The App was installed: the installation becomes an owner account of kind `github`.
pub async fn installed(
    pool: &PgPool,
    installation_id: i64,
    state: Option<&str>,
) -> Result<crate::accounts::Account> {
    // GitHub sends people here after a change made on GitHub too, without our state; the
    // installation is only accepted if one of this plane's Apps can see it.
    if let Some(state) = state {
        finish_flow(state);
    }
    let mut found = None;
    for app in apps(pool).await? {
        if let Ok(installation) = installation(&app, installation_id).await {
            found = Some((app, installation));
            break;
        }
    }
    let Some((app, installation)) = found else {
        bail!("None of your Restless GitHub Apps can see that installation. Install it again from Accounts.");
    };
    let login = installation
        .pointer("/account/login")
        .and_then(|value| value.as_str())
        .unwrap_or("GitHub")
        .to_string();
    if let Some(existing) = account_for_installation(pool, installation_id).await? {
        return Ok(existing);
    }
    crate::accounts::create_github_app_account(pool, &app, installation_id, &login).await
}

async fn account_for_installation(
    pool: &PgPool,
    installation_id: i64,
) -> Result<Option<crate::accounts::Account>> {
    let id: Option<String> = sqlx::query_scalar(
        "SELECT account_id FROM restless_authority.owner_account_credentials \
         WHERE kind = 'github_app' AND (settings->>'installation_id')::bigint = $1 LIMIT 1",
    )
    .bind(installation_id)
    .fetch_optional(pool)
    .await?;
    match id {
        Some(id) => crate::accounts::get(pool, &id).await,
        None => Ok(None),
    }
}

async fn installation(app: &App, installation_id: i64) -> Result<serde_json::Value> {
    let pem = crate::credential::resolve_reference(&app.key_reference).await?;
    let jwt = app_jwt(app.app_id, &pem, Utc::now())?;
    let response = client()?
        .get(format!("{API}/app/installations/{installation_id}"))
        .bearer_auth(jwt)
        .header("accept", "application/vnd.github+json")
        .header("x-github-api-version", "2022-11-28")
        .send()
        .await?;
    if !response.status().is_success() {
        bail!("not this App's installation");
    }
    Ok(response.json().await?)
}

// ---- Tokens ----

/// The vault keeps one-line values, and a PEM key is many lines: it is kept as base64 of the PEM.
fn key_for_vault(pem_text: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(pem_text.trim())
}

/// The App's key as the vault holds it (base64 of the PEM), or as PEM itself.
fn signing_key(stored: &str) -> Result<ring::rsa::KeyPair> {
    let stored = stored.trim();
    let pem_text = if stored.starts_with("-----BEGIN") {
        stored.to_string()
    } else {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(stored)
            .context("not a PEM key")?;
        String::from_utf8(bytes).context("not a PEM key")?
    };
    let block = pem::parse(pem_text.trim()).context("not a PEM key")?;
    match block.tag() {
        "RSA PRIVATE KEY" => ring::rsa::KeyPair::from_der(block.contents())
            .map_err(|error| anyhow::anyhow!("not an RSA key: {error}")),
        "PRIVATE KEY" => ring::rsa::KeyPair::from_pkcs8(block.contents())
            .map_err(|error| anyhow::anyhow!("not an RSA key: {error}")),
        other => bail!("unexpected key type {other}"),
    }
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// The App's own credential, good for ten minutes: what GitHub asks for to mint a token.
pub fn app_jwt(app_id: i64, pem_text: &str, now: DateTime<Utc>) -> Result<String> {
    let key = signing_key(pem_text)?;
    let header = b64(br#"{"alg":"RS256","typ":"JWT"}"#);
    // Backdated a minute for clock drift; GitHub refuses anything longer than ten minutes.
    let claims = serde_json::json!({
        "iat": now.timestamp() - 60,
        "exp": now.timestamp() + 9 * 60,
        "iss": app_id.to_string(),
    });
    let payload = b64(claims.to_string().as_bytes());
    let message = format!("{header}.{payload}");
    let mut signature = vec![0; key.public().modulus_len()];
    key.sign(
        &ring::signature::RSA_PKCS1_SHA256,
        &ring::rand::SystemRandom::new(),
        message.as_bytes(),
        &mut signature,
    )
    .map_err(|_| anyhow::anyhow!("the App's key could not sign"))?;
    Ok(format!("{message}.{}", b64(&signature)))
}

static TOKENS: Mutex<Option<HashMap<i64, (String, DateTime<Utc>)>>> = Mutex::new(None);

fn cached(installation_id: i64, now: DateTime<Utc>) -> Option<String> {
    let guard = TOKENS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard
        .as_ref()?
        .get(&installation_id)
        .filter(|(_, expires)| *expires - TOKEN_MARGIN > now)
        .map(|(token, _)| token.clone())
}

fn remember(installation_id: i64, token: &str, expires: DateTime<Utc>) {
    let mut guard = TOKENS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    guard
        .get_or_insert_with(HashMap::new)
        .insert(installation_id, (token.to_string(), expires));
}

/// Whether a credential's settings name an App installation rather than a stored token.
pub fn installation_of(settings: &serde_json::Value) -> Option<(i64, i64)> {
    let installation = settings.get("installation_id")?.as_i64()?;
    let app = settings.get("app_id")?.as_i64()?;
    Some((app, installation))
}

/// A token for the installation, minted from the App's key and reused until shortly before it
/// expires. `key_reference` is where the App's key lives in the vault.
pub async fn installation_token(
    app_id: i64,
    installation_id: i64,
    key_reference: &str,
) -> Result<String> {
    let now = Utc::now();
    if let Some(token) = cached(installation_id, now) {
        return Ok(token);
    }
    let pem = crate::credential::resolve_reference(key_reference)
        .await
        .context("the vault did not give the GitHub App's key; nothing was sent")?;
    let jwt = app_jwt(app_id, &pem, now)?;
    let response = client()?
        .post(format!(
            "{API}/app/installations/{installation_id}/access_tokens"
        ))
        .bearer_auth(jwt)
        .header("accept", "application/vnd.github+json")
        .header("x-github-api-version", "2022-11-28")
        .send()
        .await
        .context("GitHub did not answer; nothing was sent")?;
    if !response.status().is_success() {
        bail!(
            "GitHub would not give the App a token ({}). The App may have been uninstalled; install it again from Accounts.",
            response.status().as_u16()
        );
    }
    let body: serde_json::Value = response.json().await?;
    let token = body
        .get("token")
        .and_then(|value| value.as_str())
        .context("GitHub gave the App no token")?
        .to_string();
    let expires = body
        .get("expires_at")
        .and_then(|value| value.as_str())
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or(now + chrono::Duration::minutes(50));
    remember(installation_id, &token, expires);
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh 2048-bit key in GitHub's format (PKCS#1 PEM), made for the test and thrown away.
    fn test_key() -> String {
        use rsa::pkcs1::EncodeRsaPrivateKey as _;
        rsa::RsaPrivateKey::new(&mut rand::thread_rng(), 2048)
            .unwrap()
            .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
            .unwrap()
            .to_string()
    }

    #[test]
    fn the_apps_key_is_kept_as_one_line_the_vault_accepts() {
        let pem = test_key();
        let kept = key_for_vault(&pem);
        assert!(
            !kept.chars().any(char::is_control),
            "one line, no line endings"
        );
        let from_vault = signing_key(&kept).unwrap();
        let from_pem = signing_key(&pem).unwrap();
        assert_eq!(from_vault.public().as_ref(), from_pem.public().as_ref());
        assert!(signing_key("not a key").is_err());
    }

    #[test]
    fn the_app_jwt_is_signed_with_the_apps_key_and_lives_under_ten_minutes() {
        let now = DateTime::parse_from_rfc3339("2026-10-24T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let pem = test_key();
        let jwt = app_jwt(123, &pem, now).unwrap();
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);
        let claims: serde_json::Value = serde_json::from_slice(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(parts[1])
                .unwrap(),
        )
        .unwrap();
        assert_eq!(claims["iss"], "123");
        let lifetime = claims["exp"].as_i64().unwrap() - claims["iat"].as_i64().unwrap();
        assert!(lifetime <= 600, "{lifetime}");
        assert!(claims["iat"].as_i64().unwrap() < now.timestamp());

        let key = signing_key(&pem).unwrap();
        let public = ring::signature::UnparsedPublicKey::new(
            &ring::signature::RSA_PKCS1_2048_8192_SHA256,
            key.public().as_ref(),
        );
        let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[2])
            .unwrap();
        public
            .verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &signature)
            .expect("the signature verifies with the App's public key");
    }

    #[test]
    fn the_manifest_asks_for_code_work_only_and_comes_back_to_the_same_page() {
        let manifest = manifest(
            "https://app.restless.run/account/settings/accounts?x=1#top",
            Some("BlueprintLabIO"),
        );
        assert_eq!(manifest["name"], "Restless BlueprintLabIO");
        assert_eq!(manifest["public"], false);
        assert_eq!(
            manifest["redirect_url"],
            "https://app.restless.run/account/settings/accounts?github_app=created"
        );
        assert_eq!(
            manifest["setup_url"],
            "https://app.restless.run/account/settings/accounts?github_app=installed"
        );
        assert_eq!(manifest["hook_attributes"]["active"], false);
        assert_eq!(manifest["default_permissions"]["contents"], "write");
        assert!(manifest["default_permissions"]
            .get("administration")
            .is_none());
        let long = super::manifest("http://127.0.0.1:7788/account", Some(&"x".repeat(39)));
        assert!(long["name"].as_str().unwrap().len() <= 34);
    }

    #[test]
    fn the_manifest_goes_to_the_organisation_or_the_owners_own_settings() {
        assert_eq!(
            manifest_action(Some("BlueprintLabIO"), "s1"),
            "https://github.com/organizations/BlueprintLabIO/settings/apps/new?state=s1"
        );
        assert_eq!(
            manifest_action(None, "s2"),
            "https://github.com/settings/apps/new?state=s2"
        );
    }

    #[test]
    fn a_hand_off_is_known_once_started_and_gone_once_finished() {
        let state = start_flow(Some("BlueprintLabIO"));
        assert_eq!(flow(&state), Some(Some("BlueprintLabIO".to_string())));
        assert_eq!(flow("not-ours"), None);
        finish_flow(&state);
        assert_eq!(flow(&state), None);
    }

    #[test]
    fn only_plain_logins_pass() {
        assert!(valid_login("BlueprintLabIO") && valid_login("yail259") && valid_login("a-b"));
        for bad in ["", "-a", "a b", "a/b", &"x".repeat(40)] {
            assert!(!valid_login(bad), "{bad}");
        }
    }

    #[test]
    fn an_app_credential_names_its_installation() {
        let settings = serde_json::json!({ "app_id": 7, "installation_id": 99, "login": "org" });
        assert_eq!(installation_of(&settings), Some((7, 99)));
        assert_eq!(installation_of(&serde_json::json!({ "login": "x" })), None);
    }

    #[test]
    fn a_cached_token_is_reused_until_shortly_before_it_expires() {
        let now = Utc::now();
        remember(4242, "tok", now + chrono::Duration::minutes(30));
        assert_eq!(cached(4242, now).as_deref(), Some("tok"));
        assert_eq!(cached(4242, now + chrono::Duration::minutes(26)), None);
    }
}
