//! Owner entry modes for the account plane.
//!
//! The plane runs in one of two modes. **Local** is the historical posture from
//! ADR 0001: every entry point binds loopback and the local operator is the
//! `owner` principal. **Network** is ADR 0007: access is decided by verifying a
//! signed identity assertion, never by the network a connection arrived from.
//!
//! Both modes resolve to the same stable `owner` principal and run the same
//! application and Authority operations. Authentication only proves who may
//! assume that principal; it grants no Authority capability.
//!
//! This is deliberately separate from `capability.rs`. That module mints
//! internal cell-to-plane capabilities the plane itself issues. An entry
//! assertion is issued by a *different* party (Restless Cloud) about a *human*,
//! and is consumed once at the door.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use hmac::{Hmac, Mac as _};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Bumped only through the release contract's change governance. Cloud pins
/// this value from the release manifest.
pub(crate) const ASSERTION_CONTRACT_VERSION: u32 = 1;

const TOKEN_TYPE: &str = "restless-entry";
const ALGORITHM: &str = "HS256";

/// How long a plane session lives once an assertion has been exchanged for it.
/// The assertion itself is far shorter-lived; this is the browser session.
const DEFAULT_SESSION_TTL: Duration = Duration::from_secs(12 * 60 * 60);

/// Every distinct way an assertion can be refused.
///
/// This is an enum rather than one error string because a verifier that
/// collapses every failure into "invalid" is indistinguishable, from outside,
/// from a verifier whose signature check silently never runs.
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
    WrongPlane,
    Replayed,
}

impl Refusal {
    /// A stable machine-readable reason. Distinct per variant on purpose: the
    /// adversarial suite asserts ten inputs produce ten of these.
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
            Refusal::WrongPlane => "assertion_wrong_plane",
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
            Refusal::WrongAudience => "entry assertion was not minted for this plane".into(),
            Refusal::UnknownKeyVersion => "entry assertion names an unknown key version".into(),
            Refusal::BadSignature => "entry assertion signature is invalid".into(),
            Refusal::NotYetValid => "entry assertion is not valid yet".into(),
            Refusal::Expired => "entry assertion has expired".into(),
            Refusal::WrongPlane => "entry assertion is routed to a different plane".into(),
            Refusal::Replayed => "entry assertion has already been used".into(),
        }
    }
}

/// Which companies on this plane the bearer may reach.
///
/// The owner reaches every company on their plane. An invited human reaches
/// exactly one. There is deliberately no "several companies" variant: a human
/// with access to two companies holds two memberships and enters twice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum CompanyScope {
    /// Every company on this plane.
    Owner,
    /// Exactly one company.
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

/// The wire claims. Field names are the contract; changing one is a contract
/// version change under the release contract's governance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AssertionClaims {
    /// Assertion contract version.
    pub ver: u32,
    /// Issuer, matched against this plane's configured issuer.
    pub iss: String,
    /// Audience: this exact plane.
    pub aud: String,
    /// The plane this assertion routes to.
    pub plane: String,
    /// Stable owner identity that owns the plane.
    pub owner: String,
    /// Stable human user identity.
    pub sub: String,
    /// Which companies the bearer may reach.
    pub scope: CompanyScope,
    /// Active membership role: owner, admin or member.
    pub role: String,
    /// Mapped company actor, where the domain requires attribution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    /// Issued-at, not-before and expiry, as seconds since the Unix epoch.
    pub iat: u64,
    pub nbf: u64,
    pub exp: u64,
    /// Single-use identity. An assertion is consumed at entry.
    pub jti: String,
    /// Correlation identity for tracing across the Cloud/Core boundary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AssertionHeader {
    alg: String,
    typ: String,
    kid: String,
}

/// What the plane knows after verifying, and the only thing downstream code
/// may consult. Note there is no capability here: Authority is separate.
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
    /// Test-only. Exists so the adversarial suite can prove that the signature
    /// check is what rejects a tampered assertion, rather than some incidental
    /// later check that would pass a broken verifier.
    #[cfg(test)]
    Skip,
}

