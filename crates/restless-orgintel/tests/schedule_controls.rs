//! Pausing owns admission, while resume and cadence edits preserve responsibility.
use chrono::{Duration, Utc};
use restless_orgintel::OrgIntel;

#[tokio::test]
async fn paused_schedules_do_not_dispatch_or_replay_backlog_and_stale_edits_fail() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping schedule controls corpus");
        return;
    };
    let company = format!("schedule_controls_{}_test", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("exec", "exec", "executive", "Exec").await.unwrap();
    let responsibility = uuid::Uuid::new_v4();
    let (id, _, _) = org.create_interval_schedule_with_responsibility(
        "exec", "Check test records", 3600, Utc::now() - Duration::hours(10),
        responsibility, 1, "Check test records", serde_json::json!({"window_seconds": 7200}),
    ).await.unwrap();
    let fire_at = org.get_schedule(id).await.unwrap().unwrap().fire_at;
    let paused = org.update_recurring_schedule(id, fire_at, false, true, None).await.unwrap().unwrap();
    assert!(paused.paused_at.is_some());
    assert!(org.next_schedule_due_at().await.unwrap().is_none());
    assert!(org.claim_due_schedules_at(Utc::now()).await.unwrap().is_empty());
    assert!(org.update_recurring_schedule(id, fire_at, false, false, None).await.unwrap().is_none());
    let resumed = org.update_recurring_schedule(id, fire_at, true, false, None).await.unwrap().unwrap();
    assert!(resumed.fire_at > Utc::now());
    assert!(org.claim_due_schedules_at(Utc::now()).await.unwrap().is_empty());
    assert_eq!(resumed.responsibility_id, Some(responsibility));
    let edited = org.update_recurring_schedule(id, resumed.fire_at, false, false, Some(900)).await.unwrap().unwrap();
    assert_eq!(edited.interval_seconds, Some(900));
    assert_eq!(edited.responsibility_id, Some(responsibility));
    assert!(org.update_recurring_schedule(id, resumed.fire_at, false, true, None).await.unwrap().is_none());
    assert!(org.update_recurring_schedule(id, edited.fire_at, false, false, Some(30)).await.is_err());
    org.cancel_schedule(id, "exec", "Finished test").await.unwrap();
    assert!(org.update_recurring_schedule(id, edited.fire_at, false, false, None).await.unwrap().is_none());
    org.drop_schema().await.unwrap();
}
