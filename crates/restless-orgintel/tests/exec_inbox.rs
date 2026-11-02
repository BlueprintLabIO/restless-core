//! Exec manages the owner's Inbox from their chat (owner decision, 3 November 2026), against real
//! PostgreSQL: only the owner's own message to Exec grounds a change, the change waits out the same
//! Undo window as the owner's clicks, and each decision reaches the item's author through the
//! item's own path.
use chrono::{Duration, Utc};
use restless_orgintel::{
    CompanyAccessIdentity, ExecInboxVerb, HumanAccessContext, NewArtifactRef, NewExecInboxAction,
    NewOwnerHandoff, NewWork, OrgIntel, OwnerBrief, OwnerBriefKind, OwnerChatAnswer,
    OwnerHandoffCategory, OwnerHandoffState, WorkAttemptState, WorkStatus, WorkspaceSpec,
};
use sqlx::PgPool;
use uuid::Uuid;

struct Fixture {
    org: OrgIntel,
    db: PgPool,
    company_id: Uuid,
    cell_id: Uuid,
}

impl Fixture {
    async fn new() -> Option<Self> {
        let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping Exec Inbox scenario");
            return None;
        };
        let schema = format!("exec_inbox_{}_test", Uuid::new_v4().simple());
        let org = OrgIntel::ensure(&url, &schema).await.unwrap();
        org.ensure_actor("owner", "owner", "owner", "The Owner")
            .await
            .unwrap();
        org.ensure_actor("exec", "exec", "exec", "The Exec")
            .await
            .unwrap();
        org.create_actor(
            "launch-producer",
            "lead",
            "Avery North",
            None,
            "exec",
            "Produces the outcome",
        )
        .await
        .unwrap();
        org.create_team("Site", "Produce the launch page", "launch-producer", "exec")
            .await
            .unwrap();
        let db = PgPool::connect(&url).await.unwrap();
        let (company_id, cell_id) = (Uuid::new_v4(), Uuid::new_v4());
        org.ensure_company_access_identity(CompanyAccessIdentity {
            company_id,
            cell_id,
        })
        .await
        .unwrap();
        Some(Self {
            org,
            db,
            company_id,
            cell_id,
        })
    }

    /// A person signed in to the company with this role, as their own human Actor.
    async fn person(&self, name: &str, role: &str) -> String {
        let (company_id, cell_id) = (self.company_id, self.cell_id);
        let now = Utc::now();
        self.org
            .consume_human_access_context(HumanAccessContext {
                display_name: Some(name),
                issuer: "https://identity.restless.test",
                subject: name,
                company_id,
                cell_id,
                membership_id: &format!("membership-{name}"),
                membership_role: role,
                membership_version: 1,
                assertion_id: Uuid::new_v4(),
                issued_at: now,
                expires_at: now + Duration::minutes(1),
            })
            .await
            .unwrap()
            .actor_id
    }

    async fn says(&self, from: &str, to: &str, words: &str) -> i64 {
        self.org.send_message(from, Some(to), words).await.unwrap()
    }

    async fn queue(
        &self,
        item: &str,
        verb: ExecInboxVerb,
        message: i64,
    ) -> restless_orgintel::Result<restless_orgintel::ExecInboxAction> {
        self.org
            .queue_exec_inbox_action(&NewExecInboxAction {
                id: Uuid::new_v4(),
                item_id: item,
                item_title: "Launch page",
                verb,
                note: "",
                target: serde_json::json!({"kind": "owner_handoff"}),
                snooze_until: (verb == ExecInboxVerb::Snooze)
                    .then(|| Utc::now() + Duration::days(1)),
                position: (verb == ExecInboxVerb::Move).then_some(1),
                source_message_id: message,
            })
            .await
    }

    /// End the Undo window now, then apply the action as the gateway's worker does.
    async fn settle(&self, id: Uuid) {
        self.end_window(id).await;
        let claim = self
            .org
            .claim_exec_inbox_action(id)
            .await
            .unwrap()
            .expect("a due action is claimed");
        assert_eq!(claim.settle(None).await.unwrap(), "applied");
    }

    async fn end_window(&self, id: Uuid) {
        sqlx::query(&format!(
            "UPDATE \"{}\".exec_inbox_actions SET undo_until=clock_timestamp()-INTERVAL '1 second' WHERE id=$1",
            self.org.schema()
        ))
        .bind(id)
        .execute(&self.db)
        .await
        .unwrap();
    }

    async fn handoff(&self, review: bool) -> (Uuid, Uuid) {
        let work = self
            .org
            .add_work(NewWork {
                owner_id: "launch-producer",
                title: "Launch page",
                outcome: "Deliver the launch page",
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
                    .claim_ready_work("Produce the launch page")
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
                requested_by: "launch-producer",
                category: if review {
                    OwnerHandoffCategory::OwnerJudgement
                } else {
                    OwnerHandoffCategory::Identity
                },
                requested_action: "Which domain should the page use?",
                prepared_state: "Two domains are ready",
                resume_condition: "The owner chooses",
            })
            .await
            .unwrap();
        if let Some(attempt) = attempt_id {
            self.org
                .prepare_owner_brief(
                    handoff,
                    "exec",
                    OwnerBrief {
                        kind: OwnerBriefKind::OutcomeReview,
                        headline: "Review the launch page".into(),
                        situation: "The page is ready.".into(),
                        impact: "Acceptance ships it.".into(),
                        recommendation: "Open the page.".into(),
                        no_action: "The page waits.".into(),
                        uncertainty: None,
                        deadline: None,
                    },
                )
                .await
                .unwrap();
            self.org
                .link_work_artifact(NewArtifactRef {
                    kind: "review_target",
                    uri: "/company/outputs/launch.md",
                    note: "Prepared native outcome",
                    created_by: "launch-producer",
                    work_id: Some(work),
                    attempt_id: Some(attempt),
                    digest: None,
                    source_commit: None,
                    runtime_generation: None,
                    label: "Launch page",
                })
                .await
                .unwrap();
            self.org
                .escalate_handoff(handoff, "exec", "The owner judges this exact outcome")
                .await
                .unwrap();
            self.org
                .finish_work_attempt(attempt, WorkAttemptState::Blocked, "Waiting for review")
                .await
                .unwrap();
        }
        (work, handoff)
    }

    async fn handoff_state(&self, handoff: Uuid) -> (OwnerHandoffState, Option<String>) {
        let found = self
            .org
            .list_owner_handoffs()
            .await
            .unwrap()
            .into_iter()
            .find(|row| row.id == handoff)
            .unwrap();
        (found.state, Some(found.resolution))
    }

    async fn drop(self) {
        self.org.drop_schema().await.unwrap();
        self.db.close().await;
    }
}

