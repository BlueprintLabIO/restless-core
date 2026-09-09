//! Real-Postgres adversarial proof for the bounded Native Docs company
//! semantics added in migration 0048. These scenarios deliberately exercise
//! authorization, replay, concurrency and immutable history at the service and
//! database boundaries.

use restless_orgintel::{
    AcceptDocumentReview, DocumentAccess, DocumentCommentStatus, DocumentError, DocumentKind,
    DocumentRevisionDecision, DocumentRevisionScope, DocumentRevisionStatus, DocumentStatus,
    DocumentVisibility, NewDocument, NewDocumentCommentThread, NewNamedDocumentVersion, OrgIntel,
    ProposeDocumentRevision, ReplyToDocumentComment, RequestDocumentReview,
    ResolveDocumentCommentThread, ResolveDocumentRevisionProposal, SetDocumentParticipant,
};
use serde_json::{json, Value};
use sqlx::{Connection as _, PgConnection};
use uuid::Uuid;

async fn company(prefix: &str) -> Option<OrgIntel> {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").ok()?;
    let schema = format!("{prefix}{}", Uuid::new_v4().simple());
    let org = OrgIntel::ensure(&url, &schema)
        .await
        .expect("ensure scratch company schema");
    org.ensure_actor("owner", "owner", "owner", "The Owner")
        .await
        .unwrap();
    org.ensure_actor("alex", "human", "member", "Alex")
        .await
        .unwrap();
    org.ensure_actor("blair", "human", "member", "Blair")
        .await
        .unwrap();
    org.ensure_actor("research-analyst", "staff", "analyst", "Research Analyst")
        .await
        .unwrap();
    Some(org)
}

fn document_content(block_id: &str, text: &str) -> Value {
    json!({
        "type": "doc",
        "content": [{
            "type": "paragraph",
            "attrs": {"block_id": block_id},
            "content": [{"type": "text", "text": text}]
        }]
    })
}

fn comment_content(block_id: &str, text: &str, mention: Option<&str>) -> Value {
    let mut inline = vec![json!({"type": "text", "text": text})];
    if let Some(actor_id) = mention {
        inline.push(json!({
            "type": "mention",
            "attrs": {"actor_id": actor_id, "label": actor_id}
        }));
    }
    json!({
        "type": "doc",
        "content": [{
            "type": "paragraph",
            "attrs": {"block_id": block_id},
            "content": inline
        }]
    })
}

async fn create_private_document(
    org: &OrgIntel,
    text: &str,
) -> restless_orgintel::DocumentReadView {
    let content = document_content("claim", text);
    let created = org
        .create_document(NewDocument {
            command_id: Uuid::new_v4(),
            title: "Native document",
            kind: DocumentKind::Brief,
            visibility: DocumentVisibility::Participants,
            linked_room_id: None,
            inherit_room_visibility: false,
            owner_actor_id: "owner",
            created_by_actor_id: "owner",
            content_json: &content,
            reason: "Initial version",
        })
        .await
        .unwrap();
    org.get_document_for_actor(created.document_id, "owner")
        .await
        .unwrap()
}

async fn connection_for(org: &OrgIntel) -> PgConnection {
    let url = std::env::var("RESTLESS_TEST_DATABASE_URL").unwrap();
    let mut connection = PgConnection::connect(&url).await.unwrap();
    sqlx::query(&format!("SET search_path TO {}", org.schema()))
        .execute(&mut connection)
        .await
        .unwrap();
    connection
}

