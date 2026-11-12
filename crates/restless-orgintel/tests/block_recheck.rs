//! Blocked work that waits on something outside the company is looked at again without the
//! owner saying "try now" (12 November 2026): after 1, 6 and 24 hours, then never again, and
//! once shortly after the owner was active. Each recheck is one short message to the waiting
//! agent. Paused Work, a model refusal, a judgement, a legal attestation and an app request are
//! not rechecked this way.

use chrono::Duration;
use restless_orgintel::{
    NewOwnerHandoff, NewWork, OrgIntel, OwnerHandoffCategory, OwnerHandoffState, WorkAttemptState,
    WorkspaceSpec,
};

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "blockrecheck{}_test",
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

fn work<'a>(title: &'a str) -> NewWork<'a> {
    NewWork {
        owner_id: "alpha-build",
        title,
        outcome: "done",
        goal_id: None,
        priority: 1,
        expected_artifact: "note",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(1),
    }
}

async fn block(org: &OrgIntel, title: &str, reason: &str) -> uuid::Uuid {
    let id = org.add_work(work(title)).await.unwrap();
    let claimed = org.claim_ready_work("test").await.unwrap().unwrap();
    assert_eq!(claimed.work.id, id);
    org.finish_work_attempt(claimed.attempt_id, WorkAttemptState::Blocked, reason)
        .await
        .unwrap();
    id
}

async fn rechecks_sent(org: &OrgIntel) -> Vec<String> {
    org.inbox(Some("alpha-build"))
        .await
        .unwrap()
        .into_iter()
        .filter(|message| message.from_actor == "daemon" && message.body.starts_with("Recheck:"))
        .map(|message| message.body)
        .collect()
}

