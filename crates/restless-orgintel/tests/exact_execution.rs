//! Sprint 26 exact execution substrate scenarios against scratch Postgres.

use restless_orgintel::{
    NewAgentSession, NewArtifactRef, NewGateRun, NewGateRunEvidence, NewWork, NewWorkGate,
    OrgIntel, ProducingTopology, WorkAttemptState, WorkspaceSpec,
};

#[tokio::test]
async fn agent_launch_identity_is_durable_and_updates_the_running_attempt() {
    let Some(org) = company().await else { return };
    staff(&org, "builder-a").await;
    let work_id = org
        .add_work(NewWork {
            owner_id: "builder-a",
            title: "Record the certified harness",
            outcome: "one attributable launch",
            goal_id: None,
            priority: 10,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let attempt = org
        .claim_ready_work("harness identity")
        .await
        .unwrap()
        .unwrap();
    let capabilities = serde_json::json!({
        "harness": "claude-agent",
        "harness_build": "claude-agent-acp-0.73.0",
        "transport": "acp-stdio-v1",
        "exact_model_selected": true
    });
    let session = NewAgentSession {
        launch_id: "launch-claude-1",
        actor_id: "builder-a",
        responsibility: "work:certified-harness",
        work_id: Some(work_id),
        attempt_id: Some(attempt.attempt_id),
        harness: "claude-agent",
        harness_build: "claude-agent-acp-0.73.0",
        transport: "acp-stdio-v1",
        model: "anthropic/claude-sonnet-4-6",
        configured_effort: "high",
        provider_session_id: "provider-session-1",
        capabilities: &capabilities,
        resumed: false,
        reconstructed: true,
    };
    org.record_agent_session(session).await.unwrap();
    // A retry of the readiness observation is idempotent.
    org.record_agent_session(NewAgentSession {
        launch_id: "launch-claude-1",
        actor_id: "builder-a",
        responsibility: "work:certified-harness",
        work_id: Some(work_id),
        attempt_id: Some(attempt.attempt_id),
        harness: "claude-agent",
        harness_build: "claude-agent-acp-0.73.0",
        transport: "acp-stdio-v1",
        model: "anthropic/claude-sonnet-4-6",
        configured_effort: "high",
        provider_session_id: "provider-session-1",
        capabilities: &capabilities,
        resumed: false,
        reconstructed: true,
    })
    .await
    .unwrap();

    let sessions = org.list_agent_sessions(Some(work_id), 10).await.unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].harness, "claude-agent");
    assert_eq!(sessions[0].configured_effort, "high");
    assert_eq!(sessions[0].capabilities, capabilities);
    let attempts = org.list_work_attempts(Some(work_id)).await.unwrap();
    assert_eq!(attempts[0].harness.as_deref(), Some("claude-agent"));
    assert_eq!(
        attempts[0].harness_build.as_deref(),
        Some("claude-agent-acp-0.73.0")
    );
    assert_eq!(attempts[0].harness_capabilities, Some(capabilities));
    org.drop_schema().await.unwrap();
}

async fn company() -> Option<OrgIntel> {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping exact execution scenario");
        return None;
    };
    let company = format!("exactexecution{}", uuid::Uuid::new_v4().simple());
    Some(OrgIntel::ensure(&url, &company).await.unwrap())
}

async fn staff(org: &OrgIntel, id: &str) {
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    if org.active_actor("build-direction").await.unwrap().is_none() {
        org.create_actor(
            "build-direction",
            "lead",
            "Morgan Vale",
            None,
            "exec",
            "supervises exact execution",
        )
        .await
        .unwrap();
        org.create_team(
            "Build",
            "Deliver exact candidates",
            "build-direction",
            "exec",
        )
        .await
        .unwrap();
    }
    let display = format!("{} specialist", id.replace('-', " "));
    org.create_actor(id, "builder", &display, None, "build-direction", "builds")
        .await
        .unwrap();
    let team = org
        .list_teams()
        .await
        .unwrap()
        .into_iter()
        .find(|team| team.lead_actor_id == "build-direction")
        .unwrap();
    org.set_actor_team(id, Some(team.id), "build-direction", "assigned")
        .await
        .unwrap();
}

