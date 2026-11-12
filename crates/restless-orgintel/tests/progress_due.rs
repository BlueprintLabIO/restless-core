//! Progress comes to the owner without "progress?" (12 November 2026). Exec is woken for one
//! update when Work moved and the owner has heard nothing for two hours. The same changes never
//! ask twice, nothing changed never asks, and an owner message Exec has yet to answer holds it.

use chrono::{Duration, Utc};
use restless_orgintel::{NewWork, OrgIntel, WorkAttemptState, WorkspaceSpec};

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "progressdue{}_test",
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

/// Add one Work item and end its Attempt blocked: a material change.
async fn block_one(org: &OrgIntel, title: &str) {
    let id = org
        .add_work(NewWork {
            owner_id: "alpha-build",
            title,
            outcome: "done",
            goal_id: None,
            priority: 1,
            expected_artifact: "note",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let claimed = org.claim_ready_work("test").await.unwrap().unwrap();
    assert_eq!(claimed.work.id, id);
    org.finish_work_attempt(
        claimed.attempt_id,
        WorkAttemptState::Blocked,
        "needs a token",
    )
    .await
    .unwrap();
}

async fn asks(org: &OrgIntel) -> usize {
    org.inbox(Some("exec"))
        .await
        .unwrap()
        .into_iter()
        .filter(|message| {
            message.from_actor == "daemon" && message.body.starts_with("Progress update due")
        })
        .count()
}

#[tokio::test]
async fn exec_is_asked_for_one_update_when_work_moved_and_never_when_nothing_changed() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping progress update scenario");
        return;
    };
    // Nothing has happened: no ask, however long it has been.
    assert!(org
        .signal_progress_due_at(Utc::now() + Duration::days(30))
        .await
        .unwrap()
        .is_none());

    block_one(&org, "Publish the release").await;
    block_one(&org, "Write the changelog").await;
    let start = Utc::now();
    // Inside two hours of the first change: not yet.
    assert!(org
        .signal_progress_due_at(start + Duration::minutes(90))
        .await
        .unwrap()
        .is_none());
    // Two hours on: one ask covers both changes.
    let first = start + Duration::minutes(125);
    let due = org.signal_progress_due_at(first).await.unwrap().unwrap();
    assert_eq!((due.landed, due.stuck), (0, 2));
    assert_eq!(asks(&org).await, 1);
    // The same changes never ask twice, a minute or a month later.
    for later in [Duration::minutes(1), Duration::hours(3), Duration::days(30)] {
        assert!(org
            .signal_progress_due_at(first + later)
            .await
            .unwrap()
            .is_none());
    }
    assert_eq!(asks(&org).await, 1);

    // Something new moves. Still nothing until two hours after the last ask.
    block_one(&org, "Draft the launch post").await;
    assert!(org
        .signal_progress_due_at(first + Duration::minutes(60))
        .await
        .unwrap()
        .is_none());
    let second = first + Duration::minutes(121);
    let due = org.signal_progress_due_at(second).await.unwrap().unwrap();
    assert_eq!(due.stuck, 1, "only what moved since the last ask");
    assert_eq!(asks(&org).await, 2);

    // The owner asked something Exec has not answered yet: Exec is about to speak anyway.
    block_one(&org, "Tag the build").await;
    org.send_message("owner", Some("exec"), "how is the launch going?")
        .await
        .unwrap();
    assert!(org
        .signal_progress_due_at(second + Duration::minutes(130))
        .await
        .unwrap()
        .is_none());
    assert_eq!(asks(&org).await, 2);
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn what_exec_already_told_the_owner_is_not_asked_for_again() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping progress update scenario");
        return;
    };
    block_one(&org, "Publish the release").await;
    // Exec writes to the owner: that is the update, and the wait starts again from it.
    org.send_message(
        "exec",
        Some("owner"),
        "The release is blocked on a token; I asked for it.",
    )
    .await
    .unwrap();
    let told = Utc::now();
    assert!(
        org.signal_progress_due_at(told + Duration::days(30))
            .await
            .unwrap()
            .is_none(),
        "nothing moved since Exec's message"
    );
    block_one(&org, "Write the changelog").await;
    assert!(org
        .signal_progress_due_at(told + Duration::minutes(100))
        .await
        .unwrap()
        .is_none());
    let due = org
        .signal_progress_due_at(told + Duration::minutes(125))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(due.stuck, 1, "only what moved since Exec's message");
    assert_eq!(asks(&org).await, 1);
    org.drop_schema().await.unwrap();
}
