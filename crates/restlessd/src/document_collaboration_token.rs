//! Short-lived authority for the native Documents collaboration sidecar.
//!
//! This signer is deliberately independent of both Fleet entry assertions and
//! Runtime capability grants. The sidecar receives only the public JWKS and a
//! document-scoped token; it can never mint either of those other authorities.

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::Path;
use std::sync::Arc;

use anyhow::{bail, Context as _, Result};
use base64::Engine as _;
#[cfg(test)]
use chrono::Duration;
use chrono::{DateTime, Utc};
#[cfg(test)]
use ed25519_dalek::{Signature, Verifier as _};
use ed25519_dalek::{Signer as _, SigningKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use url::{Host, Url};
use uuid::Uuid;

const KEY_FILE: &str = "native-documents-collaboration.ed25519";
const TOKEN_TYPE: &str = "restless-native-documents-collaboration+jwt";
const ALGORITHM: &str = "EdDSA";
pub(crate) const AUDIENCE: &str = "restless-native-documents-collaboration";
pub(crate) const DEFAULT_TTL_SECONDS: i64 = 60;
pub(crate) const MINIMUM_TTL_SECONDS: i64 = 15;
pub(crate) const MAXIMUM_TTL_SECONDS: i64 = 120;

#[derive(Clone)]
pub(crate) struct DocumentCollaborationTokenIssuer {
    key: Arc<SigningKey>,
    key_id: Arc<str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DocumentCollaborationAccess {
    Read,
    Write,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DocumentCollaborationClaims {
    pub(crate) iss: String,
    pub(crate) aud: String,
    pub(crate) sub: String,
    pub(crate) company_id: Uuid,
    pub(crate) document_id: Uuid,
    pub(crate) actor_id: String,
    pub(crate) access: DocumentCollaborationAccess,
    pub(crate) iat: i64,
    pub(crate) nbf: i64,
    pub(crate) exp: i64,
    pub(crate) jti: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TokenHeader {
    alg: String,
    typ: String,
    kid: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DocumentCollaborationJwkSet {
    keys: [DocumentCollaborationJwk; 1],
}

#[derive(Debug, Clone, Serialize)]
struct DocumentCollaborationJwk {
    kty: &'static str,
    crv: &'static str,
    alg: &'static str,
    #[serde(rename = "use")]
    use_: &'static str,
    kid: String,
    x: String,
}

pub(crate) struct DocumentCollaborationTokenInput<'a> {
    pub(crate) issuer: &'a str,
    pub(crate) session_principal: &'a str,
    pub(crate) company_id: Uuid,
    pub(crate) document_id: Uuid,
    pub(crate) actor_id: &'a str,
    pub(crate) access: DocumentCollaborationAccess,
    pub(crate) now: DateTime<Utc>,
    pub(crate) ttl_seconds: i64,
}

#[cfg(test)]
pub(crate) struct ExpectedDocumentCollaborationScope<'a> {
    pub(crate) issuer: &'a str,
    pub(crate) session_principal: &'a str,
    pub(crate) company_id: Uuid,
    pub(crate) document_id: Uuid,
    pub(crate) actor_id: &'a str,
    pub(crate) access: DocumentCollaborationAccess,
}

impl DocumentCollaborationTokenIssuer {
    pub(crate) fn open(root: &Path) -> Result<Self> {
        let path = root.join(KEY_FILE);
        let seed = match read_key(&path) {
            Ok(seed) => seed,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let seed = fresh_seed();
                match create_key(&path, &seed) {
                    Ok(()) => seed.to_vec(),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                        read_key(&path).with_context(|| {
                            format!(
                                "read concurrent native Documents signing key {}",
                                path.display()
                            )
                        })?
                    }
                    Err(error) => {
                        return Err(error).with_context(|| {
                            format!("create native Documents signing key {}", path.display())
                        })
                    }
                }
            }
            Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
        };
        let seed: [u8; 32] = seed.try_into().map_err(|seed: Vec<u8>| {
            anyhow::anyhow!(
                "native Documents signing key {} has {} bytes; expected 32",
                path.display(),
                seed.len()
            )
        })?;
        Ok(Self::from_signing_key(SigningKey::from_bytes(&seed)))
    }

    fn from_signing_key(key: SigningKey) -> Self {
        let fingerprint = format!("{:x}", Sha256::digest(key.verifying_key().as_bytes()));
        Self {
            key: Arc::new(key),
            key_id: format!("native-documents-{}", &fingerprint[..24]).into(),
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test(seed: [u8; 32]) -> Self {
        Self::from_signing_key(SigningKey::from_bytes(&seed))
    }

    pub(crate) fn key_id(&self) -> &str {
        &self.key_id
    }

    pub(crate) fn jwks(&self) -> DocumentCollaborationJwkSet {
        DocumentCollaborationJwkSet {
            keys: [DocumentCollaborationJwk {
                kty: "OKP",
                crv: "Ed25519",
                alg: ALGORITHM,
                use_: "sig",
                kid: self.key_id().to_string(),
                x: base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .encode(self.key.verifying_key().as_bytes()),
            }],
        }
    }

    pub(crate) fn issue(&self, input: DocumentCollaborationTokenInput<'_>) -> Result<String> {
        validate_issuer(input.issuer)?;
        validate_identifier("session principal", input.session_principal, 255)?;
        validate_identifier("Actor", input.actor_id, 200)?;
        if input.company_id.is_nil() || input.document_id.is_nil() {
            bail!("native Documents collaboration scope requires non-nil company and document ids");
        }
        if !(MINIMUM_TTL_SECONDS..=MAXIMUM_TTL_SECONDS).contains(&input.ttl_seconds) {
            bail!(
                "native Documents collaboration token lifetime must be between {MINIMUM_TTL_SECONDS} and {MAXIMUM_TTL_SECONDS} seconds"
            );
        }
        let issued_at = input.now.timestamp();
        if issued_at <= 0 {
            bail!("native Documents collaboration token issued-at must be positive");
        }
        let claims = DocumentCollaborationClaims {
            iss: input.issuer.to_string(),
            aud: AUDIENCE.to_string(),
            sub: input.session_principal.to_string(),
            company_id: input.company_id,
            document_id: input.document_id,
            actor_id: input.actor_id.to_string(),
            access: input.access,
            iat: issued_at,
            nbf: issued_at,
            exp: issued_at + input.ttl_seconds,
            jti: Uuid::new_v4(),
        };
        self.mint(&claims)
    }

    fn mint(&self, claims: &DocumentCollaborationClaims) -> Result<String> {
        let header = TokenHeader {
            alg: ALGORITHM.to_string(),
            typ: TOKEN_TYPE.to_string(),
            kid: self.key_id().to_string(),
        };
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&header).context("encode native Documents token header")?);
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(claims).context("encode native Documents token claims")?);
        let signing_input = format!("{header}.{payload}");
        let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(self.key.sign(signing_input.as_bytes()).to_bytes());
        Ok(format!("{signing_input}.{signature}"))
    }

    #[cfg(test)]
    pub(crate) fn verify_at(
        &self,
        token: &str,
        expected: ExpectedDocumentCollaborationScope<'_>,
        now: DateTime<Utc>,
    ) -> Result<DocumentCollaborationClaims> {
        if token.len() > 4096 {
            bail!("native Documents collaboration token is too large");
        }
        let mut parts = token.split('.');
        let header = parts.next().context("token has no header")?;
        let payload = parts.next().context("token has no claims")?;
        let signature = parts.next().context("token has no signature")?;
        if parts.next().is_some() || header.is_empty() || payload.is_empty() || signature.is_empty()
        {
            bail!("native Documents collaboration token is malformed");
        }
        let decoded_header = decode_segment::<TokenHeader>(header, "header")?;
        if decoded_header.alg != ALGORITHM
            || decoded_header.typ != TOKEN_TYPE
            || decoded_header.kid != self.key_id()
        {
            bail!("native Documents collaboration token header is invalid");
        }
        let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(signature)
            .context("native Documents collaboration signature is not base64url")?;
        let signature = Signature::from_slice(&signature).map_err(|_| {
            anyhow::anyhow!("native Documents collaboration signature is not Ed25519")
        })?;
        self.key
            .verifying_key()
            .verify(format!("{header}.{payload}").as_bytes(), &signature)
            .map_err(|_| anyhow::anyhow!("native Documents collaboration signature is invalid"))?;
        let claims = decode_segment::<DocumentCollaborationClaims>(payload, "claims")?;
        validate_claims(&claims, &expected, now)?;
        Ok(claims)
    }
}

