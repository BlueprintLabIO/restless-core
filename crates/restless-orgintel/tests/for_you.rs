//! For you (S65): who hears about mentions, replies, comments and changes; that read state belongs
//! to each person; and that an item disappears once its source is no longer readable. Uses a
//! disposable company schema when Postgres is available.

use chrono::{Duration, Utc};
use restless_orgintel::{
    CompanyAccessIdentity, DocumentAccess, DocumentError, DocumentKind, DocumentVisibility,
    ForYouCursor, ForYouItem, ForYouTarget, NewDocument, NewDocumentCommentThread,
    NewNamedDocumentVersion, NewRoomMessageMention, OrgIntel, RemoveDocumentParticipant,
    ReplyToDocumentComment, RoomKind, SetDocumentParticipant,
};
use serde_json::{json, Value};
use sqlx::{Connection as _, PgConnection};
use uuid::Uuid;

async fn company(prefix: &str) -> Option<(OrgIntel, String)> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!("{prefix}{}", Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "Exec").await.unwrap();
    for (id, display) in [("alex", "Alex"), ("bianca", "Bianca")] {
        org.ensure_actor(id, "human", "member", display)
            .await
            .unwrap();
    }
    Some((org, url))
}

fn body(text: &str) -> Value {
    json!({"type": "doc", "content": [{
        "type": "paragraph", "attrs": {"block_id": "opening"},
        "content": [{"type": "text", "text": text}]
    }]})
}

fn comment(text: &str, mention: Option<&str>) -> Value {
    let mut inline = vec![json!({"type": "text", "text": text})];
    if let Some(actor_id) = mention {
        inline.push(json!({"type": "mention", "attrs": {"actor_id": actor_id, "label": actor_id}}));
    }
    json!({"type": "doc", "content": [{
        "type": "paragraph", "attrs": {"block_id": "c"}, "content": inline
    }]})
}

async fn document(org: &OrgIntel, title: &str, visibility: DocumentVisibility) -> Uuid {
    org.create_document(NewDocument {
        command_id: Uuid::new_v4(),
        title,
        kind: DocumentKind::Plan,
        visibility,
        linked_room_id: None,
        inherit_room_visibility: false,
        owner_actor_id: "owner",
        created_by_actor_id: "owner",
        content_json: &body("A start"),
        reason: "First draft",
    })
    .await
    .unwrap()
    .document_id
}

async fn thread(org: &OrgIntel, document_id: Uuid, author: &str, mention: Option<&str>) -> Uuid {
    org.create_document_comment_thread(NewDocumentCommentThread {
        document_id,
        actor_id: author,
        command_id: Uuid::new_v4(),
        block_id: None,
        content_json: &comment("What about this?", mention),
    })
    .await
    .unwrap()
    .thread
    .thread
    .id
}

async fn reply(org: &OrgIntel, document_id: Uuid, thread_id: Uuid, author: &str) {
    org.reply_to_document_comment(ReplyToDocumentComment {
        document_id,
        thread_id,
        reply_to_comment_id: None,
        actor_id: author,
        command_id: Uuid::new_v4(),
        content_json: &comment("Agreed", None),
    })
    .await
    .unwrap();
}

async fn version(org: &OrgIntel, document_id: Uuid, author: &str, text: &str) {
    let current = org
        .get_document_for_actor(document_id, author)
        .await
        .unwrap()
        .document
        .current_named_version_id;
    org.create_named_document_version(NewNamedDocumentVersion {
        command_id: Uuid::new_v4(),
        document_id,
        actor_id: author,
        expected_current_version_id: current,
        content_json: &body(text),
        reason: "Tightened",
    })
    .await
    .unwrap();
}

async fn items(org: &OrgIntel, viewer: &str) -> (Vec<ForYouItem>, i64) {
    let page = org.for_you_page(viewer, None, 50).await.unwrap();
    (page.items, page.unread_count)
}

fn find<'a>(items: &'a [ForYouItem], key: &str) -> Option<&'a ForYouItem> {
    items.iter().find(|item| item.key == key)
}

