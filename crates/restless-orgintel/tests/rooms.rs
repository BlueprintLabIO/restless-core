//! Human Rooms remain one projection over the existing message truth.
//! These scenarios run against a scratch company schema when Postgres is
//! available, like the rest of the OrgIntel behavioural suite.

use restless_orgintel::{
    NewOwnerHandoff, NewRoomMessageMention, NewWork, OrgIntel, OutcomeStandard,
    OwnerHandoffCategory, OwnerHandoffState, RoomKind, WorkAttemptState, WorkspaceSpec,
};
use sha2::{Digest as _, Sha256};
use sqlx::postgres::{PgConnection, PgListener};
use sqlx::Connection as _;
use std::time::Duration;

async fn company(prefix: &str) -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!("{prefix}{}_test", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("delivery-build", "staff", "builder", "Builder")
        .await
        .unwrap();
    Some(org)
}

async fn handoff_input(
    org: &OrgIntel,
    handoff_id: uuid::Uuid,
) -> restless_orgintel::OwnerHandoffInput {
    org.list_owner_handoffs()
        .await
        .unwrap()
        .into_iter()
        .find(|handoff| handoff.id == handoff_id)
        .expect("handoff exists")
        .conversation_input()
}

#[tokio::test]
async fn room_send_is_idempotent_threaded_paginated_and_transactional() {
    let Some(org) = company("rooms").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Rooms scenario");
        return;
    };

    let room = org
        .ensure_company_room("owner", &["owner", "exec"], "company-room-initial")
        .await
        .unwrap();
    let same_room = org
        .ensure_company_room("owner", &["owner", "exec"], "company-room-retry")
        .await
        .unwrap();
    assert_eq!(room.id, same_room.id, "the company Room is a singleton");
    assert_eq!(room.kind, RoomKind::Company);

    let root = org
        .send_room_message(room.id, "owner", "First question", None, "command-1")
        .await
        .unwrap();
    assert!(root.created);
    assert_eq!(root.message.effective_thread_root_id(), root.message.id);
    let retry = org
        .send_room_message(room.id, "owner", "First question", None, "command-1")
        .await
        .unwrap();
    assert!(!retry.created);
    assert_eq!(retry.message.id, root.message.id);
    assert_eq!(retry.event_id, root.event_id);

    let standard_message = org
        .send_room_message_with_standard(
            room.id,
            "owner",
            "Quality matters",
            None,
            "standard-command",
            Some(OutcomeStandard::Exceptional),
        )
        .await
        .unwrap();
    let standard_conflict = org
        .send_room_message_with_standard(
            room.id,
            "owner",
            "Quality matters",
            None,
            "standard-command",
            Some(OutcomeStandard::Fast),
        )
        .await
        .unwrap_err();
    assert!(standard_conflict.to_string().contains("conflicts"));
    assert_eq!(
        standard_message.message.outcome_standard,
        Some(OutcomeStandard::Exceptional)
    );

    let conflict = org
        .send_room_message(room.id, "owner", "A different payload", None, "command-1")
        .await
        .unwrap_err();
    assert!(conflict.to_string().contains("conflicts"));

    let second_root = org
        .send_room_message(room.id, "exec", "Second question", None, "command-2")
        .await
        .unwrap();
    let reply = org
        .send_room_message(
            room.id,
            "exec",
            "A focused answer",
            Some(root.message.id),
            "command-3",
        )
        .await
        .unwrap();
    let nested = org
        .send_room_message(
            room.id,
            "owner",
            "One clarification",
            Some(reply.message.id),
            "command-4",
        )
        .await
        .unwrap();
    assert_eq!(reply.message.parent_message_id, Some(root.message.id));
    assert_eq!(reply.message.thread_root_message_id, Some(root.message.id));
    assert_eq!(nested.message.parent_message_id, Some(reply.message.id));
    assert_eq!(nested.message.thread_root_message_id, Some(root.message.id));

    let first_page = org
        .room_messages_before("owner", room.id, None, 2)
        .await
        .unwrap();
    assert_eq!(first_page.messages.len(), 2);
    assert!(first_page.has_more);
    assert_eq!(
        first_page
            .messages
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        vec![standard_message.message.id, second_root.message.id]
    );
    let second_page = org
        .room_messages_before("owner", room.id, first_page.next_before_message_id, 2)
        .await
        .unwrap();
    assert_eq!(
        second_page
            .messages
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        vec![root.message.id]
    );
    assert!(!second_page.has_more);
    assert_eq!(second_page.next_before_message_id, None);

    let thread = org
        .room_thread_before("exec", room.id, nested.message.id, None, 10)
        .await
        .unwrap();
    assert_eq!(
        thread
            .messages
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        vec![root.message.id, reply.message.id, nested.message.id]
    );

    let latest_thread_page = org
        .room_thread_before("exec", room.id, nested.message.id, None, 1)
        .await
        .unwrap();
    assert_eq!(
        latest_thread_page
            .messages
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        vec![root.message.id, nested.message.id]
    );
    assert!(latest_thread_page.has_more);
    let older_thread_page = org
        .room_thread_before(
            "exec",
            room.id,
            nested.message.id,
            latest_thread_page.next_before_message_id,
            1,
        )
        .await
        .unwrap();
    assert_eq!(
        older_thread_page
            .messages
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        vec![root.message.id, reply.message.id]
    );
    assert!(!older_thread_page.has_more);

    let cursor = org
        .mark_room_read_through(room.id, "exec", nested.message.id)
        .await
        .unwrap();
    let unchanged = org
        .mark_room_read_through(room.id, "exec", root.message.id)
        .await
        .unwrap();
    assert_eq!(cursor.last_read_message_id, Some(nested.message.id));
    assert_eq!(unchanged.last_read_message_id, cursor.last_read_message_id);
    assert_eq!(unchanged.updated_at, cursor.updated_at);
    assert!(org
        .collaboration_events_after("owner", 0, 500)
        .await
        .unwrap()
        .iter()
        .all(|event| event.kind != "room.read_cursor.advanced.v1"));
    assert!(org
        .collaboration_events_after("exec", 0, 500)
        .await
        .unwrap()
        .iter()
        .any(|event| event.kind == "room.read_cursor.advanced.v1"));

    assert!(org
        .room_messages_before("delivery-build", room.id, None, 10)
        .await
        .unwrap_err()
        .to_string()
        .contains("access denied"));
    assert!(org
        .collaboration_events_after("delivery-build", 0, 100)
        .await
        .unwrap()
        .is_empty());

    let other = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Another bounded room",
            &["owner", "exec"],
            "another-bounded-room",
        )
        .await
        .unwrap();
    let other_root = org
        .send_room_message(other.id, "owner", "Other root", None, "other-command")
        .await
        .unwrap();
    let before = org
        .collaboration_events_after("owner", 0, 500)
        .await
        .unwrap();
    assert!(org
        .send_room_message(
            room.id,
            "owner",
            "Cross-room reply must roll back",
            Some(other_root.message.id),
            "invalid-command",
        )
        .await
        .is_err());
    let after = org
        .collaboration_events_after("owner", 0, 500)
        .await
        .unwrap();
    assert_eq!(
        after.len(),
        before.len(),
        "an invalid message cannot leave an operational event behind"
    );
    assert!(org
        .room_messages_before("owner", room.id, None, 100)
        .await
        .unwrap()
        .messages
        .iter()
        .all(|message| message.client_command_id.as_deref() != Some("invalid-command")));

    let message_events = after
        .iter()
        .filter(|event| event.kind == "room.message.created.v1")
        .collect::<Vec<_>>();
    assert_eq!(message_events.len(), 6);
    assert_eq!(
        message_events
            .iter()
            .filter(|event| event.message_id == Some(root.message.id))
            .count(),
        1,
        "an idempotent retry keeps exactly one transactional event"
    );
}

#[tokio::test]
async fn room_creation_commands_are_exact_and_serialize_across_connections() {
    let Some(org) = company("roomcreatecommands").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room create command scenario");
        return;
    };
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let other = OrgIntel::ensure(&database_url, org.schema()).await.unwrap();
    let left = org.clone();
    let right = other.clone();
    let (left_room, right_room) = tokio::join!(
        left.create_room(
            "owner",
            RoomKind::Group,
            "One exact room",
            &["exec"],
            "concurrent-exact-room",
        ),
        right.create_room(
            "owner",
            RoomKind::Group,
            "One exact room",
            &["exec"],
            "concurrent-exact-room",
        ),
    );
    let left_room = left_room.unwrap();
    let right_room = right_room.unwrap();
    assert_eq!(left_room.id, right_room.id);

    let left = org.clone();
    let right = other;
    let (first, second) = tokio::join!(
        left.create_room(
            "owner",
            RoomKind::Group,
            "Candidate A",
            &["exec"],
            "concurrent-conflicting-room",
        ),
        right.create_room(
            "owner",
            RoomKind::Group,
            "Candidate B",
            &["exec"],
            "concurrent-conflicting-room",
        ),
    );
    assert_eq!(first.is_ok() as u8 + second.is_ok() as u8, 1);
    let conflict = first.err().or_else(|| second.err()).unwrap();
    assert!(conflict.to_string().contains("different semantics"));
}

