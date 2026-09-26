use chrono::{Duration, Utc};
use restless_orgintel::{NewWork, OrgIntel, WorkspaceSpec};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

#[tokio::test]
async fn settled_linked_work_gets_one_bounded_adjudication_wake() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping real database smoke");
        return;
    };
    let company = format!("budget{}_test", Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    let start = Utc::now();
    let responsibility = Uuid::new_v4();
    org.create_exact_schedule_with_responsibility(
        "exec",
        "disposable budget smoke",
        start,
        "local_mac",
        responsibility,
        1,
        "Wait for linked Work, then decide",
        json!({"window_seconds": 7_200}),
    )
    .await
    .unwrap();
    assert_eq!(org.claim_due_schedules_at(start).await.unwrap().len(), 1);
    let opportunity = org
        .list_opportunities(Some(responsibility), 1)
        .await
        .unwrap()[0]
        .clone();
    assert_eq!(opportunity.wake_count, 1);
    let claim = org
        .claim_opportunity(opportunity.id, "exec", 3_600, start)
        .await
        .unwrap()
        .unwrap();
    let work = org
        .add_work(NewWork {
            owner_id: "exec",
            title: "Disposable linked Work",
            outcome: "Return one durable result",
            goal_id: None,
            priority: 0,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: None,
        })
        .await
        .unwrap();
    org.link_opportunity_work(opportunity.id, claim.owner_epoch, work, "primary", start)
        .await
        .unwrap();
    sqlx::query(&format!(
        "UPDATE {company}.work SET status='active' WHERE id=$1"
    ))
    .bind(work)
    .execute(&pool)
    .await
    .unwrap();
    org.settle_opportunity(
        opportunity.id,
        claim.owner_epoch,
        "waiting_retry",
        json!({"reason":"Work in progress","next_wake_at":start + Duration::minutes(5)}),
        start,
    )
    .await
    .unwrap();

    // Simulate three lost delivery windows. The fourth wake is the ordinary
    // budget; the linked Work has not settled yet.
    for minute in [5, 10, 15] {
        org.wake_due_opportunities_with_policy_at(start + Duration::minutes(minute), 20, 300)
            .await
            .unwrap();
    }
    let held = org
        .wake_due_opportunities_at(start + Duration::minutes(20))
        .await
        .unwrap();
    assert!(held.is_empty());
    let held = org.get_opportunity(opportunity.id).await.unwrap().unwrap();
    assert_eq!(held.state, "waiting_retry");
    assert_eq!(held.wake_count, 4);

    sqlx::query(&format!(
        "UPDATE {company}.work SET status='completed', resolution='finished' WHERE id=$1"
    ))
    .bind(work)
    .execute(&pool)
    .await
    .unwrap();
    let result_time = start + Duration::minutes(20) + Duration::seconds(1);
    assert_eq!(
        org.expedite_waiting_opportunities_with_work_result(result_time)
            .await
            .unwrap(),
        1
    );
    let progress = org.wake_due_opportunities_at(result_time).await.unwrap();
    assert_eq!(progress.len(), 1);
    assert_eq!(progress[0].sequence, 5);
    let adjudicating = org.get_opportunity(opportunity.id).await.unwrap().unwrap();
    assert_eq!(adjudicating.wake_count, 5);
    assert_eq!(adjudicating.state, "queued");

    // The same result cannot mint a second credit after the fifth delivery.
    let final_wake = org
        .wake_due_opportunities_at(result_time + Duration::minutes(5))
        .await
        .unwrap();
    assert_eq!(final_wake.len(), 1);
    assert!(final_wake[0].settled);
    assert_eq!(
        org.get_opportunity(opportunity.id)
            .await
            .unwrap()
            .unwrap()
            .state,
        "blocked"
    );
    pool.close().await;
    org.drop_schema().await.unwrap();
}
