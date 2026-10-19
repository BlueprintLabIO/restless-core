//! For you (S65): who hears about mentions, replies, comments, shares, Room threads and changes;
//! that read state belongs to each person; and that an item disappears once its source is no
//! longer readable. Uses a disposable company schema when Postgres is available.

use chrono::{Duration, Utc};
use restless_orgintel::{
    CompanyAccessIdentity, DocumentAccess, DocumentError, DocumentKind, DocumentVisibility,
    ForYouCursor, ForYouItem, ForYouShow, ForYouTarget, HumanAccessContext, InboxMarkChange, NewDocument,
    NewDocumentCommentThread,
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
    assert_eq!(
        mention.target,
        ForYouTarget::Document {
            document_id: pricing,
            thread_id: Some(started)
        }
    );
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
    assert_eq!(items(&org, "alex").await.1, 2, "the mention, and being given the document");
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
        &[NewRoomMessageMention::actor("alex")],
        None,
    )
    .await
    .unwrap();
    let (alex, unread) = items(&org, "alex").await;
    assert_eq!(unread, 1);
    assert!(matches!(alex[0].target, ForYouTarget::Room { room_id, .. } if room_id == room.id));
    assert_eq!(alex[0].excerpt, "Can you check the launch copy?");
    assert!(items(&org, "bianca").await.0.iter().all(|item| !item.key.starts_with("room-mention:")));
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

    // Bianca anchors a comment while Alex's next live edit is unsaved. Her comment saves the live
    // body first, but the change is still Alex's: commenting is not changing the document.
    org.mark_for_you_read("owner", &[key.clone()]).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
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
    .bind(vec![1_u8, 2, 3, 4])
    .bind(body("Alex typed more"))
    .fetch_one(&mut db)
    .await
    .unwrap();
    assert_eq!(stored, "stored");
    // Bianca opens it to write as well but types nothing. The sidecar notes only whose edits it
    // kept, so she is not named as changing it (S66); an address with no write session is ignored.
    let consumed: bool = sqlx::query_scalar(
        "SELECT orgintel_native_document_collaboration_consume($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(company_id)
    .bind(pricing)
    .bind("bianca")
    .bind(Uuid::new_v4())
    .bind("write")
    .bind(Utc::now())
    .bind(Utc::now() + Duration::seconds(60))
    .fetch_one(&mut db)
    .await
    .unwrap();
    assert!(consumed);
    let noted: i32 =
        sqlx::query_scalar("SELECT orgintel_native_document_note_editors($1,$2,$3)")
            .bind(company_id)
            .bind(pricing)
            .bind(vec!["alex".to_string(), "mallory".to_string()])
            .fetch_one(&mut db)
            .await
            .unwrap();
    assert_eq!(noted, 1);
    org.create_document_comment_thread(NewDocumentCommentThread {
        document_id: pricing,
        actor_id: "bianca",
        command_id: Uuid::new_v4(),
        block_id: Some("opening"),
        content_json: &comment("Is this final?", None),
    })
    .await
    .unwrap();
    let (owner, _) = items(&org, "owner").await;
    let changed = find(&owner, &key).unwrap();
    assert!(changed.unread);
    assert_eq!(changed.actor_ids, ["alex"], "Bianca commented; Alex changed it");

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

async fn scratch_db(org: &OrgIntel, url: &str) -> PgConnection {
    let mut db = PgConnection::connect(url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}, pg_catalog, pg_temp", org.schema()))
        .execute(&mut db)
        .await
        .unwrap();
    db
}

/// Someone else's edit to a sheet, as the sheet engine records it.
async fn sheet_edit(db: &mut PgConnection, sheet: Uuid, actor: &str) {
    sqlx::query(
        "INSERT INTO native_sheet_messages (sheet_id,sequence,revision_id,actor_id,message) \
         SELECT $1,COALESCE(max(sequence),0)+1,$2,$3,'{}'::jsonb FROM native_sheet_messages \
         WHERE sheet_id=$1",
    )
    .bind(sheet)
    .bind(Uuid::new_v4().to_string())
    .bind(actor)
    .execute(db)
    .await
    .unwrap();
}

#[tokio::test]
async fn being_shared_a_document_or_sheet_reaches_only_that_person_while_they_have_it() {
    let Some((org, url)) = company("foryoushare").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping For you share scenario");
        return;
    };
    let board = document(&org, "Board notes", DocumentVisibility::Participants).await;
    org.set_document_participant(SetDocumentParticipant {
        command_id: Uuid::new_v4(),
        document_id: board,
        actor_id: "owner",
        expected_document_version: 1,
        participant_actor_id: "alex",
        access: DocumentAccess::Comment,
    })
    .await
    .unwrap();
    let key = format!("share:{board}");
    let (alex, unread) = items(&org, "alex").await;
    let shared = find(&alex, &key).expect("Alex hears he was given the document");
    assert_eq!((shared.kind.as_str(), shared.excerpt.as_str()), ("share", "Can comment"));
    assert_eq!(shared.actor_names, ["The Owner"]);
    assert_eq!(shared.target, ForYouTarget::Document { document_id: board, thread_id: None });
    assert_eq!(unread, 1);
    assert!(find(&items(&org, "owner").await.0, &key).is_none(), "making it is not being shared it");
    assert!(find(&items(&org, "bianca").await.0, &key).is_none());

    // A sheet shared with Alex, and with Exec, who takes part like a person.
    let sheet = Uuid::new_v4();
    org.create_sheet(sheet, "owner", "Deals", "participants", &json!({"revisionId": "r0"}))
        .await
        .unwrap();
    org.share_sheet(sheet, "owner", "alex", Some("edit")).await.unwrap();
    let sheet_key = format!("sheet-share:{sheet}");
    let (alex, _) = items(&org, "alex").await;
    let shared = find(&alex, &sheet_key).expect("Alex hears he was given the sheet");
    assert_eq!(shared.target, ForYouTarget::Sheet { sheet_id: sheet, thread_id: None });
    assert_eq!(shared.excerpt, "Can edit");
    assert!(find(&items(&org, "bianca").await.0, &sheet_key).is_none());

    // Changes others make to a sheet he follows reach him; his own never do.
    let mut db = scratch_db(&org, &url).await;
    sheet_edit(&mut db, sheet, "alex").await;
    assert!(org.sheet_following(sheet, "alex").await.unwrap(), "editing follows a sheet");
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    sheet_edit(&mut db, sheet, "owner").await;
    let changes = format!("sheet-changes:{sheet}");
    let (alex, _) = items(&org, "alex").await;
    let changed = find(&alex, &changes).expect("the owner's edit reaches Alex");
    assert_eq!((changed.kind.as_str(), changed.actor_ids.clone()), ("change", vec!["owner".to_string()]));
    let (owner, _) = items(&org, "owner").await;
    assert!(find(&owner, &changes).is_some_and(|item| item.actor_ids == ["alex"]));
    org.set_sheet_follow(sheet, "alex", false).await.unwrap();
    assert!(find(&items(&org, "alex").await.0, &changes).is_none(), "leaving a sheet quiets it");
    // Bianca cannot follow a sheet she cannot open, nor learn it exists.
    assert!(matches!(
        org.set_sheet_follow(sheet, "bianca", true).await,
        Err(DocumentError::Unavailable)
    ));

    // Losing access takes the items with it.
    org.share_sheet(sheet, "owner", "alex", None).await.unwrap();
    org.remove_document_participant(RemoveDocumentParticipant {
        command_id: Uuid::new_v4(),
        document_id: board,
        actor_id: "owner",
        expected_document_version: 2,
        participant_actor_id: "alex",
    })
    .await
    .unwrap();
    let (alex, unread) = items(&org, "alex").await;
    assert!(alex.is_empty(), "nothing is left from what Alex can no longer open");
    assert_eq!(unread, 0);
}

