//! Bounded publication of immutable service candidates.
//!
//! Runtime and OrgIntel can prepare and request a candidate. Only the local
//! owner principal can authorize a resource grant, issue invitations, revoke
//! access, or stop it. Provider credentials and network/process custody stay
//! in this host-side adapter and never enter the company Runtime.

use std::fs::OpenOptions;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use base64::Engine as _;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use restless_orgintel::{ArtifactRefState, NewArtifactRef, OrgIntel};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row as _;
use uuid::Uuid;

use crate::authority::AuthorityStore;
use restlessd::published_service_contract::{
    sign_invitation, token_digest, Audience, EgressPolicy, InvitationClaims,
    ProviderCleanupReceipt, PublishRequest, PublishedServiceCandidate, ResourceLimits,
    ServiceManifest, ServiceObservations, ServiceProfile, CONTRACT_VERSION,
};
use restlessd::published_service_fixture::{load_marker, LocalFixtureConfig, LocalFixtureMarker};

const PROVIDER_ENV: &str = "RESTLESS_PUBLISHED_SERVICE_PROVIDER";
const FIXTURE_BINARY_ENV: &str = "RESTLESS_PUBLISHED_SERVICE_FIXTURE_BIN";
const LOCAL_PROVIDER: &str = "local-test";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixtureDisposition {
    Started,
    Existing,
    Restarted,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedPublicationAccess {
    pub(crate) endpoint: restlessd::published_service_contract::ProviderEndpoint,
    pub(crate) token: Option<String>,
    pub(crate) subject: Option<String>,
    pub(crate) candidate_digest: String,
    pub(crate) expires_at: DateTime<Utc>,
    pub(crate) local_self_signed_tls: bool,
}

#[derive(Clone)]
pub(crate) struct PublicationManager {
    root: PathBuf,
    authority: AuthorityStore,
    signing_key: Vec<u8>,
}

impl PublicationManager {
    pub(crate) fn new(root: &Path, authority: AuthorityStore) -> Result<Self> {
        let directory = root.join("published-services");
        std::fs::create_dir_all(&directory)
            .with_context(|| format!("create {}", directory.display()))?;
        let key_path = root.join("publication-signing.key");
        let signing_key = load_or_create_key(&key_path)?;
        Ok(Self {
            root: directory,
            authority,
            signing_key,
        })
    }

    pub(crate) async fn create_candidate(
        &self,
        org: &OrgIntel,
        company: &str,
        actor: &str,
        source_artifact_ref_id: &str,
        manifest: ServiceManifest,
    ) -> Result<Value> {
        manifest.validate()?;
        let source_id = Uuid::parse_str(source_artifact_ref_id)
            .context("source_artifact_ref_id must be an OrgIntel artifact UUID")?;
        let source = org
            .get_artifact_ref(source_id)
            .await?
            .context("source artifact does not exist")?;
        if source.state != ArtifactRefState::Available {
            bail!("source artifact is not available");
        }
        if source.created_by != actor {
            bail!(
                "candidate actor {actor:?} did not produce source artifact {}",
                source.id
            );
        }
        let work_id = source
            .work_id
            .context("source artifact has no Work provenance")?;
        let attempt_id = source
            .attempt_id
            .context("source artifact has no Attempt provenance")?;
        let source_commit = source
            .source_commit
            .as_deref()
            .context("source artifact has no exact source commit")?;
        let runtime_generation = source
            .runtime_generation
            .as_deref()
            .context("source artifact has no Runtime generation")?;
        if source.uri != manifest.image {
            bail!("manifest image does not match the source artifact locator");
        }
        let image_digest = manifest
            .image
            .rsplit_once('@')
            .map(|(_, digest)| digest)
            .context("validated OCI reference lost its digest")?;
        if source.digest.as_deref() != Some(image_digest) {
            bail!("source artifact digest does not match the immutable OCI image digest");
        }
        let manifest_digest = manifest.digest()?;

        // Candidate creation is naturally idempotent for an exact source
        // artifact + canonical manifest. Recover the existing note instead of
        // inventing another identity on a retry.
        for artifact in org.list_artifact_refs(Some(work_id)).await? {
            if artifact.kind == "published_service_candidate"
                && artifact.state == ArtifactRefState::Available
                && artifact.attempt_id == Some(attempt_id)
                && artifact.digest.as_deref() == Some(manifest_digest.as_str())
            {
                if let Ok(existing) =
                    serde_json::from_str::<PublishedServiceCandidate>(&artifact.note)
                {
                    if existing.source_artifact_ref_id == source_artifact_ref_id
                        && existing.manifest == manifest
                    {
                        existing.validate()?;
                        return Ok(json!({
                            "candidate_artifact_ref_id": artifact.id,
                            "candidate": existing,
                            "replayed": true,
                        }));
                    }
                }
            }
        }

        let candidate = PublishedServiceCandidate {
            contract_version: CONTRACT_VERSION.to_string(),
            candidate_id: format!("candidate-{}", Uuid::new_v4()),
            company: company.to_string(),
            work_id: work_id.to_string(),
            attempt_id: attempt_id.to_string(),
            producing_actor: actor.to_string(),
            source_artifact_ref_id: source_artifact_ref_id.to_string(),
            image: manifest.image.clone(),
            manifest,
            manifest_digest: manifest_digest.clone(),
            source_commit: source_commit.to_string(),
            runtime_generation: runtime_generation.to_string(),
            created_at: Utc::now(),
        };
        candidate.validate()?;
        let note = serde_json::to_string(&candidate).context("encode publication candidate")?;
        let label = format!("Published service candidate {}", candidate.candidate_id);
        let candidate_artifact_ref_id = org
            .link_work_artifact(NewArtifactRef {
                kind: "published_service_candidate",
                uri: &candidate.image,
                note: &note,
                created_by: actor,
                work_id: Some(work_id),
                attempt_id: Some(attempt_id),
                digest: Some(&manifest_digest),
                source_commit: Some(source_commit),
                runtime_generation: Some(runtime_generation),
                label: &label,
            })
            .await?;
        Ok(json!({
            "candidate_artifact_ref_id": candidate_artifact_ref_id,
            "candidate": candidate,
            "replayed": false,
        }))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn request(
        &self,
        org: &OrgIntel,
        company: &str,
        actor: &str,
        candidate_artifact_ref_id: &str,
        audience: Audience,
        start_deadline: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        resources: ResourceLimits,
        idempotency_key: &str,
    ) -> Result<Value> {
        let candidate_artifact_id = Uuid::parse_str(candidate_artifact_ref_id)
            .context("candidate_artifact_ref_id must be an OrgIntel artifact UUID")?;
        let artifact = org
            .get_artifact_ref(candidate_artifact_id)
            .await?
            .context("publication candidate artifact does not exist")?;
        if artifact.kind != "published_service_candidate"
            || artifact.state != ArtifactRefState::Available
        {
            bail!("artifact is not an available published-service candidate");
        }
        let candidate: PublishedServiceCandidate = serde_json::from_str(&artifact.note)
            .context("decode publication candidate artifact")?;
        candidate.validate()?;
        if candidate.company != company {
            bail!("candidate belongs to another company");
        }
        if artifact.digest.as_deref() != Some(candidate.manifest_digest.as_str()) {
            bail!("candidate artifact digest and candidate manifest disagree");
        }
        let intent = json!({
            "candidate_artifact_ref_id": candidate_artifact_ref_id,
            "candidate": candidate,
            "audience": audience,
            "egress": EgressPolicy::Denied,
            "start_deadline": start_deadline,
            "expires_at": expires_at,
            "resources": resources,
            "requested_by": actor,
            "idempotency_key": idempotency_key,
        });
        let request_digest = json_digest(&intent)?;
        let publication_id = format!("publication-{}", &request_digest[7..31]);
        let request = PublishRequest {
            contract_version: CONTRACT_VERSION.to_string(),
            publication_id: publication_id.clone(),
            candidate,
            audience,
            egress: EgressPolicy::Denied,
            start_deadline,
            expires_at,
            resources,
            idempotency_key: idempotency_key.to_string(),
            requested_by: actor.to_string(),
            requested_at: Utc::now(),
        };
        request.validate(Utc::now())?;
        let body = json!({
            "publication_id": publication_id,
            "candidate_artifact_ref_id": candidate_artifact_ref_id,
            "idempotency_key": idempotency_key,
            "request_digest": request_digest,
            "request": request,
        });
        let inserted = sqlx::query_scalar::<_, i64>(
            "INSERT INTO restless_authority.records (company,kind,actor_id,body) \
             VALUES ($1,'publication_request',$2,$3) \
             ON CONFLICT DO NOTHING RETURNING id",
        )
        .bind(company)
        .bind(actor)
        .bind(&body)
        .fetch_optional(self.authority.pool())
        .await?;
        let (authority_record_id, replayed, body) = if let Some(id) = inserted {
            (id, false, body)
        } else {
            let row = sqlx::query(
                "SELECT id,body FROM restless_authority.records \
                 WHERE company=$1 AND kind='publication_request' \
                   AND body->>'idempotency_key'=$2 ORDER BY id LIMIT 1",
            )
            .bind(company)
            .bind(idempotency_key)
            .fetch_one(self.authority.pool())
            .await?;
            let existing_body: Value = row.get("body");
            if existing_body.get("request_digest") != body.get("request_digest") {
                bail!("idempotency key already names a different publication request");
            }
            (row.get("id"), true, existing_body)
        };
        if !replayed {
            org.emit_event(
                "publication_requested",
                Some(actor),
                json!({
                    "authority_record_id": authority_record_id,
                    "publication_id": body["publication_id"],
                    "candidate_artifact_ref_id": candidate_artifact_ref_id,
                }),
            )
            .await?;
        }
        Ok(json!({
            "authority_record_id": authority_record_id,
            "replayed": replayed,
            "status": "awaiting_owner_authorization",
            "publication": body,
        }))
    }

    pub(crate) async fn authorize(
        &self,
        org: &OrgIntel,
        company: &str,
        publication_id: &str,
    ) -> Result<Value> {
        self.ensure_local_provider_allowed(company)?;
        let request = self.request_by_publication(company, publication_id).await?;
        request.validate(Utc::now())?;
        self.ensure_candidate_available(org, company, publication_id)
            .await?;
        if self
            .has_record(company, "publication_stopped", publication_id)
            .await?
        {
            bail!("publication has already been stopped and cannot be restarted");
        }
        let authorization = json!({
            "publication_id": publication_id,
            "request_digest": request.digest()?,
            "authorized_by": "owner",
            "authorized_at": Utc::now(),
            "consequences": {
                "image": request.candidate.image,
                "manifest_digest": request.candidate.manifest_digest,
                "profile": request.candidate.manifest.profile,
                "audience": request.audience,
                "egress": request.egress,
                "start_deadline": request.start_deadline,
                "expires_at": request.expires_at,
                "resources": request.resources,
                "declared_port": request.candidate.manifest.internal_port,
            },
        });
        let authorization_id = insert_once(
            self.authority.pool(),
            company,
            "publication_authorized",
            "owner",
            &authorization,
        )
        .await?;
        let grant = json!({
            "publication_id": publication_id,
            "provider": LOCAL_PROVIDER,
            "profile": request.candidate.manifest.profile,
            "candidate_digest": request.candidate.manifest_digest,
            "resources": request.resources,
            "egress": request.egress,
            "start_deadline": request.start_deadline,
            "expires_at": request.expires_at,
            "declared_port": request.candidate.manifest.internal_port,
            "granted_by_authority_record_id": authorization_id,
        });
        let resource_grant_id = insert_once(
            self.authority.pool(),
            company,
            "publication_resource_grant",
            "owner",
            &grant,
        )
        .await?;
        let (marker, disposition) = match self.ensure_fixture(company, &request).await {
            Ok(value) => value,
            Err(error) => {
                self.record_provider_failure(company, &request, &error)
                    .await?;
                return Err(error);
            }
        };
        let ready_body = json!({
            "publication_id": publication_id,
            "candidate_digest": request.candidate.manifest_digest,
            "resource_grant_record_id": resource_grant_id,
            "receipt": marker.receipt,
        });
        let (ready_id, replayed, recovered) = match disposition {
            FixtureDisposition::Started => (
                insert_once(
                    self.authority.pool(),
                    company,
                    "publication_ready",
                    "authority",
                    &ready_body,
                )
                .await?,
                false,
                false,
            ),
            FixtureDisposition::Existing => {
                let record = self
                    .latest_provider_receipt_record(company, publication_id)
                    .await?
                    .context("active provider has no Authority ready receipt")?;
                let unresolved_failure = self
                    .latest_provider_failure_record(company, publication_id)
                    .await?
                    .is_some_and(|failure| failure > record);
                if unresolved_failure {
                    (
                        self.authority
                            .emit(
                                company,
                                "publication_recovered",
                                Some("authority"),
                                ready_body.clone(),
                            )
                            .await?,
                        false,
                        true,
                    )
                } else {
                    (record, true, false)
                }
            }
            FixtureDisposition::Restarted => (
                self.authority
                    .emit(
                        company,
                        "publication_recovered",
                        Some("authority"),
                        ready_body.clone(),
                    )
                    .await?,
                false,
                true,
            ),
        };
        if !replayed {
            org.emit_event(
                "publication_ready",
                Some("authority"),
                json!({
                    "authority_record_id": ready_id,
                    "publication_id": publication_id,
                    "candidate_digest": request.candidate.manifest_digest,
                    "endpoint": marker.receipt.endpoint,
                    "recovered": recovered,
                }),
            )
            .await?;
        }
        Ok(json!({
            "authorization_record_id": authorization_id,
            "resource_grant_record_id": resource_grant_id,
            "ready_record_id": ready_id,
            "replayed": replayed,
            "recovered": recovered,
            "receipt": marker.receipt,
            "tls_certificate_pem": marker.tls_certificate_pem,
        }))
    }

    pub(crate) async fn invite(
        &self,
        org: &OrgIntel,
        company: &str,
        publication_id: &str,
        invitation_id: &str,
        subject: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<Value> {
        let request = self.request_by_publication(company, publication_id).await?;
        self.ensure_candidate_available(org, company, publication_id)
            .await?;
        if !self
            .has_record(company, "publication_ready", publication_id)
            .await?
            && !self
                .has_record(company, "publication_recovered", publication_id)
                .await?
        {
            bail!("publication is not ready");
        }
        if self
            .has_record(company, "publication_stopped", publication_id)
            .await?
        {
            bail!("publication is stopped");
        }
        match request.audience {
            Audience::Public => bail!("public publications do not mint reusable invitation grants"),
            Audience::OwnerOnly if subject != "owner" => {
                bail!("owner-only publication invitations must be scoped to subject owner")
            }
            _ => {}
        }
        if expires_at > request.expires_at {
            bail!("invitation cannot outlive its publication");
        }
        let claims = InvitationClaims::new(
            invitation_id.to_string(),
            publication_id.to_string(),
            company.to_string(),
            request.candidate.manifest_digest.clone(),
            subject.to_string(),
            expires_at,
        );
        claims.validate(Utc::now())?;
        let key =
            self.invitation_key(company, publication_id, &request.candidate.manifest_digest)?;
        let token = sign_invitation(&key, &claims)?;
        let digest = token_digest(&token);
        let body = json!({
            "publication_id": publication_id,
            "invitation_id": invitation_id,
            "subject": subject,
            "expires_at": expires_at,
            "candidate_digest": request.candidate.manifest_digest,
            "token_digest": digest,
        });
        let record_id = insert_invitation_once(
            self.authority.pool(),
            company,
            "publication_invitation",
            invitation_id,
            &body,
        )
        .await?;
        let recorded = self
            .authority
            .find_body(
                company,
                "publication_invitation",
                "invitation_id",
                invitation_id,
            )
            .await?
            .context("invitation insert disappeared")?;
        if recorded.get("token_digest") != body.get("token_digest") {
            bail!("invitation id already names different claims");
        }
        Ok(json!({
            "invitation_record_id": record_id,
            "publication_id": publication_id,
            "invitation_id": invitation_id,
            "subject": subject,
            "expires_at": expires_at,
            "token": token,
        }))
    }

    /// Prepare owner access without ever returning the reusable invitation to
    /// the browser-facing projection. The launch broker keeps it in memory and
    /// injects it into the bounded transport or one-time native exchange.
    pub(crate) async fn prepare_owner_access(
        &self,
        org: &OrgIntel,
        company: &str,
        publication_id: &str,
    ) -> Result<PreparedPublicationAccess> {
        let now = Utc::now();
        let request = self.request_by_publication(company, publication_id).await?;
        if request.expires_at <= now {
            bail!("publication is expired");
        }
        self.ensure_candidate_available(org, company, publication_id)
            .await?;
        if self
            .has_record(company, "publication_stopped", publication_id)
            .await?
        {
            bail!("publication is stopped");
        }
        let directory = self.publication_directory(company, publication_id)?;
        let marker = load_marker(&directory.join("ready.json"))?;
        self.validate_marker(&request, &marker)?;
        if !process_is_fixture(marker.receipt.provider_process_id)? {
            bail!("published service is not running");
        }
        let expires_at = request.expires_at.min(now + chrono::Duration::minutes(15));
        let token = match request.audience {
            Audience::Public => None,
            Audience::OwnerOnly | Audience::NamedInvitees => {
                let bucket = now.timestamp().div_euclid(15 * 60);
                // Invitation ids are company-scoped in Authority. Include a
                // stable digest of the publication so two live services in
                // the same company never alias one owner launch credential.
                let publication_key = format!("{:x}", Sha256::digest(publication_id.as_bytes()));
                let invitation = self
                    .invite(
                        org,
                        company,
                        publication_id,
                        &format!("owner-launch-{}-{bucket}", &publication_key[..16]),
                        "owner",
                        expires_at,
                    )
                    .await?;
                Some(
                    invitation
                        .get("token")
                        .and_then(Value::as_str)
                        .context("prepared invitation has no token")?
                        .to_string(),
                )
            }
        };
        Ok(PreparedPublicationAccess {
            endpoint: marker.receipt.endpoint,
            token,
            subject: (request.audience != Audience::Public).then(|| "owner".to_string()),
            candidate_digest: request.candidate.manifest_digest,
            expires_at,
            local_self_signed_tls: marker.tls_certificate_pem.is_some(),
        })
    }

    pub(crate) async fn revoke_invitation(
        &self,
        company: &str,
        invitation_id: &str,
    ) -> Result<Value> {
        let invitation = self
            .authority
            .find_body(
                company,
                "publication_invitation",
                "invitation_id",
                invitation_id,
            )
            .await?
            .context("invitation does not exist")?;
        let publication_id = required_str(&invitation, "publication_id")?;
        let digest = required_str(&invitation, "token_digest")?;
        let request = self.request_by_publication(company, publication_id).await?;
        let directory = self.publication_directory(company, publication_id)?;
        if directory.is_dir() {
            append_line_once(&directory.join("revoked.sha256"), digest)?;
        }
        let body = json!({
            "publication_id": publication_id,
            "invitation_id": invitation_id,
            "token_digest": digest,
            "candidate_digest": request.candidate.manifest_digest,
            "revoked_at": Utc::now(),
        });
        let record_id = insert_invitation_once(
            self.authority.pool(),
            company,
            "publication_invitation_revoked",
            invitation_id,
            &body,
        )
        .await?;
        Ok(
            json!({"revocation_record_id": record_id, "revoked": true, "invitation_id": invitation_id}),
        )
    }

    pub(crate) async fn observe(&self, company: &str, publication_id: &str) -> Result<Value> {
        let request = self.request_by_publication(company, publication_id).await?;
        let directory = self.publication_directory(company, publication_id)?;
        let marker = load_marker(&directory.join("ready.json"))?;
        let observations: ServiceObservations =
            serde_json::from_slice(&std::fs::read(directory.join("observations.json"))?)
                .context("decode published-service observations")?;
        let active = process_is_fixture(marker.receipt.provider_process_id)?;
        let body = json!({
            "publication_id": publication_id,
            "candidate_digest": request.candidate.manifest_digest,
            "provider_process_id": marker.receipt.provider_process_id,
            "active": active,
            "observations": observations,
            "observed_at": Utc::now(),
        });
        let record_id = self
            .authority
            .emit(
                company,
                "publication_observation",
                Some("authority"),
                body.clone(),
            )
            .await?;
        Ok(json!({"observation_record_id": record_id, "observation": body}))
    }

    pub(crate) async fn reconcile(
        &self,
        org: &OrgIntel,
        company: &str,
        publication_id: &str,
    ) -> Result<Value> {
        let request = self.request_by_publication(company, publication_id).await?;
        if self
            .has_record(company, "publication_stopped", publication_id)
            .await?
        {
            return self.cleanup_stopped(company, publication_id).await;
        }
        if request.expires_at <= Utc::now() {
            return self
                .stop_as(
                    org,
                    company,
                    publication_id,
                    "publication expired",
                    "authority",
                )
                .await;
        }
        if self
            .ensure_candidate_available(org, company, publication_id)
            .await
            .is_err()
        {
            return self
                .stop_as(
                    org,
                    company,
                    publication_id,
                    "publication candidate was superseded",
                    "authority",
                )
                .await;
        }
        if !self
            .has_record(company, "publication_authorized", publication_id)
            .await?
        {
            return Ok(
                json!({"publication_id": publication_id, "status": "awaiting_owner_authorization"}),
            );
        }
        self.authorize(org, company, publication_id).await
    }

    pub(crate) async fn stop(
        &self,
        org: &OrgIntel,
        company: &str,
        publication_id: &str,
        reason: &str,
    ) -> Result<Value> {
        self.stop_as(org, company, publication_id, reason, "owner")
            .await
    }

    async fn stop_as(
        &self,
        org: &OrgIntel,
        company: &str,
        publication_id: &str,
        reason: &str,
        actor: &str,
    ) -> Result<Value> {
        let request = self.request_by_publication(company, publication_id).await?;
        if reason.trim().is_empty() {
            bail!("publication stop needs a reason");
        }
        let directory = self.publication_directory(company, publication_id)?;
        let marker_path = directory.join("ready.json");
        let marker = load_marker(&marker_path).ok();
        if let Some(marker) = marker.as_ref() {
            self.validate_marker(&request, marker)?;
            stop_fixture_process_for_directory(marker.receipt.provider_process_id, &directory)
                .await?;
            verify_port_released(marker)?;
        } else if directory.exists() {
            // A torn or deleted receipt is not evidence of absence. The
            // process command retains its one-shot handoff path even after the
            // file is removed, so exact-directory discovery can still close
            // the bounded listener without guessing a PID or starting a
            // duplicate.
            for pid in fixture_processes_for_directory(&directory)? {
                stop_fixture_process_for_directory(pid, &directory).await?;
            }
        }
        let stopped = json!({
            "publication_id": publication_id,
            "candidate_digest": request.candidate.manifest_digest,
            "reason": reason,
            "stopped_at": Utc::now(),
            "provider_process_absent": fixture_processes_for_directory(&directory)?.is_empty(),
        });
        let stopped_id = insert_once(
            self.authority.pool(),
            company,
            "publication_stopped",
            actor,
            &stopped,
        )
        .await?;
        if directory.exists() {
            std::fs::remove_dir_all(&directory).with_context(|| {
                format!("remove exact publication directory {}", directory.display())
            })?;
        }
        let cleanup = ProviderCleanupReceipt {
            contract_version: CONTRACT_VERSION.to_string(),
            publication_id: publication_id.to_string(),
            candidate_digest: request.candidate.manifest_digest.clone(),
            provider_process_absent: true,
            route_absent: true,
            invitation_material_absent: true,
            resource_lease_released: true,
            temporary_files_absent: !directory.exists(),
            cleaned_at: Utc::now(),
        };
        let cleanup_body = serde_json::to_value(&cleanup)?;
        let cleanup_id = insert_once(
            self.authority.pool(),
            company,
            "publication_cleanup",
            "authority",
            &cleanup_body,
        )
        .await?;
        org.emit_event(
            "publication_stopped",
            Some("authority"),
            json!({
                "authority_record_id": stopped_id,
                "cleanup_record_id": cleanup_id,
                "publication_id": publication_id,
            }),
        )
        .await?;
        Ok(json!({
            "publication_id": publication_id,
            "status": "stopped",
            "stopped_record_id": stopped_id,
            "cleanup_record_id": cleanup_id,
            "cleanup": cleanup,
        }))
    }

    pub(crate) async fn show(&self, company: &str, publication_id: Option<&str>) -> Result<Value> {
        let mut kinds = serde_json::Map::new();
        for kind in [
            "publication_request",
            "publication_authorized",
            "publication_resource_grant",
            "publication_ready",
            "publication_recovered",
            "publication_failed",
            "publication_observation",
            "publication_invitation",
            "publication_invitation_revoked",
            "publication_stopped",
            "publication_cleanup",
        ] {
            let values: Vec<Value> = self
                .authority
                .records_of_kind(company, kind)
                .await?
                .into_iter()
                .filter(|record| {
                    publication_id.is_none_or(|expected| {
                        record.body.get("publication_id").and_then(Value::as_str) == Some(expected)
                    })
                })
                .map(|record| {
                    json!({
                        "authority_record_id": record.id,
                        "actor": record.actor_id,
                        "created_at": record.created_at,
                        "body": record.body,
                    })
                })
                .collect();
            kinds.insert(kind.to_string(), Value::Array(values));
        }
        Ok(Value::Object(kinds))
    }

    /// Reconcile every locally authorized publication once during daemon boot.
    /// A provider process may deliberately outlive the daemon; its exact
    /// marker/receipt makes this a read-before-restart path, not a blind replay.
    pub(crate) async fn reconcile_company(
        &self,
        org: &OrgIntel,
        company: &str,
    ) -> Result<Vec<Value>> {
        if std::env::var(PROVIDER_ENV).as_deref() != Ok(LOCAL_PROVIDER) {
            return Ok(Vec::new());
        }
        let mut outcomes = Vec::new();
        for record in self
            .authority
            .records_of_kind(company, "publication_request")
            .await?
        {
            let Some(publication_id) = record.body.get("publication_id").and_then(Value::as_str)
            else {
                continue;
            };
            if self
                .has_record(company, "publication_authorized", publication_id)
                .await?
            {
                outcomes.push(self.reconcile(org, company, publication_id).await?);
            }
        }
        Ok(outcomes)
    }

    async fn cleanup_stopped(&self, company: &str, publication_id: &str) -> Result<Value> {
        let directory = self.publication_directory(company, publication_id)?;
        if directory.exists() {
            bail!("stopped publication still has provider state; owner stop must repair it");
        }
        let cleanup = self
            .authority
            .find_body(
                company,
                "publication_cleanup",
                "publication_id",
                publication_id,
            )
            .await?;
        Ok(json!({
            "publication_id": publication_id,
            "status": "stopped",
            "cleanup": cleanup,
        }))
    }

    async fn request_by_publication(
        &self,
        company: &str,
        publication_id: &str,
    ) -> Result<PublishRequest> {
        let body = self
            .authority
            .find_body(
                company,
                "publication_request",
                "publication_id",
                publication_id,
            )
            .await?
            .context("publication request does not exist")?;
        serde_json::from_value(
            body.get("request")
                .cloned()
                .context("publication request body is malformed")?,
        )
        .context("decode publication request")
    }

    async fn ensure_candidate_available(
        &self,
        org: &OrgIntel,
        company: &str,
        publication_id: &str,
    ) -> Result<()> {
        let body = self
            .authority
            .find_body(
                company,
                "publication_request",
                "publication_id",
                publication_id,
            )
            .await?
            .context("publication request does not exist")?;
        let candidate_id = required_str(&body, "candidate_artifact_ref_id")?;
        let candidate_id =
            Uuid::parse_str(candidate_id).context("candidate artifact id is malformed")?;
        let artifact = org
            .get_artifact_ref(candidate_id)
            .await?
            .context("candidate artifact is missing")?;
        if artifact.kind != "published_service_candidate"
            || artifact.state != ArtifactRefState::Available
        {
            bail!("publication candidate is superseded or unavailable");
        }
        Ok(())
    }

    async fn has_record(&self, company: &str, kind: &str, publication_id: &str) -> Result<bool> {
        Ok(self
            .authority
            .find_body(company, kind, "publication_id", publication_id)
            .await?
            .is_some())
    }

    async fn latest_provider_receipt_record(
        &self,
        company: &str,
        publication_id: &str,
    ) -> Result<Option<i64>> {
        sqlx::query_scalar(
            "SELECT id FROM restless_authority.records WHERE company=$1 \
             AND kind IN ('publication_ready','publication_recovered') \
             AND body->>'publication_id'=$2 ORDER BY id DESC LIMIT 1",
        )
        .bind(company)
        .bind(publication_id)
        .fetch_optional(self.authority.pool())
        .await
        .context("read latest provider receipt")
    }

    async fn latest_provider_failure_record(
        &self,
        company: &str,
        publication_id: &str,
    ) -> Result<Option<i64>> {
        sqlx::query_scalar(
            "SELECT id FROM restless_authority.records WHERE company=$1 \
             AND kind='publication_failed' AND body->>'publication_id'=$2 \
             ORDER BY id DESC LIMIT 1",
        )
        .bind(company)
        .bind(publication_id)
        .fetch_optional(self.authority.pool())
        .await
        .context("read latest provider failure")
    }

    async fn record_provider_failure(
        &self,
        company: &str,
        request: &PublishRequest,
        error: &anyhow::Error,
    ) -> Result<()> {
        let error = format!("{error:#}");
        let failure_key = json_digest(&json!({
            "publication_id": request.publication_id,
            "candidate_digest": request.candidate.manifest_digest,
            "error": error,
        }))?;
        let body = json!({
            "publication_id": request.publication_id,
            "candidate_digest": request.candidate.manifest_digest,
            "provider_operation_id": request.publication_id,
            "failure_key": failure_key,
            "error": error,
            "failed_at": Utc::now(),
        });
        let _ = sqlx::query_scalar::<_, i64>(
            "INSERT INTO restless_authority.records (company,kind,actor_id,body) \
             VALUES ($1,'publication_failed','authority',$2) \
             ON CONFLICT DO NOTHING RETURNING id",
        )
        .bind(company)
        .bind(body)
        .fetch_optional(self.authority.pool())
        .await?;
        Ok(())
    }

    fn ensure_local_provider_allowed(&self, company: &str) -> Result<()> {
        if std::env::var(PROVIDER_ENV).as_deref() != Ok(LOCAL_PROVIDER) {
            bail!(
                "no published-service provider is configured; Core's local fixture requires \
                 {PROVIDER_ENV}={LOCAL_PROVIDER}, while real public ingress belongs to Cloud 14"
            );
        }
        if !company.ends_with("_test") {
            bail!("the local publication provider is restricted to throwaway _test companies");
        }
        Ok(())
    }

    async fn ensure_fixture(
        &self,
        company: &str,
        request: &PublishRequest,
    ) -> Result<(LocalFixtureMarker, FixtureDisposition)> {
        let directory = self.publication_directory(company, &request.publication_id)?;
        std::fs::create_dir_all(&directory)
            .with_context(|| format!("create {}", directory.display()))?;
        let marker_path = directory.join("ready.json");
        let marker_existed = marker_path.exists();
        let replacing_dead_fixture = if let Ok(marker) = load_marker(&marker_path) {
            self.validate_marker(request, &marker)?;
            if process_is_fixture_for_directory(marker.receipt.provider_process_id, &directory)? {
                return Ok((marker, FixtureDisposition::Existing));
            }
            true
        } else {
            marker_existed
                || self
                    .latest_provider_receipt_record(company, &request.publication_id)
                    .await?
                    .is_some()
        };
        let unaccounted = fixture_processes_for_directory(&directory)?;
        if !unaccounted.is_empty() {
            bail!(
                "provider receipt is ambiguous: fixture process(es) {:?} still name {}, so no duplicate will be started",
                unaccounted,
                directory.display()
            );
        }
        let _ = std::fs::remove_file(&marker_path);
        let key = self.invitation_key(
            company,
            &request.publication_id,
            &request.candidate.manifest_digest,
        )?;
        let config_path = directory.join("handoff.json");
        let observations_path = directory.join("observations.json");
        let revocations_path = directory.join("revoked.sha256");
        if !revocations_path.exists() {
            std::fs::write(&revocations_path, b"")?;
        }
        let config = LocalFixtureConfig {
            company: company.to_string(),
            publication_id: request.publication_id.clone(),
            candidate_digest: request.candidate.manifest_digest.clone(),
            provider_operation_id: request.publication_id.clone(),
            profile: request.candidate.manifest.profile,
            manifest: request.candidate.manifest.clone(),
            audience: request.audience,
            bind_host: "127.0.0.1".into(),
            expires_at: request.expires_at,
            invitation_key_base64: base64::engine::general_purpose::STANDARD.encode(key),
            marker_path: marker_path.clone(),
            observations_path,
            revocations_path,
        };
        if config_path.exists() {
            std::fs::remove_file(&config_path)
                .with_context(|| format!("remove stale handoff {}", config_path.display()))?;
        }
        write_private(&config_path, &serde_json::to_vec_pretty(&config)?)?;
        let binary = fixture_binary()?;
        let mut child = std::process::Command::new(&binary)
            .arg(&config_path)
            .current_dir(&directory)
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("start bounded provider fixture {}", binary.display()))?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            if let Ok(marker) = load_marker(&marker_path) {
                if marker.receipt.provider_process_id != child.id() {
                    let _ = child.kill();
                    bail!("provider ready marker names an unexpected process");
                }
                return Ok((
                    marker,
                    if replacing_dead_fixture {
                        FixtureDisposition::Restarted
                    } else {
                        FixtureDisposition::Started
                    },
                ));
            }
            if let Some(status) = child
                .try_wait()
                .context("observe provider fixture startup")?
            {
                bail!("provider fixture exited before readiness with {status}");
            }
            if tokio::time::Instant::now() >= deadline {
                let _ = child.kill();
                bail!("provider fixture did not publish its readiness event within 10 seconds");
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }

    fn invitation_key(
        &self,
        company: &str,
        publication_id: &str,
        candidate_digest: &str,
    ) -> Result<Vec<u8>> {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.signing_key)?;
        mac.update(company.as_bytes());
        mac.update(&[0]);
        mac.update(publication_id.as_bytes());
        mac.update(&[0]);
        mac.update(candidate_digest.as_bytes());
        Ok(mac.finalize().into_bytes().to_vec())
    }

    fn publication_directory(&self, company: &str, publication_id: &str) -> Result<PathBuf> {
        validate_path_segment("company", company)?;
        validate_path_segment("publication_id", publication_id)?;
        Ok(self.root.join(company).join(publication_id))
    }

    fn validate_marker(&self, request: &PublishRequest, marker: &LocalFixtureMarker) -> Result<()> {
        if marker.receipt.contract_version != CONTRACT_VERSION
            || marker.receipt.publication_id != request.publication_id
            || marker.receipt.provider_operation_id != request.publication_id
            || marker.receipt.candidate_digest != request.candidate.manifest_digest
            || marker.receipt.endpoint.profile != request.candidate.manifest.profile
            || marker.receipt.endpoint.bound_port == 0
        {
            bail!("provider marker does not match the exact authorized publication/build/profile");
        }
        Ok(())
    }
}

async fn insert_once(
    pool: &sqlx::PgPool,
    company: &str,
    kind: &str,
    actor: &str,
    body: &Value,
) -> Result<i64> {
    let inserted = sqlx::query_scalar::<_, i64>(
        "INSERT INTO restless_authority.records (company,kind,actor_id,body) \
         VALUES ($1,$2,$3,$4) ON CONFLICT DO NOTHING RETURNING id",
    )
    .bind(company)
    .bind(kind)
    .bind(actor)
    .bind(body)
    .fetch_optional(pool)
    .await?;
    if let Some(id) = inserted {
        return Ok(id);
    }
    sqlx::query_scalar(
        "SELECT id FROM restless_authority.records WHERE company=$1 AND kind=$2 \
         AND body->>'publication_id'=$3 ORDER BY id LIMIT 1",
    )
    .bind(company)
    .bind(kind)
    .bind(required_str(body, "publication_id")?)
    .fetch_one(pool)
    .await
    .with_context(|| format!("recover idempotent Authority {kind} record"))
}

async fn insert_invitation_once(
    pool: &sqlx::PgPool,
    company: &str,
    kind: &str,
    invitation_id: &str,
    body: &Value,
) -> Result<i64> {
    let inserted = sqlx::query_scalar::<_, i64>(
        "INSERT INTO restless_authority.records (company,kind,actor_id,body) \
         VALUES ($1,$2,'owner',$3) ON CONFLICT DO NOTHING RETURNING id",
    )
    .bind(company)
    .bind(kind)
    .bind(body)
    .fetch_optional(pool)
    .await?;
    if let Some(id) = inserted {
        return Ok(id);
    }
    sqlx::query_scalar(
        "SELECT id FROM restless_authority.records WHERE company=$1 AND kind=$2 \
         AND body->>'invitation_id'=$3 ORDER BY id LIMIT 1",
    )
    .bind(company)
    .bind(kind)
    .bind(invitation_id)
    .fetch_one(pool)
    .await
    .with_context(|| format!("recover idempotent Authority {kind} record"))
}

fn json_digest(value: &Value) -> Result<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(value)?)
    ))
}

