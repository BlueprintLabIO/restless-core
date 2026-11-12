//! Items that wait on the owner do not go stale in silence (12 November 2026). One nothing needs
//! any more is withdrawn with its reason. One still needed wakes Exec with a single message on
//! days 3, 9 and 21, then never again; Exec may withdraw it or raise it again once per ask, and a
//! re-raised card is the same card.

use chrono::{Duration, Utc};
use restless_orgintel::{
    InboxMarkChange, NewOwnerHandoff, NewWork, OrgIntel, OwnerHandoffCategory, OwnerHandoffState,
    WorkspaceSpec,
};
use uuid::Uuid;

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "stalehandoff{}_test",
        &Uuid::new_v4().simple().to_string()[..12]
    );
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    for (id, kind, role) in [
        ("owner", "owner", "owner"),
        ("exec", "exec", "exec"),
        ("alpha-build", "staff", "builder"),
    ] {
        org.ensure_actor(id, kind, role, id).await.unwrap();
    }
    Some(org)
}

async fn owner_step(
    org: &OrgIntel,
    title: &str,
    goal: Option<Uuid>,
    category: OwnerHandoffCategory,
) -> (Uuid, Uuid) {
    let work = org
        .add_work(NewWork {
            owner_id: "alpha-build",
            title,
            outcome: "done",
            goal_id: goal,
            priority: 1,
            expected_artifact: "note",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: work,
            attempt_id: None,
            requested_by: "alpha-build",
            category,
            requested_action: title,
            prepared_state: "prepared on the company computer",
            resume_condition: "the owner has done it",
        })
        .await
        .unwrap();
    (work, handoff)
}

async fn state_of(org: &OrgIntel, handoff: Uuid) -> (OwnerHandoffState, String) {
    let row = org
        .list_owner_handoffs()
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.id == handoff)
        .unwrap();
    (row.state, row.resolution)
}

async fn asks(org: &OrgIntel) -> Vec<String> {
    org.inbox(Some("exec"))
        .await
        .unwrap()
        .into_iter()
        .filter(|message| {
            message.from_actor == "daemon" && message.body.starts_with("These have waited")
        })
        .map(|message| message.body)
        .collect()
}

