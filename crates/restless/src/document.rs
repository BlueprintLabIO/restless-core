//! CLI serialization only; Core pins the actor and enforces document access.
use anyhow::{bail, Context, Result};
use clap::Subcommand;
use serde_json::{json, Value};
use std::{io::Read, path::PathBuf};

#[derive(Subcommand)]
pub(crate) enum DocumentCommand {
    /// List accessible documents; pass next_cursor unchanged as --cursor JSON.
    List {
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long)]
        archived: bool,
    },
    /// Search accessible named document versions.
    Search {
        query: String,
        #[arg(long, default_value_t = 0)]
        offset: i64,
        #[arg(long)]
        archived: bool,
    },
    /// Create a document from Markdown or restricted editor JSON.
    Create {
        #[arg(long)]
        title: String,
        #[arg(
            long,
            default_value = "freeform",
            value_parser = [
                "brief",
                "plan",
                "decision_note",
                "report",
                "review",
                "handbook",
                "operating_note",
                "freeform"
            ]
        )]
        kind: String,
        #[arg(
            long,
            default_value = "participants",
            value_parser = ["company", "participants"]
        )]
        visibility: String,
        #[arg(
            long,
            required_unless_present = "content_json_file",
            conflicts_with = "content_json_file"
        )]
        markdown_file: Option<PathBuf>,
        #[arg(long, required_unless_present = "markdown_file")]
        content_json_file: Option<PathBuf>,
        #[arg(long, default_value = "Initial document")]
        reason: String,
        /// Stable UUID; reuse the same key and content on retry.
        #[arg(long)]
        key: String,
    },
    /// Read the current shared body and block hashes for guarded edits.
    Read { document: String },
    /// Apply and persist live edits immediately; no checkpoint or approval is needed.
    #[command(after_long_help = r#"WORKFLOW
  restless document read DOCUMENT > before.json
  Write operations.json using content_json and the observed blocks[].hash values.
  restless document edit DOCUMENT --operations-file operations.json --key UUID
  restless document read DOCUMENT
  Verify the live result before reporting completion. A checkpoint is optional version history,
  not a Save button. Successful edit returns only after applying and persisting the live update.

OPERATIONS FILE
  A JSON array of 1 to 100 operations (not an object containing "operations").
  Insert after an existing block ID; null means the beginning:
    [{"op":"insert","after":null,"block":{"type":"paragraph","attrs":{"block_id":"NEW_UNIQUE_ID"},"content":[{"type":"text","text":"New paragraph"}]}}]
  Replace using the exact observed hash and preserving the target block ID:
    [{"op":"replace","block_id":"BLOCK_ID","expected_hash":"SHA256_FROM_READ","block":{"type":"paragraph","attrs":{"block_id":"BLOCK_ID"},"content":[{"type":"text","text":"Revised paragraph"}]}}]
  Delete using the exact observed hash:
    [{"op":"delete","block_id":"BLOCK_ID","expected_hash":"SHA256_FROM_READ"}]

CONTENT AND CONFLICTS
  Blocks use the editor JSON returned by read. Preserve untouched nodes and attributes.
  Each block needs a unique attrs.block_id (including nested block nodes); text nodes do not.
  Common blocks: paragraph, heading (attrs.level 1-6), bulletList/orderedList with listItem
  children containing paragraphs, blockquote, codeBlock (attrs.language), horizontalRule.
  Inline text: {"type":"text","text":"..."}; optional marks include bold, italic, strike, code.
  Copy an existing block when changing text rather than reconstructing unfamiliar structure.
  A block can be edited only once in one command. All guards are checked before any update.
  A stale hash, missing anchor, or changed checkpoint is a conflict, not permission to overwrite.
  Reread the live body, preserve collaborators' changes, then submit revised operations with a
  new UUID. After a lost response or transport failure, retry the SAME file and UUID: Core
  retains the exact prepared CRDT update and does not apply the edit twice.
  --operations-file /dev/stdin accepts piped JSON on Linux.
"#)]
    Edit {
        document: String,
        #[arg(long)]
        operations_file: PathBuf,
        #[arg(long)]
        key: String,
    },
    /// Save the exact body from `read` as a named version; concurrent changes cause a conflict.
    Checkpoint {
        document: String,
        /// Unmodified JSON output saved from `document read`.
        #[arg(long)]
        snapshot_file: PathBuf,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        key: String,
    },
    /// Read an immutable named snapshot, not uncheckpointed live edits.
    Snapshot {
        document: String,
        #[arg(long)]
        version: Option<String>,
    },
    Versions {
        document: String,
        #[arg(long)]
        cursor: Option<String>,
    },
    Participants {
        document: String,
        #[arg(long)]
        cursor: Option<String>,
    },
    /// Give a colleague access; requires ownership and current metadata revision.
    Share {
        document: String,
        member: String,
        #[arg(long, value_parser=["read","comment","edit"])]
        access: String,
        #[arg(long)]
        revision: i64,
        #[arg(long)]
        key: String,
    },
    Unshare {
        document: String,
        member: String,
        #[arg(long)]
        revision: i64,
        #[arg(long)]
        key: String,
    },
    Threads {
        document: String,
        #[arg(long)]
        cursor: Option<String>,
    },
    Comments {
        document: String,
        thread: String,
        #[arg(long)]
        cursor: Option<String>,
    },
    Comment {
        document: String,
        body: String,
        #[arg(long)]
        block: Option<String>,
        #[arg(long)]
        key: String,
    },
    Reply {
        document: String,
        thread: String,
        body: String,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        key: String,
    },
    Resolve {
        document: String,
        thread: String,
        #[arg(long)]
        revision: i64,
        #[arg(long)]
        key: String,
    },
    /// Pause this exact Attempt's Work for a named-version human review.
    RequestReview {
        #[arg(long)]
        document: String,
        #[arg(long = "document-revision")]
        document_revision: i64,
        #[arg(long = "named-version")]
        named_version: String,
        #[arg(long)]
        work: String,
        #[arg(long)]
        attempt: String,
        #[arg(long = "work-revision")]
        work_revision: i64,
        #[arg(long)]
        reviewer: String,
        #[arg(long)]
        summary: String,
        /// Stable UUID chosen before the first request; retry it unchanged.
        #[arg(long)]
        key: String,
    },
}

