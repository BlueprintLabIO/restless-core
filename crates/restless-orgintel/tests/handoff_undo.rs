//! Real PostgreSQL boundaries for a handoff's durable five-second Undo.
use chrono::{Duration, Utc};
use restless_orgintel::{
    CompanyAccessIdentity, ExternalMembershipControlContext, ExternalMembershipStatus,
    HandoffDecision, HumanAccessContext, HumanDecisionMembership, HumanDecisionPrincipal,
    NewArtifactRef, NewOwnerHandoff, NewWork, OrgIntel, OwnerBrief, OwnerBriefKind,
    OwnerHandoffCategory, OwnerHandoffState, OwnerReviewVersion, WorkAttemptState, WorkStatus,
    WorkspaceSpec, MEMBERSHIP_CONTROL_CONTRACT_VERSION,
};
use sqlx::PgPool;
use uuid::Uuid;

struct Fixture {
    org: OrgIntel,
    db: PgPool,
    url: String,
}
fn owner() -> HumanDecisionPrincipal {
    HumanDecisionPrincipal {
        actor_id: "owner".into(),
        owner_account: None,
        membership: None,
    }
}
impl Fixture {
    async fn new() -> Option<Self> {
        let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping live Undo scenario");
            return None;
        };
        let schema = format!("undo_{}_test", Uuid::new_v4().simple());
        let org = OrgIntel::ensure(&url, &schema).await.unwrap();
        org.ensure_actor("owner", "owner", "owner", "The Owner")
            .await
            .unwrap();
        org.ensure_actor("exec", "exec", "exec", "The Exec")
            .await
            .unwrap();
        org.create_actor(
            "undo-producer",
            "lead",
            "Avery North",
            None,
            "exec",
            "Produces the outcome",
        )
        .await
        .unwrap();
        org.create_team(
            "Undo production",
            "Produce one prepared outcome",
            "undo-producer",
            "exec",
        )
        .await
        .unwrap();
        let db = PgPool::connect(&url).await.unwrap();
        Some(Self { org, db, url })
    }
    async fn handoff(&self, review: bool) -> (Uuid, Uuid) {
        let work = self
            .org
            .add_work(NewWork {
                owner_id: "undo-producer",
                title: "Prepared outcome",
                outcome: "Deliver the exact prepared result",
                goal_id: None,
                priority: 0,
                expected_artifact: "",
                workspace: WorkspaceSpec::default(),
                attempt_limit: Some(1),
            })
            .await
            .unwrap();
        let attempt_id = if review {
            Some(
                self.org
                    .claim_ready_work("Produce the review candidate")
                    .await
                    .unwrap()
                    .unwrap()
                    .attempt_id,
            )
        } else {
            None
        };
        let handoff = self
            .org
            .request_owner_handoff(NewOwnerHandoff {
                work_id: work,
                attempt_id,
                requested_by: "undo-producer",
                category: if review {
                    OwnerHandoffCategory::OwnerJudgement
                } else {
                    OwnerHandoffCategory::Identity
                },
                requested_action: "Inspect the prepared result",
                prepared_state: "The result is ready",
                resume_condition: "The person records the exact bounded decision",
            })
            .await
            .unwrap();
        if review {
            self.org
                .prepare_owner_brief(
                    handoff,
                    "exec",
                    OwnerBrief {
                        kind: OwnerBriefKind::OutcomeReview,
                        headline: "Review the prepared outcome".into(),
                        situation: "The result is ready.".into(),
                        impact: "Acceptance completes this Work.".into(),
                        recommendation: "Inspect the exact result.".into(),
                        no_action: "The Work remains paused.".into(),
                        uncertainty: None,
                        deadline: None,
                    },
                )
                .await
                .unwrap();
            self.org
                .escalate_handoff(handoff, "exec", "The owner judges this exact outcome")
                .await
                .unwrap();
            self.artifact(work, "first.md").await;
            self.org
                .finish_work_attempt(
                    attempt_id.unwrap(),
                    WorkAttemptState::Blocked,
                    "Waiting for the exact owner review",
                )
                .await
                .unwrap();
        }
        (work, handoff)
    }
    async fn artifact(&self, work: Uuid, name: &str) {
        let attempt: Uuid = sqlx::query_scalar(&format!(
            "SELECT id FROM \"{}\".work_attempts WHERE work_id=$1 ORDER BY attempt_no DESC LIMIT 1",
            self.org.schema()
        ))
        .bind(work)
        .fetch_one(&self.db)
        .await
        .unwrap();
        self.org
            .link_work_artifact(NewArtifactRef {
                kind: "review_target",
                uri: &format!("/company/outputs/{name}"),
                note: "Prepared native outcome",
                created_by: "undo-producer",
                work_id: Some(work),
                attempt_id: Some(attempt),
                digest: None,
                source_commit: None,
                runtime_generation: None,
                label: "Prepared result",
            })
            .await
            .unwrap();
    }
    async fn version(&self, work: Uuid, handoff: Uuid) -> OwnerReviewVersion {
        let graph = self.org.work_graph_snapshot().await.unwrap();
        OwnerReviewVersion {
            handoff: graph
                .handoffs
                .iter()
                .find(|h| h.id == handoff)
                .unwrap()
                .conversation_input(),
            work_revision: graph.work.iter().find(|w| w.id == work).unwrap().revision,
            artifact_id: graph
                .artifacts
                .iter()
                .filter(|a| {
                    a.work_id == Some(work)
                        && a.kind == "review_target"
                        && a.state == restless_orgintel::ArtifactRefState::Available
                })
                .max_by_key(|a| (a.created_at, a.id))
                .map(|a| a.id),
        }
    }
    async fn due(&self, id: Uuid) {
        sqlx::query(&format!("UPDATE \"{}\".owner_handoff_decisions SET undo_until=clock_timestamp()-INTERVAL '1 second' WHERE id=$1",self.org.schema()))
            .bind(id).execute(&self.db).await.unwrap();
    }
    async fn feedback_count(&self, work: Uuid) -> i64 {
        sqlx::query_scalar(&format!(
            "SELECT count(*) FROM \"{}\".work_feedback WHERE work_id=$1",
            self.org.schema()
        ))
        .bind(work)
        .fetch_one(&self.db)
        .await
        .unwrap()
    }
    async fn drop(self) {
        self.org.drop_schema().await.unwrap();
        self.db.close().await;
    }
}

