//! Native Docs foundation scenarios. These use a disposable company schema
//! when Postgres is available, matching the rest of OrgIntel's behavioural
//! suite while keeping the pure schema/rendering checks in the module tests.

use restless_orgintel::{
    DocumentAccess, DocumentError, DocumentKind, DocumentStatus, DocumentVisibility,
    ImportDocumentMarkdown, ImportRuntimeDocument, NativeDocumentPortableEnvelope, NewDocument,
    NewNamedDocumentVersion, OrgIntel, RemoveDocumentParticipant, RequestDocumentReview,
    RestoreDocumentVersion, RoomKind, SetDocumentParticipant, UpdateDocumentMetadata,
    NATIVE_DOCUMENT_PORTABLE_ENVELOPE_VERSION,
};
use serde_json::{json, Value};
use sqlx::{Connection as _, PgConnection};
use std::time::Duration;
use tokio::time::{sleep, timeout};
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

async fn schema_connection(database_url: &str, schema: &str) -> PgConnection {
    let mut connection = PgConnection::connect(database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {schema}"))
        .execute(&mut connection)
        .await
        .unwrap();
    connection
}

/// Wait until the real read path demonstrably holds the document row FOR
/// SHARE. A probe FOR UPDATE times out only after that lock is present; unlike
/// a sleep, this makes the revocation interleaving deterministic on slow CI.
async fn wait_for_document_reader_lock(database_url: &str, schema: &str, document_id: Uuid) {
    let mut probe = schema_connection(database_url, schema).await;
    sqlx::query("SET lock_timeout = '25ms'")
        .execute(&mut probe)
        .await
        .unwrap();
    for _ in 0..80 {
        match sqlx::query("SELECT id FROM native_documents WHERE id=$1 FOR UPDATE")
            .bind(document_id)
            .fetch_optional(&mut probe)
            .await
        {
            Err(error)
                if error
                    .as_database_error()
                    .and_then(|database| database.code())
                    .as_deref()
                    == Some("55P03") =>
            {
                return;
            }
            Ok(Some(_)) => sleep(Duration::from_millis(10)).await,
            Ok(None) => panic!("document disappeared while waiting for its reader lock"),
            Err(error) => panic!("document reader lock probe failed: {error}"),
        }
    }
    panic!("document read did not acquire its authorization lock");
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
            "documents-research-room",
        )
        .await
        .unwrap();
    let first_content = content("claim", "First observation");
    let created_result = org
        .create_document(NewDocument {
            command_id: Uuid::new_v4(),
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
    let created = org
        .get_document_for_actor(created_result.document_id, "owner")
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
            command_id: Uuid::new_v4(),
            document_id,
            actor_id: "owner",
            expected_version: 1,
            title: "This must roll back",
            kind: DocumentKind::Plan,
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
        .list_documents_for_actor("blair", false, None, 100)
        .await
        .unwrap()
        .items
        .is_empty());

    org.set_document_participant(SetDocumentParticipant {
        command_id: Uuid::new_v4(),
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
            command_id: Uuid::new_v4(),
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
        command_id: Uuid::new_v4(),
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
            command_id: Uuid::new_v4(),
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
            command_id: Uuid::new_v4(),
            document_id,
            actor_id: "alex",
            expected_current_version_id: first_version_id,
            content_json: &second_content,
            reason: "Refined after evidence review",
        })
        .await
        .unwrap();
    let second_version_id = revised.result_id;
    let revised_view = org
        .get_document_for_actor(document_id, "owner")
        .await
        .unwrap();
    assert_eq!(revised_view.document.version, 4);
    assert_eq!(revised_view.current_version.version.version_number, 2);
    assert_eq!(
        revised_view.current_version.version.document_status,
        DocumentStatus::Draft
    );
    assert!(matches!(
        org.create_named_document_version(NewNamedDocumentVersion {
            command_id: Uuid::new_v4(),
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
            command_id: Uuid::new_v4(),
            document_id,
            actor_id: "owner",
            expected_version: 4,
            title: "Research brief",
            kind: DocumentKind::Brief,
            visibility: DocumentVisibility::Participants,
            linked_room_id: Some(room.id),
            inherit_room_visibility: false,
        })
        .await
        .unwrap();
    assert_eq!(narrowed.operation, "document_metadata_update");
    let narrowed_view = org
        .get_document_for_actor(document_id, "owner")
        .await
        .unwrap();
    assert_eq!(narrowed_view.document.version, 5);
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

    let review_command_id = Uuid::new_v4();
    let review = org
        .request_document_review(RequestDocumentReview {
            document_id,
            actor_id: "owner",
            command_id: review_command_id,
            expected_document_version: 5,
            expected_current_version_id: second_version_id,
            summary: "Review the current evidence",
        })
        .await
        .unwrap();

    let restored = org
        .restore_document_version(RestoreDocumentVersion {
            command_id: Uuid::new_v4(),
            document_id,
            actor_id: "owner",
            expected_current_version_id: second_version_id,
            source_version_id: first_version_id,
            reason: "Return to the original observation",
        })
        .await
        .unwrap();
    let restored_view = org
        .get_document_for_actor(document_id, "owner")
        .await
        .unwrap();
    assert_eq!(restored_view.document.version, 7);
    assert_eq!(restored_view.current_version.version.version_number, 3);
    assert_eq!(
        restored_view
            .current_version
            .version
            .restored_from_version_id,
        Some(first_version_id)
    );
    assert_eq!(
        restored_view.current_version.version.content_json,
        first_content
    );
    assert_eq!(
        restored_view.current_version.version.document_status,
        DocumentStatus::InReview
    );
    let history = org
        .list_document_versions_for_actor(document_id, "owner", None, 25)
        .await
        .unwrap();
    assert_eq!(history.items.len(), 3);
    assert_eq!(history.items[0].id, restored.result_id);
    assert_eq!(history.items[1].id, second_version_id);
    assert_eq!(history.items[2].id, first_version_id);

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
            command_id: Uuid::new_v4(),
            document_id,
            actor_id: "owner",
            expected_current_version_id: restored.result_id,
            source_version_id: restored.result_id,
            reason: "No-op restore",
        })
        .await,
        Err(DocumentError::Invalid(_))
    ));
    assert_eq!(document_events(&mut connection, document_id).await.len(), 7);
    org.remove_document_participant(RemoveDocumentParticipant {
        command_id: Uuid::new_v4(),
        document_id,
        actor_id: "owner",
        expected_document_version: 7,
        participant_actor_id: "blair",
    })
    .await
    .unwrap();
    assert!(matches!(
        org.get_document_for_actor(document_id, "blair").await,
        Err(DocumentError::Unavailable)
    ));
    assert_eq!(document_events(&mut connection, document_id).await.len(), 8);
    assert!(matches!(
        org.set_document_participant(SetDocumentParticipant {
            command_id: Uuid::new_v4(),
            document_id,
            actor_id: "alex",
            expected_document_version: 8,
            participant_actor_id: "blair",
            access: DocumentAccess::Read,
        })
        .await,
        Err(DocumentError::Unavailable)
    ));
    assert_eq!(document_events(&mut connection, document_id).await.len(), 8);

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
            "document.review.requested.v1",
            "document.version.restored.v1",
            "document.participant.removed.v1",
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
            "review_id": review.id,
            "requested_named_version_id": second_version_id,
            "requested_by_actor_id": "owner",
            "command_id": review_command_id,
        })
    );
    assert_eq!(
        events[6].body,
        json!({
            "document_id": document_id,
            "document_version": 7,
            "named_version_id": restored.result_id,
            "named_version_number": 3,
            "previous_named_version_id": second_version_id,
            "restored_from_version_id": first_version_id,
            "status": "in_review",
        })
    );
    assert_eq!(
        events[7].body,
        json!({
            "document_id": document_id,
            "document_version": 8,
            "participant_actor_id": "blair",
            "participant_status": "removed",
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
            command_id: Uuid::new_v4(),
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
        org.get_document_for_actor(created.document_id, "alex")
            .await
            .unwrap()
            .access,
        DocumentAccess::Read
    );
    assert!(matches!(
        org.get_document_for_actor(created.document_id, "research-analyst")
            .await,
        Err(DocumentError::Unavailable)
    ));
    assert!(matches!(
        org.create_document(NewDocument {
            command_id: Uuid::new_v4(),
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

#[tokio::test]
async fn explicit_revocation_cannot_split_authorization_from_current_version_read() {
    let Some(org) = company("documentaccessrace").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping document access race scenario");
        return;
    };
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let first_content = content("before-revocation", "Visible before revocation");
    let created = org
        .create_document(NewDocument {
            command_id: Uuid::new_v4(),
            title: "Revocation boundary",
            kind: DocumentKind::DecisionNote,
            visibility: DocumentVisibility::Participants,
            linked_room_id: None,
            inherit_room_visibility: false,
            owner_actor_id: "owner",
            created_by_actor_id: "owner",
            content_json: &first_content,
            reason: "Establish the authorized version",
        })
        .await
        .unwrap();
    let document_id = created.document_id;
    let created_view = org
        .get_document_for_actor(document_id, "owner")
        .await
        .unwrap();
    let first_version_id = created_view.current_version.version.id;
    org.set_document_participant(SetDocumentParticipant {
        command_id: Uuid::new_v4(),
        document_id,
        actor_id: "owner",
        expected_document_version: 1,
        participant_actor_id: "blair",
        access: DocumentAccess::Read,
    })
    .await
    .unwrap();

    // Stall the real getter only after it has locked the document but before it
    // can finish authorization. Revocation must then wait on that document
    // snapshot instead of committing between authorization and content read.
    let mut actor_blocker = schema_connection(&database_url, org.schema()).await;
    let mut actor_lock = actor_blocker.begin().await.unwrap();
    sqlx::query("SELECT id FROM actors WHERE id='blair' FOR UPDATE")
        .fetch_one(&mut *actor_lock)
        .await
        .unwrap();
    let reading = org.clone();
    let reader =
        tokio::spawn(async move { reading.get_document_for_actor(document_id, "blair").await });
    wait_for_document_reader_lock(&database_url, org.schema(), document_id).await;

    let revoking = org.clone();
    let mut revocation = tokio::spawn(async move {
        revoking
            .remove_document_participant(RemoveDocumentParticipant {
                command_id: Uuid::new_v4(),
                document_id,
                actor_id: "owner",
                expected_document_version: 2,
                participant_actor_id: "blair",
            })
            .await
    });
    assert!(
        timeout(Duration::from_millis(100), &mut revocation)
            .await
            .is_err(),
        "participant revocation crossed the in-flight authorized read"
    );

    actor_lock.commit().await.unwrap();
    let observed = timeout(Duration::from_secs(5), reader)
        .await
        .expect("authorized read should finish")
        .unwrap()
        .unwrap();
    timeout(Duration::from_secs(5), revocation)
        .await
        .expect("revocation should finish after the read")
        .unwrap()
        .unwrap();

    let post_revocation = content("after-revocation", "Created only after revocation");
    org.create_named_document_version(NewNamedDocumentVersion {
        command_id: Uuid::new_v4(),
        document_id,
        actor_id: "owner",
        expected_current_version_id: first_version_id,
        content_json: &post_revocation,
        reason: "Prove revoked readers cannot cross the version boundary",
    })
    .await
    .unwrap();
    assert_eq!(observed.current_version.version.id, first_version_id);
    assert_eq!(observed.current_version.version.content_json, first_content);
    assert!(matches!(
        org.get_document_for_actor(document_id, "blair").await,
        Err(DocumentError::Unavailable)
    ));
}

#[tokio::test]
async fn participant_grant_revalidates_the_target_actor_under_retirement_lock() {
    let Some(org) = company("documentactorretire").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping document Actor retirement race");
        return;
    };
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let initial = content("retirement", "Do not grant access after retirement");
    let created = org
        .create_document(NewDocument {
            command_id: Uuid::new_v4(),
            title: "Retirement boundary",
            kind: DocumentKind::OperatingNote,
            visibility: DocumentVisibility::Participants,
            linked_room_id: None,
            inherit_room_visibility: false,
            owner_actor_id: "owner",
            created_by_actor_id: "owner",
            content_json: &initial,
            reason: "Retirement race fixture",
        })
        .await
        .unwrap();

    let mut retirement = schema_connection(&database_url, org.schema()).await;
    let mut retirement_tx = retirement.begin().await.unwrap();
    sqlx::query("SELECT id FROM actors WHERE id='blair' FOR UPDATE")
        .fetch_one(&mut *retirement_tx)
        .await
        .unwrap();
    let granting = org.clone();
    let document_id = created.document_id;
    let grant = tokio::spawn(async move {
        granting
            .set_document_participant(SetDocumentParticipant {
                command_id: Uuid::new_v4(),
                document_id,
                actor_id: "owner",
                expected_document_version: 1,
                participant_actor_id: "blair",
                access: DocumentAccess::Read,
            })
            .await
    });
    sleep(Duration::from_millis(100)).await;
    assert!(
        !grant.is_finished(),
        "grant did not wait for Actor retirement"
    );
    sqlx::query(
        "UPDATE actors SET retired_at=now(),retired_by='owner',retirement_reason='left' \
         WHERE id='blair'",
    )
    .execute(&mut *retirement_tx)
    .await
    .unwrap();
    retirement_tx.commit().await.unwrap();

    let result = timeout(Duration::from_secs(5), grant)
        .await
        .expect("grant should finish after retirement")
        .unwrap();
    assert!(matches!(result, Err(DocumentError::Invalid(_))));
    let participants = org
        .list_document_participants_for_actor(created.document_id, "owner", None, 50)
        .await
        .unwrap();
    assert_eq!(participants.items.len(), 1);
    assert_eq!(participants.items[0].actor_id, "owner");
    assert_eq!(
        org.get_document_for_actor(created.document_id, "owner")
            .await
            .unwrap()
            .document
            .version,
        1
    );
}

#[tokio::test]
async fn room_inherited_revocation_cannot_split_authorization_from_content_read() {
    let Some(org) = company("documentroomrace").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping inherited access race scenario");
        return;
    };
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Revocation room",
            &["blair"],
            "documents-revocation-room",
        )
        .await
        .unwrap();
    let first_content = content("room-before", "Visible while in the Room");
    let created = org
        .create_document(NewDocument {
            command_id: Uuid::new_v4(),
            title: "Inherited revocation boundary",
            kind: DocumentKind::Brief,
            visibility: DocumentVisibility::Participants,
            linked_room_id: Some(room.id),
            inherit_room_visibility: true,
            owner_actor_id: "owner",
            created_by_actor_id: "owner",
            content_json: &first_content,
            reason: "Establish inherited access",
        })
        .await
        .unwrap();
    let document_id = created.document_id;
    let created_view = org
        .get_document_for_actor(document_id, "owner")
        .await
        .unwrap();
    let first_version_id = created_view.current_version.version.id;

    // Room removal serializes Actor -> Room. The document read takes Document
    // -> Actor -> Room, so queuing the reader first proves the inherited access
    // facts stay live until its exact content projection completes.
    let mut actor_blocker = schema_connection(&database_url, org.schema()).await;
    let mut actor_lock = actor_blocker.begin().await.unwrap();
    sqlx::query("SELECT id FROM actors WHERE id='blair' FOR UPDATE")
        .fetch_one(&mut *actor_lock)
        .await
        .unwrap();
    let reading = org.clone();
    let reader =
        tokio::spawn(async move { reading.get_document_for_actor(document_id, "blair").await });
    wait_for_document_reader_lock(&database_url, org.schema(), document_id).await;

    let removing = org.clone();
    let mut removal = tokio::spawn(async move {
        removing
            .remove_room_participant("owner", room.id, "blair")
            .await
    });
    assert!(
        timeout(Duration::from_millis(100), &mut removal)
            .await
            .is_err(),
        "Room access revocation crossed the in-flight authorized read"
    );

    actor_lock.commit().await.unwrap();
    let observed = timeout(Duration::from_secs(5), reader)
        .await
        .expect("inherited read should finish")
        .unwrap()
        .unwrap();
    timeout(Duration::from_secs(5), removal)
        .await
        .expect("Room removal should finish after the read")
        .unwrap()
        .unwrap();

    let post_revocation = content("room-after", "Created after Room removal");
    org.create_named_document_version(NewNamedDocumentVersion {
        command_id: Uuid::new_v4(),
        document_id,
        actor_id: "owner",
        expected_current_version_id: first_version_id,
        content_json: &post_revocation,
        reason: "Prove Room revocation fences later content",
    })
    .await
    .unwrap();
    assert_eq!(observed.current_version.version.id, first_version_id);
    assert_eq!(observed.current_version.version.content_json, first_content);
    assert!(matches!(
        org.get_document_for_actor(document_id, "blair").await,
        Err(DocumentError::Unavailable)
    ));
}

#[tokio::test]
async fn runtime_checkpoint_round_trip_is_authorized_deterministic_and_idempotent() {
    let Some(org) = company("runtimedocs").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Runtime checkpoint scenario");
        return;
    };
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let mut connection = PgConnection::connect(&database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", org.schema()))
        .execute(&mut connection)
        .await
        .unwrap();

    let initial_content = content("stable-runtime-block", "Original observation");
    let created = org
        .create_document(NewDocument {
            command_id: Uuid::new_v4(),
            title: "Runtime checkpoint",
            kind: DocumentKind::Report,
            visibility: DocumentVisibility::Company,
            linked_room_id: None,
            inherit_room_visibility: false,
            owner_actor_id: "owner",
            created_by_actor_id: "owner",
            content_json: &initial_content,
            reason: "Initial named checkpoint",
        })
        .await
        .unwrap();
    let document_id = created.document_id;
    let created_view = org
        .get_document_for_actor(document_id, "owner")
        .await
        .unwrap();
    let source_version_id = created_view.current_version.version.id;

    // Read authority is sufficient to export. The requesting Actor is recorded
    // in the event, not embedded in the checkpoint, so the same named version
    // always produces identical portable bytes.
    let reader_export = org
        .export_document_version_for_runtime(document_id, source_version_id, "alex")
        .await
        .unwrap();
    let owner_export = org
        .export_document_version_for_runtime(document_id, source_version_id, "owner")
        .await
        .unwrap();
    assert_eq!(reader_export, owner_export);
    assert_eq!(reader_export.source_document_id, document_id);
    assert_eq!(reader_export.source_named_version_id, source_version_id);
    assert_eq!(
        reader_export.source_content_hash,
        created_view.current_version.version.content_hash
    );
    assert_eq!(
        reader_export.content_hash,
        reader_export.source_content_hash
    );
    assert_eq!(reader_export.source_created_by_actor_id, "owner");
    assert_eq!(
        reader_export.source_version_reason,
        "Initial named checkpoint"
    );
    assert_eq!(reader_export.source_document_status, DocumentStatus::Draft);
    assert!(reader_export.source_company_id.is_none());
    assert_eq!(reader_export.source_company_key, org.schema());
    assert!(reader_export.markdown.contains(&document_id.to_string()));
    assert!(reader_export
        .markdown
        .contains(&source_version_id.to_string()));
    assert!(reader_export.markdown.contains(&reader_export.content_hash));

    let first_bytes = reader_export.to_json_bytes().unwrap();
    let second_bytes = owner_export.to_json_bytes().unwrap();
    assert_eq!(first_bytes, second_bytes);
    let encoded = String::from_utf8(first_bytes.clone()).unwrap();
    assert!(!encoded.contains("/company/"));
    assert!(!encoded.contains("\"path\""));
    let mut edited: NativeDocumentPortableEnvelope = serde_json::from_slice(&first_bytes).unwrap();
    assert_eq!(edited, reader_export);

    assert!(matches!(
        org.export_document_version_for_runtime(document_id, source_version_id, "research-analyst")
            .await,
        Err(DocumentError::Unavailable)
    ));
    assert_eq!(document_events(&mut connection, document_id).await.len(), 3);

    let mut unknown_version = edited.clone();
    unknown_version.envelope_version = NATIVE_DOCUMENT_PORTABLE_ENVELOPE_VERSION + 1;
    let unknown = org
        .import_runtime_document_as_named_version(ImportRuntimeDocument {
            target_document_id: document_id,
            actor_id: "owner",
            expected_current_version_id: source_version_id,
            envelope: &unknown_version,
            reason: "Unknown envelope must fail",
        })
        .await
        .unwrap_err();
    assert!(matches!(unknown, DocumentError::Invalid(_)));

    let mut corrupt = edited.clone();
    corrupt.content_json["content"][0]["content"][0]["text"] = json!("Corrupted in transit");
    let corrupt_error = org
        .import_runtime_document_as_named_version(ImportRuntimeDocument {
            target_document_id: document_id,
            actor_id: "owner",
            expected_current_version_id: source_version_id,
            envelope: &corrupt,
            reason: "Corrupt envelope must fail",
        })
        .await
        .unwrap_err();
    assert!(
        matches!(corrupt_error, DocumentError::Invalid(message) if message.contains("checksum"))
    );
    assert_eq!(document_events(&mut connection, document_id).await.len(), 3);

    // A Runtime can always recompute an envelope checksum. That must not let
    // it rewrite the immutable Core identity or history the receipt claims.
    let mut fabricated_identity = edited.clone();
    fabricated_identity.source_company_id = Some(Uuid::new_v4());
    fabricated_identity.source_company_key = "fabricated_company".into();
    fabricated_identity.source_document_id = Uuid::new_v4();
    fabricated_identity.source_named_version_id = Uuid::new_v4();
    fabricated_identity.reseal().unwrap();
    let mut fabricated_version = edited.clone();
    fabricated_version.source_named_version_number += 1;
    fabricated_version.source_document_status = DocumentStatus::Accepted;
    fabricated_version.source_restored_from_version_id = Some(Uuid::new_v4());
    fabricated_version.source_created_by_actor_id = "alex".into();
    fabricated_version.source_version_reason = "Fabricated source history".into();
    fabricated_version.source_version_created_at += chrono::Duration::seconds(1);
    fabricated_version.source_content_hash = "0".repeat(64);
    fabricated_version.reseal().unwrap();
    for fabricated in [&fabricated_identity, &fabricated_version] {
        let error = org
            .import_runtime_document_as_named_version(ImportRuntimeDocument {
                target_document_id: document_id,
                actor_id: "owner",
                expected_current_version_id: source_version_id,
                envelope: fabricated,
                reason: "Fabricated provenance must fail",
            })
            .await
            .unwrap_err();
        assert!(
            matches!(error, DocumentError::Invalid(message) if message.contains("source provenance"))
        );
    }
    assert_eq!(
        org.list_document_versions_for_actor(document_id, "owner", None, 25)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM native_document_runtime_imports WHERE document_id=$1",
        )
        .bind(document_id)
        .fetch_one(&mut connection)
        .await
        .unwrap(),
        0
    );
    assert_eq!(document_events(&mut connection, document_id).await.len(), 3);

    edited.content_json["content"][0]["content"][0]["text"] = json!("Runtime refined observation");
    edited.reseal().unwrap();
    assert_ne!(edited.content_hash, edited.source_content_hash);
    assert_eq!(
        edited.source_content_hash,
        reader_export.source_content_hash
    );
    assert_eq!(
        edited.content_json["content"][0]["attrs"]["block_id"],
        "stable-runtime-block"
    );
    assert!(edited.markdown.contains("Runtime refined observation"));
    assert_eq!(
        serde_json::from_slice::<NativeDocumentPortableEnvelope>(&edited.to_json_bytes().unwrap())
            .unwrap(),
        edited
    );

    assert!(matches!(
        org.import_runtime_document_as_named_version(ImportRuntimeDocument {
            target_document_id: document_id,
            actor_id: "alex",
            expected_current_version_id: source_version_id,
            envelope: &edited,
            reason: "Readers cannot import",
        })
        .await,
        Err(DocumentError::Unavailable)
    ));
    assert_eq!(document_events(&mut connection, document_id).await.len(), 3);

    // The named version, durable provenance receipt, current pointer and event
    // are one transaction. An event failure cannot leave an invisible import.
    sqlx::query(
        "CREATE FUNCTION reject_test_document_import_event() RETURNS trigger AS $$ \
         BEGIN \
           IF NEW.kind='document.imported.v1' THEN \
             RAISE EXCEPTION 'forced Runtime import event failure'; \
           END IF; \
           RETURN NEW; \
         END; \
         $$ LANGUAGE plpgsql",
    )
    .execute(&mut connection)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER reject_test_document_import_event \
         BEFORE INSERT ON events FOR EACH ROW \
         EXECUTE FUNCTION reject_test_document_import_event()",
    )
    .execute(&mut connection)
    .await
    .unwrap();
    assert!(matches!(
        org.import_runtime_document_as_named_version(ImportRuntimeDocument {
            target_document_id: document_id,
            actor_id: "owner",
            expected_current_version_id: source_version_id,
            envelope: &edited,
            reason: "Apply the explicit Runtime revision",
        })
        .await,
        Err(DocumentError::Database(_))
    ));
    sqlx::query("DROP TRIGGER reject_test_document_import_event ON events")
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::query("DROP FUNCTION reject_test_document_import_event()")
        .execute(&mut connection)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM native_document_runtime_imports WHERE document_id=$1",
        )
        .bind(document_id)
        .fetch_one(&mut connection)
        .await
        .unwrap(),
        0
    );
    let after_failed_import = org
        .get_document_for_actor(document_id, "owner")
        .await
        .unwrap();
    assert_eq!(after_failed_import.document.version, 1);
    assert_eq!(
        after_failed_import.document.current_named_version_id,
        source_version_id
    );
    assert_eq!(document_events(&mut connection, document_id).await.len(), 3);

    let imported = org
        .import_runtime_document_as_named_version(ImportRuntimeDocument {
            target_document_id: document_id,
            actor_id: "owner",
            expected_current_version_id: source_version_id,
            envelope: &edited,
            reason: "Apply the explicit Runtime revision",
        })
        .await
        .unwrap();
    assert_eq!(imported.target_document_id, document_id);
    assert_eq!(imported.envelope_checksum, edited.checksum);
    assert_eq!(imported.imported_version.version.version_number, 2);
    assert_eq!(
        imported.imported_version.version.content_hash,
        edited.content_hash
    );
    assert_eq!(
        imported.imported_version.version.content_json["content"][0]["attrs"]["block_id"],
        "stable-runtime-block"
    );
    assert_eq!(
        imported.imported_version.version.content_json["content"][0]["content"][0]["text"],
        "Runtime refined observation"
    );

    let provenance = org
        .get_document_runtime_import_provenance_for_actor(
            document_id,
            imported.imported_version.version.id,
            "owner",
        )
        .await
        .unwrap();
    assert_eq!(provenance.document_id, document_id);
    assert_eq!(
        provenance.imported_version_id,
        imported.imported_version.version.id
    );
    assert_eq!(provenance.target_base_version_id, source_version_id);
    assert_eq!(provenance.envelope_schema, edited.envelope_schema);
    assert_eq!(
        provenance.envelope_version,
        NATIVE_DOCUMENT_PORTABLE_ENVELOPE_VERSION as i16
    );
    assert_eq!(provenance.envelope_checksum, edited.checksum);
    assert_eq!(provenance.source_company_id, edited.source_company_id);
    assert_eq!(provenance.source_company_key, org.schema());
    assert_eq!(provenance.source_document_id, document_id);
    assert_eq!(provenance.source_named_version_id, source_version_id);
    assert_eq!(
        provenance.source_named_version_number,
        edited.source_named_version_number
    );
    assert_eq!(
        provenance.source_content_schema_version,
        edited.source_content_schema_version
    );
    assert_eq!(
        provenance.source_document_status,
        edited.source_document_status
    );
    assert_eq!(
        provenance.source_restored_from_version_id,
        edited.source_restored_from_version_id
    );
    assert_eq!(
        provenance.source_created_by_actor_id,
        edited.source_created_by_actor_id
    );
    assert_eq!(
        provenance.source_version_reason,
        edited.source_version_reason
    );
    assert_eq!(
        provenance.source_version_created_at,
        edited.source_version_created_at
    );
    assert_eq!(provenance.source_content_hash, edited.source_content_hash);
    assert_eq!(provenance.imported_content_hash, edited.content_hash);
    assert_eq!(provenance.imported_by_actor_id, "owner");
    let imported_version_id = imported.imported_version.version.id;
    for error in [
        sqlx::query(
            "UPDATE native_document_runtime_imports SET envelope_checksum=repeat('a',64) \
             WHERE document_id=$1 AND imported_version_id=$2",
        )
        .bind(document_id)
        .bind(imported_version_id)
        .execute(&mut connection)
        .await
        .unwrap_err(),
        sqlx::query(
            "DELETE FROM native_document_runtime_imports \
             WHERE document_id=$1 AND imported_version_id=$2",
        )
        .bind(document_id)
        .bind(imported_version_id)
        .execute(&mut connection)
        .await
        .unwrap_err(),
    ] {
        assert!(error
            .to_string()
            .contains("native document Runtime import receipts are immutable"));
    }
    assert!(matches!(
        org.get_document_runtime_import_provenance_for_actor(
            document_id,
            imported.imported_version.version.id,
            "research-analyst",
        )
        .await,
        Err(DocumentError::Unavailable)
    ));

    let replay = org
        .import_runtime_document_as_named_version(ImportRuntimeDocument {
            target_document_id: document_id,
            actor_id: "owner",
            expected_current_version_id: source_version_id,
            envelope: &edited,
            reason: "Apply the explicit Runtime revision",
        })
        .await
        .unwrap();
    assert_eq!(
        replay.imported_version.version.id,
        imported.imported_version.version.id
    );
    assert_eq!(
        replay.imported_version.version.created_at,
        imported.imported_version.version.created_at
    );
    assert_eq!(
        org.get_document_for_actor(document_id, "owner")
            .await
            .unwrap()
            .document
            .version,
        2
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM native_document_runtime_imports WHERE document_id=$1",
        )
        .bind(document_id)
        .fetch_one(&mut connection)
        .await
        .unwrap(),
        1,
        "an exact retry must reuse, not duplicate, durable provenance"
    );

    let mut competing = reader_export.clone();
    competing.content_json["content"][0]["content"][0]["text"] =
        json!("A different stale Runtime revision");
    competing.reseal().unwrap();
    assert!(matches!(
        org.import_runtime_document_as_named_version(ImportRuntimeDocument {
            target_document_id: document_id,
            actor_id: "owner",
            expected_current_version_id: source_version_id,
            envelope: &competing,
            reason: "A competing stale import",
        })
        .await,
        Err(DocumentError::Conflict(_))
    ));

    let events = document_events(&mut connection, document_id).await;
    assert_eq!(events.len(), 4);
    assert!(events.windows(2).all(|pair| pair[0].id < pair[1].id));
    assert_eq!(
        events
            .iter()
            .map(|event| event.kind.as_str())
            .collect::<Vec<_>>(),
        vec![
            "document.created.v1",
            "document.exported.v1",
            "document.exported.v1",
            "document.imported.v1",
        ]
    );
    assert_eq!(events[1].actor_id.as_deref(), Some("alex"));
    assert_eq!(events[2].actor_id.as_deref(), Some("owner"));
    assert_eq!(events[3].actor_id.as_deref(), Some("owner"));
    assert_eq!(
        events[3].body,
        json!({
            "document_id": document_id,
            "document_version": 2,
            "named_version_id": imported.imported_version.version.id,
            "named_version_number": 2,
            "previous_named_version_id": source_version_id,
            "status": "draft",
            "source_company_id": null,
            "source_company_key": org.schema(),
            "source_document_id": document_id,
            "source_named_version_id": source_version_id,
            "source_content_hash": reader_export.source_content_hash,
            "content_hash": edited.content_hash,
            "envelope_checksum": edited.checksum,
        })
    );
    for event in &events {
        assert_body_free_event_hint(&event.body);
        let body = event.body.to_string();
        for forbidden in [
            "Runtime checkpoint",
            "Original observation",
            "Runtime refined observation",
            "Initial named checkpoint",
            "Apply the explicit Runtime revision",
        ] {
            assert!(!body.contains(forbidden));
        }
    }

    // Events are compactable delivery hints. Source/checksum provenance must
    // remain authoritative and inspectable after compaction and reconnection.
    let stream_cursor = org.event_stream_snapshot_cursor().await.unwrap();
    assert_eq!(
        org.compact_events_through(stream_cursor).await.unwrap(),
        stream_cursor
    );
    assert!(document_events(&mut connection, document_id)
        .await
        .is_empty());
    let after_compaction = org
        .get_document_runtime_import_provenance_for_actor(
            document_id,
            imported.imported_version.version.id,
            "owner",
        )
        .await
        .unwrap();
    assert_eq!(after_compaction, provenance);

    let recovered = OrgIntel::ensure(&database_url, org.schema()).await.unwrap();
    let after_recovery = recovered
        .get_document_runtime_import_provenance_for_actor(
            document_id,
            imported.imported_version.version.id,
            "owner",
        )
        .await
        .unwrap();
    assert_eq!(after_recovery, provenance);
    assert_eq!(
        recovered
            .get_document_version_for_actor(
                document_id,
                imported.imported_version.version.id,
                "owner",
            )
            .await
            .unwrap()
            .version
            .content_hash,
        provenance.imported_content_hash
    );
}

