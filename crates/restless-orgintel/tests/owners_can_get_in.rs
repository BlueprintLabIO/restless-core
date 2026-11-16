//! Sprint 120. The attention report counts, from the company's own rows, the results the owner
//! accepted and each time the owner stepped in, by the kind the rows support; and a goal the
//! owner starts from the first-run offer is an ordinary goal that the daily review moves on.
//! Runs against a scratch Postgres company when `RESTLESS_TEST_DATABASE_URL` is available.

use chrono::{Duration, Utc};
use restless_orgintel::{
    AttentionKind, DailyReviewRhythm, InitialWorkGate, LessonScope, NewArtifactRef, NewGateRun,
    NewLesson, NewWork, OrgIntel, OwnerBrief, OwnerBriefKind, OwnerChatAnswer, OwnerReviewDecision,
    WorkAttemptState, WorkEdgeKind, WorkStatus, WorkspaceSpec, DAILY_REVIEW_MESSAGE_PREFIX,
    REVIEW_TARGET_ARTIFACT_KIND, REVIEW_TARGET_LIVE_PROBE_GATE,
};
use uuid::Uuid;

/// `menu-ops` leads the Menu team; `menu-build` produces and `menu-check` reviews.
async fn company(prefix: &str) -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "{prefix}{}_test",
        &Uuid::new_v4().simple().to_string()[..12]
    );
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    for (id, kind, role) in [
        ("owner", "owner", "owner"),
        ("exec", "exec", "exec"),
        ("menu-ops", "staff", "lead"),
        ("menu-build", "staff", "builder"),
        ("menu-check", "staff", "checker"),
    ] {
        org.ensure_actor(id, kind, role, id).await.unwrap();
    }
    let team = org
        .create_team("Menu", "the new menu", "menu-ops", "exec")
        .await
        .unwrap();
    for member in ["menu-build", "menu-check"] {
        org.set_actor_team(member, Some(team), "exec", "works on the menu")
            .await
            .unwrap();
    }
    Some(org)
}

fn work<'a>(owner: &'a str, title: &'a str, goal_id: Option<Uuid>) -> NewWork<'a> {
    NewWork {
        owner_id: owner,
        title,
        outcome: "done as asked",
        goal_id,
        priority: 1,
        expected_artifact: "",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(3),
    }
}

/// The owner writes and Exec has read it. Returns the message id.
async fn owner_says(org: &OrgIntel, body: &str) -> i64 {
    let id = org.send_message("owner", Some("exec"), body).await.unwrap();
    org.mark_read(id).await.unwrap();
    id
}

async fn exec_says(org: &OrgIntel, body: &str) {
    org.send_message("exec", Some("owner"), body).await.unwrap();
}

/// Work whose result the owner reviews, produced by `menu-build` with its review target
/// linked and probed. Returns the Work and the review waiting on it.
async fn produced_for_owner_review(org: &OrgIntel, title: &str) -> (Uuid, Uuid) {
    let probe = vec!["sh".to_string(), "-c".to_string(), "true".to_string()];
    let work_id = org
        .add_review_required_work_with_edges_and_gates(
            NewWork {
                expected_artifact: "native ReviewTarget",
                ..work("menu-build", title, None)
            },
            &[],
            &[],
            &[InitialWorkGate {
                name: REVIEW_TARGET_LIVE_PROBE_GATE,
                command: &probe,
                stage: "cumulative",
                timeout_seconds: 900,
                resources: &[],
            }],
        )
        .await
        .unwrap();
    let attempt = org
        .claim_ready_work("test")
        .await
        .unwrap()
        .expect("ready Work");
    assert_eq!(attempt.work.id, work_id);
    org.link_work_artifact(NewArtifactRef {
        kind: REVIEW_TARGET_ARTIFACT_KIND,
        uri: "http://127.0.0.1:4173/menu.html",
        note: "The menu as it will print.",
        created_by: "menu-build",
        work_id: Some(work_id),
        attempt_id: Some(attempt.attempt_id),
        digest: Some("sha256:menu"),
        source_commit: None,
        runtime_generation: Some("test-runtime"),
        label: "The menu",
    })
    .await
    .unwrap();
    let gate = org
        .list_work_gates(work_id)
        .await
        .unwrap()
        .into_iter()
        .find(|gate| gate.name == REVIEW_TARGET_LIVE_PROBE_GATE)
        .unwrap();
    org.record_gate_run(NewGateRun {
        gate_id: gate.id,
        attempt_id: attempt.attempt_id,
        exit_code: Some(0),
        output_digest: "sha256:ok",
        output_excerpt: "HTTP target responded 200",
        passed: true,
    })
    .await
    .unwrap();
    org.finish_work_attempt(
        attempt.attempt_id,
        WorkAttemptState::Produced,
        "ready to look at",
    )
    .await
    .unwrap();
    let handoff = org
        .list_owner_handoffs()
        .await
        .unwrap()
        .into_iter()
        .find(|handoff| handoff.work_id == work_id)
        .expect("a review waits on the produced Work");
    (work_id, handoff.id)
}