/// Network-mode configuration. Every field is required; a plane that cannot
/// fully describe how it verifies refuses to start rather than falling back to
/// trusting the network.
pub(crate) struct NetworkEntry {
    issuer: String,
    audience: String,
    plane: String,
    /// Public hostname this plane is reached at, used for browser-origin checks.
    host: String,
    /// Key version to shared secret. Several may be live during rotation.
    keys: HashMap<String, Vec<u8>>,
    session_ttl: Duration,
    /// Consumed single-use identities, retained until the assertion that
    /// carried them would have expired anyway.
    consumed: Mutex<HashMap<String, SystemTime>>,
}

impl NetworkEntry {
    pub(crate) fn host(&self) -> &str {
        &self.host
    }

    pub(crate) fn session_ttl(&self) -> Duration {
        self.session_ttl
    }

    /// Verify an assertion and consume its single-use identity.
    ///
    /// Order is deliberate. The signature is checked before any claim is acted
    /// on, so a tampered payload can never steer verification. Claims that
    /// select the key (`kid`) and the contract shape (`ver`) must be read
    /// first, which is why they precede it.
    pub(crate) fn verify(&self, token: &str) -> Result<VerifiedIdentity, Refusal> {
        self.verify_at(token, SystemTime::now(), SignaturePolicy::Enforce)
    }

    fn verify_at(
        &self,
        token: &str,
        now: SystemTime,
        policy: SignaturePolicy,
    ) -> Result<VerifiedIdentity, Refusal> {
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
        if claims.ver != ASSERTION_CONTRACT_VERSION {
            return Err(Refusal::UnsupportedVersion {
                got: claims.ver,
                supported: ASSERTION_CONTRACT_VERSION,
            });
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
            let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(signature_b64)
                .map_err(|_| Refusal::Malformed("signature is not base64url"))?;
            let signed = format!("{header_b64}.{payload_b64}");
            let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
            mac.update(signed.as_bytes());
            mac.verify_slice(&signature)
                .map_err(|_| Refusal::BadSignature)?;
        }

        if claims.iss != self.issuer {
            return Err(Refusal::UnknownIssuer);
        }
        if claims.aud != self.audience {
            return Err(Refusal::WrongAudience);
        }
        if claims.plane != self.plane {
            return Err(Refusal::WrongPlane);
        }

        let now_secs = unix_seconds(now);
        if claims.nbf > now_secs {
            return Err(Refusal::NotYetValid);
        }
        if claims.exp <= now_secs {
            return Err(Refusal::Expired);
        }

        // Consumed last, so a refusal never burns a legitimate identity.
        self.consume(&claims.jti, now, claims.exp)?;

        Ok(VerifiedIdentity {
            user: claims.sub,
            owner: claims.owner,
            scope: claims.scope,
            role: claims.role,
            actor: claims.actor,
            correlation: claims.cid,
        })
    }

    fn consume(&self, jti: &str, now: SystemTime, exp: u64) -> Result<(), Refusal> {
        let mut consumed = self.consumed.lock().expect("entry replay store poisoned");
        consumed.retain(|_, expires| *expires > now);
        if consumed.contains_key(jti) {
            return Err(Refusal::Replayed);
        }
        consumed.insert(
            jti.to_string(),
            UNIX_EPOCH + Duration::from_secs(exp),
        );
        Ok(())
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
        "header" => Refusal::Malformed("header is not valid JSON"),
        _ => Refusal::Malformed("payload is not valid JSON"),
    })
}

