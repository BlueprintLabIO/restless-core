//! Exec checks in with the owner (13 November 2026): hourly while they engage, one nudge three
//! hours after a check-in they did not answer, then nothing until they return. Nothing active
//! never asks, an unanswered owner message holds it, and a check-in is not owner activity.

use chrono::{Duration, Utc};
use restless_orgintel::{
    CheckinKind, CheckinRhythm, NewWork, OrgIntel, WorkAttemptState, WorkspaceSpec,
    CHECKIN_MESSAGE_PREFIX,
};

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "checkin{}_test",
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

/// Add one Work item and end its Attempt blocked: open Work, and a change.
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

/// The owner writes and Exec has read it (its answer is posted separately).
async fn owner_says(org: &OrgIntel, body: &str) {
    let id = org.send_message("owner", Some("exec"), body).await.unwrap();
    org.mark_read(id).await.unwrap();
}

async fn exec_says(org: &OrgIntel, body: &str) {
    org.send_message("exec", Some("owner"), body).await.unwrap();
}

async fn asks(org: &OrgIntel) -> usize {
    org.inbox(Some("exec"))
        .await
        .unwrap()
        .into_iter()
        .filter(|message| {
            message.from_actor == "daemon" && message.body.starts_with(CHECKIN_MESSAGE_PREFIX)
        })
        .count()
}

async fn asked(org: &OrgIntel, at: chrono::DateTime<Utc>) -> Option<CheckinKind> {
    org.signal_checkin_at(at, CheckinRhythm::default())
        .await
        .unwrap()
        .map(|(kind, _)| kind)
}

#[tokio::test]
async fn the_rhythm_follows_the_owner_and_never_posts_into_silence() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping check-in scenario");
        return;
    };
    let start = Utc::now();
    let month = Duration::days(30);

    // An empty company, and one where the owner and Exec talked but nothing is open: never.
    assert_eq!(asked(&org, start + month).await, None);
    owner_says(&org, "ship the launch page this week").await;
    exec_says(&org, "On it.").await;
    assert_eq!(
        asked(&org, start + month).await,
        None,
        "nothing active: no model is woken"
    );

    // Work opens. An hour after Exec last wrote to them, the first check-in.
    block_one(&org, "Publish the release").await;
    let talked = org.checkin_state().await.unwrap().heard_at.unwrap();
    assert_eq!(asked(&org, talked + Duration::minutes(59)).await, None);
    let first = talked + Duration::minutes(61);
    assert_eq!(asked(&org, first).await, Some(CheckinKind::Checkin));
    assert_eq!(asks(&org).await, 1);
    assert_eq!(asked(&org, first + Duration::minutes(1)).await, None);

    // Exec ended that turn quietly and nothing moved: no nudge about nothing, ever.
    assert_eq!(asked(&org, first + month).await, None);

    // Exec did post, with a question, and the owner said nothing. Not hourly: one nudge at
    // three hours.
    exec_says(
        &org,
        "The release is waiting on a token. 1. Use the old one or wait for the new? I would wait.",
    )
    .await;
    assert_eq!(asked(&org, first + Duration::minutes(90)).await, None);
    assert_eq!(asked(&org, first + Duration::minutes(179)).await, None);
    let nudge = first + Duration::hours(3);
    assert_eq!(asked(&org, nudge).await, Some(CheckinKind::Nudge));
    assert_eq!(asks(&org).await, 2);

    // After the nudge, silence, whatever happens and however long.
    exec_says(&org, "Still open: old token or wait? I would wait.").await;
    block_one(&org, "Write the changelog").await;
    assert_eq!(asked(&org, nudge + Duration::hours(1)).await, None);
    assert_eq!(asked(&org, nudge + month).await, None);
    assert_eq!(asks(&org).await, 2);

    // None of this was the owner: not activity for blocked Work, not a signal for the org review.
    let active = org.owner_last_active_at().await.unwrap().unwrap();
    assert!(active < talked, "only their own message counts as activity");
    let signals = org
        .owner_signals_after(start - Duration::hours(1))
        .await
        .unwrap();
    assert_eq!(signals.len(), 1, "{signals:?}");
    assert_eq!(signals[0].detail, "ship the launch page this week");

    // The owner returns with a one-word answer. While Exec has not read it, nothing is asked.
    let answer = org
        .send_message("owner", Some("exec"), "continue")
        .await
        .unwrap();
    assert_eq!(asked(&org, nudge + month).await, None);
    org.mark_read(answer).await.unwrap();
    exec_says(&org, "Waiting for the new token, then publishing.").await;
    // Their answer to a check-in is not a nudge they had to give.
    let signals = org
        .owner_signals_after(start - Duration::hours(1))
        .await
        .unwrap();
    assert!(
        signals.iter().all(|signal| signal.detail != "continue"),
        "{signals:?}"
    );

    // The hourly rhythm is back, counted from the last check-in they answered.
    let state = org.checkin_state().await.unwrap();
    assert_eq!(state.unanswered, 0);
    assert_eq!(state.stuck, 1, "the changelog, since the nudge");
    assert_eq!(asked(&org, nudge + Duration::minutes(59)).await, None);

    // Turned off by the owner: nothing. Turned back on: the check-in that was due.
    let due = nudge + Duration::minutes(61);
    org.set_exec_checkins("owner", false).await.unwrap();
    assert!(!org.exec_checkins("owner").await.unwrap());
    assert_eq!(asked(&org, due).await, None);
    // Someone who is not an owner cannot turn the owner's check-ins off.
    org.set_exec_checkins("owner", true).await.unwrap();
    org.set_exec_checkins("alpha-build", false).await.unwrap();
    assert_eq!(asked(&org, due).await, Some(CheckinKind::Checkin));
    assert_eq!(asks(&org).await, 3);

    // Acting without writing is engagement too (here: something the owner did for blocked Work).
    exec_says(&org, "Changelog is blocked too. 1. Skip it? I would not.").await;
    assert_eq!(org.checkin_state().await.unwrap().unanswered, 1);
    assert!(org.note_owner_activity("viewer_closed").await.unwrap());
    let state = org.checkin_state().await.unwrap();
    assert_eq!(state.unanswered, 0);
    assert_eq!(
        state.next(CheckinRhythm::default()).map(|(_, kind)| kind),
        Some(CheckinKind::Checkin),
        "hourly again, not a nudge"
    );
    org.drop_schema().await.unwrap();
}
