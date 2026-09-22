//! Real-Postgres proof for the bounded live Native Documents store. These
//! tests exercise the SQL capability boundary directly because the Hocuspocus
//! sidecar is intentionally limited to these functions rather than table
//! access.

use chrono::{Duration, Utc};
use restless_orgintel::{
    CompanyAccessIdentity, DocumentError, DocumentKind, DocumentVisibility, NewDocument,
    NewNamedDocumentVersion, OrgIntel, RestoreDocumentVersion,
};
use serde_json::{json, Value};
use sqlx::{Connection as _, PgConnection};
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
struct LoadRow {
    source_kind: String,
    state_revision: i64,
    yjs_state: Option<Vec<u8>>,
    projection_json: Value,
    seeded_from_named_version_id: Uuid,
    checkpoint_named_version_id: Uuid,
    checkpoint_state_revision: i64,
    content_hash: String,
}

#[derive(Debug, sqlx::FromRow)]
struct StoreRow {
    outcome: String,
    state_revision: i64,
    checkpoint_named_version_id: Uuid,
    content_hash: String,
    current_yjs_state: Option<Vec<u8>>,
    current_projection_json: Option<Value>,
}

fn sqlstate(error: &sqlx::Error) -> Option<String> {
    error
        .as_database_error()
        .and_then(|database| database.code())
        .map(|code| code.into_owned())
}

async fn fixture() -> Option<(OrgIntel, String, String, Uuid, Uuid, Value)> {
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let schema = format!("doclive{}", Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&database_url, &schema)
        .await
        .expect("migrate live-content scratch company");
    let company_id = Uuid::new_v4();
    org.ensure_company_access_identity(CompanyAccessIdentity {
        company_id,
        cell_id: Uuid::new_v4(),
    })
    .await
    .unwrap();
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    let projection = json!({
        "type": "doc",
        "content": [{
            "type": "paragraph",
            "attrs": {"block_id": "opening"},
            "content": [{"type": "text", "text": "A shared plan"}]
        }]
    });
    let created = org
        .create_document(NewDocument {
            command_id: Uuid::new_v4(),
            title: "Realtime plan",
            kind: DocumentKind::Plan,
            visibility: DocumentVisibility::Company,
            linked_room_id: None,
            inherit_room_visibility: false,
            owner_actor_id: "owner",
            created_by_actor_id: "owner",
            content_json: &projection,
            reason: "Initial shared plan",
        })
        .await
        .unwrap();
    Some((
        org,
        database_url,
        schema,
        company_id,
        created.document_id,
        projection,
    ))
}

