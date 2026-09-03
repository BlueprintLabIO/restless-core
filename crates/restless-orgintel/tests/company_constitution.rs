//! S35 integrated walking slice. The constitution composes independently
//! governed pillars, binds exact artifacts, exposes correction drift and
//! transfers without leaking one company's identity into another.

use chrono::Utc;
use restless_orgintel::{
    ConstitutionLearningTrigger, CultureCase, CultureConfidence, CultureEvidenceKind,
    IdentityEvidenceStatus, IdentityMigrationDisposition, IdentityPillar, IdentityPolarity,
    IdentityStatementKind, InitialConstitutionContracts, InitialCultureContract,
    InitialVisualContract, InitialVoiceContract, NewArtifactRef, NewConstitutionArtifactBinding,
    NewConstitutionLearningProposal, NewCultureEvidence, NewIdentityEvidence,
    NewIdentityMigrationDecision, NewVisualEvidence, NewVoiceEvidence, NewWork, OrgIntel,
    ProducingTopology, VisualChannel, VisualEvidenceKind, VisualRepresentation, VoiceChannel,
    VoiceEvidenceKind, WorkspaceSpec,
};

struct Fixture {
    org: OrgIntel,
    release: uuid::Uuid,
    work: uuid::Uuid,
    evidence: Vec<uuid::Uuid>,
}

