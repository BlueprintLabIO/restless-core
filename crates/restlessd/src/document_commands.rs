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
    RequestCollaboration {
        document: Uuid,
        summary: String,
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
        #[serde(default)]
        mentions: Vec<String>,
        key: Uuid,
    },
    Reply {
        document: Uuid,
        thread: Uuid,
        #[serde(default)]
        parent: Option<Uuid>,
        body: String,
        #[serde(default)]
        mentions: Vec<String>,
        key: Uuid,
    },
    Resolve {
        document: Uuid,
        thread: Uuid,
        revision: i64,
        key: Uuid,
    },
}

fn comment_content(key: Uuid, body: String, mentions: Vec<String>) -> Value {
    let mut content = vec![json!({"type":"text","text":body})];
    for actor in mentions {
        content.push(json!({"type":"text","text":" "}));
        content.push(json!({"type":"mention","attrs":{"actor_id":actor,"label":actor}}));
    }
    json!({"type":"doc","content":[{"type":"paragraph","attrs":{"block_id":format!("comment-{key}")},"content":content}]})
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
            mentions,
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
                    mentions,
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
        DocumentOperation::RequestCollaboration {
            document,
            summary,
            key,
        } => json!(
            org.request_document_collaboration(document, actor, key, &summary)
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
                DocumentContent::Markdown(body) => document_json_from_markdown(&body)?,
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
            mentions,
            key,
        } => json!(
            org.create_document_comment_thread(NewDocumentCommentThread {
                document_id: document,
                actor_id: actor,
                command_id: key,
                block_id: block.as_deref(),
                content_json: &comment_content(key, body, mentions),
            })
            .await?
        ),
        DocumentOperation::Reply {
            document,
            thread,
            parent,
            body,
            mentions,
            key,
        } => json!(
            org.reply_to_document_comment(ReplyToDocumentComment {
                document_id: document,
                thread_id: thread,
                reply_to_comment_id: parent,
                actor_id: actor,
                command_id: key,
                content_json: &comment_content(key, body, mentions),
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

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;

    #[test]
    fn operation_cannot_override_authenticated_actor() {
        let operation = json!({"operation":"snapshot","document":Uuid::new_v4(),"actor":"owner"});
        assert!(serde_json::from_value::<DocumentOperation>(operation).is_err());
        let content = json!({"format":"markdown","body":"A brief","actor":"owner"});
        assert!(serde_json::from_value::<DocumentContent>(content).is_err());
    }

    #[test]
    fn markdown_creation_is_retry_stable_and_rejects_unsupported_content() {
        let body = "# Launch plan\n\nA **shared** plan with [reference](https://example.com).\n\n- First\n- Second\n\n```rs\nlet x = 1;\n```\n";
        let first = document_json_from_markdown(body).unwrap();
        assert_eq!(first, document_json_from_markdown(body).unwrap());
        assert_eq!(first["content"][0]["type"], "heading");
        let literal = document_json_from_markdown("<script>unsafe</script>").unwrap();
        assert_eq!(
            literal["content"][0]["content"][0],
            json!({"type":"text","text":"<script>unsafe</script>"})
        );
        assert!(document_json_from_markdown("```rs\nunclosed").is_err());
        assert!(document_json_from_markdown("a\0b").is_err());
    }

    #[tokio::test]
    async fn document_commands_preserve_access_retries_and_exact_comment_threads() {
        let Ok(url) = std::env::var("RESTLESS_TEST_DATABASE_URL") else {
            return;
        };
        let schema = format!("doctools{}_test", Uuid::new_v4().simple());
        let org = OrgIntel::ensure(&url, &schema).await.unwrap();
        let outcome: Result<()> = async {
            org.ensure_actor("owner", "owner", "owner", "Owner").await?;
            org.ensure_actor("evidence-writer", "staff", "writer", "Writer")
                .await?;
            org.ensure_actor("evidence-reader", "staff", "reader", "Reader")
                .await?;
            let key = Uuid::new_v4();
            let create = || DocumentOperation::Create {
                title: "Launch brief".into(),
                kind: DocumentKind::Brief,
                visibility: DocumentVisibility::Participants,
                content: DocumentContent::Markdown("# Brief\n\nA shared positioning draft.".into()),
                reason: "Initial brief".into(),
                key,
            };
            let created = execute(&org, "evidence-writer", create()).await?;
            let replay = execute(&org, "evidence-writer", create()).await?;
            anyhow::ensure!(created == replay, "create retry changed its durable result");
            let document = Uuid::parse_str(
                created["document_id"]
                    .as_str()
                    .context("created document")?,
            )?;
            // A racing prepare must choose one durable delta, and retries must
            // never regenerate it after the shared body has changed.
            let edit_key = Uuid::new_v4();
            let edit_request =
                json!({"operations":[{"op":"insert","after":null,"block":{"type":"paragraph"}}]});
            let delta_a = json!({"checkpoint_id":document,"update_base64":"delta-a"});
            let delta_b = json!({"checkpoint_id":document,"update_base64":"delta-b"});
            let (first, second) = tokio::join!(
                org.document_edit_command(
                    document,
                    "evidence-writer",
                    edit_key,
                    &edit_request,
                    Some(&delta_a)
                ),
                org.document_edit_command(
                    document,
                    "evidence-writer",
                    edit_key,
                    &edit_request,
                    Some(&delta_b)
                ),
            );
            let selected = first?.context("first prepared edit")?;
            anyhow::ensure!(
                selected.prepared_json == second?.context("second prepared edit")?.prepared_json,
                "concurrent prepares selected different deltas"
            );
            anyhow::ensure!(
                org.document_edit_command(
                    document,
                    "evidence-writer",
                    edit_key,
                    &json!({"operations":[]}),
                    None
                )
                .await
                .is_err(),
                "changed request replayed edit key"
            );
            anyhow::ensure!(
                org.document_edit_command(
                    document,
                    "evidence-reader",
                    edit_key,
                    &edit_request,
                    None
                )
                .await
                .is_err(),
                "private prepared delta leaked"
            );
            let applied = json!({"status":"applied","checkpoint_id":document});
            anyhow::ensure!(
                org.complete_document_edit_command(document, "evidence-writer", edit_key, &applied)
                    .await?
                    == applied
            );
            anyhow::ensure!(
                org.complete_document_edit_command(
                    document,
                    "evidence-writer",
                    edit_key,
                    &json!({"status":"different"})
                )
                .await?
                    == applied,
                "retry replaced completed result"
            );
            let durable = org
                .document_edit_command(document, "evidence-writer", edit_key, &edit_request, None)
                .await?
                .context("retained edit")?;
            anyhow::ensure!(
                durable.prepared_json == selected.prepared_json
                    && durable.result_json == Some(applied)
            );
            let forged_replay = execute(&org, "evidence-reader", create()).await;
            anyhow::ensure!(
                forged_replay.is_err(),
                "another actor replayed the creator's command"
            );
            anyhow::ensure!(
                execute(
                    &org,
                    "evidence-reader",
                    DocumentOperation::Snapshot {
                        document,
                        version: None
                    }
                )
                .await
                .is_err(),
                "private document leaked"
            );
            let hidden = execute(
                &org,
                "evidence-reader",
                DocumentOperation::List {
                    cursor: None,
                    archived: false,
                },
            )
            .await?;
            anyhow::ensure!(
                hidden["items"]
                    .as_array()
                    .context("document list")?
                    .is_empty(),
                "private document appeared in discovery"
            );
            let snapshot = execute(
                &org,
                "evidence-writer",
                DocumentOperation::Snapshot {
                    document,
                    version: None,
                },
            )
            .await?;
            anyhow::ensure!(
                snapshot["body_source"] == "named_version"
                    && snapshot["live_body_included"] == false,
                "checkpoint misrepresented as live body"
            );
            let revision = snapshot["document"]["version"]
                .as_i64()
                .context("document revision")?;
            let share_key = Uuid::new_v4();
            let share = || DocumentOperation::Share {
                document,
                member: "evidence-reader".into(),
                access: DocumentAccess::Comment,
                revision,
                key: share_key,
            };
            let granted = execute(&org, "evidence-writer", share()).await?;
            anyhow::ensure!(
                granted == execute(&org, "evidence-writer", share()).await?,
                "share retry changed result"
            );
            anyhow::ensure!(
                execute(
                    &org,
                    "evidence-reader",
                    DocumentOperation::Share {
                        document,
                        member: "owner".into(),
                        access: DocumentAccess::Edit,
                        revision: revision + 1,
                        key: Uuid::new_v4()
                    }
                )
                .await
                .is_err(),
                "commenter managed membership"
            );
            let snapshot = execute(
                &org,
                "evidence-reader",
                DocumentOperation::Snapshot {
                    document,
                    version: None,
                },
            )
            .await?;
            anyhow::ensure!(snapshot["access"] == "comment", "grant did not take effect");
            let comment_key = Uuid::new_v4();
            let comment = || DocumentOperation::Comment {
                document,
                block: None,
                body: "Please review the positioning".into(),
                mentions: vec!["evidence-writer".into()],
                key: comment_key,
            };
            let first = execute(&org, "evidence-reader", comment()).await?;
            let replay = execute(&org, "evidence-reader", comment()).await?;
            anyhow::ensure!(
                first == replay,
                "comment retry duplicated or changed thread"
            );
            let thread = Uuid::parse_str(
                first["thread"]["thread"]["id"]
                    .as_str()
                    .context("comment thread")?,
            )?;
            let parent = Uuid::parse_str(
                first["first_comment"]["id"]
                    .as_str()
                    .context("first comment")?,
            )?;
            anyhow::ensure!(
                first["first_comment"]["mentioned_actor_ids"] == json!(["evidence-writer"]),
                "mention lost exact actor"
            );
            let reply_key = Uuid::new_v4();
            let reply = || DocumentOperation::Reply {
                document,
                thread,
                parent: Some(parent),
                body: "I will refine the opening.".into(),
                mentions: vec![],
                key: reply_key,
            };
            let sent = execute(&org, "evidence-writer", reply()).await?;
            anyhow::ensure!(
                sent == execute(&org, "evidence-writer", reply()).await?,
                "reply retry duplicated message"
            );
            let replies = execute(
                &org,
                "evidence-reader",
                DocumentOperation::Comments {
                    document,
                    thread,
                    cursor: None,
                },
            )
            .await?;
            let items = replies["items"].as_array().context("comment page")?;
            anyhow::ensure!(
                items.len() == 2 && items[1]["reply_to_comment_id"] == parent.to_string(),
                "reply did not stay in exact thread"
            );
            anyhow::ensure!(
                execute(
                    &org,
                    "evidence-reader",
                    DocumentOperation::Reply {
                        document,
                        thread: Uuid::new_v4(),
                        parent: Some(parent),
                        body: "Wrong thread".into(),
                        mentions: vec![],
                        key: Uuid::new_v4()
                    }
                )
                .await
                .is_err(),
                "reply crossed thread boundary"
            );
            let latest = org
                .get_document_for_actor(document, "evidence-writer")
                .await?;
            execute(
                &org,
                "evidence-writer",
                DocumentOperation::Unshare {
                    document,
                    member: "evidence-reader".into(),
                    revision: latest.document.version,
                    key: Uuid::new_v4(),
                },
            )
            .await?;
            anyhow::ensure!(
                execute(
                    &org,
                    "evidence-reader",
                    DocumentOperation::Comments {
                        document,
                        thread,
                        cursor: None
                    }
                )
                .await
                .is_err(),
                "revoked reader retained comments"
            );
            anyhow::ensure!(
                execute(
                    &org,
                    "evidence-reader",
                    DocumentOperation::Comment {
                        document,
                        block: None,
                        body: "After revocation".into(),
                        mentions: vec![],
                        key: Uuid::new_v4()
                    }
                )
                .await
                .is_err(),
                "revoked member wrote a comment"
            );
            let collaboration_key = Uuid::new_v4();
            anyhow::ensure!(
                org.request_document_collaboration(
                    document,
                    "evidence-writer",
                    collaboration_key,
                    "Work on this together"
                )
                .await
                .is_err(),
                "invitation implicitly granted owner edit access"
            );
            let latest = org
                .get_document_for_actor(document, "evidence-writer")
                .await?;
            execute(
                &org,
                "evidence-writer",
                DocumentOperation::Share {
                    document,
                    member: "owner".into(),
                    access: DocumentAccess::Edit,
                    revision: latest.document.version,
                    key: Uuid::new_v4(),
                },
            )
            .await?;
            let request = org
                .request_document_collaboration(
                    document,
                    "evidence-writer",
                    collaboration_key,
                    "Work on this together",
                )
                .await?;
            anyhow::ensure!(
                org.request_document_collaboration(
                    document,
                    "evidence-writer",
                    collaboration_key,
                    "Work on this together"
                )
                .await?
                .id == request.id
            );
            anyhow::ensure!(org
                .request_document_collaboration(
                    document,
                    "evidence-writer",
                    collaboration_key,
                    "Changed request"
                )
                .await
                .is_err());
            anyhow::ensure!(org.pending_document_attention().await?.len() == 1);
            let latest = org
                .get_document_for_actor(document, "evidence-writer")
                .await?;
            let review = org
                .request_document_review(RequestDocumentReview {
                    document_id: document,
                    actor_id: "evidence-writer",
                    command_id: Uuid::new_v4(),
                    expected_document_version: latest.document.version,
                    expected_current_version_id: latest.current_version.version.id,
                    summary: "Review this exact version",
                    work_dependency: None,
                })
                .await?;
            let queue = org.pending_document_attention().await?;
            anyhow::ensure!(
                queue.len() == 2
                    && queue.iter().any(|item| item.id == review.review.id
                        && item.named_version_id == Some(review.review.requested_version_id))
            );
            anyhow::ensure!(
                org.resolve_document_collaboration(document, "evidence-writer", request.id)
                    .await
                    .is_err(),
                "requester completed the owner's collaboration request"
            );
            let closed = org
                .resolve_document_collaboration(document, "owner", request.id)
                .await?;
            anyhow::ensure!(closed.resolved_at.is_some());
            anyhow::ensure!(
                org.resolve_document_collaboration(document, "owner", request.id)
                    .await?
                    .resolved_at
                    == closed.resolved_at
            );
            anyhow::ensure!(
                org.pending_document_attention().await?.len() == 1,
                "closing collaboration also closed the version review"
            );
            let latest = org
                .get_document_for_actor(document, "evidence-writer")
                .await?;
            execute(
                &org,
                "evidence-writer",
                DocumentOperation::Unshare {
                    document,
                    member: "owner".into(),
                    revision: latest.document.version,
                    key: Uuid::new_v4(),
                },
            )
            .await?;
            anyhow::ensure!(
                org.pending_document_attention().await?.is_empty(),
                "revoked owner could discover the document through Attention"
            );
            org.ensure_actor("exec", "exec", "exec", "Exec").await?;
            let created = execute(
                &org,
                "exec",
                DocumentOperation::Create {
                    title: "Checkpoint contract".into(),
                    kind: DocumentKind::Brief,
                    visibility: DocumentVisibility::Participants,
                    content: DocumentContent::Markdown("An observed body".into()),
                    reason: "Initial".into(),
                    key: Uuid::new_v4(),
                },
            )
            .await?;
            let document =
                Uuid::parse_str(created["document_id"].as_str().context("checkpoint doc")?)?;
            let before = org.get_document_for_actor(document, "exec").await?;
            let checkpoint = before.current_version.version.id;
            let content = before.current_version.version.content_json.clone();
            let key = Uuid::new_v4();
            let save = || DocumentOperation::Checkpoint {
                document,
                checkpoint,
                content: content.clone(),
                reason: "Observed draft".into(),
                key,
            };
            let saved = execute(&org, "exec", save()).await?;
            anyhow::ensure!(
                execute(&org, "exec", save()).await? == saved,
                "checkpoint retry changed identity"
            );
            anyhow::ensure!(
                execute(
                    &org,
                    "exec",
                    DocumentOperation::Checkpoint {
                        document,
                        checkpoint,
                        content: content.clone(),
                        reason: "Different payload".into(),
                        key,
                    }
                )
                .await
                .is_err(),
                "checkpoint accepted conflicting retry"
            );
            anyhow::ensure!(
                execute(
                    &org,
                    "exec",
                    DocumentOperation::Checkpoint {
                        document,
                        checkpoint,
                        content: content.clone(),
                        reason: "Stale checkpoint".into(),
                        key: Uuid::new_v4(),
                    }
                )
                .await
                .is_err(),
                "checkpoint accepted stale version"
            );
            let after = org.get_document_for_actor(document, "exec").await?;
            execute(
                &org,
                "exec",
                DocumentOperation::Share {
                    document,
                    member: "owner".into(),
                    access: DocumentAccess::Edit,
                    revision: after.document.version,
                    key: Uuid::new_v4(),
                },
            )
            .await?;
            let owner_key = Uuid::new_v4();
            let owner_save = || DocumentOperation::Checkpoint {
                document,
                checkpoint: after.current_version.version.id,
                content: content.clone(),
                reason: "Owner checkpoint".into(),
                key: owner_key,
            };
            execute(&org, "owner", owner_save()).await?;
            let latest = org.get_document_for_actor(document, "exec").await?;
            execute(
                &org,
                "exec",
                DocumentOperation::Unshare {
                    document,
                    member: "owner".into(),
                    revision: latest.document.version,
                    key: Uuid::new_v4(),
                },
            )
            .await?;
            anyhow::ensure!(
                execute(&org, "owner", owner_save()).await.is_err(),
                "revoked editor replayed checkpoint receipt"
            );
            // Native mentions retain an exact comment return path and use the
            // same Actor-wide lease as Room replies and ordinary conversation.
            let latest = org.get_document_for_actor(document, "exec").await?;
            execute(
                &org,
                "exec",
                DocumentOperation::Share {
                    document,
                    member: "evidence-reader".into(),
                    access: DocumentAccess::Comment,
                    revision: latest.document.version,
                    key: Uuid::new_v4(),
                },
            )
            .await?;
            let comment_key = Uuid::new_v4();
            let mention = || DocumentOperation::Comment {
                document,
                block: None,
                body: "Can you inspect this paragraph?".into(),
                mentions: vec!["evidence-reader".into()],
                key: comment_key,
            };
            let posted = execute(&org, "exec", mention()).await?;
            anyhow::ensure!(execute(&org, "exec", mention()).await? == posted);
            let observed = org
                .next_pending_document_mention("evidence-reader")
                .await?
                .context("pending document mention")?;
            anyhow::ensure!(
                observed.comment.id.to_string() == posted["first_comment"]["id"].as_str().unwrap()
            );
            let duration = std::time::Duration::from_secs(60);
            let lease = org
                .claim_actor_cognitive_session("evidence-reader", duration)
                .await?
                .context("reader lease")?;
            anyhow::ensure!(org
                .claim_actor_cognitive_session("evidence-reader", duration)
                .await?
                .is_none());
            let claimed = org
                .claim_next_pending_document_mention(&lease)
                .await?
                .context("claimed mention")?;
            anyhow::ensure!(claimed.context.mention.id == observed.mention.id);
            anyhow::ensure!(
                serde_json::to_value(&claimed.context)?["mention"]
                    .get("claim_token")
                    .is_none(),
                "lease token leaked into model context"
            );
            anyhow::ensure!(
                org.claim_next_pending_message_mention(&lease)
                    .await?
                    .is_none(),
                "Room claim replaced document focus"
            );
            let bootstrap=org.actor_context_bootstrap("evidence-reader",ActorContextFocus::DocumentMention{mention_id:observed.mention.id},8).await?;
            anyhow::ensure!(matches!(bootstrap.return_path,ActorContextReturnPath::DocumentThread{document_id,thread_id,reply_to_comment_id,resolves_mention_id} if document_id==document && thread_id==observed.thread.id && reply_to_comment_id==observed.comment.id && resolves_mention_id==observed.mention.id));
            org.record_actor_checkpoint(NewActorCheckpoint {
                client_command_id:"document-mention-memory",actor_id:"evidence-reader",expected_current_version:0,
                session:ActorCheckpointSession::CognitiveSession{lease_token:lease.token},
                body:ActorCheckpointBody {current_objective:"Answer this document comment".into(),current_status:"Inspecting exact paragraph".into(),completed:vec![],material_findings:vec![],decisions_made:vec![],changed_refs:vec![],failed_approaches:vec![],blockers:vec![],next_useful_action:"Reply to original comment".into(),expected_receiver:Some("exec".into())},sources:&[NewActorCheckpointSource { epistemic_kind:ActorContextEpistemicKind::Observation,source:ActorContextSourceRef::Document {document_id:document,named_version_id:observed.thread.anchored_version_id},statement:"Inspect the version anchoring this comment".into(),scope:None,expires_at:None}],
            }).await?;
            let boot=org.actor_context_bootstrap("evidence-reader",ActorContextFocus::DocumentMention{mention_id:observed.mention.id},8).await?;
            anyhow::ensure!(boot.checkpoint.context("document mention checkpoint")?.checkpoint.focused_document_mention_id==Some(observed.mention.id));
            anyhow::ensure!(org.actor_context_bootstrap("evidence-reader",ActorContextFocus::General,8).await?.checkpoint.is_none(),"document mention memory leaked into general context");
            let source=org.current_cognitive_model_invocation_source("evidence-reader",true).await?;
            anyhow::ensure!(matches!(source,ModelInvocationSource::DocumentMention{mention_id,..} if mention_id==observed.mention.id));
            anyhow::ensure!(org.current_cognitive_model_invocation_source("evidence-reader",false).await.is_err(),"document mention was admitted as general conversation");
            let admission=org.admit_model_invocation(NewModelInvocationAdmission {client_command_id:"document-mention-launch",actor_id:"evidence-reader",model:"fake/test",harness:"codex",configured_effort:"medium",source}).await?;
            anyhow::ensure!(matches!(admission,ModelInvocationAdmissionDecision::Admitted(_)),"document mention invocation was not admitted");
            org.renew_actor_cognitive_session(&lease, std::time::Duration::from_secs(1))
                .await?;
            tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
            let replacement = org
                .claim_actor_cognitive_session("evidence-reader", duration)
                .await?
                .context("replacement lease")?;
            let recovered = org
                .claim_next_pending_document_mention(&replacement)
                .await?
                .context("recovered mention")?;
            anyhow::ensure!(recovered.context.mention.id == claimed.context.mention.id);
            anyhow::ensure!(
                !org.release_actor_cognitive_session(&lease).await?,
                "old process released replacement lease"
            );
            anyhow::ensure!(org
                .reply_to_claimed_document_mention(&claimed, "Stale process answer")
                .await
                .is_err());
            let routed=crate::mentions::MentionClaim::Document(recovered.clone());
            anyhow::ensure!(routed.context().prompt().contains(&observed.comment.id.to_string()));
            anyhow::ensure!(routed.reply(&org,"The paragraph is clear.").await?.is_none(),"native comment became an owner conversation message");
            let reply = org
                .reply_to_claimed_document_mention(&recovered, "The paragraph is clear.")
                .await?;
            anyhow::ensure!(
                reply.thread_id == observed.thread.id
                    && reply.reply_to_comment_id == Some(observed.comment.id)
            );
            anyhow::ensure!(reply.author_actor_id == "evidence-reader");
            anyhow::ensure!(
                org.reply_to_claimed_document_mention(&recovered, "The paragraph is clear.")
                    .await?
                    .id
                    == reply.id
            );
            anyhow::ensure!(org
                .reply_to_claimed_document_mention(&recovered, "Different answer")
                .await
                .is_err());
            anyhow::ensure!(
                org.next_pending_document_mention("evidence-reader")
                    .await?
                    .is_none(),
                "reply left duplicate obligations"
            );
            org.release_actor_cognitive_session(&replacement).await?;
            execute(
                &org,
                "exec",
                DocumentOperation::Comment {
                    document,
                    block: None,
                    body: "A second question".into(),
                    mentions: vec!["evidence-reader".into()],
                    key: Uuid::new_v4(),
                },
            )
            .await?;
            let revoked_lease = org
                .claim_actor_cognitive_session("evidence-reader", duration)
                .await?
                .context("revoked lease")?;
            let revoked_claim = org
                .claim_next_pending_document_mention(&revoked_lease)
                .await?
                .context("revoked claim")?;
            let latest = org.get_document_for_actor(document, "exec").await?;
            execute(
                &org,
                "exec",
                DocumentOperation::Unshare {
                    document,
                    member: "evidence-reader".into(),
                    revision: latest.document.version,
                    key: Uuid::new_v4(),
                },
            )
            .await?;
            anyhow::ensure!(
                org.renew_actor_cognitive_session(&revoked_lease, duration)
                    .await
                    .is_err(),
                "revoked mention lease renewed"
            );
            anyhow::ensure!(org
                .reply_to_claimed_document_mention(&revoked_claim, "No longer allowed")
                .await
                .is_err());
            org.release_actor_cognitive_session(&revoked_lease).await?;
            let latest = org.get_document_for_actor(document, "exec").await?;
            execute(
                &org,
                "exec",
                DocumentOperation::Share {
                    document,
                    member: "evidence-reader".into(),
                    access: DocumentAccess::Comment,
                    revision: latest.document.version,
                    key: Uuid::new_v4(),
                },
            )
            .await?;
            anyhow::ensure!(
                org.next_pending_document_mention("evidence-reader")
                    .await?
                    .is_none(),
                "regrant resurrected cancelled mention"
            );
            Ok(())
        }
        .await;
        org.drop_schema().await.unwrap();
        outcome.unwrap();
    }
}