#[tokio::test]
async fn replies_in_a_room_thread_reach_the_people_in_it() {
    let Some((org, _)) = company("foryouroomthread").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping For you Room thread scenario");
        return;
    };
    let room = org
        .create_room("owner", RoomKind::Group, "Launch", &["owner", "alex", "bianca", "exec"], "launch")
        .await
        .unwrap();
    let root = org
        .send_room_message(room.id, "alex", "Shall we ship Friday?", None, "root")
        .await
        .unwrap()
        .message
        .id;
    let answer = org
        .send_room_message(room.id, "exec", "Friday works.", Some(root), "answer")
        .await
        .unwrap()
        .message
        .id;
    let key = format!("room-thread:{}:{root}", room.id);
    let (alex, unread) = items(&org, "alex").await;
    let replied = find(&alex, &key).expect("Alex started the thread");
    assert_eq!(replied.kind, "reply");
    assert_eq!(replied.actor_names, ["Exec"]);
    assert_eq!(replied.excerpt, "Friday works.");
    assert!(matches!(
        replied.target,
        ForYouTarget::Room { thread_root_message_id, message_id, mention_id: None, .. }
            if thread_root_message_id == root && message_id == answer
    ));
    assert_eq!(unread, 1);
    assert!(find(&items(&org, "bianca").await.0, &key).is_none(), "Bianca is not in the thread");
    assert!(find(&items(&org, "exec").await.0, &key).is_none(), "Exec's own reply is not news to it");

    // A reply that mentions Alex is his mention, not also a thread item.
    org.mark_for_you_read("alex", &[key.clone()]).await.unwrap();
    org.send_room_message_with_mentions(
        room.id,
        "bianca",
        "Alex, can you confirm?",
        Some(root),
        "confirm",
        &[NewRoomMessageMention::actor("alex")],
        None,
    )
    .await
    .unwrap();
    let (alex, unread) = items(&org, "alex").await;
    assert_eq!(unread, 1);
    assert!(!find(&alex, &key).unwrap().unread);
    assert!(alex.iter().any(|item| item.unread && item.key.starts_with("room-mention:")));

    // Reading the conversation in the Room reads its thread here too.
    let late = org
        .send_room_message(room.id, "exec", "Booked.", Some(root), "booked")
        .await
        .unwrap()
        .message
        .id;
    assert!(find(&items(&org, "alex").await.0, &key).unwrap().unread);
    org.mark_room_read_through(room.id, "alex", late).await.unwrap();
    assert!(!find(&items(&org, "alex").await.0, &key).unwrap().unread);

    // Leaving the Room takes the thread with it.
    org.remove_room_participant("owner", room.id, "alex").await.unwrap();
    assert!(find(&items(&org, "alex").await.0, &key).is_none());
}

