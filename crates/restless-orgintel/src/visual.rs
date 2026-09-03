//! Product-grounded, channel-native visual language (S33).
//!
//! The registry is evidence, not a quota. Work binds one released visual
//! contract, records only the primitive versions actually used, and can be
//! accepted only against exact native artifacts and motion states.

use super::*;

pub(super) fn required(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(OrgIntelError::InvalidWork(format!(
            "visual language {label} cannot be empty"
        )));
    }
    Ok(())
}

pub(super) fn channel_name(channel: VisualChannel) -> &'static str {
    match channel {
        VisualChannel::LandingPage => "landing_page",
        VisualChannel::Email => "email",
        VisualChannel::Product => "product",
        VisualChannel::Social => "social",
    }
}

fn native_requirements(channel: VisualChannel) -> &'static [&'static str] {
    match channel {
        VisualChannel::LandingPage => &[
            "responsive_containment",
            "keyboard_path",
            "contrast",
            "text_fit",
            "proof_legible",
            "reduced_motion_complete",
        ],
        VisualChannel::Email => &[
            "desktop_wrap",
            "narrow_wrap",
            "static_fallback",
            "text_fallback",
            "contrast",
            "proof_legible",
        ],
        VisualChannel::Product => &[
            "focus_order",
            "state_transition",
            "recovery_operable",
            "text_fit",
            "product_fidelity",
            "reduced_motion_complete",
        ],
        VisualChannel::Social => &[
            "standalone_legibility",
            "safe_crop",
            "contrast",
            "text_fit",
            "static_complete",
        ],
    }
}

fn require_checks(channel: VisualChannel, checks: &serde_json::Value) -> Result<()> {
    let object = checks.as_object().ok_or_else(|| {
        OrgIntelError::InvalidWork("native visual checks must be a JSON object".into())
    })?;
    let missing = native_requirements(channel)
        .iter()
        .filter(|key| !object.contains_key(**key))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(OrgIntelError::InvalidWork(format!(
            "native {} evidence is missing observed checks: {}",
            channel_name(channel),
            missing.join(", ")
        )));
    }
    let failed = native_requirements(channel)
        .iter()
        .filter(|key| object.get(**key) != Some(&serde_json::Value::Bool(true)))
        .copied()
        .collect::<Vec<_>>();
    if !failed.is_empty() {
        return Err(OrgIntelError::InvalidWork(format!(
            "mechanical visual checks block review: {}",
            failed.join(", ")
        )));
    }
    Ok(())
}

fn kind_name(kind: VisualEvidenceKind) -> &'static str {
    match kind {
        VisualEvidenceKind::SemanticToken => "semantic_token",
        VisualEvidenceKind::TypographyRole => "typography_role",
        VisualEvidenceKind::CompositionPrinciple => "composition_principle",
        VisualEvidenceKind::ImageryDirection => "imagery_direction",
        VisualEvidenceKind::MotionPattern => "motion_pattern",
        VisualEvidenceKind::ProductRepresentationRule => "product_representation_rule",
        VisualEvidenceKind::Primitive => "primitive",
        VisualEvidenceKind::ApprovedComposition => "approved_composition",
        VisualEvidenceKind::RejectedExample => "rejected_example",
    }
}

