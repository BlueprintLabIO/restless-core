//! Minimal durable context continuity for one Actor.
//!
//! Checkpoints are immutable Actor-authored working memory.  Their source
//! links point back to authoritative OrgIntel, Runtime, Git, or external
//! evidence and retain epistemic/trust labels; neither a checkpoint nor a
//! bootstrap projection becomes Work, a Room transcript, or Authority.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use sqlx::{Postgres, Row as _, Transaction};
use uuid::Uuid;

use crate::{OrgIntel, OrgIntelError, Result, WorkAttemptState, WorkStatus};

pub const ACTOR_CONTEXT_BOOTSTRAP_SCHEMA: &str = "restless.actor-context.bootstrap";
pub const ACTOR_CONTEXT_BOOTSTRAP_VERSION: u16 = 1;
pub const ACTOR_CHECKPOINT_SCHEMA_VERSION: u16 = 1;

// Keep application admission below the database's 32 KiB JSON-text fence so
// JSONB's normalized whitespace cannot turn an accepted boundary value into a
// database-only rejection.
const MAX_CHECKPOINT_BYTES: usize = 24 * 1024;
const MAX_CHECKPOINT_LIST_ITEMS: usize = 32;
const MAX_CHECKPOINT_ITEM_BYTES: usize = 2_000;
const MAX_CHECKPOINT_SOURCES: usize = 64;
const MAX_SOURCE_STATEMENT_BYTES: usize = 4_000;
const MAX_SOURCE_SCOPE_BYTES: usize = 1_000;
const MAX_SOURCE_PAGE: i64 = 64;
const MAX_CHECKPOINT_PAGE: i64 = 50;
const MAX_BOOTSTRAP_SOURCES: i64 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "actor_checkpoint_session_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ActorCheckpointSessionKind {
    WorkAttempt,
    CognitiveSession,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "actor_context_epistemic_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ActorContextEpistemicKind {
    Observation,
    Claim,
    Hypothesis,
    Assumption,
    Judgment,
    Principle,
    Decision,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "actor_context_source_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ActorContextSourceKind {
    Actor,
    Work,
    Attempt,
    Room,
    Message,
    Decision,
    Artifact,
    Document,
    RuntimeFile,
    GitCommit,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "actor_context_source_trust", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ActorContextSourceTrust {
    CompanyState,
    CompanyDecision,
    AuthenticatedActorInput,
    RuntimeUntrustedEvidence,
    ExternalUntrusted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorContextSourceFreshness {
    Current,
    Stale,
    Missing,
    Inaccessible,
    Expired,
    Unverifiable,
}

/// A stable source coordinate.  It carries identity/version only; source
/// content is retrieved from its owning subsystem when the Actor needs it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActorContextSourceRef {
    Actor {
        actor_id: String,
    },
    Work {
        work_id: Uuid,
        revision: i64,
    },
    Attempt {
        attempt_id: Uuid,
    },
    Room {
        room_id: Uuid,
        through_message_id: i64,
    },
    Message {
        room_id: Uuid,
        message_id: i64,
    },
    Decision {
        decision_id: Uuid,
    },
    Artifact {
        artifact_ref_id: Uuid,
        digest: Option<String>,
    },
    Document {
        document_id: Uuid,
        named_version_id: Uuid,
    },
    RuntimeFile {
        path: String,
        digest: Option<String>,
    },
    GitCommit {
        repository: String,
        commit: String,
    },
    External {
        uri: String,
    },
}