#[tokio::test]
async fn deadline_survives_reopen_and_two_workers_apply_exactly_once() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let (work, handoff) = f.handoff(false).await;
    let version = f.version(work, handoff).await;
    let id = Uuid::new_v4();
    let saved = f
        .org
        .queue_handoff_decision(id, &owner(), &version, &HandoffDecision::CompleteHumanStep)
        .await
        .unwrap();
    assert!(
        f.org.apply_handoff_decision(id).await.is_err(),
        "no mutation during Undo"
    );
    assert!(f.org.due_handoff_decisions().await.unwrap().is_empty());
    assert_eq!(f.feedback_count(work).await, 0);
    let reopened = OrgIntel::ensure(&f.url, f.org.schema()).await.unwrap();
    let replay = reopened
        .queue_handoff_decision(id, &owner(), &version, &HandoffDecision::CompleteHumanStep)
        .await
        .unwrap();
    let other_id = Uuid::new_v4();
    let other = reopened
        .queue_handoff_decision(
            other_id,
            &owner(),
            &version,
            &HandoffDecision::CompleteHumanStep,
        )
        .await
        .unwrap();
    assert_eq!(replay.undo_until, saved.undo_until);
    assert_eq!(
        (other.id, other.undo_until),
        (id, saved.undo_until),
        "another tab cannot restart the window"
    );
    let delay = (saved.undo_until - Utc::now()).to_std().unwrap_or_default()
        + std::time::Duration::from_millis(30);
    tokio::time::sleep(delay).await;
    let (a, b) = tokio::join!(
        reopened.apply_handoff_decision(id),
        f.org.apply_handoff_decision(id)
    );
    assert_eq!(
        [a.unwrap(), b.unwrap()].into_iter().filter(|x| *x).count(),
        1
    );
    assert_eq!(f.feedback_count(work).await, 1);
    let graph = f.org.work_graph_snapshot().await.unwrap();
    assert_eq!(
        graph
            .handoffs
            .iter()
            .find(|h| h.id == handoff)
            .unwrap()
            .state,
        OwnerHandoffState::Resolved
    );
    assert_eq!(
        graph.work.iter().find(|w| w.id == work).unwrap().status,
        WorkStatus::Active
    );
    let replay = reopened
        .queue_handoff_decision(
            other_id,
            &owner(),
            &version,
            &HandoffDecision::CompleteHumanStep,
        )
        .await
        .unwrap();
    assert_eq!(
        (replay.id, replay.state.as_str(), replay.undo_until),
        (id, "applied", saved.undo_until)
    );
    assert!(!f.org.apply_handoff_decision(id).await.unwrap());
    assert_eq!(
        f.org
            .queue_handoff_decision(id, &owner(), &version, &HandoffDecision::CompleteHumanStep)
            .await
            .unwrap()
            .state,
        "applied"
    );
    assert!(f
        .org
        .handoff_decisions_for_actor("owner")
        .await
        .unwrap()
        .iter()
        .any(|receipt| receipt.id == id));
    assert!(
        f.org
            .acknowledge_handoff_decision(
                id,
                &HumanDecisionPrincipal {
                    owner_account: Some("another-account".into()),
                    ..owner()
                }
            )
            .await
            .is_err(),
        "an owner alias from another account cannot acknowledge this result"
    );
    f.org
        .acknowledge_handoff_decision(id, &owner())
        .await
        .unwrap();
    assert!(!f
        .org
        .handoff_decisions_for_actor("owner")
        .await
        .unwrap()
        .iter()
        .any(|receipt| receipt.id == id));
    drop(reopened);
    f.drop().await;
}

