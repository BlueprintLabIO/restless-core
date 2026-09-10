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
    HostedRuntimeBridge,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    owner_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    plane_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    company_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cell_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runtime_generation: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    credential_epoch: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    runtime_image: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    volume_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_revision: Option<String>,
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
    pub(crate) work_id: Option<Uuid>,
    pub(crate) attempt_id: Option<Uuid>,
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

/// Exact deployment identity bound into the hosted Runtime bridge grant.
///
/// The Fleet request, signed capability, websocket registration and live
/// registry all compare this tuple. A company identifier alone is not enough:
/// after a replacement, the old container must be unable to reconnect as the
/// new generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HostedRuntimeBridgeScope {
    pub(crate) company: String,
    pub(crate) owner_id: Uuid,
    pub(crate) plane_id: Uuid,
    pub(crate) company_id: Uuid,
    pub(crate) cell_id: Uuid,
    pub(crate) runtime_id: String,
    pub(crate) runtime_generation: i64,
    pub(crate) runtime_image: String,
    pub(crate) volume_name: String,
    pub(crate) source_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HostedRuntimeBridgeGrant {
    pub(crate) scope: HostedRuntimeBridgeScope,
    pub(crate) credential_id: Uuid,
    pub(crate) credential_epoch: i64,
    pub(crate) expires_at: DateTime<Utc>,
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
            owner_id: None,
            plane_id: None,
            company_id: None,
            cell_id: None,
            runtime_id: None,
            runtime_generation: None,
            credential_epoch: None,
            runtime_image: None,
            volume_name: None,
            source_revision: None,
            session: format!("bridge-{}", Uuid::new_v4().simple()),
            expires_at: Utc::now() + RUNTIME_BRIDGE_TTL,
        })
    }

    pub(crate) fn issue_hosted_runtime_bridge(
        &self,
        scope: &HostedRuntimeBridgeScope,
        credential_id: Uuid,
        credential_epoch: i64,
        expires_at: DateTime<Utc>,
    ) -> Result<String> {
        self.issue(Claims {
            version: 1,
            kind: CapabilityKind::HostedRuntimeBridge,
            company: scope.company.clone(),
            actor: None,
            provider: None,
            model: None,
            billing: None,
            responsibility: None,
            work_id: None,
            attempt_id: None,
            owner_id: Some(scope.owner_id),
            plane_id: Some(scope.plane_id),
            company_id: Some(scope.company_id),
            cell_id: Some(scope.cell_id),
            runtime_id: Some(scope.runtime_id.clone()),
            runtime_generation: Some(scope.runtime_generation),
            credential_epoch: Some(credential_epoch),
            runtime_image: Some(scope.runtime_image.clone()),
            volume_name: Some(scope.volume_name.clone()),
            source_revision: Some(scope.source_revision.clone()),
            session: format!("hosted-bridge-{}", credential_id.simple()),
            expires_at,
        })
    }

    pub(crate) fn verify_hosted_runtime_bridge(
        &self,
        token: &str,
    ) -> Result<HostedRuntimeBridgeGrant> {
        let claims = self.verify(token)?;
        if claims.kind != CapabilityKind::HostedRuntimeBridge {
            bail!("a non-hosted capability cannot register a Runtime bridge");
        }
        let credential_id = claims
            .session
            .strip_prefix("hosted-bridge-")
            .and_then(|value| Uuid::parse_str(value).ok())
            .filter(|value| !value.is_nil())
            .context("hosted bridge capability has an invalid credential id")?;
        Ok(HostedRuntimeBridgeGrant {
            scope: HostedRuntimeBridgeScope {
                company: claims.company,
                owner_id: claims
                    .owner_id
                    .context("hosted bridge capability is missing owner_id")?,
                plane_id: claims
                    .plane_id
                    .context("hosted bridge capability is missing plane_id")?,
                company_id: claims
                    .company_id
                    .context("hosted bridge capability is missing company_id")?,
                cell_id: claims
                    .cell_id
                    .context("hosted bridge capability is missing cell_id")?,
                runtime_id: claims
                    .runtime_id
                    .context("hosted bridge capability is missing runtime_id")?,
                runtime_generation: claims
                    .runtime_generation
                    .context("hosted bridge capability is missing runtime_generation")?,
                runtime_image: claims
                    .runtime_image
                    .context("hosted bridge capability is missing runtime_image")?,
                volume_name: claims
                    .volume_name
                    .context("hosted bridge capability is missing volume_name")?,
                source_revision: claims
                    .source_revision
                    .context("hosted bridge capability is missing source_revision")?,
            },
            credential_id,
            credential_epoch: claims
                .credential_epoch
                .context("hosted bridge capability is missing credential_epoch")?,
            expires_at: claims.expires_at,
        })
    }

    /// One supervised ACP process gets this narrower coordination grant.
    pub(crate) fn issue_actor_session(
        &self,
        company: &str,
        actor: &str,
        session: &str,
        work_id: Option<Uuid>,
        attempt_id: Option<Uuid>,
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
            work_id,
            attempt_id,
            owner_id: None,
            plane_id: None,
            company_id: None,
            cell_id: None,
            runtime_id: None,
            runtime_generation: None,
            credential_epoch: None,
            runtime_image: None,
            volume_name: None,
            source_revision: None,
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
            owner_id: None,
            plane_id: None,
            company_id: None,
            cell_id: None,
            runtime_id: None,
            runtime_generation: None,
            credential_epoch: None,
            runtime_image: None,
            volume_name: None,
            source_revision: None,
            session: session.to_string(),
            expires_at: Utc::now() + SESSION_TTL,
        })
    }

    pub(crate) fn verify_coordination(&self, token: &str) -> Result<CoordinationGrant> {
        let claims = self.verify(token)?;
        let actor = match claims.kind {
            CapabilityKind::RuntimeBridge => "exec".to_string(),
            CapabilityKind::HostedRuntimeBridge => {
                bail!("a hosted bridge capability cannot call coordination directly")
            }
            CapabilityKind::ActorSession => claims
                .actor
                .context("actor session capability is missing its actor")?,
            CapabilityKind::ModelSession => bail!("a model capability cannot call coordination"),
        };
        Ok(CoordinationGrant {
            company: claims.company,
            actor,
            session: claims.session,
            work_id: claims.work_id,
            attempt_id: claims.attempt_id,
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
    let hosted_fields_present = [
        claims.owner_id.is_some(),
        claims.plane_id.is_some(),
        claims.company_id.is_some(),
        claims.cell_id.is_some(),
        claims.runtime_id.is_some(),
        claims.runtime_generation.is_some(),
        claims.credential_epoch.is_some(),
        claims.runtime_image.is_some(),
        claims.volume_name.is_some(),
        claims.source_revision.is_some(),
    ];
    let any_hosted = hosted_fields_present.iter().any(|present| *present);
    let all_hosted = hosted_fields_present.iter().all(|present| *present);
    if any_hosted && !all_hosted {
        bail!("hosted Runtime bridge capability has an incomplete deployment scope");
    }
    if let Some(runtime_id) = &claims.runtime_id {
        validate_identifier("runtime_id", runtime_id)?;
    }
    if let Some(volume_name) = &claims.volume_name {
        validate_identifier("volume_name", volume_name)?;
    }
    for (label, value) in [
        ("runtime_image", claims.runtime_image.as_deref()),
        ("source_revision", claims.source_revision.as_deref()),
    ] {
        if let Some(value) = value {
            validate_bounded_text(label, value, 512)?;
        }
    }
    if claims.runtime_generation.is_some_and(|value| value <= 0)
        || claims.credential_epoch.is_some_and(|value| value <= 0)
    {
        bail!("hosted Runtime bridge generations are invalid");
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
                || any_hosted
            {
                bail!("runtime bridge capability carries a foreign scope");
            }
        }
        CapabilityKind::HostedRuntimeBridge => {
            if claims.actor.is_some()
                || claims.provider.is_some()
                || claims.model.is_some()
                || claims.billing.is_some()
                || claims.responsibility.is_some()
                || claims.work_id.is_some()
                || claims.attempt_id.is_some()
                || !all_hosted
                || [
                    claims.owner_id,
                    claims.plane_id,
                    claims.company_id,
                    claims.cell_id,
                ]
                .into_iter()
                .flatten()
                .any(|value| value.is_nil())
            {
                bail!("hosted Runtime bridge capability has an invalid scope");
            }
        }
        CapabilityKind::ActorSession => {
            if claims.actor.is_none()
                || claims.provider.is_some()
                || claims.model.is_some()
                || claims.billing.is_some()
                || claims.responsibility.is_some()
                || any_hosted
            {
                bail!("actor session capability has an invalid scope");
            }
            if claims.attempt_id.is_some() != claims.work_id.is_some() {
                bail!("actor session capability must pair Work and Attempt coordinates");
            }
        }
        CapabilityKind::ModelSession => {
            if claims.actor.is_none()
                || claims.provider.is_none()
                || claims.model.is_none()
                || claims.billing.is_none()
                || claims.responsibility.is_none()
                || any_hosted
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

fn validate_bounded_text(label: &str, value: &str, max_len: usize) -> Result<()> {
    if value.is_empty()
        || value.len() > max_len
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        bail!("capability {label} is invalid");
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
            .issue_actor_session("acme_test", "delivery-lead", "session_1", None, None)
            .unwrap();
        assert_eq!(
            issuer.verify_coordination(&coordination).unwrap(),
            CoordinationGrant {
                company: "acme_test".into(),
                actor: "delivery-lead".into(),
                session: "session_1".into(),
                work_id: None,
                attempt_id: None,
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

        let work_id = Uuid::new_v4();
        let attempt_id = Uuid::new_v4();
        let productive = issuer
            .issue_actor_session(
                "acme_test",
                "delivery-lead",
                "session_productive",
                Some(work_id),
                Some(attempt_id),
            )
            .unwrap();
        let productive = issuer.verify_coordination(&productive).unwrap();
        assert_eq!(productive.work_id, Some(work_id));
        assert_eq!(productive.attempt_id, Some(attempt_id));
        assert!(issuer
            .issue_actor_session(
                "acme_test",
                "delivery-lead",
                "session_unpaired",
                Some(work_id),
                None,
            )
            .is_err());

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
                owner_id: None,
                plane_id: None,
                company_id: None,
                cell_id: None,
                runtime_id: None,
                runtime_generation: None,
                credential_epoch: None,
                runtime_image: None,
                volume_name: None,
                source_revision: None,
                session: "expired_1".into(),
                expires_at: Utc::now() - Duration::seconds(1),
            })
            .unwrap();
        assert!(issuer.verify_model(&token).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hosted_bridge_is_exact_and_cannot_cross_other_boundaries() {
        let (root, issuer) = issuer();
        let scope = HostedRuntimeBridgeScope {
            company: "c0123456789abcdef0123456789abcdef".into(),
            owner_id: Uuid::new_v4(),
            plane_id: Uuid::new_v4(),
            company_id: Uuid::new_v4(),
            cell_id: Uuid::new_v4(),
            runtime_id: "restless-cell-0123456789abcdef0123456789abcdef".into(),
            runtime_generation: 7,
            runtime_image: "registry.example/restless/company-runtime@sha256:0123".into(),
            volume_name: "restless-cell-0123456789abcdef0123456789abcdef-data".into(),
            source_revision: "deadbeef".into(),
        };
        let credential_id = Uuid::new_v4();
        let expires_at = Utc::now() + Duration::hours(1);
        let token = issuer
            .issue_hosted_runtime_bridge(&scope, credential_id, 3, expires_at)
            .unwrap();
        let verified = issuer.verify_hosted_runtime_bridge(&token).unwrap();
        assert_eq!(verified.scope, scope);
        assert_eq!(verified.credential_id, credential_id);
        assert_eq!(verified.credential_epoch, 3);
        assert!(issuer.verify_coordination(&token).is_err());
        assert!(issuer.verify_model(&token).is_err());

        let mut tampered = token.into_bytes();
        let last = tampered.last_mut().unwrap();
        *last = if *last == b'a' { b'b' } else { b'a' };
        assert!(issuer
            .verify_hosted_runtime_bridge(std::str::from_utf8(&tampered).unwrap())
            .is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
