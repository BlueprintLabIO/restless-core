//! Document completeness (S65): asking for access reaches only the people who manage sharing and
//! never reveals a document; a comment is its author's to edit or take back; a resolved thread
//! reopens; history names the people who edited. Uses a disposable company schema when Postgres
//! is available.

use restless_orgintel::{
    DocumentAccess, DocumentCommentStatus, DocumentError, DocumentKind, DocumentVisibility,
    EditDocumentComment, ForYouItem, NewDocument, NewDocumentCommentThread, OrgIntel,
    ReplyToDocumentComment, ResolveDocumentCommentThread, SetDocumentParticipant,
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
    org.ensure_actor("exec", "exec", "exec", "Exec")
        .await
        .unwrap();
    for (id, display) in [("alex", "Alex"), ("bianca", "Bianca"), ("casey", "Casey")] {
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

async fn private_document(org: &OrgIntel, owner: &str) -> Uuid {
    org.create_document(NewDocument {
        command_id: Uuid::new_v4(),
        title: "Board notes",
        kind: DocumentKind::Plan,
        visibility: DocumentVisibility::Participants,
        linked_room_id: None,
        inherit_room_visibility: false,
        owner_actor_id: owner,
        created_by_actor_id: owner,
        content_json: &body("Notes"),
        reason: "First draft",
    })
    .await
    .unwrap()
    .document_id
}

async fn share(org: &OrgIntel, document_id: Uuid, by: &str, with: &str, access: DocumentAccess) {
    let version = org
        .get_document_for_actor(document_id, by)
        .await
        .unwrap()
        .document
        .version;
    org.set_document_participant(SetDocumentParticipant {
        command_id: Uuid::new_v4(),
        document_id,
        actor_id: by,
        expected_document_version: version,
        participant_actor_id: with,
        access,
    })
    .await
    .unwrap();
}

async fn asks(org: &OrgIntel, viewer: &str) -> Vec<ForYouItem> {
    org.for_you_page(viewer, None, 50)
        .await
        .unwrap()
        .items
        .into_iter()
        .filter(|item| item.kind.ends_with("access"))
        .collect()
}

#[tokio::test]
async fn an_ask_for_access_reaches_only_sharing_managers_and_closes_when_granted() {
    let Some((org, url)) = company("docask").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping access requests");
        return;
    };
    let notes = private_document(&org, "alex").await;
    share(&org, notes, "alex", "casey", DocumentAccess::Read).await;

    // Asking about a document that does not exist answers the same and stores nothing.
    org.request_document_access(Uuid::new_v4(), "bianca", DocumentAccess::Read)
        .await
        .unwrap();
    // Bianca cannot open Board notes, so she asks; asking twice is one ask.
    org.request_document_access(notes, "bianca", DocumentAccess::Read)
        .await
        .unwrap();
    org.request_document_access(notes, "bianca", DocumentAccess::Read)
        .await
        .unwrap();

    // Alex made it and manages it; the company's owner manages everything. Casey only reads it,
    // and Bianca's own list says nothing about it.
    let alex = asks(&org, "alex").await;
    assert_eq!(alex.len(), 1);
    assert_eq!(alex[0].kind, "access");
    assert_eq!(alex[0].actor_ids, ["bianca"]);
    assert!(alex[0].unread);
    assert!(asks(&org, "casey").await.is_empty());
    assert!(asks(&org, "bianca").await.is_empty());
    assert!(matches!(
        org.list_document_access_requests(notes, "casey").await,
        Err(DocumentError::Unavailable)
    ));
    let listed = org
        .list_document_access_requests(notes, "alex")
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].actor_id, "bianca");

    // Asking for what one already has is not an ask.
    org.request_document_access(notes, "casey", DocumentAccess::Read)
        .await
        .unwrap();
    assert_eq!(
        org.list_document_access_requests(notes, "alex")
            .await
            .unwrap()
            .len(),
        1
    );
    // Casey may read and asks to edit: that is an ask of its own.
    org.request_document_access(notes, "casey", DocumentAccess::Edit)
        .await
        .unwrap();
    let alex = asks(&org, "alex").await;
    assert!(alex
        .iter()
        .any(|item| item.kind == "edit-access" && item.actor_ids == ["casey"]));

    // Granting Bianca answers her ask; it leaves For you and the list.
    share(&org, notes, "alex", "bianca", DocumentAccess::Comment).await;
    let alex = asks(&org, "alex").await;
    assert!(alex.iter().all(|item| item.actor_ids != ["bianca"]));
    let listed = org
        .list_document_access_requests(notes, "alex")
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].actor_id, "casey");

    // Dismissed, Casey's ask is gone, and asking again the same day raises nothing: a manager is
    // not asked over and over. A day later, asking asks anew.
    org.dismiss_document_access_request(notes, listed[0].id, "alex")
        .await
        .unwrap();
    assert!(asks(&org, "alex").await.is_empty());
    assert!(matches!(
        org.dismiss_document_access_request(notes, listed[0].id, "alex")
            .await,
        Err(DocumentError::Unavailable)
    ));
    for _ in 0..3 {
        org.request_document_access(notes, "casey", DocumentAccess::Edit)
            .await
            .unwrap();
    }
    assert!(asks(&org, "alex").await.is_empty());
    let mut db = PgConnection::connect(&url).await.unwrap();
    sqlx::query(&format!(
        "SET search_path TO {}, pg_catalog, pg_temp",
        org.schema()
    ))
    .execute(&mut db)
    .await
    .unwrap();
    sqlx::query("UPDATE native_document_access_requests SET closed_at=closed_at-interval '2 days'")
        .execute(&mut db)
        .await
        .unwrap();
    org.request_document_access(notes, "casey", DocumentAccess::Edit)
        .await
        .unwrap();
    assert_eq!(asks(&org, "alex").await.len(), 1);
    // The company's owner manages every document's sharing and hears of asks for one they read.
    share(&org, notes, "alex", "owner", DocumentAccess::Read).await;
    assert!(asks(&org, "owner")
        .await
        .iter()
        .any(|item| item.kind == "edit-access" && item.actor_ids == ["casey"]));
    // Bianca may now comment, but she does not manage sharing, so Casey's ask is not hers.
    assert!(asks(&org, "bianca").await.is_empty());
}

