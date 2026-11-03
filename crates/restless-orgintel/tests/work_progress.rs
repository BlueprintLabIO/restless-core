//! Rough progress estimates on Work (owner decision, 3 November 2026): the Work's creator is the
//! source of truth, Exec may also record one, done Work is 100%, abandoned Work keeps its last
//! estimate, and big moves post one compact line in the creator's chat with the owner.

use restless_orgintel::{
    goal_progress, NewWork, OrgIntel, ProgressReport, WorkAttemptState, WorkspaceSpec,
};

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "workprogress{}_test",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    );
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("owner", "owner", "owner", "The owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.create_actor(
        "print-direction",
        "lead",
        "Print lead",
        Some("openai-codex/gpt-5.6-sol"),
        "exec",
        "owns the print run",
    )
    .await
    .unwrap();
    let team = org
        .create_team("Print", "Ship the magnets", "print-direction", "exec")
        .await
        .unwrap();
    org.create_actor(
        "print-design",
        "staff",
        "Print designer",
        Some("openai-codex/gpt-5.6-sol"),
        "print-direction",
        "lays out the files",
    )
    .await
    .unwrap();
    org.set_actor_team("print-design", Some(team), "print-direction", "lays out")
        .await
        .unwrap();
    Some(org)
}

fn work<'a>(title: &'a str, goal_id: Option<uuid::Uuid>) -> NewWork<'a> {
    NewWork {
        owner_id: "print-design",
        title,
        outcome: "print-ready files",
        goal_id,
        priority: 1,
        expected_artifact: "",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(2),
    }
}

fn estimate(percent: i16, next: &str) -> ProgressReport<'_> {
    ProgressReport::Estimate { percent, next }
}

async fn owner_thread(org: &OrgIntel, actor: &str) -> Vec<String> {
    org.human_conversation("owner", actor, 50)
        .await
        .unwrap()
        .into_iter()
        .map(|message| message.body)
        .collect()
}

