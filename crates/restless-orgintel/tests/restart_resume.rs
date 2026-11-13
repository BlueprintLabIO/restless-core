//! Work whose Attempt a platform restart killed continues on its own (Sprint 104 A3). The bound
//! is a loop guard, a few times a day, not once per revision for good: a release, a memory
//! restart and a daemon restart in one week used to leave long Work blocked for the owner to
//! resume by hand (12 November).

use restless_orgintel::{
    NewAttemptRecovery, NewWork, OrgIntel, PlatformStop, WorkStatus, WorkspaceSpec,
    PLATFORM_RESUMES_PER_REVISION, RESTART_RESUMES_PER_DAY,
};

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
            reason: "supervisor restarted; cognitive process was lost before trustworthy semantic completion",
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
async fn restarts_resume_work_a_few_times_a_day_and_never_for_good() {
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
    let now = chrono::Utc::now();

    // A release, a memory restart and a daemon restart in one day: each one resumes.
    for restart in 1..=RESTART_RESUMES_PER_DAY {
        let claimed = org.claim_ready_work("restart").await.unwrap().unwrap();
        assert_eq!(claimed.work.id, work);
        lose_to_restart(&org, claimed.attempt_id).await;
        assert_eq!(
            org.get_work(work).await.unwrap().unwrap().status,
            WorkStatus::Blocked
        );
        assert_eq!(
            org.work_stopped_by_platform().await.unwrap(),
            vec![(work, PlatformStop::Restart)],
            "the sweep finds Work a restart stopped"
        );
        assert!(
            org.resume_after_platform_restart(work).await.unwrap(),
            "restart {restart} of the day resumes"
        );
        let resumed = org.get_work(work).await.unwrap().unwrap();
        assert_eq!(resumed.status, WorkStatus::Active);
        assert_eq!(
            resumed.attempt_limit,
            Some(1 + i32::try_from(restart).unwrap()),
            "one successor Attempt each time"
        );
        assert!(
            !org.resume_after_platform_restart(work).await.unwrap(),
            "active Work is not resumed again"
        );
        assert!(org.work_stopped_by_platform().await.unwrap().is_empty());
    }

    // A crash loop stops: one more restart the same day leaves it blocked.
    let claimed = org.claim_ready_work("loop").await.unwrap().unwrap();
    lose_to_restart(&org, claimed.attempt_id).await;
    assert!(!org.resume_after_platform_restart(work).await.unwrap());
    assert_eq!(
        org.get_work(work).await.unwrap().unwrap().status,
        WorkStatus::Blocked
    );

    // But not for good: the next day the same sweep resumes it, with nobody pressing Resume.
    let mut resumes = RESTART_RESUMES_PER_DAY;
    let mut day = 1;
    while resumes < PLATFORM_RESUMES_PER_REVISION {
        let later = now + chrono::Duration::hours(25 * day);
        assert!(
            org.resume_after_platform_restart_at(work, later)
                .await
                .unwrap(),
            "resume {} happens a day later",
            resumes + 1
        );
        resumes += 1;
        day += 1;
        let claimed = org.claim_ready_work("later").await.unwrap().unwrap();
        lose_to_restart(&org, claimed.attempt_id).await;
    }

    // Past the total for one revision the Work itself is suspect: it stays blocked for a person.
    assert!(
        !org.resume_after_platform_restart_at(work, now + chrono::Duration::days(365))
            .await
            .unwrap(),
        "the total per revision holds whatever the clock says"
    );
    assert_eq!(
        org.get_work(work).await.unwrap().unwrap().status,
        WorkStatus::Blocked
    );

    org.drop_schema().await.unwrap();
}
