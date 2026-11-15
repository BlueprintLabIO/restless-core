//! It notices and repairs itself (Sprint 118): the organisational signals are read from the
//! company's own rows and each problem owes one message to its accountable lead. A healthy
//! company raises nothing, a problem is raised once, it is raised again only when it has
//! changed, and raising it never touches Work.

use chrono::{Duration, Utc};
use restless_orgintel::{
    health_problems, HealthProblem, HealthRhythm, NewWork, OrgIntel, WorkAttemptState,
    WorkEdgeKind, WorkStatus, WorkspaceSpec, HEALTH_NOTICE_PREFIX, MAX_HEALTH_WAKES_PER_DAY,
};
use uuid::Uuid;

/// A company with one team: `alpha-ops` leads `alpha-build` and `alpha-check`. `beta-ops`
/// leads a second team with `beta-build`.
async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!("health{}_test", &Uuid::new_v4().simple().to_string()[..12]);
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
    }
    Some(org)
}

fn work<'a>(owner: &'a str, title: &'a str, goal: Option<Uuid>) -> NewWork<'a> {
    NewWork {
        owner_id: owner,
        title,
        outcome: "done",
        goal_id: goal,
        priority: 1,
        expected_artifact: "",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(1),
    }
}

/// Everything the rows show now, with every wait already passed.
async fn problems(org: &OrgIntel, rhythm: HealthRhythm) -> Vec<HealthProblem> {
    let facts = org.health_facts().await.unwrap();
    health_problems(&facts, Utc::now() + Duration::hours(2), rhythm, &[])
}

/// The health notices an actor has been sent.
async fn notices(org: &OrgIntel, actor: &str) -> Vec<String> {
    org.inbox(Some(actor))
        .await
        .unwrap()
        .into_iter()
        .filter(|message| {
            message.from_actor == "daemon" && message.body.starts_with(HEALTH_NOTICE_PREFIX)
        })
        .map(|message| message.body)
        .collect()
}

