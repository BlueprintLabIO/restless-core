//! Provider-neutral human entry for the account plane.
//!
//! Local mode keeps the historical loopback-only owner. Network mode consumes
//! the Ed25519/JWKS handoff issued by Fleet, binds the proven human to one
//! durable company Actor, and establishes a short-lived browser session.
//! Authentication, organisational responsibility, and Authority are separate
//! facts; no claim in this module grants an Authority capability.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, SystemTime};

use anyhow::Context as _;
use base64::Engine as _;
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer as _, SigningKey, Verifier as _, VerifyingKey};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex as AsyncMutex, RwLock};
use tokio_util::sync::CancellationToken;
use url::Url;
use uuid::Uuid;

/// The released Fleet/Core handoff contract.
pub(crate) const ASSERTION_CONTRACT_VERSION: u32 = 1;
pub(crate) const HANDOFF_AUDIENCE: &str = "restless-core-account-plane";
pub(crate) const MEMBERSHIP_CONTROL_AUDIENCE: &str = "restless-core-membership-control";

const TOKEN_TYPE: &str = "JWT";
const MEMBERSHIP_CONTROL_TOKEN_TYPE: &str = "restless-membership-control+jwt";
const ALGORITHM: &str = "EdDSA";
const MAX_ASSERTION_LIFETIME_SECONDS: i64 = 60;
const MAX_CLOCK_SKEW_SECONDS: i64 = 30;
const MAX_JWKS_BYTES: usize = 64 * 1024;
const MAX_JWKS_AGE: Duration = Duration::from_secs(MAX_ASSERTION_LIFETIME_SECONDS as u64);
const DEFAULT_SESSION_TTL: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Refusal {
    Malformed(&'static str),
    UnsupportedVersion { got: u32, supported: u32 },
    UnknownIssuer,
    WrongAudience,
    WrongOwner,
    WrongPlane,
    WrongHost,
    UnknownKeyVersion,
    BadSignature,
    SigningKeysUnavailable,
    NotYetValid,
    Expired,
    TooLongLived,
    InvalidMembership,
}

impl Refusal {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::Malformed(_) => "assertion_malformed",
            Self::UnsupportedVersion { .. } => "assertion_unsupported_version",
            Self::UnknownIssuer => "assertion_unknown_issuer",
            Self::WrongAudience => "assertion_wrong_audience",
            Self::WrongOwner => "assertion_wrong_owner",
            Self::WrongPlane => "assertion_wrong_plane",
            Self::WrongHost => "assertion_wrong_host",
            Self::UnknownKeyVersion => "assertion_unknown_key_version",
            Self::BadSignature => "assertion_bad_signature",
            Self::SigningKeysUnavailable => "assertion_signing_keys_unavailable",
            Self::NotYetValid => "assertion_not_yet_valid",
            Self::Expired => "assertion_expired",
            Self::TooLongLived => "assertion_too_long_lived",
            Self::InvalidMembership => "assertion_invalid_membership",
        }
    }

    pub(crate) fn message(&self) -> String {
        match self {
            Self::Malformed(what) => format!("entry assertion is malformed: {what}"),
            Self::UnsupportedVersion { got, supported } => format!(
                "entry assertion contract version {got} is not supported; this plane supports {supported}"
            ),
            Self::UnknownIssuer => "entry assertion issuer is not trusted by this plane".into(),
            Self::WrongAudience => "entry assertion was not minted for a Core account plane".into(),
            Self::WrongOwner => "entry assertion belongs to a different account owner".into(),
            Self::WrongPlane => "entry assertion is routed to a different account plane".into(),
            Self::WrongHost => "entry assertion names a different account-plane hostname".into(),
            Self::UnknownKeyVersion => "entry assertion names an unknown signing key".into(),
            Self::BadSignature => "entry assertion signature is invalid".into(),
            Self::SigningKeysUnavailable => {
                "entry assertion signing keys are temporarily unavailable".into()
            }
            Self::NotYetValid => "entry assertion is not valid yet".into(),
            Self::Expired => "entry assertion has expired".into(),
            Self::TooLongLived => "entry assertion exceeds the permitted lifetime".into(),
            Self::InvalidMembership => "entry assertion membership is invalid".into(),
        }
    }
}

/// Compatibility scope used by the existing owner boundary. A real network
/// handoff always resolves to exactly one company slug.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum CompanyScope {
    Owner,
    Company { company: String },
}

impl CompanyScope {
    pub(crate) fn permits(&self, company: &str) -> bool {
        match self {
            Self::Owner => true,
            Self::Company { company: allowed } => allowed == company,
        }
    }
}

/// Exact Cloud contract. Unknown fields are refused so a producer cannot
/// silently evolve security meaning without a contract version change.
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
    pub membership_version: i64,
    pub assertion_version: u32,
}

/// Exact signed terminal-membership command contract. It is intentionally a
/// separate claims type and token type from browser handoff assertions: the
/// two credentials cannot be substituted at either verification entry point.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MembershipControlClaims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub jti: Uuid,
    pub exp: i64,
    pub iat: i64,
    pub kid: String,
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub plane_hostname: String,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub membership_id: String,
    pub membership_role: String,
    pub membership_status: String,
    pub membership_version: i64,
    pub assertion_version: u32,
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

/// A cryptographically verified context which has not yet been consumed.
/// OrgIntel consumes it and creates the durable Actor binding atomically.
#[derive(Debug, Clone)]
pub(crate) struct VerifiedAccessContext {
    pub issuer: String,
    pub subject: String,
    pub assertion_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub membership_id: String,
    pub membership_role: String,
    pub membership_version: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct VerifiedMembershipControl {
    pub issuer: String,
    pub subject: String,
    pub assertion_id: Uuid,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub key_id: String,
    pub assertion_version: u32,
    pub owner_id: Uuid,
    pub plane_id: Uuid,
    pub plane_hostname: String,
    pub company_id: Uuid,
    pub cell_id: Uuid,
    pub membership_id: String,
    pub membership_role: String,
    pub membership_status: restless_orgintel::ExternalMembershipStatus,
    pub membership_version: i64,
}

/// The server-derived identity carried by a browser session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedIdentity {
    pub user: String,
    pub issuer: Option<String>,
    pub owner: String,
    pub scope: CompanyScope,
    pub role: String,
    pub actor: Option<String>,
    pub company_id: Option<Uuid>,
    pub cell_id: Option<Uuid>,
    pub membership_id: Option<String>,
    pub membership_version: Option<i64>,
}

/// Handler-facing principal. The browser never supplies this value.
#[derive(Debug, Clone)]
pub(crate) struct RequestPrincipal {
    actor_id: String,
    membership_role: String,
    company: Option<String>,
}

impl RequestPrincipal {
    pub(crate) fn local_owner() -> Self {
        Self {
            actor_id: "owner".into(),
            membership_role: "owner".into(),
            company: None,
        }
    }

    pub(crate) fn from_verified(identity: &VerifiedIdentity) -> Option<Self> {
        Some(Self {
            actor_id: identity.actor.clone()?,
            membership_role: identity.role.clone(),
            company: match &identity.scope {
                CompanyScope::Owner => None,
                CompanyScope::Company { company } => Some(company.clone()),
            },
        })
    }

