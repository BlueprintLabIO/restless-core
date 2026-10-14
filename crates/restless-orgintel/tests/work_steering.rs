//! The owner steers Work from its page: pause and resume, stop and restore, send done Work back,
//! and correct what it is. Each change is admitted for the same people who may resume or stop
//! it, and for nobody else. Runs only when RESTLESS_TEST_DATABASE_URL is set.

use restless_orgintel::{
    NewArtifactRef, NewWork, OrgIntel, WorkAttemptState, WorkDetailsEdit, WorkStatus, WorkspaceSpec,
};

#[tokio::test]
async fn owner_pauses_restores_reopens_and_edits_work() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping work steering scenario");
        return;
    };
    let company = format!("steer{}", std::process::id());
    let org = OrgIntel::ensure(&url, &company)
        .await
        .expect("ensure schema");
    for (id, kind, role, display) in [
        ("owner", "owner", "owner", "The Owner"),
        ("exec", "exec", "exec", "The Exec"),
        ("offer-copy", "staff", "copywriter", "Cora"),
        ("human-sam", "human", "member", "Sam"),
    ] {
        org.ensure_actor(id, kind, role, display).await.unwrap();
    }
    let work = org
        .add_work(NewWork {
            owner_id: "offer-copy",
            title: "Draft the offer",
            outcome: "a one-page offer",
            goal_id: None,
            priority: 0,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let status = |org: OrgIntel| async move { org.get_work(work).await.unwrap().unwrap() };

    // A stranger is refused every change; nothing moves.
    assert!(org.pause_work(work, "human-sam", "not mine").await.is_err());
    let stranger_edit = WorkDetailsEdit {
        work_id: work,
        title: Some("Hijacked"),
        outcome: None,
        expected_artifact: None,
        priority: None,
        goal_id: None,
    };
    assert!(org
        .update_work_details("human-sam", stranger_edit)
        .await
        .is_err());
    assert_eq!(status(org.clone()).await.title, "Draft the offer");

    // Pause is undone by the ordinary resume.
    org.pause_work(work, "owner", "wait for the price")
        .await
        .unwrap();
    let paused = status(org.clone()).await;
    assert_eq!(paused.status, WorkStatus::Blocked);
    assert!(paused.resolution.contains("wait for the price"));
    org.resume_work(work, "owner", "the price is set")
        .await
        .unwrap();
    assert_eq!(status(org.clone()).await.status, WorkStatus::Active);

    // The owner corrects the title, brief, priority and Goal.
    let goal = org.add_goal("Launch the offer", "", "owner").await.unwrap();
    org.update_work_details(
        "owner",
        WorkDetailsEdit {
            work_id: work,
            title: Some("Draft the launch offer"),
            outcome: Some("a one-page launch offer"),
            expected_artifact: Some("/company/offer.md"),
            priority: Some(1),
            goal_id: Some(Some(goal)),
        },
    )
    .await
    .unwrap();
    let edited = status(org.clone()).await;
    assert_eq!(edited.title, "Draft the launch offer");
    assert_eq!(edited.outcome, "a one-page launch offer");
    assert_eq!(edited.priority, 1);
    assert_eq!(edited.goal_id, Some(goal));

    // Stopped Work is not lost: restore brings it back in progress.
    org.abandon_work(work, "owner", "wrong offer")
        .await
        .unwrap();
    assert_eq!(status(org.clone()).await.status, WorkStatus::Abandoned);
    assert!(org
        .restore_work(work, "human-sam", "mine now")
        .await
        .is_err());
    org.restore_work(work, "owner", "it was the right offer after all")
        .await
        .unwrap();
    assert_eq!(status(org.clone()).await.status, WorkStatus::Active);

    // Done Work goes back with feedback that reaches the next run.
    let attempt = org
        .claim_ready_work("restored")
        .await
        .unwrap()
        .expect("restored Work is claimable");
    assert_eq!(attempt.work.id, work);
    org.link_work_artifact(NewArtifactRef {
        kind: "path",
        uri: "/company/offer.md",
        note: "the drafted offer",
        created_by: "offer-copy",
        work_id: Some(work),
        attempt_id: Some(attempt.attempt_id),
        digest: Some("sha256:offer"),
        source_commit: None,
        runtime_generation: Some("test"),
        label: "offer",
    })
    .await
    .unwrap();
    org.finish_work_attempt(
        attempt.attempt_id,
        WorkAttemptState::Produced,
        "offer drafted",
    )
    .await
    .unwrap();
    assert_eq!(status(org.clone()).await.status, WorkStatus::Completed);
    assert!(org.reopen_work(work, "owner", "  ").await.is_err());
    org.reopen_work(work, "owner", "lead with the guarantee")
        .await
        .unwrap();
    let reopened = status(org.clone()).await;
    assert_eq!(reopened.status, WorkStatus::Blocked);
    assert_eq!(reopened.revision, 2);
    let feedback = org.work_feedback(work).await.unwrap();
    assert!(feedback
        .iter()
        .any(|row| row.from_actor == "owner" && row.body == "lead with the guarantee"));

    org.drop_schema().await.ok();
}
