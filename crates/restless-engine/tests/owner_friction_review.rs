//! The company reviews itself on its owner's friction (20 November 2026,
//! docs/specs/owner-interaction.md D7): status checks above 8% of the owner's messages make the
//! review due even with no Work finished, recording the review settles it while the same week's
//! numbers stay high, and Work sent back after it was done makes it due again. Exec's judgements
//! of the owner's messages, recorded from a scripted Exec's replies, come first. Needs
//! `RESTLESS_TEST_DATABASE_URL`; skipped otherwise.

use chrono::Utc;
use restless_engine::org_review::{exec_block, scorecard, stored};
use restless_orgintel::{NewArtifactRef, NewWork, OrgIntel, WorkAttemptState, WorkspaceSpec};

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "frictionreview{}_test",
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

async fn owner_says(org: &OrgIntel, body: &str) {
    let id = org.send_message("owner", Some("exec"), body).await.unwrap();
    org.mark_read(id).await.unwrap();
}

async fn owner_says_id(org: &OrgIntel, body: &str) -> i64 {
    let id = org.send_message("owner", Some("exec"), body).await.unwrap();
    org.mark_read(id).await.unwrap();
    id
}

/// The scripted Exec: answers the owner and judges the message on its reply, as the prompt asks.
/// The runtime records the judgement from the reply it saved (`record_owner_turn`).
async fn exec_answers(org: &OrgIntel, owner_message: i64, judgement: &str) {
    let reply = format!(
        "Here it is.\n\n<!--restless-owner-turn:{judgement}-->\n\
         <!--restless-intent:{{\"kind\":\"conversation\",\"summary\":\"Answered.\"}}-->"
    );
    let (visible, _) = restless_orgintel::take_owner_turn_marker(&reply);
    org.send_message("exec", Some("owner"), &visible)
        .await
        .unwrap();
    assert_eq!(
        org.record_owner_turn(&reply, &[owner_message])
            .await
            .unwrap(),
        Some(owner_message)
    );
}

/// Exec's labels decide (21 November 2026): three messages in the owner's last ten it could have
/// spared them, two with the same gap, make a review due with no status-check share and no Work
/// finished, and the review is told to hand the repair to the lead who owns it as Work.
#[tokio::test]
async fn exec_judgements_make_the_review_due_and_hand_repairs_to_leads() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping owner friction review");
        return;
    };
    let fair = "{\"intents\":[\"instruction\"],\"avoidable\":\"no\"}";
    for n in 0..6 {
        let id = owner_says_id(&org, &format!("draft launch email number {n}")).await;
        exec_answers(&org, id, fair).await;
    }
    // The owner had to ask whether the drafts landed: twice the same gap.
    for body in [
        "did the emails go out? also draft one for the partners",
        "and the partner one, did that land?",
    ] {
        let id = owner_says_id(&org, body).await;
        exec_answers(
            &org,
            id,
            "{\"intents\":[\"status_check\",\"instruction\"],\"avoidable\":\"yes\",\"gap\":\"told\",\
             \"couldHaveKnown\":\"The send receipts were in the effect ledger.\",\
             \"repair\":\"The email lead posts each send with its receipt.\",\
             \"repairLead\":\"alpha-build\"}",
        )
        .await;
    }
    let card = scorecard(&org, Utc::now()).await.unwrap();
    assert_eq!((card.friction.judged, card.friction.avoidable), (8, 2));
    // Two of the last ten is not yet a review, but the same gap twice is.
    assert!(card.due, "{:?}", card.friction_due);
    assert_eq!(card.friction_due.len(), 1, "{:?}", card.friction_due);
    assert!(
        card.friction_due[0].contains("same gap came back 2 times"),
        "{:?}",
        card.friction_due
    );
    // A third avoidable message, a different gap: now the share is due too.
    let id = owner_says_id(&org, "you asked me to check the copy, can't you?").await;
    exec_answers(
        &org,
        id,
        "{\"intents\":[\"redirect\"],\"avoidable\":\"yes\",\"gap\":\"handoff\",\
         \"repair\":\"The email lead proofreads before asking.\",\"repairLead\":\"alpha-build\"}",
    )
    .await;
    let card = scorecard(&org, Utc::now()).await.unwrap();
    assert!(
        card.friction_due
            .iter()
            .any(|reason| reason.contains("3 of the owner's last 9 messages")),
        "{:?}",
        card.friction_due
    );
    let block = exec_block(&card).expect("a due review is shown to Exec");
    assert!(
        block.contains(
            "repair: The email lead posts each send with its receipt. (lead alpha-build)"
        ),
        "{block}"
    );
    assert!(
        block.contains("restless commission --lead <lead>"),
        "{block}"
    );
    // 2 status checks in 9 is 22%, but Exec judged them: the rules' share stands down.
    assert!(
        !card
            .friction_due
            .iter()
            .any(|reason| reason.contains("status checks are")),
        "{:?}",
        card.friction_due
    );

    // Exec reviews and commissions the repairs; the same week asks nothing more.
    org.record_org_review(
        "exec",
        card.signals_through,
        3,
        stored(&card),
        "Owner asked twice whether sends landed: the email lead now posts each send.",
    )
    .await
    .unwrap();
    let card = scorecard(&org, Utc::now()).await.unwrap();
    assert!(!card.due, "{:?}", card.friction_due);
    org.drop_schema().await.unwrap();
}

