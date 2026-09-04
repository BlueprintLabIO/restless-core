//! Owner entry modes for the account plane.
//!
//! **Local** preserves ADR 0001: loopback is the owner boundary. **Network**
//! implements ADR 0007: Restless Cloud proves a human's active membership with
//! a short-lived Ed25519 assertion, and Core exchanges that assertion once for
//! a browser session. Network position is never identity.
//!
//! This module is the public Cloud/Core seam. Fleet owns login, memberships and
//! private signing keys. Core fetches only Fleet's public JWKS. Authentication
//! bounds the human and company; it does not grant a Core Authority capability.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use base64::Engine as _;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex as AsyncMutex, RwLock};
use tokio::time::Instant;
use url::Url;
use uuid::Uuid;

/// Bumped only through the release contract's change governance. Cloud pins
/// this value from the release manifest.
pub(crate) const ASSERTION_CONTRACT_VERSION: u32 = 1;

const TOKEN_TYPE: &str = "JWT";
const ALGORITHM: &str = "EdDSA";
const AUDIENCE: &str = "restless-core-account-plane";
const MAX_ASSERTION_BYTES: usize = 8 * 1024;
const MAX_ASSERTION_LIFETIME: Duration = Duration::from_secs(60);
const CLOCK_SKEW: Duration = Duration::from_secs(5);
const MAX_JWKS_BYTES: usize = 64 * 1024;
const MAX_JWKS_KEYS: usize = 16;
const JWKS_FRESH_FOR: Duration = Duration::from_secs(5 * 60);
const MIN_JWKS_REFRESH_INTERVAL: Duration = Duration::from_secs(30);
const JWKS_TIMEOUT: Duration = Duration::from_secs(3);
const DEFAULT_SESSION_TTL: Duration = Duration::from_secs(12 * 60 * 60);

/// Every distinct way an assertion can be refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Refusal {
    Malformed(&'static str),
    UnsupportedVersion { got: u32, supported: u32 },
    UnknownIssuer,
    WrongAudience,
    UnknownKeyVersion,
    JwksUnavailable,
    BadSignature,
    NotYetValid,
    Expired,
    LifetimeTooLong,
    WrongOwner,
    WrongPlane,
    UnknownRole,
    Replayed,
    ReplayStoreUnavailable,
}

impl Refusal {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Refusal::Malformed(_) => "assertion_malformed",
            Refusal::UnsupportedVersion { .. } => "assertion_unsupported_version",
            Refusal::UnknownIssuer => "assertion_unknown_issuer",
            Refusal::WrongAudience => "assertion_wrong_audience",
            Refusal::UnknownKeyVersion => "assertion_unknown_key_version",
            Refusal::JwksUnavailable => "assertion_jwks_unavailable",
            Refusal::BadSignature => "assertion_bad_signature",
            Refusal::NotYetValid => "assertion_not_yet_valid",
            Refusal::Expired => "assertion_expired",
            Refusal::LifetimeTooLong => "assertion_lifetime_too_long",
            Refusal::WrongOwner => "assertion_wrong_owner",
            Refusal::WrongPlane => "assertion_wrong_plane",
            Refusal::UnknownRole => "assertion_unknown_role",
            Refusal::Replayed => "assertion_replayed",
            Refusal::ReplayStoreUnavailable => "assertion_replay_store_unavailable",
        }
    }

    pub(crate) fn message(&self) -> String {
        match self {
            Refusal::Malformed(what) => format!("entry assertion is malformed: {what}"),
            Refusal::UnsupportedVersion { got, supported } => format!(
                "entry assertion contract version {got} is not supported; this plane supports {supported}"
            ),
            Refusal::UnknownIssuer => "entry assertion issuer is not trusted by this plane".into(),
            Refusal::WrongAudience => {
                "entry assertion was not minted for a Core account plane".into()
            }
            Refusal::UnknownKeyVersion => "entry assertion names an unknown signing key".into(),
            Refusal::JwksUnavailable => "entry assertion keys are temporarily unavailable".into(),
            Refusal::BadSignature => "entry assertion signature is invalid".into(),
            Refusal::NotYetValid => "entry assertion is not valid yet".into(),
            Refusal::Expired => "entry assertion has expired".into(),
            Refusal::LifetimeTooLong => {
                "entry assertion lifetime exceeds the public contract".into()
            }
            Refusal::WrongOwner => "entry assertion is routed to a different owner".into(),
            Refusal::WrongPlane => "entry assertion is routed to a different plane".into(),
            Refusal::UnknownRole => "entry assertion names an unsupported membership role".into(),
            Refusal::Replayed => "entry assertion has already been used".into(),
            Refusal::ReplayStoreUnavailable => {
                "entry assertion replay protection is temporarily unavailable".into()
            }
        }
    }

    pub(crate) fn is_temporary(&self) -> bool {
        matches!(
            self,
            Refusal::JwksUnavailable | Refusal::ReplayStoreUnavailable
        )
    }
}

/// Which companies on this plane a browser session may reach.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum CompanyScope {
    /// The local owner reaches every locally configured company.
    Owner,
    /// A Cloud handoff reaches exactly the company named by its membership.
    Company { company: String },
}

impl CompanyScope {
    pub(crate) fn permits(&self, company: &str) -> bool {
        match self {
            CompanyScope::Owner => true,
            CompanyScope::Company { company: allowed } => allowed == company,
        }
    }
}