#[tokio::test]
async fn blocked_work_is_rechecked_three_times_on_a_backoff_and_then_left_alone() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping block recheck scenario");
        return;
    };
    assert!(
        !org.note_owner_activity("viewer_closed").await.unwrap(),
        "nothing waits, so nothing is noted"
    );
    assert_eq!(org.next_block_recheck_at().await.unwrap(), None);

    let release = block(&org, "Publish the release", "the registry token is missing").await;
    // Not this way: paused on purpose, and stopped by a model refusal (it has its own resume).
    let paused = org.add_work(work("Paused on purpose")).await.unwrap();
    org.pause_work(paused, "owner", "waiting for the launch")
        .await
        .unwrap();
    block(
        &org,
        "Write the changelog",
        "[quota] provider refused on quota or credit",
    )
    .await;

    let waits = org.external_waits().await.unwrap();
    assert_eq!(waits.len(), 1, "{waits:?}");
    assert_eq!(waits[0].work_id, release);
    assert_eq!(waits[0].agent, "alpha-build");
    let since = waits[0].since;
    assert_eq!(
        org.next_block_recheck_at().await.unwrap(),
        Some(since + Duration::hours(1))
    );

    // Nothing before the first hour.
    assert_eq!(
        org.recheck_external_waits_at(since + Duration::minutes(59))
            .await
            .unwrap(),
        0
    );
    for (hours, sent) in [(1, 1), (6, 2), (24, 3)] {
        let at = since + Duration::hours(hours) + Duration::seconds(1);
        assert_eq!(
            org.recheck_external_waits_at(at).await.unwrap(),
            1,
            "{hours}h"
        );
        assert_eq!(
            org.recheck_external_waits_at(at).await.unwrap(),
            0,
            "one recheck per threshold"
        );
        assert_eq!(rechecks_sent(&org).await.len(), sent);
    }
    assert_eq!(
        org.recheck_external_waits_at(since + Duration::days(30))
            .await
            .unwrap(),
        0,
        "it stops after three"
    );
    assert_eq!(org.next_block_recheck_at().await.unwrap(), None);
    let sent = rechecks_sent(&org).await;
    assert!(
        sent.iter().all(|body| body.contains(&release.to_string())
            && body.contains("restless work resume")
            && body.contains("registry token")),
        "{sent:?}"
    );
    // The agent's own way on still works: resuming clears the wait.
    org.resume_work(release, "alpha-build", "the token was added")
        .await
        .unwrap();
    assert!(org.external_waits().await.unwrap().is_empty());
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn a_sign_in_step_is_rechecked_after_the_owner_was_active_and_its_agent_may_confirm_it() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping sign-in recheck scenario");
        return;
    };
    let handoff = |work_id, category, action: &'static str| NewOwnerHandoff {
        work_id,
        attempt_id: None,
        requested_by: "alpha-build",
        category,
        requested_action: action,
        prepared_state: "the page is open on the company computer",
        resume_condition: "the session is signed in",
    };
    let sign_in_work = org.add_work(work("Post the listing")).await.unwrap();
    let sign_in = org
        .request_owner_handoff(handoff(
            sign_in_work,
            OwnerHandoffCategory::Identity,
            "Sign in to the marketplace",
        ))
        .await
        .unwrap();
    // A legal attestation and a judgement stay the owner's: never rechecked, never confirmed.
    let legal_work = org.add_work(work("File the form")).await.unwrap();
    let legal = org
        .request_owner_handoff(handoff(
            legal_work,
            OwnerHandoffCategory::LegalAttestation,
            "Attest the filing",
        ))
        .await
        .unwrap();
    // An app request is answered by observation in the engine, not by an agent wake.
    let app_work = org.add_work(work("Sync the tracker")).await.unwrap();
    let app = org
        .request_owner_handoff(handoff(
            app_work,
            OwnerHandoffCategory::Identity,
            "Add Linear",
        ))
        .await
        .unwrap();
    org.attach_app_to_handoff(app, "linear").await.unwrap();

    let waits = org.external_waits().await.unwrap();
    assert_eq!(waits.len(), 1, "{waits:?}");
    assert_eq!(waits[0].handoff_id, Some(sign_in));
    let since = waits[0].since;

    // Nobody asked yet: the agent cannot close its own request.
    assert!(!org
        .agent_may_confirm_human_step(sign_in, "alpha-build")
        .await
        .unwrap());

    // The owner writes to Exec. Two minutes later, not at the hour, the agent is asked to look.
    org.send_message("owner", Some("exec"), "ok signed in")
        .await
        .unwrap();
    assert!(org.note_owner_activity("viewer_closed").await.unwrap());
    assert!(
        !org.note_owner_activity("viewer_closed").await.unwrap(),
        "noted at most once a minute"
    );
    let active = org.owner_last_active_at().await.unwrap().unwrap();
    assert!(active >= since);
    assert_eq!(
        org.recheck_external_waits_at(active + Duration::seconds(30))
            .await
            .unwrap(),
        0,
        "the owner's act settles first"
    );
    assert_eq!(
        org.recheck_external_waits_at(active + Duration::minutes(3))
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        org.recheck_external_waits_at(active + Duration::minutes(4))
            .await
            .unwrap(),
        0,
        "the same activity asks once"
    );
    let sent = rechecks_sent(&org).await;
    assert_eq!(sent.len(), 1);
    assert!(
        sent[0].contains("The owner has just been active."),
        "{sent:?}"
    );
    assert!(
        sent[0].contains(&format!("resolve-handoff --handoff {sign_in}")),
        "{sent:?}"
    );

    // Asked, the waiting agent may record what it saw; nobody else, and no other kind of step.
    assert!(org
        .agent_may_confirm_human_step(sign_in, "alpha-build")
        .await
        .unwrap());
    assert!(!org
        .agent_may_confirm_human_step(sign_in, "exec")
        .await
        .unwrap());
    assert!(!org
        .agent_may_confirm_human_step(legal, "alpha-build")
        .await
        .unwrap());
    assert!(org
        .resolve_observed_handoff(
            sign_in,
            "daemon",
            "Checked by alpha-build: the session is in"
        )
        .await
        .unwrap());
    let row = org
        .list_owner_handoffs()
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.id == sign_in)
        .unwrap();
    assert_eq!(row.state, OwnerHandoffState::Resolved);
    assert_eq!(
        org.get_work(sign_in_work).await.unwrap().unwrap().status,
        restless_orgintel::WorkStatus::Active,
        "the Work goes on"
    );
    assert!(org.external_waits().await.unwrap().is_empty());
    org.drop_schema().await.unwrap();
}
