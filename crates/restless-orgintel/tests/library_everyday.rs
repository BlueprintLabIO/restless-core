//! Everyday Library boundaries (S64): a file the owner keeps without a message survives staging
//! collection and is readable only by them; an archived document is deleted for good only as a
//! whole and only once archived. Uses a disposable company schema when Postgres is available.

use restless_orgintel::{
    DocumentError, DocumentKind, DocumentVisibility, NewDocument, NewDocumentCommentThread,
    NewNamedDocumentVersion, OrgIntel, OwnerAttachmentRegistration,
};
use serde_json::{json, Value};
use sqlx::{Connection as _, PgConnection};
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
    org.ensure_actor("alex", "human", "member", "Alex")
        .await
        .unwrap();
    Some(org)
}

async fn schema_connection(org: &OrgIntel) -> PgConnection {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let mut connection = PgConnection::connect(&url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", org.schema()))
        .execute(&mut connection)
        .await
        .unwrap();
    connection
}

fn content(block_id: &str, text: &str) -> Value {
    json!({
        "type": "doc",
        "content": [{
            "type": "paragraph",
            "attrs": {"block_id": block_id},
            "content": [{"type": "text", "text": text}]
        }]
    })
}

#[tokio::test]
async fn a_kept_file_outlives_staging_collection_and_opens_only_for_its_sender() {
    let Some(org) = company("librarykept").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping kept file scenario");
        return;
    };
    let attachment_id = Uuid::new_v4();
    let command = format!("library-upload:{}", Uuid::new_v4());
    org.register_owner_attachments(
        "owner",
        "owner",
        &command,
        &"a".repeat(64),
        &[OwnerAttachmentRegistration {
            attachment_id,
            canonical_name: "price-list.pdf".into(),
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
    // Another command cannot keep a stage it did not register.
    assert!(org
        .keep_owner_attachments("owner", "library-upload:other", &[attachment_id])
        .await
        .is_err());
    org.keep_owner_attachments("owner", &command, &[attachment_id])
        .await
        .unwrap();

    // Age the row past the staging window: an unlinked stage would now be collected.
    let mut connection = schema_connection(&org).await;
    sqlx::query("UPDATE owner_attachments SET created_at=now()-interval '2 hours' WHERE attachment_id=$1")
        .bind(attachment_id)
        .execute(&mut connection)
        .await
        .unwrap();
    assert!(org
        .claim_owner_attachment_gc(8, 3600, 300)
        .await
        .unwrap()
        .is_empty());

    let page = org
        .owner_attachment_retention_page("owner", None, 100, false, None)
        .await
        .unwrap();
    assert_eq!(page.retained_files, 1);
    assert_eq!(page.attachments.len(), 1);
    assert_eq!(page.attachments[0].message_id, None);
    assert!(org
        .owner_attachment_for_actor(attachment_id, "owner")
        .await
        .unwrap()
        .is_some());
    assert!(org
        .owner_attachment_for_actor(attachment_id, "alex")
        .await
        .unwrap()
        .is_none());

    // Deleting it is the ordinary purge: claimed, then tombstoned without a staging receipt.
    assert!(org
        .request_owner_attachment_purge(attachment_id, "owner", "No longer needed")
        .await
        .unwrap());
    let claimed = org.claim_owner_attachment_gc(8, 3600, 300).await.unwrap();
    assert_eq!(claimed.len(), 1);
    let (id, retained, purge, token) = claimed[0];
    assert_eq!(id, attachment_id);
    assert!(retained && purge);
    assert!(org.complete_owner_attachment_gc(id, token).await.unwrap());
    assert!(org
        .owner_attachment_for_actor(attachment_id, "owner")
        .await
        .unwrap()
        .is_none());
    let page = org
        .owner_attachment_retention_page("owner", None, 100, false, None)
        .await
        .unwrap();
    assert_eq!(page.retained_files, 0);
}

#[tokio::test]
async fn an_archived_document_is_deleted_whole_and_only_once_archived() {
    let Some(org) = company("librarypurge").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping document purge scenario");
        return;
    };
    let created = org
        .create_document(NewDocument {
            command_id: Uuid::new_v4(),
            title: "Old draft",
            kind: DocumentKind::Freeform,
            visibility: DocumentVisibility::Company,
            linked_room_id: None,
            inherit_room_visibility: false,
            owner_actor_id: "owner",
            created_by_actor_id: "owner",
            content_json: &content("claim", "First words"),
            reason: "Created",
        })
        .await
        .unwrap();
    let document_id = created.document_id;
    let first_version_id = org
        .get_document_for_actor(document_id, "owner")
        .await
        .unwrap()
        .current_version
        .version
        .id;
    org.create_named_document_version(NewNamedDocumentVersion {
        command_id: Uuid::new_v4(),
        document_id,
        actor_id: "owner",
        expected_current_version_id: first_version_id,
        content_json: &content("claim", "Second words"),
        reason: "Revised",
    })
    .await
    .unwrap();
    org.create_document_comment_thread(NewDocumentCommentThread {
        document_id,
        actor_id: "owner",
        command_id: Uuid::new_v4(),
        block_id: Some("claim"),
        content_json: &content("note", "A comment"),
    })
    .await
    .unwrap();

    assert!(matches!(
        org.delete_archived_document(document_id, "owner").await,
        Err(DocumentError::Conflict(_))
    ));
    org.set_document_archived(document_id, "owner", true)
        .await
        .unwrap();
    // A colleague who may read it may not delete it.
    assert!(matches!(
        org.delete_archived_document(document_id, "alex").await,
        Err(DocumentError::Unavailable)
    ));
    org.delete_archived_document(document_id, "owner")
        .await
        .unwrap();

    let mut connection = schema_connection(&org).await;
    for table in [
        "native_documents WHERE id=$1",
        "native_document_versions WHERE document_id=$1",
        "native_document_comment_threads WHERE document_id=$1",
        "native_document_comments WHERE document_id=$1",
        "native_document_search_projection WHERE document_id=$1",
    ] {
        let left: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .bind(document_id)
            .fetch_one(&mut connection)
            .await
            .unwrap();
        assert_eq!(left, 0, "{table}");
    }
    // Outside that one purge, history stays immutable.
    let other = org
        .create_document(NewDocument {
            command_id: Uuid::new_v4(),
            title: "Kept",
            kind: DocumentKind::Freeform,
            visibility: DocumentVisibility::Company,
            linked_room_id: None,
            inherit_room_visibility: false,
            owner_actor_id: "owner",
            created_by_actor_id: "owner",
            content_json: &content("claim", "Stays"),
            reason: "Created",
        })
        .await
        .unwrap();
    assert!(
        sqlx::query("DELETE FROM native_document_versions WHERE document_id=$1")
            .bind(other.document_id)
            .execute(&mut connection)
            .await
            .is_err()
    );
}
