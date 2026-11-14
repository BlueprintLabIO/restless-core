//! The company moves first (Sprint 116): once a day a company with an open goal that has nothing
//! in motion, or is nearly due, has Exec asked to start the next step. No open goal never asks,
//! an unread owner message holds it, it keeps apart from check-ins, and after three reviews that
//! start nothing it goes quiet until the owner engages or a goal changes.

use chrono::{Duration, Utc};
use restless_orgintel::{
    CheckinKind, CheckinRhythm, DailyReviewRhythm, NewWork, OrgIntel, WorkspaceSpec,
    DAILY_REVIEW_MESSAGE_PREFIX, QUIET_REVIEWS_BEFORE_STOP,
};

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "dailyreview{}_test",
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

/// The owner writes and Exec has read it.
async fn owner_says(org: &OrgIntel, body: &str) {
    let id = org.send_message("owner", Some("exec"), body).await.unwrap();
    org.mark_read(id).await.unwrap();
}

async fn exec_says(org: &OrgIntel, body: &str) {
    org.send_message("exec", Some("owner"), body).await.unwrap();
}

/// The daily review messages Exec has been sent, oldest first.
async fn reviews(org: &OrgIntel) -> Vec<String> {
    org.inbox(Some("exec"))
        .await
        .unwrap()
        .into_iter()
        .filter(|message| {
            message.from_actor == "daemon" && message.body.starts_with(DAILY_REVIEW_MESSAGE_PREFIX)
        })
        .map(|message| message.body)
        .collect()
}

async fn reviewed(org: &OrgIntel, at: chrono::DateTime<Utc>) -> bool {
    org.signal_daily_review_at(at, DailyReviewRhythm::default())
        .await
        .unwrap()
        .is_some()
}

async fn checked_in(org: &OrgIntel, at: chrono::DateTime<Utc>) -> Option<CheckinKind> {
    org.signal_checkin_at(at, CheckinRhythm::default())
        .await
        .unwrap()
        .map(|(kind, _)| kind)
}