    pub(crate) fn actor_id(&self) -> &str {
        &self.actor_id
    }

    pub(crate) fn membership_role(&self) -> &str {
        &self.membership_role
    }

    pub(crate) fn permits_company(&self, company: &str) -> bool {
        self.company
            .as_deref()
            .is_none_or(|allowed| allowed == company)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SignaturePolicy {
    Enforce,
    #[cfg(test)]
    Skip,
}

/// Network-mode configuration. JWKS is loaded before the listener is exposed
/// and refreshed when a new key id appears during key rotation.
pub(crate) struct NetworkEntry {
    issuer: String,
    owner_id: Uuid,
    plane_id: Uuid,
    host: String,
    jwks_url: Url,
    client: reqwest::Client,
    keys: RwLock<HashMap<String, VerifyingKey>>,
    last_refresh: RwLock<Option<tokio::time::Instant>>,
    refresh: AsyncMutex<()>,
    session_ttl: Duration,
}

impl NetworkEntry {
    pub(crate) fn host(&self) -> &str {
        &self.host
    }

    pub(crate) fn session_ttl(&self) -> Duration {
        self.session_ttl
    }

    /// Fetch and validate the signing set before exposing the network listener.
    pub(crate) async fn prepare(&self) -> anyhow::Result<()> {
        self.refresh_keys(true).await
    }

    async fn refresh_keys(&self, force: bool) -> anyhow::Result<()> {
        let _serial = self.refresh.lock().await;
        if !force
            && self
                .last_refresh
                .read()
                .await
                .is_some_and(|refreshed| refreshed.elapsed() < MAX_JWKS_AGE)
        {
            return Ok(());
        }
        let response = self
            .client
            .get(self.jwks_url.clone())
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .context_msg("fetch RESTLESS_ENTRY_JWKS_URL")?;
        if !response.status().is_success() {
            anyhow::bail!(
                "RESTLESS_ENTRY_JWKS_URL returned HTTP {}",
                response.status()
            );
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_JWKS_BYTES as u64)
        {
            anyhow::bail!("RESTLESS_ENTRY_JWKS_URL response exceeds 64 KiB");
        }
        let bytes = response
            .bytes()
            .await
            .context_msg("read RESTLESS_ENTRY_JWKS_URL")?;
        if bytes.len() > MAX_JWKS_BYTES {
            anyhow::bail!("RESTLESS_ENTRY_JWKS_URL response exceeds 64 KiB");
        }
        let keys = parse_jwks(&bytes)?;
        *self.keys.write().await = keys;
        *self.last_refresh.write().await = Some(tokio::time::Instant::now());
        Ok(())
    }

    /// Verify a handoff. Replay consumption deliberately happens later, in
    /// the same OrgIntel transaction as principal-to-Actor binding.
    pub(crate) async fn verify(&self, token: &str) -> Result<VerifiedAccessContext, Refusal> {
        // A removed key must stop authorising assertions without a process
        // restart. Never retain a JWKS longer than the assertion lifetime.
        self.refresh_keys(false)
            .await
            .map_err(|_| Refusal::SigningKeysUnavailable)?;
        match self
            .verify_at(token, Utc::now(), SignaturePolicy::Enforce)
            .await
        {
            Err(Refusal::UnknownKeyVersion) => {
                // A newly published key may appear between process start and
                // handoff issuance. Refresh once, then report the stable error.
                self.refresh_keys(true)
                    .await
                    .map_err(|_| Refusal::SigningKeysUnavailable)?;
                self.verify_at(token, Utc::now(), SignaturePolicy::Enforce)
                    .await
            }
            result => result,
        }
    }

    /// Verify a terminal membership command. Its distinct token type,
    /// audience and closed claims shape prevent a browser handoff from being
    /// replayed at the control endpoint (or vice versa).
    pub(crate) async fn verify_membership_control(
        &self,
        token: &str,
    ) -> Result<VerifiedMembershipControl, Refusal> {
        self.refresh_keys(false)
            .await
            .map_err(|_| Refusal::SigningKeysUnavailable)?;
        match self
            .verify_membership_control_at(token, Utc::now(), SignaturePolicy::Enforce)
            .await
        {
            Err(Refusal::UnknownKeyVersion) => {
                self.refresh_keys(true)
                    .await
                    .map_err(|_| Refusal::SigningKeysUnavailable)?;
                self.verify_membership_control_at(token, Utc::now(), SignaturePolicy::Enforce)
                    .await
            }
            result => result,
        }
    }

    async fn verify_at(
        &self,
        token: &str,
        now: DateTime<Utc>,
        policy: SignaturePolicy,
    ) -> Result<VerifiedAccessContext, Refusal> {
        let parsed = parse_assertion(token)?;
        let key = self
            .keys
            .read()
            .await
            .get(&parsed.header.kid)
            .cloned()
            .ok_or(Refusal::UnknownKeyVersion)?;

        let enforce = match policy {
            SignaturePolicy::Enforce => true,
            #[cfg(test)]
            SignaturePolicy::Skip => false,
        };
        if enforce {
            let signature = Signature::from_slice(&parsed.signature)
                .map_err(|_| Refusal::Malformed("signature is not 64-byte Ed25519"))?;
            key.verify(parsed.signing_input.as_bytes(), &signature)
                .map_err(|_| Refusal::BadSignature)?;
        }

        validate_claims(
            parsed.claims,
            &parsed.header,
            &self.issuer,
            self.owner_id,
            self.plane_id,
            now,
        )
    }

    async fn verify_membership_control_at(
        &self,
        token: &str,
        now: DateTime<Utc>,
        policy: SignaturePolicy,
    ) -> Result<VerifiedMembershipControl, Refusal> {
        let parsed = parse_membership_control(token)?;
        let key = self
            .keys
            .read()
            .await
            .get(&parsed.header.kid)
            .cloned()
            .ok_or(Refusal::UnknownKeyVersion)?;

        let enforce = match policy {
            SignaturePolicy::Enforce => true,
            #[cfg(test)]
            SignaturePolicy::Skip => false,
        };
        if enforce {
            let signature = Signature::from_slice(&parsed.signature)
                .map_err(|_| Refusal::Malformed("signature is not 64-byte Ed25519"))?;
            key.verify(parsed.signing_input.as_bytes(), &signature)
                .map_err(|_| Refusal::BadSignature)?;
        }

        validate_membership_control_claims(
            parsed.claims,
            &parsed.header,
            &self.issuer,
            self.owner_id,
            self.plane_id,
            &self.host,
            now,
        )
    }

    #[cfg(test)]
    fn for_test(signing_key: &SigningKey) -> Self {
        Self {
            issuer: "https://cloud.restless.test".into(),
            owner_id: Uuid::parse_str("018f0000-0000-7000-8000-000000000001").unwrap(),
            plane_id: Uuid::parse_str("018f0000-0000-7000-8000-000000000002").unwrap(),
            host: "aris.restless.test".into(),
            jwks_url: Url::parse("https://cloud.restless.test/.well-known/jwks.json").unwrap(),
            client: reqwest::Client::new(),
            keys: RwLock::new(HashMap::from([(
                "test-2026-01".into(),
                signing_key.verifying_key(),
            )])),
            last_refresh: RwLock::new(Some(tokio::time::Instant::now())),
            refresh: AsyncMutex::new(()),
            session_ttl: DEFAULT_SESSION_TTL,
        }
    }
}

struct ParsedAssertion {
    header: AssertionHeader,
    claims: AssertionClaims,
    signing_input: String,
    signature: Vec<u8>,
}

struct ParsedMembershipControl {
    header: AssertionHeader,
    claims: MembershipControlClaims,
    signing_input: String,
    signature: Vec<u8>,
}

fn parse_assertion(token: &str) -> Result<ParsedAssertion, Refusal> {
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
    if claims.kid != header.kid {
        return Err(Refusal::Malformed("header and payload key ids differ"));
    }
    let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(signature_b64)
        .map_err(|_| Refusal::Malformed("signature is not base64url"))?;
    Ok(ParsedAssertion {
        header,
        claims,
        signing_input: format!("{header_b64}.{payload_b64}"),
        signature,
    })
}

fn parse_membership_control(token: &str) -> Result<ParsedMembershipControl, Refusal> {
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
    if header.typ != MEMBERSHIP_CONTROL_TOKEN_TYPE {
        return Err(Refusal::Malformed("unexpected token type"));
    }
    if header.alg != ALGORITHM {
        return Err(Refusal::Malformed("unexpected signature algorithm"));
    }
    let claims: MembershipControlClaims = decode_segment(payload_b64, "payload")?;
    if claims.kid != header.kid {
        return Err(Refusal::Malformed("header and payload key ids differ"));
    }
    let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(signature_b64)
        .map_err(|_| Refusal::Malformed("signature is not base64url"))?;
    Ok(ParsedMembershipControl {
        header,
        claims,
        signing_input: format!("{header_b64}.{payload_b64}"),
        signature,
    })
}

fn validate_claims(
    claims: AssertionClaims,
    header: &AssertionHeader,
    issuer: &str,
    owner_id: Uuid,
    plane_id: Uuid,
    now: DateTime<Utc>,
) -> Result<VerifiedAccessContext, Refusal> {
    if claims.assertion_version != ASSERTION_CONTRACT_VERSION {
        return Err(Refusal::UnsupportedVersion {
            got: claims.assertion_version,
            supported: ASSERTION_CONTRACT_VERSION,
        });
    }
    if claims.iss.trim_end_matches('/') != issuer {
        return Err(Refusal::UnknownIssuer);
    }
    if claims.aud != HANDOFF_AUDIENCE {
        return Err(Refusal::WrongAudience);
    }
    if claims.owner_id != owner_id {
        return Err(Refusal::WrongOwner);
    }
    if claims.plane_id != plane_id {
        return Err(Refusal::WrongPlane);
    }
    if claims.kid != header.kid
        || claims.sub.trim().is_empty()
        || claims.sub.len() > 512
        || claims.membership_id.trim().is_empty()
        || claims.membership_id.len() > 512
        || claims.membership_version < 0
        || !matches!(
            claims.membership_role.as_str(),
            "owner" | "admin" | "member"
        )
        || claims.company_id.is_nil()
        || claims.cell_id.is_nil()
        || claims.jti.is_nil()
    {
        return Err(Refusal::InvalidMembership);
    }
    if claims.iat > now.timestamp() + MAX_CLOCK_SKEW_SECONDS {
        return Err(Refusal::NotYetValid);
    }
    if claims.exp <= now.timestamp() {
        return Err(Refusal::Expired);
    }
    if claims.exp <= claims.iat
        || claims.exp.saturating_sub(claims.iat) > MAX_ASSERTION_LIFETIME_SECONDS
    {
        return Err(Refusal::TooLongLived);
    }
    let issued_at = DateTime::from_timestamp(claims.iat, 0)
        .ok_or(Refusal::Malformed("issued-at is out of range"))?;
    let expires_at = DateTime::from_timestamp(claims.exp, 0)
        .ok_or(Refusal::Malformed("expiry is out of range"))?;
    Ok(VerifiedAccessContext {
        issuer: claims.iss.trim_end_matches('/').to_string(),
        subject: claims.sub,
        assertion_id: claims.jti,
        issued_at,
        expires_at,
        owner_id: claims.owner_id,
        plane_id: claims.plane_id,
        company_id: claims.company_id,
        cell_id: claims.cell_id,
        membership_id: claims.membership_id,
        membership_role: claims.membership_role,
        membership_version: claims.membership_version,
    })
}

fn validate_membership_control_claims(
    claims: MembershipControlClaims,
    header: &AssertionHeader,
    issuer: &str,
    owner_id: Uuid,
    plane_id: Uuid,
    plane_hostname: &str,
    now: DateTime<Utc>,
) -> Result<VerifiedMembershipControl, Refusal> {
    if claims.assertion_version != ASSERTION_CONTRACT_VERSION {
        return Err(Refusal::UnsupportedVersion {
            got: claims.assertion_version,
            supported: ASSERTION_CONTRACT_VERSION,
        });
    }
    if claims.iss.trim_end_matches('/') != issuer {
        return Err(Refusal::UnknownIssuer);
    }
    if claims.aud != MEMBERSHIP_CONTROL_AUDIENCE {
        return Err(Refusal::WrongAudience);
    }
    if claims.owner_id != owner_id {
        return Err(Refusal::WrongOwner);
    }
    if claims.plane_id != plane_id {
        return Err(Refusal::WrongPlane);
    }
    if claims.plane_hostname != plane_hostname {
        return Err(Refusal::WrongHost);
    }
    let membership_status = match claims.membership_status.as_str() {
        "suspended" => restless_orgintel::ExternalMembershipStatus::Suspended,
        "removed" => restless_orgintel::ExternalMembershipStatus::Removed,
        _ => return Err(Refusal::InvalidMembership),
    };
    if claims.kid != header.kid
        || claims.sub.trim().is_empty()
        || claims.sub.len() > 512
        || claims.membership_id.trim().is_empty()
        || claims.membership_id.len() > 512
        || claims.membership_version < 0
        || !matches!(
            claims.membership_role.as_str(),
            "owner" | "admin" | "member"
        )
        || claims.company_id.is_nil()
        || claims.cell_id.is_nil()
        || claims.jti.is_nil()
    {
        return Err(Refusal::InvalidMembership);
    }
    if claims.iat > now.timestamp() + MAX_CLOCK_SKEW_SECONDS {
        return Err(Refusal::NotYetValid);
    }
    if claims.exp <= now.timestamp() {
        return Err(Refusal::Expired);
    }
    if claims.exp <= claims.iat
        || claims.exp.saturating_sub(claims.iat) > MAX_ASSERTION_LIFETIME_SECONDS
    {
        return Err(Refusal::TooLongLived);
    }
    let issued_at = DateTime::from_timestamp(claims.iat, 0)
        .ok_or(Refusal::Malformed("issued-at is out of range"))?;
    let expires_at = DateTime::from_timestamp(claims.exp, 0)
        .ok_or(Refusal::Malformed("expiry is out of range"))?;
    Ok(VerifiedMembershipControl {
        issuer: claims.iss.trim_end_matches('/').to_string(),
        subject: claims.sub,
        assertion_id: claims.jti,
        issued_at,
        expires_at,
        key_id: claims.kid,
        assertion_version: claims.assertion_version,
        owner_id: claims.owner_id,
        plane_id: claims.plane_id,
        plane_hostname: claims.plane_hostname,
        company_id: claims.company_id,
        cell_id: claims.cell_id,
        membership_id: claims.membership_id,
        membership_role: claims.membership_role,
        membership_status,
        membership_version: claims.membership_version,
    })
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

fn parse_jwks(raw: &[u8]) -> anyhow::Result<HashMap<String, VerifyingKey>> {
    let set: JwkSet = serde_json::from_slice(raw).context_msg("parse Fleet JWKS")?;
    if set.keys.is_empty() || set.keys.len() > 16 {
        anyhow::bail!("Fleet JWKS must publish between 1 and 16 keys");
    }
    let mut parsed = HashMap::new();
    for key in set.keys {
        if key.kty != "OKP"
            || key.crv != "Ed25519"
            || key.alg != ALGORITHM
            || key.use_ != "sig"
            || key.kid.is_empty()
            || key.kid.len() > 64
            || !key
                .kid
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            anyhow::bail!("Fleet JWKS contains an unsupported signing key");
        }
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&key.x)
            .map_err(|_| anyhow::anyhow!("Fleet JWKS Ed25519 key is not base64url"))?;
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Fleet JWKS Ed25519 key must be exactly 32 bytes"))?;
        let verifying_key = VerifyingKey::from_bytes(&bytes)
            .map_err(|_| anyhow::anyhow!("Fleet JWKS Ed25519 key is invalid"))?;
        if parsed.insert(key.kid, verifying_key).is_some() {
            anyhow::bail!("Fleet JWKS contains a duplicate key id");
        }
    }
    Ok(parsed)
}

#[derive(Clone)]
pub(crate) enum EntryMode {
    Local,
    Network(std::sync::Arc<NetworkEntry>),
}

impl EntryMode {
    pub(crate) fn from_env() -> anyhow::Result<Self> {
        let mode = std::env::var("RESTLESS_ENTRY_MODE").unwrap_or_else(|_| "local".to_string());
        match mode.as_str() {
            "local" => Ok(Self::Local),
            "network" => {
                let allow_insecure = matches!(
                    std::env::var("RESTLESS_ENTRY_ALLOW_INSECURE_HTTP").as_deref(),
                    Ok("1") | Ok("true")
                );
                let issuer = canonical_service_url(
                    "RESTLESS_ENTRY_ISSUER",
                    &required("RESTLESS_ENTRY_ISSUER")?,
                    allow_insecure,
                )?;
                let jwks_url = canonical_service_url(
                    "RESTLESS_ENTRY_JWKS_URL",
                    &required("RESTLESS_ENTRY_JWKS_URL")?,
                    allow_insecure,
                )?;
                validate_issuer_jwks(&issuer, &jwks_url)?;
                let owner_id = required_uuid("RESTLESS_ENTRY_OWNER_ID")?;
                let plane_id = required_uuid("RESTLESS_ENTRY_PLANE_ID")?;
                let host = required("RESTLESS_ENTRY_HOST")?.to_ascii_lowercase();
                if host.len() > 253
                    || host.contains('/')
                    || host.contains(':')
                    || host.parse::<std::net::IpAddr>().is_ok()
                    || !host.contains('.')
                {
                    anyhow::bail!(
                        "RESTLESS_ENTRY_HOST must be the account plane hostname without scheme or port"
                    );
                }
                let session_ttl = match std::env::var("RESTLESS_ENTRY_SESSION_TTL_SECONDS") {
                    Ok(value) => {
                        let seconds: u64 = value.parse().map_err(|_| {
                            anyhow::anyhow!(
                                "RESTLESS_ENTRY_SESSION_TTL_SECONDS must be positive seconds"
                            )
                        })?;
                        if !(60..=24 * 60 * 60).contains(&seconds) {
                            anyhow::bail!(
                                "RESTLESS_ENTRY_SESSION_TTL_SECONDS must be between 60 and 86400"
                            );
                        }
                        Duration::from_secs(seconds)
                    }
                    Err(_) => DEFAULT_SESSION_TTL,
                };
                let client = reqwest::Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .timeout(Duration::from_secs(5))
                    .build()
                    .context_msg("build entry JWKS client")?;
                Ok(Self::Network(std::sync::Arc::new(NetworkEntry {
                    issuer: issuer.as_str().trim_end_matches('/').into(),
                    owner_id,
                    plane_id,
                    host,
                    jwks_url,
                    client,
                    keys: RwLock::new(HashMap::new()),
                    last_refresh: RwLock::new(None),
                    refresh: AsyncMutex::new(()),
                    session_ttl,
                })))
            }
            other => anyhow::bail!("RESTLESS_ENTRY_MODE must be local or network, not {other:?}"),
        }
    }