impl DocumentCommand {
    pub(crate) fn request(self, company: String) -> Result<Value> {
        let operation = match self {
            Self::List { cursor, archived } => {
                json!({"operation":"list","cursor":decode_cursor(cursor)?,"archived":archived})
            }
            Self::Search {
                query,
                offset,
                archived,
            } => json!({"operation":"search","query":query,"offset":offset,"archived":archived}),
            Self::Create {
                title,
                kind,
                visibility,
                markdown_file,
                content_json_file,
                reason,
                key,
            } => {
                let content = match (markdown_file, content_json_file) {
                    (Some(path), None) => json!({"format":"markdown","body":content_file(path)?}),
                    (None, Some(path)) => {
                        let body = serde_json::from_str::<Value>(&content_file(path)?)
                            .context("parse document JSON")?;
                        json!({"format":"json","body":body})
                    }
                    _ => bail!("choose exactly one document content file"),
                };
                json!({
                    "operation": "create",
                    "title": title,
                    "kind": kind,
                    "visibility": visibility,
                    "content": content,
                    "reason": reason,
                    "key": key,
                })
            }
            Self::Read { document } => json!({"operation":"read","document":document}),
            Self::Edit {
                document,
                operations_file,
                key,
            } => {
                let operations: Value = serde_json::from_str(&content_file(operations_file)?)
                    .context("parse edit operations")?;
                json!({"operation":"edit","document":document,"operations":operations,"key":key})
            }
            Self::Checkpoint {
                document,
                snapshot_file,
                reason,
                key,
            } => {
                let snapshot: Value = serde_json::from_str(&content_file(snapshot_file)?)
                    .context("parse live document snapshot")?;
                let checkpoint = snapshot
                    .get("checkpoint_id")
                    .and_then(Value::as_str)
                    .context(
                        "snapshot must be the JSON output of document read (checkpoint_id missing)",
                    )?;
                let content = snapshot
                    .get("content_json")
                    .filter(|value| value.is_object())
                    .context(
                        "snapshot must be the JSON output of document read (content_json missing)",
                    )?;
                json!({"operation":"checkpoint","document":document,"checkpoint":checkpoint,
                    "content":content,"reason":reason,"key":key})
            }
            Self::Snapshot { document, version } => {
                json!({"operation":"snapshot","document":document,"version":version})
            }
            Self::Versions { document, cursor } => {
                json!({"operation":"versions","document":document,"cursor":decode_cursor(cursor)?})
            }
            Self::Participants { document, cursor } => {
                json!({"operation":"participants","document":document,"cursor":decode_cursor(cursor)?})
            }
            Self::Share {
                document,
                member,
                access,
                revision,
                key,
            } => {
                json!({
                    "operation": "share",
                    "document": document,
                    "member": member,
                    "access": access,
                    "revision": revision,
                    "key": key,
                })
            }
            Self::Unshare {
                document,
                member,
                revision,
                key,
            } => {
                json!({"operation":"unshare","document":document,"member":member,"revision":revision,"key":key})
            }
            Self::Threads { document, cursor } => {
                json!({"operation":"threads","document":document,"cursor":decode_cursor(cursor)?})
            }
            Self::Comments {
                document,
                thread,
                cursor,
            } => {
                json!({"operation":"comments","document":document,"thread":thread,"cursor":decode_cursor(cursor)?})
            }
            Self::Comment {
                document,
                body,
                block,
                key,
            } => {
                json!({"operation":"comment","document":document,"body":body,"block":block,"key":key})
            }
            Self::Reply {
                document,
                thread,
                body,
                parent,
                key,
            } => {
                json!({"operation":"reply","document":document,"thread":thread,"body":body,"parent":parent,"key":key})
            }
            Self::Resolve {
                document,
                thread,
                revision,
                key,
            } => {
                json!({"operation":"resolve","document":document,"thread":thread,"revision":revision,"key":key})
            }
            Self::RequestReview {
                document,
                document_revision,
                named_version,
                work,
                attempt,
                work_revision,
                reviewer,
                summary,
                key,
            } => {
                return Ok(json!({
                    "cmd":"document-review-request", "company":company, "document_id":document,
                    "expected_document_version":document_revision,"named_version_id":named_version,
                    "document_work_id":work,"document_attempt_id":attempt,"expected_work_revision":work_revision,
                    "reviewer_actor_id":reviewer,"review_summary":summary,"document_command_id":key,
                }));
            }
        };
        Ok(json!({"cmd":"document-operation","company":company,"document_operation":operation}))
    }
}

fn decode_cursor(raw: Option<String>) -> Result<Option<Value>> {
    raw.map(|raw| serde_json::from_str(&raw).context("parse --cursor as returned next_cursor JSON"))
        .transpose()
}

fn content_file(path: PathBuf) -> Result<String> {
    const LIMIT: u64 = 1_100_000;
    let mut raw = String::new();
    std::fs::File::open(path)?
        .take(LIMIT + 1)
        .read_to_string(&mut raw)?;
    if raw.is_empty() || raw.len() as u64 > LIMIT {
        bail!("document content file must contain 1 to {LIMIT} UTF-8 bytes");
    }
    Ok(raw)
}
