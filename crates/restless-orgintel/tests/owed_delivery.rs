//! What an accountable actor is still owed (S19-T1).
//!
//! The owner reported that finished work "never shows up in the exec chat". The
//! mechanism was that owed work was inferred from a global wake watermark
//! rather than from the owed thing: one unrelated Exec wake moved
//! `latest_event_at("wake")` past a pending judgement and it never triggered a
//! wake again, and a lead's message to the Exec had no durable recovery path at
//! all. Since Exec is the only actor that can put an ordinary judgement in
//! front of the owner, a lost trigger here is a lost owner attention item.
//!
//! These are behavioural assertions about who is still owed what, not schema
//! assertions. Each one would pass trivially against the old watermark for the
//! happy path, so every case here is one the watermark actually got wrong.
//! Runs only when RESTLESS_TEST_DATABASE_URL is set.

use restless_orgintel::{
    NewOwnerHandoff, NewWork, OrgIntel, OwnerHandoffCategory, OwnerHandoffState, WorkspaceSpec,
};

async fn company(prefix: &str) -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!("{prefix}{}", std::process::id());
    let org = OrgIntel::ensure(&url, &name).await.expect("ensure schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    Some(org)
}

async fn work_for(org: &OrgIntel, owner: &str) -> uuid::Uuid {
    org.add_work(NewWork {
        owner_id: owner,
        title: "a bounded outcome",
        outcome: "an outcome that reaches a judgement",
        goal_id: None,
        priority: 0,
        expected_artifact: "",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(1),
    })
    .await
    .unwrap()
}

async fn judgement_from(org: &OrgIntel, by: &str, work: uuid::Uuid) -> uuid::Uuid {
    org.request_owner_handoff(NewOwnerHandoff {
        work_id: work,
        attempt_id: None,
        requested_by: by,
        category: OwnerHandoffCategory::OwnerJudgement,
        requested_action: "decide whether the prepared outcome ships",
        prepared_state: "the candidate is prepared and live-probed",
        resume_condition: "a decision is recorded",
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn accountable_lead_can_reply_to_owner_for_staff_work() {
    let Some(org) = company("leadreply").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping lead-reply scenario");
        return;
    };
    org.ensure_actor("game-product", "staff", "lead", "Game lead")
        .await
        .unwrap();
    org.ensure_actor("gameplay-build", "staff", "builder", "Gameplay builder")
        .await
        .unwrap();
    let team = org
        .create_team(
            "Game product",
            "own the playable result",
            "game-product",
            "exec",
        )
        .await
        .unwrap();
    org.set_actor_team(
        "gameplay-build",
        Some(team),
        "exec",
        "the builder owns implementation under the accountable lead",
    )
    .await
    .unwrap();
    let work = work_for(&org, "gameplay-build").await;

    let message = org
        .send_work_message_to_owner(
            "game-product",
            work,
            "The team repaired the route-zero rejection and the exact candidate is ready.",
        )
        .await
        .expect("the accountable lead speaks for Staff-owned Work");
    assert_eq!(org.message_work_id(message).await.unwrap(), Some(work));
    assert!(org
        .send_work_message_to_owner("exec", work, "I am not this Work's accountable lead.")
        .await
        .is_err());
}

/// The exact reported failure. A judgement sits assigned to the Exec; Exec is
/// given it once; it stays pending because Exec did not settle it that turn.
/// Under the watermark it never triggered a wake again. It must stay owed until
/// a turn has actually carried it, and stop being a trigger once one has.
#[tokio::test]
async fn a_judgement_stays_owed_until_a_turn_actually_carried_it() {
    let Some(org) = company("owed").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping owed-delivery scenario");
        return;
    };
    org.ensure_actor("offer-strategy", "staff", "lead", "Offer lead")
        .await
        .unwrap();
    let work = work_for(&org, "offer-strategy").await;
    let handoff = judgement_from(&org, "exec", work).await;

    assert_eq!(
        org.undelivered_handoff_count("exec").await.unwrap(),
        1,
        "a new judgement assigned to the Exec is owed to it"
    );

    // Unrelated Exec turns happen. Under the watermark each of these silenced
    // the handoff permanently; none of them carried it.
    for _ in 0..3 {
        org.emit_event(
            "wake",
            Some("exec"),
            serde_json::json!({ "reason": "owner chat" }),
        )
        .await
        .unwrap();
        org.emit_event("wake_end", Some("exec"), serde_json::json!({}))
            .await
            .unwrap();
    }
    assert_eq!(
        org.undelivered_handoff_count("exec").await.unwrap(),
        1,
        "unrelated wakes must not consume a judgement they never carried"
    );

    // A turn that actually carried it completes.
    assert_eq!(org.mark_handoffs_delivered(&[handoff]).await.unwrap(), 1);
    assert_eq!(
        org.undelivered_handoff_count("exec").await.unwrap(),
        0,
        "a carried judgement stops being a wake trigger"
    );
    assert_eq!(
        org.mark_handoffs_delivered(&[handoff]).await.unwrap(),
        0,
        "re-delivery is a no-op, so the first delivery time stays truthful"
    );

    // Delivery gates the trigger, never the context: it is still pending and
    // still in the assignee's queue.
    let still_owed = org.handoffs_assigned_to("exec").await.unwrap();
    assert_eq!(still_owed.len(), 1);
    assert_eq!(still_owed[0].state, OwnerHandoffState::Pending);
    assert!(still_owed[0].delivered_at.is_some());
}

/// A judgement whose assignee or prepared meaning changes is a different thing
/// to be given, so it becomes owed again to whoever now holds it.
#[tokio::test]
async fn reassigning_or_refreshing_a_judgement_makes_it_owed_again() {
    let Some(org) = company("reowed").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping re-owed scenario");
        return;
    };
    org.ensure_actor("offer-strategy", "staff", "lead", "Offer lead")
        .await
        .unwrap();
    org.ensure_actor("offer-copy", "staff", "copywriter", "Writer")
        .await
        .unwrap();
    let team = org
        .create_team("Centre offer", "own the offer", "offer-strategy", "exec")
        .await
        .unwrap();
    org.set_actor_team("offer-copy", Some(team), "exec", "the offer needs copy")
        .await
        .unwrap();

    let work = work_for(&org, "offer-copy").await;
    let handoff = judgement_from(&org, "offer-copy", work).await;

    // A worker's judgement goes to its lead first, not straight to the Exec.
    assert_eq!(
        org.undelivered_handoff_count("offer-strategy")
            .await
            .unwrap(),
        1
    );
    assert_eq!(org.undelivered_handoff_count("exec").await.unwrap(), 0);

    org.mark_handoffs_delivered(&[handoff]).await.unwrap();
    assert_eq!(
        org.undelivered_handoff_count("offer-strategy")
            .await
            .unwrap(),
        0
    );

    // The lead cannot settle it and passes it up. That is a new thing for the
    // Exec to be given.
    org.escalate_handoff(handoff, "offer-strategy", "outside this team's charter")
        .await
        .unwrap();
    assert_eq!(
        org.undelivered_handoff_count("exec").await.unwrap(),
        1,
        "an escalated judgement is owed to its new assignee"
    );

    org.mark_handoffs_delivered(&[handoff]).await.unwrap();
    assert_eq!(org.undelivered_handoff_count("exec").await.unwrap(), 0);

    // The prepared outcome changes underneath the pending request.
    org.refresh_owner_handoff(
        handoff,
        "offer-copy",
        "decide whether the corrected outcome ships",
        "the candidate was rebuilt after owner feedback",
        "a decision is recorded",
    )
    .await
    .unwrap();
    assert_eq!(
        org.undelivered_handoff_count("exec").await.unwrap(),
        1,
        "a changed prepared meaning is owed again"
    );
}