/// Exact Fleet v1 handoff claims. Semantic additions require a version bump,
/// rather than silently becoming unaudited authorization inputs in Core.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssertionClaims {
    iss: String,
    aud: String,
    sub: String,
    jti: Uuid,
    exp: i64,
    iat: i64,
    kid: String,
    owner_id: Uuid,
    plane_id: Uuid,
    company_id: Uuid,
    cell_id: Uuid,
    membership_id: String,
    membership_role: String,
    assertion_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssertionHeader {
    alg: String,
    typ: String,
    kid: String,
}

#[derive(Debug, Deserialize)]
struct JwkSet {
    keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize)]
struct Jwk {
    kty: String,
    crv: String,
    alg: String,
    #[serde(rename = "use")]
    use_: String,
    kid: String,
    x: String,
}

/// The only identity downstream Core code may consult. There is deliberately
/// no Authority capability in this type.
#[derive(Debug, Clone)]
pub(crate) struct VerifiedIdentity {
    pub user: String,
    pub owner: String,
    pub scope: CompanyScope,
    pub role: String,
    pub actor: Option<String>,
    pub correlation: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SignaturePolicy {
    Enforce,
    /// Lets the adversarial test prove which gate rejects a forged signature.
    #[cfg(test)]
    Skip,
}

#[async_trait]
trait VerificationKeySource: Send + Sync {
    async fn key(&self, key_id: &str) -> Result<VerifyingKey, Refusal>;
}

struct JwksCache {
    keys: HashMap<String, VerifyingKey>,
    refreshed_at: Option<Instant>,
}

/// Rotation-aware, bounded JWKS reader. Unknown `kid` values cannot turn the
/// account plane into an unbounded fetch proxy because negative refreshes are
/// rate-limited.
struct HttpJwksSource {
    url: Url,
    client: reqwest::Client,
    cache: RwLock<JwksCache>,
    refresh: AsyncMutex<()>,
}

impl HttpJwksSource {
    fn new(url: Url, allow_insecure_http: bool) -> anyhow::Result<Self> {
        let client = reqwest::Client::builder()
            .https_only(!allow_insecure_http)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(JWKS_TIMEOUT)
            .build()?;
        Ok(Self {
            url,
            client,
            cache: RwLock::new(JwksCache {
                keys: HashMap::new(),
                refreshed_at: None,
            }),
            refresh: AsyncMutex::new(()),
        })
    }

    async fn cached_key(&self, key_id: &str) -> Option<(VerifyingKey, bool)> {
        let cache = self.cache.read().await;
        let key = cache.keys.get(key_id).cloned()?;
        let fresh = cache
            .refreshed_at
            .is_some_and(|at| at.elapsed() < JWKS_FRESH_FOR);
        Some((key, fresh))
    }

    async fn recently_refreshed_without(&self, key_id: &str) -> bool {
        let cache = self.cache.read().await;
        !cache.keys.contains_key(key_id)
            && cache
                .refreshed_at
                .is_some_and(|at| at.elapsed() < MIN_JWKS_REFRESH_INTERVAL)
    }

    async fn fetch(&self) -> Result<HashMap<String, VerifyingKey>, Refusal> {
        let mut response = self
            .client
            .get(self.url.clone())
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|_| Refusal::JwksUnavailable)?;
        if !response.status().is_success()
            || response
                .content_length()
                .is_some_and(|length| length > MAX_JWKS_BYTES as u64)
        {
            return Err(Refusal::JwksUnavailable);
        }

        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| Refusal::JwksUnavailable)?
        {
            if bytes.len().saturating_add(chunk.len()) > MAX_JWKS_BYTES {
                return Err(Refusal::JwksUnavailable);
            }
            bytes.extend_from_slice(&chunk);
        }
        parse_jwks(&bytes)
    }
}

#[async_trait]
impl VerificationKeySource for HttpJwksSource {
    async fn key(&self, key_id: &str) -> Result<VerifyingKey, Refusal> {
        if !valid_key_id(key_id) {
            return Err(Refusal::Malformed("invalid key id"));
        }
        if let Some((key, true)) = self.cached_key(key_id).await {
            return Ok(key);
        }
        if self.recently_refreshed_without(key_id).await {
            return Err(Refusal::UnknownKeyVersion);
        }

        let _refresh = self.refresh.lock().await;
        if let Some((key, true)) = self.cached_key(key_id).await {
            return Ok(key);
        }
        if self.recently_refreshed_without(key_id).await {
            return Err(Refusal::UnknownKeyVersion);
        }

        let keys = self.fetch().await?;
        let key = keys.get(key_id).cloned();
        *self.cache.write().await = JwksCache {
            keys,
            refreshed_at: Some(Instant::now()),
        };
        key.ok_or(Refusal::UnknownKeyVersion)
    }
}

fn parse_jwks(bytes: &[u8]) -> Result<HashMap<String, VerifyingKey>, Refusal> {
    let set: JwkSet = serde_json::from_slice(bytes).map_err(|_| Refusal::JwksUnavailable)?;
    if set.keys.is_empty() || set.keys.len() > MAX_JWKS_KEYS {
        return Err(Refusal::JwksUnavailable);
    }
    let mut keys = HashMap::with_capacity(set.keys.len());
    for jwk in set.keys {
        if jwk.kty != "OKP"
            || jwk.crv != "Ed25519"
            || jwk.alg != ALGORITHM
            || jwk.use_ != "sig"
            || !valid_key_id(&jwk.kid)
        {
            return Err(Refusal::JwksUnavailable);
        }
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(jwk.x)
            .map_err(|_| Refusal::JwksUnavailable)?;
        let raw: [u8; 32] = raw.try_into().map_err(|_| Refusal::JwksUnavailable)?;
        let key = VerifyingKey::from_bytes(&raw).map_err(|_| Refusal::JwksUnavailable)?;
        if keys.insert(jwk.kid, key).is_some() {
            return Err(Refusal::JwksUnavailable);
        }
    }
    Ok(keys)
}

