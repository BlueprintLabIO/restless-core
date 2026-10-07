//! The Exec-gate shadow, end to end through a real relay and decision model: an unread result
//! notice is decided once with its probabilities, nothing about the notice changes, and the wake's
//! outcome is written back. Needs `RESTLESS_TEST_DATABASE_URL` plus a relay at
//! `RESTLESS_TEST_DECISIONS_BASE_URL` with bearer `RESTLESS_TEST_DECISIONS_TOKEN`; skipped otherwise.

use restless_engine::decisions::{
    DecisionClient, EXEC_GATE, settle_exec_results, shadow_exec_results_with,
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
    shadow_exec_results_with(&client, &org, "A small web studio.").await;
    shadow_exec_results_with(&client, &org, "A small web studio.").await;

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
            "robin",
            serde_json::json!({"state": "x", "questions": {}})
        )
        .await
        .is_err(),
        "a decision with no questions is refused"
    );

    settle_exec_results(&org, &[routine, wanted, unrelated], true, false).await;
    let settled = org.recent_decisions(Some(EXEC_GATE), 10).await.unwrap();
    assert!(
        settled
            .iter()
            .all(|row| row.outcome.as_deref() == Some("update"))
    );
}