async fn fixture(
    url: &str,
    schema_prefix: &str,
    product_fact: &str,
    voice_rule: &str,
    visual_rule: &str,
    conduct: &str,
) -> Fixture {
    let schema = format!("{}{}", schema_prefix, uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(url, &schema).await.unwrap();
    org.ensure_actor("owner", "owner", "owner", "Owner")
        .await
        .unwrap();
    org.ensure_actor("identity-maker", "staff", "writer", "Identity maker")
        .await
        .unwrap();
    org.ensure_actor(
        "product-direction",
        "staff",
        "design lead",
        "Product direction",
    )
    .await
    .unwrap();
    let team = org
        .create_team(
            "Product direction",
            "Produce one decision-ready product release package.",
            "product-direction",
            "owner",
        )
        .await
        .unwrap();
    org.set_actor_team(
        "identity-maker",
        Some(team),
        "product-direction",
        "the writer owns production under one accountable lead",
    )
    .await
    .unwrap();
    let now = Utc::now();
    let truth = org
        .add_identity_evidence(NewIdentityEvidence {
            pillar: IdentityPillar::Truth,
            statement_kind: IdentityStatementKind::Fact,
            claim_key: "product.promise",
            statement: product_fact,
            author_id: "owner",
            source: "signed product decision",
            authority: "owner",
            scope: "company",
            observed_at: now,
            evidence_locator: "evidence/product-decision",
            polarity: IdentityPolarity::Neutral,
            status: IdentityEvidenceStatus::Active,
            channel: None,
            audience: None,
            supersedes_evidence_id: None,
            exception_expires_at: None,
            exception_indefinite: false,
        })
        .await
        .unwrap();
    let voice = org
        .add_voice_evidence(NewVoiceEvidence {
            kind: VoiceEvidenceKind::ExpressionPrinciple,
            claim_key: "voice.product",
            passage_or_principle: voice_rule,
            author_id: "owner",
            named_author: None,
            source: "accepted human copy",
            authority: "owner",
            scope: "company",
            observed_at: now,
            evidence_locator: "evidence/accepted-copy",
            judgement_reason:
                "The reader understood the action and consequence without translation.",
            polarity: IdentityPolarity::Positive,
            channel: Some(VoiceChannel::ProductUi),
            audience: Some("operators"),
            supersedes_evidence_id: None,
        })
        .await
        .unwrap();
    let visual = org
        .add_visual_evidence(NewVisualEvidence {
            kind: VisualEvidenceKind::CompositionPrinciple,
            claim_key: "visual.product",
            statement: visual_rule,
            author_id: "owner",
            source: "accepted native product capture",
            authority: "owner",
            scope: "company",
            observed_at: now,
            evidence_locator: "evidence/product-capture",
            rationale: "The composition makes current state and consequence legible.",
            purpose: "product decision surface",
            polarity: IdentityPolarity::Positive,
            channel: Some(VisualChannel::Product),
            semantic_role: None,
            value: None,
            reduced_motion_replacement: Some("preserve state changes without interpolation"),
            product_truth_locator: Some("evidence/product-decision"),
            origin: None,
            licence: None,
            framework: None,
            dependencies: &serde_json::json!([]),
            adaptation_status: None,
            accessibility_notes: "Meaning survives colour removal and keyboard navigation.",
            supersedes_evidence_id: None,
        })
        .await
        .unwrap();
    let culture = org
        .add_culture_evidence(NewCultureEvidence {
            kind: CultureEvidenceKind::FoundingDecision,
            case_kind: Some(CultureCase::QualityTradeoff),
            claim_key: "culture.quality",
            statement: "The accountable lead protects the quality of the shipped outcome under schedule pressure.",
            author_id: "owner",
            source: "exercised product decision",
            authority: "owner",
            scope: "company",
            observed_at: now,
            evidence_locator: "evidence/quality-decision",
            polarity: IdentityPolarity::Positive,
            situation: "A smaller release is complete while a larger one is visually unfinished.",
            consequence: "Customers will judge the product by the shipped surface.",
            actors: "product lead and design engineer",
            decision_authority: "accountable product lead within approved Work",
            conduct,
            observed_outcome: "The smaller coherent release was accepted without a repair sprint.",
            confidence: CultureConfidence::OwnerFounded,
            counterexample: "Shipping the larger broken surface to satisfy an internal date.",
            boundary_conditions: "Safety, honesty and external-effect authority are unchanged.",
            operational_implication:
                "Prefer a smaller complete outcome when expansion lowers quality.",
            actor_scope: "role:accountable lead",
            supersedes_evidence_id: None,
        })
        .await
        .unwrap();
    let evidence = vec![truth, voice, visual, culture];
    let proposal = org
        .propose_identity_release("identity-maker", "first integrated release", &evidence)
        .await
        .unwrap();
    let release = org
        .promote_identity_proposal(
            proposal,
            "owner",
            "authority:identity:integrated-1",
            "Established four independently evidenced identity pillars.",
            now,
        )
        .await
        .unwrap();
    let work = org
        .add_commissioned_work_with_constitution(
            NewWork {
                owner_id: "identity-maker",
                title: "product release package",
                outcome: "A decision-ready product release package",
                goal_id: None,
                priority: 1,
                expected_artifact: "native release asset",
                workspace: WorkspaceSpec::default(),
                attempt_limit: Some(1),
            },
            &[],
            &[],
            &[],
            false,
            None,
            "product-direction",
            ProducingTopology::CoherentSingleWorker,
            &InitialConstitutionContracts {
                voice: Some(InitialVoiceContract {
                    channel: VoiceChannel::ProductUi,
                    author: "product lead".into(),
                    audience: "operators".into(),
                    reader_situation: "An operator needs to decide whether work is ready.".into(),
                    desired_understanding: "The state, evidence and next decision are visible."
                        .into(),
                    desired_action: "Make the bounded decision.".into(),
                    proof: "The exact product state and evidence reference.".into(),
                    consequence: "The operator spends less attention reconstructing state.".into(),
                }),
                visual: Some(InitialVisualContract {
                    channel: VisualChannel::Product,
                    audience: "operators".into(),
                    outcome: "Make state and consequence immediately legible.".into(),
                    information_hierarchy: "decision, evidence, supporting context".into(),
                    proof: "exact product state".into(),
                    density: "dense enough for work; no decorative dashboard filler".into(),
                    imagery_role: "none; the product itself is the proof".into(),
                    motion_role: "show state transition only".into(),
                    product_representation: VisualRepresentation::ExactProduct,
                    product_truth_locator: Some("evidence/product-decision".into()),
                    requested_departure: None,
                }),
                culture: Some(InitialCultureContract {
                    case_kind: CultureCase::QualityTradeoff,
                    actor: "product lead".into(),
                    actor_role: "accountable lead".into(),
                    team: "product".into(),
                    consequence: "A poor release consumes customer trust and a repair sprint."
                        .into(),
                    decision_boundary:
                        "May narrow scope; may not publish without existing authority.".into(),
                }),
            },
        )
        .await
        .unwrap();
    Fixture {
        org,
        release,
        work,
        evidence,
    }
}

async fn artifact(org: &OrgIntel, label: &str, digest_byte: char) -> uuid::Uuid {
    let digest = digest_byte.to_string().repeat(64);
    org.link_work_artifact(NewArtifactRef {
        kind: "rendered-asset",
        uri: &format!("artifact://{label}"),
        note: "exact immutable review artifact",
        created_by: "identity-maker",
        work_id: None,
        attempt_id: None,
        digest: Some(&digest),
        source_commit: None,
        runtime_generation: Some("s35"),
        label,
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn constitution_is_bounded_immutable_and_company_specific() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Company Constitution scenario");
        return;
    };
    let first = fixture(
        &url,
        "constitutiona",
        "Restless runs accountable company work and returns evidence-backed decisions.",
        "Name the person, action, proof and consequence in plain language.",
        "Use the product's compact operational grammar; motion explains a real state change.",
        "Narrow scope before accepting visibly unfinished work.",
    )
    .await;
    let brief = first
        .org
        .compile_constitution(first.work, 32 * 1024)
        .await
        .unwrap();
    let repeated = first
        .org
        .compile_constitution(first.work, 32 * 1024)
        .await
        .unwrap();
    assert_eq!(brief, repeated);
    assert!(brief.bytes <= 32 * 1024);
    assert_eq!(brief.pillars.len(), 4);
    assert!(brief
        .pillars
        .iter()
        .all(|pillar| pillar.status == "available"));
    assert!(
        brief
            .pillars
            .iter()
            .all(|pillar| !pillar.included_evidence_ids.is_empty()),
        "{:?}",
        brief.pillars
    );
    assert!(brief.body.contains("Effect boundary"));
    let included = brief
        .pillars
        .iter()
        .flat_map(|pillar| pillar.included_evidence_ids.iter().copied())
        .collect::<Vec<_>>();

    let exact = artifact(&first.org, "product-ui-v1", 'a').await;
    let binding = first
        .org
        .bind_constitution_artifact(NewConstitutionArtifactBinding {
            artifact_ref_id: exact,
            work_id: first.work,
            channel: "product_ui",
            audience: "operators",
            named_author: "product lead",
            producer: "identity-maker",
            accountable_lead: "product lead",
            company_voice: "company",
            native_evidence: &serde_json::json!({"renderer":"browser","viewport":"1440x900","verdict":"accept"}),
            constitution_digest: &brief.digest,
            evidence_ids: &included,
        })
        .await
        .unwrap();
    assert_eq!(binding.release_id, first.release);
    let changed = first
        .org
        .bind_constitution_artifact(NewConstitutionArtifactBinding {
            artifact_ref_id: exact,
            work_id: first.work,
            channel: "blog",
            audience: "everyone",
            named_author: "someone else",
            producer: "identity-maker",
            accountable_lead: "product lead",
            company_voice: "company",
            native_evidence: &serde_json::json!({"renderer":"browser"}),
            constitution_digest: &brief.digest,
            evidence_ids: &included,
        })
        .await
        .unwrap_err();
    assert!(changed.to_string().contains("immutable"));

    let corrected = first
        .org
        .add_identity_evidence(NewIdentityEvidence {
            pillar: IdentityPillar::Truth,
            statement_kind: IdentityStatementKind::Fact,
            claim_key: "product.promise",
            statement: "Restless runs bounded company work and returns exact evidence for an owner decision.",
            author_id: "owner",
            source: "signed correction",
            authority: "owner",
            scope: "company",
            observed_at: Utc::now(),
            evidence_locator: "evidence/product-correction",
            polarity: IdentityPolarity::Neutral,
            status: IdentityEvidenceStatus::Active,
            channel: None,
            audience: None,
            supersedes_evidence_id: Some(first.evidence[0]),
            exception_expires_at: None,
            exception_indefinite: false,
        })
        .await
        .unwrap();
    let correction = first
        .org
        .propose_identity_release(
            "identity-maker",
            "correct one product fact",
            &[
                corrected,
                first.evidence[1],
                first.evidence[2],
                first.evidence[3],
            ],
        )
        .await
        .unwrap();
    let next = first
        .org
        .promote_identity_proposal(
            correction,
            "owner",
            "authority:identity:correction",
            "Corrected one product fact; expression and culture remain unchanged.",
            Utc::now(),
        )
        .await
        .unwrap();
    let drift = first.org.compute_identity_drift(next).await.unwrap();
    assert_eq!(drift.len(), 1);
    assert_eq!(drift[0].artifact_ref_id, exact);
    assert_eq!(drift[0].new_evidence_id, Some(corrected));
    assert!(first
        .org
        .decide_identity_migration(NewIdentityMigrationDecision {
            drift_finding_id: drift[0].id,
            disposition: IdentityMigrationDisposition::Revise,
            decided_by: "identity-maker",
            rationale: "staff cannot decide",
            authority_record_id: "authority:no",
        })
        .await
        .unwrap_err()
        .to_string()
        .contains("only the owner"));
    let decision = first
        .org
        .decide_identity_migration(NewIdentityMigrationDecision {
            drift_finding_id: drift[0].id,
            disposition: IdentityMigrationDisposition::Revise,
            decided_by: "owner",
            rationale: "Revise only this decision surface because it repeats the corrected fact.",
            authority_record_id: "authority:migration:1",
        })
        .await
        .unwrap();
    assert_eq!(decision.disposition, IdentityMigrationDisposition::Revise);
    assert!(first
        .org
        .decide_identity_migration(NewIdentityMigrationDecision {
            drift_finding_id: drift[0].id,
            disposition: IdentityMigrationDisposition::Retain,
            decided_by: "owner",
            rationale: "A later request cannot rewrite the first owner decision.",
            authority_record_id: "authority:migration:2",
        })
        .await
        .unwrap_err()
        .to_string()
        .contains("migration decision is immutable"));

    let after = artifact(&first.org, "product-ui-v2", 'b').await;
    let learning = first
        .org
        .propose_constitution_learning(NewConstitutionLearningProposal {
            created_by: "identity-maker",
            evidence_id: corrected,
            pillar: IdentityPillar::Truth,
            trigger_kind: ConstitutionLearningTrigger::OwnerEvidence,
            triggering_event: "owner corrected the accepted product statement",
            before_artifact_ref_id: exact,
            after_artifact_ref_id: after,
            scope: "product promise",
            contradiction_check: "No active accepted evidence contradicts the corrected statement.",
        })
        .await
        .unwrap();
    assert_ne!(learning, uuid::Uuid::nil());

    let generated = first
        .org
        .add_identity_evidence(NewIdentityEvidence {
            pillar: IdentityPillar::Voice,
            statement_kind: IdentityStatementKind::Guidance,
            claim_key: "voice.generated-majority",
            statement: "Repeat the phrase because several model drafts used it.",
            author_id: "identity-maker",
            source: "generated repetition",
            authority: "model majority",
            scope: "company",
            observed_at: Utc::now(),
            evidence_locator: "generated/drafts",
            polarity: IdentityPolarity::Positive,
            status: IdentityEvidenceStatus::Active,
            channel: None,
            audience: None,
            supersedes_evidence_id: None,
            exception_expires_at: None,
            exception_indefinite: false,
        })
        .await
        .unwrap();
    assert!(first
        .org
        .propose_constitution_learning(NewConstitutionLearningProposal {
            created_by: "identity-maker",
            evidence_id: generated,
            pillar: IdentityPillar::Voice,
            trigger_kind: ConstitutionLearningTrigger::ExercisedOutcome,
            triggering_event: "the models repeated a phrase",
            before_artifact_ref_id: exact,
            after_artifact_ref_id: after,
            scope: "company voice",
            contradiction_check: "No human outcome evidence was found.",
        })
        .await
        .unwrap_err()
        .to_string()
        .contains("cannot propose constitution policy"));

    let plain_work = first
        .org
        .add_work(NewWork {
            owner_id: "identity-maker",
            title: "internal inventory",
            outcome: "List exact affected records",
            goal_id: None,
            priority: 1,
            expected_artifact: "inventory",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let plain = first
        .org
        .compile_constitution(plain_work, 32 * 1024)
        .await
        .unwrap();
    assert_eq!(
        plain
            .pillars
            .iter()
            .filter(|pillar| pillar.status.starts_with("unavailable"))
            .count(),
        3
    );
    assert!(plain
        .body
        .contains("do not generate a substitute house style"));
    assert!(plain
        .body
        .contains("do not generate generic visual defaults"));
    assert!(plain.body.contains("do not infer personality"));

    let second = fixture(
        &url,
        "constitutionb",
        "Harbour Ledger gives marine crews an offline maintenance record.",
        "Write like a practical vessel log: calm, terse and specific.",
        "Use spacious paper-like records, marine blue ink and no ornamental motion.",
        "Stop and document uncertainty before a safety-critical maintenance decision.",
    )
    .await;
    let second_brief = second
        .org
        .compile_constitution(second.work, 32 * 1024)
        .await
        .unwrap();
    assert_ne!(brief.digest, second_brief.digest);
    assert!(second_brief.body.contains("Harbour Ledger"));
    assert!(!second_brief.body.contains("Restless runs"));
    assert!(!second_brief.body.contains("accountable company work"));

    first.org.drop_schema().await.unwrap();
    second.org.drop_schema().await.unwrap();
}
