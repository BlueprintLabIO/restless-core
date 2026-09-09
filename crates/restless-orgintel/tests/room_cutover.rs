//! Pre-release cutover checks for the explicit Message-to-Room invariant.

use restless_orgintel::{OrgIntel, RoomKind};
use sqlx::postgres::PgConnection;
use sqlx::Connection as _;

async fn company(prefix: &str) -> Option<(String, OrgIntel)> {
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let company = format!("{prefix}{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&database_url, &company)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("daemon", "system", "system-sender", "The Daemon")
        .await
        .unwrap();
    org.ensure_actor("product-direction", "staff", "lead", "Product Direction")
        .await
        .unwrap();
    Some((database_url, org))
}

#[tokio::test]
async fn internal_message_workflows_persist_explicit_canonical_rooms() {
    let Some((database_url, org)) = company("roomcutover").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room cutover scenario");
        return;
    };

    org.create_team(
        "Product",
        "Own the complete product outcome.",
        "product-direction",
        "exec",
    )
    .await
    .unwrap();
    let reply_id = org
        .send_message("product-direction", None, "The product outcome is ready.")
        .await
        .unwrap();

    let rooms = org
        .room_page_for_actor("product-direction", None, 20)
        .await
        .unwrap()
        .rooms;
    assert!(rooms.iter().any(|room| {
        room.kind == RoomKind::Direct
            && room.canonical_key.as_deref() == Some("direct:4:exec:17:product-direction")
    }));
    assert!(rooms.iter().any(|room| {
        room.kind == RoomKind::Direct
            && room.canonical_key.as_deref() == Some("direct:5:owner:17:product-direction")
    }));

    let mut connection = PgConnection::connect(&database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", org.schema()))
        .execute(&mut connection)
        .await
        .unwrap();
    let (room_id, current_plain_text, event_id): (uuid::Uuid, String, Option<i64>) =
        sqlx::query_as(
            "SELECT room_id,current_plain_text,room_created_event_id \
             FROM messages WHERE id=$1",
        )
        .bind(reply_id)
        .fetch_one(&mut connection)
        .await
        .unwrap();
    assert_eq!(current_plain_text, "The product outcome is ready.");
    assert!(
        event_id.is_some(),
        "the Room event receipt commits atomically"
    );
    let participants: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM room_participants \
         WHERE room_id=$1 AND left_at IS NULL",
    )
    .bind(room_id)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(participants, 2);

    org.drop_schema().await.expect("drop scratch schema");
}

#[tokio::test]
async fn database_rejects_a_message_without_explicit_room_identity() {
    let Some((database_url, org)) = company("roomrequired").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Room invariant scenario");
        return;
    };
    let mut connection = PgConnection::connect(&database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", org.schema()))
        .execute(&mut connection)
        .await
        .unwrap();

    let error = sqlx::query(
        "INSERT INTO messages (from_actor,to_actor,body) \
         VALUES ('owner','exec','must not be inferred')",
    )
    .execute(&mut connection)
    .await
    .expect_err("room_id must be a required write-boundary fact");
    assert_eq!(
        error.as_database_error().and_then(|error| error.code()),
        Some(std::borrow::Cow::Borrowed("23502"))
    );
    let bridge_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pg_trigger \
         WHERE tgrelid='messages'::regclass \
           AND tgname='message_room_compatibility_bridge' AND NOT tgisinternal)",
    )
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert!(!bridge_exists);

    org.drop_schema().await.expect("drop scratch schema");
}