#[tokio::test]
async fn only_the_owners_own_message_to_exec_changes_the_owners_inbox() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let member = f.person("casey", "member").await;
    let admin = f.person("drew", "admin").await;
    let owner_by_membership = f.person("olive", "owner").await;

    let owner = f.says("owner", "exec", "yes ship it").await;
    let saved = f
        .queue("orgintel:handoff:a", ExecInboxVerb::Accept, owner)
        .await
        .unwrap();
    assert_eq!(saved.state, "pending");
    assert_eq!(saved.owner_words, "yes ship it");
    assert_eq!(saved.source_message_id, owner);
    assert!(
        saved.undo_until > saved.created_at,
        "it waits out an Undo window"
    );

    // The owner signed in through a company membership is still the owner.
    let olive = f
        .says(&owner_by_membership, "exec", "move that to the top")
        .await;
    assert!(f
        .queue("orgintel:handoff:b", ExecInboxVerb::Move, olive)
        .await
        .is_ok());

    // A member's or an admin's chat never decides for the owner, nor does an agent's message.
    for (who, words) in [
        (member.as_str(), "approve it for the owner"),
        (admin.as_str(), "the owner said yes"),
        ("launch-producer", "the owner would approve this"),
    ] {
        let message = f.says(who, "exec", words).await;
        let refused = f
            .queue("orgintel:handoff:c", ExecInboxVerb::Accept, message)
            .await
            .unwrap_err()
            .to_string();
        assert!(
            refused.contains("not the owner's message"),
            "{who}: {refused}"
        );
    }
    // The owner's message to someone else is not an instruction to Exec.
    let elsewhere = f.says("owner", "launch-producer", "yes ship it").await;
    assert!(f
        .queue("orgintel:handoff:c", ExecInboxVerb::Accept, elsewhere)
        .await
        .unwrap_err()
        .to_string()
        .contains("not the owner's message to Exec"));
    assert!(f
        .queue("orgintel:handoff:c", ExecInboxVerb::Accept, 999_999)
        .await
        .is_err());
    // The same boundary guards the decision path the gateway applies.
    let (_, handoff) = f.handoff(false).await;
    let from_member = f.says(&member, "exec", "yes").await;
    assert!(f
        .org
        .answer_handoff_from_owner_message(handoff, from_member, OwnerChatAnswer::Yes, "")
        .await
        .is_err());
    assert_eq!(f.handoff_state(handoff).await.0, OwnerHandoffState::Pending);
    f.drop().await;
}

