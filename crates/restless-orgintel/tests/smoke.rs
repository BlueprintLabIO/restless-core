//! Live Postgres round-trip for the OrgIntel core (T5). OrgIntel testing is
//! behavioural: a scratch company schema is created, every table is written
//! and read back, the Work graph is exercised, and the schema
//! is dropped at the end. Runs only when RESTLESS_TEST_DATABASE_URL is set —
//! the daemon's own acceptance covers this against the real database.

use chrono::Utc;
use restless_orgintel::{
    NewArtifactRef, NewGateRun, NewOwnerHandoff, NewWork, NewWorkGate, OrgIntel, OwnerBrief,
    OwnerBriefKind, OwnerHandoffCategory, OwnerHandoffState, OwnerReviewDecision, WorkAttemptState,
    WorkEdgeKind, WorkStatus, WorkspaceSpec,
};

fn outcome_review_brief(headline: &str) -> OwnerBrief {
    OwnerBrief {
        kind: OwnerBriefKind::OutcomeReview,
        headline: headline.into(),
        situation: "The exact candidate is ready in its native review surface.".into(),
        impact: "Acceptance completes this Work; changes start a source-linked revision.".into(),
        recommendation: "Review the candidate and accept it if the evidence matches.".into(),
        no_action: "The prepared candidate remains paused without shipping.".into(),
        uncertainty: None,
        deadline: None,
    }
}

