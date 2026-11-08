//! A Reno-like team lead's day, turn by turn, through the real Work graph: the Exec commissions a
//! goal, the lead plans it and waits on a designer, reads the result, resumes its Work, hands on
//! its own next step, posts a status, then gets the owner's note, a failure and the Exec's new
//! direction. Production (5-8 Nov) ran every one of these turns on the main model; routine ones
//! now run on the everyday model while owner-facing work, repairs and first planning stay on the
//! main model. The decision relay is scripted to hedge (0.62) as the production decision model
//! did (0.28-0.68), and the lead sits on its own Claude login with the company's main model, as
//! Reno's lead does. Needs `RESTLESS_TEST_DATABASE_URL`; skipped otherwise.

use axum::{routing::post, Json, Router};
use restless_engine::authority::AuthorityStore;
use restless_engine::lead_tier::{route, LeadTurn, WorkFacts, LEAD_TIER};
use restless_engine::model_gateway::{pins_own_model, with_lead_everyday_model};
use restless_engine::runtime::{AgentHarness, CompanyConfig};
use restless_orgintel::{
    ClaimedWork, NewWork, OrgIntel, ProducingTopology, WorkAttemptState, WorkspaceSpec,
};
use serde_json::{json, Value};

const LEAD: &str = "reno-supervisor";
const DESIGNER: &str = "studio-designer";

/// Hedges like the production decision model, and is sure a pricing call is not routine.
async fn decide(Json(request): Json<Value>) -> Json<Value> {
    let routine = if request["state"].to_string().contains("pricing") {
        0.3
    } else {
        0.62
    };
    Json(json!({
        "model": "relay/scripted",
        "answers": {"routine": {"type": "noul", "noul": routine}},
    }))
}

fn reno(name: &str) -> CompanyConfig {
    toml::from_str(&format!(
        r#"name = "{name}"
model = "anthropic/claude-opus-5-5"
everyday_model = "anthropic/claude-sonnet-5-5"
decision_model = "included"

[agent_intelligence.default]
connection = "account:anthropic@e69f625023c149158e3b9d79ce9f7d63"
model = "claude-opus-5-5"

[agent_intelligence.{LEAD}]
connection = "account:anthropic@dcbf955c63a14e58958cf9efa0d6f97a"
model = "claude-opus-5-5"
"#
    ))
    .expect("company config parses")
}

fn work<'a>(owner: &'a str, title: &'a str) -> NewWork<'a> {
    NewWork {
        owner_id: owner,
        title,
        outcome: title,
        goal_id: None,
        priority: 1,
        expected_artifact: "",
        workspace: WorkspaceSpec::default(),
        attempt_limit: None,
    }
}

async fn commission(org: &OrgIntel, new: NewWork<'_>, by: &str, source: Option<i64>) -> uuid::Uuid {
    org.add_commissioned_work_with_edges_and_gates(
        new,
        &[],
        &[],
        &[],
        false,
        source,
        by,
        ProducingTopology::default(),
    )
    .await
    .unwrap()
}

async fn claim(org: &OrgIntel, work_id: uuid::Uuid) -> ClaimedWork {
    let claimed = org
        .claim_ready_work("dependency graph became ready")
        .await
        .unwrap()
        .expect("ready Work");
    assert_eq!(claimed.work.id, work_id, "{}", claimed.work.title);
    claimed
}

/// The lead picks its blocked Work back up, as a resume or a new Work message does in production.
async fn resume(org: &OrgIntel, work_id: uuid::Uuid, reason: &str) {
    let work = org.get_work(work_id).await.unwrap().unwrap();
    if work.status == restless_orgintel::WorkStatus::Blocked {
        org.resume_work(work_id, LEAD, reason).await.unwrap();
    }
}

struct Day {
    org: OrgIntel,
    config: CompanyConfig,
    turns: Vec<(&'static str, bool, &'static str)>,
}