impl OrgIntel {
    pub async fn add_visual_evidence(&self, input: NewVisualEvidence<'_>) -> Result<Uuid> {
        for (label, value) in [
            ("claim key", input.claim_key),
            ("statement", input.statement),
            ("author", input.author_id),
            ("source", input.source),
            ("authority", input.authority),
            ("scope", input.scope),
            ("evidence locator", input.evidence_locator),
            ("purpose", input.purpose),
            ("rationale", input.rationale),
            ("accessibility notes", input.accessibility_notes),
        ] {
            required(label, value)?;
        }
        if !input.dependencies.is_array() {
            return Err(OrgIntelError::InvalidWork(
                "visual primitive dependencies must be a JSON array".into(),
            ));
        }
        if input.kind == VisualEvidenceKind::MotionPattern {
            required(
                "reduced-motion replacement",
                input.reduced_motion_replacement.unwrap_or(""),
            )?;
        }
        if input.kind == VisualEvidenceKind::ProductRepresentationRule {
            required(
                "product truth locator",
                input.product_truth_locator.unwrap_or(""),
            )?;
        }
        if input.kind == VisualEvidenceKind::Primitive {
            for (label, value) in [
                ("primitive origin", input.origin.unwrap_or("")),
                ("primitive licence", input.licence.unwrap_or("")),
                ("primitive framework", input.framework.unwrap_or("")),
                (
                    "primitive adaptation status",
                    input.adaptation_status.unwrap_or(""),
                ),
            ] {
                required(label, value)?;
            }
        }
        if input.kind == VisualEvidenceKind::SemanticToken {
            required("semantic role", input.semantic_role.unwrap_or(""))?;
            required("token value", input.value.unwrap_or(""))?;
        }
        if input.kind == VisualEvidenceKind::RejectedExample
            && input.polarity != IdentityPolarity::Negative
        {
            return Err(OrgIntelError::InvalidWork(
                "rejected visual examples must remain negative evidence".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO company_identity_evidence (id,pillar,statement_kind,claim_key,statement,author_id,source,authority,scope,observed_at,evidence_locator,polarity,status,channel,audience,supersedes_evidence_id,exception_expires_at,exception_indefinite) VALUES ($1,'visual',$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'active',$12,NULL,$13,NULL,FALSE)")
            .bind(id)
            .bind(if input.kind == VisualEvidenceKind::RejectedExample || input.kind == VisualEvidenceKind::ApprovedComposition { IdentityStatementKind::Example } else { IdentityStatementKind::Guidance })
            .bind(input.claim_key.trim()).bind(input.statement.trim()).bind(input.author_id).bind(input.source.trim()).bind(input.authority.trim()).bind(input.scope.trim()).bind(input.observed_at).bind(input.evidence_locator.trim()).bind(input.polarity).bind(input.channel.map(channel_name)).bind(input.supersedes_evidence_id).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO company_visual_evidence_details (evidence_id,kind,channel,purpose,rationale,semantic_role,value,reduced_motion_replacement,product_truth_locator,origin,licence,framework,dependencies,adaptation_status,accessibility_notes) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)")
            .bind(id).bind(input.kind).bind(input.channel).bind(input.purpose.trim()).bind(input.rationale.trim()).bind(input.semantic_role.map(str::trim)).bind(input.value.map(str::trim)).bind(input.reduced_motion_replacement.map(str::trim)).bind(input.product_truth_locator.map(str::trim)).bind(input.origin.map(str::trim)).bind(input.licence.map(str::trim)).bind(input.framework.map(str::trim)).bind(input.dependencies).bind(input.adaptation_status.map(str::trim)).bind(input.accessibility_notes.trim()).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(id)
    }

    pub async fn bind_visual_work_contract(
        &self,
        input: NewVisualWorkContract<'_>,
    ) -> Result<VisualWorkContractRow> {
        for (label, value) in [
            ("binding actor", input.bound_by),
            ("audience", input.audience),
            ("outcome", input.outcome),
            ("information hierarchy", input.information_hierarchy),
            ("proof", input.proof),
            ("density", input.density),
            ("imagery role", input.imagery_role),
            ("motion role", input.motion_role),
        ] {
            required(label, value)?;
        }
        if input.product_representation == VisualRepresentation::ExactProduct {
            required(
                "product truth locator",
                input.product_truth_locator.unwrap_or(""),
            )?;
        }
        let identity = self.bind_work_identity(input.work_id).await?;
        let canonical = format!("work={}\nrelease={}\nchannel={}\nbound_by={}\naudience={}\noutcome={}\nhierarchy={}\nproof={}\ndensity={}\nimagery={}\nmotion={}\nrepresentation={:?}\nproduct_truth={}\ndeparture={}\n", input.work_id, identity.release_id, channel_name(input.channel), input.bound_by.trim(), input.audience.trim(), input.outcome.trim(), input.information_hierarchy.trim(), input.proof.trim(), input.density.trim(), input.imagery_role.trim(), input.motion_role.trim(), input.product_representation, input.product_truth_locator.unwrap_or("").trim(), input.requested_departure.unwrap_or("").trim());
        let digest = format!("{:x}", Sha256::digest(canonical.as_bytes()));
        sqlx::query("INSERT INTO company_visual_work_contracts (work_id,release_id,channel,bound_by,audience,outcome,information_hierarchy,proof,density,imagery_role,motion_role,product_representation,product_truth_locator,requested_departure,contract_digest) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15) ON CONFLICT (work_id) DO NOTHING")
            .bind(input.work_id).bind(identity.release_id).bind(input.channel).bind(input.bound_by.trim()).bind(input.audience.trim()).bind(input.outcome.trim()).bind(input.information_hierarchy.trim()).bind(input.proof.trim()).bind(input.density.trim()).bind(input.imagery_role.trim()).bind(input.motion_role.trim()).bind(input.product_representation).bind(input.product_truth_locator.map(str::trim)).bind(input.requested_departure.map(str::trim)).bind(&digest).execute(&self.pool).await?;
        let row = self
            .visual_work_contract(input.work_id)
            .await?
            .expect("inserted visual contract");
        if row.contract_digest != digest {
            return Err(OrgIntelError::InvalidWork(
                "visual contract is set once for this Work; create a revision or new Work".into(),
            ));
        }
        Ok(row)
    }

    pub async fn visual_work_contract(
        &self,
        work_id: Uuid,
    ) -> Result<Option<VisualWorkContractRow>> {
        Ok(sqlx::query_as("SELECT work_id,release_id,channel,bound_by,audience,outcome,information_hierarchy,proof,density,imagery_role,motion_role,product_representation,product_truth_locator,requested_departure,contract_digest,bound_at FROM company_visual_work_contracts WHERE work_id=$1").bind(work_id).fetch_optional(&self.pool).await?)
    }

    pub async fn record_visual_primitive_use(
        &self,
        input: NewVisualPrimitiveUse<'_>,
    ) -> Result<()> {
        required("primitive version", input.primitive_version)?;
        required("primitive purpose", input.purpose)?;
        let usable: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM company_visual_work_contracts c JOIN company_identity_release_evidence re ON re.release_id=c.release_id JOIN company_visual_evidence_details d ON d.evidence_id=re.evidence_id JOIN company_identity_evidence e ON e.id=d.evidence_id WHERE c.work_id=$1 AND d.evidence_id=$2 AND d.kind='primitive' AND e.status='active' AND d.adaptation_status IN ('native','verified','adapted_and_verified'))")
            .bind(input.work_id).bind(input.evidence_id).fetch_one(&self.pool).await?;
        if !usable {
            return Err(OrgIntelError::InvalidWork("primitive is stale, unsupported, unverified, or absent from the Work-bound release".into()));
        }
        sqlx::query("INSERT INTO company_visual_primitive_uses (work_id,evidence_id,primitive_version,purpose) VALUES ($1,$2,$3,$4) ON CONFLICT (work_id,evidence_id) DO UPDATE SET primitive_version=EXCLUDED.primitive_version,purpose=EXCLUDED.purpose")
            .bind(input.work_id).bind(input.evidence_id).bind(input.primitive_version.trim()).bind(input.purpose.trim()).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn compile_visual_direction(
        &self,
        work_id: Uuid,
        max_bytes: usize,
    ) -> Result<VisualDirectionBrief> {
        if max_bytes < 1024 {
            return Err(OrgIntelError::InvalidWork(
                "visual direction bound must be at least 1024 bytes".into(),
            ));
        }
        let contract = self
            .visual_work_contract(work_id)
            .await?
            .ok_or_else(|| OrgIntelError::InvalidWork("Work has no visual contract".into()))?;
        let rows = sqlx::query_as::<_, (Uuid, VisualEvidenceKind, String, String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>)>("SELECT e.id,d.kind,e.statement,d.purpose,d.rationale,d.reduced_motion_replacement,d.product_truth_locator,d.origin,d.licence,d.framework FROM company_identity_release_evidence re JOIN company_identity_evidence e ON e.id=re.evidence_id JOIN company_visual_evidence_details d ON d.evidence_id=e.id WHERE re.release_id=$1 AND e.status='active' AND (d.channel IS NULL OR d.channel=$2) ORDER BY d.kind,e.claim_key,e.id")
            .bind(contract.release_id).bind(contract.channel).fetch_all(&self.pool).await?;
        let header = format!("# Company Visual Direction\nwork: {}\nrelease: {}\nchannel: {}\naudience: {}\noutcome: {}\n\n## Composition contract\n- Information hierarchy: {}\n- Proof: {}\n- Density: {}\n- Imagery role: {}\n- Motion role: {}\n- Product representation: {:?}\n- Product truth: {}\n- Requested departure: {}\n\nRegistry entries are capabilities, never quotas. Select only what serves this outcome. Exact product evidence must use the product truth locator; otherwise remain visibly abstract.\n\n## Released visual evidence\n", contract.work_id, contract.release_id, channel_name(contract.channel), contract.audience, contract.outcome, contract.information_hierarchy, contract.proof, contract.density, contract.imagery_role, contract.motion_role, contract.product_representation, contract.product_truth_locator.as_deref().unwrap_or("not required"), contract.requested_departure.as_deref().unwrap_or("none"));
        let lines = rows.iter().map(|(_, kind, statement, purpose, rationale, reduced, truth, origin, licence, framework)| format!("- [{}] {} — purpose: {}; judgement: {}; reduced/static: {}; product truth: {}; provenance: {} / {} / {}\n", kind_name(*kind), statement, purpose, rationale, reduced.as_deref().unwrap_or("not motion"), truth.as_deref().unwrap_or("not product depiction"), origin.as_deref().unwrap_or("company evidence"), licence.as_deref().unwrap_or("internal"), framework.as_deref().unwrap_or("native"))).collect::<Vec<_>>();
        let mut included = Vec::new();
        for i in 0..rows.len() {
            let candidate = format!(
                "{}{}\n## Native acceptance\n{}\n",
                header,
                lines[..=i].join(""),
                native_requirements(contract.channel).join(", ")
            );
            if candidate.len() > max_bytes {
                break;
            }
            included.push(rows[i].0);
        }
        let omitted = rows
            .iter()
            .map(|row| row.0)
            .filter(|id| !included.contains(id))
            .collect::<Vec<_>>();
        let body = format!("{}{}\n## Native acceptance\nRequired observed checks: {}. A restrained control must compete under the same truth and hierarchy. Judge identity, hierarchy, density, proof, product fidelity, motion meaning and visible defects; never component count.\n\n## Omission account\n{}\n", header, lines[..included.len()].join(""), native_requirements(contract.channel).join(", "), if omitted.is_empty() { "none".into() } else { format!("{} evidence items omitted by byte bound: {}", omitted.len(), omitted.iter().map(Uuid::to_string).collect::<Vec<_>>().join(",")) });
        let digest = format!("{:x}", Sha256::digest(body.as_bytes()));
        let bytes = body.len();
        Ok(VisualDirectionBrief {
            contract,
            body,
            digest,
            included_evidence_ids: included,
            omitted_evidence_ids: omitted,
            bytes,
        })
    }

    pub async fn record_visual_render_evidence(
        &self,
        input: NewVisualRenderEvidence<'_>,
    ) -> Result<Uuid> {
        for (label, value) in [
            ("renderer", input.renderer),
            ("renderer version", input.renderer_version),
            ("captured by", input.captured_by),
        ] {
            required(label, value)?;
        }
        require_checks(input.channel, input.native_checks)?;
        let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM artifact_refs a JOIN company_visual_work_contracts c ON c.work_id=$1 WHERE a.id=$2 AND (a.work_id IS NULL OR a.work_id=$1) AND a.state='available' AND a.digest IS NOT NULL AND c.channel=$3)").bind(input.work_id).bind(input.artifact_ref_id).bind(input.channel).fetch_one(&self.pool).await?;
        if !valid {
            return Err(OrgIntelError::InvalidWork(
                "visual review needs the available exact artifact for the Work-bound channel"
                    .into(),
            ));
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO company_visual_render_evidence (id,work_id,artifact_ref_id,channel,renderer,renderer_version,viewport_width,viewport_height,motion_state,native_checks,captured_by) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)")
            .bind(id).bind(input.work_id).bind(input.artifact_ref_id).bind(input.channel).bind(input.renderer.trim()).bind(input.renderer_version.trim()).bind(input.viewport_width).bind(input.viewport_height).bind(input.motion_state).bind(input.native_checks).bind(input.captured_by.trim()).execute(&self.pool).await?;
        Ok(id)
    }

    pub async fn record_visual_review(&self, input: NewVisualReview<'_>) -> Result<Uuid> {
        required("reviewer", input.reviewer)?;
        let producer: String = sqlx::query_scalar("SELECT a.created_by FROM company_visual_render_evidence r JOIN artifact_refs a ON a.id=r.artifact_ref_id WHERE r.id=$1 AND a.state='available'").bind(input.render_evidence_id).fetch_optional(&self.pool).await?.ok_or_else(|| OrgIntelError::InvalidWork("visual capture is absent or stale after artifact revision".into()))?;
        if producer == input.reviewer.trim() {
            return Err(OrgIntelError::InvalidWork(
                "consequential visual work needs an independent art director, not its producer"
                    .into(),
            ));
        }
        let findings = [
            input.identity_findings,
            input.hierarchy_findings,
            input.density_findings,
            input.proof_findings,
            input.product_fidelity_findings,
            input.motion_findings,
            input.defect_findings,
        ]
        .join("");
        if input.verdict != VisualReviewVerdict::Accept && findings.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "a visual revision or rejection needs a concrete finding".into(),
            ));
        }
        if let Some(control) = input.control_render_evidence_id {
            let comparable: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM company_visual_render_evidence candidate JOIN company_visual_render_evidence control ON control.id=$2 WHERE candidate.id=$1 AND candidate.work_id=control.work_id AND candidate.channel=control.channel AND candidate.motion_state=control.motion_state)").bind(input.render_evidence_id).bind(control).fetch_one(&self.pool).await?;
            if !comparable {
                return Err(OrgIntelError::InvalidWork(
                    "restrained control must share Work, channel and motion state".into(),
                ));
            }
        }
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO company_visual_reviews (id,render_evidence_id,control_render_evidence_id,reviewer,verdict,identity_findings,hierarchy_findings,density_findings,proof_findings,product_fidelity_findings,motion_findings,defect_findings,departure_decision) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)")
            .bind(id).bind(input.render_evidence_id).bind(input.control_render_evidence_id).bind(input.reviewer.trim()).bind(input.verdict).bind(input.identity_findings.trim()).bind(input.hierarchy_findings.trim()).bind(input.density_findings.trim()).bind(input.proof_findings.trim()).bind(input.product_fidelity_findings.trim()).bind(input.motion_findings.trim()).bind(input.defect_findings.trim()).bind(input.departure_decision.trim()).execute(&self.pool).await?;
        Ok(id)
    }
}
