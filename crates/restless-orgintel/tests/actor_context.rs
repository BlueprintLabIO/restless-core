//! Real-Postgres adversarial coverage for Sprint 49 Actor context continuity.

use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use restless_orgintel::{
    ActorCheckpointBody, ActorCheckpointSession, ActorContextEpistemicKind, ActorContextFocus,
    ActorContextFocusProjection, ActorContextReturnPath, ActorContextSourceFreshness,
    ActorContextSourceRef, ActorContextSourceTrust, DocumentAccess, DocumentKind,
    DocumentVisibility, NewActorCheckpoint, NewActorCheckpointSource, NewDocument,
    NewRoomMessageMention, NewWork, OrgIntel, RoomKind, SetDocumentParticipant, WorkspaceSpec,
};
use serde_json::json;
use sqlx::postgres::PgConnection;
use sqlx::{Connection as _, Executor as _};
use uuid::Uuid;

async fn company(prefix: &str) -> Option<(String, OrgIntel)> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!("{prefix}{}_test", Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch Actor-context company");
    org.ensure_actor("owner", "owner", "owner", "Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "executive", "Exec")
        .await
        .unwrap();
    org.ensure_actor("delivery-build", "staff", "builder", "Delivery Builder")
        .await
        .unwrap();
    org.ensure_actor("research-probe", "staff", "researcher", "Research Probe")
        .await
        .unwrap();
    Some((url, org))
}

fn body(status: &str) -> ActorCheckpointBody {
    ActorCheckpointBody {
        current_objective: "Ship the bounded result".into(),
        current_status: status.into(),
        completed: vec!["Inspected the exact input".into()],
        material_findings: vec!["The current candidate has one remaining seam".into()],
        decisions_made: vec![],
        changed_refs: vec!["/company/worktrees/result.md".into()],
        failed_approaches: vec!["A broad rewrite obscured the contract".into()],
        blockers: vec![],
        next_useful_action: "Repair the one remaining seam and run the native gate".into(),
        expected_receiver: Some("delivery lead".into()),
    }
}

fn source(source: ActorContextSourceRef, statement: &str) -> NewActorCheckpointSource {
    NewActorCheckpointSource {
        epistemic_kind: ActorContextEpistemicKind::Observation,
        source,
        statement: statement.into(),
        scope: Some("current Work".into()),
        expires_at: None,
    }
}

async fn claimed_work(org: &OrgIntel, owner_id: &str) -> (Uuid, Uuid) {
    let work_id = org
        .add_work(NewWork {
            owner_id,
            title: "Bounded continuity",
            outcome: "one exact restart-safe result",
            goal_id: None,
            priority: 1,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(3),
        })
        .await
        .unwrap();
    let claimed = org
        .claim_ready_work("Actor checkpoint test")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claimed.work.id, work_id);
    (work_id, claimed.attempt_id)
}

