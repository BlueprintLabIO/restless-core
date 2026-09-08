//! Native Docs phase-one company semantics.
//!
//! The records in this module are a Core collaboration subsystem: metadata,
//! explicit Actor access and immutable named structured-content versions. They
//! do not create a file shadow in the Company Runtime. A later Yjs sidecar may
//! own the live collaborative body; these named versions remain checkpoints and
//! the structured read-only representation.

use std::collections::{BTreeMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest as _, Sha256};
use sqlx::{Postgres, Row as _, Transaction};
use uuid::Uuid;

use crate::OrgIntel;

const DOCUMENT_SCHEMA_VERSION: i16 = 1;
const MAX_DOCUMENT_BYTES: usize = 1_000_000;
const MAX_DOCUMENT_NODES: usize = 20_000;
const MAX_DOCUMENT_DEPTH: usize = 64;
const MAX_DOCUMENT_TEXT_CHARS: usize = 500_000;
pub const NATIVE_DOCUMENT_PORTABLE_ENVELOPE_SCHEMA: &str =
    "restless.native-document.runtime-checkpoint";
pub const NATIVE_DOCUMENT_PORTABLE_ENVELOPE_VERSION: u16 = 1;
const MAX_PORTABLE_ENVELOPE_BYTES: usize = 2_500_000;

