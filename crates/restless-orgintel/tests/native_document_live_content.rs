//! Real-Postgres proof for the bounded live Native Documents store. These
//! tests exercise the SQL capability boundary directly because the Hocuspocus
//! sidecar is intentionally limited to these functions rather than table
//! access.

use chrono::{Duration, Utc};
use restless_orgintel::{
    CompanyAccessIdentity, DocumentKind, DocumentVisibility, NewDocument, OrgIntel,
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
