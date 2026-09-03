//! Small signed grants for the two host-to-Runtime boundaries.
//!
//! These are not user accounts or a policy engine. The local appliance has
//! exactly one human owner and one Company Runtime. A fixed, short-lived
//! signed claim lets the daemon derive the Runtime's company, actor and model
//! scope without trusting caller-provided JSON or environment variables.

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::Path;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use base64::Engine as _;
use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac as _};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

const TOKEN_VERSION: &str = "r1";
const KEY_FILE: &str = "runtime-capability.key";
const RUNTIME_BRIDGE_TTL: Duration = Duration::hours(24);
const SESSION_TTL: Duration = Duration::minutes(45);

type HmacSha256 = Hmac<Sha256>;

/// An installation-local issuer. The key never crosses into a Runtime; a
/// Runtime only sees a grant that is already bounded by this signer.
#[derive(Clone)]
pub(crate) struct CapabilityIssuer {
    key: Arc<[u8]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CapabilityKind {
    RuntimeBridge,
    ActorSession,
    ModelSession,
}

/// The intentionally fixed claim shape. It is internal to this module so a
/// new caller cannot quietly add ambient authority to a token.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Claims {
    version: u8,
    kind: CapabilityKind,
    company: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    actor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    billing: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    responsibility: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    work_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    attempt_id: Option<Uuid>,
    session: String,
    expires_at: DateTime<Utc>,
}

/// Identity derived from a TCP coordination capability, never from a request
/// field. A bridge grants the Runtime's standing Exec identity; an actor
/// session grants exactly the supervised actor that launched it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CoordinationGrant {
    pub(crate) company: String,
    pub(crate) actor: String,
    pub(crate) session: String,
}

/// Scope that the Runtime-facing model relay verifies before forwarding one
/// pi-native request to the host-only OMP gateway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelGrant {
    pub(crate) company: String,
    pub(crate) actor: String,
    pub(crate) session: String,
    pub(crate) provider: String,
    pub(crate) model: String,
    pub(crate) billing: String,
    pub(crate) responsibility: String,
    pub(crate) work_id: Option<Uuid>,
    pub(crate) attempt_id: Option<Uuid>,
}

