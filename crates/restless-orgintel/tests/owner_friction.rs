//! The company knows what kind of message the owner sent and counts the owner's friction with
//! it (20 November 2026, docs/specs/owner-interaction.md D3 and D7): every owner message gets a
//! class stored on it, agent messages never do, a redirect run is two redirects in a row on the
//! same thread, reopened and landed Work are counted, and the owner's answer to a check-in is an
//! approval, not a nudge. Runs only when RESTLESS_TEST_DATABASE_URL is set.

use chrono::{Duration, Utc};
use restless_orgintel::{
    CheckinKind, CheckinRhythm, NewArtifactRef, NewWork, OrgIntel, OwnerIntent, WorkAttemptState,
    WorkStatus, WorkspaceSpec,
};

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "friction{}_test",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
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

/// The owner writes to `to` and it has been read.
async fn owner_to(org: &OrgIntel, to: &str, body: &str) -> i64 {
    let id = org.send_message("owner", Some(to), body).await.unwrap();
    org.mark_read(id).await.unwrap();
    id
}

async fn exec_says(org: &OrgIntel, body: &str) -> i64 {
    org.send_message("exec", Some("owner"), body).await.unwrap()
}

fn work<'a>(title: &'a str) -> NewWork<'a> {
    NewWork {
        owner_id: "alpha-build",
        title,
        outcome: "done",
        goal_id: None,
        priority: 1,
        expected_artifact: "",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(1),
    }
}