#[tokio::test]
async fn markdown_import_is_a_new_authorized_version_with_loss_safe_command_replay() {
    let Some(org) = company("markdownops").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Markdown operations scenario");
        return;
    };
    let initial = content("claim", "Original Markdown evidence");
    let created = org
        .create_document(NewDocument {
            command_id: Uuid::new_v4(),
            title: "Markdown evidence",
            kind: DocumentKind::Report,
            visibility: DocumentVisibility::Participants,
            linked_room_id: None,
            inherit_room_visibility: false,
            owner_actor_id: "owner",
            created_by_actor_id: "owner",
            content_json: &initial,
            reason: "Initial checkpoint",
        })
        .await
        .unwrap();
    let document_id = created.document_id;
    let initial_view = org
        .get_document_for_actor(document_id, "owner")
        .await
        .unwrap();
    let initial_version_id = initial_view.current_version.version.id;
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let mut connection = schema_connection(&database_url, org.schema()).await;
    let events_before_export = document_events(&mut connection, document_id).await.len();
    let export = org
        .export_document_version_as_markdown(document_id, initial_version_id, "owner")
        .await
        .unwrap();
    let repeated_export = org
        .export_document_version_as_markdown(document_id, initial_version_id, "owner")
        .await
        .unwrap();
    assert_eq!(
        repeated_export.source_content_hash,
        export.source_content_hash
    );
    assert_eq!(
        document_events(&mut connection, document_id).await.len(),
        events_before_export,
        "Markdown GET export must remain a read projection"
    );
    assert!(export
        .markdown
        .contains(&format!("source_document_id: {document_id}")));
    assert!(export
        .markdown
        .contains(&format!("source_named_version_id: {initial_version_id}")));
    assert!(export
        .markdown
        .contains("<!-- restless:block-id: claim -->"));
    let edited = export
        .markdown
        .replace("Original Markdown evidence", "Imported Markdown evidence");

    assert!(matches!(
        org.import_markdown_as_named_document_version(ImportDocumentMarkdown {
            command_id: Uuid::new_v4(),
            document_id,
            actor_id: "alex",
            expected_current_version_id: initial_version_id,
            markdown: &edited,
            reason: "Reader must not import",
        })
        .await,
        Err(DocumentError::Unavailable)
    ));

    org.set_document_participant(SetDocumentParticipant {
        command_id: Uuid::new_v4(),
        document_id,
        actor_id: "owner",
        expected_document_version: 1,
        participant_actor_id: "alex",
        access: DocumentAccess::Edit,
    })
    .await
    .unwrap();
    let command_id = Uuid::new_v4();
    let imported = org
        .import_markdown_as_named_document_version(ImportDocumentMarkdown {
            command_id,
            document_id,
            actor_id: "alex",
            expected_current_version_id: initial_version_id,
            markdown: &edited,
            reason: "Apply reviewed Markdown",
        })
        .await
        .unwrap();
    assert_eq!(imported.operation, "markdown_import");
    let imported_view = org
        .get_document_version_for_actor(document_id, imported.result_id, "owner")
        .await
        .unwrap();
    assert_eq!(imported_view.version.version_number, 2);
    assert_eq!(imported_view.version.created_by_actor_id, "alex");
    assert_eq!(imported_view.version.reason, "Apply reviewed Markdown");
    assert_eq!(
        imported_view.version.plain_text,
        "Imported Markdown evidence"
    );
    assert_eq!(
        imported_view.version.content_json["content"][0]["attrs"]["block_id"],
        "claim"
    );

    org.remove_document_participant(RemoveDocumentParticipant {
        command_id: Uuid::new_v4(),
        document_id,
        actor_id: "owner",
        expected_document_version: 3,
        participant_actor_id: "alex",
    })
    .await
    .unwrap();
    let replay = org
        .import_markdown_as_named_document_version(ImportDocumentMarkdown {
            command_id,
            document_id,
            actor_id: "alex",
            expected_current_version_id: initial_version_id,
            markdown: &edited,
            reason: "Apply reviewed Markdown",
        })
        .await
        .unwrap();
    assert_eq!(replay, imported);
    assert!(matches!(
        org.import_markdown_as_named_document_version(ImportDocumentMarkdown {
            command_id,
            document_id,
            actor_id: "alex",
            expected_current_version_id: initial_version_id,
            markdown: &export.markdown,
            reason: "Apply reviewed Markdown",
        })
        .await,
        Err(DocumentError::Conflict(_))
    ));
    assert!(matches!(
        org.import_markdown_as_named_document_version(ImportDocumentMarkdown {
            command_id: Uuid::new_v4(),
            document_id,
            actor_id: "owner",
            expected_current_version_id: initial_version_id,
            markdown: &edited,
            reason: "Stale competing import",
        })
        .await,
        Err(DocumentError::Conflict(_))
    ));
    assert!(matches!(
        org.import_markdown_as_named_document_version(ImportDocumentMarkdown {
            command_id: Uuid::new_v4(),
            document_id: Uuid::new_v4(),
            actor_id: "owner",
            expected_current_version_id: initial_version_id,
            markdown: &edited,
            reason: "Guessed target",
        })
        .await,
        Err(DocumentError::Unavailable)
    ));

    for event in document_events(&mut connection, document_id).await {
        assert_body_free_event_hint(&event.body);
    }
}