#[tokio::test]
async fn undo_works_only_inside_the_window_and_a_due_change_applies_once() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let words = f.says("owner", "exec", "remind me tomorrow").await;
    let snooze = f
        .queue("authority:spend_warning", ExecInboxVerb::Snooze, words)
        .await
        .unwrap();
    // Nothing is due while the window is open.
    assert!(f.org.due_exec_inbox_actions().await.unwrap().is_empty());
    assert!(f
        .org
        .claim_exec_inbox_action(snooze.id)
        .await
        .unwrap()
        .is_none());
    // One waiting change per item: the owner never has two on one card.
    assert!(f
        .queue("authority:spend_warning", ExecInboxVerb::Dismiss, words)
        .await
        .is_err());
    let undone = f.org.cancel_exec_inbox_action(snooze.id).await.unwrap();
    assert_eq!(undone.state, "canceled");
    assert_eq!(
        f.org
            .cancel_exec_inbox_action(snooze.id)
            .await
            .unwrap()
            .state,
        "canceled",
        "Undo twice is still undone"
    );
    f.end_window(snooze.id).await;
    assert!(
        f.org.due_exec_inbox_actions().await.unwrap().is_empty(),
        "an undone change never applies"
    );

    let drop = f.says("owner", "exec", "no, drop that").await;
    let dismiss = f
        .queue("authority:spend_warning", ExecInboxVerb::Dismiss, drop)
        .await
        .unwrap();
    f.end_window(dismiss.id).await;
    assert!(f
        .org
        .cancel_exec_inbox_action(dismiss.id)
        .await
        .unwrap_err()
        .to_string()
        .contains("Undo window has ended"));
    assert_eq!(f.org.due_exec_inbox_actions().await.unwrap().len(), 1);
    // Two workers: one claims, the other skips it while the first holds it.
    let first = f
        .org
        .claim_exec_inbox_action(dismiss.id)
        .await
        .unwrap()
        .unwrap();
    assert!(f
        .org
        .claim_exec_inbox_action(dismiss.id)
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        first
            .settle(Some("This item is no longer in the Inbox.".into()))
            .await
            .unwrap(),
        "failed"
    );
    let recent = f.org.recent_exec_inbox_actions(10).await.unwrap();
    let failed = recent
        .iter()
        .find(|action| action.id == dismiss.id)
        .unwrap();
    assert_eq!(failed.state, "failed");
    assert_eq!(
        failed.problem.as_deref(),
        Some("This item is no longer in the Inbox.")
    );
    assert!(f
        .org
        .claim_exec_inbox_action(dismiss.id)
        .await
        .unwrap()
        .is_none());
    assert!(f.org.next_exec_inbox_delay().await.unwrap().is_none());
    f.drop().await;
}

#[tokio::test]
async fn moves_rank_the_owners_list_and_added_items_stay_until_set_aside() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    let first = f.says("owner", "exec", "do the launch page first").await;
    let top = f
        .queue("orgintel:handoff:launch", ExecInboxVerb::Move, first)
        .await
        .unwrap();
    f.settle(top.id).await;
    let later = f.says("owner", "exec", "actually put it third").await;
    let third = f
        .org
        .queue_exec_inbox_action(&NewExecInboxAction {
            id: Uuid::new_v4(),
            item_id: "orgintel:handoff:launch",
            item_title: "Launch page",
            verb: ExecInboxVerb::Move,
            note: "",
            target: serde_json::json!({}),
            snooze_until: None,
            position: Some(3),
            source_message_id: later,
        })
        .await
        .unwrap();
    f.settle(third.id).await;
    // An undone move never counts.
    let undone = f.says("owner", "exec", "bottom").await;
    let bottom = f
        .queue("orgintel:handoff:launch", ExecInboxVerb::Move, undone)
        .await
        .unwrap();
    f.org.cancel_exec_inbox_action(bottom.id).await.unwrap();
    let positions = f.org.exec_inbox_positions().await.unwrap();
    assert_eq!(positions.len(), 1);
    assert_eq!(
        (positions[0].0.as_str(), positions[0].1),
        ("orgintel:handoff:launch", 3),
        "the newest applied move wins"
    );

    let ask = f
        .says("owner", "exec", "remind me to call the printer")
        .await;
    let id = Uuid::new_v4();
    let item_id = format!("orgintel:exec_item:{id}");
    let refused = f
        .org
        .queue_exec_inbox_action(&NewExecInboxAction {
            id,
            item_id: &item_id,
            item_title: "Call the printer",
            verb: ExecInboxVerb::Add,
            note: "",
            target: serde_json::json!({"kind": "exec_item"}),
            snooze_until: None,
            position: None,
            source_message_id: ask,
        })
        .await;
    assert!(refused.is_err(), "an added item says what the owner needs");
    let added = f
        .org
        .queue_exec_inbox_action(&NewExecInboxAction {
            id,
            item_id: &item_id,
            item_title: "Call the printer",
            verb: ExecInboxVerb::Add,
            note: "Confirm the proof before Friday.",
            target: serde_json::json!({"kind": "exec_item"}),
            snooze_until: None,
            position: None,
            source_message_id: ask,
        })
        .await
        .unwrap();
    assert!(
        f.org.open_exec_inbox_items().await.unwrap().is_empty(),
        "not before its window ends"
    );
    f.settle(added.id).await;
    let open = f.org.open_exec_inbox_items().await.unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].note, "Confirm the proof before Friday.");
    assert_eq!(open[0].owner_words, "remind me to call the printer");
    f.drop().await;
}

