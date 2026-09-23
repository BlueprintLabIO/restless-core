//! Sprint 38 frozen-clock schedule corpus.
//!
//! These cases use a disposable schema and the real Postgres constraints. The
//! OS wake adapter carries no occurrence data, so duplicate/reordered wake is
//! represented by repeated claims at the same durable instant.

use chrono::{DateTime, NaiveTime, Utc};
use restless_orgintel::OrgIntel;

fn at(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

async fn weekday_responsibility(
    org: &OrgIntel,
    reason: &str,
    local_time: NaiveTime,
    after: DateTime<Utc>,
    missed_policy: &str,
    grace: Option<i64>,
    requirement: &str,
) -> (uuid::Uuid, DateTime<Utc>, bool) {
    org.create_weekday_schedule_with_responsibility(
        "ops-research",
        reason,
        local_time,
        "Australia/Sydney",
        after,
        missed_policy,
        grace,
        requirement,
        uuid::Uuid::new_v4(),
        1,
        reason,
        serde_json::json!({ "window_seconds": 7_200 }),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn appliance_misfires_are_bounded_exact_and_honest_about_local_execution() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping appliance schedule corpus");
        return;
    };
    let company = format!("schedule{}_test", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("ops-research", "staff", "operator", "Worker")
        .await
        .unwrap();

    let local_time = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
    let first_window = at("2026-08-30T00:00:00Z");
    let resume = at("2026-09-03T12:00:00Z");

    // S2/S5/S8/S11: one bounded catch-up. A second or backwards-clock wake
    // observes the advanced durable identity and cannot deliver it again.
    let (catch_up, original, _) = weekday_responsibility(
        &org,
        "bounded catch-up",
        local_time,
        first_window,
        "catch_up_once",
        Some(400_000),
        "local_mac",
    )
    .await;
    assert_eq!(org.claim_due_schedules_at(resume).await.unwrap().len(), 1);
    assert!(org.claim_due_schedules_at(resume).await.unwrap().is_empty());
    assert!(org
        .claim_due_schedules_at(resume - chrono::Duration::hours(2))
        .await
        .unwrap()
        .is_empty());
    let catch_up_occurrences = org.list_schedule_occurrences(catch_up, 20).await.unwrap();
    assert_eq!(catch_up_occurrences.len(), 1);
    assert_eq!(catch_up_occurrences[0].scheduled_for, original);
    assert_eq!(catch_up_occurrences[0].disposition, "fired");

    // S6: a late occurrence is terminally visible but creates no actor wake.
    let (skipped, _, _) = weekday_responsibility(
        &org,
        "skip stale work",
        local_time,
        first_window,
        "skip_if_late",
        Some(60),
        "local_mac",
    )
    .await;
    assert!(org.claim_due_schedules_at(resume).await.unwrap().is_empty());
    let skipped_occurrences = org.list_schedule_occurrences(skipped, 20).await.unwrap();
    assert_eq!(skipped_occurrences.len(), 1);
    assert_eq!(skipped_occurrences[0].disposition, "skipped");
    assert!(skipped_occurrences[0]
        .detail
        .as_deref()
        .unwrap()
        .contains("did not permit catch-up"));

    // S7/S8: however long the downtime, coalescing writes one compressed
    // skipped range and executes only the latest useful occurrence.
    let (coalesced, _, _) = weekday_responsibility(
        &org,
        "latest useful view",
        local_time,
        first_window,
        "coalesce_latest",
        Some(86_400),
        "local_mac",
    )
    .await;
    assert_eq!(org.claim_due_schedules_at(resume).await.unwrap().len(), 1);
    let coalesced_occurrences = org.list_schedule_occurrences(coalesced, 20).await.unwrap();
    assert_eq!(coalesced_occurrences.len(), 2);
    assert_eq!(
        coalesced_occurrences
            .iter()
            .filter(|row| row.disposition == "fired")
            .count(),
        1
    );
    let range = coalesced_occurrences
        .iter()
        .find(|row| row.superseded_count > 0)
        .unwrap();
    assert!(range.supersedes_through.is_some());
    assert_eq!(range.superseded_count, 3);

    // S9: cancellation wins over a late wake.
    let (cancelled, _, _) = weekday_responsibility(
        &org,
        "cancel before wake",
        local_time,
        first_window,
        "catch_up_once",
        Some(400_000),
        "local_mac",
    )
    .await;
    assert!(org
        .cancel_schedule(cancelled, "ops-research", "the opportunity ended")
        .await
        .unwrap());
    assert!(org.claim_due_schedules_at(resume).await.unwrap().is_empty());
    assert!(org
        .list_schedule_occurrences(cancelled, 20)
        .await
        .unwrap()
        .is_empty());

    // S12: the laptop must never claim an always-on workload. It remains due
    // and visible for the Cloud runner under the same schedule identity.
    let (always_on, _, _) = weekday_responsibility(
        &org,
        "requires continuous availability",
        local_time,
        first_window,
        "coalesce_latest",
        Some(86_400),
        "always_on",
    )
    .await;
    assert!(org.claim_due_schedules_at(resume).await.unwrap().is_empty());
    let due = org
        .list_schedules(Some("ops-research"), false)
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.id == always_on)
        .unwrap();
    assert_eq!(due.machine_requirement, "always_on");
    assert!(due.fire_at <= resume);

    // Exec delivery must survive a restart after the occurrence commits but
    // before any model turn starts. Reopening the real store retains exactly
    // one owed input, even when the timer is delivered again.
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    let exec_schedule = org
        .add_schedule(
            "exec",
            None,
            "inspect the current sales responsibility",
            resume,
        )
        .await
        .unwrap();
    assert_eq!(org.claim_due_schedules_at(resume).await.unwrap().len(), 1);
    let reopened = OrgIntel::ensure(&url, &company).await.unwrap();
    assert!(reopened
        .claim_due_schedules_at(resume)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(reopened.owed_conversation_count("exec").await.unwrap(), 1);
    let inbox = reopened.conversation_inbox("exec").await.unwrap();
    assert_eq!(inbox.len(), 1);
    assert!(inbox[0].body.contains(&exec_schedule.to_string()));
    assert!(inbox[0]
        .body
        .contains("inspect the current sales responsibility"));

    org.drop_schema().await.unwrap();
}