#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    #[error("invalid native document: {0}")]
    Invalid(String),
    #[error("native document is unavailable")]
    Unavailable,
    #[error("native document conflict: {0}")]
    Conflict(String),
    #[error("native document state is corrupt: {0}")]
    Corrupt(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

pub type DocumentResult<T> = std::result::Result<T, DocumentError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "native_document_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DocumentKind {
    Brief,
    Plan,
    DecisionNote,
    Report,
    Review,
    Handbook,
    OperatingNote,
    Freeform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "native_document_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    Draft,
    InReview,
    Accepted,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "native_document_visibility", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DocumentVisibility {
    Company,
    Participants,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "native_document_access", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum DocumentAccess {
    Read,
    Comment,
    Edit,
}

impl DocumentAccess {
    fn rank(self) -> u8 {
        match self {
            Self::Read => 1,
            Self::Comment => 2,
            Self::Edit => 3,
        }
    }

    fn permits(self, required: Self) -> bool {
        self.rank() >= required.rank()
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DocumentRow {
    pub id: Uuid,
    pub title: String,
    pub kind: DocumentKind,
    pub status: DocumentStatus,
    pub visibility: DocumentVisibility,
    pub linked_room_id: Option<Uuid>,
    pub inherit_room_visibility: bool,
    pub owner_actor_id: String,
    pub current_named_version_id: Uuid,
    pub created_by_actor_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DocumentVersionRow {
    pub id: Uuid,
    pub document_id: Uuid,
    pub version_number: i64,
    pub schema_version: i16,
    pub content_json: Value,
    pub plain_text: String,
    pub content_hash: String,
    pub document_status: DocumentStatus,
    pub restored_from_version_id: Option<Uuid>,
    pub created_by_actor_id: String,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DocumentParticipantRow {
    pub document_id: Uuid,
    pub actor_id: String,
    pub access: DocumentAccess,
    pub added_by_actor_id: String,
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentSummary {
    pub document: DocumentRow,
    pub access: DocumentAccess,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentVersionView {
    pub version: DocumentVersionRow,
    /// Server-rendered only from the restricted schema. It contains no raw
    /// HTML from the stored document.
    pub rendered_html: String,
    /// Deterministic explicit export projection. The structured JSON remains
    /// authoritative; editing this text never mutates the Doc.
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentReadView {
    pub document: DocumentRow,
    pub access: DocumentAccess,
    pub current_version: DocumentVersionView,
}

/// One self-contained, deterministic checkpoint that may cross the explicit
/// Company Runtime boundary. It contains no host path and performs no implicit
/// filesystem I/O. `content_json` is the editable portable representation;
/// `plain_text` and `markdown` are deterministic projections sealed by the
/// checksum. Source fields remain unchanged when Runtime work edits and
/// reseals the content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeDocumentPortableEnvelope {
    pub envelope_schema: String,
    pub envelope_version: u16,
    pub source_company_id: Option<Uuid>,
    pub source_company_key: String,
    pub source_document_id: Uuid,
    pub source_named_version_id: Uuid,
    pub source_named_version_number: i64,
    pub source_content_schema_version: i16,
    pub source_document_status: DocumentStatus,
    pub source_restored_from_version_id: Option<Uuid>,
    pub source_created_by_actor_id: String,
    pub source_version_reason: String,
    pub source_version_created_at: DateTime<Utc>,
    pub source_content_hash: String,
    pub content_json: Value,
    pub plain_text: String,
    pub markdown: String,
    pub content_hash: String,
    pub checksum: String,
}

impl NativeDocumentPortableEnvelope {
    /// Re-derive every portable projection and checksum after an explicit
    /// Runtime edit to `content_json`. This never writes back to Core.
    pub fn reseal(&mut self) -> DocumentResult<()> {
        validate_portable_envelope_header(self)?;
        let content = validate_document_json(&self.content_json)?;
        self.content_json = content.content_json;
        self.plain_text = content.plain_text;
        self.content_hash = content.content_hash;
        self.markdown = portable_markdown(self, &content.markdown);
        self.checksum.clear();
        self.checksum = portable_envelope_checksum(self)?;
        ensure_portable_envelope_bound(self)?;
        Ok(())
    }

    /// Canonical JSON bytes suitable for a Runtime `.json` checkpoint. Object
    /// key order in the structured document cannot change these bytes.
    pub fn to_json_bytes(&self) -> DocumentResult<Vec<u8>> {
        validate_portable_envelope(self)?;
        serde_json::to_value(self)
            .map(|value| canonical_json(&value))
            .map_err(|error| {
                DocumentError::Invalid(format!("portable envelope is not JSON: {error}"))
            })
    }
}

pub struct ImportRuntimeDocument<'a> {
    pub target_document_id: Uuid,
    pub actor_id: &'a str,
    pub expected_current_version_id: Uuid,
    pub envelope: &'a NativeDocumentPortableEnvelope,
    pub reason: &'a str,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentImportResult {
    pub target_document_id: Uuid,
    pub imported_version: DocumentVersionView,
    pub envelope_checksum: String,
}

/// Durable receipt for one explicit Runtime import. The immutable named
/// version owns the imported body; this record keeps only bounded source and
/// checksum provenance so audit and replay do not depend on retained events.
#[derive(Debug, Clone, PartialEq, Serialize, sqlx::FromRow)]
pub struct DocumentRuntimeImportProvenance {
    pub document_id: Uuid,
    pub imported_version_id: Uuid,
    pub target_base_version_id: Uuid,
    pub envelope_schema: String,
    pub envelope_version: i16,
    pub envelope_checksum: String,
    pub source_company_id: Option<Uuid>,
    pub source_company_key: String,
    pub source_document_id: Uuid,
    pub source_named_version_id: Uuid,
    pub source_named_version_number: i64,
    pub source_content_schema_version: i16,
    pub source_document_status: DocumentStatus,
    pub source_restored_from_version_id: Option<Uuid>,
    pub source_created_by_actor_id: String,
    pub source_version_reason: String,
    pub source_version_created_at: DateTime<Utc>,
    pub source_content_hash: String,
    pub imported_content_hash: String,
    pub imported_by_actor_id: String,
    pub imported_at: DateTime<Utc>,
}

pub struct NewDocument<'a> {
    pub title: &'a str,
    pub kind: DocumentKind,
    pub visibility: DocumentVisibility,
    pub linked_room_id: Option<Uuid>,
    pub inherit_room_visibility: bool,
    pub owner_actor_id: &'a str,
    pub created_by_actor_id: &'a str,
    pub content_json: &'a Value,
    pub reason: &'a str,
}

pub struct UpdateDocumentMetadata<'a> {
    pub document_id: Uuid,
    pub actor_id: &'a str,
    pub expected_version: i64,
    pub title: &'a str,
    pub kind: DocumentKind,
    pub status: DocumentStatus,
    pub visibility: DocumentVisibility,
    pub linked_room_id: Option<Uuid>,
    pub inherit_room_visibility: bool,
}

pub struct NewNamedDocumentVersion<'a> {
    pub document_id: Uuid,
    pub actor_id: &'a str,
    pub expected_current_version_id: Uuid,
    pub content_json: &'a Value,
    pub reason: &'a str,
}

pub struct RestoreDocumentVersion<'a> {
    pub document_id: Uuid,
    pub actor_id: &'a str,
    pub expected_current_version_id: Uuid,
    pub source_version_id: Uuid,
    pub reason: &'a str,
}

pub struct SetDocumentParticipant<'a> {
    pub document_id: Uuid,
    pub actor_id: &'a str,
    pub expected_document_version: i64,
    pub participant_actor_id: &'a str,
    pub access: DocumentAccess,
}

#[derive(Debug, Clone)]
pub struct ValidatedDocument {
    pub content_json: Value,
    pub plain_text: String,
    pub rendered_html: String,
    pub markdown: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildRule {
    Document,
    Block,
    Inline,
    ListItem,
    TaskItem,
    TableRow,
    TableCell,
    Text,
    None,
}

#[derive(Default)]
struct ValidationState {
    nodes: usize,
    text_chars: usize,
    block_ids: HashSet<String>,
}

/// Validate one Tiptap/ProseMirror-compatible JSON document and derive every
/// non-authoritative representation from that exact value. Unknown nodes,
/// marks, attributes and object fields are refused; arbitrary HTML therefore
/// never crosses the storage or rendering boundary.
pub fn validate_document_json(content: &Value) -> DocumentResult<ValidatedDocument> {
    let serialized = serde_json::to_vec(content)
        .map_err(|error| DocumentError::Invalid(format!("content is not JSON: {error}")))?;
    if serialized.len() > MAX_DOCUMENT_BYTES {
        return Err(DocumentError::Invalid(format!(
            "content exceeds {MAX_DOCUMENT_BYTES} bytes"
        )));
    }

    let mut state = ValidationState::default();
    validate_node(content, ChildRule::Document, 0, &mut state)?;

    let canonical_bytes = canonical_json(content);
    let canonical: Value = serde_json::from_slice(&canonical_bytes).map_err(|error| {
        DocumentError::Invalid(format!("content cannot be normalized: {error}"))
    })?;
    let content_hash = sha256_hex(&canonical_bytes);
    let rendered_html = render_node(&canonical)?;
    let markdown = render_markdown_document(&canonical)?;
    let mut plain = String::new();
    collect_plain_text(&canonical, &mut plain)?;

    Ok(ValidatedDocument {
        content_json: canonical,
        plain_text: normalize_plain_text(&plain),
        rendered_html,
        markdown,
        content_hash,
    })
}

fn validate_node(
    value: &Value,
    expected: ChildRule,
    depth: usize,
    state: &mut ValidationState,
) -> DocumentResult<()> {
    if depth > MAX_DOCUMENT_DEPTH {
        return Err(DocumentError::Invalid(format!(
            "content exceeds {MAX_DOCUMENT_DEPTH} levels"
        )));
    }
    state.nodes += 1;
    if state.nodes > MAX_DOCUMENT_NODES {
        return Err(DocumentError::Invalid(format!(
            "content exceeds {MAX_DOCUMENT_NODES} nodes"
        )));
    }

    let object = value
        .as_object()
        .ok_or_else(|| DocumentError::Invalid("every content node must be an object".into()))?;
    let node_type = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| DocumentError::Invalid("every content node needs a string type".into()))?;
    if !node_allowed(node_type, expected) {
        return Err(DocumentError::Invalid(format!(
            "node {node_type:?} is not allowed in this position"
        )));
    }

    if node_type == "text" {
        reject_unknown_keys(object, &["type", "text", "marks"], "text node")?;
        let text = object
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| DocumentError::Invalid("text node needs string text".into()))?;
        reject_nul("text", text)?;
        state.text_chars += text.chars().count();
        if state.text_chars > MAX_DOCUMENT_TEXT_CHARS {
            return Err(DocumentError::Invalid(format!(
                "content exceeds {MAX_DOCUMENT_TEXT_CHARS} text characters"
            )));
        }
        validate_marks(object.get("marks"))?;
        return Ok(());
    }

    reject_unknown_keys(object, &["type", "attrs", "content"], "content node")?;
    validate_node_attrs(node_type, object.get("attrs"), state)?;
    let children = match object.get("content") {
        None => &[][..],
        Some(Value::Array(children)) => children.as_slice(),
        Some(_) => {
            return Err(DocumentError::Invalid(format!(
                "node {node_type:?} content must be an array"
            )));
        }
    };
    let child_rule = child_rule(node_type);
    if child_rule == ChildRule::None && !children.is_empty() {
        return Err(DocumentError::Invalid(format!(
            "leaf node {node_type:?} cannot contain children"
        )));
    }
    if matches!(
        node_type,
        "bulletList" | "orderedList" | "taskList" | "table" | "tableRow"
    ) && children.is_empty()
    {
        return Err(DocumentError::Invalid(format!(
            "node {node_type:?} cannot be empty"
        )));
    }
    if node_type == "doc" {
        for child in children {
            let block_id = child
                .get("attrs")
                .and_then(|attrs| attrs.get("block_id"))
                .and_then(Value::as_str);
            if block_id.is_none() {
                return Err(DocumentError::Invalid(
                    "every top-level content block needs a stable block_id".into(),
                ));
            }
        }
    }
    for child in children {
        validate_node(child, child_rule, depth + 1, state)?;
    }
    Ok(())
}

fn node_allowed(node_type: &str, expected: ChildRule) -> bool {
    match expected {
        ChildRule::Document => node_type == "doc",
        ChildRule::Block => matches!(
            node_type,
            "paragraph"
                | "heading"
                | "bulletList"
                | "orderedList"
                | "taskList"
                | "blockquote"
                | "codeBlock"
                | "horizontalRule"
                | "table"
        ),
        ChildRule::Inline => matches!(node_type, "text" | "hardBreak" | "mention" | "reference"),
        ChildRule::ListItem => node_type == "listItem",
        ChildRule::TaskItem => node_type == "taskItem",
        ChildRule::TableRow => node_type == "tableRow",
        ChildRule::TableCell => matches!(node_type, "tableCell" | "tableHeader"),
        ChildRule::Text => node_type == "text",
        ChildRule::None => false,
    }
}

fn child_rule(node_type: &str) -> ChildRule {
    match node_type {
        "doc" | "blockquote" | "listItem" | "taskItem" | "tableCell" | "tableHeader" => {
            ChildRule::Block
        }
        "paragraph" | "heading" => ChildRule::Inline,
        "bulletList" | "orderedList" => ChildRule::ListItem,
        "taskList" => ChildRule::TaskItem,
        "table" => ChildRule::TableRow,
        "tableRow" => ChildRule::TableCell,
        "codeBlock" => ChildRule::Text,
        _ => ChildRule::None,
    }
}

fn validate_node_attrs(
    node_type: &str,
    attrs: Option<&Value>,
    state: &mut ValidationState,
) -> DocumentResult<()> {
    let attrs = optional_object(attrs, &format!("{node_type} attrs"))?;
    let allowed = match node_type {
        "heading" => &["level", "block_id"][..],
        "orderedList" => &["start", "block_id"][..],
        "taskItem" => &["checked", "block_id"][..],
        "codeBlock" => &["language", "block_id"][..],
        "mention" => &["actor_id", "label"][..],
        "reference" => &["kind", "id", "label"][..],
        "paragraph" | "bulletList" | "taskList" | "blockquote" | "horizontalRule" | "table"
        | "listItem" | "tableRow" | "tableCell" | "tableHeader" => &["block_id"][..],
        "doc" | "hardBreak" => &[][..],
        _ => &[][..],
    };
    reject_unknown_keys(attrs, allowed, &format!("{node_type} attrs"))?;

    if let Some(block_id) = attrs.get("block_id") {
        let block_id = bounded_string("block_id", block_id, 128)?;
        if !block_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
        {
            return Err(DocumentError::Invalid(
                "block_id contains unsupported characters".into(),
            ));
        }
        if !state.block_ids.insert(block_id.to_string()) {
            return Err(DocumentError::Invalid(format!(
                "block_id {block_id:?} is repeated"
            )));
        }
    }

    match node_type {
        "heading" => {
            let level = attrs.get("level").and_then(Value::as_i64).ok_or_else(|| {
                DocumentError::Invalid("heading needs integer level 1 through 6".into())
            })?;
            if !(1..=6).contains(&level) {
                return Err(DocumentError::Invalid(
                    "heading level must be between 1 and 6".into(),
                ));
            }
        }
        "orderedList" => {
            if let Some(start) = attrs.get("start") {
                let start = start.as_i64().ok_or_else(|| {
                    DocumentError::Invalid("ordered-list start must be an integer".into())
                })?;
                if !(1..=100_000).contains(&start) {
                    return Err(DocumentError::Invalid(
                        "ordered-list start must be between 1 and 100000".into(),
                    ));
                }
            }
        }
        "taskItem" => {
            if !attrs.get("checked").is_some_and(Value::is_boolean) {
                return Err(DocumentError::Invalid(
                    "task item needs a boolean checked attribute".into(),
                ));
            }
        }
        "codeBlock" => {
            if let Some(language) = attrs.get("language") {
                bounded_string("code-block language", language, 64)?;
            }
        }
        "mention" => {
            bounded_string(
                "mention actor_id",
                attrs
                    .get("actor_id")
                    .ok_or_else(|| DocumentError::Invalid("mention needs actor_id".into()))?,
                128,
            )?;
            bounded_string(
                "mention label",
                attrs
                    .get("label")
                    .ok_or_else(|| DocumentError::Invalid("mention needs label".into()))?,
                200,
            )?;
        }
        "reference" => {
            let kind = bounded_string(
                "reference kind",
                attrs
                    .get("kind")
                    .ok_or_else(|| DocumentError::Invalid("reference needs kind".into()))?,
                32,
            )?;
            if !matches!(
                kind,
                "work" | "decision" | "attention" | "document" | "artifact" | "actor"
            ) {
                return Err(DocumentError::Invalid(format!(
                    "reference kind {kind:?} is unsupported"
                )));
            }
            bounded_string(
                "reference id",
                attrs
                    .get("id")
                    .ok_or_else(|| DocumentError::Invalid("reference needs id".into()))?,
                200,
            )?;
            bounded_string(
                "reference label",
                attrs
                    .get("label")
                    .ok_or_else(|| DocumentError::Invalid("reference needs label".into()))?,
                300,
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn validate_marks(value: Option<&Value>) -> DocumentResult<()> {
    let Some(value) = value else {
        return Ok(());
    };
    let marks = value
        .as_array()
        .ok_or_else(|| DocumentError::Invalid("text marks must be an array".into()))?;
    let mut seen = HashSet::new();
    for mark in marks {
        let object = mark
            .as_object()
            .ok_or_else(|| DocumentError::Invalid("every text mark must be an object".into()))?;
        reject_unknown_keys(object, &["type", "attrs"], "text mark")?;
        let mark_type = object
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| DocumentError::Invalid("every text mark needs a string type".into()))?;
        if !matches!(mark_type, "bold" | "italic" | "strike" | "code" | "link") {
            return Err(DocumentError::Invalid(format!(
                "mark {mark_type:?} is unsupported"
            )));
        }
        if !seen.insert(mark_type) {
            return Err(DocumentError::Invalid(format!(
                "mark {mark_type:?} is repeated"
            )));
        }
        let attrs = optional_object(object.get("attrs"), &format!("{mark_type} mark attrs"))?;
        if mark_type == "link" {
            reject_unknown_keys(attrs, &["href", "title"], "link mark attrs")?;
            let href = bounded_string(
                "link href",
                attrs
                    .get("href")
                    .ok_or_else(|| DocumentError::Invalid("link needs href".into()))?,
                2_048,
            )?;
            if !safe_href(href) {
                return Err(DocumentError::Invalid(
                    "link href must be https, http, mailto, an absolute path, or a fragment".into(),
                ));
            }
            if let Some(title) = attrs.get("title") {
                bounded_string("link title", title, 300)?;
            }
        } else if !attrs.is_empty() {
            return Err(DocumentError::Invalid(format!(
                "mark {mark_type:?} does not accept attributes"
            )));
        }
    }
    Ok(())
}

fn optional_object<'a>(
    value: Option<&'a Value>,
    label: &str,
) -> DocumentResult<&'a Map<String, Value>> {
    static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    match value {
        None => Ok(EMPTY.get_or_init(Map::new)),
        Some(Value::Object(object)) => Ok(object),
        Some(_) => Err(DocumentError::Invalid(format!("{label} must be an object"))),
    }
}

fn reject_unknown_keys(
    object: &Map<String, Value>,
    allowed: &[&str],
    label: &str,
) -> DocumentResult<()> {
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(DocumentError::Invalid(format!(
            "{label} contains unsupported field {key:?}"
        )));
    }
    Ok(())
}

fn bounded_string<'a>(label: &str, value: &'a Value, maximum: usize) -> DocumentResult<&'a str> {
    let string = value
        .as_str()
        .ok_or_else(|| DocumentError::Invalid(format!("{label} must be a string")))?;
    if string.trim().is_empty() || string.chars().count() > maximum {
        return Err(DocumentError::Invalid(format!(
            "{label} must contain 1 to {maximum} characters"
        )));
    }
    reject_nul(label, string)?;
    Ok(string)
}

fn reject_nul(label: &str, value: &str) -> DocumentResult<()> {
    if value.contains('\0') {
        return Err(DocumentError::Invalid(format!("{label} contains NUL")));
    }
    Ok(())
}

fn safe_href(value: &str) -> bool {
    if value
        .chars()
        .any(|character| character.is_control() || character.is_whitespace())
    {
        return false;
    }
    if let Some(authority_and_path) = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
    {
        return !authority_and_path.is_empty() && !authority_and_path.starts_with('/');
    }
    if let Some(address) = value.strip_prefix("mailto:") {
        return address
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && !domain.is_empty());
    }
    (value.starts_with('/') && !value.starts_with("//") && !value.contains('\\'))
        || (value.starts_with('#') && value.len() > 1)
}