fn fresh_seed() -> [u8; 32] {
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    let mut hasher = Sha256::new();
    hasher.update(first.as_bytes());
    hasher.update(second.as_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
fn decode_segment<T: for<'de> Deserialize<'de>>(segment: &str, name: &str) -> Result<T> {
    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(segment)
        .with_context(|| format!("native Documents collaboration {name} is not base64url"))?;
    serde_json::from_slice(&raw)
        .with_context(|| format!("native Documents collaboration {name} is not valid JSON"))
}

#[cfg(test)]
fn validate_claims(
    claims: &DocumentCollaborationClaims,
    expected: &ExpectedDocumentCollaborationScope<'_>,
    now: DateTime<Utc>,
) -> Result<()> {
    validate_issuer(&claims.iss)?;
    validate_identifier("session principal", &claims.sub, 255)?;
    validate_identifier("Actor", &claims.actor_id, 200)?;
    if claims.aud != AUDIENCE
        || claims.iss != expected.issuer
        || claims.sub != expected.session_principal
        || claims.company_id != expected.company_id
        || claims.document_id != expected.document_id
        || claims.actor_id != expected.actor_id
        || claims.access != expected.access
        || claims.company_id.is_nil()
        || claims.document_id.is_nil()
        || claims.jti.is_nil()
    {
        bail!("native Documents collaboration token scope is invalid");
    }
    let now = now.timestamp();
    let lifetime = claims.exp.saturating_sub(claims.iat);
    if claims.iat <= 0
        || claims.nbf != claims.iat
        || !(MINIMUM_TTL_SECONDS..=MAXIMUM_TTL_SECONDS).contains(&lifetime)
        || claims.iat > now
        || claims.nbf > now
        || claims.exp <= now
    {
        bail!("native Documents collaboration token time bounds are invalid");
    }
    Ok(())
}

fn validate_issuer(value: &str) -> Result<()> {
    let url = Url::parse(value).context("parse native Documents collaboration issuer")?;
    let loopback_host = match url.host() {
        Some(Host::Domain("localhost")) => true,
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        _ => false,
    };
    let loopback_http = url.scheme() == "http" && loopback_host;
    if (url.scheme() != "https" && !loopback_http)
        || url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || value.ends_with('/')
    {
        bail!("native Documents collaboration issuer must be an HTTPS origin or bounded loopback HTTP origin");
    }
    Ok(())
}

fn validate_identifier(label: &str, value: &str, maximum: usize) -> Result<()> {
    if value.trim().is_empty() || value.len() > maximum || value.chars().any(char::is_control) {
        bail!("native Documents collaboration {label} is invalid");
    }
    Ok(())
}

fn create_key(path: &Path, seed: &[u8; 32]) -> std::io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(seed)?;
    file.sync_all()?;
    Ok(())
}

fn read_key(path: &Path) -> std::io::Result<Vec<u8>> {
    match fs::symlink_metadata(path) {
        Ok(_) => {}
        Err(error) => return Err(error),
    }
    ensure_private_regular_file(path).map_err(|error| {
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, error.to_string())
    })?;
    fs::read(path)
}

