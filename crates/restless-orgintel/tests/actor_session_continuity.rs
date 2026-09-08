//! Concurrency proof for one primary productive session per durable Actor.

use restless_orgintel::{NewWork, OrgIntel, WorkAttemptState, WorkspaceSpec};
use std::sync::Arc;

#[tokio::test]
async fn concurrent_scheduler_scans_start_only_one_attempt_for_an_actor() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Actor session concurrency proof");
        return;
    };
    let company = format!("actorsession{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("delivery-maker", "staff", "builder", "Maker")
        .await
        .unwrap();

    for title in ["First independent outcome", "Second independent outcome"] {
        org.add_work(NewWork {
            owner_id: "delivery-maker",
            title,
            outcome: "Produce one bounded result without sharing a sovereign session.",
            goal_id: None,
            priority: 10,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(2),
        })
        .await
        .unwrap();
    }

    let barrier = Arc::new(tokio::sync::Barrier::new(3));
    let first_org = org.clone();
    let first_barrier = Arc::clone(&barrier);
    let first = tokio::spawn(async move {
        first_barrier.wait().await;
        first_org.claim_ready_work("concurrent scan one").await
    });
    let second_org = org.clone();
    let second_barrier = Arc::clone(&barrier);
    let second = tokio::spawn(async move {
        second_barrier.wait().await;
        second_org.claim_ready_work("concurrent scan two").await
    });
    barrier.wait().await;

    let claims = [
        first.await.unwrap().unwrap(),
        second.await.unwrap().unwrap(),
    ];
    let mut started = claims.into_iter().flatten().collect::<Vec<_>>();
    assert_eq!(
        started.len(),
        1,
        "concurrent scans must not start two primary Attempts for one Actor"
    );

    let running = started.pop().unwrap();
    org.finish_work_attempt(
        running.attempt_id,
        WorkAttemptState::Blocked,
        "bounded test pause",
    )
    .await
    .unwrap();
    let successor = org
        .claim_ready_work("prior Actor session ended")
        .await
        .unwrap()
        .expect("the Actor can accept its other ready Work after termination");
    assert_ne!(successor.work.id, running.work.id);
}
