//! Durable actor lifecycle and accountable one-level roster constraints
//! (S06-T3/T4). Runs against a scratch Postgres company when
//! `RESTLESS_TEST_DATABASE_URL` is available.

use restless_orgintel::{NewWork, OrgIntel, WorkspaceSpec};

async fn create_actor(org: &OrgIntel, id: &str, role: &str) {
    org.create_actor(
        id,
        role,
        id,
        Some("kimi-k2.5"),
        "exec",
        &format!("{role} provides a distinct contribution"),
    )
    .await
    .unwrap();
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

    org.add_actor("owner", "owner", "The Owner").await.unwrap();
    org.add_actor("exec", "exec", "The Exec").await.unwrap();
    for (id, role) in [
        ("offer-lead", "lead"),
        ("delivery-lead", "lead"),
        ("replacement-lead", "lead"),
        ("writer", "copywriter"),
        ("critic", "critic"),
        ("renderer", "renderer"),
        ("retiring-researcher", "researcher"),
    ] {
        create_actor(&org, id, role).await;
    }

    assert!(
        org.create_actor(
            "writer-v2",
            "copywriter",
            "Revision-shaped duplicate",
            None,
            "writer",
            "a new revision"
        )
        .await
        .is_err(),
        "an ordinary member cannot mint another organisational identity"
    );
    assert!(
        org.create_actor(
            "writer",
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
            "offer-lead",
            "exec",
        )
        .await
        .unwrap();
    let delivery = org
        .create_team(
            "Paper delivery",
            "Generate and validate full papers and answers",
            "delivery-lead",
            "owner",
        )
        .await
        .unwrap();
    assert!(
        org.update_team(
            offer,
            Some("Centre offer renamed"),
            None,
            "offer-lead",
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
            "replacement-lead",
            "offer-lead"
        )
        .await
        .is_err(),
        "a lead cannot commission a second level"
    );

    org.set_actor_team(
        "writer",
        Some(offer),
        "offer-lead",
        "writes simple centre-facing copy",
    )
    .await
    .unwrap();
    org.set_actor_team(
        "critic",
        Some(offer),
        "offer-lead",
        "checks commercial clarity independently",
    )
    .await
    .unwrap();

    assert!(
        org.set_actor_team(
            "writer",
            Some(delivery),
            "delivery-lead",
            "poach a specialist"
        )
        .await
        .is_err(),
        "a lead cannot poach another team's member"
    );
    assert!(
        org.set_actor_team(
            "writer",
            Some(delivery),
            "offer-lead",
            "place my member in somebody else's team"
        )
        .await
        .is_err(),
        "a lead cannot mutate another team's roster"
    );
    assert!(
        org.set_actor_team(
            "offer-lead",
            None,
            "offer-lead",
            "remove my own accountability"
        )
        .await
        .is_err(),
        "a lead cannot release itself"
    );

    org.set_actor_team(
        "critic",
        None,
        "offer-lead",
        "the independent review is complete",
    )
    .await
    .unwrap();
    org.set_actor_team(
        "critic",
        Some(delivery),
        "delivery-lead",
        "reviews answer-key correctness",
    )
    .await
    .unwrap();

    org.set_actor_team(
        "renderer",
        Some(offer),
        "offer-lead",
        "renders the sample PDFs",
    )
    .await
    .unwrap();
    let renderer_work = org
        .add_work(NewWork {
            owner_id: "renderer",
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
            owner_id: "writer",
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
            "critic",
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
        "offer-lead",
        "rendering and copy are independently reviewable",
    )
    .await
    .unwrap();
    assert!(
        org.set_actor_team(
            "renderer",
            None,
            "offer-lead",
            "free capacity before Work is settled"
        )
        .await
        .is_err(),
        "a lead cannot release a member whose open Work would lose attribution"
    );
    org.set_actor_team(
        "renderer",
        Some(delivery),
        "exec",
        "company-level override moves delivery work to the delivery team",
    )
    .await
    .unwrap();
    let moved = org.get_work(renderer_work).await.unwrap().unwrap();
    let renderer = org.active_actor("renderer").await.unwrap().unwrap();
    assert_eq!(moved.owner_id, "renderer");
    assert_eq!(renderer.team_id, Some(delivery));

    assert!(
        org.set_team_lead(delivery, "critic", "exec", "make a current member the lead")
            .await
            .is_err(),
        "a replacement lead must be explicitly unassigned before appointment"
    );
    org.set_team_lead(
        delivery,
        "replacement-lead",
        "owner",
        "appoint fresh accountability for paper delivery",
    )
    .await
    .unwrap();

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
        org.add_actor_with_model(
            "retiring-researcher",
            "researcher",
            "retiring-researcher",
            Some("kimi-k2.5")
        )
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
        event.actor_id.as_deref() == Some("offer-lead")
            && event.body["member_actor_id"] == "writer"
            && event.body["reason"] == "writes simple centre-facing copy"
    }));

    org.drop_schema().await.expect("drop scratch schema");
}