impl Day {
    async fn work_turn(&mut self, label: &'static str, claimed: &ClaimedWork) -> bool {
        let facts = WorkFacts::of(&self.org, claimed).await;
        let id = format!("attempt:{}", claimed.attempt_id);
        let routed = route(
            &self.org,
            &self.config,
            LEAD,
            AgentHarness::RestlessManaged,
            facts.turn(claimed),
            &id,
        )
        .await;
        self.turns.push((label, routed.everyday, routed.reason));
        routed.everyday
    }

    async fn conversation(&mut self, label: &'static str, from_exec: bool, mail: &str) -> bool {
        let mail = [mail.to_string()];
        let turn = LeadTurn::Conversation {
            owner_facing: false,
            mention: false,
            scheduled: false,
            from_exec,
            mail: &mail,
        };
        let id = format!("conversation:{LEAD}:{}", uuid::Uuid::new_v4());
        let routed = route(
            &self.org,
            &self.config,
            LEAD,
            AgentHarness::RestlessManaged,
            turn,
            &id,
        )
        .await;
        self.turns.push((label, routed.everyday, routed.reason));
        routed.everyday
    }
}

#[tokio::test]
async fn a_reno_like_lead_chain_runs_its_routine_turns_on_the_everyday_model() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("database unset; skipping the lead chain");
        return;
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, Router::new().route("/v1/decisions", post(decide)))
            .await
            .unwrap();
    });
    // The plane's relay, as on a hosted plane. This test binary holds no other test that reads it.
    unsafe {
        std::env::set_var("GPT_BASE_URL", &base);
        std::env::set_var("RESTLESS_HOSTED_MODEL_RELAY_TOKEN", "relay-test-token");
        std::env::remove_var("RESTLESS_DECISIONS");
    }
    let name = format!(
        "leadchain{}",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    );
    let org = OrgIntel::ensure(&url, &name).await.unwrap();
    for (id, kind, role) in [
        ("owner", "owner", "owner"),
        ("exec", "exec", "exec"),
        (LEAD, "staff", "lead"),
        (DESIGNER, "staff", "designer"),
    ] {
        org.ensure_actor(id, kind, role, id).await.unwrap();
    }
    org.create_team("Studio", "Win and deliver new sites", LEAD, "exec")
        .await
        .unwrap();
    let config = reno(&name).for_agent(LEAD);
    // Reno's lead: the company's main model on a second Claude login is a connection choice.
    assert!(!pins_own_model(&config, LEAD));
    let authority = AuthorityStore::connect(&url).await.unwrap();
    assert_eq!(
        with_lead_everyday_model(
            &config,
            LEAD,
            AgentHarness::RestlessManaged,
            vec!["anthropic/claude-opus-5-5".into()],
            &authority,
        )
        .await
        .unwrap(),
        ["anthropic/claude-sonnet-5-5", "anthropic/claude-opus-5-5"]
    );
    let mut day = Day {
        org,
        config,
        turns: Vec::new(),
    };
    let org = day.org.clone();

    // 1. The Exec commissions a goal, with its brief on the Work's thread: first planning. The
    // brief rides on every later Attempt of this Work, as it did in production.
    let goal = commission(
        &org,
        work(LEAD, "Next four prospects: concept sites"),
        "exec",
        None,
    )
    .await;
    org.send_work_message(
        "exec",
        LEAD,
        goal,
        "Build concept sites for the next four prospects.",
    )
    .await
    .unwrap();
    let first = claim(&org, goal).await;
    assert!(!day.work_turn("plan the commissioned goal", &first).await);
    // The lead hands the build to its designer and waits.
    let build = commission(
        &org,
        work(DESIGNER, "Build the Taylor & Ward concept"),
        LEAD,
        None,
    )
    .await;
    org.finish_work_attempt(
        first.attempt_id,
        WorkAttemptState::Blocked,
        "Waiting on the designer",
    )
    .await
    .unwrap();
    let staff = claim(&org, build).await;
    org.finish_work_attempt(
        staff.attempt_id,
        WorkAttemptState::Produced,
        "Concept is live",
    )
    .await
    .unwrap();

    // 2. The designer's result arrives: read it and route it.
    assert!(
        day.conversation(
            "read the Staff result",
            false,
            "- message from studio-designer: the Taylor & Ward concept is live"
        )
        .await
    );
    // 3. The lead's Work resumes with the designer's note: hand off the next step.
    org.send_work_message(DESIGNER, LEAD, goal, "Concept is live, preview linked.")
        .await
        .unwrap();
    resume(&org, goal, "the designer's concept is in").await;
    let resumed = claim(&org, goal).await;
    assert_eq!(resumed.attempt_no, 2);
    assert!(
        day.work_turn("resume after the Staff result", &resumed)
            .await
    );
    let next = commission(
        &org,
        work(LEAD, "Put the reviewed concept live"),
        LEAD,
        None,
    )
    .await;
    org.finish_work_attempt(
        resumed.attempt_id,
        WorkAttemptState::Blocked,
        "Next step handed on",
    )
    .await
    .unwrap();
    // 4. The lead's own next step.
    let step = claim(&org, next).await;
    assert!(day.work_turn("the lead's own next step", &step).await);
    org.finish_work_attempt(step.attempt_id, WorkAttemptState::Produced, "Live")
        .await
        .unwrap();
    // 5. A status update to the Exec.
    assert!(
        day.conversation(
            "status update",
            false,
            "- message from studio-designer: the live check passed"
        )
        .await
    );
    // 6. The owner writes on the goal: owner-facing.
    org.send_work_message("owner", LEAD, goal, "Make the motion subtler.")
        .await
        .unwrap();
    resume(&org, goal, "the owner asked for subtler motion").await;
    let noted = claim(&org, goal).await;
    assert!(!day.work_turn("the owner's note", &noted).await);
    // 7. That Attempt fails; its rerun is a repair.
    org.finish_work_attempt(
        noted.attempt_id,
        WorkAttemptState::Failed,
        "Declared complete without its artifact",
    )
    .await
    .unwrap();
    resume(&org, goal, "link the artifact this time").await;
    let repair = claim(&org, goal).await;
    assert!(!day.work_turn("repair after a failure", &repair).await);
    // 8. New direction from the Exec, and 9. a pricing call the decision model calls not routine.
    assert!(
        !day.conversation(
            "direction from the Exec",
            true,
            "- message from exec: new sectors"
        )
        .await
    );
    assert!(
        !day.conversation(
            "a pricing call",
            false,
            "- message from studio-designer: should we change our pricing for trades?"
        )
        .await
    );

    let everyday = day
        .turns
        .iter()
        .filter(|(_, everyday, _)| *everyday)
        .count();
    for (label, everyday, reason) in &day.turns {
        eprintln!(
            "{label:32} {} ({reason})",
            if *everyday { "everyday" } else { "main" }
        );
    }
    assert_eq!((everyday, day.turns.len()), (4, 9), "{:#?}", day.turns);
    let reasons: Vec<_> = day.turns.iter().map(|(_, _, reason)| *reason).collect();
    assert_eq!(
        reasons,
        [
            "first planning of a new goal",
            "routine, by the decision model",
            "routine, by the decision model",
            "routine, by the decision model",
            "routine, by the decision model",
            "owner-facing work",
            "a repair or retry",
            "direction from the Exec",
            "not routine, by the decision model",
        ]
    );
    // Every turn is in the decision log, with its reason.
    let logged = org.recent_decisions(Some(LEAD_TIER), 50).await.unwrap();
    assert_eq!(logged.len(), 9);
    assert_eq!(
        logged.iter().filter(|row| row.choice == "everyday").count(),
        4
    );
    org.drop_schema().await.unwrap();
}
