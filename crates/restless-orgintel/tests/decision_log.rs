//! A structured decision is recorded once per subject with its probabilities, and the company's
//! later answer is written once. Runs against a scratch Postgres company when
//! `RESTLESS_TEST_DATABASE_URL` is set.

use restless_orgintel::{NewDecision, OrgIntel};

#[tokio::test]
async fn a_decision_is_recorded_once_and_settled_once() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping decision log scenario");
        return;
    };
    let company = format!("decisions{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    let answers = serde_json::json!({"owner_update": {"type": "noul", "noul": 0.04}, "next_step": {"type": "noul", "noul": 0.11}});
    let decision = |subject: &'static str, choice: &'static str| NewDecision {
        kind: "exec_gate",
        subject,
        mode: "shadow",
        model: "typesafe/jev-1.13-20260917",
        answers: &answers,
        choice,
        latency_ms: 431,
    };

    assert!(
        org.record_decision(decision("101", "skip_wake"))
            .await
            .unwrap()
    );
    assert!(
        !org.record_decision(decision("101", "wake")).await.unwrap(),
        "a subject keeps its first decision"
    );
    assert!(org.record_decision(decision("102", "wake")).await.unwrap());
    assert_eq!(
        org.decided_subjects("exec_gate", &["101".into(), "103".into()])
            .await
            .unwrap(),
        vec!["101".to_string()]
    );

    // The wake that consumed both notices stayed quiet; a later relabel cannot rewrite it.
    let both = ["101".to_string(), "102".to_string()];
    assert_eq!(
        org.settle_decision_outcomes("exec_gate", &both, "quiet")
            .await
            .unwrap(),
        2
    );
    assert_eq!(
        org.settle_decision_outcomes("exec_gate", &both, "update")
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        org.settle_decision_outcomes("routing", &both, "quiet")
            .await
            .unwrap(),
        0
    );

    let recent = org.recent_decisions(Some("exec_gate"), 10).await.unwrap();
    assert_eq!(recent.len(), 2);
    let first = recent.iter().find(|row| row.subject == "101").unwrap();
    assert_eq!(
        (first.choice.as_str(), first.outcome.as_deref()),
        ("skip_wake", Some("quiet"))
    );
    assert_eq!(first.answers["owner_update"]["noul"], 0.04);
    assert!(
        org.recent_decisions(Some("routing"), 10)
            .await
            .unwrap()
            .is_empty()
    );
}
