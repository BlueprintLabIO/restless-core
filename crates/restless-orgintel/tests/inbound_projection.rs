//! One Authority source reference becomes one OrgIntel message even when the
//! projection call is replayed after a crash window.

use restless_orgintel::{NewWork, OrgIntel, WorkspaceSpec};

#[tokio::test]
async fn external_source_projection_is_exactly_once() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping inbound projection scenario");
        return;
    };
    let company = format!("inboundprojection{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("world", "system", "external-sender", "The outside world")
        .await
        .unwrap();

    let source = "authority://inbound/42";
    let metadata = serde_json::json!({
        "transport_authenticated": true,
        "sender_content_trusted": false,
    });
    let first = org
        .project_external_message_once(
            "world",
            "exec",
            "[UNTRUSTED EXTERNAL EVIDENCE]\nhello",
            source,
            "resend",
            "evt_delivery_42",
            Some("email_7"),
            Some("<message_7@example.com>"),
            Some("thread_3"),
            Some("https://resend.com/emails/email_7"),
            &metadata,
            None,
            None,
        )
        .await
        .unwrap();
    let replay = org
        .project_external_message_once(
            "world",
            "exec",
            "this replay must not replace the first projection",
            source,
            "resend",
            "evt_delivery_42",
            Some("email_7"),
            Some("<message_7@example.com>"),
            Some("thread_3"),
            None,
            &metadata,
            None,
            None,
        )
        .await
        .unwrap();

    assert!(first.1);
    assert!(!replay.1);
    assert_eq!(first.0, replay.0);
    let inbox = org.inbox(Some("exec")).await.unwrap();
    assert_eq!(inbox.len(), 1);
    assert!(inbox[0].body.contains("UNTRUSTED EXTERNAL EVIDENCE"));
    assert!(!inbox[0].body.contains("replay must not replace"));

    org.create_actor(
        "customer-direction",
        "lead",
        "Avery Holt",
        None,
        "exec",
        "own customer outcomes",
    )
    .await
    .unwrap();
    org.create_actor(
        "customer-writer",
        "writer",
        "Mira Chen",
        None,
        "exec",
        "produce customer responses",
    )
    .await
    .unwrap();
    let team = org
        .create_team(
            "Customer response",
            "Resolve the exact customer outcome",
            "customer-direction",
            "exec",
        )
        .await
        .unwrap();
    org.set_actor_team(
        "customer-writer",
        Some(team),
        "customer-direction",
        "worker owns production while lead supervises",
    )
    .await
    .unwrap();
    let work = org
        .add_work_from_external_message_with_edges_and_gates(
            NewWork {
                owner_id: "customer-writer",
                title: "Prepare the grounded response",
                outcome: "one inspectable unsent response",
                goal_id: None,
                priority: 5,
                expected_artifact: "response.md",
                workspace: WorkspaceSpec::default(),
                attempt_limit: Some(2),
            },
            &[],
            &[],
            &[],
            false,
            first.0,
            "customer-direction",
        )
        .await
        .unwrap();
    let duplicate = org
        .add_work_from_external_message_with_edges_and_gates(
            NewWork {
                owner_id: "customer-writer",
                title: "Duplicate response",
                outcome: "this Work must never be created",
                goal_id: None,
                priority: 5,
                expected_artifact: "duplicate.md",
                workspace: WorkspaceSpec::default(),
                attempt_limit: Some(1),
            },
            &[],
            &[],
            &[],
            false,
            first.0,
            "customer-direction",
        )
        .await
        .unwrap_err();
    assert!(duplicate.to_string().contains("already commissioned Work"));

    let sources = org.work_external_message_sources(work).await.unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].source_ref, source);
    assert_eq!(sources[0].message_id, first.0);
    assert_eq!(sources[0].provider, "resend");
    assert_eq!(
        sources[0].source_url.as_deref(),
        Some("https://resend.com/emails/email_7")
    );
    assert!(sources[0].body.contains("UNTRUSTED EXTERNAL EVIDENCE"));

    let route = org
        .external_thread_route("resend", &["<message_7@example.com>".to_string()])
        .await
        .unwrap();
    assert_eq!(
        route,
        Some(("customer-writer".to_string(), Some(work))),
        "a reply reference routes to the existing Staff-owned Work"
    );
    let reply = org
        .project_external_message_once(
            "world",
            "customer-writer",
            "[UNTRUSTED EXTERNAL EVIDENCE]\nmid-Work reply",
            "authority://inbound/43",
            "resend",
            "evt_delivery_43",
            Some("email_8"),
            Some("<message_8@example.com>"),
            None,
            None,
            &metadata,
            Some(work),
            None,
        )
        .await
        .unwrap();
    assert!(org.message_is_work_attempt_input(reply.0).await.unwrap());
    let sources = org.work_external_message_sources(work).await.unwrap();
    assert_eq!(sources.len(), 2);
    assert_eq!(sources[1].message_id, reply.0);
    assert!(sources[1].body.contains("mid-Work reply"));
    let claimed = org
        .claim_ready_work("start exact response production")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claimed.work.id, work);
    let mid_work_route = org
        .external_thread_route("resend", &["<message_8@example.com>".to_string()])
        .await
        .unwrap();
    assert_eq!(
        mid_work_route,
        Some(("customer-direction".to_string(), Some(work))),
        "a reply during production reaches the non-producing supervisor"
    );

    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn department_projection_linearizes_with_lead_replacement() {
    let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping department routing race");
        return;
    };
    let company = format!("departmentroute{}", uuid::Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &company).await.unwrap();
    org.ensure_actor("exec", "exec", "exec", "The Exec")
        .await
        .unwrap();
    org.ensure_actor("world", "system", "external-sender", "The outside world")
        .await
        .unwrap();
    for (id, display) in [
        ("customer-direction", "Avery Holt"),
        ("customer-guide", "Morgan Reed"),
    ] {
        org.create_actor(id, "lead", display, None, "exec", "own customer outcomes")
            .await
            .unwrap();
    }
    let team_id = org
        .create_team(
            "Customer response",
            "Own customer outcomes",
            "customer-direction",
            "exec",
        )
        .await
        .unwrap();

    let projecting = org.clone();
    let replacing = org.clone();
    let metadata = serde_json::json!({
        "route": "department_address",
        "sender_content_trusted": false,
    });
    let (projection, replacement) = tokio::join!(
        async move {
            projecting
                .project_external_message_once(
                    "world",
                    "customer-direction",
                    "[UNTRUSTED EXTERNAL EVIDENCE]\nA concurrent department message",
                    "authority://inbound/department-race",
                    "resend",
                    "department-race-event",
                    None,
                    None,
                    None,
                    None,
                    &metadata,
                    None,
                    Some(team_id),
                )
                .await
        },
        async move {
            replacing
                .set_team_lead(
                    team_id,
                    "customer-guide",
                    "exec",
                    "rotate customer accountability",
                )
                .await
        }
    );
    replacement.unwrap();
    let (message_id, created, routed_to) = projection.unwrap();
    assert!(created);
    assert!(matches!(
        routed_to.as_str(),
        "customer-direction" | "customer-guide"
    ));
    assert!(org
        .inbox(Some("customer-direction"))
        .await
        .unwrap()
        .is_empty());
    if routed_to == "customer-direction" {
        let cancellation = org
            .latest_event("actor.conversation.cancelled.v1")
            .await
            .unwrap()
            .expect("send-first is terminally accounted for by replacement");
        assert!(cancellation.body["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|message| message["message_id"] == message_id));
    } else {
        assert!(org
            .inbox(Some("customer-guide"))
            .await
            .unwrap()
            .iter()
            .any(|message| message.id == message_id));
    }
    org.drop_schema().await.unwrap();
}
