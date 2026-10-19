//! The Exec-gate shadow, end to end through a real relay and decision model: an unread result
//! notice is decided once with its probabilities, nothing about the notice changes, and the wake's
//! outcome is written back. Needs `RESTLESS_TEST_DATABASE_URL` plus a relay at
//! `RESTLESS_TEST_DECISIONS_BASE_URL` with bearer `RESTLESS_TEST_DECISIONS_TOKEN`; skipped otherwise.

use restless_engine::decisions::{
    settle_exec_results, shadow_exec_results_with, DecisionClient, EXEC_GATE,
};
use restless_orgintel::OrgIntel;

#[tokio::test]
async fn a_result_notice_is_decided_in_shadow_and_settled_by_the_wake() {
    let (Ok(url), Ok(base), Ok(token)) = (
        std::env::var("RESTLESS_TEST_DATABASE_URL"),
        std::env::var("RESTLESS_TEST_DECISIONS_BASE_URL"),
        std::env::var("RESTLESS_TEST_DECISIONS_TOKEN"),
    ) else {
        eprintln!("decision relay or database unset; skipping exec gate shadow scenario");
        return;
    };
    let company = format!("decide{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
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
        .send_message("daemon", Some("exec"), "Completed Work result for Exec delivery consideration: 8\nWork 8: The website the owner asked to review is complete; critic passed; live preview ready. Resolution: done.")
        .await
        .unwrap();
    let unrelated = org
        .send_message(
            "daemon",
            Some("exec"),
            "Material Runtime supervisor events for Work 9 (1 exception): blocked",
        )
        .await
        .unwrap();

    let client = DecisionClient::new(&base, token).expect("relay base parses");
    shadow_exec_results_with(&client, &org, "A small web studio.", false).await;
    shadow_exec_results_with(&client, &org, "A small web studio.", false).await;

    let decisions = org.recent_decisions(Some(EXEC_GATE), 10).await.unwrap();
    let subjects: Vec<&str> = decisions.iter().map(|row| row.subject.as_str()).collect();
    assert_eq!(
        decisions.len(),
        2,
        "each result notice decided once, exceptions never: {subjects:?}"
    );
    assert!(!subjects.contains(&unrelated.to_string().as_str()));
    let of = |id: i64| {
        decisions
            .iter()
            .find(|row| row.subject == id.to_string())
            .unwrap()
    };
    for row in [of(routine), of(wanted)] {
        assert_eq!(row.mode, "shadow");
        assert!(
            row.answers["owner_update"]["noul"].is_number()
                && row.answers["next_step"]["noul"].is_number(),
            "{:?}",
            row.answers
        );
        eprintln!(
            "message {} -> {} {} in {}ms by {}",
            row.subject, row.choice, row.answers, row.latency_ms, row.model
        );
    }
    assert_eq!(
        of(wanted).choice,
        "wake",
        "a deliverable the owner asked for must wake the Exec"
    );
    // Shadow changes nothing: all three notices are still unread for the Exec.
    assert_eq!(org.inbox(Some("exec")).await.unwrap().len(), 3);

    // A Staff triage question through the same route, answered and logged under that actor. The
    // CLI path needs the relay in its environment, as on a hosted plane.
    unsafe {
        std::env::set_var("GPT_BASE_URL", &base);
        std::env::set_var(
            "RESTLESS_HOSTED_MODEL_RELAY_TOKEN",
            std::env::var("RESTLESS_TEST_DECISIONS_TOKEN").unwrap(),
        );
    }
    let answer = restless_engine::decisions::ask_for_actor(
        &org,
        None,
        "robin",
        serde_json::json!({
            "state": {"email": "Plumber in Parramatta, need a 5-page site live before 1 March, budget around $5k. Can we talk this week?"},
            "questions": {"lead": {"type": "choice", "instructions": "How promising is this sales lead?",
                "criteria": {"hot": "Concrete need, budget or timeline; wants to talk soon.", "warm": "Interested but vague.", "cold": "Not a buyer.", "spam": "Junk."}}}
        }),
    )
    .await
    .expect("a staff decision is answered");
    assert_eq!(answer["answers"]["lead"]["choice"], "hot", "{answer}");
    let staff = org.recent_decisions(Some("staff"), 10).await.unwrap();
    assert_eq!(staff.len(), 1);
    assert!(
        staff[0].subject.starts_with("robin:")
            && staff[0].mode == "active"
            && staff[0].choice == "lead=hot",
        "{:?}",
        staff[0]
    );
    assert!(
        restless_engine::decisions::ask_for_actor(
            &org,
            None,
            "robin",
            serde_json::json!({"state": "x", "questions": {}})
        )
        .await
        .is_err(),
        "a decision with no questions is refused"
    );

    settle_exec_results(&org, &[routine, wanted, unrelated], true, false).await;
    let settled = org.recent_decisions(Some(EXEC_GATE), 10).await.unwrap();
    assert!(settled
        .iter()
        .all(|row| row.outcome.as_deref() == Some("update")));
}

/// The active gate (S102), against OpenRouter's decisions API with a company-named model: a routine
/// result is consumed without an Exec wake only on a confident double no, and a deliverable the
/// owner asked for always stays for the Exec. Needs `RESTLESS_TEST_DATABASE_URL` and
/// `RESTLESS_TEST_OPENROUTER_KEY`; `RESTLESS_TEST_DECISION_MODEL` defaults to `typesafe/jev-1.14`.
#[tokio::test]
async fn an_active_gate_consumes_only_confident_routine_results() {
    let (Ok(url), Ok(key)) = (
        std::env::var("RESTLESS_TEST_DATABASE_URL"),
        std::env::var("RESTLESS_TEST_OPENROUTER_KEY"),
    ) else {
        eprintln!("OpenRouter key or database unset; skipping active exec gate scenario");
        return;
    };
    let model = std::env::var("RESTLESS_TEST_DECISION_MODEL")
        .unwrap_or_else(|_| "typesafe/jev-1.14".into());
    let company = format!("gate{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("daemon", "system", "system-sender", "The daemon")
        .await
        .unwrap();
    let routine = org
        .send_message(
            "daemon",
            Some("exec"),
            "Completed Work result for Exec delivery consideration: 7
Work 7: Nightly backup completed and verified. Resolution: done.",
        )
        .await
        .unwrap();
    let wanted = org
        .send_message("daemon", Some("exec"), "Completed Work result for Exec delivery consideration: 8
Work 8: The website the owner asked to review is complete; critic passed; live preview ready. Resolution: done.")
        .await
        .unwrap();
    let client = DecisionClient::openrouter(key, &model).expect("OpenRouter route");
    shadow_exec_results_with(&client, &org, "A small web studio.", true).await;

    let decisions = org.recent_decisions(Some(EXEC_GATE), 10).await.unwrap();
    assert_eq!(decisions.len(), 2, "both results decided: {decisions:?}");
    let of = |id: i64| {
        decisions
            .iter()
            .find(|row| row.subject == id.to_string())
            .unwrap()
    };
    for row in [of(routine), of(wanted)] {
        eprintln!(
            "message {} -> {} ({}) {} in {}ms by {}",
            row.subject, row.choice, row.mode, row.answers, row.latency_ms, row.model
        );
    }
    assert_eq!(of(wanted).choice, "wake");
    assert_eq!(of(wanted).mode, "shadow");
    let unread: Vec<i64> = org
        .inbox(Some("exec"))
        .await
        .unwrap()
        .iter()
        .map(|message| message.id)
        .collect();
    assert!(
        unread.contains(&wanted),
        "the deliverable stays for the Exec"
    );
    // A consumed result is exactly an active skip; anything else is still unread.
    assert_eq!(
        !unread.contains(&routine),
        of(routine).mode == "active" && of(routine).choice == "skip_wake"
    );
}
