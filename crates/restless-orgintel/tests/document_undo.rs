//! Native Document Undo is an actual PostgreSQL decision boundary, including the review note.
use chrono::{Duration, Utc};
use restless_orgintel::*;
use serde_json::json;
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
fn body(text: &str) -> serde_json::Value {
    json!({"type":"doc","content":[{"type":"paragraph","attrs":{"block_id":Uuid::new_v4().to_string()},"content":[{"type":"text","text":text}]}]})
}
impl Fixture {
    async fn new() -> Option<Self> {
        let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping live native Undo scenario");
            return None;
        };
        assert!(
            !url.trim().is_empty(),
            "A real isolated PostgreSQL URL is required"
        );
        let org = OrgIntel::ensure(
            &url,
            &format!("native_undo_{}_test", Uuid::new_v4().simple()),
        )
        .await
        .unwrap();
        org.ensure_actor("owner", "owner", "owner", "Owner")
            .await
            .unwrap();
        org.ensure_actor("exec", "exec", "exec", "Exec")
            .await
            .unwrap();
        let db = PgPool::connect(&url).await.unwrap();
        Some(Self { org, db, url })
    }
    async fn document(&self) -> Uuid {
        self.org
            .create_document(NewDocument {
                command_id: Uuid::new_v4(),
                title: "Native Undo review",
                kind: DocumentKind::Review,
                visibility: DocumentVisibility::Participants,
                linked_room_id: None,
                inherit_room_visibility: false,
                owner_actor_id: "owner",
                created_by_actor_id: "owner",
                content_json: &body("The exact prepared document"),
                reason: "A disposable review",
            })
            .await
            .unwrap()
            .document_id
    }
    async fn review(&self, document: Uuid, reviewer: Option<&str>) -> (Uuid, DocumentDecision) {
        let view = self
            .org
            .get_document_for_actor(document, "owner")
            .await
            .unwrap();
        let review = self
            .org
            .request_document_review(RequestDocumentReview {
                document_id: document,
                actor_id: "owner",
                command_id: Uuid::new_v4(),
                expected_document_version: view.document.version,
                expected_current_version_id: view.current_version.version.id,
                summary: "Judge this exact saved document",
                reviewer_actor_id: reviewer,
                work_dependency: None,
            })
            .await
            .unwrap();
        let view = self
            .org
            .get_document_for_actor(document, "owner")
            .await
            .unwrap();
        (
            review.review.id,
            DocumentDecision::AcceptReview {
                expected_document_version: view.document.version,
                expected_review_version: review.review.version,
                accepted_version_name: "Reviewed exact version".into(),
                feedback: "The saved review note".into(),
            },
        )
    }
    async fn due(&self, id: Uuid) {
        sqlx::query(&format!("UPDATE \"{}\".owner_document_decisions SET undo_until=clock_timestamp()-INTERVAL '1 second' WHERE id=$1",self.org.schema())).bind(id).execute(&self.db).await.unwrap();
    }
    async fn comments(&self, document: Uuid) -> i64 {
        sqlx::query_scalar(&format!(
            "SELECT count(*) FROM \"{}\".native_document_comments WHERE document_id=$1",
            self.org.schema()
        ))
        .bind(document)
        .fetch_one(&self.db)
        .await
        .unwrap()
    }
    async fn versions(&self, document: Uuid) -> i64 {
        sqlx::query_scalar(&format!(
            "SELECT count(*) FROM \"{}\".native_document_versions WHERE document_id=$1",
            self.org.schema()
        ))
        .bind(document)
        .fetch_one(&self.db)
        .await
        .unwrap()
    }
    async fn drop(self) {
        self.org.drop_schema().await.unwrap();
        self.db.close().await;
    }
    async fn member(&self) -> HumanDecisionPrincipal {
        self.org
            .ensure_actor("native-alice", "human", "admin", "Alice")
            .await
            .unwrap();
        sqlx::query(&format!("INSERT INTO \"{}\".human_principal_actor_bindings(issuer,subject,company_id,actor_id,membership_id,membership_role,membership_version,last_asserted_at) VALUES('https://native.test','alice',$1,'native-alice','native-membership','admin',1,now())",self.org.schema())).bind(Uuid::new_v4()).execute(&self.db).await.unwrap();
        HumanDecisionPrincipal {
            actor_id: "native-alice".into(),
            owner_account: None,
            membership: Some(HumanDecisionMembership {
                membership_id: "native-membership".into(),
                membership_version: 1,
                membership_role: "admin".into(),
            }),
        }
    }
    async fn share(&self, document: Uuid, actor: &str) {
        let view = self
            .org
            .get_document_for_actor(document, "owner")
            .await
            .unwrap();
        self.org
            .set_document_participant(SetDocumentParticipant {
                command_id: Uuid::new_v4(),
                document_id: document,
                actor_id: "owner",
                expected_document_version: view.document.version,
                participant_actor_id: actor,
                access: DocumentAccess::Edit,
            })
            .await
            .unwrap();
    }
}
#[tokio::test]
async fn review_note_and_acceptance_wait_for_original_deadline_and_commit_once_after_reopen() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let doc = f.document().await;
    let (review, decision) = f.review(doc, None).await;
    let id = Uuid::new_v4();
    let saved = f
        .org
        .queue_document_decision(id, doc, review, &owner(), &decision)
        .await
        .unwrap();
    assert_eq!(saved.state, "pending");
    assert!(saved.undo_until > Utc::now() + Duration::seconds(3));
    assert!(f.org.apply_document_decision(id).await.is_err());
    assert_eq!(f.comments(doc).await, 0);
    assert_eq!(f.versions(doc).await, 1);
    let reopened = OrgIntel::ensure(&f.url, f.org.schema()).await.unwrap();
    let replay = reopened
        .queue_document_decision(id, doc, review, &owner(), &decision)
        .await
        .unwrap();
    assert_eq!(replay.undo_until, saved.undo_until);
    let other_id = Uuid::new_v4();
    let other = reopened
        .queue_document_decision(other_id, doc, review, &owner(), &decision)
        .await
        .unwrap();
    assert_eq!(other.id, id);
    assert_eq!(other.undo_until, saved.undo_until);
    let remaining = (saved.undo_until - Utc::now()).to_std().unwrap_or_default();
    tokio::time::sleep(remaining + std::time::Duration::from_millis(100)).await;
    let (a, b) = tokio::join!(
        f.org.apply_document_decision(id),
        reopened.apply_document_decision(id)
    );
    assert_eq!(usize::from(a.unwrap()) + usize::from(b.unwrap()), 1);
    assert_eq!(f.comments(doc).await, 1);
    assert_eq!(f.versions(doc).await, 2);
    assert_eq!(
        f.org
            .get_document_for_actor(doc, "owner")
            .await
            .unwrap()
            .document
            .status,
        DocumentStatus::Accepted
    );
    assert!(
        f.org
            .latest_document_decision_draft(doc, review, &owner())
            .await
            .unwrap()
            .is_none(),
        "An applied decision cannot resurrect an old form draft"
    );
    assert!(!f.org.apply_document_decision(id).await.unwrap());
    assert_eq!(f.comments(doc).await, 1);
    let replay = reopened
        .queue_document_decision(other_id, doc, review, &owner(), &decision)
        .await
        .unwrap();
    assert_eq!(
        (replay.id, replay.state.as_str(), replay.undo_until),
        (id, "applied", saved.undo_until)
    );
    assert!(f.org.cancel_document_decision(id, &owner()).await.is_err());
    f.drop().await;
}
#[tokio::test]
async fn undo_keeps_the_document_request_and_its_note_unpublished() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let doc = f.document().await;
    let (review, decision) = f.review(doc, None).await;
    let id = Uuid::new_v4();
    let saved = f
        .org
        .queue_document_decision(id, doc, review, &owner(), &decision)
        .await
        .unwrap();
    let other_id = Uuid::new_v4();
    assert_eq!(
        f.org
            .queue_document_decision(other_id, doc, review, &owner(), &decision)
            .await
            .unwrap()
            .id,
        id
    );
    let impostor = HumanDecisionPrincipal {
        owner_account: Some("another-account".into()),
        ..owner()
    };
    assert!(f.org.cancel_document_decision(id, &impostor).await.is_err());
    assert!(f
        .org
        .acknowledge_document_decision(id, &owner())
        .await
        .is_err());
    let canceled = f.org.cancel_document_decision(id, &owner()).await.unwrap();
    assert_eq!(canceled.state, "canceled");
    let replay = f
        .org
        .queue_document_decision(id, doc, review, &owner(), &decision)
        .await
        .unwrap();
    assert_eq!(replay.state, "canceled");
    assert_eq!(replay.undo_until, saved.undo_until);
    assert!(!f.org.apply_document_decision(id).await.unwrap());
    assert_eq!(f.comments(doc).await, 0);
    assert_eq!(
        f.org
            .get_document_for_actor(doc, "owner")
            .await
            .unwrap()
            .document
            .status,
        DocumentStatus::InReview
    );
    assert!(f
        .org
        .acknowledge_document_decision(id, &impostor)
        .await
        .is_err());
    f.org
        .acknowledge_document_decision(id, &owner())
        .await
        .unwrap();
    assert!(!f
        .org
        .document_decisions_for_actor("owner")
        .await
        .unwrap()
        .iter()
        .any(|row| row.id == id));
    let recovered = f
        .org
        .latest_document_decision_draft(doc, review, &owner())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        recovered.decision, decision,
        "Acknowledging Undo must not discard the saved form input"
    );
    assert!(
        f.org
            .latest_document_decision_draft(doc, review, &impostor)
            .await
            .unwrap()
            .is_none(),
        "An owner alias from another account cannot recover this saved draft"
    );
    let fresh = f
        .org
        .queue_document_decision(Uuid::new_v4(), doc, review, &owner(), &decision)
        .await
        .unwrap();
    assert_ne!(fresh.id, id);
    let replay = f
        .org
        .queue_document_decision(other_id, doc, review, &owner(), &decision)
        .await
        .unwrap();
    assert_eq!(
        (replay.id, replay.state.as_str(), replay.undo_until),
        (id, "canceled", saved.undo_until)
    );
    f.org
        .cancel_document_decision(fresh.id, &owner())
        .await
        .unwrap();
    f.drop().await;
}
#[tokio::test]
async fn a_new_named_version_refuses_the_old_intent_without_posting_its_note() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let doc = f.document().await;
    let (review, decision) = f.review(doc, None).await;
    let id = Uuid::new_v4();
    f.org
        .queue_document_decision(id, doc, review, &owner(), &decision)
        .await
        .unwrap();
    let before = f.org.get_document_for_actor(doc, "owner").await.unwrap();
    let newer = f
        .org
        .create_named_document_version(NewNamedDocumentVersion {
            command_id: Uuid::new_v4(),
            document_id: doc,
            actor_id: "owner",
            expected_current_version_id: before.current_version.version.id,
            content_json: &body("A later candidate the person has not accepted"),
            reason: "New source candidate",
        })
        .await
        .unwrap();
    f.due(id).await;
    assert!(f.org.apply_document_decision(id).await.is_err());
    f.org
        .fail_document_decision(id, "The candidate changed")
        .await
        .unwrap();
    assert_eq!(f.comments(doc).await, 0);
    let now = f.org.get_document_for_actor(doc, "owner").await.unwrap();
    assert_eq!(now.current_version.version.id, newer.result_id);
    assert_ne!(now.document.status, DocumentStatus::Accepted);
    let rows = f.org.document_decisions_for_actor("owner").await.unwrap();
    assert!(rows
        .iter()
        .any(|row| row.id == id && row.state == "failed" && row.seen_at.is_none()));
    f.drop().await;
}
#[tokio::test]
async fn assigned_human_access_and_membership_are_checked_again_at_acceptance() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let alice = f.member().await;
    let doc = f.document().await;
    f.share(doc, &alice.actor_id).await;
    let (review, decision) = f.review(doc, Some(&alice.actor_id)).await;
    assert!(
        f.org
            .queue_document_decision(Uuid::new_v4(), doc, review, &owner(), &decision)
            .await
            .is_err(),
        "another editor cannot accept the named person's review"
    );
    let id = Uuid::new_v4();
    f.org
        .queue_document_decision(id, doc, review, &alice, &decision)
        .await
        .unwrap();
    sqlx::query(&format!("UPDATE \"{}\".human_principal_actor_bindings SET membership_role='member',membership_version=2 WHERE actor_id=$1",f.org.schema())).bind(&alice.actor_id).execute(&f.db).await.unwrap();
    f.due(id).await;
    assert!(f.org.apply_document_decision(id).await.is_err());
    assert_eq!(f.comments(doc).await, 0);
    assert_eq!(
        f.org
            .get_document_for_actor(doc, "owner")
            .await
            .unwrap()
            .document
            .status,
        DocumentStatus::InReview
    );
    f.org
        .fail_document_decision(id, "Your role changed")
        .await
        .unwrap();
    let current = HumanDecisionPrincipal {
        membership: Some(HumanDecisionMembership {
            membership_version: 2,
            membership_role: "member".into(),
            ..alice.membership.unwrap()
        }),
        ..alice
    };
    let id = Uuid::new_v4();
    f.org
        .queue_document_decision(id, doc, review, &current, &decision)
        .await
        .unwrap();
    f.due(id).await;
    assert!(f.org.apply_document_decision(id).await.unwrap());
    assert_eq!(f.comments(doc).await, 1);
    f.drop().await;
}
#[tokio::test]
async fn a_failed_document_commit_rolls_back_note_version_and_review_together() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let doc = f.document().await;
    let (review, decision) = f.review(doc, None).await;
    let id = Uuid::new_v4();
    f.org
        .queue_document_decision(id, doc, review, &owner(), &decision)
        .await
        .unwrap();
    f.due(id).await;
    sqlx::raw_sql(&format!("CREATE FUNCTION \"{0}\".refuse_native_undo_accept() RETURNS trigger AS $$ BEGIN IF NEW.kind='document.review.accepted.v1' THEN RAISE EXCEPTION 'isolated acceptance failure'; END IF; RETURN NEW; END; $$ LANGUAGE plpgsql; CREATE TRIGGER refuse_native_undo_accept BEFORE INSERT ON \"{0}\".events FOR EACH ROW EXECUTE FUNCTION \"{0}\".refuse_native_undo_accept();",f.org.schema())).execute(&f.db).await.unwrap();
    assert!(f.org.apply_document_decision(id).await.is_err());
    assert_eq!(f.comments(doc).await, 0);
    assert_eq!(f.versions(doc).await, 1);
    assert_eq!(
        f.org
            .get_document_for_actor(doc, "owner")
            .await
            .unwrap()
            .document
            .status,
        DocumentStatus::InReview
    );
    f.org
        .fail_document_decision(id, "Acceptance could not be recorded")
        .await
        .unwrap();
    sqlx::raw_sql(&format!("DROP TRIGGER refuse_native_undo_accept ON \"{0}\".events; DROP FUNCTION \"{0}\".refuse_native_undo_accept();",f.org.schema())).execute(&f.db).await.unwrap();
    let fresh = Uuid::new_v4();
    f.org
        .queue_document_decision(fresh, doc, review, &owner(), &decision)
        .await
        .unwrap();
    f.due(fresh).await;
    assert!(f.org.apply_document_decision(fresh).await.unwrap());
    assert_eq!(f.comments(doc).await, 1);
    assert_eq!(f.versions(doc).await, 2);
    f.drop().await;
}
#[tokio::test]
async fn collaboration_completion_is_recoverable_cancelable_and_once_only() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let doc = f.document().await;
    let request = f
        .org
        .request_document_collaboration(doc, "owner", Uuid::new_v4(), "Complete this prepared edit")
        .await
        .unwrap();
    let view = f.org.get_document_for_actor(doc, "owner").await.unwrap();
    let decision = DocumentDecision::CompleteCollaboration {
        expected_document_version: view.document.version,
        expected_summary: request.summary.clone(),
    };
    let canceled = Uuid::new_v4();
    f.org
        .queue_document_decision(canceled, doc, request.id, &owner(), &decision)
        .await
        .unwrap();
    f.org
        .cancel_document_decision(canceled, &owner())
        .await
        .unwrap();
    assert!(!f.org.apply_document_decision(canceled).await.unwrap());
    let id = Uuid::new_v4();
    let saved = f
        .org
        .queue_document_decision(id, doc, request.id, &owner(), &decision)
        .await
        .unwrap();
    assert!(f.org.apply_document_decision(id).await.is_err());
    let reopened = OrgIntel::ensure(&f.url, f.org.schema()).await.unwrap();
    let replay = reopened
        .queue_document_decision(id, doc, request.id, &owner(), &decision)
        .await
        .unwrap();
    assert_eq!(replay.undo_until, saved.undo_until);
    f.due(id).await;
    let (a, b) = tokio::join!(
        f.org.apply_document_decision(id),
        reopened.apply_document_decision(id)
    );
    assert_eq!(usize::from(a.unwrap()) + usize::from(b.unwrap()), 1);
    let events: i64 = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM \"{}\".events WHERE kind='document.collaboration.resolved.v1'",
        f.org.schema()
    ))
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert_eq!(events, 1);
    assert!(f.org.pending_document_attention().await.unwrap().is_empty());
    f.drop().await;
}

