//! The quality bar belongs to the outcome: a Goal sets it and its Work
//! inherits it unless a piece of Work states its own. Runs against a scratch
//! Postgres company when `RESTLESS_TEST_DATABASE_URL` is available.

use restless_orgintel::{NewWork, OrgIntel, OutcomeStandard, WorkspaceSpec};

#[tokio::test]
async fn work_inherits_its_goal_bar_until_it_states_its_own() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping goal standard scenario");
        return;
    };
    let company = format!("goalbar{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    let work = |title: &'static str, goal_id| NewWork {
        owner_id: "exec",
        title,
        outcome: "A useful result",
        goal_id,
        priority: 0,
        expected_artifact: "",
        workspace: WorkspaceSpec::default(),
        attempt_limit: None,
    };
    let goal = org.add_goal("Launch", "", "owner").await.unwrap();
    let listed = org.list_goals().await.unwrap();
    assert_eq!(listed[0].outcome_standard, OutcomeStandard::default());

    org.set_goal_standard(goal, OutcomeStandard::Fast, "owner")
        .await
        .unwrap();
    let served = org.add_work(work("Draft copy", Some(goal))).await.unwrap();
    let loose = org.add_work(work("Tidy files", None)).await.unwrap();
    assert_eq!(
        org.effective_work_standard(served).await.unwrap(),
        OutcomeStandard::Fast,
        "Work inherits its Goal's bar"
    );
    assert_eq!(
        org.effective_work_standard(loose).await.unwrap(),
        OutcomeStandard::default(),
        "Work that serves no Goal takes the default"
    );

    org.set_work_standard(served, Some(OutcomeStandard::Frontier), "owner")
        .await
        .unwrap();
    org.set_goal_standard(goal, OutcomeStandard::Thorough, "owner")
        .await
        .unwrap();
    assert_eq!(
        org.effective_work_standard(served).await.unwrap(),
        OutcomeStandard::Frontier,
        "a Work's own bar outlasts a later Goal change"
    );
    let graph = org.work_graph_snapshot().await.unwrap();
    assert_eq!(graph.standards.len(), 1);
    assert_eq!(graph.standards[0].work_id, served);

    org.set_work_standard(served, None, "owner").await.unwrap();
    assert_eq!(
        org.effective_work_standard(served).await.unwrap(),
        OutcomeStandard::Thorough,
        "inherit returns the Work to its Goal's current bar"
    );
    assert!(!org
        .set_goal_standard(uuid::Uuid::new_v4(), OutcomeStandard::Fast, "owner")
        .await
        .unwrap());

    org.close().await;
    let cleanup = sqlx::PgPool::connect(&url).await.unwrap();
    sqlx::query(&format!("DROP SCHEMA {company} CASCADE"))
        .execute(&cleanup)
        .await
        .unwrap();
    cleanup.close().await;
}
