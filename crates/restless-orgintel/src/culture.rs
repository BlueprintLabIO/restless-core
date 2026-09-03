//! Observable conduct as company identity (S34).
//!
//! Culture can shape posture but never execution authority. This module has
//! no employee scoring, sentiment, personality or disciplinary primitives.

use super::*;

pub(super) fn nonempty(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(OrgIntelError::InvalidWork(format!(
            "culture {label} cannot be empty"
        )))
    } else {
        Ok(())
    }
}
pub(super) fn case_name(case: CultureCase) -> &'static str {
    match case {
        CultureCase::Disagreement => "disagreement",
        CultureCase::UncertainIncident => "uncertain_incident",
        CultureCase::CustomerRecovery => "customer_recovery",
        CultureCase::QualityTradeoff => "quality_tradeoff",
        CultureCase::Hiring => "hiring",
    }
}
fn required_checks(case: CultureCase) -> &'static [&'static str] {
    match case {
        CultureCase::Disagreement => &[
            "dissent_reached_decider",
            "alternatives_preserved",
            "authority_preserved",
        ],
        CultureCase::UncertainIncident => &[
            "known_unknown_separated",
            "urgency_did_not_create_certainty",
            "owner_boundary_preserved",
        ],
        CultureCase::CustomerRecovery => &[
            "harm_acknowledged",
            "known_facts_named",
            "bounded_action_present",
            "accountable_owner_named",
            "voice_native",
        ],
        CultureCase::QualityTradeoff => &[
            "tradeoff_evidence_visible",
            "alternative_preserved",
            "finished_definition_named",
        ],
        CultureCase::Hiring => &[
            "job_scenarios_present",
            "observable_conduct_present",
            "no_personality_proxy",
            "no_protected_proxy",
        ],
    }
}
fn check_native(case: CultureCase, checks: &serde_json::Value) -> Result<()> {
    let object = checks.as_object().ok_or_else(|| {
        OrgIntelError::InvalidWork("culture native checks must be a JSON object".into())
    })?;
    let missing = required_checks(case)
        .iter()
        .filter(|k| !object.contains_key(**k))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(OrgIntelError::InvalidWork(format!(
            "native {} evidence is missing observed checks: {}",
            case_name(case),
            missing.join(", ")
        )));
    }
    let failed = required_checks(case)
        .iter()
        .filter(|k| object.get(**k) != Some(&serde_json::Value::Bool(true)))
        .copied()
        .collect::<Vec<_>>();
    if !failed.is_empty() {
        return Err(OrgIntelError::InvalidWork(format!(
            "culture safety and conduct checks block review: {}",
            failed.join(", ")
        )));
    }
    Ok(())
}