#[tokio::test]
async fn a_work_bound_native_review_never_resumes_its_work_during_undo() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    f.org
        .create_actor(
            "native-producer",
            "lead",
            "Avery North",
            None,
            "exec",
            "Produces a reviewed document",
        )
        .await
        .unwrap();
    f.org
        .create_team(
            "Native production",
            "Prepare the exact document",
            "native-producer",
            "exec",
        )
        .await
        .unwrap();
    f.org
        .add_work(NewWork {
            owner_id: "native-producer",
            title: "Prepare a native review",
            outcome: "Deliver the reviewed document",
            goal_id: None,
            priority: 0,
            expected_artifact: "",
            workspace: WorkspaceSpec::default(),
            attempt_limit: Some(2),
        })
        .await
        .unwrap();
    let claim = f
        .org
        .claim_ready_work("Prepare a native document")
        .await
        .unwrap()
        .unwrap();
    let doc = f.document().await;
    f.share(doc, "native-producer").await;
    let before = f.org.get_document_for_actor(doc, "owner").await.unwrap();
    let review = f
        .org
        .request_document_review(RequestDocumentReview {
            document_id: doc,
            actor_id: "native-producer",
            command_id: Uuid::new_v4(),
            expected_document_version: before.document.version,
            expected_current_version_id: before.current_version.version.id,
            summary: "Review this exact production result",
            reviewer_actor_id: None,
            work_dependency: Some(DocumentWorkReviewDependencyInput {
                work_id: claim.work.id,
                attempt_id: claim.attempt_id,
                expected_work_revision: claim.work.revision,
                reviewer_actor_id: "owner",
            }),
        })
        .await
        .unwrap();
    let view = f.org.get_document_for_actor(doc, "owner").await.unwrap();
    let decision = DocumentDecision::AcceptReview {
        expected_document_version: view.document.version,
        expected_review_version: review.review.version,
        accepted_version_name: "Reviewed production result".into(),
        feedback: "Continue from this reviewed result".into(),
    };
    assert!(
        f.org
            .queue_document_decision(Uuid::new_v4(), doc, review.review.id, &owner(), &decision)
            .await
            .is_err(),
        "A producer still running cannot create an owner acceptance intent"
    );
    f.org
        .finish_work_attempt(
            claim.attempt_id,
            WorkAttemptState::Blocked,
            "Waiting for the saved native review",
        )
        .await
        .unwrap();
    let id = Uuid::new_v4();
    f.org
        .queue_document_decision(id, doc, review.review.id, &owner(), &decision)
        .await
        .unwrap();
    assert!(f.org.apply_document_decision(id).await.is_err());
    let status: String = sqlx::query_scalar(&format!(
        "SELECT status::text FROM \"{}\".work WHERE id=$1",
        f.org.schema()
    ))
    .bind(claim.work.id)
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert_eq!(status, "blocked");
    let feedback: i64 = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM \"{}\".work_feedback WHERE work_id=$1",
        f.org.schema()
    ))
    .bind(claim.work.id)
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert_eq!(feedback, 0);
    f.due(id).await;
    assert!(f.org.apply_document_decision(id).await.unwrap());
    assert!(!f.org.apply_document_decision(id).await.unwrap());
    let status: String = sqlx::query_scalar(&format!(
        "SELECT status::text FROM \"{}\".work WHERE id=$1",
        f.org.schema()
    ))
    .bind(claim.work.id)
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert_eq!(status, "active");
    let feedback: i64 = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM \"{}\".work_feedback WHERE work_id=$1",
        f.org.schema()
    ))
    .bind(claim.work.id)
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert_eq!(feedback, 1);
    assert_eq!(
        f.comments(doc).await,
        0,
        "Work-bound feedback reaches its producer, not an unrelated comment"
    );
    f.drop().await;
}