#[tokio::test]
async fn open_goals_are_reviewed_daily_until_the_company_goes_quiet() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping daily review scenario");
        return;
    };
    let start = Utc::now();
    let day = Duration::hours(24);
    let month = Duration::days(30);

    // An empty company, and one whose owner and Exec talked but set no goal: never.
    assert!(!reviewed(&org, start + month).await);
    owner_says(&org, "morning").await;
    exec_says(&org, "Morning.").await;
    assert!(!reviewed(&org, start + day + Duration::minutes(5)).await);
    assert!(!reviewed(&org, start + month).await);

    // The owner sets a goal. Nothing is in motion toward it. A check-in comes first, an hour on.
    let goal = org
        .add_goal("Three paying clients", "", "owner")
        .await
        .unwrap();
    let heard = org.checkin_state().await.unwrap().heard_at.unwrap();
    assert_eq!(
        checked_in(&org, heard + Duration::minutes(61)).await,
        Some(CheckinKind::Checkin)
    );
    // Check-ins switched off: that choice is about messages, and does not stop the work.
    org.set_exec_checkins("owner", false).await.unwrap();

    // A day after the goal was set, the first review; not before, and not twice.
    let stirred = org.daily_review_state().await.unwrap().stirred_at.unwrap();
    assert!(!reviewed(&org, stirred + Duration::hours(23)).await);
    // A company found already quiet (here: looked at two days later) is not woken.
    assert!(!reviewed(&org, stirred + day * 2 + Duration::minutes(1)).await);
    let first = stirred + day + Duration::minutes(1);
    assert!(reviewed(&org, first).await);
    let asked = reviews(&org).await;
    assert_eq!(asked.len(), 1);
    assert!(
        asked[0].contains("- Three paying clients: nothing in motion\n"),
        "{}",
        asked[0]
    );
    assert!(
        !asked[0].contains(&goal.to_string()),
        "no ids: {}",
        asked[0]
    );
    assert!(!reviewed(&org, first + Duration::minutes(1)).await);
    assert!(!reviewed(&org, first + Duration::hours(23)).await);

    // Exec says what it started. The owner's short reply is unread: the next review waits.
    exec_says(
        &org,
        "I started outreach toward three paying clients. Tell me to undo it.",
    )
    .await;
    let second = first + day;
    let reply = org
        .send_message("owner", Some("exec"), "ok thanks")
        .await
        .unwrap();
    assert!(!reviewed(&org, second).await, "held for the unread message");
    org.mark_read(reply).await.unwrap();
    // Their reply to what a review posted is an answer, not a nudge for the org review.
    let signals = org
        .owner_signals_after(start - Duration::hours(1))
        .await
        .unwrap();
    assert!(
        signals.iter().all(|signal| signal.detail != "ok thanks"),
        "{signals:?}"
    );
    assert!(reviewed(&org, second).await, "held, not dropped");

    // Nothing is started and the owner says nothing more: two more, then it stops by itself.
    assert_eq!(org.daily_review_state().await.unwrap().quiet_reviews, 1);
    let third = second + day;
    assert!(reviewed(&org, third).await);
    let fourth = third + day;
    assert!(reviewed(&org, fourth).await);
    let state = org.daily_review_state().await.unwrap();
    assert_eq!(state.quiet_reviews, QUIET_REVIEWS_BEFORE_STOP);
    assert!(!reviewed(&org, fourth + day).await);
    assert!(!reviewed(&org, fourth + month).await);
    assert_eq!(reviews(&org).await.len(), 4);

    // The goal changes: reviewed again.
    org.set_goal_details(goal, None, Some("3 signed contracts"), None, "owner")
        .await
        .unwrap();
    assert_eq!(org.daily_review_state().await.unwrap().quiet_reviews, 0);
    let fifth = fourth + day;
    assert!(reviewed(&org, fifth).await);

    // Work is started toward the goal: that is not a quiet review, and with the goal in motion
    // there is nothing to start, so no model is woken.
    org.add_work(NewWork {
        owner_id: "alpha-build",
        title: "Write to twelve prospects",
        outcome: "twelve sent",
        goal_id: Some(goal),
        priority: 1,
        expected_artifact: "note",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(1),
    })
    .await
    .unwrap();
    let state = org.daily_review_state().await.unwrap();
    assert_eq!(state.quiet_reviews, 0);
    assert!(state.goals[0].in_motion);
    let sixth = fifth + day;
    assert!(!reviewed(&org, sixth).await);
    assert!(!reviewed(&org, sixth + month).await);

    // Unless it is nearly due. Check-ins are back on and the owner is engaged, so both are due
    // in the same scan: the check-in goes first and the review waits out the settling time.
    let settled = sixth + Duration::minutes(15);
    org.set_goal_details(
        goal,
        None,
        None,
        Some(Some((settled + Duration::days(2)).date_naive())),
        "owner",
    )
    .await
    .unwrap();
    org.set_exec_checkins("owner", true).await.unwrap();
    owner_says(&org, "how is it going").await;
    exec_says(&org, "Twelve prospects written to.").await;
    assert_eq!(checked_in(&org, sixth).await, Some(CheckinKind::Checkin));
    assert!(!reviewed(&org, sixth).await, "never in the same scan");
    assert!(!reviewed(&org, sixth + Duration::minutes(14)).await);
    assert!(reviewed(&org, settled).await);
    let asked = reviews(&org).await;
    assert!(
        asked
            .last()
            .unwrap()
            .contains("- Three paying clients: work in motion, due in 2 days ("),
        "{asked:?}"
    );

    // And the other way round: a check-in due just after a review waits too.
    owner_says(&org, "good").await;
    let seventh = settled + day;
    assert!(reviewed(&org, seventh).await);
    assert_eq!(checked_in(&org, seventh + Duration::minutes(1)).await, None);
    assert_eq!(
        checked_in(&org, seventh + Duration::minutes(15)).await,
        Some(CheckinKind::Checkin)
    );

    // The goal is closed: nothing more, ever.
    org.close_goal(goal, "owner").await.unwrap();
    assert!(!reviewed(&org, seventh + month).await);
    org.drop_schema().await.unwrap();
}
