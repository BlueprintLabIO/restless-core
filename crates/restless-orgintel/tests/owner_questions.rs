//! A question for the owner travels; the work does not stop (Sprint 118). A worker's question,
//! or one a lead has held for an hour, is recorded once, rides Exec's next check-in, and stays
//! open until Exec has passed the owner's answer to its lead. Recording it never blocks Work.

use chrono::{Duration, Utc};
use restless_orgintel::{
    health_problems, CheckinKind, CheckinRhythm, HealthRhythm, NewOwnerHandoff, NewOwnerQuestion,
    NewWork, OrgIntel, OwnerHandoffCategory, OwnerHandoffState, WorkAttemptState, WorkStatus,
    WorkspaceSpec, CHECKIN_MESSAGE_PREFIX, HEALTH_NOTICE_PREFIX,
};
use uuid::Uuid;

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!("ownerq{}_test", &Uuid::new_v4().simple().to_string()[..12]);
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    for (id, kind, role) in [
        ("owner", "owner", "owner"),
        ("exec", "exec", "exec"),
        ("alpha-ops", "staff", "lead"),
        ("alpha-build", "staff", "builder"),
    ] {
        org.ensure_actor(id, kind, role, id).await.unwrap();
    }
    let team = org
        .create_team("Alpha", "the menu, priced and printed", "alpha-ops", "exec")
        .await
        .unwrap();
    org.set_actor_team("alpha-build", Some(team), "exec", "builds for this team")
        .await
        .unwrap();
    Some(org)
}

fn work<'a>(owner: &'a str, title: &'a str) -> NewWork<'a> {
    NewWork {
        owner_id: owner,
        title,
        outcome: "done",
        goal_id: None,
        priority: 1,
        expected_artifact: "",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(2),
    }
}

/// The owner writes and Exec has read it.
async fn owner_says(org: &OrgIntel, body: &str) {
    let id = org.send_message("owner", Some("exec"), body).await.unwrap();
    org.mark_read(id).await.unwrap();
}

async fn exec_says(org: &OrgIntel, body: &str) {
    org.send_message("exec", Some("owner"), body).await.unwrap();
}

/// The check-in messages Exec has been sent, oldest first.
async fn checkins(org: &OrgIntel) -> Vec<String> {
    org.inbox(Some("exec"))
        .await
        .unwrap()
        .into_iter()
        .filter(|message| {
            message.from_actor == "daemon" && message.body.starts_with(CHECKIN_MESSAGE_PREFIX)
        })
        .map(|message| message.body)
        .collect()
}

async fn checked_in(org: &OrgIntel, at: chrono::DateTime<Utc>) -> Option<CheckinKind> {
    org.signal_checkin_at(at, CheckinRhythm::default())
        .await
        .unwrap()
        .map(|(kind, _)| kind)
}