#[tokio::test]
async fn undo_cancels_without_effect_and_old_request_ids_never_create_a_new_decision() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let (work, handoff) = f.handoff(false).await;
    let version = f.version(work, handoff).await;
    let id = Uuid::new_v4();
    f.org
        .queue_handoff_decision(id, &owner(), &version, &HandoffDecision::CompleteHumanStep)
        .await
        .unwrap();
    let other_id = Uuid::new_v4();
    assert_eq!(
        f.org
            .queue_handoff_decision(
                other_id,
                &owner(),
                &version,
                &HandoffDecision::CompleteHumanStep
            )
            .await
            .unwrap()
            .id,
        id
    );
    assert!(f
        .org
        .queue_handoff_decision(Uuid::new_v4(), &owner(), &version, &HandoffDecision::Skip)
        .await
        .is_err());
    assert!(
        f.org
            .acknowledge_handoff_decision(id, &owner())
            .await
            .is_err(),
        "a pending decision cannot be acknowledged away"
    );
    let impostor = HumanDecisionPrincipal {
        owner_account: Some("another-owner".into()),
        ..owner()
    };
    assert!(f.org.cancel_handoff_decision(id, &impostor).await.is_err());
    assert_eq!(
        f.org
            .cancel_handoff_decision(id, &owner())
            .await
            .unwrap()
            .state,
        "canceled"
    );
    let next = f
        .org
        .queue_handoff_decision(Uuid::new_v4(), &owner(), &version, &HandoffDecision::Skip)
        .await
        .unwrap();
    let replay = f
        .org
        .queue_handoff_decision(id, &owner(), &version, &HandoffDecision::CompleteHumanStep)
        .await
        .unwrap();
    assert_eq!(replay.state, "canceled");
    let other_replay = f
        .org
        .queue_handoff_decision(
            other_id,
            &owner(),
            &version,
            &HandoffDecision::CompleteHumanStep,
        )
        .await
        .unwrap();
    assert_eq!(
        (
            other_replay.id,
            other_replay.state.as_str(),
            other_replay.undo_until
        ),
        (id, "canceled", replay.undo_until)
    );
    assert_ne!(next.id, replay.id);
    assert!(!f.org.apply_handoff_decision(id).await.unwrap());
    assert_eq!(f.feedback_count(work).await, 0);
    f.org
        .cancel_handoff_decision(next.id, &owner())
        .await
        .unwrap();
    f.drop().await;
}

