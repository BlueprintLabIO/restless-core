//! Integrated, bounded Company Constitution (S35).
//!
//! This joins proven pillar contracts without flattening their authority or
//! evidence semantics. It does not add publication authority or mutate assets.
use super::*;
use std::collections::BTreeSet;
fn required(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(OrgIntelError::InvalidWork(format!(
            "constitution {label} cannot be empty"
        )))
    } else {
        Ok(())
    }
}
fn drift_kind(pillar: IdentityPillar) -> IdentityDriftKind {
    match pillar {
        IdentityPillar::Truth => IdentityDriftKind::TruthStale,
        IdentityPillar::Voice => IdentityDriftKind::VoiceDifference,
        IdentityPillar::Visual => IdentityDriftKind::VisualDifference,
        IdentityPillar::Culture => IdentityDriftKind::CultureDifference,
    }
}

impl OrgIntel {
    pub async fn compile_constitution(
        &self,
        work_id: Uuid,
        max_bytes: usize,
    ) -> Result<ConstitutionBrief> {
        if max_bytes < 8192 {
            return Err(OrgIntelError::InvalidWork(
                "constitution brief bound must be at least 8192 bytes".into(),
            ));
        }
        let binding = self.bind_work_identity(work_id).await?;
        let share = max_bytes / 5;
        let truths=sqlx::query_as::<_,IdentityEvidenceRow>(&format!("{} e JOIN company_identity_release_evidence re ON re.evidence_id=e.id WHERE re.release_id=$1 AND e.pillar='truth' AND e.status='active' ORDER BY e.claim_key,e.id",super::identity::evidence_select())).bind(binding.release_id).fetch_all(&self.pool).await?;
        let mut claims = std::collections::BTreeMap::<String, BTreeSet<String>>::new();
        for row in &truths {
            if row.statement_kind == IdentityStatementKind::Fact {
                claims
                    .entry(row.claim_key.clone())
                    .or_default()
                    .insert(row.statement.clone());
            }
        }
        let conflicts = claims
            .into_iter()
            .filter_map(|(key, values)| (values.len() > 1).then_some(key))
            .collect::<Vec<_>>();
        if !conflicts.is_empty() {
            return Err(OrgIntelError::InvalidWork(format!(
                "constitution truth conflict blocks dependent expression: {}",
                conflicts.join(", ")
            )));
        }
        let mut accounts = Vec::new();
        let mut sections = Vec::new();
        let mut truth_lines = String::new();
        let mut truth_ids = Vec::new();
        for row in &truths {
            let line = format!(
                "- [{}] {} — source: {}; evidence: {}\n",
                row.claim_key, row.statement, row.source, row.evidence_locator
            );
            if truth_lines.len() + line.len() > share {
                break;
            }
            truth_lines.push_str(&line);
            truth_ids.push(row.id);
        }
        let truth_omitted = truths
            .iter()
            .map(|r| r.id)
            .filter(|id| !truth_ids.contains(id))
            .collect::<Vec<_>>();
        let truth_digest = format!("{:x}", Sha256::digest(truth_lines.as_bytes()));
        sections.push(format!(
            "## Company Truth [authoritative facts]\n{truth_lines}"
        ));
        accounts.push(ConstitutionPillarAccount {
            pillar: IdentityPillar::Truth,
            status: "available".into(),
            digest: Some(truth_digest),
            bytes: truth_lines.len(),
            included_evidence_ids: truth_ids,
            omitted_evidence_ids: truth_omitted,
        });
        if self.voice_work_contract(work_id).await?.is_some() {
            let brief = self
                .compile_voice_contract(work_id, share.max(1024))
                .await?;
            sections.push(format!(
                "## Company Voice [channel and author contract]\n{}",
                brief.body
            ));
            accounts.push(ConstitutionPillarAccount {
                pillar: IdentityPillar::Voice,
                status: "available".into(),
                digest: Some(brief.digest),
                bytes: brief.bytes,
                included_evidence_ids: brief.included_evidence_ids,
                omitted_evidence_ids: brief.omitted_evidence_ids,
            });
        } else {
            sections.push("## Company Voice\nunavailable for this Work; do not generate a substitute house style.\n".into());
            accounts.push(ConstitutionPillarAccount {
                pillar: IdentityPillar::Voice,
                status: "unavailable: no Work-bound contract".into(),
                digest: None,
                bytes: 0,
                included_evidence_ids: vec![],
                omitted_evidence_ids: vec![],
            });
        }
        if self.visual_work_contract(work_id).await?.is_some() {
            let brief = self
                .compile_visual_direction(work_id, share.max(1024))
                .await?;
            sections.push(format!(
                "## Visual Language [native art direction]\n{}",
                brief.body
            ));
            accounts.push(ConstitutionPillarAccount {
                pillar: IdentityPillar::Visual,
                status: "available".into(),
                digest: Some(brief.digest),
                bytes: brief.bytes,
                included_evidence_ids: brief.included_evidence_ids,
                omitted_evidence_ids: brief.omitted_evidence_ids,
            });
        } else {
            sections.push("## Visual Language\nunavailable for this Work; do not generate generic visual defaults.\n".into());
            accounts.push(ConstitutionPillarAccount {
                pillar: IdentityPillar::Visual,
                status: "unavailable: no Work-bound contract".into(),
                digest: None,
                bytes: 0,
                included_evidence_ids: vec![],
                omitted_evidence_ids: vec![],
            });
        }
        if self.culture_work_contract(work_id).await?.is_some() {
            let brief = self
                .compile_culture_posture(work_id, share.max(1024))
                .await?;
            sections.push(format!(
                "## Operating Culture [consequence-relevant posture]\n{}",
                brief.body
            ));
            accounts.push(ConstitutionPillarAccount {
                pillar: IdentityPillar::Culture,
                status: "available".into(),
                digest: Some(brief.digest),
                bytes: brief.bytes,
                included_evidence_ids: brief.included_evidence_ids,
                omitted_evidence_ids: brief.omitted_evidence_ids,
            });
        } else {
            sections.push("## Operating Culture\nunavailable for this Work; do not infer personality or generic values.\n".into());
            accounts.push(ConstitutionPillarAccount {
                pillar: IdentityPillar::Culture,
                status: "unavailable: no Work-bound posture".into(),
                digest: None,
                bytes: 0,
                included_evidence_ids: vec![],
                omitted_evidence_ids: vec![],
            });
        }
        let header=format!("# Company Constitution Brief\nwork: {work_id}\nrelease: {}\n\nPillars retain separate authority, provenance, conflicts and omission accounts. Channel, author, visual and culture context cannot widen capability, effects or owner authority.\n\n",binding.release_id);
        let body=format!("{}{}\n## Effect boundary\nThis brief cannot approve publication, outreach, customer contact, deployment, payment or another external effect. Existing source-owned controls remain authoritative.\n",header,sections.join("\n"));
        if body.len() > max_bytes {
            return Err(OrgIntelError::InvalidWork(format!(
                "constitution compiler exceeded its bound: {} > {max_bytes}",
                body.len()
            )));
        }
        let digest = format!("{:x}", Sha256::digest(body.as_bytes()));
        let bytes = body.len();
        Ok(ConstitutionBrief {
            work_id,
            release_id: binding.release_id,
            body,
            digest,
            pillars: accounts,
            bytes,
        })
    }