fn canonical_json(value: &Value) -> Vec<u8> {
    fn write(value: &Value, output: &mut String) {
        match value {
            Value::Object(object) => {
                output.push('{');
                let sorted = object.iter().collect::<BTreeMap<_, _>>();
                for (index, (key, value)) in sorted.into_iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    output.push_str(&serde_json::to_string(key).expect("JSON object key"));
                    output.push(':');
                    write(value, output);
                }
                output.push('}');
            }
            Value::Array(array) => {
                output.push('[');
                for (index, value) in array.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    write(value, output);
                }
                output.push(']');
            }
            _ => output.push_str(&serde_json::to_string(value).expect("JSON scalar")),
        }
    }

    let mut output = String::new();
    write(value, &mut output);
    output.into_bytes()
}

fn sha256_hex(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn valid_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn document_status_name(status: DocumentStatus) -> &'static str {
    match status {
        DocumentStatus::Draft => "draft",
        DocumentStatus::InReview => "in_review",
        DocumentStatus::Accepted => "accepted",
        DocumentStatus::Archived => "archived",
    }
}

fn portable_markdown(envelope: &NativeDocumentPortableEnvelope, content: &str) -> String {
    let hosted_company_id = envelope
        .source_company_id
        .map(|company_id| company_id.to_string())
        .unwrap_or_else(|| "unbound".into());
    format!(
        "---\nrestless_envelope: {}\nrestless_envelope_version: {}\n\
         source_company_id: {}\nsource_company_key: {}\nsource_document_id: {}\n\
         source_named_version_id: {}\nsource_named_version_number: {}\n\
         source_content_schema_version: {}\nsource_content_hash: {}\ncontent_hash: {}\n\
         document_status: {}\n---\n\n{}",
        envelope.envelope_schema,
        envelope.envelope_version,
        hosted_company_id,
        envelope.source_company_key,
        envelope.source_document_id,
        envelope.source_named_version_id,
        envelope.source_named_version_number,
        envelope.source_content_schema_version,
        envelope.source_content_hash,
        envelope.content_hash,
        document_status_name(envelope.source_document_status),
        content
    )
}

fn portable_envelope_value(envelope: &NativeDocumentPortableEnvelope) -> DocumentResult<Value> {
    serde_json::to_value(envelope)
        .map_err(|error| DocumentError::Invalid(format!("portable envelope is not JSON: {error}")))
}

fn portable_envelope_checksum(envelope: &NativeDocumentPortableEnvelope) -> DocumentResult<String> {
    let mut value = portable_envelope_value(envelope)?;
    value
        .as_object_mut()
        .expect("portable envelope serializes as an object")
        .insert("checksum".into(), Value::String(String::new()));
    Ok(sha256_hex(&canonical_json(&value)))
}

fn ensure_portable_envelope_bound(envelope: &NativeDocumentPortableEnvelope) -> DocumentResult<()> {
    let bytes = canonical_json(&portable_envelope_value(envelope)?);
    if bytes.len() > MAX_PORTABLE_ENVELOPE_BYTES {
        return Err(DocumentError::Invalid(format!(
            "portable envelope exceeds {MAX_PORTABLE_ENVELOPE_BYTES} bytes"
        )));
    }
    Ok(())
}

fn validate_portable_envelope_header(
    envelope: &NativeDocumentPortableEnvelope,
) -> DocumentResult<()> {
    if envelope.envelope_schema != NATIVE_DOCUMENT_PORTABLE_ENVELOPE_SCHEMA {
        return Err(DocumentError::Invalid(
            "unknown native document portable envelope schema".into(),
        ));
    }
    if envelope.envelope_version != NATIVE_DOCUMENT_PORTABLE_ENVELOPE_VERSION {
        return Err(DocumentError::Invalid(format!(
            "unsupported native document portable envelope version {}",
            envelope.envelope_version
        )));
    }
    if envelope.source_content_schema_version != DOCUMENT_SCHEMA_VERSION {
        return Err(DocumentError::Invalid(format!(
            "unsupported portable document content schema {}",
            envelope.source_content_schema_version
        )));
    }
    if envelope.source_named_version_number < 1 {
        return Err(DocumentError::Invalid(
            "portable source named version must be positive".into(),
        ));
    }
    for (label, value, maximum) in [
        (
            "source company key",
            envelope.source_company_key.as_str(),
            128,
        ),
        (
            "source version Actor",
            envelope.source_created_by_actor_id.as_str(),
            200,
        ),
        (
            "source version reason",
            envelope.source_version_reason.as_str(),
            500,
        ),
    ] {
        if clean_bounded(label, value, maximum)? != value {
            return Err(DocumentError::Invalid(format!(
                "portable {label} must already be canonical"
            )));
        }
    }
    if !valid_sha256_hex(&envelope.source_content_hash) {
        return Err(DocumentError::Invalid(
            "portable source content hash is invalid".into(),
        ));
    }
    Ok(())
}

fn validate_portable_envelope(
    envelope: &NativeDocumentPortableEnvelope,
) -> DocumentResult<ValidatedDocument> {
    validate_portable_envelope_header(envelope)?;
    ensure_portable_envelope_bound(envelope)?;
    if !valid_sha256_hex(&envelope.checksum) {
        return Err(DocumentError::Invalid(
            "portable envelope checksum is invalid".into(),
        ));
    }
    if portable_envelope_checksum(envelope)? != envelope.checksum {
        return Err(DocumentError::Invalid(
            "portable envelope checksum does not match its payload".into(),
        ));
    }

    let content = validate_document_json(&envelope.content_json)?;
    if content.content_hash != envelope.content_hash {
        return Err(DocumentError::Invalid(
            "portable content hash does not match structured content".into(),
        ));
    }
    if content.plain_text != envelope.plain_text {
        return Err(DocumentError::Invalid(
            "portable plain-text projection does not match structured content".into(),
        ));
    }
    if portable_markdown(envelope, &content.markdown) != envelope.markdown {
        return Err(DocumentError::Invalid(
            "portable Markdown projection does not match structured content".into(),
        ));
    }
    Ok(content)
}

fn deterministic_import_version_id(
    target_document_id: Uuid,
    actor_id: &str,
    expected_current_version_id: Uuid,
    reason: &str,
    envelope_checksum: &str,
) -> Uuid {
    fn add_component(hasher: &mut Sha256, value: &[u8]) {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value);
    }

    let mut hasher = Sha256::new();
    add_component(&mut hasher, b"restless.native-document.import.v1");
    add_component(&mut hasher, target_document_id.as_bytes());
    add_component(&mut hasher, actor_id.as_bytes());
    add_component(&mut hasher, expected_current_version_id.as_bytes());
    add_component(&mut hasher, reason.as_bytes());
    add_component(&mut hasher, envelope_checksum.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    // RFC 9562 UUIDv8: application-defined payload with the standard variant.
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

fn render_node(value: &Value) -> DocumentResult<String> {
    let object = value
        .as_object()
        .ok_or_else(|| DocumentError::Corrupt("stored node is not an object".into()))?;
    let node_type = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| DocumentError::Corrupt("stored node has no type".into()))?;
    let children = object
        .get("content")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut inner = String::new();
    for child in children {
        inner.push_str(&render_node(child)?);
    }
    let block = block_id_attribute(object.get("attrs"));
    match node_type {
        "doc" => Ok(inner),
        "paragraph" => Ok(format!("<p{block}>{inner}</p>")),
        "heading" => {
            let level = object["attrs"]["level"].as_i64().unwrap_or(1);
            Ok(format!("<h{level}{block}>{inner}</h{level}>"))
        }
        "text" => render_text(object),
        "hardBreak" => Ok("<br>".into()),
        "bulletList" => Ok(format!("<ul{block}>{inner}</ul>")),
        "orderedList" => {
            let start = object
                .get("attrs")
                .and_then(|attrs| attrs.get("start"))
                .and_then(Value::as_i64);
            let start = start
                .filter(|start| *start != 1)
                .map(|start| format!(" start=\"{start}\""))
                .unwrap_or_default();
            Ok(format!("<ol{block}{start}>{inner}</ol>"))
        }
        "listItem" => Ok(format!("<li{block}>{inner}</li>")),
        "taskList" => Ok(format!("<ul data-task-list=\"true\"{block}>{inner}</ul>")),
        "taskItem" => {
            let checked = object["attrs"]["checked"].as_bool().unwrap_or(false);
            Ok(format!(
                "<li data-task-item=\"true\" data-checked=\"{checked}\"{block}>{inner}</li>"
            ))
        }
        "blockquote" => Ok(format!("<blockquote{block}>{inner}</blockquote>")),
        "codeBlock" => {
            let language = object
                .get("attrs")
                .and_then(|attrs| attrs.get("language"))
                .and_then(Value::as_str)
                .map(|language| format!(" data-language=\"{}\"", escape_html_attribute(language)))
                .unwrap_or_default();
            Ok(format!("<pre{block}><code{language}>{inner}</code></pre>"))
        }
        "horizontalRule" => Ok(format!("<hr{block}>")),
        "table" => Ok(format!("<table{block}><tbody>{inner}</tbody></table>")),
        "tableRow" => Ok(format!("<tr{block}>{inner}</tr>")),
        "tableHeader" => Ok(format!("<th{block}>{inner}</th>")),
        "tableCell" => Ok(format!("<td{block}>{inner}</td>")),
        "mention" => {
            let attrs = &object["attrs"];
            let actor = escape_html_attribute(attrs["actor_id"].as_str().unwrap_or_default());
            let label = escape_html(attrs["label"].as_str().unwrap_or_default());
            Ok(format!(
                "<span data-actor-id=\"{actor}\" data-native-mention=\"true\">@{label}</span>"
            ))
        }
        "reference" => {
            let attrs = &object["attrs"];
            let kind = escape_html_attribute(attrs["kind"].as_str().unwrap_or_default());
            let id = escape_html_attribute(attrs["id"].as_str().unwrap_or_default());
            let label = escape_html(attrs["label"].as_str().unwrap_or_default());
            Ok(format!(
                "<span data-reference-kind=\"{kind}\" data-reference-id=\"{id}\">{label}</span>"
            ))
        }
        _ => Err(DocumentError::Corrupt(format!(
            "stored node {node_type:?} is unsupported"
        ))),
    }
}

fn render_text(object: &Map<String, Value>) -> DocumentResult<String> {
    let text = object
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| DocumentError::Corrupt("stored text node has no text".into()))?;
    let mut rendered = escape_html(text);
    if let Some(marks) = object.get("marks").and_then(Value::as_array) {
        for mark in marks.iter().rev() {
            let mark = mark
                .as_object()
                .ok_or_else(|| DocumentError::Corrupt("stored mark is not an object".into()))?;
            match mark.get("type").and_then(Value::as_str).unwrap_or_default() {
                "bold" => rendered = format!("<strong>{rendered}</strong>"),
                "italic" => rendered = format!("<em>{rendered}</em>"),
                "strike" => rendered = format!("<s>{rendered}</s>"),
                "code" => rendered = format!("<code>{rendered}</code>"),
                "link" => {
                    let attrs = mark
                        .get("attrs")
                        .and_then(Value::as_object)
                        .ok_or_else(|| {
                            DocumentError::Corrupt("stored link has no attributes".into())
                        })?;
                    let href = escape_html_attribute(
                        attrs
                            .get("href")
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                    );
                    let title = attrs
                        .get("title")
                        .and_then(Value::as_str)
                        .map(|title| format!(" title=\"{}\"", escape_html_attribute(title)))
                        .unwrap_or_default();
                    rendered = format!(
                        "<a href=\"{href}\"{title} rel=\"noopener noreferrer\">{rendered}</a>"
                    );
                }
                other => {
                    return Err(DocumentError::Corrupt(format!(
                        "stored mark {other:?} is unsupported"
                    )));
                }
            }
        }
    }
    Ok(rendered)
}