impl CapabilityIssuer {
    pub(crate) fn open(root: &Path) -> Result<Self> {
        let path = root.join(KEY_FILE);
        let key = match fs::read(&path) {
            Ok(key) => {
                ensure_private_key_file(&path)?;
                key
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let mut key = Vec::with_capacity(32);
                key.extend_from_slice(Uuid::new_v4().as_bytes());
                key.extend_from_slice(Uuid::new_v4().as_bytes());
                match create_key(&path, &key) {
                    Ok(()) => key,
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                        let key = fs::read(&path).with_context(|| {
                            format!("read concurrent capability key {}", path.display())
                        })?;
                        ensure_private_key_file(&path)?;
                        key
                    }
                    Err(error) => {
                        return Err(error)
                            .with_context(|| format!("create capability key {}", path.display()))
                    }
                }
            }
            Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
        };
        if key.len() != 32 {
            bail!(
                "capability key {} has {} bytes; expected 32",
                path.display(),
                key.len()
            );
        }
        Ok(Self { key: key.into() })
    }

    /// A company computer's ordinary bridge identity. It is deliberately
    /// less precise than an actor session, and therefore always becomes Exec.
    /// restless up refreshes the file materialised inside the Runtime.
    pub(crate) fn issue_runtime_bridge(&self, company: &str) -> Result<String> {
        self.issue(Claims {
            version: 1,
            kind: CapabilityKind::RuntimeBridge,
            company: company.to_string(),
            actor: None,
            provider: None,
            model: None,
            billing: None,
            responsibility: None,
            work_id: None,
            attempt_id: None,
            session: format!("bridge-{}", Uuid::new_v4().simple()),
            expires_at: Utc::now() + RUNTIME_BRIDGE_TTL,
        })
    }

    /// One supervised ACP process gets this narrower coordination grant.
    pub(crate) fn issue_actor_session(
        &self,
        company: &str,
        actor: &str,
        session: &str,
    ) -> Result<String> {
        self.issue(Claims {
            version: 1,
            kind: CapabilityKind::ActorSession,
            company: company.to_string(),
            actor: Some(actor.to_string()),
            provider: None,
            model: None,
            billing: None,
            responsibility: None,
            work_id: None,
            attempt_id: None,
            session: session.to_string(),
            expires_at: Utc::now() + SESSION_TTL,
        })
    }

    /// Model access is a separate grant so a coordination bearer cannot be
    /// replayed at the model relay.
    #[expect(
        clippy::too_many_arguments,
        reason = "the signed grant boundary keeps company, actor, session, provider, model, billing and productive coordinates explicit"
    )]
    pub(crate) fn issue_model_session(
        &self,
        company: &str,
        actor: &str,
        session: &str,
        provider: &str,
        model: &str,
        billing: &str,
        responsibility: &str,
        work_id: Option<Uuid>,
        attempt_id: Option<Uuid>,
    ) -> Result<String> {
        self.issue(Claims {
            version: 1,
            kind: CapabilityKind::ModelSession,
            company: company.to_string(),
            actor: Some(actor.to_string()),
            provider: Some(provider.to_string()),
            model: Some(model.to_string()),
            billing: Some(billing.to_string()),
            responsibility: Some(responsibility.to_string()),
            work_id,
            attempt_id,
            session: session.to_string(),
            expires_at: Utc::now() + SESSION_TTL,
        })
    }

    pub(crate) fn verify_coordination(&self, token: &str) -> Result<CoordinationGrant> {
        let claims = self.verify(token)?;
        let actor = match claims.kind {
            CapabilityKind::RuntimeBridge => "exec".to_string(),
            CapabilityKind::ActorSession => claims
                .actor
                .context("actor session capability is missing its actor")?,
            CapabilityKind::ModelSession => bail!("a model capability cannot call coordination"),
        };
        Ok(CoordinationGrant {
            company: claims.company,
            actor,
            session: claims.session,
        })
    }

    pub(crate) fn verify_model(&self, token: &str) -> Result<ModelGrant> {
        let claims = self.verify(token)?;
        if claims.kind != CapabilityKind::ModelSession {
            bail!("a non-model capability cannot call the model relay");
        }
        Ok(ModelGrant {
            company: claims.company,
            actor: claims
                .actor
                .context("model capability is missing its actor")?,
            session: claims.session,
            provider: claims
                .provider
                .context("model capability is missing its provider")?,
            model: claims
                .model
                .context("model capability is missing its exact model")?,
            billing: claims
                .billing
                .context("model capability is missing its billing policy")?,
            responsibility: claims
                .responsibility
                .context("model capability is missing its responsibility")?,
            work_id: claims.work_id,
            attempt_id: claims.attempt_id,
        })
    }

    fn issue(&self, claims: Claims) -> Result<String> {
        validate_claims(&claims)?;
        let payload = serde_json::to_vec(&claims).context("encode capability claim")?;
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload);
        let signed = format!("{TOKEN_VERSION}.{payload}");
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("32-byte HMAC key");
        mac.update(signed.as_bytes());
        let signature =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        Ok(format!("{signed}.{signature}"))
    }

    fn verify(&self, token: &str) -> Result<Claims> {
        let mut parts = token.split('.');
        let version = parts.next().context("capability has no version")?;
        let encoded = parts.next().context("capability has no payload")?;
        let signature = parts.next().context("capability has no signature")?;
        if parts.next().is_some()
            || version != TOKEN_VERSION
            || encoded.is_empty()
            || signature.is_empty()
        {
            bail!("malformed capability");
        }
        let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(signature)
            .context("capability signature is not base64url")?;
        let signed = format!("{version}.{encoded}");
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("32-byte HMAC key");
        mac.update(signed.as_bytes());
        mac.verify_slice(&signature)
            .map_err(|_| anyhow::anyhow!("capability signature is invalid"))?;
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded)
            .context("capability payload is not base64url")?;
        let claims =
            serde_json::from_slice::<Claims>(&payload).context("decode capability claim")?;
        validate_claims(&claims)?;
        if claims.expires_at <= Utc::now() {
            bail!("capability expired at {}", claims.expires_at.to_rfc3339());
        }
        Ok(claims)
    }
}

