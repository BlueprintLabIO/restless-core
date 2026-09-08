//! Native Docs foundation scenarios. These use a disposable company schema
//! when Postgres is available, matching the rest of OrgIntel's behavioural
//! suite while keeping the pure schema/rendering checks in the module tests.

use restless_orgintel::{
    DocumentAccess, DocumentError, DocumentKind, DocumentStatus, DocumentVisibility, NewDocument,
    NewNamedDocumentVersion, OrgIntel, RestoreDocumentVersion, RoomKind, SetDocumentParticipant,
    UpdateDocumentMetadata,
};
use serde_json::{json, Value};
use sqlx::{Connection as _, PgConnection};
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
struct DocumentEventRow {
    id: i64,
    kind: String,
    actor_id: Option<String>,
    document_id: Uuid,
    body: Value,
}

async fn company(prefix: &str) -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!("{prefix}{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("alex", "human", "member", "Alex")
        .await
        .unwrap();
    org.ensure_actor("blair", "human", "member", "Blair")
        .await
        .unwrap();
    org.ensure_actor("research-analyst", "staff", "analyst", "Research Analyst")
        .await
        .unwrap();
    Some(org)
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

async fn document_events(
    connection: &mut PgConnection,
    document_id: Uuid,
) -> Vec<DocumentEventRow> {
    sqlx::query_as(
        "SELECT id,kind,actor_id,document_id,body FROM events \
         WHERE document_id=$1 ORDER BY id",
    )
    .bind(document_id)
    .fetch_all(connection)
    .await
    .unwrap()
}

fn assert_body_free_event_hint(value: &Value) {
    match value {
        Value::Object(fields) => {
            for (key, child) in fields {
                assert!(
                    !matches!(
                        key.as_str(),
                        "title"
                            | "content"
                            | "content_json"
                            | "plain_text"
                            | "rendered_html"
                            | "markdown"
                            | "reason"
                            | "body"
                    ),
                    "document event leaked content-bearing field {key:?}: {value}"
                );
                assert_body_free_event_hint(child);
            }
        }
        Value::Array(values) => {
            for child in values {
                assert_body_free_event_hint(child);
            }
        }
        _ => {}
    }
}

#[tokio::test]
async fn room_visibility_participants_versions_restore_and_concurrency_are_coherent() {
    let Some(org) = company("documents").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Documents scenario");
        return;
    };
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let mut connection = PgConnection::connect(&database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", org.schema()))
        .execute(&mut connection)
        .await
        .unwrap();

    let room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Research",
            &["alex", "research-analyst"],
        )
        .await
        .unwrap();
    let first_content = content("claim", "First observation");
    let created = org
        .create_document(NewDocument {
            title: "Research brief",
            kind: DocumentKind::Brief,
            visibility: DocumentVisibility::Participants,
            linked_room_id: Some(room.id),
            inherit_room_visibility: true,
            owner_actor_id: "owner",
            created_by_actor_id: "owner",
            content_json: &first_content,
            reason: "Initial brief",
        })
        .await
        .unwrap();
    let document_id = created.document.id;
    let first_version_id = created.current_version.version.id;
    assert_eq!(created.access, DocumentAccess::Edit);
    assert_eq!(created.document.version, 1);
    assert_eq!(created.current_version.markdown, "First observation");

    let events = document_events(&mut connection, document_id).await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "document.created.v1");
    assert_eq!(events[0].actor_id.as_deref(), Some("owner"));
    assert_eq!(events[0].document_id, document_id);
    assert_eq!(
        events[0].body,
        json!({
            "document_id": document_id,
            "document_version": 1,
            "named_version_id": first_version_id,
            "named_version_number": 1,
            "status": "draft",
            "visibility": "participants",
            "linked_room_id": room.id,
            "inherit_room_visibility": true,
            "owner_actor_id": "owner",
        })
    );

    // If event publication fails after the authoritative UPDATE, dropping the
    // transaction must roll back both sides. This proves the event is not a
    // best-effort write after commit.
    sqlx::query(
        "CREATE FUNCTION reject_test_document_event() RETURNS trigger AS $$ \
         BEGIN \
           IF NEW.kind='document.metadata.updated.v1' THEN \
             RAISE EXCEPTION 'forced document event failure'; \
           END IF; \
           RETURN NEW; \
         END; \
         $$ LANGUAGE plpgsql",
    )
    .execute(&mut connection)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER reject_test_document_event \
         BEFORE INSERT ON events FOR EACH ROW \
         EXECUTE FUNCTION reject_test_document_event()",
    )
    .execute(&mut connection)
    .await
    .unwrap();
    let forced_failure = org
        .update_document_metadata(UpdateDocumentMetadata {
            document_id,
            actor_id: "owner",
            expected_version: 1,
            title: "This must roll back",
            kind: DocumentKind::Plan,
            status: DocumentStatus::Accepted,
            visibility: DocumentVisibility::Participants,
            linked_room_id: Some(room.id),
            inherit_room_visibility: false,
        })
        .await
        .unwrap_err();
    assert!(matches!(forced_failure, DocumentError::Database(_)));
    sqlx::query("DROP TRIGGER reject_test_document_event ON events")
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::query("DROP FUNCTION reject_test_document_event()")
        .execute(&mut connection)
        .await
        .unwrap();
    let unchanged = org
        .get_document_for_actor(document_id, "owner")
        .await
        .unwrap();
    assert_eq!(unchanged.document.title, "Research brief");
    assert_eq!(unchanged.document.kind, DocumentKind::Brief);
    assert_eq!(unchanged.document.status, DocumentStatus::Draft);
    assert_eq!(unchanged.document.version, 1);
    assert_eq!(document_events(&mut connection, document_id).await.len(), 1);

    for inherited_reader in ["alex", "research-analyst"] {
        let view = org
            .get_document_for_actor(document_id, inherited_reader)
            .await
            .unwrap();
        assert_eq!(view.access, DocumentAccess::Read);
    }
    assert!(matches!(
        org.get_document_for_actor(document_id, "blair").await,
        Err(DocumentError::Unavailable)
    ));
    assert!(org
        .list_documents_for_actor("blair", false)
        .await
        .unwrap()
        .is_empty());

    org.set_document_participant(SetDocumentParticipant {
        document_id,
        actor_id: "owner",
        expected_document_version: 1,
        participant_actor_id: "alex",
        access: DocumentAccess::Edit,
    })
    .await
    .unwrap();
    let stale = org
        .set_document_participant(SetDocumentParticipant {
            document_id,
            actor_id: "owner",
            expected_document_version: 1,
            participant_actor_id: "blair",
            access: DocumentAccess::Read,
        })
        .await
        .unwrap_err();
    assert!(matches!(stale, DocumentError::Conflict(_)));
    assert_eq!(document_events(&mut connection, document_id).await.len(), 2);
    org.set_document_participant(SetDocumentParticipant {
        document_id,
        actor_id: "owner",
        expected_document_version: 2,
        participant_actor_id: "blair",
        access: DocumentAccess::Read,
    })
    .await
    .unwrap();

    let second_content = content("claim", "Revised observation");
    assert!(matches!(
        org.create_named_document_version(NewNamedDocumentVersion {
            document_id,
            actor_id: "research-analyst",
            expected_current_version_id: first_version_id,
            content_json: &second_content,
            reason: "Agent should not inherit edit access",
        })
        .await,
        Err(DocumentError::Unavailable)
    ));
    assert_eq!(document_events(&mut connection, document_id).await.len(), 3);
    let revised = org
        .create_named_document_version(NewNamedDocumentVersion {
            document_id,
            actor_id: "alex",
            expected_current_version_id: first_version_id,
            content_json: &second_content,
            reason: "Refined after evidence review",
        })
        .await
        .unwrap();
    let second_version_id = revised.current_version.version.id;
    assert_eq!(revised.document.version, 4);
    assert_eq!(revised.current_version.version.version_number, 2);
    assert_eq!(
        revised.current_version.version.document_status,
        DocumentStatus::Draft
    );
    assert!(matches!(
        org.create_named_document_version(NewNamedDocumentVersion {
            document_id,
            actor_id: "alex",
            expected_current_version_id: first_version_id,
            content_json: &second_content,
            reason: "Stale write",
        })
        .await,
        Err(DocumentError::Conflict(_))
    ));
    assert_eq!(document_events(&mut connection, document_id).await.len(), 4);

    let narrowed = org
        .update_document_metadata(UpdateDocumentMetadata {
            document_id,
            actor_id: "owner",
            expected_version: 4,
            title: "Research brief",
            kind: DocumentKind::Brief,
            status: DocumentStatus::InReview,
            visibility: DocumentVisibility::Participants,
            linked_room_id: Some(room.id),
            inherit_room_visibility: false,
        })
        .await
        .unwrap();
    assert_eq!(narrowed.document.version, 5);
    assert!(matches!(
        org.get_document_for_actor(document_id, "research-analyst")
            .await,
        Err(DocumentError::Unavailable)
    ));
    assert_eq!(
        org.get_document_for_actor(document_id, "blair")
            .await
            .unwrap()
            .access,
        DocumentAccess::Read
    );

    let restored = org
        .restore_document_version(RestoreDocumentVersion {
            document_id,
            actor_id: "owner",
            expected_current_version_id: second_version_id,
            source_version_id: first_version_id,
            reason: "Return to the original observation",
        })
        .await
        .unwrap();
    assert_eq!(restored.document.version, 6);
    assert_eq!(restored.current_version.version.version_number, 3);
    assert_eq!(
        restored.current_version.version.restored_from_version_id,
        Some(first_version_id)
    );
    assert_eq!(restored.current_version.version.content_json, first_content);
    assert_eq!(
        restored.current_version.version.document_status,
        DocumentStatus::InReview
    );
    let history = org
        .list_document_versions_for_actor(document_id, "owner")
        .await
        .unwrap();
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].version.id, restored.current_version.version.id);
    assert_eq!(history[1].version.id, second_version_id);
    assert_eq!(history[2].version.id, first_version_id);

    let immutable =
        sqlx::query("UPDATE native_document_versions SET reason='tampered' WHERE id=$1")
            .bind(first_version_id)
            .execute(&mut connection)
            .await
            .unwrap_err();
    assert!(immutable
        .to_string()
        .contains("native document versions are immutable"));

    assert!(matches!(
        org.restore_document_version(RestoreDocumentVersion {
            document_id,
            actor_id: "owner",
            expected_current_version_id: restored.current_version.version.id,
            source_version_id: restored.current_version.version.id,
            reason: "No-op restore",
        })
        .await,
        Err(DocumentError::Invalid(_))
    ));
    assert_eq!(document_events(&mut connection, document_id).await.len(), 6);
    org.remove_document_participant(document_id, "owner", "blair", 6)
        .await
        .unwrap();
    assert!(matches!(
        org.get_document_for_actor(document_id, "blair").await,
        Err(DocumentError::Unavailable)
    ));
    assert_eq!(document_events(&mut connection, document_id).await.len(), 7);
    assert!(matches!(
        org.set_document_participant(SetDocumentParticipant {
            document_id,
            actor_id: "alex",
            expected_document_version: 7,
            participant_actor_id: "blair",
            access: DocumentAccess::Read,
        })
        .await,
        Err(DocumentError::Unavailable)
    ));
    assert_eq!(document_events(&mut connection, document_id).await.len(), 7);

    let archived = org
        .update_document_metadata(UpdateDocumentMetadata {
            document_id,
            actor_id: "owner",
            expected_version: 7,
            title: "Research brief",
            kind: DocumentKind::Brief,
            status: DocumentStatus::Archived,
            visibility: DocumentVisibility::Participants,
            linked_room_id: Some(room.id),
            inherit_room_visibility: false,
        })
        .await
        .unwrap();
    assert_eq!(archived.document.status, DocumentStatus::Archived);
    assert_eq!(archived.document.version, 8);
    assert!(org
        .list_documents_for_actor("owner", false)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        org.list_documents_for_actor("owner", true)
            .await
            .unwrap()
            .len(),
        1
    );

    let events = document_events(&mut connection, document_id).await;
    assert_eq!(events.len(), 8);
    assert!(events.windows(2).all(|pair| pair[0].id < pair[1].id));
    assert_eq!(
        events
            .iter()
            .map(|event| event.kind.as_str())
            .collect::<Vec<_>>(),
        vec![
            "document.created.v1",
            "document.participant.added.v1",
            "document.participant.added.v1",
            "document.named_version.created.v1",
            "document.metadata.updated.v1",
            "document.version.restored.v1",
            "document.participant.removed.v1",
            "document.metadata.updated.v1",
        ]
    );
    assert_eq!(
        events
            .iter()
            .map(|event| event.body["document_version"].as_i64().unwrap())
            .collect::<Vec<_>>(),
        (1..=8).collect::<Vec<_>>()
    );
    assert!(events.iter().all(|event| event.document_id == document_id));
    assert_eq!(
        events
            .iter()
            .map(|event| event.actor_id.as_deref())
            .collect::<Vec<_>>(),
        vec![
            Some("owner"),
            Some("owner"),
            Some("owner"),
            Some("alex"),
            Some("owner"),
            Some("owner"),
            Some("owner"),
            Some("owner"),
        ]
    );
    assert_eq!(
        events[1].body,
        json!({
            "document_id": document_id,
            "document_version": 2,
            "participant_actor_id": "alex",
            "participant_status": "active",
            "access": "edit",
        })
    );
    assert_eq!(
        events[2].body,
        json!({
            "document_id": document_id,
            "document_version": 3,
            "participant_actor_id": "blair",
            "participant_status": "active",
            "access": "read",
        })
    );
    assert_eq!(events[3].actor_id.as_deref(), Some("alex"));
    assert_eq!(
        events[3].body,
        json!({
            "document_id": document_id,
            "document_version": 4,
            "named_version_id": second_version_id,
            "named_version_number": 2,
            "previous_named_version_id": first_version_id,
            "status": "draft",
        })
    );
    assert_eq!(
        events[4].body,
        json!({
            "document_id": document_id,
            "document_version": 5,
            "status": "in_review",
            "visibility": "participants",
            "linked_room_id": room.id,
            "inherit_room_visibility": false,
        })
    );
    assert_eq!(
        events[5].body,
        json!({
            "document_id": document_id,
            "document_version": 6,
            "named_version_id": restored.current_version.version.id,
            "named_version_number": 3,
            "previous_named_version_id": second_version_id,
            "restored_from_version_id": first_version_id,
            "status": "in_review",
        })
    );
    assert_eq!(
        events[6].body,
        json!({
            "document_id": document_id,
            "document_version": 7,
            "participant_actor_id": "blair",
            "participant_status": "removed",
        })
    );
    assert_eq!(
        events[7].body,
        json!({
            "document_id": document_id,
            "document_version": 8,
            "status": "archived",
            "visibility": "participants",
            "linked_room_id": room.id,
            "inherit_room_visibility": false,
        })
    );
    for event in &events {
        assert_body_free_event_hint(&event.body);
        let encoded = event.body.to_string();
        for forbidden in [
            "Research brief",
            "First observation",
            "Revised observation",
            "Initial brief",
            "Refined after evidence review",
            "Return to the original observation",
        ] {
            assert!(!encoded.contains(forbidden));
        }
    }
}

