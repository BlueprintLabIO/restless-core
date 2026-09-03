//! Source-owned company expression identity (S31).
//!
//! Evidence and proposals are ordinary recoverable OrgIntel state. A release
//! becomes effective only through an owner-authored decision carrying the
//! Authority record that admitted it. The compiler is deterministic and
//! bounded; it never turns conflicting facts into fluent certainty.

use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn nonempty(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(OrgIntelError::InvalidWork(format!(
            "company identity {label} cannot be empty"
        )));
    }
    Ok(())
}

pub(super) fn evidence_select() -> &'static str {
    "SELECT id,pillar,statement_kind,claim_key,statement,author_id,source,authority,scope,\
            observed_at,evidence_locator,polarity,status,channel,audience,supersedes_evidence_id,\
            exception_expires_at,exception_indefinite,created_at FROM company_identity_evidence"
}

fn release_select() -> &'static str {
    "SELECT id,predecessor,effective_from,promoted_by,authority_record_id,change_account,created_at \
     FROM company_identity_releases"
}

impl OrgIntel {
    pub async fn add_identity_evidence(&self, input: NewIdentityEvidence<'_>) -> Result<Uuid> {
        for (label, value) in [
            ("claim key", input.claim_key),
            ("statement", input.statement),
            ("author", input.author_id),
            ("source", input.source),
            ("authority", input.authority),
            ("scope", input.scope),
            ("evidence locator", input.evidence_locator),
        ] {
            nonempty(label, value)?;
        }
        if input.statement_kind == IdentityStatementKind::Exception
            && input.exception_expires_at.is_none()
            && !input.exception_indefinite
        {
            return Err(OrgIntelError::InvalidWork(
                "company identity exception needs an expiry or deliberate indefinite scope".into(),
            ));
        }
        if input.statement_kind != IdentityStatementKind::Exception
            && (input.exception_expires_at.is_some() || input.exception_indefinite)
        {
            return Err(OrgIntelError::InvalidWork(
                "only a company identity exception may carry exception lifetime".into(),
            ));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO company_identity_evidence \
             (id,pillar,statement_kind,claim_key,statement,author_id,source,authority,scope,\
              observed_at,evidence_locator,polarity,status,channel,audience,supersedes_evidence_id,\
              exception_expires_at,exception_indefinite) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)",
        )
        .bind(id)
        .bind(input.pillar)
        .bind(input.statement_kind)
        .bind(input.claim_key.trim())
        .bind(input.statement.trim())
        .bind(input.author_id)
        .bind(input.source.trim())
        .bind(input.authority.trim())
        .bind(input.scope.trim())
        .bind(input.observed_at)
        .bind(input.evidence_locator.trim())
        .bind(input.polarity)
        .bind(input.status)
        .bind(input.channel.map(str::trim))
        .bind(input.audience.map(str::trim))
        .bind(input.supersedes_evidence_id)
        .bind(input.exception_expires_at)
        .bind(input.exception_indefinite)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn propose_identity_release(
        &self,
        created_by: &str,
        rationale: &str,
        evidence_ids: &[Uuid],
    ) -> Result<Uuid> {
        nonempty("proposal author", created_by)?;
        nonempty("proposal rationale", rationale)?;
        if evidence_ids.is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "company identity proposal needs evidence".into(),
            ));
        }
        let unique = evidence_ids.iter().copied().collect::<BTreeSet<_>>();
        if unique.len() != evidence_ids.len() {
            return Err(OrgIntelError::InvalidWork(
                "company identity proposal repeats evidence".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let predecessor: Option<Uuid> = sqlx::query_scalar(
            "SELECT release_id FROM company_identity_current_release WHERE singleton=TRUE",
        )
        .fetch_optional(&mut *tx)
        .await?;
        let present: i64 =
            sqlx::query_scalar("SELECT count(*) FROM company_identity_evidence WHERE id = ANY($1)")
                .bind(evidence_ids)
                .fetch_one(&mut *tx)
                .await?;
        if present != evidence_ids.len() as i64 {
            return Err(OrgIntelError::InvalidWork(
                "company identity proposal cites missing evidence".into(),
            ));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO company_identity_proposals \
             (id,created_by,rationale,expected_predecessor) VALUES ($1,$2,$3,$4)",
        )
        .bind(id)
        .bind(created_by)
        .bind(rationale.trim())
        .bind(predecessor)
        .execute(&mut *tx)
        .await?;
        for evidence_id in evidence_ids {
            sqlx::query(
                "INSERT INTO company_identity_proposal_evidence (proposal_id,evidence_id) \
                 VALUES ($1,$2)",
            )
            .bind(id)
            .bind(evidence_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(id)
    }

    /// Promote a complete proposal. This is intentionally owner-only and
    /// requires the Authority record written by the authenticated entry path.
    pub async fn promote_identity_proposal(
        &self,
        proposal_id: Uuid,
        decided_by: &str,
        authority_record_id: &str,
        change_account: &str,
        effective_from: DateTime<Utc>,
    ) -> Result<Uuid> {
        if decided_by != "owner" {
            return Err(OrgIntelError::InvalidWork(
                "only the authenticated owner may promote company identity".into(),
            ));
        }
        nonempty("Authority record", authority_record_id)?;
        nonempty("change account", change_account)?;
        let mut tx = self.pool.begin().await?;
        let proposal = sqlx::query(
            "SELECT state,expected_predecessor FROM company_identity_proposals \
             WHERE id=$1 FOR UPDATE",
        )
        .bind(proposal_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidWork("company identity proposal not found".into()))?;
        if proposal.get::<IdentityProposalState, _>("state") != IdentityProposalState::Pending {
            return Err(OrgIntelError::InvalidWork(
                "company identity proposal is already decided".into(),
            ));
        }
        let expected: Option<Uuid> = proposal.get("expected_predecessor");
        let current: Option<Uuid> = sqlx::query_scalar(
            "SELECT release_id FROM company_identity_current_release WHERE singleton=TRUE FOR UPDATE",
        )
        .fetch_optional(&mut *tx)
        .await?;
        if current != expected {
            return Err(OrgIntelError::InvalidWork(
                "company identity proposal is stale against the effective release".into(),
            ));
        }

        let conflicts = sqlx::query(
            "SELECT e.claim_key FROM company_identity_proposal_evidence pe \
             JOIN company_identity_evidence e ON e.id=pe.evidence_id \
             WHERE pe.proposal_id=$1 AND e.pillar='truth' AND e.statement_kind='fact' \
               AND e.status='active' \
             GROUP BY e.claim_key HAVING count(DISTINCT e.statement) > 1",
        )
        .bind(proposal_id)
        .fetch_all(&mut *tx)
        .await?;
        if !conflicts.is_empty() {
            let keys = conflicts
                .iter()
                .map(|row| row.get::<String, _>("claim_key"))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(OrgIntelError::InvalidWork(format!(
                "company identity has unresolved truth conflicts: {keys}"
            )));
        }
        let expired_exception: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM company_identity_proposal_evidence pe \
             JOIN company_identity_evidence e ON e.id=pe.evidence_id \
             WHERE pe.proposal_id=$1 AND e.statement_kind='exception' \
               AND NOT e.exception_indefinite AND e.exception_expires_at <= $2)",
        )
        .bind(proposal_id)
        .bind(effective_from)
        .fetch_one(&mut *tx)
        .await?;
        if expired_exception {
            return Err(OrgIntelError::InvalidWork(
                "company identity proposal contains an expired exception".into(),
            ));
        }

        let release_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO company_identity_releases \
             (id,predecessor,effective_from,promoted_by,authority_record_id,change_account) \
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(release_id)
        .bind(current)
        .bind(effective_from)
        .bind(decided_by)
        .bind(authority_record_id.trim())
        .bind(change_account.trim())
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO company_identity_release_evidence (release_id,evidence_id) \
             SELECT $2,evidence_id FROM company_identity_proposal_evidence WHERE proposal_id=$1",
        )
        .bind(proposal_id)
        .bind(release_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO company_identity_current_release (singleton,release_id) VALUES (TRUE,$1) \
             ON CONFLICT (singleton) DO UPDATE SET release_id=EXCLUDED.release_id",
        )
        .bind(release_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE company_identity_proposals SET state='promoted',decided_by=$2,\
             authority_record_id=$3,decision_rationale=$4,decided_at=now() WHERE id=$1",
        )
        .bind(proposal_id)
        .bind(decided_by)
        .bind(authority_record_id.trim())
        .bind(change_account.trim())
        .execute(&mut *tx)
        .await?;

        // A correction does not rewrite old releases or bindings. It marks the
        // affected historical outcomes discoverably stale.
        sqlx::query(
            "UPDATE company_identity_work_bindings b SET stale_at=COALESCE(b.stale_at,now()), \
             stale_reason=CASE WHEN b.stale_reason='' THEN 'bound identity evidence was corrected' \
                               ELSE b.stale_reason END \
             WHERE EXISTS (\
               SELECT 1 FROM company_identity_proposal_evidence pe \
               JOIN company_identity_evidence replacement ON replacement.id=pe.evidence_id \
               JOIN company_identity_release_evidence old ON old.evidence_id=replacement.supersedes_evidence_id \
               WHERE pe.proposal_id=$1 AND old.release_id=b.release_id\
             )",
        )
        .bind(proposal_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO decisions (id,title,body,decided_by) VALUES ($1,$2,$3,$4)")
            .bind(Uuid::new_v4())
            .bind("Promote company identity release")
            .bind(format!(
            "Promoted release {release_id} from proposal {proposal_id}; Authority record {}. {}",
            authority_record_id.trim(),
            change_account.trim()
        ))
            .bind(decided_by)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(release_id)
    }

    pub async fn reject_identity_proposal(
        &self,
        proposal_id: Uuid,
        decided_by: &str,
        authority_record_id: &str,
        rationale: &str,
    ) -> Result<()> {
        if decided_by != "owner" {
            return Err(OrgIntelError::InvalidWork(
                "only the authenticated owner may reject company identity".into(),
            ));
        }
        nonempty("Authority record", authority_record_id)?;
        nonempty("rejection rationale", rationale)?;
        let changed = sqlx::query(
            "UPDATE company_identity_proposals SET state='rejected',decided_by=$2,\
             authority_record_id=$3,decision_rationale=$4,decided_at=now() \
             WHERE id=$1 AND state='pending'",
        )
        .bind(proposal_id)
        .bind(decided_by)
        .bind(authority_record_id.trim())
        .bind(rationale.trim())
        .execute(&self.pool)
        .await?
        .rows_affected();
        if changed != 1 {
            return Err(OrgIntelError::InvalidWork(
                "company identity proposal is missing or already decided".into(),
            ));
        }
        Ok(())
    }

    pub async fn bind_work_identity(&self, work_id: Uuid) -> Result<IdentityWorkBindingRow> {
        let release_id: Uuid = sqlx::query_scalar(
            "SELECT release_id FROM company_identity_current_release WHERE singleton=TRUE",
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork("company has no released expression identity".into())
        })?;
        sqlx::query(
            "INSERT INTO company_identity_work_bindings (work_id,release_id) VALUES ($1,$2) \
             ON CONFLICT (work_id) DO NOTHING",
        )
        .bind(work_id)
        .bind(release_id)
        .execute(&self.pool)
        .await?;
        let binding = sqlx::query_as::<_, IdentityWorkBindingRow>(
            "SELECT work_id,release_id,bound_at,stale_at,stale_reason \
             FROM company_identity_work_bindings WHERE work_id=$1",
        )
        .bind(work_id)
        .fetch_one(&self.pool)
        .await?;
        if binding.release_id != release_id {
            // Set-once is the point: a later release never rewrites history.
            return Ok(binding);
        }
        Ok(binding)
    }

    pub async fn company_identity_snapshot(&self) -> Result<CompanyIdentitySnapshot> {
        let current_release = sqlx::query_as::<_, IdentityReleaseRow>(&format!(
            "{} WHERE id=(SELECT release_id FROM company_identity_current_release WHERE singleton=TRUE)",
            release_select()
        ))
        .fetch_optional(&self.pool)
        .await?;
        let releases = sqlx::query_as::<_, IdentityReleaseRow>(&format!(
            "{} ORDER BY effective_from DESC,id DESC",
            release_select()
        ))
        .fetch_all(&self.pool)
        .await?;
        let pending_proposals = sqlx::query_as::<_, IdentityProposalRow>(
            "SELECT id,created_by,rationale,expected_predecessor,state,decided_by,authority_record_id,\
             decision_rationale,created_at,decided_at FROM company_identity_proposals \
             WHERE state='pending' ORDER BY created_at,id",
        )
        .fetch_all(&self.pool)
        .await?;
        let evidence = sqlx::query_as::<_, IdentityEvidenceRow>(&format!(
            "{} ORDER BY pillar,statement_kind,claim_key,id",
            evidence_select()
        ))
        .fetch_all(&self.pool)
        .await?;
        let proposal_evidence = sqlx::query_as::<_, IdentityProposalEvidenceRow>(
            "SELECT proposal_id,evidence_id FROM company_identity_proposal_evidence \
             ORDER BY proposal_id,evidence_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let release_evidence = sqlx::query_as::<_, IdentityReleaseEvidenceRow>(
            "SELECT release_id,evidence_id FROM company_identity_release_evidence \
             ORDER BY release_id,evidence_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let bindings = sqlx::query_as::<_, IdentityWorkBindingRow>(
            "SELECT work_id,release_id,bound_at,stale_at,stale_reason \
             FROM company_identity_work_bindings ORDER BY bound_at,work_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let voice_evidence_details = sqlx::query_as::<_, VoiceEvidenceDetailRow>(
            "SELECT evidence_id,kind,judgement_reason,named_author,channel,audience \
             FROM company_voice_evidence_details ORDER BY kind,evidence_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let voice_work_contracts = sqlx::query_as::<_, VoiceWorkContractRow>(
            "SELECT work_id,release_id,channel,author,bound_by,audience,reader_situation,desired_understanding,\
             desired_action,proof,consequence,contract_digest,bound_at \
             FROM company_voice_work_contracts ORDER BY bound_at,work_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let voice_render_evidence = sqlx::query_as::<_, VoiceRenderEvidenceRow>(
            "SELECT id,artifact_ref_id,channel,renderer,renderer_version,semantic_checks,captured_by,\
             captured_at FROM company_voice_render_evidence ORDER BY captured_at,id",
        )
        .fetch_all(&self.pool)
        .await?;
        let voice_reviews = sqlx::query_as::<_, VoiceReviewRow>(
            "SELECT id,render_evidence_id,reviewer,verdict,factual_findings,abstraction_findings,\
             repetition_findings,channel_findings,authorship_findings,concepts_removed,created_at \
             FROM company_voice_reviews ORDER BY created_at,id",
        )
        .fetch_all(&self.pool)
        .await?;
        let visual_evidence_details = sqlx::query_as::<_, VisualEvidenceDetailRow>(
            "SELECT evidence_id,kind,channel,purpose,rationale,semantic_role,value,reduced_motion_replacement,\
             product_truth_locator,origin,licence,framework,dependencies,adaptation_status,accessibility_notes \
             FROM company_visual_evidence_details ORDER BY kind,evidence_id",
        ).fetch_all(&self.pool).await?;
        let visual_work_contracts = sqlx::query_as::<_, VisualWorkContractRow>(
            "SELECT work_id,release_id,channel,bound_by,audience,outcome,information_hierarchy,proof,density,\
             imagery_role,motion_role,product_representation,product_truth_locator,requested_departure,\
             contract_digest,bound_at FROM company_visual_work_contracts ORDER BY bound_at,work_id",
        ).fetch_all(&self.pool).await?;
        let visual_primitive_uses = sqlx::query_as::<_, VisualPrimitiveUseRow>(
            "SELECT work_id,evidence_id,primitive_version,purpose FROM company_visual_primitive_uses ORDER BY work_id,evidence_id",
        ).fetch_all(&self.pool).await?;
        let visual_render_evidence = sqlx::query_as::<_, VisualRenderEvidenceRow>(
            "SELECT id,work_id,artifact_ref_id,channel,renderer,renderer_version,viewport_width,viewport_height,\
             motion_state,native_checks,captured_by,captured_at FROM company_visual_render_evidence ORDER BY captured_at,id",
        ).fetch_all(&self.pool).await?;
        let visual_reviews = sqlx::query_as::<_, VisualReviewRow>(
            "SELECT id,render_evidence_id,control_render_evidence_id,reviewer,verdict,identity_findings,\
             hierarchy_findings,density_findings,proof_findings,product_fidelity_findings,motion_findings,\
             defect_findings,departure_decision,created_at FROM company_visual_reviews ORDER BY created_at,id",
        ).fetch_all(&self.pool).await?;
        let culture_evidence_details=sqlx::query_as::<_,CultureEvidenceDetailRow>("SELECT evidence_id,kind,case_kind,situation,consequence,actors,decision_authority,conduct,observed_outcome,confidence,counterexample,boundary_conditions,operational_implication,actor_scope FROM company_culture_evidence_details ORDER BY kind,evidence_id").fetch_all(&self.pool).await?;
        let culture_work_contracts=sqlx::query_as::<_,CultureWorkContractRow>("SELECT work_id,release_id,case_kind,actor,actor_role,team,consequence,decision_boundary,bound_by,contract_digest,bound_at FROM company_culture_work_contracts ORDER BY bound_at,work_id").fetch_all(&self.pool).await?;
        let culture_case_records=sqlx::query_as::<_,CultureCaseRecordRow>("SELECT id,work_id,artifact_ref_id,case_kind,decision,alternatives,unknowns,correction_of,correction_account,customer_action,native_checks,recorded_by,recorded_at FROM company_culture_case_records ORDER BY recorded_at,id").fetch_all(&self.pool).await?;
        let culture_reviews=sqlx::query_as::<_,CultureReviewRow>("SELECT id,case_record_id,reviewer,verdict,conduct_findings,dissent_findings,uncertainty_findings,correction_findings,authority_findings,customer_or_hiring_findings,slogan_recitation_detected,created_at FROM company_culture_reviews ORDER BY created_at,id").fetch_all(&self.pool).await?;
        let constitution_artifact_bindings = sqlx::query_as::<_, ConstitutionArtifactBindingRow>(
            "SELECT artifact_ref_id,work_id,release_id,channel,audience,named_author,producer,accountable_lead,company_voice,native_evidence,constitution_digest,bound_at FROM company_constitution_artifact_bindings ORDER BY bound_at,artifact_ref_id",
        ).fetch_all(&self.pool).await?;
        let constitution_artifact_evidence = sqlx::query_as::<_, ConstitutionArtifactEvidenceRow>(
            "SELECT artifact_ref_id,evidence_id FROM company_constitution_artifact_evidence ORDER BY artifact_ref_id,evidence_id",
        ).fetch_all(&self.pool).await?;
        let constitution_learning_proposals = sqlx::query_as::<_, ConstitutionLearningProposalRow>(
            "SELECT proposal_id,evidence_id,pillar,trigger_kind,triggering_event,before_artifact_ref_id,after_artifact_ref_id,scope,contradiction_check,created_at FROM company_constitution_learning_proposals ORDER BY created_at,proposal_id",
        ).fetch_all(&self.pool).await?;
        let identity_drift_findings = sqlx::query_as::<_, IdentityDriftFindingRow>(
            "SELECT id,artifact_ref_id,from_release_id,to_release_id,kind,old_evidence_id,new_evidence_id,dependency,consequence,created_at FROM company_identity_drift_findings ORDER BY created_at,id",
        ).fetch_all(&self.pool).await?;
        let identity_migration_decisions = sqlx::query_as::<_, IdentityMigrationDecisionRow>(
            "SELECT drift_finding_id,disposition,decided_by,rationale,authority_record_id,decided_at FROM company_identity_migration_decisions ORDER BY decided_at,drift_finding_id",
        ).fetch_all(&self.pool).await?;
        Ok(CompanyIdentitySnapshot {
            current_release,
            releases,
            pending_proposals,
            evidence,
            proposal_evidence,
            release_evidence,
            bindings,
            voice_evidence_details,
            voice_work_contracts,
            voice_render_evidence,
            voice_reviews,
            visual_evidence_details,
            visual_work_contracts,
            visual_primitive_uses,
            visual_render_evidence,
            visual_reviews,
            culture_evidence_details,
            culture_work_contracts,
            culture_case_records,
            culture_reviews,
            constitution_artifact_bindings,
            constitution_artifact_evidence,
            constitution_learning_proposals,
            identity_drift_findings,
            identity_migration_decisions,
        })
    }

    pub async fn compile_identity_brief(
        &self,
        request: IdentityBriefRequest<'_>,
    ) -> Result<IdentityBrief> {
        for (label, value) in [
            ("outcome", request.outcome),
            ("channel", request.channel),
            ("audience", request.audience),
            ("author", request.author),
        ] {
            nonempty(label, value)?;
        }
        if request.max_bytes < 512 {
            return Err(OrgIntelError::InvalidWork(
                "company identity brief bound must be at least 512 bytes".into(),
            ));
        }
        let release_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM company_identity_releases WHERE id=$1)",
        )
        .bind(request.release_id)
        .fetch_one(&self.pool)
        .await?;
        if !release_exists {
            return Err(OrgIntelError::InvalidWork(
                "company identity release not found".into(),
            ));
        }
        let rows = sqlx::query_as::<_, IdentityEvidenceRow>(&format!(
            "{} e JOIN company_identity_release_evidence re ON re.evidence_id=e.id \
             WHERE re.release_id=$1 AND e.status <> 'corrected' \
               AND (e.scope='company' OR e.scope=$2) \
               AND (e.channel IS NULL OR e.channel=$3) \
               AND (e.audience IS NULL OR e.audience=$4) \
               AND (e.statement_kind <> 'exception' OR e.exception_indefinite \
                    OR e.exception_expires_at>$5) \
             ORDER BY e.pillar,e.statement_kind,e.claim_key,e.id",
            evidence_select()
        ))
        .bind(request.release_id)
        .bind(format!("outcome:{}", request.outcome.trim()))
        .bind(request.channel.trim())
        .bind(request.audience.trim())
        .bind(request.now)
        .fetch_all(&self.pool)
        .await?;

        let mut facts = BTreeMap::<String, BTreeSet<String>>::new();
        for row in &rows {
            if row.pillar == IdentityPillar::Truth
                && row.statement_kind == IdentityStatementKind::Fact
                && row.status == IdentityEvidenceStatus::Active
            {
                facts
                    .entry(row.claim_key.clone())
                    .or_default()
                    .insert(row.statement.clone());
            }
        }
        let conflict_keys = facts
            .into_iter()
            .filter_map(|(key, statements)| (statements.len() > 1).then_some(key))
            .collect::<Vec<_>>();
        if !conflict_keys.is_empty() {
            return Err(OrgIntelError::InvalidWork(format!(
                "company identity brief blocked by conflicting truth: {}",
                conflict_keys.join(", ")
            )));
        }

        let header = format!(
            "# Company Identity Brief\nrelease: {}\noutcome: {}\nchannel: {}\naudience: {}\nauthor: {}\n\n",
            request.release_id,
            request.outcome.trim(),
            request.channel.trim(),
            request.audience.trim(),
            request.author.trim()
        );
        let lines = rows
            .iter()
            .map(|row| {
                format!(
                    "- [{:?}/{:?}/{:?}] {} — source: {}; evidence: {}\n",
                    row.pillar,
                    row.statement_kind,
                    row.polarity,
                    row.statement,
                    row.source,
                    row.evidence_locator
                )
            })
            .collect::<Vec<_>>();
        let mut included = Vec::new();
        for index in 0..rows.len() {
            let tentative_included = rows[..=index].iter().map(|row| row.id).collect::<Vec<_>>();
            let omitted = rows[index + 1..]
                .iter()
                .map(|row| row.id)
                .collect::<Vec<_>>();
            let body = render_brief(&header, &lines[..=index], &omitted);
            if body.len() <= request.max_bytes {
                included = tentative_included;
            } else {
                break;
            }
        }
        let included_count = included.len();
        let omitted = rows[included_count..]
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>();
        let body = render_brief(&header, &lines[..included_count], &omitted);
        if body.len() > request.max_bytes {
            return Err(OrgIntelError::InvalidWork(format!(
                "company identity brief metadata and omission account exceed {} bytes",
                request.max_bytes
            )));
        }
        let digest = format!("{:x}", Sha256::digest(body.as_bytes()));
        Ok(IdentityBrief {
            release_id: request.release_id,
            outcome: request.outcome.trim().into(),
            channel: request.channel.trim().into(),
            audience: request.audience.trim().into(),
            author: request.author.trim().into(),
            bytes: body.len(),
            body,
            digest,
            included_evidence_ids: included,
            omitted_evidence_ids: omitted,
        })
    }
}

fn render_brief(header: &str, lines: &[String], omitted: &[Uuid]) -> String {
    let mut body = String::from(header);
    body.push_str("## Authoritative guidance\n");
    for line in lines {
        body.push_str(line);
    }
    body.push_str("\n## Omission account\n");
    if omitted.is_empty() {
        body.push_str("none\n");
    } else {
        body.push_str(&format!(
            "{} evidence items omitted by byte bound: ",
            omitted.len()
        ));
        body.push_str(
            &omitted
                .iter()
                .map(Uuid::to_string)
                .collect::<Vec<_>>()
                .join(","),
        );
        body.push('\n');
    }
    body
}
