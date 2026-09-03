//! S33 walking slice: one released visual language produces four distinct
//! native directions, retrieves capabilities without quotas, binds exact
//! captures and lets a restrained control beat decorative spectacle.

use chrono::Utc;
use restless_orgintel::*;
use std::collections::BTreeSet;

async fn add_work(org: &OrgIntel, channel: VisualChannel) -> uuid::Uuid {
    org.add_work(NewWork {
        owner_id: "design-engineer",
        title: &format!("{channel:?} visual"),
        outcome: "Create one product-grounded native visual",
        goal_id: None,
        priority: 1,
        expected_artifact: "native rendered visual",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(1),
    })
    .await
    .unwrap()
}

fn artifact<'a>(label: &'a str, digest: &'a str) -> NewArtifactRef<'a> {
    NewArtifactRef {
        kind: "native_visual",
        uri: label,
        note: "desktop, narrow and reduced-motion capture",
        created_by: "design-engineer",
        work_id: None,
        attempt_id: None,
        digest: Some(digest),
        source_commit: None,
        runtime_generation: Some("browser-native-v1"),
        label,
    }
}

fn base_evidence<'a>(
    kind: VisualEvidenceKind,
    key: &'a str,
    statement: &'a str,
    dependencies: &'a serde_json::Value,
) -> NewVisualEvidence<'a> {
    NewVisualEvidence {
        kind,
        claim_key: key,
        statement,
        author_id: "owner",
        source: "accepted product and brand review",
        authority: "owner",
        scope: "company",
        observed_at: Utc::now(),
        evidence_locator: "product@sha256:ground-truth",
        rationale: "The current product and strongest accepted public work share this decision.",
        purpose: "Make product truth recognisable across channels.",
        polarity: IdentityPolarity::Positive,
        channel: None,
        semantic_role: None,
        value: None,
        reduced_motion_replacement: None,
        product_truth_locator: None,
        origin: None,
        licence: None,
        framework: None,
        dependencies,
        adaptation_status: None,
        accessibility_notes: "Preserve contrast, focus, text fit and information hierarchy.",
        supersedes_evidence_id: None,
    }
}