async fn connection(database_url: &str, schema: &str) -> PgConnection {
    let mut connection = PgConnection::connect(database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {schema}, pg_catalog, pg_temp"))
        .execute(&mut connection)
        .await
        .unwrap();
    connection
}

async fn load(connection: &mut PgConnection, company_id: Uuid, document_id: Uuid) -> LoadRow {
    sqlx::query_as(
        "SELECT source_kind,state_revision,yjs_state,projection_json,\
                seeded_from_named_version_id,checkpoint_named_version_id,\
                checkpoint_state_revision,content_hash \
         FROM orgintel_native_document_yjs_load($1,$2)",
    )
    .bind(company_id)
    .bind(document_id)
    .fetch_one(connection)
    .await
    .unwrap()
}

#[tokio::test]
async fn live_state_is_seeded_cas_stored_replayed_and_recovered() {
    let Some((org, database_url, schema, company_id, document_id, seed_projection)) =
        fixture().await
    else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping native Docs live-content proof");
        return;
    };
    let mut db = connection(&database_url, org.schema()).await;

    let seed = load(&mut db, company_id, document_id).await;
    assert_eq!(seed.source_kind, "seed");
    assert_eq!(seed.state_revision, 0);
    assert_eq!(seed.yjs_state, None);
    assert_eq!(seed.projection_json, seed_projection);
    assert_eq!(
        seed.seeded_from_named_version_id,
        seed.checkpoint_named_version_id
    );
    assert_eq!(seed.checkpoint_state_revision, 0);
    assert_eq!(seed.content_hash.len(), 64);

    let now = Utc::now();
    let null_session_scope: bool = sqlx::query_scalar(
        "SELECT orgintel_native_document_collaboration_consume($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(company_id)
    .bind(document_id)
    .bind(Option::<&str>::None)
    .bind(Uuid::new_v4())
    .bind("write")
    .bind(now)
    .bind(now + Duration::seconds(60))
    .fetch_one(&mut db)
    .await
    .unwrap();
    assert!(!null_session_scope);

    let invalid_store = |error: &sqlx::Error| assert_eq!(sqlstate(error).as_deref(), Some("22023"));
    let null_revision = sqlx::query_as::<_, StoreRow>(
        "SELECT outcome,state_revision,checkpoint_named_version_id,content_hash,\
                current_yjs_state,current_projection_json \
         FROM orgintel_native_document_yjs_store($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(company_id)
    .bind(document_id)
    .bind(Option::<i64>::None)
    .bind(seed.checkpoint_named_version_id)
    .bind(Uuid::new_v4())
    .bind(vec![1_u8, 2])
    .bind(json!({"type":"doc","content":[]}))
    .fetch_one(&mut db)
    .await
    .unwrap_err();
    invalid_store(&null_revision);
    let null_checkpoint = sqlx::query_as::<_, StoreRow>(
        "SELECT outcome,state_revision,checkpoint_named_version_id,content_hash,\
                current_yjs_state,current_projection_json \
         FROM orgintel_native_document_yjs_store($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(company_id)
    .bind(document_id)
    .bind(0_i64)
    .bind(Option::<Uuid>::None)
    .bind(Uuid::new_v4())
    .bind(vec![1_u8, 2])
    .bind(json!({"type":"doc","content":[]}))
    .fetch_one(&mut db)
    .await
    .unwrap_err();
    invalid_store(&null_checkpoint);
    let nil_store_id = sqlx::query_as::<_, StoreRow>(
        "SELECT outcome,state_revision,checkpoint_named_version_id,content_hash,\
                current_yjs_state,current_projection_json \
         FROM orgintel_native_document_yjs_store($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(company_id)
    .bind(document_id)
    .bind(0_i64)
    .bind(seed.checkpoint_named_version_id)
    .bind(Uuid::nil())
    .bind(vec![1_u8, 2])
    .bind(json!({"type":"doc","content":[]}))
    .fetch_one(&mut db)
    .await
    .unwrap_err();
    invalid_store(&nil_store_id);
    assert_eq!(
        load(&mut db, company_id, document_id).await.source_kind,
        "seed"
    );

    let session_id = Uuid::new_v4();
    let consumed: bool = sqlx::query_scalar(
        "SELECT orgintel_native_document_collaboration_consume($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(company_id)
    .bind(document_id)
    .bind("owner")
    .bind(session_id)
    .bind("write")
    .bind(now)
    .bind(now + Duration::seconds(60))
    .fetch_one(&mut db)
    .await
    .unwrap();
    assert!(consumed);
    let replayed_session: bool = sqlx::query_scalar(
        "SELECT orgintel_native_document_collaboration_consume($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(company_id)
    .bind(document_id)
    .bind("owner")
    .bind(session_id)
    .bind("write")
    .bind(now)
    .bind(now + Duration::seconds(60))
    .fetch_one(&mut db)
    .await
    .unwrap();
    assert!(!replayed_session);

    let first_state = vec![1_u8, 2, 3, 4];
    let first_projection = json!({"type":"doc","content":[]});
    let store_id = Uuid::new_v4();
    let stored: StoreRow = sqlx::query_as(
        "SELECT outcome,state_revision,checkpoint_named_version_id,content_hash,\
                current_yjs_state,current_projection_json \
         FROM orgintel_native_document_yjs_store($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(company_id)
    .bind(document_id)
    .bind(0_i64)
    .bind(seed.checkpoint_named_version_id)
    .bind(store_id)
    .bind(&first_state)
    .bind(&first_projection)
    .fetch_one(&mut db)
    .await
    .unwrap();
    assert_eq!(stored.outcome, "stored");
    assert_eq!(stored.state_revision, 1);
    assert_eq!(
        stored.checkpoint_named_version_id,
        seed.checkpoint_named_version_id
    );
    assert_eq!(stored.content_hash.len(), 64);
    assert_eq!(stored.current_yjs_state, None);
    assert_eq!(stored.current_projection_json, None);

    let replay: StoreRow = sqlx::query_as(
        "SELECT outcome,state_revision,checkpoint_named_version_id,content_hash,\
                current_yjs_state,current_projection_json \
         FROM orgintel_native_document_yjs_store($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(company_id)
    .bind(document_id)
    .bind(0_i64)
    .bind(seed.checkpoint_named_version_id)
    .bind(store_id)
    .bind(&first_state)
    .bind(&first_projection)
    .fetch_one(&mut db)
    .await
    .unwrap();
    assert_eq!(replay.outcome, "replayed");
    assert_eq!(replay.state_revision, stored.state_revision);
    assert_eq!(replay.content_hash, stored.content_hash);

    let collision = sqlx::query_as::<_, StoreRow>(
        "SELECT outcome,state_revision,checkpoint_named_version_id,content_hash,\
                current_yjs_state,current_projection_json \
         FROM orgintel_native_document_yjs_store($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(company_id)
    .bind(document_id)
    .bind(0_i64)
    .bind(seed.checkpoint_named_version_id)
    .bind(store_id)
    .bind(vec![9_u8, 9])
    .bind(&first_projection)
    .fetch_one(&mut db)
    .await
    .unwrap_err();
    assert_eq!(sqlstate(&collision).as_deref(), Some("23505"));

    let stale: StoreRow = sqlx::query_as(
        "SELECT outcome,state_revision,checkpoint_named_version_id,content_hash,\
                current_yjs_state,current_projection_json \
         FROM orgintel_native_document_yjs_store($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(company_id)
    .bind(document_id)
    .bind(0_i64)
    .bind(seed.checkpoint_named_version_id)
    .bind(Uuid::new_v4())
    .bind(vec![8_u8, 8])
    .bind(json!({"type":"doc","content":[{"type":"paragraph"}]}))
    .fetch_one(&mut db)
    .await
    .unwrap();
    assert_eq!(stale.outcome, "conflict");
    assert_eq!(stale.state_revision, 1);
    assert_eq!(stale.current_yjs_state, Some(first_state.clone()));
    assert_eq!(
        stale.current_projection_json,
        Some(first_projection.clone())
    );

    drop(db);
    drop(org);
    let recovered = OrgIntel::ensure(&database_url, &schema).await.unwrap();
    let mut db = connection(&database_url, recovered.schema()).await;
    let after_restart = load(&mut db, company_id, document_id).await;
    assert_eq!(after_restart.source_kind, "state");
    assert_eq!(after_restart.state_revision, 1);
    assert_eq!(after_restart.yjs_state, Some(first_state));
    assert_eq!(after_restart.projection_json, first_projection);
    assert_eq!(after_restart.content_hash, stored.content_hash);
}

#[tokio::test]
async fn named_version_changes_atomically_advance_or_reset_the_live_checkpoint() {
    let Some((org, database_url, _schema, company_id, document_id, seed_projection)) =
        fixture().await
    else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping native Docs checkpoint proof");
        return;
    };
    let mut db = connection(&database_url, org.schema()).await;
    let seed = load(&mut db, company_id, document_id).await;
    let live_state = vec![1_u8, 2, 3, 4];

    let stored: StoreRow = sqlx::query_as(
        "SELECT outcome,state_revision,checkpoint_named_version_id,content_hash,\
                current_yjs_state,current_projection_json \
         FROM orgintel_native_document_yjs_store($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(company_id)
    .bind(document_id)
    .bind(0_i64)
    .bind(seed.checkpoint_named_version_id)
    .bind(Uuid::new_v4())
    .bind(&live_state)
    .bind(&seed_projection)
    .fetch_one(&mut db)
    .await
    .unwrap();
    assert_eq!(stored.outcome, "stored");

    let unchanged = org
        .create_named_document_version(NewNamedDocumentVersion {
            command_id: Uuid::new_v4(),
            document_id,
            actor_id: "owner",
            expected_current_version_id: seed.checkpoint_named_version_id,
            content_json: &seed_projection,
            reason: "Checkpoint the converged live body",
        })
        .await
        .unwrap();
    let advanced = load(&mut db, company_id, document_id).await;
    assert_eq!(advanced.source_kind, "state");
    assert_eq!(advanced.state_revision, 1);
    assert_eq!(advanced.yjs_state, Some(live_state.clone()));
    assert_eq!(advanced.checkpoint_named_version_id, unchanged.result_id);
    assert_eq!(advanced.checkpoint_state_revision, 1);

    let replacement_projection = json!({
        "type": "doc",
        "content": [{
            "type": "paragraph",
            "attrs": {"block_id": "replacement"},
            "content": [{"type": "text", "text": "A restored direction"}]
        }]
    });
    let stale_checkpoint = org
        .create_named_document_version(NewNamedDocumentVersion {
            command_id: Uuid::new_v4(),
            document_id,
            actor_id: "owner",
            expected_current_version_id: unchanged.result_id,
            content_json: &replacement_projection,
            reason: "A stale non-collaborative body",
        })
        .await
        .unwrap_err();
    assert!(matches!(stale_checkpoint, DocumentError::Conflict(_)));
    let after_refusal = load(&mut db, company_id, document_id).await;
    assert_eq!(
        after_refusal.checkpoint_named_version_id,
        unchanged.result_id
    );
    assert_eq!(after_refusal.yjs_state, Some(live_state.clone()));

    let replacement_state = vec![5_u8, 6, 7, 8];
    let live_change: StoreRow = sqlx::query_as(
        "SELECT outcome,state_revision,checkpoint_named_version_id,content_hash,\
                current_yjs_state,current_projection_json \
         FROM orgintel_native_document_yjs_store($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(company_id)
    .bind(document_id)
    .bind(1_i64)
    .bind(unchanged.result_id)
    .bind(Uuid::new_v4())
    .bind(&replacement_state)
    .bind(&replacement_projection)
    .fetch_one(&mut db)
    .await
    .unwrap();
    assert_eq!(live_change.outcome, "stored");
    assert_eq!(live_change.state_revision, 2);

    let changed = org
        .restore_document_version(RestoreDocumentVersion {
            command_id: Uuid::new_v4(),
            document_id,
            actor_id: "owner",
            expected_current_version_id: unchanged.result_id,
            source_version_id: seed.checkpoint_named_version_id,
            reason: "Explicitly restore the initial direction",
        })
        .await
        .unwrap();
    let reset = load(&mut db, company_id, document_id).await;
    assert_eq!(reset.source_kind, "seed");
    assert_eq!(reset.state_revision, 0);
    assert_eq!(reset.yjs_state, None);
    assert_eq!(reset.projection_json, seed_projection);
    assert_eq!(reset.checkpoint_named_version_id, changed.result_id);

    let stale_writer: StoreRow = sqlx::query_as(
        "SELECT outcome,state_revision,checkpoint_named_version_id,content_hash,\
                current_yjs_state,current_projection_json \
         FROM orgintel_native_document_yjs_store($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(company_id)
    .bind(document_id)
    .bind(2_i64)
    .bind(unchanged.result_id)
    .bind(Uuid::new_v4())
    .bind(&replacement_state)
    .bind(&replacement_projection)
    .fetch_one(&mut db)
    .await
    .unwrap();
    assert_eq!(stale_writer.outcome, "conflict");
    assert_eq!(stale_writer.state_revision, 0);
    assert_eq!(stale_writer.checkpoint_named_version_id, changed.result_id);
    assert_eq!(stale_writer.current_yjs_state, None);
    assert_eq!(stale_writer.current_projection_json, Some(seed_projection));
}

async fn store_history_body(
    db: &mut PgConnection,
    company: Uuid,
    document: Uuid,
    previous: &LoadRow,
    content: &Value,
) -> StoreRow {
    sqlx::query_as("SELECT outcome,state_revision,checkpoint_named_version_id,content_hash,current_yjs_state,current_projection_json FROM orgintel_native_document_yjs_store($1,$2,$3,$4,$5,$6,$7)")
        .bind(company).bind(document).bind(previous.state_revision).bind(previous.checkpoint_named_version_id)
        .bind(Uuid::new_v4()).bind(vec![1_u8,2,3]).bind(content).fetch_one(db).await.unwrap()
}

#[tokio::test]
async fn automatic_history_waits_for_idle_deduplicates_and_preserves_live_lineage() {
    let Some((org, url, schema, company, document, mut body)) = fixture().await else {
        return;
    };
    let mut db = connection(&url, &schema).await;
    let seed = load(&mut db, company, document).await;
    body["content"][0]["content"][0]["text"] = json!("An automatically preserved edit");
    body["content"][0]["content"][0]["marks"] =
        json!([{"type":"link","attrs":{"href":"https://example.com","title":null}}]);
    body["content"].as_array_mut().unwrap().push(json!({"type":"codeBlock","attrs":{"block_id":"code","language":null},"content":[{"type":"text","text":"let answer = 42;"}]}));
    assert_eq!(
        store_history_body(&mut db, company, document, &seed, &body)
            .await
            .outcome,
        "stored"
    );
    assert_eq!(
        org.snapshot_document_history(20).await.unwrap(),
        0,
        "do not checkpoint every keystroke"
    );
    sqlx::query("UPDATE native_document_yjs_state SET persisted_at=now()-interval '11 seconds' WHERE document_id=$1")
        .bind(document).execute(&mut db).await.unwrap();
    let (a, b) = tokio::join!(
        org.snapshot_document_history(20),
        org.snapshot_document_history(20)
    );
    assert_eq!(
        a.unwrap() + b.unwrap(),
        1,
        "concurrent maintenance must create exactly one version"
    );
    assert_eq!(org.snapshot_document_history(20).await.unwrap(), 0);
    let saved = load(&mut db, company, document).await;
    assert_eq!(saved.projection_json, body);
    assert_eq!(saved.yjs_state, Some(vec![1, 2, 3]));
    assert_eq!(saved.state_revision, 1);
    assert_eq!(saved.checkpoint_state_revision, 1);
    assert_eq!(
        saved.seeded_from_named_version_id,
        seed.seeded_from_named_version_id
    );
    assert_ne!(
        saved.checkpoint_named_version_id,
        seed.checkpoint_named_version_id
    );
    let snapshot: Value =
        sqlx::query_scalar("SELECT content_json FROM native_document_versions WHERE id=$1")
            .bind(saved.checkpoint_named_version_id)
            .fetch_one(&mut db)
            .await
            .unwrap();
    assert_eq!(snapshot, body);
    let mut next_body = body.clone();
    next_body["content"][0]["content"][0]["text"] = json!("The next edit survives");
    let stale = store_history_body(&mut db, company, document, &seed, &next_body).await;
    assert_eq!(stale.outcome, "conflict");
    assert_eq!(
        stale.checkpoint_named_version_id,
        saved.checkpoint_named_version_id
    );
    assert_eq!(stale.current_yjs_state, saved.yjs_state);
    assert_eq!(
        store_history_body(&mut db, company, document, &saved, &next_body)
            .await
            .outcome,
        "stored"
    );
    assert_eq!(
        load(&mut db, company, document).await.projection_json,
        next_body
    );
}

#[tokio::test]
async fn restore_preserves_unsnapshotted_live_edits_and_is_retry_safe() {
    let Some((org, url, schema, company, document, original)) = fixture().await else {
        return;
    };
    let mut db = connection(&url, &schema).await;
    let seed = load(&mut db, company, document).await;
    let mut body = original.clone();
    body["content"][0]["content"][0]["text"] = json!("Keep this draft before restoring");
    assert_eq!(
        store_history_body(&mut db, company, document, &seed, &body)
            .await
            .outcome,
        "stored"
    );
    let command = Uuid::new_v4();
    for _ in 0..2 {
        org.restore_document_version(RestoreDocumentVersion {
            command_id: command,
            document_id: document,
            actor_id: "owner",
            expected_current_version_id: seed.checkpoint_named_version_id,
            source_version_id: seed.checkpoint_named_version_id,
            reason: "Restore original",
        })
        .await
        .unwrap();
    }
    let versions: Vec<(Value,String)> = sqlx::query_as("SELECT content_json,reason FROM native_document_versions WHERE document_id=$1 ORDER BY version_number")
        .bind(document).fetch_all(&mut db).await.unwrap();
    assert_eq!(versions.len(), 3);
    assert_eq!(versions[1], (body, "Before restore".into()));
    assert_eq!(versions[2].0, original);
    assert_eq!(
        load(&mut db, company, document).await.projection_json,
        original
    );
}

#[tokio::test]
async fn review_captures_current_live_body_without_manual_checkpoint() {
    let Some((org, url, schema, company, document, mut body)) = fixture().await else {
        return;
    };
    let mut db = connection(&url, &schema).await;
    let seed = load(&mut db, company, document).await;
    body["content"][0]["content"][0]["text"] = json!("Review what I just edited");
    assert_eq!(
        store_history_body(&mut db, company, document, &seed, &body)
            .await
            .outcome,
        "stored"
    );
    let view = org.get_document_for_actor(document, "owner").await.unwrap();
    org.request_document_review(restless_orgintel::RequestDocumentReview {
        command_id: Uuid::new_v4(),
        document_id: document,
        actor_id: "owner",
        expected_document_version: view.document.version,
        expected_current_version_id: seed.checkpoint_named_version_id,
        summary: "Please review",
        work_dependency: None,
    })
    .await
    .unwrap();
    let reviewed: Value=sqlx::query_scalar("SELECT version.content_json FROM native_document_reviews review JOIN native_document_versions version ON version.id=review.requested_version_id WHERE review.document_id=$1")
        .bind(document).fetch_one(&mut db).await.unwrap();
    assert_eq!(reviewed, body);
    assert_eq!(
        load(&mut db, company, document)
            .await
            .seeded_from_named_version_id,
        seed.seeded_from_named_version_id
    );
}

#[tokio::test]
async fn comments_anchor_live_blocks_without_waiting_for_idle_history() {
    use restless_orgintel::{
        DocumentAccess, DocumentCommentAnchorState, NewDocumentCommentThread,
        SetDocumentParticipant,
    };
    let Some((org, url, schema, company, document, mut body)) = fixture().await else {
        return;
    };
    org.ensure_actor("colleague", "human", "colleague", "A colleague")
        .await
        .unwrap();
    let view = org.get_document_for_actor(document, "owner").await.unwrap();
    org.set_document_participant(SetDocumentParticipant {
        command_id: Uuid::new_v4(),
        document_id: document,
        actor_id: "owner",
        expected_document_version: view.document.version,
        participant_actor_id: "colleague",
        access: DocumentAccess::Comment,
    })
    .await
    .unwrap();
    let mut db = connection(&url, &schema).await;
    let seed = load(&mut db, company, document).await;
    body["content"].as_array_mut().unwrap().push(json!({
        "type":"paragraph", "attrs":{"block_id":"new-live-paragraph"},
        "content":[{"type":"text","text":"A just-written paragraph"}]
    }));
    assert_eq!(
        store_history_body(&mut db, company, document, &seed, &body)
            .await
            .outcome,
        "stored"
    );
    let comment = json!({"type":"doc","content":[{"type":"paragraph","attrs":{"block_id":"feedback"},"content":[{"type":"text","text":"Please clarify this."}]}]});
    let command = Uuid::new_v4();
    let input = |command_id| NewDocumentCommentThread {
        command_id,
        document_id: document,
        actor_id: "colleague",
        block_id: Some("new-live-paragraph"),
        content_json: &comment,
    };
    let (first, second) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(
            org.create_document_comment_thread(input(command)),
            org.create_document_comment_thread(input(Uuid::new_v4()))
        )
    })
    .await
    .expect("concurrent anchors do not deadlock");
    let first = first.unwrap();
    second.unwrap();
    assert_eq!(
        first.thread.anchor_state,
        DocumentCommentAnchorState::Active
    );
    let saved = load(&mut db, company, document).await;
    assert_eq!(
        saved.seeded_from_named_version_id,
        seed.seeded_from_named_version_id
    );
    assert_eq!(saved.projection_json, body);
    assert_ne!(
        saved.checkpoint_named_version_id,
        seed.checkpoint_named_version_id
    );
    assert_eq!(
        first.thread.thread.anchored_version_id,
        saved.checkpoint_named_version_id
    );
    let version_body: Value =
        sqlx::query_scalar("SELECT content_json FROM native_document_versions WHERE id=$1")
            .bind(saved.checkpoint_named_version_id)
            .fetch_one(&mut db)
            .await
            .unwrap();
    assert_eq!(version_body, body);
    body["content"].as_array_mut().unwrap().pop();
    assert_eq!(
        store_history_body(&mut db, company, document, &saved, &body)
            .await
            .outcome,
        "stored"
    );
    let threads = org
        .list_document_comment_threads(document, "colleague", None, 10)
        .await
        .unwrap();
    assert!(threads
        .items
        .iter()
        .all(|thread| thread.anchor_state == DocumentCommentAnchorState::Orphaned));
    let replay = org
        .create_document_comment_thread(input(command))
        .await
        .unwrap();
    assert_eq!(replay.thread.thread.id, first.thread.thread.id);
    assert!(matches!(
        org.create_document_comment_thread(input(Uuid::new_v4()))
            .await,
        Err(DocumentError::Invalid(_))
    ));
    assert_eq!(
        load(&mut db, company, document)
            .await
            .checkpoint_named_version_id,
        saved.checkpoint_named_version_id,
        "replayed and refused comments do not advance document history"
    );
}
