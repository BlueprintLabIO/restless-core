//! S32 walking slice: one released identity compiles six explicit human
//! situations, exact rendered artifacts receive blinded copy review, and a
//! meaningful owner edit becomes only a scoped proposal.

use chrono::Utc;
use restless_orgintel::{
    IdentityEvidenceStatus, IdentityPillar, IdentityPolarity, IdentityStatementKind,
    NewArtifactRef, NewIdentityEvidence, NewVoiceEvidence, NewVoiceLearningProposal,
    NewVoiceRenderEvidence, NewVoiceReview, NewVoiceWorkContract, NewWork, OrgIntel, VoiceChannel,
    VoiceEvidenceKind, VoiceLearningKind, VoiceReviewVerdict, WorkspaceSpec,
};
use std::collections::BTreeSet;

async fn work(org: &OrgIntel, title: &str) -> uuid::Uuid {
    org.add_work(NewWork {
        owner_id: "brand-writer",
        title,
        outcome: title,
        goal_id: None,
        priority: 1,
        expected_artifact: "rendered copy artifact",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(1),
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn one_company_voice_stays_human_and_distinct_across_channels() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping company voice scenario");
        return;
    };
    let company = format!("companyvoice{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("brand-writer", "staff", "writer", "Writer")
        .await
        .unwrap();
    let now = Utc::now();
    let truth = org
        .add_identity_evidence(NewIdentityEvidence {
            pillar: IdentityPillar::Truth,
            statement_kind: IdentityStatementKind::Fact,
            claim_key: "product.promise",
            statement:
                "Restless runs company work and returns evidence-backed decisions to the owner.",
            author_id: "owner",
            source: "founder product decision",
            authority: "owner",
            scope: "company",
            observed_at: now,
            evidence_locator: "docs/identity/product-promise.md",
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
    let principle = org
        .add_voice_evidence(NewVoiceEvidence {
            kind: VoiceEvidenceKind::ExpressionPrinciple,
            claim_key: "voice.value_first",
            passage_or_principle: "Lead with the concrete value in plain human language.",
            author_id: "owner",
            named_author: None,
            source: "accepted owner review",
            authority: "owner",
            scope: "company",
            observed_at: now,
            evidence_locator: "docs/dogfood/voice-review.md",
            judgement_reason: "Readers understood the outcome without translating product jargon.",
            polarity: IdentityPolarity::Positive,
            channel: None,
            audience: None,
            supersedes_evidence_id: None,
        })
        .await
        .unwrap();
    let plain = org
        .add_voice_evidence(NewVoiceEvidence {
            kind: VoiceEvidenceKind::ApprovedPassage,
            claim_key: "voice.plain_control",
            passage_or_principle: "You give Restless an outcome. It runs the work and brings the decision back to you.",
            author_id: "owner",
            named_author: None,
            source: "plain control selected by owner",
            authority: "owner",
            scope: "company",
            observed_at: now,
            evidence_locator: "docs/dogfood/plain-control.md",
            judgement_reason: "The plain candidate retained the full meaning with less abstraction.",
            polarity: IdentityPolarity::Positive,
            channel: None,
            audience: None,
            supersedes_evidence_id: None,
        })
        .await
        .unwrap();
    let rejected = org
        .add_voice_evidence(NewVoiceEvidence {
            kind: VoiceEvidenceKind::RejectedPassage,
            claim_key: "voice.abstract_negative",
            passage_or_principle: "Operational intelligence compounds through an autonomous fabric of accountable outcomes.",
            author_id: "owner",
            named_author: None,
            source: "blinded copy desk",
            authority: "owner-reviewed evidence",
            scope: "company",
            observed_at: now,
            evidence_locator: "docs/dogfood/abstract-negative.md",
            judgement_reason: "Abstract noun chain concealed the person, action and proof.",
            polarity: IdentityPolarity::Negative,
            channel: None,
            audience: None,
            supersedes_evidence_id: None,
        })
        .await
        .unwrap();
    let founder = org
        .add_voice_evidence(NewVoiceEvidence {
            kind: VoiceEvidenceKind::NamedAuthor,
            claim_key: "voice.founder",
            passage_or_principle:
                "Use first person only for decisions and observations the founder can own.",
            author_id: "owner",
            named_author: Some("founder"),
            source: "signed founder note",
            authority: "founder",
            scope: "company",
            observed_at: now,
            evidence_locator: "docs/identity/founder-note.md",
            judgement_reason:
                "The founder could sign this without anonymous institutional omniscience.",
            polarity: IdentityPolarity::Positive,
            channel: None,
            audience: None,
            supersedes_evidence_id: None,
        })
        .await
        .unwrap();
    let blog = org
        .add_voice_evidence(NewVoiceEvidence {
            kind: VoiceEvidenceKind::ChannelObservation,
            claim_key: "voice.blog_standalone",
            passage_or_principle: "A Blog supplies its own context and may include raw observations before reasoning.",
            author_id: "owner",
            named_author: None,
            source: "accepted Blog review",
            authority: "owner",
            scope: "company",
            observed_at: now,
            evidence_locator: "docs/dogfood/blog-review.md",
            judgement_reason: "Readers could understand the argument without opening internal files.",
            polarity: IdentityPolarity::Positive,
            channel: Some(VoiceChannel::Blog),
            audience: Some("owners"),
            supersedes_evidence_id: None,
        })
        .await
        .unwrap();
    let proposal = org
        .propose_identity_release(
            "brand-writer",
            "first typed human voice",
            &[truth, principle, plain, rejected, founder, blog],
        )
        .await
        .unwrap();
    let release = org
        .promote_identity_proposal(
            proposal,
            "owner",
            "authority:voice:release-1",
            "Established human voice evidence and channel-specific observations.",
            now,
        )
        .await
        .unwrap();

    let channels = [
        VoiceChannel::Newsletter,
        VoiceChannel::FounderEmail,
        VoiceChannel::Support,
        VoiceChannel::TransactionalEmail,
        VoiceChannel::ProductUi,
        VoiceChannel::Blog,
    ];
    let mut digests = BTreeSet::new();
    for channel in channels {
        let title = format!("{channel:?} copy");
        let work_id = work(&org, &title).await;
        let author = if matches!(channel, VoiceChannel::Support) {
            "support lead"
        } else {
            "founder"
        };
        let contract = org
            .bind_voice_work_contract(NewVoiceWorkContract {
                work_id,
                channel,
                author,
                bound_by: "brand-editor",
                audience: "owners",
                reader_situation:
                    "An owner needs to understand what Restless changes in their work.",
                desired_understanding:
                    "Restless runs accountable work and returns evidence-backed decisions.",
                desired_action: "Choose whether to try one bounded outcome.",
                proof: "The released product promise and an inspectable completed outcome.",
                consequence: "The owner spends less attention coordinating incomplete work.",
            })
            .await
            .unwrap();
        assert_eq!(contract.release_id, release);
        let brief = org.compile_voice_contract(work_id, 4_096).await.unwrap();
        assert_eq!(
            brief,
            org.compile_voice_contract(work_id, 4_096).await.unwrap()
        );
        assert!(brief.bytes <= 4_096);
        assert!(brief.body.contains("plain human language"));
        assert!(brief.body.contains("plain candidate"));
        assert!(brief.body.contains("Negative examples are evidence"));
        if channel == VoiceChannel::Blog {
            assert!(brief.body.contains("standalone context"));
        }
        if author == "support lead" {
            assert!(!brief.body.contains("Use first person only"));
        }
        digests.insert(brief.digest);
    }
    assert_eq!(digests.len(), 6, "the six channel contracts must differ");

    let before = org
        .link_work_artifact(NewArtifactRef {
            kind: "rendered_copy",
            uri: "/company/reviews/founder-email-before.html",
            note: "desktop and narrow email rendering",
            created_by: "brand-writer",
            work_id: None,
            attempt_id: None,
            digest: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            source_commit: None,
            runtime_generation: Some("email-renderer-v1"),
            label: "Founder email control",
        })
        .await
        .unwrap();
    let render = org
        .record_voice_render_evidence(NewVoiceRenderEvidence {
            artifact_ref_id: before,
            channel: VoiceChannel::FounderEmail,
            renderer: "email-native-test",
            renderer_version: "1.0.0",
            semantic_checks: &serde_json::json!({
                "text_fallback": true,
                "desktop_wrap": true,
                "narrow_wrap": true,
                "subject_visible": true,
                "preheader_visible": true,
                "links_operable": true
            }),
            captured_by: "copy-desk",
        })
        .await
        .unwrap();
    assert!(org
        .record_voice_review(NewVoiceReview {
            render_evidence_id: render,
            reviewer: "brand-writer",
            verdict: VoiceReviewVerdict::Accept,
            factual_findings: "",
            abstraction_findings: "",
            repetition_findings: "",
            channel_findings: "",
            authorship_findings: "",
            concepts_removed: "",
        })
        .await
        .unwrap_err()
        .to_string()
        .contains("fresh reviewer"));
    assert!(org
        .record_voice_review(NewVoiceReview {
            render_evidence_id: render,
            reviewer: "copy-desk",
            verdict: VoiceReviewVerdict::Revise,
            factual_findings: "",
            abstraction_findings: "",
            repetition_findings: "",
            channel_findings: "",
            authorship_findings: "",
            concepts_removed: "",
        })
        .await
        .unwrap_err()
        .to_string()
        .contains("concrete finding"));
    org.record_voice_review(NewVoiceReview {
        render_evidence_id: render,
        reviewer: "copy-desk",
        verdict: VoiceReviewVerdict::Revise,
        factual_findings: "",
        abstraction_findings: "Opening hides the decision behind an abstract noun chain.",
        repetition_findings: "",
        channel_findings: "",
        authorship_findings: "The founder would not plausibly sign the institutional opening.",
        concepts_removed: "autonomous fabric",
    })
    .await
    .unwrap();

    let after = org
        .link_work_artifact(NewArtifactRef {
            kind: "rendered_copy",
            uri: "/company/reviews/founder-email-after.html",
            note: "owner-edited native rendering",
            created_by: "owner",
            work_id: None,
            attempt_id: None,
            digest: Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            source_commit: None,
            runtime_generation: Some("email-renderer-v1"),
            label: "Founder email owner edit",
        })
        .await
        .unwrap();
    assert!(org
        .propose_voice_learning(NewVoiceLearningProposal {
            created_by: "brand-writer",
            before_artifact_ref_id: before,
            after_artifact_ref_id: after,
            change_kind: VoiceLearningKind::Typo,
            claim_key: "voice.typo",
            observation: "Corrected a typo.",
            motivating_decision: "Typo only.",
            scope: "company",
            source: "owner edit",
            evidence_locator: "/company/reviews/founder-email-after.html",
            channel: Some(VoiceChannel::FounderEmail),
            named_author: Some("founder"),
            audience: Some("owners"),
            observed_at: now,
        })
        .await
        .unwrap()
        .is_none());
    let learning = org
        .propose_voice_learning(NewVoiceLearningProposal {
            created_by: "brand-writer",
            before_artifact_ref_id: before,
            after_artifact_ref_id: after,
            change_kind: VoiceLearningKind::VoiceObservation,
            claim_key: "voice.founder_decision_first",
            observation: "Founder emails state the decision before the institutional context.",
            motivating_decision:
                "The owner moved the decision into the first sentence and signed it.",
            scope: "company",
            source: "owner edit after blinded copy review",
            evidence_locator: "/company/reviews/founder-email-after.html",
            channel: Some(VoiceChannel::FounderEmail),
            named_author: Some("founder"),
            audience: Some("owners"),
            observed_at: now,
        })
        .await
        .unwrap()
        .unwrap();
    let snapshot = org.company_identity_snapshot().await.unwrap();
    assert_eq!(snapshot.current_release.unwrap().id, release);
    assert!(snapshot
        .pending_proposals
        .iter()
        .any(|row| row.id == learning));
    org.drop_schema().await.unwrap();
}