#[tokio::test]
async fn marks_are_per_person_unread_wins_over_all_read_and_archive_lasts_until_news() {
    let Some((org, _)) = company("foryoumarks").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping For you marks scenario");
        return;
    };
    let pricing = document(&org, "Pricing", DocumentVisibility::Company).await;
    let started = thread(&org, pricing, "alex", None).await;
    reply(&org, pricing, started, "bianca").await;
    let key = format!("thread:{started}");
    assert!(find(&items(&org, "alex").await.0, &key).unwrap().unread);

    // Marked unread, it stays unread past "mark all read" until it is opened.
    org.mark_all_for_you_read("alex").await.unwrap();
    org.mark_for_you_unread("alex", &[key.clone()]).await.unwrap();
    let page = org.for_you_page_filtered("alex", None, 50, true).await.unwrap();
    assert_eq!(page.items.iter().map(|item| item.key.as_str()).collect::<Vec<_>>(), [key.as_str()]);
    assert_eq!(page.unread_count, 1);
    assert!(find(&items(&org, "owner").await.0, &key).unwrap().unread, "Alex's marks are his own");
    org.mark_all_for_you_read("alex").await.unwrap();
    assert_eq!(items(&org, "alex").await.1, 0, "mark all read clears a marked-unread item");
    org.mark_for_you_unread("alex", &[key.clone()]).await.unwrap();
    org.mark_for_you_read("alex", &[key.clone()]).await.unwrap();
    assert_eq!(items(&org, "alex").await.1, 0, "opening it reads it");

    // Archived, it leaves the list; unarchived it comes back; news brings it back by itself.
    org.archive_for_you("alex", &[key.clone()], true).await.unwrap();
    assert!(find(&items(&org, "alex").await.0, &key).is_none());
    let archived = |org: &OrgIntel| {
        let org = org.clone();
        async move {
            org.for_you_page_showing("alex", None, 50, ForYouShow::Archived)
                .await
                .unwrap()
                .items
        }
    };
    assert!(find(&archived(&org).await, &key).is_some(), "Archived shows it");
    assert!(org
        .for_you_page_showing("bianca", None, 50, ForYouShow::Archived)
        .await
        .unwrap()
        .items
        .is_empty(), "Alex's archive is Alex's own");
    org.mark_all_for_you_read("alex").await.unwrap();
    assert!(find(&items(&org, "alex").await.0, &key).is_none(), "mark all read keeps it archived");
    org.archive_for_you("alex", &[key.clone()], false).await.unwrap();
    assert!(find(&items(&org, "alex").await.0, &key).is_some_and(|item| !item.unread));
    assert!(find(&archived(&org).await, &key).is_none(), "unarchived leaves Archived");
    // Undo puts back exactly what was archived: an unread item comes back unread.
    org.mark_for_you_unread("alex", &[key.clone()]).await.unwrap();
    org.archive_for_you("alex", &[key.clone()], true).await.unwrap();
    assert_eq!(items(&org, "alex").await.1, 0, "an archived item is not counted");
    org.archive_for_you("alex", &[key.clone()], false).await.unwrap();
    assert!(find(&items(&org, "alex").await.0, &key).is_some_and(|item| item.unread));
    org.mark_for_you_read("alex", &[key.clone()]).await.unwrap();
    org.archive_for_you("alex", &[key.clone()], true).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    reply(&org, pricing, started, "bianca").await;
    let (alex, unread) = items(&org, "alex").await;
    assert!(find(&alex, &key).is_some_and(|item| item.unread), "a new reply resurfaces it");
    assert!(find(&archived(&org).await, &key).is_none(), "news takes it out of Archived");
    assert_eq!(unread, 1);

    // Malformed marks are refused; a Room thread mark names its Room and root.
    assert!(matches!(
        org.archive_for_you("alex", &["attention:x".into()], true).await,
        Err(DocumentError::Invalid(_))
    ));
    org.mark_for_you_read("alex", &[format!("room-thread:{}:7", Uuid::new_v4())])
        .await
        .unwrap();
}