#[tokio::test]
async fn company_visibility_is_human_readable_not_agent_global() {
    let Some(org) = company("companydocs").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Documents access scenario");
        return;
    };
    let body = content("company-note", "Company context");
    let created = org
        .create_document(NewDocument {
            title: "Company note",
            kind: DocumentKind::OperatingNote,
            visibility: DocumentVisibility::Company,
            linked_room_id: None,
            inherit_room_visibility: false,
            owner_actor_id: "owner",
            created_by_actor_id: "owner",
            content_json: &body,
            reason: "Share company context",
        })
        .await
        .unwrap();

    assert_eq!(
        org.get_document_for_actor(created.document.id, "alex")
            .await
            .unwrap()
            .access,
        DocumentAccess::Read
    );
    assert!(matches!(
        org.get_document_for_actor(created.document.id, "research-analyst")
            .await,
        Err(DocumentError::Unavailable)
    ));
    assert!(matches!(
        org.create_document(NewDocument {
            title: "Invalid inheritance",
            kind: DocumentKind::Freeform,
            visibility: DocumentVisibility::Company,
            linked_room_id: None,
            inherit_room_visibility: true,
            owner_actor_id: "owner",
            created_by_actor_id: "owner",
            content_json: &body,
            reason: "Invalid",
        })
        .await,
        Err(DocumentError::Invalid(_))
    ));
}
