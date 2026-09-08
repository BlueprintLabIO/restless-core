//! Authoritative cursor/replay semantics for the one compactable OrgIntel
//! operational event stream.

use restless_orgintel::OrgIntel;
use sqlx::Connection as _;
use std::time::Duration;

async fn company(prefix: &str) -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!("{prefix}{}", uuid::Uuid::new_v4().simple());
    Some(
        OrgIntel::ensure(&url, &name)
            .await
            .expect("ensure scratch company schema"),
    )
}

#[tokio::test]
async fn empty_stream_and_invalid_inputs_are_explicit() {
    let Some(org) = company("eventempty").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping event cursor scenario");
        return;
    };

    assert_eq!(org.event_stream_snapshot_cursor().await.unwrap(), 0);
    let page = org.replay_events_after(0, 50).await.unwrap();
    assert!(page.events.is_empty());
    assert_eq!(page.requested_after_event_id, 0);
    assert_eq!(page.next_after_event_id, 0);
    assert_eq!(page.snapshot_cursor, 0);
    assert_eq!(page.compacted_through_event_id, 0);
    assert_eq!(page.oldest_available_event_id, None);
    assert!(!page.has_more);
    assert!(!page.resync_required);

    let unknown_future = org.replay_events_after(1, 50).await.unwrap();
    assert!(unknown_future.resync_required);
    assert!(unknown_future.events.is_empty());

    for invalid in [
        org.replay_events_after(-1, 10).await,
        org.replay_events_after(0, 0).await,
        org.replay_events_after(0, 501).await,
    ] {
        assert!(invalid
            .unwrap_err()
            .to_string()
            .contains("invalid operational event cursor"));
    }
    assert!(org
        .emit_event("", None, serde_json::json!({}))
        .await
        .unwrap_err()
        .to_string()
        .contains("event kind"));
    assert!(org
        .emit_event(
            "event.history.compacted.v1",
            None,
            serde_json::json!({ "through_event_id": 9000 }),
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("reserved"));
}

#[tokio::test]
async fn snapshot_cursor_waits_for_an_earlier_in_flight_event_id() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping cursor ordering scenario");
        return;
    };
    let name = format!("eventordering{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");

    let mut connection = sqlx::PgConnection::connect(&url)
        .await
        .expect("connect independent event writer");
    sqlx::query(&format!("SET search_path TO {name}, public"))
        .execute(&mut connection)
        .await
        .expect("select scratch company schema");
    let mut in_flight = connection.begin().await.expect("begin event writer");
    let earlier: i64 = sqlx::query_scalar(
        "INSERT INTO events (kind,body) VALUES ('test.in-flight.v1','{}') RETURNING id",
    )
    .fetch_one(&mut *in_flight)
    .await
    .expect("allocate earlier event id without committing");

    let later = org
        .emit_event("test.committed-later.v1", None, serde_json::json!({}))
        .await
        .expect("commit a later event id first");
    assert!(earlier < later);

    let cursor_org = org.clone();
    let mut cursor_read =
        tokio::spawn(async move { cursor_org.event_stream_snapshot_cursor().await.unwrap() });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut cursor_read)
            .await
            .is_err(),
        "the committed-prefix cursor must wait for the earlier writer"
    );

    in_flight.commit().await.expect("commit earlier event");
    let cursor = tokio::time::timeout(Duration::from_secs(5), cursor_read)
        .await
        .expect("cursor read should unblock")
        .expect("cursor task should succeed");
    assert_eq!(cursor, later);

    let replay = org.replay_events_after(0, 10).await.unwrap();
    assert_eq!(
        replay
            .events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>(),
        vec![earlier, later]
    );
}

