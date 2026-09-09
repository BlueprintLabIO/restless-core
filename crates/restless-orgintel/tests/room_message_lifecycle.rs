//! Append-only Room Message lifecycle and authorised search.

use restless_orgintel::{NewRoomMessageMention, OrgIntel, RoomKind};
use sqlx::postgres::PgConnection;
use sqlx::{Connection as _, Row as _};
use std::time::Duration;

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
    org.ensure_actor("research-science", "staff", "researcher", "Researcher")
        .await
        .unwrap();
    Some(org)
}

#[tokio::test]
async fn edits_and_deletes_are_immutable_retry_safe_and_visible_as_tombstones() {
    let Some(org) = company("room_lifecycle").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room lifecycle scenario");
        return;
    };
    let room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Release room",
            &["owner", "exec"],
            "room-lifecycle-create",
        )
        .await
        .unwrap();
    let root = org
        .send_room_message(
            room.id,
            "exec",
            "The release is amber",
            None,
            "room-lifecycle-root",
        )
        .await
        .unwrap();
    assert_eq!(root.message.revision_number, 0);
    let reply = org
        .send_room_message(
            room.id,
            "owner",
            "What changed?",
            Some(root.message.id),
            "room-lifecycle-reply",
        )
        .await
        .unwrap();
    let fuzzy_rooms = org
        .search_rooms_for_actor("owner", "Releaze", None, 20)
        .await
        .unwrap();
    assert_eq!(fuzzy_rooms.rooms.len(), 1);
    assert_eq!(fuzzy_rooms.rooms[0].id, room.id);

    let edited = org
        .edit_room_message(
            room.id,
            "exec",
            root.message.id,
            0,
            "The release is green after rollback testing",
            "room-lifecycle-edit",
        )
        .await
        .unwrap();
    assert!(edited.created);
    assert_eq!(edited.revision.revision_number, 1);
    let retry = org
        .edit_room_message(
            room.id,
            "exec",
            root.message.id,
            0,
            "The release is green after rollback testing",
            "room-lifecycle-edit",
        )
        .await
        .unwrap();
    assert!(!retry.created);
    assert_eq!(retry.revision.id, edited.revision.id);
    assert_eq!(
        retry.revision.created_event_id,
        edited.revision.created_event_id
    );

    let changed_retry = org
        .edit_room_message(
            room.id,
            "exec",
            root.message.id,
            0,
            "A conflicting retry",
            "room-lifecycle-edit",
        )
        .await
        .unwrap_err();
    assert!(changed_retry.to_string().contains("conflicts"));
    let stale = org
        .edit_room_message(
            room.id,
            "exec",
            root.message.id,
            0,
            "A stale concurrent edit",
            "room-lifecycle-stale",
        )
        .await
        .unwrap_err();
    assert!(stale.to_string().contains("current revision is 1"));
    let foreign_edit = org
        .edit_room_message(
            room.id,
            "owner",
            root.message.id,
            1,
            "Owners do not rewrite someone else's words",
            "room-lifecycle-foreign-edit",
        )
        .await
        .unwrap_err();
    assert!(foreign_edit.to_string().contains("original Message author"));

    let visible = org
        .room_thread_before("owner", room.id, root.message.id, None, 20)
        .await
        .unwrap();
    assert_eq!(visible.messages[0].body, edited.revision.body);
    assert_eq!(visible.messages[0].revision_number, 1);
    assert!(visible.messages[0].edited_at.is_some());
    assert_eq!(visible.messages[1].id, reply.message.id);
    let history = org
        .room_message_revisions_before("owner", room.id, root.message.id, None, 10)
        .await
        .unwrap();
    assert_eq!(history.revisions.len(), 1);
    assert_eq!(history.revisions[0].id, edited.revision.id);

    let before_delete_search = org
        .search_room_messages("owner", "rollback", None, 20)
        .await
        .unwrap();
    assert_eq!(before_delete_search.messages.len(), 1);
    assert_eq!(before_delete_search.messages[0].id, root.message.id);

    let deleted = org
        .delete_room_message(room.id, "owner", root.message.id, "room-lifecycle-delete")
        .await
        .unwrap();
    assert!(deleted.created, "the Room owner may moderate a Message");
    let delete_retry = org
        .delete_room_message(room.id, "owner", root.message.id, "room-lifecycle-delete")
        .await
        .unwrap();
    assert!(!delete_retry.created);
    assert_eq!(delete_retry.tombstone.id, deleted.tombstone.id);
    assert_eq!(
        delete_retry.tombstone.created_event_id,
        deleted.tombstone.created_event_id
    );
    let edit_receipt_after_delete = org
        .edit_room_message(
            room.id,
            "exec",
            root.message.id,
            0,
            "The release is green after rollback testing",
            "room-lifecycle-edit",
        )
        .await
        .unwrap();
    assert!(!edit_receipt_after_delete.created);
    assert_eq!(edit_receipt_after_delete.revision.id, edited.revision.id);
    let tombstoned = org
        .room_thread_before("owner", room.id, root.message.id, None, 20)
        .await
        .unwrap();
    assert_eq!(tombstoned.messages[0].revision_number, 1);
    assert_eq!(tombstoned.messages[0].id, root.message.id);
    assert_eq!(tombstoned.messages[0].body, "");
    assert!(tombstoned.messages[0].deleted_at.is_some());
    assert_eq!(tombstoned.messages[1].id, reply.message.id);
    assert!(org
        .search_room_messages("owner", "rollback", None, 20)
        .await
        .unwrap()
        .messages
        .is_empty());
    assert!(org
        .edit_room_message(
            room.id,
            "exec",
            root.message.id,
            1,
            "Deleted content cannot be revived by edit",
            "room-lifecycle-after-delete",
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("deleted"));
    assert!(org
        .room_message_revisions_before("owner", room.id, root.message.id, None, 10)
        .await
        .unwrap_err()
        .to_string()
        .contains("unavailable for a deleted"));

    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let mut database = PgConnection::connect(&database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", org.schema()))
        .execute(&mut database)
        .await
        .unwrap();
    let stored = sqlx::query("SELECT body,current_plain_text FROM messages WHERE id=$1")
        .bind(root.message.id)
        .fetch_one(&mut database)
        .await
        .unwrap();
    assert_eq!(stored.get::<String, _>("body"), "The release is amber");
    assert_eq!(stored.get::<String, _>("current_plain_text"), "");
    assert!(
        sqlx::query("UPDATE room_message_revisions SET body='tampered' WHERE id=$1")
            .bind(edited.revision.id)
            .execute(&mut database)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM room_message_tombstones WHERE id=$1")
            .bind(deleted.tombstone.id)
            .execute(&mut database)
            .await
            .is_err()
    );

    let events = org
        .room_events_after("owner", room.id, 0, 500)
        .await
        .unwrap();
    assert!(events
        .events
        .iter()
        .any(|event| event.kind == "room.message.edited.v1"));
    assert!(events
        .events
        .iter()
        .any(|event| event.kind == "room.message.deleted.v1"));
    org.remove_room_participant("owner", room.id, "exec")
        .await
        .unwrap();
    let edit_receipt_after_removal = org
        .edit_room_message(
            room.id,
            "exec",
            root.message.id,
            0,
            "The release is green after rollback testing",
            "room-lifecycle-edit",
        )
        .await
        .unwrap();
    assert!(!edit_receipt_after_removal.created);
    assert_eq!(edit_receipt_after_removal.revision.id, edited.revision.id);

    let direct = org
        .create_room(
            "owner",
            RoomKind::Direct,
            "Owner and Exec",
            &["exec"],
            "room-lifecycle-direct",
        )
        .await
        .unwrap();
    let instruction = org
        .send_room_message(
            direct.id,
            "owner",
            "Initial private instruction",
            None,
            "room-lifecycle-direct-instruction",
        )
        .await
        .unwrap();
    let attachment_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO owner_attachments \
         (attachment_id,sender_actor_id,target_actor_id,client_command_id,\
          client_payload_sha256,canonical_name,canonical_media_type,size_bytes,\
          content_sha256,message_id,linked_at,staging_finished_at) \
         VALUES ($1,'owner','exec','room-lifecycle-attachment',$2,\
                 'proof.txt','text/plain',5,$3,$4,now(),now())",
    )
    .bind(attachment_id)
    .bind("a".repeat(64))
    .bind("b".repeat(64))
    .bind(instruction.message.id)
    .execute(&mut database)
    .await
    .unwrap();
    assert!(org
        .owner_attachment_for_actor(attachment_id, "owner")
        .await
        .unwrap()
        .is_some());
    org.edit_room_message(
        direct.id,
        "owner",
        instruction.message.id,
        0,
        "Updated private instruction",
        "room-lifecycle-direct-edit",
    )
    .await
    .unwrap();
    let inbox = org.conversation_inbox("exec").await.unwrap();
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].body, "Updated private instruction");
    org.delete_room_message(
        direct.id,
        "owner",
        instruction.message.id,
        "room-lifecycle-direct-delete-own",
    )
    .await
    .unwrap();
    assert!(org.conversation_inbox("exec").await.unwrap().is_empty());
    assert!(org
        .owner_attachment_for_actor(attachment_id, "owner")
        .await
        .unwrap()
        .is_none());

    let reply = org
        .send_room_message(
            direct.id,
            "exec",
            "Private reply",
            None,
            "room-lifecycle-direct-reply",
        )
        .await
        .unwrap();
    let arbitrary_owner_delete = org
        .delete_room_message(
            direct.id,
            "owner",
            reply.message.id,
            "room-lifecycle-direct-moderate",
        )
        .await
        .unwrap_err();
    assert!(arbitrary_owner_delete
        .to_string()
        .contains("author may delete it in a direct Room"));
    org.delete_room_message(
        direct.id,
        "exec",
        reply.message.id,
        "room-lifecycle-direct-delete-own-reply",
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn deleting_a_source_cancels_mentions_and_search_never_crosses_room_access() {
    let Some(org) = company("room_search").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room search scenario");
        return;
    };
    let shared = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Incident review",
            &["owner", "exec"],
            "room-search-shared",
        )
        .await
        .unwrap();
    let private = org
        .create_room(
            "delivery-build",
            RoomKind::Group,
            "Private launch",
            &["delivery-build", "research-science"],
            "room-search-private",
        )
        .await
        .unwrap();
    assert!(org
        .search_rooms_for_actor("owner", "Private launch", None, 20)
        .await
        .unwrap()
        .rooms
        .is_empty());
    assert_eq!(
        org.search_rooms_for_actor("research-science", "Privat launch", None, 20)
            .await
            .unwrap()
            .rooms[0]
            .id,
        private.id
    );
    let mentioned = org
        .send_room_message_with_mentions(
            shared.id,
            "owner",
            "Please inspect the unusual semaphore failure",
            None,
            "room-search-mentioned",
            None,
            &[NewRoomMessageMention::actor("exec")],
            None,
        )
        .await
        .unwrap();
    org.send_room_message(
        private.id,
        "delivery-build",
        "The secret semaphore is heliotrope",
        None,
        "room-search-secret",
    )
    .await
    .unwrap();

    let owner_results = org
        .search_room_messages("owner", "semaphore", None, 1)
        .await
        .unwrap();
    assert_eq!(owner_results.messages.len(), 1);
    assert_eq!(owner_results.messages[0].id, mentioned.message.id);
    assert!(
        !owner_results.has_more,
        "inaccessible hits do not affect paging"
    );
    assert!(org
        .search_room_messages("research-science", "unusual", None, 20)
        .await
        .unwrap()
        .messages
        .is_empty());

    org.delete_room_message(
        shared.id,
        "owner",
        mentioned.message.id,
        "room-search-delete-mentioned",
    )
    .await
    .unwrap();
    let pending = org
        .pending_message_mentions_for_actor("exec", 0, 20)
        .await
        .unwrap();
    assert!(pending.is_empty());
    let room_page = org
        .room_messages_before("owner", shared.id, None, 20)
        .await
        .unwrap();
    assert!(room_page.mentions.is_empty());
    let events = org
        .room_events_after("exec", shared.id, 0, 500)
        .await
        .unwrap();
    assert!(events
        .events
        .iter()
        .any(|event| event.kind == "room.mention.cancelled.v1"));

    let outsider_history = org
        .room_message_revisions_before("delivery-build", shared.id, mentioned.message.id, None, 10)
        .await
        .unwrap_err();
    assert!(outsider_history.to_string().contains("access denied"));
}

