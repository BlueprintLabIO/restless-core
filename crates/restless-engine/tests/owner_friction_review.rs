//! The company reviews itself on its owner's friction (20 November 2026,
//! docs/specs/owner-interaction.md D7): status checks above 8% of the owner's messages make the
//! review due even with no Work finished, recording the review settles it while the same week's
//! numbers stay high, and Work sent back after it was done makes it due again. Needs
//! `RESTLESS_TEST_DATABASE_URL`; skipped otherwise.

use chrono::Utc;
use restless_engine::org_review::{exec_block, scorecard, stored};
use restless_orgintel::{NewArtifactRef, NewWork, OrgIntel, WorkAttemptState, WorkspaceSpec};

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "frictionreview{}_test",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    );
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    for (id, kind, role) in [
        ("owner", "owner", "owner"),
        ("exec", "exec", "exec"),
        ("alpha-build", "staff", "builder"),
    ] {
        org.ensure_actor(id, kind, role, id).await.unwrap();
    }
    Some(org)
}

async fn owner_says(org: &OrgIntel, body: &str) {
    let id = org.send_message("owner", Some("exec"), body).await.unwrap();
    org.mark_read(id).await.unwrap();
}

#[tokio::test]
async fn owner_friction_makes_the_review_due_once_per_new_signal() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping owner friction review");
        return;
    };
    // Nothing said: nothing due, and an ordinary wake carries no review.
    let card = scorecard(&org, Utc::now()).await.unwrap();
    assert!(!card.due);
    assert!(exec_block(&card).is_none());

    // Eight plain requests and two "progress?": 20% status checks, no Work finished at all.
    for n in 0..8 {
        owner_says(&org, &format!("draft launch email number {n}")).await;
    }
    owner_says(&org, "progress?").await;
    owner_says(&org, "status").await;
    let card = scorecard(&org, Utc::now()).await.unwrap();
    assert_eq!(card.friction.owner_messages, 10);
    assert_eq!(card.friction.status_checks, 2);
    assert!(card.due, "{:?}", card.friction_due);
    assert_eq!(card.friction_due.len(), 1, "{:?}", card.friction_due);
    assert!(
        card.friction_due[0].contains("20%"),
        "{:?}",
        card.friction_due
    );
    let block = exec_block(&card).expect("a due review is shown to Exec");
    assert!(
        block.contains("status checks 2 (20%, baseline 8%)"),
        "{block}"
    );
    assert!(block.contains("review due: status checks"), "{block}");

    // Exec reviews. The week still holds 20%, but nothing is new: not due.
    org.record_org_review(
        "exec",
        card.signals_through,
        2,
        stored(&card),
        "Status asked twice: Exec now posts when Work lands.",
    )
    .await
    .unwrap();
    let card = scorecard(&org, Utc::now()).await.unwrap();
    assert_eq!(card.friction.status_checks, 2);
    assert!(!card.due, "{:?}", card.friction_due);
    assert!(card.last_review.as_ref().unwrap().friction.is_some());

    // Work lands and the owner sends it back: due again, on that alone.
    let work = org
        .add_work(NewWork {
            owner_id: "alpha-build",
            title: "Draft the launch email",
            outcome: "an email",
            goal_id: None,
            priority: 1,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let attempt = org.claim_ready_work("test").await.unwrap().unwrap();
    org.link_work_artifact(NewArtifactRef {
        kind: "path",
        uri: "/company/email.md",
        note: "the email",
        created_by: "alpha-build",
        work_id: Some(work),
        attempt_id: Some(attempt.attempt_id),
        digest: Some("sha256:email"),
        source_commit: None,
        runtime_generation: Some("test"),
        label: "email",
    })
    .await
    .unwrap();
    org.finish_work_attempt(attempt.attempt_id, WorkAttemptState::Produced, "drafted")
        .await
        .unwrap();
    let card = scorecard(&org, Utc::now()).await.unwrap();
    assert_eq!(card.friction.work_landed, 1);
    assert!(!card.due, "landing is not friction");
    org.reopen_work(work, "owner", "lead with the guarantee")
        .await
        .unwrap();
    let card = scorecard(&org, Utc::now()).await.unwrap();
    assert!(card.due);
    assert!(
        card.friction_due
            .iter()
            .any(|reason| reason.contains("sent back after it was done")),
        "{:?}",
        card.friction_due
    );
    org.drop_schema().await.unwrap();
}