fn validate_claims(claims: &Claims) -> Result<()> {
    if claims.version != 1 {
        bail!("unsupported capability claim version");
    }
    validate_identifier("company", &claims.company)?;
    validate_identifier("session", &claims.session)?;
    if let Some(actor) = &claims.actor {
        validate_identifier("actor", actor)?;
    }
    if let Some(provider) = &claims.provider {
        if provider.is_empty()
            || !provider
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            bail!("capability provider is invalid");
        }
    }
    if let Some(model) = &claims.model {
        if model.is_empty()
            || model.len() > 300
            || !model.bytes().all(|byte| {
                byte.is_ascii_lowercase()
                    || byte.is_ascii_uppercase()
                    || byte.is_ascii_digit()
                    || matches!(byte, b'-' | b'_' | b'.' | b'/' | b':')
            })
        {
            bail!("capability model is invalid");
        }
    }
    if let Some(billing) = &claims.billing {
        if !matches!(billing.as_str(), "metered_api" | "subscription") {
            bail!("capability billing policy is invalid");
        }
    }
    if let Some(responsibility) = &claims.responsibility {
        if responsibility.is_empty()
            || responsibility.len() > 300
            || responsibility
                .chars()
                .any(|character| character.is_control())
        {
            bail!("capability responsibility is invalid");
        }
    }
    if claims.work_id == Some(Uuid::nil()) || claims.attempt_id == Some(Uuid::nil()) {
        bail!("capability Work and Attempt coordinates must be non-nil");
    }
    match claims.kind {
        CapabilityKind::RuntimeBridge => {
            if claims.actor.is_some()
                || claims.provider.is_some()
                || claims.model.is_some()
                || claims.billing.is_some()
                || claims.responsibility.is_some()
                || claims.work_id.is_some()
                || claims.attempt_id.is_some()
            {
                bail!("runtime bridge capability carries a foreign scope");
            }
        }
        CapabilityKind::ActorSession => {
            if claims.actor.is_none()
                || claims.provider.is_some()
                || claims.model.is_some()
                || claims.billing.is_some()
                || claims.responsibility.is_some()
                || claims.work_id.is_some()
                || claims.attempt_id.is_some()
            {
                bail!("actor session capability has an invalid scope");
            }
        }
        CapabilityKind::ModelSession => {
            if claims.actor.is_none()
                || claims.provider.is_none()
                || claims.model.is_none()
                || claims.billing.is_none()
                || claims.responsibility.is_none()
            {
                bail!("model capability has an incomplete scope");
            }
            if claims.attempt_id.is_some() != claims.work_id.is_some() {
                bail!("model capability must pair Work and Attempt coordinates");
            }
        }
    }
    Ok(())
}

fn validate_identifier(label: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 160
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_uppercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'-' | b'_')
        })
    {
        bail!("capability {label} is invalid");
    }
    Ok(())
}

fn create_key(path: &Path, key: &[u8]) -> std::io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(key)?;
    file.sync_all()?;
    Ok(())
}

fn ensure_private_key_file(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        let mode = fs::metadata(path)
            .with_context(|| format!("inspect capability key {}", path.display()))?
            .mode();
        if mode & 0o077 != 0 {
            bail!(
                "capability key {} must not be readable by group or others",
                path.display()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issuer() -> (std::path::PathBuf, CapabilityIssuer) {
        let root =
            std::env::temp_dir().join(format!("restless-capability-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let issuer = CapabilityIssuer::open(&root).unwrap();
        (root, issuer)
    }

    #[test]
    fn scoped_claims_reject_tampering_and_cross_boundary_replay() {
        let (root, issuer) = issuer();
        let coordination = issuer
            .issue_actor_session("acme_test", "delivery-lead", "session_1")
            .unwrap();
        assert_eq!(
            issuer.verify_coordination(&coordination).unwrap(),
            CoordinationGrant {
                company: "acme_test".into(),
                actor: "delivery-lead".into(),
                session: "session_1".into(),
            }
        );
        assert!(issuer.verify_model(&coordination).is_err());

        let tampered = format!("{coordination}x");
        assert!(issuer.verify_coordination(&tampered).is_err());

        let model = issuer
            .issue_model_session(
                "acme_test",
                "delivery-lead",
                "session_1",
                "moonshot",
                "moonshot/kimi-k3",
                "metered_api",
                "work:delivery",
                None,
                None,
            )
            .unwrap();
        assert_eq!(issuer.verify_model(&model).unwrap().provider, "moonshot");
        assert_eq!(
            issuer.verify_model(&model).unwrap().model,
            "moonshot/kimi-k3"
        );
        assert!(issuer.verify_coordination(&model).is_err());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn expired_claim_is_refused_after_a_valid_signature() {
        let (root, issuer) = issuer();
        let token = issuer
            .issue(Claims {
                version: 1,
                kind: CapabilityKind::ModelSession,
                company: "acme_test".into(),
                actor: Some("delivery-lead".into()),
                provider: Some("moonshot".into()),
                model: Some("moonshot/kimi-k3".into()),
                billing: Some("metered_api".into()),
                responsibility: Some("work:delivery".into()),
                work_id: None,
                attempt_id: None,
                session: "expired_1".into(),
                expires_at: Utc::now() - Duration::seconds(1),
            })
            .unwrap();
        assert!(issuer.verify_model(&token).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
