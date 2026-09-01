//! Owner entry modes for the account plane.
//!
//! Local mode keeps the historical loopback-only owner boundary. Network mode
//! consumes Fleet's compact Ed25519 JWS, derives company scope from its signed
//! membership claims, and records the one-use `jti` in PostgreSQL before a
//! browser session is created. Entry authentication never grants an Authority
//! capability.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use anyhow::Context as _;
use base64::Engine as _;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer as _, SigningKey, Verifier as _, VerifyingKey};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

pub(crate) const ASSERTION_CONTRACT_VERSION: u32 = 1;
pub(crate) const ASSERTION_AUDIENCE: &str = "restless-core-account-plane";

const TOKEN_TYPE: &str = "JWT";
const ALGORITHM: &str = "EdDSA";
const MAX_ASSERTION_TTL_SECONDS: i64 = 120;
const CLOCK_SKEW_SECONDS: i64 = 5;
const DEFAULT_SESSION_TTL: Duration = Duration::from_secs(12 * 60 * 60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Refusal {
    Malformed(&'static str),
    UnsupportedVersion { got: u32, supported: u32 },
    UnknownIssuer,
    WrongAudience,
    UnknownKeyVersion,
    BadSignature,
    NotYetValid,
    Expired,
    WrongOwner,
    WrongPlane,
    InvalidMembership,
    Replayed,
}

impl Refusal {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Refusal::Malformed(_) => "assertion_malformed",
            Refusal::UnsupportedVersion { .. } => "assertion_unsupported_version",
            Refusal::UnknownIssuer => "assertion_unknown_issuer",
            Refusal::WrongAudience => "assertion_wrong_audience",
            Refusal::UnknownKeyVersion => "assertion_unknown_key_version",
            Refusal::BadSignature => "assertion_bad_signature",
            Refusal::NotYetValid => "assertion_not_yet_valid",
            Refusal::Expired => "assertion_expired",
            Refusal::WrongOwner => "assertion_wrong_owner",
            Refusal::WrongPlane => "assertion_wrong_plane",
            Refusal::InvalidMembership => "assertion_invalid_membership",
            Refusal::Replayed => "assertion_replayed",
        }
    }

    pub(crate) fn message(&self) -> String {
        match self {
            Refusal::Malformed(what) => format!("entry assertion is malformed: {what}"),
            Refusal::UnsupportedVersion { got, supported } => format!(
                "entry assertion contract version {got} is not supported; this plane supports {supported}"
            ),
            Refusal::UnknownIssuer => "entry assertion issuer is not trusted by this plane".into(),
            Refusal::WrongAudience => "entry assertion was not minted for a Core account plane".into(),
            Refusal::UnknownKeyVersion => "entry assertion names an unknown Fleet signing key".into(),
            Refusal::BadSignature => "entry assertion signature is invalid".into(),
            Refusal::NotYetValid => "entry assertion is not valid yet".into(),
            Refusal::Expired => "entry assertion has expired or is not short-lived".into(),
            Refusal::WrongOwner => "entry assertion is routed to a different owner".into(),
            Refusal::WrongPlane => "entry assertion is routed to a different plane".into(),
            Refusal::InvalidMembership => "entry assertion has invalid membership claims".into(),
            Refusal::Replayed => "entry assertion has already been used".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum CompanyScope {
    Owner,
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

/// The exact Fleet V1 claims shape. UUID types make malformed routing and
/// scope identifiers fail before any of them reach application code.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AssertionClaims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub jti: Uuid,
    pub exp: i64,
    pub iat: i64,
    pub kid: String,
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub membership_id: String,
    pub membership_role: String,
    pub assertion_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AssertionHeader {
    alg: String,
    typ: String,
    kid: String,
}

#[derive(Debug, Clone)]
pub(crate) struct VerifiedIdentity {
    pub user: String,
    pub owner: String,
    pub scope: CompanyScope,
    pub role: String,
    pub actor: Option<String>,
    pub correlation: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct VerifiedAssertion {
    pub identity: VerifiedIdentity,
    pub jti: Uuid,
    pub issuer: String,
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub membership_id: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SignaturePolicy {
    Enforce,
    #[cfg(test)]
    Skip,
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

pub(crate) struct NetworkEntry {
    issuer: String,
    owner_id: Uuid,
    plane_id: Uuid,
    host: String,
    keys: HashMap<String, VerifyingKey>,
    session_ttl: Duration,
}

impl NetworkEntry {
    pub(crate) fn host(&self) -> &str {
        &self.host
    }

    pub(crate) fn session_ttl(&self) -> Duration {
        self.session_ttl
    }

    pub(crate) fn owner_id(&self) -> Uuid {
        self.owner_id
    }

    pub(crate) fn plane_id(&self) -> Uuid {
        self.plane_id
    }

    pub(crate) fn verify(&self, token: &str) -> Result<VerifiedAssertion, Refusal> {
        self.verify_at(token, Utc::now().timestamp(), SignaturePolicy::Enforce)
    }

    fn verify_at(
        &self,
        token: &str,
        now: i64,
        policy: SignaturePolicy,
    ) -> Result<VerifiedAssertion, Refusal> {
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

        let claims: AssertionClaims = decode_segment(payload_b64, "payload")?;
        if claims.assertion_version != ASSERTION_CONTRACT_VERSION {
            return Err(Refusal::UnsupportedVersion {
                got: claims.assertion_version,
                supported: ASSERTION_CONTRACT_VERSION,
            });
        }
        if claims.kid != header.kid {
            return Err(Refusal::Malformed("header and claims key IDs differ"));
        }
        let key = self
            .keys
            .get(&header.kid)
            .ok_or(Refusal::UnknownKeyVersion)?;

        let enforce = match policy {
            SignaturePolicy::Enforce => true,
            #[cfg(test)]
            SignaturePolicy::Skip => false,
        };
        if enforce {
            let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(signature_b64)
                .map_err(|_| Refusal::Malformed("signature is not base64url"))?;
            let signature = Signature::from_slice(&raw)
                .map_err(|_| Refusal::Malformed("signature is not Ed25519"))?;
            let signed = format!("{header_b64}.{payload_b64}");
            key.verify(signed.as_bytes(), &signature)
                .map_err(|_| Refusal::BadSignature)?;
        }

        if claims.iss != self.issuer {
            return Err(Refusal::UnknownIssuer);
        }
        if claims.aud != ASSERTION_AUDIENCE {
            return Err(Refusal::WrongAudience);
        }
        if claims.owner_id != self.owner_id {
            return Err(Refusal::WrongOwner);
        }
        if claims.plane_id != self.plane_id {
            return Err(Refusal::WrongPlane);
        }
        if claims.iat > now + CLOCK_SKEW_SECONDS {
            return Err(Refusal::NotYetValid);
        }
        if claims.exp <= now
            || claims.exp <= claims.iat
            || claims.exp - claims.iat > MAX_ASSERTION_TTL_SECONDS
        {
            return Err(Refusal::Expired);
        }
        if claims.sub.trim().is_empty()
            || claims.membership_id.trim().is_empty()
            || !matches!(
                claims.membership_role.as_str(),
                "owner" | "admin" | "member"
            )
        {
            return Err(Refusal::InvalidMembership);
        }
        let expires_at = DateTime::from_timestamp(claims.exp, 0).ok_or(Refusal::Malformed(
            "expiry is outside the supported time range",
        ))?;

        // A Cloud membership always grants one company, including an owner's
        // membership. Plane-wide owner scope exists only for trusted local mode.
        let identity = VerifiedIdentity {
            user: claims.sub,
            owner: claims.owner_id.to_string(),
            scope: CompanyScope::Company {
                company: claims.company_id.to_string(),
            },
            role: claims.membership_role.clone(),
            actor: Some(claims.membership_id.clone()),
            correlation: Some(claims.jti.to_string()),
        };
        Ok(VerifiedAssertion {
            identity,
            jti: claims.jti,
            issuer: claims.iss,
            owner_id: claims.owner_id,
            plane_id: claims.plane_id,
            company_id: claims.company_id,
            cell_id: claims.cell_id,
            membership_id: claims.membership_id,
            expires_at,
        })
    }
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
        "header" => Refusal::Malformed("header is not valid contract JSON"),
        _ => Refusal::Malformed("payload is not valid contract JSON"),
    })
}

fn parse_jwks(raw: &str) -> anyhow::Result<HashMap<String, VerifyingKey>> {
    let set: JwkSet = serde_json::from_str(raw).context("parse Fleet JWKS JSON")?;
    let mut keys = HashMap::new();
    for key in set.keys {
        if key.kty != "OKP" || key.crv != "Ed25519" || key.alg != ALGORITHM || key.use_ != "sig" {
            anyhow::bail!("Fleet JWKS contains a key that is not an Ed25519 signing key");
        }
        if key.kid.is_empty()
            || key.kid.len() > 64
            || !key
                .kid
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
            || keys.contains_key(&key.kid)
        {
            anyhow::bail!("Fleet JWKS contains an invalid or duplicate kid");
        }
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(key.x)
            .context("decode Fleet Ed25519 public key")?;
        let bytes: [u8; 32] = raw
            .try_into()
            .map_err(|_| anyhow::anyhow!("Fleet Ed25519 public key must be exactly 32 bytes"))?;
        keys.insert(
            key.kid,
            VerifyingKey::from_bytes(&bytes).context("parse Fleet Ed25519 public key")?,
        );
    }
    if keys.is_empty() {
        anyhow::bail!("Fleet JWKS contains no usable signing keys");
    }
    Ok(keys)
}

/// Atomically and durably consume the assertion before a browser session is
/// established. The primary key is the concurrency gate: two racing requests
/// can never both report an inserted row.
pub(crate) async fn consume_once(
    pool: &PgPool,
    assertion: &VerifiedAssertion,
) -> anyhow::Result<bool> {
    let inserted = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO restless_authority.entry_assertions \
         (jti, issuer, owner_id, plane_id, company_id, cell_id, membership_id, expires_at) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8) \
         ON CONFLICT (jti) DO NOTHING RETURNING jti",
    )
    .bind(assertion.jti)
    .bind(&assertion.issuer)
    .bind(assertion.owner_id)
    .bind(assertion.plane_id)
    .bind(assertion.company_id)
    .bind(assertion.cell_id)
    .bind(&assertion.membership_id)
    .bind(assertion.expires_at)
    .fetch_optional(pool)
    .await
    .context("consume Fleet entry assertion")?;
    Ok(inserted.is_some())
}

#[derive(Clone)]
pub(crate) enum EntryMode {
    Local,
    Network(std::sync::Arc<NetworkEntry>),
}

impl EntryMode {
    pub(crate) async fn from_env() -> anyhow::Result<Self> {
        let mode = std::env::var("RESTLESS_ENTRY_MODE").unwrap_or_else(|_| "local".to_string());
        match mode.as_str() {
            "local" => Ok(EntryMode::Local),
            "network" => {
                let issuer = required("RESTLESS_ENTRY_ISSUER")?
                    .trim_end_matches('/')
                    .to_string();
                let owner_id = required("RESTLESS_ENTRY_OWNER_ID")?
                    .parse::<Uuid>()
                    .context("RESTLESS_ENTRY_OWNER_ID must be a UUID")?;
                let plane_id = required("RESTLESS_ENTRY_PLANE_ID")?
                    .parse::<Uuid>()
                    .context("RESTLESS_ENTRY_PLANE_ID must be a UUID")?;
                let host = required("RESTLESS_ENTRY_HOST")?;
                let allow_insecure = std::env::var("RESTLESS_ENTRY_ALLOW_INSECURE_HTTP")
                    .is_ok_and(|value| value == "1");
                let issuer_url =
                    validate_https_url(&issuer, allow_insecure, "RESTLESS_ENTRY_ISSUER")?;
                let jwks_url = std::env::var("RESTLESS_ENTRY_JWKS_URL")
                    .unwrap_or_else(|_| format!("{issuer}/.well-known/jwks.json"));
                let parsed_jwks =
                    validate_https_url(&jwks_url, allow_insecure, "RESTLESS_ENTRY_JWKS_URL")?;
                if issuer_url.scheme() != parsed_jwks.scheme()
                    || issuer_url.host_str() != parsed_jwks.host_str()
                    || issuer_url.port_or_known_default() != parsed_jwks.port_or_known_default()
                {
                    anyhow::bail!("RESTLESS_ENTRY_JWKS_URL must use the Fleet issuer origin");
                }
                let jwks = match std::env::var("RESTLESS_ENTRY_JWKS_JSON") {
                    Ok(raw) if !raw.trim().is_empty() => raw,
                    _ => reqwest::Client::new()
                        .get(parsed_jwks)
                        .send()
                        .await
                        .context("fetch Fleet JWKS")?
                        .error_for_status()
                        .context("Fleet JWKS returned an error")?
                        .text()
                        .await
                        .context("read Fleet JWKS")?,
                };
                let session_ttl = match std::env::var("RESTLESS_ENTRY_SESSION_TTL_SECONDS") {
                    Ok(value) => Duration::from_secs(value.parse().map_err(|_| {
                        anyhow::anyhow!("RESTLESS_ENTRY_SESSION_TTL_SECONDS must be seconds")
                    })?),
                    Err(_) => DEFAULT_SESSION_TTL,
                };
                Ok(EntryMode::Network(std::sync::Arc::new(NetworkEntry {
                    issuer,
                    owner_id,
                    plane_id,
                    host,
                    keys: parse_jwks(&jwks)?,
                    session_ttl,
                })))
            }
            other => {
                anyhow::bail!("RESTLESS_ENTRY_MODE must be `local` or `network`, not `{other}`")
            }
        }
    }

    pub(crate) fn network(&self) -> Option<&std::sync::Arc<NetworkEntry>> {
        match self {
            EntryMode::Local => None,
            EntryMode::Network(entry) => Some(entry),
        }
    }
}

fn validate_https_url(
    value: &str,
    allow_insecure: bool,
    variable: &str,
) -> anyhow::Result<url::Url> {
    let parsed = url::Url::parse(value).with_context(|| format!("parse {variable}"))?;
    let insecure_loopback = allow_insecure
        && parsed.scheme() == "http"
        && parsed
            .host_str()
            .is_some_and(|host| host == "localhost" || host == "127.0.0.1");
    if parsed.host_str().is_none() || (parsed.scheme() != "https" && !insecure_loopback) {
        anyhow::bail!("{variable} must use HTTPS (HTTP is test-only on loopback)");
    }
    Ok(parsed)
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
        self.sessions
            .lock()
            .expect("session store poisoned")
            .remove(token);
    }
}

pub(crate) fn company_in_path(path: &str) -> Option<&str> {
    let rest = path
        .strip_prefix("/api/companies/")
        .or_else(|| path.strip_prefix("/desktop/"))?;
    let company = rest.split('/').next()?;
    (!company.is_empty()).then_some(company)
}

/// Test issuer for the standalone Core release run. Hosted compatibility uses
/// Fleet's real `/v1/entries` endpoint, never this command.
pub(crate) fn mint_from_env() -> anyhow::Result<String> {
    let issuer = required("RESTLESS_ENTRY_ISSUER")?
        .trim_end_matches('/')
        .to_string();
    let kid = required("RESTLESS_ENTRY_TEST_SIGNING_KEY_ID")?;
    let raw = base64::engine::general_purpose::STANDARD
        .decode(required("RESTLESS_ENTRY_TEST_SIGNING_KEY_B64")?)
        .context("RESTLESS_ENTRY_TEST_SIGNING_KEY_B64 must be base64")?;
    let key: [u8; 32] = raw.try_into().map_err(|_| {
        anyhow::anyhow!("RESTLESS_ENTRY_TEST_SIGNING_KEY_B64 must decode to 32 bytes")
    })?;
    let now = Utc::now().timestamp();
    let claims = AssertionClaims {
        iss: issuer,
        aud: ASSERTION_AUDIENCE.into(),
        sub: std::env::var("RESTLESS_ENTRY_TEST_USER").unwrap_or_else(|_| "test-user".into()),
        jti: Uuid::new_v4(),
        exp: now + 45,
        iat: now,
        kid: kid.clone(),
        owner_id: required("RESTLESS_ENTRY_OWNER_ID")?.parse()?,
        plane_id: required("RESTLESS_ENTRY_PLANE_ID")?.parse()?,
        company_id: required("RESTLESS_ENTRY_TEST_COMPANY_ID")?.parse()?,
        cell_id: required("RESTLESS_ENTRY_TEST_CELL_ID")?.parse()?,
        membership_id: std::env::var("RESTLESS_ENTRY_TEST_MEMBERSHIP_ID")
            .unwrap_or_else(|_| "membership-test".into()),
        membership_role: std::env::var("RESTLESS_ENTRY_TEST_ROLE")
            .unwrap_or_else(|_| "owner".into()),
        assertion_version: ASSERTION_CONTRACT_VERSION,
    };
    Ok(mint(&claims, &kid, &SigningKey::from_bytes(&key)))
}

pub(crate) fn test_jwks_from_env() -> anyhow::Result<String> {
    let kid = required("RESTLESS_ENTRY_TEST_SIGNING_KEY_ID")?;
    let raw = base64::engine::general_purpose::STANDARD
        .decode(required("RESTLESS_ENTRY_TEST_SIGNING_KEY_B64")?)
        .context("RESTLESS_ENTRY_TEST_SIGNING_KEY_B64 must be base64")?;
    let key: [u8; 32] = raw.try_into().map_err(|_| {
        anyhow::anyhow!("RESTLESS_ENTRY_TEST_SIGNING_KEY_B64 must decode to 32 bytes")
    })?;
    let x = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(SigningKey::from_bytes(&key).verifying_key().as_bytes());
    Ok(serde_json::json!({
        "keys": [{
            "kty": "OKP",
            "crv": "Ed25519",
            "alg": ALGORITHM,
            "use": "sig",
            "kid": kid,
            "x": x
        }]
    })
    .to_string())
}

fn mint(claims: &AssertionClaims, kid: &str, key: &SigningKey) -> String {
    let header = AssertionHeader {
        alg: ALGORITHM.into(),
        typ: TOKEN_TYPE.into(),
        kid: kid.into(),
    };
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&header).expect("header encodes"));
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(claims).expect("claims encode"));
    let signed = format!("{header}.{payload}");
    let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(key.sign(signed.as_bytes()).to_bytes());
    format!("{signed}.{signature}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const KID: &str = "fleet-test-2026";

    fn signing_key() -> SigningKey {
        SigningKey::from_bytes(&[7; 32])
    }

    fn ids() -> (Uuid, Uuid, Uuid, Uuid) {
        (
            Uuid::from_u128(1),
            Uuid::from_u128(2),
            Uuid::from_u128(3),
            Uuid::from_u128(4),
        )
    }

    fn plane() -> NetworkEntry {
        let (owner_id, plane_id, _, _) = ids();
        NetworkEntry {
            issuer: "https://fleet.example.test".into(),
            owner_id,
            plane_id,
            host: "owner.example.test".into(),
            keys: HashMap::from([(KID.into(), signing_key().verifying_key())]),
            session_ttl: DEFAULT_SESSION_TTL,
        }
    }

    fn claims() -> AssertionClaims {
        let (owner_id, plane_id, company_id, cell_id) = ids();
        AssertionClaims {
            iss: "https://fleet.example.test".into(),
            aud: ASSERTION_AUDIENCE.into(),
            sub: "better-auth-user".into(),
            jti: Uuid::from_u128(5),
            exp: 1045,
            iat: 1000,
            kid: KID.into(),
            owner_id,
            plane_id,
            company_id,
            cell_id,
            membership_id: "membership-1".into(),
            membership_role: "owner".into(),
            assertion_version: ASSERTION_CONTRACT_VERSION,
        }
    }

    fn token(claims: &AssertionClaims) -> String {
        mint(claims, KID, &signing_key())
    }

    fn refuse(entry: &NetworkEntry, claims: &AssertionClaims) -> Refusal {
        entry
            .verify_at(&token(claims), 1010, SignaturePolicy::Enforce)
            .expect_err("assertion should be refused")
    }

    #[test]
    fn fleet_shaped_ed25519_assertion_is_accepted_and_company_scoped() {
        let assertion = plane()
            .verify_at(&token(&claims()), 1010, SignaturePolicy::Enforce)
            .expect("valid assertion");
        assert_eq!(assertion.identity.user, "better-auth-user");
        assert_eq!(assertion.identity.owner, Uuid::from_u128(1).to_string());
        assert_eq!(
            assertion.identity.scope,
            CompanyScope::Company {
                company: Uuid::from_u128(3).to_string()
            }
        );
        assert_eq!(assertion.identity.actor.as_deref(), Some("membership-1"));
    }

    #[test]
    fn wrong_algorithm_and_hmac_shape_are_refused() {
        let original = token(&claims());
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(br#"{"alg":"HS256","typ":"JWT","kid":"fleet-test-2026"}"#);
        let token = format!("{header}.{}", original.split_once('.').unwrap().1);
        assert!(matches!(
            plane().verify_at(&token, 1010, SignaturePolicy::Enforce),
            Err(Refusal::Malformed("unexpected signature algorithm"))
        ));
    }

    #[test]
    fn signature_check_is_what_rejects_a_forgery() {
        let forged = mint(&claims(), KID, &SigningKey::from_bytes(&[9; 32]));
        assert_eq!(
            plane()
                .verify_at(&forged, 1010, SignaturePolicy::Enforce)
                .expect_err("forgery"),
            Refusal::BadSignature
        );
        assert!(plane()
            .verify_at(&forged, 1010, SignaturePolicy::Skip)
            .is_ok());
    }

    #[test]
    fn issuer_audience_owner_plane_key_and_version_are_bound() {
        let mut changed = claims();
        changed.iss = "https://other.example.test".into();
        assert_eq!(refuse(&plane(), &changed), Refusal::UnknownIssuer);
        changed = claims();
        changed.aud = "another-audience".into();
        assert_eq!(refuse(&plane(), &changed), Refusal::WrongAudience);
        changed = claims();
        changed.owner_id = Uuid::from_u128(20);
        assert_eq!(refuse(&plane(), &changed), Refusal::WrongOwner);
        changed = claims();
        changed.plane_id = Uuid::from_u128(20);
        assert_eq!(refuse(&plane(), &changed), Refusal::WrongPlane);
        changed = claims();
        changed.assertion_version = 2;
        assert!(matches!(
            refuse(&plane(), &changed),
            Refusal::UnsupportedVersion { .. }
        ));
        changed = claims();
        changed.kid = "unknown".into();
        let unknown = mint(&changed, "unknown", &signing_key());
        assert_eq!(
            plane()
                .verify_at(&unknown, 1010, SignaturePolicy::Enforce)
                .expect_err("unknown kid"),
            Refusal::UnknownKeyVersion
        );
    }

    #[test]
    fn time_and_membership_are_strict() {
        assert_eq!(
            plane()
                .verify_at(&token(&claims()), 1045, SignaturePolicy::Enforce)
                .expect_err("expired"),
            Refusal::Expired
        );
        let mut changed = claims();
        changed.iat = 1020;
        changed.exp = 1060;
        assert_eq!(refuse(&plane(), &changed), Refusal::NotYetValid);
        changed = claims();
        changed.exp = changed.iat + MAX_ASSERTION_TTL_SECONDS + 1;
        assert_eq!(refuse(&plane(), &changed), Refusal::Expired);
        changed = claims();
        changed.membership_role = "superadmin".into();
        assert_eq!(refuse(&plane(), &changed), Refusal::InvalidMembership);
    }

    #[test]
    fn jwks_accepts_only_fleet_ed25519_signing_keys() {
        let x = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(signing_key().verifying_key().as_bytes());
        let raw = format!(
            r#"{{"keys":[{{"kty":"OKP","crv":"Ed25519","alg":"EdDSA","use":"sig","kid":"{KID}","x":"{x}"}}]}}"#
        );
        assert_eq!(parse_jwks(&raw).expect("JWKS").len(), 1);
        assert!(parse_jwks(&raw.replace("EdDSA", "HS256")).is_err());
        assert!(parse_jwks(r#"{"keys":[]}"#).is_err());
    }

    #[test]
    fn company_scope_and_sessions_remain_host_local() {
        let company = Uuid::from_u128(3).to_string();
        let one = CompanyScope::Company {
            company: company.clone(),
        };
        assert!(one.permits(&company));
        assert!(!one.permits("other"));
        let store = SessionStore::default();
        let token = store.establish(
            plane()
                .verify_at(&token(&claims()), 1010, SignaturePolicy::Enforce)
                .unwrap()
                .identity,
            Duration::from_secs(60),
        );
        assert!(store.resolve(&token).is_some());
        store.revoke(&token);
        assert!(store.resolve(&token).is_none());
    }

    #[test]
    fn company_is_derived_from_the_path_in_one_place() {
        assert_eq!(company_in_path("/api/companies/a/cockpit"), Some("a"));
        assert_eq!(company_in_path("/desktop/a/observe"), Some("a"));
        assert_eq!(company_in_path("/api/companies/"), None);
        assert_eq!(company_in_path("/"), None);
    }

    #[test]
    fn every_refusal_reason_is_distinct() {
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
            Refusal::BadSignature.code(),
            Refusal::NotYetValid.code(),
            Refusal::Expired.code(),
            Refusal::WrongOwner.code(),
            Refusal::WrongPlane.code(),
            Refusal::InvalidMembership.code(),
            Refusal::Replayed.code(),
        ];
        let unique: std::collections::BTreeSet<_> = reasons.iter().collect();
        assert_eq!(unique.len(), reasons.len());
    }

    #[tokio::test]
    #[ignore = "requires RESTLESS_TEST_DATABASE_URL pointing at isolated PostgreSQL"]
    async fn durable_replay_store_has_one_winner_and_survives_reconnect() {
        let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL")
            .expect("RESTLESS_TEST_DATABASE_URL is required");
        let store = crate::authority::AuthorityStore::connect(&database_url)
            .await
            .expect("Authority store");
        let assertion = plane()
            .verify_at(&token(&claims()), 1010, SignaturePolicy::Enforce)
            .expect("valid assertion");

        let (left, right) = tokio::join!(
            consume_once(store.pool(), &assertion),
            consume_once(store.pool(), &assertion)
        );
        assert_ne!(left.expect("left consume"), right.expect("right consume"));

        drop(store);
        let reopened = crate::authority::AuthorityStore::connect(&database_url)
            .await
            .expect("reopened Authority store");
        assert!(!consume_once(reopened.pool(), &assertion)
            .await
            .expect("consume after reconnect"));
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM restless_authority.entry_assertions WHERE jti = $1",
        )
        .bind(assertion.jti)
        .fetch_one(reopened.pool())
        .await
        .expect("count assertion");
        assert_eq!(count, 1);
    }
}
