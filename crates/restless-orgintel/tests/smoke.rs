//! Live Postgres round-trip for the OrgIntel core (T5). OrgIntel testing is
//! behavioural: a scratch company schema is created, every table is written
//! and read back, the commitment state machine is exercised, and the schema
//! is dropped at the end. Runs only when RESTLESS_TEST_DATABASE_URL is set —
//! the daemon's own acceptance covers this against the real database.

use restless_orgintel::{CommitmentState, OrgIntel};

#[tokio::test]
async fn company_schema_round_trip() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping live Postgres round-trip");
        return;
    };
    let company = format!("smoke{}", std::process::id());
    let org = OrgIntel::ensure(&url, &company).await.expect("ensure schema");
    assert_eq!(org.schema(), company);

    // Every table gets written and read back — the T5 guard is that no table
    // exists that a company never writes to, so the smoke test writes to all.
    org.add_actor("exec", "exec", "The Exec").await.unwrap();
    org.add_actor("owner", "owner", "The Owner").await.unwrap();
    org.add_actor("staff-builder", "staff", "Builder").await.unwrap();

    let goal = org.add_goal("Ship the walking skeleton", "", "exec").await.unwrap();
    let goals = org.list_goals().await.unwrap();
    assert_eq!(goals.len(), 1);
    assert!(goals[0].closed_at.is_none());

    let commitment = org
        .add_commitment("staff-builder", "First slice", "prove the loop", Some(goal))
        .await
        .unwrap();
    org.set_commitment_state(commitment, CommitmentState::Active, "").await.unwrap();
    org.set_commitment_state(commitment, CommitmentState::Completed, "slice shipped")
        .await
        .unwrap();
    let commitments = org.list_commitments().await.unwrap();
    assert_eq!(commitments[0].state, CommitmentState::Completed);
    assert_eq!(commitments[0].resolution, "slice shipped");

    let message = org.send_message("exec", None, "status: alive").await.unwrap();
    let inbox = org.inbox(None).await.unwrap();
    assert_eq!(inbox.len(), 1);
    assert_eq!(inbox[0].from_actor, "exec");
    org.mark_read(message).await.unwrap();
    assert!(org.inbox(None).await.unwrap().is_empty());

    org.add_artifact_ref("path", "/company/outputs/demo/index.html", "first output", "exec")
        .await
        .unwrap();
    org.add_decision("Codex over Claude Code", "T3 spike evidence", "exec").await.unwrap();
    let event = org
        .emit_event("wake", Some("exec"), serde_json::json!({"reason": "tick"}))
        .await
        .unwrap();
    assert!(event > 0);
    let events = org.list_events(10).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "wake");

    let tables = org.table_names().await.unwrap();
    for expected in [
        "actors",
        "goals",
        "commitments",
        "messages",
        "artifact_refs",
        "decisions",
        "events",
    ] {
        assert!(tables.contains(&expected.to_string()), "missing table {expected}");
    }

    // A second handle to the same company sees the same state (schema pin).
    let again = OrgIntel::ensure(&url, &company).await.unwrap();
    assert_eq!(again.list_commitments().await.unwrap().len(), 1);

    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn bad_schema_names_are_rejected() {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/restless".to_string());
    for name in ["", "1cosmon", "Cosmon", "cos;mon", "cosmon\"; DROP TABLE actors; --"] {
        assert!(OrgIntel::ensure(&url, name).await.is_err(), "accepted {name:?}");
    }
}
