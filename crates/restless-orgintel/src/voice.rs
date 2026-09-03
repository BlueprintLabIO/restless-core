//! Channel-credible human company voice (S32).
//!
//! Voice evidence remains ordinary Company Identity evidence. A Work contract
//! names the human situation and authorship explicitly; review binds the exact
//! rendered artifact rather than a producer rationale or phrase linter.

use super::*;
use std::collections::BTreeSet;

pub(super) fn voice_nonempty(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(OrgIntelError::InvalidWork(format!(
            "company voice {label} cannot be empty"
        )));
    }
    Ok(())
}

pub(super) fn channel_name(channel: VoiceChannel) -> &'static str {
    match channel {
        VoiceChannel::Newsletter => "newsletter",
        VoiceChannel::FounderEmail => "founder_email",
        VoiceChannel::Support => "support",
        VoiceChannel::TransactionalEmail => "transactional_email",
        VoiceChannel::ProductUi => "product_ui",
        VoiceChannel::Blog => "blog",
    }
}

fn information_order(channel: VoiceChannel) -> &'static str {
    match channel {
        VoiceChannel::Newsletter => {
            "observation → pointed conclusion → reasoning and proof → byline"
        }
        VoiceChannel::FounderEmail => {
            "personal decision → concrete reason → requested action → sign-off"
        }
        VoiceChannel::Support => "acknowledge the problem → known state → next action → ownership",
        VoiceChannel::TransactionalEmail => "status → consequence → required action → support path",
        VoiceChannel::ProductUi => "current state → consequence → recovery action",
        VoiceChannel::Blog => {
            "standalone context → raw observations → deeper reasoning → useful conclusion"
        }
    }
}

fn statement_kind(kind: VoiceEvidenceKind) -> IdentityStatementKind {
    match kind {
        VoiceEvidenceKind::ApprovedPassage | VoiceEvidenceKind::RejectedPassage => {
            IdentityStatementKind::Example
        }
        VoiceEvidenceKind::ExpressionPrinciple | VoiceEvidenceKind::Vocabulary => {
            IdentityStatementKind::Guidance
        }
        VoiceEvidenceKind::NamedAuthor | VoiceEvidenceKind::ChannelObservation => {
            IdentityStatementKind::Observation
        }
    }
}