fn valid_key_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

/// Atomic replay-store contract. The in-memory implementation is correct for
/// one process. Hosted deployment still needs a durable implementation before
/// replay rejection can survive an account-plane restart.
#[async_trait]
pub(crate) trait AssertionReplayStore: Send + Sync {
    /// Returns true only for the first successful insert of this identity.
    async fn consume(
        &self,
        identity: Uuid,
        expires_at: SystemTime,
        now: SystemTime,
    ) -> anyhow::Result<bool>;
}

#[cfg(test)]
#[derive(Default)]
struct InMemoryAssertionReplayStore {
    consumed: AsyncMutex<HashMap<Uuid, SystemTime>>,
}

#[cfg(test)]
#[async_trait]
impl AssertionReplayStore for InMemoryAssertionReplayStore {
    async fn consume(
        &self,
        identity: Uuid,
        expires_at: SystemTime,
        now: SystemTime,
    ) -> anyhow::Result<bool> {
        let mut consumed = self.consumed.lock().await;
        consumed.retain(|_, expiry| *expiry > now);
        if consumed.contains_key(&identity) {
            return Ok(false);
        }
        consumed.insert(identity, expires_at);
        Ok(true)
    }
}

#[async_trait]
impl AssertionReplayStore for crate::authority::AuthorityStore {
    async fn consume(
        &self,
        identity: Uuid,
        expires_at: SystemTime,
        now: SystemTime,
    ) -> anyhow::Result<bool> {
        self.consume_entry_assertion(identity, expires_at.into(), now.into())
            .await
    }
}

/// Complete network-mode verifier configuration. A partial configuration is a
/// startup error; there is no network-position fallback.
pub(crate) struct NetworkEntry {
    issuer: String,
    issuer_origin: String,
    owner: Uuid,
    plane: Uuid,
    /// Exact public authority (`host[:port]`) for Host validation.
    host: String,
    plane_origin: String,
    secure_cookie: bool,
    keys: Arc<dyn VerificationKeySource>,
    replay: Arc<dyn AssertionReplayStore>,
    session_ttl: Duration,
}

impl NetworkEntry {
    pub(crate) fn host(&self) -> &str {
        &self.host
    }

    pub(crate) fn plane_origin(&self) -> &str {
        &self.plane_origin
    }

    pub(crate) fn issuer_origin(&self) -> &str {
        &self.issuer_origin
    }

    pub(crate) fn secure_cookie(&self) -> bool {
        self.secure_cookie
    }

    pub(crate) fn session_ttl(&self) -> Duration {
        self.session_ttl
    }

    pub(crate) async fn verify(&self, token: &str) -> Result<VerifiedIdentity, Refusal> {
        self.verify_at(token, SystemTime::now(), SignaturePolicy::Enforce)
            .await
    }

    async fn verify_at(
        &self,
        token: &str,
        now: SystemTime,
        policy: SignaturePolicy,
    ) -> Result<VerifiedIdentity, Refusal> {
        if token.is_empty() || token.len() > MAX_ASSERTION_BYTES {
            return Err(Refusal::Malformed("token length is invalid"));
        }
        let mut parts = token.split('.');
        let header_b64 = parts.next().ok_or(Refusal::Malformed("no header"))?;
        let payload_b64 = parts.next().ok_or(Refusal::Malformed("no payload"))?;
        let signature_b64 = parts.next().ok_or(Refusal::Malformed("no signature"))?;
        if parts.next().is_some() {
            return Err(Refusal::Malformed("too many segments"));
        }
        if header_b64.is_empty() || payload_b64.is_empty() || signature_b64.is_empty() {
            return Err(Refusal::Malformed("empty segment"));
        }

        let header: AssertionHeader = decode_segment(header_b64, "header")?;
        if header.typ != TOKEN_TYPE {
            return Err(Refusal::Malformed("unexpected token type"));
        }
        if header.alg != ALGORITHM {
            return Err(Refusal::Malformed("unexpected signature algorithm"));
        }
        if !valid_key_id(&header.kid) {
            return Err(Refusal::Malformed("invalid key id"));
        }

        let claims: AssertionClaims = decode_segment(payload_b64, "payload")?;
        if claims.assertion_version != ASSERTION_CONTRACT_VERSION {
            return Err(Refusal::UnsupportedVersion {
                got: claims.assertion_version,
                supported: ASSERTION_CONTRACT_VERSION,
            });
        }
        if claims.kid != header.kid {
            return Err(Refusal::Malformed("header and payload key ids differ"));
        }

        let key = self.keys.key(&header.kid).await?;
        let enforce = match policy {
            SignaturePolicy::Enforce => true,
            #[cfg(test)]
            SignaturePolicy::Skip => false,
        };
        if enforce {
            let raw_signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(signature_b64)
                .map_err(|_| Refusal::Malformed("signature is not base64url"))?;
            let signature = Signature::from_slice(&raw_signature)
                .map_err(|_| Refusal::Malformed("signature is not Ed25519"))?;
            let signed = format!("{header_b64}.{payload_b64}");
            key.verify_strict(signed.as_bytes(), &signature)
                .map_err(|_| Refusal::BadSignature)?;
        }

        // No payload value below is acted on before its signature passes.
        if claims.iss != self.issuer {
            return Err(Refusal::UnknownIssuer);
        }
        if claims.aud != AUDIENCE {
            return Err(Refusal::WrongAudience);
        }
        if claims.owner_id != self.owner {
            return Err(Refusal::WrongOwner);
        }
        if claims.plane_id != self.plane {
            return Err(Refusal::WrongPlane);
        }
        validate_claim_identity(&claims)?;
        validate_claim_time(&claims, now)?;

        // Consumed last, so a refusal never burns a legitimate assertion.
        let replay_expiry = unix_time(claims.exp)?
            .checked_add(CLOCK_SKEW)
            .ok_or(Refusal::Malformed("expiry is out of range"))?;
        let fresh = self
            .replay
            .consume(claims.jti, replay_expiry, now)
            .await
            .map_err(|_| Refusal::ReplayStoreUnavailable)?;
        if !fresh {
            return Err(Refusal::Replayed);
        }

        Ok(VerifiedIdentity {
            user: claims.sub,
            owner: claims.owner_id.to_string(),
            scope: CompanyScope::Company {
                company: company_runtime_name(claims.company_id),
            },
            role: claims.membership_role,
            actor: Some(claims.membership_id),
            correlation: Some(claims.jti.to_string()),
        })
    }
}