#[tokio::test]
async fn acceptance_rechecks_the_displayed_candidate_then_accepts_only_a_new_review() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let (work, handoff) = f.handoff(true).await;
    let version = f.version(work, handoff).await;
    let action = HandoffDecision::AcceptReview {
        feedback: "".into(),
    };
    let saved = f
        .org
        .queue_handoff_decision(Uuid::new_v4(), &owner(), &version, &action)
        .await
        .unwrap();
    f.artifact(work, "replacement.md").await;
    f.due(saved.id).await;
    let error = f.org.apply_handoff_decision(saved.id).await.unwrap_err();
    assert!(matches!(
        error,
        restless_orgintel::OrgIntelError::ReviewChanged
    ));
    assert_eq!(f.feedback_count(work).await, 0);
    assert_eq!(
        f.org
            .work_graph_snapshot()
            .await
            .unwrap()
            .work
            .iter()
            .find(|w| w.id == work)
            .unwrap()
            .status,
        WorkStatus::Blocked
    );
    f.org
        .fail_handoff_decision(saved.id, &error.plain_message())
        .await
        .unwrap();
    let current = f.version(work, handoff).await;
    assert_ne!(current.artifact_id, version.artifact_id);
    let next = f
        .org
        .queue_handoff_decision(Uuid::new_v4(), &owner(), &current, &action)
        .await
        .unwrap();
    f.due(next.id).await;
    assert!(f.org.apply_handoff_decision(next.id).await.unwrap());
    assert_eq!(
        f.org
            .work_graph_snapshot()
            .await
            .unwrap()
            .work
            .iter()
            .find(|w| w.id == work)
            .unwrap()
            .status,
        WorkStatus::Completed
    );
    assert!(
        f.org
            .cancel_handoff_decision(next.id, &owner())
            .await
            .is_err(),
        "a completed effect cannot be undone"
    );
    f.drop().await;
}