    pub(crate) fn network(&self) -> Option<&std::sync::Arc<NetworkEntry>> {
        match self {
            Self::Local => None,
            Self::Network(entry) => Some(entry),
        }
    }

    /// Immutable account-plane coordinates already validated for network
    /// entry. Internal Fleet contracts reuse this exact tuple instead of
    /// reparsing a second, potentially divergent set of environment values.
    pub(crate) fn network_coordinates(&self) -> Option<(Uuid, Uuid, &str)> {
        self.network()
            .map(|entry| (entry.owner_id, entry.plane_id, entry.host.as_str()))
    }
}

fn canonical_service_url(variable: &str, raw: &str, allow_insecure: bool) -> anyhow::Result<Url> {
    let url = Url::parse(raw).with_context(|| format!("parse {variable}"))?;
    let safe_scheme = url.scheme() == "https"
        || (allow_insecure
            && url.scheme() == "http"
            && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1")));
    if !safe_scheme
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        anyhow::bail!("{variable} must be an HTTPS URL without credentials, query, or fragment");
    }
    Ok(url)
}

fn validate_issuer_jwks(issuer: &Url, jwks: &Url) -> anyhow::Result<()> {
    if issuer.path() != "/" {
        anyhow::bail!("RESTLESS_ENTRY_ISSUER must be an origin URL with no path");
    }
    if issuer.scheme() != jwks.scheme()
        || issuer.host_str() != jwks.host_str()
        || issuer.port_or_known_default() != jwks.port_or_known_default()
        || jwks.path() != "/.well-known/jwks.json"
    {
        anyhow::bail!("RESTLESS_ENTRY_JWKS_URL must be the issuer origin's /.well-known/jwks.json");
    }
    Ok(())
}

fn required(variable: &str) -> anyhow::Result<String> {
    let value = std::env::var(variable).unwrap_or_default();
    if value.trim().is_empty() {
        anyhow::bail!("network entry mode requires {variable}; refusing to trust network position");
    }
    Ok(value.trim().to_string())
}

fn required_uuid(variable: &str) -> anyhow::Result<Uuid> {
    let value = required(variable)?;
    let parsed = Uuid::parse_str(&value).with_context(|| format!("parse {variable} as UUID"))?;
    if parsed.is_nil() {
        anyhow::bail!("{variable} must not be the nil UUID");
    }
    Ok(parsed)
}

trait ContextMsg<T> {
    fn context_msg(self, message: &'static str) -> anyhow::Result<T>;
}

impl<T, E> ContextMsg<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn context_msg(self, message: &'static str) -> anyhow::Result<T> {
        self.map_err(anyhow::Error::from).context(message)
    }
}

/// Browser sessions are intentionally in memory. A process restart revokes
/// them; the durable assertion consumption prevents the original credential
/// from being exchanged again afterward.
#[derive(Default)]
pub(crate) struct SessionStore {
    sessions: Mutex<HashMap<String, SessionRecord>>,
    reconciliation_guards: Mutex<HashMap<PrincipalSessionKey, Weak<AsyncMutex<()>>>>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct PrincipalSessionKey {
    issuer: String,
    subject: String,
    company_id: Uuid,
}

struct SessionRecord {
    identity: VerifiedIdentity,
    expires_at: SystemTime,
    cancellation: CancellationToken,
}

#[derive(Clone)]
pub(crate) struct SessionLease {
    pub(crate) identity: VerifiedIdentity,
    pub(crate) expires_at: SystemTime,
    pub(crate) cancellation: CancellationToken,
}

impl SessionLease {
    pub(crate) fn is_ended(&self) -> bool {
        self.cancellation.is_cancelled() || self.expires_at <= SystemTime::now()
    }