#[tokio::test]
async fn replay_is_bounded_oldest_first_and_duplicate_safe() {
    let Some(org) = company("eventreplay").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping event replay scenario");
        return;
    };

    let first = org
        .emit_event("test.first.v1", None, serde_json::json!({ "value": 1 }))
        .await
        .unwrap();
    let second = org
        .emit_event("test.second.v1", None, serde_json::json!({ "value": 2 }))
        .await
        .unwrap();
    let third = org
        .emit_event("test.third.v1", None, serde_json::json!({ "value": 3 }))
        .await
        .unwrap();
    // The cursor is deliberately captured before an independent projection
    // read. Consumers replay after it and tolerate overlap/duplicates.
    let cursor_before_projection = org.event_stream_snapshot_cursor().await.unwrap();
    assert_eq!(cursor_before_projection, third);
    let _independent_projection = org.list_events(10).await.unwrap();

    let page_one = org.replay_events_after(0, 2).await.unwrap();
    assert_eq!(
        page_one
            .events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>(),
        vec![first, second]
    );
    assert_eq!(page_one.next_after_event_id, second);
    assert_eq!(page_one.snapshot_cursor, cursor_before_projection);
    assert!(page_one.has_more);
    assert!(!page_one.resync_required);

    let duplicate_delivery = org.replay_events_after(0, 2).await.unwrap();
    assert_eq!(
        duplicate_delivery
            .events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>(),
        vec![first, second],
        "at-least-once replay uses stable IDs for consumer deduplication"
    );
    let page_two = org
        .replay_events_after(page_one.next_after_event_id, 2)
        .await
        .unwrap();
    assert_eq!(
        page_two
            .events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>(),
        vec![third]
    );
    assert!(!page_two.has_more);
    assert_eq!(page_two.next_after_event_id, third);

    let after_snapshot = org
        .emit_event(
            "test.after-snapshot.v1",
            None,
            serde_json::json!({ "value": 4 }),
        )
        .await
        .unwrap();
    let catch_up = org
        .replay_events_after(cursor_before_projection, 10)
        .await
        .unwrap();
    assert_eq!(
        catch_up
            .events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>(),
        vec![after_snapshot]
    );
}

#[tokio::test]
async fn compacted_history_requires_resync_without_claiming_exactly_once() {
    let Some(org) = company("eventcompact").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping event compaction scenario");
        return;
    };

    let first = org
        .emit_event("test.one.v1", None, serde_json::json!({}))
        .await
        .unwrap();
    let second = org
        .emit_event("test.two.v1", None, serde_json::json!({}))
        .await
        .unwrap();
    let third = org
        .emit_event("test.three.v1", None, serde_json::json!({}))
        .await
        .unwrap();
    let fourth = org
        .emit_event("test.four.v1", None, serde_json::json!({}))
        .await
        .unwrap();
    assert!(first < second && second < third && third < fourth);

    assert_eq!(org.compact_events_through(second).await.unwrap(), second);
    let cursor_after_compaction = org.event_stream_snapshot_cursor().await.unwrap();
    assert_eq!(org.compact_events_through(second).await.unwrap(), second);
    assert_eq!(
        org.event_stream_snapshot_cursor().await.unwrap(),
        cursor_after_compaction,
        "repeating one compaction command must not append another marker"
    );
    let expired = org.replay_events_after(0, 50).await.unwrap();
    assert!(expired.resync_required);
    assert!(expired.events.is_empty());
    assert_eq!(expired.compacted_through_event_id, second);
    assert_eq!(expired.oldest_available_event_id, Some(third));

    let caught_up_before_the_floor = org.replay_events_after(second, 50).await.unwrap();
    assert!(!caught_up_before_the_floor.resync_required);
    assert_eq!(
        caught_up_before_the_floor
            .events
            .iter()
            .take(2)
            .map(|event| event.id)
            .collect::<Vec<_>>(),
        vec![third, fourth]
    );
    assert!(caught_up_before_the_floor
        .events
        .iter()
        .any(|event| event.kind == "event.history.compacted.v1"));

    let snapshot = org.event_stream_snapshot_cursor().await.unwrap();
    let impossible_future = org.replay_events_after(snapshot + 100, 10).await.unwrap();
    assert!(impossible_future.resync_required);
    assert!(impossible_future.events.is_empty());

    assert!(org
        .compact_events_through(snapshot + 1)
        .await
        .unwrap_err()
        .to_string()
        .contains("newer than stream cursor"));
}
