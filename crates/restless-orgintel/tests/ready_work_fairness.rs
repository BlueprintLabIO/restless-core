//! S113 A: ready Work behind a busy owner. The waiting reason must read the facts the claim
//! decides by, overdue Work goes ahead of mail, and neither a person nor mail can be starved.
//! Runs only when RESTLESS_TEST_DATABASE_URL is set.

use std::time::Duration;

use restless_orgintel::{
    MailSettle, NewWork, OrgIntel, WorkAttemptState, WorkEdgeKind, WorkStartFacts, WorkWaitReason,
    WorkspaceSpec,
};
use uuid::Uuid;

async fn company(name: &str) -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let schema = format!(
        "fair_{name}_{}_{}",
        std::process::id(),
        &Uuid::new_v4().simple().to_string()[..6]
    );
    let org = OrgIntel::ensure(&url, &schema).await.expect("schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("bakery-web", "staff", "bakery-web", "Lead")
        .await
        .unwrap();
    org.ensure_actor("bakery-pack", "staff", "maker", "Peer")
        .await
        .unwrap();
    Some(org)
}

async fn work(org: &OrgIntel, title: &str) -> Uuid {
    org.add_work(NewWork {
        goal_id: None,
        owner_id: "bakery-web",
        title,
        outcome: "done",
        priority: 10,
        expected_artifact: "a file",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(5),
    })
    .await
    .unwrap()
}

