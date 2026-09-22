//! Durable model-invocation admission is independent of daemon memory and
//! exact across competing OrgIntel pools. These scenarios use a scratch
//! company schema when PostgreSQL is available.

use restless_orgintel::{
    ModelInvocationAdmission, ModelInvocationAdmissionDecision, ModelInvocationKind,
    ModelInvocationOutcome, ModelInvocationSettlement, ModelInvocationSource, ModelInvocationState,
    NewModelInvocationAdmission, NewRoomMessageMention, NewWork, OrgIntel, RoomKind,
    WorkAttemptState, WorkspaceSpec,
};
use sqlx::postgres::PgConnection;
use sqlx::Connection as _;
use std::time::Duration;
use uuid::Uuid;

async fn company(prefix: &str) -> Option<(String, OrgIntel)> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let name = format!("{prefix}{}_test", Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &name)
        .await
        .expect("ensure scratch invocation-admission company");
    org.ensure_actor("owner", "owner", "owner", "Owner")
        .await
        .unwrap();
    org.ensure_actor("exec", "exec", "exec", "Exec")
        .await
        .unwrap();
    org.ensure_actor("delivery-build", "staff", "builder", "Delivery Builder")
        .await
        .unwrap();
    Some((url, org))
}

async fn configure_policy(
    database_url: &str,
    org: &OrgIntel,
    company_limit: i32,
    actor_limit: i32,
) {
    let mut connection = PgConnection::connect(database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", org.schema()))
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE model_invocation_policy \
         SET company_limit=$1,actor_limit=$2,updated_at=current_timestamp",
    )
    .bind(company_limit)
    .bind(actor_limit)
    .execute(&mut connection)
    .await
    .unwrap();
}

fn admitted(decision: ModelInvocationAdmissionDecision) -> ModelInvocationAdmission {
    match decision {
        ModelInvocationAdmissionDecision::Admitted(admission) => admission,
        ModelInvocationAdmissionDecision::AlreadyAdmitted(receipt) => {
            panic!("expected admission, got replay receipt {}", receipt.id)
        }
        ModelInvocationAdmissionDecision::AlreadySettled(receipt) => {
            panic!("expected admission, got settled receipt {}", receipt.id)
        }
        ModelInvocationAdmissionDecision::Expired(receipt) => {
            panic!("expected admission, got expired receipt {}", receipt.id)
        }
        ModelInvocationAdmissionDecision::Denied { scope, budget } => panic!(
            "expected admission, got {scope:?} denial at company {}/{} actor {}/{}",
            budget.company_used, budget.company_limit, budget.actor_used, budget.actor_limit
        ),
    }
}