fn require_native_checks(channel: VoiceChannel, checks: &serde_json::Value) -> Result<()> {
    let required: &[&str] = match channel {
        VoiceChannel::Newsletter
        | VoiceChannel::FounderEmail
        | VoiceChannel::TransactionalEmail => &[
            "desktop_wrap",
            "narrow_wrap",
            "text_fallback",
            "subject_visible",
            "preheader_visible",
            "links_operable",
        ],
        VoiceChannel::Support => &[
            "desktop_wrap",
            "narrow_wrap",
            "links_operable",
            "reply_context_visible",
        ],
        VoiceChannel::ProductUi => &[
            "state_visible",
            "action_operable",
            "recovery_operable",
            "focus_order",
            "truncation_checked",
        ],
        VoiceChannel::Blog => &[
            "standalone",
            "internal_dependencies",
            "reading_measure",
            "links_operable",
            "responsive_wrap",
        ],
    };
    let missing = required
        .iter()
        .filter(|key| checks.get(**key).is_none())
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(OrgIntelError::InvalidWork(format!(
            "native {} evidence is missing observed checks: {}",
            channel_name(channel),
            missing.join(", ")
        )));
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct CompiledVoiceEvidence {
    id: Uuid,
    pillar: IdentityPillar,
    statement_kind: IdentityStatementKind,
    statement: String,
    source: String,
    evidence_locator: String,
    polarity: IdentityPolarity,
    voice_kind: Option<VoiceEvidenceKind>,
    judgement_reason: Option<String>,
    named_author: Option<String>,
}

impl OrgIntel {
    pub async fn add_voice_evidence(&self, input: NewVoiceEvidence<'_>) -> Result<Uuid> {
        for (label, value) in [
            ("claim key", input.claim_key),
            ("passage or principle", input.passage_or_principle),
            ("author", input.author_id),
            ("source", input.source),
            ("authority", input.authority),
            ("scope", input.scope),
            ("evidence locator", input.evidence_locator),
            ("judgement reason", input.judgement_reason),
        ] {
            voice_nonempty(label, value)?;
        }
        if input.kind == VoiceEvidenceKind::NamedAuthor && input.named_author.is_none() {
            return Err(OrgIntelError::InvalidWork(
                "named-author voice evidence must name its author".into(),
            ));
        }
        if let Some(author) = input.named_author {
            voice_nonempty("named author", author)?;
        }
        if let Some(audience) = input.audience {
            voice_nonempty("audience", audience)?;
        }
        if input.kind == VoiceEvidenceKind::RejectedPassage
            && input.polarity != IdentityPolarity::Negative
        {
            return Err(OrgIntelError::InvalidWork(
                "rejected voice evidence must remain negative evidence".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let evidence_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO company_identity_evidence \
             (id,pillar,statement_kind,claim_key,statement,author_id,source,authority,scope,\
              observed_at,evidence_locator,polarity,status,channel,audience,supersedes_evidence_id,\
              exception_expires_at,exception_indefinite) \
             VALUES ($1,'voice',$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,'active',$12,$13,$14,NULL,FALSE)",
        )
        .bind(evidence_id)
        .bind(statement_kind(input.kind))
        .bind(input.claim_key.trim())
        .bind(input.passage_or_principle.trim())
        .bind(input.author_id)
        .bind(input.source.trim())
        .bind(input.authority.trim())
        .bind(input.scope.trim())
        .bind(input.observed_at)
        .bind(input.evidence_locator.trim())
        .bind(input.polarity)
        .bind(input.channel.map(channel_name))
        .bind(input.audience.map(str::trim))
        .bind(input.supersedes_evidence_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO company_voice_evidence_details \
             (evidence_id,kind,judgement_reason,named_author,channel,audience) \
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(evidence_id)
        .bind(input.kind)
        .bind(input.judgement_reason.trim())
        .bind(input.named_author.map(str::trim))
        .bind(input.channel)
        .bind(input.audience.map(str::trim))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(evidence_id)
    }

    pub async fn bind_voice_work_contract(
        &self,
        input: NewVoiceWorkContract<'_>,
    ) -> Result<VoiceWorkContractRow> {
        for (label, value) in [
            ("author", input.author),
            ("binding actor", input.bound_by),
            ("audience", input.audience),
            ("reader situation", input.reader_situation),
            ("desired understanding", input.desired_understanding),
            ("desired action", input.desired_action),
            ("proof", input.proof),
            ("consequence", input.consequence),
        ] {
            voice_nonempty(label, value)?;
        }
        let identity = self.bind_work_identity(input.work_id).await?;
        let canonical = format!(
            "work={}\nrelease={}\nchannel={}\nauthor={}\nbound_by={}\naudience={}\nreader={}\nunderstanding={}\naction={}\nproof={}\nconsequence={}\n",
            input.work_id,
            identity.release_id,
            channel_name(input.channel),
            input.author.trim(),
            input.bound_by.trim(),
            input.audience.trim(),
            input.reader_situation.trim(),
            input.desired_understanding.trim(),
            input.desired_action.trim(),
            input.proof.trim(),
            input.consequence.trim(),
        );
        let digest = format!("{:x}", Sha256::digest(canonical.as_bytes()));
        sqlx::query(
            "INSERT INTO company_voice_work_contracts \
             (work_id,release_id,channel,author,bound_by,audience,reader_situation,desired_understanding,\
              desired_action,proof,consequence,contract_digest) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) ON CONFLICT (work_id) DO NOTHING",
        )
        .bind(input.work_id)
        .bind(identity.release_id)
        .bind(input.channel)
        .bind(input.author.trim())
        .bind(input.bound_by.trim())
        .bind(input.audience.trim())
        .bind(input.reader_situation.trim())
        .bind(input.desired_understanding.trim())
        .bind(input.desired_action.trim())
        .bind(input.proof.trim())
        .bind(input.consequence.trim())
        .bind(&digest)
        .execute(&self.pool)
        .await?;
        let row = sqlx::query_as::<_, VoiceWorkContractRow>(
            "SELECT work_id,release_id,channel,author,bound_by,audience,reader_situation,desired_understanding,\
             desired_action,proof,consequence,contract_digest,bound_at \
             FROM company_voice_work_contracts WHERE work_id=$1",
        )
        .bind(input.work_id)
        .fetch_one(&self.pool)
        .await?;
        if row.contract_digest != digest {
            return Err(OrgIntelError::InvalidWork(
                "company voice contract is set once for this Work; create a revision or new Work"
                    .into(),
            ));
        }
        Ok(row)
    }

    pub async fn voice_work_contract(&self, work_id: Uuid) -> Result<Option<VoiceWorkContractRow>> {
        Ok(sqlx::query_as::<_, VoiceWorkContractRow>(
            "SELECT work_id,release_id,channel,author,bound_by,audience,reader_situation,desired_understanding,\
             desired_action,proof,consequence,contract_digest,bound_at \
             FROM company_voice_work_contracts WHERE work_id=$1",
        )
        .bind(work_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn compile_voice_contract(
        &self,
        work_id: Uuid,
        max_bytes: usize,
    ) -> Result<VoiceContractBrief> {
        if max_bytes < 1024 {
            return Err(OrgIntelError::InvalidWork(
                "company voice contract bound must be at least 1024 bytes".into(),
            ));
        }
        let contract = self.voice_work_contract(work_id).await?.ok_or_else(|| {
            OrgIntelError::InvalidWork("Work has no company voice contract".into())
        })?;
        let rows = sqlx::query_as::<_, CompiledVoiceEvidence>(
            "SELECT e.id,e.pillar,e.statement_kind,e.statement,e.source,e.evidence_locator,\
                    e.polarity,d.kind AS voice_kind,d.judgement_reason,d.named_author \
             FROM company_identity_release_evidence re \
             JOIN company_identity_evidence e ON e.id=re.evidence_id \
             LEFT JOIN company_voice_evidence_details d ON d.evidence_id=e.id \
             WHERE re.release_id=$1 AND e.status='active' AND (\
               (e.pillar='truth' AND e.statement_kind='fact') OR \
               (e.pillar='voice' AND d.evidence_id IS NOT NULL \
                AND (d.channel IS NULL OR d.channel=$2) \
                AND (d.audience IS NULL OR d.audience=$4) \
                AND (d.kind <> 'named_author' OR d.named_author=$3))\
             ) ORDER BY e.pillar,e.statement_kind,e.claim_key,e.id",
        )
        .bind(contract.release_id)
        .bind(contract.channel)
        .bind(&contract.author)
        .bind(&contract.audience)
        .fetch_all(&self.pool)
        .await?;
        let header = format!(
            "# Company Voice Contract\nwork: {}\nrelease: {}\nchannel: {}\nauthor: {}\naudience: {}\n\n\
             ## Communication outcome\nCreate the strongest truthful expression for this human situation. Start with value, clarity and a natural voice; use evidence to make the message credible rather than making process or governance the message.\n- Reader: {}\n- Value they should understand: {}\n- Action that should feel easy: {}\n- Credibility available to support the value: {}\n- Why this communication matters: {}\n- Information order: {}\n\n",
            contract.work_id,
            contract.release_id,
            channel_name(contract.channel),
            contract.author,
            contract.audience,
            contract.reader_situation,
            contract.desired_understanding,
            contract.desired_action,
            contract.proof,
            contract.consequence,
            information_order(contract.channel),
        );
        let lines = rows
            .iter()
            .map(|row| {
                if row.pillar == IdentityPillar::Truth {
                    format!(
                        "- [Truth/{:?}] {} — source: {}; evidence: {}\n",
                        row.statement_kind, row.statement, row.source, row.evidence_locator
                    )
                } else {
                    format!(
                        "- [Voice/{:?}/{:?}] {} — judgement: {}; author scope: {}; source: {}; evidence: {}\n",
                        row.voice_kind.expect("voice detail selected"),
                        row.polarity,
                        row.statement,
                        row.judgement_reason.as_deref().unwrap_or("not recorded"),
                        row.named_author.as_deref().unwrap_or("company bounds"),
                        row.source,
                        row.evidence_locator,
                    )
                }
            })
            .collect::<Vec<_>>();
        let mut included_count = 0;
        for count in 0..=rows.len() {
            let omitted = rows[count..].iter().map(|row| row.id).collect::<Vec<_>>();
            let body = render_voice_contract(&header, &lines[..count], &omitted);
            if body.len() <= max_bytes {
                included_count = count;
            } else {
                break;
            }
        }
        let included = rows[..included_count]
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>();
        let omitted = rows[included_count..]
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>();
        let body = render_voice_contract(&header, &lines[..included_count], &omitted);
        if body.len() > max_bytes {
            return Err(OrgIntelError::InvalidWork(format!(
                "company voice contract metadata exceed {max_bytes} bytes"
            )));
        }
        let digest = format!("{:x}", Sha256::digest(body.as_bytes()));
        Ok(VoiceContractBrief {
            contract,
            bytes: body.len(),
            body,
            digest,
            included_evidence_ids: included,
            omitted_evidence_ids: omitted,
        })
    }

    pub async fn record_voice_render_evidence(
        &self,
        input: NewVoiceRenderEvidence<'_>,
    ) -> Result<Uuid> {
        for (label, value) in [
            ("renderer", input.renderer),
            ("renderer version", input.renderer_version),
            ("captured by", input.captured_by),
        ] {
            voice_nonempty(label, value)?;
        }
        if !input.semantic_checks.is_object() {
            return Err(OrgIntelError::InvalidWork(
                "native voice checks must be a JSON object".into(),
            ));
        }
        require_native_checks(input.channel, input.semantic_checks)?;
        let available: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM artifact_refs WHERE id=$1 AND state='available' \
             AND digest IS NOT NULL)",
        )
        .bind(input.artifact_ref_id)
        .fetch_one(&self.pool)
        .await?;
        if !available {
            return Err(OrgIntelError::InvalidWork(
                "native voice review needs an available artifact with an exact digest".into(),
            ));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO company_voice_render_evidence \
             (id,artifact_ref_id,channel,renderer,renderer_version,semantic_checks,captured_by) \
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(id)
        .bind(input.artifact_ref_id)
        .bind(input.channel)
        .bind(input.renderer.trim())
        .bind(input.renderer_version.trim())
        .bind(input.semantic_checks)
        .bind(input.captured_by.trim())
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn record_voice_review(&self, input: NewVoiceReview<'_>) -> Result<Uuid> {
        voice_nonempty("reviewer", input.reviewer)?;
        let producer: String = sqlx::query_scalar(
            "SELECT a.created_by FROM company_voice_render_evidence r \
             JOIN artifact_refs a ON a.id=r.artifact_ref_id \
             WHERE r.id=$1 AND a.state='available'",
        )
        .bind(input.render_evidence_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidWork(
                "voice render evidence is absent or stale after artifact revision".into(),
            )
        })?;
        if producer == input.reviewer.trim() {
            return Err(OrgIntelError::InvalidWork(
                "consequential voice copy needs a fresh reviewer, not its artifact producer".into(),
            ));
        }
        let findings = [
            input.factual_findings,
            input.abstraction_findings,
            input.repetition_findings,
            input.channel_findings,
            input.authorship_findings,
        ]
        .join("");
        if input.verdict != VoiceReviewVerdict::Accept && findings.trim().is_empty() {
            return Err(OrgIntelError::InvalidWork(
                "a voice revision or rejection needs a concrete finding".into(),
            ));
        }
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO company_voice_reviews \
             (id,render_evidence_id,reviewer,verdict,factual_findings,abstraction_findings,\
              repetition_findings,channel_findings,authorship_findings,concepts_removed) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(id)
        .bind(input.render_evidence_id)
        .bind(input.reviewer.trim())
        .bind(input.verdict)
        .bind(input.factual_findings.trim())
        .bind(input.abstraction_findings.trim())
        .bind(input.repetition_findings.trim())
        .bind(input.channel_findings.trim())
        .bind(input.authorship_findings.trim())
        .bind(input.concepts_removed.trim())
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn propose_voice_learning(
        &self,
        input: NewVoiceLearningProposal<'_>,
    ) -> Result<Option<Uuid>> {
        if input.change_kind != VoiceLearningKind::VoiceObservation {
            return Ok(None);
        }
        if input.before_artifact_ref_id == input.after_artifact_ref_id {
            return Err(OrgIntelError::InvalidWork(
                "voice learning needs distinct before and after artifacts".into(),
            ));
        }
        for artifact in [input.before_artifact_ref_id, input.after_artifact_ref_id] {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM artifact_refs WHERE id=$1 AND digest IS NOT NULL)",
            )
            .bind(artifact)
            .fetch_one(&self.pool)
            .await?;
            if !exists {
                return Err(OrgIntelError::InvalidWork(
                    "voice learning needs exact before and after artifact identities".into(),
                ));
            }
        }
        let evidence_id = self
            .add_voice_evidence(NewVoiceEvidence {
                kind: VoiceEvidenceKind::ChannelObservation,
                claim_key: input.claim_key,
                passage_or_principle: input.observation,
                author_id: input.created_by,
                named_author: input.named_author,
                source: input.source,
                authority: "proposal only",
                scope: input.scope,
                observed_at: input.observed_at,
                evidence_locator: input.evidence_locator,
                judgement_reason: input.motivating_decision,
                polarity: IdentityPolarity::Positive,
                channel: input.channel,
                audience: input.audience,
                supersedes_evidence_id: None,
            })
            .await?;
        let current: Uuid = sqlx::query_scalar(
            "SELECT release_id FROM company_identity_current_release WHERE singleton=TRUE",
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidWork("company has no released identity".into()))?;
        let mut evidence_ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT evidence_id FROM company_identity_release_evidence WHERE release_id=$1 \
             ORDER BY evidence_id",
        )
        .bind(current)
        .fetch_all(&self.pool)
        .await?;
        evidence_ids.push(evidence_id);
        let unique = evidence_ids.iter().copied().collect::<BTreeSet<_>>();
        evidence_ids = unique.into_iter().collect();
        let proposal_id = self
            .propose_identity_release(input.created_by, input.motivating_decision, &evidence_ids)
            .await?;
        sqlx::query(
            "INSERT INTO company_voice_learning_proposals \
             (proposal_id,evidence_id,before_artifact_ref_id,after_artifact_ref_id,change_kind,\
              motivating_decision,scope) VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(proposal_id)
        .bind(evidence_id)
        .bind(input.before_artifact_ref_id)
        .bind(input.after_artifact_ref_id)
        .bind(input.change_kind)
        .bind(input.motivating_decision.trim())
        .bind(input.scope.trim())
        .execute(&self.pool)
        .await?;
        Ok(Some(proposal_id))
    }
}

fn render_voice_contract(header: &str, lines: &[String], omitted: &[Uuid]) -> String {
    let mut body = String::from(header);
    body.push_str("## Released truth and voice evidence\n");
    for line in lines {
        body.push_str(line);
    }
    body.push_str(
        "\n## Copy-desk prompts\nJudge abstraction, unsupported claims, repetition, channel fit and whether the named author could sign it. Negative examples are evidence, never a phrase blacklist. Compare a plain control under the same facts.\n\n## Omission account\n",
    );
    if omitted.is_empty() {
        body.push_str("none\n");
    } else {
        body.push_str(&format!(
            "{} evidence items omitted by byte bound: {}\n",
            omitted.len(),
            omitted
                .iter()
                .map(Uuid::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    body
}
