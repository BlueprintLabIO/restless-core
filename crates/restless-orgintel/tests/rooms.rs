//! Human Rooms remain one projection over the existing message truth.
//! These scenarios run against a scratch company schema when Postgres is
//! available, like the rest of the OrgIntel behavioural suite.

use restless_orgintel::{NewWork, OrgIntel, OutcomeStandard, RoomKind, WorkspaceSpec};

async fn company(prefix: &str) -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!("{prefix}{}", uuid::Uuid::new_v4().simple());
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

#[tokio::test]
async fn room_send_is_idempotent_threaded_paginated_and_transactional() {
    let Some(org) = company("rooms").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Rooms scenario");
        return;
    };

    let room = org
        .ensure_company_room("owner", &["owner", "exec"])
        .await
        .unwrap();
    let same_room = org
        .ensure_company_room("owner", &["owner", "exec"])
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
        .room_messages_after("owner", room.id, None, 2)
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
        vec![root.message.id, standard_message.message.id]
    );
    let second_page = org
        .room_messages_after("owner", room.id, first_page.next_after_message_id, 2)
        .await
        .unwrap();
    assert_eq!(
        second_page
            .messages
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        vec![second_root.message.id, reply.message.id]
    );
    assert!(second_page.has_more);
    let third_page = org
        .room_messages_after("owner", room.id, second_page.next_after_message_id, 2)
        .await
        .unwrap();
    assert_eq!(
        third_page
            .messages
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        vec![nested.message.id]
    );
    assert!(!third_page.has_more);

    let thread = org
        .room_thread_after("exec", room.id, nested.message.id, None, 10)
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
        .room_messages_after("delivery-build", room.id, None, 10)
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
        .room_messages_after("owner", room.id, None, 100)
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
async fn room_send_retry_keeps_its_receipt_after_event_compaction() {
    let Some(org) = company("roomretrycompact").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping compacted Room retry scenario");
        return;
    };

    let room = org
        .ensure_company_room("owner", &["owner", "exec"])
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
        .room_messages_after("owner", room.id, None, 10)
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
        org.room_messages_after("owner", room.id, None, 10)
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
async fn room_event_replay_is_authorized_isolated_ordered_and_body_free() {
    let Some(org) = company("roomreplay").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room replay scenario");
        return;
    };

    let first_room = org
        .create_room("owner", RoomKind::Group, "Owner and Exec", &["exec"])
        .await
        .unwrap();
    let second_room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Owner and Builder",
            &["delivery-build"],
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
        .create_room("owner", RoomKind::Group, "Compaction", &["exec"])
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
async fn legacy_owner_conversation_and_direct_room_share_message_truth() {
    let Some(org) = company("roomcompat").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Rooms compatibility scenario");
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
        .list_rooms_for_actor("owner")
        .await
        .unwrap()
        .into_iter()
        .find(|room| room.kind == RoomKind::Direct)
        .expect("the compatibility bridge creates one direct Room");
    let bridged = org
        .room_messages_after("owner", direct.id, None, 20)
        .await
        .unwrap();
    assert_eq!(
        bridged
            .messages
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        vec![owner_message, actor_message],
        "legacy reads and Rooms point at the same messages.id values"
    );

    let same_direct = org
        .create_room("owner", RoomKind::Direct, "Owner and Exec", &["exec"])
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
        .ensure_company_room("owner", &["owner", "exec"])
        .await
        .unwrap();
    let other_company = company("roomaccessother").await.unwrap();
    assert!(other_company
        .room_messages_after("owner", company_room.id, None, 10)
        .await
        .unwrap_err()
        .to_string()
        .contains("access denied"));
    assert!(org
        .ensure_company_room("delivery-build", &["delivery-build"])
        .await
        .unwrap_err()
        .to_string()
        .contains("access denied"));
    assert!(org
        .ensure_company_room("exec", &["exec", "delivery-build"])
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
        .create_room("owner", RoomKind::Direct, "Owner and Exec", &["exec"])
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
        .create_room("a:b", RoomKind::Direct, "First pair", &["c"])
        .await
        .unwrap();
    let second = org
        .create_room("a", RoomKind::Direct, "Second pair", &["b:c"])
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