#[tokio::test]
async fn document_search_and_backlinks_are_bounded_access_safe_and_rebuildable() {
    let Some(org) = company("documentretrieval").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Document retrieval scenario");
        return;
    };
    let target_content = content(
        "target",
        &format!("{} targettoken {}", "x".repeat(1_500), "y".repeat(700)),
    );
    let target = org
        .create_document(NewDocument {
            command_id: Uuid::new_v4(),
            title: "Target evidence",
            kind: DocumentKind::Brief,
            visibility: DocumentVisibility::Company,
            linked_room_id: None,
            inherit_room_visibility: false,
            owner_actor_id: "owner",
            created_by_actor_id: "owner",
            content_json: &target_content,
            reason: "Search target",
        })
        .await
        .unwrap();
    let target_id = target.document_id;
    let reference_content = |block_id: &str, text: &str| {
        json!({
            "type":"doc",
            "content":[{
                "type":"paragraph",
                "attrs":{"block_id":block_id},
                "content":[
                    {"type":"text","text":text},
                    {"type":"reference","attrs":{
                        "kind":"document",
                        "id":target_id,
                        "label":"Target evidence"
                    }},
                    {"type":"text","text":" then "},
                    {"type":"reference","attrs":{
                        "kind":"actor",
                        "id":"research-analyst",
                        "label":"Research Analyst"
                    }}
                ]
            }]
        })
    };
    let visible_source_content = reference_content("visible-source", "sharedtoken ");
    let visible_source = org
        .create_document(NewDocument {
            command_id: Uuid::new_v4(),
            title: "Shared source",
            kind: DocumentKind::Plan,
            visibility: DocumentVisibility::Company,
            linked_room_id: None,
            inherit_room_visibility: false,
            owner_actor_id: "owner",
            created_by_actor_id: "owner",
            content_json: &visible_source_content,
            reason: "Visible backlink",
        })
        .await
        .unwrap();
    let secret_source_content = reference_content("secret-source", "secrettoken ");
    let secret_source = org
        .create_document(NewDocument {
            command_id: Uuid::new_v4(),
            title: "Private source",
            kind: DocumentKind::DecisionNote,
            visibility: DocumentVisibility::Participants,
            linked_room_id: None,
            inherit_room_visibility: false,
            owner_actor_id: "owner",
            created_by_actor_id: "owner",
            content_json: &secret_source_content,
            reason: "Private backlink",
        })
        .await
        .unwrap();

    let search = org
        .search_documents_for_actor("alex", "targettoken", false, 0, 1)
        .await
        .unwrap();
    assert_eq!(search.items.len(), 1);
    assert_eq!(search.items[0].document_id, target_id);
    assert!(search.items[0].snippet.chars().count() <= 400);
    assert!(search.items[0].snippet.contains("targettoken"));
    assert!(org
        .search_documents_for_actor("alex", "secrettoken", false, 0, 50)
        .await
        .unwrap()
        .items
        .is_empty());
    let paged = org
        .search_documents_for_actor("owner", "evidence", false, 0, 1)
        .await
        .unwrap();
    assert_eq!(paged.items.len(), 1);
    assert_eq!(paged.next_offset, Some(1));
    assert!(org
        .search_documents_for_actor("owner", &"q".repeat(257), false, 0, 50)
        .await
        .is_err());
    assert!(org
        .search_documents_for_actor("owner", "evidence", false, 10_001, 50)
        .await
        .is_err());
    assert!(org
        .search_documents_for_actor("owner", "evidence", false, 0, 51)
        .await
        .is_err());

    let links = org
        .document_links_for_actor(target_id, "alex", 50)
        .await
        .unwrap();
    assert_eq!(links.incoming_document_references.len(), 1);
    assert_eq!(
        links.incoming_document_references[0].source_document_id,
        visible_source.document_id
    );
    assert_ne!(
        links.incoming_document_references[0].source_document_id,
        secret_source.document_id
    );
    let outgoing = org
        .document_links_for_actor(visible_source.document_id, "alex", 50)
        .await
        .unwrap();
    assert_eq!(outgoing.outgoing.len(), 2);
    assert_eq!(outgoing.outgoing[0].target_kind, "document");
    assert_eq!(outgoing.outgoing[0].target_id, target_id.to_string());
    assert_eq!(outgoing.outgoing[1].target_kind, "actor");
    assert_eq!(outgoing.outgoing[1].target_id, "research-analyst");
    assert!(matches!(
        org.document_links_for_actor(secret_source.document_id, "alex", 50)
            .await,
        Err(DocumentError::Unavailable)
    ));
    assert!(matches!(
        org.document_links_for_actor(Uuid::new_v4(), "owner", 50)
            .await,
        Err(DocumentError::Unavailable)
    ));

    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let mut connection = schema_connection(&database_url, org.schema()).await;
    sqlx::query(
        "UPDATE native_document_search_projection SET title='corrupt',plain_text='corrupt' \
         WHERE document_id=$1",
    )
    .bind(target_id)
    .execute(&mut connection)
    .await
    .unwrap();
    sqlx::query("DELETE FROM native_document_reference_projection WHERE source_document_id=$1")
        .bind(visible_source.document_id)
        .execute(&mut connection)
        .await
        .unwrap();
    assert!(org
        .search_documents_for_actor("alex", "targettoken", false, 0, 50)
        .await
        .unwrap()
        .items
        .is_empty());
    assert!(org
        .document_links_for_actor(visible_source.document_id, "alex", 50)
        .await
        .unwrap()
        .outgoing
        .is_empty());
    assert_eq!(
        org.rebuild_document_retrieval_projections().await.unwrap(),
        3
    );
    assert_eq!(
        org.search_documents_for_actor("alex", "targettoken", false, 0, 50)
            .await
            .unwrap()
            .items[0]
            .document_id,
        target_id
    );
    assert_eq!(
        org.document_links_for_actor(visible_source.document_id, "alex", 50)
            .await
            .unwrap()
            .outgoing
            .len(),
        2
    );

    // Retained pre-release/corrupt content can contain a future or malformed
    // reference-like node because the database only enforces a JSON object.
    // Projection repair must omit that node instead of aborting the company.
    let malformed_version_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO native_document_versions \
         (id,document_id,version_number,schema_version,content_json,plain_text,content_hash,\
          document_status,created_by_actor_id,reason) \
         VALUES ($1,$2,2,1,$3,'malformed retained content',repeat('a',64),'draft','owner',\
                 'Retained malformed projection source')",
    )
    .bind(malformed_version_id)
    .bind(secret_source.document_id)
    .bind(json!({
        "type":"doc",
        "content":[{"type":"reference","attrs":{"kind":"future","id":"","label":""}}]
    }))
    .execute(&mut connection)
    .await
    .unwrap();
    sqlx::query("UPDATE native_documents SET current_named_version_id=$2 WHERE id=$1")
        .bind(secret_source.document_id)
        .bind(malformed_version_id)
        .execute(&mut connection)
        .await
        .unwrap();
    assert_eq!(
        org.rebuild_document_retrieval_projections().await.unwrap(),
        3
    );
    let malformed_reference_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM native_document_reference_projection \
         WHERE source_document_id=$1",
    )
    .bind(secret_source.document_id)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(malformed_reference_count, 0);
}