fn validate_claim_identity(claims: &AssertionClaims) -> Result<(), Refusal> {
    if claims.jti.is_nil()
        || claims.owner_id.is_nil()
        || claims.plane_id.is_nil()
        || claims.company_id.is_nil()
        || claims.cell_id.is_nil()
    {
        return Err(Refusal::Malformed("identity UUID is nil"));
    }
    if !valid_identity_text(&claims.sub) || !valid_identity_text(&claims.membership_id) {
        return Err(Refusal::Malformed(
            "subject or membership identity is invalid",
        ));
    }
    if !matches!(
        claims.membership_role.as_str(),
        "owner" | "admin" | "member"
    ) {
        return Err(Refusal::UnknownRole);
    }
    Ok(())
}

fn valid_identity_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.chars().any(char::is_control)
        && value.trim() == value
}

fn validate_claim_time(claims: &AssertionClaims, now: SystemTime) -> Result<(), Refusal> {
    if claims.iat < 0 || claims.exp <= claims.iat {
        return Err(Refusal::Malformed("invalid assertion time range"));
    }
    let lifetime = (claims.exp - claims.iat) as u64;
    if lifetime > MAX_ASSERTION_LIFETIME.as_secs() {
        return Err(Refusal::LifetimeTooLong);
    }
    let now = unix_seconds(now);
    if claims.iat > now.saturating_add(CLOCK_SKEW.as_secs() as i64) {
        return Err(Refusal::NotYetValid);
    }
    if claims.exp.saturating_add(CLOCK_SKEW.as_secs() as i64) <= now {
        return Err(Refusal::Expired);
    }
    Ok(())
}

/// Matches the full-platform Runtime supervisor's documented Core-internal
/// PostgreSQL-safe slug while every external Cloud contract retains the UUID.
fn company_runtime_name(company_id: Uuid) -> String {
    format!("c{}", company_id.simple())
}

fn decode_segment<T: for<'de> Deserialize<'de>>(
    segment: &str,
    what: &'static str,
) -> Result<T, Refusal> {
    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(segment)
        .map_err(|_| match what {
            "header" => Refusal::Malformed("header is not base64url"),
            _ => Refusal::Malformed("payload is not base64url"),
        })?;
    serde_json::from_slice(&raw).map_err(|_| match what {
        "header" => Refusal::Malformed("header is not valid JSON"),
        _ => Refusal::Malformed("payload is not valid JSON"),
    })
}

fn unix_seconds(at: SystemTime) -> i64 {
    at.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .try_into()
        .unwrap_or(i64::MAX)
}

fn unix_time(seconds: i64) -> Result<SystemTime, Refusal> {
    let seconds: u64 = seconds
        .try_into()
        .map_err(|_| Refusal::Malformed("negative expiry"))?;
    UNIX_EPOCH
        .checked_add(Duration::from_secs(seconds))
        .ok_or(Refusal::Malformed("expiry is out of range"))
}

/// How this plane decides who may enter.
#[derive(Clone)]
pub(crate) enum EntryMode {
    Local,
    Network(Arc<NetworkEntry>),
}

impl EntryMode {
    /// Production startup injects its durable, atomic replay store. Local mode
    /// ignores it; network mode has no process-local fallback.
    pub(crate) fn from_env_with_replay(
        replay: Arc<dyn AssertionReplayStore>,
    ) -> anyhow::Result<Self> {
        let mode = std::env::var("RESTLESS_ENTRY_MODE").unwrap_or_else(|_| "local".to_string());
        match mode.as_str() {
            "local" => Ok(EntryMode::Local),
            "network" => {
                let allow_insecure_http = optional_bool("RESTLESS_ENTRY_ALLOW_INSECURE_HTTP")?;
                let (issuer, issuer_origin) =
                    configured_issuer(&required("RESTLESS_ENTRY_ISSUER")?, allow_insecure_http)?;
                let jwks_url = configured_jwks_url(
                    &required("RESTLESS_ENTRY_JWKS_URL")?,
                    &issuer_origin,
                    allow_insecure_http,
                )?;
                let owner = required_uuid("RESTLESS_ENTRY_OWNER_ID")?;
                let plane = required_uuid("RESTLESS_ENTRY_PLANE_ID")?;
                let host = configured_host(&required("RESTLESS_ENTRY_HOST")?)?;
                let scheme = if allow_insecure_http { "http" } else { "https" };
                let plane_origin = format!("{scheme}://{host}");
                let session_ttl = match std::env::var("RESTLESS_ENTRY_SESSION_TTL_SECONDS") {
                    Ok(value) => Duration::from_secs(value.parse().map_err(|_| {
                        anyhow::anyhow!("RESTLESS_ENTRY_SESSION_TTL_SECONDS must be seconds")
                    })?),
                    Err(_) => DEFAULT_SESSION_TTL,
                };
                if session_ttl.is_zero() || session_ttl > Duration::from_secs(7 * 24 * 60 * 60) {
                    anyhow::bail!(
                        "RESTLESS_ENTRY_SESSION_TTL_SECONDS must be between 1 and 604800"
                    );
                }
                Ok(EntryMode::Network(Arc::new(NetworkEntry {
                    issuer,
                    issuer_origin,
                    owner,
                    plane,
                    host,
                    plane_origin,
                    secure_cookie: !allow_insecure_http,
                    keys: Arc::new(HttpJwksSource::new(jwks_url, allow_insecure_http)?),
                    replay,
                    session_ttl,
                })))
            }
            other => {
                anyhow::bail!("RESTLESS_ENTRY_MODE must be `local` or `network`, not `{other}`")
            }
        }
    }

