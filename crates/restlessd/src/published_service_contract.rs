use std::net::IpAddr;

use anyhow::{bail, Context, Result};
use base64::Engine as _;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CONTRACT_VERSION: &str = "published-service.v1";
const TOKEN_VERSION: &str = "ps1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceProfile {
    HttpsWebsocketDemo,
    GodotEnetUdp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Audience {
    OwnerOnly,
    NamedInvitees,
    Public,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EgressPolicy {
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ReadinessProbe {
    Http {
        path: String,
        websocket_path: String,
    },
    Udp {
        request: String,
        response: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceManifest {
    pub contract_version: String,
    pub image: String,
    pub profile: ServiceProfile,
    pub internal_port: u16,
    pub readiness: ReadinessProbe,
}

impl ServiceManifest {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != CONTRACT_VERSION {
            bail!(
                "unsupported published-service contract {:?}; expected {CONTRACT_VERSION}",
                self.contract_version
            );
        }
        validate_immutable_oci_ref(&self.image)?;
        if self.internal_port == 0 {
            bail!("internal_port must be in 1..=65535");
        }
        match (&self.profile, &self.readiness) {
            (
                ServiceProfile::HttpsWebsocketDemo,
                ReadinessProbe::Http {
                    path,
                    websocket_path,
                },
            ) => {
                validate_path("HTTP readiness path", path)?;
                validate_path("WebSocket path", websocket_path)?;
                if path == websocket_path || path == "/" || websocket_path == "/" {
                    bail!("HTTP readiness and WebSocket paths must be distinct and may not replace the service root");
                }
            }
            (ServiceProfile::GodotEnetUdp, ReadinessProbe::Udp { request, response }) => {
                validate_probe("UDP readiness request", request)?;
                validate_probe("UDP readiness response", response)?;
            }
            _ => bail!("readiness kind does not match the declared service profile"),
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).context("encode service manifest")?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceLimits {
    pub cpu_millis: u32,
    pub memory_mib: u32,
    pub ephemeral_storage_mib: u32,
    pub max_connections: u32,
}

impl ResourceLimits {
    pub fn validate(&self) -> Result<()> {
        if !(50..=8_000).contains(&self.cpu_millis) {
            bail!("cpu_millis must be in 50..=8000");
        }
        if !(32..=32_768).contains(&self.memory_mib) {
            bail!("memory_mib must be in 32..=32768");
        }
        if !(32..=65_536).contains(&self.ephemeral_storage_mib) {
            bail!("ephemeral_storage_mib must be in 32..=65536");
        }
        if !(1..=10_000).contains(&self.max_connections) {
            bail!("max_connections must be in 1..=10000");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishedServiceCandidate {
    pub contract_version: String,
    pub candidate_id: String,
    pub company: String,
    pub work_id: String,
    pub attempt_id: String,
    pub producing_actor: String,
    pub source_artifact_ref_id: String,
    pub image: String,
    pub manifest: ServiceManifest,
    pub manifest_digest: String,
    pub source_commit: String,
    pub runtime_generation: String,
    pub created_at: DateTime<Utc>,
}

impl PublishedServiceCandidate {
    pub fn validate(&self) -> Result<()> {
        if self.contract_version != CONTRACT_VERSION
            || self.manifest.contract_version != CONTRACT_VERSION
        {
            bail!("candidate and manifest must use {CONTRACT_VERSION}");
        }
        validate_identifier("candidate_id", &self.candidate_id)?;
        validate_identifier("company", &self.company)?;
        validate_identifier("work_id", &self.work_id)?;
        validate_identifier("attempt_id", &self.attempt_id)?;
        validate_identifier("producing_actor", &self.producing_actor)?;
        validate_identifier("source_artifact_ref_id", &self.source_artifact_ref_id)?;
        validate_identifier("runtime_generation", &self.runtime_generation)?;
        validate_immutable_oci_ref(&self.image)?;
        if self.image != self.manifest.image {
            bail!("candidate image and manifest image differ");
        }
        let digest = self.manifest.digest()?;
        if self.manifest_digest != digest {
            bail!("manifest_digest does not match the canonical manifest");
        }
        validate_source_commit(&self.source_commit)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishRequest {
    pub contract_version: String,
    pub publication_id: String,
    pub candidate: PublishedServiceCandidate,
    pub audience: Audience,
    pub egress: EgressPolicy,
    pub start_deadline: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub resources: ResourceLimits,
    pub idempotency_key: String,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
}

impl PublishRequest {
    pub fn validate(&self, now: DateTime<Utc>) -> Result<()> {
        if self.contract_version != CONTRACT_VERSION {
            bail!("publish request must use {CONTRACT_VERSION}");
        }
        validate_identifier("publication_id", &self.publication_id)?;
        validate_identifier("idempotency_key", &self.idempotency_key)?;
        validate_identifier("requested_by", &self.requested_by)?;
        self.candidate.validate()?;
        self.resources.validate()?;
        if self.egress != EgressPolicy::Denied {
            bail!("published-service.v1 permits no service egress");
        }
        if self.start_deadline <= now || self.start_deadline >= self.expires_at {
            bail!("start_deadline must be in the future and before publication expiry");
        }
        if self.start_deadline > now + chrono::Duration::hours(24) {
            bail!("start_deadline may not exceed 24 hours");
        }
        if self.expires_at <= now {
            bail!("publication expiry must be in the future");
        }
        if self.expires_at > now + chrono::Duration::days(30) {
            bail!("publication expiry may not exceed 30 days");
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String> {
        let bytes = serde_json::to_vec(self).context("encode publish request")?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderEndpoint {
    pub profile: ServiceProfile,
    pub public_endpoint: String,
    pub bound_port: u16,
    pub transport_security: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderReadyReceipt {
    pub contract_version: String,
    pub publication_id: String,
    pub candidate_digest: String,
    pub provider_operation_id: String,
    pub endpoint: ProviderEndpoint,
    /// Stable identity of the publication-scoped invitation verification key.
    /// This is a digest, never the verification material itself.
    pub invitation_key_id: String,
    pub provider_process_id: u32,
    pub ready_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderCleanupReceipt {
    pub contract_version: String,
    pub publication_id: String,
    pub candidate_digest: String,
    pub provider_process_absent: bool,
    pub route_absent: bool,
    pub invitation_material_absent: bool,
    pub resource_lease_released: bool,
    pub temporary_files_absent: bool,
    pub cleaned_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceObservations {
    pub accepted_connections: u64,
    pub rejected_connections: u64,
    pub messages_received: u64,
    /// Provider measurements remain `None` when the provider cannot observe them.
    pub cpu_millis_observed: Option<u64>,
    pub peak_memory_mib_observed: Option<u64>,
    pub ephemeral_storage_mib_observed: Option<u64>,
    pub last_activity_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvitationClaims {
    pub version: String,
    pub invitation_id: String,
    pub publication_id: String,
    pub company: String,
    pub candidate_digest: String,
    pub subject: String,
    pub expires_at: DateTime<Utc>,
}

impl InvitationClaims {
    pub fn new(
        invitation_id: String,
        publication_id: String,
        company: String,
        candidate_digest: String,
        subject: String,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            version: TOKEN_VERSION.to_string(),
            invitation_id,
            publication_id,
            company,
            candidate_digest,
            subject,
            expires_at,
        }
    }

    pub fn validate(&self, now: DateTime<Utc>) -> Result<()> {
        if self.version != TOKEN_VERSION {
            bail!("unsupported invitation token version");
        }
        for (name, value) in [
            ("invitation_id", &self.invitation_id),
            ("publication_id", &self.publication_id),
            ("company", &self.company),
            ("candidate_digest", &self.candidate_digest),
            ("subject", &self.subject),
        ] {
            validate_identifier(name, value)?;
        }
        if self.expires_at <= now {
            bail!("invitation is expired");
        }
        Ok(())
    }
}

pub fn sign_invitation(secret: &[u8], claims: &InvitationClaims) -> Result<String> {
    if secret.len() < 32 {
        bail!("invitation signing key must contain at least 32 bytes");
    }
    let payload = serde_json::to_vec(claims).context("encode invitation claims")?;
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload);
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).context("construct invitation signer")?;
    mac.update(encoded.as_bytes());
    let signature =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    Ok(format!("{encoded}.{signature}"))
}

pub fn verify_invitation(
    secret: &[u8],
    token: &str,
    expected_company: &str,
    expected_publication: &str,
    expected_candidate: &str,
    now: DateTime<Utc>,
) -> Result<InvitationClaims> {
    let (encoded, signature) = token
        .split_once('.')
        .context("invitation token must have payload and signature")?;
    if encoded.contains('.') || signature.contains('.') {
        bail!("invitation token has too many segments");
    }
    let supplied = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(signature)
        .context("decode invitation signature")?;
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret).context("construct invitation verifier")?;
    mac.update(encoded.as_bytes());
    mac.verify_slice(&supplied)
        .context("invitation signature is invalid")?;
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .context("decode invitation payload")?;
    let claims: InvitationClaims =
        serde_json::from_slice(&payload).context("decode invitation claims")?;
    claims.validate(now)?;
    if claims.company != expected_company {
        bail!("invitation is scoped to another company");
    }
    if claims.publication_id != expected_publication {
        bail!("invitation is scoped to another publication");
    }
    if claims.candidate_digest != expected_candidate {
        bail!("invitation is scoped to another build");
    }
    Ok(claims)
}

pub fn token_digest(token: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(token.as_bytes()))
}

pub fn validate_immutable_oci_ref(image: &str) -> Result<()> {
    let Some((repository, digest)) = image.rsplit_once("@sha256:") else {
        bail!("image must be an immutable OCI reference ending in @sha256:<64 lowercase hex>");
    };
    if repository.is_empty()
        || repository.len() > 512
        || repository.chars().any(char::is_whitespace)
        || !repository.contains('/')
    {
        bail!("image repository is malformed");
    }
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("image digest must contain exactly 64 lowercase hexadecimal characters");
    }
    Ok(())
}

pub fn ensure_loopback_bind(host: &str) -> Result<()> {
    let ip: IpAddr = host
        .parse()
        .context("local fixture bind_host must be an IP address")?;
    if !ip.is_loopback() {
        bail!("local published-service fixture may bind only a loopback address");
    }
    Ok(())
}

fn validate_path(name: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 256
        || !value.starts_with('/')
        || value.contains("..")
        || value.chars().any(char::is_control)
    {
        bail!("{name} must be a bounded absolute path without traversal");
    }
    Ok(())
}

fn validate_probe(name: &str, value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
        bail!("{name} must contain 1..=256 printable characters");
    }
    Ok(())
}

fn validate_identifier(name: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 256
        || value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        bail!("{name} must be a non-empty bounded identifier without whitespace");
    }
    Ok(())
}

fn validate_source_commit(value: &str) -> Result<()> {
    if value.len() != 40 && value.len() != 64 {
        bail!("source_commit must be a full 40- or 64-character hexadecimal revision");
    }
    if !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("source_commit must be hexadecimal");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret() -> [u8; 32] {
        [7; 32]
    }

    #[test]
    fn immutable_image_validation_rejects_tags_and_uppercase_digests() {
        assert!(validate_immutable_oci_ref("registry.example/game:latest").is_err());
        assert!(validate_immutable_oci_ref(&format!(
            "registry.example/game@sha256:{}",
            "A".repeat(64)
        ))
        .is_err());
        validate_immutable_oci_ref(&format!("registry.example/game@sha256:{}", "a".repeat(64)))
            .unwrap();
    }

    #[test]
    fn invitations_are_signed_scoped_expiring_and_tamper_evident() {
        let now = Utc::now();
        let claims = InvitationClaims::new(
            "invite-1".into(),
            "publication-1".into(),
            "swift-arrival".into(),
            format!("sha256:{}", "b".repeat(64)),
            "playtester@example.com".into(),
            now + chrono::Duration::minutes(5),
        );
        let token = sign_invitation(&secret(), &claims).unwrap();
        assert_eq!(
            verify_invitation(
                &secret(),
                &token,
                "swift-arrival",
                "publication-1",
                &format!("sha256:{}", "b".repeat(64)),
                now,
            )
            .unwrap(),
            claims
        );
        assert!(verify_invitation(
            &secret(),
            &token,
            "another-company",
            "publication-1",
            &format!("sha256:{}", "b".repeat(64)),
            now,
        )
        .is_err());
        assert!(verify_invitation(
            &secret(),
            &token,
            "swift-arrival",
            "publication-2",
            &format!("sha256:{}", "b".repeat(64)),
            now,
        )
        .is_err());
        assert!(verify_invitation(
            &secret(),
            &token,
            "swift-arrival",
            "publication-1",
            &format!("sha256:{}", "b".repeat(64)),
            now + chrono::Duration::minutes(6),
        )
        .is_err());
        assert!(verify_invitation(
            &secret(),
            &(token + "x"),
            "swift-arrival",
            "publication-1",
            &format!("sha256:{}", "b".repeat(64)),
            now,
        )
        .is_err());
    }

    #[test]
    fn released_contract_corpus_has_expected_acceptance_boundary() {
        let http: ServiceManifest = serde_json::from_str(include_str!(
            "../../../docs/sprints/sprint-36/contract/v1/valid-https-websocket.json"
        ))
        .unwrap();
        http.validate().unwrap();
        let udp: ServiceManifest = serde_json::from_str(include_str!(
            "../../../docs/sprints/sprint-36/contract/v1/valid-godot-enet-udp.json"
        ))
        .unwrap();
        udp.validate().unwrap();
        let mutable: ServiceManifest = serde_json::from_str(include_str!(
            "../../../docs/sprints/sprint-36/contract/v1/invalid-mutable-image.json"
        ))
        .unwrap();
        assert!(mutable.validate().is_err());
        let ready: ProviderReadyReceipt = serde_json::from_str(include_str!(
            "../../../docs/sprints/sprint-36/contract/v1/provider-ready.json"
        ))
        .unwrap();
        assert_eq!(ready.contract_version, CONTRACT_VERSION);
        assert!(ready.invitation_key_id.starts_with("sha256:"));
        let fixed_now = DateTime::parse_from_rfc3339("2026-09-02T00:10:00Z")
            .unwrap()
            .with_timezone(&Utc);
        for raw in [
            include_str!(
                "../../../docs/sprints/sprint-36/contract/v1/publish-request-https-websocket.json"
            ),
            include_str!(
                "../../../docs/sprints/sprint-36/contract/v1/publish-request-godot-enet-udp.json"
            ),
        ] {
            let request: PublishRequest = serde_json::from_str(raw).unwrap();
            assert_eq!(
                request.candidate.manifest_digest,
                request.candidate.manifest.digest().unwrap(),
                "golden manifest digest for {:?}",
                request.candidate.manifest.profile
            );
            request.validate(fixed_now).unwrap();
        }
        let cleanup: ProviderCleanupReceipt = serde_json::from_str(include_str!(
            "../../../docs/sprints/sprint-36/contract/v1/provider-cleanup.json"
        ))
        .unwrap();
        assert!(cleanup.provider_process_absent && cleanup.route_absent);
    }
}