#[tokio::test]
async fn durable_room_and_mention_quotas_serialize_across_connections() {
    let Some(org) = company("roomquotas").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room quota scenario");
        return;
    };
    org.ensure_actor("quota-author", "human", "member", "Quota Author")
        .await
        .unwrap();
    let room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Quota collaboration",
            &["exec", "quota-author"],
            "quota-room",
        )
        .await
        .unwrap();
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let other = OrgIntel::ensure(&database_url, org.schema()).await.unwrap();
    let mut raw = PgConnection::connect(&database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", org.schema()))
        .execute(&mut raw)
        .await
        .unwrap();

    // Seed 127 active owner-created Rooms (including `room`) without spending
    // time on 126 HTTP-shaped commands. The production count and table lock
    // remain the authority under test.
    sqlx::query(
        "INSERT INTO rooms (id,kind,title,created_by) \
         SELECT gen_random_uuid(),'group','quota seed ' || value,'owner' \
         FROM generate_series(1,126) value",
    )
    .execute(&mut raw)
    .await
    .unwrap();
    let left = org.clone();
    let right = other.clone();
    let (first, second) = tokio::join!(
        left.create_room(
            "owner",
            RoomKind::Group,
            "Room quota candidate A",
            &["exec"],
            "room-quota-candidate-a",
        ),
        right.create_room(
            "owner",
            RoomKind::Group,
            "Room quota candidate B",
            &["exec"],
            "room-quota-candidate-b",
        ),
    );
    assert_eq!(first.is_ok() as u8 + second.is_ok() as u8, 1);
    assert!(first
        .err()
        .or_else(|| second.err())
        .unwrap()
        .to_string()
        .contains("at most 128 active Rooms"));

    // Seed one below the 256-pending boundary. Separate OrgIntel pools race
    // the final slot; the transaction-scoped semantic locks make exactly one
    // command win. A different author then proves the recipient-side cap.
    sqlx::query(
        "WITH seeded AS ( \
           INSERT INTO messages (room_id,from_actor,body) \
           SELECT $1,'owner','quota mention ' || value \
           FROM generate_series(1,255) value RETURNING id \
         ) \
         INSERT INTO message_mentions \
           (id,room_id,message_id,thread_root_message_id,mentioned_actor_id,kind,created_event_id) \
         SELECT gen_random_uuid(),$1,id,id,'exec','exec',-id FROM seeded",
    )
    .bind(room.id)
    .execute(&mut raw)
    .await
    .unwrap();
    let mention = NewRoomMessageMention::actor("exec");
    let first_org = org.clone();
    let second_org = other;
    let first_mentions = [mention.clone()];
    let second_mentions = [mention];
    let (first, second) = tokio::join!(
        first_org.send_room_message_with_mentions(
            room.id,
            "owner",
            "Use the final pending slot A",
            None,
            "mention-quota-a",
            None,
            &first_mentions,
            None,
        ),
        second_org.send_room_message_with_mentions(
            room.id,
            "owner",
            "Use the final pending slot B",
            None,
            "mention-quota-b",
            None,
            &second_mentions,
            None,
        ),
    );
    assert_eq!(first.is_ok() as u8 + second.is_ok() as u8, 1);
    assert!(first
        .err()
        .or_else(|| second.err())
        .unwrap()
        .to_string()
        .contains("at most 256 pending mentions"));
    assert!(org
        .send_room_message_with_mentions(
            room.id,
            "quota-author",
            "The recipient quota must also hold.",
            None,
            "recipient-quota",
            None,
            &[NewRoomMessageMention::actor("exec")],
            None,
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("maximum 256 pending mentions"));
}

#[tokio::test]
async fn structured_mentions_are_retry_safe_recipient_relative_and_explicitly_resolved() {
    let Some(org) = company("roommentions").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room mention scenario");
        return;
    };
    org.ensure_actor("delivery-review", "staff", "reviewer", "Delivery Reviewer")
        .await
        .unwrap();
    org.ensure_actor(
        "release-automation",
        "system",
        "automation",
        "Release Automation",
    )
    .await
    .unwrap();
    let delivery_team = org
        .create_team(
            "Delivery",
            "Own the release outcome.",
            "delivery-build",
            "exec",
        )
        .await
        .unwrap();
    org.set_actor_team(
        "delivery-review",
        Some(delivery_team),
        "exec",
        "Review supports the release outcome.",
    )
    .await
    .unwrap();
    let room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Launch decision",
            &[
                "owner",
                "exec",
                "delivery-build",
                "delivery-review",
                "release-automation",
            ],
            "launch-decision-room",
        )
        .await
        .unwrap();
    let other = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Unrelated room",
            &["owner", "exec"],
            "unrelated-room",
        )
        .await
        .unwrap();
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let mut listener = PgListener::connect(&database_url).await.unwrap();
    listener.listen("restless_orgintel").await.unwrap();
    let company_schema = org.schema().to_string();

    let mut exec_mention = NewRoomMessageMention::actor("exec");
    exec_mention.why_this_actor = Some("company-wide release judgement".into());
    exec_mention.expected_response = Some("recommend ship or hold with one reason".into());
    exec_mention.recommendation = Some("ship after the final probe".into());
    exec_mention.alternatives = vec!["ship now".into(), "hold one day".into()];
    exec_mention.evidence = vec!["release probe 42".into()];
    exec_mention.uncertainty = Some("traffic shape is estimated".into());
    exec_mention.affected_scope = Some("public release".into());
    let builder_mention = NewRoomMessageMention::actor("delivery-build");

    let staff_question = org
        .send_room_message_with_mentions(
            room.id,
            "owner",
            "Can the reviewer answer directly?",
            None,
            "ordinary-staff-mention",
            None,
            &[NewRoomMessageMention::actor("delivery-review")],
            None,
        )
        .await
        .unwrap();
    assert_eq!(staff_question.mentions.len(), 1);
    assert_eq!(
        staff_question.mentions[0].mentioned_actor_id,
        "delivery-review"
    );
    assert!(org
        .send_room_message_with_mentions(
            room.id,
            "owner",
            "Can automation answer?",
            None,
            "service-mention",
            None,
            &[NewRoomMessageMention::actor("release-automation")],
            None,
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("cannot receive"));
    assert!(org
        .room_messages_before("owner", room.id, None, 100)
        .await
        .unwrap()
        .messages
        .iter()
        .all(|message| message.client_command_id.as_deref() != Some("service-mention")));

    let sent = org
        .send_room_message_with_mentions(
            room.id,
            "owner",
            "Should we release this build?",
            None,
            "mention-command",
            None,
            &[builder_mention.clone(), exec_mention.clone()],
            None,
        )
        .await
        .unwrap();
    assert!(sent.created);
    assert_eq!(sent.mentions.len(), 2);
    assert_eq!(sent.mentions[0].mentioned_actor_id, "delivery-build");
    assert_eq!(sent.mentions[1].mentioned_actor_id, "exec");
    assert!(sent.resolved_mention.is_none());
    let wake = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let notification = listener.recv().await.unwrap();
            let payload: serde_json::Value = serde_json::from_str(notification.payload()).unwrap();
            if payload["company"] == company_schema
                && payload["kind"] == "mention"
                && payload["body"]["to"] == "exec"
            {
                break payload;
            }
        }
    })
    .await
    .expect("a committed mention publishes a bounded Runtime wake hint");
    assert_eq!(wake["body"]["room_id"], room.id.to_string());
    assert_eq!(wake["body"]["message_id"], sent.message.id);
    assert!(!wake.to_string().contains("Should we release"));

    // Mention order is presentation, not semantics. Retry returns the exact
    // original receipts and no duplicate event/mention rows.
    let retry = org
        .send_room_message_with_mentions(
            room.id,
            "owner",
            "Should we release this build?",
            None,
            "mention-command",
            None,
            &[exec_mention.clone(), builder_mention.clone()],
            None,
        )
        .await
        .unwrap();
    assert!(!retry.created);
    assert_eq!(retry.message.id, sent.message.id);
    assert_eq!(
        retry
            .mentions
            .iter()
            .map(|mention| mention.id)
            .collect::<Vec<_>>(),
        sent.mentions
            .iter()
            .map(|mention| mention.id)
            .collect::<Vec<_>>()
    );
    let mut drifted = exec_mention.clone();
    drifted.recommendation = Some("hold".into());
    assert!(org
        .send_room_message_with_mentions(
            room.id,
            "owner",
            "Should we release this build?",
            None,
            "mention-command",
            None,
            &[drifted, builder_mention.clone()],
            None,
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("conflicts"));

    let exec_pending = org
        .pending_message_mentions_for_actor("exec", 0, 10)
        .await
        .unwrap();
    assert_eq!(exec_pending.len(), 1);
    assert_eq!(exec_pending[0].room_title, "Launch decision");
    assert_eq!(
        exec_pending[0].message.body,
        "Should we release this build?"
    );
    assert_eq!(exec_pending[0].message.id, sent.message.id);
    let exec_context = exec_pending[0].clone();
    let exec_mention_id = exec_pending[0].mention.id;

    // A same-thread message is not a resolution unless it names the exact
    // mention. This prevents unrelated chatter from clearing Attention.
    let unrelated_reply = org
        .send_room_message(
            room.id,
            "exec",
            "I am looking at the release notes.",
            Some(sent.message.id),
            "unrelated-reply",
        )
        .await
        .unwrap();
    assert!(unrelated_reply.resolved_mention.is_none());
    assert!(org
        .next_pending_message_mention("exec")
        .await
        .unwrap()
        .is_some());

    // Neither another Actor nor another Room/Thread can spend the receipt.
    assert!(org
        .send_room_message_with_mentions(
            room.id,
            "delivery-build",
            "My view",
            Some(sent.message.id),
            "wrong-resolver",
            None,
            &[],
            Some(exec_mention_id),
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("only the mentioned Actor"));
    let other_root = org
        .send_room_message(other.id, "owner", "Other topic", None, "other-topic")
        .await
        .unwrap();
    assert!(org
        .send_room_message_with_mentions(
            other.id,
            "exec",
            "Wrong room",
            Some(other_root.message.id),
            "wrong-room-resolution",
            None,
            &[],
            Some(exec_mention_id),
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("exact Room and Thread"));

    let exec_lease = org
        .claim_actor_cognitive_session("exec", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    let exec_claim = org
        .claim_next_pending_message_mention(&exec_lease)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(exec_claim.context.mention.id, exec_context.mention.id);
    let resolved = org
        .reply_to_claimed_message_mention(&exec_claim, "Ship after the final probe.")
        .await
        .unwrap();
    assert!(resolved.created);
    let receipt = resolved.resolved_mention.as_ref().unwrap();
    assert_eq!(receipt.id, exec_mention_id);
    assert_eq!(receipt.resolution_message_id, Some(resolved.message.id));
    assert!(receipt.resolved_event_id.is_some());
    assert!(org
        .next_pending_message_mention("exec")
        .await
        .unwrap()
        .is_none());
    let resolution_retry = org
        .reply_to_claimed_message_mention(&exec_claim, "Ship after the final probe.")
        .await
        .unwrap();
    assert!(!resolution_retry.created);
    assert_eq!(
        resolution_retry.resolved_mention.unwrap().id,
        exec_mention_id
    );

    // Two model workers cannot both spend the same recipient-relative
    // Attention receipt. The stable runtime command key and row lock admit one
    // exact answer and refuse semantic drift from the loser.
    let builder_lease = org
        .claim_team_lead_cognitive_session("delivery-build", delivery_team, Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    let builder_claim = org
        .claim_next_pending_message_mention(&builder_lease)
        .await
        .unwrap()
        .unwrap();
    let first_org = org.clone();
    let first_context = builder_claim.clone();
    let first = tokio::spawn(async move {
        first_org
            .reply_to_claimed_message_mention(&first_context, "Ship")
            .await
    });
    let second_org = org.clone();
    let second_context = builder_claim.clone();
    let second = tokio::spawn(async move {
        second_org
            .reply_to_claimed_message_mention(&second_context, "Hold")
            .await
    });
    let outcomes = [first.await.unwrap(), second.await.unwrap()];
    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|result| result
                .as_ref()
                .is_err_and(|error| error.to_string().contains("conflicts")))
            .count(),
        1
    );
    org.release_actor_cognitive_session(&exec_lease)
        .await
        .unwrap();
    org.release_actor_cognitive_session(&builder_lease)
        .await
        .unwrap();

    let page = org
        .room_thread_before("owner", room.id, sent.message.id, None, 20)
        .await
        .unwrap();
    assert_eq!(page.mentions.len(), 2);
    assert!(page
        .mentions
        .iter()
        .any(|mention| mention.id == exec_mention_id && mention.resolved_at.is_some()));
    let events = org
        .room_events_after("owner", room.id, 0, 100)
        .await
        .unwrap();
    assert!(events
        .events
        .iter()
        .any(|event| event.kind == "room.mention.created.v1"));
    assert!(events
        .events
        .iter()
        .any(|event| event.kind == "room.mention.resolved.v1"));

    let removed_attention = org
        .send_room_message_with_mentions(
            room.id,
            "owner",
            "One more build question",
            None,
            "removed-attention",
            None,
            &[NewRoomMessageMention::actor("delivery-build")],
            None,
        )
        .await
        .unwrap();
    assert_eq!(removed_attention.mentions.len(), 1);
    assert!(org
        .next_pending_message_mention("delivery-build")
        .await
        .unwrap()
        .is_some());

    // Current participation, not historical mention identity, grants access.
    org.remove_room_participant("owner", room.id, "delivery-build")
        .await
        .unwrap();
    assert!(org
        .pending_message_mentions_for_actor("delivery-build", 0, 10)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn room_send_retry_keeps_its_receipt_after_event_compaction() {
    let Some(org) = company("roomretrycompact").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping compacted Room retry scenario");
        return;
    };

    let room = org
        .ensure_company_room("owner", &["owner", "exec"], "company-room-replay")
        .await
        .unwrap();
    let original = org
        .send_room_message(
            room.id,
            "owner",
            "The durable command result",
            None,
            "durable-command",
        )
        .await
        .unwrap();
    assert!(original.created);

    assert_eq!(
        org.compact_events_through(original.event_id).await.unwrap(),
        original.event_id
    );
    assert!(org
        .events_of_kind("room.message.created.v1")
        .await
        .unwrap()
        .iter()
        .all(|event| event.id != original.event_id));

    let cursor_before_retry = org.event_stream_snapshot_cursor().await.unwrap();
    let messages_before_retry = org
        .room_messages_before("owner", room.id, None, 10)
        .await
        .unwrap()
        .messages
        .into_iter()
        .map(|message| message.id)
        .collect::<Vec<_>>();
    let retry = org
        .send_room_message(
            room.id,
            "owner",
            "The durable command result",
            None,
            "durable-command",
        )
        .await
        .unwrap();
    assert!(!retry.created);
    assert_eq!(retry.message.id, original.message.id);
    assert_eq!(retry.event_id, original.event_id);
    assert_eq!(
        org.event_stream_snapshot_cursor().await.unwrap(),
        cursor_before_retry,
        "an exact retry must not append another operational event"
    );
    assert_eq!(
        org.room_messages_before("owner", room.id, None, 10)
            .await
            .unwrap()
            .messages
            .into_iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        messages_before_retry,
        "an exact retry must not insert another authoritative Message"
    );

    let conflict = org
        .send_room_message(
            room.id,
            "owner",
            "A different payload",
            None,
            "durable-command",
        )
        .await
        .unwrap_err();
    assert!(conflict.to_string().contains("conflicts"));
    assert_eq!(
        org.event_stream_snapshot_cursor().await.unwrap(),
        cursor_before_retry,
        "a conflicting retry must not append an operational event"
    );
}

#[tokio::test]
async fn room_list_is_bounded_newest_first_and_keyset_paginated() {
    let Some(org) = company("roomlistpage").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room list pagination scenario");
        return;
    };

    let mut visible_ids = Vec::new();
    for index in 0..3 {
        visible_ids.push(
            org.create_room(
                "owner",
                RoomKind::Group,
                &format!("Visible {index}"),
                &["exec"],
                &format!("visible-room-{index}"),
            )
            .await
            .unwrap()
            .id,
        );
    }
    let private = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Private",
            &["delivery-build"],
            "private-room-list",
        )
        .await
        .unwrap();

    // Equal timestamps exercise the UUID tie-breaker rather than relying on
    // timing differences between test statements.
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let mut raw = PgConnection::connect(&database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", org.schema()))
        .execute(&mut raw)
        .await
        .unwrap();
    sqlx::query("UPDATE rooms SET created_at='2026-01-01T00:00:00Z'")
        .execute(&mut raw)
        .await
        .unwrap();

    visible_ids.sort_by(|left, right| right.cmp(left));
    let first = org.room_page_for_actor("exec", None, 2).await.unwrap();
    assert_eq!(
        first.rooms.iter().map(|room| room.id).collect::<Vec<_>>(),
        visible_ids[..2]
    );
    assert!(first.has_more);
    let cursor = (
        first.next_before_created_at.unwrap(),
        first.next_before_room_id.unwrap(),
    );

    let second = org
        .room_page_for_actor("exec", Some(cursor), 2)
        .await
        .unwrap();
    assert_eq!(
        second.rooms.iter().map(|room| room.id).collect::<Vec<_>>(),
        visible_ids[2..]
    );
    assert!(!second.has_more);
    assert!(second.next_before_created_at.is_none());
    assert!(second.next_before_room_id.is_none());
    assert!(first
        .rooms
        .iter()
        .chain(second.rooms.iter())
        .all(|room| room.id != private.id));

    for limit in [0, 101] {
        assert!(org
            .room_page_for_actor("exec", None, limit)
            .await
            .unwrap_err()
            .to_string()
            .contains("Room list limit"));
    }
}

#[tokio::test]
async fn room_event_replay_is_authorized_isolated_ordered_and_body_free() {
    let Some(org) = company("roomreplay").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room replay scenario");
        return;
    };

    let first_room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Owner and Exec",
            &["exec"],
            "replay-owner-exec-room",
        )
        .await
        .unwrap();
    let second_room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Owner and Builder",
            &["delivery-build"],
            "replay-owner-builder-room",
        )
        .await
        .unwrap();
    let before_messages = org.event_stream_snapshot_cursor().await.unwrap();

    let first = org
        .send_room_message(first_room.id, "owner", "First", None, "replay-first")
        .await
        .unwrap();
    let other_first = org
        .send_room_message(
            second_room.id,
            "owner",
            "Private elsewhere",
            None,
            "replay-other-first",
        )
        .await
        .unwrap();
    let general = org
        .emit_event(
            "test.company-general.v1",
            Some("owner"),
            serde_json::json!({ "secret": "never a Room hint" }),
        )
        .await
        .unwrap();
    let second = org
        .send_room_message(first_room.id, "exec", "Second", None, "replay-second")
        .await
        .unwrap();
    let other_second = org
        .send_room_message(
            second_room.id,
            "delivery-build",
            "Still elsewhere",
            None,
            "replay-other-second",
        )
        .await
        .unwrap();
    let third = org
        .send_room_message(first_room.id, "owner", "Third", None, "replay-third")
        .await
        .unwrap();
    org.mark_room_read_through(first_room.id, "exec", third.message.id)
        .await
        .unwrap();

    for (denied_actor, denied_room) in [
        ("delivery-build", first_room.id),
        ("owner", uuid::Uuid::new_v4()),
    ] {
        assert!(org
            .room_events_after(denied_actor, denied_room, before_messages, 10)
            .await
            .unwrap_err()
            .to_string()
            .contains("access denied"));
    }

    for invalid in [
        org.room_events_after("owner", first_room.id, -1, 10).await,
        org.room_events_after("owner", first_room.id, before_messages, 0)
            .await,
        org.room_events_after("owner", first_room.id, before_messages, 501)
            .await,
    ] {
        assert!(invalid
            .unwrap_err()
            .to_string()
            .contains("invalid operational event cursor"));
    }

    let first_page = org
        .room_events_after("owner", first_room.id, before_messages, 2)
        .await
        .unwrap();
    assert_eq!(
        first_page
            .events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>(),
        vec![first.event_id, second.event_id]
    );
    assert!(first_page
        .events
        .iter()
        .all(|event| event.room_id == first_room.id));
    assert!(first_page.has_more);
    assert!(!first_page.resync_required);
    assert_eq!(first_page.next_after_event_id, second.event_id);
    assert!(first_page.snapshot_cursor >= third.event_id);

    let duplicate_page = org
        .room_events_after("owner", first_room.id, before_messages, 2)
        .await
        .unwrap();
    assert_eq!(
        duplicate_page
            .events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>(),
        vec![first.event_id, second.event_id],
        "at-least-once Room replay keeps stable ids for deduplication"
    );

    let last_page = org
        .room_events_after("owner", first_room.id, first_page.next_after_event_id, 2)
        .await
        .unwrap();
    assert_eq!(
        last_page
            .events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>(),
        vec![third.event_id]
    );
    assert!(!last_page.has_more);
    assert_eq!(last_page.next_after_event_id, last_page.snapshot_cursor);
    assert!(!last_page
        .events
        .iter()
        .any(|event| matches!(event.id, id if id == other_first.event_id || id == other_second.event_id || id == general)));

    let other_page = org
        .room_events_after("owner", second_room.id, before_messages, 10)
        .await
        .unwrap();
    assert_eq!(
        other_page
            .events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>(),
        vec![other_first.event_id, other_second.event_id]
    );
    assert!(other_page
        .events
        .iter()
        .all(|event| event.room_id == second_room.id));

    let exec_page = org
        .room_events_after("exec", first_room.id, before_messages, 10)
        .await
        .unwrap();
    let exec_read_event_id = exec_page
        .events
        .iter()
        .find(|event| event.kind == "room.read_cursor.advanced.v1")
        .expect("a participant sees its own read-cursor hint")
        .id;
    assert!(first_page
        .events
        .iter()
        .chain(last_page.events.iter())
        .all(|event| event.kind != "room.read_cursor.advanced.v1"));

    let at_snapshot = org
        .room_events_after("owner", first_room.id, last_page.snapshot_cursor, 10)
        .await
        .unwrap();
    assert!(at_snapshot.events.is_empty());
    assert!(!at_snapshot.has_more);
    assert!(!at_snapshot.resync_required);
    assert_eq!(at_snapshot.next_after_event_id, at_snapshot.snapshot_cursor);

    let serialized = serde_json::to_value(&first_page).unwrap();
    assert!(serialized["events"]
        .as_array()
        .unwrap()
        .iter()
        .all(|event| event.get("body").is_none()));

    org.compact_events_through(third.event_id).await.unwrap();
    let owner_private_floor = org
        .room_events_after("owner", first_room.id, third.event_id, 10)
        .await
        .unwrap();
    assert!(owner_private_floor.events.is_empty());
    assert_eq!(owner_private_floor.oldest_available_event_id, None);
    let exec_private_floor = org
        .room_events_after("exec", first_room.id, third.event_id, 10)
        .await
        .unwrap();
    assert_eq!(
        exec_private_floor.oldest_available_event_id,
        Some(exec_read_event_id)
    );
    assert_eq!(
        exec_private_floor
            .events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>(),
        vec![exec_read_event_id]
    );
}

