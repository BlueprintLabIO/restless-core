//! Recipient discovery for durable peer conversation wakes.
//!
//! This exercises the real Postgres message rules. It is intentionally small:
//! the daemon scheduler consumes this query, while OrgIntel remains the owner
//! of which unread messages genuinely owe a conversation turn.

use restless_orgintel::{NewWork, OrgIntel, WorkspaceSpec};

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "peerwake{}_test",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    );
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    for (id, kind, role) in [
        ("owner", "owner", "owner"),
        ("exec", "exec", "exec"),
        ("alpha-build", "staff", "builder"),
        ("beta-research", "staff", "researcher"),
        ("gamma-review", "staff", "reviewer"),
    ] {
        org.ensure_actor(id, kind, role, id).await.unwrap();
    }
    Some(org)
}

#[tokio::test]
async fn ordinary_peer_mail_discovers_only_active_recipients_until_delivery() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping peer conversation wake scenario");
        return;
    };

    let beta_message = org
        .send_message(
            "alpha-build",
            Some("beta-research"),
            "Which API contract did your investigation confirm?",
        )
        .await
        .unwrap();
    org.send_message(
        "beta-research",
        Some("gamma-review"),
        "Please check the evidence before I answer.",
    )
    .await
    .unwrap();
    org.send_message(
        "alpha-build",
        Some("alpha-build"),
        "A private note is not an owed peer turn.",
    )
    .await
    .unwrap();
    org.send_message(
        "alpha-build",
        Some("exec"),
        "Executive mail has its own wake path.",
    )
    .await
    .unwrap();

    assert_eq!(
        org.actors_owing_conversation_mail(128).await.unwrap(),
        ["beta-research", "gamma-review"]
    );
    assert_eq!(
        org.actors_owing_conversation_mail(1).await.unwrap(),
        ["beta-research"],
        "recipient discovery has a deterministic bounded scan"
    );

    org.mark_read(beta_message).await.unwrap();
    assert_eq!(
        org.actors_owing_conversation_mail(128).await.unwrap(),
        ["gamma-review"],
        "a delivered peer message stops triggering repair wakes"
    );

    org.retire_actor(
        "gamma-review",
        "exec",
        "the reviewer left after the message arrived",
    )
    .await
    .unwrap();
    assert!(org
        .actors_owing_conversation_mail(128)
        .await
        .unwrap()
        .is_empty());

    let reply = org
        .send_message(
            "beta-research",
            Some("alpha-build"),
            "The confirmed API contract is v2.",
        )
        .await
        .unwrap();
    let context = org
        .direct_conversation_before("alpha-build", reply, 6)
        .await
        .unwrap();
    assert!(context.iter().any(|message| message.id == beta_message));
    assert!(
        org.direct_conversation_before("gamma-review", reply, 6)
            .await
            .unwrap()
            .is_empty(),
        "a different actor cannot read this direct conversation"
    );
    assert_eq!(
        org.actors_owing_conversation_mail(128).await.unwrap(),
        ["alpha-build"],
    );
    org.mark_read(reply).await.unwrap();
    assert!(org
        .actors_owing_conversation_mail(128)
        .await
        .unwrap()
        .is_empty());

    let work = org
        .add_work(NewWork {
            owner_id: "alpha-build",
            title: "Build the adapter",
            outcome: "An adapter using the confirmed API contract",
            goal_id: None,
            priority: 1,
            expected_artifact: "adapter",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let attempt = org
        .claim_ready_work("peer checkpoint")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(attempt.work.id, work);
    let peer_during_work = org
        .send_message(
            "beta-research",
            Some("alpha-build"),
            "One correction: v2 requires an idempotency key.",
        )
        .await
        .unwrap();
    let delivered = org
        .checkpoint_attempt_feedback(attempt.attempt_id)
        .await
        .unwrap();
    assert_eq!(
        delivered
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        [peer_during_work]
    );
    assert!(
        org.checkpoint_attempt_feedback(attempt.attempt_id)
            .await
            .unwrap()
            .is_empty(),
        "the same peer message is delivered once to the running Attempt"
    );
    assert!(
        org.actors_owing_conversation_mail(128)
            .await
            .unwrap()
            .is_empty(),
        "checkpoint delivery clears the separate idle wake"
    );
    let work_feedback = org
        .send_work_message(
            "owner",
            "alpha-build",
            work,
            "Keep the adapter compatible with the current client.",
        )
        .await
        .unwrap();
    let delivered = org
        .checkpoint_attempt_feedback(attempt.attempt_id)
        .await
        .unwrap();
    assert_eq!(
        delivered
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        [work_feedback],
        "peer delivery must not advance the Work feedback cursor"
    );

    org.drop_schema().await.unwrap();
}