#[tokio::test]
async fn comments_are_access_scoped_replayable_bounded_and_concurrently_resolved() {
    let Some(org) = company("doccomments").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping native Docs comment scenario");
        return;
    };
    let other = company("doccommentsother").await.unwrap();
    let created = create_private_document(&org, "A claim worth reviewing").await;
    let document_id = created.document.id;
    org.set_document_participant(SetDocumentParticipant {
        command_id: Uuid::new_v4(),
        document_id,
        actor_id: "owner",
        expected_document_version: 1,
        participant_actor_id: "alex",
        access: DocumentAccess::Comment,
    })
    .await
    .unwrap();

    let command_id = Uuid::new_v4();
    let comment = comment_content("comment-one", "Please verify this with ", Some("owner"));
    let first = org
        .create_document_comment_thread(NewDocumentCommentThread {
            document_id,
            actor_id: "alex",
            command_id,
            block_id: Some("claim"),
            content_json: &comment,
        })
        .await
        .unwrap();
    let replay = org
        .create_document_comment_thread(NewDocumentCommentThread {
            document_id,
            actor_id: "alex",
            command_id,
            block_id: Some("claim"),
            content_json: &comment,
        })
        .await
        .unwrap();
    assert_eq!(replay.thread.thread.id, first.thread.thread.id);
    assert_eq!(replay.first_comment.id, first.first_comment.id);
    assert_eq!(
        replay.first_comment.created_at,
        first.first_comment.created_at
    );
    assert_eq!(first.first_comment.mentioned_actor_ids, vec!["owner"]);

    let drift = comment_content("comment-drift", "A different command body", None);
    assert!(matches!(
        org.create_document_comment_thread(NewDocumentCommentThread {
            document_id,
            actor_id: "alex",
            command_id,
            block_id: Some("claim"),
            content_json: &drift,
        })
        .await,
        Err(DocumentError::Conflict(_))
    ));

    let reply_command = Uuid::new_v4();
    let reply_content = comment_content("reply-one", "Evidence attached", None);
    let reply = org
        .reply_to_document_comment(ReplyToDocumentComment {
            document_id,
            thread_id: first.thread.thread.id,
            reply_to_comment_id: Some(first.first_comment.id),
            actor_id: "alex",
            command_id: reply_command,
            content_json: &reply_content,
        })
        .await
        .unwrap();
    let reply_replay = org
        .reply_to_document_comment(ReplyToDocumentComment {
            document_id,
            thread_id: first.thread.thread.id,
            reply_to_comment_id: Some(first.first_comment.id),
            actor_id: "alex",
            command_id: reply_command,
            content_json: &reply_content,
        })
        .await
        .unwrap();
    assert_eq!(reply.id, reply_replay.id);
    assert_eq!(reply.reply_to_comment_id, Some(first.first_comment.id));

    let page = org
        .list_document_comments(document_id, first.thread.thread.id, "alex", None, 1)
        .await
        .unwrap();
    assert_eq!(page.items.len(), 1);
    assert!(page.next_cursor.is_some());
    let second_page = org
        .list_document_comments(
            document_id,
            first.thread.thread.id,
            "alex",
            page.next_cursor.as_ref(),
            1,
        )
        .await
        .unwrap();
    assert_eq!(second_page.items.len(), 1);
    assert_eq!(second_page.items[0].id, reply.id);
    assert!(second_page.next_cursor.is_none());
    assert!(matches!(
        org.list_document_comments(document_id, first.thread.thread.id, "alex", None, 0)
            .await,
        Err(DocumentError::Invalid(_))
    ));
    assert!(matches!(
        org.list_document_comment_threads(document_id, "blair", None, 10)
            .await,
        Err(DocumentError::Unavailable)
    ));
    assert!(matches!(
        other
            .list_document_comment_threads(document_id, "owner", None, 10)
            .await,
        Err(DocumentError::Unavailable)
    ));

    let first_resolve_command = Uuid::new_v4();
    let second_resolve_command = Uuid::new_v4();
    let left_org = org.clone();
    let right_org = org.clone();
    let thread_id = first.thread.thread.id;
    let (left, right) = tokio::join!(
        left_org.resolve_document_comment_thread(ResolveDocumentCommentThread {
            document_id,
            thread_id,
            actor_id: "owner",
            command_id: first_resolve_command,
            expected_thread_version: 1,
        }),
        right_org.resolve_document_comment_thread(ResolveDocumentCommentThread {
            document_id,
            thread_id,
            actor_id: "owner",
            command_id: second_resolve_command,
            expected_thread_version: 1,
        })
    );
    assert_eq!(left.is_ok() as u8 + right.is_ok() as u8, 1);
    let resolved = left.ok().or_else(|| right.ok()).unwrap();
    assert_eq!(resolved.status, DocumentCommentStatus::Resolved);
    assert_eq!(resolved.version, 2);
    let creation_after_resolution = org
        .create_document_comment_thread(NewDocumentCommentThread {
            document_id,
            actor_id: "alex",
            command_id,
            block_id: Some("claim"),
            content_json: &comment,
        })
        .await
        .unwrap();
    assert_eq!(
        creation_after_resolution.thread.thread.status,
        DocumentCommentStatus::Open
    );
    assert_eq!(creation_after_resolution.thread.thread.version, 1);

    let mut connection = connection_for(&org).await;
    let immutable =
        sqlx::query("UPDATE native_document_comments SET plain_text='tampered' WHERE id=$1")
            .bind(first.first_comment.id)
            .execute(&mut connection)
            .await;
    assert!(immutable.is_err());
    let event: Value = sqlx::query_scalar(
        "SELECT body FROM events WHERE document_id=$1 \
         AND kind='document.comment_thread.created.v1'",
    )
    .bind(document_id)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(
        event,
        json!({
            "document_id": document_id,
            "source_named_version_id": created.current_version.version.id,
            "thread_id": first.thread.thread.id,
            "comment_id": first.first_comment.id,
            "block_id": "claim",
            "author_actor_id": "alex",
            "mentioned_actor_ids": ["owner"],
            "command_id": command_id,
        })
    );
    let created_event_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM events WHERE document_id=$1 \
         AND kind='document.comment_thread.created.v1'",
    )
    .bind(document_id)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(
        created_event_count, 1,
        "command replay duplicated its event"
    );
}