fn unix_seconds(at: SystemTime) -> u64 {
    at.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Mint an assertion. Restless Cloud is the real issuer; this exists so the
/// plane's own tests and the end-to-end run have a test issuer, and so the
/// wire format has exactly one definition rather than two that drift.
pub(crate) fn mint(claims: &AssertionClaims, key_version: &str, key: &[u8]) -> String {
    let header = AssertionHeader {
        alg: ALGORITHM.to_string(),
        typ: TOKEN_TYPE.to_string(),
        kid: key_version.to_string(),
    };
    let header_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&header).expect("header encodes"));
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(claims).expect("claims encode"));
    let signed = format!("{header_b64}.{payload_b64}");
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(signed.as_bytes());
    let signature =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    format!("{signed}.{signature}")
}

/// How this plane decides who may enter.
#[derive(Clone)]
pub(crate) enum EntryMode {
    /// ADR 0001: loopback only, local operator is the owner principal.
    Local,
    /// ADR 0007: access decided by verifying a signed assertion.
    Network(std::sync::Arc<NetworkEntry>),
}

impl EntryMode {
    /// Read the mode from the environment.
    ///
    /// Network mode requires a complete description of how verification works.
    /// A missing field is a startup failure naming the field — never a silent
    /// downgrade to trusting the network, which is the failure this whole
    /// module exists to prevent.
    pub(crate) fn from_env() -> anyhow::Result<Self> {
        let mode = std::env::var("RESTLESS_ENTRY_MODE").unwrap_or_else(|_| "local".to_string());
        match mode.as_str() {
            "local" => Ok(EntryMode::Local),
            "network" => {
                let issuer = required("RESTLESS_ENTRY_ISSUER")?;
                let audience = required("RESTLESS_ENTRY_AUDIENCE")?;
                let plane = required("RESTLESS_ENTRY_PLANE")?;
                let host = required("RESTLESS_ENTRY_HOST")?;
                let keys = parse_keys(&required("RESTLESS_ENTRY_KEYS")?)?;
                let session_ttl = match std::env::var("RESTLESS_ENTRY_SESSION_TTL_SECONDS") {
                    Ok(value) => Duration::from_secs(value.parse().map_err(|_| {
                        anyhow::anyhow!("RESTLESS_ENTRY_SESSION_TTL_SECONDS must be seconds")
                    })?),
                    Err(_) => DEFAULT_SESSION_TTL,
                };
                Ok(EntryMode::Network(std::sync::Arc::new(NetworkEntry {
                    issuer,
                    audience,
                    plane,
                    host,
                    keys,
                    session_ttl,
                    consumed: Mutex::new(HashMap::new()),
                })))
            }
            other => anyhow::bail!(
                "RESTLESS_ENTRY_MODE must be `local` or `network`, not `{other}`"
            ),
        }
    }

    pub(crate) fn network(&self) -> Option<&std::sync::Arc<NetworkEntry>> {
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
            "network entry mode requires {variable}; refusing to start rather than \
             accepting requests on network position alone"
        );
    }
    Ok(value)
}

/// `v1:<base64url secret>,v2:<base64url secret>` — several live at once so a
/// key can be rotated without a window where no assertion verifies.
fn parse_keys(raw: &str) -> anyhow::Result<HashMap<String, Vec<u8>>> {
    let mut keys = HashMap::new();
    for entry in raw.split(',').map(str::trim).filter(|e| !e.is_empty()) {
        let (version, secret) = entry
            .split_once(':')
            .context_msg("RESTLESS_ENTRY_KEYS entries must be `<version>:<base64url secret>`")?;
        let secret = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(secret.trim())
            .map_err(|_| anyhow::anyhow!("RESTLESS_ENTRY_KEYS secret is not base64url"))?;
        if secret.len() < 32 {
            anyhow::bail!("RESTLESS_ENTRY_KEYS secrets must be at least 32 bytes");
        }
        keys.insert(version.trim().to_string(), secret);
    }
    if keys.is_empty() {
        anyhow::bail!("RESTLESS_ENTRY_KEYS names no usable key version");
    }
    Ok(keys)
}

