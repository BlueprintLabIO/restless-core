//! Work whose Attempt a platform restart killed continues once on its own
//! (Sprint 104 A3); a second restart in the same revision waits for the lead.

use restless_orgintel::{NewAttemptRecovery, NewWork, OrgIntel, WorkStatus, WorkspaceSpec};

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "restartresume{}_test",
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

async fn lose_to_restart(org: &OrgIntel, attempt_id: uuid::Uuid) {
    let observation = serde_json::json!({"workdir": "/company/worktrees/adapter"});
    org.ensure_actor("daemon", "system", "system-sender", "The daemon")
        .await
        .unwrap();
    org.record_unknown_attempt_recovery(
        attempt_id,
        NewAttemptRecovery {
            observed_by: "daemon",
            reason: "supervisor restarted",
            workspace: "/company/worktrees/adapter",
            start_observation: &observation,
            end_observation: &observation,
            start_summary: "unobserved",
            end_summary: "unobserved",
            changed_since_start: false,
            observation_digest: None,
            end_commit: None,
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn a_restart_resumes_work_once_per_revision() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping restart resume scenario");
        return;
    };
    let work = org
        .add_work(NewWork {
            owner_id: "alpha-build",
            title: "Build the adapter",
            outcome: "An adapter",
            goal_id: None,
            priority: 1,
            expected_artifact: "adapter",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let first = org.claim_ready_work("first").await.unwrap().unwrap();
    lose_to_restart(&org, first.attempt_id).await;
    assert_eq!(
        org.get_work(work).await.unwrap().unwrap().status,
        WorkStatus::Blocked
    );

    assert!(org.resume_after_platform_restart(work).await.unwrap());
    let resumed = org.get_work(work).await.unwrap().unwrap();
    assert_eq!(resumed.status, WorkStatus::Active);
    assert_eq!(resumed.attempt_limit, Some(2), "one successor Attempt");
    assert!(
        !org.resume_after_platform_restart(work).await.unwrap(),
        "active Work is not resumed again"
    );

    let second = org
        .claim_ready_work("after restart")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second.work.id, work);
    lose_to_restart(&org, second.attempt_id).await;
    assert!(
        !org.resume_after_platform_restart(work).await.unwrap(),
        "a second restart in the same revision waits for the lead"
    );
    assert_eq!(
        org.get_work(work).await.unwrap().unwrap().status,
        WorkStatus::Blocked
    );

    org.drop_schema().await.unwrap();
}