    pub(crate) async fn ended(&self) {
        let remaining = self
            .expires_at
            .duration_since(SystemTime::now())
            .unwrap_or(Duration::ZERO);
        tokio::select! {
            _ = self.cancellation.cancelled() => {},
            _ = tokio::time::sleep(remaining) => {},
        }
    }
}

impl SessionStore {
    /// Serialize the durable membership transition and in-memory session
    /// reconciliation for one hosted principal. The database remains the
    /// authority; this guard prevents two completed requests from applying
    /// their post-commit session effects in the opposite order.
    pub(crate) fn reconciliation_guard(
        &self,
        issuer: &str,
        subject: &str,
        company_id: Uuid,
    ) -> Arc<AsyncMutex<()>> {
        let key = PrincipalSessionKey {
            issuer: issuer.trim_end_matches('/').to_string(),
            subject: subject.to_string(),
            company_id,
        };
        let mut guards = self
            .reconciliation_guards
            .lock()
            .expect("session reconciliation registry poisoned");
        guards.retain(|_, guard| guard.strong_count() > 0);
        if let Some(guard) = guards.get(&key).and_then(Weak::upgrade) {
            return guard;
        }
        let guard = Arc::new(AsyncMutex::new(()));
        guards.insert(key, Arc::downgrade(&guard));
        guard
    }

