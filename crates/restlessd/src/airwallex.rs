//! The one provider-specific finance adapter proven by Sprint 08.
//!
//! There is deliberately no `BankProvider` trait. Airwallex's authentication,
//! transfer approval and status vocabulary stay here until a second observed
//! provider proves a repeated seam.

use anyhow::{bail, Context as _, Result};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use hmac::{Hmac, Mac as _};
use reqwest::{Method, StatusCode, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use sqlx::{PgPool, Row as _};
use std::{collections::HashMap, sync::OnceLock};

use crate::credential::FinanceCredential;
use crate::finance::{PaymentIntent, PaymentState};
use crate::runtime::CompanyConfig;

const LIVE_API: &str = "https://api.airwallex.com";
const SANDBOX_API: &str = "https://api.sandbox.airwallex.com";
const TOKEN_REFRESH_MARGIN_SECONDS: i64 = 30;
const PROVIDER_TIMEOUT_SECONDS: u64 = 30;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Environment {
    Sandbox,
    Live,
}

impl Environment {
    fn base_url(&self) -> &'static str {
        match self {
            Self::Sandbox => SANDBOX_API,
            Self::Live => LIVE_API,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectionInput {
    pub environment: Environment,
    /// Exact date-version observed for this account during T0. Airwallex
    /// changes the latest version over time, so this is evidence, not a build
    /// constant.
    pub api_version: String,
    pub client_id: String,
    pub account_ref: String,
    /// Provider-native web destination configured and verified by the owner.
    pub approval_url: String,
    pub read_scopes: Vec<String>,
    pub submit_scopes: Vec<String>,
    pub approval_workflow_observed: bool,
    #[serde(default)]
    pub observed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Connection {
    #[serde(flatten)]
    pub configured: ConnectionInput,
    pub updated_by: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceObservation {
    pub provider: String,
    pub account_ref: String,
    pub balances: Vec<Balance>,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub account_type: String,
    pub currency: String,
    pub available_amount: serde_json::Number,
    pub pending_amount: serde_json::Number,
    #[serde(default)]
    pub reserved_amount: Option<serde_json::Number>,
    pub total_amount: serde_json::Number,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionProbe {
    pub provider: &'static str,
    pub environment: Environment,
    pub account_ref: String,
    pub api_version: String,
    pub read_authentication: &'static str,
    pub read_balances: &'static str,
    pub read_transfers: &'static str,
    pub submit_authentication: &'static str,
    pub submit_transfer_read: &'static str,
    pub credentials_distinct: bool,
    pub approval_workflow_observed: bool,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct LoginResponse {
    token: String,
    expires_at: String,
}

#[derive(Debug, Deserialize)]
struct TransferResponse {
    id: String,
    status: String,
    #[serde(default)]
    request_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TransferList {
    #[serde(default)]
    items: Vec<TransferResponse>,
}

#[derive(Debug, Deserialize)]
struct WebhookEnvelope {
    id: String,
    #[serde(default)]
    name: String,
    data: WebhookData,
}

#[derive(Debug, Deserialize)]
struct WebhookData {
    object: WebhookTransfer,
}

#[derive(Debug, Deserialize)]
struct WebhookTransfer {
    id: String,
    #[serde(default)]
    request_id: Option<String>,
}

pub async fn ensure_schema(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS restless_authority.airwallex_connections (\
           company TEXT PRIMARY KEY, body JSONB NOT NULL, updated_by TEXT NOT NULL, \
           updated_at TIMESTAMPTZ NOT NULL DEFAULT now()\
         )",
    )
    .execute(pool)
    .await
    .context("create Airwallex connections")?;
    Ok(())
}

pub async fn set_connection(
    store: &crate::authority::AuthorityStore,
    company: &str,
    input: ConnectionInput,
    owner: &str,
) -> Result<Connection> {
    validate_connection(&input)?;
    let now = Utc::now();
    let connection = Connection {
        configured: input,
        updated_by: owner.to_string(),
        updated_at: now,
    };
    sqlx::query(
        "INSERT INTO restless_authority.airwallex_connections (company,body,updated_by) \
         VALUES ($1,$2,$3) ON CONFLICT (company) DO UPDATE \
         SET body=EXCLUDED.body,updated_by=EXCLUDED.updated_by,updated_at=now()",
    )
    .bind(company)
    .bind(serde_json::to_value(&connection)?)
    .bind(owner)
    .execute(store.pool())
    .await?;
    store
        .emit(
            company,
            "provider_connection_changed",
            Some(owner),
            serde_json::json!({
                "provider": "airwallex",
                "environment": connection.configured.environment,
                "account_ref": connection.configured.account_ref,
                "api_version": connection.configured.api_version,
                "read_scopes": connection.configured.read_scopes,
                "submit_scopes": connection.configured.submit_scopes,
                "approval_workflow_observed": connection.configured.approval_workflow_observed,
                "observed_at": connection.configured.observed_at,
            }),
        )
        .await?;
    Ok(connection)
}

pub async fn connection(
    store: &crate::authority::AuthorityStore,
    company: &str,
) -> Result<Option<Connection>> {
    let row =
        sqlx::query("SELECT body FROM restless_authority.airwallex_connections WHERE company=$1")
            .bind(company)
            .fetch_optional(store.pool())
            .await?;
    row.map(|row| serde_json::from_value(row.get("body")))
        .transpose()
        .context("decode Airwallex connection")
}

pub async fn observe_balances(
    config: &CompanyConfig,
    authority: &crate::authority::AuthorityStore,
) -> Result<BalanceObservation> {
    let connection = connection(authority, &config.name)
        .await?
        .context("Airwallex is not connected for this company")?;
    let key = crate::credential::resolve_finance(config, FinanceCredential::Read).await?;
    let client = Client::login(&connection, &key).await?;
    let balances: Vec<Balance> = client.get("/api/v1/balances/current").await?;
    let observation = BalanceObservation {
        provider: "airwallex".into(),
        account_ref: connection.configured.account_ref,
        balances,
        observed_at: Utc::now(),
    };
    authority
        .emit(
            &config.name,
            "finance_balance_observed",
            Some("daemon"),
            serde_json::to_value(&observation)?,
        )
        .await?;
    Ok(observation)
}

/// Live-check both scoped credentials against the actual account. This does
/// not claim Transfers:Write or approval behavior: only a sandbox submission
/// entering IN_APPROVAL can prove those consequential capabilities.
pub async fn probe_connection(
    config: &CompanyConfig,
    authority: &crate::authority::AuthorityStore,
) -> Result<ConnectionProbe> {
    let connection = connection(authority, &config.name)
        .await?
        .context("Airwallex is not connected for this company")?;
    let read_key = crate::credential::resolve_finance(config, FinanceCredential::Read).await?;
    let submit_key = crate::credential::resolve_finance(config, FinanceCredential::Submit).await?;
    let credentials_distinct = read_key.as_bytes() != submit_key.as_bytes();
    if !credentials_distinct {
        bail!("Airwallex read and submit bindings resolved to the same credential");
    }
    let read = Client::login(&connection, &read_key).await?;
    let _: Vec<Balance> = read.get("/api/v1/balances/current").await?;
    let _: TransferList = read.get("/api/v1/transfers?page=0&page_size=1").await?;
    let submit = Client::login(&connection, &submit_key).await?;
    let _: TransferList = submit.get("/api/v1/transfers?page=0&page_size=1").await?;
    let probe = ConnectionProbe {
        provider: "airwallex",
        environment: connection.configured.environment,
        account_ref: connection.configured.account_ref,
        api_version: connection.configured.api_version,
        read_authentication: "observed",
        read_balances: "observed",
        read_transfers: "observed",
        submit_authentication: "observed",
        submit_transfer_read: "observed",
        credentials_distinct,
        approval_workflow_observed: connection.configured.approval_workflow_observed,
        observed_at: Utc::now(),
    };
    authority
        .emit(
            &config.name,
            "provider_connection_probed",
            Some("daemon"),
            serde_json::to_value(&probe)?,
        )
        .await?;
    Ok(probe)
}

pub async fn submit_reserved(
    config: &CompanyConfig,
    authority: &crate::authority::AuthorityStore,
    key: &str,
) -> Result<PaymentIntent> {
    let connection = connection(authority, &config.name)
        .await?
        .context("Airwallex is not connected for this company")?;
    if !connection.configured.approval_workflow_observed {
        bail!("Airwallex API-initiated transfer approval was not live-probed for this account");
    }
    let intent = crate::finance::payment(authority, &config.name, key)
        .await?
        .context("payment must be reserved before provider submission")?;
    if intent.state != PaymentState::Reserved {
        bail!(
            "payment {:?} is {}; reconcile it instead of submitting again",
            key,
            intent.state.as_str()
        );
    }
    if intent.request.currency != "AUD" {
        bail!("the first Airwallex adapter slice supports exact AUD minor units only");
    }
    if key.chars().count() > 50 {
        bail!("Airwallex request_id is limited to 50 characters");
    }
    let api_key = crate::credential::resolve_finance(config, FinanceCredential::Submit).await?;
    let client = Client::login(&connection, &api_key).await?;
    let reference: String = intent.request.purpose.chars().take(140).collect();
    let body = serde_json::json!({
        "beneficiary_id": intent.request.provider_beneficiary_ref,
        "transfer_amount": minor_amount(intent.request.amount_minor),
        "transfer_currency": intent.request.currency,
        "source_currency": intent.request.currency,
        "transfer_method": "LOCAL",
        "reason": "business_expenses",
        "reference": reference,
        "request_id": key,
    });
    let created: TransferResponse = match client.post("/api/v1/transfers/create", &body).await {
        Ok(created) => created,
        Err(create_error) => match client.transfer_by_request_id(key).await {
            Ok(Some(found)) => found,
            Ok(None) | Err(_) => {
                crate::finance::mark_unknown(authority, &config.name, key).await?;
                return Err(create_error.context(
                    "Airwallex create outcome is unknown; reserved amount remains held until reconciliation",
                ));
            }
        },
    };
    if created.request_id.as_deref() != Some(key) {
        crate::finance::mark_unknown(authority, &config.name, key).await?;
        bail!(
            "Airwallex response did not preserve the exact request_id; outcome is unknown and the reservation remains held"
        );
    }
    let observed = crate::finance::confirm_provider_state(
        authority,
        &config.name,
        key,
        &created.id,
        &created.status,
        Some(&connection.configured.approval_url),
    )
    .await?;
    if observed.payment.state != PaymentState::InApproval {
        bail!(
            "Airwallex created transfer {} in raw state {:?}, not required IN_APPROVAL; it was recorded and financial effects should be frozen",
            created.id,
            created.status
        );
    }
    Ok(observed.payment)
}

pub async fn reconcile_payment(
    config: &CompanyConfig,
    authority: &crate::authority::AuthorityStore,
    key: &str,
) -> Result<crate::finance::ProviderObservation> {
    let connection = connection(authority, &config.name)
        .await?
        .context("Airwallex is not connected for this company")?;
    let intent = crate::finance::payment(authority, &config.name, key)
        .await?
        .context("payment intent not found")?;
    let api_key = crate::credential::resolve_finance(config, FinanceCredential::Read).await?;
    let client = Client::login(&connection, &api_key).await?;
    let transfer = match intent.provider_transfer_id.as_deref() {
        Some(id) => {
            client
                .get::<TransferResponse>(&format!("/api/v1/transfers/{id}"))
                .await?
        }
        None => client.transfer_by_request_id(key).await?.context(
            "provider has no transfer for the reserved request_id; outcome remains unknown",
        )?,
    };
    if transfer
        .request_id
        .as_deref()
        .is_some_and(|observed| observed != key)
    {
        bail!("Airwallex transfer response carried a different request_id");
    }
    crate::finance::confirm_provider_state(
        authority,
        &config.name,
        key,
        &transfer.id,
        &transfer.status,
        Some(&connection.configured.approval_url),
    )
    .await
}

/// Verify before parsing, then reconcile from authenticated API truth before
/// recording the event once. A redelivery deliberately re-runs the idempotent
/// read: this closes the crash window where payment state committed but the
/// OrgIntel continuation did not. The payload is a wake-up hint, never
/// confirmation by itself.
pub async fn receive_webhook(
    config: &CompanyConfig,
    authority: &crate::authority::AuthorityStore,
    timestamp: &str,
    signature: &str,
    raw_body: &[u8],
) -> Result<Option<crate::finance::ProviderObservation>> {
    let signing_secret =
        crate::credential::resolve_finance(config, FinanceCredential::Webhook).await?;
    verify_webhook(&signing_secret, timestamp, signature, raw_body, Utc::now())?;
    let event: WebhookEnvelope =
        serde_json::from_slice(raw_body).context("decode verified Airwallex webhook")?;
    if !event.name.starts_with("payout.transfer.") {
        return Ok(None);
    }
    let key = match event.data.object.request_id {
        Some(key) => {
            if crate::finance::payment(authority, &config.name, &key)
                .await?
                .is_none()
            {
                return Ok(None);
            }
            key
        }
        None => {
            let Some(intent) = crate::finance::payment_by_provider_id(
                authority,
                &config.name,
                &event.data.object.id,
            )
            .await?
            else {
                return Ok(None);
            };
            intent.request.idempotency_key
        }
    };
    // Reconcile before committing the dedupe receipt. If the authenticated
    // provider read is transiently unavailable, Airwallex's retry must get a
    // real second chance rather than being suppressed by an event row whose
    // consequence was never observed.
    let observation = reconcile_payment(config, authority, &key).await?;
    if observation.payment.provider_transfer_id.as_deref() != Some(event.data.object.id.as_str()) {
        bail!("verified Airwallex event did not match the reconciled provider transfer");
    }
    authority
        .emit_inbound_once(
            &config.name,
            serde_json::json!({
                "provider": "airwallex",
                "provider_event_id": event.id,
                "provider_transfer_id": event.data.object.id,
                "provider_payment_state": observation.payment.state,
                "received_at": Utc::now(),
            }),
        )
        .await?;
    Ok(Some(observation))
}

fn validate_connection(input: &ConnectionInput) -> Result<()> {
    if input.client_id.trim().is_empty() || input.account_ref.trim().is_empty() {
        bail!("Airwallex connection needs client and account references");
    }
    let api_version = NaiveDate::parse_from_str(&input.api_version, "%Y-%m-%d")
        .context("Airwallex API version must be an observed YYYY-MM-DD date")?;
    if api_version.to_string() != input.api_version {
        bail!("Airwallex API version must be an observed YYYY-MM-DD date");
    }
    let approval = Url::parse(&input.approval_url).context("parse Airwallex approval URL")?;
    if approval.scheme() != "https"
        || !approval
            .host_str()
            .is_some_and(|host| host == "airwallex.com" || host.ends_with(".airwallex.com"))
    {
        bail!("Airwallex owner action must use an https airwallex.com origin");
    }
    let has = |scopes: &[String], wanted: &str| scopes.iter().any(|scope| scope == wanted);
    if !has(&input.read_scopes, "Balances:Read") || !has(&input.read_scopes, "Transfers:Read") {
        bail!("Airwallex read key needs the observed Balances:Read and Transfers:Read contract");
    }
    if !has(&input.submit_scopes, "Transfers:Read")
        || !has(&input.submit_scopes, "Transfers:Write")
        || input
            .submit_scopes
            .iter()
            .any(|scope| scope.starts_with("Beneficiaries:") || scope.contains("Admin"))
    {
        bail!(
            "Airwallex submit key needs Transfers read/write and must not carry Beneficiary or admin scope"
        );
    }
    if input.approval_workflow_observed && input.observed_at.is_none() {
        bail!("an observed approval workflow needs an observation time");
    }
    Ok(())
}

fn minor_amount(amount_minor: i64) -> String {
    format!("{}.{:02}", amount_minor / 100, amount_minor % 100)
}

fn verify_webhook(
    secret: &str,
    timestamp: &str,
    signature: &str,
    body: &[u8],
    now: DateTime<Utc>,
) -> Result<()> {
    let millis: i64 = timestamp
        .parse()
        .context("Airwallex timestamp is not milliseconds")?;
    let observed = DateTime::<Utc>::from_timestamp_millis(millis)
        .context("Airwallex timestamp is out of range")?;
    if (now - observed).num_seconds().unsigned_abs() > 300 {
        bail!("Airwallex webhook timestamp is outside the five-minute tolerance");
    }
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())?;
    mac.update(timestamp.as_bytes());
    mac.update(body);
    let supplied = decode_hex(signature)?;
    mac.verify_slice(&supplied)
        .map_err(|_| anyhow::anyhow!("Airwallex webhook signature is invalid"))
}

fn decode_hex(value: &str) -> Result<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        bail!("signature hex has odd length");
    }
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair)?;
            Ok(u8::from_str_radix(text, 16)?)
        })
        .collect()
}

#[derive(Clone)]
struct AccessToken {
    value: String,
    expires_at: DateTime<Utc>,
}

impl AccessToken {
    fn is_current(&self, now: DateTime<Utc>) -> bool {
        self.expires_at > now + Duration::seconds(TOKEN_REFRESH_MARGIN_SECONDS)
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
struct TokenCacheKey {
    base: String,
    client_id: String,
    api_key_digest: [u8; 32],
}

static TOKEN_CACHE: OnceLock<tokio::sync::Mutex<HashMap<TokenCacheKey, AccessToken>>> =
    OnceLock::new();

fn token_cache() -> &'static tokio::sync::Mutex<HashMap<TokenCacheKey, AccessToken>> {
    TOKEN_CACHE.get_or_init(|| tokio::sync::Mutex::new(HashMap::new()))
}

fn token_cache_key(base: &Url, client_id: &str, api_key: &str) -> TokenCacheKey {
    let digest = Sha256::digest(api_key.as_bytes());
    let mut api_key_digest = [0_u8; 32];
    api_key_digest.copy_from_slice(&digest);
    TokenCacheKey {
        base: base.as_str().to_string(),
        client_id: client_id.to_string(),
        api_key_digest,
    }
}

async fn authenticate(
    http: &reqwest::Client,
    base: &Url,
    client_id: &str,
    api_key: &str,
    force_refresh: bool,
) -> Result<AccessToken> {
    let key = token_cache_key(base, client_id, api_key);
    // Holding this process-local lock across login prevents concurrent wakes
    // from exhausting Airwallex's authentication rate limit. The cache stores
    // only the short-lived bearer token and an API-key digest.
    let mut cache = token_cache().lock().await;
    if !force_refresh {
        if let Some(token) = cache.get(&key).filter(|token| token.is_current(Utc::now())) {
            return Ok(token.clone());
        }
    }
    let response = http
        .post(base.join("/api/v1/authentication/login")?)
        .header("x-client-id", client_id)
        .header("x-api-key", api_key)
        .send()
        .await
        .context("Airwallex authentication transport failed")?;
    let login: LoginResponse = provider_json(response, "authenticate").await?;
    if login.token.trim().is_empty() {
        bail!("Airwallex authentication returned an empty access token");
    }
    let expires_at = DateTime::parse_from_rfc3339(&login.expires_at)
        .context("Airwallex authentication returned an invalid token expiry")?
        .with_timezone(&Utc);
    let token = AccessToken {
        value: login.token,
        expires_at,
    };
    if !token.is_current(Utc::now()) {
        bail!("Airwallex authentication returned an already-expiring access token");
    }
    cache.insert(key, token.clone());
    Ok(token)
}

struct Client {
    http: reqwest::Client,
    base: Url,
    client_id: String,
    api_key: String,
    api_version: String,
    token: tokio::sync::Mutex<AccessToken>,
}

impl Client {
    async fn login(connection: &Connection, api_key: &str) -> Result<Self> {
        let base = Url::parse(connection.configured.environment.base_url())?;
        Self::login_at(
            base,
            &connection.configured.client_id,
            api_key,
            &connection.configured.api_version,
        )
        .await
    }

    async fn login_at(
        base: Url,
        client_id: &str,
        api_key: &str,
        api_version: &str,
    ) -> Result<Self> {
        let http = reqwest::Client::builder()
            .https_only(base.scheme() == "https")
            .timeout(std::time::Duration::from_secs(PROVIDER_TIMEOUT_SECONDS))
            .build()?;
        let token = authenticate(&http, &base, client_id, api_key, false).await?;
        Ok(Self {
            http,
            base,
            client_id: client_id.to_string(),
            api_key: api_key.to_string(),
            api_version: api_version.to_string(),
            token: tokio::sync::Mutex::new(token),
        })
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request(Method::GET, path, None).await
    }

    async fn post<T: DeserializeOwned>(&self, path: &str, body: &serde_json::Value) -> Result<T> {
        self.request(Method::POST, path, Some(body)).await
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<T> {
        let url = self.base.join(path)?;
        let response = self.send(method.clone(), url.clone(), body, false).await?;
        let response = if response.status() == StatusCode::UNAUTHORIZED {
            // A rejected bearer token is known not to have performed the
            // requested effect, so one forced re-authentication is safe. No
            // other transport/provider failure is retried here.
            self.send(method, url, body, true).await?
        } else {
            response
        };
        provider_json(response, "request").await
    }

    async fn send(
        &self,
        method: Method,
        url: Url,
        body: Option<&serde_json::Value>,
        force_refresh: bool,
    ) -> Result<reqwest::Response> {
        let token = if force_refresh {
            let refreshed =
                authenticate(&self.http, &self.base, &self.client_id, &self.api_key, true).await?;
            *self.token.lock().await = refreshed.clone();
            refreshed
        } else {
            let current = self.token.lock().await.clone();
            if current.is_current(Utc::now()) {
                current
            } else {
                let refreshed = authenticate(
                    &self.http,
                    &self.base,
                    &self.client_id,
                    &self.api_key,
                    false,
                )
                .await?;
                *self.token.lock().await = refreshed.clone();
                refreshed
            }
        };
        let mut request = self
            .http
            .request(method, url)
            .bearer_auth(&token.value)
            .header("x-api-version", &self.api_version)
            .header("content-type", "application/json");
        if let Some(body) = body {
            request = request.json(body);
        }
        request
            .send()
            .await
            .context("Airwallex API transport failed")
    }

    async fn transfer_by_request_id(&self, request_id: &str) -> Result<Option<TransferResponse>> {
        let mut url = self.base.join("/api/v1/transfers")?;
        url.query_pairs_mut().append_pair("request_id", request_id);
        let response = self.send(Method::GET, url.clone(), None, false).await?;
        let response = if response.status() == StatusCode::UNAUTHORIZED {
            self.send(Method::GET, url, None, true).await?
        } else {
            response
        };
        let list: TransferList = provider_json(response, "look up transfer").await?;
        let mut exact = list
            .items
            .into_iter()
            .filter(|item| item.request_id.as_deref() == Some(request_id));
        let first = exact.next();
        if exact.next().is_some() {
            bail!("Airwallex returned multiple transfers for one request_id");
        }
        Ok(first)
    }
}

async fn provider_json<T: DeserializeOwned>(
    response: reqwest::Response,
    operation: &str,
) -> Result<T> {
    let status = response.status();
    if !status.is_success() {
        // Never retain or print an unfiltered provider body: providers may
        // reflect request headers or sensitive submitted values in errors.
        let class = match status {
            StatusCode::UNAUTHORIZED => "credential rejected",
            StatusCode::FORBIDDEN => "scope or account denied",
            StatusCode::TOO_MANY_REQUESTS => "rate limited",
            value if value.is_server_error() => "provider unavailable",
            _ => "request rejected",
        };
        bail!("Airwallex {operation} failed: {status} {class}");
    }
    response
        .json()
        .await
        .with_context(|| format!("Airwallex {operation} returned invalid JSON"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        extract::Query,
        http::{HeaderMap, StatusCode},
        routing::{get, post},
        Json, Router,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    const TEST_API_VERSION: &str = "2026-07-17";

    #[test]
    fn connection_refuses_beneficiary_write_and_unobserved_claims() {
        let base = ConnectionInput {
            environment: Environment::Sandbox,
            api_version: TEST_API_VERSION.into(),
            client_id: "client".into(),
            account_ref: "account".into(),
            approval_url: "https://demo.airwallex.com/app/transfers".into(),
            read_scopes: vec!["Balances:Read".into(), "Transfers:Read".into()],
            submit_scopes: vec!["Transfers:Read".into(), "Transfers:Write".into()],
            approval_workflow_observed: false,
            observed_at: None,
        };
        validate_connection(&base).unwrap();
        assert!(validate_connection(&ConnectionInput {
            submit_scopes: vec!["Transfers:Write".into(), "Beneficiaries:Write".into()],
            ..base.clone()
        })
        .is_err());
        assert!(validate_connection(&ConnectionInput {
            approval_workflow_observed: true,
            ..base
        })
        .is_err());
    }

    #[test]
    fn connection_requires_an_observed_date_version_and_business_account_sandbox() {
        assert_eq!(
            Environment::Sandbox.base_url(),
            "https://api.sandbox.airwallex.com"
        );
        let input = ConnectionInput {
            environment: Environment::Sandbox,
            api_version: "latest".into(),
            client_id: "client".into(),
            account_ref: "account".into(),
            approval_url: "https://demo.airwallex.com/app/transfers".into(),
            read_scopes: vec!["Balances:Read".into(), "Transfers:Read".into()],
            submit_scopes: vec!["Transfers:Read".into(), "Transfers:Write".into()],
            approval_workflow_observed: false,
            observed_at: None,
        };
        assert!(validate_connection(&input).is_err());
    }

    #[test]
    fn webhook_signature_uses_exact_timestamp_plus_raw_body() {
        let timestamp = "1787068800000";
        let body = br#"{"id":"evt-1"}"#;
        let mut mac = Hmac::<Sha256>::new_from_slice(b"secret").unwrap();
        mac.update(timestamp.as_bytes());
        mac.update(body);
        let signature = mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let now = DateTime::<Utc>::from_timestamp_millis(1_787_068_800_000).unwrap();
        verify_webhook("secret", timestamp, &signature, body, now).unwrap();
        assert!(verify_webhook("secret", timestamp, &signature, b"{}", now).is_err());
    }

    #[tokio::test]
    async fn fake_provider_preserves_scoped_auth_version_and_request_identity() {
        let app = Router::new()
            .route(
                "/api/v1/authentication/login",
                post(|headers: HeaderMap| async move {
                    assert_eq!(headers["x-client-id"], "client-id");
                    assert_eq!(headers["x-api-key"], "read-or-submit-key");
                    Json(serde_json::json!({
                        "token": "short-lived-token",
                        "expires_at": (Utc::now() + Duration::minutes(30)).to_rfc3339()
                    }))
                }),
            )
            .route(
                "/api/v1/transfers/create",
                post(
                    |headers: HeaderMap, Json(body): Json<serde_json::Value>| async move {
                        assert_eq!(headers["authorization"], "Bearer short-lived-token");
                        assert_eq!(headers["x-api-version"], TEST_API_VERSION);
                        assert_eq!(body["beneficiary_id"], "beneficiary-1");
                        assert_eq!(body["request_id"], "payment-1");
                        Json(serde_json::json!({
                            "id": "transfer-1",
                            "status": "IN_APPROVAL",
                            "request_id": "payment-1"
                        }))
                    },
                ),
            )
            .route(
                "/api/v1/transfers",
                get(
                    |headers: HeaderMap,
                     Query(query): Query<std::collections::HashMap<String, String>>| async move {
                        assert_eq!(headers["authorization"], "Bearer short-lived-token");
                        assert_eq!(headers["x-api-version"], TEST_API_VERSION);
                        assert_eq!(
                            query.get("request_id").map(String::as_str),
                            Some("payment-1")
                        );
                        Json(serde_json::json!({
                            "items": [{
                                "id": "transfer-1",
                                "status": "IN_APPROVAL",
                                "request_id": "payment-1"
                            }]
                        }))
                    },
                ),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let client = Client::login_at(
            Url::parse(&format!("http://{address}")).unwrap(),
            "client-id",
            "read-or-submit-key",
            TEST_API_VERSION,
        )
        .await
        .unwrap();
        let created: TransferResponse = client
            .post(
                "/api/v1/transfers/create",
                &serde_json::json!({
                    "beneficiary_id": "beneficiary-1",
                    "request_id": "payment-1"
                }),
            )
            .await
            .unwrap();
        assert_eq!(created.id, "transfer-1");
        assert_eq!(created.status, "IN_APPROVAL");
        assert_eq!(created.request_id.as_deref(), Some("payment-1"));
        assert_eq!(
            client
                .transfer_by_request_id("payment-1")
                .await
                .unwrap()
                .unwrap()
                .id,
            "transfer-1"
        );
        server.abort();
    }

    #[tokio::test]
    async fn expired_access_token_is_refreshed_once_then_reused_in_memory() {
        let login_count = Arc::new(AtomicUsize::new(0));
        let login_counter = Arc::clone(&login_count);
        let app = Router::new()
            .route(
                "/api/v1/authentication/login",
                post(move || {
                    let login_counter = Arc::clone(&login_counter);
                    async move {
                        login_counter.fetch_add(1, Ordering::SeqCst);
                        Json(serde_json::json!({
                            "token": "refreshed-token",
                            "expires_at": (Utc::now() + Duration::minutes(30)).to_rfc3339()
                        }))
                    }
                }),
            )
            .route(
                "/probe",
                get(|headers: HeaderMap| async move {
                    assert_eq!(headers["authorization"], "Bearer refreshed-token");
                    Json(serde_json::json!({"ok": true}))
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let base = Url::parse(&format!("http://{address}")).unwrap();
        let cache_key = token_cache_key(&base, "client-id", "api-key");
        token_cache().lock().await.insert(
            cache_key.clone(),
            AccessToken {
                value: "expired-token".into(),
                expires_at: Utc::now() - Duration::minutes(1),
            },
        );

        let first = Client::login_at(base.clone(), "client-id", "api-key", TEST_API_VERSION)
            .await
            .unwrap();
        let _: serde_json::Value = first.get("/probe").await.unwrap();
        let second = Client::login_at(base, "client-id", "api-key", TEST_API_VERSION)
            .await
            .unwrap();
        let _: serde_json::Value = second.get("/probe").await.unwrap();

        assert_eq!(login_count.load(Ordering::SeqCst), 1);
        token_cache().lock().await.remove(&cache_key);
        server.abort();
    }

    #[tokio::test]
    async fn rejected_bearer_token_reauthenticates_once_without_broad_retry() {
        let login_count = Arc::new(AtomicUsize::new(0));
        let request_count = Arc::new(AtomicUsize::new(0));
        let login_counter = Arc::clone(&login_count);
        let request_counter = Arc::clone(&request_count);
        let app = Router::new()
            .route(
                "/api/v1/authentication/login",
                post(move || {
                    let login_counter = Arc::clone(&login_counter);
                    async move {
                        let attempt = login_counter.fetch_add(1, Ordering::SeqCst) + 1;
                        Json(serde_json::json!({
                            "token": format!("token-{attempt}"),
                            "expires_at": (Utc::now() + Duration::minutes(30)).to_rfc3339()
                        }))
                    }
                }),
            )
            .route(
                "/probe",
                get(move |headers: HeaderMap| {
                    let request_counter = Arc::clone(&request_counter);
                    async move {
                        request_counter.fetch_add(1, Ordering::SeqCst);
                        if headers["authorization"] == "Bearer token-2" {
                            (StatusCode::OK, Json(serde_json::json!({"ok": true})))
                        } else {
                            (
                                StatusCode::UNAUTHORIZED,
                                Json(serde_json::json!({"error": "expired"})),
                            )
                        }
                    }
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let base = Url::parse(&format!("http://{address}")).unwrap();
        let cache_key = token_cache_key(&base, "client-id", "api-key");

        let client = Client::login_at(base, "client-id", "api-key", TEST_API_VERSION)
            .await
            .unwrap();
        let response: serde_json::Value = client.get("/probe").await.unwrap();

        assert_eq!(response["ok"], true);
        assert_eq!(login_count.load(Ordering::SeqCst), 2);
        assert_eq!(request_count.load(Ordering::SeqCst), 2);
        token_cache().lock().await.remove(&cache_key);
        server.abort();
    }

    #[tokio::test]
    async fn fake_provider_cannot_reflect_sensitive_response_text() {
        let app = Router::new().route(
            "/failure",
            get(|| async {
                (
                    StatusCode::BAD_GATEWAY,
                    "reflected-finance-secret-must-never-escape",
                )
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let response = reqwest::get(format!("http://{address}/failure"))
            .await
            .unwrap();
        let error = format!(
            "{:#}",
            provider_json::<serde_json::Value>(response, "test")
                .await
                .unwrap_err()
        );
        assert!(error.contains("502"), "{error}");
        assert!(!error.contains("reflected-finance-secret"), "{error}");
        server.abort();
    }
}
