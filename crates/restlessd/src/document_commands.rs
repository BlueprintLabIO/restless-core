//! Actor-pinned native document operations shared by installed harnesses.
//! Named snapshots are explicitly distinct from the live collaborative body.
use anyhow::Result;
use restless_orgintel::*;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(
    tag = "format",
    content = "body",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(crate) enum DocumentContent {
    Markdown(String),
    Json(Value),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum DocumentOperation {
    List {
        #[serde(default)]
        cursor: Option<DocumentListCursor>,
        #[serde(default)]
        archived: bool,
    },
    Search {
        query: String,
        #[serde(default)]
        offset: i64,
        #[serde(default)]
        archived: bool,
    },
    Create {
        title: String,
        kind: DocumentKind,
        visibility: DocumentVisibility,
        content: DocumentContent,
        reason: String,
        key: Uuid,
    },
    Read {
        document: Uuid,
    },
    Edit {
        document: Uuid,
        operations: Value,
        key: Uuid,
    },
    Checkpoint {
        document: Uuid,
        checkpoint: Uuid,
        content: Value,
        reason: String,
        key: Uuid,
    },
    Snapshot {
        document: Uuid,
        #[serde(default)]
        version: Option<Uuid>,
    },
    Versions {
        document: Uuid,
        #[serde(default)]
        cursor: Option<DocumentVersionCursor>,
    },
    Participants {
        document: Uuid,
        #[serde(default)]
        cursor: Option<DocumentParticipantCursor>,
    },
    Share {
        document: Uuid,
        member: String,
        access: DocumentAccess,
        revision: i64,
        key: Uuid,
    },
    Unshare {
        document: Uuid,
        member: String,
        revision: i64,
        key: Uuid,
    },
    Threads {
        document: Uuid,
        #[serde(default)]
        cursor: Option<DocumentPageCursor>,
    },
    Comments {
        document: Uuid,
        thread: Uuid,
        #[serde(default)]
        cursor: Option<DocumentPageCursor>,
    },
    Comment {
        document: Uuid,
        #[serde(default)]
        block: Option<String>,
        body: String,
        key: Uuid,
    },
    Reply {
        document: Uuid,
        thread: Uuid,
        #[serde(default)]
        parent: Option<Uuid>,
        body: String,
        key: Uuid,
    },
    Resolve {
        document: Uuid,
        thread: Uuid,
        revision: i64,
        key: Uuid,
    },
}

fn comment_content(key: Uuid, body: String) -> Value {
    json!({
        "type": "doc",
        "content": [{
            "type": "paragraph",
            "attrs": {"block_id": format!("comment-{key}")},
            "content": [{"type": "text", "text": body}],
        }],
    })
}

pub(crate) async fn execute_with_body(
    root: &std::path::Path,
    org: &OrgIntel,
    actor: &str,
    operation: DocumentOperation,
) -> Result<Value> {
    match operation {
        DocumentOperation::Read { document } => {
            crate::owner::agent_document_body(root, org, actor, document, &json!({"action":"read"}))
                .await
        }
        DocumentOperation::Edit {
            document,
            operations,
            key,
        } => {
            let request = json!({"operations":operations});
            let existing = org
                .document_edit_command(document, actor, key, &request, None)
                .await?;
            let selected = match existing {
                Some(edit) => edit,
                None => {
                    let prepared = crate::owner::agent_document_body(
                        root,
                        org,
                        actor,
                        document,
                        &json!({"action":"prepare","operations":request["operations"]}),
                    )
                    .await?;
                    org.document_edit_command(document, actor, key, &request, Some(&prepared))
                        .await?
                        .ok_or_else(|| anyhow::anyhow!("prepared edit was not retained"))?
                }
            };
            if let Some(result) = selected.result_json {
                return Ok(result);
            }
            let result = crate::owner::agent_document_body(
                root,
                org,
                actor,
                document,
                &json!({"action":"apply","checkpoint_id":selected.prepared_json["checkpoint_id"],
                    "update_base64":selected.prepared_json["update_base64"]}),
            )
            .await?;
            org.complete_document_edit_command(document, actor, key, &result)
                .await
                .map_err(Into::into)
        }
        DocumentOperation::Comment {
            document,
            block,
            body,
            key,
        } => {
            if block.is_some() {
                crate::owner::agent_document_body(
                    root,
                    org,
                    actor,
                    document,
                    &json!({"action":"read"}),
                )
                .await?;
            }
            execute(
                org,
                actor,
                DocumentOperation::Comment {
                    document,
                    block,
                    body,
                    key,
                },
            )
            .await
        }
        other => execute(org, actor, other).await,
    }
}

pub(crate) async fn execute(
    org: &OrgIntel,
    actor: &str,
    operation: DocumentOperation,
) -> Result<Value> {
    Ok(match operation {
        DocumentOperation::Checkpoint {
            document,
            checkpoint,
            content,
            reason,
            key,
        } => json!(
            org.create_named_document_version(NewNamedDocumentVersion {
                document_id: document,
                command_id: key,
                actor_id: actor,
                expected_current_version_id: checkpoint,
                content_json: &content,
                reason: &reason,
            })
            .await?
        ),
        DocumentOperation::Read { .. } | DocumentOperation::Edit { .. } => {
            anyhow::bail!("live body requires the document service")
        }
        DocumentOperation::List { cursor, archived } => json!(
            org.list_documents_for_actor(actor, archived, cursor.as_ref(), 50)
                .await?
        ),
        DocumentOperation::Search {
            query,
            offset,
            archived,
        } => json!(
            org.search_documents_for_actor(actor, &query, archived, offset, 50)
                .await?
        ),
        DocumentOperation::Create {
            title,
            kind,
            visibility,
            content,
            reason,
            key,
        } => {
            let content = match content {
                DocumentContent::Markdown(body) => markdown_body_to_document(&body)?,
                DocumentContent::Json(value) => value,
            };
            json!(
                org.create_document(NewDocument {
                    command_id: key,
                    title: &title,
                    kind,
                    visibility,
                    linked_room_id: None,
                    inherit_room_visibility: false,
                    owner_actor_id: actor,
                    created_by_actor_id: actor,
                    content_json: &content,
                    reason: &reason,
                })
                .await?
            )
        }
        DocumentOperation::Snapshot { document, version } => {
            let read = org.get_document_for_actor(document, actor).await?;
            let snapshot = match version {
                Some(version) => {
                    org.get_document_version_for_actor(document, version, actor)
                        .await?
                }
                None => read.current_version,
            };
            json!({"document":read.document,"access":read.access,"named_version":snapshot,
                "body_source":"named_version", "live_body_included":false})
        }
        DocumentOperation::Versions { document, cursor } => json!(
            org.list_document_versions_for_actor(document, actor, cursor.as_ref(), 25)
                .await?
        ),
        DocumentOperation::Participants { document, cursor } => json!(
            org.list_document_participants_for_actor(document, actor, cursor.as_ref(), 50)
                .await?
        ),
        DocumentOperation::Share {
            document,
            member,
            access,
            revision,
            key,
        } => json!(
            org.set_document_participant(SetDocumentParticipant {
                command_id: key,
                document_id: document,
                actor_id: actor,
                expected_document_version: revision,
                participant_actor_id: &member,
                access,
            })
            .await?
        ),
        DocumentOperation::Unshare {
            document,
            member,
            revision,
            key,
        } => json!(
            org.remove_document_participant(RemoveDocumentParticipant {
                command_id: key,
                document_id: document,
                actor_id: actor,
                expected_document_version: revision,
                participant_actor_id: &member,
            })
            .await?
        ),
        DocumentOperation::Threads { document, cursor } => json!(
            org.list_document_comment_threads(document, actor, cursor.as_ref(), 50)
                .await?
        ),
        DocumentOperation::Comments {
            document,
            thread,
            cursor,
        } => json!(
            org.list_document_comments(document, thread, actor, cursor.as_ref(), 50)
                .await?
        ),
        DocumentOperation::Comment {
            document,
            block,
            body,
            key,
        } => json!(
            org.create_document_comment_thread(NewDocumentCommentThread {
                document_id: document,
                actor_id: actor,
                command_id: key,
                block_id: block.as_deref(),
                content_json: &comment_content(key, body),
            })
            .await?
        ),
        DocumentOperation::Reply {
            document,
            thread,
            parent,
            body,
            key,
        } => json!(
            org.reply_to_document_comment(ReplyToDocumentComment {
                document_id: document,
                thread_id: thread,
                reply_to_comment_id: parent,
                actor_id: actor,
                command_id: key,
                content_json: &comment_content(key, body),
            })
            .await?
        ),
        DocumentOperation::Resolve {
            document,
            thread,
            revision,
            key,
        } => json!(
            org.resolve_document_comment_thread(ResolveDocumentCommentThread {
                document_id: document,
                thread_id: thread,
                actor_id: actor,
                command_id: key,
                expected_thread_version: revision,
            })
            .await?
        ),
    })
}