    pub(crate) fn network(&self) -> Option<&Arc<NetworkEntry>> {
        match self {
            EntryMode::Local => None,
            EntryMode::Network(entry) => Some(entry),
        }
    }
}

fn required(variable: &str) -> anyhow::Result<String> {
    let value = std::env::var(variable).unwrap_or_default();
    if value.trim().is_empty() {
        anyhow::bail!(
            "network entry mode requires {variable}; refusing to start rather than accepting requests on network position alone"
        );
    }
    Ok(value)
}

fn required_uuid(variable: &str) -> anyhow::Result<Uuid> {
    let value = required(variable)?;
    let id =
        Uuid::parse_str(value.trim()).map_err(|_| anyhow::anyhow!("{variable} must be a UUID"))?;
    if id.is_nil() {
        anyhow::bail!("{variable} must not be the nil UUID");
    }
    Ok(id)
}

fn optional_bool(variable: &str) -> anyhow::Result<bool> {
    match std::env::var(variable) {
        Err(_) => Ok(false),
        Ok(value) if value == "1" || value.eq_ignore_ascii_case("true") => Ok(true),
        Ok(value) if value == "0" || value.eq_ignore_ascii_case("false") => Ok(false),
        Ok(_) => anyhow::bail!("{variable} must be true, false, 1 or 0"),
    }
}

fn configured_issuer(raw: &str, allow_insecure_http: bool) -> anyhow::Result<(String, String)> {
    let url = Url::parse(raw.trim())
        .map_err(|_| anyhow::anyhow!("RESTLESS_ENTRY_ISSUER must be a URL"))?;
    validate_http_url(&url, allow_insecure_http, "RESTLESS_ENTRY_ISSUER")?;
    if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
        anyhow::bail!(
            "RESTLESS_ENTRY_ISSUER must be an origin URL without path, query or fragment"
        );
    }
    let origin = url.origin().ascii_serialization();
    Ok((origin.clone(), origin))
}

fn configured_jwks_url(
    raw: &str,
    issuer_origin: &str,
    allow_insecure_http: bool,
) -> anyhow::Result<Url> {
    let url = Url::parse(raw.trim())
        .map_err(|_| anyhow::anyhow!("RESTLESS_ENTRY_JWKS_URL must be a URL"))?;
    validate_http_url(&url, allow_insecure_http, "RESTLESS_ENTRY_JWKS_URL")?;
    if url.origin().ascii_serialization() != issuer_origin
        || url.path() != "/.well-known/jwks.json"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        anyhow::bail!(
            "RESTLESS_ENTRY_JWKS_URL must be the configured issuer's /.well-known/jwks.json"
        );
    }
    Ok(url)
}

fn validate_http_url(url: &Url, allow_insecure_http: bool, variable: &str) -> anyhow::Result<()> {
    if url.host_str().is_none()
        || url.username() != ""
        || url.password().is_some()
        || (url.scheme() != "https" && !(allow_insecure_http && url.scheme() == "http"))
    {
        anyhow::bail!("{variable} must use HTTPS and contain no credentials");
    }
    Ok(())
}

fn configured_host(raw: &str) -> anyhow::Result<String> {
    let value = raw.trim();
    if value.is_empty()
        || value.contains('/')
        || value.contains('@')
        || value.parse::<axum::http::uri::Authority>().is_err()
    {
        anyhow::bail!("RESTLESS_ENTRY_HOST must be a host or host:port authority");
    }
    Ok(value.to_ascii_lowercase())
}

/// Browser sessions established after an assertion is consumed.
#[derive(Default)]
pub(crate) struct SessionStore {
    sessions: Mutex<HashMap<String, (VerifiedIdentity, SystemTime)>>,
}

impl SessionStore {
    pub(crate) fn establish(&self, identity: VerifiedIdentity, ttl: Duration) -> String {
        let token = Uuid::new_v4().to_string();
        let mut sessions = self.sessions.lock().expect("session store poisoned");
        let now = SystemTime::now();
        sessions.retain(|_, (_, expires)| *expires > now);
        sessions.insert(token.clone(), (identity, now + ttl));
        token
    }

    pub(crate) fn resolve(&self, token: &str) -> Option<VerifiedIdentity> {
        let mut sessions = self.sessions.lock().expect("session store poisoned");
        let now = SystemTime::now();
        sessions.retain(|_, (_, expires)| *expires > now);
        sessions.get(token).map(|(identity, _)| identity.clone())
    }

    pub(crate) fn revoke(&self, token: &str) {
        let mut sessions = self.sessions.lock().expect("session store poisoned");
        sessions.remove(token);
    }
}

