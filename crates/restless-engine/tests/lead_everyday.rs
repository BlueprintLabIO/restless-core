//! Team leads on the everyday model (owner decision, 4 Nov): a lead's routine step runs on it,
//! owner-facing work and reviews never do, the owner's switch keeps every lead turn on the main
//! model, and once an everyday attempt escalates the same Work (or the lead's conversation that
//! day) stays on the main model. Needs `RESTLESS_TEST_DATABASE_URL` for the decision log.

use restless_engine::lead_tier::{
    record_escalation, route, sticky_subject, Escalation, LeadTurn, LEAD_ESCALATION, LEAD_TIER,
};
use restless_engine::runtime::{AgentHarness, CompanyConfig};
use restless_orgintel::{OrgIntel, WorkRow, WorkStatus};

fn work(commissioned_by: &str, owner_review_required: bool) -> WorkRow {
    WorkRow {
        id: uuid::Uuid::new_v4(),
        goal_id: None,
        owner_id: "docs-lead".into(),
        title: "Hand the checked policy on".into(),
        outcome: "Pass the checked delivery policy to the Exec.".into(),
        status: WorkStatus::Active,
        resolution: String::new(),
        priority: 0,
        expected_artifact: String::new(),
        owner_review_required,
        producing_topology: Default::default(),
        commissioned_by: commissioned_by.into(),
        repo: None,
        base_ref: None,
        integration_branch: None,
        worktree: None,
        revision: 1,
        attempt_limit: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

fn step(work: &WorkRow) -> LeadTurn<'_> {
    LeadTurn::Work {
        work,
        attempt_no: 1,
        reviews: false,
        feedback: false,
        prerequisites: false,
    }
}

#[tokio::test]
async fn routine_lead_turns_use_the_everyday_model_and_escalations_stick() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("database unset; skipping lead everyday routing");
        return;
    };
    let company = format!(
        "leadday{}",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    );
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    // No decision model: the rule alone decides.
    let mut config: CompanyConfig = toml::from_str(&format!(
        "name = \"{company}\"\nmodel = \"anthropic/claude-opus-5-5\"\ndecision_model = \"off\"\n"
    ))
    .unwrap();
    let harness = AgentHarness::RestlessManaged;
    let lead = "docs-lead";

    // The lead's own next step is routine.
    let own = work(lead, false);
    let first = route(&org, &config, lead, harness, step(&own), "attempt:1").await;
    assert!(first.everyday, "{first:?}");

    // Owner-facing work, a review and a repair stay on the main model.
    let for_owner = work(lead, true);
    assert!(
        !route(&org, &config, lead, harness, step(&for_owner), "attempt:2")
            .await
            .everyday
    );
    let review = LeadTurn::Work {
        work: &own,
        attempt_no: 1,
        reviews: true,
        feedback: false,
        prerequisites: true,
    };
    assert!(
        !route(&org, &config, lead, harness, review, "attempt:3")
            .await
            .everyday
    );
    let repair = LeadTurn::Work {
        work: &own,
        attempt_no: 2,
        reviews: false,
        feedback: false,
        prerequisites: false,
    };
    assert!(
        !route(&org, &config, lead, harness, repair, "attempt:4")
            .await
            .everyday
    );
    // An owner reply in conversation stays on the main model; team mail is routine.
    let mail = ["- message 4 from maya: the draft is checked".to_string()];
    let conversation = |owner_facing| LeadTurn::Conversation {
        owner_facing,
        mention: false,
        scheduled: false,
        from_exec: false,
        mail: &mail,
    };
    assert!(
        !route(
            &org,
            &config,
            lead,
            harness,
            conversation(true),
            "conversation:1"
        )
        .await
        .everyday
    );
    assert!(
        route(
            &org,
            &config,
            lead,
            harness,
            conversation(false),
            "conversation:2"
        )
        .await
        .everyday
    );

    // The everyday attempt escalates: recorded, settled, and the same Work sticks to the main
    // model on its next turn. Other Work is unaffected.
    record_escalation(
        &org,
        lead,
        &Escalation {
            turn_id: "attempt:1".into(),
            sticky_subject: sticky_subject(lead, &step(&own)),
        },
        "anthropic/claude-sonnet-5-5",
        "anthropic/claude-opus-5-5",
        "the lead asked to think harder: a new trade-off",
    )
    .await;
    let again = route(&org, &config, lead, harness, step(&own), "attempt:5").await;
    assert!(!again.everyday);
    assert_eq!(again.reason, "an everyday attempt was escalated earlier");
    let other = work(lead, false);
    assert!(
        route(&org, &config, lead, harness, step(&other), "attempt:6")
            .await
            .everyday
    );
    // The lead's conversation sticks for the day once one of its turns escalates.
    record_escalation(
        &org,
        lead,
        &Escalation {
            turn_id: "conversation:2".into(),
            sticky_subject: sticky_subject(lead, &conversation(false)),
        },
        "anthropic/claude-sonnet-5-5",
        "anthropic/claude-opus-5-5",
        "the turn failed: invalid tool call",
    )
    .await;
    assert!(
        !route(
            &org,
            &config,
            lead,
            harness,
            conversation(false),
            "conversation:3"
        )
        .await
        .everyday
    );

    let tiers = org.recent_decisions(Some(LEAD_TIER), 50).await.unwrap();
    let of = |subject: &str| tiers.iter().find(|row| row.subject == subject).unwrap();
    assert_eq!(of("attempt:1").choice, "everyday");
    assert_eq!(of("attempt:1").outcome.as_deref(), Some("escalated"));
    assert_eq!(of("attempt:2").choice, "main");
    assert_eq!(of("attempt:2").answers["reason"], "owner-facing work");
    assert_eq!(of("attempt:5").choice, "main");
    let escalations = org
        .recent_decisions(Some(LEAD_ESCALATION), 10)
        .await
        .unwrap();
    assert_eq!(escalations.len(), 2);
    assert!(escalations
        .iter()
        .any(|row| row.answers["reason"] == "the lead asked to think harder: a new trade-off"));

    // The owner's switch: every lead turn on the main model.
    config.leads_use_everyday = Some(false);
    assert!(
        !route(&org, &config, lead, harness, step(&other), "attempt:7")
            .await
            .everyday
    );

    org.drop_schema().await.unwrap();
}
