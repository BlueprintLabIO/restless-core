//! The company's own records of how its shape is working: each Attempt carries the team it ran
//! under, the scorecard reads them, a review is recorded by Exec or the owner only, and a
//! producer gets one open critic at a time.

use restless_orgintel::{NewWork, OrgIntel, WorkAttemptState, WorkEdgeKind, WorkspaceSpec};

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "orgrev{}_test",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    );
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    for (id, kind, role) in [
        ("owner", "owner", "owner"),
        ("exec", "exec", "exec"),
        ("docs-direction", "staff", "direction"),
        ("docs-writer", "staff", "writer"),
        ("policy-checker", "staff", "checker"),
        ("legal-checker", "staff", "checker"),
    ] {
        org.ensure_actor(id, kind, role, id).await.unwrap();
    }
    Some(org)
}

fn work<'a>(owner_id: &'a str, title: &'a str) -> NewWork<'a> {
    NewWork {
        owner_id,
        title,
        outcome: title,
        goal_id: None,
        priority: 0,
        expected_artifact: "",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(1),
    }
}

#[tokio::test]
async fn a_producer_has_one_open_critic_at_a_time() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping org review scenario");
        return;
    };
    let draft = org
        .add_work(work("docs-writer", "Draft the delivery policy"))
        .await
        .unwrap();
    let check = org
        .add_work_with_edges(
            work("policy-checker", "Check the draft"),
            &[draft],
            &[draft],
        )
        .await
        .unwrap();

    // A second critic declared at creation is refused, with the way on, and nothing is created.
    let refused = org
        .add_work_with_edges(
            work("legal-checker", "Check the draft again"),
            &[draft],
            &[draft],
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(refused.contains("already has an open critic"), "{refused}");
    assert!(refused.contains(&check.to_string()), "{refused}");
    assert_eq!(org.list_work().await.unwrap().len(), 2);

    // Added after creation: the same rule.
    let reader = org
        .add_work_with_edges(work("legal-checker", "Read the draft"), &[draft], &[])
        .await
        .unwrap();
    let refused = org
        .add_work_edge(reader, draft, WorkEdgeKind::Revises)
        .await
        .unwrap_err()
        .to_string();
    assert!(refused.contains("already has an open critic"), "{refused}");
}

#[tokio::test]
async fn attempts_carry_their_team_and_the_scorecard_reads_them() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping org review scenario");
        return;
    };
    let since = chrono::Utc::now() - chrono::Duration::hours(1);
    assert!(org.team_scores(since).await.unwrap().is_empty());
    assert!(org.goal_minds(since).await.unwrap().is_empty());
    assert!(org.owner_signals_after(since).await.unwrap().is_empty());
    assert!(org.latest_org_review().await.unwrap().is_none());

    let team = org
        .create_team(
            "Docs",
            "Keeps the policies right",
            "docs-direction",
            "owner",
        )
        .await
        .unwrap();
    org.add_work(work("docs-direction", "Write the delivery policy"))
        .await
        .unwrap();
    let claimed = org.claim_ready_work("test").await.unwrap().unwrap();
    org.finish_work_attempt(
        claimed.attempt_id,
        WorkAttemptState::Failed,
        "ran out of road",
    )
    .await
    .unwrap();

    let scores = org.team_scores(since).await.unwrap();
    assert_eq!(scores.len(), 1, "{scores:?}");
    let docs = &scores[0];
    assert_eq!(docs.team_id, Some(team));
    assert_eq!(docs.team_name, "Docs");
    assert_eq!(docs.lead_actor_id.as_deref(), Some("docs-direction"));
    assert_eq!((docs.attempts, docs.landed, docs.failed), (1, 0, 1));
    assert_eq!(
        (docs.critic_attempts, docs.minds, docs.work_items),
        (0, 1, 1)
    );

    // Only Exec or the owner reviews the company's shape, and a review must say something.
    let stored = serde_json::json!({ "teams": scores });
    let refused = org
        .record_org_review(
            "docs-direction",
            chrono::Utc::now(),
            3,
            stored.clone(),
            "Mine now.",
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(refused.contains("only Exec or the owner"), "{refused}");
    assert!(org
        .record_org_review("exec", chrono::Utc::now(), 3, stored.clone(), "  ")
        .await
        .is_err());
    let review = org
        .record_org_review(
            "exec",
            chrono::Utc::now(),
            3,
            stored,
            "Docs failed its only attempt; kept the team, changed the brief.",
        )
        .await
        .unwrap();
    let latest = org.latest_org_review().await.unwrap().unwrap();
    assert_eq!(latest.id, review.id);
    assert_eq!(latest.signal_count, 3);
    assert_eq!(org.list_org_reviews(10).await.unwrap().len(), 1);
}