#[tokio::test]
async fn review_acceptance_is_explicit_idempotent_and_has_one_concurrent_winner() {
    let Some(org) = company("docreviews").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping native Docs review scenario");
        return;
    };
    let other = company("docreviewsother").await.unwrap();
    let created = create_private_document(&org, "Reviewed body").await;
    let document_id = created.document.id;
    let requested_version_id = created.current_version.version.id;
    let request_command = Uuid::new_v4();
    let requested = org
        .request_document_review(RequestDocumentReview {
            document_id,
            actor_id: "owner",
            command_id: request_command,
            expected_document_version: 1,
            expected_current_version_id: requested_version_id,
            summary: "Check this before it becomes company guidance",
        })
        .await
        .unwrap();
    let request_replay = org
        .request_document_review(RequestDocumentReview {
            document_id,
            actor_id: "owner",
            command_id: request_command,
            expected_document_version: 1,
            expected_current_version_id: requested_version_id,
            summary: "Check this before it becomes company guidance",
        })
        .await
        .unwrap();
    assert_eq!(requested.id, request_replay.id);
    assert_eq!(requested.created_at, request_replay.created_at);
    assert!(matches!(
        org.get_document_review_for_actor(document_id, requested.id, "blair")
            .await,
        Err(DocumentError::Unavailable)
    ));
    assert!(matches!(
        other
            .get_document_review_for_actor(document_id, requested.id, "owner")
            .await,
        Err(DocumentError::Unavailable)
    ));
    let review_page = org
        .list_document_reviews(document_id, "owner", None, 1)
        .await
        .unwrap();
    assert_eq!(review_page.items.len(), 1);
    assert_eq!(review_page.items[0].id, requested.id);

    let left_command = Uuid::new_v4();
    let right_command = Uuid::new_v4();
    let left_org = org.clone();
    let right_org = org.clone();
    let review_id = requested.id;
    let (left, right) = tokio::join!(
        left_org.accept_document_review(AcceptDocumentReview {
            document_id,
            review_id,
            actor_id: "owner",
            command_id: left_command,
            expected_document_version: 2,
            expected_review_version: 1,
            accepted_version_name: "Accepted evidence v1",
        }),
        right_org.accept_document_review(AcceptDocumentReview {
            document_id,
            review_id,
            actor_id: "owner",
            command_id: right_command,
            expected_document_version: 2,
            expected_review_version: 1,
            accepted_version_name: "Competing acceptance",
        })
    );
    assert_eq!(left.is_ok() as u8 + right.is_ok() as u8, 1);
    let (accepted, winning_command, winning_name) = match (left, right) {
        (Ok(accepted), Err(DocumentError::Conflict(_))) => {
            (accepted, left_command, "Accepted evidence v1")
        }
        (Err(DocumentError::Conflict(_)), Ok(accepted)) => {
            (accepted, right_command, "Competing acceptance")
        }
        result => panic!("unexpected concurrent acceptance result: {result:?}"),
    };
    assert_eq!(
        accepted.review.status,
        restless_orgintel::DocumentReviewStatus::Accepted
    );
    let accepted_version = accepted.accepted_version.as_ref().unwrap();
    assert_eq!(accepted_version.version.reason, winning_name);
    assert_eq!(
        accepted_version.version.document_status,
        DocumentStatus::Accepted
    );
    assert_eq!(
        accepted_version.version.content_json,
        created.current_version.version.content_json
    );
    assert_ne!(accepted_version.version.id, requested_version_id);

    let replay = org
        .accept_document_review(AcceptDocumentReview {
            document_id,
            review_id,
            actor_id: "owner",
            command_id: winning_command,
            expected_document_version: 2,
            expected_review_version: 1,
            accepted_version_name: winning_name,
        })
        .await
        .unwrap();
    assert_eq!(
        replay.accepted_version.as_ref().unwrap().version.id,
        accepted_version.version.id
    );
    assert_eq!(replay.review.accepted_at, accepted.review.accepted_at);
    let current = org
        .get_document_for_actor(document_id, "owner")
        .await
        .unwrap();
    assert_eq!(current.document.status, DocumentStatus::Accepted);
    assert_eq!(
        current.current_version.version.id,
        accepted_version.version.id
    );
    let request_after_acceptance = org
        .request_document_review(RequestDocumentReview {
            document_id,
            actor_id: "owner",
            command_id: request_command,
            expected_document_version: 1,
            expected_current_version_id: requested_version_id,
            summary: "Check this before it becomes company guidance",
        })
        .await
        .unwrap();
    assert_eq!(
        request_after_acceptance.status,
        restless_orgintel::DocumentReviewStatus::Requested
    );
    assert_eq!(request_after_acceptance.version, 1);
    assert!(request_after_acceptance.accepted_version_id.is_none());

    let post_acceptance_content = document_content("claim", "A later owner revision");
    let post_acceptance = org
        .create_named_document_version(NewNamedDocumentVersion {
            command_id: Uuid::new_v4(),
            document_id,
            actor_id: "owner",
            expected_current_version_id: accepted_version.version.id,
            content_json: &post_acceptance_content,
            reason: "Prepare another review",
        })
        .await
        .unwrap();
    let first_later_review = org
        .request_document_review(RequestDocumentReview {
            document_id,
            actor_id: "owner",
            command_id: Uuid::new_v4(),
            expected_document_version: 4,
            expected_current_version_id: post_acceptance.result_id,
            summary: "Review the later revision",
        })
        .await
        .unwrap();
    let newest_content = document_content("claim", "The base moved again");
    let newest = org
        .create_named_document_version(NewNamedDocumentVersion {
            command_id: Uuid::new_v4(),
            document_id,
            actor_id: "owner",
            expected_current_version_id: post_acceptance.result_id,
            content_json: &newest_content,
            reason: "Move the pending review base",
        })
        .await
        .unwrap();
    let second_later_review = org
        .request_document_review(RequestDocumentReview {
            document_id,
            actor_id: "owner",
            command_id: Uuid::new_v4(),
            expected_document_version: 6,
            expected_current_version_id: newest.result_id,
            summary: "Review the current revision, not its stale predecessor",
        })
        .await
        .unwrap();
    assert_eq!(
        org.get_document_review_for_actor(document_id, first_later_review.id, "owner")
            .await
            .unwrap()
            .status,
        restless_orgintel::DocumentReviewStatus::Stale
    );
    assert_eq!(
        second_later_review.status,
        restless_orgintel::DocumentReviewStatus::Requested
    );
    let final_base = document_content("claim", "One final base change");
    let final_advanced = org
        .create_named_document_version(NewNamedDocumentVersion {
            command_id: Uuid::new_v4(),
            document_id,
            actor_id: "owner",
            expected_current_version_id: newest.result_id,
            content_json: &final_base,
            reason: "Invalidate the second review",
        })
        .await
        .unwrap();
    let stale_accept_command = Uuid::new_v4();
    let stale_review = org
        .accept_document_review(AcceptDocumentReview {
            document_id,
            review_id: second_later_review.id,
            actor_id: "owner",
            command_id: stale_accept_command,
            expected_document_version: 8,
            expected_review_version: 1,
            accepted_version_name: "Must not be created",
        })
        .await
        .unwrap();
    assert_eq!(
        stale_review.review.status,
        restless_orgintel::DocumentReviewStatus::Stale
    );
    assert!(stale_review.accepted_version.is_none());
    let stale_review_replay = org
        .accept_document_review(AcceptDocumentReview {
            document_id,
            review_id: second_later_review.id,
            actor_id: "owner",
            command_id: stale_accept_command,
            expected_document_version: 8,
            expected_review_version: 1,
            accepted_version_name: "Must not be created",
        })
        .await
        .unwrap();
    assert_eq!(
        stale_review_replay.review.stale_at,
        stale_review.review.stale_at
    );
    assert!(stale_review_replay.accepted_version.is_none());
    assert_eq!(
        org.get_document_for_actor(document_id, "owner")
            .await
            .unwrap()
            .current_version
            .version
            .id,
        final_advanced.result_id
    );

    let mut connection = connection_for(&org).await;
    let immutable =
        sqlx::query("UPDATE native_document_versions SET reason='rewritten' WHERE id=$1")
            .bind(requested_version_id)
            .execute(&mut connection)
            .await;
    assert!(immutable.is_err());
    let event: Value = sqlx::query_scalar(
        "SELECT body FROM events WHERE document_id=$1 AND kind='document.review.accepted.v1'",
    )
    .bind(document_id)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(event["review_id"], json!(review_id));
    assert_eq!(
        event["requested_named_version_id"],
        json!(requested_version_id)
    );
    assert_eq!(
        event["accepted_named_version_id"],
        json!(accepted_version.version.id)
    );
    assert_eq!(event["accepted_by_actor_id"], "owner");
    assert_eq!(event["command_id"], json!(winning_command));
    let accepted_event_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM events WHERE document_id=$1 \
         AND kind='document.review.accepted.v1'",
    )
    .bind(document_id)
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(
        accepted_event_count, 1,
        "accept replay duplicated its event"
    );
}

