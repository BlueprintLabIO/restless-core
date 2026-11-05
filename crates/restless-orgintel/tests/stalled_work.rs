//! Work stopped with nobody asked gets one owner card and one notice to its lead per stall
//! (S109 blockers audit, finding 10): blocked Work with no pending handoff, and Work stranded by an
//! abandoned requirement (left over from before #603). Paused Work and Work waiting on an owner
//! handoff are not stalled: someone already knows.

use restless_orgintel::{
    NewWork, OrgIntel, StallCause, WorkAttemptState, WorkEdgeKind, WorkspaceSpec,
};

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "stalledwork{}_test",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    );
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    for (id, kind, role) in [
        ("owner", "owner", "owner"),
        ("exec", "exec", "exec"),
        ("alpha-build", "staff", "builder"),
        ("beta-check", "staff", "checker"),
    ] {
        org.ensure_actor(id, kind, role, id).await.unwrap();
    }
    Some(org)
}

fn work<'a>(owner: &'a str, title: &'a str) -> NewWork<'a> {
    NewWork {
        owner_id: owner,
        title,
        outcome: "done",
        goal_id: None,
        priority: 1,
        expected_artifact: "note",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(1),
    }
}

/// S109 finding 18: a judgement in the owner's queue whose source changed after it was briefed
/// keeps its card (attention), and whoever briefed it is asked once to brief it again.
#[tokio::test]
async fn a_stale_owner_brief_is_asked_for_again_once() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping stale brief scenario");
        return;
    };
    let work = org
        .add_work(work("alpha-build", "Choose the launch price"))
        .await
        .unwrap();
    let handoff = org
        .request_owner_handoff(restless_orgintel::NewOwnerHandoff {
            work_id: work,
            attempt_id: None,
            requested_by: "alpha-build",
            category: restless_orgintel::OwnerHandoffCategory::OwnerJudgement,
            requested_action: "decide the price",
            prepared_state: "two prices prepared",
            resume_condition: "a decision is recorded",
        })
        .await
        .unwrap();
    org.prepare_owner_brief(
        handoff,
        "exec",
        restless_orgintel::OwnerBrief {
            kind: restless_orgintel::OwnerBriefKind::Decision,
            headline: "Choose the launch price".into(),
            situation: "Two prices are prepared.".into(),
            impact: "The price sets the first month's revenue.".into(),
            recommendation: "Choose the lower price.".into(),
            no_action: "The launch waits.".into(),
            uncertainty: None,
            deadline: None,
        },
    )
    .await
    .unwrap();
    org.escalate_handoff(handoff, "exec", "the price is the owner's call")
        .await
        .unwrap();
    assert_eq!(
        org.ask_for_stale_owner_briefs().await.unwrap(),
        0,
        "a current brief"
    );

    // The prepared meaning changes under the brief.
    org.refresh_owner_handoff(
        handoff,
        "alpha-build",
        "decide the price, now with a third option",
        "three prices prepared",
        "a decision is recorded",
    )
    .await
    .unwrap();
    let row = org
        .list_owner_handoffs()
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.id == handoff)
        .unwrap();
    if row.assigned_to.is_some() {
        // Refreshing already returned it to someone below the owner: their queue holds it.
        assert_eq!(org.ask_for_stale_owner_briefs().await.unwrap(), 0);
    } else {
        assert_eq!(org.ask_for_stale_owner_briefs().await.unwrap(), 1);
        assert_eq!(
            org.ask_for_stale_owner_briefs().await.unwrap(),
            0,
            "asked once per revision"
        );
        let told = org.inbox(Some("exec")).await.unwrap();
        assert!(
            told.iter().any(|message| message.from_actor == "daemon"
                && message.body.contains("prepare-owner-brief")),
            "{told:?}"
        );
    }
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn stalled_work_is_found_once_and_its_lead_is_told_once() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping stalled Work scenario");
        return;
    };
    // Blocked by its Attempt, nobody asked.
    let release = org
        .add_work(work("alpha-build", "Reconcile the 0.50.4 release"))
        .await
        .unwrap();
    let claimed = org.claim_ready_work("blocked").await.unwrap().unwrap();
    assert_eq!(claimed.work.id, release);
    org.finish_work_attempt(
        claimed.attempt_id,
        WorkAttemptState::Blocked,
        "the registry token is missing",
    )
    .await
    .unwrap();

    // Paused on purpose: not stalled.
    let paused = org
        .add_work(work("alpha-build", "Paused on purpose"))
        .await
        .unwrap();
    org.pause_work(paused, "owner", "waiting for the launch")
        .await
        .unwrap();

    // Stranded by an abandoned requirement, as Frankensurf's "Prove which agent tools work" was.
    let upstream = org
        .add_work(work("alpha-build", "Prove the tools"))
        .await
        .unwrap();
    let stranded = org
        .add_work(work("beta-check", "Check the proof"))
        .await
        .unwrap();
    org.add_work_edge(upstream, stranded, WorkEdgeKind::Requires)
        .await
        .unwrap();
    org.abandon_work(upstream, "owner", "superseded")
        .await
        .unwrap();

    // Nothing has been stopped for a day yet.
    assert!(org
        .stalled_work(restless_orgintel::STALLED_AFTER)
        .await
        .unwrap()
        .is_empty());

    let stalled = org.stalled_work(chrono::Duration::zero()).await.unwrap();
    assert_eq!(stalled.len(), 2, "{stalled:?}");
    let blocked = stalled.iter().find(|work| work.work_id == release).unwrap();
    assert_eq!(blocked.lead, "alpha-build");
    assert!(
        matches!(&blocked.cause, StallCause::Blocked { reason } if reason.contains("registry token")),
        "{blocked:?}"
    );
    assert!(blocked.way_through().contains("resume"));
    let requires = stalled
        .iter()
        .find(|work| work.work_id == stranded)
        .unwrap();
    assert_eq!(requires.lead, "beta-check");
    assert!(matches!(
        &requires.cause,
        StallCause::AbandonedRequirement { upstream_id, .. } if *upstream_id == upstream
    ));
    assert!(!stalled.iter().any(|work| work.work_id == paused));

    // Each lead hears once, from the daemon, never in Exec's name.
    assert_eq!(
        org.tell_leads_of_stalled_work(chrono::Duration::zero())
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        org.tell_leads_of_stalled_work(chrono::Duration::zero())
            .await
            .unwrap(),
        0,
        "a stall is told once"
    );
    let alpha = org.inbox(Some("alpha-build")).await.unwrap();
    let notice = alpha
        .iter()
        .find(|message| message.body.contains(&release.to_string()))
        .expect("the blocked Work's lead is told");
    assert_eq!(notice.from_actor, "daemon");
    assert!(
        notice.body.contains("restless work resume"),
        "{}",
        notice.body
    );
    let beta = org.inbox(Some("beta-check")).await.unwrap();
    assert!(
        beta.iter().any(|message| message.from_actor == "daemon"
            && message.body.contains(&stranded.to_string())
            && message.body.contains("--remove")),
        "the stranded Work's lead hears how to unblock it: {beta:?}"
    );

    // Work a model provider refused is listed for resuming once the refusal clears; the rest is not.
    let refused = org
        .add_work(work("alpha-build", "Write the changelog"))
        .await
        .unwrap();
    let claimed = org.claim_ready_work("refused").await.unwrap().unwrap();
    assert_eq!(claimed.work.id, refused);
    org.finish_work_attempt(
        claimed.attempt_id,
        WorkAttemptState::Blocked,
        "[quota] provider refused on quota or credit — owner action required: 402",
    )
    .await
    .unwrap();
    assert_eq!(
        org.work_blocked_by_provider_refusal().await.unwrap(),
        vec![refused]
    );

    // Dropping the requirement clears the stranded Work.
    org.remove_work_edge(
        upstream,
        stranded,
        WorkEdgeKind::Requires,
        "owner",
        "superseded",
    )
    .await
    .unwrap();
    let stalled = org.stalled_work(chrono::Duration::zero()).await.unwrap();
    assert!(!stalled.iter().any(|work| work.work_id == stranded));
    assert!(stalled.iter().any(|work| work.work_id == release));

    org.drop_schema().await.unwrap();
}
