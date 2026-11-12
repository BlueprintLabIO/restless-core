//! Work whose turn a watchdog cut ("[resumable] the agent produced nothing for 9 minutes")
//! continues on its own, a bounded number of times per revision (S109 blockers audit). Before,
//! nothing ever claimed it again: Sprint 107 sat blocked for five days with no card.

use restless_orgintel::{NewWork, OrgIntel, WorkAttemptState, WorkStatus, WorkspaceSpec};

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "interruptresume{}_test",
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

const CUT: &str = "[resumable] the agent produced nothing for 9 minutes and the turn was cut";

#[tokio::test]
async fn an_interrupted_turn_resumes_a_bounded_number_of_times() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping interruption resume scenario");
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
    for round in 0..restless_orgintel::INTERRUPTION_RESUMES_PER_DAY {
        let claimed = org.claim_ready_work("round").await.unwrap().unwrap();
        assert_eq!(claimed.work.id, work);
        org.finish_work_attempt(claimed.attempt_id, WorkAttemptState::Blocked, CUT)
            .await
            .unwrap();
        assert_eq!(
            org.get_work(work).await.unwrap().unwrap().status,
            WorkStatus::Blocked
        );
        assert!(
            org.resume_after_interruption(work, CUT).await.unwrap(),
            "round {round} resumes"
        );
        assert_eq!(
            org.get_work(work).await.unwrap().unwrap().status,
            WorkStatus::Active
        );
        assert!(
            !org.resume_after_interruption(work, CUT).await.unwrap(),
            "active Work is not resumed again"
        );
    }
    let claimed = org.claim_ready_work("last").await.unwrap().unwrap();
    org.finish_work_attempt(claimed.attempt_id, WorkAttemptState::Blocked, CUT)
        .await
        .unwrap();
    assert!(
        !org.resume_after_interruption(work, CUT).await.unwrap(),
        "past the day's bound, the revision waits"
    );
    assert_eq!(
        org.get_work(work).await.unwrap().unwrap().status,
        WorkStatus::Blocked
    );
    // The daemon's sweep finds it, and the next day it goes on by itself (12 November: the bound
    // was per revision for good, so a third cut or usage-limit reset stranded long Work).
    assert_eq!(
        org.work_stopped_by_platform().await.unwrap(),
        vec![(
            work,
            restless_orgintel::PlatformStop::Interruption(CUT.into())
        )]
    );
    assert!(org
        .resume_after_interruption_at(work, CUT, chrono::Utc::now() + chrono::Duration::hours(25))
        .await
        .unwrap());

    // A turn the owner interrupted is the owner's to restart: the sweep leaves it alone.
    let claimed = org.claim_ready_work("owner").await.unwrap().unwrap();
    org.finish_work_attempt(
        claimed.attempt_id,
        WorkAttemptState::Blocked,
        "[resumable] the owner interrupted the turn",
    )
    .await
    .unwrap();
    assert!(org.work_stopped_by_platform().await.unwrap().is_empty());

    org.drop_schema().await.unwrap();
}