    pub(crate) fn establish(&self, identity: VerifiedIdentity, ttl: Duration) -> String {
        let token = Uuid::new_v4().to_string();
        let mut sessions = self.sessions.lock().expect("session store poisoned");
        let now = SystemTime::now();
        sessions.retain(|_, session| {
            if session.expires_at <= now {
                session.cancellation.cancel();
                false
            } else {
                true
            }
        });
        sessions.insert(
            token.clone(),
            SessionRecord {
                identity,
                expires_at: now + ttl,
                cancellation: CancellationToken::new(),
            },
        );
        token
    }

    pub(crate) fn resolve_lease(&self, token: &str) -> Option<SessionLease> {
        let mut sessions = self.sessions.lock().expect("session store poisoned");
        let now = SystemTime::now();
        sessions.retain(|_, session| {
            if session.expires_at <= now {
                session.cancellation.cancel();
                false
            } else {
                true
            }
        });
        sessions.get(token).map(|session| SessionLease {
            identity: session.identity.clone(),
            expires_at: session.expires_at,
            cancellation: session.cancellation.clone(),
        })
    }

    pub(crate) fn revoke(&self, token: &str) {
        if let Some(session) = self
            .sessions
            .lock()
            .expect("session store poisoned")
            .remove(token)
        {
            session.cancellation.cancel();
        }
    }

    /// Drop only sessions covered by one committed terminal membership
    /// version. Newer re-entry and every other company/member remain live.
    pub(crate) fn revoke_membership_through(
        &self,
        issuer: &str,
        company_id: Uuid,
        membership_id: &str,
        membership_version: i64,
    ) -> usize {
        let mut sessions = self.sessions.lock().expect("session store poisoned");
        let before = sessions.len();
        let now = SystemTime::now();
        sessions.retain(|_, session| {
            if session.expires_at <= now {
                session.cancellation.cancel();
                return false;
            }
            let covered = session
                .identity
                .issuer
                .as_deref()
                .is_some_and(|session_issuer| {
                    session_issuer.trim_end_matches('/') == issuer.trim_end_matches('/')
                })
                && session.identity.company_id == Some(company_id)
                && session.identity.membership_id.as_deref() == Some(membership_id)
                && session
                    .identity
                    .membership_version
                    .is_some_and(|version| version <= membership_version);
            if covered {
                session.cancellation.cancel();
            }
            !covered
        });
        before - sessions.len()
    }

    /// Cancel browser leases for this hosted principal whose durable security
    /// tuple has been replaced by a successful active handoff. Exact-current
    /// sessions remain usable so opening another tab does not log out the first.
    pub(crate) fn revoke_principal_except_current(&self, current: &VerifiedIdentity) -> usize {
        let (Some(current_issuer), Some(current_company_id)) =
            (current.issuer.as_deref(), current.company_id)
        else {
            return 0;
        };
        let mut sessions = self.sessions.lock().expect("session store poisoned");
        let before = sessions.len();
        let now = SystemTime::now();
        sessions.retain(|_, session| {
            if session.expires_at <= now {
                session.cancellation.cancel();
                return false;
            }
            let same_principal = session.identity.user == current.user
                && session.identity.company_id == Some(current_company_id)
                && session.identity.issuer.as_deref().is_some_and(|issuer| {
                    issuer.trim_end_matches('/') == current_issuer.trim_end_matches('/')
                });
            let same_security_tuple = same_principal
                && session.identity.owner == current.owner
                && session.identity.scope == current.scope
                && session.identity.role == current.role
                && session.identity.actor == current.actor
                && session.identity.cell_id == current.cell_id
                && session.identity.membership_id == current.membership_id
                && session.identity.membership_version == current.membership_version;
            let superseded = same_principal && !same_security_tuple;
            if superseded {
                session.cancellation.cancel();
            }
            !superseded
        });
        before - sessions.len()
    }
}

/// The single place company scope is derived from a request path.
pub(crate) fn company_in_path(path: &str) -> Option<&str> {
    let rest = path
        .strip_prefix("/api/companies/")
        .or_else(|| path.strip_prefix("/desktop/"))?;
    let company = rest.split('/').next()?;
    (!company.is_empty()).then_some(company)
}

fn mint(claims: &AssertionClaims, key_id: &str, key: &SigningKey) -> String {
    mint_with_type(claims, TOKEN_TYPE, key_id, key)
}

fn mint_with_type<T: Serialize>(
    claims: &T,
    token_type: &str,
    key_id: &str,
    key: &SigningKey,
) -> String {
    let header = AssertionHeader {
        alg: ALGORITHM.into(),
        typ: token_type.into(),
        kid: key_id.into(),
    };
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&header).expect("header encodes"));
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(claims).expect("claims encode"));
    let signing_input = format!("{header}.{payload}");
    let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(key.sign(signing_input.as_bytes()).to_bytes());
    format!("{signing_input}.{signature}")
}