#[tokio::test]
async fn agent_proposals_detect_stale_bases_and_never_overwrite_history() {
    let Some(org) = company("docproposals").await else {
        eprintln!("RESTLESS_TEST_DATABASE_URL unset; skipping native Docs proposal scenario");
        return;
    };
    let created = create_private_document(&org, "Original claim").await;
    let document_id = created.document.id;
    let initial_version_id = created.current_version.version.id;
    org.set_document_participant(SetDocumentParticipant {
        command_id: Uuid::new_v4(),
        document_id,
        actor_id: "owner",
        expected_document_version: 1,
        participant_actor_id: "research-analyst",
        access: DocumentAccess::Edit,
    })
    .await
    .unwrap();

    let direct_agent_edit = document_content("claim", "An unreviewed agent overwrite");
    assert!(matches!(
        org.create_named_document_version(NewNamedDocumentVersion {
            command_id: Uuid::new_v4(),
            document_id,
            actor_id: "research-analyst",
            expected_current_version_id: initial_version_id,
            content_json: &direct_agent_edit,
            reason: "Bypass proposal review",
        })
        .await,
        Err(DocumentError::Unavailable)
    ));

    let proposed_block = json!({
        "type": "paragraph",
        "attrs": {"block_id": "claim"},
        "content": [{"type": "text", "text": "Agent-supported claim"}]
    });
    let proposal_command = Uuid::new_v4();
    let proposal = org
        .propose_document_revision(ProposeDocumentRevision {
            document_id,
            actor_id: "research-analyst",
            command_id: proposal_command,
            base_version_id: initial_version_id,
            scope: DocumentRevisionScope::Block,
            block_id: Some("claim"),
            proposed_content_json: &proposed_block,
            summary: "Replace the claim with the evidence-backed wording",
        })
        .await
        .unwrap();
    let proposal_replay = org
        .propose_document_revision(ProposeDocumentRevision {
            document_id,
            actor_id: "research-analyst",
            command_id: proposal_command,
            base_version_id: initial_version_id,
            scope: DocumentRevisionScope::Block,
            block_id: Some("claim"),
            proposed_content_json: &proposed_block,
            summary: "Replace the claim with the evidence-backed wording",
        })
        .await
        .unwrap();
    assert_eq!(proposal.id, proposal_replay.id);
    assert_eq!(proposal.proposed_by_actor_id, "research-analyst");
    let proposal_page = org
        .list_document_revision_proposals(document_id, "owner", None, 1)
        .await
        .unwrap();
    assert_eq!(proposal_page.items.len(), 1);
    assert_eq!(proposal_page.items[0].id, proposal.id);
    assert!(matches!(
        org.list_document_revision_proposals(document_id, "blair", None, 10)
            .await,
        Err(DocumentError::Unavailable)
    ));

    let owner_edit = document_content("claim", "Owner changed the claim first");
    let advanced = org
        .create_named_document_version(NewNamedDocumentVersion {
            command_id: Uuid::new_v4(),
            document_id,
            actor_id: "owner",
            expected_current_version_id: initial_version_id,
            content_json: &owner_edit,
            reason: "Owner revision",
        })
        .await
        .unwrap();
    let stale_command = Uuid::new_v4();
    let stale = org
        .resolve_document_revision_proposal(ResolveDocumentRevisionProposal {
            document_id,
            proposal_id: proposal.id,
            actor_id: "owner",
            command_id: stale_command,
            expected_proposal_version: 1,
            decision: DocumentRevisionDecision::Accept,
            resolution_summary: "Accept if it still applies cleanly",
            accepted_version_name: Some("Agent revision"),
        })
        .await
        .unwrap();
    assert_eq!(stale.proposal.status, DocumentRevisionStatus::Stale);
    assert!(stale.accepted_version.is_none());
    let after_stale = org
        .get_document_for_actor(document_id, "owner")
        .await
        .unwrap();
    assert_eq!(after_stale.current_version.version.id, advanced.result_id);
    let stale_replay = org
        .resolve_document_revision_proposal(ResolveDocumentRevisionProposal {
            document_id,
            proposal_id: proposal.id,
            actor_id: "owner",
            command_id: stale_command,
            expected_proposal_version: 1,
            decision: DocumentRevisionDecision::Accept,
            resolution_summary: "Accept if it still applies cleanly",
            accepted_version_name: Some("Agent revision"),
        })
        .await
        .unwrap();
    assert_eq!(stale_replay.proposal.status, DocumentRevisionStatus::Stale);
    let creation_after_stale = org
        .propose_document_revision(ProposeDocumentRevision {
            document_id,
            actor_id: "research-analyst",
            command_id: proposal_command,
            base_version_id: initial_version_id,
            scope: DocumentRevisionScope::Block,
            block_id: Some("claim"),
            proposed_content_json: &proposed_block,
            summary: "Replace the claim with the evidence-backed wording",
        })
        .await
        .unwrap();
    assert_eq!(
        creation_after_stale.status,
        DocumentRevisionStatus::Proposed
    );
    assert_eq!(creation_after_stale.version, 1);
    assert!(creation_after_stale.resolved_by_actor_id.is_none());

    let whole = document_content("claim", "A fresh agent proposal");
    let fresh = org
        .propose_document_revision(ProposeDocumentRevision {
            document_id,
            actor_id: "research-analyst",
            command_id: Uuid::new_v4(),
            base_version_id: advanced.result_id,
            scope: DocumentRevisionScope::WholeDocument,
            block_id: None,
            proposed_content_json: &whole,
            summary: "Fresh proposal against the current checkpoint",
        })
        .await
        .unwrap();
    assert!(matches!(
        org.resolve_document_revision_proposal(ResolveDocumentRevisionProposal {
            document_id,
            proposal_id: fresh.id,
            actor_id: "research-analyst",
            command_id: Uuid::new_v4(),
            expected_proposal_version: 1,
            decision: DocumentRevisionDecision::Accept,
            resolution_summary: "Self-accept the agent proposal",
            accepted_version_name: Some("Self-accepted revision"),
        })
        .await,
        Err(DocumentError::Unavailable)
    ));
    let accepted = org
        .resolve_document_revision_proposal(ResolveDocumentRevisionProposal {
            document_id,
            proposal_id: fresh.id,
            actor_id: "owner",
            command_id: Uuid::new_v4(),
            expected_proposal_version: 1,
            decision: DocumentRevisionDecision::Accept,
            resolution_summary: "Evidence checked",
            accepted_version_name: Some("Evidence-checked revision"),
        })
        .await
        .unwrap();
    assert_eq!(accepted.proposal.status, DocumentRevisionStatus::Accepted);
    let accepted_version_id = accepted.accepted_version.as_ref().unwrap().version.id;
    assert_eq!(
        accepted
            .accepted_version
            .as_ref()
            .unwrap()
            .version
            .plain_text,
        "A fresh agent proposal"
    );
    let history = org
        .get_document_version_for_actor(document_id, initial_version_id, "owner")
        .await
        .unwrap();
    assert_eq!(history.version.plain_text, "Original claim");

    let rejected_proposal = org
        .propose_document_revision(ProposeDocumentRevision {
            document_id,
            actor_id: "research-analyst",
            command_id: Uuid::new_v4(),
            base_version_id: accepted_version_id,
            scope: DocumentRevisionScope::WholeDocument,
            block_id: None,
            proposed_content_json: &document_content("claim", "Not good enough"),
            summary: "A proposal the reviewer should reject",
        })
        .await
        .unwrap();
    let rejected = org
        .resolve_document_revision_proposal(ResolveDocumentRevisionProposal {
            document_id,
            proposal_id: rejected_proposal.id,
            actor_id: "owner",
            command_id: Uuid::new_v4(),
            expected_proposal_version: 1,
            decision: DocumentRevisionDecision::Reject,
            resolution_summary: "The evidence does not support this wording",
            accepted_version_name: None,
        })
        .await
        .unwrap();
    assert_eq!(rejected.proposal.status, DocumentRevisionStatus::Rejected);
    assert!(rejected.accepted_version.is_none());
    assert_eq!(
        org.get_document_for_actor(document_id, "owner")
            .await
            .unwrap()
            .current_version
            .version
            .id,
        accepted_version_id
    );

    let human_attempt = org
        .propose_document_revision(ProposeDocumentRevision {
            document_id,
            actor_id: "owner",
            command_id: Uuid::new_v4(),
            base_version_id: accepted_version_id,
            scope: DocumentRevisionScope::WholeDocument,
            block_id: None,
            proposed_content_json: &whole,
            summary: "Humans edit directly; they do not impersonate agent proposals",
        })
        .await;
    assert!(matches!(human_attempt, Err(DocumentError::Unavailable)));

    let mut connection = connection_for(&org).await;
    let event: Value = sqlx::query_scalar(
        "SELECT body FROM events WHERE document_id=$1 \
         AND kind='document.revision.proposed.v1' AND body->>'proposal_id'=$2",
    )
    .bind(document_id)
    .bind(proposal.id.to_string())
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(event["base_named_version_id"], json!(initial_version_id));
    assert_eq!(event["proposed_by_actor_id"], "research-analyst");
    assert_eq!(event["command_id"], json!(proposal_command));
    let proposed_event_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM events WHERE document_id=$1 \
         AND kind='document.revision.proposed.v1' AND body->>'proposal_id'=$2",
    )
    .bind(document_id)
    .bind(proposal.id.to_string())
    .fetch_one(&mut connection)
    .await
    .unwrap();
    assert_eq!(
        proposed_event_count, 1,
        "proposal replay duplicated its event"
    );
    let proposal_tamper = sqlx::query(
        "UPDATE native_document_revision_proposals SET summary='rewritten' WHERE id=$1",
    )
    .bind(proposal.id)
    .execute(&mut connection)
    .await;
    assert!(proposal_tamper.is_err());
}