#[tokio::test]
async fn a_comment_is_its_authors_to_edit_or_take_back_and_threads_reopen() {
    let Some((org, url)) = company("doccomment").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping comment edits");
        return;
    };
    let notes = private_document(&org, "alex").await;
    share(&org, notes, "alex", "bianca", DocumentAccess::Comment).await;
    share(&org, notes, "alex", "casey", DocumentAccess::Read).await;
    let started = org
        .create_document_comment_thread(NewDocumentCommentThread {
            document_id: notes,
            actor_id: "bianca",
            command_id: Uuid::new_v4(),
            block_id: None,
            content_json: &comment("Is this right?", None),
        })
        .await
        .unwrap();
    let thread = started.thread.thread.id;
    let first = started.first_comment.id;
    let edit = |actor: &'static str, text: &'static str| {
        let org = &org;
        async move {
            let content = comment(text, Some("alex"));
            org.edit_document_comment(EditDocumentComment {
                document_id: notes,
                thread_id: thread,
                comment_id: first,
                actor_id: actor,
                content_json: &content,
            })
            .await
        }
    };

    // Only Bianca edits her comment; the edit carries its mention and says it was edited.
    assert!(matches!(
        edit("alex", "Changed").await,
        Err(DocumentError::Unavailable)
    ));
    let edited = edit("bianca", "Is this figure right?").await.unwrap();
    assert!(edited.plain_text.starts_with("Is this figure right?"));
    assert_eq!(edited.mentioned_actor_ids, ["alex"]);
    assert!(edited.edited_at.is_some());
    assert_eq!(edited.created_at, started.first_comment.created_at);

    // Changing a comment any other way is still refused.
    let mut db = PgConnection::connect(&url).await.unwrap();
    sqlx::query(&format!(
        "SET search_path TO {}, pg_catalog, pg_temp",
        org.schema()
    ))
    .execute(&mut db)
    .await
    .unwrap();
    assert!(
        sqlx::query("UPDATE native_document_comments SET plain_text='x' WHERE id=$1")
            .bind(first)
            .execute(&mut db)
            .await
            .is_err()
    );
    assert!(sqlx::query(
        "UPDATE native_document_comments SET author_actor_id='alex',edited_at=now() WHERE id=$1"
    )
    .bind(first)
    .execute(&mut db)
    .await
    .is_err());

    // Resolved, the thread opens again for the person who started it, not for a reader.
    let resolved = org
        .resolve_document_comment_thread(ResolveDocumentCommentThread {
            document_id: notes,
            thread_id: thread,
            actor_id: "bianca",
            command_id: Uuid::new_v4(),
            expected_thread_version: 1,
        })
        .await
        .unwrap();
    assert!(matches!(
        org.reopen_document_comment_thread(notes, thread, "casey", resolved.version)
            .await,
        Err(DocumentError::Unavailable)
    ));
    let reopened = org
        .reopen_document_comment_thread(notes, thread, "bianca", resolved.version)
        .await
        .unwrap();
    assert_eq!(reopened.status, DocumentCommentStatus::Open);
    assert!(reopened.resolved_at.is_none());
    let reply = org
        .reply_to_document_comment(ReplyToDocumentComment {
            document_id: notes,
            thread_id: thread,
            reply_to_comment_id: None,
            actor_id: "alex",
            command_id: Uuid::new_v4(),
            content_json: &comment("Yes", None),
        })
        .await
        .unwrap();

    // Alex cannot take back Bianca's comment; Bianca can, and it leaves the thread. Taking back
    // the last comment takes the thread out of the list. The company's owner may take back any.
    assert!(matches!(
        org.delete_document_comment(notes, thread, first, "alex")
            .await,
        Err(DocumentError::Unavailable)
    ));
    org.delete_document_comment(notes, thread, first, "bianca")
        .await
        .unwrap();
    assert!(matches!(
        edit("bianca", "Again").await,
        Err(DocumentError::Unavailable)
    ));
    let left = org
        .list_document_comments(notes, thread, "alex", None, 50)
        .await
        .unwrap();
    assert_eq!(left.items.len(), 1);
    assert_eq!(left.items[0].id, reply.id);
    share(&org, notes, "alex", "owner", DocumentAccess::Read).await;
    org.delete_document_comment(notes, thread, reply.id, "owner")
        .await
        .unwrap();
    let threads = org
        .list_document_comment_threads(notes, "alex", None, 50)
        .await
        .unwrap();
    assert!(threads.items.is_empty());
    // Bianca hears nothing more of a comment that was taken back.
    let mentions = org.for_you_page("alex", None, 50).await.unwrap();
    assert!(mentions.items.iter().all(|item| item.kind != "mention"));
}

#[tokio::test]
async fn history_names_the_people_who_were_editing() {
    let Some((org, url)) = company("dochistory").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping version editors");
        return;
    };
    let notes = private_document(&org, "alex").await;
    let page = org
        .list_document_versions_for_actor(notes, "alex", None, 10)
        .await
        .unwrap();
    let version = &page.items[0];
    assert!(version.editor_actor_ids.is_empty());
    let mut db = PgConnection::connect(&url).await.unwrap();
    sqlx::query(&format!(
        "SET search_path TO {}, pg_catalog, pg_temp",
        org.schema()
    ))
    .execute(&mut db)
    .await
    .unwrap();
    for actor in ["bianca", "alex"] {
        sqlx::query(
            "INSERT INTO native_document_version_editors (document_id,version_id,actor_id) \
             VALUES ($1,$2,$3)",
        )
        .bind(notes)
        .bind(version.id)
        .bind(actor)
        .execute(&mut db)
        .await
        .unwrap();
    }
    let page = org
        .list_document_versions_for_actor(notes, "alex", None, 10)
        .await
        .unwrap();
    assert_eq!(page.items[0].editor_actor_ids, ["alex", "bianca"]);
}