#[tokio::test]
async fn inbox_marks_follow_the_person_and_only_that_person() {
    let Some((org, _)) = company("foryouinbox").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Inbox marks scenario");
        return;
    };
    let until = Utc::now() + Duration::hours(1);
    org.set_inbox_marks(
        "owner",
        &[InboxMarkChange {
            item_id: "handoff:1".into(),
            seen_stamp: Some("2026-10-13T00:00:00Z".into()),
            snoozed_until: Some(Some(until)),
        }],
    )
    .await
    .unwrap();
    // Seeing it again leaves the snooze alone; waking it clears only the snooze.
    org.set_inbox_marks(
        "owner",
        &[InboxMarkChange {
            item_id: "handoff:1".into(),
            seen_stamp: Some("2026-10-13T01:00:00Z".into()),
            snoozed_until: None,
        }],
    )
    .await
    .unwrap();
    let marks = org.inbox_marks("owner").await.unwrap();
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].seen_stamp.as_deref(), Some("2026-10-13T01:00:00Z"));
    assert_eq!(marks[0].snoozed_until.map(|at| at.timestamp()), Some(until.timestamp()));
    org.set_inbox_marks(
        "owner",
        &[InboxMarkChange {
            item_id: "handoff:1".into(),
            seen_stamp: None,
            snoozed_until: Some(None),
        }],
    )
    .await
    .unwrap();
    let marks = org.inbox_marks("owner").await.unwrap();
    assert_eq!(marks[0].snoozed_until, None);
    assert_eq!(marks[0].seen_stamp.as_deref(), Some("2026-10-13T01:00:00Z"));
    assert!(org.inbox_marks("alex").await.unwrap().is_empty(), "marks are the owner's own");
    assert!(org.set_inbox_marks("owner", &[]).await.is_err());
}

#[tokio::test]
async fn a_document_mention_still_unread_after_a_while_is_owed_an_email_until_it_is_read() {
    let Some((org, _)) = company("foryouemail").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping For you email scenario");
        return;
    };
    let identity = CompanyAccessIdentity {
        company_id: Uuid::new_v4(),
        cell_id: Uuid::new_v4(),
    };
    org.ensure_company_access_identity(identity).await.unwrap();
    let now = Utc::now();
    let binding = org
        .consume_human_access_context(HumanAccessContext {
            display_name: None,
            issuer: "https://fleet.example.test",
            subject: "member-user",
            company_id: identity.company_id,
            cell_id: identity.cell_id,
            membership_id: "member-membership",
            membership_role: "member",
            membership_version: 1,
            assertion_id: Uuid::new_v4(),
            issued_at: now,
            expires_at: now + Duration::minutes(5),
        })
        .await
        .unwrap();
    let member = binding.actor_id.as_str();
    let pricing = document(&org, "Pricing", DocumentVisibility::Company).await;
    let started = thread(&org, pricing, "bianca", Some(member)).await;
    // Fresh, it waits for the cockpit.
    assert!(org.pending_human_for_you_notifications(Utc::now()).await.unwrap().is_empty());
    let later = Utc::now() + Duration::minutes(15);
    let owed = org.pending_human_for_you_notifications(later).await.unwrap();
    assert_eq!(owed.len(), 1, "only the hosted person is owed; the local owner has no binding");
    assert_eq!(owed[0].recipient.actor_id, member);
    assert!(owed[0].item.key.starts_with("doc-mention:"));
    assert_eq!(owed[0].item.target, ForYouTarget::Document { document_id: pricing, thread_id: Some(started) });
    let _ = started;
    // History is not news.
    assert!(org
        .pending_human_for_you_notifications(Utc::now() + Duration::days(3))
        .await
        .unwrap()
        .is_empty());
    // Opened in the cockpit, it is withdrawn.
    org.mark_for_you_read(member, &[owed[0].item.key.clone()]).await.unwrap();
    assert!(org.pending_human_for_you_notifications(later).await.unwrap().is_empty());
    // Marking it unread again is a reminder to themselves, never an email.
    org.mark_for_you_unread(member, &[owed[0].item.key.clone()]).await.unwrap();
    assert!(org.pending_human_for_you_notifications(later).await.unwrap().is_empty());
}