#[tokio::test]
async fn assigned_member_and_admin_recheck_membership_before_the_delayed_effect() {
    for role in ["member", "admin"] {
        let Some(f) = Fixture::new().await else {
            return;
        };
        let company_id = Uuid::new_v4();
        let cell_id = Uuid::new_v4();
        f.org
            .ensure_company_access_identity(CompanyAccessIdentity {
                company_id,
                cell_id,
            })
            .await
            .unwrap();
        let now = Utc::now();
        let binding = f
            .org
            .consume_human_access_context(HumanAccessContext {
                display_name: Some("Casey"),
                issuer: "https://identity.restless.test",
                subject: "casey",
                company_id,
                cell_id,
                membership_id: "membership-casey",
                membership_role: role,
                membership_version: 1,
                assertion_id: Uuid::new_v4(),
                issued_at: now,
                expires_at: now + Duration::minutes(1),
            })
            .await
            .unwrap();
        let (work, handoff) = f.handoff(false).await;
        f.org
            .delegate_handoff(
                handoff,
                &binding.actor_id,
                "Complete this bounded personal step",
            )
            .await
            .unwrap();
        let principal = HumanDecisionPrincipal {
            actor_id: binding.actor_id,
            owner_account: None,
            membership: Some(HumanDecisionMembership {
                membership_id: "membership-casey".into(),
                membership_version: 1,
                membership_role: role.into(),
            }),
        };
        let version = f.version(work, handoff).await;
        let saved = f
            .org
            .queue_handoff_decision(
                Uuid::new_v4(),
                &principal,
                &version,
                &HandoffDecision::CompleteHumanStep,
            )
            .await
            .unwrap();
        let status = if role == "member" {
            ExternalMembershipStatus::Removed
        } else {
            ExternalMembershipStatus::Active
        };
        let new_role = if role == "admin" { "member" } else { role };
        f.org
            .apply_external_membership_control(ExternalMembershipControlContext {
                issuer: "https://identity.restless.test",
                subject: "casey",
                assertion_id: Uuid::new_v4(),
                issued_at: Utc::now(),
                expires_at: Utc::now() + Duration::seconds(45),
                key_id: "control-test",
                assertion_version: MEMBERSHIP_CONTROL_CONTRACT_VERSION,
                owner_id: Uuid::new_v4(),
                plane_id: Uuid::new_v4(),
                plane_hostname: "plane.restless.test",
                company_id,
                cell_id,
                membership_id: "membership-casey",
                membership_role: new_role,
                membership_status: status,
                membership_version: 2,
            })
            .await
            .unwrap();
        f.due(saved.id).await;
        let error = f.org.apply_handoff_decision(saved.id).await.unwrap_err();
        assert!(error.plain_message().contains("access changed"));
        assert_eq!(f.feedback_count(work).await, 0);
        f.org
            .fail_handoff_decision(saved.id, &error.plain_message())
            .await
            .unwrap();
        if role == "admin" {
            let current = HumanDecisionPrincipal {
                membership: Some(HumanDecisionMembership {
                    membership_role: "member".into(),
                    membership_version: 2,
                    membership_id: "membership-casey".into(),
                }),
                ..principal
            };
            let next = f
                .org
                .queue_handoff_decision(
                    Uuid::new_v4(),
                    &current,
                    &version,
                    &HandoffDecision::CompleteHumanStep,
                )
                .await
                .unwrap();
            f.due(next.id).await;
            assert!(
                f.org.apply_handoff_decision(next.id).await.unwrap(),
                "a current allowed membership can still act"
            );
        }
        f.drop().await;
    }
}