#[tokio::test]
async fn exec_is_asked_once_for_all_stale_items_on_a_backoff_and_may_raise_each_again_once() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping stale handoff scenario");
        return;
    };
    let (_, lease) = owner_step(
        &org,
        "Sign the lease",
        None,
        OwnerHandoffCategory::LegalAttestation,
    )
    .await;
    let (_, login) = owner_step(
        &org,
        "Sign in to the bank",
        None,
        OwnerHandoffCategory::Identity,
    )
    .await;
    // Snoozed by the owner: theirs to bring back, never raised over their head.
    let (_, snoozed) = owner_step(
        &org,
        "Confirm the address",
        None,
        OwnerHandoffCategory::Identity,
    )
    .await;
    org.set_inbox_marks(
        "owner",
        &[InboxMarkChange {
            item_id: format!("orgintel:handoff:{snoozed}"),
            seen_stamp: None,
            snoozed_until: Some(Some(Utc::now() + Duration::days(60))),
        }],
    )
    .await
    .unwrap();
    let start = Utc::now();

    // Nothing for the first three days, and no message when nothing is due.
    assert!(org
        .signal_stale_owner_handoffs_at(start + Duration::days(2))
        .await
        .unwrap()
        .is_empty());
    assert!(asks(&org).await.is_empty());
    // Not asked yet, so Exec can neither raise nor withdraw it.
    assert!(org
        .reraise_owner_handoff(lease, "exec", "The office move waits on it.", start)
        .await
        .is_err());
    assert!(org
        .withdraw_stale_owner_handoff(lease, "exec", "no longer needed")
        .await
        .is_err());

    // Day 3: one message names both items; asking again the same day sends nothing.
    let day3 = start + Duration::days(3) + Duration::minutes(1);
    let asked = org.signal_stale_owner_handoffs_at(day3).await.unwrap();
    assert_eq!(asked.len(), 2, "{asked:?}");
    assert!(org
        .signal_stale_owner_handoffs_at(day3 + Duration::hours(1))
        .await
        .unwrap()
        .is_empty());
    let sent = asks(&org).await;
    assert_eq!(sent.len(), 1, "one message covers every item: {sent:?}");
    assert!(sent[0].contains(&lease.to_string()) && sent[0].contains(&login.to_string()));
    assert!(
        sent[0].contains("\"Sign the lease\": 3 days"),
        "{}",
        sent[0]
    );
    assert!(!sent[0].contains(&snoozed.to_string()));

    // Exec raises one again: only Exec, with a reason, once per ask. The card keeps its id.
    assert!(org
        .reraise_owner_handoff(lease, "alpha-build", "The office move waits on it.", day3)
        .await
        .is_err());
    assert!(org
        .reraise_owner_handoff(lease, "exec", "  ", day3)
        .await
        .is_err());
    let raised = org
        .reraise_owner_handoff(lease, "exec", "The office move waits on it.", day3)
        .await
        .unwrap();
    assert_eq!(raised.count, 1);
    assert!(
        org.reraise_owner_handoff(lease, "exec", "Again.", day3)
            .await
            .is_err(),
        "never more than the backoff allows"
    );
    let reraises = org.handoff_reraises().await.unwrap();
    assert_eq!(reraises.len(), 1);
    assert_eq!(reraises[&lease].reason, "The office move waits on it.");
    assert_eq!(state_of(&org, lease).await.0, OwnerHandoffState::Pending);

    // Exec withdraws the other: only Exec, and the Work says why it no longer waits.
    assert!(org
        .withdraw_stale_owner_handoff(login, "alpha-build", "the bank export is enough")
        .await
        .is_err());
    org.withdraw_stale_owner_handoff(login, "exec", "the bank export is enough")
        .await
        .unwrap();
    let (state, resolution) = state_of(&org, login).await;
    assert_eq!(state, OwnerHandoffState::Withdrawn);
    assert!(
        resolution.contains("the bank export is enough"),
        "{resolution}"
    );

    // Days 9 and 21 ask again about what is left, then never.
    for (day, total) in [(8, 1), (10, 2), (20, 2), (22, 3), (50, 3)] {
        org.signal_stale_owner_handoffs_at(start + Duration::days(day))
            .await
            .unwrap();
        assert_eq!(asks(&org).await.len(), total, "day {day}");
    }
    let last = asks(&org).await.pop().unwrap();
    assert!(last.contains(&lease.to_string()) && !last.contains(&login.to_string()));
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn an_item_nothing_needs_any_more_is_withdrawn_with_its_reason() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping obsolete handoff scenario");
        return;
    };
    let goal = org
        .add_goal("Open the Sydney office", "", "owner")
        .await
        .unwrap();
    let (work, closed) = owner_step(
        &org,
        "Sign the lease",
        Some(goal),
        OwnerHandoffCategory::LegalAttestation,
    )
    .await;
    let (_, payment) = owner_step(
        &org,
        "Pay the deposit",
        Some(goal),
        OwnerHandoffCategory::PaymentConfirmation,
    )
    .await;
    let (_, live) = owner_step(
        &org,
        "Sign in to the bank",
        None,
        OwnerHandoffCategory::Identity,
    )
    .await;

    assert!(
        org.withdraw_obsolete_handoffs().await.unwrap().is_empty(),
        "everything is still needed"
    );
    assert!(org.close_goal(goal, "owner").await.unwrap());
    let withdrawn = org.withdraw_obsolete_handoffs().await.unwrap();
    assert_eq!(withdrawn.len(), 1, "{withdrawn:?}");
    assert_eq!(withdrawn[0].id, closed);
    assert_eq!(
        withdrawn[0].reason,
        "Withdrawn: the goal it was for is closed."
    );
    assert!(org.withdraw_obsolete_handoffs().await.unwrap().is_empty());

    assert_eq!(state_of(&org, closed).await.0, OwnerHandoffState::Withdrawn);
    // A payment keeps its provider's state; an item still needed is untouched.
    assert_eq!(state_of(&org, payment).await.0, OwnerHandoffState::Pending);
    assert_eq!(state_of(&org, live).await.0, OwnerHandoffState::Pending);
    // The Work is paused, not stuck: no new "stuck" card replaces the one withdrawn.
    let paused = org.get_work(work).await.unwrap().unwrap();
    assert!(
        paused.resolution.starts_with("paused by the daemon"),
        "{}",
        paused.resolution
    );
    assert!(org
        .stalled_work(Duration::zero())
        .await
        .unwrap()
        .iter()
        .all(|stalled| stalled.work_id != work));
    org.drop_schema().await.unwrap();
}