fn block_id_attribute(attrs: Option<&Value>) -> String {
    attrs
        .and_then(|attrs| attrs.get("block_id"))
        .and_then(Value::as_str)
        .map(|id| format!(" data-block-id=\"{}\"", escape_html_attribute(id)))
        .unwrap_or_default()
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn escape_html_attribute(value: &str) -> String {
    escape_html(value)
}

fn render_markdown_document(value: &Value) -> DocumentResult<String> {
    let object = value
        .as_object()
        .ok_or_else(|| DocumentError::Corrupt("stored document is not an object".into()))?;
    if object.get("type").and_then(Value::as_str) != Some("doc") {
        return Err(DocumentError::Corrupt(
            "stored structured content is not a document".into(),
        ));
    }
    let blocks = object
        .get("content")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut rendered = Vec::with_capacity(blocks.len());
    for block in blocks {
        rendered.push(render_markdown_block(block, 0)?);
    }
    Ok(rendered
        .into_iter()
        .filter(|block| !block.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
        .trim()
        .to_string())
}

fn render_markdown_block(value: &Value, depth: usize) -> DocumentResult<String> {
    let object = value
        .as_object()
        .ok_or_else(|| DocumentError::Corrupt("stored block is not an object".into()))?;
    let node_type = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| DocumentError::Corrupt("stored block has no type".into()))?;
    let children = object
        .get("content")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    match node_type {
        "paragraph" => render_markdown_inline_children(children),
        "heading" => {
            let level = object["attrs"]["level"].as_u64().unwrap_or(1).clamp(1, 6);
            Ok(format!(
                "{} {}",
                "#".repeat(level as usize),
                render_markdown_inline_children(children)?
            ))
        }
        "bulletList" => render_markdown_list(children, depth, false, false, 1),
        "orderedList" => {
            let start = object
                .get("attrs")
                .and_then(|attrs| attrs.get("start"))
                .and_then(Value::as_i64)
                .unwrap_or(1);
            render_markdown_list(children, depth, true, false, start)
        }
        "taskList" => render_markdown_list(children, depth, false, true, 1),
        "blockquote" => {
            let inner = render_markdown_blocks(children, depth)?;
            Ok(inner
                .lines()
                .map(|line| format!("> {line}"))
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "codeBlock" => {
            let mut code = String::new();
            for child in children {
                let child = child.as_object().ok_or_else(|| {
                    DocumentError::Corrupt("stored code child is not an object".into())
                })?;
                code.push_str(
                    child
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                );
            }
            let language = object
                .get("attrs")
                .and_then(|attrs| attrs.get("language"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            let fence = if code.contains("```") { "````" } else { "```" };
            Ok(format!("{fence}{language}\n{code}\n{fence}"))
        }
        "horizontalRule" => Ok("---".into()),
        "table" => render_markdown_table(children),
        other => Err(DocumentError::Corrupt(format!(
            "stored block {other:?} cannot be projected to Markdown"
        ))),
    }
}

fn render_markdown_blocks(children: &[Value], depth: usize) -> DocumentResult<String> {
    let mut blocks = Vec::with_capacity(children.len());
    for child in children {
        blocks.push(render_markdown_block(child, depth)?);
    }
    Ok(blocks.join("\n\n"))
}

fn render_markdown_list(
    items: &[Value],
    depth: usize,
    ordered: bool,
    task: bool,
    start: i64,
) -> DocumentResult<String> {
    let mut lines = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let object = item
            .as_object()
            .ok_or_else(|| DocumentError::Corrupt("stored list item is not an object".into()))?;
        let children = object
            .get("content")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let marker = if task {
            if object["attrs"]["checked"].as_bool().unwrap_or(false) {
                "- [x]".to_string()
            } else {
                "- [ ]".to_string()
            }
        } else if ordered {
            format!("{}.", start + index as i64)
        } else {
            "-".to_string()
        };
        let indent = "  ".repeat(depth);
        let body = render_markdown_blocks(children, depth + 1)?;
        let mut body_lines = body.lines();
        let first = body_lines.next().unwrap_or_default();
        lines.push(format!("{indent}{marker} {first}"));
        let continuation_indent = " ".repeat(indent.len() + marker.len() + 1);
        lines.extend(body_lines.map(|line| format!("{continuation_indent}{line}")));
    }
    Ok(lines.join("\n"))
}

fn render_markdown_table(rows: &[Value]) -> DocumentResult<String> {
    let mut rendered_rows = Vec::with_capacity(rows.len());
    for row in rows {
        let cells = row
            .get("content")
            .and_then(Value::as_array)
            .ok_or_else(|| DocumentError::Corrupt("stored table row has no cells".into()))?;
        let mut rendered_cells = Vec::with_capacity(cells.len());
        for cell in cells {
            let blocks = cell
                .get("content")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or_default();
            rendered_cells.push(
                render_markdown_blocks(blocks, 0)?
                    .replace('\n', "<br>")
                    .replace('|', "\\|"),
            );
        }
        rendered_rows.push(rendered_cells);
    }
    let width = rendered_rows.iter().map(Vec::len).max().unwrap_or_default();
    if width == 0 {
        return Ok(String::new());
    }
    for row in &mut rendered_rows {
        row.resize(width, String::new());
    }
    let mut lines = Vec::with_capacity(rendered_rows.len() + 1);
    lines.push(format!("| {} |", rendered_rows[0].join(" | ")));
    lines.push(format!("| {} |", vec!["---"; width].join(" | ")));
    for row in rendered_rows.iter().skip(1) {
        lines.push(format!("| {} |", row.join(" | ")));
    }
    Ok(lines.join("\n"))
}

fn render_markdown_inline_children(children: &[Value]) -> DocumentResult<String> {
    let mut output = String::new();
    for child in children {
        output.push_str(&render_markdown_inline(child)?);
    }
    Ok(output)
}

fn render_markdown_inline(value: &Value) -> DocumentResult<String> {
    let object = value
        .as_object()
        .ok_or_else(|| DocumentError::Corrupt("stored inline node is not an object".into()))?;
    match object
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
    {
        "text" => {
            let raw = object
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let mut rendered = escape_markdown_text(raw);
            if let Some(marks) = object.get("marks").and_then(Value::as_array) {
                for mark in marks.iter().rev() {
                    let mark = mark.as_object().ok_or_else(|| {
                        DocumentError::Corrupt("stored text mark is not an object".into())
                    })?;
                    match mark.get("type").and_then(Value::as_str).unwrap_or_default() {
                        "bold" => rendered = format!("**{rendered}**"),
                        "italic" => rendered = format!("_{rendered}_"),
                        "strike" => rendered = format!("~~{rendered}~~"),
                        "code" => rendered = format!("`{rendered}`"),
                        "link" => {
                            let attrs =
                                mark.get("attrs")
                                    .and_then(Value::as_object)
                                    .ok_or_else(|| {
                                        DocumentError::Corrupt(
                                            "stored link has no attributes".into(),
                                        )
                                    })?;
                            let href = attrs
                                .get("href")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .replace('\\', "%5C")
                                .replace('(', "%28")
                                .replace(')', "%29");
                            let title = attrs
                                .get("title")
                                .and_then(Value::as_str)
                                .map(|title| format!(" \"{}\"", title.replace('"', "\\\"")))
                                .unwrap_or_default();
                            rendered = format!("[{rendered}]({href}{title})");
                        }
                        other => {
                            return Err(DocumentError::Corrupt(format!(
                                "stored mark {other:?} cannot be projected to Markdown"
                            )));
                        }
                    }
                }
            }
            Ok(rendered)
        }
        "hardBreak" => Ok("  \n".into()),
        "mention" => Ok(format!(
            "@{}",
            escape_markdown_text(object["attrs"]["label"].as_str().unwrap_or_default())
        )),
        "reference" => Ok(escape_markdown_text(
            object["attrs"]["label"].as_str().unwrap_or_default(),
        )),
        other => Err(DocumentError::Corrupt(format!(
            "stored inline node {other:?} cannot be projected to Markdown"
        ))),
    }
}

fn escape_markdown_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(
            character,
            '\\' | '`'
                | '*'
                | '_'
                | '{'
                | '}'
                | '['
                | ']'
                | '<'
                | '>'
                | '#'
                | '+'
                | '-'
                | '!'
                | '|'
        ) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

fn collect_plain_text(value: &Value, output: &mut String) -> DocumentResult<()> {
    let object = value
        .as_object()
        .ok_or_else(|| DocumentError::Corrupt("stored node is not an object".into()))?;
    let node_type = object
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| DocumentError::Corrupt("stored node has no type".into()))?;
    match node_type {
        "text" => output.push_str(
            object
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default(),
        ),
        "mention" => {
            output.push('@');
            output.push_str(object["attrs"]["label"].as_str().unwrap_or_default());
        }
        "reference" => output.push_str(object["attrs"]["label"].as_str().unwrap_or_default()),
        "hardBreak" => output.push('\n'),
        _ => {
            if let Some(children) = object.get("content").and_then(Value::as_array) {
                for child in children {
                    collect_plain_text(child, output)?;
                }
            }
            if matches!(
                node_type,
                "paragraph"
                    | "heading"
                    | "listItem"
                    | "taskItem"
                    | "blockquote"
                    | "codeBlock"
                    | "horizontalRule"
                    | "tableRow"
            ) {
                output.push('\n');
            } else if matches!(node_type, "tableCell" | "tableHeader") {
                output.push('\t');
            }
        }
    }
    Ok(())
}

fn normalize_plain_text(value: &str) -> String {
    let mut output = String::new();
    let mut previous_blank = true;
    for line in value.lines() {
        let line = line.trim_end_matches([' ', '\t']);
        let blank = line.is_empty();
        if blank && previous_blank {
            continue;
        }
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(line);
        previous_blank = blank;
    }
    output.trim().to_string()
}

fn document_select() -> &'static str {
    "SELECT id,title,kind,status,visibility,linked_room_id,inherit_room_visibility,\
            owner_actor_id,current_named_version_id,created_by_actor_id,created_at,updated_at,version \
     FROM native_documents"
}

fn version_select() -> &'static str {
    "SELECT id,document_id,version_number,schema_version,content_json,plain_text,content_hash,\
            document_status,restored_from_version_id,created_by_actor_id,reason,created_at \
     FROM native_document_versions"
}

fn runtime_import_provenance_select() -> &'static str {
    "SELECT document_id,imported_version_id,target_base_version_id,envelope_schema,\
            envelope_version,envelope_checksum,source_company_id,source_company_key,\
            source_document_id,source_named_version_id,source_named_version_number,\
            source_content_schema_version,source_document_status,\
            source_restored_from_version_id,source_created_by_actor_id,source_version_reason,\
            source_version_created_at,source_content_hash,imported_content_hash,\
            imported_by_actor_id,imported_at \
     FROM native_document_runtime_imports"
}