#[tokio::test]
async fn the_requester_receives_the_review_note_only_when_acceptance_commits() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let doc = f.document().await;
    f.share(doc, "exec").await;
    let view = f.org.get_document_for_actor(doc, "owner").await.unwrap();
    let review = f
        .org
        .request_document_review(RequestDocumentReview {
            document_id: doc,
            actor_id: "exec",
            command_id: Uuid::new_v4(),
            expected_document_version: view.document.version,
            expected_current_version_id: view.current_version.version.id,
            summary: "Please judge the exact saved version",
            reviewer_actor_id: None,
            work_dependency: None,
        })
        .await
        .unwrap();
    let view = f.org.get_document_for_actor(doc, "owner").await.unwrap();
    let decision = DocumentDecision::AcceptReview {
        expected_document_version: view.document.version,
        expected_review_version: review.review.version,
        accepted_version_name: "Reviewed with the note".into(),
        feedback: "The requester's useful review feedback".into(),
    };
    let id = Uuid::new_v4();
    f.org
        .queue_document_decision(id, doc, review.review.id, &owner(), &decision)
        .await
        .unwrap();
    assert_eq!(f.comments(doc).await, 0);
    let before: i64 = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM \"{}\".native_document_mentions WHERE document_id=$1",
        f.org.schema()
    ))
    .bind(doc)
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert_eq!(before, 0);
    f.due(id).await;
    assert!(f.org.apply_document_decision(id).await.unwrap());
    let mentioned: Vec<String> = sqlx::query_scalar(&format!(
        "SELECT mentioned_actor_ids FROM \"{}\".native_document_comments WHERE document_id=$1",
        f.org.schema()
    ))
    .bind(doc)
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert_eq!(mentioned, vec!["exec"]);
    let after:i64=sqlx::query_scalar(&format!("SELECT count(*) FROM \"{}\".native_document_mentions WHERE document_id=$1 AND mentioned_actor_id='exec'",f.org.schema())).bind(doc).fetch_one(&f.db).await.unwrap();
    assert_eq!(after, 1);
    assert!(!f.org.apply_document_decision(id).await.unwrap());
    assert_eq!(f.comments(doc).await, 1);
    f.drop().await;
}