#[tokio::test]
async fn room_event_replay_requires_resync_after_compaction_and_access_after_removal() {
    let Some(org) = company("roomreplaycompact").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping compacted Room replay scenario");
        return;
    };

    let room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Compaction",
            &["exec"],
            "compaction-room",
        )
        .await
        .unwrap();
    let first = org
        .send_room_message(room.id, "owner", "Before floor", None, "floor-first")
        .await
        .unwrap();
    let second = org
        .send_room_message(room.id, "exec", "After floor", None, "floor-second")
        .await
        .unwrap();

    assert_eq!(
        org.compact_events_through(first.event_id).await.unwrap(),
        first.event_id
    );
    let expired = org
        .room_events_after("owner", room.id, 0, 10)
        .await
        .unwrap();
    assert!(expired.resync_required);
    assert!(expired.events.is_empty());
    assert_eq!(expired.compacted_through_event_id, first.event_id);
    assert_eq!(expired.oldest_available_event_id, Some(second.event_id));
    assert_eq!(expired.next_after_event_id, 0);

    let retained = org
        .room_events_after("owner", room.id, first.event_id, 10)
        .await
        .unwrap();
    assert!(!retained.resync_required);
    assert_eq!(
        retained
            .events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>(),
        vec![second.event_id]
    );

    org.remove_room_participant("owner", room.id, "exec")
        .await
        .unwrap();
    assert!(org
        .room_events_after("exec", room.id, first.event_id, 10)
        .await
        .unwrap_err()
        .to_string()
        .contains("access denied"));
}

#[tokio::test]
async fn a_slow_room_replay_does_not_block_an_unrelated_room_writer() {
    let Some(org) = company("roomreplayisolation").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room replay isolation scenario");
        return;
    };

    let slow_room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Slow replay",
            &["exec"],
            "slow-room-replay-isolation",
        )
        .await
        .unwrap();
    let independent_room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Independent writer",
            &["delivery-build"],
            "independent-room-replay-isolation",
        )
        .await
        .unwrap();

    // Hold the exact lock used while one replay captures its committed Room
    // prefix. A same-Room writer must wait; an unrelated Room writer must not.
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let mut raw = PgConnection::connect(&database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", org.schema()))
        .execute(&mut raw)
        .await
        .unwrap();
    let mut slow_replay = raw.begin().await.unwrap();
    sqlx::query("SELECT room_id FROM room_event_streams WHERE room_id=$1 FOR SHARE")
        .bind(slow_room.id)
        .execute(&mut *slow_replay)
        .await
        .unwrap();

    let blocked_org = org.clone();
    let mut same_room_write = Box::pin(async move {
        blocked_org
            .send_room_message(
                slow_room.id,
                "owner",
                "Wait for this Room's replay",
                None,
                "same-room-replay-lock",
            )
            .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut same_room_write)
            .await
            .is_err(),
        "the same Room writer must wait for its replay watermark lock"
    );

    let independent = tokio::time::timeout(
        Duration::from_secs(2),
        org.send_room_message(
            independent_room.id,
            "owner",
            "This Room remains available",
            None,
            "independent-room-write",
        ),
    )
    .await
    .expect("an unrelated Room writer must not wait for the slow replay")
    .unwrap();
    assert!(independent.created);

    slow_replay.commit().await.unwrap();
    let same_room = tokio::time::timeout(Duration::from_secs(2), same_room_write)
        .await
        .expect("the same Room writer resumes after replay releases its watermark")
        .unwrap();
    assert!(same_room.created);
    assert!(
        same_room.event_id > independent.event_id,
        "the blocked Room writer must allocate its event identity only after acquiring its stream row"
    );
}

