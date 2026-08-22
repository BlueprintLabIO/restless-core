//! Durable actor lifecycle and accountable one-level roster constraints
//! (S06-T3/T4). Runs against a scratch Postgres company when
//! `RESTLESS_TEST_DATABASE_URL` is available.

use restless_orgintel::{
    InitialWorkGate, NewWork, NewWorkGate, OrgIntel, WorkAttemptState, WorkspaceSpec,
};

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
                },
                InitialWorkGate {
                    name: "build",
                    command: &build,
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
    org.add_work_gate(NewWorkGate {
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
                },
                InitialWorkGate {
                    name: "verify",
                    command: &build,
                },
            ],
        )
        .await;
    assert!(duplicate.is_err());
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