#[tokio::test]
async fn concurrent_edits_have_one_revision_winner() {
    let Some(org) = company("room_edit_race").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room edit race");
        return;
    };
    let room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Concurrency",
            &["owner", "exec"],
            "room-edit-race-create",
        )
        .await
        .unwrap();
    let message = org
        .send_room_message(room.id, "exec", "Initial", None, "room-edit-race-message")
        .await
        .unwrap();
    let left_org = org.clone();
    let right_org = org.clone();
    let left = tokio::spawn(async move {
        left_org
            .edit_room_message(
                room.id,
                "exec",
                message.message.id,
                0,
                "Left revision",
                "room-edit-race-left",
            )
            .await
    });
    let right = tokio::spawn(async move {
        right_org
            .edit_room_message(
                room.id,
                "exec",
                message.message.id,
                0,
                "Right revision",
                "room-edit-race-right",
            )
            .await
    });
    let (left, right) = tokio::join!(left, right);
    let results = [left.unwrap(), right.unwrap()];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| result
                .as_ref()
                .is_err_and(|error| error.to_string().contains("current revision is 1")))
            .count(),
        1
    );
}

#[tokio::test]
async fn concurrent_room_commands_replay_and_claimed_replies_do_not_upgrade_locks() {
    let Some(org) = company("room_command_replay").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room command replay scenario");
        return;
    };
    let room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Command replay",
            &["owner", "exec"],
            "room-command-replay-create",
        )
        .await
        .unwrap();

    let left_org = org.clone();
    let right_org = org.clone();
    let left = tokio::spawn(async move {
        left_org
            .send_room_message(
                room.id,
                "exec",
                "One durable answer",
                None,
                "room-command-concurrent-send",
            )
            .await
    });
    let right = tokio::spawn(async move {
        right_org
            .send_room_message(
                room.id,
                "exec",
                "One durable answer",
                None,
                "room-command-concurrent-send",
            )
            .await
    });
    let sends = [left.await.unwrap().unwrap(), right.await.unwrap().unwrap()];
    assert_eq!(sends.iter().filter(|result| result.created).count(), 1);
    assert_eq!(sends[0].message.id, sends[1].message.id);

    org.remove_room_participant("owner", room.id, "exec")
        .await
        .unwrap();
    let lifecycle_retry = org
        .send_room_message(
            room.id,
            "exec",
            "One durable answer",
            None,
            "room-command-concurrent-send",
        )
        .await
        .unwrap();
    assert!(!lifecycle_retry.created);
    assert_eq!(lifecycle_retry.message.id, sends[0].message.id);

    let mention_room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Claim replay",
            &["owner", "exec"],
            "room-claim-replay-create",
        )
        .await
        .unwrap();
    org.send_room_message_with_mentions(
        mention_room.id,
        "owner",
        "Please make the call",
        None,
        "room-claim-replay-mention",
        None,
        &[NewRoomMessageMention::actor("exec")],
        None,
    )
    .await
    .unwrap();
    let lease = org
        .claim_actor_cognitive_session("exec", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    let claim = org
        .claim_next_pending_message_mention(&lease)
        .await
        .unwrap()
        .unwrap();
    let left_org = org.clone();
    let left_claim = claim.clone();
    let left = tokio::spawn(async move {
        left_org
            .reply_to_claimed_message_mention(&left_claim, "Proceed with evidence")
            .await
    });
    let right_org = org.clone();
    let right_claim = claim.clone();
    let right = tokio::spawn(async move {
        right_org
            .reply_to_claimed_message_mention(&right_claim, "Proceed with evidence")
            .await
    });
    let replies = [left.await.unwrap().unwrap(), right.await.unwrap().unwrap()];
    assert_eq!(replies.iter().filter(|result| result.created).count(), 1);
    assert_eq!(replies[0].message.id, replies[1].message.id);
}