fn runtime_import_provenance_matches(
    provenance: &DocumentRuntimeImportProvenance,
    target_document_id: Uuid,
    imported_version_id: Uuid,
    target_base_version_id: Uuid,
    envelope: &NativeDocumentPortableEnvelope,
    imported_content_hash: &str,
    imported_by_actor_id: &str,
) -> bool {
    provenance.document_id == target_document_id
        && provenance.imported_version_id == imported_version_id
        && provenance.target_base_version_id == target_base_version_id
        && provenance.envelope_schema == envelope.envelope_schema
        && provenance.envelope_version == envelope.envelope_version as i16
        && provenance.envelope_checksum == envelope.checksum
        && provenance.source_company_id == envelope.source_company_id
        && provenance.source_company_key == envelope.source_company_key
        && provenance.source_document_id == envelope.source_document_id
        && provenance.source_named_version_id == envelope.source_named_version_id
        && provenance.source_named_version_number == envelope.source_named_version_number
        && provenance.source_content_schema_version == envelope.source_content_schema_version
        && provenance.source_document_status == envelope.source_document_status
        && provenance.source_restored_from_version_id == envelope.source_restored_from_version_id
        && provenance.source_created_by_actor_id == envelope.source_created_by_actor_id
        && provenance.source_version_reason == envelope.source_version_reason
        && provenance.source_version_created_at == envelope.source_version_created_at
        && provenance.source_content_hash == envelope.source_content_hash
        && provenance.imported_content_hash == imported_content_hash
        && provenance.imported_by_actor_id == imported_by_actor_id
}

async fn require_runtime_source_claims(
    tx: &mut Transaction<'_, Postgres>,
    company_schema: &str,
    target_document_id: Uuid,
    expected_current_version_id: Uuid,
    envelope: &NativeDocumentPortableEnvelope,
) -> DocumentResult<()> {
    let company_id: Option<Uuid> =
        sqlx::query_scalar("SELECT company_id FROM company_access_identity WHERE singleton=TRUE")
            .fetch_optional(&mut **tx)
            .await?;
    if envelope.source_company_id != company_id
        || envelope.source_company_key != company_schema
        || envelope.source_document_id != target_document_id
        || envelope.source_named_version_id != expected_current_version_id
    {
        return Err(DocumentError::Invalid(
            "portable source provenance does not identify this Core document checkpoint".into(),
        ));
    }

    let source = sqlx::query_as::<_, DocumentVersionRow>(&format!(
        "{} WHERE document_id=$1 AND id=$2",
        version_select()
    ))
    .bind(target_document_id)
    .bind(expected_current_version_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        DocumentError::Invalid(
            "portable source provenance does not identify a stored named version".into(),
        )
    })?;
    if envelope.source_named_version_number != source.version_number
        || envelope.source_content_schema_version != source.schema_version
        || envelope.source_document_status != source.document_status
        || envelope.source_restored_from_version_id != source.restored_from_version_id
        || envelope.source_created_by_actor_id != source.created_by_actor_id
        || envelope.source_version_reason != source.reason
        || envelope.source_version_created_at != source.created_at
        || envelope.source_content_hash != source.content_hash
    {
        return Err(DocumentError::Invalid(
            "portable source provenance does not match the immutable named version".into(),
        ));
    }
    Ok(())
}

async fn active_actor(tx: &mut Transaction<'_, Postgres>, actor_id: &str) -> DocumentResult<bool> {
    Ok(
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL)",
        )
        .bind(actor_id)
        .fetch_one(&mut **tx)
        .await?,
    )
}

/// Append a body-free document refetch hint to the existing operational event
/// stream. Callers invoke this inside the same transaction as the authoritative
/// mutation, so neither the change nor its notification can commit alone.
async fn append_document_event(
    tx: &mut Transaction<'_, Postgres>,
    kind: &str,
    document_id: Uuid,
    actor_id: &str,
    body: Value,
) -> DocumentResult<i64> {
    Ok(sqlx::query_scalar(
        "INSERT INTO events (kind,document_id,actor_id,body) \
         VALUES ($1,$2,$3,$4) RETURNING id",
    )
    .bind(kind)
    .bind(document_id)
    .bind(actor_id)
    .bind(body)
    .fetch_one(&mut **tx)
    .await?)
}

async fn active_room_participant(
    tx: &mut Transaction<'_, Postgres>,
    room_id: Uuid,
    actor_id: &str,
) -> DocumentResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(\
           SELECT 1 FROM rooms room \
           JOIN room_participants participant ON participant.room_id=room.id \
           JOIN actors actor ON actor.id=participant.actor_id \
           WHERE room.id=$1 AND participant.actor_id=$2 \
             AND room.archived_at IS NULL AND participant.left_at IS NULL \
             AND actor.retired_at IS NULL\
         )",
    )
    .bind(room_id)
    .bind(actor_id)
    .fetch_one(&mut **tx)
    .await?)
}