#[tokio::test]
async fn exec_records_one_unambiguous_worker_under_an_accountable_lead() {
    let Some(org) = company().await else { return };
    staff(&org, "builder-a").await;
    for message in org.inbox(Some("build-direction")).await.unwrap() {
        org.mark_read(message.id).await.unwrap();
    }
    let gate_command = vec!["true".to_string()];
    let gate = restless_orgintel::InitialWorkGate {
        name: "exact-pass",
        command: &gate_command,
        stage: "cumulative",
        timeout_seconds: 10,
        resources: &[],
    };
    let work_id = org
        .add_commissioned_work_with_edges_and_gates(
            NewWork {
                owner_id: "builder-a",
                title: "Produce the coherent candidate",
                outcome: "one gate-passing candidate",
                goal_id: None,
                priority: 10,
                expected_artifact: "",
                workspace: WorkspaceSpec::default(),
                attempt_limit: Some(1),
            },
            &[],
            &[],
            &[gate],
            false,
            None,
            "exec",
            ProducingTopology::CoherentSingleWorker,
        )
        .await
        .unwrap();
    let work = org.get_work(work_id).await.unwrap().unwrap();
    assert_eq!(work.owner_id, "builder-a");
    assert_eq!(work.commissioned_by, "exec");
    assert_eq!(
        work.producing_topology,
        ProducingTopology::CoherentSingleWorker
    );
    let commission = org
        .list_events(20)
        .await
        .unwrap()
        .into_iter()
        .find(|event| event.kind == "work_commissioned")
        .unwrap();
    assert_eq!(commission.actor_id.as_deref(), Some("exec"));
    assert_eq!(
        commission.body["accountable_lead_id"],
        serde_json::json!("build-direction")
    );

    let attempt = org.claim_ready_work("exact route").await.unwrap().unwrap();
    assert_eq!(attempt.work.id, work_id);
    let gate_id = org.list_work_gates(work_id).await.unwrap()[0].id;
    org.record_gate_run(NewGateRun {
        gate_id,
        attempt_id: attempt.attempt_id,
        exit_code: Some(0),
        output_digest: "sha256:exact-pass",
        output_excerpt: "pass",
        passed: true,
    })
    .await
    .unwrap();
    org.finish_work_attempt(
        attempt.attempt_id,
        WorkAttemptState::Produced,
        "exact candidate passed",
    )
    .await
    .unwrap();
    assert!(org
        .flush_terminal_supervisor_notices(100)
        .await
        .unwrap()
        .is_empty());
    assert!(org.inbox(Some("build-direction")).await.unwrap().is_empty());
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn coordinates_leases_gate_cache_and_feedback_are_exact() {
    let Some(org) = company().await else { return };
    staff(&org, "builder-a").await;
    staff(&org, "builder-b").await;
    let work_a = org
        .add_work(NewWork {
            owner_id: "builder-a",
            title: "Candidate A",
            outcome: "produce A",
            goal_id: None,
            priority: 10,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let work_b = org
        .add_work(NewWork {
            owner_id: "builder-b",
            title: "Candidate B",
            outcome: "produce B",
            goal_id: None,
            priority: 9,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let a = org.claim_ready_work("a").await.unwrap().unwrap();
    let b = org.claim_ready_work("b").await.unwrap().unwrap();
    let commit = "1111111111111111111111111111111111111111";
    let tree = "2222222222222222222222222222222222222222";
    org.bind_attempt_execution_coordinates(
        a.attempt_id,
        Some("main"),
        Some(commit),
        Some(tree),
        "image:one",
    )
    .await
    .unwrap();
    assert!(org
        .bind_attempt_execution_coordinates(
            a.attempt_id,
            Some("main"),
            Some("3333333333333333333333333333333333333333"),
            Some(tree),
            "image:one",
        )
        .await
        .is_err());

    let lease_a = org
        .acquire_runtime_resource(a.attempt_id, None, "port", "24632", "holder-a")
        .await
        .unwrap()
        .unwrap();
    assert!(org
        .acquire_runtime_resource(b.attempt_id, None, "port", "24632", "holder-b")
        .await
        .unwrap()
        .is_none());
    org.release_runtime_resource(lease_a.id, "holder-a", "done")
        .await
        .unwrap();
    assert!(org
        .acquire_runtime_resource(b.attempt_id, None, "port", "24632", "holder-b")
        .await
        .unwrap()
        .is_some());

    let gate = org
        .add_work_gate(NewWorkGate {
            work_id: work_a,
            name: "focused",
            cwd: "/company",
            command: &["true".into()],
            created_by: "builder-a",
        })
        .await
        .unwrap();
    org.configure_work_gate(gate, "focused", 10, &[])
        .await
        .unwrap();
    let run = org
        .record_governed_gate_run(NewGateRunEvidence {
            gate_id: gate,
            attempt_id: a.attempt_id,
            exit_code: Some(0),
            output_digest: "sha256:pass",
            output_excerpt: "ok",
            passed: true,
            candidate_tree: tree,
            definition_digest: "definition",
            toolchain_fingerprint: "image:one",
            status: "conclusive",
            duration_ms: Some(1),
            cache_source_run_id: None,
            leaked_processes: 0,
        })
        .await
        .unwrap();
    assert_eq!(
        org.find_cached_gate_run(gate, tree, "definition", "image:one")
            .await
            .unwrap()
            .unwrap()
            .id,
        run
    );
    assert!(org
        .find_cached_gate_run(gate, tree, "changed", "image:one")
        .await
        .unwrap()
        .is_none());
    assert!(org
        .find_cached_gate_run(
            gate,
            "4444444444444444444444444444444444444444",
            "definition",
            "image:one",
        )
        .await
        .unwrap()
        .is_none());
    assert!(org
        .find_cached_gate_run(gate, tree, "definition", "image:two")
        .await
        .unwrap()
        .is_none());

    let message = org
        .send_work_message("build-direction", "builder-a", work_a, "one safe delta")
        .await
        .unwrap();
    let delivered = org.checkpoint_attempt_feedback(a.attempt_id).await.unwrap();
    assert!(delivered.iter().any(|item| item.id == message));
    assert!(org
        .checkpoint_attempt_feedback(a.attempt_id)
        .await
        .unwrap()
        .is_empty());
    org.request_attempt_interrupt(work_b, "build-direction", "wrong lineage")
        .await
        .unwrap();
    assert!(org.list_work_attempts(Some(work_b)).await.unwrap()[0]
        .interrupt_requested_at
        .is_some());

    org.finish_work_attempt(a.attempt_id, WorkAttemptState::Produced, "done")
        .await
        .unwrap();
    org.finish_work_attempt(b.attempt_id, WorkAttemptState::Abandoned, "interrupted")
        .await
        .unwrap();
    assert!(org.reconcile_runtime_resources().await.unwrap() >= 1);
    assert!(org.list_live_runtime_resources().await.unwrap().is_empty());
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn nominal_completion_is_silent_and_material_failures_coalesce_by_work() {
    let Some(org) = company().await else { return };
    staff(&org, "builder-a").await;
    for message in org.inbox(Some("build-direction")).await.unwrap() {
        org.mark_read(message.id).await.unwrap();
    }
    let nominal = org
        .add_work(NewWork {
            owner_id: "builder-a",
            title: "nominal candidate",
            outcome: "finish with an exact passing gate",
            goal_id: None,
            priority: 2,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let nominal_gate = org
        .add_work_gate(NewWorkGate {
            work_id: nominal,
            name: "nominal-pass",
            cwd: "@attempt",
            command: &["true".into()],
            created_by: "build-direction",
        })
        .await
        .unwrap();
    let attempt = org.claim_ready_work("nominal").await.unwrap().unwrap();
    for index in 0..100 {
        let uri = format!("/company/run/progress/{index}");
        let digest = format!("sha256:{index:064x}");
        org.link_work_artifact(NewArtifactRef {
            kind: "progress_evidence",
            uri: &uri,
            note: "nonterminal observable progress",
            created_by: "builder-a",
            work_id: Some(attempt.work.id),
            attempt_id: Some(attempt.attempt_id),
            digest: Some(&digest),
            source_commit: None,
            runtime_generation: None,
            label: "progress",
        })
        .await
        .unwrap();
    }
    assert!(org
        .flush_terminal_supervisor_notices(100)
        .await
        .unwrap()
        .is_empty());
    assert!(org.inbox(Some("build-direction")).await.unwrap().is_empty());
    org.record_gate_run(NewGateRun {
        gate_id: nominal_gate,
        attempt_id: attempt.attempt_id,
        exit_code: Some(0),
        output_digest: "sha256:nominal-pass",
        output_excerpt: "pass",
        passed: true,
    })
    .await
    .unwrap();
    org.finish_work_attempt(
        attempt.attempt_id,
        WorkAttemptState::Produced,
        "nominal candidate passed",
    )
    .await
    .unwrap();
    assert!(
        org.flush_terminal_supervisor_notices(100)
            .await
            .unwrap()
            .is_empty(),
        "a clean passing completion is observable without a lead wake"
    );
    assert!(org.inbox(Some("build-direction")).await.unwrap().is_empty());

    let blocker = org
        .add_work(NewWork {
            owner_id: "builder-a",
            title: "blocked decision",
            outcome: "surface the blocker",
            goal_id: None,
            priority: 3,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(2),
        })
        .await
        .unwrap();
    let first_blocked = org.claim_ready_work("blocker one").await.unwrap().unwrap();
    assert_eq!(first_blocked.work.id, blocker);
    org.finish_work_attempt(
        first_blocked.attempt_id,
        WorkAttemptState::Blocked,
        "contract ambiguity in one gate episode",
    )
    .await
    .unwrap();
    org.resume_work(
        blocker,
        "build-direction",
        "the lead clarified the contract but the same gate episode must be observed once more",
    )
    .await
    .unwrap();
    let second_blocked = org.claim_ready_work("blocker two").await.unwrap().unwrap();
    org.finish_work_attempt(
        second_blocked.attempt_id,
        WorkAttemptState::Failed,
        "clarified gate still failed",
    )
    .await
    .unwrap();
    assert_eq!(
        org.flush_terminal_supervisor_notices(100)
            .await
            .unwrap()
            .len(),
        1,
        "two material terminals from one Work coalesce into one decision wake"
    );
    let inbox = org.inbox(Some("build-direction")).await.unwrap();
    assert_eq!(inbox.len(), 1);
    assert!(inbox[0]
        .body
        .starts_with("Material Runtime supervisor events for Work "));
    assert!(inbox[0].body.contains("2 exceptions"));
    assert!(
        org.flush_terminal_supervisor_notices(100)
            .await
            .unwrap()
            .is_empty(),
        "repeated delivery is exactly once"
    );
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn every_material_boundary_routes_one_durable_prompt_to_the_accountable_lead() {
    let Some(org) = company().await else { return };
    staff(&org, "builder-a").await;
    staff(&org, "builder-b").await;
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    for message in org.inbox(Some("build-direction")).await.unwrap() {
        org.mark_read(message.id).await.unwrap();
    }
    let work = org
        .add_work(NewWork {
            owner_id: "builder-a",
            title: "material-boundary fixture",
            outcome: "keep one producer while the lead settles judgement",
            goal_id: None,
            priority: 10,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(2),
        })
        .await
        .unwrap();
    let attempt = org
        .claim_ready_work("material boundary")
        .await
        .unwrap()
        .unwrap();

    org.finish_work_attempt(
        attempt.attempt_id,
        WorkAttemptState::Blocked,
        "the contract is ambiguous and needs supervisory interpretation",
    )
    .await
    .unwrap();
    assert_eq!(
        org.flush_terminal_supervisor_notices(16)
            .await
            .unwrap()
            .len(),
        1
    );
    let ambiguity = org.inbox(Some("build-direction")).await.unwrap();
    assert_eq!(ambiguity.len(), 1, "ambiguity creates one prompt wake fact");
    org.mark_read(ambiguity[0].id).await.unwrap();

    org.send_work_message(
        "builder-a",
        "build-direction",
        work,
        "Effect authority requested: sending the candidate externally crosses the approved boundary.",
    )
    .await
    .unwrap();
    let effect = org.inbox(Some("build-direction")).await.unwrap();
    assert_eq!(
        effect.len(),
        1,
        "effect authority creates one lead obligation"
    );
    org.mark_read(effect[0].id).await.unwrap();

    org.send_work_message(
        "builder-b",
        "build-direction",
        work,
        "Cross-worker conflict: my interface evidence contradicts builder-a's current assumption.",
    )
    .await
    .unwrap();
    let conflict = org.inbox(Some("build-direction")).await.unwrap();
    assert_eq!(
        conflict.len(),
        1,
        "cross-worker conflict creates one lead obligation"
    );
    org.mark_read(conflict[0].id).await.unwrap();

    org.resume_work(
        work,
        "build-direction",
        "the accountable lead resolved the ambiguity for the correction fixture",
    )
    .await
    .unwrap();
    let resumed = org
        .claim_ready_work("owner correction")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(resumed.work.id, work);

    let owner_feedback = org
        .send_work_message(
            "owner",
            "builder-a",
            work,
            "Owner correction: preserve the smaller declared outcome.",
        )
        .await
        .unwrap();
    assert!(org
        .message_is_work_attempt_input(owner_feedback)
        .await
        .unwrap());
    let correction = org.inbox(Some("build-direction")).await.unwrap();
    assert_eq!(
        correction.len(),
        1,
        "owner correction creates one control notice"
    );
    assert!(correction[0].body.contains("Material owner correction"));

    let schema = org.schema().to_string();
    drop(org);
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let restarted = OrgIntel::ensure(&database_url, &schema).await.unwrap();
    assert_eq!(
        restarted
            .inbox(Some("build-direction"))
            .await
            .unwrap()
            .len(),
        1,
        "an offline daemon cannot lose an unread material obligation"
    );
    assert!(restarted
        .flush_terminal_supervisor_notices(16)
        .await
        .unwrap()
        .is_empty());
    restarted.drop_schema().await.unwrap();
}
