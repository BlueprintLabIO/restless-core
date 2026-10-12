//! Files in group rooms, colleague chats and thread replies (S65, G51): a person attaches files
//! they uploaded and kept; every current participant may then read them, and nobody else.
//! Uses a disposable company schema when Postgres is available.

use restless_orgintel::{OrgIntel, OwnerAttachmentRegistration, RoomKind};
use uuid::Uuid;

async fn company(prefix: &str) -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!("{prefix}{}", Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    for (id, name) in [("alex", "Alex"), ("blair", "Blair"), ("casey", "Casey")] {
        org.ensure_actor(id, "human", "member", name).await.unwrap();
    }
    Some(org)
}

/// Upload and keep one file as the Library upload route does.
async fn kept_file(org: &OrgIntel, sender: &str, name: &str) -> Uuid {
    let attachment_id = Uuid::new_v4();
    let command = format!("library-upload:{}", Uuid::new_v4());
    org.register_owner_attachments(
        sender,
        sender,
        &command,
        &"a".repeat(64),
        &[OwnerAttachmentRegistration {
            attachment_id,
            canonical_name: name.into(),
            canonical_media_type: "application/pdf".into(),
            size_bytes: 12,
            content_sha256: "b".repeat(64),
        }],
        24,
        64 << 20,
        512,
        1 << 30,
        128,
        256 << 20,
    )
    .await
    .unwrap();
    org.keep_owner_attachments(sender, &command, &[attachment_id])
        .await
        .unwrap();
    attachment_id
}

async fn readable(org: &OrgIntel, attachment_id: Uuid, actor: &str) -> bool {
    org.owner_attachment_for_actor(attachment_id, actor)
        .await
        .unwrap()
        .is_some()
}

#[tokio::test]
async fn a_room_message_carries_its_senders_files_to_participants_only() {
    let Some(org) = company("roomfiles").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping room attachment scenario");
        return;
    };
    let room = org
        .create_room(
            "alex",
            RoomKind::Group,
            "Launch",
            &["alex", "blair", "exec"],
            "room-files-create",
        )
        .await
        .unwrap();
    let plan = kept_file(&org, "alex", "plan.pdf").await;
    // Before it is sent, only its uploader may open it.
    assert!(readable(&org, plan, "alex").await);
    assert!(!readable(&org, plan, "blair").await);

    // Someone else cannot attach a file Alex uploaded.
    let refused = org
        .send_room_message_with_attachments(
            room.id,
            "blair",
            "Here it is",
            None,
            "room-files-borrowed",
            &[],
            None,
            &[plan],
        )
        .await;
    assert!(refused.is_err());

    let sent = org
        .send_room_message_with_attachments(
            room.id,
            "alex",
            "Attached: plan.pdf",
            None,
            "room-files-send",
            &[],
            None,
            &[plan],
        )
        .await
        .unwrap();
    assert!(sent.created);
    assert_eq!(sent.message.attachments.len(), 1);
    assert_eq!(sent.message.attachments[0].attachment_id, plan);
    assert_eq!(sent.message.attachments[0].name, "plan.pdf");

    // A retry of the same command replays; the same file cannot be attached again.
    let replay = org
        .send_room_message_with_attachments(
            room.id,
            "alex",
            "Attached: plan.pdf",
            None,
            "room-files-send",
            &[],
            None,
            &[plan],
        )
        .await
        .unwrap();
    assert!(!replay.created);
    assert_eq!(replay.message.id, sent.message.id);
    assert_eq!(replay.message.attachments.len(), 1);
    assert!(org
        .send_room_message_with_attachments(
            room.id,
            "alex",
            "Again",
            None,
            "room-files-again",
            &[],
            None,
            &[plan],
        )
        .await
        .is_err());

    // Every participant sees it on the message, and may open it.
    let page = org
        .room_messages_before("blair", room.id, None, 20)
        .await
        .unwrap();
    assert_eq!(page.messages.len(), 1);
    assert_eq!(page.messages[0].attachments.len(), 1);
    assert!(readable(&org, plan, "blair").await);

    // A thread reply carries files too.
    let notes = kept_file(&org, "blair", "notes.pdf").await;
    let reply = org
        .send_room_message_with_attachments(
            room.id,
            "blair",
            "My notes",
            Some(sent.message.id),
            "room-files-reply",
            &[],
            None,
            &[notes],
        )
        .await
        .unwrap();
    let thread = org
        .room_thread_before("alex", room.id, sent.message.id, None, 20)
        .await
        .unwrap();
    let shown = thread
        .messages
        .iter()
        .find(|message| message.id == reply.message.id)
        .unwrap();
    assert_eq!(shown.attachments[0].attachment_id, notes);
    assert!(readable(&org, notes, "alex").await);

    // Someone outside the room is refused.
    assert!(!readable(&org, plan, "casey").await);
    assert!(!readable(&org, notes, "casey").await);

    // A participant removed from the room is refused from then on; the uploader keeps theirs.
    org.remove_room_participant("alex", room.id, "blair")
        .await
        .unwrap();
    assert!(!readable(&org, plan, "blair").await);
    assert!(readable(&org, notes, "blair").await);
    assert!(readable(&org, plan, "alex").await);

    // The attached file still counts as retained, against the same limits and purge path.
    let retained = org
        .owner_attachment_retention_page("owner", None, 100, false)
        .await
        .unwrap();
    assert_eq!(retained.retained_files, 2);
    assert!(retained
        .attachments
        .iter()
        .all(|row| row.message_id.is_some()));
    assert!(org.claim_owner_attachment_gc(8, 1, 300).await.unwrap().is_empty());
}