#[tokio::test]
async fn mentions_replies_and_comments_reach_the_right_people_with_read_state_per_person() {
    let Some((org, _)) = company("foryoureach").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping For you reach scenario");
        return;
    };
    // The owner made Pricing, so the owner follows it.
    let pricing = document(&org, "Pricing", DocumentVisibility::Company).await;
    let started = thread(&org, pricing, "bianca", Some("alex")).await;

    // Alex was mentioned; the owner follows the document; Bianca acted and hears nothing.
    let (alex, alex_unread) = items(&org, "alex").await;
    let mention = alex
        .iter()
        .find(|item| item.kind == "mention")
        .expect("Alex hears about the mention");
    assert!(mention.unread);
    assert_eq!(mention.actor_ids, ["bianca"]);
    assert_eq!(mention.actor_names, ["Bianca"]);
    assert_eq!(mention.title, "Pricing");
    assert_eq!(mention.target, ForYouTarget::Document { document_id: pricing });
    assert_eq!(alex_unread, 1, "the mention is not also a thread item");
    let thread_key = format!("thread:{started}");
    let (owner, owner_unread) = items(&org, "owner").await;
    let commented = find(&owner, &thread_key).expect("the owner follows Pricing");
    assert_eq!(commented.kind, "comment");
    assert!(commented.unread);
    assert_eq!(owner_unread, 1);
    assert!(items(&org, "bianca").await.0.is_empty());
    assert!(items(&org, "exec").await.0.is_empty());

    // Alex answers in the thread: his mention is settled, and Bianca hears the reply.
    reply(&org, pricing, started, "alex").await;
    let (alex, alex_unread) = items(&org, "alex").await;
    assert_eq!(alex_unread, 0, "answering a mention settles it");
    assert!(alex.iter().all(|item| !item.unread));
    let (bianca, _) = items(&org, "bianca").await;
    let replied = find(&bianca, &thread_key).expect("Bianca started the thread");
    assert_eq!(replied.kind, "reply");
    assert_eq!(replied.actor_ids, ["alex"]);
    // The owner's one item for the thread coalesces both people.
    let (owner, _) = items(&org, "owner").await;
    assert_eq!(
        find(&owner, &thread_key).unwrap().actor_ids,
        ["alex", "bianca"]
    );

    // Reading is per person and per item.
    org.mark_for_you_read("bianca", &[thread_key.clone()])
        .await
        .unwrap();
    assert!(!find(&items(&org, "bianca").await.0, &thread_key).unwrap().unread);
    assert!(find(&items(&org, "owner").await.0, &thread_key).unwrap().unread);

    // Mark all read is a cursor; anything newer is unread again, naming only who came after.
    org.mark_all_for_you_read("owner").await.unwrap();
    assert_eq!(items(&org, "owner").await.1, 0);
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    reply(&org, pricing, started, "bianca").await;
    let (owner, owner_unread) = items(&org, "owner").await;
    assert_eq!(owner_unread, 1);
    assert_eq!(find(&owner, &thread_key).unwrap().actor_ids, ["bianca"]);
    // Bianca's own reply is not news to her.
    assert!(!find(&items(&org, "bianca").await.0, &thread_key).unwrap().unread);

    // Explicitly leaving a document stops new threads on it; a thread you are in still reaches you.
    org.set_document_follow(pricing, "owner", false).await.unwrap();
    let later = thread(&org, pricing, "bianca", None).await;
    let (owner, _) = items(&org, "owner").await;
    assert!(find(&owner, &format!("thread:{later}")).is_none());
    assert!(!org.document_follow_state(pricing, "owner").await.unwrap().following);
    org.set_document_follow(pricing, "owner", true).await.unwrap();
    assert!(find(&items(&org, "owner").await.0, &format!("thread:{later}")).is_some());

    // Malformed read marks are refused.
    assert!(matches!(
        org.mark_for_you_read("owner", &["anything:else".into()]).await,
        Err(DocumentError::Invalid(_))
    ));
}

