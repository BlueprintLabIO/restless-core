//! It learns (Sprint 119). Work sent back or failed once is an observation and no rule; the
//! second of its kind is a lesson that names where it came from; a standing instruction from
//! the owner is a rule at once and a one-off correction is not; what a wake carries is capped;
//! a learned lesson nobody needed is retired with its reason and kept; and the weekly message
//! and the daily review carry what changed. Runs against a scratch Postgres company when
//! `RESTLESS_TEST_DATABASE_URL` is available.

use chrono::{Duration, Utc};
use restless_orgintel::{
    carries_org_review, lessons_block, lessons_for_wake, DailyReviewRhythm, LessonChange,
    LessonScope, NewLesson, NewWork, OrgIntel, OutcomeVerdict, WorkAttemptState, WorkEdgeKind,
    WorkspaceSpec, DAILY_REVIEW_MESSAGE_PREFIX, LEARNED_BY, LEARNED_LESSONS_KEPT,
    LEARNED_LESSON_UNUSED_AFTER, LESSONS_BLOCK_BYTES, LESSONS_SHOWN, OBSERVATION_LIVES,
    ORG_REVIEW_RIDER, UNUSED_LESSON_REASON, WHAT_CHANGED_HEADING,
};
use uuid::Uuid;

/// `alpha-ops` leads the Alpha team (`alpha-build`, `alpha-check`); `beta-ops` leads Beta
/// (`beta-build`).
async fn company() -> Option<(OrgIntel, Uuid, Uuid)> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!("learns{}_test", &Uuid::new_v4().simple().to_string()[..12]);
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    for (id, kind, role) in [
        ("owner", "owner", "owner"),
        ("exec", "exec", "exec"),
        ("alpha-ops", "staff", "lead"),
        ("alpha-build", "staff", "builder"),
        ("alpha-check", "staff", "checker"),
        ("beta-ops", "staff", "lead"),
        ("beta-build", "staff", "builder"),
    ] {
        org.ensure_actor(id, kind, role, id).await.unwrap();
    }
    let mut teams = Vec::new();
    for (team, lead, members) in [
        ("Alpha", "alpha-ops", vec!["alpha-build", "alpha-check"]),
        ("Beta", "beta-ops", vec!["beta-build"]),
    ] {
        let id = org
            .create_team(team, "one accountable outcome", lead, "exec")
            .await
            .unwrap();
        for member in members {
            org.set_actor_team(member, Some(id), "exec", "builds for this team")
                .await
                .unwrap();
        }
        teams.push(id);
    }
    Some((org, teams[0], teams[1]))
}

fn work<'a>(owner: &'a str, title: &'a str, outcome: &'a str) -> NewWork<'a> {
    NewWork {
        owner_id: owner,
        title,
        outcome,
        goal_id: None,
        priority: 1,
        expected_artifact: "",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(3),
    }
}

/// Claim the next ready Work, which must be `expected`, and end its Attempt.
async fn run(org: &OrgIntel, expected: Uuid, state: WorkAttemptState, summary: &str) {
    let claimed = org
        .claim_ready_work("test")
        .await
        .unwrap()
        .expect("ready Work");
    assert_eq!(
        claimed.work.id, expected,
        "claimed {:?}",
        claimed.work.title
    );
    org.finish_work_attempt(claimed.attempt_id, state, summary)
        .await
        .unwrap();
}

/// `alpha-build` produces `title`; `alpha-check` reviews it and sends it back for `reason`.
async fn produced_and_sent_back(org: &OrgIntel, title: &str, outcome: &str, reason: &str) -> Uuid {
    let made = org
        .add_work(work("alpha-build", title, outcome))
        .await
        .unwrap();
    let review_title = format!("Review: {title}");
    let review = org
        .add_work(work("alpha-check", &review_title, "checked"))
        .await
        .unwrap();
    org.add_work_edge(made, review, WorkEdgeKind::Requires)
        .await
        .unwrap();
    org.add_work_edge(review, made, WorkEdgeKind::Revises)
        .await
        .unwrap();
    run(org, made, WorkAttemptState::Produced, "draft").await;
    run(org, review, WorkAttemptState::ChangesRequested, reason).await;
    made
}

/// What an actor was told when Work was sent back.
async fn told(org: &OrgIntel, actor: &str) -> Vec<String> {
    org.inbox(Some(actor))
        .await
        .unwrap()
        .into_iter()
        .map(|message| message.body)
        .filter(|body| body.starts_with("Review rejected Work"))
        .collect()
}

