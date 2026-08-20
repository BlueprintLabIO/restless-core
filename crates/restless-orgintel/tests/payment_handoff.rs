//! Provider observation may resume an irreducible owner step, but can never
//! answer open-ended owner judgement. Runs against a disposable company schema.

use restless_orgintel::{
    NewOwnerHandoff, NewWork, OrgIntel, OwnerHandoffCategory, OwnerHandoffState, WorkStatus,
    WorkspaceSpec,
};

#[tokio::test]
async fn authenticated_provider_observation_resumes_only_the_exact_human_step() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping payment handoff scenario");
        return;
    };
    let company = format!("payhandoff{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("daemon", "system", "system-sender", "The daemon")
        .await
        .unwrap();

    let payment_work = org
        .add_work(NewWork {
            owner_id: "exec",
            title: "Pay the accepted invoice",
            outcome: "Provider-confirmed payment or rejection resumes the company",
            goal_id: None,
            priority: 50,
            expected_artifact: "authenticated provider receipt",
            workspace: WorkspaceSpec::default(),
            attempt_limit: None,
        })
        .await
        .unwrap();
    let payment_handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: payment_work,
            attempt_id: None,
            requested_by: "exec",
            category: OwnerHandoffCategory::PaymentConfirmation,
            requested_action: "approve the exact transfer in the provider",
            prepared_state: "provider transfer transfer-test is IN_APPROVAL",
            resume_condition: "authenticated provider state leaves IN_APPROVAL",
        })
        .await
        .unwrap();

    assert!(org
        .resolve_observed_handoff(
            payment_handoff,
            "daemon",
            "authenticated provider API observed transfer-test as scheduled",
        )
        .await
        .unwrap());
    assert!(!org
        .resolve_observed_handoff(payment_handoff, "daemon", "duplicate webhook")
        .await
        .unwrap());
    let graph = org.work_graph_snapshot().await.unwrap();
    assert_eq!(
        graph
            .handoffs
            .iter()
            .find(|handoff| handoff.id == payment_handoff)
            .unwrap()
            .state,
        OwnerHandoffState::Resolved
    );
    assert_eq!(
        graph
            .work
            .iter()
            .find(|work| work.id == payment_work)
            .unwrap()
            .status,
        WorkStatus::Active
    );

    let judgement_work = org
        .add_work(NewWork {
            owner_id: "exec",
            title: "Choose a strategy",
            outcome: "Owner judgement determines the strategy",
            goal_id: None,
            priority: 40,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: None,
        })
        .await
        .unwrap();
    let judgement_handoff = org
        .request_owner_handoff(NewOwnerHandoff {
            work_id: judgement_work,
            attempt_id: None,
            requested_by: "exec",
            category: OwnerHandoffCategory::OwnerJudgement,
            requested_action: "choose the strategy",
            prepared_state: "two evidence-backed choices are ready",
            resume_condition: "owner records a choice",
        })
        .await
        .unwrap();
    assert!(org
        .resolve_observed_handoff(judgement_handoff, "daemon", "provider claims option A")
        .await
        .is_err());

    org.drop_schema().await.unwrap();
}