#[tokio::test]
async fn the_creator_sets_estimates_others_cannot_and_history_is_kept() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping work progress scenario");
        return;
    };
    let id = org.add_work(work("Magnets", None)).await.unwrap();
    let row = org.get_work(id).await.unwrap().unwrap();
    assert_eq!(row.commissioned_by, "print-direction");
    assert!(
        org.work_progress(Some(id)).await.unwrap().is_empty(),
        "no estimate yet means nothing, not 0%"
    );

    // The producer is not the source of truth, nor is anyone else.
    let refused = org
        .record_work_progress(id, "print-design", estimate(50, "export print files"))
        .await
        .unwrap_err()
        .to_string();
    assert!(refused.contains("only the Work's creator"), "{refused}");
    assert!(org
        .record_work_progress(id, "owner", estimate(50, "x"))
        .await
        .is_err());

    // Rough on purpose: rounded to tens, and 100 waits for done.
    let first = org
        .record_work_progress(id, "print-direction", estimate(14, "sketch the layout"))
        .await
        .unwrap();
    assert_eq!(first.progress.percent, Some(10));
    assert_eq!(first.progress.estimated_by, "print-direction");
    assert!(org
        .record_work_progress(id, "print-direction", estimate(97, "x"))
        .await
        .is_err());
    assert!(org
        .record_work_progress(id, "print-direction", estimate(40, "  "))
        .await
        .is_err());

    // Exec may record one for any Work.
    org.record_work_progress(id, "exec", estimate(40, "export print files"))
        .await
        .unwrap();
    // Blocked drops the number and says why.
    let blocked = org
        .record_work_progress(
            id,
            "print-direction",
            ProgressReport::Blocked {
                why: "waiting on the printer's bleed spec",
            },
        )
        .await
        .unwrap();
    assert_eq!(blocked.progress.percent, None);
    assert_eq!(blocked.progress.state, "blocked");
    org.record_work_progress(id, "print-direction", estimate(60, "export print files"))
        .await
        .unwrap();

    let history = org.work_progress(Some(id)).await.unwrap();
    assert_eq!(
        history
            .iter()
            .map(|row| (row.state.as_str(), row.percent))
            .collect::<Vec<_>>(),
        vec![
            ("estimate", Some(10)),
            ("estimate", Some(40)),
            ("blocked", None),
            ("estimate", Some(60)),
        ],
        "oldest first; the newest is current"
    );
    assert_eq!(history[1].estimated_by, "exec");

    // History stays short in projections.
    for step in 0..12 {
        org.record_work_progress(id, "print-direction", estimate(60 + (step % 2) * 10, "x"))
            .await
            .unwrap();
    }
    let snapshot = org.work_graph_snapshot().await.unwrap();
    let carried = snapshot
        .progress
        .iter()
        .filter(|row| row.work_id == id)
        .count();
    assert_eq!(carried as i64, restless_orgintel::PROGRESS_HISTORY);
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn big_moves_post_one_quiet_line_in_the_creators_thread() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping work progress chat scenario");
        return;
    };
    let id = org.add_work(work("Magnets", None)).await.unwrap();
    let lead_inbox = org.inbox(Some("print-direction")).await.unwrap().len();
    // The lead asked the owner something; a progress line must not bury that question.
    org.send_message("print-direction", None, "Matte or gloss?")
        .await
        .unwrap();

    let small = org
        .record_work_progress(id, "print-direction", estimate(10, "sketch"))
        .await
        .unwrap();
    assert_eq!(small.posted_message_id, None, "10 points is not news");
    let big = org
        .record_work_progress(id, "print-direction", estimate(60, "export print files"))
        .await
        .unwrap();
    assert!(big.posted_message_id.is_some());
    let wobble = org
        .record_work_progress(id, "print-direction", estimate(70, "export print files"))
        .await
        .unwrap();
    assert_eq!(
        wobble.posted_message_id, None,
        "measured from the last line posted"
    );
    // Exec's estimate still speaks in the creator's thread, not Exec's.
    let exec = org
        .record_work_progress(id, "exec", estimate(80, "proof the colours"))
        .await
        .unwrap();
    assert!(exec.posted_message_id.is_some());

    let thread = owner_thread(&org, "print-direction").await;
    assert_eq!(
        thread,
        vec![
            "Matte or gloss?".to_string(),
            "Magnets: ~60% (next: export print files)".to_string(),
            "Magnets: ~80% (next: proof the colours)".to_string(),
        ]
    );
    assert!(owner_thread(&org, "exec").await.is_empty());

    // The lines are already read and are never the agent's open word to the owner.
    let latest = org
        .latest_human_agent_messages("owner", &["print-direction".to_string()])
        .await
        .unwrap();
    assert_eq!(latest.len(), 1);
    assert_eq!(latest[0].body, "Matte or gloss?");
    let posted = org
        .human_conversation("owner", "print-direction", 50)
        .await
        .unwrap();
    assert!(posted[1].read_at.is_some() && posted[2].read_at.is_some());
    assert_eq!(
        org.inbox(Some("print-direction")).await.unwrap().len(),
        lead_inbox,
        "a progress line wakes nobody"
    );
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn done_is_100_and_abandoned_keeps_its_last_estimate() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping work progress settlement scenario");
        return;
    };
    let told = org.add_work(work("Magnets", None)).await.unwrap();
    org.record_work_progress(told, "print-direction", estimate(60, "export print files"))
        .await
        .unwrap();
    let claimed = org.claim_ready_work("test runtime").await.unwrap().unwrap();
    assert_eq!(claimed.work.id, told);
    // Checkpoint passed: the creator owes an estimate until it records one.
    org.finish_work_attempt(
        claimed.attempt_id,
        WorkAttemptState::Produced,
        "files exported",
    )
    .await
    .unwrap();
    let done = org.work_progress(Some(told)).await.unwrap();
    let last = done.last().unwrap();
    assert_eq!((last.state.as_str(), last.percent), ("done", Some(100)));
    assert_eq!(
        owner_thread(&org, "print-direction").await.last().unwrap(),
        "Magnets: done (100%)"
    );
    assert!(org
        .record_work_progress(told, "print-direction", estimate(50, "x"))
        .await
        .is_err());

    // Quiet Work completes quietly: no line when the owner was never told.
    let quiet = org.add_work(work("Stickers", None)).await.unwrap();
    let claimed = org.claim_ready_work("test runtime").await.unwrap().unwrap();
    assert_eq!(claimed.work.id, quiet);
    org.finish_work_attempt(claimed.attempt_id, WorkAttemptState::Produced, "printed")
        .await
        .unwrap();
    assert_eq!(
        org.work_progress(Some(quiet)).await.unwrap()[0].percent,
        Some(100)
    );
    assert_eq!(owner_thread(&org, "print-direction").await.len(), 2);

    let stopped = org.add_work(work("Posters", None)).await.unwrap();
    org.record_work_progress(stopped, "print-direction", estimate(30, "pick paper"))
        .await
        .unwrap();
    org.abandon_work(stopped, "exec", "the event moved online")
        .await
        .unwrap();
    let kept = org.work_progress(Some(stopped)).await.unwrap();
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].percent, Some(30));
    assert!(org
        .record_work_progress(stopped, "print-direction", estimate(40, "x"))
        .await
        .is_err());
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn checkpoints_owe_the_creator_an_estimate() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping work progress owed scenario");
        return;
    };
    let id = org.add_work(work("Magnets", None)).await.unwrap();
    assert!(
        org.work_progress_owed("print-direction")
            .await
            .unwrap()
            .is_empty(),
        "no checkpoint yet"
    );
    let claimed = org.claim_ready_work("test runtime").await.unwrap().unwrap();
    org.finish_work_attempt(
        claimed.attempt_id,
        WorkAttemptState::Blocked,
        "needs the bleed spec",
    )
    .await
    .unwrap();
    let owed = org.work_progress_owed("print-direction").await.unwrap();
    assert_eq!(owed.len(), 1);
    assert_eq!(owed[0].work_id, id);
    assert!(
        org.work_progress_owed("exec").await.unwrap().is_empty(),
        "owed to the creator only"
    );
    org.record_work_progress(
        id,
        "print-direction",
        ProgressReport::Blocked {
            why: "needs the bleed spec",
        },
    )
    .await
    .unwrap();
    assert!(org
        .work_progress_owed("print-direction")
        .await
        .unwrap()
        .is_empty());
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn a_goal_rolls_up_its_parts_equally() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping goal roll-up scenario");
        return;
    };
    let goal = org
        .add_goal("Market stall", "Sell at Saturday's market", "exec")
        .await
        .unwrap();
    let nobody = org.add_goal("Website", "", "exec").await.unwrap();
    let magnets = org.add_work(work("Magnets", Some(goal))).await.unwrap();
    let posters = org.add_work(work("Posters", Some(goal))).await.unwrap();
    let banner = org.add_work(work("Banner", Some(goal))).await.unwrap();
    let stopped = org.add_work(work("Flyers", Some(goal))).await.unwrap();
    org.add_work(work("Landing page", Some(nobody)))
        .await
        .unwrap();

    let roll = |org: &OrgIntel, goal| {
        let org = org.clone();
        async move {
            let snapshot = org.work_graph_snapshot().await.unwrap();
            goal_progress(goal, &snapshot.work, &snapshot.progress)
        }
    };
    assert_eq!(
        roll(&org, goal).await.percent,
        None,
        "nothing estimated: nothing shown"
    );
    assert_eq!(roll(&org, nobody).await.percent, None);

    org.record_work_progress(posters, "print-direction", estimate(40, "pick paper"))
        .await
        .unwrap();
    org.record_work_progress(banner, "print-direction", estimate(20, "measure"))
        .await
        .unwrap();
    org.record_work_progress(
        banner,
        "print-direction",
        ProgressReport::Blocked { why: "no van" },
    )
    .await
    .unwrap();
    org.abandon_work(stopped, "exec", "not needed")
        .await
        .unwrap();
    let claimed = org.claim_ready_work("test runtime").await.unwrap().unwrap();
    let completed = claimed.work.id;
    org.finish_work_attempt(claimed.attempt_id, WorkAttemptState::Produced, "done")
        .await
        .unwrap();
    assert!([magnets, posters, banner].contains(&completed));

    let rolled = roll(&org, goal).await;
    assert_eq!(rolled.total, 3, "stopped Work is left out");
    assert_eq!(rolled.done, 1);
    // One part done (100); of the others, the estimated ones count as given (a blocked one keeps
    // its last number) and an unestimated one counts as 0.
    let expected = match completed {
        id if id == magnets => (100 + 40 + 20) as f64 / 3.0,
        id if id == posters => (0 + 100 + 20) as f64 / 3.0,
        _ => (0 + 40 + 100) as f64 / 3.0,
    };
    assert_eq!(rolled.percent, Some(expected.round() as i16));
    org.drop_schema().await.unwrap();
}
