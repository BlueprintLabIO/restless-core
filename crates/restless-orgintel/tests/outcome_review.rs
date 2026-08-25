//! S17-T5: a qualified produced outcome reaches its accountable lead first,
//! then the owner exactly once after supervised inspection and admission.
//!
//! This is a real scratch-Postgres scenario. The Runtime's live probe is
//! represented by its recorded deterministic gate result; no pretend Runtime
//! or market source is introduced into OrgIntel.

use restless_orgintel::{
    InitialWorkGate, NewArtifactRef, NewGateRun, NewWork, OrgIntel, OwnerBrief, OwnerBriefKind,
    OwnerHandoffCategory, OwnerHandoffState, OwnerReviewDecision, WorkAttemptState, WorkStatus,
    WorkspaceSpec, REVIEW_TARGET_ARTIFACT_KIND, REVIEW_TARGET_LIVE_PROBE_GATE,
};

async fn add_review_required_work(org: &OrgIntel, title: &str) -> uuid::Uuid {
    let probe_command = vec![
        "sh".to_string(),
        "-c".to_string(),
        "test -n review-target".to_string(),
    ];
    org.add_review_required_work_with_edges_and_gates(
        NewWork {
            owner_id: "research-worker",
            title,
            outcome: "a prepared, evidence-linked research outcome",
            goal_id: None,
            priority: 10,
            expected_artifact: "native ReviewTarget",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(2),
        },
        &[],
        &[],
        &[InitialWorkGate {
            name: REVIEW_TARGET_LIVE_PROBE_GATE,
            command: &probe_command,
        }],
    )
    .await
    .unwrap()
}

async fn record_probe(org: &OrgIntel, work_id: uuid::Uuid, attempt_id: uuid::Uuid, passed: bool) {
    let gate = org
        .list_work_gates(work_id)
        .await
        .unwrap()
        .into_iter()
        .find(|gate| gate.name == REVIEW_TARGET_LIVE_PROBE_GATE)
        .expect("declared ReviewTarget probe gate");
    org.record_gate_run(NewGateRun {
        gate_id: gate.id,
        attempt_id,
        exit_code: passed.then_some(0).or(Some(1)),
        output_digest: if passed {
            "sha256:review-target-available"
        } else {
            "sha256:review-target-unavailable"
        },
        output_excerpt: if passed {
            "HTTP target responded 200"
        } else {
            "HTTP target was not reachable"
        },
        passed,
    })
    .await
    .unwrap();
}