#[tokio::test]
async fn owner_conversation_and_direct_room_share_message_truth() {
    let Some(org) = company("roomdirect").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping direct Room scenario");
        return;
    };

    let (owner_message, _) = org
        .send_owner_conversation_message("exec", "Legacy owner request", false)
        .await
        .unwrap();
    let actor_message = org
        .send_message("exec", None, "Legacy actor reply")
        .await
        .unwrap();

    let direct = org
        .room_page_for_actor("owner", None, 100)
        .await
        .unwrap()
        .rooms
        .into_iter()
        .find(|room| room.kind == RoomKind::Direct)
        .expect("the message APIs create one explicit direct Room");
    let messages = org
        .room_messages_before("owner", direct.id, None, 20)
        .await
        .unwrap();
    assert_eq!(
        messages
            .messages
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        vec![owner_message, actor_message],
        "legacy reads and Rooms point at the same messages.id values"
    );

    let same_direct = org
        .create_room(
            "owner",
            RoomKind::Direct,
            "Owner and Exec",
            &["exec"],
            "same-owner-exec-direct",
        )
        .await
        .unwrap();
    assert_eq!(same_direct.id, direct.id);
    let room_owner_message = org
        .send_room_message(
            direct.id,
            "owner",
            "Room-authored owner request",
            None,
            "direct-1",
        )
        .await
        .unwrap();
    let room_actor_message = org
        .send_room_message(
            direct.id,
            "exec",
            "Room-authored actor reply",
            None,
            "direct-2",
        )
        .await
        .unwrap();
    assert_eq!(room_owner_message.message.to_actor.as_deref(), Some("exec"));
    assert_eq!(room_actor_message.message.to_actor, None);

    let legacy_history = org.owner_conversation("exec", 20).await.unwrap();
    assert_eq!(
        legacy_history
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        vec![
            owner_message,
            actor_message,
            room_owner_message.message.id,
            room_actor_message.message.id,
        ],
        "new direct messages remain visible in the established owner transcript"
    );

    let group = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Human collaboration",
            &["owner", "exec"],
            "human-collaboration-room",
        )
        .await
        .unwrap();
    let group_message = org
        .send_room_message(
            group.id,
            "exec",
            "Visible only in the Room",
            None,
            "group-1",
        )
        .await
        .unwrap();
    assert!(!org
        .inbox(None)
        .await
        .unwrap()
        .iter()
        .any(|message| message.id == group_message.message.id));
    assert!(!org
        .owner_conversation("exec", 20)
        .await
        .unwrap()
        .iter()
        .any(|message| message.id == group_message.message.id));
}