fn ensure_private_regular_file(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("inspect native Documents signing key {}", path.display()))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        bail!(
            "native Documents signing key {} must be a regular file",
            path.display()
        );
    }
    if metadata.len() != 32 {
        bail!(
            "native Documents signing key {} has {} bytes; expected 32",
            path.display(),
            metadata.len()
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        if metadata.mode() & 0o077 != 0 {
            bail!(
                "native Documents signing key {} must not be readable by group or others",
                path.display()
            );
        }
        if metadata.nlink() != 1 {
            bail!(
                "native Documents signing key {} must not have hard links",
                path.display()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instant() -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).unwrap()
    }

    fn scope<'a>() -> ExpectedDocumentCollaborationScope<'a> {
        ExpectedDocumentCollaborationScope {
            issuer: "https://plane.restless.test",
            session_principal: "session-partition",
            company_id: Uuid::parse_str("11111111-1111-7111-8111-111111111111").unwrap(),
            document_id: Uuid::parse_str("22222222-2222-7222-8222-222222222222").unwrap(),
            actor_id: "owner",
            access: DocumentCollaborationAccess::Write,
        }
    }

    fn token(issuer: &DocumentCollaborationTokenIssuer) -> String {
        let expected = scope();
        issuer
            .issue(DocumentCollaborationTokenInput {
                issuer: expected.issuer,
                session_principal: expected.session_principal,
                company_id: expected.company_id,
                document_id: expected.document_id,
                actor_id: expected.actor_id,
                access: expected.access,
                now: instant(),
                ttl_seconds: DEFAULT_TTL_SECONDS,
            })
            .unwrap()
    }

    #[test]
    fn key_is_persistent_private_and_separate_from_runtime_authority() {
        let root = std::env::temp_dir().join(format!(
            "restless-native-documents-token-{}",
            Uuid::new_v4().simple()
        ));
        fs::create_dir(&root).unwrap();
        fs::write(root.join("runtime-capability.key"), [7_u8; 32]).unwrap();
        let first = DocumentCollaborationTokenIssuer::open(&root).unwrap();
        let second = DocumentCollaborationTokenIssuer::open(&root).unwrap();
        assert_eq!(first.key_id(), second.key_id());
        assert_ne!(fs::read(root.join(KEY_FILE)).unwrap(), vec![7_u8; 32]);
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            assert_eq!(fs::metadata(root.join(KEY_FILE)).unwrap().mode() & 0o077, 0);
        }
        fs::remove_file(root.join(KEY_FILE)).unwrap();
        fs::remove_file(root.join("runtime-capability.key")).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn refuses_exposed_or_indirect_private_key_files() {
        use std::os::unix::fs::{symlink, PermissionsExt as _};

        let root = std::env::temp_dir().join(format!(
            "restless-native-documents-key-safety-{}",
            Uuid::new_v4().simple()
        ));
        fs::create_dir(&root).unwrap();
        let key_path = root.join(KEY_FILE);
        fs::write(&key_path, [17_u8; 32]).unwrap();
        fs::set_permissions(&key_path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(DocumentCollaborationTokenIssuer::open(&root).is_err());
        fs::remove_file(&key_path).unwrap();

        let target = root.join("target-key");
        fs::write(&target, [19_u8; 32]).unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
        symlink(&target, &key_path).unwrap();
        assert!(DocumentCollaborationTokenIssuer::open(&root).is_err());

        fs::remove_file(&key_path).unwrap();
        fs::remove_file(target).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn compact_token_binds_exact_scope_type_signature_and_time() {
        let issuer = DocumentCollaborationTokenIssuer::for_test([9; 32]);
        let token = token(&issuer);
        let claims = issuer.verify_at(&token, scope(), instant()).unwrap();
        assert_eq!(claims.aud, AUDIENCE);
        assert_eq!(claims.nbf, claims.iat);
        assert_eq!(claims.exp - claims.iat, DEFAULT_TTL_SECONDS);
        assert!(!claims.jti.is_nil());

        let mut wrong_document = scope();
        wrong_document.document_id = Uuid::new_v4();
        assert!(issuer.verify_at(&token, wrong_document, instant()).is_err());

        let mut wrong_company = scope();
        wrong_company.company_id = Uuid::new_v4();
        assert!(issuer.verify_at(&token, wrong_company, instant()).is_err());

        let mut wrong_access = scope();
        wrong_access.access = DocumentCollaborationAccess::Read;
        assert!(issuer.verify_at(&token, wrong_access, instant()).is_err());

        assert!(issuer
            .verify_at(
                &token,
                scope(),
                instant() + Duration::seconds(DEFAULT_TTL_SECONDS)
            )
            .is_err());

        let mut tampered = token.into_bytes();
        let last = tampered.len() - 1;
        tampered[last] = if tampered[last] == b'A' { b'B' } else { b'A' };
        assert!(issuer
            .verify_at(std::str::from_utf8(&tampered).unwrap(), scope(), instant())
            .is_err());
    }

    #[test]
    fn token_header_and_jwks_publish_only_the_expected_public_contract() {
        let issuer = DocumentCollaborationTokenIssuer::for_test([11; 32]);
        let token = token(&issuer);
        let header = token.split('.').next().unwrap();
        let header: serde_json::Value = serde_json::from_slice(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(header)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(header["alg"], ALGORITHM);
        assert_eq!(header["typ"], TOKEN_TYPE);
        assert_eq!(header["kid"], issuer.key_id());

        let jwks = serde_json::to_value(issuer.jwks()).unwrap();
        assert_eq!(jwks["keys"].as_array().unwrap().len(), 1);
        assert_eq!(jwks["keys"][0]["kty"], "OKP");
        assert_eq!(jwks["keys"][0]["crv"], "Ed25519");
        assert_eq!(jwks["keys"][0]["alg"], ALGORITHM);
        assert_eq!(jwks["keys"][0]["use"], "sig");
        assert_eq!(jwks["keys"][0]["kid"], issuer.key_id());
        assert!(jwks["keys"][0].get("x").is_some());
        assert!(jwks["keys"][0].get("d").is_none());
        assert_eq!(jwks["keys"][0].as_object().unwrap().len(), 6);
    }

    #[test]
    fn refuses_unsafe_issuers_and_out_of_contract_lifetimes() {
        let issuer = DocumentCollaborationTokenIssuer::for_test([13; 32]);
        let expected = scope();
        for unsafe_issuer in [
            "http://plane.restless.test",
            "https://plane.restless.test/path",
            "https://user@plane.restless.test",
        ] {
            assert!(issuer
                .issue(DocumentCollaborationTokenInput {
                    issuer: unsafe_issuer,
                    session_principal: expected.session_principal,
                    company_id: expected.company_id,
                    document_id: expected.document_id,
                    actor_id: expected.actor_id,
                    access: expected.access,
                    now: instant(),
                    ttl_seconds: DEFAULT_TTL_SECONDS,
                })
                .is_err());
        }
        for ttl_seconds in [MINIMUM_TTL_SECONDS - 1, MAXIMUM_TTL_SECONDS + 1] {
            assert!(issuer
                .issue(DocumentCollaborationTokenInput {
                    issuer: expected.issuer,
                    session_principal: expected.session_principal,
                    company_id: expected.company_id,
                    document_id: expected.document_id,
                    actor_id: expected.actor_id,
                    access: expected.access,
                    now: instant(),
                    ttl_seconds,
                })
                .is_err());
        }
        assert!(validate_issuer("http://127.0.0.1:7788").is_ok());
        assert!(validate_issuer("http://[::1]:7788").is_ok());
    }
}