#[tokio::test]
async fn a_deleted_message_shows_no_files_and_stops_opening_them_for_others() {
    let Some(org) = company("roomfilesdel").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping deleted attachment scenario");
        return;
    };
    let room = org
        .create_room(
            "alex",
            RoomKind::Group,
            "Ops",
            &["alex", "blair"],
            "room-files-del-create",
        )
        .await
        .unwrap();
    let file = kept_file(&org, "alex", "draft.pdf").await;
    let sent = org
        .send_room_message_with_attachments(
            room.id,
            "alex",
            "Draft",
            None,
            "room-files-del-send",
            &[],
            None,
            &[file],
        )
        .await
        .unwrap();
    org.delete_room_message(room.id, "alex", sent.message.id, "room-files-del-delete")
        .await
        .unwrap();
    let page = org
        .room_messages_before("blair", room.id, None, 20)
        .await
        .unwrap();
    assert!(page.messages[0].attachments.is_empty());
    assert!(!readable(&org, file, "blair").await);
}

#[tokio::test]
async fn the_exec_reads_a_direct_messages_files_as_paths() {
    let Some(org) = company("roomfilesexec").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping direct attachment scenario");
        return;
    };
    let room = org
        .create_room(
            "alex",
            RoomKind::Direct,
            "Alex and Exec",
            &["exec"],
            "room-files-direct-create",
        )
        .await
        .unwrap();
    let file = kept_file(&org, "alex", "brief.pdf").await;
    // A body that already names the file's path is not given the path twice.
    let named = format!("Read /var/lib/restless-owner-attachments/{file}/content");
    let sent = org
        .send_room_message_with_attachments(
            room.id,
            "alex",
            &named,
            None,
            "room-files-direct-send",
            &[],
            None,
            &[file],
        )
        .await
        .unwrap();
    let inbox = org.conversation_inbox("exec").await.unwrap();
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].body, named);
    // Once an edit drops the path, the Exec is given it again.
    org.edit_room_message(
        room.id,
        "alex",
        sent.message.id,
        0,
        "Please read this",
        "room-files-direct-edit",
    )
    .await
    .unwrap();
    let inbox = org.conversation_inbox("exec").await.unwrap();
    assert_eq!(inbox.len(), 1);
    assert!(inbox[0].body.starts_with("Please read this"));
    assert!(inbox[0].body.contains(&format!(
        "- brief.pdf: /var/lib/restless-owner-attachments/{file}/content"
    )));
}
