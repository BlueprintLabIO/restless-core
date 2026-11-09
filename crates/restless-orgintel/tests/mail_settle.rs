//! Fewer wakes (S110): machine mail settles before it owes a turn, a person's message never
//! waits, and a lead whose Work is commissioned with its appointment is not woken to read it.
//! It settles only while something could still add to it (`has_work_in_motion`).

use restless_orgintel::{
    MailSettle, NewWork, OrgIntel, ProducingTopology, WorkAttemptState, WorkspaceSpec,
};

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "settle{}_test",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    );
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    for (id, kind, role) in [
        ("owner", "owner", "owner"),
        ("exec", "exec", "exec"),
        ("daemon", "system", "system-sender"),
        ("bakery-web", "staff", "lead"),
    ] {
        org.ensure_actor(id, kind, role, id).await.unwrap();
    }
    Some(org)
}

const SETTLE: MailSettle = MailSettle {
    quiet_secs: 1,
    max_secs: 30,
    only_while_in_motion: false,
};

/// As the scheduler settles mail (S113): only while something could add to the recipient's wake.
const SCOPED: MailSettle = MailSettle {
    quiet_secs: 30,
    max_secs: 60,
    only_while_in_motion: true,
};

fn work<'a>(owner: &'a str, title: &'a str) -> NewWork<'a> {
    NewWork {
        owner_id: owner,
        title,
        outcome: title,
        goal_id: None,
        priority: 1,
        expected_artifact: "note",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(1),
    }
}

#[tokio::test]
async fn a_result_waits_only_while_more_can_join_it() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping mail settle scenario");
        return;
    };
    org.send_message("daemon", Some("exec"), "Completed Work result A")
        .await
        .unwrap();
    // Nothing else can add to Exec's wake: owed at once, although the window has not passed.
    assert_eq!(
        org.owed_conversation_count_settled("exec", SCOPED)
            .await
            .unwrap(),
        1
    );
    // Ready Work's result would land in the same turn, so now it waits for it.
    org.add_work(work("bakery-web", "Next piece"))
        .await
        .unwrap();
    assert_eq!(
        org.owed_conversation_count_settled("exec", SCOPED)
            .await
            .unwrap(),
        0
    );
    // The bound: however long Work stays in motion, the oldest result is not held past max_secs.
    let bound = MailSettle {
        quiet_secs: 30,
        max_secs: 1,
        only_while_in_motion: true,
    };
    tokio::time::sleep(std::time::Duration::from_millis(1_300)).await;
    assert_eq!(
        org.owed_conversation_count_settled("exec", bound)
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn another_teams_work_does_not_hold_a_leads_mail() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping mail settle scenario");
        return;
    };
    org.ensure_actor("market-writer", "staff", "lead", "market-writer")
        .await
        .unwrap();
    org.add_work(work("market-writer", "Unrelated"))
        .await
        .unwrap();
    org.send_message("daemon", Some("bakery-web"), "A note for the lead")
        .await
        .unwrap();
    assert_eq!(
        org.actors_owing_conversation_mail_settled(128, SCOPED)
            .await
            .unwrap(),
        ["bakery-web"],
        "the other team's Work cannot add to this lead's wake"
    );
    // Work this lead commissioned can: its mail waits for that result.
    org.add_commissioned_work_with_edges_and_gates(
        work("market-writer", "Commissioned by the lead"),
        &[],
        &[],
        &[],
        false,
        None,
        "bakery-web",
        ProducingTopology::default(),
    )
    .await
    .unwrap();
    assert!(org
        .actors_owing_conversation_mail_settled(128, SCOPED)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn result_notices_landing_together_owe_one_settled_wake() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping mail settle scenario");
        return;
    };
    org.send_message("daemon", Some("exec"), "Completed Work result A")
        .await
        .unwrap();
    org.send_message("daemon", Some("exec"), "Completed Work result B")
        .await
        .unwrap();
    // Still settling: nothing owed yet, and the scheduler knows when to look again.
    assert_eq!(
        org.owed_conversation_count_settled("exec", SETTLE)
            .await
            .unwrap(),
        0
    );
    assert!(org.next_mail_settles_at(SETTLE).await.unwrap().is_some());
    // Without settling (as before) both are owed at once.
    assert_eq!(org.owed_conversation_count("exec").await.unwrap(), 2);

    tokio::time::sleep(std::time::Duration::from_millis(1_300)).await;
    // Settled: both owed together, one wake carries them, and nothing is left to time.
    assert_eq!(
        org.owed_conversation_count_settled("exec", SETTLE)
            .await
            .unwrap(),
        2
    );
    assert!(org.next_mail_settles_at(SETTLE).await.unwrap().is_none());
}