#[tokio::test]
async fn owner_messages_are_classified_and_the_friction_is_counted() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping owner friction scenario");
        return;
    };
    let start = Utc::now() - Duration::minutes(1);
    let empty = org.owner_friction(start, start).await.unwrap();
    assert_eq!(empty.owner_messages, 0);
    assert_eq!(empty.work_landed, 0);

    // A big ask with no method, then two corrections in a row on the same thread.
    let outcome = owner_to(&org, "exec", "fix all root causes properly").await;
    exec_says(&org, "On it.").await;
    let first = owner_to(&org, "exec", "no, that's wrong").await;
    let second = owner_to(&org, "exec", "I said the importer, not the exporter").await;
    let status = owner_to(&org, "exec", "progress?").await;
    let instruction = owner_to(&org, "exec", "rename the button to Save").await;
    owner_to(&org, "exec", "thanks").await;
    // A correction to someone else is another thread: it neither starts nor breaks a run here.
    let elsewhere = owner_to(&org, "alpha-build", "no").await;
    let wrong = owner_to(&org, "exec", "wrong").await;
    let still = owner_to(&org, "exec", "still broken").await;
    let reply = exec_says(&org, "Fixing the importer now.").await;

    let friction = org.owner_friction(start, start).await.unwrap();
    for (id, want) in [
        (outcome, OwnerIntent::OutcomeOnly),
        (first, OwnerIntent::Redirect),
        (second, OwnerIntent::Redirect),
        (status, OwnerIntent::StatusCheck),
        (instruction, OwnerIntent::Instruction),
        (elsewhere, OwnerIntent::Redirect),
        (wrong, OwnerIntent::Redirect),
        (still, OwnerIntent::Redirect),
    ] {
        assert_eq!(
            org.message_owner_intent(id).await.unwrap(),
            Some(want),
            "message {id}"
        );
    }
    assert_eq!(
        org.message_owner_intent(reply).await.unwrap(),
        None,
        "an agent's message is never classified"
    );
    assert_eq!(friction.owner_messages, 9, "{friction:?}");
    assert_eq!(friction.status_checks, 1);
    assert_eq!(
        (friction.outcome_only, friction.outcome_only_redirected),
        (1, 1)
    );
    assert_eq!(
        (friction.instructions, friction.instructions_redirected),
        (1, 0)
    );
    assert_eq!(friction.redirects, 5);
    assert_eq!(
        friction.redirect_runs, 2,
        "\"no, that's wrong\" then \"I said\", and \"wrong\" then \"still broken\""
    );
    assert_eq!(
        (friction.new_status_checks, friction.new_redirect_runs),
        (1, 2)
    );

    // Classified once: reading again stores nothing new.
    assert_eq!(org.classify_owner_messages().await.unwrap(), 0);

    // Work that lands, then is sent back after it was done.
    let landed = org.add_work(work("Fix the importer")).await.unwrap();
    let attempt = org.claim_ready_work("test").await.unwrap().unwrap();
    assert_eq!(attempt.work.id, landed);
    org.link_work_artifact(NewArtifactRef {
        kind: "path",
        uri: "/company/importer.md",
        note: "the fix",
        created_by: "alpha-build",
        work_id: Some(landed),
        attempt_id: Some(attempt.attempt_id),
        digest: Some("sha256:importer"),
        source_commit: None,
        runtime_generation: Some("test"),
        label: "fix",
    })
    .await
    .unwrap();
    org.finish_work_attempt(attempt.attempt_id, WorkAttemptState::Produced, "fixed")
        .await
        .unwrap();
    assert_eq!(
        org.get_work(landed).await.unwrap().unwrap().status,
        WorkStatus::Completed
    );
    let friction = org.owner_friction(start, start).await.unwrap();
    assert_eq!((friction.work_landed, friction.work_reopened), (1, 0));
    org.reopen_work(landed, "owner", "the importer still drops rows")
        .await
        .unwrap();
    let friction = org.owner_friction(start, start).await.unwrap();
    assert_eq!(
        (
            friction.work_landed,
            friction.work_reopened,
            friction.new_work_reopened
        ),
        (0, 1, 1),
        "reopened Work is no longer landed"
    );

    // After a review (here: counted from now) nothing is new, though the week still holds it.
    let later = org.owner_friction(start, Utc::now()).await.unwrap();
    assert_eq!(later.redirect_runs, 2);
    assert_eq!(
        (
            later.new_status_checks,
            later.new_redirect_runs,
            later.new_work_reopened
        ),
        (0, 0, 0)
    );
    // And a week that starts after all of it is empty.
    let next_week = org
        .owner_friction(Utc::now() + Duration::seconds(1), Utc::now())
        .await
        .unwrap();
    assert_eq!(next_week.owner_messages, 0);
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn an_answer_to_a_checkin_is_an_approval_and_still_not_a_nudge() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping owner friction scenario");
        return;
    };
    let start = Utc::now() - Duration::minutes(1);
    owner_to(&org, "exec", "ship the launch page this week").await;
    exec_says(&org, "On it.").await;
    // Open Work, so Exec has something to check in about.
    org.add_work(work("Publish the release")).await.unwrap();
    let claimed = org.claim_ready_work("test").await.unwrap().unwrap();
    org.finish_work_attempt(
        claimed.attempt_id,
        WorkAttemptState::Blocked,
        "needs a token",
    )
    .await
    .unwrap();
    let talked = org.checkin_state().await.unwrap().heard_at.unwrap();
    let asked = org
        .signal_checkin_at(talked + Duration::minutes(61), CheckinRhythm::default())
        .await
        .unwrap()
        .map(|(kind, _)| kind);
    assert_eq!(asked, Some(CheckinKind::Checkin));
    exec_says(
        &org,
        "The release waits on a token. 1. Use the old one or wait? I would wait.",
    )
    .await;

    // "continue" answers it; the next "continue" is the owner pushing.
    let answer = owner_to(&org, "exec", "continue").await;
    let push = owner_to(&org, "exec", "continue").await;
    org.owner_friction(start, start).await.unwrap();
    assert_eq!(
        org.message_owner_intent(answer).await.unwrap(),
        Some(OwnerIntent::Approval)
    );
    assert_eq!(
        org.message_owner_intent(push).await.unwrap(),
        Some(OwnerIntent::Instruction)
    );
    // The check-in rule for the org review is unchanged: the answer is not a signal, the push is.
    let signals = org.owner_signals_after(start).await.unwrap();
    let continues = signals
        .iter()
        .filter(|signal| signal.detail == "continue")
        .count();
    assert_eq!(continues, 1, "{signals:?}");
    org.drop_schema().await.unwrap();
}
