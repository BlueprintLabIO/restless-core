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
    let (catch_up, original, _) = org
        .add_weekday_schedule_with_policy(
            "ops-research",
            "bounded catch-up",
            local_time,
            "Australia/Sydney",
            first_window,
            "catch_up_once",
            Some(400_000),
        )
        .await
        .unwrap();
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
    let (skipped, _, _) = org
        .add_weekday_schedule_with_policy(
            "ops-research",
            "skip stale work",
            local_time,
            "Australia/Sydney",
            first_window,
            "skip_if_late",
            Some(60),
        )
        .await
        .unwrap();
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
    let (coalesced, _, _) = org
        .add_weekday_schedule_with_policy(
            "ops-research",
            "latest useful view",
            local_time,
            "Australia/Sydney",
            first_window,
            "coalesce_latest",
            Some(86_400),
        )
        .await
        .unwrap();
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
    let (cancelled, _, _) = org
        .add_weekday_schedule_with_policy(
            "ops-research",
            "cancel before wake",
            local_time,
            "Australia/Sydney",
            first_window,
            "catch_up_once",
            Some(400_000),
        )
        .await
        .unwrap();
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
    let (always_on, _, _) = org
        .add_weekday_schedule_with_policy_and_requirement(
            "ops-research",
            "requires continuous availability",
            local_time,
            "Australia/Sydney",
            first_window,
            "coalesce_latest",
            Some(86_400),
            "always_on",
        )
        .await
        .unwrap();
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

    org.drop_schema().await.unwrap();
}