/// The single place company scope is derived from a request path.
pub(crate) fn company_in_path(path: &str) -> Option<&str> {
    let rest = path
        .strip_prefix("/api/companies/")
        .or_else(|| path.strip_prefix("/desktop/"))?;
    let company = rest.split('/').next()?;
    if company.is_empty() {
        None
    } else {
        Some(company)
    }
}

/// Test issuer for Core's own end-to-end verification run. Production Fleet
/// owns the real private key and never supplies it to Core.
pub(crate) fn mint_from_env() -> anyhow::Result<String> {
    use ed25519_dalek::{Signer as _, SigningKey};

    let issuer = configured_issuer(
        &required("RESTLESS_ENTRY_ISSUER")?,
        optional_bool("RESTLESS_ENTRY_ALLOW_INSECURE_HTTP")?,
    )?
    .0;
    let key_id = required("RESTLESS_ENTRY_TEST_SIGNING_KEY_ID")?;
    if !valid_key_id(&key_id) {
        anyhow::bail!("RESTLESS_ENTRY_TEST_SIGNING_KEY_ID is invalid");
    }
    let seed = base64::engine::general_purpose::STANDARD
        .decode(required("RESTLESS_ENTRY_TEST_SIGNING_KEY_B64")?)
        .map_err(|_| anyhow::anyhow!("RESTLESS_ENTRY_TEST_SIGNING_KEY_B64 must be base64"))?;
    let seed: [u8; 32] = seed.try_into().map_err(|_| {
        anyhow::anyhow!("RESTLESS_ENTRY_TEST_SIGNING_KEY_B64 must decode to 32 bytes")
    })?;
    let signing_key = SigningKey::from_bytes(&seed);
    let now = unix_seconds(SystemTime::now());
    let ttl = std::env::var("RESTLESS_ENTRY_TEST_TTL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(45);
    if ttl <= 0 || ttl > MAX_ASSERTION_LIFETIME.as_secs() as i64 {
        anyhow::bail!("RESTLESS_ENTRY_TEST_TTL_SECONDS must be between 1 and 60");
    }
    let claims = AssertionClaims {
        iss: issuer,
        aud: AUDIENCE.into(),
        sub: required("RESTLESS_ENTRY_TEST_USER")?,
        jti: Uuid::new_v4(),
        exp: now + ttl,
        iat: now,
        kid: key_id.clone(),
        owner_id: required_uuid("RESTLESS_ENTRY_OWNER_ID")?,
        plane_id: required_uuid("RESTLESS_ENTRY_PLANE_ID")?,
        company_id: required_uuid("RESTLESS_ENTRY_TEST_COMPANY_ID")?,
        cell_id: required_uuid("RESTLESS_ENTRY_TEST_CELL_ID")?,
        membership_id: required("RESTLESS_ENTRY_TEST_MEMBERSHIP_ID")?,
        membership_role: std::env::var("RESTLESS_ENTRY_TEST_ROLE")
            .unwrap_or_else(|_| "owner".into()),
        assertion_version: ASSERTION_CONTRACT_VERSION,
    };
    validate_claim_identity(&claims).map_err(|refusal| anyhow::anyhow!(refusal.message()))?;
    let header = AssertionHeader {
        alg: ALGORITHM.into(),
        typ: TOKEN_TYPE.into(),
        kid: key_id,
    };
    let header =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header)?);
    let payload =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims)?);
    let signed = format!("{header}.{payload}");
    let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(signing_key.sign(signed.as_bytes()).to_bytes());
    Ok(format!("{signed}.{signature}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer as _, SigningKey};

    const KEY_ID: &str = "fleet-2026-09";
    const OWNER: Uuid = Uuid::from_u128(0x11111111_1111_4111_8111_111111111111);
    const PLANE: Uuid = Uuid::from_u128(0x22222222_2222_4222_8222_222222222222);
    const COMPANY: Uuid = Uuid::from_u128(0x33333333_3333_4333_8333_333333333333);
    const CELL: Uuid = Uuid::from_u128(0x44444444_4444_4444_8444_444444444444);
    const JTI: Uuid = Uuid::from_u128(0x55555555_5555_4555_8555_555555555555);

    struct StaticKeys(HashMap<String, VerifyingKey>);

    #[async_trait]
    impl VerificationKeySource for StaticKeys {
        async fn key(&self, key_id: &str) -> Result<VerifyingKey, Refusal> {
            self.0
                .get(key_id)
                .cloned()
                .ok_or(Refusal::UnknownKeyVersion)
        }
    }

    fn signing_key() -> SigningKey {
        SigningKey::from_bytes(&[7; 32])
    }

    fn plane() -> NetworkEntry {
        let mut keys = HashMap::new();
        keys.insert(KEY_ID.into(), signing_key().verifying_key());
        NetworkEntry {
            issuer: "https://fleet.restless.test".into(),
            issuer_origin: "https://fleet.restless.test".into(),
            owner: OWNER,
            plane: PLANE,
            host: "plane.restless.test".into(),
            plane_origin: "https://plane.restless.test".into(),
            secure_cookie: true,
            keys: Arc::new(StaticKeys(keys)),
            replay: Arc::new(InMemoryAssertionReplayStore::default()),
            session_ttl: DEFAULT_SESSION_TTL,
        }
    }

    fn at(seconds: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(seconds)
    }

    fn claims() -> AssertionClaims {
        AssertionClaims {
            iss: "https://fleet.restless.test".into(),
            aud: AUDIENCE.into(),
            sub: "better-auth-user-1".into(),
            jti: JTI,
            exp: 1015,
            iat: 970,
            kid: KEY_ID.into(),
            owner_id: OWNER,
            plane_id: PLANE,
            company_id: COMPANY,
            cell_id: CELL,
            membership_id: "membership-1".into(),
            membership_role: "owner".into(),
            assertion_version: ASSERTION_CONTRACT_VERSION,
        }
    }

    fn mint(claims: &AssertionClaims, header_kid: &str, key: &SigningKey) -> String {
        let header = AssertionHeader {
            alg: ALGORITHM.into(),
            typ: TOKEN_TYPE.into(),
            kid: header_kid.into(),
        };
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&header).unwrap());
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(claims).unwrap());
        let signed = format!("{header}.{payload}");
        let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(key.sign(signed.as_bytes()).to_bytes());
        format!("{signed}.{signature}")
    }

    async fn refuse(entry: &NetworkEntry, claims: &AssertionClaims) -> Refusal {
        entry
            .verify_at(
                &mint(claims, &claims.kid, &signing_key()),
                at(1000),
                SignaturePolicy::Enforce,
            )
            .await
            .expect_err("assertion should have been refused")
    }

    #[tokio::test]
    async fn fleet_v1_assertion_is_accepted_once_and_company_scoped() {
        let identity = plane()
            .verify_at(
                &mint(&claims(), KEY_ID, &signing_key()),
                at(1000),
                SignaturePolicy::Enforce,
            )
            .await
            .expect("valid assertion");
        assert_eq!(identity.user, "better-auth-user-1");
        assert_eq!(identity.owner, OWNER.to_string());
        assert_eq!(
            identity.scope,
            CompanyScope::Company {
                company: format!("c{}", COMPANY.simple())
            }
        );
        assert_eq!(identity.role, "owner");
        assert_eq!(identity.actor.as_deref(), Some("membership-1"));
        assert_eq!(identity.correlation, Some(JTI.to_string()));
    }

    #[tokio::test]
    async fn role_is_an_exact_reviewed_literal_and_never_expands_scope() {
        for role in ["owner", "admin", "member"] {
            let mut candidate = claims();
            candidate.membership_role = role.into();
            candidate.jti = Uuid::new_v4();
            let identity = plane()
                .verify_at(
                    &mint(&candidate, KEY_ID, &signing_key()),
                    at(1000),
                    SignaturePolicy::Enforce,
                )
                .await
                .unwrap_or_else(|error| panic!("{role} refused: {error:?}"));
            assert!(matches!(identity.scope, CompanyScope::Company { .. }));
        }
        for role in ["editor", "viewer", "OWNER", ""] {
            let mut candidate = claims();
            candidate.membership_role = role.into();
            assert_eq!(refuse(&plane(), &candidate).await, Refusal::UnknownRole);
        }
    }

    #[tokio::test]
    async fn issuer_audience_owner_and_plane_are_all_bound() {
        let mut candidate = claims();
        candidate.iss = "https://attacker.test".into();
        assert_eq!(refuse(&plane(), &candidate).await, Refusal::UnknownIssuer);
        candidate = claims();
        candidate.aud = "some-other-service".into();
        assert_eq!(refuse(&plane(), &candidate).await, Refusal::WrongAudience);
        candidate = claims();
        candidate.owner_id = Uuid::new_v4();
        assert_eq!(refuse(&plane(), &candidate).await, Refusal::WrongOwner);
        candidate = claims();
        candidate.plane_id = Uuid::new_v4();
        assert_eq!(refuse(&plane(), &candidate).await, Refusal::WrongPlane);
    }

    #[tokio::test]
    async fn header_and_payload_key_ids_must_match() {
        let token = mint(&claims(), "other-key", &signing_key());
        assert_eq!(
            plane()
                .verify_at(&token, at(1000), SignaturePolicy::Enforce)
                .await
                .expect_err("mismatched key ids"),
            Refusal::Malformed("header and payload key ids differ")
        );
        let mut candidate = claims();
        candidate.kid = "other-key".into();
        assert_eq!(
            refuse(&plane(), &candidate).await,
            Refusal::UnknownKeyVersion
        );
    }

    #[tokio::test]
    async fn assertion_lifetime_is_short_and_clock_skew_is_bounded() {
        let mut candidate = claims();
        candidate.exp = 1031;
        assert_eq!(refuse(&plane(), &candidate).await, Refusal::LifetimeTooLong);
        candidate = claims();
        candidate.iat = 1006;
        candidate.exp = 1051;
        assert_eq!(refuse(&plane(), &candidate).await, Refusal::NotYetValid);
        candidate = claims();
        candidate.iat = 950;
        candidate.exp = 994;
        assert_eq!(refuse(&plane(), &candidate).await, Refusal::Expired);
    }

    #[tokio::test]
    async fn nil_or_blank_authority_identities_are_refused() {
        let mut candidate = claims();
        candidate.company_id = Uuid::nil();
        assert!(matches!(
            refuse(&plane(), &candidate).await,
            Refusal::Malformed(_)
        ));
        candidate = claims();
        candidate.cell_id = Uuid::nil();
        assert!(matches!(
            refuse(&plane(), &candidate).await,
            Refusal::Malformed(_)
        ));
        candidate = claims();
        candidate.membership_id = " ".into();
        assert!(matches!(
            refuse(&plane(), &candidate).await,
            Refusal::Malformed(_)
        ));
    }

    #[tokio::test]
    async fn a_tampered_signature_is_refused_by_the_signature_check() {
        let forged_key = SigningKey::from_bytes(&[9; 32]);
        let forged = mint(&claims(), KEY_ID, &forged_key);
        assert_eq!(
            plane()
                .verify_at(&forged, at(1000), SignaturePolicy::Enforce)
                .await
                .expect_err("forged signature"),
            Refusal::BadSignature
        );
        let identity = plane()
            .verify_at(&forged, at(1000), SignaturePolicy::Skip)
            .await
            .expect("skipping only the signature makes the same token pass");
        assert_eq!(identity.user, "better-auth-user-1");
    }

    #[tokio::test]
    async fn replay_consumption_is_atomic_under_concurrency() {
        let entry = Arc::new(plane());
        let token = mint(&claims(), KEY_ID, &signing_key());
        let first = {
            let entry = entry.clone();
            let token = token.clone();
            tokio::spawn(async move {
                entry
                    .verify_at(&token, at(1000), SignaturePolicy::Enforce)
                    .await
            })
        };
        let second = {
            let entry = entry.clone();
            tokio::spawn(async move {
                entry
                    .verify_at(&token, at(1000), SignaturePolicy::Enforce)
                    .await
            })
        };
        let results = [first.await.unwrap(), second.await.unwrap()];
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, Err(Refusal::Replayed)))
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn malformed_and_wrong_algorithm_tokens_are_refused() {
        for token in ["", "one.two", "a.b.c.d", "not-base64!.x.y"] {
            assert!(matches!(
                plane()
                    .verify_at(token, at(1000), SignaturePolicy::Enforce)
                    .await
                    .expect_err("malformed"),
                Refusal::Malformed(_)
            ));
        }
        let header = serde_json::json!({ "alg": "HS256", "typ": "JWT", "kid": KEY_ID });
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header.to_string());
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&claims()).unwrap());
        let token = format!("{header}.{payload}.signature");
        assert_eq!(
            plane()
                .verify_at(&token, at(1000), SignaturePolicy::Enforce)
                .await
                .expect_err("HS256 algorithm confusion"),
            Refusal::Malformed("unexpected signature algorithm")
        );
    }

    #[test]
    fn fleet_jwks_shape_is_accepted_and_wrong_key_metadata_is_refused() {
        let x = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(signing_key().verifying_key().as_bytes());
        let valid = serde_json::json!({
            "keys": [{ "kty": "OKP", "crv": "Ed25519", "alg": "EdDSA", "use": "sig", "kid": KEY_ID, "x": x }]
        });
        assert_eq!(parse_jwks(valid.to_string().as_bytes()).unwrap().len(), 1);
        for (field, value) in [
            ("kty", "RSA"),
            ("crv", "X25519"),
            ("alg", "HS256"),
            ("use", "enc"),
        ] {
            let mut invalid = valid.clone();
            invalid["keys"][0][field] = value.into();
            assert_eq!(
                parse_jwks(invalid.to_string().as_bytes()).expect_err("wrong key metadata"),
                Refusal::JwksUnavailable
            );
        }
    }

    #[test]
    fn entry_urls_are_https_same_issuer_and_exact() {
        let (issuer, origin) = configured_issuer("https://fleet.example.test", false).unwrap();
        assert_eq!(issuer, "https://fleet.example.test");
        assert!(configured_jwks_url(
            "https://fleet.example.test/.well-known/jwks.json",
            &origin,
            false
        )
        .is_ok());
        assert!(configured_jwks_url(
            "https://attacker.test/.well-known/jwks.json",
            &origin,
            false
        )
        .is_err());
        assert!(configured_jwks_url(
            "http://fleet.example.test/.well-known/jwks.json",
            &origin,
            false
        )
        .is_err());
        assert!(configured_issuer("https://fleet.example.test/path", false).is_err());
    }

    #[test]
    fn company_scope_and_path_derivation_are_exact() {
        let scope = CompanyScope::Company {
            company: "aris".into(),
        };
        assert!(scope.permits("aris"));
        assert!(!scope.permits("other"));
        assert_eq!(company_in_path("/api/companies/aris/cockpit"), Some("aris"));
        assert_eq!(company_in_path("/desktop/aris/observe"), Some("aris"));
        assert_eq!(company_in_path("/api/companies/"), None);
        assert_eq!(company_in_path("/"), None);
    }

    #[test]
    fn every_refusal_reason_has_a_distinct_machine_code() {
        let reasons = [
            Refusal::Malformed("x").code(),
            Refusal::UnsupportedVersion {
                got: 2,
                supported: 1,
            }
            .code(),
            Refusal::UnknownIssuer.code(),
            Refusal::WrongAudience.code(),
            Refusal::UnknownKeyVersion.code(),
            Refusal::JwksUnavailable.code(),
            Refusal::BadSignature.code(),
            Refusal::NotYetValid.code(),
            Refusal::Expired.code(),
            Refusal::LifetimeTooLong.code(),
            Refusal::WrongOwner.code(),
            Refusal::WrongPlane.code(),
            Refusal::UnknownRole.code(),
            Refusal::Replayed.code(),
            Refusal::ReplayStoreUnavailable.code(),
        ];
        let unique: std::collections::BTreeSet<_> = reasons.iter().collect();
        assert_eq!(unique.len(), reasons.len());
    }

    #[test]
    fn a_session_is_reusable_after_the_single_use_door() {
        let store = SessionStore::default();
        let identity = VerifiedIdentity {
            user: "user-1".into(),
            owner: OWNER.to_string(),
            scope: CompanyScope::Company {
                company: "aris".into(),
            },
            role: "member".into(),
            actor: Some("membership-1".into()),
            correlation: Some(JTI.to_string()),
        };
        let token = store.establish(identity, Duration::from_secs(60));
        assert!(store.resolve(&token).is_some());
        assert!(store.resolve(&token).is_some());
        store.revoke(&token);
        assert!(store.resolve(&token).is_none());
    }
}