#[tokio::test]
async fn one_visual_language_stays_native_distinct_and_product_grounded() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping company visual scenario");
        return;
    };
    let company = format!("companyvisual{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    for (id, kind, role, name) in [
        ("owner", "owner", "owner", "Owner"),
        ("design-engineer", "staff", "designer", "Design engineer"),
        ("art-director", "staff", "reviewer", "Art director"),
    ] {
        org.ensure_actor(id, kind, role, name).await.unwrap();
    }
    let now = Utc::now();
    let no_dependencies = serde_json::json!([]);
    let truth=org.add_identity_evidence(NewIdentityEvidence { pillar:IdentityPillar::Truth, statement_kind:IdentityStatementKind::Fact, claim_key:"product.current", statement:"Restless shows accountable Work, evidence and owner decisions in a calm operational cockpit.", author_id:"owner", source:"current product", authority:"owner", scope:"company", observed_at:now, evidence_locator:"product@sha256:ground-truth", polarity:IdentityPolarity::Neutral, status:IdentityEvidenceStatus::Active, channel:None, audience:None, supersedes_evidence_id:None, exception_expires_at:None, exception_indefinite:false }).await.unwrap();

    let mut token = base_evidence(
        VisualEvidenceKind::SemanticToken,
        "visual.accent",
        "Use the product's mint accent only for state, agency and decisive action.",
        &no_dependencies,
    );
    token.semantic_role = Some("action.accent");
    token.value = Some("product-token:--accent");
    let token = org.add_visual_evidence(token).await.unwrap();
    let composition = org
        .add_visual_evidence(base_evidence(
            VisualEvidenceKind::CompositionPrinciple,
            "visual.hierarchy",
            "Lead with one human outcome, then show exact proof with generous but not empty space.",
            &no_dependencies,
        ))
        .await
        .unwrap();
    let mut motion = base_evidence(
        VisualEvidenceKind::MotionPattern,
        "visual.motion",
        "Motion reveals causal progress between intent, work, evidence and decision.",
        &no_dependencies,
    );
    motion.reduced_motion_replacement =
        Some("Show the complete causal path at rest with the current state strongly marked.");
    let motion = org.add_visual_evidence(motion).await.unwrap();
    let mut product=base_evidence(VisualEvidenceKind::ProductRepresentationRule,"visual.product_truth","Use exact current product UI for evidence; otherwise make the composition visibly abstract.", &no_dependencies);
    product.product_truth_locator = Some("product@sha256:ground-truth");
    let product = org.add_visual_evidence(product).await.unwrap();
    let mut primitive = base_evidence(
        VisualEvidenceKind::Primitive,
        "visual.primitive.liquid",
        "Liquid material field for a meaningful hero state transition.",
        &no_dependencies,
    );
    primitive.origin = Some("Cult UI hero liquid metal");
    primitive.licence = Some("MIT");
    primitive.framework = Some("React island in Astro");
    primitive.adaptation_status = Some("adapted_and_verified");
    let primitive_dependencies = serde_json::json!(["react", "motion"]);
    primitive.dependencies = &primitive_dependencies;
    let primitive = org.add_visual_evidence(primitive).await.unwrap();
    let rejected = base_evidence(
        VisualEvidenceKind::RejectedExample,
        "visual.goofy",
        "Do not use blurry pseudo-product blobs or tiny diagram labels as proof.",
        &no_dependencies,
    );
    let rejected = org.add_visual_evidence(rejected).await.unwrap_err();
    assert!(rejected.to_string().contains("rejected visual examples"));
    let mut rejected = base_evidence(
        VisualEvidenceKind::RejectedExample,
        "visual.goofy",
        "Do not use blurry pseudo-product blobs or tiny diagram labels as proof.",
        &no_dependencies,
    );
    rejected.polarity = IdentityPolarity::Negative;
    let rejected = org.add_visual_evidence(rejected).await.unwrap();

    let proposal = org
        .propose_identity_release(
            "design-engineer",
            "Release product-grounded visual grammar.",
            &[
                truth,
                token,
                composition,
                motion,
                product,
                primitive,
                rejected,
            ],
        )
        .await
        .unwrap();
    let release=org.promote_identity_proposal(proposal,"owner","authority:visual:release-1","Established one visual grammar with product truth, native motion and negative evidence.",now).await.unwrap();
    let mut digests = BTreeSet::new();
    let mut works = Vec::new();
    for channel in [
        VisualChannel::LandingPage,
        VisualChannel::Email,
        VisualChannel::Product,
        VisualChannel::Social,
    ] {
        let work_id = add_work(&org, channel).await;
        let representation = if channel == VisualChannel::Product {
            VisualRepresentation::ExactProduct
        } else {
            VisualRepresentation::ClearlyAbstract
        };
        let row = org
            .bind_visual_work_contract(NewVisualWorkContract {
                work_id,
                channel,
                bound_by: "design-engineer",
                audience: "company owners",
                outcome: "Understand how Restless returns accountable decisions",
                information_hierarchy:
                    "Outcome → exact or clearly abstract mechanism → proof → action",
                proof: "Current product evidence and one completed outcome",
                density: match channel {
                    VisualChannel::Social => "one idea, legible without post",
                    VisualChannel::Email => "compact static header",
                    VisualChannel::Product => "operational density",
                    VisualChannel::LandingPage => "scannable scroll sequence",
                },
                imagery_role: "Explain the operating loop; never decorate empty space",
                motion_role: if channel == VisualChannel::Email {
                    "none; coherent static state"
                } else {
                    "reveal causal state change only"
                },
                product_representation: representation,
                product_truth_locator: (representation == VisualRepresentation::ExactProduct)
                    .then_some("product@sha256:ground-truth"),
                requested_departure: (channel == VisualChannel::Social)
                    .then_some("warmer announcement colour"),
            })
            .await
            .unwrap();
        assert_eq!(row.release_id, release);
        let brief = org.compile_visual_direction(work_id, 8192).await.unwrap();
        assert_eq!(
            brief,
            org.compile_visual_direction(work_id, 8192).await.unwrap()
        );
        assert!(brief.body.contains("never quotas"));
        assert!(brief.body.contains("restrained control"));
        assert!(brief.body.contains("product@sha256:ground-truth"));
        digests.insert(brief.digest);
        works.push((channel, work_id));
    }
    assert_eq!(
        digests.len(),
        4,
        "channel direction must not clone one template"
    );
    let landing = works[0].1;
    org.record_visual_primitive_use(NewVisualPrimitiveUse {
        work_id: landing,
        evidence_id: primitive,
        primitive_version: "cult-ui@2026-08-31",
        purpose: "Show evidence returning to a decision",
    })
    .await
    .unwrap();
    assert!(org
        .record_visual_primitive_use(NewVisualPrimitiveUse {
            work_id: landing,
            evidence_id: composition,
            primitive_version: "n/a",
            purpose: "inflate component count"
        })
        .await
        .unwrap_err()
        .to_string()
        .contains("unsupported"));

    let effect = org
        .link_work_artifact(artifact(
            "/reviews/effect-rich.html",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ))
        .await
        .unwrap();
    let control = org
        .link_work_artifact(artifact(
            "/reviews/restrained.html",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ))
        .await
        .unwrap();
    let checks = serde_json::json!({"responsive_containment":true,"keyboard_path":true,"contrast":true,"text_fit":true,"proof_legible":true,"reduced_motion_complete":true});
    let effect_render = org
        .record_visual_render_evidence(NewVisualRenderEvidence {
            work_id: landing,
            artifact_ref_id: effect,
            channel: VisualChannel::LandingPage,
            renderer: "native browser",
            renderer_version: "1",
            viewport_width: 1440,
            viewport_height: 1000,
            motion_state: VisualMotionState::Reduced,
            native_checks: &checks,
            captured_by: "art-director",
        })
        .await
        .unwrap();
    let control_render = org
        .record_visual_render_evidence(NewVisualRenderEvidence {
            work_id: landing,
            artifact_ref_id: control,
            channel: VisualChannel::LandingPage,
            renderer: "native browser",
            renderer_version: "1",
            viewport_width: 1440,
            viewport_height: 1000,
            motion_state: VisualMotionState::Reduced,
            native_checks: &checks,
            captured_by: "art-director",
        })
        .await
        .unwrap();
    assert!(org
        .record_visual_review(NewVisualReview {
            render_evidence_id: effect_render,
            control_render_evidence_id: Some(control_render),
            reviewer: "design-engineer",
            verdict: VisualReviewVerdict::Accept,
            identity_findings: "",
            hierarchy_findings: "",
            density_findings: "",
            proof_findings: "",
            product_fidelity_findings: "",
            motion_findings: "",
            defect_findings: "",
            departure_decision: ""
        })
        .await
        .unwrap_err()
        .to_string()
        .contains("independent art director"));
    org.record_visual_review(NewVisualReview {
        render_evidence_id: effect_render,
        control_render_evidence_id: Some(control_render),
        reviewer: "art-director",
        verdict: VisualReviewVerdict::Reject,
        identity_findings: "Both read as Restless through product typography and state colour.",
        hierarchy_findings: "The effect competes with the proof; the restrained control wins.",
        density_findings: "",
        proof_findings: "Control keeps completed Work legible in the first view.",
        product_fidelity_findings: "The abstract mechanism is visibly abstract.",
        motion_findings: "Reduced state is complete, but spectacle adds no meaning.",
        defect_findings: "",
        departure_decision: "Scope the warmer colour to social only.",
    })
    .await
    .unwrap();
    let snapshot = org.company_identity_snapshot().await.unwrap();
    assert_eq!(snapshot.visual_work_contracts.len(), 4);
    assert_eq!(snapshot.visual_primitive_uses.len(), 1);
    assert_eq!(snapshot.visual_reviews.len(), 1);
    org.drop_schema().await.unwrap();
}