    pub async fn bind_constitution_artifact(
        &self,
        input: NewConstitutionArtifactBinding<'_>,
    ) -> Result<ConstitutionArtifactBindingRow> {
        for (label, value) in [
            ("channel", input.channel),
            ("audience", input.audience),
            ("named author", input.named_author),
            ("producer", input.producer),
            ("accountable lead", input.accountable_lead),
            ("company voice", input.company_voice),
            ("constitution digest", input.constitution_digest),
        ] {
            required(label, value)?;
        }
        if !input.native_evidence.is_object() {
            return Err(OrgIntelError::InvalidWork(
                "constitution native evidence must be a JSON object".into(),
            ));
        }
        let brief = self.compile_constitution(input.work_id, 32 * 1024).await?;
        if brief.digest != input.constitution_digest {
            return Err(OrgIntelError::InvalidWork(
                "artifact constitution digest does not match the deterministic Work-bound brief"
                    .into(),
            ));
        }
        let artifact_valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM artifact_refs WHERE id=$1 AND (work_id IS NULL OR work_id=$2) AND state='available' AND digest IS NOT NULL)").bind(input.artifact_ref_id).bind(input.work_id).fetch_one(&self.pool).await?;
        if !artifact_valid {
            return Err(OrgIntelError::InvalidWork(
                "constitution binding needs an exact available artifact version".into(),
            ));
        }
        let available = brief
            .pillars
            .iter()
            .flat_map(|p| p.included_evidence_ids.iter().copied())
            .collect::<BTreeSet<_>>();
        let requested = input.evidence_ids.iter().copied().collect::<BTreeSet<_>>();
        if requested.is_empty() || !requested.is_subset(&available) {
            return Err(OrgIntelError::InvalidWork("artifact truth and pillar dependencies must be explicit members of the compiled brief".into()));
        }
        let mut tx = self.pool.begin().await?;
        let inserted = sqlx::query("INSERT INTO company_constitution_artifact_bindings (artifact_ref_id,work_id,release_id,channel,audience,named_author,producer,accountable_lead,company_voice,native_evidence,constitution_digest) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) ON CONFLICT(artifact_ref_id) DO NOTHING").bind(input.artifact_ref_id).bind(input.work_id).bind(brief.release_id).bind(input.channel.trim()).bind(input.audience.trim()).bind(input.named_author.trim()).bind(input.producer.trim()).bind(input.accountable_lead.trim()).bind(input.company_voice.trim()).bind(input.native_evidence).bind(input.constitution_digest.trim()).execute(&mut *tx).await?.rows_affected() == 1;
        let row:ConstitutionArtifactBindingRow=sqlx::query_as("SELECT artifact_ref_id,work_id,release_id,channel,audience,named_author,producer,accountable_lead,company_voice,native_evidence,constitution_digest,bound_at FROM company_constitution_artifact_bindings WHERE artifact_ref_id=$1").bind(input.artifact_ref_id).fetch_one(&mut *tx).await?;
        let same_binding = row.work_id == input.work_id
            && row.release_id == brief.release_id
            && row.channel == input.channel.trim()
            && row.audience == input.audience.trim()
            && row.named_author == input.named_author.trim()
            && row.producer == input.producer.trim()
            && row.accountable_lead == input.accountable_lead.trim()
            && row.company_voice == input.company_voice.trim()
            && row.native_evidence == *input.native_evidence
            && row.constitution_digest == input.constitution_digest.trim();
        if !same_binding {
            return Err(OrgIntelError::InvalidWork(
                "accepted artifact identity binding is immutable; create a new artifact version"
                    .into(),
            ));
        }
        let existing = sqlx::query_scalar::<_, Uuid>(
            "SELECT evidence_id FROM company_constitution_artifact_evidence WHERE artifact_ref_id=$1",
        )
        .bind(input.artifact_ref_id)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .collect::<BTreeSet<_>>();
        if !inserted && existing != requested {
            return Err(OrgIntelError::InvalidWork(
                "accepted artifact identity dependencies are immutable; create a new artifact version"
                    .into(),
            ));
        }
        if inserted {
            for evidence_id in requested {
                sqlx::query("INSERT INTO company_constitution_artifact_evidence (artifact_ref_id,evidence_id) VALUES ($1,$2)").bind(input.artifact_ref_id).bind(evidence_id).execute(&mut *tx).await?;
            }
        }
        tx.commit().await?;
        Ok(row)
    }

    pub async fn propose_constitution_learning(
        &self,
        input: NewConstitutionLearningProposal<'_>,
    ) -> Result<Uuid> {
        for (label, value) in [
            ("creator", input.created_by),
            ("triggering event", input.triggering_event),
            ("scope", input.scope),
            ("contradiction check", input.contradiction_check),
        ] {
            required(label, value)?;
        }
        if input.before_artifact_ref_id == input.after_artifact_ref_id {
            return Err(OrgIntelError::InvalidWork(
                "constitution learning needs exact distinct before and after artifacts".into(),
            ));
        }
        for id in [input.before_artifact_ref_id, input.after_artifact_ref_id] {
            let exact: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM artifact_refs WHERE id=$1 AND digest IS NOT NULL)",
            )
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
            if !exact {
                return Err(OrgIntelError::InvalidWork(
                    "constitution learning needs exact before and after artifact identities".into(),
                ));
            }
        }
        let evidence = sqlx::query(
            "SELECT pillar,source,authority FROM company_identity_evidence WHERE id=$1",
        )
        .bind(input.evidence_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork("constitution learning evidence not found".into())
        })?;
        if evidence.get::<IdentityPillar, _>("pillar") != input.pillar {
            return Err(OrgIntelError::InvalidWork(
                "learning pillar does not match attributed evidence".into(),
            ));
        }
        let provenance = format!(
            "{} {}",
            evidence.get::<String, _>("source"),
            evidence.get::<String, _>("authority")
        )
        .to_lowercase();
        if [
            "generated repetition",
            "model majority",
            "evaluator preference",
            "mere frequency",
        ]
        .iter()
        .any(|bad| provenance.contains(bad))
        {
            return Err(OrgIntelError::InvalidWork("generated repetition, evaluator taste and frequency cannot propose constitution policy".into()));
        }
        let current: Uuid = sqlx::query_scalar(
            "SELECT release_id FROM company_identity_current_release WHERE singleton=TRUE",
        )
        .fetch_one(&self.pool)
        .await?;
        let mut ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT evidence_id FROM company_identity_release_evidence WHERE release_id=$1",
        )
        .bind(current)
        .fetch_all(&self.pool)
        .await?;
        ids.push(input.evidence_id);
        ids.sort();
        ids.dedup();
        let proposal = self
            .propose_identity_release(input.created_by, input.triggering_event, &ids)
            .await?;
        sqlx::query("INSERT INTO company_constitution_learning_proposals (proposal_id,evidence_id,pillar,trigger_kind,triggering_event,before_artifact_ref_id,after_artifact_ref_id,scope,contradiction_check) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)").bind(proposal).bind(input.evidence_id).bind(input.pillar).bind(input.trigger_kind).bind(input.triggering_event.trim()).bind(input.before_artifact_ref_id).bind(input.after_artifact_ref_id).bind(input.scope.trim()).bind(input.contradiction_check.trim()).execute(&self.pool).await?;
        Ok(proposal)
    }

    pub async fn compute_identity_drift(
        &self,
        to_release_id: Uuid,
    ) -> Result<Vec<IdentityDriftFindingRow>> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM company_identity_releases WHERE id=$1)",
        )
        .bind(to_release_id)
        .fetch_one(&self.pool)
        .await?;
        if !exists {
            return Err(OrgIntelError::InvalidWork(
                "target identity release not found".into(),
            ));
        }
        let dependencies=sqlx::query("SELECT b.artifact_ref_id,b.release_id,e.evidence_id,old.pillar,old.claim_key,old.statement FROM company_constitution_artifact_bindings b JOIN company_constitution_artifact_evidence e ON e.artifact_ref_id=b.artifact_ref_id JOIN company_identity_evidence old ON old.id=e.evidence_id WHERE b.release_id<>$1 ORDER BY b.artifact_ref_id,e.evidence_id").bind(to_release_id).fetch_all(&self.pool).await?;
        for row in dependencies {
            let artifact: Uuid = row.get("artifact_ref_id");
            let old_release: Uuid = row.get("release_id");
            let old_id: Uuid = row.get("evidence_id");
            let still_present:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM company_identity_release_evidence WHERE release_id=$1 AND evidence_id=$2)").bind(to_release_id).bind(old_id).fetch_one(&self.pool).await?;
            if still_present {
                continue;
            }
            let replacement:Option<Uuid>=sqlx::query_scalar("SELECT e.id FROM company_identity_release_evidence re JOIN company_identity_evidence e ON e.id=re.evidence_id WHERE re.release_id=$1 AND e.supersedes_evidence_id=$2 ORDER BY e.created_at DESC,e.id LIMIT 1").bind(to_release_id).bind(old_id).fetch_optional(&self.pool).await?;
            let pillar: IdentityPillar = row.get("pillar");
            let kind = if replacement.is_some() {
                drift_kind(pillar)
            } else {
                IdentityDriftKind::UnknownDependency
            };
            let dependency = format!(
                "{}: {}",
                row.get::<String, _>("claim_key"),
                row.get::<String, _>("statement")
            );
            let consequence = if replacement.is_some() {
                format!(
                    "artifact depends on superseded {:?} evidence; decide retain, revise or retire",
                    pillar
                )
            } else {
                "dependency is absent from the target release and has no explicit replacement; status remains unknown".into()
            };
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO company_identity_drift_findings (id,artifact_ref_id,from_release_id,to_release_id,kind,old_evidence_id,new_evidence_id,dependency,consequence) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT(artifact_ref_id,to_release_id,kind,old_evidence_id) DO NOTHING").bind(id).bind(artifact).bind(old_release).bind(to_release_id).bind(kind).bind(old_id).bind(replacement).bind(dependency).bind(consequence).execute(&self.pool).await?;
        }
        Ok(sqlx::query_as("SELECT id,artifact_ref_id,from_release_id,to_release_id,kind,old_evidence_id,new_evidence_id,dependency,consequence,created_at FROM company_identity_drift_findings WHERE to_release_id=$1 ORDER BY artifact_ref_id,kind,id").bind(to_release_id).fetch_all(&self.pool).await?)
    }

    pub async fn decide_identity_migration(
        &self,
        input: NewIdentityMigrationDecision<'_>,
    ) -> Result<IdentityMigrationDecisionRow> {
        if input.decided_by != "owner" {
            return Err(OrgIntelError::InvalidWork(
                "only the owner may decide identity migration".into(),
            ));
        }
        required("migration rationale", input.rationale)?;
        required("Authority record", input.authority_record_id)?;
        sqlx::query("INSERT INTO company_identity_migration_decisions (drift_finding_id,disposition,decided_by,rationale,authority_record_id) VALUES ($1,$2,$3,$4,$5) ON CONFLICT(drift_finding_id) DO NOTHING").bind(input.drift_finding_id).bind(input.disposition).bind(input.decided_by).bind(input.rationale.trim()).bind(input.authority_record_id.trim()).execute(&self.pool).await?;
        let decision: IdentityMigrationDecisionRow = sqlx::query_as("SELECT drift_finding_id,disposition,decided_by,rationale,authority_record_id,decided_at FROM company_identity_migration_decisions WHERE drift_finding_id=$1").bind(input.drift_finding_id).fetch_one(&self.pool).await?;
        if decision.disposition != input.disposition
            || decision.decided_by != input.decided_by
            || decision.rationale != input.rationale.trim()
            || decision.authority_record_id != input.authority_record_id.trim()
        {
            return Err(OrgIntelError::InvalidWork(
                "identity migration decision is immutable; create a new drift finding for a new decision"
                    .into(),
            ));
        }
        Ok(decision)
    }
}