#[tokio::test]
async fn work_admission_and_settlement_are_exact_restart_durable_receipts() {
    let Some((database_url, org)) = company("invokeexact").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping invocation exactness scenario");
        return;
    };
    let work_id = org
        .add_work(NewWork {
            owner_id: "delivery-build",
            title: "Produce exact evidence",
            outcome: "one verified result",
            goal_id: None,
            priority: 1,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(1),
        })
        .await
        .unwrap();
    let attempt = org
        .claim_ready_work("model invocation exactness")
        .await
        .unwrap()
        .unwrap();
    let source = ModelInvocationSource::Work {
        work_id,
        attempt_id: attempt.attempt_id,
    };
    let admission = admitted(
        org.admit_model_invocation(NewModelInvocationAdmission {
            client_command_id: "work-provider-1",
            actor_id: "delivery-build",
            model: "openai/gpt-test",
            harness: "codex",
            configured_effort: "high",
            source,
        })
        .await
        .unwrap(),
    );
    let admission_id = admission.id();

    let other = OrgIntel::ensure(&database_url, org.schema()).await.unwrap();
    let replay = other
        .admit_model_invocation(NewModelInvocationAdmission {
            client_command_id: "work-provider-1",
            actor_id: "delivery-build",
            model: "openai/gpt-test",
            harness: "codex",
            configured_effort: "high",
            source,
        })
        .await
        .unwrap();
    match replay {
        ModelInvocationAdmissionDecision::AlreadyAdmitted(receipt) => {
            assert_eq!(receipt.id, admission_id)
        }
        _ => panic!("an exact launch replay must return a receipt without provider authority"),
    }
    let conflict = other
        .admit_model_invocation(NewModelInvocationAdmission {
            client_command_id: "work-provider-1",
            actor_id: "delivery-build",
            model: "anthropic/drifted-model",
            harness: "codex",
            configured_effort: "high",
            source,
        })
        .await
        .unwrap_err();
    assert!(conflict.to_string().contains("different launch semantics"));
    let harness_conflict = other
        .admit_model_invocation(NewModelInvocationAdmission {
            client_command_id: "work-provider-1",
            actor_id: "delivery-build",
            model: "openai/gpt-test",
            harness: "claude-agent",
            configured_effort: "high",
            source,
        })
        .await
        .unwrap_err();
    assert!(harness_conflict
        .to_string()
        .contains("different launch semantics"));
    let effort_conflict = other
        .admit_model_invocation(NewModelInvocationAdmission {
            client_command_id: "work-provider-1",
            actor_id: "delivery-build",
            model: "openai/gpt-test",
            harness: "codex",
            configured_effort: "xhigh",
            source,
        })
        .await
        .unwrap_err();
    assert!(effort_conflict
        .to_string()
        .contains("different launch semantics"));

    let (first_settlement, settlement_replay) = tokio::join!(
        other.settle_model_invocation(
            &admission,
            ModelInvocationSettlement {
                outcome: ModelInvocationOutcome::Completed,
                evidence: serde_json::json!({"z": 2, "a": 1}),
            },
        ),
        org.settle_model_invocation(
            &admission,
            ModelInvocationSettlement {
                outcome: ModelInvocationOutcome::Completed,
                evidence: serde_json::json!({"a": 1, "z": 2}),
            },
        )
    );
    let first_settlement = first_settlement.unwrap();
    let settlement_replay = settlement_replay.unwrap();
    assert_ne!(
        first_settlement.created, settlement_replay.created,
        "concurrent exact settlement creates one durable result"
    );
    assert_eq!(
        first_settlement.invocation.state,
        ModelInvocationState::Settled
    );
    assert_eq!(settlement_replay.invocation.id, admission_id);
    assert!(org
        .settle_model_invocation(
            &admission,
            ModelInvocationSettlement {
                outcome: ModelInvocationOutcome::Failed,
                evidence: serde_json::json!({"a": 1, "z": 2}),
            },
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("different evidence"));
    match other
        .admit_model_invocation(NewModelInvocationAdmission {
            client_command_id: "work-provider-1",
            actor_id: "delivery-build",
            model: "openai/gpt-test",
            harness: "codex",
            configured_effort: "high",
            source,
        })
        .await
        .unwrap()
    {
        ModelInvocationAdmissionDecision::AlreadySettled(receipt) => {
            assert_eq!(receipt.id, admission_id)
        }
        _ => panic!("settled launch replay must not authorize a second provider call"),
    }

    other
        .finish_work_attempt(
            attempt.attempt_id,
            WorkAttemptState::Blocked,
            "exact receipt scenario complete",
        )
        .await
        .unwrap();
    let schema = org.schema().to_string();
    drop(org);
    drop(other);
    let restarted = OrgIntel::ensure(&database_url, &schema).await.unwrap();
    let receipts = restarted.recent_model_invocations(10).await.unwrap();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].id, admission_id);
    assert_eq!(receipts[0].state, ModelInvocationState::Settled);
}

