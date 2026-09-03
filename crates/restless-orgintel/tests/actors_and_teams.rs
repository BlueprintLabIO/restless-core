//! Durable actor lifecycle and accountable one-level roster constraints
//! (S06-T3/T4). Runs against a scratch Postgres company when
//! `RESTLESS_TEST_DATABASE_URL` is available.

use restless_orgintel::{
    ArtifactRefState, InitialWorkGate, NewArtifactRef, NewAttemptRecovery, NewWork, NewWorkGate,
    OrgIntel, OutcomeStandard, OutcomeStandardSource, WorkAttemptState, WorkspaceSpec,
};

#[tokio::test]
async fn commissioned_outcome_standard_keeps_its_owner_source() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping outcome-standard scenario");
        return;
    };
    let company = format!("standard{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    create_actor(&org, "site-direction", "design lead").await;

    let (message_id, _) = org
        .send_owner_conversation_message_with_standard(
            "exec",
            "Create the launch page.",
            false,
            Some(OutcomeStandard::Frontier),
        )
        .await
        .unwrap();
    let team_id = org
        .create_team_with_standard(
            "Launch page",
            "Create and prove one distinctive public page.",
            "site-direction",
            "exec",
            OutcomeStandard::Frontier,
            OutcomeStandardSource::OwnerOverride,
            Some(message_id),
        )
        .await
        .unwrap();

    let team = org
        .list_teams()
        .await
        .unwrap()
        .into_iter()
        .find(|team| team.id == team_id)
        .unwrap();
    assert_eq!(team.outcome_standard, OutcomeStandard::Frontier);
    assert_eq!(
        team.outcome_standard_source,
        OutcomeStandardSource::OwnerOverride
    );
    assert_eq!(team.standard_source_message_id, Some(message_id));
}

#[tokio::test]
async fn mutable_artifact_locator_has_one_available_version() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping artifact version scenario");
        return;
    };
    let company = format!("artifact{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    create_actor(&org, "evidence-writer", "writer").await;

    let work = org
        .add_work(NewWork {
            owner_id: "evidence-writer",
            title: "Publish a changing campaign ledger",
            outcome: "one current attributable ledger version",
            goal_id: None,
            priority: 1,
            expected_artifact: "/company/outputs/campaign.jsonl",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(2),
        })
        .await
        .unwrap();
    let first = org
        .claim_ready_work("first version")
        .await
        .unwrap()
        .unwrap();
    let old = NewArtifactRef {
        kind: "output",
        uri: "/company/outputs/campaign.jsonl",
        note: "first version",
        created_by: "evidence-writer",
        work_id: Some(work),
        attempt_id: Some(first.attempt_id),
        digest: Some("sha256:old"),
        source_commit: None,
        runtime_generation: Some("test"),
        label: "campaign ledger",
    };
    let old_id = org.link_work_artifact(old).await.unwrap();
    let duplicate_id = org
        .link_work_artifact(NewArtifactRef {
            kind: "output",
            uri: "/company/outputs/campaign.jsonl",
            note: "replayed attachment",
            created_by: "evidence-writer",
            work_id: Some(work),
            attempt_id: Some(first.attempt_id),
            digest: Some("sha256:old"),
            source_commit: None,
            runtime_generation: Some("test"),
            label: "campaign ledger",
        })
        .await
        .unwrap();
    assert_eq!(
        duplicate_id, old_id,
        "same-Attempt replay must be idempotent"
    );
    org.finish_work_attempt(
        first.attempt_id,
        WorkAttemptState::Blocked,
        "output repair needed",
    )
    .await
    .unwrap();
    org.resume_work(work, "owner", "output location is writable")
        .await
        .unwrap();
    let second = org
        .claim_ready_work("publish replacement")
        .await
        .unwrap()
        .unwrap();
    let new_id = org
        .link_work_artifact(NewArtifactRef {
            kind: "output",
            uri: "/company/outputs/campaign.jsonl",
            note: "current version",
            created_by: "evidence-writer",
            work_id: Some(work),
            attempt_id: Some(second.attempt_id),
            digest: Some("sha256:new"),
            source_commit: None,
            runtime_generation: Some("test"),
            label: "campaign ledger",
        })
        .await
        .unwrap();

    let versions = org.list_artifact_refs(Some(work)).await.unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(
        versions.iter().find(|row| row.id == old_id).unwrap().state,
        ArtifactRefState::Superseded
    );
    assert_eq!(
        versions.iter().find(|row| row.id == new_id).unwrap().state,
        ArtifactRefState::Available
    );
    assert_eq!(
        versions
            .iter()
            .filter(|row| row.state == ArtifactRefState::Available)
            .count(),
        1
    );

    org.finish_work_attempt(
        second.attempt_id,
        WorkAttemptState::Abandoned,
        "test cleanup",
    )
    .await
    .unwrap();
    org.drop_schema().await.expect("drop scratch schema");
}