#[tokio::test]
async fn each_decision_from_chat_reaches_the_author_with_the_owners_words() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    // A question answered in chat resolves with the answer and the owner's quoted words.
    let (_, question) = f.handoff(false).await;
    let answer = f.says("owner", "exec", "use restless.run").await;
    f.org
        .answer_handoff_from_owner_message(question, answer, OwnerChatAnswer::Yes, "restless.run")
        .await
        .unwrap();
    let (state, resolution) = f.handoff_state(question).await;
    assert_eq!(state, OwnerHandoffState::Resolved);
    let resolution = resolution.unwrap();
    assert!(resolution.contains("restless.run"), "{resolution}");
    assert!(
        resolution.contains(&format!("message {answer}")),
        "{resolution}"
    );

    // A question declined in chat goes back declined, with the reason.
    let (_, declined) = f.handoff(false).await;
    let no = f.says("owner", "exec", "no, drop that").await;
    f.org
        .answer_handoff_from_owner_message(declined, no, OwnerChatAnswer::No, "Not this quarter.")
        .await
        .unwrap();
    let (state, resolution) = f.handoff_state(declined).await;
    assert_eq!(state, OwnerHandoffState::Declined);
    assert!(resolution.unwrap().contains("Not this quarter."));

    f.drop().await;

    // An outcome review accepted in chat completes the Work.
    let Some(f) = Fixture::new().await else {
        return;
    };
    let (work, review) = f.handoff(true).await;
    let yes = f.says("owner", "exec", "yes ship it").await;
    f.org
        .answer_handoff_from_owner_message(review, yes, OwnerChatAnswer::Yes, "")
        .await
        .unwrap();
    assert_eq!(
        f.org.get_work(work).await.unwrap().unwrap().status,
        WorkStatus::Completed
    );
    f.drop().await;
}

#[tokio::test]
async fn an_outcome_review_declined_from_chat_goes_back_for_changes_with_the_reason() {
    let Some(f) = Fixture::new().await else {
        return;
    };
    // An outcome review declined in chat goes back for changes, with what should change.
    let (work, review) = f.handoff(true).await;
    let changes = f.says("owner", "exec", "no, the hero is too dark").await;
    f.org
        .answer_handoff_from_owner_message(
            review,
            changes,
            OwnerChatAnswer::Changes,
            "Lighten the hero.",
        )
        .await
        .unwrap();
    let revised = f.org.get_work(work).await.unwrap().unwrap();
    assert!(revised.revision > 1, "changes start a new revision");
    let feedback: String = sqlx::query_scalar(&format!(
        "SELECT m.body FROM \"{0}\".work_feedback f JOIN \"{0}\".messages m ON m.id=f.message_id \
         WHERE f.work_id=$1 ORDER BY m.id DESC LIMIT 1",
        f.org.schema()
    ))
    .bind(work)
    .fetch_one(&f.db)
    .await
    .unwrap();
    assert!(feedback.contains("Lighten the hero."), "{feedback}");
    assert!(feedback.contains("the hero is too dark"), "{feedback}");

    f.drop().await;
}