#[tokio::test]
async fn a_company_owner_keeps_the_verified_actor_and_loses_owner_decisions_on_demotion() {
    for first_role in ["admin", "owner"] {
        let Some(f) = Fixture::new().await else {
            return;
        };
        let company_id = Uuid::new_v4();
        let cell_id = Uuid::new_v4();
        f.org
            .ensure_company_access_identity(CompanyAccessIdentity {
                company_id,
                cell_id,
            })
            .await
            .unwrap();
        let now = Utc::now();
        let binding = f
            .org
            .consume_human_access_context(HumanAccessContext {
                display_name: Some("Casey"),
                issuer: "https://identity.restless.test",
                subject: "casey-owner",
                company_id,
                cell_id,
                membership_id: "membership-owner",
                membership_role: first_role,
                membership_version: 1,
                assertion_id: Uuid::new_v4(),
                issued_at: now,
                expires_at: now + Duration::minutes(1),
            })
            .await
            .unwrap();
        let owner_version = if first_role == "admin" { 2 } else { 1 };
        let control = |role, status, version| ExternalMembershipControlContext {
            issuer: "https://identity.restless.test",
            subject: "casey-owner",
            assertion_id: Uuid::new_v4(),
            issued_at: Utc::now(),
            expires_at: Utc::now() + Duration::seconds(45),
            key_id: "control-test",
            assertion_version: MEMBERSHIP_CONTROL_CONTRACT_VERSION,
            owner_id: Uuid::new_v4(),
            plane_id: Uuid::new_v4(),
            plane_hostname: "plane.restless.test",
            company_id,
            cell_id,
            membership_id: "membership-owner",
            membership_role: role,
            membership_status: status,
            membership_version: version,
        };
        if first_role == "admin" {
            f.org
                .apply_external_membership_control(control(
                    "owner",
                    ExternalMembershipStatus::Active,
                    owner_version,
                ))
                .await
                .unwrap();
        }
        let principal = HumanDecisionPrincipal {
            actor_id: binding.actor_id,
            owner_account: None,
            membership: Some(HumanDecisionMembership {
                membership_id: "membership-owner".into(),
                membership_version: owner_version,
                membership_role: "owner".into(),
            }),
        };
        let (work, handoff) = f.handoff(false).await;
        let version = f.version(work, handoff).await;
        let saved = f
            .org
            .queue_handoff_decision(
                Uuid::new_v4(),
                &principal,
                &version,
                &HandoffDecision::CompleteHumanStep,
            )
            .await
            .unwrap();
        f.due(saved.id).await;
        assert!(f.org.apply_handoff_decision(saved.id).await.unwrap());
        assert_eq!(
            f.feedback_count(work).await,
            1,
            "a current owner succeeds even when their verified Actor is not the owner alias"
        );
        let (work, handoff) = f.handoff(false).await;
        let version = f.version(work, handoff).await;
        let saved = f
            .org
            .queue_handoff_decision(
                Uuid::new_v4(),
                &principal,
                &version,
                &HandoffDecision::CompleteHumanStep,
            )
            .await
            .unwrap();
        f.org
            .apply_external_membership_control(control(
                "member",
                ExternalMembershipStatus::Active,
                owner_version + 1,
            ))
            .await
            .unwrap();
        f.due(saved.id).await;
        let error = f.org.apply_handoff_decision(saved.id).await.unwrap_err();
        f.org
            .fail_handoff_decision(saved.id, &error.plain_message())
            .await
            .unwrap();
        let current = HumanDecisionPrincipal {
            membership: Some(HumanDecisionMembership {
                membership_id: "membership-owner".into(),
                membership_version: owner_version + 1,
                membership_role: "member".into(),
            }),
            ..principal
        };
        assert!(f
            .org
            .queue_handoff_decision(
                Uuid::new_v4(),
                &current,
                &version,
                &HandoffDecision::CompleteHumanStep
            )
            .await
            .is_err());
        assert_eq!(
            f.feedback_count(work).await,
            0,
            "the old owner alias must not retain owner permission"
        );
        f.drop().await;
    }
}

#[tokio::test]
async fn committed_decision_changes_emit_body_free_hints_for_other_clients() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let (work, handoff) = f.handoff(false).await;
    let version = f.version(work, handoff).await;
    let mut listener = sqlx::postgres::PgListener::connect(&f.url).await.unwrap();
    listener.listen(OrgIntel::NOTIFY_CHANNEL).await.unwrap();
    let id = Uuid::new_v4();
    f.org
        .queue_handoff_decision(id, &owner(), &version, &HandoffDecision::CompleteHumanStep)
        .await
        .unwrap();
    async fn next_hint(listener: &mut sqlx::postgres::PgListener, company: &str) {
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let hint = listener.recv().await.unwrap();
                let value: serde_json::Value = serde_json::from_str(hint.payload()).unwrap();
                if value["company"] == company && value["kind"] == "handoff_decision_changed" {
                    assert_eq!(value["body"], serde_json::json!({}));
                    return;
                }
            }
        })
        .await
        .expect("Committed decision must refresh other open clients");
    }
    next_hint(&mut listener, f.org.schema()).await;
    f.org.cancel_handoff_decision(id, &owner()).await.unwrap();
    next_hint(&mut listener, f.org.schema()).await;
    let id = Uuid::new_v4();
    f.org
        .queue_handoff_decision(id, &owner(), &version, &HandoffDecision::CompleteHumanStep)
        .await
        .unwrap();
    next_hint(&mut listener, f.org.schema()).await;
    f.org
        .fail_handoff_decision(id, "Changed while pending")
        .await
        .unwrap();
    next_hint(&mut listener, f.org.schema()).await;
    f.drop().await;
}
