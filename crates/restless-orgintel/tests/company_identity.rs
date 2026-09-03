//! S31 walking slice: source-owned evidence becomes effective only through an
//! owner promotion, survives restart, binds outcomes immutably, compiles into
//! bounded context, and makes corrected historical bindings visibly stale.

use chrono::{Duration, Utc};
use restless_orgintel::{
    IdentityBriefRequest, IdentityEvidenceStatus, IdentityPillar, IdentityPolarity,
    IdentityStatementKind, NewIdentityEvidence, NewWork, OrgIntel, WorkspaceSpec,
};

async fn work(org: &OrgIntel, title: &str) -> uuid::Uuid {
    org.add_work(NewWork {
        owner_id: "identity-writer",
        title,
        outcome: title,
        goal_id: None,
        priority: 1,
        expected_artifact: "identity-bound artifact",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(1),
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn released_identity_is_owner_governed_bounded_and_restart_stable() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping company identity scenario");
        return;
    };
    let company = format!("companyidentity{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("identity-writer", "staff", "writer", "Identity writer")
        .await
        .unwrap();
    assert!(org
        .company_identity_snapshot()
        .await
        .unwrap()
        .current_release
        .is_none());

    let now = Utc::now();
    let product_fact = org
        .add_identity_evidence(NewIdentityEvidence {
            pillar: IdentityPillar::Truth,
            statement_kind: IdentityStatementKind::Fact,
            claim_key: "product.promise",
            statement: "Restless runs company work and returns decisions to the owner.",
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
    let product_belief = org
        .add_identity_evidence(NewIdentityEvidence {
            pillar: IdentityPillar::Truth,
            statement_kind: IdentityStatementKind::Belief,
            claim_key: "market.owner_attention",
            statement: "Owners value fewer interventions more than more agent activity.",
            author_id: "identity-writer",
            source: "dogfood observation",
            authority: "attributed staff belief",
            scope: "company",
            observed_at: now,
            evidence_locator: "docs/dogfood/owner-attention.md",
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
        .add_identity_evidence(NewIdentityEvidence {
            pillar: IdentityPillar::Voice,
            statement_kind: IdentityStatementKind::Guidance,
            claim_key: "voice.value_first",
            statement: "Lead with the concrete value in plain human language.",
            author_id: "owner",
            source: "accepted landing-page review",
            authority: "owner",
            scope: "company",
            observed_at: now,
            evidence_locator: "docs/dogfood/landing-page-review.md",
            polarity: IdentityPolarity::Positive,
            status: IdentityEvidenceStatus::Active,
            channel: Some("website"),
            audience: None,
            supersedes_evidence_id: None,
            exception_expires_at: None,
            exception_indefinite: false,
        })
        .await
        .unwrap();
    let exception = org
        .add_identity_evidence(NewIdentityEvidence {
            pillar: IdentityPillar::Voice,
            statement_kind: IdentityStatementKind::Exception,
            claim_key: "voice.technical_precision",
            statement: "The migration note may use exact database terminology.",
            author_id: "owner",
            source: "bounded owner exception",
            authority: "owner",
            scope: "outcome:migration note",
            observed_at: now,
            evidence_locator: "docs/decisions/migration-language.md",
            polarity: IdentityPolarity::Neutral,
            status: IdentityEvidenceStatus::Active,
            channel: Some("blog"),
            audience: Some("engineers"),
            supersedes_evidence_id: None,
            exception_expires_at: Some(now + Duration::hours(1)),
            exception_indefinite: false,
        })
        .await
        .unwrap();
    let conflict = org
        .add_identity_evidence(NewIdentityEvidence {
            pillar: IdentityPillar::Truth,
            statement_kind: IdentityStatementKind::Fact,
            claim_key: "product.promise",
            statement: "Restless is a passive reporting dashboard.",
            author_id: "identity-writer",
            source: "rejected draft",
            authority: "attributed staff claim",
            scope: "company",
            observed_at: now,
            evidence_locator: "docs/rejected/passive-dashboard.md",
            polarity: IdentityPolarity::Negative,
            status: IdentityEvidenceStatus::Active,
            channel: None,
            audience: None,
            supersedes_evidence_id: None,
            exception_expires_at: None,
            exception_indefinite: false,
        })
        .await
        .unwrap();

    let conflicting = org
        .propose_identity_release(
            "identity-writer",
            "surface the contradiction rather than choosing fluent copy",
            &[product_fact, conflict],
        )
        .await
        .unwrap();
    let conflict_error = org
        .promote_identity_proposal(
            conflicting,
            "owner",
            "authority:identity:conflict",
            "attempted first release",
            now,
        )
        .await
        .unwrap_err();
    assert!(conflict_error.to_string().contains("product.promise"));

    let proposal = org
        .propose_identity_release(
            "identity-writer",
            "first evidence-backed release",
            &[product_fact, product_belief, voice, exception],
        )
        .await
        .unwrap();
    assert!(org
        .promote_identity_proposal(
            proposal,
            "exec",
            "authority:identity:not-owner",
            "must fail",
            now,
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("authenticated owner"));
    let release = org
        .promote_identity_proposal(
            proposal,
            "owner",
            "authority:identity:release-1",
            "Established product truth and initial website voice.",
            now,
        )
        .await
        .unwrap();

    let first_work = work(&org, "website home page").await;
    let first_binding = org.bind_work_identity(first_work).await.unwrap();
    assert_eq!(first_binding.release_id, release);
    let brief_request = || IdentityBriefRequest {
        release_id: release,
        outcome: "website home page",
        channel: "website",
        audience: "owners",
        author: "identity-writer",
        max_bytes: 1_600,
        now,
    };
    let first_brief = org.compile_identity_brief(brief_request()).await.unwrap();
    let repeated_brief = org.compile_identity_brief(brief_request()).await.unwrap();
    assert_eq!(first_brief.digest, repeated_brief.digest);
    assert_eq!(first_brief.body, repeated_brief.body);
    assert!(first_brief.bytes <= 1_600);
    assert!(first_brief.body.contains("runs company work"));
    assert!(first_brief.body.contains("Lead with the concrete value"));
    assert!(!first_brief.body.contains("database terminology"));

    drop(org);
    let restarted = OrgIntel::ensure(&url, &company).await.unwrap();
    let second_work = work(&restarted, "product overview").await;
    assert_eq!(
        restarted
            .bind_work_identity(second_work)
            .await
            .unwrap()
            .release_id,
        release
    );
    assert_eq!(
        restarted
            .bind_work_identity(first_work)
            .await
            .unwrap()
            .release_id,
        release
    );

    let corrected_fact = restarted
        .add_identity_evidence(NewIdentityEvidence {
            pillar: IdentityPillar::Truth,
            statement_kind: IdentityStatementKind::Fact,
            claim_key: "product.promise",
            statement: "Restless runs accountable company work and returns evidence-backed decisions to the owner.",
            author_id: "owner",
            source: "founder correction",
            authority: "owner",
            scope: "company",
            observed_at: now + Duration::minutes(10),
            evidence_locator: "docs/identity/product-promise-v2.md",
            polarity: IdentityPolarity::Neutral,
            status: IdentityEvidenceStatus::Active,
            channel: None,
            audience: None,
            supersedes_evidence_id: Some(product_fact),
            exception_expires_at: None,
            exception_indefinite: false,
        })
        .await
        .unwrap();
    let correction = restarted
        .propose_identity_release(
            "identity-writer",
            "correct product truth without rewriting historical outcomes",
            &[corrected_fact, product_belief, voice],
        )
        .await
        .unwrap();
    let release_two = restarted
        .promote_identity_proposal(
            correction,
            "owner",
            "authority:identity:release-2",
            "Corrected product promise and retired the bounded exception.",
            now + Duration::minutes(10),
        )
        .await
        .unwrap();
    assert_ne!(release_two, release);
    let snapshot = restarted.company_identity_snapshot().await.unwrap();
    assert_eq!(snapshot.current_release.unwrap().id, release_two);
    assert_eq!(snapshot.releases.len(), 2);
    assert!(snapshot
        .bindings
        .iter()
        .filter(|binding| binding.release_id == release)
        .all(|binding| binding.stale_at.is_some()));
    assert_eq!(
        restarted
            .bind_work_identity(first_work)
            .await
            .unwrap()
            .release_id,
        release,
        "historical Work keeps its exact release"
    );
    restarted.drop_schema().await.unwrap();
}