#[tokio::test]
async fn concurrent_company_and_actor_admission_are_serialized_across_pools() {
    let Some((database_url, org)) = company("invokeconcurrent").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping invocation concurrency scenario");
        return;
    };
    org.ensure_actor("research-probe", "staff", "researcher", "Research Probe")
        .await
        .unwrap();
    configure_policy(&database_url, &org, 1, 1).await;
    let delivery = org
        .claim_actor_cognitive_session("delivery-build", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    let research = org
        .claim_actor_cognitive_session("research-probe", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    let other = OrgIntel::ensure(&database_url, org.schema()).await.unwrap();
    let (first, second) = tokio::join!(
        org.admit_model_invocation(NewModelInvocationAdmission {
            client_command_id: "company-race-delivery",
            actor_id: "delivery-build",
            model: "openai/gpt-test",
            harness: "codex",
            configured_effort: "high",
            source: ModelInvocationSource::OwnerConversation {
                cognitive_lease_token: delivery.token,
            },
        }),
        other.admit_model_invocation(NewModelInvocationAdmission {
            client_command_id: "company-race-research",
            actor_id: "research-probe",
            model: "openai/gpt-test",
            harness: "codex",
            configured_effort: "high",
            source: ModelInvocationSource::OwnerConversation {
                cognitive_lease_token: research.token,
            },
        })
    );
    let decisions = [first.unwrap(), second.unwrap()];
    assert_eq!(
        decisions
            .iter()
            .filter(|decision| matches!(decision, ModelInvocationAdmissionDecision::Admitted(_)))
            .count(),
        1
    );
    assert_eq!(
        decisions
            .iter()
            .filter(|decision| matches!(
                decision,
                ModelInvocationAdmissionDecision::Denied {
                    scope: restless_orgintel::ModelInvocationLimitScope::Company,
                    ..
                }
            ))
            .count(),
        1
    );
    let budget = org
        .model_invocation_budget_snapshot("delivery-build")
        .await
        .unwrap();
    assert_eq!(budget.company_used, 1);
    org.release_actor_cognitive_session(&delivery)
        .await
        .unwrap();
    org.release_actor_cognitive_session(&research)
        .await
        .unwrap();

    let Some((actor_url, actor_org)) = company("invokeactorrace").await else {
        unreachable!()
    };
    configure_policy(&actor_url, &actor_org, 4, 1).await;
    let lease = actor_org
        .claim_actor_cognitive_session("delivery-build", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    let actor_other = OrgIntel::ensure(&actor_url, actor_org.schema())
        .await
        .unwrap();
    let source = ModelInvocationSource::OwnerConversation {
        cognitive_lease_token: lease.token,
    };
    let (first, second) = tokio::join!(
        actor_org.admit_model_invocation(NewModelInvocationAdmission {
            client_command_id: "actor-race-a",
            actor_id: "delivery-build",
            model: "openai/gpt-test",
            harness: "codex",
            configured_effort: "high",
            source,
        }),
        actor_other.admit_model_invocation(NewModelInvocationAdmission {
            client_command_id: "actor-race-b",
            actor_id: "delivery-build",
            model: "openai/gpt-test",
            harness: "codex",
            configured_effort: "high",
            source,
        })
    );
    let decisions = [first.unwrap(), second.unwrap()];
    assert_eq!(
        decisions
            .iter()
            .filter(|decision| matches!(decision, ModelInvocationAdmissionDecision::Admitted(_)))
            .count(),
        1
    );
    assert_eq!(
        decisions
            .iter()
            .filter(|decision| matches!(
                decision,
                ModelInvocationAdmissionDecision::Denied {
                    scope: restless_orgintel::ModelInvocationLimitScope::Actor,
                    ..
                }
            ))
            .count(),
        1
    );
}

#[tokio::test]
async fn repeatedly_resolved_mentions_hit_the_durable_actor_bound() {
    let Some((database_url, org)) = company("invokementions").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping repeated mention budget scenario");
        return;
    };
    configure_policy(&database_url, &org, 8, 2).await;
    org.create_team(
        "Delivery",
        "Own delivery decisions",
        "delivery-build",
        "exec",
    )
    .await
    .unwrap();
    let room = org
        .create_room(
            "owner",
            RoomKind::Group,
            "Bounded questions",
            &["owner", "delivery-build"],
            "bounded-mention-room",
        )
        .await
        .unwrap();
    for index in 0..3 {
        org.send_room_message_with_mentions(
            room.id,
            "owner",
            &format!("Question {index}"),
            None,
            &format!("bounded-question-{index}"),
            None,
            &[NewRoomMessageMention::actor("delivery-build")],
            None,
        )
        .await
        .unwrap();
    }
    for index in 0..3 {
        let lease = org
            .claim_actor_cognitive_session("delivery-build", Duration::from_secs(60))
            .await
            .unwrap()
            .unwrap();
        let mention = org
            .claim_next_pending_message_mention(&lease)
            .await
            .unwrap()
            .unwrap();
        let decision = org
            .admit_model_invocation(NewModelInvocationAdmission {
                client_command_id: &format!("mention-launch-{index}"),
                actor_id: "delivery-build",
                model: "openai/gpt-test",
                harness: "codex",
                configured_effort: "high",
                source: ModelInvocationSource::RoomMention {
                    cognitive_lease_token: lease.token,
                    mention_id: mention.context.mention.id,
                },
            })
            .await
            .unwrap();
        if index < 2 {
            let admission = admitted(decision);
            org.settle_model_invocation(
                &admission,
                ModelInvocationSettlement {
                    outcome: ModelInvocationOutcome::Completed,
                    evidence: serde_json::json!({"answer": index}),
                },
            )
            .await
            .unwrap();
            org.reply_to_claimed_message_mention(&mention, &format!("Answer {index}"))
                .await
                .unwrap();
        } else {
            assert!(matches!(
                decision,
                ModelInvocationAdmissionDecision::Denied {
                    scope: restless_orgintel::ModelInvocationLimitScope::Actor,
                    ..
                }
            ));
        }
        org.release_actor_cognitive_session(&lease).await.unwrap();
    }
    let budget = org
        .model_invocation_budget_snapshot("delivery-build")
        .await
        .unwrap();
    assert_eq!(budget.actor_used, 2);
    assert_eq!(budget.actor_remaining, 0);
    assert!(org
        .next_pending_message_mention("delivery-build")
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn expiry_is_bounded_and_only_reclaims_when_the_source_claim_is_dead() {
    let Some((database_url, org)) = company("invokeexpiry").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping invocation expiry scenario");
        return;
    };
    configure_policy(&database_url, &org, 4, 2).await;
    let lease = org
        .claim_actor_cognitive_session("delivery-build", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    let source = ModelInvocationSource::OwnerConversation {
        cognitive_lease_token: lease.token,
    };
    let first = admitted(
        org.admit_model_invocation(NewModelInvocationAdmission {
            client_command_id: "abandoned-a",
            actor_id: "delivery-build",
            model: "openai/gpt-test",
            harness: "codex",
            configured_effort: "high",
            source,
        })
        .await
        .unwrap(),
    );
    let second = admitted(
        org.admit_model_invocation(NewModelInvocationAdmission {
            client_command_id: "abandoned-b",
            actor_id: "delivery-build",
            model: "openai/gpt-test",
            harness: "codex",
            configured_effort: "high",
            source,
        })
        .await
        .unwrap(),
    );
    let mut connection = PgConnection::connect(&database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", org.schema()))
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE model_invocation_admissions \
         SET admitted_at=current_timestamp-interval '2 hours',\
             reclaim_after=current_timestamp-interval '1 hour' \
         WHERE id=ANY($1)",
    )
    .bind([first.id(), second.id()])
    .execute(&mut connection)
    .await
    .unwrap();

    assert!(org
        .reconcile_expired_model_invocations(1)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        org.model_invocation_budget_snapshot("delivery-build")
            .await
            .unwrap()
            .actor_used,
        2
    );
    org.release_actor_cognitive_session(&lease).await.unwrap();
    assert!(org
        .admit_model_invocation(NewModelInvocationAdmission {
            client_command_id: "abandoned-a",
            actor_id: "delivery-build",
            model: "openai/gpt-test",
            harness: "codex",
            configured_effort: "high",
            source,
        })
        .await
        .unwrap_err()
        .to_string()
        .contains("exact live cognitive lease"));

    let restarted = OrgIntel::ensure(&database_url, org.schema()).await.unwrap();
    let first_batch = restarted
        .reconcile_expired_model_invocations(1)
        .await
        .unwrap();
    assert_eq!(first_batch.len(), 1);
    let after_one = restarted.recent_model_invocations(10).await.unwrap();
    assert_eq!(
        after_one
            .iter()
            .filter(|receipt| receipt.state == ModelInvocationState::Admitted)
            .count(),
        1
    );
    let second_batch = restarted
        .reconcile_expired_model_invocations(1)
        .await
        .unwrap();
    assert_eq!(second_batch.len(), 1);
    assert_eq!(
        restarted
            .model_invocation_budget_snapshot("delivery-build")
            .await
            .unwrap()
            .actor_used,
        0
    );
}

#[tokio::test]
async fn bounded_reconciliation_does_not_let_an_older_live_claim_starve_dead_rows() {
    let Some((database_url, org)) = company("invokereconcilefair").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping invocation reconciliation scenario");
        return;
    };
    org.ensure_actor("research-probe", "staff", "researcher", "Research Probe")
        .await
        .unwrap();
    configure_policy(&database_url, &org, 4, 2).await;
    let live_lease = org
        .claim_actor_cognitive_session("delivery-build", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    let dead_lease = org
        .claim_actor_cognitive_session("research-probe", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    let live = admitted(
        org.admit_model_invocation(NewModelInvocationAdmission {
            client_command_id: "old-live-claim",
            actor_id: "delivery-build",
            model: "openai/gpt-test",
            harness: "codex",
            configured_effort: "high",
            source: ModelInvocationSource::OwnerConversation {
                cognitive_lease_token: live_lease.token,
            },
        })
        .await
        .unwrap(),
    );
    let dead = admitted(
        org.admit_model_invocation(NewModelInvocationAdmission {
            client_command_id: "newer-dead-claim",
            actor_id: "research-probe",
            model: "openai/gpt-test",
            harness: "codex",
            configured_effort: "high",
            source: ModelInvocationSource::OwnerConversation {
                cognitive_lease_token: dead_lease.token,
            },
        })
        .await
        .unwrap(),
    );
    let mut connection = PgConnection::connect(&database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", org.schema()))
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE model_invocation_admissions SET \
         admitted_at=current_timestamp-interval '3 hours', \
         reclaim_after=CASE WHEN id=$1 \
           THEN current_timestamp-interval '2 hours' \
           ELSE current_timestamp-interval '1 hour' END \
         WHERE id=ANY($2)",
    )
    .bind(live.id())
    .bind([live.id(), dead.id()])
    .execute(&mut connection)
    .await
    .unwrap();
    org.release_actor_cognitive_session(&dead_lease)
        .await
        .unwrap();

    let reclaimed = org.reconcile_expired_model_invocations(1).await.unwrap();
    assert_eq!(reclaimed.len(), 1);
    assert_eq!(reclaimed[0].id, dead.id());
    let receipts = org.recent_model_invocations(10).await.unwrap();
    assert_eq!(
        receipts
            .iter()
            .find(|receipt| receipt.id == live.id())
            .unwrap()
            .state,
        ModelInvocationState::Admitted
    );

    org.release_actor_cognitive_session(&live_lease)
        .await
        .unwrap();
    let final_reclaim = org.reconcile_expired_model_invocations(1).await.unwrap();
    assert_eq!(final_reclaim.len(), 1);
    assert_eq!(final_reclaim[0].id, live.id());
}

#[tokio::test]
async fn company_schemas_are_isolated_and_invalid_direct_bypass_fails_closed() {
    let Some((database_url, alpha)) = company("invokealpha").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping invocation isolation scenario");
        return;
    };
    let Some((_, beta)) = company("invokebeta").await else {
        unreachable!()
    };
    configure_policy(&database_url, &alpha, 1, 1).await;
    configure_policy(&database_url, &beta, 1, 1).await;
    let alpha_lease = alpha
        .claim_actor_cognitive_session("delivery-build", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    let beta_lease = beta
        .claim_actor_cognitive_session("delivery-build", Duration::from_secs(60))
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        alpha
            .admit_model_invocation(NewModelInvocationAdmission {
                client_command_id: "same-company-local-key",
                actor_id: "delivery-build",
                model: "openai/gpt-test",
                harness: "codex",
                configured_effort: "high",
                source: ModelInvocationSource::OwnerConversation {
                    cognitive_lease_token: alpha_lease.token,
                },
            })
            .await
            .unwrap(),
        ModelInvocationAdmissionDecision::Admitted(_)
    ));
    assert!(matches!(
        beta.admit_model_invocation(NewModelInvocationAdmission {
            client_command_id: "same-company-local-key",
            actor_id: "delivery-build",
            model: "openai/gpt-test",
            harness: "codex",
            configured_effort: "high",
            source: ModelInvocationSource::OwnerConversation {
                cognitive_lease_token: beta_lease.token,
            },
        })
        .await
        .unwrap(),
        ModelInvocationAdmissionDecision::Admitted(_)
    ));
    assert_eq!(alpha.recent_model_invocations(10).await.unwrap().len(), 1);
    assert_eq!(beta.recent_model_invocations(10).await.unwrap().len(), 1);

    let fake_source = ModelInvocationSource::OwnerConversation {
        cognitive_lease_token: Uuid::new_v4(),
    };
    assert!(alpha
        .admit_model_invocation(NewModelInvocationAdmission {
            client_command_id: "forged-runtime-permit",
            actor_id: "delivery-build",
            model: "openai/gpt-test",
            harness: "codex",
            configured_effort: "high",
            source: fake_source,
        })
        .await
        .unwrap_err()
        .to_string()
        .contains("exact live cognitive lease"));

    let mut connection = PgConnection::connect(&database_url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", alpha.schema()))
        .execute(&mut connection)
        .await
        .unwrap();
    let direct = sqlx::query(
        "INSERT INTO model_invocation_admissions \
         (id,admission_token,client_command_id,client_payload_sha256,actor_id,kind,\
          subject_id,model,window_started_at,reclaim_after) \
         VALUES ($1,$2,'direct-bypass',$3,'delivery-build',$4,$5,'openai/gpt-test',\
                 date_trunc('hour',current_timestamp),current_timestamp+interval '1 hour')",
    )
    .bind(Uuid::new_v4())
    .bind(Uuid::new_v4())
    .bind("0".repeat(64))
    .bind(ModelInvocationKind::Work)
    .bind(Uuid::new_v4())
    .execute(&mut connection)
    .await;
    assert!(
        direct.is_err(),
        "database shape constraints reject a bypass row"
    );
}