impl ActorContextSourceRef {
    pub fn kind(&self) -> ActorContextSourceKind {
        match self {
            Self::Actor { .. } => ActorContextSourceKind::Actor,
            Self::Work { .. } => ActorContextSourceKind::Work,
            Self::Attempt { .. } => ActorContextSourceKind::Attempt,
            Self::Room { .. } => ActorContextSourceKind::Room,
            Self::Message { .. } => ActorContextSourceKind::Message,
            Self::Decision { .. } => ActorContextSourceKind::Decision,
            Self::Artifact { .. } => ActorContextSourceKind::Artifact,
            Self::Document { .. } => ActorContextSourceKind::Document,
            Self::RuntimeFile { .. } => ActorContextSourceKind::RuntimeFile,
            Self::GitCommit { .. } => ActorContextSourceKind::GitCommit,
            Self::External { .. } => ActorContextSourceKind::External,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorCheckpointBody {
    pub current_objective: String,
    pub current_status: String,
    pub completed: Vec<String>,
    pub material_findings: Vec<String>,
    pub decisions_made: Vec<String>,
    pub changed_refs: Vec<String>,
    pub failed_approaches: Vec<String>,
    pub blockers: Vec<String>,
    pub next_useful_action: String,
    pub expected_receiver: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewActorCheckpointSource {
    pub epistemic_kind: ActorContextEpistemicKind,
    pub source: ActorContextSourceRef,
    pub statement: String,
    pub scope: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorCheckpointSession {
    WorkAttempt { work_id: Uuid, attempt_id: Uuid },
    CognitiveSession { lease_token: Uuid },
}

pub struct NewActorCheckpoint<'a> {
    pub client_command_id: &'a str,
    pub actor_id: &'a str,
    /// Zero means that this Actor must not have a prior checkpoint.
    pub expected_current_version: i64,
    pub session: ActorCheckpointSession,
    pub body: ActorCheckpointBody,
    pub sources: &'a [NewActorCheckpointSource],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorCheckpointRow {
    pub id: Uuid,
    pub actor_id: String,
    pub checkpoint_version: i64,
    pub schema_version: i16,
    pub client_command_id: String,
    pub client_payload_sha256: String,
    pub session_kind: ActorCheckpointSessionKind,
    pub work_id: Option<Uuid>,
    pub attempt_id: Option<Uuid>,
    pub focused_mention_id: Option<Uuid>,
    pub body: ActorCheckpointBody,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorCheckpointSource {
    pub ordinal: i16,
    pub epistemic_kind: ActorContextEpistemicKind,
    pub source_kind: ActorContextSourceKind,
    pub source_trust: ActorContextSourceTrust,
    pub source_author_actor_id: Option<String>,
    pub source: ActorContextSourceRef,
    /// This remains Actor-authored working memory. `source_trust` classifies
    /// the linked source, not this statement as new authority.
    pub statement: String,
    pub scope: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
    pub freshness: ActorContextSourceFreshness,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorCheckpointView {
    pub checkpoint: ActorCheckpointRow,
    pub sources: Vec<ActorCheckpointSource>,
    pub source_count: i64,
    pub sources_truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorCheckpointWriteResult {
    pub checkpoint: ActorCheckpointView,
    pub created: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorContextSourcePage {
    pub sources: Vec<ActorCheckpointSource>,
    pub next_after_ordinal: Option<i16>,
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorContextIdentity {
    pub actor_id: String,
    pub actor_class: String,
    pub display: String,
    pub role: String,
    pub team_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorContextFocus {
    General,
    WorkAttempt { work_id: Uuid, attempt_id: Uuid },
    RoomMention { mention_id: Uuid },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActorContextFocusProjection {
    General,
    WorkAttempt {
        work_id: Uuid,
        work_revision: i64,
        owner_actor_id: String,
        title: String,
        outcome: String,
        status: WorkStatus,
        expected_artifact: String,
        attempt_id: Uuid,
        attempt_no: i32,
        attempt_state: WorkAttemptState,
        input_fingerprint: String,
    },
    RoomMention {
        mention_id: Uuid,
        room_id: Uuid,
        room_title: String,
        message_id: i64,
        thread_root_message_id: i64,
        from_actor_id: String,
        triggering_message: String,
        linked_work_id: Option<Uuid>,
        why_this_actor: Option<String>,
        expected_response: Option<String>,
        affected_scope: Option<String>,
        /// Participant-authored Room content is authenticated attribution,
        /// never an owner Directive or Runtime policy.
        source_trust: ActorContextSourceTrust,
        content_grants_authority: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActorContextReturnPath {
    ActorInbox {
        actor_id: String,
    },
    WorkAttempt {
        work_id: Uuid,
        attempt_id: Uuid,
    },
    RoomThread {
        room_id: Uuid,
        thread_root_message_id: i64,
        reply_to_message_id: i64,
        resolves_mention_id: Uuid,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorContextBootstrap {
    pub schema: String,
    pub version: u16,
    pub generated_at: DateTime<Utc>,
    pub actor: ActorContextIdentity,
    pub focus: ActorContextFocusProjection,
    pub return_path: ActorContextReturnPath,
    /// These coordinates are retrieval links, not copied source bodies.
    pub anchors: Vec<ActorContextSourceRef>,
    pub checkpoint: Option<ActorCheckpointView>,
    pub loaded_stale_checkpoint_sources: usize,
    pub loaded_untrusted_checkpoint_sources: usize,
    pub context_grants_authority: bool,
}

#[derive(sqlx::FromRow)]
struct CheckpointDbRow {
    id: Uuid,
    actor_id: String,
    checkpoint_version: i64,
    schema_version: i16,
    client_command_id: String,
    client_payload_sha256: String,
    session_kind: ActorCheckpointSessionKind,
    work_id: Option<Uuid>,
    attempt_id: Option<Uuid>,
    focused_mention_id: Option<Uuid>,
    body: serde_json::Value,
    recorded_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct CheckpointSourceDbRow {
    ordinal: i16,
    epistemic_kind: ActorContextEpistemicKind,
    source_kind: ActorContextSourceKind,
    source_trust: ActorContextSourceTrust,
    source_author_actor_id: Option<String>,
    source_ref: serde_json::Value,
    statement: String,
    scope: Option<String>,
    expires_at: Option<DateTime<Utc>>,
    observed_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct CheckpointDigest<'a> {
    schema_version: u16,
    actor_id: &'a str,
    expected_current_version: i64,
    session: CheckpointSessionDigest,
    body: &'a ActorCheckpointBody,
    sources: &'a [NewActorCheckpointSource],
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CheckpointSessionDigest {
    WorkAttempt { work_id: Uuid, attempt_id: Uuid },
    CognitiveSession { lease_token: Uuid },
}

fn clean_required(label: &str, value: &str, maximum: usize) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > maximum {
        return Err(OrgIntelError::InvalidContext(format!(
            "{label} needs 1 to {maximum} bytes"
        )));
    }
    Ok(value.to_string())
}

fn clean_optional(label: &str, value: Option<&str>, maximum: usize) -> Result<Option<String>> {
    value
        .map(|value| clean_required(label, value, maximum))
        .transpose()
}

fn clean_list(label: &str, values: &[String]) -> Result<Vec<String>> {
    if values.len() > MAX_CHECKPOINT_LIST_ITEMS {
        return Err(OrgIntelError::InvalidContext(format!(
            "a checkpoint may contain at most {MAX_CHECKPOINT_LIST_ITEMS} {label}"
        )));
    }
    values
        .iter()
        .map(|value| clean_required(label, value, MAX_CHECKPOINT_ITEM_BYTES))
        .collect()
}

fn normalize_body(body: &ActorCheckpointBody) -> Result<ActorCheckpointBody> {
    let normalized = ActorCheckpointBody {
        current_objective: clean_required(
            "checkpoint current objective",
            &body.current_objective,
            4_000,
        )?,
        current_status: clean_required("checkpoint current status", &body.current_status, 4_000)?,
        completed: clean_list("completed items", &body.completed)?,
        material_findings: clean_list("material findings", &body.material_findings)?,
        decisions_made: clean_list("decisions", &body.decisions_made)?,
        changed_refs: clean_list("changed references", &body.changed_refs)?,
        failed_approaches: clean_list("failed approaches", &body.failed_approaches)?,
        blockers: clean_list("blockers", &body.blockers)?,
        next_useful_action: clean_required(
            "checkpoint next useful action",
            &body.next_useful_action,
            4_000,
        )?,
        expected_receiver: clean_optional(
            "checkpoint expected receiver",
            body.expected_receiver.as_deref(),
            1_000,
        )?,
    };
    let bytes = serde_json::to_vec(&normalized).expect("checkpoint body serializes");
    if bytes.len() > MAX_CHECKPOINT_BYTES {
        return Err(OrgIntelError::InvalidContext(format!(
            "checkpoint body exceeds {MAX_CHECKPOINT_BYTES} bytes"
        )));
    }
    Ok(normalized)
}

fn normalize_sources(values: &[NewActorCheckpointSource]) -> Result<Vec<NewActorCheckpointSource>> {
    if values.is_empty() || values.len() > MAX_CHECKPOINT_SOURCES {
        return Err(OrgIntelError::InvalidContext(format!(
            "a checkpoint needs 1 to {MAX_CHECKPOINT_SOURCES} source links"
        )));
    }
    values
        .iter()
        .map(|value| {
            Ok(NewActorCheckpointSource {
                epistemic_kind: value.epistemic_kind,
                source: normalize_source_ref(&value.source)?,
                statement: clean_required(
                    "checkpoint source statement",
                    &value.statement,
                    MAX_SOURCE_STATEMENT_BYTES,
                )?,
                scope: clean_optional(
                    "checkpoint source scope",
                    value.scope.as_deref(),
                    MAX_SOURCE_SCOPE_BYTES,
                )?,
                expires_at: value.expires_at,
            })
        })
        .collect()
}

fn clean_digest(label: &str, value: Option<&str>) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().to_ascii_lowercase();
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(OrgIntelError::InvalidContext(format!(
            "{label} must be a 64-character hexadecimal digest"
        )));
    }
    Ok(Some(value))
}

fn normalize_source_ref(source: &ActorContextSourceRef) -> Result<ActorContextSourceRef> {
    Ok(match source {
        ActorContextSourceRef::Actor { actor_id } => ActorContextSourceRef::Actor {
            actor_id: clean_required("source Actor id", actor_id, 255)?,
        },
        ActorContextSourceRef::Work { work_id, revision } => {
            if *revision < 1 {
                return Err(OrgIntelError::InvalidContext(
                    "a Work source revision must be positive".into(),
                ));
            }
            ActorContextSourceRef::Work {
                work_id: *work_id,
                revision: *revision,
            }
        }
        ActorContextSourceRef::Attempt { attempt_id } => ActorContextSourceRef::Attempt {
            attempt_id: *attempt_id,
        },
        ActorContextSourceRef::Room {
            room_id,
            through_message_id,
        } => {
            if *through_message_id < 0 {
                return Err(OrgIntelError::InvalidContext(
                    "a Room source cursor must be non-negative".into(),
                ));
            }
            ActorContextSourceRef::Room {
                room_id: *room_id,
                through_message_id: *through_message_id,
            }
        }
        ActorContextSourceRef::Message {
            room_id,
            message_id,
        } => {
            if *message_id < 1 {
                return Err(OrgIntelError::InvalidContext(
                    "a Message source id must be positive".into(),
                ));
            }
            ActorContextSourceRef::Message {
                room_id: *room_id,
                message_id: *message_id,
            }
        }
        ActorContextSourceRef::Decision { decision_id } => ActorContextSourceRef::Decision {
            decision_id: *decision_id,
        },
        ActorContextSourceRef::Artifact {
            artifact_ref_id,
            digest,
        } => ActorContextSourceRef::Artifact {
            artifact_ref_id: *artifact_ref_id,
            digest: clean_digest("Artifact source digest", digest.as_deref())?,
        },
        ActorContextSourceRef::Document {
            document_id,
            named_version_id,
        } => ActorContextSourceRef::Document {
            document_id: *document_id,
            named_version_id: *named_version_id,
        },
        ActorContextSourceRef::RuntimeFile { path, digest } => {
            let path = clean_required("Runtime file source path", path, 4_096)?;
            if (path != "/company" && !path.starts_with("/company/"))
                || path.split('/').any(|part| part == "..")
            {
                return Err(OrgIntelError::InvalidContext(
                    "a Runtime file source must be an absolute /company path without traversal"
                        .into(),
                ));
            }
            ActorContextSourceRef::RuntimeFile {
                path,
                digest: clean_digest("Runtime file source digest", digest.as_deref())?,
            }
        }
        ActorContextSourceRef::GitCommit { repository, commit } => {
            let repository = clean_required("Git source repository", repository, 2_048)?;
            let commit = clean_required("Git source commit", commit, 64)?.to_ascii_lowercase();
            if commit.len() < 7 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(OrgIntelError::InvalidContext(
                    "a Git source commit must be a 7 to 64 character hexadecimal object id".into(),
                ));
            }
            ActorContextSourceRef::GitCommit { repository, commit }
        }
        ActorContextSourceRef::External { uri } => {
            let uri = clean_required("external source URI", uri, 4_096)?;
            if !uri.starts_with("https://") && !uri.starts_with("http://") {
                return Err(OrgIntelError::InvalidContext(
                    "an external source must use an explicit http:// or https:// URI".into(),
                ));
            }
            ActorContextSourceRef::External { uri }
        }
    })
}

fn canonical_json(value: &serde_json::Value, output: &mut Vec<u8>) {
    match value {
        serde_json::Value::Null => output.extend_from_slice(b"null"),
        serde_json::Value::Bool(value) => {
            output.extend_from_slice(if *value { b"true" } else { b"false" })
        }
        serde_json::Value::Number(value) => output.extend_from_slice(value.to_string().as_bytes()),
        serde_json::Value::String(value) => {
            output.extend_from_slice(
                serde_json::to_string(value)
                    .expect("JSON string serializes")
                    .as_bytes(),
            );
        }
        serde_json::Value::Array(values) => {
            output.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                canonical_json(value, output);
            }
            output.push(b']');
        }
        serde_json::Value::Object(values) => {
            output.push(b'{');
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort();
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                output.extend_from_slice(
                    serde_json::to_string(key)
                        .expect("JSON key serializes")
                        .as_bytes(),
                );
                output.push(b':');
                canonical_json(&values[key], output);
            }
            output.push(b'}');
        }
    }
}

fn checkpoint_digest(
    actor_id: &str,
    expected_current_version: i64,
    session: ActorCheckpointSession,
    body: &ActorCheckpointBody,
    sources: &[NewActorCheckpointSource],
) -> String {
    let session = match session {
        ActorCheckpointSession::WorkAttempt {
            work_id,
            attempt_id,
        } => CheckpointSessionDigest::WorkAttempt {
            work_id,
            attempt_id,
        },
        ActorCheckpointSession::CognitiveSession { lease_token } => {
            CheckpointSessionDigest::CognitiveSession { lease_token }
        }
    };
    let value = serde_json::to_value(CheckpointDigest {
        schema_version: ACTOR_CHECKPOINT_SCHEMA_VERSION,
        actor_id,
        expected_current_version,
        session,
        body,
        sources,
    })
    .expect("checkpoint semantics serialize");
    let mut bytes = Vec::new();
    canonical_json(&value, &mut bytes);
    format!("{:x}", Sha256::digest(bytes))
}

fn checkpoint_row(row: CheckpointDbRow) -> Result<ActorCheckpointRow> {
    let body = serde_json::from_value(row.body).map_err(|error| {
        OrgIntelError::InvalidContext(format!("stored checkpoint body is invalid: {error}"))
    })?;
    Ok(ActorCheckpointRow {
        id: row.id,
        actor_id: row.actor_id,
        checkpoint_version: row.checkpoint_version,
        schema_version: row.schema_version,
        client_command_id: row.client_command_id,
        client_payload_sha256: row.client_payload_sha256,
        session_kind: row.session_kind,
        work_id: row.work_id,
        attempt_id: row.attempt_id,
        focused_mention_id: row.focused_mention_id,
        body,
        recorded_at: row.recorded_at,
    })
}

const CHECKPOINT_SELECT: &str = "id,actor_id,checkpoint_version,schema_version,client_command_id,\
 client_payload_sha256,session_kind,work_id,attempt_id,focused_mention_id,body,recorded_at";

const SOURCE_SELECT: &str =
    "ordinal,epistemic_kind,source_kind,source_trust,source_author_actor_id,\
 source_ref,statement,scope,expires_at,observed_at";

async fn active_room_access(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: &str,
    room_id: Uuid,
) -> Result<bool> {
    Ok(sqlx::query_scalar::<_, bool>(
        "SELECT TRUE FROM rooms room \
         JOIN room_participants participant ON participant.room_id=room.id \
         JOIN actors actor ON actor.id=participant.actor_id \
         WHERE room.id=$1 AND participant.actor_id=$2 \
           AND room.archived_at IS NULL AND participant.left_at IS NULL \
           AND actor.retired_at IS NULL \
         FOR SHARE OF room,participant,actor",
    )
    .bind(room_id)
    .bind(actor_id)
    .fetch_optional(&mut **tx)
    .await?
    .unwrap_or(false))
}

/// Mirror the native-Doc read boundary without returning any document body.
/// Holding the document row until the checkpoint transaction commits keeps a
/// linked source atomic with access revocation and named-version movement.
async fn document_source_author_if_visible(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: &str,
    document_id: Uuid,
    named_version_id: Uuid,
) -> Result<Option<String>> {
    let document = sqlx::query(
        "SELECT owner_actor_id,visibility::text AS visibility,linked_room_id,\
                inherit_room_visibility \
         FROM native_documents WHERE id=$1 FOR SHARE",
    )
    .bind(document_id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(document) = document else {
        return Ok(None);
    };
    let version_author = sqlx::query_scalar::<_, String>(
        "SELECT created_by_actor_id FROM native_document_versions \
         WHERE document_id=$1 AND id=$2",
    )
    .bind(document_id)
    .bind(named_version_id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(version_author) = version_author else {
        return Ok(None);
    };
    let actor_class = sqlx::query_scalar::<_, String>(
        "SELECT actor_class FROM actors WHERE id=$1 AND retired_at IS NULL FOR SHARE",
    )
    .bind(actor_id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(actor_class) = actor_class else {
        return Ok(None);
    };
    if document.get::<String, _>("owner_actor_id") == actor_id {
        return Ok(Some(version_author));
    }
    let explicit: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM native_document_participants \
         WHERE document_id=$1 AND actor_id=$2 AND removed_at IS NULL)",
    )
    .bind(document_id)
    .bind(actor_id)
    .fetch_one(&mut **tx)
    .await?;
    if explicit {
        return Ok(Some(version_author));
    }
    let visibility: String = document.get("visibility");
    if visibility == "company" && actor_class == "human" {
        return Ok(Some(version_author));
    }
    if visibility == "participants" && document.get::<bool, _>("inherit_room_visibility") {
        let room_id: Option<Uuid> = document.get("linked_room_id");
        if let Some(room_id) = room_id {
            if active_room_access(tx, actor_id, room_id).await? {
                return Ok(Some(version_author));
            }
        }
    }
    Ok(None)
}

async fn source_metadata(
    tx: &mut Transaction<'_, Postgres>,
    checkpoint_actor_id: &str,
    source: &ActorContextSourceRef,
) -> Result<(ActorContextSourceTrust, Option<String>)> {
    match source {
        ActorContextSourceRef::Actor { actor_id } => {
            sqlx::query_scalar::<_, String>("SELECT id FROM actors WHERE id=$1")
                .bind(actor_id)
                .fetch_optional(&mut **tx)
                .await?
                .ok_or_else(|| {
                    OrgIntelError::InvalidContext(format!(
                        "source Actor {actor_id:?} does not exist"
                    ))
                })?;
            Ok((
                ActorContextSourceTrust::CompanyState,
                Some(actor_id.clone()),
            ))
        }
        ActorContextSourceRef::Work { work_id, revision } => {
            let row = sqlx::query("SELECT owner_id,revision FROM work WHERE id=$1")
                .bind(work_id)
                .fetch_optional(&mut **tx)
                .await?
                .ok_or_else(|| {
                    OrgIntelError::InvalidContext(format!("source Work {work_id} does not exist"))
                })?;
            let current: i64 = row.get("revision");
            if *revision > current {
                return Err(OrgIntelError::InvalidContext(format!(
                    "source Work revision {revision} is newer than current revision {current}"
                )));
            }
            Ok((
                ActorContextSourceTrust::CompanyState,
                Some(row.get("owner_id")),
            ))
        }
        ActorContextSourceRef::Attempt { attempt_id } => {
            let actor =
                sqlx::query_scalar::<_, String>("SELECT actor_id FROM work_attempts WHERE id=$1")
                    .bind(attempt_id)
                    .fetch_optional(&mut **tx)
                    .await?
                    .ok_or_else(|| {
                        OrgIntelError::InvalidContext(format!(
                            "source Attempt {attempt_id} does not exist"
                        ))
                    })?;
            Ok((ActorContextSourceTrust::CompanyState, Some(actor)))
        }
        ActorContextSourceRef::Room {
            room_id,
            through_message_id,
        } => {
            let creator = sqlx::query_scalar::<_, String>(
                "SELECT room.created_by FROM rooms room \
                 JOIN room_participants participant ON participant.room_id=room.id \
                 JOIN actors actor ON actor.id=participant.actor_id \
                 WHERE room.id=$1 AND participant.actor_id=$2 \
                   AND room.archived_at IS NULL AND participant.left_at IS NULL \
                   AND actor.retired_at IS NULL \
                 FOR SHARE OF room,participant",
            )
            .bind(room_id)
            .bind(checkpoint_actor_id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(|| OrgIntelError::InvalidContext(format!("source Room {room_id} is not visible to checkpoint Actor {checkpoint_actor_id:?}")))?;
            let newest: i64 =
                sqlx::query_scalar("SELECT COALESCE(MAX(id),0) FROM messages WHERE room_id=$1")
                    .bind(room_id)
                    .fetch_one(&mut **tx)
                    .await?;
            if *through_message_id > newest {
                return Err(OrgIntelError::InvalidContext(format!(
                    "Room source cursor {through_message_id} is newer than current cursor {newest}"
                )));
            }
            if *through_message_id > 0 {
                let exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM messages WHERE room_id=$1 AND id=$2)",
                )
                .bind(room_id)
                .bind(through_message_id)
                .fetch_one(&mut **tx)
                .await?;
                if !exists {
                    return Err(OrgIntelError::InvalidContext(
                        "a Room source cursor must identify a Message in that Room".into(),
                    ));
                }
            }
            Ok((ActorContextSourceTrust::CompanyState, Some(creator)))
        }
        ActorContextSourceRef::Message {
            room_id,
            message_id,
        } => {
            let author = sqlx::query_scalar::<_, String>(
                "SELECT message.from_actor FROM messages message \
                 JOIN rooms room ON room.id=message.room_id \
                 JOIN room_participants participant ON participant.room_id=room.id \
                 JOIN actors actor ON actor.id=participant.actor_id \
                 WHERE message.room_id=$1 AND message.id=$2 \
                   AND participant.actor_id=$3 AND participant.left_at IS NULL \
                   AND room.archived_at IS NULL AND actor.retired_at IS NULL \
                 FOR SHARE OF room,participant,message",
            )
            .bind(room_id)
            .bind(message_id)
            .bind(checkpoint_actor_id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(|| OrgIntelError::InvalidContext(format!("source Message {message_id} in Room {room_id} is not visible to checkpoint Actor {checkpoint_actor_id:?}")))?;
            Ok((
                ActorContextSourceTrust::AuthenticatedActorInput,
                Some(author),
            ))
        }
        ActorContextSourceRef::Decision { decision_id } => {
            let author =
                sqlx::query_scalar::<_, String>("SELECT decided_by FROM decisions WHERE id=$1")
                    .bind(decision_id)
                    .fetch_optional(&mut **tx)
                    .await?
                    .ok_or_else(|| {
                        OrgIntelError::InvalidContext(format!(
                            "source Decision {decision_id} does not exist"
                        ))
                    })?;
            Ok((ActorContextSourceTrust::CompanyDecision, Some(author)))
        }
        ActorContextSourceRef::Artifact {
            artifact_ref_id, ..
        } => {
            let author =
                sqlx::query_scalar::<_, String>("SELECT created_by FROM artifact_refs WHERE id=$1")
                    .bind(artifact_ref_id)
                    .fetch_optional(&mut **tx)
                    .await?
                    .ok_or_else(|| {
                        OrgIntelError::InvalidContext(format!(
                            "source Artifact {artifact_ref_id} does not exist"
                        ))
                    })?;
            Ok((
                ActorContextSourceTrust::RuntimeUntrustedEvidence,
                Some(author),
            ))
        }
        ActorContextSourceRef::Document {
            document_id,
            named_version_id,
        } => {
            let author = document_source_author_if_visible(
                tx,
                checkpoint_actor_id,
                *document_id,
                *named_version_id,
            )
            .await?
            .ok_or_else(|| OrgIntelError::InvalidContext(format!("source document version {named_version_id} in Document {document_id} is not visible to checkpoint Actor {checkpoint_actor_id:?}")))?;
            Ok((ActorContextSourceTrust::CompanyState, Some(author)))
        }
        ActorContextSourceRef::RuntimeFile { .. } | ActorContextSourceRef::GitCommit { .. } => {
            Ok((ActorContextSourceTrust::RuntimeUntrustedEvidence, None))
        }
        ActorContextSourceRef::External { .. } => {
            Ok((ActorContextSourceTrust::ExternalUntrusted, None))
        }
    }
}

async fn source_freshness(
    tx: &mut Transaction<'_, Postgres>,
    checkpoint_actor_id: &str,
    source: &ActorContextSourceRef,
    expires_at: Option<DateTime<Utc>>,
) -> Result<ActorContextSourceFreshness> {
    let freshness = match source {
        ActorContextSourceRef::Actor { actor_id } => {
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1)")
                    .bind(actor_id)
                    .fetch_one(&mut **tx)
                    .await?;
            if exists {
                ActorContextSourceFreshness::Current
            } else {
                ActorContextSourceFreshness::Missing
            }
        }
        ActorContextSourceRef::Work { work_id, revision } => {
            match sqlx::query_scalar::<_, i64>("SELECT revision FROM work WHERE id=$1")
                .bind(work_id)
                .fetch_optional(&mut **tx)
                .await?
            {
                None => ActorContextSourceFreshness::Missing,
                Some(current) if current == *revision => ActorContextSourceFreshness::Current,
                Some(_) => ActorContextSourceFreshness::Stale,
            }
        }
        ActorContextSourceRef::Attempt { attempt_id } => {
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM work_attempts WHERE id=$1)")
                    .bind(attempt_id)
                    .fetch_one(&mut **tx)
                    .await?;
            if exists {
                ActorContextSourceFreshness::Current
            } else {
                ActorContextSourceFreshness::Missing
            }
        }
        ActorContextSourceRef::Room {
            room_id,
            through_message_id,
        } => {
            if !active_room_access(tx, checkpoint_actor_id, *room_id).await? {
                return Ok(ActorContextSourceFreshness::Inaccessible);
            }
            match sqlx::query_scalar::<_, i64>("SELECT COALESCE(MAX(message.id),0) FROM rooms room LEFT JOIN messages message ON message.room_id=room.id WHERE room.id=$1 GROUP BY room.id")
                .bind(room_id).fetch_optional(&mut **tx).await? {
                None => ActorContextSourceFreshness::Missing,
                Some(current) if current == *through_message_id => ActorContextSourceFreshness::Current,
                Some(_) => ActorContextSourceFreshness::Stale,
            }
        }
        ActorContextSourceRef::Message {
            room_id,
            message_id,
        } => {
            if !active_room_access(tx, checkpoint_actor_id, *room_id).await? {
                return Ok(ActorContextSourceFreshness::Inaccessible);
            }
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM messages WHERE room_id=$1 AND id=$2)",
            )
            .bind(room_id)
            .bind(message_id)
            .fetch_one(&mut **tx)
            .await?;
            if exists {
                ActorContextSourceFreshness::Current
            } else {
                ActorContextSourceFreshness::Missing
            }
        }
        ActorContextSourceRef::Decision { decision_id } => {
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM decisions WHERE id=$1)")
                    .bind(decision_id)
                    .fetch_one(&mut **tx)
                    .await?;
            if exists {
                ActorContextSourceFreshness::Current
            } else {
                ActorContextSourceFreshness::Missing
            }
        }
        ActorContextSourceRef::Artifact {
            artifact_ref_id,
            digest,
        } => {
            match sqlx::query("SELECT digest,state::text AS state FROM artifact_refs WHERE id=$1")
                .bind(artifact_ref_id)
                .fetch_optional(&mut **tx)
                .await?
            {
                None => ActorContextSourceFreshness::Missing,
                Some(row) => {
                    let current_digest: Option<String> = row.get("digest");
                    let state: String = row.get("state");
                    if state != "available"
                        || digest
                            .as_ref()
                            .is_some_and(|digest| current_digest.as_ref() != Some(digest))
                    {
                        ActorContextSourceFreshness::Stale
                    } else {
                        ActorContextSourceFreshness::Current
                    }
                }
            }
        }
        ActorContextSourceRef::Document {
            document_id,
            named_version_id,
        } => {
            if document_source_author_if_visible(
                tx,
                checkpoint_actor_id,
                *document_id,
                *named_version_id,
            )
            .await?
            .is_none()
            {
                return Ok(ActorContextSourceFreshness::Inaccessible);
            }
            match sqlx::query_scalar::<_, Uuid>(
                "SELECT current_named_version_id FROM native_documents WHERE id=$1",
            )
            .bind(document_id)
            .fetch_optional(&mut **tx)
            .await?
            {
                None => ActorContextSourceFreshness::Missing,
                Some(current) if current == *named_version_id => {
                    ActorContextSourceFreshness::Current
                }
                Some(_) => ActorContextSourceFreshness::Stale,
            }
        }
        ActorContextSourceRef::RuntimeFile { .. }
        | ActorContextSourceRef::GitCommit { .. }
        | ActorContextSourceRef::External { .. } => ActorContextSourceFreshness::Unverifiable,
    };
    if !matches!(
        freshness,
        ActorContextSourceFreshness::Missing | ActorContextSourceFreshness::Stale
    ) && expires_at.is_some_and(|expires_at| expires_at <= Utc::now())
    {
        Ok(ActorContextSourceFreshness::Expired)
    } else {
        Ok(freshness)
    }
}

async fn source_row(
    tx: &mut Transaction<'_, Postgres>,
    checkpoint_actor_id: &str,
    row: CheckpointSourceDbRow,
) -> Result<ActorCheckpointSource> {
    let source: ActorContextSourceRef =
        serde_json::from_value(row.source_ref).map_err(|error| {
            OrgIntelError::InvalidContext(format!("stored checkpoint source is invalid: {error}"))
        })?;
    if source.kind() != row.source_kind {
        return Err(OrgIntelError::InvalidContext(
            "stored checkpoint source kind does not match its typed reference".into(),
        ));
    }
    let freshness = source_freshness(tx, checkpoint_actor_id, &source, row.expires_at).await?;
    Ok(ActorCheckpointSource {
        ordinal: row.ordinal,
        epistemic_kind: row.epistemic_kind,
        source_kind: row.source_kind,
        source_trust: row.source_trust,
        source_author_actor_id: row.source_author_actor_id,
        source,
        statement: row.statement,
        scope: row.scope,
        expires_at: row.expires_at,
        observed_at: row.observed_at,
        freshness,
    })
}

async fn checkpoint_view_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    checkpoint: CheckpointDbRow,
    source_limit: i64,
) -> Result<ActorCheckpointView> {
    let source_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM actor_context_checkpoint_sources WHERE checkpoint_id=$1",
    )
    .bind(checkpoint.id)
    .fetch_one(&mut **tx)
    .await?;
    let actor_id = checkpoint.actor_id.clone();
    let rows: Vec<CheckpointSourceDbRow> = sqlx::query_as(&format!(
        "SELECT {SOURCE_SELECT} FROM actor_context_checkpoint_sources WHERE checkpoint_id=$1 ORDER BY ordinal LIMIT $2"
    ))
    .bind(checkpoint.id)
    .bind(source_limit)
    .fetch_all(&mut **tx)
    .await?;
    let mut sources = Vec::with_capacity(rows.len());
    for row in rows {
        sources.push(source_row(tx, &actor_id, row).await?);
    }
    Ok(ActorCheckpointView {
        checkpoint: checkpoint_row(checkpoint)?,
        sources,
        source_count,
        sources_truncated: source_count > source_limit,
    })
}

/// Native Document reads and access mutations lock Document before Actor.
/// Pre-lock every linked Document in stable order before taking the Actor-wide
/// checkpoint fence so a mixed Room/Document checkpoint preserves both the
/// Document -> Actor and Actor -> Room lock orders used by their owners.
async fn lock_document_sources_before_actor(
    tx: &mut Transaction<'_, Postgres>,
    sources: &[NewActorCheckpointSource],
) -> Result<()> {
    let mut document_ids = sources
        .iter()
        .filter_map(|source| match &source.source {
            ActorContextSourceRef::Document { document_id, .. } => Some(*document_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    document_ids.sort_unstable();
    document_ids.dedup();
    for document_id in document_ids {
        sqlx::query("SELECT id FROM native_documents WHERE id=$1 FOR SHARE")
            .bind(document_id)
            .fetch_optional(&mut **tx)
            .await?;
    }
    Ok(())
}

impl OrgIntel {
    /// Persist one Actor-owned checkpoint under the Actor-wide primary-session
    /// fence. Exact command replay returns the original immutable version;
    /// competing versions serialize on the Actor row and stale CAS loses.
    pub async fn record_actor_checkpoint(
        &self,
        request: NewActorCheckpoint<'_>,
    ) -> Result<ActorCheckpointWriteResult> {
        let command_id = clean_required("checkpoint command id", request.client_command_id, 128)?;
        let actor_id = clean_required("checkpoint Actor id", request.actor_id, 255)?;
        if request.expected_current_version < 0 {
            return Err(OrgIntelError::InvalidContext(
                "expected checkpoint version must be non-negative".into(),
            ));
        }
        let body = normalize_body(&request.body)?;
        let sources = normalize_sources(request.sources)?;
        let payload_sha256 = checkpoint_digest(
            &actor_id,
            request.expected_current_version,
            request.session,
            &body,
            &sources,
        );

        let mut tx = self.pool.begin().await?;
        lock_document_sources_before_actor(&mut tx, &sources).await?;
        let actor_class = sqlx::query_scalar::<_, String>(
            "SELECT actor_class FROM actors WHERE id=$1 AND retired_at IS NULL FOR UPDATE",
        )
        .bind(&actor_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidContext(format!("checkpoint Actor {actor_id:?} is not active"))
        })?;
        if actor_class != "agent" {
            return Err(OrgIntelError::InvalidContext(
                "only a durable agent Actor may record a context checkpoint".into(),
            ));
        }

        if let Some(prior) = sqlx::query_as::<_, CheckpointDbRow>(&format!(
            "SELECT {CHECKPOINT_SELECT} FROM actor_context_checkpoints WHERE actor_id=$1 AND client_command_id=$2"
        ))
        .bind(&actor_id)
        .bind(&command_id)
        .fetch_optional(&mut *tx)
        .await?
        {
            if prior.client_payload_sha256 != payload_sha256 {
                return Err(OrgIntelError::ContextConflict(
                    "checkpoint command was already used with different semantics".into(),
                ));
            }
            let checkpoint = checkpoint_view_in_tx(&mut tx, prior, MAX_CHECKPOINT_SOURCES as i64).await?;
            tx.commit().await?;
            return Ok(ActorCheckpointWriteResult { checkpoint, created: false });
        }

        let (session_kind, work_id, attempt_id, focused_mention_id) = match request.session {
            ActorCheckpointSession::WorkAttempt {
                work_id,
                attempt_id,
            } => {
                let row = sqlx::query(
                    "SELECT attempt.actor_id,attempt.state,attempt.work_id,\
                            attempt.revision AS attempt_revision,work.revision AS work_revision,\
                            work.status AS work_status \
                     FROM work_attempts attempt JOIN work ON work.id=attempt.work_id \
                     WHERE attempt.id=$1 AND work.id=$2 FOR SHARE OF attempt,work",
                )
                .bind(attempt_id)
                .bind(work_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| {
                    OrgIntelError::InvalidContext(
                        "checkpoint Work/Attempt source does not exist".into(),
                    )
                })?;
                let attempt_actor: String = row.get("actor_id");
                let state: WorkAttemptState = row.get("state");
                let attempt_revision: i64 = row.get("attempt_revision");
                let work_revision: i64 = row.get("work_revision");
                let work_status: WorkStatus = row.get("work_status");
                if attempt_actor != actor_id
                    || state != WorkAttemptState::Running
                    || attempt_revision != work_revision
                    || work_status != WorkStatus::Active
                {
                    return Err(OrgIntelError::ContextConflict(
                        "a Work checkpoint needs this Actor's exact current running Work revision"
                            .into(),
                    ));
                }
                (
                    ActorCheckpointSessionKind::WorkAttempt,
                    Some(work_id),
                    Some(attempt_id),
                    None,
                )
            }
            ActorCheckpointSession::CognitiveSession { lease_token } => {
                let focus = sqlx::query_scalar::<_, Option<Uuid>>(
                    "SELECT focused_mention_id FROM actor_cognitive_leases \
                     WHERE actor_id=$1 AND lease_token=$2 AND claimed_until>now() \
                       AND revoked_at IS NULL FOR SHARE",
                )
                .bind(&actor_id)
                .bind(lease_token)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| {
                    OrgIntelError::ContextConflict(
                        "checkpoint cognitive session is not this Actor's live primary lease"
                            .into(),
                    )
                })?;
                (
                    ActorCheckpointSessionKind::CognitiveSession,
                    None,
                    None,
                    focus,
                )
            }
        };

        let current_version: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(checkpoint_version),0) FROM actor_context_checkpoints WHERE actor_id=$1",
        )
        .bind(&actor_id)
        .fetch_one(&mut *tx)
        .await?;
        if current_version != request.expected_current_version {
            return Err(OrgIntelError::ContextConflict(format!(
                "Actor checkpoint is version {current_version}, expected {}",
                request.expected_current_version
            )));
        }

        let mut metadata = Vec::with_capacity(sources.len());
        for source in &sources {
            metadata.push(source_metadata(&mut tx, &actor_id, &source.source).await?);
        }

        let checkpoint_id = Uuid::new_v4();
        let checkpoint_version = current_version + 1;
        let body_json = serde_json::to_value(&body).expect("checkpoint body serializes");
        let inserted: CheckpointDbRow = sqlx::query_as(&format!(
            "INSERT INTO actor_context_checkpoints \
             (id,actor_id,checkpoint_version,schema_version,client_command_id,client_payload_sha256,\
              session_kind,work_id,attempt_id,focused_mention_id,body) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) RETURNING {CHECKPOINT_SELECT}"
        ))
        .bind(checkpoint_id)
        .bind(&actor_id)
        .bind(checkpoint_version)
        .bind(ACTOR_CHECKPOINT_SCHEMA_VERSION as i16)
        .bind(&command_id)
        .bind(&payload_sha256)
        .bind(session_kind)
        .bind(work_id)
        .bind(attempt_id)
        .bind(focused_mention_id)
        .bind(body_json)
        .fetch_one(&mut *tx)
        .await?;

        for (ordinal, (source, (trust, author))) in sources.iter().zip(metadata).enumerate() {
            sqlx::query(
                "INSERT INTO actor_context_checkpoint_sources \
                 (checkpoint_id,ordinal,epistemic_kind,source_kind,source_trust,source_author_actor_id,\
                  source_ref,statement,scope,expires_at) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
            )
            .bind(checkpoint_id)
            .bind(ordinal as i16)
            .bind(source.epistemic_kind)
            .bind(source.source.kind())
            .bind(trust)
            .bind(author)
            .bind(serde_json::to_value(&source.source).expect("source reference serializes"))
            .bind(&source.statement)
            .bind(source.scope.as_deref())
            .bind(source.expires_at)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "INSERT INTO events (kind,actor_id,body) VALUES ('actor.context.checkpoint.recorded.v1',$1,$2)",
        )
        .bind(&actor_id)
        .bind(serde_json::json!({
            "checkpoint_id": checkpoint_id,
            "checkpoint_version": checkpoint_version,
            "session_kind": session_kind,
            "work_id": work_id,
            "attempt_id": attempt_id,
            "focused_mention_id": focused_mention_id,
            "source_count": sources.len(),
        }))
        .execute(&mut *tx)
        .await?;
        let checkpoint =
            checkpoint_view_in_tx(&mut tx, inserted, MAX_CHECKPOINT_SOURCES as i64).await?;
        tx.commit().await?;
        Ok(ActorCheckpointWriteResult {
            checkpoint,
            created: true,
        })
    }

    pub async fn latest_actor_checkpoint(
        &self,
        actor_id: &str,
        source_limit: i64,
    ) -> Result<Option<ActorCheckpointView>> {
        if !(1..=MAX_SOURCE_PAGE).contains(&source_limit) {
            return Err(OrgIntelError::InvalidContext(format!(
                "checkpoint source limit must be between 1 and {MAX_SOURCE_PAGE}"
            )));
        }
        let mut tx = self.pool.begin().await?;
        let checkpoint = sqlx::query_as::<_, CheckpointDbRow>(&format!(
            "SELECT {CHECKPOINT_SELECT} FROM actor_context_checkpoints WHERE actor_id=$1 ORDER BY checkpoint_version DESC LIMIT 1"
        ))
        .bind(actor_id)
        .fetch_optional(&mut *tx)
        .await?;
        let checkpoint = match checkpoint {
            Some(checkpoint) => {
                Some(checkpoint_view_in_tx(&mut tx, checkpoint, source_limit).await?)
            }
            None => None,
        };
        tx.commit().await?;
        Ok(checkpoint)
    }

    pub async fn actor_checkpoints_before(
        &self,
        actor_id: &str,
        before_version: Option<i64>,
        limit: i64,
    ) -> Result<Vec<ActorCheckpointRow>> {
        if before_version.is_some_and(|value| value < 1)
            || !(1..=MAX_CHECKPOINT_PAGE).contains(&limit)
        {
            return Err(OrgIntelError::InvalidContext(format!(
                "checkpoint cursor must be positive and limit must be between 1 and {MAX_CHECKPOINT_PAGE}"
            )));
        }
        let rows: Vec<CheckpointDbRow> = sqlx::query_as(&format!(
            "SELECT {CHECKPOINT_SELECT} FROM actor_context_checkpoints \
             WHERE actor_id=$1 AND ($2::bigint IS NULL OR checkpoint_version<$2) \
             ORDER BY checkpoint_version DESC LIMIT $3"
        ))
        .bind(actor_id)
        .bind(before_version)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(checkpoint_row).collect()
    }

    pub async fn actor_checkpoint_sources_after(
        &self,
        actor_id: &str,
        checkpoint_id: Uuid,
        after_ordinal: Option<i16>,
        limit: i64,
    ) -> Result<ActorContextSourcePage> {
        if after_ordinal.is_some_and(|value| value < 0) || !(1..=MAX_SOURCE_PAGE).contains(&limit) {
            return Err(OrgIntelError::InvalidContext(format!(
                "source cursor must be non-negative and limit must be between 1 and {MAX_SOURCE_PAGE}"
            )));
        }
        let mut tx = self.pool.begin().await?;
        let owns: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actor_context_checkpoints WHERE id=$1 AND actor_id=$2)",
        )
        .bind(checkpoint_id)
        .bind(actor_id)
        .fetch_one(&mut *tx)
        .await?;
        if !owns {
            return Err(OrgIntelError::InvalidContext(
                "Actor checkpoint does not exist".into(),
            ));
        }
        let rows: Vec<CheckpointSourceDbRow> = sqlx::query_as(&format!(
            "SELECT {SOURCE_SELECT} FROM actor_context_checkpoint_sources \
             WHERE checkpoint_id=$1 AND ordinal>COALESCE($2,-1) ORDER BY ordinal LIMIT $3"
        ))
        .bind(checkpoint_id)
        .bind(after_ordinal)
        .bind(limit + 1)
        .fetch_all(&mut *tx)
        .await?;
        let has_more = rows.len() as i64 > limit;
        let mut sources = Vec::new();
        for row in rows.into_iter().take(limit as usize) {
            sources.push(source_row(&mut tx, actor_id, row).await?);
        }
        tx.commit().await?;
        Ok(ActorContextSourcePage {
            next_after_ordinal: sources.last().map(|source| source.ordinal),
            sources,
            has_more,
        })
    }

    /// Materialise the compact OrgIntel-owned portion of a fresh Actor
    /// session.  It includes one exact focus, return coordinates, source links
    /// and (when focus-compatible) the latest checkpoint; no Room/company
    /// history or source body is replicated into the envelope.
    pub async fn actor_context_bootstrap(
        &self,
        actor_id: &str,
        requested_focus: ActorContextFocus,
        checkpoint_source_limit: i64,
    ) -> Result<ActorContextBootstrap> {
        if !(1..=MAX_BOOTSTRAP_SOURCES).contains(&checkpoint_source_limit) {
            return Err(OrgIntelError::InvalidContext(format!(
                "bootstrap checkpoint source limit must be between 1 and {MAX_BOOTSTRAP_SOURCES}"
            )));
        }
        let mut tx = self.pool.begin().await?;
        let actor_row = sqlx::query(
            "SELECT id,actor_class,display,role,team_id FROM actors \
             WHERE id=$1 AND retired_at IS NULL FOR SHARE",
        )
        .bind(actor_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidContext(format!("Actor {actor_id:?} is not active"))
        })?;
        let actor = ActorContextIdentity {
            actor_id: actor_row.get("id"),
            actor_class: actor_row.get("actor_class"),
            display: actor_row.get("display"),
            role: actor_row.get("role"),
            team_id: actor_row.get("team_id"),
        };

        let (focus, return_path, anchors, checkpoint_filter): (
            _,
            _,
            _,
            Option<(Option<Uuid>, Option<Uuid>)>,
        ) = match requested_focus {
            ActorContextFocus::General => (
                ActorContextFocusProjection::General,
                ActorContextReturnPath::ActorInbox {
                    actor_id: actor_id.to_string(),
                },
                vec![ActorContextSourceRef::Actor {
                    actor_id: actor_id.to_string(),
                }],
                None,
            ),
            ActorContextFocus::WorkAttempt {
                work_id,
                attempt_id,
            } => {
                let row = sqlx::query(
                    "SELECT work.revision AS work_revision,work.owner_id,work.title,work.outcome,\
                            work.status,work.expected_artifact,attempt.attempt_no,\
                            attempt.revision AS attempt_revision,attempt.state AS attempt_state,\
                            attempt.input_fingerprint,attempt.actor_id \
                     FROM work JOIN work_attempts attempt ON attempt.work_id=work.id \
                     WHERE work.id=$1 AND attempt.id=$2 FOR SHARE OF work,attempt",
                )
                .bind(work_id)
                .bind(attempt_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| {
                    OrgIntelError::InvalidContext("focused Work/Attempt does not exist".into())
                })?;
                let attempt_actor: String = row.get("actor_id");
                let attempt_revision: i64 = row.get("attempt_revision");
                let attempt_state: WorkAttemptState = row.get("attempt_state");
                let work_revision: i64 = row.get("work_revision");
                let work_status: WorkStatus = row.get("status");
                if attempt_actor != actor_id
                    || attempt_state != WorkAttemptState::Running
                    || attempt_revision != work_revision
                    || work_status != WorkStatus::Active
                {
                    return Err(OrgIntelError::InvalidContext(
                        "focused Attempt is not this Actor's exact current running Work revision"
                            .into(),
                    ));
                }
                (
                    ActorContextFocusProjection::WorkAttempt {
                        work_id,
                        work_revision,
                        owner_actor_id: row.get("owner_id"),
                        title: row.get("title"),
                        outcome: row.get("outcome"),
                        status: work_status,
                        expected_artifact: row.get("expected_artifact"),
                        attempt_id,
                        attempt_no: row.get("attempt_no"),
                        attempt_state,
                        input_fingerprint: row.get("input_fingerprint"),
                    },
                    ActorContextReturnPath::WorkAttempt {
                        work_id,
                        attempt_id,
                    },
                    vec![
                        ActorContextSourceRef::Work {
                            work_id,
                            revision: work_revision,
                        },
                        ActorContextSourceRef::Attempt { attempt_id },
                    ],
                    Some((Some(work_id), None)),
                )
            }
            ActorContextFocus::RoomMention { mention_id } => {
                let row = sqlx::query(
                    "SELECT mention.room_id,room.title AS room_title,mention.message_id,\
                            mention.thread_root_message_id,message.from_actor,\
                            COALESCE(revision.body,message.body) AS body,\
                            mention.work_id,mention.why_this_actor,mention.expected_response,\
                            mention.affected_scope \
                     FROM message_mentions mention \
                     JOIN rooms room ON room.id=mention.room_id \
                     JOIN messages message ON message.id=mention.message_id AND message.room_id=mention.room_id \
                     LEFT JOIN room_message_revisions revision ON revision.id=message.latest_revision_id \
                     JOIN room_participants participant ON participant.room_id=mention.room_id \
                       AND participant.actor_id=mention.mentioned_actor_id \
                     WHERE mention.id=$1 AND mention.mentioned_actor_id=$2 \
                       AND mention.resolution_message_id IS NULL AND mention.cancelled_event_id IS NULL \
                       AND message.deleted_at IS NULL \
                       AND participant.left_at IS NULL AND room.archived_at IS NULL \
                     FOR SHARE OF room,participant,message,mention",
                )
                .bind(mention_id).bind(actor_id).fetch_optional(&mut *tx).await?
                .ok_or_else(|| OrgIntelError::InvalidContext("focused Room mention is not current for this Actor".into()))?;
                let room_id: Uuid = row.get("room_id");
                let message_id: i64 = row.get("message_id");
                let thread_root_message_id: i64 = row.get("thread_root_message_id");
                let linked_work_id: Option<Uuid> = row.get("work_id");
                let mut anchors = vec![
                    ActorContextSourceRef::Room {
                        room_id,
                        through_message_id: message_id,
                    },
                    ActorContextSourceRef::Message {
                        room_id,
                        message_id,
                    },
                ];
                if thread_root_message_id != message_id {
                    anchors.push(ActorContextSourceRef::Message {
                        room_id,
                        message_id: thread_root_message_id,
                    });
                }
                if let Some(work_id) = linked_work_id {
                    let revision =
                        sqlx::query_scalar::<_, i64>("SELECT revision FROM work WHERE id=$1")
                            .bind(work_id)
                            .fetch_one(&mut *tx)
                            .await?;
                    anchors.push(ActorContextSourceRef::Work { work_id, revision });
                }
                (
                    ActorContextFocusProjection::RoomMention {
                        mention_id,
                        room_id,
                        room_title: row.get("room_title"),
                        message_id,
                        thread_root_message_id,
                        from_actor_id: row.get("from_actor"),
                        triggering_message: row.get("body"),
                        linked_work_id,
                        why_this_actor: row.get("why_this_actor"),
                        expected_response: row.get("expected_response"),
                        affected_scope: row.get("affected_scope"),
                        source_trust: ActorContextSourceTrust::AuthenticatedActorInput,
                        content_grants_authority: false,
                    },
                    ActorContextReturnPath::RoomThread {
                        room_id,
                        thread_root_message_id,
                        reply_to_message_id: message_id,
                        resolves_mention_id: mention_id,
                    },
                    anchors,
                    Some((None, Some(mention_id))),
                )
            }
        };

        let checkpoint_query = match checkpoint_filter {
            None => format!("SELECT {CHECKPOINT_SELECT} FROM actor_context_checkpoints WHERE actor_id=$1 AND session_kind='cognitive_session' AND focused_mention_id IS NULL ORDER BY checkpoint_version DESC LIMIT 1"),
            Some((Some(_), None)) => format!("SELECT {CHECKPOINT_SELECT} FROM actor_context_checkpoints WHERE actor_id=$1 AND work_id=$2 ORDER BY checkpoint_version DESC LIMIT 1"),
            Some((None, Some(_))) => format!("SELECT {CHECKPOINT_SELECT} FROM actor_context_checkpoints WHERE actor_id=$1 AND focused_mention_id=$2 ORDER BY checkpoint_version DESC LIMIT 1"),
            Some((None, None)) | Some((Some(_), Some(_))) => unreachable!("focus filter shape"),
        };
        let checkpoint = match checkpoint_filter {
            None => {
                sqlx::query_as::<_, CheckpointDbRow>(&checkpoint_query)
                    .bind(actor_id)
                    .fetch_optional(&mut *tx)
                    .await?
            }
            Some((Some(work_id), None)) => {
                sqlx::query_as::<_, CheckpointDbRow>(&checkpoint_query)
                    .bind(actor_id)
                    .bind(work_id)
                    .fetch_optional(&mut *tx)
                    .await?
            }
            Some((None, Some(mention_id))) => {
                sqlx::query_as::<_, CheckpointDbRow>(&checkpoint_query)
                    .bind(actor_id)
                    .bind(mention_id)
                    .fetch_optional(&mut *tx)
                    .await?
            }
            _ => unreachable!("focus filter shape"),
        };
        let checkpoint = match checkpoint {
            Some(checkpoint) => {
                Some(checkpoint_view_in_tx(&mut tx, checkpoint, checkpoint_source_limit).await?)
            }
            None => None,
        };
        let loaded_stale_checkpoint_sources = checkpoint.as_ref().map_or(0, |checkpoint| {
            checkpoint
                .sources
                .iter()
                .filter(|source| {
                    matches!(
                        source.freshness,
                        ActorContextSourceFreshness::Stale
                            | ActorContextSourceFreshness::Missing
                            | ActorContextSourceFreshness::Inaccessible
                            | ActorContextSourceFreshness::Expired
                    )
                })
                .count()
        });
        let loaded_untrusted_checkpoint_sources = checkpoint.as_ref().map_or(0, |checkpoint| {
            checkpoint
                .sources
                .iter()
                .filter(|source| {
                    matches!(
                        source.source_trust,
                        ActorContextSourceTrust::AuthenticatedActorInput
                            | ActorContextSourceTrust::RuntimeUntrustedEvidence
                            | ActorContextSourceTrust::ExternalUntrusted
                    )
                })
                .count()
        });
        tx.commit().await?;
        Ok(ActorContextBootstrap {
            schema: ACTOR_CONTEXT_BOOTSTRAP_SCHEMA.into(),
            version: ACTOR_CONTEXT_BOOTSTRAP_VERSION,
            generated_at: Utc::now(),
            actor,
            focus,
            return_path,
            anchors,
            checkpoint,
            loaded_stale_checkpoint_sources,
            loaded_untrusted_checkpoint_sources,
            context_grants_authority: false,
        })
    }
}