async fn access_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    document_id: Uuid,
    actor_id: &str,
) -> DocumentResult<Option<DocumentAccess>> {
    // Reads and writes both serialize first on the document. In particular,
    // participant revocation and named-version creation take this row FOR
    // UPDATE, so neither can split an authorized projection into an old access
    // decision followed by post-revocation content at READ COMMITTED.
    let document: Option<(String, DocumentVisibility, Option<Uuid>, bool)> = sqlx::query_as(
        "SELECT owner_actor_id,visibility,linked_room_id,inherit_room_visibility \
         FROM native_documents WHERE id=$1 FOR SHARE",
    )
    .bind(document_id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((owner_actor_id, visibility, linked_room_id, inherit_room_visibility)) = document
    else {
        return Ok(None);
    };

    // Actor retirement is another access revocation. Hold the same Actor row
    // that retirement updates until the caller has finished its projection.
    let actor_class: Option<String> = sqlx::query_scalar(
        "SELECT actor_class FROM actors WHERE id=$1 AND retired_at IS NULL FOR SHARE",
    )
    .bind(actor_id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(actor_class) = actor_class else {
        return Ok(None);
    };

    if owner_actor_id == actor_id {
        return Ok(Some(DocumentAccess::Edit));
    }

    // Document-participant mutations already take the document row FOR UPDATE
    // before touching this row. The explicit row lock also documents the exact
    // access fact retained by this transaction.
    let explicit_access: Option<DocumentAccess> = sqlx::query_scalar(
        "SELECT access FROM native_document_participants \
         WHERE document_id=$1 AND actor_id=$2 AND removed_at IS NULL FOR SHARE",
    )
    .bind(document_id)
    .bind(actor_id)
    .fetch_optional(&mut **tx)
    .await?;
    if explicit_access.is_some() {
        return Ok(explicit_access);
    }

    if visibility == DocumentVisibility::Company && actor_class == "human" {
        return Ok(Some(DocumentAccess::Read));
    }

    if visibility == DocumentVisibility::Participants && inherit_room_visibility {
        let Some(room_id) = linked_room_id else {
            return Ok(None);
        };
        // Room-derived authority must remain stable too. Room participant
        // removal uses Actor -> Room, and we already hold the Actor row; taking
        // both Room access rows here therefore makes the content read atomic
        // with authorization without introducing the inverse lock order.
        let inherited: Option<bool> = sqlx::query_scalar(
            "SELECT TRUE FROM rooms room \
             JOIN room_participants participant ON participant.room_id=room.id \
             WHERE room.id=$1 AND participant.actor_id=$2 \
               AND room.archived_at IS NULL AND participant.left_at IS NULL \
             FOR SHARE OF room,participant",
        )
        .bind(room_id)
        .bind(actor_id)
        .fetch_optional(&mut **tx)
        .await?;
        if inherited.is_some() {
            return Ok(Some(DocumentAccess::Read));
        }
    }

    Ok(None)
}

async fn require_access(
    tx: &mut Transaction<'_, Postgres>,
    document_id: Uuid,
    actor_id: &str,
    required: DocumentAccess,
) -> DocumentResult<DocumentAccess> {
    let access = access_in_transaction(tx, document_id, actor_id)
        .await?
        .ok_or(DocumentError::Unavailable)?;
    if !access.permits(required) {
        return Err(DocumentError::Unavailable);
    }
    Ok(access)
}

fn clean_bounded(label: &str, value: &str, maximum: usize) -> DocumentResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > maximum {
        return Err(DocumentError::Invalid(format!(
            "{label} must contain 1 to {maximum} characters"
        )));
    }
    reject_nul(label, value)?;
    Ok(value.to_string())
}

fn validate_visibility_link(
    visibility: DocumentVisibility,
    linked_room_id: Option<Uuid>,
    inherit_room_visibility: bool,
) -> DocumentResult<()> {
    if inherit_room_visibility
        && (visibility != DocumentVisibility::Participants || linked_room_id.is_none())
    {
        return Err(DocumentError::Invalid(
            "Room visibility inheritance needs a linked, participant-visible document".into(),
        ));
    }
    Ok(())
}

fn version_view(version: DocumentVersionRow) -> DocumentResult<DocumentVersionView> {
    if version.schema_version != DOCUMENT_SCHEMA_VERSION {
        return Err(DocumentError::Corrupt(format!(
            "unsupported structured-content schema {}",
            version.schema_version
        )));
    }
    let validated = validate_document_json(&version.content_json)
        .map_err(|error| DocumentError::Corrupt(error.to_string()))?;
    if validated.content_hash != version.content_hash {
        return Err(DocumentError::Corrupt(format!(
            "version {} content hash does not match",
            version.id
        )));
    }
    if validated.plain_text != version.plain_text {
        return Err(DocumentError::Corrupt(format!(
            "version {} plain-text projection does not match",
            version.id
        )));
    }
    Ok(DocumentVersionView {
        version,
        rendered_html: validated.rendered_html,
        markdown: validated.markdown,
    })
}

impl OrgIntel {
    pub async fn create_document(
        &self,
        input: NewDocument<'_>,
    ) -> DocumentResult<DocumentReadView> {
        let title = clean_bounded("title", input.title, 200)?;
        let reason = clean_bounded("version reason", input.reason, 500)?;
        validate_visibility_link(
            input.visibility,
            input.linked_room_id,
            input.inherit_room_visibility,
        )?;
        let content = validate_document_json(input.content_json)?;
        let document_id = Uuid::new_v4();
        let version_id = Uuid::new_v4();
        let mut tx = self.pool.begin().await?;
        for actor in [input.owner_actor_id, input.created_by_actor_id] {
            if !active_actor(&mut tx, actor).await? {
                return Err(DocumentError::Invalid(format!(
                    "Actor {actor:?} is not active"
                )));
            }
        }
        if let Some(room_id) = input.linked_room_id {
            for actor in [input.owner_actor_id, input.created_by_actor_id] {
                if !active_room_participant(&mut tx, room_id, actor).await? {
                    return Err(DocumentError::Unavailable);
                }
            }
        }

        sqlx::query(
            "INSERT INTO native_documents \
             (id,title,kind,status,visibility,linked_room_id,inherit_room_visibility,\
              owner_actor_id,current_named_version_id,created_by_actor_id) \
             VALUES ($1,$2,$3,'draft',$4,$5,$6,$7,$8,$9)",
        )
        .bind(document_id)
        .bind(&title)
        .bind(input.kind)
        .bind(input.visibility)
        .bind(input.linked_room_id)
        .bind(input.inherit_room_visibility)
        .bind(input.owner_actor_id)
        .bind(version_id)
        .bind(input.created_by_actor_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO native_document_versions \
             (id,document_id,version_number,schema_version,content_json,plain_text,content_hash,\
              document_status,created_by_actor_id,reason) \
             VALUES ($1,$2,1,$3,$4,$5,$6,'draft',$7,$8)",
        )
        .bind(version_id)
        .bind(document_id)
        .bind(DOCUMENT_SCHEMA_VERSION)
        .bind(&content.content_json)
        .bind(&content.plain_text)
        .bind(&content.content_hash)
        .bind(input.created_by_actor_id)
        .bind(&reason)
        .execute(&mut *tx)
        .await?;
        for actor in [input.owner_actor_id, input.created_by_actor_id] {
            sqlx::query(
                "INSERT INTO native_document_participants \
                 (document_id,actor_id,access,added_by_actor_id) VALUES ($1,$2,'edit',$3) \
                 ON CONFLICT(document_id,actor_id) DO UPDATE SET \
                   access='edit',added_by_actor_id=EXCLUDED.added_by_actor_id,added_at=now(),removed_at=NULL",
            )
            .bind(document_id)
            .bind(actor)
            .bind(input.created_by_actor_id)
            .execute(&mut *tx)
            .await?;
        }
        append_document_event(
            &mut tx,
            "document.created.v1",
            document_id,
            input.created_by_actor_id,
            json!({
                "document_id": document_id,
                "document_version": 1,
                "named_version_id": version_id,
                "named_version_number": 1,
                "status": DocumentStatus::Draft,
                "visibility": input.visibility,
                "linked_room_id": input.linked_room_id,
                "inherit_room_visibility": input.inherit_room_visibility,
                "owner_actor_id": input.owner_actor_id,
            }),
        )
        .await?;
        tx.commit().await?;
        self.get_document_for_actor(document_id, input.created_by_actor_id)
            .await
    }

    pub async fn list_documents_for_actor(
        &self,
        actor_id: &str,
        include_archived: bool,
    ) -> DocumentResult<Vec<DocumentSummary>> {
        #[derive(sqlx::FromRow)]
        struct Listed {
            id: Uuid,
            title: String,
            kind: DocumentKind,
            status: DocumentStatus,
            visibility: DocumentVisibility,
            linked_room_id: Option<Uuid>,
            inherit_room_visibility: bool,
            owner_actor_id: String,
            current_named_version_id: Uuid,
            created_by_actor_id: String,
            created_at: DateTime<Utc>,
            updated_at: DateTime<Utc>,
            version: i64,
            access: DocumentAccess,
        }

        let rows = sqlx::query_as::<_, Listed>(
            "SELECT d.id,d.title,d.kind,d.status,d.visibility,d.linked_room_id,\
                    d.inherit_room_visibility,d.owner_actor_id,\
                    d.current_named_version_id,d.created_by_actor_id,d.created_at,d.updated_at,d.version,\
                    CASE \
                      WHEN d.owner_actor_id=$1 THEN 'edit'::native_document_access \
                      WHEN p.access='edit' THEN 'edit'::native_document_access \
                      WHEN p.access='comment' THEN 'comment'::native_document_access \
                      WHEN p.access='read' THEN 'read'::native_document_access \
                      ELSE 'read'::native_document_access END AS access \
             FROM native_documents d \
             JOIN actors a ON a.id=$1 AND a.retired_at IS NULL \
             LEFT JOIN native_document_participants p \
               ON p.document_id=d.id AND p.actor_id=$1 AND p.removed_at IS NULL \
             WHERE ($2 OR d.status<>'archived') \
               AND (d.owner_actor_id=$1 OR p.actor_id IS NOT NULL \
                    OR (d.visibility='company' AND a.actor_class='human') \
                    OR (d.visibility='participants' AND d.inherit_room_visibility \
                        AND EXISTS(\
                          SELECT 1 FROM rooms room \
                          JOIN room_participants rp ON rp.room_id=room.id \
                          WHERE room.id=d.linked_room_id AND room.archived_at IS NULL \
                            AND rp.actor_id=$1 AND rp.left_at IS NULL\
                        ))) \
             ORDER BY d.updated_at DESC,d.id",
        )
        .bind(actor_id)
        .bind(include_archived)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| DocumentSummary {
                document: DocumentRow {
                    id: row.id,
                    title: row.title,
                    kind: row.kind,
                    status: row.status,
                    visibility: row.visibility,
                    linked_room_id: row.linked_room_id,
                    inherit_room_visibility: row.inherit_room_visibility,
                    owner_actor_id: row.owner_actor_id,
                    current_named_version_id: row.current_named_version_id,
                    created_by_actor_id: row.created_by_actor_id,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    version: row.version,
                },
                access: row.access,
            })
            .collect())
    }

    pub async fn document_access_for_actor(
        &self,
        document_id: Uuid,
        actor_id: &str,
    ) -> DocumentResult<Option<DocumentAccess>> {
        let mut tx = self.pool.begin().await?;
        let access = access_in_transaction(&mut tx, document_id, actor_id).await?;
        tx.commit().await?;
        Ok(access)
    }

    pub async fn get_document_for_actor(
        &self,
        document_id: Uuid,
        actor_id: &str,
    ) -> DocumentResult<DocumentReadView> {
        let mut tx = self.pool.begin().await?;
        let access = require_access(&mut tx, document_id, actor_id, DocumentAccess::Read).await?;
        let document =
            sqlx::query_as::<_, DocumentRow>(&format!("{} WHERE id=$1", document_select()))
                .bind(document_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(DocumentError::Unavailable)?;
        let current = sqlx::query_as::<_, DocumentVersionRow>(&format!(
            "{} WHERE document_id=$1 AND id=$2",
            version_select()
        ))
        .bind(document_id)
        .bind(document.current_named_version_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            DocumentError::Corrupt(format!(
                "document {document_id} has no current named version"
            ))
        })?;
        tx.commit().await?;
        Ok(DocumentReadView {
            document,
            access,
            current_version: version_view(current)?,
        })
    }

    pub async fn update_document_metadata(
        &self,
        input: UpdateDocumentMetadata<'_>,
    ) -> DocumentResult<DocumentReadView> {
        let title = clean_bounded("title", input.title, 200)?;
        validate_visibility_link(
            input.visibility,
            input.linked_room_id,
            input.inherit_room_visibility,
        )?;
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT owner_actor_id,version FROM native_documents WHERE id=$1 FOR UPDATE",
        )
        .bind(input.document_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(DocumentError::Unavailable)?;
        require_access(
            &mut tx,
            input.document_id,
            input.actor_id,
            DocumentAccess::Edit,
        )
        .await?;
        if row.get::<String, _>("owner_actor_id") != input.actor_id {
            return Err(DocumentError::Unavailable);
        }
        if let Some(room_id) = input.linked_room_id {
            if !active_room_participant(&mut tx, room_id, input.actor_id).await? {
                return Err(DocumentError::Unavailable);
            }
        }
        let actual = row.get::<i64, _>("version");
        if actual != input.expected_version {
            return Err(DocumentError::Conflict(format!(
                "metadata is version {actual}, expected {}",
                input.expected_version
            )));
        }
        let document_version: i64 = sqlx::query_scalar(
            "UPDATE native_documents SET title=$2,kind=$3,status=$4,visibility=$5,\
             linked_room_id=$6,inherit_room_visibility=$7,version=version+1 \
             WHERE id=$1 RETURNING version",
        )
        .bind(input.document_id)
        .bind(title)
        .bind(input.kind)
        .bind(input.status)
        .bind(input.visibility)
        .bind(input.linked_room_id)
        .bind(input.inherit_room_visibility)
        .fetch_one(&mut *tx)
        .await?;
        append_document_event(
            &mut tx,
            "document.metadata.updated.v1",
            input.document_id,
            input.actor_id,
            json!({
                "document_id": input.document_id,
                "document_version": document_version,
                "status": input.status,
                "visibility": input.visibility,
                "linked_room_id": input.linked_room_id,
                "inherit_room_visibility": input.inherit_room_visibility,
            }),
        )
        .await?;
        tx.commit().await?;
        self.get_document_for_actor(input.document_id, input.actor_id)
            .await
    }

    pub async fn create_named_document_version(
        &self,
        input: NewNamedDocumentVersion<'_>,
    ) -> DocumentResult<DocumentReadView> {
        let reason = clean_bounded("version reason", input.reason, 500)?;
        let content = validate_document_json(input.content_json)?;
        let mut tx = self.pool.begin().await?;
        let document = sqlx::query(
            "SELECT current_named_version_id,status FROM native_documents WHERE id=$1 FOR UPDATE",
        )
        .bind(input.document_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(DocumentError::Unavailable)?;
        require_access(
            &mut tx,
            input.document_id,
            input.actor_id,
            DocumentAccess::Edit,
        )
        .await?;
        let current = document.get::<Uuid, _>("current_named_version_id");
        let status = document.get::<DocumentStatus, _>("status");
        if current != input.expected_current_version_id {
            return Err(DocumentError::Conflict(format!(
                "current named version is {current}, expected {}",
                input.expected_current_version_id
            )));
        }
        let next_number: i64 = sqlx::query_scalar(
            "SELECT COALESCE(max(version_number),0)+1 FROM native_document_versions WHERE document_id=$1",
        )
        .bind(input.document_id)
        .fetch_one(&mut *tx)
        .await?;
        let version_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO native_document_versions \
             (id,document_id,version_number,schema_version,content_json,plain_text,content_hash,\
              document_status,created_by_actor_id,reason) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
        )
        .bind(version_id)
        .bind(input.document_id)
        .bind(next_number)
        .bind(DOCUMENT_SCHEMA_VERSION)
        .bind(&content.content_json)
        .bind(&content.plain_text)
        .bind(&content.content_hash)
        .bind(status)
        .bind(input.actor_id)
        .bind(reason)
        .execute(&mut *tx)
        .await?;
        let document_version: i64 = sqlx::query_scalar(
            "UPDATE native_documents SET current_named_version_id=$2,version=version+1 \
             WHERE id=$1 RETURNING version",
        )
        .bind(input.document_id)
        .bind(version_id)
        .fetch_one(&mut *tx)
        .await?;
        append_document_event(
            &mut tx,
            "document.named_version.created.v1",
            input.document_id,
            input.actor_id,
            json!({
                "document_id": input.document_id,
                "document_version": document_version,
                "named_version_id": version_id,
                "named_version_number": next_number,
                "previous_named_version_id": current,
                "status": status,
            }),
        )
        .await?;
        tx.commit().await?;
        self.get_document_for_actor(input.document_id, input.actor_id)
            .await
    }

    /// Restore one historical checkpoint by appending an attributed version.
    /// The source and every later checkpoint remain immutable and readable.
    pub async fn restore_document_version(
        &self,
        input: RestoreDocumentVersion<'_>,
    ) -> DocumentResult<DocumentReadView> {
        let reason = clean_bounded("restore reason", input.reason, 500)?;
        let mut tx = self.pool.begin().await?;
        let document = sqlx::query(
            "SELECT current_named_version_id,status FROM native_documents WHERE id=$1 FOR UPDATE",
        )
        .bind(input.document_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(DocumentError::Unavailable)?;
        require_access(
            &mut tx,
            input.document_id,
            input.actor_id,
            DocumentAccess::Edit,
        )
        .await?;
        let current = document.get::<Uuid, _>("current_named_version_id");
        if current != input.expected_current_version_id {
            return Err(DocumentError::Conflict(format!(
                "current named version is {current}, expected {}",
                input.expected_current_version_id
            )));
        }
        if input.source_version_id == current {
            return Err(DocumentError::Invalid(
                "restore source is already the current named version".into(),
            ));
        }
        let source = sqlx::query_as::<_, DocumentVersionRow>(&format!(
            "{} WHERE document_id=$1 AND id=$2",
            version_select()
        ))
        .bind(input.document_id)
        .bind(input.source_version_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(DocumentError::Unavailable)?;
        version_view(source.clone())?;

        let next_number: i64 = sqlx::query_scalar(
            "SELECT COALESCE(max(version_number),0)+1 FROM native_document_versions WHERE document_id=$1",
        )
        .bind(input.document_id)
        .fetch_one(&mut *tx)
        .await?;
        let version_id = Uuid::new_v4();
        let status = document.get::<DocumentStatus, _>("status");
        sqlx::query(
            "INSERT INTO native_document_versions \
             (id,document_id,version_number,schema_version,content_json,plain_text,content_hash,\
              document_status,restored_from_version_id,created_by_actor_id,reason) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
        )
        .bind(version_id)
        .bind(input.document_id)
        .bind(next_number)
        .bind(source.schema_version)
        .bind(&source.content_json)
        .bind(&source.plain_text)
        .bind(&source.content_hash)
        .bind(status)
        .bind(source.id)
        .bind(input.actor_id)
        .bind(reason)
        .execute(&mut *tx)
        .await?;
        let document_version: i64 = sqlx::query_scalar(
            "UPDATE native_documents SET current_named_version_id=$2,version=version+1 \
             WHERE id=$1 RETURNING version",
        )
        .bind(input.document_id)
        .bind(version_id)
        .fetch_one(&mut *tx)
        .await?;
        append_document_event(
            &mut tx,
            "document.version.restored.v1",
            input.document_id,
            input.actor_id,
            json!({
                "document_id": input.document_id,
                "document_version": document_version,
                "named_version_id": version_id,
                "named_version_number": next_number,
                "previous_named_version_id": current,
                "restored_from_version_id": source.id,
                "status": status,
            }),
        )
        .await?;
        tx.commit().await?;
        self.get_document_for_actor(input.document_id, input.actor_id)
            .await
    }

    pub async fn list_document_versions_for_actor(
        &self,
        document_id: Uuid,
        actor_id: &str,
    ) -> DocumentResult<Vec<DocumentVersionView>> {
        let mut tx = self.pool.begin().await?;
        require_access(&mut tx, document_id, actor_id, DocumentAccess::Read).await?;
        let versions = sqlx::query_as::<_, DocumentVersionRow>(&format!(
            "{} WHERE document_id=$1 ORDER BY version_number DESC",
            version_select()
        ))
        .bind(document_id)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        versions.into_iter().map(version_view).collect()
    }

    pub async fn get_document_version_for_actor(
        &self,
        document_id: Uuid,
        version_id: Uuid,
        actor_id: &str,
    ) -> DocumentResult<DocumentVersionView> {
        let mut tx = self.pool.begin().await?;
        require_access(&mut tx, document_id, actor_id, DocumentAccess::Read).await?;
        let version = sqlx::query_as::<_, DocumentVersionRow>(&format!(
            "{} WHERE document_id=$1 AND id=$2",
            version_select()
        ))
        .bind(document_id)
        .bind(version_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(DocumentError::Unavailable)?;
        tx.commit().await?;
        version_view(version)
    }

    /// Inspect the durable receipt for a Runtime-imported named version. Read
    /// authority is derived from current server state; compacted events are
    /// neither required nor treated as authorization.
    pub async fn get_document_runtime_import_provenance_for_actor(
        &self,
        document_id: Uuid,
        imported_version_id: Uuid,
        actor_id: &str,
    ) -> DocumentResult<DocumentRuntimeImportProvenance> {
        let mut tx = self.pool.begin().await?;
        require_access(&mut tx, document_id, actor_id, DocumentAccess::Read).await?;
        let provenance = sqlx::query_as::<_, DocumentRuntimeImportProvenance>(&format!(
            "{} WHERE document_id=$1 AND imported_version_id=$2",
            runtime_import_provenance_select()
        ))
        .bind(document_id)
        .bind(imported_version_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(DocumentError::Unavailable)?;
        tx.commit().await?;
        Ok(provenance)
    }

    /// Materialize an exact named version as a deterministic portable Runtime
    /// checkpoint. The caller chooses what to do with the returned JSON and
    /// Markdown; Core never accepts a path and never writes the filesystem.
    pub async fn export_document_version_for_runtime(
        &self,
        document_id: Uuid,
        version_id: Uuid,
        actor_id: &str,
    ) -> DocumentResult<NativeDocumentPortableEnvelope> {
        let mut tx = self.pool.begin().await?;
        let document_version: i64 =
            sqlx::query_scalar("SELECT version FROM native_documents WHERE id=$1 FOR SHARE")
                .bind(document_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or(DocumentError::Unavailable)?;
        require_access(&mut tx, document_id, actor_id, DocumentAccess::Read).await?;
        let version = sqlx::query_as::<_, DocumentVersionRow>(&format!(
            "{} WHERE document_id=$1 AND id=$2",
            version_select()
        ))
        .bind(document_id)
        .bind(version_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(DocumentError::Unavailable)?;
        let source_company_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT company_id FROM company_access_identity WHERE singleton=TRUE",
        )
        .fetch_optional(&mut *tx)
        .await?;
        let view = version_view(version.clone())?;
        let mut envelope = NativeDocumentPortableEnvelope {
            envelope_schema: NATIVE_DOCUMENT_PORTABLE_ENVELOPE_SCHEMA.into(),
            envelope_version: NATIVE_DOCUMENT_PORTABLE_ENVELOPE_VERSION,
            source_company_id,
            source_company_key: self.schema.clone(),
            source_document_id: document_id,
            source_named_version_id: version.id,
            source_named_version_number: version.version_number,
            source_content_schema_version: version.schema_version,
            source_document_status: version.document_status,
            source_restored_from_version_id: version.restored_from_version_id,
            source_created_by_actor_id: version.created_by_actor_id,
            source_version_reason: version.reason,
            source_version_created_at: version.created_at,
            source_content_hash: version.content_hash,
            content_json: version.content_json,
            plain_text: view.version.plain_text,
            markdown: String::new(),
            content_hash: view.version.content_hash,
            checksum: String::new(),
        };
        envelope.reseal()?;
        append_document_event(
            &mut tx,
            "document.exported.v1",
            document_id,
            actor_id,
            json!({
                "document_id": document_id,
                "document_version": document_version,
                "named_version_id": version_id,
                "named_version_number": envelope.source_named_version_number,
                "status": envelope.source_document_status,
                "content_hash": envelope.content_hash,
                "envelope_checksum": envelope.checksum,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(envelope)
    }

    /// Explicitly apply a validated Runtime checkpoint as one new immutable
    /// named version. Exact retries resolve to the deterministic imported
    /// version; different writes against a stale current version conflict.
    pub async fn import_runtime_document_as_named_version(
        &self,
        input: ImportRuntimeDocument<'_>,
    ) -> DocumentResult<DocumentImportResult> {
        let content = validate_portable_envelope(input.envelope)?;
        let actor_id = clean_bounded("importing Actor", input.actor_id, 200)?;
        let reason = clean_bounded("import reason", input.reason, 500)?;
        let imported_version_id = deterministic_import_version_id(
            input.target_document_id,
            &actor_id,
            input.expected_current_version_id,
            &reason,
            &input.envelope.checksum,
        );

        let mut tx = self.pool.begin().await?;
        let document = sqlx::query(
            "SELECT current_named_version_id,status FROM native_documents WHERE id=$1 FOR UPDATE",
        )
        .bind(input.target_document_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(DocumentError::Unavailable)?;
        require_access(
            &mut tx,
            input.target_document_id,
            &actor_id,
            DocumentAccess::Edit,
        )
        .await?;
        require_runtime_source_claims(
            &mut tx,
            &self.schema,
            input.target_document_id,
            input.expected_current_version_id,
            input.envelope,
        )
        .await?;

        let replay = sqlx::query_as::<_, DocumentVersionRow>(&format!(
            "{} WHERE document_id=$1 AND id=$2",
            version_select()
        ))
        .bind(input.target_document_id)
        .bind(imported_version_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(version) = replay {
            let provenance = sqlx::query_as::<_, DocumentRuntimeImportProvenance>(&format!(
                "{} WHERE document_id=$1 AND imported_version_id=$2",
                runtime_import_provenance_select()
            ))
            .bind(input.target_document_id)
            .bind(imported_version_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                DocumentError::Corrupt(
                    "deterministic Runtime import has no durable provenance receipt".into(),
                )
            })?;
            if version.content_json != content.content_json
                || version.content_hash != content.content_hash
                || version.plain_text != content.plain_text
                || version.created_by_actor_id != actor_id
                || version.reason != reason
                || !runtime_import_provenance_matches(
                    &provenance,
                    input.target_document_id,
                    imported_version_id,
                    input.expected_current_version_id,
                    input.envelope,
                    &content.content_hash,
                    &actor_id,
                )
            {
                return Err(DocumentError::Corrupt(
                    "deterministic Runtime import identity has conflicting provenance".into(),
                ));
            }
            tx.commit().await?;
            return Ok(DocumentImportResult {
                target_document_id: input.target_document_id,
                imported_version: version_view(version)?,
                envelope_checksum: input.envelope.checksum.clone(),
            });
        }

        let current = document.get::<Uuid, _>("current_named_version_id");
        if current != input.expected_current_version_id {
            return Err(DocumentError::Conflict(format!(
                "current named version is {current}, expected {}",
                input.expected_current_version_id
            )));
        }
        let status = document.get::<DocumentStatus, _>("status");
        let next_number: i64 = sqlx::query_scalar(
            "SELECT COALESCE(max(version_number),0)+1 FROM native_document_versions WHERE document_id=$1",
        )
        .bind(input.target_document_id)
        .fetch_one(&mut *tx)
        .await?;
        let imported = sqlx::query_as::<_, DocumentVersionRow>(
            "INSERT INTO native_document_versions \
             (id,document_id,version_number,schema_version,content_json,plain_text,content_hash,\
              document_status,created_by_actor_id,reason) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) \
             RETURNING id,document_id,version_number,schema_version,content_json,plain_text,\
                       content_hash,document_status,restored_from_version_id,\
                       created_by_actor_id,reason,created_at",
        )
        .bind(imported_version_id)
        .bind(input.target_document_id)
        .bind(next_number)
        .bind(DOCUMENT_SCHEMA_VERSION)
        .bind(&content.content_json)
        .bind(&content.plain_text)
        .bind(&content.content_hash)
        .bind(status)
        .bind(&actor_id)
        .bind(&reason)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO native_document_runtime_imports \
             (document_id,imported_version_id,target_base_version_id,envelope_schema,\
              envelope_version,envelope_checksum,source_company_id,source_company_key,\
              source_document_id,source_named_version_id,source_named_version_number,\
              source_content_schema_version,source_document_status,\
              source_restored_from_version_id,source_created_by_actor_id,source_version_reason,\
              source_version_created_at,source_content_hash,imported_content_hash,\
              imported_by_actor_id) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)",
        )
        .bind(input.target_document_id)
        .bind(imported_version_id)
        .bind(input.expected_current_version_id)
        .bind(&input.envelope.envelope_schema)
        .bind(input.envelope.envelope_version as i16)
        .bind(&input.envelope.checksum)
        .bind(input.envelope.source_company_id)
        .bind(&input.envelope.source_company_key)
        .bind(input.envelope.source_document_id)
        .bind(input.envelope.source_named_version_id)
        .bind(input.envelope.source_named_version_number)
        .bind(input.envelope.source_content_schema_version)
        .bind(input.envelope.source_document_status)
        .bind(input.envelope.source_restored_from_version_id)
        .bind(&input.envelope.source_created_by_actor_id)
        .bind(&input.envelope.source_version_reason)
        .bind(input.envelope.source_version_created_at)
        .bind(&input.envelope.source_content_hash)
        .bind(&content.content_hash)
        .bind(&actor_id)
        .execute(&mut *tx)
        .await?;
        let document_version: i64 = sqlx::query_scalar(
            "UPDATE native_documents SET current_named_version_id=$2,version=version+1 \
             WHERE id=$1 RETURNING version",
        )
        .bind(input.target_document_id)
        .bind(imported_version_id)
        .fetch_one(&mut *tx)
        .await?;
        append_document_event(
            &mut tx,
            "document.imported.v1",
            input.target_document_id,
            &actor_id,
            json!({
                "document_id": input.target_document_id,
                "document_version": document_version,
                "named_version_id": imported_version_id,
                "named_version_number": next_number,
                "previous_named_version_id": current,
                "status": status,
                "source_company_id": input.envelope.source_company_id,
                "source_company_key": input.envelope.source_company_key,
                "source_document_id": input.envelope.source_document_id,
                "source_named_version_id": input.envelope.source_named_version_id,
                "source_content_hash": input.envelope.source_content_hash,
                "content_hash": content.content_hash,
                "envelope_checksum": input.envelope.checksum,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(DocumentImportResult {
            target_document_id: input.target_document_id,
            imported_version: version_view(imported)?,
            envelope_checksum: input.envelope.checksum.clone(),
        })
    }

    pub async fn list_document_participants_for_actor(
        &self,
        document_id: Uuid,
        actor_id: &str,
    ) -> DocumentResult<Vec<DocumentParticipantRow>> {
        let mut tx = self.pool.begin().await?;
        require_access(&mut tx, document_id, actor_id, DocumentAccess::Read).await?;
        let participants = sqlx::query_as::<_, DocumentParticipantRow>(
            "SELECT document_id,actor_id,access,added_by_actor_id,added_at \
             FROM native_document_participants \
             WHERE document_id=$1 AND removed_at IS NULL ORDER BY added_at,actor_id",
        )
        .bind(document_id)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(participants)
    }

    pub async fn set_document_participant(
        &self,
        input: SetDocumentParticipant<'_>,
    ) -> DocumentResult<DocumentParticipantRow> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT owner_actor_id,version FROM native_documents WHERE id=$1 FOR UPDATE",
        )
        .bind(input.document_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(DocumentError::Unavailable)?;
        require_access(
            &mut tx,
            input.document_id,
            input.actor_id,
            DocumentAccess::Edit,
        )
        .await?;
        let owner = row.get::<String, _>("owner_actor_id");
        if owner != input.actor_id {
            return Err(DocumentError::Unavailable);
        }
        let actual = row.get::<i64, _>("version");
        if actual != input.expected_document_version {
            return Err(DocumentError::Conflict(format!(
                "metadata is version {actual}, expected {}",
                input.expected_document_version
            )));
        }
        if !active_actor(&mut tx, input.participant_actor_id).await? {
            return Err(DocumentError::Invalid(format!(
                "Actor {:?} is not active",
                input.participant_actor_id
            )));
        }
        if input.participant_actor_id == owner && input.access != DocumentAccess::Edit {
            return Err(DocumentError::Invalid(
                "the document owner must retain edit access".into(),
            ));
        }
        let participant_was_active: bool = sqlx::query_scalar(
            "SELECT EXISTS(\
               SELECT 1 FROM native_document_participants \
               WHERE document_id=$1 AND actor_id=$2 AND removed_at IS NULL\
             )",
        )
        .bind(input.document_id)
        .bind(input.participant_actor_id)
        .fetch_one(&mut *tx)
        .await?;
        let participant = sqlx::query_as::<_, DocumentParticipantRow>(
            "INSERT INTO native_document_participants \
             (document_id,actor_id,access,added_by_actor_id) VALUES ($1,$2,$3,$4) \
             ON CONFLICT(document_id,actor_id) DO UPDATE SET \
               access=EXCLUDED.access,added_by_actor_id=EXCLUDED.added_by_actor_id,\
               added_at=now(),removed_at=NULL \
             RETURNING document_id,actor_id,access,added_by_actor_id,added_at",
        )
        .bind(input.document_id)
        .bind(input.participant_actor_id)
        .bind(input.access)
        .bind(input.actor_id)
        .fetch_one(&mut *tx)
        .await?;
        let document_version: i64 = sqlx::query_scalar(
            "UPDATE native_documents SET version=version+1 WHERE id=$1 RETURNING version",
        )
        .bind(input.document_id)
        .fetch_one(&mut *tx)
        .await?;
        append_document_event(
            &mut tx,
            if participant_was_active {
                "document.participant.updated.v1"
            } else {
                "document.participant.added.v1"
            },
            input.document_id,
            input.actor_id,
            json!({
                "document_id": input.document_id,
                "document_version": document_version,
                "participant_actor_id": input.participant_actor_id,
                "participant_status": "active",
                "access": input.access,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(participant)
    }

    pub async fn remove_document_participant(
        &self,
        document_id: Uuid,
        actor_id: &str,
        participant_actor_id: &str,
        expected_document_version: i64,
    ) -> DocumentResult<()> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query(
            "SELECT owner_actor_id,version FROM native_documents WHERE id=$1 FOR UPDATE",
        )
        .bind(document_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(DocumentError::Unavailable)?;
        require_access(&mut tx, document_id, actor_id, DocumentAccess::Edit).await?;
        let owner = row.get::<String, _>("owner_actor_id");
        if owner != actor_id || participant_actor_id == owner {
            return Err(DocumentError::Unavailable);
        }
        let actual = row.get::<i64, _>("version");
        if actual != expected_document_version {
            return Err(DocumentError::Conflict(format!(
                "metadata is version {actual}, expected {expected_document_version}"
            )));
        }
        let changed = sqlx::query(
            "UPDATE native_document_participants SET removed_at=now() \
             WHERE document_id=$1 AND actor_id=$2 AND removed_at IS NULL",
        )
        .bind(document_id)
        .bind(participant_actor_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if changed != 1 {
            return Err(DocumentError::Unavailable);
        }
        let document_version: i64 = sqlx::query_scalar(
            "UPDATE native_documents SET version=version+1 WHERE id=$1 RETURNING version",
        )
        .bind(document_id)
        .fetch_one(&mut *tx)
        .await?;
        append_document_event(
            &mut tx,
            "document.participant.removed.v1",
            document_id,
            actor_id,
            json!({
                "document_id": document_id,
                "document_version": document_version,
                "participant_actor_id": participant_actor_id,
                "participant_status": "removed",
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn restricted_document_renders_without_raw_html() {
        let document = json!({
            "type": "doc",
            "content": [
                {
                    "type": "heading",
                    "attrs": {"level": 2, "block_id": "opening"},
                    "content": [{"type": "text", "text": "Evidence < first"}]
                },
                {
                    "type": "paragraph",
                    "attrs": {"block_id": "source"},
                    "content": [
                        {"type": "text", "text": "Read ", "marks": [{"type": "bold"}]},
                        {"type": "text", "text": "the source", "marks": [{
                            "type": "link", "attrs": {"href": "https://restless.run"}
                        }]},
                        {"type": "hardBreak"},
                        {"type": "mention", "attrs": {"actor_id": "exec", "label": "Exec"}}
                    ]
                }
            ]
        });
        let validated = validate_document_json(&document).unwrap();
        assert!(validated
            .rendered_html
            .contains("data-block-id=\"opening\""));
        assert!(validated.rendered_html.contains("Evidence &lt; first"));
        assert!(validated
            .rendered_html
            .contains("rel=\"noopener noreferrer\""));
        assert!(validated.plain_text.contains("Evidence < first"));
        assert!(validated.plain_text.contains("@Exec"));
        assert!(validated.markdown.contains("## Evidence \\< first"));
        assert!(validated
            .markdown
            .contains("[the source](https://restless.run)"));
        assert_eq!(validated.content_hash.len(), 64);
    }

    #[test]
    fn raw_html_unknown_fields_and_unsafe_links_are_refused() {
        for document in [
            json!({"type":"doc","content":[{"type":"rawHtml","html":"<script>x()</script>"}]}),
            json!({"type":"doc","content":[{"type":"paragraph","style":"position:fixed"}]}),
            json!({"type":"doc","content":[{"type":"paragraph","attrs":{"block_id":"link"},"content":[{
                "type":"text","text":"click","marks":[{"type":"link","attrs":{"href":"javascript:alert(1)"}}]
            }]}]}),
            json!({"type":"doc","content":[{"type":"paragraph","attrs":{"block_id":"link"},"content":[{
                "type":"text","text":"click","marks":[{"type":"link","attrs":{"href":"//evil.example"}}]
            }]}]}),
        ] {
            assert!(validate_document_json(&document).is_err());
        }
    }

    #[test]
    fn hashes_are_stable_across_object_key_order() {
        let first = serde_json::from_str::<Value>(
            r#"{"type":"doc","content":[{"type":"paragraph","attrs":{"block_id":"same-block"},"content":[{"type":"text","text":"same"}]}]}"#,
        )
        .unwrap();
        let second = serde_json::from_str::<Value>(
            r#"{"content":[{"content":[{"text":"same","type":"text"}],"attrs":{"block_id":"same-block"},"type":"paragraph"}],"type":"doc"}"#,
        )
        .unwrap();
        assert_eq!(
            validate_document_json(&first).unwrap().content_hash,
            validate_document_json(&second).unwrap().content_hash
        );
    }

    #[test]
    fn repeated_block_identity_is_refused() {
        let document = json!({
            "type":"doc",
            "content":[
                {"type":"paragraph","attrs":{"block_id":"same"}},
                {"type":"heading","attrs":{"level":2,"block_id":"same"}}
            ]
        });
        assert!(validate_document_json(&document)
            .unwrap_err()
            .to_string()
            .contains("is repeated"));
    }

    #[test]
    fn every_top_level_block_needs_stable_identity() {
        let missing = json!({
            "type":"doc",
            "content":[{"type":"paragraph","content":[{"type":"text","text":"orphan"}]}]
        });
        assert!(validate_document_json(&missing)
            .unwrap_err()
            .to_string()
            .contains("stable block_id"));

        let before = json!({
            "type":"doc",
            "content":[
                {"type":"paragraph","attrs":{"block_id":"keep-me"},"content":[{"type":"text","text":"Before"}]},
                {"type":"horizontalRule","attrs":{"block_id":"also-keep"}}
            ]
        });
        let after = json!({
            "type":"doc",
            "content":[
                {"type":"paragraph","attrs":{"block_id":"keep-me"},"content":[{"type":"text","text":"After"}]},
                {"type":"horizontalRule","attrs":{"block_id":"also-keep"}}
            ]
        });
        let before = validate_document_json(&before).unwrap();
        let after = validate_document_json(&after).unwrap();
        assert_eq!(
            before.content_json["content"][0]["attrs"]["block_id"],
            after.content_json["content"][0]["attrs"]["block_id"]
        );
        assert_ne!(before.content_hash, after.content_hash);
    }
}