async fn sql(org: &OrgIntel, statement: &str, work_id: Uuid) {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let statement = statement.replace("work ", &format!("\"{}\".work ", org.schema()));
    sqlx::query(&statement)
        .bind(work_id)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

/// The clock of a Work: make it look ready since this long ago. `updated_at` is kept by a trigger,
/// so the trigger steps aside for this one write.
async fn age_work(org: &OrgIntel, work_id: Uuid, seconds: i64) {
    let schema = org.schema();
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(&format!(
        "ALTER TABLE \"{schema}\".work DISABLE TRIGGER work_touch_updated_at"
    ))
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(&format!(
        "UPDATE \"{schema}\".work SET created_at=now()-interval '{seconds} seconds', \
         updated_at=now()-interval '{seconds} seconds' WHERE id=$1"
    ))
    .bind(work_id)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query(&format!(
        "ALTER TABLE \"{schema}\".work ENABLE TRIGGER work_touch_updated_at"
    ))
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    pool.close().await;
}

fn facts_for(rows: &[WorkStartFacts], work_id: Uuid) -> WorkStartFacts {
    rows.iter()
        .find(|row| row.work_id == work_id)
        .cloned()
        .expect("startable Work has facts")
}

const LIMIT: Duration = Duration::from_secs(120);

/// Steady mail must not hold a finished Attempt open: once the worker has declared its outcome,
/// agent mail waits unread for the next turn, and the plain checkpoint still delivers it.
#[tokio::test]
async fn agent_mail_cannot_hold_a_declared_attempt_open() {
    let Some(org) = company("hold").await else {
        return;
    };
    work(&org, "ready").await;
    let attempt = org.claim_ready_work("scan").await.unwrap().expect("claim");
    let mail = org
        .send_message("bakery-pack", Some("bakery-web"), "any news?")
        .await
        .unwrap();

    let directed = org
        .checkpoint_attempt_directed_feedback(attempt.attempt_id)
        .await
        .unwrap();
    assert!(
        directed.is_empty(),
        "agent mail does not reopen a declared turn"
    );
    assert_eq!(org.conversation_inbox("bakery-web").await.unwrap().len(), 1);

    let plain = org
        .checkpoint_attempt_feedback(attempt.attempt_id)
        .await
        .unwrap();
    assert_eq!(plain.iter().map(|m| m.id).collect::<Vec<_>>(), vec![mail]);
    assert!(org
        .conversation_inbox("bakery-web")
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn the_waiting_reason_cannot_disagree_with_the_claim() {
    let Some(org) = company("label").await else {
        return;
    };
    let ready = work(&org, "ready").await;

    // Free owner: no reason, and the claim takes it.
    let facts = org.work_start_facts().await.unwrap();
    assert_eq!(facts_for(&facts, ready).wait(false, None), None);
    assert!(org.has_ready_work().await.unwrap());

    // Registry says the owner is supervised: the label says conversation and the scan excludes it.
    assert_eq!(
        facts_for(&facts, ready).wait(true, None),
        Some(WorkWaitReason::OwnerInConversation)
    );
    assert!(org
        .claim_ready_work_excluding("scan", &["bakery-web".to_string()])
        .await
        .unwrap()
        .is_none());

    // Live lease: same reason, from the database alone, and no claim.
    let lease = org
        .claim_actor_cognitive_session("bakery-web", Duration::from_secs(60))
        .await
        .unwrap()
        .expect("lease");
    let facts = org.work_start_facts().await.unwrap();
    assert!(facts_for(&facts, ready).owner_in_conversation);
    assert_eq!(
        facts_for(&facts, ready).wait(false, None),
        Some(WorkWaitReason::OwnerInConversation)
    );
    assert!(org.claim_ready_work("scan").await.unwrap().is_none());
    org.release_actor_cognitive_session(&lease).await.unwrap();

    // Running Attempt on another Work of the owner.
    let other = work(&org, "other").await;
    let running = org.claim_ready_work("scan").await.unwrap().expect("claim");
    let waiting = if running.work.id == ready {
        other
    } else {
        ready
    };
    let facts = org.work_start_facts().await.unwrap();
    assert_eq!(
        facts_for(&facts, waiting).wait(false, None),
        Some(WorkWaitReason::OwnerOnOtherWork)
    );
    assert!(org.claim_ready_work("scan").await.unwrap().is_none());
    assert!(
        !facts.iter().any(|row| row.work_id == running.work.id),
        "running Work is not waiting"
    );
    org.finish_work_attempt(running.attempt_id, WorkAttemptState::Abandoned, "done")
        .await
        .unwrap();

    // A requirement not done.
    let downstream = work(&org, "downstream").await;
    org.add_work_edge(waiting, downstream, WorkEdgeKind::Requires)
        .await
        .unwrap();
    let facts = org.work_start_facts().await.unwrap();
    assert_eq!(
        facts_for(&facts, downstream).wait(false, None),
        Some(WorkWaitReason::WaitingOnWork)
    );

    // Capacity only speaks when nothing else holds this Work.
    assert_eq!(
        facts_for(&facts, waiting).wait(false, Some(WorkWaitReason::NotEnoughMemory)),
        Some(WorkWaitReason::NotEnoughMemory)
    );
    assert_eq!(
        facts_for(&facts, downstream).wait(false, Some(WorkWaitReason::NoActorSlot)),
        Some(WorkWaitReason::WaitingOnWork)
    );
}

#[tokio::test]
async fn overdue_means_waited_for_a_turn_not_merely_old() {
    let Some(org) = company("age").await else {
        return;
    };
    let fresh = work(&org, "fresh").await;
    assert!(!org.has_overdue_ready_work(&[], LIMIT).await.unwrap());
    assert!(org
        .claim_overdue_work_excluding("scan", &[], LIMIT)
        .await
        .unwrap()
        .is_none());

    age_work(&org, fresh, 121).await;
    assert!(org.has_overdue_ready_work(&[], LIMIT).await.unwrap());
    assert!(
        !org.has_overdue_ready_work(&["bakery-web".to_string()], LIMIT)
            .await
            .unwrap(),
        "an owner whose mail goes first is left out"
    );

    // A requirement that just finished starts the wait again, however old the Work is.
    let downstream = work(&org, "downstream").await;
    org.add_work_edge(fresh, downstream, WorkEdgeKind::Requires)
        .await
        .unwrap();
    age_work(&org, downstream, 3600).await;
    let claimed = org
        .claim_overdue_work_excluding("scan", &[], LIMIT)
        .await
        .unwrap()
        .expect("the aged upstream is overdue");
    assert_eq!(claimed.work.id, fresh);
    org.finish_work_attempt(claimed.attempt_id, WorkAttemptState::Abandoned, "again")
        .await
        .unwrap();
    sql(
        &org,
        "UPDATE work SET status='completed' WHERE id=$1",
        fresh,
    )
    .await;
    assert!(
        org.claim_overdue_work_excluding("scan", &[], LIMIT)
            .await
            .unwrap()
            .is_none(),
        "dependency finished moments ago: this Work has not waited yet"
    );
    assert!(
        org.has_ready_work().await.unwrap(),
        "yet it is ready for an ordinary claim"
    );
}

#[tokio::test]
async fn a_person_goes_first_and_mail_is_passed_at_most_once() {
    let Some(org) = company("mail").await else {
        return;
    };
    let ready = work(&org, "ready").await;
    age_work(&org, ready, 300).await;

    // Staff mail alone: nothing holds the Work back, and the mail stays unread.
    let staff_mail = org
        .send_message("bakery-pack", Some("bakery-web"), "any news?")
        .await
        .unwrap();
    assert!(org.actors_owed_by_a_person(128).await.unwrap().is_empty());
    let passed = org
        .actors_with_mail_passed_by_work(128, MailSettle::IMMEDIATE)
        .await
        .unwrap();
    assert!(
        passed.is_empty(),
        "this mail has not sat through an Attempt yet: {passed:?}"
    );

    let claimed = org
        .claim_overdue_work_excluding("scan", &[], LIMIT)
        .await
        .unwrap()
        .expect("overdue Work goes ahead of Staff mail");
    assert_eq!(claimed.work.id, ready);
    let inbox = org.conversation_inbox("bakery-web").await.unwrap();
    assert_eq!(
        inbox.len(),
        1,
        "the mail is durable and still unread during the Attempt"
    );
    assert_eq!(inbox[0].id, staff_mail);

    // One actor, one process: no second claim while the Attempt runs.
    let other = work(&org, "second").await;
    age_work(&org, other, 300).await;
    assert!(org
        .claim_overdue_work_excluding("scan", &[], LIMIT)
        .await
        .unwrap()
        .is_none());
    assert!(org
        .claim_actor_cognitive_session("bakery-web", Duration::from_secs(60))
        .await
        .unwrap()
        .is_none());

    org.finish_work_attempt(claimed.attempt_id, WorkAttemptState::Abandoned, "ended")
        .await
        .unwrap();

    // The Attempt is over: the mail that sat through it goes first even though more Work is overdue.
    assert_eq!(
        org.actors_with_mail_passed_by_work(128, MailSettle::IMMEDIATE)
            .await
            .unwrap(),
        vec!["bakery-web".to_string()]
    );
    let inbox = org.conversation_inbox("bakery-web").await.unwrap();
    assert_eq!(inbox.len(), 1, "delivered once, when the actor next wakes");
    org.mark_read(staff_mail).await.unwrap();
    assert!(org
        .conversation_inbox("bakery-web")
        .await
        .unwrap()
        .is_empty());
    assert!(org
        .actors_with_mail_passed_by_work(128, MailSettle::IMMEDIATE)
        .await
        .unwrap()
        .is_empty());

    // Mail arriving after that does not hold the overdue Work.
    org.send_message("bakery-pack", Some("bakery-web"), "next one")
        .await
        .unwrap();
    assert!(org
        .actors_with_mail_passed_by_work(128, MailSettle::IMMEDIATE)
        .await
        .unwrap()
        .is_empty());

    // The owner's own message does.
    org.send_message("owner", Some("bakery-web"), "stop and look at this")
        .await
        .unwrap();
    assert_eq!(
        org.actors_owed_by_a_person(128).await.unwrap(),
        vec!["bakery-web".to_string()]
    );
}

/// S113 C: the next Work in a chain is claimable the moment its requirement finishes, and the
/// chain claim takes only that Work, never unrelated ready Work ahead of the scan's order.
#[tokio::test]
async fn the_next_work_in_a_chain_starts_on_its_requirement_finishing() {
    let Some(org) = company("chain").await else {
        return;
    };
    const WINDOW: Duration = Duration::from_secs(30);
    let draft = work(&org, "draft").await;
    let check = org
        .add_work(NewWork {
            goal_id: None,
            owner_id: "bakery-pack",
            title: "check",
            outcome: "checked",
            priority: 10,
            expected_artifact: "a note",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(5),
        })
        .await
        .unwrap();
    org.add_work_edge(draft, check, WorkEdgeKind::Requires)
        .await
        .unwrap();
    // Unrelated ready Work the chain pass must leave for the scan.
    let elsewhere = work(&org, "elsewhere").await;
    assert!(!org.has_chain_work_ready(&[], WINDOW).await.unwrap());
    assert!(org
        .claim_chain_work_excluding("t", &[], WINDOW)
        .await
        .unwrap()
        .is_none());

    let claimed = org
        .claim_ready_work_excluding("t", &["bakery-pack".to_string()])
        .await
        .unwrap()
        .expect("the draft is ready");
    assert_eq!(claimed.work.id, draft);
    org.finish_work_attempt(claimed.attempt_id, WorkAttemptState::Failed, "ran")
        .await
        .unwrap();
    sql(
        &org,
        "UPDATE work SET status='completed' WHERE id=$1",
        draft,
    )
    .await;

    // A person's mail keeps its turn: its owner is left out and the chain pass takes nothing.
    assert!(org
        .claim_chain_work_excluding("t", &["bakery-pack".to_string()], WINDOW)
        .await
        .unwrap()
        .is_none());
    let next = org
        .claim_chain_work_excluding("t", &[], WINDOW)
        .await
        .unwrap()
        .expect("the check starts now");
    assert_eq!(next.work.id, check);
    assert_eq!(next.work.owner_id, "bakery-pack");
    assert!(org.get_work(elsewhere).await.unwrap().is_some());
}