#[tokio::test]
async fn a_persons_message_never_waits_and_carries_settling_mail_with_it() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping mail settle scenario");
        return;
    };
    org.send_message("daemon", Some("exec"), "Completed Work result")
        .await
        .unwrap();
    org.send_message("owner", Some("exec"), "How is the site going?")
        .await
        .unwrap();
    assert_eq!(
        org.owed_conversation_count_settled("exec", SETTLE)
            .await
            .unwrap(),
        2
    );
    org.send_message("owner", Some("bakery-web"), "Use the blue logo")
        .await
        .unwrap();
    assert_eq!(
        org.actors_owing_conversation_mail_settled(128, SETTLE)
            .await
            .unwrap(),
        ["bakery-web"]
    );
}

#[tokio::test]
async fn a_commissioned_lead_is_not_woken_to_read_its_appointment() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping mail settle scenario");
        return;
    };
    org.create_team("Site", "Build the bakery site", "bakery-web", "exec")
        .await
        .unwrap();
    // The appointment is machine mail: it settles before it wakes the lead.
    assert_eq!(org.owed_conversation_count("bakery-web").await.unwrap(), 1);
    assert!(org
        .actors_owing_conversation_mail_settled(128, SETTLE)
        .await
        .unwrap()
        .is_empty());
    // `restless commission` adds the lead's Work right after; the Work carries the charter.
    org.add_commissioned_work_with_edges_and_gates(
        NewWork {
            owner_id: "bakery-web",
            title: "Bakery site",
            outcome: "A one-page site with every brief fact",
            goal_id: None,
            priority: 1,
            expected_artifact: "site",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        },
        &[],
        &[],
        &[],
        false,
        None,
        "exec",
        ProducingTopology::default(),
    )
    .await
    .unwrap();
    assert_eq!(org.owed_conversation_count("bakery-web").await.unwrap(), 0);
    tokio::time::sleep(std::time::Duration::from_millis(1_300)).await;
    assert!(org
        .actors_owing_conversation_mail_settled(128, SETTLE)
        .await
        .unwrap()
        .is_empty());
}

/// The scheduler waits for mail to settle only while something in motion could add to it: ready
/// Work, a running Attempt, or a turn holding its cognitive lease. Once all of that is over, the
/// company is idle and settled mail is owed at once.
#[tokio::test]
async fn mail_waits_only_while_something_is_in_motion() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping mail settle scenario");
        return;
    };
    assert!(
        !org.has_work_in_motion().await.unwrap(),
        "a new company is idle"
    );
    org.add_work(NewWork {
        owner_id: "bakery-web",
        title: "Bakery site",
        outcome: "A one-page site",
        goal_id: None,
        priority: 1,
        expected_artifact: "site",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(1),
    })
    .await
    .unwrap();
    assert!(
        org.has_work_in_motion().await.unwrap(),
        "ready Work starts and reports"
    );
    let claimed = org
        .claim_ready_work("ready")
        .await
        .unwrap()
        .expect("the Work is ready");
    assert!(
        org.has_work_in_motion().await.unwrap(),
        "a running Attempt reports"
    );
    org.finish_work_attempt(claimed.attempt_id, WorkAttemptState::Failed, "stopped")
        .await
        .unwrap();
    assert!(
        !org.has_work_in_motion().await.unwrap(),
        "its only Attempt is spent: nothing else is coming"
    );
    let lease = org
        .claim_actor_cognitive_session("bakery-web", std::time::Duration::from_secs(60))
        .await
        .unwrap()
        .expect("the lead is free");
    assert!(
        org.has_work_in_motion().await.unwrap(),
        "a turn in flight may write"
    );
    org.release_actor_cognitive_session(&lease).await.unwrap();
    assert!(!org.has_work_in_motion().await.unwrap());
}