fn is_nudge(body: &str) -> bool {
    matches!(
        body.trim().to_lowercase().as_str(),
        "progress?" | "continue" | "go on"
    )
}

#[tokio::test]
async fn the_attention_report_counts_each_time_the_owner_stepped_in_by_what_the_rows_show() {
    let Some(org) = company("attention").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping the attention report scenario");
        return;
    };
    let start = Utc::now() - Duration::minutes(1);

    // 1. A tapped answer quotes the question it answers.
    owner_says(&org, "> Exec: Which comes first?\n\nClients first").await;
    // 2. A reply straight after a question is an answer, even in a nudge's words.
    exec_says(
        &org,
        "Shall I continue?\n\n<!--restless-intent:{\"kind\":\"conversation\",\"summary\":\"Asking\",\"ownerNeed\":\"Shall I continue?\"}-->",
    )
    .await;
    owner_says(&org, "continue").await;
    // 3. The same kind of words with nothing asked: the owner had to come and pull.
    owner_says(&org, "progress?").await;
    // 4. Unprompted, and no record says what it was.
    owner_says(&org, "Also look at catering for offices next month.").await;
    // 5. A message Exec recorded a rule from was a correction.
    let corrected = owner_says(&org, "From now on, always show prices with GST.").await;
    org.record_lesson(NewLesson {
        rule: "Always show prices with GST.",
        scope: LessonScope::Company,
        source_message_id: Some(corrected),
        recorded_by: "exec",
    })
    .await
    .unwrap();

    // 6. A goal the owner set is new direction; one Exec set is not the owner's act.
    org.add_goal("A new menu, priced and printed", "", "owner")
        .await
        .unwrap();
    org.add_goal("Tidy the supplier list", "", "exec")
        .await
        .unwrap();
    // 7. The first reply after a daily brief answers it.
    let stirred = org.daily_review_state().await.unwrap().stirred_at.unwrap();
    assert!(
        org.signal_daily_review_at(
            stirred + Duration::hours(24) + Duration::minutes(1),
            DailyReviewRhythm::default()
        )
        .await
        .unwrap()
        .is_some(),
        "a daily review is due a day after the goal was set"
    );
    exec_says(
        &org,
        "Today I am starting the lunch prices unless you say no.",
    )
    .await;
    owner_says(&org, "go on").await;

    // 8. A result accepted on its card.
    let (menu, review) = produced_for_owner_review(&org, "Print-ready lunch menu").await;
    org.prepare_owner_brief(
        review,
        "menu-ops",
        OwnerBrief {
            kind: OwnerBriefKind::OutcomeReview,
            headline: "Look over the lunch menu".into(),
            situation: "The lunch menu is laid out and priced.".into(),
            impact: "It goes to print once you are happy.".into(),
            recommendation: "Accept it.".into(),
            no_action: "Nothing is printed.".into(),
            uncertainty: None,
            deadline: None,
        },
    )
    .await
    .unwrap();
    org.escalate_handoff(review, "menu-ops", "checked; the owner's taste decides")
        .await
        .unwrap();
    org.escalate_handoff(review, "exec", "ready for the owner")
        .await
        .unwrap();
    org.decide_owner_review(review, OwnerReviewDecision::Accepted, "accepted")
        .await
        .unwrap();
    assert_eq!(
        org.get_work(menu).await.unwrap().unwrap().status,
        WorkStatus::Completed
    );

    // 9. A result accepted in chat: one act, not a message and a decision.
    let (_dinner, dinner_review) = produced_for_owner_review(&org, "Print-ready dinner menu").await;
    let go = owner_says(&org, "The dinner menu looks good, go.").await;
    org.answer_handoff_from_owner_message(
        dinner_review,
        go,
        OwnerChatAnswer::Yes,
        "Accepted in chat.",
    )
    .await
    .unwrap();

    // Work a checker sent back is the company's own doing, not the owner's.
    let made = org
        .add_work(work("menu-build", "Price the specials", None))
        .await
        .unwrap();
    let check = org
        .add_work(work("menu-check", "Check the specials", None))
        .await
        .unwrap();
    org.add_work_edge(made, check, WorkEdgeKind::Requires)
        .await
        .unwrap();
    org.add_work_edge(check, made, WorkEdgeKind::Revises)
        .await
        .unwrap();
    for (state, summary) in [
        (WorkAttemptState::Produced, "draft"),
        (
            WorkAttemptState::ChangesRequested,
            "Prices are shown without GST.",
        ),
    ] {
        let claimed = org
            .claim_ready_work("test")
            .await
            .unwrap()
            .expect("ready Work");
        org.finish_work_attempt(claimed.attempt_id, state, summary)
            .await
            .unwrap();
    }

    // 10. Done Work the owner sent back.
    org.reopen_work(menu, "owner", "The soup is missing.")
        .await
        .unwrap();

    let end = Utc::now() + Duration::minutes(1);
    let report = org
        .attention_report(start, end, is_nudge, Vec::new())
        .await
        .unwrap();
    let count = |kind: AttentionKind| {
        report
            .by_kind
            .iter()
            .find(|row| row.kind == kind)
            .unwrap()
            .count
    };
    let listed = report.summary();
    assert_eq!(report.accepted_results.len(), 2, "{listed}");
    assert_eq!(report.total, 10, "{listed}");
    assert_eq!(count(AttentionKind::AcceptedResult), 2, "{listed}");
    assert_eq!(count(AttentionKind::SentBack), 2, "{listed}");
    assert_eq!(count(AttentionKind::Answered), 3, "{listed}");
    assert_eq!(count(AttentionKind::Direction), 1, "{listed}");
    assert_eq!(count(AttentionKind::Chased), 1, "{listed}");
    assert_eq!(count(AttentionKind::Unclassified), 1, "{listed}");
    assert_eq!(
        count(AttentionKind::Approved) + count(AttentionKind::HumanStep),
        0
    );
    assert_eq!(report.per_accepted_result, Some(5.0));
    assert_eq!(report.chased_per_accepted_result, Some(0.5));
    // What each one was is in the owner's words or the title of what they settled.
    for part in [
        "Chased for status: progress?",
        "Answered the company: Clients first",
        "Answered the company: continue",
        "Answered the company: go on",
        "Not told by the records: Also look at catering for offices next month.",
        "Sent work back or corrected it: From now on, always show prices with GST.",
        "Sent work back or corrected it: Sent back: Print-ready lunch menu",
        "Gave new direction: Set a goal: A new menu, priced and printed",
        "Accepted a result: Print-ready lunch menu",
        "Accepted a result: Print-ready dinner menu",
    ] {
        assert!(listed.contains(part), "missing {part:?} in\n{listed}");
    }
    assert!(
        !listed.contains("looks good, go"),
        "counted twice:\n{listed}"
    );
    assert!(!listed.contains("Tidy the supplier list"), "{listed}");

    // Outside the stretch asked for, nothing; and what another plane holds is counted as given.
    let later = org
        .attention_report(end, end + Duration::days(3), is_nudge, Vec::new())
        .await
        .unwrap();
    assert_eq!((later.total, later.accepted_results.len()), (0, 0));
    assert_eq!(later.per_accepted_result, None);
    let approval = restless_orgintel::Intervention::new(
        start + Duration::seconds(30),
        AttentionKind::Approved,
        "First contact with the printer Approved.",
        "approval",
    );
    let with_approval = org
        .attention_report(start, end, is_nudge, vec![approval])
        .await
        .unwrap();
    assert_eq!(with_approval.total, 11);
    assert_eq!(with_approval.per_accepted_result, Some(5.5));
}

