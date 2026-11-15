//! It comes to you with the right questions (Sprint 117). Once a week a company with an open goal
//! and something happening has Exec asked for at most three questions about direction, told
//! with what the company's own records show: the goals, the assumptions written into Work, and
//! what the daily reviews started. One unanswered session and it goes quiet until the owner is
//! back. It keeps apart from check-ins and daily reviews. The brief Exec writes from a daily
//! review is read back without waking anything.

use chrono::{Duration, Utc};
use restless_orgintel::{
    CheckinKind, CheckinRhythm, DailyReviewRhythm, DirectionSessionRhythm, NewWork, OrgIntel,
    WorkspaceSpec, BRIEF_WITHIN, DIRECTION_SESSION_MESSAGE_PREFIX,
};

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "direction{}_test",
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

/// The direction session messages Exec has been sent, oldest first.
async fn sessions(org: &OrgIntel) -> Vec<String> {
    org.inbox(Some("exec"))
        .await
        .unwrap()
        .into_iter()
        .filter(|message| {
            message.from_actor == "daemon"
                && message.body.starts_with(DIRECTION_SESSION_MESSAGE_PREFIX)
        })
        .map(|message| message.body)
        .collect()
}

async fn asked(org: &OrgIntel, at: chrono::DateTime<Utc>) -> bool {
    org.signal_direction_session_at(at, DirectionSessionRhythm::default())
        .await
        .unwrap()
        .is_some()
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

async fn work(org: &OrgIntel, goal: uuid::Uuid, title: &str, outcome: &str) {
    org.add_work(NewWork {
        owner_id: "alpha-build",
        title,
        outcome,
        goal_id: Some(goal),
        priority: 1,
        expected_artifact: "note",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(1),
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_week_of_activity_brings_one_direction_session_then_quiet_until_the_owner_answers() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping direction session scenario");
        return;
    };
    let start = Utc::now();
    let rhythm = DirectionSessionRhythm::default();
    let week = Duration::days(7);
    let day = Duration::hours(24);

    // No goal: never, however much is said.
    owner_says(&org, "morning").await;
    exec_says(&org, "Morning.").await;
    assert_eq!(
        org.direction_session_state().await.unwrap().next(rhythm),
        None
    );
    assert!(!asked(&org, start + week).await);

    // Two goals. A check-in comes an hour on, as it would.
    let clients = org
        .add_goal("Three paying clients", "", "owner")
        .await
        .unwrap();
    org.add_goal("Open the Melbourne shop", "", "owner")
        .await
        .unwrap();
    let heard = org.checkin_state().await.unwrap().heard_at.unwrap();
    assert_eq!(
        checked_in(&org, heard + Duration::minutes(61)).await,
        Some(CheckinKind::Checkin)
    );
    let set = org
        .direction_session_state()
        .await
        .unwrap()
        .first_goal_at
        .unwrap();
    assert!(
        !asked(&org, set + Duration::days(6)).await,
        "not before a week"
    );

    // Day one: the daily review. Exec starts Work toward the first goal, writes down what it
    // rests on, and tells the owner. That message is the brief.
    let stirred = org.daily_review_state().await.unwrap().stirred_at.unwrap();
    let first_review = stirred + day + Duration::minutes(1);
    assert!(reviewed(&org, first_review).await);
    assert_eq!(org.latest_brief(BRIEF_WITHIN).await.unwrap(), None);
    work(
        &org,
        clients,
        "List twelve local prospects",
        "Twelve local firms listed with a contact each.\n\n\
         Assumptions:\n\
         - Local firms answer cold email.\n\
         - Twelve prospects is enough to learn from.\n\
         Least sure of: whether price or timing decides it.",
    )
    .await;
    exec_says(
        &org,
        "Nothing finished yet. Today I am starting a list of twelve local prospects toward \
         three paying clients; tell me to undo it. I need nothing from you.",
    )
    .await;
    let brief = org.latest_brief(BRIEF_WITHIN).await.unwrap().unwrap();
    assert!(brief.body.starts_with("Nothing finished yet."), "{brief:?}");
    assert!(!brief.answered);
    // What Exec says later is not the brief; the first thing it wrote after the review is.
    exec_says(&org, "The list is half done.").await;
    assert_eq!(
        org.latest_brief(BRIEF_WITHIN)
            .await
            .unwrap()
            .unwrap()
            .message_id,
        brief.message_id
    );
    // Too late to be that review's brief: nothing is shown as one.
    assert_eq!(org.latest_brief(Duration::zero()).await.unwrap(), None);

    // A week after the goals were set: one session, told from the records.
    let first = set + week;
    assert!(asked(&org, first).await);
    let told = sessions(&org).await;
    assert_eq!(told.len(), 1);
    for part in [
        "- Three paying clients: work in motion\n",
        "- Open the Melbourne shop: nothing in motion\n",
        "- List twelve local prospects (toward Three paying clients): assumes Local firms answer \
         cold email.; assumes Twelve prospects is enough to learn from.; least sure of whether \
         price or timing decides it.\n",
        "Started by daily reviews: List twelve local prospects\n",
    ] {
        assert!(told[0].contains(part), "missing {part:?} in {}", told[0]);
    }
    assert!(
        !told[0].contains(&clients.to_string()),
        "no ids: {}",
        told[0]
    );
    assert!(
        !asked(&org, first + Duration::minutes(1)).await,
        "not twice"
    );

    // Exec asks its questions. They are not a brief: the brief is still the review's.
    exec_says(
        &org,
        "Two questions about direction. 1. Clients or the shop first? 2. Local only?",
    )
    .await;
    assert_eq!(
        org.latest_brief(BRIEF_WITHIN)
            .await
            .unwrap()
            .unwrap()
            .message_id,
        brief.message_id
    );

    // It keeps apart: a daily review due in the same scan waits out the settling time. What
    // Exec writes from that review is the new brief.
    assert!(!reviewed(&org, first).await, "never in the same scan");
    assert!(!reviewed(&org, first + Duration::minutes(14)).await);
    assert!(reviewed(&org, first + Duration::minutes(15)).await);
    exec_says(
        &org,
        "The list of twelve is done. Today I am starting the emails to them.",
    )
    .await;
    let second = org.latest_brief(BRIEF_WITHIN).await.unwrap().unwrap();
    assert!(second.body.starts_with("The list of twelve is done."));
    assert!(second.review_id > brief.review_id && !second.answered);

    // The owner does not answer the questions: no further session, however busy the company
    // is and however long it has been.
    work(
        &org,
        clients,
        "Write to the twelve",
        "Twelve emails sent.\n\nAssumptions:\n- A short email beats a long one.\n\
         Least sure of: who answers the shared inbox.",
    )
    .await;
    let silent = org.direction_session_state().await.unwrap();
    assert_eq!(silent.unanswered, 1);
    assert_eq!(silent.next(rhythm), None);
    assert!(!asked(&org, first + week).await);
    assert!(!asked(&org, first + week * 8).await);
    assert_eq!(sessions(&org).await.len(), 1);

    // The owner answers one. It is an answer, not a nudge for the org review, and the brief
    // has now been seen.
    owner_says(&org, "1. Clients first").await;
    exec_says(&org, "Clients first. I have moved the shop behind it.").await;
    let signals = org
        .owner_signals_after(start - Duration::hours(1))
        .await
        .unwrap();
    assert!(
        signals
            .iter()
            .all(|signal| signal.detail != "1. Clients first"),
        "{signals:?}"
    );
    assert!(
        org.latest_brief(BRIEF_WITHIN)
            .await
            .unwrap()
            .unwrap()
            .answered
    );

    // The rhythm resumes a week after the last session, and the next one would be told only
    // what was written since.
    let back = org.direction_session_state().await.unwrap();
    assert_eq!(back.unanswered, 0);
    assert_eq!(back.next(rhythm), Some(first + week));
    assert_eq!(
        back.assumptions
            .iter()
            .map(|recorded| recorded.work.as_str())
            .collect::<Vec<_>>(),
        ["Write to the twelve"]
    );
    assert_eq!(back.started_by_reviews, ["Write to the twelve"]);

    // A check-in due just after the session waits too, and a session never follows a wake
    // within the settling time.
    assert_eq!(checked_in(&org, first + Duration::minutes(16)).await, None);
    let later = first + Duration::minutes(40);
    assert_eq!(checked_in(&org, later).await, Some(CheckinKind::Checkin));
    let state = org.direction_session_state().await.unwrap();
    assert_eq!(state.last_wake, Some(later));
    let eager = DirectionSessionRhythm {
        every: Duration::minutes(1),
        settle: Duration::minutes(15),
    };
    assert_eq!(state.next(eager), Some(later + Duration::minutes(15)));

    // Check-ins switched off: Exec does not come with questions either.
    org.set_exec_checkins("owner", false).await.unwrap();
    assert_eq!(
        org.direction_session_state().await.unwrap().next(rhythm),
        None
    );
    org.set_exec_checkins("owner", true).await.unwrap();

    // Every goal closed: nothing more, ever.
    for goal in org.list_goals().await.unwrap() {
        org.close_goal(goal.id, "owner").await.unwrap();
    }
    assert_eq!(
        org.direction_session_state().await.unwrap().next(rhythm),
        None
    );
    assert!(!asked(&org, first + week * 2).await);
    org.drop_schema().await.unwrap();
}