#[tokio::test]
async fn successor_attempt_consumes_prior_same_revision_output_without_relabelling_its_producer() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping successor artifact scenario");
        return;
    };
    let company = format!("successorartifact{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    create_actor(&org, "evidence-writer", "writer").await;

    let work = org
        .add_work(NewWork {
            owner_id: "evidence-writer",
            title: "Publish one evidence record",
            outcome: "retain the exact artifact while narrowing its claim",
            goal_id: None,
            priority: 1,
            expected_artifact: "/company/outputs/loop.md",
            workspace: WorkspaceSpec {
                repo: Some("product".into()),
                base_ref: Some("main".into()),
                integration_branch: None,
                worktree: None,
            },
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let first = org
        .claim_ready_work("produce evidence")
        .await
        .unwrap()
        .unwrap();
    let candidate = "1234567890abcdef1234567890abcdef12345678";
    let artifact = org
        .link_work_artifact(NewArtifactRef {
            kind: "output",
            uri: "/company/outputs/loop.md",
            note: "exact evidence version",
            created_by: "evidence-writer",
            work_id: Some(work),
            attempt_id: Some(first.attempt_id),
            digest: Some("sha256:exact"),
            source_commit: Some(candidate),
            runtime_generation: Some("test"),
            label: "loop evidence",
        })
        .await
        .unwrap();
    let feedback = org
        .send_work_message(
            "exec",
            "evidence-writer",
            work,
            "Keep the exact artifact; narrow only the conclusion drawn from it.",
        )
        .await
        .unwrap();
    assert_eq!(
        org.finish_work_attempt(
            first.attempt_id,
            WorkAttemptState::Produced,
            "the artifact exists but this terminal report predates direct feedback",
        )
        .await
        .unwrap(),
        WorkAttemptState::Produced
    );

    let successor = org
        .claim_ready_work("consume corrected claim")
        .await
        .unwrap()
        .expect("late feedback opens one successor revision without discarding output");
    assert!(successor
        .feedback
        .iter()
        .any(|message| message.id == feedback));
    assert!(successor.inputs.iter().any(|input| input.id == artifact));
    assert_eq!(successor.effective_base_ref.as_deref(), Some(candidate));
    let snapshot = org.work_graph_snapshot().await.unwrap();
    assert!(snapshot.attempt_inputs.iter().any(|input| {
        input.attempt_id == successor.attempt_id && input.artifact_ref_id == artifact
    }));
    assert_eq!(
        org.finish_work_attempt(
            successor.attempt_id,
            WorkAttemptState::Produced,
            "validated the inherited artifact against the new feedback without changing it",
        )
        .await
        .unwrap(),
        WorkAttemptState::Produced,
        "consuming an exact prior artifact is valid provenance; the successor need not pretend to produce it"
    );
    let artifacts = org.list_artifact_refs(Some(work)).await.unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].attempt_id, Some(first.attempt_id));
    assert_eq!(
        org.get_work(work).await.unwrap().unwrap().status,
        restless_orgintel::WorkStatus::Completed
    );

    org.drop_schema().await.expect("drop scratch schema");
}

