//! `decision_model = "included"`: the hosted relay's decision model, with the gate active and no
//! key of the company's own. Against a scripted relay on loopback: a routine result is consumed
//! without an Exec wake, a deliverable stays for the Exec, an unset company stays in shadow, and a
//! routine first Attempt is tiered to the everyday model through the same relay. Needs
//! `RESTLESS_TEST_DATABASE_URL`; skipped otherwise.

use axum::{routing::post, Json, Router};
use restless_engine::decisions::{
    decision_route, shadow_exec_results, work_suits_everyday_model, DecisionRoute, EXEC_GATE,
    WORK_TIER,
};
use restless_engine::runtime::CompanyConfig;
use restless_orgintel::{NewWork, OrgIntel, WorkspaceSpec};
use serde_json::{json, Value};

/// A scripted relay: a nightly backup and a rename are routine, anything else matters.
async fn decide(Json(request): Json<Value>) -> Json<Value> {
    let state = request["state"].to_string();
    let routine = state.contains("Nightly backup") || state.contains("Rename");
    let p = if routine { 0.05 } else { 0.95 };
    let mut answers = serde_json::Map::new();
    for id in request["questions"].as_object().unwrap().keys() {
        let noul = if id == "routine" { 1.0 - p } else { p };
        answers.insert(id.clone(), json!({ "type": "noul", "noul": noul }));
    }
    Json(json!({ "model": "relay/scripted", "answers": answers }))
}

fn company(name: &str, decision_model: Option<&str>) -> CompanyConfig {
    let mut raw = format!("name = \"{name}\"\nmodel = \"anthropic/claude-opus-5-5\"\n");
    if let Some(value) = decision_model {
        raw.push_str(&format!("decision_model = \"{value}\"\n"));
    }
    toml::from_str(&raw).expect("company config parses")
}

async fn exec_with_results(url: &str, name: &str) -> (OrgIntel, i64, i64) {
    let org = OrgIntel::ensure(url, name).await.unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("daemon", "system", "system-sender", "The daemon")
        .await
        .unwrap();
    let routine = org
        .send_message("daemon", Some("exec"), "Completed Work result for Exec delivery consideration: 7\nWork 7: Nightly backup completed and verified. Resolution: done.")
        .await
        .unwrap();
    let wanted = org
        .send_message("daemon", Some("exec"), "Completed Work result for Exec delivery consideration: 8\nWork 8: The website the owner asked to review is complete; live preview ready. Resolution: done.")
        .await
        .unwrap();
    (org, routine, wanted)
}

#[test]
fn included_is_the_relay_with_the_gate_active() {
    assert_eq!(
        decision_route(&company("a", None)),
        DecisionRoute::RelayShadow
    );
    assert!(!DecisionRoute::RelayShadow.active());
    for value in ["included", "Included", " included "] {
        assert_eq!(
            decision_route(&company("a", Some(value))),
            DecisionRoute::Relay
        );
    }
    assert!(DecisionRoute::Relay.active());
    assert_eq!(
        decision_route(&company("a", Some("typesafe/jev-1.14"))),
        DecisionRoute::OpenRouter("typesafe/jev-1.14")
    );
}

#[tokio::test]
async fn included_decisions_gate_results_and_tier_work_through_the_relay() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("database unset; skipping included decisions scenario");
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

    // Included: the routine result is consumed, the deliverable stays for the Exec.
    let name = format!("incl{}", uuid::Uuid::new_v4().simple());
    let (org, routine, wanted) = exec_with_results(&url, &name).await;
    let included = company(&name, Some("included"));
    shadow_exec_results(&org, &included, true).await;
    let unread: Vec<i64> = org
        .inbox(Some("exec"))
        .await
        .unwrap()
        .iter()
        .map(|message| message.id)
        .collect();
    assert_eq!(unread, vec![wanted], "only the deliverable wakes the Exec");
    let decisions = org.recent_decisions(Some(EXEC_GATE), 10).await.unwrap();
    let of = |id: i64| {
        decisions
            .iter()
            .find(|row| row.subject == id.to_string())
            .unwrap()
    };
    assert_eq!(
        (of(routine).choice.as_str(), of(routine).mode.as_str()),
        ("skip_wake", "active")
    );
    assert_eq!(
        (of(wanted).choice.as_str(), of(wanted).mode.as_str()),
        ("wake", "shadow")
    );
    assert_eq!(of(routine).model, "relay/scripted");

    // Routine Work tiers to the everyday model through the same relay; judgement Work does not.
    for (title, everyday) in [
        ("Rename the invoices folder", true),
        ("Design the new brand", false),
    ] {
        let id = org
            .add_work(NewWork {
                owner_id: "exec",
                title,
                outcome: title,
                goal_id: None,
                priority: 1,
                expected_artifact: "",
                workspace: WorkspaceSpec::default(),
                attempt_limit: Some(3),
            })
            .await
            .unwrap();
        let work = org.get_work(id).await.unwrap().unwrap();
        assert_eq!(
            work_suits_everyday_model(&org, &included, &work).await,
            everyday,
            "{title}"
        );
    }
    assert_eq!(
        org.recent_decisions(Some(WORK_TIER), 10)
            .await
            .unwrap()
            .len(),
        2
    );

    // Unset: the same relay decides, in shadow, and nothing is consumed.
    let name = format!("shad{}", uuid::Uuid::new_v4().simple());
    let (org, routine, _) = exec_with_results(&url, &name).await;
    shadow_exec_results(&org, &company(&name, None), true).await;
    assert_eq!(org.inbox(Some("exec")).await.unwrap().len(), 2);
    let decisions = org.recent_decisions(Some(EXEC_GATE), 10).await.unwrap();
    let routine = decisions
        .iter()
        .find(|row| row.subject == routine.to_string())
        .unwrap();
    assert_eq!(
        (routine.choice.as_str(), routine.mode.as_str()),
        ("skip_wake", "shadow")
    );
}