#[tokio::test]
async fn a_starter_outcome_is_an_ordinary_goal_that_the_daily_review_moves_on() {
    let Some(org) = company("starter").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping the starter outcome scenario");
        return;
    };
    // What the first-run offer does when a starter is tapped: one goal, set by the owner, whose
    // text carries its finish line. Nothing else is special about it.
    let objective = "Reply to every customer enquiry within a day\nDone when: every enquiry from the last week has a reply drafted for you to approve.";
    let goal = org
        .add_goal(
            "Reply to every customer enquiry within a day",
            objective,
            "owner",
        )
        .await
        .unwrap();
    let goals = org.list_goals().await.unwrap();
    assert_eq!(goals.len(), 1);
    assert_eq!(goals[0].id, goal);
    assert_eq!(goals[0].created_by, "owner");

    // With nothing started under it, the next daily review asks Exec to start its first step.
    let stirred = org.daily_review_state().await.unwrap().stirred_at.unwrap();
    let first = stirred + Duration::hours(24) + Duration::minutes(1);
    assert!(org
        .signal_daily_review_at(first, DailyReviewRhythm::default())
        .await
        .unwrap()
        .is_some());
    let asked: Vec<String> = org
        .inbox(Some("exec"))
        .await
        .unwrap()
        .into_iter()
        .filter(|message| message.body.starts_with(DAILY_REVIEW_MESSAGE_PREFIX))
        .map(|message| message.body)
        .collect();
    assert_eq!(asked.len(), 1);
    assert!(
        asked[0].contains("- Reply to every customer enquiry within a day: nothing in motion\n"),
        "{}",
        asked[0]
    );

    // Its first piece of Work is ordinary Work under that goal, and counts as the goal moving.
    let first_step = org
        .add_work(work(
            "menu-ops",
            "Draft replies to this week's enquiries",
            Some(goal),
        ))
        .await
        .unwrap();
    assert_eq!(
        org.get_work(first_step).await.unwrap().unwrap().goal_id,
        Some(goal)
    );
    let ready = org
        .claim_ready_work("test")
        .await
        .unwrap()
        .expect("the first step is ready");
    assert_eq!(ready.work.id, first_step);
    assert!(
        org.signal_daily_review_at(first + Duration::hours(24), DailyReviewRhythm::default())
            .await
            .unwrap()
            .is_none(),
        "a goal with work in motion needs no next step"
    );
}