fn required_str<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .with_context(|| format!("record needs string field {field}"))
}

fn validate_path_segment(name: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        bail!("{name} is not a safe bounded path segment");
    }
    Ok(())
}

fn load_or_create_key(path: &Path) -> Result<Vec<u8>> {
    match std::fs::read(path) {
        Ok(key) if key.len() >= 32 => return Ok(key),
        Ok(_) => bail!("publication signing key is too short"),
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            return Err(error).with_context(|| format!("read {}", path.display()));
        }
        Err(_) => {}
    }
    let mut key = vec![0_u8; 32];
    std::fs::File::open("/dev/urandom")
        .context("open operating-system random source")?
        .read_exact(&mut key)
        .context("read publication signing key")?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    match options.open(path) {
        Ok(mut file) => file.write_all(&key)?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return std::fs::read(path)
                .with_context(|| format!("read raced key {}", path.display()));
        }
        Err(error) => return Err(error).with_context(|| format!("create {}", path.display())),
    }
    Ok(key)
}

fn fixture_binary() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os(FIXTURE_BINARY_ENV) {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        bail!(
            "{FIXTURE_BINARY_ENV} does not name a file: {}",
            path.display()
        );
    }
    let path = std::env::current_exe()?
        .parent()
        .context("daemon executable has no parent")?
        .join("restless-published-service-fixture");
    if !path.is_file() {
        bail!(
            "published-service fixture is not built at {}; build the workspace or set {FIXTURE_BINARY_ENV}",
            path.display()
        );
    }
    Ok(path)
}

