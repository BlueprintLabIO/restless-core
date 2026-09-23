//! Sprint 26 exact execution substrate scenarios against scratch Postgres.

use restless_orgintel::{
    CompanyAccessIdentity, ExternalMembershipControlContext, ExternalMembershipStatus,
    HumanAccessContext, NewAgentSession, NewArtifactRef, NewGateRun, NewGateRunEvidence, NewWork,
    NewWorkGate, OrgIntel, ProducingTopology, WorkAttemptState, WorkspaceSpec,
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
    assert_eq!(
        org.flush_terminal_supervisor_notices(100)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(org.conversation_inbox("exec").await.unwrap().len(), 1);
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
    let replayed_lease_a = org
        .acquire_runtime_resource(a.attempt_id, None, "port", "24632", "holder-a")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(replayed_lease_a.id, lease_a.id);
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
async fn completion_reaches_exec_and_material_failures_coalesce_by_work() {
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
    assert_eq!(
        org.flush_terminal_supervisor_notices(100)
            .await
            .unwrap()
            .len(),
        1,
        "a clean passing completion reaches Exec without a ceremonial lead wake"
    );
    let results = org.conversation_inbox("exec").await.unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0]
        .body
        .starts_with("Completed Work result for Exec"));
    assert!(!results[0].body.contains("/company/run/progress/"));
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

#[tokio::test]
async fn superseded_success_is_not_delivered_to_exec() {
    use sqlx::Connection as _;

    let Some(org) = company().await else { return };
    staff(&org, "builder-a").await;
    let work_id = org
        .add_work(NewWork {
            owner_id: "builder-a",
            title: "Superseded result",
            outcome: "Produce one result",
            goal_id: None,
            priority: 0,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let attempt = org
        .claim_ready_work("first revision")
        .await
        .unwrap()
        .unwrap();
    org.finish_work_attempt(attempt.attempt_id, WorkAttemptState::Produced, "old result")
        .await
        .unwrap();

    // Reproduce a revision change committed after settlement but before the
    // terminal outbox is flushed. The old successful Attempt is historical,
    // not a result that Exec should deliver to the owner.
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let mut connection = sqlx::PgConnection::connect(&database_url).await.unwrap();
    sqlx::query(&format!(
        "SET search_path TO {}, pg_catalog, pg_temp",
        org.schema()
    ))
    .execute(&mut connection)
    .await
    .unwrap();
    sqlx::query("UPDATE work SET status='active',revision=revision+1 WHERE id=$1")
        .bind(work_id)
        .execute(&mut connection)
        .await
        .unwrap();
    connection.close().await.unwrap();

    assert!(org
        .flush_terminal_supervisor_notices(100)
        .await
        .unwrap()
        .is_empty());
    assert!(org.conversation_inbox("exec").await.unwrap().is_empty());
    assert!(org
        .flush_terminal_supervisor_notices(100)
        .await
        .unwrap()
        .is_empty());
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn completed_result_survives_reconnect_and_posts_one_proactive_reply() {
    use std::time::Duration;
    let Some(org) = company().await else { return };
    staff(&org, "builder-a").await;
    org.ensure_actor("owner", "owner", "owner", "Owner")
        .await
        .unwrap();
    let work_id = org
        .add_work(NewWork {
            owner_id: "builder-a",
            title: "Requested research",
            outcome: "Return useful notes",
            goal_id: None,
            priority: 0,
            expected_artifact: "Notes",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let attempt = org
        .claim_ready_work("requested notes")
        .await
        .unwrap()
        .unwrap();
    org.link_work_artifact(NewArtifactRef {
        kind: "path",
        uri: "/company/outputs/notes.md",
        note: "Source-backed result",
        created_by: "builder-a",
        work_id: Some(work_id),
        attempt_id: Some(attempt.attempt_id),
        digest: None,
        source_commit: None,
        runtime_generation: None,
        label: "Notes",
    })
    .await
    .unwrap();
    org.finish_work_attempt(
        attempt.attempt_id,
        WorkAttemptState::Produced,
        "Notes are ready",
    )
    .await
    .unwrap();
    // Simulate losing the process after settlement but before dispatch.
    let schema = org.schema().to_string();
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    drop(org);
    let org = OrgIntel::ensure(&database_url, &schema).await.unwrap();
    let (first, concurrent) = tokio::join!(
        org.flush_terminal_supervisor_notices(100),
        org.flush_terminal_supervisor_notices(100)
    );
    assert_eq!(first.unwrap().len() + concurrent.unwrap().len(), 1);
    let inbox = org.conversation_inbox("exec").await.unwrap();
    assert_eq!(inbox.len(), 1);
    let input = inbox[0].id;
    assert_eq!(inbox[0].from_actor, "daemon");
    assert!(inbox[0].body.contains("/company/outputs/notes.md"));
    // A failed/interrupted model turn cannot consume the completion notice.
    let interrupted = org
        .claim_actor_cognitive_session("exec", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    org.release_actor_cognitive_session(&interrupted)
        .await
        .unwrap();
    assert!(org
        .finalize_cognitive_conversation(&interrupted, Some("Stale reply"), None, &[input], &[])
        .await
        .is_err());
    assert_eq!(org.owed_conversation_count("exec").await.unwrap(), 1);
    // Background inputs alone are sufficient; no owner message is sent.
    let lease = org
        .claim_actor_cognitive_session("exec", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    let reply = "Your requested notes are ready: /company/outputs/notes.md";
    let id = org
        .finalize_cognitive_conversation(&lease, Some(reply), None, &[input], &[])
        .await
        .unwrap()
        .unwrap();
    org.release_actor_cognitive_session(&lease).await.unwrap();
    // Simulate losing the commit receipt and process after finalization. The
    // same fenced operation remains replayable after reconnect and lease end.
    drop(org);
    let org = OrgIntel::ensure(&database_url, &schema).await.unwrap();
    assert_eq!(
        org.finalize_cognitive_conversation(&lease, Some(reply), None, &[input], &[])
            .await
            .unwrap(),
        Some(id)
    );
    assert_eq!(org.owed_conversation_count("exec").await.unwrap(), 0);
    assert_eq!(org.inbox(None).await.unwrap().len(), 1);
    assert!(org
        .flush_terminal_supervisor_notices(100)
        .await
        .unwrap()
        .is_empty());
    // Uneventful background observations can be acknowledged without chatter.
    let observation = org
        .send_message("daemon", Some("exec"), "Already reported; nothing changed")
        .await
        .unwrap();
    let quiet = org
        .claim_actor_cognitive_session("exec", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    org.finalize_cognitive_conversation(&quiet, None, None, &[observation], &[])
        .await
        .unwrap();
    org.release_actor_cognitive_session(&quiet).await.unwrap();
    assert_eq!(org.owed_conversation_count("exec").await.unwrap(), 0);
    assert_eq!(org.inbox(None).await.unwrap().len(), 1);
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn human_direct_rooms_are_replied_and_consumed_one_person_at_a_time() {
    use std::time::Duration;

    let Some(org) = company().await else { return };
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    for human in ["human-alice", "human-bob"] {
        org.ensure_actor(human, "human", "company-member", human)
            .await
            .unwrap();
    }
    let owner_result = org
        .send_message("daemon", Some("exec"), "owner-only completed result")
        .await
        .unwrap();
    let (alice_input, _) = org
        .send_human_conversation_message("human-alice", "exec", "Alice asks first", false)
        .await
        .unwrap();
    let (bob_input, _) = org
        .send_human_conversation_message("human-bob", "exec", "Bob asks second", false)
        .await
        .unwrap();

    let (alice_turn, sender) = org.conversation_inbox_for_turn("exec").await.unwrap();
    assert_eq!(sender.as_deref(), Some("human-alice"));
    assert_eq!(
        alice_turn
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        vec![alice_input]
    );
    let lease = org
        .claim_actor_cognitive_session("exec", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    assert!(org
        .finalize_cognitive_conversation_to(&lease, "human-alice", None, None, &[alice_input], &[],)
        .await
        .unwrap_err()
        .to_string()
        .contains("cannot be consumed by a quiet"));
    let reply = "Alice's exact answer";
    let reply_id = org
        .finalize_cognitive_conversation_to(
            &lease,
            "human-alice",
            Some(reply),
            None,
            &[alice_input],
            &[],
        )
        .await
        .unwrap()
        .unwrap();
    org.release_actor_cognitive_session(&lease).await.unwrap();

    let schema = org.schema().to_string();
    let database_url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    drop(org);
    let org = OrgIntel::ensure(&database_url, &schema).await.unwrap();
    assert_eq!(
        org.finalize_cognitive_conversation_to(
            &lease,
            "human-alice",
            Some(reply),
            None,
            &[alice_input],
            &[],
        )
        .await
        .unwrap(),
        Some(reply_id)
    );
    assert!(org
        .finalize_cognitive_conversation_to(
            &lease,
            "human-bob",
            Some(reply),
            None,
            &[alice_input],
            &[],
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("different semantics"));
    let alice = org
        .human_conversation("human-alice", "exec", 20)
        .await
        .unwrap();
    assert_eq!(
        alice.iter().filter(|message| message.body == reply).count(),
        1
    );
    assert_eq!(
        alice.last().unwrap().to_actor.as_deref(),
        Some("human-alice")
    );
    let bob = org
        .human_conversation("human-bob", "exec", 20)
        .await
        .unwrap();
    assert_eq!(bob.len(), 1);
    assert_eq!(bob[0].id, bob_input);
    assert_eq!(org.owed_conversation_count("exec").await.unwrap(), 2);
    let (bob_turn, sender) = org.conversation_inbox_for_turn("exec").await.unwrap();
    assert_eq!(sender.as_deref(), Some("human-bob"));
    assert_eq!(bob_turn.len(), 1);
    assert_eq!(bob_turn[0].id, bob_input);
    assert!(!bob_turn.iter().any(|message| message.id == owner_result));
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn human_conversation_interrupt_is_scoped_to_the_exact_sender_and_room() {
    let Some(org) = company().await else { return };
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    for human in ["human-alice", "human-bob"] {
        org.ensure_actor(human, "human", "company-member", human)
            .await
            .unwrap();
    }
    let (alice_input, _) = org
        .send_human_conversation_message("human-alice", "exec", "cancel mine", false)
        .await
        .unwrap();
    let (bob_input, _) = org
        .send_human_conversation_message("human-bob", "exec", "keep mine", false)
        .await
        .unwrap();

    assert!(!org
        .interrupt_human_conversation_message("human-bob", "exec", alice_input)
        .await
        .unwrap());
    assert!(org
        .interrupt_human_conversation_message("human-alice", "exec", alice_input)
        .await
        .unwrap());
    let (turn, sender) = org.conversation_inbox_for_turn("exec").await.unwrap();
    assert_eq!(sender.as_deref(), Some("human-bob"));
    assert_eq!(
        turn.iter().map(|message| message.id).collect::<Vec<_>>(),
        vec![bob_input]
    );
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn removed_human_input_is_not_admitted_and_other_humans_remain_owed() {
    let Some(org) = company().await else { return };
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    let company_id = uuid::Uuid::new_v4();
    let cell_id = uuid::Uuid::new_v4();
    org.ensure_company_access_identity(CompanyAccessIdentity {
        company_id,
        cell_id,
    })
    .await
    .unwrap();
    let now = chrono::Utc::now();
    let bind = |subject: &'static str, membership: &'static str| HumanAccessContext {
        display_name: Some(subject),
        issuer: "https://accounts.example.test",
        subject,
        company_id,
        cell_id,
        membership_id: membership,
        membership_role: "member",
        membership_version: 1,
        assertion_id: uuid::Uuid::new_v4(),
        issued_at: now,
        expires_at: now + chrono::Duration::minutes(10),
    };
    let alice = org
        .consume_human_access_context(bind("alice", "membership-alice"))
        .await
        .unwrap();
    let bob = org
        .consume_human_access_context(bind("bob", "membership-bob"))
        .await
        .unwrap();
    // Exec has already read the two join announcements; only human input remains.
    for announcement in org.inbox(Some("exec")).await.unwrap() {
        org.mark_read(announcement.id).await.unwrap();
    }
    let (alice_message, _) = org
        .send_human_conversation_message(&alice.actor_id, "exec", "Alice queued", false)
        .await
        .unwrap();
    let (bob_message, _) = org
        .send_human_conversation_message(&bob.actor_id, "exec", "Bob remains", false)
        .await
        .unwrap();

    org.apply_external_membership_control(ExternalMembershipControlContext {
        issuer: "https://accounts.example.test",
        subject: "alice",
        assertion_id: uuid::Uuid::new_v4(),
        issued_at: now,
        expires_at: now + chrono::Duration::minutes(10),
        key_id: "test-key",
        assertion_version: restless_orgintel::MEMBERSHIP_CONTROL_CONTRACT_VERSION,
        owner_id: uuid::Uuid::new_v4(),
        plane_id: uuid::Uuid::new_v4(),
        plane_hostname: "company.example.test",
        company_id,
        cell_id,
        membership_id: "membership-alice",
        membership_role: "member",
        membership_status: ExternalMembershipStatus::Removed,
        membership_version: 2,
    })
    .await
    .unwrap();

    let lease = org
        .claim_actor_cognitive_session("exec", std::time::Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    assert!(org
        .finalize_cognitive_conversation_to(
            &lease,
            &alice.actor_id,
            Some("late private reply"),
            None,
            &[alice_message],
            &[],
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("no longer has active company access"));
    org.release_actor_cognitive_session(&lease).await.unwrap();
    assert_eq!(org.owed_conversation_count("exec").await.unwrap(), 1);
    let (turn, sender) = org.conversation_inbox_for_turn("exec").await.unwrap();
    assert_eq!(sender.as_deref(), Some(bob.actor_id.as_str()));
    assert_eq!(
        turn.iter().map(|message| message.id).collect::<Vec<_>>(),
        vec![bob_message]
    );
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn owner_role_change_fences_the_captured_turn_before_finalization() {
    let Some(org) = company().await else { return };
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    let company_id = uuid::Uuid::new_v4();
    let cell_id = uuid::Uuid::new_v4();
    org.ensure_company_access_identity(CompanyAccessIdentity {
        company_id,
        cell_id,
    })
    .await
    .unwrap();
    let now = chrono::Utc::now();
    let access = |membership_role, membership_version| HumanAccessContext {
        display_name: Some("Alice"),
        issuer: "https://accounts.example.test",
        subject: "alice-owner",
        company_id,
        cell_id,
        membership_id: "membership-alice-owner",
        membership_role,
        membership_version,
        assertion_id: uuid::Uuid::new_v4(),
        issued_at: now,
        expires_at: now + chrono::Duration::minutes(10),
    };
    let alice = org
        .consume_human_access_context(access("owner", 1))
        .await
        .unwrap();
    // Exec has already read the join announcement; only human input remains.
    for announcement in org.inbox(Some("exec")).await.unwrap() {
        org.mark_read(announcement.id).await.unwrap();
    }
    let (message_id, _) = org
        .send_human_conversation_message(&alice.actor_id, "exec", "owner direction", false)
        .await
        .unwrap();
    let lease = org
        .claim_actor_cognitive_session("exec", std::time::Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();

    org.consume_human_access_context(access("member", 2))
        .await
        .unwrap();
    assert!(org
        .finalize_cognitive_conversation_to_with_owner_scope(
            &lease,
            &alice.actor_id,
            true,
            Some("must not commit"),
            None,
            &[message_id],
            &[],
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("membership-owner role changed"));
    assert_eq!(org.owed_conversation_count("exec").await.unwrap(), 1);
    org.release_actor_cognitive_session(&lease).await.unwrap();
    org.drop_schema().await.unwrap();
}
