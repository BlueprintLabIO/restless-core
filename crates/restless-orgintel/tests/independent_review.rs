//! A review is independent of what it reviews: Work that may revise a producer is never owned by
//! the producer's own owner, whether the review edge is declared at creation or added after.

use restless_orgintel::{NewWork, OrgIntel, WorkEdgeKind, WorkspaceSpec};

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "indrev{}_test",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    );
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    for (id, kind, role) in [
        ("owner", "owner", "owner"),
        ("exec", "exec", "exec"),
        ("docs-writer", "staff", "writer"),
        ("policy-checker", "staff", "checker"),
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
        expected_artifact: "/company/outputs/delivery-policy.md",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(1),
    }
}

#[tokio::test]
async fn a_producer_cannot_own_the_review_of_its_own_work() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping independent review scenario");
        return;
    };
    let draft = org
        .add_work(work("docs-writer", "Draft the delivery policy"))
        .await
        .unwrap();

    // Declared at creation: refused, with the way on, and nothing is created.
    let refused = org
        .add_work_with_edges(
            work("docs-writer", "Check my own draft"),
            &[draft],
            &[draft],
        )
        .await
        .unwrap_err()
        .to_string();
    assert!(refused.contains("not independent"), "{refused}");
    assert!(refused.contains("restless people create"), "{refused}");
    assert_eq!(org.list_work().await.unwrap().len(), 1);

    // Someone else may review it.
    let check = org
        .add_work_with_edges(
            work("policy-checker", "Check the draft"),
            &[draft],
            &[draft],
        )
        .await
        .unwrap();
    assert!(org.list_work_edges().await.unwrap().iter().any(|edge| {
        edge.from_work_id == check && edge.to_work_id == draft && edge.kind == WorkEdgeKind::Revises
    }));

    // Added after creation: the same rule.
    let own = org
        .add_work_with_edges(work("docs-writer", "Read my own draft"), &[draft], &[])
        .await
        .unwrap();
    let refused = org
        .add_work_edge(own, draft, WorkEdgeKind::Revises)
        .await
        .unwrap_err()
        .to_string();
    assert!(refused.contains("not independent"), "{refused}");

    // A dependent that only reads the draft, and may not revise it, is not a review.
    let follow_up = org
        .add_work_with_edges(work("docs-writer", "Publish the draft"), &[draft], &[])
        .await;
    assert!(follow_up.is_ok());
}
