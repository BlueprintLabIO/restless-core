//! Sprint 26 exact execution substrate scenarios against scratch Postgres.

use restless_orgintel::{
    NewArtifactRef, NewGateRunEvidence, NewWork, NewWorkGate, OrgIntel, WorkAttemptState,
    WorkspaceSpec,
};

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
async fn terminal_facts_for_one_lead_coalesce_into_one_wake() {
    let Some(org) = company().await else { return };
    staff(&org, "builder-a").await;
    staff(&org, "builder-b").await;
    for message in org.inbox(Some("build-direction")).await.unwrap() {
        org.mark_read(message.id).await.unwrap();
    }
    for (owner, priority) in [("builder-a", 2), ("builder-b", 1)] {
        org.add_work(NewWork {
            owner_id: owner,
            title: owner,
            outcome: "finish",
            goal_id: None,
            priority,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    }
    let a = org.claim_ready_work("a").await.unwrap().unwrap();
    let b = org.claim_ready_work("b").await.unwrap().unwrap();
    for index in 0..100 {
        let uri = format!("/company/run/progress/{index}");
        let digest = format!("sha256:{index:064x}");
        org.link_work_artifact(NewArtifactRef {
            kind: "progress_evidence",
            uri: &uri,
            note: "nonterminal observable progress",
            created_by: "builder-a",
            work_id: Some(a.work.id),
            attempt_id: Some(a.attempt_id),
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
    org.finish_work_attempt(a.attempt_id, WorkAttemptState::Produced, "a done")
        .await
        .unwrap();
    org.finish_work_attempt(b.attempt_id, WorkAttemptState::Produced, "b done")
        .await
        .unwrap();
    let messages = org.flush_terminal_supervisor_notices(100).await.unwrap();
    assert_eq!(messages.len(), 1);
    let inbox = org.inbox(Some("build-direction")).await.unwrap();
    assert_eq!(inbox.len(), 1);
    assert!(inbox[0].body.contains("2 decision checkpoints"));
    assert!(org
        .flush_terminal_supervisor_notices(100)
        .await
        .unwrap()
        .is_empty());

    let blocker = org
        .add_work(NewWork {
            owner_id: "builder-a",
            title: "blocked decision",
            outcome: "surface the blocker",
            goal_id: None,
            priority: 3,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let blocked = org.claim_ready_work("blocker").await.unwrap().unwrap();
    assert_eq!(blocked.work.id, blocker);
    org.finish_work_attempt(
        blocked.attempt_id,
        WorkAttemptState::Blocked,
        "requires an accountable choice",
    )
    .await
    .unwrap();
    assert_eq!(
        org.flush_terminal_supervisor_notices(100)
            .await
            .unwrap()
            .len(),
        1,
        "a genuine blocker becomes one prompt decision wake"
    );
    assert_eq!(org.inbox(Some("build-direction")).await.unwrap().len(), 2);
    org.drop_schema().await.unwrap();
}