trait ContextMsg<T> {
    fn context_msg(self, message: &'static str) -> anyhow::Result<T>;
}

impl<T> ContextMsg<T> for Option<T> {
    fn context_msg(self, message: &'static str) -> anyhow::Result<T> {
        self.ok_or_else(|| anyhow::anyhow!(message))
    }
}

/// Browser sessions established after an assertion is consumed.
///
/// The assertion is the door; this is the room. A replayed assertion cannot
/// create a second session because the assertion is consumed before a session
/// is minted.
#[derive(Default)]
pub(crate) struct SessionStore {
    sessions: Mutex<HashMap<String, (VerifiedIdentity, SystemTime)>>,
}

impl SessionStore {
    pub(crate) fn establish(&self, identity: VerifiedIdentity, ttl: Duration) -> String {
        let token = uuid::Uuid::new_v4().to_string();
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
///
/// Two call sites that each decide scope is how one of them ends up deciding
/// it differently, so there is exactly one.
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

/// The test issuer.
///
/// Restless Cloud is the real issuer. This exists so the plane's own end-to-end
/// run (S27-T5) can mint an assertion against the same wire format the verifier
/// reads, rather than a second implementation that drifts from it. It is not a
/// Core identity product and must not grow into one.
pub(crate) fn mint_from_env() -> anyhow::Result<String> {
    let issuer = required("RESTLESS_ENTRY_ISSUER")?;
    let audience = required("RESTLESS_ENTRY_AUDIENCE")?;
    let plane = required("RESTLESS_ENTRY_PLANE")?;
    let keys = parse_keys(&required("RESTLESS_ENTRY_KEYS")?)?;
    let (key_version, key) = keys
        .iter()
        .next()
        .expect("parse_keys refuses an empty set");

    let scope = match std::env::var("RESTLESS_ENTRY_TEST_COMPANY") {
        Ok(company) if !company.trim().is_empty() => CompanyScope::Company {
            company: company.trim().to_string(),
        },
        _ => CompanyScope::Owner,
    };
    let ttl: u64 = std::env::var("RESTLESS_ENTRY_TEST_TTL_SECONDS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(120);
    let now = unix_seconds(SystemTime::now());

    let claims = AssertionClaims {
        ver: ASSERTION_CONTRACT_VERSION,
        iss: issuer,
        aud: audience,
        plane,
        owner: std::env::var("RESTLESS_ENTRY_TEST_OWNER").unwrap_or_else(|_| "owner".into()),
        sub: std::env::var("RESTLESS_ENTRY_TEST_USER").unwrap_or_else(|_| "test-user".into()),
        scope,
        role: std::env::var("RESTLESS_ENTRY_TEST_ROLE").unwrap_or_else(|_| "owner".into()),
        actor: Some("owner".into()),
        iat: now,
        nbf: now,
        exp: now + ttl,
        jti: uuid::Uuid::new_v4().to_string(),
        cid: None,
    };
    Ok(mint(&claims, key_version, key))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY_VERSION: &str = "v1";

    fn key() -> Vec<u8> {
        vec![7u8; 32]
    }

    fn plane() -> NetworkEntry {
        let mut keys = HashMap::new();
        keys.insert(KEY_VERSION.to_string(), key());
        NetworkEntry {
            issuer: "https://cloud.restless.test".into(),
            audience: "plane-aris".into(),
            plane: "plane-aris".into(),
            host: "aris.restless.test".into(),
            keys,
            session_ttl: DEFAULT_SESSION_TTL,
            consumed: Mutex::new(HashMap::new()),
        }
    }

    fn at(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    /// A claim set that verifies cleanly at t=1000.
    fn claims() -> AssertionClaims {
        AssertionClaims {
            ver: ASSERTION_CONTRACT_VERSION,
            iss: "https://cloud.restless.test".into(),
            aud: "plane-aris".into(),
            plane: "plane-aris".into(),
            owner: "owner-1".into(),
            sub: "user-1".into(),
            scope: CompanyScope::Owner,
            role: "owner".into(),
            actor: Some("owner".into()),
            iat: 900,
            nbf: 900,
            exp: 1200,
            jti: "assertion-1".into(),
            cid: Some("corr-1".into()),
        }
    }

    fn token(claims: &AssertionClaims) -> String {
        mint(claims, KEY_VERSION, &key())
    }

    fn refuse(entry: &NetworkEntry, claims: &AssertionClaims) -> Refusal {
        entry
            .verify_at(&token(claims), at(1000), SignaturePolicy::Enforce)
            .expect_err("assertion should have been refused")
    }

    #[test]
    fn a_valid_assertion_is_accepted_once() {
        let entry = plane();
        let identity = entry
            .verify_at(&token(&claims()), at(1000), SignaturePolicy::Enforce)
            .expect("valid assertion");
        assert_eq!(identity.user, "user-1");
        assert_eq!(identity.owner, "owner-1");
        assert_eq!(identity.scope, CompanyScope::Owner);
        assert_eq!(identity.correlation.as_deref(), Some("corr-1"));
    }

    #[test]
    fn expired_is_refused() {
        let entry = plane();
        let refusal = entry
            .verify_at(&token(&claims()), at(1300), SignaturePolicy::Enforce)
            .expect_err("expired");
        assert_eq!(refusal, Refusal::Expired);
    }

    #[test]
    fn not_yet_valid_is_refused() {
        let entry = plane();
        let refusal = entry
            .verify_at(&token(&claims()), at(800), SignaturePolicy::Enforce)
            .expect_err("not yet valid");
        assert_eq!(refusal, Refusal::NotYetValid);
    }

    #[test]
    fn wrong_audience_is_refused() {
        let mut claims = claims();
        claims.aud = "plane-someone-else".into();
        assert_eq!(refuse(&plane(), &claims), Refusal::WrongAudience);
    }

    #[test]
    fn unknown_issuer_is_refused() {
        let mut claims = claims();
        claims.iss = "https://not-our-cloud.test".into();
        assert_eq!(refuse(&plane(), &claims), Refusal::UnknownIssuer);
    }

    #[test]
    fn wrong_plane_is_refused() {
        let mut claims = claims();
        claims.plane = "plane-other".into();
        assert_eq!(refuse(&plane(), &claims), Refusal::WrongPlane);
    }

    #[test]
    fn unsupported_contract_version_is_refused() {
        let mut claims = claims();
        claims.ver = ASSERTION_CONTRACT_VERSION + 1;
        assert!(matches!(
            refuse(&plane(), &claims),
            Refusal::UnsupportedVersion { .. }
        ));
    }

    #[test]
    fn unknown_key_version_is_refused() {
        let entry = plane();
        let token = mint(&claims(), "v-unknown", &key());
        let refusal = entry
            .verify_at(&token, at(1000), SignaturePolicy::Enforce)
            .expect_err("unknown key version");
        assert_eq!(refusal, Refusal::UnknownKeyVersion);
    }

    #[test]
    fn a_tampered_signature_is_refused() {
        let entry = plane();
        // Minted with a key this plane does not hold, under a key version it does.
        let token = mint(&claims(), KEY_VERSION, &[9u8; 32]);
        let refusal = entry
            .verify_at(&token, at(1000), SignaturePolicy::Enforce)
            .expect_err("bad signature");
        assert_eq!(refusal, Refusal::BadSignature);
    }

    #[test]
    fn a_replayed_assertion_is_refused() {
        let entry = plane();
        let token = token(&claims());
        entry
            .verify_at(&token, at(1000), SignaturePolicy::Enforce)
            .expect("first use succeeds");
        let refusal = entry
            .verify_at(&token, at(1001), SignaturePolicy::Enforce)
            .expect_err("second use refused");
        assert_eq!(refusal, Refusal::Replayed);
    }

    #[test]
    fn a_malformed_assertion_is_refused() {
        let entry = plane();
        for bad in ["", "one.two", "a.b.c.d", "not-base64!.x.y"] {
            let refusal = entry
                .verify_at(bad, at(1000), SignaturePolicy::Enforce)
                .expect_err("malformed");
            assert!(
                matches!(refusal, Refusal::Malformed(_)),
                "{bad:?} produced {refusal:?}"
            );
        }
    }

    /// Every refusal reason is distinct. A verifier that collapses failures
    /// into one reason passes every test above while proving nothing.
    #[test]
    fn every_refusal_reason_is_distinct() {
        let reasons = [
            Refusal::Malformed("x").code(),
            Refusal::UnsupportedVersion { got: 2, supported: 1 }.code(),
            Refusal::UnknownIssuer.code(),
            Refusal::WrongAudience.code(),
            Refusal::UnknownKeyVersion.code(),
            Refusal::BadSignature.code(),
            Refusal::NotYetValid.code(),
            Refusal::Expired.code(),
            Refusal::WrongPlane.code(),
            Refusal::Replayed.code(),
        ];
        let unique: std::collections::BTreeSet<_> = reasons.iter().collect();
        assert_eq!(unique.len(), reasons.len(), "refusal reasons collide");
    }

    /// The inverse check S27-T3 requires.
    ///
    /// A suite that still passes with signature verification disabled is
    /// testing nothing. This proves the signature check is what rejects a
    /// forged assertion: with the check skipped, the same forged token passes
    /// every other gate and verifies. If a later refactor made some incidental
    /// check reject it instead, this test fails and says so.
    #[test]
    fn the_signature_check_is_what_rejects_a_forgery() {
        let entry = plane();
        let forged = mint(&claims(), KEY_VERSION, &[9u8; 32]);

        assert_eq!(
            entry
                .verify_at(&forged, at(1000), SignaturePolicy::Enforce)
                .expect_err("enforced"),
            Refusal::BadSignature
        );

        let accepted = entry
            .verify_at(&forged, at(1000), SignaturePolicy::Skip)
            .expect("with the signature check skipped, nothing else rejects a forgery");
        assert_eq!(accepted.user, "user-1");
    }

    #[test]
    fn company_scope_permits_only_its_own_company() {
        let owner = CompanyScope::Owner;
        assert!(owner.permits("aris"));
        assert!(owner.permits("anything"));

        let one = CompanyScope::Company {
            company: "aris".into(),
        };
        assert!(one.permits("aris"));
        assert!(!one.permits("other"));
        assert!(!one.permits("aris-2"));
        assert!(!one.permits(""));
    }

    #[test]
    fn company_is_derived_from_the_path_in_one_place() {
        assert_eq!(company_in_path("/api/companies/aris/cockpit"), Some("aris"));
        assert_eq!(company_in_path("/api/companies/aris"), Some("aris"));
        assert_eq!(company_in_path("/desktop/aris/observe"), Some("aris"));
        assert_eq!(company_in_path("/desktop/aris"), Some("aris"));
        assert_eq!(company_in_path("/api/companies/"), None);
        assert_eq!(company_in_path("/api/health"), None);
        assert_eq!(company_in_path("/"), None);
    }

    #[test]
    fn keys_must_be_versioned_and_long_enough() {
        let good = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([1u8; 32]);
        let parsed = parse_keys(&format!("v1:{good}")).expect("valid key");
        assert_eq!(parsed.len(), 1);

        let short = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([1u8; 8]);
        assert!(parse_keys(&format!("v1:{short}")).is_err());
        assert!(parse_keys("v1").is_err());
        assert!(parse_keys("").is_err());
    }

    #[test]
    fn a_session_is_single_use_at_the_door_but_reusable_after() {
        let store = SessionStore::default();
        let identity = VerifiedIdentity {
            user: "user-1".into(),
            owner: "owner-1".into(),
            scope: CompanyScope::Owner,
            role: "owner".into(),
            actor: None,
            correlation: None,
        };
        let token = store.establish(identity, Duration::from_secs(60));
        assert!(store.resolve(&token).is_some());
        assert!(store.resolve(&token).is_some());
        store.revoke(&token);
        assert!(store.resolve(&token).is_none());
    }
}