#[tokio::test]
async fn an_item_disappears_once_its_source_is_no_longer_readable() {
    let Some((org, _)) = company("foryouaccess").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping For you access scenario");
        return;
    };
    let private = document(&org, "Board notes", DocumentVisibility::Participants).await;
    for (person, version) in [("alex", 1), ("bianca", 2)] {
        org.set_document_participant(SetDocumentParticipant {
            command_id: Uuid::new_v4(),
            document_id: private,
            actor_id: "owner",
            expected_document_version: version,
            participant_actor_id: person,
            access: DocumentAccess::Comment,
        })
        .await
        .unwrap();
    }
    thread(&org, private, "bianca", Some("alex")).await;
    assert_eq!(items(&org, "alex").await.1, 1);
    // Someone who cannot read the document cannot follow it or learn it exists.
    assert!(matches!(
        org.set_document_follow(private, "exec", true).await,
        Err(DocumentError::Unavailable)
    ));
    assert!(matches!(
        org.document_follow_state(private, "exec").await,
        Err(DocumentError::Unavailable)
    ));

    org.remove_document_participant(RemoveDocumentParticipant {
        command_id: Uuid::new_v4(),
        document_id: private,
        actor_id: "owner",
        expected_document_version: 3,
        participant_actor_id: "alex",
    })
    .await
    .unwrap();
    let (alex, unread) = items(&org, "alex").await;
    assert!(alex.is_empty(), "the mention went with access to its document");
    assert_eq!(unread, 0);

    // A Room mention reaches its person only while they are in the Room.
    let room = org
        .create_room("owner", RoomKind::Group, "Launch", &["owner", "alex", "bianca"], "launch")
        .await
        .unwrap();
    org.send_room_message_with_mentions(
        room.id,
        "bianca",
        "Can you check the launch copy?",
        None,
        "launch-ask",
        None,
        &[NewRoomMessageMention::actor("alex")],
        None,
    )
    .await
    .unwrap();
    let (alex, unread) = items(&org, "alex").await;
    assert_eq!(unread, 1);
    assert!(matches!(alex[0].target, ForYouTarget::Room { room_id, .. } if room_id == room.id));
    assert_eq!(alex[0].excerpt, "Can you check the launch copy?");
    assert!(items(&org, "bianca").await.0.is_empty());
    org.remove_room_participant("owner", room.id, "alex")
        .await
        .unwrap();
    assert!(items(&org, "alex").await.0.is_empty());
}