/// Test issuer command retained for release-contract exercises. A production
/// account plane never carries this private seed.
pub(crate) fn mint_from_env() -> anyhow::Result<String> {
    let key_bytes = base64::engine::general_purpose::STANDARD
        .decode(required("RESTLESS_ENTRY_TEST_SIGNING_KEY_B64")?)
        .map_err(|_| anyhow::anyhow!("RESTLESS_ENTRY_TEST_SIGNING_KEY_B64 must be base64"))?;
    let key_bytes: [u8; 32] = key_bytes.try_into().map_err(|_| {
        anyhow::anyhow!("RESTLESS_ENTRY_TEST_SIGNING_KEY_B64 must decode to exactly 32 bytes")
    })?;
    let signing_key = SigningKey::from_bytes(&key_bytes);
    let key_id = required("RESTLESS_ENTRY_TEST_SIGNING_KEY_ID")?;
    let now = Utc::now().timestamp();
    let ttl = std::env::var("RESTLESS_ENTRY_TEST_TTL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(45);
    if !(1..=MAX_ASSERTION_LIFETIME_SECONDS).contains(&ttl) {
        anyhow::bail!("RESTLESS_ENTRY_TEST_TTL_SECONDS must be between 1 and 60");
    }
    let claims = AssertionClaims {
        iss: required("RESTLESS_ENTRY_ISSUER")?
            .trim_end_matches('/')
            .into(),
        aud: HANDOFF_AUDIENCE.into(),
        sub: std::env::var("RESTLESS_ENTRY_TEST_USER").unwrap_or_else(|_| "test-user".into()),
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
        membership_version: std::env::var("RESTLESS_ENTRY_TEST_MEMBERSHIP_VERSION")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
        assertion_version: ASSERTION_CONTRACT_VERSION,
    };
    Ok(mint(&claims, &key_id, &signing_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use axum::{routing::get, Router};

    const KEY_ID: &str = "test-2026-01";

    fn key() -> SigningKey {
        SigningKey::from_bytes(&[7; 32])
    }

    fn other_key() -> SigningKey {
        SigningKey::from_bytes(&[9; 32])
    }

    fn at(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(seconds, 0).unwrap()
    }

    fn claims() -> AssertionClaims {
        AssertionClaims {
            iss: "https://cloud.restless.test".into(),
            aud: HANDOFF_AUDIENCE.into(),
            sub: "user-1".into(),
            jti: Uuid::parse_str("018f0000-0000-7000-8000-000000000003").unwrap(),
            exp: 1_010,
            iat: 950,
            kid: KEY_ID.into(),
            owner_id: Uuid::parse_str("018f0000-0000-7000-8000-000000000001").unwrap(),
            plane_id: Uuid::parse_str("018f0000-0000-7000-8000-000000000002").unwrap(),
            company_id: Uuid::parse_str("018f0000-0000-7000-8000-000000000004").unwrap(),
            cell_id: Uuid::parse_str("018f0000-0000-7000-8000-000000000005").unwrap(),
            membership_id: "membership-1".into(),
            membership_role: "owner".into(),
            membership_version: 7,
            assertion_version: ASSERTION_CONTRACT_VERSION,
        }
    }

    fn token(claims: &AssertionClaims) -> String {
        mint(claims, &claims.kid, &key())
    }

    fn membership_control_claims() -> MembershipControlClaims {
        MembershipControlClaims {
            iss: "https://cloud.restless.test".into(),
            aud: MEMBERSHIP_CONTROL_AUDIENCE.into(),
            sub: "user-1".into(),
            jti: Uuid::parse_str("018f0000-0000-7000-8000-000000000006").unwrap(),
            exp: 1_010,
            iat: 950,
            kid: KEY_ID.into(),
            owner_id: Uuid::parse_str("018f0000-0000-7000-8000-000000000001").unwrap(),
            plane_id: Uuid::parse_str("018f0000-0000-7000-8000-000000000002").unwrap(),
            plane_hostname: "aris.restless.test".into(),
            company_id: Uuid::parse_str("018f0000-0000-7000-8000-000000000004").unwrap(),
            cell_id: Uuid::parse_str("018f0000-0000-7000-8000-000000000005").unwrap(),
            membership_id: "membership-1".into(),
            membership_role: "member".into(),
            membership_status: "suspended".into(),
            membership_version: 8,
            assertion_version: ASSERTION_CONTRACT_VERSION,
        }
    }

    fn membership_control_token(claims: &MembershipControlClaims) -> String {
        mint_with_type(claims, MEMBERSHIP_CONTROL_TOKEN_TYPE, &claims.kid, &key())
    }

    fn jwks(key_id: &str, signing_key: &SigningKey) -> String {
        let x = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(signing_key.verifying_key().as_bytes());
        serde_json::json!({
            "keys": [{
                "kty": "OKP",
                "crv": "Ed25519",
                "alg": "EdDSA",
                "use": "sig",
                "kid": key_id,
                "x": x,
            }]
        })
        .to_string()
    }

    async fn refusal(claims: &AssertionClaims) -> Refusal {
        NetworkEntry::for_test(&key())
            .verify_at(&token(claims), at(1_000), SignaturePolicy::Enforce)
            .await
            .expect_err("assertion should be refused")
    }

    #[tokio::test]
    async fn cloud_contract_assertion_verifies() {
        let verified = NetworkEntry::for_test(&key())
            .verify_at(&token(&claims()), at(1_000), SignaturePolicy::Enforce)
            .await
            .expect("valid Cloud assertion");
        assert_eq!(verified.subject, "user-1");
        assert_eq!(verified.membership_version, 7);
        assert_eq!(verified.membership_role, "owner");
        assert_eq!(verified.issued_at, at(950));
        assert_eq!(verified.expires_at, at(1_010));
    }

    #[tokio::test]
    async fn cloud_membership_control_contract_verifies_and_cannot_cross_use_handoffs() {
        let entry = NetworkEntry::for_test(&key());
        let control = membership_control_claims();
        let verified = entry
            .verify_membership_control_at(
                &membership_control_token(&control),
                at(1_000),
                SignaturePolicy::Enforce,
            )
            .await
            .expect("valid Cloud membership control");
        assert_eq!(verified.subject, "user-1");
        assert_eq!(verified.membership_version, 8);
        assert_eq!(
            verified.membership_status,
            restless_orgintel::ExternalMembershipStatus::Suspended
        );

        assert_eq!(
            entry
                .verify_membership_control_at(
                    &token(&claims()),
                    at(1_000),
                    SignaturePolicy::Enforce,
                )
                .await
                .unwrap_err(),
            Refusal::Malformed("unexpected token type")
        );
        assert_eq!(
            entry
                .verify_at(
                    &membership_control_token(&control),
                    at(1_000),
                    SignaturePolicy::Enforce,
                )
                .await
                .unwrap_err(),
            Refusal::Malformed("unexpected token type")
        );
    }

    #[tokio::test]
    async fn membership_control_coordinates_shape_signature_and_lifetime_are_strict() {
        async fn refusal(claims: &MembershipControlClaims) -> Refusal {
            NetworkEntry::for_test(&key())
                .verify_membership_control_at(
                    &membership_control_token(claims),
                    at(1_000),
                    SignaturePolicy::Enforce,
                )
                .await
                .expect_err("membership control should be refused")
        }

        let mut candidate = membership_control_claims();
        candidate.aud = HANDOFF_AUDIENCE.into();
        assert_eq!(refusal(&candidate).await, Refusal::WrongAudience);

        let mut candidate = membership_control_claims();
        candidate.owner_id = Uuid::new_v4();
        assert_eq!(refusal(&candidate).await, Refusal::WrongOwner);

        let mut candidate = membership_control_claims();
        candidate.plane_id = Uuid::new_v4();
        assert_eq!(refusal(&candidate).await, Refusal::WrongPlane);

        let mut candidate = membership_control_claims();
        candidate.plane_hostname = "another.restless.test".into();
        assert_eq!(refusal(&candidate).await, Refusal::WrongHost);

        let mut candidate = membership_control_claims();
        candidate.membership_status = "active".into();
        assert_eq!(refusal(&candidate).await, Refusal::InvalidMembership);

        let mut candidate = membership_control_claims();
        candidate.membership_version = -1;
        assert_eq!(refusal(&candidate).await, Refusal::InvalidMembership);

        let mut candidate = membership_control_claims();
        candidate.exp = candidate.iat + 61;
        assert_eq!(refusal(&candidate).await, Refusal::TooLongLived);

        let forged = mint_with_type(
            &membership_control_claims(),
            MEMBERSHIP_CONTROL_TOKEN_TYPE,
            KEY_ID,
            &other_key(),
        );
        assert_eq!(
            NetworkEntry::for_test(&key())
                .verify_membership_control_at(&forged, at(1_000), SignaturePolicy::Enforce)
                .await
                .unwrap_err(),
            Refusal::BadSignature
        );

        let mut unknown_field = serde_json::to_value(membership_control_claims()).unwrap();
        unknown_field["unexpected"] = serde_json::json!(true);
        let unknown_field = mint_with_type(
            &unknown_field,
            MEMBERSHIP_CONTROL_TOKEN_TYPE,
            KEY_ID,
            &key(),
        );
        assert_eq!(
            NetworkEntry::for_test(&key())
                .verify_membership_control_at(&unknown_field, at(1_000), SignaturePolicy::Enforce,)
                .await
                .unwrap_err(),
            Refusal::Malformed("payload is not valid JSON")
        );
    }

    #[tokio::test]
    async fn wrong_contract_coordinates_are_distinct_refusals() {
        let mut candidate = claims();
        candidate.iss = "https://attacker.test".into();
        assert_eq!(refusal(&candidate).await, Refusal::UnknownIssuer);

        let mut candidate = claims();
        candidate.aud = "another-audience".into();
        assert_eq!(refusal(&candidate).await, Refusal::WrongAudience);

        let mut candidate = claims();
        candidate.owner_id = Uuid::new_v4();
        assert_eq!(refusal(&candidate).await, Refusal::WrongOwner);

        let mut candidate = claims();
        candidate.plane_id = Uuid::new_v4();
        assert_eq!(refusal(&candidate).await, Refusal::WrongPlane);

        let mut candidate = claims();
        candidate.assertion_version += 1;
        assert!(matches!(
            refusal(&candidate).await,
            Refusal::UnsupportedVersion { .. }
        ));
    }

    #[tokio::test]
    async fn expiry_lifetime_and_future_issuance_are_refused() {
        let entry = NetworkEntry::for_test(&key());
        assert_eq!(
            entry
                .verify_at(&token(&claims()), at(1_010), SignaturePolicy::Enforce)
                .await
                .unwrap_err(),
            Refusal::Expired
        );

        let mut future = claims();
        future.iat = 1_031;
        future.exp = 1_091;
        assert_eq!(refusal(&future).await, Refusal::NotYetValid);

        let mut long = claims();
        long.iat = 950;
        long.exp = 1_011;
        assert_eq!(refusal(&long).await, Refusal::TooLongLived);
    }

    #[tokio::test]
    async fn membership_shape_is_closed_and_versioned() {
        let mut candidate = claims();
        candidate.membership_role = "authority-owner".into();
        assert_eq!(refusal(&candidate).await, Refusal::InvalidMembership);

        let mut candidate = claims();
        candidate.membership_version = -1;
        assert_eq!(refusal(&candidate).await, Refusal::InvalidMembership);

        let mut candidate = claims();
        candidate.sub.clear();
        assert_eq!(refusal(&candidate).await, Refusal::InvalidMembership);
    }

    #[tokio::test]
    async fn signature_and_key_selection_are_enforced() {
        let forged = mint(&claims(), KEY_ID, &other_key());
        let entry = NetworkEntry::for_test(&key());
        assert_eq!(
            entry
                .verify_at(&forged, at(1_000), SignaturePolicy::Enforce)
                .await
                .unwrap_err(),
            Refusal::BadSignature
        );
        assert_eq!(
            entry
                .verify_at(&forged, at(1_000), SignaturePolicy::Skip)
                .await
                .expect("only signature enforcement rejects this fixture")
                .subject,
            "user-1"
        );

        let mut unknown = claims();
        unknown.kid = "rotated-key".into();
        assert_eq!(
            NetworkEntry::for_test(&key())
                .verify_at(&token(&unknown), at(1_000), SignaturePolicy::Enforce)
                .await
                .unwrap_err(),
            Refusal::UnknownKeyVersion
        );
    }

    #[tokio::test]
    async fn stale_jwks_is_refreshed_before_a_known_key_can_authorise_entry() {
        let published = Arc::new(RwLock::new(jwks(KEY_ID, &key())));
        let response = Arc::clone(&published);
        let app = Router::new().route(
            "/.well-known/jwks.json",
            get(move || {
                let response = Arc::clone(&response);
                async move { response.read().await.clone() }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let issuer = format!("http://{address}");
        let entry = NetworkEntry {
            issuer: issuer.clone(),
            owner_id: Uuid::parse_str("018f0000-0000-7000-8000-000000000001").unwrap(),
            plane_id: Uuid::parse_str("018f0000-0000-7000-8000-000000000002").unwrap(),
            host: "plane.restless.test".into(),
            jwks_url: Url::parse(&format!("{issuer}/.well-known/jwks.json")).unwrap(),
            client: reqwest::Client::new(),
            keys: RwLock::new(HashMap::new()),
            last_refresh: RwLock::new(None),
            refresh: AsyncMutex::new(()),
            session_ttl: DEFAULT_SESSION_TTL,
        };
        entry.prepare().await.unwrap();
        let now = Utc::now();
        let mut candidate = claims();
        candidate.iss = issuer;
        candidate.iat = now.timestamp();
        candidate.exp = now.timestamp() + 45;
        let assertion = token(&candidate);
        entry.verify(&assertion).await.expect("published key");

        *published.write().await = jwks("replacement-key", &other_key());
        *entry.last_refresh.write().await =
            Some(tokio::time::Instant::now() - MAX_JWKS_AGE - Duration::from_millis(1));
        assert_eq!(
            entry.verify(&assertion).await.unwrap_err(),
            Refusal::UnknownKeyVersion
        );
        server.abort();
    }

    #[test]
    fn fleet_jwks_shape_is_validated() {
        let published = jwks(KEY_ID, &key());
        let parsed = parse_jwks(published.as_bytes()).expect("Cloud JWKS");
        assert_eq!(parsed.get(KEY_ID), Some(&key().verifying_key()));

        assert!(parse_jwks(br#"{"keys":[]}"#).is_err());
        assert!(parse_jwks(
            br#"{"keys":[{"kty":"RSA","crv":"Ed25519","alg":"EdDSA","use":"sig","kid":"x","x":"x"}]}"#
        )
        .is_err());
    }

    #[test]
    fn fleet_issuer_and_jwks_use_the_exact_published_locations() {
        let issuer = Url::parse("https://cloud.restless.run/").unwrap();
        let jwks = Url::parse("https://cloud.restless.run/.well-known/jwks.json").unwrap();
        validate_issuer_jwks(&issuer, &jwks).expect("exact Cloud locations");

        assert!(validate_issuer_jwks(
            &Url::parse("https://cloud.restless.run/api").unwrap(),
            &jwks,
        )
        .is_err());
        assert!(validate_issuer_jwks(
            &issuer,
            &Url::parse("https://keys.restless.run/.well-known/jwks.json").unwrap(),
        )
        .is_err());
        assert!(validate_issuer_jwks(
            &issuer,
            &Url::parse("https://cloud.restless.run/keys.json").unwrap(),
        )
        .is_err());
    }

    #[test]
    fn request_scope_is_server_derived() {
        let scoped = VerifiedIdentity {
            user: "user".into(),
            issuer: Some("https://cloud.restless.test".into()),
            owner: Uuid::new_v4().to_string(),
            scope: CompanyScope::Company {
                company: "aris".into(),
            },
            role: "member".into(),
            actor: Some("human-1".into()),
            company_id: Some(Uuid::new_v4()),
            cell_id: Some(Uuid::new_v4()),
            membership_id: Some("membership-1".into()),
            membership_version: Some(2),
        };
        let principal = RequestPrincipal::from_verified(&scoped).unwrap();
        assert_eq!(principal.actor_id(), "human-1");
        assert!(principal.permits_company("aris"));
        assert!(!principal.permits_company("other"));
        assert_eq!(principal.membership_role(), "member");
    }

    #[test]
    fn company_is_derived_from_the_path_in_one_place() {
        assert_eq!(company_in_path("/api/companies/aris/cockpit"), Some("aris"));
        assert_eq!(company_in_path("/desktop/aris/observe"), Some("aris"));
        assert_eq!(company_in_path("/api/companies/"), None);
        assert_eq!(company_in_path("/health"), None);
    }

    #[test]
    fn session_restart_is_fail_closed() {
        let store = SessionStore::default();
        let identity = VerifiedIdentity {
            user: "user".into(),
            issuer: None,
            owner: "owner".into(),
            scope: CompanyScope::Owner,
            role: "owner".into(),
            actor: Some("owner".into()),
            company_id: None,
            cell_id: None,
            membership_id: None,
            membership_version: None,
        };
        let token = store.establish(identity, Duration::from_secs(60));
        assert!(store.resolve_lease(&token).is_some());
        store.revoke(&token);
        assert!(store.resolve_lease(&token).is_none());
        assert!(SessionStore::default().resolve_lease(&token).is_none());
    }

    #[tokio::test]
    async fn session_reconciliation_is_serialized_per_principal() {
        let store = Arc::new(SessionStore::default());
        let company_id = Uuid::new_v4();
        let first =
            store.reconciliation_guard("https://cloud.restless.test/", "user-1", company_id);
        let same = store.reconciliation_guard("https://cloud.restless.test", "user-1", company_id);
        assert!(
            Arc::ptr_eq(&first, &same),
            "issuer normalization shares a guard"
        );

        let held = first.lock().await;
        let waiter = tokio::spawn(async move {
            let _held = same.lock().await;
        });
        tokio::task::yield_now().await;
        assert!(
            !waiter.is_finished(),
            "same principal waits for reconciliation"
        );

        let other = store.reconciliation_guard("https://cloud.restless.test", "user-2", company_id);
        let _other_held = tokio::time::timeout(Duration::from_secs(1), other.lock())
            .await
            .expect("a different principal has an independent guard");
        drop(held);
        tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .expect("same-principal waiter resumes")
            .expect("waiter task succeeds");
    }

    #[test]
    fn membership_revocation_targets_only_covered_sessions() {
        let store = SessionStore::default();
        let company_id = Uuid::new_v4();
        let identity = |issuer: &str, membership: &str, version: i64| VerifiedIdentity {
            user: format!("user-{membership}"),
            issuer: Some(issuer.into()),
            owner: "owner".into(),
            scope: CompanyScope::Company {
                company: "aris".into(),
            },
            role: "member".into(),
            actor: Some(format!("actor-{membership}-{version}")),
            company_id: Some(company_id),
            cell_id: Some(Uuid::new_v4()),
            membership_id: Some(membership.into()),
            membership_version: Some(version),
        };
        let covered = store.establish(
            identity("https://cloud.restless.test", "membership-1", 4),
            Duration::from_secs(60),
        );
        let covered_cancellation = store
            .resolve_lease(&covered)
            .expect("covered session lease")
            .cancellation;
        let newer = store.establish(
            identity("https://cloud.restless.test", "membership-1", 6),
            Duration::from_secs(60),
        );
        let other_membership = store.establish(
            identity("https://cloud.restless.test", "membership-2", 4),
            Duration::from_secs(60),
        );
        let other_issuer = store.establish(
            identity("https://another.restless.test", "membership-1", 4),
            Duration::from_secs(60),
        );

        assert_eq!(
            store.revoke_membership_through(
                "https://cloud.restless.test",
                company_id,
                "membership-1",
                5,
            ),
            1
        );
        assert!(store.resolve_lease(&covered).is_none());
        assert!(covered_cancellation.is_cancelled());
        assert!(store.resolve_lease(&newer).is_some());
        assert!(store.resolve_lease(&other_membership).is_some());
        assert!(store.resolve_lease(&other_issuer).is_some());
    }
}