#[tokio::test]
async fn company_schema_round_trip() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping live Postgres round-trip");
        return;
    };
    let company = format!("smoke{}", std::process::id());
    let org = OrgIntel::ensure(&url, &company)
        .await
        .expect("ensure schema");
    assert_eq!(org.schema(), company);

    // Every table gets written and read back — the T5 guard is that no table
    // exists that a company never writes to, so the smoke test writes to all.
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("delivery-build", "staff", "builder", "Builder")
        .await
        .unwrap();
    org.ensure_actor("delivery-verify", "staff", "verifier", "Verifier")
        .await
        .unwrap();

    let goal = org
        .add_goal("Ship the walking skeleton", "", "exec")
        .await
        .unwrap();
    let goals = org.list_goals().await.unwrap();
    assert_eq!(goals.len(), 1);
    assert!(goals[0].closed_at.is_none());

    let work = org
        .add_work(NewWork {
            owner_id: "delivery-build",
            title: "First slice",
            outcome: "prove the loop",
            goal_id: Some(goal),
            priority: 10,
            expected_artifact: "HTML page",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(3),
        })
        .await
        .unwrap();
    let feedback = org
        .send_work_message(
            "owner",
            "delivery-build",
            work,
            "Use the short human headline and keep the exact answer key.",
        )
        .await
        .unwrap();
    let unpaired_review = org
        .add_work_with_edges(
            NewWork {
                owner_id: "delivery-verify",
                title: "Unsafe early review",
                outcome: "must not start before the producer exists",
                goal_id: Some(goal),
                priority: 6,
                expected_artifact: "",
                workspace: WorkspaceSpec::default(),
                attempt_limit: Some(1),
            },
            &[],
            &[work],
        )
        .await
        .unwrap_err();
    assert!(unpaired_review
        .to_string()
        .contains("must require that same producer"));
    let review = org
        .add_work_with_edges(
            NewWork {
                owner_id: "delivery-verify",
                title: "Independent review",
                outcome: "review the exact producer artifact",
                goal_id: Some(goal),
                priority: 5,
                expected_artifact: "",
                workspace: WorkspaceSpec::default(),
                attempt_limit: Some(1),
            },
            &[work],
            &[work],
        )
        .await
        .unwrap();
    let initial_edges = org.list_work_edges().await.unwrap();
    assert!(initial_edges.iter().any(|edge| {
        edge.from_work_id == work
            && edge.to_work_id == review
            && edge.kind == WorkEdgeKind::Requires
    }));
    assert!(initial_edges.iter().any(|edge| {
        edge.from_work_id == review && edge.to_work_id == work && edge.kind == WorkEdgeKind::Revises
    }));
    let unpaired_repair = org
        .add_work(NewWork {
            owner_id: "delivery-verify",
            title: "Unpaired graph repair",
            outcome: "prove revision power cannot outrun its prerequisite",
            goal_id: Some(goal),
            priority: -100,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    assert!(org
        .add_work_edge(unpaired_repair, work, WorkEdgeKind::Revises)
        .await
        .unwrap_err()
        .to_string()
        .contains("paired requires edge"));
    org.abandon_work(
        unpaired_repair,
        "exec",
        "the unsafe graph-repair probe was correctly refused",
    )
    .await
    .unwrap();
    let attempt = org.claim_ready_work("smoke").await.unwrap().unwrap();
    assert!(org
        .add_work_edge(review, work, WorkEdgeKind::Requires)
        .await
        .unwrap_err()
        .to_string()
        .contains("after Work"));
    // The higher-level verifier exists, but the hard graph edge keeps it from
    // starting before the producer has accepted evidence.
    assert_eq!(attempt.work.id, work);
    assert_eq!(attempt.feedback.len(), 1);
    assert_eq!(attempt.feedback[0].id, feedback);
    org.link_work_artifact(NewArtifactRef {
        kind: "path",
        uri: "/company/outputs/demo/index.html",
        note: "first output",
        created_by: "delivery-build",
        work_id: Some(work),
        attempt_id: Some(attempt.attempt_id),
        digest: Some("sha256:abc"),
        source_commit: None,
        runtime_generation: Some("test"),
        label: "landing page",
    })
    .await
    .unwrap();
    // Sprint 14's detached review checkout is supporting evidence tied to
    // the exact producer Attempt. It is deliberately another ordinary
    // artifact reference, not a second candidate or custody state.
    org.link_work_artifact(NewArtifactRef {
        kind: "review_copy",
        uri: "/company/reviews/attempt-smoke",
        note: "supporting review copy; not a replacement candidate",
        created_by: "delivery-build",
        work_id: Some(work),
        attempt_id: Some(attempt.attempt_id),
        digest: Some("sha256:review-copy"),
        source_commit: Some("0123456789012345678901234567890123456789"),
        runtime_generation: Some("test"),
        label: "Supporting review copy (not candidate)",
    })
    .await
    .unwrap();
    assert!(org
        .list_artifact_refs(Some(work))
        .await
        .unwrap()
        .iter()
        .any(|artifact| {
            artifact.kind == "review_copy"
                && artifact.attempt_id == Some(attempt.attempt_id)
                && artifact.source_commit.as_deref()
                    == Some("0123456789012345678901234567890123456789")
                && artifact.label == "Supporting review copy (not candidate)"
        }));
    let gate = org
        .add_work_gate(NewWorkGate {
            work_id: work,
            name: "html exists",
            cwd: "/company",
            command: &["test".into(), "-f".into(), "outputs/demo/index.html".into()],
            created_by: "exec",
        })
        .await
        .unwrap();
    org.record_gate_run(NewGateRun {
        gate_id: gate,
        attempt_id: attempt.attempt_id,
        exit_code: Some(0),
        output_digest: "sha256:empty",
        output_excerpt: "",
        passed: true,
    })
    .await
    .unwrap();
    org.finish_work_attempt(
        attempt.attempt_id,
        WorkAttemptState::Produced,
        "slice shipped",
    )
    .await
    .unwrap();
    let rows = org.list_work().await.unwrap();
    assert_eq!(rows[0].status, WorkStatus::Completed);
    assert_eq!(rows[0].resolution, "slice shipped");

    // Re-open the service handle between graph nodes. Readiness lives in
    // Postgres, so a supervisor/daemon restart cannot forget the handoff or
    // manufacture a duplicate producer Attempt.
    let restarted = OrgIntel::ensure(&url, &company)
        .await
        .expect("re-open OrgIntel after producer completion");
    let review_attempt = restarted
        .claim_ready_work("producer completed")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(review_attempt.work.id, review);
    restarted
        .finish_work_attempt(
            review_attempt.attempt_id,
            WorkAttemptState::ChangesRequested,
            "fix the headline",
        )
        .await
        .unwrap();
    assert_eq!(org.get_work(work).await.unwrap().unwrap().revision, 2);
    let after_feedback = org.work_graph_snapshot().await.unwrap();
    assert!(after_feedback.artifacts.iter().any(|artifact| {
        artifact.digest.as_deref() == Some("sha256:abc")
            && artifact.state == restless_orgintel::ArtifactRefState::Superseded
    }));

    // Review does not auto-launch the same mechanism. A coordinator records
    // what changed, then explicitly resumes the new revision.
    org.resume_work(
        work,
        "exec",
        "producer brief now requires the critic's exact headline correction",
    )
    .await
    .unwrap();

    let second = org
        .claim_ready_work("review requested a new producer revision")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second.work.id, work);
    assert_eq!(second.work.revision, 2);
    assert!(second.feedback.iter().any(|message| message.id == feedback));
    assert!(
        second
            .feedback
            .iter()
            .any(|message| message.from_actor == "delivery-verify"
                && message.body == "fix the headline"),
        "critic feedback must be exact input to the next producer revision"
    );
    assert_eq!(
        org.list_actors()
            .await
            .unwrap()
            .iter()
            .filter(|actor| actor.kind == "staff")
            .count(),
        2,
        "producer and critic identities survive the revision; no v2 actors"
    );
    org.link_work_artifact(NewArtifactRef {
        kind: "path",
        uri: "/company/outputs/demo/index.html",
        note: "corrected output",
        created_by: "delivery-build",
        work_id: Some(work),
        attempt_id: Some(second.attempt_id),
        digest: Some("sha256:def"),
        source_commit: Some("def456"),
        runtime_generation: Some("test"),
        label: "landing page revision 2",
    })
    .await
    .unwrap();
    org.record_gate_run(NewGateRun {
        gate_id: gate,
        attempt_id: second.attempt_id,
        exit_code: Some(0),
        output_digest: "sha256:gate-two",
        output_excerpt: "",
        passed: true,
    })
    .await
    .unwrap();
    assert_eq!(
        org.finish_work_attempt(
            second.attempt_id,
            WorkAttemptState::Produced,
            "corrected slice shipped",
        )
        .await
        .unwrap(),
        WorkAttemptState::Produced
    );

    let second_review = org
        .claim_ready_work("producer revision two completed")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second_review.work.id, review);
    assert_eq!(second_review.work.revision, 2);
    assert_eq!(second_review.inputs.len(), 1);
    assert_eq!(
        second_review.inputs[0].digest.as_deref(),
        Some("sha256:def")
    );
    org.finish_work_attempt(
        second_review.attempt_id,
        WorkAttemptState::Produced,
        "revision two independently accepted",
    )
    .await
    .unwrap();

    // A crashed process closes its Attempt and preserves the exact workspace.
    // A coordinator records the repair before the same durable Work is claimed
    // again; the scheduler never burns attempts in a blind retry loop.
    let crash_work = org
        .add_work(NewWork {
            owner_id: "delivery-build",
            title: "Recover preserved worktree",
            outcome: "continue the same workspace after a process crash",
            goal_id: Some(goal),
            priority: 50,
            expected_artifact: "",
            workspace: WorkspaceSpec {
                repo: Some("study".into()),
                base_ref: Some("dev".into()),
                integration_branch: Some("dev".into()),
                worktree: Some("recover-test".into()),
            },
            attempt_limit: Some(3),
        })
        .await
        .unwrap();
    let crashed = org
        .claim_ready_work("runtime launch")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(crashed.work.id, crash_work);
    org.finish_work_attempt(
        crashed.attempt_id,
        WorkAttemptState::Failed,
        "process crashed; worktree preserved",
    )
    .await
    .unwrap();
    org.resume_work(
        crash_work,
        "exec",
        "verified the preserved worktree and selected the same durable builder",
    )
    .await
    .unwrap();
    let recovered = org
        .claim_ready_work("supervisor recovery")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(recovered.work.id, crash_work);
    assert_eq!(recovered.attempt_no, 2);
    assert_eq!(recovered.work.repo.as_deref(), Some("study"));
    assert_eq!(recovered.work.base_ref.as_deref(), Some("dev"));
    assert_eq!(recovered.work.worktree.as_deref(), Some("recover-test"));
    org.finish_work_attempt(
        recovered.attempt_id,
        WorkAttemptState::Produced,
        "recovered from the same workspace",
    )
    .await
    .unwrap();

    let running_handoff_work = org
        .add_work(NewWork {
            owner_id: "delivery-build",
            title: "Prepare a bounded owner intervention",
            outcome: "attach the exact running Attempt before blocking on the owner",
            goal_id: Some(goal),
            priority: 40,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let running_handoff_attempt = org
        .claim_ready_work("prepare owner intervention")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(running_handoff_attempt.work.id, running_handoff_work);
    let invalid_handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: running_handoff_work,
            attempt_id: None,
            requested_by: "delivery-build",
            category: OwnerHandoffCategory::Identity,
            requested_action: "sign in",
            prepared_state: "browser at login",
            resume_condition: "session cookie observed",
        })
        .await
        .expect_err("a running Work must name the Attempt being handed over");
    assert!(invalid_handoff
        .to_string()
        .contains("must attach that Attempt"));
    let running_handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: running_handoff_work,
            attempt_id: Some(running_handoff_attempt.attempt_id),
            requested_by: "delivery-build",
            category: OwnerHandoffCategory::Identity,
            requested_action: "sign in",
            prepared_state: "browser at login",
            resume_condition: "session cookie observed",
        })
        .await
        .unwrap();
    assert_eq!(
        org.list_work_attempts(Some(running_handoff_work))
            .await
            .unwrap()
            .into_iter()
            .find(|row| row.id == running_handoff_attempt.attempt_id)
            .unwrap()
            .state,
        WorkAttemptState::Running,
        "a handoff blocks Work without lying that its supervised process stopped"
    );
    assert_eq!(
        org.get_work(running_handoff_work)
            .await
            .unwrap()
            .unwrap()
            .status,
        WorkStatus::Blocked
    );
    org.resolve_owner_handoff(
        running_handoff,
        OwnerHandoffState::Resolved,
        "sign-in observed",
    )
    .await
    .unwrap();
    assert_eq!(
        org.get_work(running_handoff_work)
            .await
            .unwrap()
            .unwrap()
            .attempt_limit,
        Some(1),
        "resolving a live Attempt's handoff must not grant a speculative retry"
    );
    let resumed_input = org.consume_inbox_for_actor("delivery-build").await.unwrap();
    let resumed_message_id = resumed_input
        .iter()
        .find(|message| message.body == "sign-in observed")
        .map(|message| message.id)
        .expect("the still-running Attempt must actually receive the owner response before it may continue");
    assert_eq!(
        org.list_work_attempts(Some(running_handoff_work))
            .await
            .unwrap()
            .into_iter()
            .find(|row| row.id == running_handoff_attempt.attempt_id)
            .unwrap()
            .state,
        WorkAttemptState::Running
    );
    org.finish_work_attempt(
        running_handoff_attempt.attempt_id,
        WorkAttemptState::Produced,
        "the same supervised Attempt continued after the owner response",
    )
    .await
    .unwrap();
    assert_eq!(
        org.get_work(running_handoff_work)
            .await
            .unwrap()
            .unwrap()
            .status,
        WorkStatus::Completed
    );

    let owner_work = org
        .add_work(NewWork {
            owner_id: "delivery-build",
            title: "Owner sign-in",
            outcome: "prepare sign-in",
            goal_id: Some(goal),
            priority: 1,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: None,
        })
        .await
        .unwrap();
    let handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: owner_work,
            attempt_id: None,
            requested_by: "delivery-build",
            category: OwnerHandoffCategory::Identity,
            requested_action: "sign in",
            prepared_state: "browser at login",
            resume_condition: "session cookie observed",
        })
        .await
        .unwrap();
    org.resolve_owner_handoff(handoff, OwnerHandoffState::Resolved, "signed in")
        .await
        .unwrap();

    let judgement_work = org
        .add_work(NewWork {
            owner_id: "delivery-build",
            title: "Choose the human headline",
            outcome: "apply the owner's copy judgement",
            goal_id: Some(goal),
            priority: 1,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: None,
        })
        .await
        .unwrap();
    let judgement_handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: judgement_work,
            attempt_id: None,
            requested_by: "delivery-build",
            category: OwnerHandoffCategory::OwnerJudgement,
            requested_action: "choose the clearer headline",
            prepared_state: "both finished headlines are visible",
            resume_condition: "owner records an explicit outcome review",
        })
        .await
        .unwrap();
    org.prepare_owner_brief(
        judgement_handoff,
        "delivery-build",
        outcome_review_brief("Choose the clearer human headline"),
    )
    .await
    .unwrap();
    org.escalate_handoff(
        judgement_handoff,
        "exec",
        "the owner must judge the final public voice",
    )
    .await
    .unwrap();
    org.send_work_message(
        "owner",
        "delivery-build",
        judgement_work,
        "Use the short headline. It sounds more human.",
    )
    .await
    .unwrap();
    let lead_reply = org
        .send_work_message_to_owner(
            "delivery-build",
            judgement_work,
            "I can make that change. Do you want the supporting line shorter too?",
        )
        .await
        .unwrap();
    let judgement_handoff = org
        .list_owner_handoffs()
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.id == judgement_handoff)
        .unwrap();
    assert_eq!(judgement_handoff.state, OwnerHandoffState::Pending);
    assert_eq!(
        org.get_work(judgement_work).await.unwrap().unwrap().status,
        WorkStatus::Blocked
    );
    org.decide_owner_review(
        judgement_handoff.id,
        OwnerReviewDecision::ChangesRequested,
        "Use the short headline. It sounds more human.",
    )
    .await
    .unwrap();
    let revised = org.get_work(judgement_work).await.unwrap().unwrap();
    assert_eq!(revised.status, WorkStatus::Blocked);
    assert_eq!(revised.revision, 2);
    assert_eq!(
        org.list_owner_handoffs()
            .await
            .unwrap()
            .into_iter()
            .find(|row| row.id == judgement_handoff.id)
            .unwrap()
            .state,
        OwnerHandoffState::Declined
    );
    let review_conversation = org
        .owner_work_conversation("delivery-build", judgement_work, 100)
        .await
        .unwrap();
    assert_eq!(review_conversation.len(), 2);
    assert_eq!(review_conversation[1].from_actor, "delivery-build");
    org.resume_work(
        judgement_work,
        "exec",
        "owner feedback is now the exact revision brief",
    )
    .await
    .unwrap();
    assert!(
        org.message_is_work_attempt_input(review_conversation[0].id)
            .await
            .unwrap(),
        "after repair, request-changes feedback must route through the new Work Attempt, not a competing free-form wake"
    );
    org.mark_read(lead_reply).await.unwrap();

    let accepted_handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: judgement_work,
            attempt_id: None,
            requested_by: "delivery-build",
            category: OwnerHandoffCategory::OwnerJudgement,
            requested_action: "accept or request another revision",
            prepared_state: "the revised outcome is open",
            resume_condition: "owner records an explicit outcome review",
        })
        .await
        .unwrap();
    org.prepare_owner_brief(
        accepted_handoff,
        "delivery-build",
        outcome_review_brief("Accept the revised human headline"),
    )
    .await
    .unwrap();
    org.escalate_handoff(
        accepted_handoff,
        "exec",
        "the exact revised public voice needs owner acceptance",
    )
    .await
    .unwrap();
    org.decide_owner_review(
        accepted_handoff,
        OwnerReviewDecision::Accepted,
        "Accepted as the outcome to ship.",
    )
    .await
    .unwrap();
    assert_eq!(
        org.get_work(judgement_work).await.unwrap().unwrap().status,
        WorkStatus::Completed
    );
    org.add_schedule(
        "delivery-build",
        Some(owner_work),
        "wait for opening",
        Utc::now(),
    )
    .await
    .unwrap();
    assert_eq!(org.claim_due_schedules().await.unwrap().len(), 1);

    let direct_schedule = org
        .add_schedule(
            "delivery-build",
            None,
            "inspect the accepted build; no new evidence may mean no work",
            Utc::now(),
        )
        .await
        .unwrap();
    assert_eq!(org.claim_due_schedules().await.unwrap().len(), 1);
    assert!(org.claim_due_schedules().await.unwrap().is_empty());
    let scheduled_mail = org.inbox(Some("delivery-build")).await.unwrap();
    let scheduled_mail = scheduled_mail
        .iter()
        .find(|message| message.body.contains(&direct_schedule.to_string()))
        .expect("a direct schedule becomes one durable addressed wake fact");
    assert_eq!(scheduled_mail.from_actor, "daemon");
    assert!(scheduled_mail
        .body
        .contains("not evidence that production is necessary"));
    org.mark_read(scheduled_mail.id).await.unwrap();
    assert!(org
        .list_schedules(Some("delivery-build"), false)
        .await
        .unwrap()
        .is_empty());
    assert!(org
        .list_schedules(Some("delivery-build"), true)
        .await
        .unwrap()
        .iter()
        .any(|schedule| schedule.id == direct_schedule));

    let message = org
        .send_message("exec", None, "status: alive")
        .await
        .unwrap();
    let inbox = org.inbox(None).await.unwrap();
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].from_actor, "exec");
    org.mark_read(message).await.unwrap();
    assert!(org.inbox(None).await.unwrap().is_empty());

    // The owner handover surface is an ordinary two-party read over messages,
    // not a thread entity or a scripted workflow. Unrelated company mail must
    // never leak into the selected actor's conversation.
    org.send_message("owner", Some("delivery-build"), "Please take another look")
        .await
        .unwrap();
    org.send_message(
        "delivery-build",
        None,
        "I need your judgement on the open page",
    )
    .await
    .unwrap();
    org.send_message("exec", Some("delivery-build"), "internal coordination")
        .await
        .unwrap();
    let conversation = org.owner_conversation("delivery-build", 100).await.unwrap();
    assert!(conversation
        .iter()
        .any(|message| message.body == "Please take another look"));
    assert!(conversation
        .iter()
        .any(|message| message.body == "I need your judgement on the open page"));
    assert!(conversation
        .iter()
        .all(|message| matches!(message.from_actor.as_str(), "owner" | "delivery-build")));
    assert!(!conversation
        .iter()
        .any(|message| message.body == "internal coordination"));

    org.add_decision("Codex over Claude Code", "T3 spike evidence", "exec")
        .await
        .unwrap();
    let event = org
        .emit_event("wake", Some("exec"), serde_json::json!({"reason": "tick"}))
        .await
        .unwrap();
    assert!(event > 0);
    let events = org.list_events(10).await.unwrap();
    assert!(events.len() >= 4, "repair events must remain observable");
    assert_eq!(events[0].kind, "wake");

    let tables = org.table_names().await.unwrap();
    for expected in [
        "actors",
        "goals",
        "work",
        "work_edges",
        "work_attempts",
        "work_attempt_inputs",
        "work_feedback",
        "work_attempt_feedback",
        "work_gates",
        "work_gate_runs",
        "owner_handoffs",
        "schedules",
        "messages",
        "artifact_refs",
        "decisions",
        "events",
    ] {
        assert!(
            tables.contains(&expected.to_string()),
            "missing table {expected}"
        );
    }

    // A second handle to the same company sees the same state (schema pin).
    let again = OrgIntel::ensure(&url, &company).await.unwrap();
    assert_eq!(again.list_work().await.unwrap().len(), 7);
    let graph = again.work_graph_snapshot().await.unwrap();
    assert_eq!(graph.attempt_feedback.len(), 4);
    assert!(graph
        .attempt_feedback
        .iter()
        .any(|link| link.message_id == feedback));
    assert!(graph
        .attempt_feedback
        .iter()
        .any(|link| link.message_id != feedback));
    assert!(graph
        .attempt_feedback
        .iter()
        .any(|link| link.message_id == resumed_message_id));

    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn goals_group_new_and_existing_work_without_becoming_a_workflow() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Goal behavioral test");
        return;
    };
    let company = format!("goals{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("delivery-build", "staff", "builder", "Builder")
        .await
        .unwrap();

    let launch = org
        .add_goal(
            "Publish the centre offer",
            "Make the complete paper offer real",
            "exec",
        )
        .await
        .unwrap();
    let validation = org
        .add_goal("Validate centre demand", "", "exec")
        .await
        .unwrap();
    let goals = org.list_goals().await.unwrap();
    assert_eq!(goals.len(), 2);
    assert_eq!(goals[0].title, "Publish the centre offer");
    assert_eq!(goals[0].created_by, "exec");

    let under_goal = org
        .add_work(NewWork {
            owner_id: "delivery-build",
            title: "Prepare the offer",
            outcome: "one inspectable centre offer",
            goal_id: Some(launch),
            priority: 20,
            expected_artifact: "offer",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    assert_eq!(
        org.get_work(under_goal).await.unwrap().unwrap().goal_id,
        Some(launch)
    );

    let previously_unassigned = org
        .add_work(NewWork {
            owner_id: "delivery-build",
            title: "Check the sample",
            outcome: "one checked sample",
            goal_id: None,
            priority: 10,
            expected_artifact: "sample check",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    assert_eq!(
        org.set_work_goal(previously_unassigned, launch, "exec")
            .await
            .unwrap(),
        None
    );
    assert_eq!(
        org.set_work_goal(previously_unassigned, validation, "exec")
            .await
            .unwrap(),
        Some(launch),
        "the same ordinary write path reassigns existing Work"
    );
    assert_eq!(
        org.get_work(previously_unassigned)
            .await
            .unwrap()
            .unwrap()
            .goal_id,
        Some(validation)
    );

    let nonexistent = uuid::Uuid::new_v4();
    assert!(org
        .set_work_goal(previously_unassigned, nonexistent, "exec")
        .await
        .unwrap_err()
        .to_string()
        .contains("does not exist in this company"));
    assert!(org
        .add_work(NewWork {
            owner_id: "delivery-build",
            title: "Misfiled Work",
            outcome: "must not be inserted",
            goal_id: Some(nonexistent),
            priority: 0,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap_err()
        .to_string()
        .contains("does not exist in this company"));
    assert_eq!(org.list_work().await.unwrap().len(), 2);

    let events = org.list_events(20).await.unwrap();
    assert!(events
        .iter()
        .any(|event| event.kind == "goal_created" && event.actor_id.as_deref() == Some("exec")));
    assert!(events.iter().any(|event| {
        event.kind == "work_goal_assigned"
            && event.actor_id.as_deref() == Some("exec")
            && event.body["work_id"] == previously_unassigned.to_string()
            && event.body["goal_id"] == validation.to_string()
            && event.body["previous_goal_id"] == launch.to_string()
    }));

    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn bad_schema_names_are_rejected() {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/restless".to_string());
    for name in [
        "",
        "1cosmon",
        "Cosmon",
        "cos;mon",
        "cosmon\"; DROP TABLE actors; --",
    ] {
        assert!(
            OrgIntel::ensure(&url, name).await.is_err(),
            "accepted {name:?}"
        );
    }
}

#[tokio::test]
async fn one_durable_actor_has_one_live_attempt() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping actor claim scenario");
        return;
    };
    let company = format!("claim{}", std::process::id());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("delivery-build", "staff", "design-engineer", "Builder")
        .await
        .unwrap();
    for title in ["first surface", "second surface"] {
        org.add_work(NewWork {
            owner_id: "delivery-build",
            title,
            outcome: "one bounded surface",
            goal_id: None,
            priority: 0,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(2),
        })
        .await
        .unwrap();
    }
    let first = org.claim_ready_work("ready").await.unwrap().unwrap();
    assert!(org.claim_ready_work("also ready").await.unwrap().is_none());
    org.finish_work_attempt(
        first.attempt_id,
        WorkAttemptState::Abandoned,
        "test releases actor",
    )
    .await
    .unwrap();
    assert!(org
        .claim_ready_work("actor released")
        .await
        .unwrap()
        .is_some());
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn a_busy_conversation_actor_does_not_consume_ready_work() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping busy actor claim scenario");
        return;
    };
    let company = format!("busyclaim{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("validation-review", "staff", "lead", "Busy validation lead")
        .await
        .unwrap();
    org.ensure_actor("sample-build", "staff", "builder", "Available builder")
        .await
        .unwrap();

    let busy_work = org
        .add_work(NewWork {
            owner_id: "validation-review",
            title: "Validate the published site",
            outcome: "the deployed outcome is checked",
            goal_id: None,
            priority: 100,
            expected_artifact: "validation evidence",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(2),
        })
        .await
        .unwrap();
    let free_work = org
        .add_work(NewWork {
            owner_id: "sample-build",
            title: "Prepare the next sample",
            outcome: "one ready sample exists",
            goal_id: None,
            priority: 10,
            expected_artifact: "sample PDF",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(2),
        })
        .await
        .unwrap();

    let claimed = org
        .claim_ready_work_excluding(
            "scheduler sees a conversation turn",
            &["validation-review".to_string()],
        )
        .await
        .unwrap()
        .expect("the other ready actor remains claimable");
    assert_eq!(claimed.work.id, free_work);
    assert!(
        org.list_work_attempts(Some(busy_work))
            .await
            .unwrap()
            .is_empty(),
        "skipping a busy actor must not create or consume an Attempt"
    );
    let untouched = org.get_work(busy_work).await.unwrap().unwrap();
    assert_eq!(untouched.status, WorkStatus::Proposed);

    org.finish_work_attempt(
        claimed.attempt_id,
        WorkAttemptState::Abandoned,
        "release the free actor for the test",
    )
    .await
    .unwrap();
    let later = org
        .claim_ready_work("the conversation ended")
        .await
        .unwrap()
        .expect("the preserved public claim path can take the formerly busy actor");
    assert_eq!(later.work.id, busy_work);

    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn evidence_findings_complete_without_review_power_and_formal_review_still_invalidates() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping review semantics scenario");
        return;
    };
    let company = format!("reviewsem{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("candidate-build", "staff", "builder", "Producer")
        .await
        .unwrap();
    org.ensure_actor(
        "evidence-research",
        "staff",
        "researcher",
        "Evidence researcher",
    )
    .await
    .unwrap();
    org.ensure_actor("acceptance-review", "staff", "critic", "Final critic")
        .await
        .unwrap();

    let producer = org
        .add_work(NewWork {
            owner_id: "candidate-build",
            title: "Build the candidate",
            outcome: "one candidate exists for review",
            goal_id: None,
            priority: 30,
            expected_artifact: "candidate",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(2),
        })
        .await
        .unwrap();
    let evidence = org
        .add_work(NewWork {
            owner_id: "evidence-research",
            title: "Research candidate evidence",
            outcome: "a report identifies issues for the final critic",
            goal_id: None,
            priority: 20,
            expected_artifact: "evidence report",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let critic = org
        .add_work_with_edges(
            NewWork {
                owner_id: "acceptance-review",
                title: "Final acceptance review",
                outcome: "judge the candidate using the evidence report",
                goal_id: None,
                priority: 10,
                expected_artifact: "",
                workspace: WorkspaceSpec::default(),
                attempt_limit: Some(1),
            },
            &[producer, evidence],
            &[producer],
        )
        .await
        .unwrap();

    let producer_attempt = org
        .claim_ready_work("candidate ready")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(producer_attempt.work.id, producer);
    org.link_work_artifact(NewArtifactRef {
        kind: "path",
        uri: "/company/outputs/candidate.html",
        note: "candidate",
        created_by: "candidate-build",
        work_id: Some(producer),
        attempt_id: Some(producer_attempt.attempt_id),
        digest: Some("sha256:candidate"),
        source_commit: None,
        runtime_generation: Some("test"),
        label: "candidate",
    })
    .await
    .unwrap();
    org.finish_work_attempt(
        producer_attempt.attempt_id,
        WorkAttemptState::Produced,
        "candidate produced",
    )
    .await
    .unwrap();

    let evidence_attempt = org
        .claim_ready_work("research ready")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(evidence_attempt.work.id, evidence);
    org.link_work_artifact(NewArtifactRef {
        kind: "path",
        uri: "/company/outputs/evidence.md",
        note: "research findings",
        created_by: "evidence-research",
        work_id: Some(evidence),
        attempt_id: Some(evidence_attempt.attempt_id),
        digest: Some("sha256:evidence"),
        source_commit: None,
        runtime_generation: Some("test"),
        label: "evidence report",
    })
    .await
    .unwrap();
    let evidence_gate = org
        .add_work_gate(NewWorkGate {
            work_id: evidence,
            name: "report rendered",
            cwd: "/company",
            command: &["test".into(), "-s".into(), "outputs/evidence.md".into()],
            created_by: "evidence-research",
        })
        .await
        .unwrap();
    org.record_gate_run(NewGateRun {
        gate_id: evidence_gate,
        attempt_id: evidence_attempt.attempt_id,
        exit_code: Some(0),
        output_digest: "sha256:gate-evidence",
        output_excerpt: "",
        passed: true,
    })
    .await
    .unwrap();
    let findings = "the candidate needs changes; final critic should judge these findings";
    assert_eq!(
        org.finish_work_attempt(
            evidence_attempt.attempt_id,
            WorkAttemptState::ChangesRequested,
            findings,
        )
        .await
        .unwrap(),
        WorkAttemptState::Produced,
        "an evidence node without a revises edge produces findings rather than invalidating"
    );
    let evidence_row = org.get_work(evidence).await.unwrap().unwrap();
    assert_eq!(evidence_row.status, WorkStatus::Completed);
    assert_eq!(evidence_row.resolution, findings);
    let evidence_attempt_row = org
        .list_work_attempts(Some(evidence))
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(evidence_attempt_row.state, WorkAttemptState::Produced);
    assert_eq!(evidence_attempt_row.summary, findings);

    let critic_attempt = org
        .claim_ready_work("inputs complete")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(critic_attempt.work.id, critic);
    assert!(critic_attempt
        .inputs
        .iter()
        .any(|input| input.digest.as_deref() == Some("sha256:evidence")));
    let criticism = "replace the unsupported claim before acceptance";
    assert_eq!(
        org.finish_work_attempt(
            critic_attempt.attempt_id,
            WorkAttemptState::ChangesRequested,
            criticism,
        )
        .await
        .unwrap(),
        WorkAttemptState::ChangesRequested,
        "a formal reviewer with a revises edge retains invalidating power"
    );
    let invalidated = org.get_work(producer).await.unwrap().unwrap();
    assert_eq!(invalidated.revision, 2);
    assert_eq!(invalidated.status, WorkStatus::Blocked);
    assert!(invalidated.resolution.contains(criticism));
    assert!(org
        .list_artifact_refs(Some(producer))
        .await
        .unwrap()
        .iter()
        .any(
            |artifact| artifact.digest.as_deref() == Some("sha256:candidate")
                && artifact.state == restless_orgintel::ArtifactRefState::Superseded
        ));

    // The semantic conversion still enters the ordinary Produced validation
    // path. A no-edge evidence node cannot use changes_requested to evade its
    // declared artifact or deterministic gate.
    let unproven = org
        .add_work(NewWork {
            owner_id: "evidence-research",
            title: "Unproven evidence",
            outcome: "must link its declared report",
            goal_id: None,
            priority: 50,
            expected_artifact: "required report",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let unproven_attempt = org
        .claim_ready_work("missing report")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(unproven_attempt.work.id, unproven);
    assert_eq!(
        org.finish_work_attempt(
            unproven_attempt.attempt_id,
            WorkAttemptState::ChangesRequested,
            "findings without the required report",
        )
        .await
        .unwrap(),
        WorkAttemptState::Failed
    );
    assert!(org
        .get_work(unproven)
        .await
        .unwrap()
        .unwrap()
        .resolution
        .contains("without linking expected artifact"));

    let ungated = org
        .add_work(NewWork {
            owner_id: "evidence-research",
            title: "Ungated evidence",
            outcome: "must pass its declared check",
            goal_id: None,
            priority: 50,
            expected_artifact: "checked report",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let ungated_attempt = org
        .claim_ready_work("unchecked report")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ungated_attempt.work.id, ungated);
    org.link_work_artifact(NewArtifactRef {
        kind: "path",
        uri: "/company/outputs/unchecked.md",
        note: "not yet checked",
        created_by: "evidence-research",
        work_id: Some(ungated),
        attempt_id: Some(ungated_attempt.attempt_id),
        digest: Some("sha256:unchecked"),
        source_commit: None,
        runtime_generation: Some("test"),
        label: "unchecked report",
    })
    .await
    .unwrap();
    org.add_work_gate(NewWorkGate {
        work_id: ungated,
        name: "report check",
        cwd: "/company",
        command: &["test".into(), "-s".into(), "outputs/unchecked.md".into()],
        created_by: "evidence-research",
    })
    .await
    .unwrap();
    assert_eq!(
        org.finish_work_attempt(
            ungated_attempt.attempt_id,
            WorkAttemptState::ChangesRequested,
            "findings before the check ran",
        )
        .await
        .unwrap(),
        WorkAttemptState::Failed
    );
    assert!(org
        .get_work(ungated)
        .await
        .unwrap()
        .unwrap()
        .resolution
        .contains("gates did not pass"));

    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn superseded_work_is_abandoned_with_attribution_but_never_while_running() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Work abandonment test");
        return;
    };
    let company = format!("abandon{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("delivery-build", "staff", "builder", "Builder")
        .await
        .unwrap();

    let proposed = org
        .add_work(NewWork {
            owner_id: "delivery-build",
            title: "Superseded brief",
            outcome: "old target that must not run",
            goal_id: None,
            priority: 20,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(2),
        })
        .await
        .unwrap();
    let stale_handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: proposed,
            attempt_id: None,
            requested_by: "delivery-build",
            category: OwnerHandoffCategory::Identity,
            requested_action: "Authorize the superseded path",
            prepared_state: "The obsolete provider page is prepared",
            resume_condition: "The obsolete connection reports ready",
        })
        .await
        .unwrap();
    org.abandon_work(proposed, "owner", "the review target moved")
        .await
        .unwrap();
    let row = org.get_work(proposed).await.unwrap().unwrap();
    assert_eq!(row.status, WorkStatus::Abandoned);
    assert!(row.resolution.contains("the review target moved"));
    let retired_handoff = org
        .list_owner_handoffs()
        .await
        .unwrap()
        .into_iter()
        .find(|handoff| handoff.id == stale_handoff)
        .unwrap();
    assert_eq!(retired_handoff.state, OwnerHandoffState::Withdrawn);
    assert!(retired_handoff
        .resolution
        .contains("Work was abandoned by owner"));
    assert!(org
        .list_events(10)
        .await
        .unwrap()
        .iter()
        .any(|event| event.kind == "work_abandoned" && event.actor_id.as_deref() == Some("owner")));

    let running = org
        .add_work(NewWork {
            owner_id: "delivery-build",
            title: "Observed process",
            outcome: "finish or stop before retirement",
            goal_id: None,
            priority: 10,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(2),
        })
        .await
        .unwrap();
    let attempt = org
        .claim_ready_work("test running guard")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(attempt.work.id, running);
    assert!(org
        .abandon_work(running, "owner", "superseded while process is live")
        .await
        .unwrap_err()
        .to_string()
        .contains("running Attempt"));
    org.finish_work_attempt(
        attempt.attempt_id,
        WorkAttemptState::Failed,
        "process stopped and observed",
    )
    .await
    .unwrap();
    org.abandon_work(running, "owner", "replacement Work now carries the outcome")
        .await
        .unwrap();
    assert_eq!(
        org.get_work(running).await.unwrap().unwrap().status,
        WorkStatus::Abandoned
    );

    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn unsettled_work_reassignment_is_attributed_and_never_steals_a_running_attempt() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Work reassignment test");
        return;
    };
    let company = format!("assign{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("delivery-build", "staff", "builder", "Builder A")
        .await
        .unwrap();
    org.ensure_actor("delivery-repair", "staff", "builder", "Builder B")
        .await
        .unwrap();
    let work = org
        .add_work(NewWork {
            owner_id: "delivery-build",
            title: "Outcome with a new accountable owner",
            outcome: "the reassigned actor receives the same durable responsibility",
            goal_id: None,
            priority: 20,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(2),
        })
        .await
        .unwrap();

    assert_eq!(
        org.reassign_work(
            work,
            "delivery-repair",
            "owner",
            "Builder B now leads this outcome"
        )
        .await
        .unwrap(),
        "delivery-build"
    );
    assert_eq!(
        org.get_work(work).await.unwrap().unwrap().owner_id,
        "delivery-repair"
    );
    assert!(org
        .reassign_work(
            work,
            "delivery-build",
            "delivery-repair",
            "take it back without a team"
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("lead may only"));

    let attempt = org
        .claim_ready_work("reassigned owner starts")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(attempt.work.owner_id, "delivery-repair");
    assert!(org
        .reassign_work(
            work,
            "delivery-build",
            "owner",
            "do not steal a live process"
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("running Attempt"));
    org.finish_work_attempt(
        attempt.attempt_id,
        WorkAttemptState::Failed,
        "stopped for repair",
    )
    .await
    .unwrap();
    org.reassign_work(
        work,
        "delivery-build",
        "owner",
        "Builder A owns the repaired mechanism",
    )
    .await
    .unwrap();
    assert_eq!(
        org.get_work(work).await.unwrap().unwrap().owner_id,
        "delivery-build"
    );
    assert!(
        org.list_events(10)
            .await
            .unwrap()
            .iter()
            .any(|event| event.kind == "work_reassigned"
                && event.actor_id.as_deref() == Some("owner"))
    );

    let handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: work,
            attempt_id: None,
            requested_by: "delivery-build",
            category: OwnerHandoffCategory::OwnerJudgement,
            requested_action: "Review the first candidate",
            prepared_state: "Candidate commit old is running",
            resume_condition: "Owner accepts or requests changes",
        })
        .await
        .unwrap();
    org.prepare_owner_brief(
        handoff,
        "delivery-build",
        outcome_review_brief("Review the first candidate"),
    )
    .await
    .unwrap();
    let prepared = org
        .list_owner_handoffs()
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.id == handoff)
        .unwrap();
    assert!(prepared.owner_brief_is_current(1));
    org.refresh_owner_handoff(
        handoff,
        "delivery-build",
        "Review the exact final candidate",
        "Candidate commit final is running and its gates passed",
        "Owner accepts this exact commit or requests exact changes",
    )
    .await
    .unwrap();
    let refreshed = org
        .list_owner_handoffs()
        .await
        .unwrap()
        .into_iter()
        .find(|row| row.id == handoff)
        .unwrap();
    assert_eq!(
        refreshed.requested_action,
        "Review the exact final candidate"
    );
    assert_eq!(
        refreshed.prepared_state,
        "Candidate commit final is running and its gates passed"
    );
    assert!(
        !refreshed.owner_brief_is_current(1),
        "material source refresh must make the older authored meaning stale"
    );
    assert!(org
        .refresh_owner_handoff(
            handoff,
            "delivery-repair",
            "Replace another actor's review",
            "Unattributed replacement",
            "Owner decides",
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("only the Work owner"));
    assert!(org.list_events(10).await.unwrap().iter().any(|event| {
        event.kind == "owner_handoff_refreshed"
            && event.actor_id.as_deref() == Some("delivery-build")
    }));

    org.drop_schema().await.unwrap();
}