async fn link_review_target(org: &OrgIntel, work_id: uuid::Uuid, attempt_id: uuid::Uuid) {
    org.link_work_artifact(NewArtifactRef {
        kind: REVIEW_TARGET_ARTIFACT_KIND,
        uri: "http://127.0.0.1:4173/reports/emerging-robotics.html",
        note: "Runtime materialised the native report and the declared probe observed it.",
        created_by: "research-worker",
        work_id: Some(work_id),
        attempt_id: Some(attempt_id),
        digest: Some("sha256:review-target"),
        source_commit: Some("0123456789012345678901234567890123456789"),
        runtime_generation: Some("test-runtime"),
        label: "Prepared emerging-robotics research dossier",
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn qualified_outcome_review_is_once_only_and_unqualified_outcomes_stay_blocked() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping qualified outcome-review scenario");
        return;
    };
    let company = format!("outcomereview{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.create_actor(
        "research-direction",
        "lead",
        "Research lead",
        Some("zai/glm-5.3"),
        "exec",
        "owns a bounded evidence-led research outcome",
    )
    .await
    .unwrap();
    org.create_actor(
        "research-worker",
        "analyst",
        "Research analyst",
        Some("zai/glm-5.3"),
        "exec",
        "produces bounded evidence-led research outcomes",
    )
    .await
    .unwrap();
    let team = org
        .create_team(
            "Research",
            "Produce and supervise bounded evidence-led research outcomes",
            "research-direction",
            "exec",
        )
        .await
        .unwrap();
    org.set_actor_team(
        "research-worker",
        Some(team),
        "research-direction",
        "worker produces while the lead supervises and judges",
    )
    .await
    .unwrap();

    let missing_contract = org
        .add_review_required_work_with_edges_and_gates(
            NewWork {
                owner_id: "research-worker",
                title: "Invalid review contract",
                outcome: "must never enter the scheduler",
                goal_id: None,
                priority: 0,
                expected_artifact: "native ReviewTarget",
                workspace: WorkspaceSpec::default(),
                attempt_limit: Some(1),
            },
            &[],
            &[],
            &[],
        )
        .await
        .unwrap_err();
    assert!(missing_contract
        .to_string()
        .contains(REVIEW_TARGET_LIVE_PROBE_GATE));

    let qualified_work =
        add_review_required_work(&org, "Review the prepared robotics dossier").await;
    let qualified_attempt = org
        .claim_ready_work("test qualified owner-review outcome")
        .await
        .unwrap()
        .expect("review-required Work should be claimable");
    assert_eq!(qualified_attempt.work.id, qualified_work);
    link_review_target(&org, qualified_work, qualified_attempt.attempt_id).await;
    record_probe(&org, qualified_work, qualified_attempt.attempt_id, true).await;

    assert_eq!(
        org.finish_work_attempt(
            qualified_attempt.attempt_id,
            WorkAttemptState::Produced,
            "research dossier is ready for owner outcome review",
        )
        .await
        .unwrap(),
        WorkAttemptState::Produced
    );
    let qualified_row = org.get_work(qualified_work).await.unwrap().unwrap();
    assert!(qualified_row.owner_review_required);
    assert_eq!(qualified_row.status, WorkStatus::Blocked);
    assert!(qualified_row
        .resolution
        .contains("awaiting accountable-lead outcome review"));
    let qualified_handoffs = org
        .list_owner_handoffs()
        .await
        .unwrap()
        .into_iter()
        .filter(|handoff| handoff.work_id == qualified_work)
        .collect::<Vec<_>>();
    assert_eq!(qualified_handoffs.len(), 1);
    let handoff = &qualified_handoffs[0];
    assert_eq!(handoff.category, OwnerHandoffCategory::OwnerJudgement);
    assert_eq!(handoff.state, OwnerHandoffState::Pending);
    assert_eq!(
        handoff.assigned_to.as_deref(),
        Some("research-direction"),
        "the worker's qualified outcome reaches its non-producing lead first"
    );
    assert_eq!(handoff.attempt_id, Some(qualified_attempt.attempt_id));
    assert!(handoff.prepared_state.contains("emerging-robotics.html"));
    assert!(handoff.owner_brief.is_none());

    // A duplicate completion report after a worker restart cannot make a
    // second owner request: the Attempt state and handoff insert share one
    // transaction, and terminal replay returns the recorded terminal state.
    assert_eq!(
        org.finish_work_attempt(
            qualified_attempt.attempt_id,
            WorkAttemptState::Produced,
            "replayed terminal report after process recovery",
        )
        .await
        .unwrap(),
        WorkAttemptState::Produced
    );
    assert_eq!(
        org.list_owner_handoffs()
            .await
            .unwrap()
            .iter()
            .filter(|candidate| candidate.work_id == qualified_work)
            .count(),
        1
    );
    assert_eq!(
        org.get_work(qualified_work).await.unwrap().unwrap().status,
        WorkStatus::Blocked,
        "completion never auto-accepts the owner's judgement"
    );

    let early_owner_review = org
        .decide_owner_review(handoff.id, OwnerReviewDecision::Accepted, "accepted")
        .await
        .unwrap_err();
    assert!(early_owner_review
        .to_string()
        .contains("still assigned below"));

    org.prepare_owner_brief(
        handoff.id,
        "research-direction",
        OwnerBrief {
            kind: OwnerBriefKind::OutcomeReview,
            headline: "Accept the prepared robotics dossier".into(),
            situation: "The research worker produced the declared dossier and the accountable lead inspected its live-probed ReviewTarget.".into(),
            impact: "The owner can judge the exact native outcome without setup or implementation archaeology.".into(),
            recommendation: "Accept the prepared dossier.".into(),
            no_action: "The prepared Work remains blocked and unsent.".into(),
            uncertainty: Some("The owner retains the final taste judgement.".into()),
            deadline: None,
        },
    )
    .await
    .unwrap();
    org.escalate_handoff(
        handoff.id,
        "research-direction",
        "lead inspection passed; final owner taste judgement remains",
    )
    .await
    .unwrap();
    let at_exec = org
        .list_owner_handoffs()
        .await
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.id == handoff.id)
        .unwrap();
    assert_eq!(at_exec.assigned_to.as_deref(), Some("exec"));
    assert!(at_exec.owner_brief_is_current(qualified_row.revision));
    assert_eq!(
        at_exec.owner_brief.as_ref().map(|brief| brief.kind),
        Some(OwnerBriefKind::OutcomeReview)
    );
    org.escalate_handoff(
        handoff.id,
        "exec",
        "lead-prepared outcome is ready for the owner's irreducible judgement",
    )
    .await
    .unwrap();
    org.decide_owner_review(handoff.id, OwnerReviewDecision::Accepted, "accepted")
        .await
        .unwrap();
    assert_eq!(
        org.get_work(qualified_work).await.unwrap().unwrap().status,
        WorkStatus::Completed
    );

    let missing_target = add_review_required_work(&org, "Missing ReviewTarget stays blocked").await;
    let missing_target_attempt = org
        .claim_ready_work("test missing ReviewTarget")
        .await
        .unwrap()
        .unwrap();
    record_probe(
        &org,
        missing_target,
        missing_target_attempt.attempt_id,
        true,
    )
    .await;
    assert_eq!(
        org.finish_work_attempt(
            missing_target_attempt.attempt_id,
            WorkAttemptState::Produced,
            "there is no review target",
        )
        .await
        .unwrap(),
        WorkAttemptState::Failed
    );
    let missing_target_row = org.get_work(missing_target).await.unwrap().unwrap();
    assert_eq!(missing_target_row.status, WorkStatus::Blocked);
    assert!(missing_target_row
        .resolution
        .contains("without linking one available ReviewTarget"));
    assert!(org
        .list_owner_handoffs()
        .await
        .unwrap()
        .iter()
        .all(|handoff| handoff.work_id != missing_target));

    let failed_probe =
        add_review_required_work(&org, "Failed ReviewTarget probe stays blocked").await;
    let failed_probe_attempt = org
        .claim_ready_work("test failed ReviewTarget probe")
        .await
        .unwrap()
        .unwrap();
    link_review_target(&org, failed_probe, failed_probe_attempt.attempt_id).await;
    record_probe(&org, failed_probe, failed_probe_attempt.attempt_id, false).await;
    assert_eq!(
        org.finish_work_attempt(
            failed_probe_attempt.attempt_id,
            WorkAttemptState::Produced,
            "target was materialised but its probe failed",
        )
        .await
        .unwrap(),
        WorkAttemptState::Failed
    );
    let failed_probe_row = org.get_work(failed_probe).await.unwrap().unwrap();
    assert_eq!(failed_probe_row.status, WorkStatus::Blocked);
    assert!(failed_probe_row
        .resolution
        .contains("deterministic Work gates did not pass"));
    assert!(org
        .list_owner_handoffs()
        .await
        .unwrap()
        .iter()
        .all(|handoff| handoff.work_id != failed_probe));

    org.drop_schema().await.unwrap();
}