#[tokio::test]
async fn repaired_blocked_work_resumes_from_its_clean_terminal_candidate() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping retained candidate scenario");
        return;
    };
    let company = format!("retainedcandidate{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    create_actor(&org, "delivery-builder", "builder").await;

    let original = "1111111111111111111111111111111111111111";
    let candidate = "2222222222222222222222222222222222222222";
    let candidate_tree = "3333333333333333333333333333333333333333";
    let work = org
        .add_work(NewWork {
            owner_id: "delivery-builder",
            title: "Retain useful work across a repaired gate blocker",
            outcome: "resume validation from the exact committed candidate",
            goal_id: None,
            priority: 1,
            expected_artifact: "candidate",
            workspace: WorkspaceSpec {
                repo: Some("product".into()),
                base_ref: Some(original.into()),
                integration_branch: None,
                worktree: Some("delivery-candidate".into()),
            },
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let first = org.claim_ready_work("produce").await.unwrap().unwrap();
    assert_eq!(first.effective_base_ref.as_deref(), Some(original));
    org.bind_attempt_terminal_coordinates(
        first.attempt_id,
        Some(candidate),
        Some(candidate_tree),
        Some("clean-status"),
        0,
    )
    .await
    .unwrap();
    org.finish_work_attempt(
        first.attempt_id,
        WorkAttemptState::Blocked,
        "candidate committed; gate mechanism needs repair",
    )
    .await
    .unwrap();
    org.resume_work(work, "exec", "the gate now has a leased Runtime port")
        .await
        .unwrap();

    let successor = org.claim_ready_work("validate").await.unwrap().unwrap();
    assert_eq!(successor.attempt_no, 2);
    assert_eq!(successor.effective_base_ref.as_deref(), Some(candidate));
    assert_eq!(
        org.list_work_attempts(Some(work))
            .await
            .unwrap()
            .into_iter()
            .find(|attempt| attempt.id == successor.attempt_id)
            .unwrap()
            .requested_source_ref
            .as_deref(),
        Some(candidate)
    );
    assert_eq!(
        org.list_work_attempts(Some(work))
            .await
            .unwrap()
            .into_iter()
            .find(|attempt| attempt.id == first.attempt_id)
            .unwrap()
            .terminal_source_commit
            .as_deref(),
        Some(candidate)
    );

    org.drop_schema().await.expect("drop scratch schema");
}

#[tokio::test]
async fn same_repository_dependency_starts_from_the_exact_upstream_commit() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping dependency lineage scenario");
        return;
    };
    let company = format!("lineage{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    create_actor(&org, "delivery-builder", "builder").await;
    create_actor(&org, "quality-reviewer", "reviewer").await;

    let producer = org
        .add_work(NewWork {
            owner_id: "delivery-builder",
            title: "Produce the candidate",
            outcome: "commit one candidate",
            goal_id: None,
            priority: 10,
            expected_artifact: "candidate",
            workspace: WorkspaceSpec {
                repo: Some("product".into()),
                base_ref: Some("main".into()),
                integration_branch: Some("main".into()),
                worktree: None,
            },
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let reviewer = org
        .add_work_with_edges(
            NewWork {
                owner_id: "quality-reviewer",
                title: "Review the candidate",
                outcome: "inspect and record the exact candidate",
                goal_id: None,
                priority: 5,
                expected_artifact: "review",
                workspace: WorkspaceSpec {
                    repo: Some("product".into()),
                    base_ref: Some("main".into()),
                    integration_branch: Some("main".into()),
                    worktree: None,
                },
                attempt_limit: Some(1),
            },
            &[producer],
            &[producer],
        )
        .await
        .unwrap();

    let producer_attempt = org.claim_ready_work("producer").await.unwrap().unwrap();
    let commit = "0123456789012345678901234567890123456789";
    org.link_work_artifact(NewArtifactRef {
        kind: "output",
        uri: "git:product@0123456:candidate.md",
        note: "candidate linked without a redundant full hash",
        created_by: "delivery-builder",
        work_id: Some(producer),
        attempt_id: Some(producer_attempt.attempt_id),
        digest: None,
        source_commit: None,
        runtime_generation: None,
        label: "candidate",
    })
    .await
    .unwrap();
    assert_eq!(
        org.bind_attempt_artifacts_to_observed_commit(producer_attempt.attempt_id, commit)
            .await
            .unwrap(),
        1
    );
    org.finish_work_attempt(
        producer_attempt.attempt_id,
        WorkAttemptState::Produced,
        "candidate produced",
    )
    .await
    .unwrap();

    let review_attempt = org.claim_ready_work("review").await.unwrap().unwrap();
    assert_eq!(review_attempt.work.id, reviewer);
    assert_eq!(review_attempt.work.base_ref.as_deref(), Some("main"));
    assert_eq!(review_attempt.effective_base_ref.as_deref(), Some(commit));
    assert!(review_attempt
        .inputs
        .iter()
        .any(|artifact| artifact.source_commit.as_deref() == Some(commit)));
    org.finish_work_attempt(
        review_attempt.attempt_id,
        WorkAttemptState::ChangesRequested,
        "one bounded source-grounding correction",
    )
    .await
    .unwrap();
    org.resume_work(
        producer,
        "exec",
        "the reviewer supplied one bounded source-grounding correction",
    )
    .await
    .unwrap();
    let revision = org
        .claim_ready_work("producer revision")
        .await
        .unwrap()
        .expect("review feedback should release producer revision 2");
    assert_eq!(revision.work.id, producer);
    assert_eq!(revision.work.revision, 2);
    assert_eq!(
        revision.effective_base_ref.as_deref(),
        Some(commit),
        "revision starts from the rejected candidate rather than stale main"
    );

    org.drop_schema().await.expect("drop scratch schema");
}

async fn create_actor(org: &OrgIntel, id: &str, role: &str) {
    let display = format!("{} colleague", id.replace('-', " "));
    org.create_actor(
        id,
        role,
        &display,
        Some("kimi-k2.5"),
        "exec",
        &format!("{role} provides a distinct contribution"),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn exec_can_dispatch_a_second_department_while_each_lead_supervises_staff_work() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Exec availability scenario");
        return;
    };
    let company = format!("dispatch{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    create_actor(&org, "product-direction", "lead").await;
    create_actor(&org, "product-builder", "builder").await;

    let product_team = org
        .create_team(
            "Product direction",
            "Advance the playable product outcome",
            "product-direction",
            "exec",
        )
        .await
        .unwrap();
    org.set_actor_team(
        "product-builder",
        Some(product_team),
        "product-direction",
        "the builder owns end-to-end production while the lead supervises",
    )
    .await
    .unwrap();
    let product = org
        .add_work(NewWork {
            owner_id: "product-builder",
            title: "Advance product",
            outcome: "one accepted playable candidate",
            goal_id: None,
            priority: 10,
            expected_artifact: "commit",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let product_attempt = org
        .claim_ready_work("product-runtime")
        .await
        .unwrap()
        .expect("first department Staff Work should be claimable");
    assert_eq!(product_attempt.work.id, product);
    assert_eq!(product_attempt.work.owner_id, "product-builder");

    // The first department is still running. Exec is not its Work owner and
    // can commission an unrelated owner request through a second lead.
    create_actor(&org, "research-direction", "lead").await;
    create_actor(&org, "research-analyst", "analyst").await;
    let research_team = org
        .create_team(
            "Research direction",
            "Produce a sourced decision memo",
            "research-direction",
            "exec",
        )
        .await
        .unwrap();
    org.set_actor_team(
        "research-analyst",
        Some(research_team),
        "research-direction",
        "the analyst owns the sourced memo while the lead supervises",
    )
    .await
    .unwrap();
    let research = org
        .add_work(NewWork {
            owner_id: "research-analyst",
            title: "Research decision",
            outcome: "one accepted sourced recommendation",
            goal_id: None,
            priority: 9,
            expected_artifact: "memo",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let research_attempt = org
        .claim_ready_work("research-runtime")
        .await
        .unwrap()
        .expect("second department Staff Work should run while the first remains claimed");
    assert_eq!(research_attempt.work.id, research);
    assert_eq!(research_attempt.work.owner_id, "research-analyst");
    assert_ne!(product_attempt.attempt_id, research_attempt.attempt_id);
    let attempts = org.list_work_attempts(None).await.unwrap();
    assert_eq!(
        attempts
            .iter()
            .filter(|attempt| attempt.state == WorkAttemptState::Running)
            .count(),
        2,
        "two departments run while Exec owns neither Attempt"
    );
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn initial_work_gates_commit_atomically_and_follow_the_attempt_workspace() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping atomic gate scenario");
        return;
    };
    let company = format!("gates{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    create_actor(&org, "release-build", "engineer").await;

    let check = vec!["pnpm".to_string(), "check".to_string()];
    let build = vec!["pnpm".to_string(), "build".to_string()];
    let work = org
        .add_work_with_edges_and_gates(
            NewWork {
                owner_id: "release-build",
                title: "Produce a gate-checked release",
                outcome: "typecheck and build both exit zero",
                goal_id: None,
                priority: 0,
                expected_artifact: "commit",
                workspace: WorkspaceSpec {
                    repo: Some("study".into()),
                    base_ref: Some("main".into()),
                    ..WorkspaceSpec::default()
                },
                attempt_limit: Some(2),
            },
            &[],
            &[],
            &[
                InitialWorkGate {
                    name: "typecheck",
                    command: &check,
                    stage: "focused",
                    timeout_seconds: 900,
                    resources: &[],
                },
                InitialWorkGate {
                    name: "build",
                    command: &build,
                    stage: "cumulative",
                    timeout_seconds: 900,
                    resources: &[],
                },
            ],
        )
        .await
        .unwrap();

    let gates = org.list_work_gates(work).await.unwrap();
    assert_eq!(gates.len(), 2);
    assert!(gates.iter().all(|gate| gate.cwd == "@attempt"));
    let ordered = gates
        .iter()
        .map(|gate| (gate.sequence_no, gate.name.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        ordered,
        vec![(0, "typecheck"), (1, "build")],
        "atomic gates retain the caller's declared pipeline order"
    );
    let mistaken = org
        .add_work_gate(NewWorkGate {
            work_id: work,
            name: "smoke",
            cwd: "/company",
            command: &check,
            created_by: "release-build",
        })
        .await
        .unwrap();
    let appended = org.list_work_gates(work).await.unwrap();
    assert_eq!(
        appended
            .iter()
            .map(|gate| (gate.sequence_no, gate.name.as_str()))
            .collect::<Vec<_>>(),
        vec![(0, "typecheck"), (1, "build"), (2, "smoke")],
        "a later gate appends after the atomic pipeline"
    );
    assert!(org
        .retire_work_gate(
            mistaken,
            "release-build",
            "the command checks the shared checkout rather than the Attempt",
        )
        .await
        .unwrap());
    assert!(!org
        .retire_work_gate(
            mistaken,
            "release-build",
            "a repeated repair must remain idempotent",
        )
        .await
        .unwrap());
    assert_eq!(
        org.list_work_gates(work).await.unwrap().len(),
        2,
        "retired gates no longer participate in later Attempts"
    );
    let graph = org.work_graph_snapshot().await.unwrap();
    let historical = graph
        .gates
        .iter()
        .find(|gate| gate.id == mistaken)
        .expect("retired gate remains visible with its historical runs");
    assert_eq!(historical.retired_by.as_deref(), Some("release-build"));
    assert!(historical
        .retired_reason
        .as_deref()
        .unwrap()
        .contains("shared checkout"));
    let replacement = org
        .add_work_gate(NewWorkGate {
            work_id: work,
            name: "smoke",
            cwd: "@attempt",
            command: &build,
            created_by: "release-build",
        })
        .await
        .expect("a corrected active gate may reuse the retired gate's semantic name");
    let active = org.list_work_gates(work).await.unwrap();
    assert_eq!(active.iter().filter(|gate| gate.name == "smoke").count(), 1);
    assert_eq!(
        active
            .iter()
            .find(|gate| gate.id == replacement)
            .map(|gate| gate.sequence_no),
        Some(3),
        "the replacement remains an append-only declaration"
    );

    let duplicate = org
        .add_work_with_edges_and_gates(
            NewWork {
                owner_id: "release-build",
                title: "Reject an ambiguous gate contract",
                outcome: "duplicate gate names do not enter the graph",
                goal_id: None,
                priority: 0,
                expected_artifact: "",
                workspace: WorkspaceSpec::default(),
                attempt_limit: None,
            },
            &[],
            &[],
            &[
                InitialWorkGate {
                    name: "verify",
                    command: &check,
                    stage: "cumulative",
                    timeout_seconds: 900,
                    resources: &[],
                },
                InitialWorkGate {
                    name: "verify",
                    command: &build,
                    stage: "cumulative",
                    timeout_seconds: 900,
                    resources: &[],
                },
            ],
        )
        .await;
    assert!(duplicate.is_err());
}

#[tokio::test]
async fn unknown_attempt_recovery_is_one_capsule_addressed_to_the_accountable_lead() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping recovery capsule scenario");
        return;
    };
    let company = format!("recovery{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("daemon", "system", "system-sender", "The daemon")
        .await
        .unwrap();
    create_actor(&org, "product-direction", "lead").await;
    create_actor(&org, "world-builder", "world engineer").await;
    let team = org
        .create_team(
            "Product",
            "Ship one coherent playable outcome",
            "product-direction",
            "exec",
        )
        .await
        .unwrap();
    let commission = org
        .consume_inbox_for_actor("product-direction")
        .await
        .unwrap();
    assert_eq!(commission.len(), 1);
    assert!(commission[0].body.contains(&team.to_string()));
    org.set_actor_team(
        "world-builder",
        Some(team),
        "product-direction",
        "owns one stable world-building seam",
    )
    .await
    .unwrap();
    let work = org
        .add_work(NewWork {
            owner_id: "world-builder",
            title: "Build the crossing",
            outcome: "produce one inspectable crossing candidate",
            goal_id: None,
            priority: 10,
            expected_artifact: "commit",
            workspace: WorkspaceSpec {
                repo: Some("cosmon".into()),
                base_ref: Some("dev".into()),
                integration_branch: Some("dev".into()),
                worktree: Some("crossing-recovery".into()),
            },
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let attempt = org
        .claim_ready_work("runtime")
        .await
        .unwrap()
        .expect("Work should be claimable");
    let linked_artifact = org
        .link_work_artifact(NewArtifactRef {
            kind: "path",
            uri: "/company/worktrees/crossing-recovery/scene.glb",
            note: "Staff linked this file before its cognitive process exited",
            created_by: "world-builder",
            work_id: Some(work),
            attempt_id: Some(attempt.attempt_id),
            digest: Some("scene-digest"),
            source_commit: Some("abc123"),
            runtime_generation: None,
            label: "Crossing scene candidate",
        })
        .await
        .unwrap();
    let start = serde_json::json!({
        "workdir": "/company/worktrees/crossing-recovery",
        "source_commit": "aaaaaa",
        "status_digest": "clean",
        "dirty_entries": 0,
    });
    let end = serde_json::json!({
        "workdir": "/company/worktrees/crossing-recovery",
        "source_commit": "abc123",
        "status_digest": "changed",
        "dirty_entries": 1,
    });
    let recovery = NewAttemptRecovery {
        observed_by: "daemon",
        reason: "simulated process exit after writing an artifact",
        workspace: "/company/worktrees/crossing-recovery",
        start_observation: &start,
        end_observation: &end,
        start_summary: "HEAD aaaaaa with 0 changed entries",
        end_summary: "HEAD abc123 with 1 changed entries",
        changed_since_start: true,
        observation_digest: Some("workspace-digest"),
        end_commit: Some("abc123"),
    };
    let notice = org
        .record_unknown_attempt_recovery(attempt.attempt_id, recovery)
        .await
        .unwrap()
        .expect("first reconciliation must make one capsule");
    assert_eq!(notice.work_id, work);
    assert_eq!(notice.actor_id, "world-builder");
    assert_eq!(notice.coordinator_id, "product-direction");
    assert!(notice.artifact_ref_ids.contains(&linked_artifact));

    let duplicate_start = start.clone();
    let duplicate_end = end.clone();
    assert!(org
        .record_unknown_attempt_recovery(
            attempt.attempt_id,
            NewAttemptRecovery {
                observed_by: "daemon",
                reason: "duplicate reconciliation after daemon restart",
                workspace: "/company/worktrees/crossing-recovery",
                start_observation: &duplicate_start,
                end_observation: &duplicate_end,
                start_summary: "HEAD aaaaaa with 0 changed entries",
                end_summary: "HEAD abc123 with 1 changed entries",
                changed_since_start: true,
                observation_digest: Some("workspace-digest"),
                end_commit: Some("abc123"),
            },
        )
        .await
        .unwrap()
        .is_none());

    let state = org.list_work_attempts(Some(work)).await.unwrap();
    assert_eq!(state.len(), 1);
    assert_eq!(state[0].state, WorkAttemptState::Failed);
    assert!(state[0].summary.contains("productive outcome unknown"));
    assert_eq!(
        org.get_work(work).await.unwrap().unwrap().status,
        restless_orgintel::WorkStatus::Blocked
    );
    let lead_mail = org.inbox(Some("product-direction")).await.unwrap();
    assert_eq!(lead_mail.len(), 1, "only the accountable lead wakes");
    assert_eq!(lead_mail[0].id, notice.message_id);
    assert!(lead_mail[0].body.contains("productive outcome is UNKNOWN"));
    assert!(lead_mail[0].body.contains("Crossing scene candidate"));
    assert!(org.inbox(Some("exec")).await.unwrap().is_empty());
    assert_eq!(
        org.events_of_kind("attempt_process_ended")
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        org.events_of_kind("attempt_recovery_capsule")
            .await
            .unwrap()
            .len(),
        1
    );
    let attempt_artifacts = org
        .list_artifact_refs(Some(work))
        .await
        .unwrap()
        .into_iter()
        .filter(|artifact| artifact.attempt_id == Some(attempt.attempt_id))
        .collect::<Vec<_>>();
    assert_eq!(
        attempt_artifacts.len(),
        2,
        "one Staff-linked artifact plus one runtime observation, never duplicates"
    );
    assert!(attempt_artifacts.iter().any(|artifact| {
        artifact.kind == "git_worktree_observation" && artifact.created_by == "daemon"
    }));

    org.resume_work(
        work,
        "product-direction",
        "inspected the preserved candidate and selected a bounded repair",
    )
    .await
    .unwrap();
    assert_eq!(
        org.get_work(work).await.unwrap().unwrap().attempt_limit,
        Some(2),
        "an explicit repaired resume grants exactly one attributable successor Attempt"
    );
    let repaired = org
        .claim_ready_work("lead-approved repair")
        .await
        .unwrap()
        .expect("only an explicit lead repair can create another Attempt");
    assert_eq!(repaired.work.id, work);
    assert!(repaired
        .feedback
        .iter()
        .any(|message| message.id == notice.message_id));

    // A plain process loss is still unknown, but it must not invent a
    // worktree-change artifact when both observed Git snapshots agree.
    let unchanged = serde_json::json!({
        "workdir": "/company/worktrees/crossing-recovery",
        "source_commit": "abc123",
        "status_digest": "clean",
        "dirty_entries": 0,
    });
    let no_change_notice = org
        .record_unknown_attempt_recovery(
            repaired.attempt_id,
            NewAttemptRecovery {
                observed_by: "daemon",
                reason: "simulated process exit with no workspace change",
                workspace: "/company/worktrees/crossing-recovery",
                start_observation: &unchanged,
                end_observation: &unchanged,
                start_summary: "HEAD abc123 with 0 changed entries",
                end_summary: "HEAD abc123 with 0 changed entries",
                changed_since_start: false,
                observation_digest: Some("unchanged-digest"),
                end_commit: Some("abc123"),
            },
        )
        .await
        .unwrap()
        .expect("a later Attempt gets its own one recovery capsule");
    assert_eq!(no_change_notice.work_id, work);
    assert!(
        org.list_artifact_refs(Some(work))
            .await
            .unwrap()
            .iter()
            .all(|artifact| artifact.attempt_id != Some(repaired.attempt_id)),
        "no changed Git fact means no synthetic recovery artifact"
    );

    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn material_member_message_wakes_the_lead_and_late_direct_feedback_gets_a_successor() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping direct lead message scenario");
        return;
    };
    let company = format!("message{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    create_actor(&org, "product-direction", "game lead").await;
    create_actor(&org, "world-builder", "world engineer").await;
    let team = org
        .create_team(
            "Game product",
            "Deliver one coherent playable scene",
            "product-direction",
            "exec",
        )
        .await
        .unwrap();
    let commission = org
        .consume_inbox_for_actor("product-direction")
        .await
        .unwrap();
    assert_eq!(commission.len(), 1);
    assert!(commission[0].body.contains(&team.to_string()));
    org.set_actor_team(
        "world-builder",
        Some(team),
        "product-direction",
        "owns one stable world-building seam",
    )
    .await
    .unwrap();
    let work = org
        .add_work(NewWork {
            owner_id: "world-builder",
            title: "Build the landing route",
            outcome: "make the landing route runnable in the playable scene",
            goal_id: None,
            priority: 10,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(2),
        })
        .await
        .unwrap();
    let attempt = org
        .claim_ready_work("test launch")
        .await
        .unwrap()
        .expect("member Work should be running");

    let lead_message = org
        .send_work_message(
            "world-builder",
            "product-direction",
            work,
            "The new terrain collider changes the landing-route interface; choose the integration shape before I continue.",
        )
        .await
        .unwrap();
    assert!(
        !org.message_is_work_attempt_input(lead_message)
            .await
            .unwrap(),
        "a direct member-to-lead question wakes the accountable judgement, not a second producer"
    );
    let lead_mail = org.inbox(Some("product-direction")).await.unwrap();
    assert_eq!(lead_mail.len(), 1);
    assert_eq!(lead_mail[0].id, lead_message);
    assert!(org.inbox(Some("exec")).await.unwrap().is_empty());
    assert_eq!(
        org.list_work_attempts(Some(work)).await.unwrap().len(),
        1,
        "a direct lead message does not manufacture a second Attempt"
    );

    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    let owner_feedback = org
        .send_work_message(
            "owner",
            "world-builder",
            work,
            "Keep the route readable from the first encounter.",
        )
        .await
        .unwrap();
    assert!(
        org.message_is_work_attempt_input(owner_feedback)
            .await
            .unwrap(),
        "feedback to the current Work owner remains deterministic Attempt input"
    );
    let lead_mail = org.inbox(Some("product-direction")).await.unwrap();
    let correction_notice = lead_mail
        .iter()
        .find(|message| message.body.contains("Material owner correction"))
        .expect("owner correction should wake the accountable lead exactly once");
    assert!(correction_notice.body.contains(&owner_feedback.to_string()));
    assert!(
        !org.message_is_work_attempt_input(correction_notice.id)
            .await
            .unwrap(),
        "the lead control notice cannot become a second producing Attempt input"
    );
    assert_eq!(
        attempt.attempt_id,
        org.list_work_attempts(Some(work)).await.unwrap()[0].id
    );

    let direct_decision = org
        .send_work_message(
            "product-direction",
            "world-builder",
            work,
            "Use landing_zone_id and continue the route integration.",
        )
        .await
        .unwrap();
    assert!(
        org.message_is_work_attempt_input(direct_decision)
            .await
            .unwrap(),
        "the lead's direct response is Work input for the member, never an Exec relay"
    );

    assert_eq!(
        org.finish_work_attempt(
            attempt.attempt_id,
            WorkAttemptState::Produced,
            "stale process reported the original route complete",
        )
        .await
        .unwrap(),
        WorkAttemptState::Produced,
        "late ordinary feedback preserves useful output and opens one successor revision"
    );
    let attempts = org.list_work_attempts(Some(work)).await.unwrap();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].state, WorkAttemptState::Produced);
    assert!(org
        .get_work(work)
        .await
        .unwrap()
        .unwrap()
        .resolution
        .contains(&direct_decision.to_string()));
    assert_eq!(
        org.get_work(work).await.unwrap().unwrap().status,
        restless_orgintel::WorkStatus::Active,
        "the changed assignment stays ready for one sequential successor"
    );
    let first_snapshot = org.work_graph_snapshot().await.unwrap();
    assert!(
        !first_snapshot.attempt_feedback.iter().any(|feedback| {
            feedback.attempt_id == attempt.attempt_id && feedback.message_id == direct_decision
        }),
        "the original Attempt keeps its immutable input boundary"
    );

    let successor = org
        .claim_ready_work("late direct Work feedback")
        .await
        .unwrap()
        .expect("the changed direct feedback releases one successor Attempt");
    assert_eq!(successor.work.id, work);
    assert_eq!(successor.work.revision, 2);
    assert_eq!(successor.attempt_no, 1);
    assert!(successor.feedback.iter().any(|message| {
        message.id == direct_decision && message.body.contains("landing_zone_id")
    }));
    assert_ne!(successor.input_fingerprint, attempt.input_fingerprint);
    let successor_snapshot = org.work_graph_snapshot().await.unwrap();
    assert!(successor_snapshot.attempt_feedback.iter().any(|feedback| {
        feedback.attempt_id == successor.attempt_id && feedback.message_id == direct_decision
    }));

    let live_decision = org
        .send_work_message(
            "product-direction",
            "world-builder",
            work,
            "The same landing_zone_id decision is confirmed for this live Attempt.",
        )
        .await
        .unwrap();
    let consumed = org.consume_inbox_for_actor("world-builder").await.unwrap();
    assert!(consumed.iter().any(|message| message.id == live_decision));
    assert!(
        org.inbox(Some("world-builder")).await.unwrap().is_empty(),
        "a self-read consumes only the actor's own addressed mail"
    );
    let consumed_snapshot = org.work_graph_snapshot().await.unwrap();
    assert!(consumed_snapshot.attempt_feedback.iter().any(|feedback| {
        feedback.attempt_id == successor.attempt_id && feedback.message_id == live_decision
    }));
    assert_eq!(
        org.finish_work_attempt(
            successor.attempt_id,
            WorkAttemptState::Produced,
            "the live actor observed and applied the confirmed decision",
        )
        .await
        .unwrap(),
        WorkAttemptState::Produced,
        "an Attempt that actually consumed its direct feedback may finish without another retry"
    );
    assert_eq!(
        org.get_work(work).await.unwrap().unwrap().status,
        restless_orgintel::WorkStatus::Completed
    );

    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn durable_people_and_one_level_rosters_refuse_ghosts_and_poaching() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping actor/team scenario");
        return;
    };
    let company = format!("actors{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company)
        .await
        .expect("ensure scratch company schema");

    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("world", "system", "external-sender", "The outside world")
        .await
        .unwrap();
    org.ensure_actor("daemon", "system", "system-sender", "The daemon")
        .await
        .unwrap();
    let provenance = org.list_actors().await.unwrap();
    assert!(provenance
        .iter()
        .filter(|actor| matches!(actor.id.as_str(), "world" | "daemon"))
        .all(|actor| actor.kind == "system"));
    for (id, role) in [
        ("offer-strategy", "lead"),
        ("delivery-operations", "lead"),
        ("delivery-governance", "lead"),
        ("centre-copy", "copywriter"),
        ("centre-critique", "critic"),
        ("paper-rendering", "renderer"),
        ("retiring-researcher", "researcher"),
    ] {
        create_actor(&org, id, role).await;
    }

    org.change_actor_model(
        "centre-copy",
        "anthropic/claude-sonnet-4-5",
        "exec",
        "try the stronger model on the next assignment",
    )
    .await
    .unwrap();
    assert_eq!(
        org.active_actor("centre-copy")
            .await
            .unwrap()
            .unwrap()
            .model
            .as_deref(),
        Some("anthropic/claude-sonnet-4-5")
    );
    org.ensure_actor_with_model(
        "centre-copy",
        "staff",
        "copywriter",
        "Mira",
        Some("anthropic/claude-haiku-4-5"),
    )
    .await
    .unwrap();
    assert_eq!(
        org.active_actor("centre-copy")
            .await
            .unwrap()
            .unwrap()
            .model
            .as_deref(),
        Some("anthropic/claude-sonnet-4-5"),
        "lifecycle ensure must not overwrite an explicit actor preference"
    );
    assert!(
        org.change_actor_model(
            "centre-copy",
            "anthropic/claude-haiku-4-5",
            "centre-copy",
            "silently rewrite my own model",
        )
        .await
        .is_err(),
        "ordinary Staff cannot rewrite their own model preference"
    );

    for invalid in [
        "staff-centre-critic",
        "site-validation-lead",
        "centre-critic-live",
        "copy-critic-v2",
        "release-build-retry",
    ] {
        assert!(
            org.create_actor(
                invalid,
                "critic",
                "Distinct colleague name",
                None,
                "exec",
                "attempt to encode assignment history",
            )
            .await
            .is_err(),
            "{invalid} must be rejected at creation",
        );
        assert!(org.active_actor(invalid).await.unwrap().is_none());
    }

    assert!(
        org.create_actor(
            "writer-v2",
            "copywriter",
            "Revision-shaped duplicate",
            None,
            "centre-copy",
            "a new revision"
        )
        .await
        .is_err(),
        "an ordinary member cannot mint another organisational identity"
    );
    assert!(
        org.create_actor(
            "centre-copy",
            "copywriter",
            "Writer again",
            None,
            "exec",
            "retry the same person"
        )
        .await
        .is_err(),
        "commissioning must reuse an existing stable id rather than overwrite it"
    );

    let offer = org
        .create_team(
            "Centre offer",
            "Sell complete selective practice papers to tutoring centres",
            "offer-strategy",
            "exec",
        )
        .await
        .unwrap();
    let delivery = org
        .create_team(
            "Paper delivery",
            "Generate and validate full papers and answers",
            "delivery-operations",
            "owner",
        )
        .await
        .unwrap();
    assert!(
        org.update_team(
            offer,
            Some("Centre offer renamed"),
            None,
            "offer-strategy",
            "the lead may not widen its own charter"
        )
        .await
        .is_err(),
        "only the owner or Exec may change the commissioned charter"
    );
    org.update_team(
        offer,
        Some("Tutoring-centre offer"),
        Some("Publish and validate the selective practice-paper offer for tutoring centres"),
        "exec",
        "make the outcome and review target explicit",
    )
    .await
    .unwrap();
    let updated_offer = org
        .list_teams()
        .await
        .unwrap()
        .into_iter()
        .find(|team| team.id == offer)
        .unwrap();
    assert_eq!(updated_offer.name, "Tutoring-centre offer");
    assert!(updated_offer.brief.contains("Publish and validate"));
    assert!(
        org.create_team(
            "Subteam",
            "A lead must not create a nested team",
            "delivery-governance",
            "offer-strategy"
        )
        .await
        .is_err(),
        "a lead cannot commission a second level"
    );

    org.set_actor_team(
        "centre-copy",
        Some(offer),
        "offer-strategy",
        "writes simple centre-facing copy",
    )
    .await
    .unwrap();
    org.set_actor_team(
        "centre-critique",
        Some(offer),
        "offer-strategy",
        "checks commercial clarity independently",
    )
    .await
    .unwrap();

    assert!(
        org.set_actor_team(
            "centre-copy",
            Some(delivery),
            "delivery-operations",
            "poach a specialist"
        )
        .await
        .is_err(),
        "a lead cannot poach another team's member"
    );
    assert!(
        org.set_actor_team(
            "centre-copy",
            Some(delivery),
            "offer-strategy",
            "place my member in somebody else's team"
        )
        .await
        .is_err(),
        "a lead cannot mutate another team's roster"
    );
    assert!(
        org.set_actor_team(
            "offer-strategy",
            None,
            "offer-strategy",
            "remove my own accountability"
        )
        .await
        .is_err(),
        "a lead cannot release itself"
    );

    org.set_actor_team(
        "centre-critique",
        None,
        "offer-strategy",
        "the independent review is complete",
    )
    .await
    .unwrap();
    org.set_actor_team(
        "centre-critique",
        Some(delivery),
        "delivery-operations",
        "reviews answer-key correctness",
    )
    .await
    .unwrap();

    let critique_work = org
        .add_work(NewWork {
            owner_id: "centre-critique",
            title: "Critique the answer-key contract",
            outcome: "one durable critic owns the original and revised judgement",
            goal_id: None,
            priority: 100,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(2),
        })
        .await
        .unwrap();
    let review_work = org
        .add_work_with_edges(
            NewWork {
                owner_id: "delivery-governance",
                title: "Review the answer-key critique",
                outcome: "return one exact revision request",
                goal_id: None,
                priority: 99,
                expected_artifact: "",
                workspace: WorkspaceSpec::default(),
                attempt_limit: Some(1),
            },
            &[critique_work],
            &[critique_work],
        )
        .await
        .unwrap();
    let attributed_message = org
        .send_work_message(
            "exec",
            "centre-critique",
            critique_work,
            "Keep the acceptance evidence explicit across the revision.",
        )
        .await
        .unwrap();
    assert_eq!(
        org.message_work_id(attributed_message).await.unwrap(),
        Some(critique_work)
    );

    let first_critique = org
        .claim_ready_work("first critique")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first_critique.work.id, critique_work);
    assert_eq!(first_critique.work.owner_id, "centre-critique");
    org.finish_work_attempt(
        first_critique.attempt_id,
        WorkAttemptState::Produced,
        "first critique ready for independent review",
    )
    .await
    .unwrap();
    let review = org
        .claim_ready_work("review critique")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(review.work.id, review_work);
    org.finish_work_attempt(
        review.attempt_id,
        WorkAttemptState::ChangesRequested,
        "name the answer-key evidence explicitly",
    )
    .await
    .unwrap();
    let revision = org.get_work(critique_work).await.unwrap().unwrap();
    assert_eq!(revision.owner_id, "centre-critique");
    assert_eq!(revision.revision, 2);
    org.resume_work(
        critique_work,
        "exec",
        "the exact review feedback is ready for the same critic",
    )
    .await
    .unwrap();
    let revised_critique = org
        .claim_ready_work("revised critique")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(revised_critique.work.id, critique_work);
    assert_eq!(revised_critique.work.owner_id, "centre-critique");
    assert_eq!(revised_critique.work.revision, 2);
    org.finish_work_attempt(
        revised_critique.attempt_id,
        WorkAttemptState::Produced,
        "revised critique names the acceptance evidence",
    )
    .await
    .unwrap();

    org.set_actor_team(
        "paper-rendering",
        Some(offer),
        "offer-strategy",
        "renders the sample PDFs",
    )
    .await
    .unwrap();
    let renderer_work = org
        .add_work(NewWork {
            owner_id: "paper-rendering",
            title: "Render four sample papers",
            outcome: "four complete PDFs with answer keys",
            goal_id: None,
            priority: 1,
            expected_artifact: "four PDFs",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(2),
        })
        .await
        .unwrap();
    let writer_work = org
        .add_work(NewWork {
            owner_id: "centre-copy",
            title: "Write the centre-facing explanation",
            outcome: "plain centre copy",
            goal_id: None,
            priority: 0,
            expected_artifact: "copy",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    org.add_work_edge(
        renderer_work,
        writer_work,
        restless_orgintel::WorkEdgeKind::Requires,
    )
    .await
    .unwrap();
    assert!(
        org.remove_work_edge(
            renderer_work,
            writer_work,
            restless_orgintel::WorkEdgeKind::Requires,
            "centre-critique",
            "not my graph"
        )
        .await
        .is_err(),
        "a member cannot mutate the team graph"
    );
    org.remove_work_edge(
        renderer_work,
        writer_work,
        restless_orgintel::WorkEdgeKind::Requires,
        "offer-strategy",
        "rendering and copy are independently reviewable",
    )
    .await
    .unwrap();
    assert!(
        org.set_actor_team(
            "paper-rendering",
            None,
            "offer-strategy",
            "free capacity before Work is settled"
        )
        .await
        .is_err(),
        "a lead cannot release a member whose open Work would lose attribution"
    );
    org.set_actor_team(
        "paper-rendering",
        Some(delivery),
        "exec",
        "company-level override moves delivery work to the delivery team",
    )
    .await
    .unwrap();
    let moved = org.get_work(renderer_work).await.unwrap().unwrap();
    let renderer = org.active_actor("paper-rendering").await.unwrap().unwrap();
    assert_eq!(moved.owner_id, "paper-rendering");
    assert_eq!(renderer.team_id, Some(delivery));

    assert!(
        org.set_team_lead(
            delivery,
            "centre-critique",
            "exec",
            "make a current member the lead"
        )
        .await
        .is_err(),
        "a replacement lead must be explicitly unassigned before appointment"
    );
    org.set_actor_team(
        "centre-critique",
        None,
        "exec",
        "release the critic before an explicit leadership appointment",
    )
    .await
    .unwrap();
    org.set_team_lead(
        delivery,
        "centre-critique",
        "exec",
        "make the durable critic identity accountable for delivery",
    )
    .await
    .unwrap();
    let promoted = org.active_actor("centre-critique").await.unwrap().unwrap();
    assert_eq!(promoted.id, "centre-critique");
    assert_eq!(promoted.team_id, Some(delivery));
    assert_eq!(
        org.list_teams()
            .await
            .unwrap()
            .into_iter()
            .find(|team| team.id == delivery)
            .unwrap()
            .lead_actor_id,
        "centre-critique"
    );
    org.set_team_lead(
        delivery,
        "delivery-governance",
        "owner",
        "appoint fresh accountability for paper delivery",
    )
    .await
    .unwrap();
    let former_lead = org.active_actor("centre-critique").await.unwrap().unwrap();
    assert_eq!(former_lead.id, "centre-critique");
    assert_eq!(former_lead.team_id, Some(delivery));
    org.set_actor_team(
        "centre-critique",
        Some(offer),
        "exec",
        "move the same durable critic identity after leadership changes",
    )
    .await
    .unwrap();
    let moved_former_lead = org.active_actor("centre-critique").await.unwrap().unwrap();
    assert_eq!(moved_former_lead.id, "centre-critique");
    assert_eq!(moved_former_lead.team_id, Some(offer));
    let critique_attempts = org.list_work_attempts(Some(critique_work)).await.unwrap();
    assert_eq!(critique_attempts.len(), 2);
    assert!(critique_attempts
        .iter()
        .all(|attempt| attempt.actor_id == "centre-critique"));
    assert!(org
        .inbox(Some("centre-critique"))
        .await
        .unwrap()
        .iter()
        .any(|message| message.id == attributed_message));

    let actor_events = org.events_of_kind("actor_created").await.unwrap();
    assert!(actor_events.iter().any(|event| {
        event.body["actor_id"] == "centre-copy"
            && event.body["reason"] == "copywriter provides a distinct contribution"
    }));
    assert!(actor_events.iter().any(|event| {
        event.body["actor_id"] == "centre-critique"
            && event.body["reason"] == "critic provides a distinct contribution"
    }));
    let model_events = org.events_of_kind("actor_model_changed").await.unwrap();
    assert!(model_events.iter().any(|event| {
        event.actor_id.as_deref() == Some("exec")
            && event.body["actor_id"] == "centre-copy"
            && event.body["model"] == "anthropic/claude-sonnet-4-5"
            && event.body["reason"] == "try the stronger model on the next assignment"
    }));

    org.retire_actor(
        "retiring-researcher",
        "exec",
        "the market research assignment no longer exists",
    )
    .await
    .unwrap();
    assert!(org
        .active_actor("retiring-researcher")
        .await
        .unwrap()
        .is_none());
    let retired = org
        .list_actors_including_retired()
        .await
        .unwrap()
        .into_iter()
        .find(|actor| actor.id == "retiring-researcher")
        .unwrap();
    assert_eq!(retired.retired_by.as_deref(), Some("exec"));
    assert!(retired.retirement_reason.contains("no longer exists"));
    assert!(
        org.update_actor_model("retiring-researcher", "kimi-k2.5")
            .await
            .is_err(),
        "a runtime wake must not silently resurrect a retired identity"
    );
    assert!(
        org.add_work(NewWork {
            owner_id: "retiring-researcher",
            title: "A revision-shaped assignment",
            outcome: "must be refused",
            goal_id: None,
            priority: 0,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: None,
        })
        .await
        .is_err(),
        "Work requires an existing active actor and never reopens retirement"
    );

    let roster_events = org.events_of_kind("team_roster_changed").await.unwrap();
    assert!(roster_events.iter().any(|event| {
        event.actor_id.as_deref() == Some("offer-strategy")
            && event.body["member_actor_id"] == "centre-copy"
            && event.body["reason"] == "writes simple centre-facing copy"
    }));

    org.drop_schema().await.expect("drop scratch schema");
}