async fn statuses(org: &OrgIntel) -> Vec<(Uuid, WorkStatus, i64)> {
    let mut all: Vec<_> = org
        .list_work()
        .await
        .unwrap()
        .into_iter()
        .map(|work| (work.id, work.status, work.revision))
        .collect();
    all.sort_by_key(|(id, _, _)| *id);
    all
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

#[tokio::test]
async fn a_healthy_company_with_open_work_raises_nothing() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping healthy company scenario");
        return;
    };
    let goal = org
        .add_goal("Three paying clients", "", "owner")
        .await
        .unwrap();
    // Work waiting, Work that failed once, Work blocked with one job behind it, a finished
    // result, and two different jobs under one goal.
    let list = org
        .add_work(work(
            "alpha-build",
            "List twelve local prospects",
            Some(goal),
        ))
        .await
        .unwrap();
    run(&org, list, WorkAttemptState::Produced, "twelve listed").await;
    let offer = org
        .add_work(work("alpha-build", "Draft the one-line offer", Some(goal)))
        .await
        .unwrap();
    run(
        &org,
        offer,
        WorkAttemptState::Blocked,
        "waiting for the price",
    )
    .await;
    let send = org
        .add_work(work(
            "alpha-check",
            "Write to the first three prospects",
            Some(goal),
        ))
        .await
        .unwrap();
    org.add_work_edge(offer, send, WorkEdgeKind::Requires)
        .await
        .unwrap();
    org.add_work(work("beta-build", "Set up the shared inbox", Some(goal)))
        .await
        .unwrap();

    let rhythm = HealthRhythm::default();
    let found = problems(&org, rhythm).await;
    assert!(found.is_empty(), "{found:?}");
    let before = statuses(&org).await;
    assert!(org
        .raise_health_problems_at(Utc::now(), &found, rhythm)
        .await
        .unwrap()
        .is_empty());
    for actor in ["exec", "alpha-ops", "beta-ops", "alpha-build"] {
        assert!(notices(&org, actor).await.is_empty(), "{actor}");
    }
    assert_eq!(statuses(&org).await, before);
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn each_problem_is_raised_once_to_its_lead_and_again_only_when_it_changes() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping health problem scenario");
        return;
    };
    let rhythm = HealthRhythm::default();
    let now = Utc::now();

    // One blocker holding two jobs. A builder owns it; its team's lead answers for it.
    let prices = org
        .add_work(work("alpha-build", "Get the supplier price list", None))
        .await
        .unwrap();
    run(
        &org,
        prices,
        WorkAttemptState::Blocked,
        "the supplier price list is missing",
    )
    .await;
    let menu = org
        .add_work(work("alpha-check", "Price the menu", None))
        .await
        .unwrap();
    let print = org
        .add_work(work("alpha-check", "Print the menu", None))
        .await
        .unwrap();
    for behind in [menu, print] {
        org.add_work_edge(prices, behind, WorkEdgeKind::Requires)
            .await
            .unwrap();
    }
    // Not yet: it has been blocked for less than half an hour.
    let facts = org.health_facts().await.unwrap();
    assert!(health_problems(&facts, now, rhythm, &[]).is_empty());

    let found = problems(&org, rhythm).await;
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].kind, "shared-blocker");
    assert_eq!(found[0].to, "alpha-ops");
    let before = statuses(&org).await;
    let raised = org
        .raise_health_problems_at(now, &found, rhythm)
        .await
        .unwrap();
    assert_eq!(raised.len(), 1);
    let told = notices(&org, "alpha-ops").await;
    assert_eq!(told.len(), 1, "{told:?}");
    assert!(
        told[0].contains("2 other open jobs require it"),
        "{}",
        told[0]
    );
    assert!(told[0].contains("the supplier price list is missing"));
    assert!(told[0].contains("Nothing has been paused or changed"));
    // Advice only: no Work changed state or revision, and nobody else was told.
    assert_eq!(statuses(&org).await, before);
    assert!(notices(&org, "exec").await.is_empty());
    assert!(notices(&org, "alpha-build").await.is_empty());

    // The same problem on the next sweep, and the one after: nothing.
    for later in [Duration::minutes(10), Duration::days(3)] {
        let again = problems(&org, rhythm).await;
        assert_eq!(again[0].key, found[0].key);
        assert!(org
            .raise_health_problems_at(now + later, &again, rhythm)
            .await
            .unwrap()
            .is_empty());
    }
    assert_eq!(notices(&org, "alpha-ops").await.len(), 1);

    // The lead resumes it and it stops the same way again. That is a new Attempt, so the
    // blocker is told once more, and two failures the same way ask for a replan.
    org.resume_work(prices, "alpha-ops", "asked the supplier again")
        .await
        .unwrap();
    run(
        &org,
        prices,
        WorkAttemptState::Blocked,
        "the supplier price list is missing",
    )
    .await;
    let changed = problems(&org, rhythm).await;
    let kinds: Vec<_> = changed.iter().map(|problem| problem.kind).collect();
    assert_eq!(kinds, ["replan", "shared-blocker"], "{changed:?}");
    assert!(changed.iter().all(|problem| problem.to == "alpha-ops"));
    assert_ne!(changed[1].key, found[0].key);
    let raised = org
        .raise_health_problems_at(now + Duration::hours(1), &changed, rhythm)
        .await
        .unwrap();
    assert_eq!(raised.len(), 2);
    let told = notices(&org, "alpha-ops").await;
    assert_eq!(told.len(), 3);
    let replan = told
        .iter()
        .find(|body| body.contains("2 times in a row the same way"))
        .expect("the replan ask");
    assert!(replan.contains("change the approach, split the Work, or drop it"));
    assert!(replan.contains("restless work resume"));
    // A third Attempt runs only once someone has resumed the Work: the notice did not.
    assert!(org.claim_ready_work("test").await.unwrap().is_none());
    assert!(org
        .raise_health_problems_at(now + Duration::hours(2), &changed, rhythm)
        .await
        .unwrap()
        .is_empty());

    // Unblocked: the problem is gone and stays gone.
    org.resume_work(prices, "alpha-ops", "the supplier sent a new list")
        .await
        .unwrap();
    run(&org, prices, WorkAttemptState::Produced, "price list saved").await;
    assert!(problems(&org, rhythm).await.is_empty());

    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn duplicate_work_goes_to_exec_and_waits_its_turn() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping duplicate Work scenario");
        return;
    };
    let rhythm = HealthRhythm::default();
    let now = Utc::now();
    let goal = org
        .add_goal("Three paying clients", "", "owner")
        .await
        .unwrap();
    // Two teams were each given the same job under one goal. A third job is only related.
    let first = org
        .add_work(work(
            "alpha-build",
            "List twelve local prospects",
            Some(goal),
        ))
        .await
        .unwrap();
    let second = org
        .add_work(work(
            "beta-build",
            "List twelve local prospects",
            Some(goal),
        ))
        .await
        .unwrap();
    org.add_work(work(
        "alpha-check",
        "Write to the twelve local prospects",
        Some(goal),
    ))
    .await
    .unwrap();

    let found = problems(&org, rhythm).await;
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].kind, "duplicate-work");
    assert_eq!(
        found[0].to, "exec",
        "two leads: nobody below Exec answers for both"
    );

    // Exec is about to answer the owner: the notice waits, and is not lost.
    let unread = org
        .send_message("owner", Some("exec"), "how is it going?")
        .await
        .unwrap();
    assert!(org
        .raise_health_problems_at(now, &found, rhythm)
        .await
        .unwrap()
        .is_empty());
    assert!(notices(&org, "exec").await.is_empty());
    org.mark_read(unread).await.unwrap();
    let before = statuses(&org).await;
    assert_eq!(
        org.raise_health_problems_at(now, &found, rhythm)
            .await
            .unwrap()
            .len(),
        1
    );
    let told = notices(&org, "exec").await;
    assert_eq!(told.len(), 1);
    assert!(told[0].contains("read like the same job"), "{}", told[0]);
    assert!(told[0].contains("say which you kept"));
    assert_eq!(
        statuses(&org).await,
        before,
        "both jobs carry on until Exec decides"
    );
    assert!(org
        .raise_health_problems_at(now + Duration::days(1), &found, rhythm)
        .await
        .unwrap()
        .is_empty());

    // Exec stops one of the two: nothing left to raise.
    org.abandon_work(second, "exec", "the same job as the Alpha team's")
        .await
        .unwrap();
    assert!(problems(&org, rhythm).await.is_empty());
    assert_eq!(
        org.get_work(first).await.unwrap().unwrap().status,
        WorkStatus::Proposed
    );
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn review_that_keeps_sending_work_back_is_a_loop_past_the_third_revision() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping review loop scenario");
        return;
    };
    let rhythm = HealthRhythm::default();
    let copy = org
        .add_work(work("alpha-build", "Landing page copy", None))
        .await
        .unwrap();
    let review = org
        .add_work(work("alpha-check", "Review the landing page copy", None))
        .await
        .unwrap();
    org.add_work_edge(copy, review, WorkEdgeKind::Requires)
        .await
        .unwrap();
    org.add_work_edge(review, copy, WorkEdgeKind::Revises)
        .await
        .unwrap();

    for round in 1..=3_i64 {
        run(&org, copy, WorkAttemptState::Produced, "draft").await;
        run(
            &org,
            review,
            WorkAttemptState::ChangesRequested,
            "the headline is vague",
        )
        .await;
        let sent_back = org.get_work(copy).await.unwrap().unwrap();
        assert_eq!(sent_back.revision, round + 1);
        let found = problems(&org, rhythm).await;
        if round < 3 {
            // Sent back on its second and third revision: ordinary review, not a loop.
            assert!(found.is_empty(), "round {round}: {found:?}");
            org.resume_work(copy, "alpha-ops", "headline rewritten as asked")
                .await
                .unwrap();
        } else {
            assert_eq!(found.len(), 1, "{found:?}");
            assert_eq!(found[0].kind, "review-loop");
            assert_eq!(found[0].to, "alpha-ops");
            assert_eq!(found[0].key, format!("review-loop:{copy}:4"));
            let raised = org
                .raise_health_problems_at(Utc::now(), &found, rhythm)
                .await
                .unwrap();
            assert_eq!(raised.len(), 1);
            assert!(org
                .raise_health_problems_at(Utc::now(), &found, rhythm)
                .await
                .unwrap()
                .is_empty());
        }
    }
    let told = notices(&org, "alpha-ops").await;
    assert_eq!(told.len(), 1);
    assert!(told[0].contains("is on revision 4 and review has sent it back again"));
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn a_wrong_detector_cannot_spend_freely_and_a_missing_lead_falls_to_exec() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping health bound scenario");
        return;
    };
    let rhythm = HealthRhythm::default();
    let now = Utc::now();
    let flood: Vec<HealthProblem> = (0..MAX_HEALTH_WAKES_PER_DAY + 3)
        .map(|n| HealthProblem {
            kind: "replan",
            key: format!("replan:test:{n}"),
            to: "alpha-ops".into(),
            work_id: None,
            detail: format!("Problem {n}."),
        })
        .collect();
    let raised = org
        .raise_health_problems_at(now, &flood, rhythm)
        .await
        .unwrap();
    assert_eq!(raised.len() as i64, MAX_HEALTH_WAKES_PER_DAY);
    assert_eq!(
        notices(&org, "alpha-ops").await.len() as i64,
        MAX_HEALTH_WAKES_PER_DAY
    );
    // Over the bound waits; it is not dropped. A day later the rest are raised.
    assert!(org
        .raise_health_problems_at(now + Duration::hours(1), &flood, rhythm)
        .await
        .unwrap()
        .is_empty());
    let rest = org
        .raise_health_problems_at(now + Duration::hours(25), &flood, rhythm)
        .await
        .unwrap();
    assert_eq!(rest.len(), 3);

    // A lead who has gone, or a person, is not sent a notice: Exec answers for it.
    let orphan = [HealthProblem {
        kind: "review-loop",
        key: "review-loop:test:orphan".into(),
        to: "someone-who-left".into(),
        work_id: None,
        detail: "An orphaned problem.".into(),
    }];
    let raised = org
        .raise_health_problems_at(now + Duration::hours(50), &orphan, rhythm)
        .await
        .unwrap();
    assert_eq!(raised.len(), 1);
    assert_eq!(raised[0].to, "exec");
    assert_eq!(notices(&org, "exec").await.len(), 1);
    assert!(org
        .health_problem_raised("review-loop:test:orphan")
        .await
        .unwrap());
    org.drop_schema().await.unwrap();
}
