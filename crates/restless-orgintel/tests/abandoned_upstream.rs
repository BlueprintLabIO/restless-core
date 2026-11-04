//! Work that requires abandoned Work can never start (readiness waits for a completed upstream).
//! Abandoning tells each open dependent's lead the two ways forward, and no new requirement on
//! abandoned Work is accepted (S109 blockers audit: Frankensurf Work sat proposed from 18 Oct).

use restless_orgintel::{NewWork, OrgIntel, WorkEdgeKind, WorkspaceSpec};

async fn company() -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!(
        "abandonedup{}_test",
        &uuid::Uuid::new_v4().simple().to_string()[..12]
    );
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch company schema");
    for (id, kind, role) in [
        ("owner", "owner", "owner"),
        ("exec", "exec", "exec"),
        ("alpha-build", "staff", "builder"),
        ("beta-check", "staff", "checker"),
    ] {
        org.ensure_actor(id, kind, role, id).await.unwrap();
    }
    Some(org)
}

fn work<'a>(owner: &'a str, title: &'a str) -> NewWork<'a> {
    NewWork {
        owner_id: owner,
        title,
        outcome: "done",
        goal_id: None,
        priority: 1,
        expected_artifact: "note",
        workspace: WorkspaceSpec::default(),
        attempt_limit: Some(1),
    }
}

#[tokio::test]
async fn abandoning_upstream_work_tells_its_dependents() {
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping abandoned upstream scenario");
        return;
    };
    let upstream = org
        .add_work(work("alpha-build", "Prove the tools"))
        .await
        .unwrap();
    let dependent = org
        .add_work(work("beta-check", "Check the proof"))
        .await
        .unwrap();
    org.add_work_edge(upstream, dependent, WorkEdgeKind::Requires)
        .await
        .unwrap();

    org.abandon_work(upstream, "exec", "superseded by a narrower probe")
        .await
        .unwrap();

    let told = org.inbox(Some("beta-check")).await.unwrap();
    assert!(
        told.iter().any(|message| message.from_actor == "exec"
            && message.body.contains(&dependent.to_string())
            && message.body.contains("--remove")),
        "the dependent's lead hears how to unblock it: {told:?}"
    );

    let later = org
        .add_work(work("beta-check", "Another check"))
        .await
        .unwrap();
    let refused = org
        .add_work_edge(upstream, later, WorkEdgeKind::Requires)
        .await
        .expect_err("abandoned Work cannot be required");
    assert!(refused.to_string().contains("abandoned"), "{refused}");

    org.drop_schema().await.unwrap();
}

/// A produced result that open Work still requires is an intermediate step: no Exec wake for it.
/// The last link's result reaches Exec as before (4 Nov: each such notice cost Exec a full wake to
/// conclude "internal step, stay quiet").
#[tokio::test]
async fn an_intermediate_result_does_not_wake_exec_and_the_last_one_does() {
    use restless_orgintel::WorkAttemptState;
    let Some(org) = company().await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping intermediate result scenario");
        return;
    };
    // No expected artifact: a produced Attempt then completes the Work without linking one.
    let quick = |owner, title| NewWork {
        expected_artifact: "",
        ..work(owner, title)
    };
    let upstream = org.add_work(quick("alpha-build", "Draft")).await.unwrap();
    let dependent = org
        .add_work(quick("beta-check", "Check the draft"))
        .await
        .unwrap();
    org.add_work_edge(upstream, dependent, WorkEdgeKind::Requires)
        .await
        .unwrap();
    for message in org.conversation_inbox("exec").await.unwrap() {
        org.mark_read(message.id).await.unwrap();
    }

    let first = org.claim_ready_work("draft").await.unwrap().unwrap();
    assert_eq!(first.work.id, upstream);
    org.finish_work_attempt(first.attempt_id, WorkAttemptState::Produced, "drafted")
        .await
        .unwrap();
    assert!(org
        .flush_terminal_supervisor_notices(100)
        .await
        .unwrap()
        .is_empty());
    assert!(org.conversation_inbox("exec").await.unwrap().is_empty());
    assert!(
        org.flush_terminal_supervisor_notices(100)
            .await
            .unwrap()
            .is_empty(),
        "the skipped notice is settled, not retried"
    );

    let second = org.claim_ready_work("check").await.unwrap().unwrap();
    assert_eq!(second.work.id, dependent);
    org.finish_work_attempt(second.attempt_id, WorkAttemptState::Produced, "checked")
        .await
        .unwrap();
    assert_eq!(
        org.flush_terminal_supervisor_notices(100)
            .await
            .unwrap()
            .len(),
        1
    );
    let results = org.conversation_inbox("exec").await.unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].body.contains(&dependent.to_string()));

    org.drop_schema().await.unwrap();
}