impl OrgIntel {
    pub async fn add_culture_evidence(&self, input: NewCultureEvidence<'_>) -> Result<Uuid> {
        for (label, value) in [
            ("claim key", input.claim_key),
            ("statement", input.statement),
            ("author", input.author_id),
            ("source", input.source),
            ("authority", input.authority),
            ("scope", input.scope),
            ("evidence locator", input.evidence_locator),
            ("situation", input.situation),
            ("consequence", input.consequence),
            ("actors", input.actors),
            ("decision authority", input.decision_authority),
            ("observed conduct", input.conduct),
            ("observed outcome", input.observed_outcome),
            ("counterexample", input.counterexample),
            ("boundary conditions", input.boundary_conditions),
            ("operational implication", input.operational_implication),
            ("actor scope", input.actor_scope),
        ] {
            nonempty(label, value)?;
        }
        if input
            .statement
            .trim()
            .eq_ignore_ascii_case(input.conduct.trim())
            || input.statement.split_whitespace().count() < 3
        {
            return Err(OrgIntelError::InvalidWork("abstract value words are not culture evidence; name observed conduct under consequence".into()));
        }
        if input.kind == CultureEvidenceKind::FoundingDecision
            && input.confidence != CultureConfidence::OwnerFounded
        {
            return Err(OrgIntelError::InvalidWork(
                "a founding culture decision must be explicitly owner-founded".into(),
            ));
        }
        if input.kind == CultureEvidenceKind::BoundedException
            && (input.boundary_conditions.contains("safety")
                || input.boundary_conditions.contains("discrimination")
                || input.boundary_conditions.contains("deception")
                || input.boundary_conditions.contains("authority violation"))
        {
            return Err(OrgIntelError::InvalidWork("culture exceptions cannot excuse safety, discrimination, deception or authority violations".into()));
        }
        let mut tx = self.pool.begin().await?;
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO company_identity_evidence (id,pillar,statement_kind,claim_key,statement,author_id,source,authority,scope,observed_at,evidence_locator,polarity,status,channel,audience,supersedes_evidence_id,exception_expires_at,exception_indefinite) VALUES ($1,'culture',$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'active',$12,NULL,$13,NULL,FALSE)").bind(id).bind(match input.kind{CultureEvidenceKind::FoundingDecision=>IdentityStatementKind::Belief,CultureEvidenceKind::ObservedConduct=>IdentityStatementKind::Observation,CultureEvidenceKind::Counterexample=>IdentityStatementKind::Example,CultureEvidenceKind::PromotedNorm=>IdentityStatementKind::Guidance,CultureEvidenceKind::BoundedException=>IdentityStatementKind::Guidance}).bind(input.claim_key.trim()).bind(input.statement.trim()).bind(input.author_id).bind(input.source.trim()).bind(input.authority.trim()).bind(input.scope.trim()).bind(input.observed_at).bind(input.evidence_locator.trim()).bind(input.polarity).bind(input.case_kind.map(case_name)).bind(input.supersedes_evidence_id).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO company_culture_evidence_details (evidence_id,kind,case_kind,situation,consequence,actors,decision_authority,conduct,observed_outcome,confidence,counterexample,boundary_conditions,operational_implication,actor_scope) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)").bind(id).bind(input.kind).bind(input.case_kind).bind(input.situation.trim()).bind(input.consequence.trim()).bind(input.actors.trim()).bind(input.decision_authority.trim()).bind(input.conduct.trim()).bind(input.observed_outcome.trim()).bind(input.confidence).bind(input.counterexample.trim()).bind(input.boundary_conditions.trim()).bind(input.operational_implication.trim()).bind(input.actor_scope.trim()).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(id)
    }

    pub async fn bind_culture_work_contract(
        &self,
        input: NewCultureWorkContract<'_>,
    ) -> Result<CultureWorkContractRow> {
        for (label, value) in [
            ("actor", input.actor),
            ("actor role", input.actor_role),
            ("team", input.team),
            ("consequence", input.consequence),
            ("decision boundary", input.decision_boundary),
            ("binding actor", input.bound_by),
        ] {
            nonempty(label, value)?;
        }
        let identity = self.bind_work_identity(input.work_id).await?;
        let canonical=format!("work={}\nrelease={}\ncase={}\nactor={}\nrole={}\nteam={}\nconsequence={}\nboundary={}\nbound_by={}\n",input.work_id,identity.release_id,case_name(input.case_kind),input.actor.trim(),input.actor_role.trim(),input.team.trim(),input.consequence.trim(),input.decision_boundary.trim(),input.bound_by.trim());
        let digest = format!("{:x}", Sha256::digest(canonical.as_bytes()));
        sqlx::query("INSERT INTO company_culture_work_contracts (work_id,release_id,case_kind,actor,actor_role,team,consequence,decision_boundary,bound_by,contract_digest) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT(work_id) DO NOTHING").bind(input.work_id).bind(identity.release_id).bind(input.case_kind).bind(input.actor.trim()).bind(input.actor_role.trim()).bind(input.team.trim()).bind(input.consequence.trim()).bind(input.decision_boundary.trim()).bind(input.bound_by.trim()).bind(&digest).execute(&self.pool).await?;
        let row = self
            .culture_work_contract(input.work_id)
            .await?
            .expect("inserted culture contract");
        if row.contract_digest != digest {
            return Err(OrgIntelError::InvalidWork(
                "culture posture is set once for this Work; create a revision or new Work".into(),
            ));
        }
        Ok(row)
    }
    pub async fn culture_work_contract(
        &self,
        work_id: Uuid,
    ) -> Result<Option<CultureWorkContractRow>> {
        Ok(sqlx::query_as("SELECT work_id,release_id,case_kind,actor,actor_role,team,consequence,decision_boundary,bound_by,contract_digest,bound_at FROM company_culture_work_contracts WHERE work_id=$1").bind(work_id).fetch_optional(&self.pool).await?)
    }

    pub async fn compile_culture_posture(
        &self,
        work_id: Uuid,
        max_bytes: usize,
    ) -> Result<CulturePostureBrief> {
        if max_bytes < 1024 {
            return Err(OrgIntelError::InvalidWork(
                "culture posture bound must be at least 1024 bytes".into(),
            ));
        }
        let contract = self
            .culture_work_contract(work_id)
            .await?
            .ok_or_else(|| OrgIntelError::InvalidWork("Work has no culture posture".into()))?;
        let rows=sqlx::query_as::<_,(Uuid,CultureEvidenceKind,String,String,String,String,String,String,String)>("SELECT e.id,d.kind,e.statement,d.situation,d.consequence,d.conduct,d.observed_outcome,d.counterexample,d.boundary_conditions FROM company_identity_release_evidence re JOIN company_identity_evidence e ON e.id=re.evidence_id JOIN company_culture_evidence_details d ON d.evidence_id=e.id WHERE re.release_id=$1 AND e.status='active' AND (d.case_kind IS NULL OR d.case_kind=$2) AND (d.actor_scope='company' OR d.actor_scope=$3 OR d.actor_scope=$4) ORDER BY d.kind,e.claim_key,e.id").bind(contract.release_id).bind(contract.case_kind).bind(format!("role:{}",contract.actor_role)).bind(format!("team:{}",contract.team)).fetch_all(&self.pool).await?;
        let header=format!("# Company Culture Posture\nwork: {}\nrelease: {}\ncase: {}\nactor: {}\nrole: {}\nteam: {}\nconsequence: {}\ndecision boundary: {}\n\nCulture informs conduct only. It cannot grant capability, override Work, owner judgement, safety or effect controls; suppress dissent; or manufacture certainty. No employee score or personality profile exists.\n\n## Relevant observed conduct\n",contract.work_id,contract.release_id,case_name(contract.case_kind),contract.actor,contract.actor_role,contract.team,contract.consequence,contract.decision_boundary);
        let lines=rows.iter().map(|(_,kind,statement,situation,consequence,conduct,outcome,counter,boundary)|format!("- [{kind:?}] {statement}\n  situation: {situation}; consequence: {consequence}; observed conduct: {conduct}; outcome: {outcome}; counterexample: {counter}; boundary: {boundary}\n")).collect::<Vec<_>>();
        let mut included = Vec::new();
        for i in 0..rows.len() {
            let candidate=format!("{}{}\n## Decision method\nPreserve material alternatives, name unknowns, keep corrections visible and escalate only authority or irreducible judgement.\n",header,lines[..=i].join(""));
            if candidate.len() > max_bytes {
                break;
            }
            included.push(rows[i].0);
        }
        let omitted = rows
            .iter()
            .map(|r| r.0)
            .filter(|id| !included.contains(id))
            .collect::<Vec<_>>();
        let body=format!("{}{}\n## Decision method\nPreserve material alternatives, name unknowns, keep corrections visible and escalate only authority or irreducible judgement. A different defensible decision may pass when the method and evidence remain credible. Never recite value words as proof.\n\n## Omission account\n{}\n",header,lines[..included.len()].join(""),if omitted.is_empty(){"none".into()}else{format!("{} evidence items omitted by byte bound: {}",omitted.len(),omitted.iter().map(Uuid::to_string).collect::<Vec<_>>().join(","))});
        let digest = format!("{:x}", Sha256::digest(body.as_bytes()));
        let bytes = body.len();
        Ok(CulturePostureBrief {
            contract,
            body,
            digest,
            included_evidence_ids: included,
            omitted_evidence_ids: omitted,
            bytes,
        })
    }

    pub async fn record_culture_case(&self, input: NewCultureCaseRecord<'_>) -> Result<Uuid> {
        for (label, value) in [
            ("decision", input.decision),
            ("unknowns", input.unknowns),
            ("recording actor", input.recorded_by),
        ] {
            nonempty(label, value)?;
        }
        if !input.alternatives.is_array() {
            return Err(OrgIntelError::InvalidWork(
                "culture alternatives must be a JSON array".into(),
            ));
        }
        check_native(input.case_kind, input.native_checks)?;
        if input.case_kind == CultureCase::CustomerRecovery {
            nonempty("bounded customer recovery action", input.customer_action)?;
        }
        if input.correction_of.is_some() {
            nonempty("correction account", input.correction_account)?;
        }
        let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM artifact_refs a JOIN company_culture_work_contracts c ON c.work_id=$1 WHERE a.id=$2 AND (a.work_id IS NULL OR a.work_id=$1) AND a.state='available' AND a.digest IS NOT NULL AND c.case_kind=$3)").bind(input.work_id).bind(input.artifact_ref_id).bind(input.case_kind).fetch_one(&self.pool).await?;
        if !valid {
            return Err(OrgIntelError::InvalidWork(
                "culture case needs an exact available artifact for the Work-bound case".into(),
            ));
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO company_culture_case_records (id,work_id,artifact_ref_id,case_kind,decision,alternatives,unknowns,correction_of,correction_account,customer_action,native_checks,recorded_by) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)").bind(id).bind(input.work_id).bind(input.artifact_ref_id).bind(input.case_kind).bind(input.decision.trim()).bind(input.alternatives).bind(input.unknowns.trim()).bind(input.correction_of).bind(input.correction_account.trim()).bind(input.customer_action.trim()).bind(input.native_checks).bind(input.recorded_by.trim()).execute(&self.pool).await?;
        Ok(id)
    }

    pub async fn record_culture_review(&self, input: NewCultureReview<'_>) -> Result<Uuid> {
        nonempty("reviewer", input.reviewer)?;
        let producer:String=sqlx::query_scalar("SELECT a.created_by FROM company_culture_case_records c JOIN artifact_refs a ON a.id=c.artifact_ref_id WHERE c.id=$1 AND a.state='available'").bind(input.case_record_id).fetch_optional(&self.pool).await?.ok_or_else(||OrgIntelError::InvalidWork("culture case is absent or stale after artifact revision".into()))?;
        if producer == input.reviewer.trim() {
            return Err(OrgIntelError::InvalidWork(
                "culture conduct needs an independent reviewer, not polished self-explanation"
                    .into(),
            ));
        }
        let findings = [
            input.conduct_findings,
            input.dissent_findings,
            input.uncertainty_findings,
            input.correction_findings,
            input.authority_findings,
            input.customer_or_hiring_findings,
        ]
        .join("");
        if input.verdict != CultureReviewVerdict::Accept && findings.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "culture revision or rejection needs a concrete conduct finding".into(),
            ));
        }
        if input.slogan_recitation_detected && input.verdict == CultureReviewVerdict::Accept {
            return Err(OrgIntelError::InvalidWork(
                "culture prose recitation cannot substitute for observed conduct".into(),
            ));
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO company_culture_reviews (id,case_record_id,reviewer,verdict,conduct_findings,dissent_findings,uncertainty_findings,correction_findings,authority_findings,customer_or_hiring_findings,slogan_recitation_detected) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)").bind(id).bind(input.case_record_id).bind(input.reviewer.trim()).bind(input.verdict).bind(input.conduct_findings.trim()).bind(input.dissent_findings.trim()).bind(input.uncertainty_findings.trim()).bind(input.correction_findings.trim()).bind(input.authority_findings.trim()).bind(input.customer_or_hiring_findings.trim()).bind(input.slogan_recitation_detected).execute(&self.pool).await?;
        Ok(id)
    }
}