#[tokio::test]
async fn canonical_room_audiences_cannot_be_claimed_or_mutated_by_members() {
    let Some(org) = company("roomaccess").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Rooms access scenario");
        return;
    };

    let company_room = org
        .ensure_company_room("owner", &["owner", "exec"], "company-room-owner")
        .await
        .unwrap();
    let other_company = company("roomaccessother").await.unwrap();
    assert!(other_company
        .room_messages_before("owner", company_room.id, None, 10)
        .await
        .unwrap_err()
        .to_string()
        .contains("access denied"));
    assert!(org
        .ensure_company_room(
            "delivery-build",
            &["delivery-build"],
            "company-room-builder-denied",
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("access denied"));
    assert!(org
        .ensure_company_room(
            "exec",
            &["exec", "delivery-build"],
            "company-room-add-builder",
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("only the Room owner"));
    assert!(org
        .add_room_participant("exec", company_room.id, "delivery-build")
        .await
        .unwrap_err()
        .to_string()
        .contains("only the Room owner"));

    org.add_room_participant("owner", company_room.id, "delivery-build")
        .await
        .unwrap();
    org.send_room_message(
        company_room.id,
        "delivery-build",
        "I was explicitly admitted",
        None,
        "admitted-send",
    )
    .await
    .unwrap();
    assert!(org
        .remove_room_participant("exec", company_room.id, "delivery-build")
        .await
        .unwrap_err()
        .to_string()
        .contains("only the Room owner"));
    org.remove_room_participant("owner", company_room.id, "delivery-build")
        .await
        .unwrap();
    assert!(org
        .send_room_message(
            company_room.id,
            "delivery-build",
            "This must not land",
            None,
            "removed-send",
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("access denied"));
    assert!(org
        .collaboration_events_after("delivery-build", 0, 100)
        .await
        .unwrap()
        .is_empty());

    let direct = org
        .create_room(
            "owner",
            RoomKind::Direct,
            "Owner and Exec",
            &["exec"],
            "immutable-owner-exec-direct",
        )
        .await
        .unwrap();
    assert!(org
        .add_room_participant("owner", direct.id, "delivery-build")
        .await
        .unwrap_err()
        .to_string()
        .contains("immutable"));
}

#[tokio::test]
async fn direct_room_keys_are_unambiguous_for_arbitrary_stable_actor_ids() {
    let Some(org) = company("roomkeys").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Rooms key scenario");
        return;
    };
    for actor in ["a:b", "c", "a", "b:c"] {
        org.ensure_actor(actor, "human", "company-member", actor)
            .await
            .unwrap();
    }

    let first = org
        .create_room(
            "a:b",
            RoomKind::Direct,
            "First pair",
            &["c"],
            "collision-first-pair",
        )
        .await
        .unwrap();
    let second = org
        .create_room(
            "a",
            RoomKind::Direct,
            "Second pair",
            &["b:c"],
            "collision-second-pair",
        )
        .await
        .unwrap();
    assert_ne!(first.id, second.id);
    assert_ne!(first.canonical_key, second.canonical_key);
}

#[tokio::test]
async fn human_direct_focus_is_attributed_and_isolated_per_person() {
    let Some(org) = company("humanrooms").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping human Rooms scenario");
        return;
    };
    for actor in ["human-alice", "human-bob"] {
        org.ensure_actor(actor, "human", "company-member", actor)
            .await
            .unwrap();
    }

    let (alice_first, alice_initial_focus) = org
        .send_human_conversation_message("human-alice", "exec", "Alice's first question", false)
        .await
        .unwrap();
    assert_eq!(alice_initial_focus.after_message_id, 0);
    let (bob_first, _) = org
        .send_human_conversation_message("human-bob", "exec", "Bob's question", false)
        .await
        .unwrap();
    let (alice_second, alice_focus) = org
        .send_human_conversation_message_with_standard(
            "human-alice",
            "exec",
            "Alice starts a new subject",
            true,
            Some(OutcomeStandard::Thorough),
        )
        .await
        .unwrap();
    assert_eq!(alice_focus.after_message_id, alice_first);
    assert_eq!(
        org.human_conversation_focus("human-bob", "exec")
            .await
            .unwrap()
            .after_message_id,
        0
    );
    assert_eq!(
        org.human_conversation("human-alice", "exec", 20)
            .await
            .unwrap()
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        vec![alice_first, alice_second]
    );
    assert_eq!(
        org.human_conversation("human-bob", "exec", 20)
            .await
            .unwrap()
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        vec![bob_first]
    );
    let alice_second_row = org
        .human_conversation("human-alice", "exec", 20)
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(alice_second_row.from_actor, "human-alice");
    assert_eq!(alice_second_row.to_actor.as_deref(), Some("exec"));
    assert_eq!(
        alice_second_row.outcome_standard,
        Some(OutcomeStandard::Thorough)
    );

    let work_id = org
        .add_work(NewWork {
            owner_id: "exec",
            title: "Answer one scoped question",
            outcome: "one attributed answer",
            goal_id: None,
            priority: 1,
            expected_artifact: "none",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let work_message = org
        .send_work_message(
            "human-alice",
            "exec",
            work_id,
            "Alice's Work-scoped correction",
        )
        .await
        .unwrap();
    assert_eq!(
        org.human_work_conversation("human-alice", "exec", work_id, 20)
            .await
            .unwrap()
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        vec![work_message]
    );
    assert!(org
        .human_work_conversation("human-bob", "exec", work_id, 20)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn concurrent_first_read_cursor_updates_never_regress() {
    let Some(org) = company("roomcursor").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room cursor race scenario");
        return;
    };

    for iteration in 0..12 {
        let room = org
            .create_room(
                "owner",
                RoomKind::Group,
                &format!("Cursor race {iteration}"),
                &["exec"],
                &format!("cursor-race-room-{iteration}"),
            )
            .await
            .unwrap();
        let lower = org
            .send_room_message(
                room.id,
                "owner",
                "Lower watermark",
                None,
                &format!("lower-{iteration}"),
            )
            .await
            .unwrap();
        let higher = org
            .send_room_message(
                room.id,
                "owner",
                "Higher watermark",
                None,
                &format!("higher-{iteration}"),
            )
            .await
            .unwrap();
        let (low_result, high_result) = tokio::join!(
            org.mark_room_read_through(room.id, "exec", lower.message.id),
            org.mark_room_read_through(room.id, "exec", higher.message.id),
        );
        low_result.unwrap();
        high_result.unwrap();
        assert_eq!(
            org.room_read_cursor("exec", room.id)
                .await
                .unwrap()
                .unwrap()
                .last_read_message_id,
            Some(higher.message.id)
        );
    }
}

#[tokio::test]
async fn direct_mentions_skip_legacy_mail_and_plain_room_retry_keeps_pre_upgrade_digest() {
    let Some(org) = company("roommentionlegacy").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping direct mention compatibility");
        return;
    };
    let direct = org
        .create_room(
            "owner",
            RoomKind::Direct,
            "Owner and Exec",
            &["exec"],
            "mention-owner-exec-direct",
        )
        .await
        .unwrap();
    let plain = org
        .send_room_message(direct.id, "owner", "Plain legacy mail", None, "plain-mail")
        .await
        .unwrap();
    assert!(org
        .inbox(Some("exec"))
        .await
        .unwrap()
        .iter()
        .any(|message| message.id == plain.message.id));
    org.mark_read(plain.message.id).await.unwrap();

    let mentioned = org
        .send_room_message_with_mentions(
            direct.id,
            "owner",
            "One focused question",
            None,
            "direct-exec-mention",
            None,
            &[NewRoomMessageMention::actor("exec")],
            None,
        )
        .await
        .unwrap();
    assert_eq!(mentioned.message.to_actor, None);
    assert!(org.inbox(Some("exec")).await.unwrap().is_empty());
    assert_eq!(org.owed_conversation_count("exec").await.unwrap(), 0);

    let owner_plain = org
        .send_room_message(direct.id, "exec", "Plain owner mail", None, "owner-plain")
        .await
        .unwrap();
    assert!(org
        .inbox(None)
        .await
        .unwrap()
        .iter()
        .any(|message| message.id == owner_plain.message.id));
    org.mark_read(owner_plain.message.id).await.unwrap();
    let owner_mention = org
        .send_room_message_with_mentions(
            direct.id,
            "exec",
            "A question for the human",
            None,
            "direct-owner-mention",
            None,
            &[NewRoomMessageMention::actor("owner")],
            None,
        )
        .await
        .unwrap();
    assert!(!org
        .inbox(None)
        .await
        .unwrap()
        .iter()
        .any(|message| message.id == owner_mention.message.id));

    let company_room = org
        .ensure_company_room("owner", &["owner", "exec"], "company-room-mention")
        .await
        .unwrap();
    let legacy_body = "An ordinary command created before mention support";
    let legacy_command = "pre-0044-command";
    let mut digest = Sha256::new();
    for part in [
        company_room.id.to_string(),
        "owner".to_string(),
        String::new(),
        String::new(),
        legacy_body.to_string(),
    ] {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part.as_bytes());
    }
    let legacy_digest = format!("{:x}", digest.finalize());
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let mut connection = PgConnection::connect(&database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", org.schema()))
        .execute(&mut connection)
        .await
        .unwrap();
    let legacy_id: i64 = sqlx::query_scalar(
        "INSERT INTO messages \
         (room_id,from_actor,to_actor,body,client_command_id,client_payload_sha256) \
         VALUES ($1,'owner',NULL,$2,$3,$4) RETURNING id",
    )
    .bind(company_room.id)
    .bind(legacy_body)
    .bind(legacy_command)
    .bind(&legacy_digest)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    let retried = org
        .send_room_message(company_room.id, "owner", legacy_body, None, legacy_command)
        .await
        .unwrap();
    assert!(!retried.created);
    assert_eq!(retried.message.id, legacy_id);
}

#[tokio::test]
async fn actor_cognitive_lease_is_cross_pool_renewable_reclaimable_and_excludes_work() {
    let Some(org) = company("actorcognitive").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping cognitive lease scenario");
        return;
    };
    let team_id = org
        .create_team("Delivery", "Own delivery", "delivery-build", "exec")
        .await
        .unwrap();
    let room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Two mentions",
            &["owner", "delivery-build"],
            "two-mentions-room",
        )
        .await
        .unwrap();
    for index in 0..2 {
        org.send_room_message_with_mentions(
            room.id,
            "owner",
            &format!("Question {index}"),
            None,
            &format!("question-{index}"),
            None,
            &[NewRoomMessageMention::actor("delivery-build")],
            None,
        )
        .await
        .unwrap();
    }
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let other = OrgIntel::ensure(&database_url, org.schema()).await.unwrap();
    let (first, second) = tokio::join!(
        org.claim_team_lead_cognitive_session("delivery-build", team_id, Duration::from_secs(60)),
        other.claim_team_lead_cognitive_session("delivery-build", team_id, Duration::from_secs(60))
    );
    let mut winners = [first.unwrap(), second.unwrap()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    assert_eq!(winners.len(), 1);
    let winner = winners.pop().unwrap();
    let claimed = org
        .claim_next_pending_message_mention(&winner)
        .await
        .unwrap()
        .unwrap();
    assert!(other
        .claim_actor_cognitive_session("delivery-build", Duration::from_secs(60))
        .await
        .unwrap()
        .is_none());
    org.release_actor_cognitive_session(&winner).await.unwrap();

    let stale = org
        .claim_team_lead_cognitive_session("delivery-build", team_id, Duration::from_secs(1))
        .await
        .unwrap()
        .unwrap();
    let stale_claim = org
        .claim_next_pending_message_mention(&stale)
        .await
        .unwrap()
        .unwrap();
    tokio::time::sleep(Duration::from_millis(1_100)).await;
    let replacement = other
        .claim_team_lead_cognitive_session("delivery-build", team_id, Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    assert!(org
        .renew_actor_cognitive_session(&stale, Duration::from_secs(60))
        .await
        .is_err());
    assert!(org
        .reply_to_claimed_message_mention(&stale_claim, "Stale answer")
        .await
        .is_err());
    let replacement_claim = other
        .claim_next_pending_message_mention(&replacement)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        replacement_claim.context.mention.id,
        claimed.context.mention.id
    );
    other
        .release_actor_cognitive_session(&replacement)
        .await
        .unwrap();

    org.ensure_actor("research-craft", "staff", "researcher", "Research Craft")
        .await
        .unwrap();
    let work_id = org
        .add_work(NewWork {
            owner_id: "research-craft",
            title: "Prove one exclusion",
            outcome: "one durable concurrency proof",
            goal_id: None,
            priority: 1,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let conversation = org
        .claim_actor_cognitive_session("research-craft", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    assert!(other
        .claim_ready_work("must wait for conversation")
        .await
        .unwrap()
        .is_none());
    org.release_actor_cognitive_session(&conversation)
        .await
        .unwrap();
    let attempt = other
        .claim_ready_work("conversation released")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(attempt.work.id, work_id);
    assert!(org
        .claim_actor_cognitive_session("research-craft", Duration::from_secs(60))
        .await
        .unwrap()
        .is_none());
    other
        .finish_work_attempt(
            attempt.attempt_id,
            WorkAttemptState::Blocked,
            "bounded concurrency proof complete",
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn lifecycle_revocation_preserves_named_mentions_fences_output_and_reclaims_immediately() {
    let Some(org) = company("cognitiverevoke").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping cognitive revocation scenario");
        return;
    };
    org.ensure_actor("delivery-guide", "staff", "lead", "New Delivery Lead")
        .await
        .unwrap();
    let team_id = org
        .create_team("Delivery", "Own delivery", "delivery-build", "exec")
        .await
        .unwrap();
    let owner_input = org
        .send_message("owner", Some("delivery-build"), "Give me the release call")
        .await
        .unwrap();
    let lease = org
        .claim_team_lead_cognitive_session("delivery-build", team_id, Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    assert!(org
        .send_message("delivery-build", None, "unfenced reply")
        .await
        .unwrap_err()
        .to_string()
        .contains("exact cognitive-session lease"));
    let reply = org
        .finalize_cognitive_conversation(
            &lease,
            Some("Release after the signed probe."),
            None,
            &[owner_input],
            &[],
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        org.finalize_cognitive_conversation(
            &lease,
            Some("Release after the signed probe."),
            None,
            &[owner_input],
            &[],
        )
        .await
        .unwrap(),
        Some(reply),
        "a lost finalization receipt replays the one committed owner reply"
    );

    let room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Lifecycle",
            &["owner", "delivery-build", "delivery-guide"],
            "mention-lifecycle-room",
        )
        .await
        .unwrap();
    let question = org
        .send_room_message_with_mentions(
            room.id,
            "owner",
            "Can the old lead answer?",
            None,
            "lifecycle-mention",
            None,
            &[NewRoomMessageMention::actor("delivery-build")],
            None,
        )
        .await
        .unwrap();
    let claim = org
        .claim_next_pending_message_mention(&lease)
        .await
        .unwrap()
        .unwrap();
    let cancelled_direct_input = org
        .send_message(
            "owner",
            Some("delivery-build"),
            "This unread direction must receive a lifecycle receipt",
        )
        .await
        .unwrap();
    org.set_team_lead(
        team_id,
        "delivery-guide",
        "exec",
        "the responsibility changed",
    )
    .await
    .unwrap();
    let work_id = org
        .add_work(NewWork {
            owner_id: "delivery-build",
            title: "Prove lifecycle fencing",
            outcome: "one attributable proof",
            goal_id: None,
            priority: 1,
            expected_artifact: "none",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    assert!(org
        .claim_team_lead_cognitive_session("delivery-build", team_id, Duration::from_secs(60))
        .await
        .unwrap()
        .is_none());
    assert!(org
        .renew_actor_cognitive_session(&lease, Duration::from_secs(60))
        .await
        .is_err());
    assert!(org
        .send_message_with_cognitive_lease(&lease, None, "stale fenced reply")
        .await
        .is_err());
    assert!(org
        .send_message("delivery-build", None, "stale unfenced reply")
        .await
        .is_err());
    assert!(org
        .reply_to_claimed_message_mention(&claim, "Stale Room answer")
        .await
        .is_err());
    assert!(org
        .next_pending_message_mention("delivery-build")
        .await
        .unwrap()
        .is_some());
    assert_eq!(
        org.owed_conversation_count("delivery-build").await.unwrap(),
        0
    );
    let preserved_input = org
        .owner_conversation("delivery-build", 50)
        .await
        .unwrap()
        .into_iter()
        .find(|message| message.id == cancelled_direct_input)
        .expect("the cancelled input remains attributable in the transcript");
    assert!(preserved_input.read_at.is_some());
    assert!(org
        .latest_event("actor.conversation.cancelled.v1")
        .await
        .unwrap()
        .is_some());
    let thread = org
        .room_thread_before("owner", room.id, question.message.id, None, 20)
        .await
        .unwrap();
    assert_eq!(thread.mentions.len(), 1);
    assert!(thread.mentions[0].cancelled_event_id.is_none());
    assert!(thread.mentions[0].resolution_message_id.is_none());
    assert!(!org
        .room_events_after("owner", room.id, 0, 100)
        .await
        .unwrap()
        .events
        .iter()
        .any(|event| event.kind == "room.mention.cancelled.v1"));
    assert!(org
        .claim_ready_work("revoked conversation still excludes Work")
        .await
        .unwrap()
        .is_none());
    let guide_lease = org
        .claim_team_lead_cognitive_session("delivery-guide", team_id, Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let mut raw = PgConnection::connect(&database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", org.schema()))
        .execute(&mut raw)
        .await
        .unwrap();
    assert!(
        sqlx::query("UPDATE actor_cognitive_leases SET focused_mention_id=$2 WHERE actor_id=$1",)
            .bind("delivery-guide")
            .bind(question.mentions[0].id)
            .execute(&mut raw)
            .await
            .is_err(),
        "the database rejects a lease focused on another Actor's mention"
    );
    org.release_actor_cognitive_session(&guide_lease)
        .await
        .unwrap();
    let replacement_lease = org
        .claim_actor_cognitive_session("delivery-build", Duration::from_secs(60))
        .await
        .unwrap()
        .expect("a revoked conversation can be replaced immediately");
    let replacement_claim = org
        .claim_next_pending_message_mention(&replacement_lease)
        .await
        .unwrap()
        .expect("the direct named-Actor obligation survives the lead change");
    assert_eq!(
        replacement_claim.context.mention.id,
        question.mentions[0].id
    );
    org.reply_to_claimed_message_mention(&replacement_claim, "The named Actor can still answer.")
        .await
        .unwrap();
    assert!(!org.release_actor_cognitive_session(&lease).await.unwrap());
    org.release_actor_cognitive_session(&replacement_lease)
        .await
        .unwrap();
    let attempt = org
        .claim_ready_work("exact holder released")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(attempt.work.id, work_id);
    org.finish_work_attempt(
        attempt.attempt_id,
        WorkAttemptState::Blocked,
        "bounded lifecycle proof complete",
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn lead_change_fences_a_previously_captured_conversation_envelope() {
    let Some(org) = company("staleenvelope").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping stale envelope scenario");
        return;
    };
    org.ensure_actor("new-direction", "staff", "lead", "New Direction")
        .await
        .unwrap();
    let team_id = org
        .create_team("Envelope", "Own one envelope", "delivery-build", "exec")
        .await
        .unwrap();
    let captured = org
        .send_message(
            "owner",
            Some("delivery-build"),
            "This exact input was captured before responsibility changed",
        )
        .await
        .unwrap();
    let stale_lease = org
        .claim_team_lead_cognitive_session("delivery-build", team_id, Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();

    org.set_team_lead(
        team_id,
        "new-direction",
        "exec",
        "responsibility changed before finalization",
    )
    .await
    .unwrap();
    let error = org
        .finalize_cognitive_conversation(
            &stale_lease,
            Some("This stale answer must never persist"),
            None,
            &[captured],
            &[],
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("no longer current"));
    assert!(!org
        .owner_conversation("delivery-build", 50)
        .await
        .unwrap()
        .iter()
        .any(|message| message.body == "This stale answer must never persist"));
    assert_eq!(
        org.owed_conversation_count("delivery-build").await.unwrap(),
        0
    );
}

#[tokio::test]
async fn actor_retirement_and_room_removal_share_one_lock_order_and_cancel_once() {
    let Some(org) = company("retireroomrace").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping retirement/Room race");
        return;
    };
    org.ensure_actor("room-guest", "human", "member", "Room Guest")
        .await
        .unwrap();
    let room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Lifecycle race",
            &["owner", "room-guest"],
            "lifecycle-race-room",
        )
        .await
        .unwrap();
    let sent = org
        .send_room_message_with_mentions(
            room.id,
            "owner",
            "Please answer before this audience changes.",
            None,
            "retire-room-race",
            None,
            &[NewRoomMessageMention::actor("room-guest")],
            None,
        )
        .await
        .unwrap();
    let mention_id = sent.mentions[0].id;

    let retiring = org.clone();
    let removing = org.clone();
    let (retired, removed) = tokio::time::timeout(Duration::from_secs(5), async move {
        tokio::join!(
            retiring.retire_actor("room-guest", "exec", "the participant left the company"),
            removing.remove_room_participant("owner", room.id, "room-guest"),
        )
    })
    .await
    .expect("retirement and Room removal must not deadlock");
    retired.unwrap();
    removed.unwrap();

    let mention = org
        .room_messages_before("owner", room.id, None, 20)
        .await
        .unwrap()
        .mentions
        .into_iter()
        .find(|mention| mention.id == mention_id)
        .unwrap();
    assert!(mention.cancelled_at.is_some());
    assert!(mention.resolved_at.is_none());
    let cancellation_events = org
        .room_events_after("owner", room.id, 0, 100)
        .await
        .unwrap()
        .events
        .into_iter()
        .filter(|event| event.kind == "room.mention.cancelled.v1")
        .count();
    assert_eq!(cancellation_events, 1);
}

#[tokio::test]
async fn retiring_a_group_room_owner_archives_the_room_and_terminally_cancels_mentions() {
    let Some(org) = company("retireownedroom").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping owned Room retirement scenario");
        return;
    };
    org.ensure_actor("room-founder", "human", "member", "Room Founder")
        .await
        .unwrap();
    let room = org
        .create_room(
            "room-founder",
            RoomKind::Group,
            "Founder-owned collaboration",
            &["exec"],
            "founder-owned-room",
        )
        .await
        .unwrap();
    let sent = org
        .send_room_message_with_mentions(
            room.id,
            "room-founder",
            "Please make the final call.",
            None,
            "founder-mention",
            None,
            &[NewRoomMessageMention::actor("exec")],
            None,
        )
        .await
        .unwrap();
    let mention_id = sent.mentions[0].id;

    org.retire_actor("room-founder", "exec", "the founder left the company")
        .await
        .unwrap();

    let replay = org
        .create_room(
            "room-founder",
            RoomKind::Group,
            "Founder-owned collaboration",
            &["exec"],
            "founder-owned-room",
        )
        .await
        .expect("an exact lost receipt replays after creator retirement and Room archival");
    assert_eq!(replay.id, room.id);
    assert!(replay.archived_at.is_some());
    assert!(org
        .create_room(
            "room-founder",
            RoomKind::Group,
            "Drift after retirement",
            &["exec"],
            "founder-owned-room",
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("different semantics"));

    assert!(!org
        .room_page_for_actor("exec", None, 100)
        .await
        .unwrap()
        .rooms
        .iter()
        .any(|candidate| candidate.id == room.id));
    assert!(!org
        .pending_message_mentions_for_actor("exec", 0, 100)
        .await
        .unwrap()
        .iter()
        .any(|context| context.mention.id == mention_id));
    let cancellations = org
        .events_of_kind("room.mention.cancelled.v1")
        .await
        .unwrap();
    assert!(cancellations.iter().any(|event| {
        event.body["mention_id"] == mention_id.to_string()
            && event.body["mentioned_actor_id"] == "exec"
    }));
    let archived = org.events_of_kind("room.archived.v1").await.unwrap();
    assert!(archived.iter().any(|event| {
        event.body["room_id"] == room.id.to_string()
            && event.body["former_owner_actor_id"] == "room-founder"
    }));
}

#[tokio::test]
async fn cognitive_finalization_accepts_exact_in_turn_handoff_outcomes_only() {
    let Some(org) = company("handofffinalize").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping handoff finalization scenario");
        return;
    };
    org.ensure_actor("final-direction", "staff", "lead", "Final Direction")
        .await
        .unwrap();
    org.ensure_actor("final-builder", "staff", "builder", "Final Builder")
        .await
        .unwrap();
    let team_id = org
        .create_team("Final", "Own finalization", "final-direction", "exec")
        .await
        .unwrap();
    org.set_actor_team("final-builder", Some(team_id), "exec", "join Final")
        .await
        .unwrap();
    let lease = org
        .claim_team_lead_cognitive_session("final-direction", team_id, Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();

    let resolved_work = org
        .add_work(NewWork {
            owner_id: "final-builder",
            title: "Resolve during turn",
            outcome: "one resolved judgement",
            goal_id: None,
            priority: 1,
            expected_artifact: "none",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let resolved_handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: resolved_work,
            attempt_id: None,
            requested_by: "final-builder",
            category: OwnerHandoffCategory::OwnerJudgement,
            requested_action: "choose the bounded route",
            prepared_state: "both routes compared",
            resume_condition: "the lead records a decision",
        })
        .await
        .unwrap();
    let resolved_input = handoff_input(&org, resolved_handoff).await;
    org.resolve_handoff_as(
        resolved_handoff,
        "final-direction",
        OwnerHandoffState::Resolved,
        "Use the bounded route",
    )
    .await
    .unwrap();
    org.finalize_cognitive_conversation(&lease, None, None, &[], &[resolved_input])
        .await
        .expect("an exact terminal outcome authored during this turn is handled");

    let escalated_work = org
        .add_work(NewWork {
            owner_id: "final-builder",
            title: "Escalate during turn",
            outcome: "one attributed escalation",
            goal_id: None,
            priority: 1,
            expected_artifact: "none",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let escalated_handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: escalated_work,
            attempt_id: None,
            requested_by: "final-builder",
            category: OwnerHandoffCategory::OwnerJudgement,
            requested_action: "decide outside the team remit",
            prepared_state: "team options exhausted",
            resume_condition: "Exec records the wider judgement",
        })
        .await
        .unwrap();
    let escalated_input = handoff_input(&org, escalated_handoff).await;
    org.escalate_handoff(
        escalated_handoff,
        "final-direction",
        "this needs company-wide judgement",
    )
    .await
    .unwrap();
    org.finalize_cognitive_conversation(&lease, None, None, &[], &[escalated_input])
        .await
        .expect("an exact attributed escalation during this turn is handled");

    let stale_work = org
        .add_work(NewWork {
            owner_id: "final-builder",
            title: "Reject unrelated reassignment",
            outcome: "no stale acknowledgement",
            goal_id: None,
            priority: 1,
            expected_artifact: "none",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let stale_handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: stale_work,
            attempt_id: None,
            requested_by: "final-builder",
            category: OwnerHandoffCategory::OwnerJudgement,
            requested_action: "decide one route",
            prepared_state: "decision is ready",
            resume_condition: "the assigned actor decides",
        })
        .await
        .unwrap();
    let stale_input = handoff_input(&org, stale_handoff).await;
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let mut raw = PgConnection::connect(&database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", org.schema()))
        .execute(&mut raw)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE owner_handoffs SET assigned_to='exec',delivered_at=NULL \
         WHERE id=$1 AND state='pending'",
    )
    .bind(stale_handoff)
    .execute(&mut raw)
    .await
    .unwrap();
    assert!(org
        .finalize_cognitive_conversation(&lease, None, None, &[], &[stale_input])
        .await
        .unwrap_err()
        .to_string()
        .contains("not all handled"));

    let refreshed_work = org
        .add_work(NewWork {
            owner_id: "final-builder",
            title: "Reject a refreshed prompt snapshot",
            outcome: "the refreshed handoff remains owed",
            goal_id: None,
            priority: 1,
            expected_artifact: "none",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let refreshed_handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: refreshed_work,
            attempt_id: None,
            requested_by: "final-builder",
            category: OwnerHandoffCategory::OwnerJudgement,
            requested_action: "choose snapshot A",
            prepared_state: "snapshot A was shown to the model",
            resume_condition: "snapshot A receives a decision",
        })
        .await
        .unwrap();
    let snapshot_a = handoff_input(&org, refreshed_handoff).await;
    org.refresh_owner_handoff(
        refreshed_handoff,
        "final-builder",
        "choose snapshot B",
        "snapshot B supersedes what the model saw",
        "snapshot B receives a fresh decision",
    )
    .await
    .unwrap();
    assert!(org
        .finalize_cognitive_conversation(&lease, None, None, &[], &[snapshot_a])
        .await
        .unwrap_err()
        .to_string()
        .contains("not all handled"));
    let refreshed = org
        .list_owner_handoffs()
        .await
        .unwrap()
        .into_iter()
        .find(|handoff| handoff.id == refreshed_handoff)
        .unwrap();
    assert_eq!(refreshed.requested_action, "choose snapshot B");
    assert!(refreshed.delivered_at.is_none());

    let concurrent_resolve_work = org
        .add_work(NewWork {
            owner_id: "final-builder",
            title: "Resolve while the turn finalizes",
            outcome: "one serializable resolution",
            goal_id: None,
            priority: 1,
            expected_artifact: "none",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let concurrent_resolve_handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: concurrent_resolve_work,
            attempt_id: None,
            requested_by: "final-builder",
            category: OwnerHandoffCategory::OwnerJudgement,
            requested_action: "resolve concurrently",
            prepared_state: "the decision is prepared",
            resume_condition: "one resolution commits",
        })
        .await
        .unwrap();
    let concurrent_resolve_input = handoff_input(&org, concurrent_resolve_handoff).await;
    let resolving = org.clone();
    let finalizing = org.clone();
    let finalizing_lease = lease.clone();
    let concurrent_resolve_inputs = [concurrent_resolve_input];
    let (resolved, finalized) = tokio::time::timeout(Duration::from_secs(5), async move {
        tokio::join!(
            resolving.resolve_handoff_as(
                concurrent_resolve_handoff,
                "final-direction",
                OwnerHandoffState::Resolved,
                "The concurrent resolution is authoritative",
            ),
            finalizing.finalize_cognitive_conversation(
                &finalizing_lease,
                None,
                None,
                &[],
                &concurrent_resolve_inputs,
            ),
        )
    })
    .await
    .expect("resolve/finalize must share a deadlock-free lock order");
    resolved.unwrap();
    finalized.unwrap();

    let concurrent_escalate_work = org
        .add_work(NewWork {
            owner_id: "final-builder",
            title: "Escalate while the turn finalizes",
            outcome: "one serializable escalation",
            goal_id: None,
            priority: 1,
            expected_artifact: "none",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let concurrent_escalate_handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: concurrent_escalate_work,
            attempt_id: None,
            requested_by: "final-builder",
            category: OwnerHandoffCategory::OwnerJudgement,
            requested_action: "escalate concurrently",
            prepared_state: "the team boundary is explicit",
            resume_condition: "Exec receives the judgement",
        })
        .await
        .unwrap();
    let concurrent_escalate_input = handoff_input(&org, concurrent_escalate_handoff).await;
    let escalating = org.clone();
    let finalizing = org.clone();
    let finalizing_lease = lease.clone();
    let concurrent_escalate_inputs = [concurrent_escalate_input];
    let (escalated, finalized) = tokio::time::timeout(Duration::from_secs(5), async move {
        tokio::join!(
            escalating.escalate_handoff(
                concurrent_escalate_handoff,
                "final-direction",
                "the decision belongs at company altitude",
            ),
            finalizing.finalize_cognitive_conversation(
                &finalizing_lease,
                None,
                None,
                &[],
                &concurrent_escalate_inputs,
            ),
        )
    })
    .await
    .expect("escalate/finalize must share a deadlock-free lock order");
    escalated.unwrap();
    finalized.unwrap();
    org.release_actor_cognitive_session(&lease).await.unwrap();
}

#[tokio::test]
async fn inspecting_conversation_inbox_preserves_atomic_reply_inputs() {
    let Some(org) = company("inboxinspect").await else {
        return;
    };
    let input = org
        .send_message("owner", Some("exec"), "Please answer this")
        .await
        .unwrap();
    assert_eq!(
        org.consume_inbox_for_actor("exec").await.unwrap()[0].id,
        input
    );
    let lease = org
        .claim_actor_cognitive_session("exec", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    for _ in 0..2 {
        let observed = org.consume_inbox_for_actor("exec").await.unwrap();
        assert_eq!(observed[0].id, input);
        assert!(observed[0].read_at.is_none());
    }
    let later = org
        .send_message("owner", Some("exec"), "A later question")
        .await
        .unwrap();
    org.consume_inbox_for_actor("exec").await.unwrap();
    let reply = org
        .finalize_cognitive_conversation(&lease, Some("The first answer"), None, &[input], &[])
        .await
        .unwrap();
    assert!(reply.is_some());
    assert_eq!(
        org.finalize_cognitive_conversation(&lease, Some("The first answer"), None, &[input], &[])
            .await
            .unwrap(),
        reply
    );
    let remaining = org.conversation_inbox("exec").await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, later);
    org.release_actor_cognitive_session(&lease).await.unwrap();
}

#[tokio::test]
async fn consumed_owner_input_cannot_be_replied_to_after_interrupt() {
    let Some(org) = company("cognitiveinterrupt").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping interrupt fence scenario");
        return;
    };
    let input = org
        .send_message("owner", Some("exec"), "This input is being replaced")
        .await
        .unwrap();
    let lease = org
        .claim_actor_cognitive_session("exec", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    assert!(org
        .interrupt_owner_conversation_message("exec", input)
        .await
        .unwrap());
    assert!(org
        .finalize_cognitive_conversation(&lease, Some("A stale answer"), None, &[input], &[],)
        .await
        .unwrap_err()
        .to_string()
        .contains("no longer current"));
    assert!(!org
        .owner_conversation("exec", 20)
        .await
        .unwrap()
        .iter()
        .any(|message| message.body == "A stale answer"));
    let current_lease = org
        .claim_actor_cognitive_session("exec", Duration::from_secs(60))
        .await
        .unwrap()
        .expect("an interrupted token is fenced strongly enough for immediate replacement");
    assert!(!org.release_actor_cognitive_session(&lease).await.unwrap());

    let current_input = org
        .send_message(
            "owner",
            Some("exec"),
            "This input should commit exactly once",
        )
        .await
        .unwrap();
    let committed = org
        .finalize_cognitive_conversation(
            &current_lease,
            Some("One current answer"),
            None,
            &[current_input],
            &[],
        )
        .await
        .unwrap();
    org.release_actor_cognitive_session(&current_lease)
        .await
        .unwrap();
    assert_eq!(
        org.finalize_cognitive_conversation(
            &current_lease,
            Some("One current answer"),
            None,
            &[current_input],
            &[],
        )
        .await
        .unwrap(),
        committed,
        "the durable receipt remains replayable after the process releases its lease"
    );
    assert_eq!(
        org.owner_conversation("exec", 50)
            .await
            .unwrap()
            .iter()
            .filter(|message| message.body == "One current answer")
            .count(),
        1
    );
}

#[tokio::test]
async fn work_feedback_routes_with_responsibility_without_copying_attempt_input() {
    let Some(org) = company("workfeedbackroute").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Work feedback routing scenario");
        return;
    };
    for (id, role) in [
        ("alpha-direction", "lead"),
        ("alpha-builder", "builder"),
        ("alpha-research", "researcher"),
        ("beta-direction", "lead"),
        ("beta-builder", "builder"),
    ] {
        org.ensure_actor(id, "staff", role, id).await.unwrap();
    }
    let alpha = org
        .create_team("Alpha", "Own alpha", "alpha-direction", "exec")
        .await
        .unwrap();
    org.set_actor_team("alpha-builder", Some(alpha), "exec", "staff alpha")
        .await
        .unwrap();
    org.set_actor_team("alpha-research", Some(alpha), "exec", "staff alpha")
        .await
        .unwrap();
    let beta = org
        .create_team("Beta", "Own beta", "beta-direction", "exec")
        .await
        .unwrap();
    org.set_actor_team("beta-builder", Some(beta), "exec", "staff beta")
        .await
        .unwrap();

    let same_team_work = org
        .add_work(NewWork {
            owner_id: "alpha-builder",
            title: "Same-team route",
            outcome: "one routing proof",
            goal_id: None,
            priority: 1,
            expected_artifact: "none",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let producer_input = org
        .send_work_message(
            "owner",
            "alpha-builder",
            same_team_work,
            "PRODUCER INPUT ONCE",
        )
        .await
        .unwrap();
    let lead_input = org
        .send_work_message(
            "owner",
            "alpha-direction",
            same_team_work,
            "LEAD JUDGEMENT ONLY",
        )
        .await
        .unwrap();
    org.reassign_work(
        same_team_work,
        "alpha-research",
        "exec",
        "move within the accountable team",
    )
    .await
    .unwrap();
    assert!(!org
        .inbox(Some("alpha-builder"))
        .await
        .unwrap()
        .iter()
        .any(|message| message.id == producer_input));
    assert!(org
        .inbox(Some("alpha-research"))
        .await
        .unwrap()
        .iter()
        .any(|message| message.id == producer_input));
    assert!(org
        .conversation_inbox("alpha-direction")
        .await
        .unwrap()
        .iter()
        .any(|message| message.id == lead_input));
    let attempt = org
        .claim_ready_work("same-team responsibility moved")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(attempt.work.id, same_team_work);
    assert_eq!(
        attempt
            .feedback
            .iter()
            .filter(|message| message.body == "PRODUCER INPUT ONCE")
            .count(),
        1
    );
    assert!(!attempt
        .feedback
        .iter()
        .any(|message| message.body.contains("LEAD JUDGEMENT ONLY")));
    org.finish_work_attempt(
        attempt.attempt_id,
        WorkAttemptState::Produced,
        "same-team proof complete",
    )
    .await
    .unwrap();

    let cross_team_work = org
        .add_work(NewWork {
            owner_id: "alpha-builder",
            title: "Cross-team route",
            outcome: "one cross-team routing proof",
            goal_id: None,
            priority: 2,
            expected_artifact: "none",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let prior_producer = org
        .send_work_message(
            "owner",
            "alpha-builder",
            cross_team_work,
            "CROSS PRODUCER INPUT",
        )
        .await
        .unwrap();
    let prior_lead = org
        .send_work_message(
            "owner",
            "alpha-direction",
            cross_team_work,
            "CROSS LEAD INPUT",
        )
        .await
        .unwrap();
    org.reassign_work(
        cross_team_work,
        "beta-builder",
        "exec",
        "move to the beta responsibility",
    )
    .await
    .unwrap();
    assert!(org
        .inbox(Some("beta-builder"))
        .await
        .unwrap()
        .iter()
        .any(|message| message.id == prior_producer));
    assert!(!org
        .inbox(Some("alpha-direction"))
        .await
        .unwrap()
        .iter()
        .any(|message| message.id == prior_lead));
    assert!(org
        .conversation_inbox("beta-direction")
        .await
        .unwrap()
        .iter()
        .any(|message| message.id == prior_lead));
    let attempt = org
        .claim_ready_work("cross-team responsibility moved")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(attempt.work.id, cross_team_work);
    assert_eq!(
        attempt
            .feedback
            .iter()
            .filter(|message| message.body == "CROSS PRODUCER INPUT")
            .count(),
        1
    );
    assert!(!attempt
        .feedback
        .iter()
        .any(|message| message.body.contains("CROSS LEAD INPUT")));
    org.finish_work_attempt(
        attempt.attempt_id,
        WorkAttemptState::Produced,
        "cross-team proof complete",
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn work_message_admission_linearizes_with_owner_and_lead_lifecycle() {
    let Some(org) = company("workmessagerace").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Work message lifecycle races");
        return;
    };
    for (id, role, display) in [
        ("route-direction", "lead", "Avery North"),
        ("route-guidance", "lead", "Morgan Hale"),
        ("route-builder", "builder", "Kai Mercer"),
        ("route-research", "researcher", "Riley Quinn"),
    ] {
        org.create_actor(id, role, display, None, "exec", "routing race proof")
            .await
            .unwrap();
    }
    let team = org
        .create_team(
            "Routing",
            "Own the routed outcome",
            "route-direction",
            "exec",
        )
        .await
        .unwrap();
    for actor in ["route-builder", "route-research"] {
        org.set_actor_team(
            actor,
            Some(team),
            "route-direction",
            "member owns production",
        )
        .await
        .unwrap();
    }

    let lead_work = org
        .add_work(NewWork {
            owner_id: "route-builder",
            title: "Lead-routing race",
            outcome: "one linearized lead input",
            goal_id: None,
            priority: 1,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let sending = org.clone();
    let replacing = org.clone();
    let (sent, replaced) = tokio::time::timeout(Duration::from_secs(5), async move {
        tokio::join!(
            async move {
                sending
                    .send_work_message(
                        "owner",
                        "route-direction",
                        lead_work,
                        "A lead-routed judgement",
                    )
                    .await
            },
            async move {
                replacing
                    .set_team_lead(
                        team,
                        "route-guidance",
                        "exec",
                        "rotate accountability while input is admitted",
                    )
                    .await
            }
        )
    })
    .await
    .expect("Work send and lead replacement must not deadlock");
    replaced.unwrap();
    assert!(org
        .conversation_inbox("route-direction")
        .await
        .unwrap()
        .is_empty());
    if let Ok(message_id) = sent {
        assert!(org
            .conversation_inbox("route-guidance")
            .await
            .unwrap()
            .iter()
            .any(|message| message.id == message_id));
    }

    let owner_work = org
        .add_work(NewWork {
            owner_id: "route-builder",
            title: "Owner-routing race",
            outcome: "one linearized producer input",
            goal_id: None,
            priority: 2,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let sending = org.clone();
    let reassigning = org.clone();
    let (sent, reassigned) = tokio::time::timeout(Duration::from_secs(5), async move {
        tokio::join!(
            async move {
                sending
                    .send_work_message(
                        "owner",
                        "route-builder",
                        owner_work,
                        "A producer-routed fact",
                    )
                    .await
            },
            async move {
                reassigning
                    .reassign_work(
                        owner_work,
                        "route-research",
                        "exec",
                        "move responsibility while input is admitted",
                    )
                    .await
            }
        )
    })
    .await
    .expect("Work send and reassignment must not deadlock");
    reassigned.unwrap();
    assert!(!org
        .inbox(Some("route-builder"))
        .await
        .unwrap()
        .iter()
        .any(|message| sent.as_ref().is_ok_and(|id| message.id == *id)));
    if let Ok(message_id) = sent {
        assert!(org
            .inbox(Some("route-research"))
            .await
            .unwrap()
            .iter()
            .any(|message| message.id == message_id));
    }
}

#[tokio::test]
async fn work_message_attention_reference_is_rechecked_inside_the_message_transaction() {
    let Some(org) = company("attentionmessagerace").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Attention message race");
        return;
    };
    org.create_actor(
        "attention-direction",
        "lead",
        "Avery North",
        None,
        "exec",
        "Attention race proof",
    )
    .await
    .unwrap();
    org.create_actor(
        "attention-builder",
        "builder",
        "Kai Mercer",
        None,
        "exec",
        "Attention race proof",
    )
    .await
    .unwrap();
    let team = org
        .create_team(
            "Attention",
            "Own exact Attention",
            "attention-direction",
            "exec",
        )
        .await
        .unwrap();
    org.set_actor_team(
        "attention-builder",
        Some(team),
        "attention-direction",
        "builder owns the exact Work",
    )
    .await
    .unwrap();
    let work_id = org
        .add_work(NewWork {
            owner_id: "attention-builder",
            title: "Race one Attention reply",
            outcome: "one exact pending handoff reference",
            goal_id: None,
            priority: 1,
            expected_artifact: "none",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let handoff_id = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id,
            attempt_id: None,
            requested_by: "attention-builder",
            category: OwnerHandoffCategory::OwnerJudgement,
            requested_action: "choose the exact route",
            prepared_state: "the bounded alternatives are prepared",
            resume_condition: "the accountable lead records one answer",
        })
        .await
        .unwrap();
    let expected_handoff = handoff_input(&org, handoff_id).await;

    // Hold the handoff row across the sender's earlier Work/Team/Actor reads.
    // This deterministically opens the precise admission window: the sender
    // can only continue after this transaction makes the projected Attention
    // reference terminal.
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let mut raw = PgConnection::connect(&database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", org.schema()))
        .execute(&mut raw)
        .await
        .unwrap();
    let mut lifecycle = raw.begin().await.unwrap();
    sqlx::query("SELECT id FROM owner_handoffs WHERE id=$1 FOR UPDATE")
        .bind(handoff_id)
        .execute(&mut *lifecycle)
        .await
        .unwrap();
    let command_id = format!("attention-race-{}", uuid::Uuid::new_v4());
    let digest = "a".repeat(64);
    let mut sending = Box::pin(org.send_work_message_idempotent(
        "owner",
        "attention-direction",
        work_id,
        "This body was projected from the exact pending Attention item.",
        Some(&expected_handoff),
        &[],
        &command_id,
        &digest,
    ));
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut sending)
            .await
            .is_err(),
        "the Message transaction must wait for the exact handoff row"
    );
    sqlx::query(
        "UPDATE owner_handoffs SET state='resolved',resolution='resolved concurrently',\
         resolved_at=now() WHERE id=$1",
    )
    .bind(handoff_id)
    .execute(&mut *lifecycle)
    .await
    .unwrap();
    lifecycle.commit().await.unwrap();
    let error = tokio::time::timeout(Duration::from_secs(5), sending)
        .await
        .expect("Attention/message race must not deadlock")
        .unwrap_err();
    assert!(error.to_string().contains("no longer pending"));
    assert!(org
        .conversation_command_receipt("owner", "attention-direction", &command_id, &digest,)
        .await
        .unwrap()
        .is_none());

    let refreshed_work = org
        .add_work(NewWork {
            owner_id: "attention-builder",
            title: "Refresh one projected Attention item",
            outcome: "only the presentation actually sent is persisted",
            goal_id: None,
            priority: 1,
            expected_artifact: "none",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let refreshed_handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: refreshed_work,
            attempt_id: None,
            requested_by: "attention-builder",
            category: OwnerHandoffCategory::OwnerJudgement,
            requested_action: "review projection A",
            prepared_state: "projection A is ready",
            resume_condition: "projection A is answered",
        })
        .await
        .unwrap();
    let projection_a = handoff_input(&org, refreshed_handoff).await;
    org.refresh_owner_handoff(
        refreshed_handoff,
        "attention-builder",
        "review projection B",
        "projection B replaced the displayed evidence",
        "projection B is answered",
    )
    .await
    .unwrap();
    let stale_command = format!("attention-refresh-a-{}", uuid::Uuid::new_v4());
    let stale_digest = "b".repeat(64);
    assert!(org
        .send_work_message_idempotent(
            "owner",
            "attention-direction",
            refreshed_work,
            "This body embeds projection A.",
            Some(&projection_a),
            &[],
            &stale_command,
            &stale_digest,
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("changed"));
    assert!(org
        .conversation_command_receipt(
            "owner",
            "attention-direction",
            &stale_command,
            &stale_digest,
        )
        .await
        .unwrap()
        .is_none());
    let projection_b = handoff_input(&org, refreshed_handoff).await;
    let fresh_command = format!("attention-refresh-b-{}", uuid::Uuid::new_v4());
    let fresh_digest = "c".repeat(64);
    let (fresh_message, created) = org
        .send_work_message_idempotent(
            "owner",
            "attention-direction",
            refreshed_work,
            "This body embeds projection B.",
            Some(&projection_b),
            &[],
            &fresh_command,
            &fresh_digest,
        )
        .await
        .unwrap();
    assert!(created);
    assert_eq!(
        org.message_work_id(fresh_message).await.unwrap(),
        Some(refreshed_work)
    );
}

#[tokio::test]
async fn work_linked_finalization_linearizes_with_responsibility_change() {
    let Some(org) = company("workfinalizerace").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Work finalization race");
        return;
    };
    for (id, role, display) in [
        ("reply-direction", "lead", "Morgan Hale"),
        ("reply-builder", "builder", "Riley Quinn"),
        ("reply-other", "builder", "Jordan Ellis"),
    ] {
        org.create_actor(id, role, display, None, "exec", "reply race proof")
            .await
            .unwrap();
    }
    let team = org
        .create_team(
            "Reply",
            "Own Work-linked replies",
            "reply-direction",
            "exec",
        )
        .await
        .unwrap();
    org.set_actor_team(
        "reply-builder",
        Some(team),
        "reply-direction",
        "builder owns production",
    )
    .await
    .unwrap();
    let work_id = org
        .add_work(NewWork {
            owner_id: "reply-builder",
            title: "Fence one stale Work reply",
            outcome: "no reply under stale responsibility",
            goal_id: None,
            priority: 1,
            expected_artifact: "none",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let input = org
        .send_work_message(
            "owner",
            "reply-direction",
            work_id,
            "Answer only while you still own this responsibility.",
        )
        .await
        .unwrap();
    let lease = org
        .claim_team_lead_cognitive_session("reply-direction", team, Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();

    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let mut raw = PgConnection::connect(&database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", org.schema()))
        .execute(&mut raw)
        .await
        .unwrap();
    let mut lifecycle = raw.begin().await.unwrap();
    sqlx::query("SELECT id FROM work WHERE id=$1 FOR UPDATE")
        .bind(work_id)
        .execute(&mut *lifecycle)
        .await
        .unwrap();
    let captured_inputs = [input];
    let mut finalizing = Box::pin(org.finalize_cognitive_conversation(
        &lease,
        Some("This stale Work answer must not persist."),
        Some(work_id),
        &captured_inputs,
        &[],
    ));
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut finalizing)
            .await
            .is_err(),
        "Work-linked finalization must acquire the outer Work authority lock"
    );
    sqlx::query(
        "UPDATE work SET owner_id='reply-other',revision=revision+1,updated_at=now() WHERE id=$1",
    )
    .bind(work_id)
    .execute(&mut *lifecycle)
    .await
    .unwrap();
    lifecycle.commit().await.unwrap();
    let error = tokio::time::timeout(Duration::from_secs(5), finalizing)
        .await
        .expect("Work lifecycle/finalization lock order must not deadlock")
        .unwrap_err();
    assert!(error.to_string().contains("not replying actor"));
    assert!(!org
        .owner_conversation("reply-direction", 20)
        .await
        .unwrap()
        .iter()
        .any(|message| message.body == "This stale Work answer must not persist."));
    org.release_actor_cognitive_session(&lease).await.unwrap();
}

#[tokio::test]
async fn conversation_batches_are_bounded_exact_and_drain_without_duplication() {
    let Some(org) = company("conversationbatch").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping conversation batching scenario");
        return;
    };
    let body = "x".repeat(20_000);
    let mut all_ids = Vec::new();
    for index in 0..10 {
        all_ids.push(
            org.send_message("owner", Some("exec"), &format!("{index:02}:{body}"))
                .await
                .unwrap(),
        );
    }
    assert!(org
        .send_message("owner", Some("exec"), &"z".repeat(70_000))
        .await
        .unwrap_err()
        .to_string()
        .contains("65536"));
    let lease = org
        .claim_actor_cognitive_session("exec", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    let first = org.conversation_inbox("exec").await.unwrap();
    assert_eq!(first.len(), 6);
    assert!(
        first
            .iter()
            .map(|message| message.body.len())
            .sum::<usize>()
            <= 128 * 1024
    );
    let first_ids = first.iter().map(|message| message.id).collect::<Vec<_>>();
    assert_eq!(first_ids, all_ids[..6]);
    org.finalize_cognitive_conversation(
        &lease,
        Some("I received the first bounded batch."),
        None,
        &first_ids,
        &[],
    )
    .await
    .unwrap();
    org.release_actor_cognitive_session(&lease).await.unwrap();
    let second = org.conversation_inbox("exec").await.unwrap();
    let second_ids = second.iter().map(|message| message.id).collect::<Vec<_>>();
    assert_eq!(second_ids, all_ids[6..]);
    let lease = org
        .claim_actor_cognitive_session("exec", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    org.finalize_cognitive_conversation(
        &lease,
        Some("I received the remaining bounded batch."),
        None,
        &second_ids,
        &[],
    )
    .await
    .unwrap();
    assert!(org.conversation_inbox("exec").await.unwrap().is_empty());
    assert_eq!(org.owed_conversation_count("exec").await.unwrap(), 0);
    let owner_thread = org.owner_conversation("exec", 20).await.unwrap();
    assert!(owner_thread
        .iter()
        .any(|message| message.body == "I received the first bounded batch."));
    assert!(owner_thread
        .iter()
        .any(|message| message.body == "I received the remaining bounded batch."));
    org.release_actor_cognitive_session(&lease).await.unwrap();
}

#[tokio::test]
async fn lead_admission_rejects_services_and_actors_with_unsettled_work() {
    let Some(org) = company("leadclass").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping lead-class scenario");
        return;
    };
    org.ensure_actor("service-bot", "system", "bot", "Service Bot")
        .await
        .unwrap();
    assert!(org
        .create_team("Invalid", "No service lead", "service-bot", "exec")
        .await
        .unwrap_err()
        .to_string()
        .contains("active agent Actor"));

    org.ensure_actor("candidate-direction", "staff", "lead", "Candidate Lead")
        .await
        .unwrap();
    let candidate_work = org
        .add_work(NewWork {
            owner_id: "candidate-direction",
            title: "Settle before promotion",
            outcome: "one completed responsibility",
            goal_id: None,
            priority: 1,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    assert!(org
        .create_team("Blocked", "No stalled Work", "candidate-direction", "exec",)
        .await
        .unwrap_err()
        .to_string()
        .contains("existing Work"));
    let attempt = org
        .claim_ready_work("settle promotion candidate")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(attempt.work.id, candidate_work);
    org.finish_work_attempt(
        attempt.attempt_id,
        WorkAttemptState::Produced,
        "candidate Work settled",
    )
    .await
    .unwrap();
    org.create_team("Valid", "Now accountable", "candidate-direction", "exec")
        .await
        .unwrap();
}

#[tokio::test]
async fn cognitive_finalization_links_owner_reply_to_work_once() {
    let Some(org) = company("cognitiveworkreply").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Work reply finalization scenario");
        return;
    };
    let work_id = org
        .add_work(NewWork {
            owner_id: "exec",
            title: "Answer one Work question",
            outcome: "one linked reply",
            goal_id: None,
            priority: 1,
            expected_artifact: "none",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let input = org
        .send_work_message("owner", "exec", work_id, "What is the decision?")
        .await
        .unwrap();
    let lease = org
        .claim_actor_cognitive_session("exec", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    let reply = org
        .finalize_cognitive_conversation(
            &lease,
            Some("Proceed with the bounded route."),
            Some(work_id),
            &[input],
            &[],
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(org.message_work_id(reply).await.unwrap(), Some(work_id));
    assert_eq!(
        org.finalize_cognitive_conversation(
            &lease,
            Some("Proceed with the bounded route."),
            Some(work_id),
            &[input],
            &[],
        )
        .await
        .unwrap(),
        Some(reply)
    );
    org.release_actor_cognitive_session(&lease).await.unwrap();
}