#[tokio::test]
async fn checkpoint_replay_cas_and_restart_are_exact() {
    let Some((database_url, org)) = company("actorcontextcas").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Actor context scenario");
        return;
    };
    let (work_id, attempt_id) = claimed_work(&org, "delivery-build").await;
    let sources = vec![source(
        ActorContextSourceRef::Work {
            work_id,
            revision: 1,
        },
        "Work revision one is the current contract",
    )];
    let first = org
        .record_actor_checkpoint(NewActorCheckpoint {
            client_command_id: "checkpoint-one",
            actor_id: "delivery-build",
            expected_current_version: 0,
            session: ActorCheckpointSession::WorkAttempt {
                work_id,
                attempt_id,
            },
            body: body("Candidate inspected"),
            sources: &sources,
        })
        .await
        .unwrap();
    assert!(first.created);
    assert_eq!(first.checkpoint.checkpoint.checkpoint_version, 1);

    let restarted = OrgIntel::ensure(&database_url, org.schema()).await.unwrap();
    let replay = restarted
        .record_actor_checkpoint(NewActorCheckpoint {
            client_command_id: "checkpoint-one",
            actor_id: "  delivery-build  ",
            expected_current_version: 0,
            session: ActorCheckpointSession::WorkAttempt {
                work_id,
                attempt_id,
            },
            body: body("Candidate inspected"),
            sources: &sources,
        })
        .await
        .unwrap();
    assert!(!replay.created);
    assert_eq!(
        replay.checkpoint.checkpoint.id,
        first.checkpoint.checkpoint.id
    );

    let drift = restarted
        .record_actor_checkpoint(NewActorCheckpoint {
            client_command_id: "checkpoint-one",
            actor_id: "delivery-build",
            expected_current_version: 0,
            session: ActorCheckpointSession::WorkAttempt {
                work_id,
                attempt_id,
            },
            body: body("Different semantics"),
            sources: &sources,
        })
        .await
        .unwrap_err();
    assert!(drift.to_string().contains("different semantics"));

    let left_sources = sources.clone();
    let right_sources = sources.clone();
    let left = restarted.clone();
    let right = org.clone();
    let (left_result, right_result) = tokio::join!(
        left.record_actor_checkpoint(NewActorCheckpoint {
            client_command_id: "checkpoint-two-left",
            actor_id: "delivery-build",
            expected_current_version: 1,
            session: ActorCheckpointSession::WorkAttempt {
                work_id,
                attempt_id,
            },
            body: body("Left contender"),
            sources: &left_sources,
        }),
        right.record_actor_checkpoint(NewActorCheckpoint {
            client_command_id: "checkpoint-two-right",
            actor_id: "delivery-build",
            expected_current_version: 1,
            session: ActorCheckpointSession::WorkAttempt {
                work_id,
                attempt_id,
            },
            body: body("Right contender"),
            sources: &right_sources,
        })
    );
    assert_eq!(left_result.is_ok() as u8 + right_result.is_ok() as u8, 1);
    let loser = left_result.err().or_else(|| right_result.err()).unwrap();
    assert!(loser.to_string().contains("version 2, expected 1"));
    assert_eq!(
        restarted
            .latest_actor_checkpoint("delivery-build", 8)
            .await
            .unwrap()
            .unwrap()
            .checkpoint
            .checkpoint_version,
        2
    );

    let mut oversized_body = body("Oversized checkpoint");
    oversized_body.completed = (0..20).map(|_| "x".repeat(2_000)).collect();
    let oversized = restarted
        .record_actor_checkpoint(NewActorCheckpoint {
            client_command_id: "checkpoint-too-large",
            actor_id: "delivery-build",
            expected_current_version: 2,
            session: ActorCheckpointSession::WorkAttempt {
                work_id,
                attempt_id,
            },
            body: oversized_body,
            sources: &sources,
        })
        .await
        .unwrap_err();
    assert!(oversized.to_string().contains("checkpoint body exceeds"));

    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn source_freshness_trust_bounds_and_company_isolation_survive_restart() {
    let Some((database_url, org)) = company("actorcontextsource").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Actor context scenario");
        return;
    };
    let (work_id, attempt_id) = claimed_work(&org, "delivery-build").await;
    let hidden_room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Private research",
            &["owner", "research-probe"],
            "hidden-source-room",
        )
        .await
        .unwrap();
    let hidden_message = org
        .send_room_message(
            hidden_room.id,
            "owner",
            "This Message is not visible to delivery-build.",
            None,
            "hidden-source-message",
        )
        .await
        .unwrap();
    let document_content = json!({
        "type": "doc",
        "content": [{
            "type": "paragraph",
            "attrs": {"block_id": "private"},
            "content": [{"type": "text", "text": "Private source"}]
        }]
    });
    let hidden_document = org
        .create_document(NewDocument {
            command_id: Uuid::new_v4(),
            title: "Private source",
            kind: DocumentKind::OperatingNote,
            visibility: DocumentVisibility::Participants,
            linked_room_id: None,
            inherit_room_visibility: false,
            owner_actor_id: "owner",
            created_by_actor_id: "owner",
            content_json: &document_content,
            reason: "Access-bound source fixture",
        })
        .await
        .unwrap();
    let hidden_document_view = org
        .get_document_for_actor(hidden_document.document_id, "owner")
        .await
        .unwrap();
    let hidden_room_sources = vec![source(
        ActorContextSourceRef::Message {
            room_id: hidden_room.id,
            message_id: hidden_message.message.id,
        },
        "A guessed Message id must not be link-laundered",
    )];
    assert!(org
        .record_actor_checkpoint(NewActorCheckpoint {
            client_command_id: "hidden-room-source",
            actor_id: "delivery-build",
            expected_current_version: 0,
            session: ActorCheckpointSession::WorkAttempt {
                work_id,
                attempt_id,
            },
            body: body("Trying an invisible Room source"),
            sources: &hidden_room_sources,
        })
        .await
        .unwrap_err()
        .to_string()
        .contains("not visible"));
    let hidden_document_sources = vec![source(
        ActorContextSourceRef::Document {
            document_id: hidden_document.document_id,
            named_version_id: hidden_document_view.current_version.version.id,
        },
        "A guessed private Doc id must not be link-laundered",
    )];
    assert!(org
        .record_actor_checkpoint(NewActorCheckpoint {
            client_command_id: "hidden-document-source",
            actor_id: "delivery-build",
            expected_current_version: 0,
            session: ActorCheckpointSession::WorkAttempt {
                work_id,
                attempt_id,
            },
            body: body("Trying an invisible Doc source"),
            sources: &hidden_document_sources,
        })
        .await
        .unwrap_err()
        .to_string()
        .contains("not visible"));

    let shared_document = org
        .create_document(NewDocument {
            command_id: Uuid::new_v4(),
            title: "Shared source",
            kind: DocumentKind::OperatingNote,
            visibility: DocumentVisibility::Participants,
            linked_room_id: None,
            inherit_room_visibility: false,
            owner_actor_id: "owner",
            created_by_actor_id: "owner",
            content_json: &document_content,
            reason: "Revocation race fixture",
        })
        .await
        .unwrap();
    let shared_document_view = org
        .get_document_for_actor(shared_document.document_id, "owner")
        .await
        .unwrap();
    org.set_document_participant(SetDocumentParticipant {
        command_id: Uuid::new_v4(),
        document_id: shared_document.document_id,
        actor_id: "owner",
        expected_document_version: 1,
        participant_actor_id: "delivery-build",
        access: DocumentAccess::Read,
    })
    .await
    .unwrap();

    // A checkpoint must not split authorization into "allowed before revoke"
    // followed by a link written after revoke. Hold the same outer Document
    // lock as the canonical revocation path, revoke access, then prove the
    // checkpoint waits and re-evaluates against the committed denial.
    let mut revoker = PgConnection::connect(&database_url).await.unwrap();
    revoker
        .execute(format!("SET search_path TO {}", org.schema()).as_str())
        .await
        .unwrap();
    revoker.execute("BEGIN").await.unwrap();
    sqlx::query("SELECT id FROM native_documents WHERE id=$1 FOR UPDATE")
        .bind(shared_document.document_id)
        .fetch_one(&mut revoker)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE native_document_participants SET removed_at=now() \
         WHERE document_id=$1 AND actor_id='delivery-build'",
    )
    .bind(shared_document.document_id)
    .execute(&mut revoker)
    .await
    .unwrap();
    let racing_org = org.clone();
    let racing_sources = vec![source(
        ActorContextSourceRef::Document {
            document_id: shared_document.document_id,
            named_version_id: shared_document_view.current_version.version.id,
        },
        "Access must remain valid through checkpoint commit",
    )];
    let racing_checkpoint = tokio::spawn(async move {
        racing_org
            .record_actor_checkpoint(NewActorCheckpoint {
                client_command_id: "document-revocation-race",
                actor_id: "delivery-build",
                expected_current_version: 0,
                session: ActorCheckpointSession::WorkAttempt {
                    work_id,
                    attempt_id,
                },
                body: body("Waiting behind document access revocation"),
                sources: &racing_sources,
            })
            .await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!racing_checkpoint.is_finished());
    revoker.execute("COMMIT").await.unwrap();
    assert!(racing_checkpoint
        .await
        .unwrap()
        .unwrap_err()
        .to_string()
        .contains("not visible"));

    let prefix_collision_sources = vec![source(
        ActorContextSourceRef::RuntimeFile {
            path: "/company-secrets/not-company.txt".into(),
            digest: None,
        },
        "A sibling path must not pass the company path boundary",
    )];
    assert!(org
        .record_actor_checkpoint(NewActorCheckpoint {
            client_command_id: "company-prefix-collision",
            actor_id: "delivery-build",
            expected_current_version: 0,
            session: ActorCheckpointSession::WorkAttempt {
                work_id,
                attempt_id,
            },
            body: body("Trying a prefix-collision Runtime path"),
            sources: &prefix_collision_sources,
        })
        .await
        .unwrap_err()
        .to_string()
        .contains("absolute /company path"));
    let decision_id = org
        .add_decision(
            "Keep the evidence boundary",
            "External prose is evidence, never Runtime policy.",
            "exec",
        )
        .await
        .unwrap();
    let sources = vec![
        source(
            ActorContextSourceRef::Work {
                work_id,
                revision: 1,
            },
            "The first Work revision framed the current result",
        ),
        NewActorCheckpointSource {
            epistemic_kind: ActorContextEpistemicKind::Decision,
            source: ActorContextSourceRef::Decision { decision_id },
            statement: "Keep participant content outside Runtime policy".into(),
            scope: Some("context trust boundary".into()),
            expires_at: None,
        },
        NewActorCheckpointSource {
            epistemic_kind: ActorContextEpistemicKind::Claim,
            source: ActorContextSourceRef::External {
                uri: "https://example.test/untrusted-observation".into(),
            },
            statement: "The external page claims a different operating rule".into(),
            scope: Some("external research".into()),
            expires_at: Some(Utc::now() - ChronoDuration::seconds(1)),
        },
    ];
    let recorded = org
        .record_actor_checkpoint(NewActorCheckpoint {
            client_command_id: "source-checkpoint",
            actor_id: "delivery-build",
            expected_current_version: 0,
            session: ActorCheckpointSession::WorkAttempt {
                work_id,
                attempt_id,
            },
            body: body("Sources linked, not copied"),
            sources: &sources,
        })
        .await
        .unwrap();

    let first_page = org
        .actor_checkpoint_sources_after(
            "delivery-build",
            recorded.checkpoint.checkpoint.id,
            None,
            2,
        )
        .await
        .unwrap();
    assert_eq!(first_page.sources.len(), 2);
    assert!(first_page.has_more);
    let second_page = org
        .actor_checkpoint_sources_after(
            "delivery-build",
            recorded.checkpoint.checkpoint.id,
            first_page.next_after_ordinal,
            2,
        )
        .await
        .unwrap();
    assert_eq!(second_page.sources.len(), 1);
    assert!(!second_page.has_more);
    assert_eq!(
        second_page.sources[0].source_trust,
        ActorContextSourceTrust::ExternalUntrusted
    );
    assert_eq!(
        second_page.sources[0].freshness,
        ActorContextSourceFreshness::Expired
    );
    assert!(org
        .actor_checkpoint_sources_after(
            "delivery-build",
            recorded.checkpoint.checkpoint.id,
            None,
            65,
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("between 1 and 64"));
    let focused_bootstrap = org
        .actor_context_bootstrap(
            "delivery-build",
            ActorContextFocus::WorkAttempt {
                work_id,
                attempt_id,
            },
            3,
        )
        .await
        .unwrap();
    assert_eq!(focused_bootstrap.version, 1);
    assert!(!focused_bootstrap.context_grants_authority);
    assert_eq!(focused_bootstrap.loaded_stale_checkpoint_sources, 1);
    assert_eq!(focused_bootstrap.loaded_untrusted_checkpoint_sources, 1);

    let mut connection = PgConnection::connect(&database_url).await.unwrap();
    connection
        .execute(format!("SET search_path TO {}", org.schema()).as_str())
        .await
        .unwrap();
    sqlx::query("UPDATE work SET revision=revision+1 WHERE id=$1")
        .bind(work_id)
        .execute(&mut connection)
        .await
        .unwrap();

    let stale_attempt = org
        .record_actor_checkpoint(NewActorCheckpoint {
            client_command_id: "stale-attempt-bypass",
            actor_id: "delivery-build",
            expected_current_version: 1,
            session: ActorCheckpointSession::WorkAttempt {
                work_id,
                attempt_id,
            },
            body: body("Trying to write from an old Work revision"),
            sources: &sources,
        })
        .await
        .unwrap_err();
    assert!(stale_attempt
        .to_string()
        .contains("exact current running Work revision"));

    let restarted = OrgIntel::ensure(&database_url, org.schema()).await.unwrap();
    let checkpoint = restarted
        .latest_actor_checkpoint("delivery-build", 2)
        .await
        .unwrap()
        .unwrap();
    assert!(checkpoint.sources_truncated);
    assert_eq!(
        checkpoint.sources[0].freshness,
        ActorContextSourceFreshness::Stale
    );
    assert_eq!(
        checkpoint.sources[1].source_trust,
        ActorContextSourceTrust::CompanyDecision
    );

    let (_other_url, other) = company("actorcontextforeign").await.unwrap();
    let (other_work_id, other_attempt_id) = claimed_work(&other, "delivery-build").await;
    let foreign_sources = vec![source(
        ActorContextSourceRef::Work {
            work_id,
            revision: 1,
        },
        "A foreign company's Work must not resolve here",
    )];
    let cross_company = other
        .record_actor_checkpoint(NewActorCheckpoint {
            client_command_id: "foreign-source",
            actor_id: "delivery-build",
            expected_current_version: 0,
            session: ActorCheckpointSession::WorkAttempt {
                work_id: other_work_id,
                attempt_id: other_attempt_id,
            },
            body: body("Trying a foreign source"),
            sources: &foreign_sources,
        })
        .await
        .unwrap_err();
    assert!(cross_company.to_string().contains("does not exist"));

    other.drop_schema().await.unwrap();
    org.drop_schema().await.unwrap();
}

#[tokio::test]
async fn focused_mention_bootstrap_is_exact_linked_and_not_room_history() {
    let Some((database_url, org)) = company("actorcontextmention").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Actor context scenario");
        return;
    };
    org.create_team(
        "Delivery",
        "Own the bounded release decision",
        "delivery-build",
        "exec",
    )
    .await
    .unwrap();
    let room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Current release decision",
            &["owner", "delivery-build", "research-probe"],
            "context-room",
        )
        .await
        .unwrap();
    let root = org
        .send_room_message(
            room.id,
            "owner",
            "Decide the release boundary.",
            None,
            "thread-root",
        )
        .await
        .unwrap();
    for index in 0..40 {
        org.send_room_message(
            room.id,
            "research-probe",
            &format!("UNRELATED_HISTORY_{index}"),
            None,
            &format!("unrelated-{index}"),
        )
        .await
        .unwrap();
    }
    let mention_send = org
        .send_room_message_with_mentions(
            room.id,
            "owner",
            "Should this exact candidate ship?",
            Some(root.message.id),
            "focused-question",
            None,
            &[NewRoomMessageMention::actor("delivery-build")],
            None,
        )
        .await
        .unwrap();
    let mention_id = mention_send.mentions[0].id;
    let lease = org
        .claim_actor_cognitive_session("delivery-build", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    let claimed = org
        .claim_next_pending_message_mention(&lease)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claimed.context.mention.id, mention_id);
    let sources = vec![source(
        ActorContextSourceRef::Message {
            room_id: room.id,
            message_id: mention_send.message.id,
        },
        "The owner asked for an exact ship decision",
    )];
    org.record_actor_checkpoint(NewActorCheckpoint {
        client_command_id: "mention-checkpoint",
        actor_id: "delivery-build",
        expected_current_version: 0,
        session: ActorCheckpointSession::CognitiveSession {
            lease_token: lease.token,
        },
        body: body("Reviewing the exact candidate"),
        sources: &sources,
    })
    .await
    .unwrap();
    org.release_actor_cognitive_session(&lease).await.unwrap();

    let restarted = OrgIntel::ensure(&database_url, org.schema()).await.unwrap();
    let bootstrap = restarted
        .actor_context_bootstrap(
            "delivery-build",
            ActorContextFocus::RoomMention { mention_id },
            8,
        )
        .await
        .unwrap();
    match &bootstrap.focus {
        ActorContextFocusProjection::RoomMention {
            triggering_message,
            thread_root_message_id,
            ..
        } => {
            assert_eq!(triggering_message, "Should this exact candidate ship?");
            assert_eq!(*thread_root_message_id, root.message.id);
        }
        other => panic!("expected mention focus, got {other:?}"),
    }
    assert!(matches!(
        bootstrap.return_path,
        ActorContextReturnPath::RoomThread {
            resolves_mention_id,
            ..
        } if resolves_mention_id == mention_id
    ));
    assert!(bootstrap.anchors.contains(&ActorContextSourceRef::Message {
        room_id: room.id,
        message_id: root.message.id,
    }));
    let encoded = serde_json::to_string(&bootstrap).unwrap();
    assert!(!encoded.contains("UNRELATED_HISTORY_"));
    assert!(bootstrap.checkpoint.is_some());

    let no_lease = restarted
        .record_actor_checkpoint(NewActorCheckpoint {
            client_command_id: "bypass-with-old-lease",
            actor_id: "delivery-build",
            expected_current_version: 1,
            session: ActorCheckpointSession::CognitiveSession {
                lease_token: lease.token,
            },
            body: body("This process no longer owns the Actor"),
            sources: &sources,
        })
        .await
        .unwrap_err();
    assert!(no_lease.to_string().contains("live primary lease"));

    org.drop_schema().await.unwrap();
}