/// The fallback: with no judgement from Exec, the rules' classes decide.
#[tokio::test]
async fn owner_friction_makes_the_review_due_once_per_new_signal() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping owner friction review");
        return;
    };
    // Nothing said: nothing due, and an ordinary wake carries no review.
    let card = scorecard(&org, Utc::now()).await.unwrap();
    assert!(!card.due);
    assert!(exec_block(&card).is_none());

    // Eight plain requests and two "progress?": 20% status checks, no Work finished at all.
    for n in 0..8 {
        owner_says(&org, &format!("draft launch email number {n}")).await;
    }
    owner_says(&org, "progress?").await;
    owner_says(&org, "status").await;
    let card = scorecard(&org, Utc::now()).await.unwrap();
    assert_eq!(card.friction.owner_messages, 10);
    assert_eq!(card.friction.status_checks, 2);
    assert!(card.due, "{:?}", card.friction_due);
    assert_eq!(card.friction_due.len(), 1, "{:?}", card.friction_due);
    assert!(
        card.friction_due[0].contains("20%"),
        "{:?}",
        card.friction_due
    );
    let block = exec_block(&card).expect("a due review is shown to Exec");
    assert!(
        block.contains("status checks 2 (20%, baseline 8%)"),
        "{block}"
    );
    assert!(block.contains("review due: status checks"), "{block}");

    // Exec reviews. The week still holds 20%, but nothing is new: not due.
    org.record_org_review(
        "exec",
        card.signals_through,
        2,
        stored(&card),
        "Status asked twice: Exec now posts when Work lands.",
    )
    .await
    .unwrap();
    let card = scorecard(&org, Utc::now()).await.unwrap();
    assert_eq!(card.friction.status_checks, 2);
    assert!(!card.due, "{:?}", card.friction_due);
    assert!(card.last_review.as_ref().unwrap().friction.is_some());

    // Work lands and the owner sends it back: due again, on that alone.
    let work = org
        .add_work(NewWork {
            owner_id: "alpha-build",
            title: "Draft the launch email",
            outcome: "an email",
            goal_id: None,
            priority: 1,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let attempt = org.claim_ready_work("test").await.unwrap().unwrap();
    org.link_work_artifact(NewArtifactRef {
        kind: "path",
        uri: "/company/email.md",
        note: "the email",
        created_by: "alpha-build",
        work_id: Some(work),
        attempt_id: Some(attempt.attempt_id),
        digest: Some("sha256:email"),
        source_commit: None,
        runtime_generation: Some("test"),
        label: "email",
    })
    .await
    .unwrap();
    org.finish_work_attempt(attempt.attempt_id, WorkAttemptState::Produced, "drafted")
        .await
        .unwrap();
    let card = scorecard(&org, Utc::now()).await.unwrap();
    assert_eq!(card.friction.work_landed, 1);
    assert!(!card.due, "landing is not friction");
    org.reopen_work(work, "owner", "lead with the guarantee")
        .await
        .unwrap();
    let card = scorecard(&org, Utc::now()).await.unwrap();
    assert!(card.due);
    assert!(
        card.friction_due
            .iter()
            .any(|reason| reason.contains("sent back after it was done")),
        "{:?}",
        card.friction_due
    );
    org.drop_schema().await.unwrap();
}