fn process_is_fixture(pid: u32) -> Result<bool> {
    Ok(process_command(pid)?
        .is_some_and(|command| command.contains("restless-published-service-fixture")))
}

fn process_is_fixture_for_directory(pid: u32, directory: &Path) -> Result<bool> {
    let directory = directory.to_string_lossy();
    Ok(process_command(pid)?.is_some_and(|command| {
        command.contains("restless-published-service-fixture")
            && command.contains(directory.as_ref())
    }))
}

fn process_command(pid: u32) -> Result<Option<String>> {
    let output = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()
        .context("inspect provider fixture process")?;
    if !output.status.success() {
        return Ok(None);
    }
    let command = String::from_utf8_lossy(&output.stdout);
    let command = command.trim();
    Ok((!command.is_empty()).then(|| command.to_string()))
}

#[cfg(test)]
async fn stop_fixture_process(pid: u32) -> Result<()> {
    if !process_is_fixture(pid)? {
        return Ok(());
    }
    let status = std::process::Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status()
        .context("terminate provider fixture")?;
    if !status.success() {
        bail!("failed to terminate provider fixture process {pid}");
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while process_is_fixture(pid)? {
        if tokio::time::Instant::now() >= deadline {
            bail!("provider fixture process {pid} did not terminate");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Ok(())
}

async fn stop_fixture_process_for_directory(pid: u32, directory: &Path) -> Result<()> {
    if !process_is_fixture_for_directory(pid, directory)? {
        if process_command(pid)?.is_some() {
            bail!(
                "refusing to terminate process {pid}: it is not the fixture for {}",
                directory.display()
            );
        }
        return Ok(());
    }
    let status = std::process::Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status()
        .context("terminate exact provider fixture")?;
    if !status.success() {
        bail!("failed to terminate provider fixture process {pid}");
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while process_is_fixture_for_directory(pid, directory)? {
        if tokio::time::Instant::now() >= deadline {
            bail!("provider fixture process {pid} did not terminate");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Ok(())
}

fn fixture_processes_for_directory(directory: &Path) -> Result<Vec<u32>> {
    let output = std::process::Command::new("ps")
        .args(["-axo", "pid=,command="])
        .output()
        .context("enumerate bounded provider fixture processes")?;
    if !output.status.success() {
        bail!("could not enumerate provider fixture processes");
    }
    let directory = directory.to_string_lossy();
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| {
            line.contains("restless-published-service-fixture") && line.contains(directory.as_ref())
        })
        .filter_map(|line| line.split_whitespace().next()?.parse::<u32>().ok())
        .collect())
}

fn verify_port_released(marker: &LocalFixtureMarker) -> Result<()> {
    let address = ("127.0.0.1", marker.receipt.endpoint.bound_port);
    match marker.receipt.endpoint.profile {
        ServiceProfile::HttpsWebsocketDemo => {
            let listener = std::net::TcpListener::bind(address)
                .context("published HTTPS port remains occupied after provider stop")?;
            drop(listener);
        }
        ServiceProfile::GodotEnetUdp => {
            let socket = std::net::UdpSocket::bind(address)
                .context("published UDP port remains occupied after provider stop")?;
            drop(socket);
        }
    }
    Ok(())
}

fn append_line_once(path: &Path, line: &str) -> Result<()> {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    if existing.lines().any(|existing| existing == line) {
        return Ok(());
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .with_context(|| format!("create one-shot handoff {}", path.display()))?;
    file.write_all(bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use restless_orgintel::{NewWork, WorkspaceSpec};

    use super::*;

    #[tokio::test]
    async fn candidate_request_and_authority_accounting_are_exact_and_idempotent() {
        let Ok(database_url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping publication Authority scenario");
            return;
        };
        let suffix = Uuid::new_v4().simple().to_string();
        let company = format!("pub_{}_test", &suffix[..16]);
        let root = std::env::temp_dir().join(format!("restless-publication-manager-{suffix}"));
        std::fs::create_dir_all(&root).unwrap();
        let authority = AuthorityStore::connect(&database_url).await.unwrap();
        let org = OrgIntel::ensure(&database_url, &company).await.unwrap();
        org.ensure_actor("game-builder", "staff", "game-developer", "Game Builder")
            .await
            .unwrap();
        let work_id = org
            .add_work(NewWork {
                owner_id: "game-builder",
                title: "Build immutable multiplayer demo",
                outcome: "A digest-addressed server image and bounded service manifest",
                goal_id: None,
                priority: 10,
                expected_artifact: "immutable OCI image",
                workspace: WorkspaceSpec::default(),
                attempt_limit: Some(1),
            })
            .await
            .unwrap();
        let attempt = org
            .claim_ready_work("publication-test")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(attempt.work.id, work_id);
        let image_digest = format!("sha256:{}", "a".repeat(64));
        let image = format!("registry.example/restless/game@{image_digest}");
        let source_id = org
            .link_work_artifact(NewArtifactRef {
                kind: "container_image",
                uri: &image,
                note: "built by the exact Attempt",
                created_by: "game-builder",
                work_id: Some(work_id),
                attempt_id: Some(attempt.attempt_id),
                digest: Some(&image_digest),
                source_commit: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
                runtime_generation: Some("runtime-generation-1"),
                label: "Game server image",
            })
            .await
            .unwrap();
        let manager = PublicationManager::new(&root, authority.clone()).unwrap();
        let manifest = ServiceManifest {
            contract_version: CONTRACT_VERSION.into(),
            image,
            profile: ServiceProfile::GodotEnetUdp,
            internal_port: 7777,
            readiness: restlessd::published_service_contract::ReadinessProbe::Udp {
                request: "RESTLESS_READY_V1".into(),
                response: "RESTLESS_READY_V1_OK".into(),
            },
        };
        let first_candidate = manager
            .create_candidate(
                &org,
                &company,
                "game-builder",
                &source_id.to_string(),
                manifest.clone(),
            )
            .await
            .unwrap();
        let replayed_candidate = manager
            .create_candidate(
                &org,
                &company,
                "game-builder",
                &source_id.to_string(),
                manifest,
            )
            .await
            .unwrap();
        assert_eq!(
            first_candidate["candidate_artifact_ref_id"],
            replayed_candidate["candidate_artifact_ref_id"]
        );
        assert_eq!(replayed_candidate["replayed"], true);
        let candidate_id = first_candidate["candidate_artifact_ref_id"]
            .as_str()
            .unwrap();
        let expires_at = Utc::now() + chrono::Duration::hours(1);
        let start_deadline = Utc::now() + chrono::Duration::minutes(10);
        let limits = ResourceLimits {
            cpu_millis: 500,
            memory_mib: 512,
            ephemeral_storage_mib: 512,
            max_connections: 8,
        };
        let first = manager
            .request(
                &org,
                &company,
                "game-builder",
                candidate_id,
                Audience::NamedInvitees,
                start_deadline,
                expires_at,
                limits.clone(),
                "publish-game-v1",
            )
            .await
            .unwrap();
        let replay = manager
            .request(
                &org,
                &company,
                "game-builder",
                candidate_id,
                Audience::NamedInvitees,
                start_deadline,
                expires_at,
                limits,
                "publish-game-v1",
            )
            .await
            .unwrap();
        assert_eq!(
            first["publication"]["publication_id"],
            replay["publication"]["publication_id"]
        );
        assert_eq!(replay["replayed"], true);
        let conflicting = manager
            .request(
                &org,
                &company,
                "game-builder",
                candidate_id,
                Audience::Public,
                start_deadline,
                expires_at,
                ResourceLimits {
                    cpu_millis: 500,
                    memory_mib: 512,
                    ephemeral_storage_mib: 512,
                    max_connections: 8,
                },
                "publish-game-v1",
            )
            .await;
        assert!(conflicting
            .unwrap_err()
            .to_string()
            .contains("different publication request"));

        let (racing_a, racing_b) = tokio::join!(
            manager.request(
                &org,
                &company,
                "game-builder",
                candidate_id,
                Audience::NamedInvitees,
                start_deadline,
                expires_at,
                ResourceLimits {
                    cpu_millis: 500,
                    memory_mib: 512,
                    ephemeral_storage_mib: 512,
                    max_connections: 8,
                },
                "publish-game-race",
            ),
            manager.request(
                &org,
                &company,
                "game-builder",
                candidate_id,
                Audience::NamedInvitees,
                start_deadline,
                expires_at,
                ResourceLimits {
                    cpu_millis: 500,
                    memory_mib: 512,
                    ephemeral_storage_mib: 512,
                    max_connections: 8,
                },
                "publish-game-race",
            )
        );
        let racing_a = racing_a.unwrap();
        let racing_b = racing_b.unwrap();
        assert_eq!(
            racing_a["publication"]["publication_id"],
            racing_b["publication"]["publication_id"]
        );
        assert_ne!(racing_a["replayed"], racing_b["replayed"]);

        let publication_id = first["publication"]["publication_id"]
            .as_str()
            .unwrap()
            .to_string();
        let grant = json!({"publication_id": "publication-accounting-test", "resources": {"cpu_millis": 500}});
        let first_grant = insert_once(
            authority.pool(),
            &company,
            "publication_resource_grant",
            "owner",
            &grant,
        )
        .await
        .unwrap();
        let replayed_grant = insert_once(
            authority.pool(),
            &company,
            "publication_resource_grant",
            "owner",
            &grant,
        )
        .await
        .unwrap();
        assert_eq!(
            first_grant, replayed_grant,
            "resource accounting must happen once"
        );

        if std::env::var(PROVIDER_ENV).as_deref() == Ok(LOCAL_PROVIDER)
            && std::env::var_os(FIXTURE_BINARY_ENV).is_some()
        {
            let fixture_binary = std::env::var_os(FIXTURE_BINARY_ENV).unwrap();
            std::env::set_var(FIXTURE_BINARY_ENV, root.join("missing-fixture-binary"));
            let provider_failure = manager
                .authorize(&org, &company, &publication_id)
                .await
                .unwrap_err();
            assert!(provider_failure
                .to_string()
                .contains("does not name a file"));
            assert_eq!(
                authority
                    .records_of_kind(&company, "publication_failed")
                    .await
                    .unwrap()
                    .into_iter()
                    .filter(|record| {
                        record.body.get("publication_id").and_then(Value::as_str)
                            == Some(publication_id.as_str())
                    })
                    .count(),
                1
            );
            std::env::set_var(FIXTURE_BINARY_ENV, fixture_binary);
            let ready = manager
                .authorize(&org, &company, &publication_id)
                .await
                .unwrap();
            assert_eq!(ready["replayed"], false);
            let marker_path = manager
                .publication_directory(&company, &publication_id)
                .unwrap()
                .join("ready.json");
            let first_marker = load_marker(&marker_path).unwrap();
            let invitation = manager
                .invite(
                    &org,
                    &company,
                    &publication_id,
                    "invite-player-1",
                    "player-1@example.com",
                    Utc::now() + chrono::Duration::minutes(10),
                )
                .await
                .unwrap();
            let token = invitation["token"].as_str().unwrap();
            let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
            let endpoint = ("127.0.0.1", first_marker.receipt.endpoint.bound_port);
            let packet =
                serde_json::to_vec(&json!({"token": token, "payload": "player-one"})).unwrap();
            socket.send_to(&packet, endpoint).await.unwrap();
            let mut buffer = [0_u8; 4096];
            let (length, _) = socket.recv_from(&mut buffer).await.unwrap();
            let response: Value = serde_json::from_slice(&buffer[..length]).unwrap();
            assert_eq!(response["ok"], true);
            manager
                .revoke_invitation(&company, "invite-player-1")
                .await
                .unwrap();
            socket.send_to(&packet, endpoint).await.unwrap();
            let (length, _) = socket.recv_from(&mut buffer).await.unwrap();
            let response: Value = serde_json::from_slice(&buffer[..length]).unwrap();
            assert_eq!(response["ok"], false);
            assert!(response["error"].as_str().unwrap().contains("revoked"));
            let observed = manager.observe(&company, &publication_id).await.unwrap();
            assert!(
                observed["observation"]["observations"]["accepted_connections"]
                    .as_u64()
                    .unwrap()
                    >= 1
            );

            // A torn receipt while the listener is still alive is ambiguous.
            // Reconciliation must refuse a duplicate, then recover normally
            // once the exact receipt is restored.
            std::fs::write(&marker_path, b"{").unwrap();
            let ambiguous = manager
                .reconcile(&org, &company, &publication_id)
                .await
                .unwrap_err();
            assert!(ambiguous.to_string().contains("ambiguous"));
            assert_eq!(
                fixture_processes_for_directory(
                    &manager
                        .publication_directory(&company, &publication_id)
                        .unwrap()
                )
                .unwrap(),
                vec![first_marker.receipt.provider_process_id]
            );
            std::fs::write(
                &marker_path,
                serde_json::to_vec_pretty(&first_marker).unwrap(),
            )
            .unwrap();
            let resolved = manager
                .reconcile(&org, &company, &publication_id)
                .await
                .unwrap();
            assert_eq!(resolved["recovered"], true);

            // A fresh manager represents a daemon restart. It adopts the live
            // provider without another ready receipt or resource grant.
            let restarted_manager = PublicationManager::new(&root, authority.clone()).unwrap();
            let adopted = restarted_manager
                .reconcile(&org, &company, &publication_id)
                .await
                .unwrap();
            assert_eq!(adopted["replayed"], true);

            // Provider death is different: the stable operation is restarted
            // and recorded as recovery, while authority/accounting stay once.
            stop_fixture_process(first_marker.receipt.provider_process_id)
                .await
                .unwrap();
            let recovered = restarted_manager
                .reconcile(&org, &company, &publication_id)
                .await
                .unwrap();
            assert_eq!(recovered["recovered"], true);
            let recovered_marker = load_marker(&marker_path).unwrap();
            assert_ne!(
                first_marker.receipt.provider_process_id,
                recovered_marker.receipt.provider_process_id
            );
            for kind in ["publication_authorized", "publication_resource_grant"] {
                let count = authority
                    .records_of_kind(&company, kind)
                    .await
                    .unwrap()
                    .into_iter()
                    .filter(|record| {
                        record.body.get("publication_id").and_then(Value::as_str)
                            == Some(publication_id.as_str())
                    })
                    .count();
                assert_eq!(count, 1, "{kind} must remain exactly once after recovery");
            }

            let expiring_start = Utc::now() + chrono::Duration::seconds(1);
            let expiring_end = Utc::now() + chrono::Duration::seconds(3);
            let expiring = restarted_manager
                .request(
                    &org,
                    &company,
                    "game-builder",
                    candidate_id,
                    Audience::NamedInvitees,
                    expiring_start,
                    expiring_end,
                    ResourceLimits {
                        cpu_millis: 500,
                        memory_mib: 512,
                        ephemeral_storage_mib: 512,
                        max_connections: 2,
                    },
                    "publish-game-expiry",
                )
                .await
                .unwrap();
            let expiring_id = expiring["publication"]["publication_id"].as_str().unwrap();
            restarted_manager
                .authorize(&org, &company, expiring_id)
                .await
                .unwrap();
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
            let expired = restarted_manager
                .reconcile(&org, &company, expiring_id)
                .await
                .unwrap();
            assert_eq!(expired["status"], "stopped");
            assert_eq!(expired["cleanup"]["route_absent"], true);

            org.retire_work_artifact(
                Uuid::parse_str(candidate_id).unwrap(),
                "game-builder",
                "a newer immutable build supersedes this candidate",
            )
            .await
            .unwrap();
            let stopped = restarted_manager
                .reconcile(&org, &company, &publication_id)
                .await
                .unwrap();
            assert_eq!(stopped["status"], "stopped");
            assert_eq!(stopped["cleanup"]["provider_process_absent"], true);
            assert_eq!(stopped["cleanup"]["route_absent"], true);
            assert!(!restarted_manager
                .publication_directory(&company, &publication_id)
                .unwrap()
                .exists());
        }

        authority.delete_test_company(&company).await.unwrap();
        org.drop_schema().await.unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