#[tokio::test]
async fn work_sent_back_once_is_an_observation_and_twice_the_same_way_is_a_lesson() {
    let Some((org, alpha, _beta)) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping it-learns scenario");
        return;
    };

    // The first send-back: one observation, no rule, nothing new on anyone's wake.
    let menu = produced_and_sent_back(
        &org,
        "Price the lunch menu",
        "Every lunch dish priced.\nAssumptions:\n- Prices include GST.",
        "Prices are shown without GST.",
    )
    .await;
    let seen = org.list_observations(10).await.unwrap();
    assert_eq!(seen.len(), 1, "{seen:?}");
    let first = &seen[0];
    assert_eq!(first.source, "sent_back");
    assert_eq!(first.work_id, Some(menu));
    assert_eq!(first.work_title, "Price the lunch menu");
    assert_eq!(first.by, "alpha-check");
    assert_eq!(first.team_id, Some(alpha));
    assert_eq!(first.expected, "Every lunch dish priced.");
    assert_eq!(first.happened, "Prices are shown without GST.");
    assert_eq!(first.next_time, None);
    assert_eq!(first.lesson_id, None);
    assert!(org.list_lessons(true).await.unwrap().is_empty());
    assert!(org
        .lessons_for_actor("alpha-build")
        .await
        .unwrap()
        .is_empty());
    // With no Brand released, Exec and leads are given none and nothing is invented.
    assert_eq!(org.brand_brief().await.unwrap(), None);
    // Its lead is asked, in the message it was already getting, what it would do differently.
    let lead = told(&org, "alpha-ops").await;
    assert_eq!(lead.len(), 1, "{lead:?}");
    assert!(
        lead[0].contains("Prices are shown without GST."),
        "{}",
        lead[0]
    );
    assert!(
        lead[0].contains(&format!("restless lessons add --once --work {menu}")),
        "{}",
        lead[0]
    );
    // The outcome record reads the same rows.
    let record = org.outcome_record(menu).await.unwrap();
    assert_eq!(record.verdict, OutcomeVerdict::SentBack);
    assert_eq!(record.sent_back, 1);
    assert_eq!(record.expected, "Every lunch dish priced.");
    assert_eq!(record.assumptions, vec!["Prices include GST."]);
    assert_eq!(record.cost_label, "unknown");

    // A different problem on other Work: a second observation, still no rule.
    produced_and_sent_back(
        &org,
        "Pick photos for the menu",
        "Six photos chosen.",
        "The photos are stock photos.",
    )
    .await;
    assert_eq!(org.list_observations(10).await.unwrap().len(), 2);
    assert!(org.list_lessons(true).await.unwrap().is_empty());

    // The same kind of problem on a second piece of Work, in other words: now it is a lesson.
    let quote = produced_and_sent_back(
        &org,
        "Write the catering quote",
        "A quote for forty covers.",
        "The quote shows prices without GST",
    )
    .await;
    let lessons = org.list_lessons(true).await.unwrap();
    assert_eq!(lessons.len(), 1, "{lessons:?}");
    let lesson = &lessons[0];
    assert!(lesson.learned);
    assert_eq!(lesson.recorded_by, LEARNED_BY);
    assert_eq!(lesson.status, "active");
    assert_eq!(
        (lesson.scope_kind.as_str(), lesson.team_id),
        ("team", Some(alpha))
    );
    assert_eq!(
        lesson.rule,
        "Work has been sent back twice for the same reason: \"The quote shows prices without GST\". \
         Check for it before handing work in."
    );
    // It records which observations it came from.
    let from = org.lesson_observations(lesson.id).await.unwrap();
    assert_eq!(
        from.iter().map(|seen| seen.work_id).collect::<Vec<_>>(),
        vec![Some(menu), Some(quote)]
    );
    // The team reads it on its next wake, apart from what the owner ruled. Another team does not.
    let mine = org.lessons_for_actor("alpha-build").await.unwrap();
    let block = lessons_block(&mine);
    assert!(
        block.starts_with("# What the company has learned"),
        "{block}"
    );
    assert!(block.contains("prices without GST"), "{block}");
    assert!(!block.contains("# What the owner has ruled"), "{block}");
    assert_eq!(org.lessons_for_actor("alpha-ops").await.unwrap().len(), 1);
    assert!(org
        .lessons_for_actor("beta-build")
        .await
        .unwrap()
        .is_empty());

    // The lead says what it would do differently. Another team's lead may not; nor may anyone
    // on Work nothing went wrong on.
    let refused = org
        .note_next_time(quote, "beta-ops", "Show every price with GST included.")
        .await
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("only the Work's owner, its lead or Exec"),
        "{refused}"
    );
    let noted = org
        .note_next_time(
            quote,
            "alpha-ops",
            "Show every price with GST included, on every quote and menu.",
        )
        .await
        .unwrap();
    assert_eq!(noted.lesson_id, Some(lesson.id));
    // The lesson takes the lead's words: a sentence someone can follow.
    let lesson = org.lesson(lesson.id).await.unwrap();
    assert_eq!(
        lesson.rule,
        "Show every price with GST included, on every quote and menu."
    );
    assert_eq!(lesson.recurrence_count, 0);

    // A third time, this one from the owner on finished Work: counted on the lesson, not a second.
    org.resume_work(quote, "alpha-ops", "prices now include GST")
        .await
        .unwrap();
    run(&org, quote, WorkAttemptState::Produced, "quote with GST").await;
    let done = org.outcome_record(quote).await.unwrap();
    assert_eq!(done.verdict, OutcomeVerdict::Finished);
    assert_eq!((done.attempts, done.sent_back), (2, 1));
    assert_eq!(done.happened, "quote with GST");
    assert_eq!(
        org.outcome_records(30)
            .await
            .unwrap()
            .iter()
            .filter(|record| record.work_id == quote)
            .count(),
        1
    );
    org.reopen_work(
        quote,
        "owner",
        "Prices are still shown without GST on page two.",
    )
    .await
    .unwrap();
    let lessons = org.list_lessons(true).await.unwrap();
    assert_eq!(lessons.len(), 1, "{lessons:?}");
    assert_eq!(lessons[0].recurrence_count, 1);
    assert_eq!(org.lesson_observations(lesson.id).await.unwrap().len(), 3);
    assert_eq!(org.outcome_record(quote).await.unwrap().sent_back, 2);
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn a_failure_that_was_the_works_own_is_observed_and_the_platforms_is_not() {
    let Some((org, _alpha, beta)) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping it-learns scenario");
        return;
    };
    let first = org
        .add_work(work(
            "beta-build",
            "Fetch the supplier prices",
            "A price list.",
        ))
        .await
        .unwrap();
    run(
        &org,
        first,
        WorkAttemptState::Failed,
        "the supplier price page would not load after 3 tries",
    )
    .await;
    let seen = org.list_observations(10).await.unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(
        (seen[0].source.as_str(), seen[0].by.as_str()),
        ("failed", "beta-build")
    );
    assert!(org.list_lessons(true).await.unwrap().is_empty());

    // The provider's limit is not the Work's failure: nothing is observed.
    let second = org
        .add_work(work(
            "beta-build",
            "Fetch the freight prices",
            "A freight list.",
        ))
        .await
        .unwrap();
    run(
        &org,
        second,
        WorkAttemptState::Failed,
        "[quota] the provider limit was reached",
    )
    .await;
    assert_eq!(org.list_observations(10).await.unwrap().len(), 1);

    // The same failure on other Work: a lesson for that team, in the failure's own words.
    let third = org
        .add_work(work(
            "beta-build",
            "Fetch the wholesale prices",
            "A wholesale list.",
        ))
        .await
        .unwrap();
    run(
        &org,
        third,
        WorkAttemptState::Failed,
        "the supplier price page would not load after 5 tries",
    )
    .await;
    let lessons = org.list_lessons(true).await.unwrap();
    assert_eq!(lessons.len(), 1, "{lessons:?}");
    assert_eq!(lessons[0].team_id, Some(beta));
    assert_eq!(
        lessons[0].rule,
        "Work has failed twice the same way: \"the supplier price page would not load after 5 tries\". \
         Change the approach before it is tried again."
    );
    assert_eq!(
        org.outcome_record(third).await.unwrap().verdict,
        OutcomeVerdict::Failed
    );
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn a_standing_instruction_is_a_rule_at_once_and_a_one_off_correction_waits_for_a_repeat() {
    let Some((org, _alpha, _beta)) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping it-learns scenario");
        return;
    };
    let now = Utc::now();
    let said = |body: &'static str| {
        let org = org.clone();
        async move { org.send_message("owner", Some("exec"), body).await.unwrap() }
    };

    // "From now on": a rule in the same turn, as before.
    let standing = said("From now on, never use stock photos.").await;
    let ruled = org
        .record_lesson(NewLesson {
            rule: "Never use stock photos.",
            scope: LessonScope::Company,
            source_message_id: Some(standing),
            recorded_by: "exec",
        })
        .await
        .unwrap();
    assert!(!ruled.learned);
    assert_eq!(org.list_lessons(false).await.unwrap().len(), 1);

    // A one-off correction: noted, and no rule.
    let once = said("This quote should have gone as a PDF.").await;
    let correction = |message: i64| NewLesson {
        rule: "Send quotes as a PDF, not as a link.",
        scope: LessonScope::Company,
        source_message_id: Some(message),
        recorded_by: "exec",
    };
    let noted = org
        .record_correction_at(now, correction(once))
        .await
        .unwrap();
    assert_eq!(noted.lesson, None);
    assert_eq!(org.list_lessons(false).await.unwrap().len(), 1);
    // The same message noted twice is refused, and it must be a person's message.
    assert!(org
        .record_correction_at(now, correction(once))
        .await
        .is_err());
    let from_exec = org.send_message("exec", None, "Noted.").await.unwrap();
    assert!(org
        .record_correction_at(now, correction(from_exec))
        .await
        .is_err());
    // A worker may not note one for the company.
    let mut by_worker = correction(once);
    by_worker.recorded_by = "alpha-build";
    assert!(org.record_correction_at(now, by_worker).await.is_err());

    // The same correction after the first has expired: an observation again, not a repeat.
    let later = said("Again a link. PDF please.").await;
    let after = now + OBSERVATION_LIVES + Duration::days(2);
    let noted = org
        .record_correction_at(after, correction(later))
        .await
        .unwrap();
    assert_eq!(noted.lesson, None, "an expired observation never counts");
    assert_eq!(org.list_lessons(false).await.unwrap().len(), 1);

    // And once more inside the window: the second of its kind, so now it is a rule.
    let third = said("Link again!").await;
    let noted = org
        .record_correction_at(after + Duration::days(1), correction(third))
        .await
        .unwrap();
    let Some((lesson_id, LessonChange::Promoted)) = noted.lesson else {
        panic!("expected a promotion, got {:?}", noted.lesson);
    };
    let lesson = org.lesson(lesson_id).await.unwrap();
    assert!(lesson.learned);
    assert_eq!(lesson.rule, "Send quotes as a PDF, not as a link.");
    assert_eq!(lesson.scope_kind, "company");
    assert_eq!(lesson.source_message_id, Some(third));
    // Two observations made it; the expired one is not among them.
    let from = org.lesson_observations(lesson_id).await.unwrap();
    assert_eq!(from.len(), 2, "{from:?}");
    assert!(from.iter().all(|seen| seen.source == "correction"));
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn the_wake_is_capped_and_an_unused_learned_lesson_is_retired_kept_and_comes_back() {
    let Some((org, _alpha, _beta)) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping it-learns scenario");
        return;
    };
    let now = Utc::now();
    let said = org
        .send_message("owner", Some("exec"), "House rules, from now on.")
        .await
        .unwrap();
    // Twenty-five standing instructions from the owner.
    for index in 0..25 {
        org.record_lesson(NewLesson {
            rule: &format!("House rule number {index} always holds here."),
            scope: LessonScope::Company,
            source_message_id: Some(said),
            recorded_by: "exec",
        })
        .await
        .unwrap();
    }
    // Two lessons the company learned: one long ago, one last week.
    let learned_at = |when: chrono::DateTime<Utc>, rule: &'static str| {
        let org = org.clone();
        async move {
            let mut lesson = None;
            for _ in 0..2 {
                let message = org.send_message("owner", Some("exec"), rule).await.unwrap();
                lesson = org
                    .record_correction_at(
                        when,
                        NewLesson {
                            rule,
                            scope: LessonScope::Company,
                            source_message_id: Some(message),
                            recorded_by: "exec",
                        },
                    )
                    .await
                    .unwrap()
                    .lesson;
            }
            lesson.expect("promoted on the second").0
        }
    };
    let stale = learned_at(
        now - Duration::days(60),
        "Send quotes as a PDF, not as a link.",
    )
    .await;
    let fresh = learned_at(
        now - Duration::days(7),
        "Check every phone number by calling it.",
    )
    .await;

    // The cap: twenty lessons and two thousand bytes, both learned lessons among them.
    let all = org.lessons_for_actor("alpha-build").await.unwrap();
    assert_eq!(all.len(), 27);
    let (ruled, learned) = lessons_for_wake(&all);
    assert_eq!(ruled.len() + learned.len(), LESSONS_SHOWN);
    assert_eq!(learned.len(), 2);
    assert!(learned.len() <= LEARNED_LESSONS_KEPT);
    let block = lessons_block(&all);
    assert_eq!(block.matches("\n- House rule").count(), 18, "{block}");
    assert!(block.contains("Check every phone number") && block.contains("Send quotes as a PDF"));
    assert!(
        block.contains("- … 7 more not shown: `restless lessons list`"),
        "{block}"
    );
    let lines: usize = block
        .lines()
        .filter(|line| line.starts_with("- ") && !line.contains("more not shown"))
        .map(|line| line.len() + 1)
        .sum();
    assert!(lines <= LESSONS_BLOCK_BYTES, "{lines} bytes of lessons");
    // Most recently useful first.
    assert!(block.find("Check every phone number") < block.find("Send quotes as a PDF"));

    // Pruning: the learned lesson nobody needed for six weeks is retired, with the reason.
    assert!(now - Duration::days(60) < now - LEARNED_LESSON_UNUSED_AFTER);
    let retired = org.prune_learned_lessons_at(now).await.unwrap();
    assert_eq!(
        retired.iter().map(|lesson| lesson.id).collect::<Vec<_>>(),
        vec![stale]
    );
    assert_eq!(retired[0].status, "retired");
    assert_eq!(retired[0].retirement_reason, UNUSED_LESSON_REASON);
    assert_eq!(retired[0].retired_by.as_deref(), Some(LEARNED_BY));
    // It stays in the record; it is only no longer read.
    assert_eq!(org.list_lessons(true).await.unwrap().len(), 27);
    assert_eq!(org.list_lessons(false).await.unwrap().len(), 26);
    assert!(org.prune_learned_lessons_at(now).await.unwrap().is_empty());
    // The owner's instructions are never pruned, however old they get.
    let much_later = org
        .prune_learned_lessons_at(now + Duration::days(400))
        .await
        .unwrap();
    assert_eq!(
        much_later
            .iter()
            .map(|lesson| lesson.id)
            .collect::<Vec<_>>(),
        vec![fresh]
    );
    let left = org.list_lessons(false).await.unwrap();
    assert_eq!(left.len(), 25);
    assert!(left.iter().all(|lesson| !lesson.learned));

    // The owner can bring one back.
    let restored = org.restore_lesson(fresh, "owner").await.unwrap();
    assert_eq!(restored.status, "active");
    // And a dropped one comes back by itself when its problem does.
    let again = org
        .send_message("owner", Some("exec"), "A link again. PDF.")
        .await
        .unwrap();
    let observed = org
        .record_correction_at(
            now + Duration::days(401),
            NewLesson {
                rule: "Send quotes as a PDF, not as a link.",
                scope: LessonScope::Company,
                source_message_id: Some(again),
                recorded_by: "exec",
            },
        )
        .await
        .unwrap();
    assert_eq!(observed.lesson, Some((stale, LessonChange::Restored)));
    let back = org.lesson(stale).await.unwrap();
    assert_eq!((back.status.as_str(), back.recurrence_count), ("active", 1));
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn the_weekly_message_says_what_changed_and_the_team_review_rides_a_daily_review_weekly() {
    let Some((org, _alpha, beta)) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping it-learns scenario");
        return;
    };
    // Nothing has changed about how the company works: the weekly message has no such part.
    // (Teams made by this test's setup are the company being set up, but they are changes too.)
    let before = org.working_changes_since(Some(Utc::now())).await.unwrap();
    assert!(before.is_empty(), "{before:?}");

    // A rule from the owner, a learned lesson, a rule dropped and a team closed.
    let since = Utc::now();
    let said = org
        .send_message("owner", Some("exec"), "From now on, never post on Sundays.")
        .await
        .unwrap();
    org.mark_read(said).await.unwrap();
    let sundays = org
        .record_lesson(NewLesson {
            rule: "Never post on Sundays.",
            scope: LessonScope::Company,
            source_message_id: Some(said),
            recorded_by: "exec",
        })
        .await
        .unwrap();
    produced_and_sent_back(
        &org,
        "Price the lunch menu",
        "priced",
        "Prices are shown without GST.",
    )
    .await;
    let quote = produced_and_sent_back(
        &org,
        "Write the quote",
        "quoted",
        "Prices are shown without GST.",
    )
    .await;
    org.note_next_time(quote, "alpha-ops", "Show every price with GST included.")
        .await
        .unwrap();
    org.retire_lesson(sundays.id, "owner", "We now post every day.")
        .await
        .unwrap();
    org.disband_team(beta, "exec", "one lead now carries both jobs")
        .await
        .unwrap();

    let changed = org.working_changes_since(Some(since)).await.unwrap();
    let all = changed.join("\n");
    for line in [
        "New rule from the owner, for the whole company: Never post on Sundays.",
        "Learned, for the Alpha team, after the same thing went wrong twice: Show every price with GST included.",
        "Dropped a rule (Never post on Sundays.) because: We now post every day.",
        "Closed the Beta team because: one lead now carries both jobs",
    ] {
        assert!(changed.iter().any(|said| said == line), "missing {line:?} in {all}");
    }
    // Plain words: no ids.
    for id in [
        sundays.id.to_string(),
        beta.to_string(),
        quote.to_string(),
        sundays.short_id(),
    ] {
        assert!(!all.contains(&id), "{all}");
    }
    // The weekly direction session carries them, with the heading Exec is to use.
    let state = org.direction_session_state().await.unwrap();
    assert_eq!(
        state.changed,
        org.working_changes_since(Some(Utc::now() - Duration::days(7)))
            .await
            .unwrap()
    );
    let message = state.message(Utc::now());
    assert!(message.contains(WHAT_CHANGED_HEADING), "{message}");
    assert!(message.contains("- Closed the Beta team because: one lead now carries both jobs"));

    // The team review rides a daily review: only when due by the owner's signal, and at most
    // once a week. It never asks for a turn of its own.
    let rhythm = DailyReviewRhythm::default();
    let day = Duration::hours(24);
    org.set_exec_checkins("owner", false).await.unwrap();
    let goal = org
        .add_goal("Three paying clients", "", "owner")
        .await
        .unwrap();
    let stirred = org.daily_review_state().await.unwrap().stirred_at.unwrap();
    let reviews = || async {
        org.inbox(Some("exec"))
            .await
            .unwrap()
            .into_iter()
            .filter(|message| message.body.starts_with(DAILY_REVIEW_MESSAGE_PREFIX))
            .map(|message| carries_org_review(&message.from_actor, &message.body))
            .collect::<Vec<_>>()
    };
    // No daily review is due yet: a due team review wakes nobody.
    assert!(org
        .signal_daily_review_with_at(stirred + Duration::hours(2), rhythm, true)
        .await
        .unwrap()
        .is_none());
    assert_eq!(org.org_review_last_carried().await.unwrap(), None);
    // The first daily review, with a team review due: it carries it, with no owner message.
    let first = stirred + day + Duration::minutes(1);
    assert!(org
        .signal_daily_review_with_at(first, rhythm, true)
        .await
        .unwrap()
        .is_some());
    assert_eq!(reviews().await, vec![true]);
    let asked = org.inbox(Some("exec")).await.unwrap();
    assert!(asked.last().unwrap().body.contains(ORG_REVIEW_RIDER));
    assert_eq!(org.org_review_last_carried().await.unwrap(), Some(first));
    // The next day it is still due and Exec has not recorded it: not carried again.
    assert!(org
        .signal_daily_review_with_at(first + day, rhythm, true)
        .await
        .unwrap()
        .is_some());
    assert_eq!(reviews().await, vec![true, false]);
    // A week on with no new evidence: a daily review, and no team review, whatever the date.
    org.set_goal_details(goal, None, Some("3 signed contracts"), None, "owner")
        .await
        .unwrap();
    let week = first + day * 7 + Duration::minutes(1);
    assert!(org
        .signal_daily_review_with_at(week, rhythm, false)
        .await
        .unwrap()
        .is_some());
    assert_eq!(reviews().await, vec![true, false, false]);
    // New evidence: carried once more, a week after the last time.
    assert!(org
        .signal_daily_review_with_at(week + day, rhythm, true)
        .await
        .unwrap()
        .is_some());
    assert_eq!(reviews().await, vec![true, false, false, true]);
    org.drop_schema().await.unwrap();
}