/// A lead reporting a prepared outcome to the Exec is the case the old
/// owner-only recovery filter dropped entirely. Unread mail is owed regardless
/// of sender; an Exec note to itself is not a second wake; and Work-linked
/// feedback belongs to the Work Attempt rather than a free-form conversation.
#[tokio::test]
async fn unread_mail_is_owed_by_sender_independent_rules() {
    let Some(org) = company("owedmail").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping owed-mail scenario");
        return;
    };
    org.ensure_actor("offer-strategy", "staff", "lead", "Offer lead")
        .await
        .unwrap();

    assert_eq!(org.owed_conversation_count("exec").await.unwrap(), 0);

    let self_note = org
        .send_message("exec", Some("exec"), "a note to myself")
        .await
        .unwrap();
    assert_eq!(
        org.owed_conversation_count("exec").await.unwrap(),
        0,
        "an Exec note to itself is transcript, not owed work"
    );

    let from_lead = org
        .send_message(
            "offer-strategy",
            Some("exec"),
            "the outcome is prepared for review",
        )
        .await
        .unwrap();
    assert_eq!(
        org.owed_conversation_count("exec").await.unwrap(),
        1,
        "a lead's report to the Exec is owed work"
    );

    // Unrelated wakes cannot consume it; only reading it does.
    org.emit_event(
        "wake",
        Some("exec"),
        serde_json::json!({ "reason": "schedule" }),
    )
    .await
    .unwrap();
    org.emit_event("wake_end", Some("exec"), serde_json::json!({}))
        .await
        .unwrap();
    assert_eq!(
        org.owed_conversation_count("exec").await.unwrap(),
        1,
        "a wake that never carried the message must not consume it"
    );

    org.mark_read(from_lead).await.unwrap();
    assert_eq!(org.owed_conversation_count("exec").await.unwrap(), 0);
    org.mark_read(self_note).await.unwrap();
}