#[tokio::test]
async fn changes_by_others_coalesce_per_document_and_say_who_changed_it() {
    let Some((org, url)) = company("foryouchanges").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping For you changes scenario");
        return;
    };
    let company_id = Uuid::new_v4();
    org.ensure_company_access_identity(CompanyAccessIdentity {
        company_id,
        cell_id: Uuid::new_v4(),
    })
    .await
    .unwrap();
    let pricing = document(&org, "Pricing", DocumentVisibility::Company).await;
    let key = format!("changes:{pricing}");
    // Exec edits what it is given to edit, as a person would.
    org.set_document_participant(SetDocumentParticipant {
        command_id: Uuid::new_v4(),
        document_id: pricing,
        actor_id: "owner",
        expected_document_version: 1,
        participant_actor_id: "exec",
        access: DocumentAccess::Edit,
    })
    .await
    .unwrap();

    version(&org, pricing, "bianca", "Bianca's prices").await;
    version(&org, pricing, "exec", "Exec's prices").await;
    let (owner, unread) = items(&org, "owner").await;
    let changed = find(&owner, &key).expect("one item for the document");
    assert_eq!(unread, 1);
    assert_eq!(changed.kind, "change");
    assert_eq!(changed.actor_ids, ["bianca", "exec"]);
    let notice = org
        .document_follow_state(pricing, "owner")
        .await
        .unwrap()
        .changed_since_you_looked
        .expect("the document says it changed");
    assert_eq!(notice.actor_names, ["Bianca", "Exec"]);
    // Bianca follows by editing; Exec's later change reaches her, her own does not.
    let (bianca, _) = items(&org, "bianca").await;
    assert_eq!(find(&bianca, &key).unwrap().actor_ids, ["exec"]);

    // Opening the document reads it.
    org.mark_for_you_read("owner", &[key.clone()]).await.unwrap();
    let state = org.document_follow_state(pricing, "owner").await.unwrap();
    assert!(state.following);
    assert!(state.changed_since_you_looked.is_none());
    assert!(!find(&items(&org, "owner").await.0, &key).unwrap().unread);
    version(&org, pricing, "owner", "The owner's prices").await;
    assert_eq!(items(&org, "owner").await.1, 0, "your own change is not news");

    // A live edit saved by the daemon is credited to whoever held a write session; the person
    // editing does not hear about it.
    let mut db = PgConnection::connect(&url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}, pg_catalog, pg_temp", org.schema()))
        .execute(&mut db)
        .await
        .unwrap();
    let now = Utc::now();
    let consumed: bool = sqlx::query_scalar(
        "SELECT orgintel_native_document_collaboration_consume($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(company_id)
    .bind(pricing)
    .bind("alex")
    .bind(Uuid::new_v4())
    .bind("write")
    .bind(now)
    .bind(now + Duration::seconds(60))
    .fetch_one(&mut db)
    .await
    .unwrap();
    assert!(consumed);
    let (revision, checkpoint): (i64, Uuid) = sqlx::query_as(
        "SELECT state_revision,checkpoint_named_version_id FROM orgintel_native_document_yjs_load($1,$2)",
    )
    .bind(company_id)
    .bind(pricing)
    .fetch_one(&mut db)
    .await
    .unwrap();
    let stored: String = sqlx::query_scalar(
        "SELECT outcome FROM orgintel_native_document_yjs_store($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(company_id)
    .bind(pricing)
    .bind(revision)
    .bind(checkpoint)
    .bind(Uuid::new_v4())
    .bind(vec![1_u8, 2, 3])
    .bind(body("Alex typed this live"))
    .fetch_one(&mut db)
    .await
    .unwrap();
    assert_eq!(stored, "stored");
    sqlx::query(
        "UPDATE native_document_yjs_state SET persisted_at=now()-interval '11 seconds' WHERE document_id=$1",
    )
    .bind(pricing)
    .execute(&mut db)
    .await
    .unwrap();
    assert_eq!(org.snapshot_document_history(20).await.unwrap(), 1);
    let (owner, unread) = items(&org, "owner").await;
    assert_eq!(unread, 1);
    assert_eq!(find(&owner, &key).unwrap().actor_ids, ["alex"]);
    assert_eq!(find(&owner, &key).unwrap().actor_names, ["Alex"]);
    let (alex, _) = items(&org, "alex").await;
    assert!(
        find(&alex, &key).is_none_or(|item| !item.actor_ids.contains(&"alex".to_string())),
        "Alex is never told about his own edit"
    );

    // Pages walk back by cursor without repeating an item.
    let first = org.for_you_page("owner", None, 1).await.unwrap();
    assert_eq!(first.items.len(), 1);
    let cursor = first.next_before.clone();
    if let Some(cursor) = cursor {
        let encoded = cursor.encode();
        let parsed = ForYouCursor::parse(&encoded).unwrap();
        assert_eq!(parsed, cursor);
        let next = org.for_you_page("owner", Some(&parsed), 1).await.unwrap();
        assert!(next.items.iter().all(|item| item.key != first.items[0].key));
    }
    assert!(ForYouCursor::parse("not a cursor").is_none());

    // Follows and editors go with the document when it is deleted for good.
    org.set_document_follow(pricing, "alex", true).await.unwrap();
    org.set_document_archived(pricing, "owner", true).await.unwrap();
    assert!(find(&items(&org, "owner").await.0, &key).is_none(), "archived documents fall quiet");
    org.delete_archived_document(pricing, "owner").await.unwrap();
    let left: i64 = sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM native_document_follows WHERE document_id=$1) \
              + (SELECT count(*) FROM native_document_version_editors WHERE document_id=$1)",
    )
    .bind(pricing)
    .fetch_one(&mut db)
    .await
    .unwrap();
    assert_eq!(left, 0);
}
