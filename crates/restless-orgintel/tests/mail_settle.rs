//! Fewer wakes (S110): machine mail settles before it owes a turn, a person's message never
//! waits, and a lead whose Work is commissioned with its appointment is not woken to read it.

use restless_orgintel::{MailSettle, NewWork, OrgIntel, ProducingTopology, WorkspaceSpec};

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
};

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