#[tokio::test]
async fn a_workers_question_rides_the_next_check_in_and_the_work_goes_on() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping owner question scenario");
        return;
    };
    owner_says(&org, "price the new menu and get it printed").await;
    exec_says(&org, "On it.").await;
    let menu = org
        .add_work(work("alpha-build", "Price the menu"))
        .await
        .unwrap();
    let claimed = org.claim_ready_work("test").await.unwrap().unwrap();

    // The worker hits a question only the owner can answer, picks the likely answer and
    // finishes. The question is recorded once.
    let ask = NewOwnerQuestion {
        key: &format!("attempt:{}", claimed.attempt_id),
        work_id: menu,
        asked_by: "alpha-build",
        question: "Should the menu show prices with or without tax?",
        assumed: "with tax, as on the old menu",
        source: "attempt",
    };
    assert!(org.record_owner_question(ask).await.unwrap());
    assert!(
        !org.record_owner_question(ask).await.unwrap(),
        "once per Attempt"
    );
    org.finish_work_attempt(
        claimed.attempt_id,
        WorkAttemptState::Produced,
        "menu priced",
    )
    .await
    .unwrap();
    // Nothing was blocked: the Work finished on its assumption, and nobody was woken for it.
    assert_eq!(
        org.get_work(menu).await.unwrap().unwrap().status,
        WorkStatus::Completed
    );
    assert!(
        !org.inbox(Some("alpha-ops"))
            .await
            .unwrap()
            .iter()
            .any(|message| message.body.contains("with or without tax")),
        "the lead does not have to relay it"
    );
    let open = org.open_owner_questions().await.unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(
        open[0].lead, "alpha-ops",
        "the answer goes to the Work's lead"
    );
    assert_eq!(open[0].carried, 0);
    // A blank question and one about Work that is not there are not recorded.
    assert!(!org
        .record_owner_question(NewOwnerQuestion {
            key: "attempt:blank",
            question: "   ",
            ..ask
        })
        .await
        .unwrap());
    assert!(!org
        .record_owner_question(NewOwnerQuestion {
            key: "attempt:nowhere",
            work_id: Uuid::new_v4(),
            ..ask
        })
        .await
        .unwrap());

    // The question is itself a reason to check in, and it rides that check-in.
    let state = org.checkin_state().await.unwrap();
    assert_eq!((state.team_questions, state.uncarried_questions), (1, 1));
    let due = state.heard_at.unwrap() + Duration::minutes(61);
    assert_eq!(checked_in(&org, due - Duration::minutes(5)).await, None);
    assert_eq!(checked_in(&org, due).await, Some(CheckinKind::Checkin));
    let asked = checkins(&org).await;
    assert_eq!(asked.len(), 1);
    for part in [
        "Questions from the team for the owner",
        "one of your three question slots",
        // The lead by the name people see, and the id Exec writes back to.
        "(`alpha-ops`) on \"Price the menu\"",
        "Should the menu show prices with or without tax?",
        "Meanwhile assuming: with tax, as on the old menu",
        "restless message --to",
    ] {
        assert!(asked[0].contains(part), "missing {part:?} in {}", asked[0]);
    }

    // Exec puts it to the owner. It stays open until the answer has gone back down.
    exec_says(
        &org,
        "1. Should the menu show prices with or without tax? I would show them with tax.",
    )
    .await;
    let open = org.open_owner_questions().await.unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].carried, 1);
    assert_eq!(org.checkin_state().await.unwrap().uncarried_questions, 0);
    // Exec telling the lead something before the owner has answered settles nothing.
    org.send_message(
        "exec",
        Some("alpha-ops"),
        "I have asked the owner about tax.",
    )
    .await
    .unwrap();
    assert_eq!(org.open_owner_questions().await.unwrap().len(), 1);
    // The owner answers; still open until Exec passes it on, naming the Work it is about.
    owner_says(&org, "Without tax").await;
    assert_eq!(org.open_owner_questions().await.unwrap().len(), 1);
    org.send_message("exec", Some("alpha-ops"), "Thanks for the print quotes.")
        .await
        .unwrap();
    assert_eq!(
        org.open_owner_questions().await.unwrap().len(),
        1,
        "a message about something else settles nothing"
    );
    org.send_message(
        "exec",
        Some("alpha-ops"),
        "Price the menu: the owner says prices are shown without tax.",
    )
    .await
    .unwrap();
    assert!(org.open_owner_questions().await.unwrap().is_empty());
    assert_eq!(org.checkin_state().await.unwrap().team_questions, 0);
    let lead_mail = org.inbox(Some("alpha-ops")).await.unwrap();
    assert!(
        lead_mail
            .iter()
            .any(|message| message.body.contains("without tax")),
        "the answer reached the lead: {lead_mail:?}"
    );
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn a_question_a_lead_has_held_for_an_hour_goes_up_and_the_work_is_let_go() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping held question scenario");
        return;
    };
    let rhythm = HealthRhythm::default();
    owner_says(&org, "get the new menu printed").await;
    exec_says(&org, "On it.").await;
    let print = org
        .add_work(work("alpha-build", "Print the menu"))
        .await
        .unwrap();
    // The worker stops and reports back to its lead, as its brief told it to.
    let handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: print,
            attempt_id: None,
            requested_by: "alpha-build",
            category: OwnerHandoffCategory::OwnerJudgement,
            requested_action:
                "Which printer do we use: the one near the shop or the cheaper one online?",
            prepared_state: "Both quotes are saved with the menu file.",
            resume_condition: "a printer is chosen",
        })
        .await
        .unwrap();
    assert_eq!(
        org.get_work(print).await.unwrap().unwrap().status,
        WorkStatus::Blocked
    );
    let facts = org.health_facts().await.unwrap();
    assert_eq!(facts.held.len(), 1);
    assert_eq!(facts.held[0].holder, "alpha-ops");
    let asked_at = facts.held[0].since;

    // Within the hour it is the lead's to settle: nothing is raised or carried.
    let soon = asked_at + Duration::minutes(59);
    assert!(health_problems(&facts, soon, rhythm, &[]).is_empty());
    assert_eq!(
        org.carry_held_questions_at(soon, &facts.held, rhythm)
            .await
            .unwrap(),
        0
    );
    assert!(org.open_owner_questions().await.unwrap().is_empty());

    // The lead sat on it. After an hour the question goes to the owner's next check-in whatever
    // the lead does, and the lead is told once to let the Work go on.
    let late = asked_at + Duration::minutes(61);
    assert_eq!(
        org.carry_held_questions_at(late, &facts.held, rhythm)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        org.carry_held_questions_at(late + Duration::hours(5), &facts.held, rhythm)
            .await
            .unwrap(),
        0,
        "recorded once"
    );
    let found = health_problems(&facts, late, rhythm, &[]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].kind, "held-question");
    assert_eq!(found[0].to, "alpha-ops");
    assert_eq!(
        org.raise_health_problems_at(late, &found, rhythm)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(org
        .raise_health_problems_at(late + Duration::days(2), &found, rhythm)
        .await
        .unwrap()
        .is_empty());
    let told: Vec<_> = org
        .inbox(Some("alpha-ops"))
        .await
        .unwrap()
        .into_iter()
        .filter(|message| message.body.starts_with(HEALTH_NOTICE_PREFIX))
        .collect();
    assert_eq!(told.len(), 1);
    assert!(
        told[0].body.contains("Which printer do we use"),
        "{}",
        told[0].body
    );
    assert!(told[0].body.contains("Do not keep the Work waiting"));
    assert!(told[0].body.contains(&handoff.to_string()));
    // The notice changed nothing by itself.
    assert_eq!(
        org.get_work(print).await.unwrap().unwrap().status,
        WorkStatus::Blocked
    );

    // The lead does as the notice says: answers with a stated assumption. The Work is released
    // and the question is still on its way to the owner.
    org.resolve_handoff_as(
        handoff,
        "alpha-ops",
        OwnerHandoffState::Resolved,
        "Assumed: the printer near the shop, because the proof can be checked in person.",
    )
    .await
    .unwrap();
    assert_ne!(
        org.get_work(print).await.unwrap().unwrap().status,
        WorkStatus::Blocked,
        "the Work moved on"
    );
    assert!(org.health_facts().await.unwrap().held.is_empty());
    let open = org.open_owner_questions().await.unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].lead, "alpha-ops");
    assert!(open[0].question.contains("Which printer do we use"));

    let due = org.checkin_state().await.unwrap().heard_at.unwrap() + Duration::minutes(61);
    assert_eq!(
        checked_in(&org, due.max(late)).await,
        Some(CheckinKind::Checkin)
    );
    let asked = checkins(&org).await;
    assert_eq!(asked.len(), 1);
    assert!(asked[0].contains("Which printer do we use"), "{}", asked[0]);
    assert!(
        asked[0].contains(
            "Meanwhile assuming: the printer near the shop, because the proof can be checked in person."
        ),
        "the lead's assumption travels with the question: {}",
        asked[0]
    );

    // A held judgement its lead settles for real is no longer a question for the owner.
    let paper = org
        .add_work(work("alpha-build", "Order the paper"))
        .await
        .unwrap();
    let second = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: paper,
            attempt_id: None,
            requested_by: "alpha-build",
            category: OwnerHandoffCategory::OwnerJudgement,
            requested_action: "Matt or gloss paper?",
            prepared_state: "Both samples are in the shop.",
            resume_condition: "a paper is chosen",
        })
        .await
        .unwrap();
    let held = org.health_facts().await.unwrap().held;
    let later = held[0].since + Duration::minutes(61);
    assert_eq!(
        org.carry_held_questions_at(later, &held, rhythm)
            .await
            .unwrap(),
        1
    );
    let open = org.open_owner_questions().await.unwrap();
    assert!(open
        .iter()
        .any(|question| question.question == "Matt or gloss paper?"));
    org.resolve_handoff_as(
        second,
        "alpha-ops",
        OwnerHandoffState::Resolved,
        "Matt: it is what the old menu used and the owner chose it then.",
    )
    .await
    .unwrap();
    let open = org.open_owner_questions().await.unwrap();
    assert!(
        !open
            .iter()
            .any(|question| question.question == "Matt or gloss paper?"),
        "{open:?}"
    );
    org.drop_schema().await.unwrap();
}
