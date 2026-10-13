//! Cell-local native workbooks. Upstream JSON and accepted OT messages are the
//! only body truth. The replay log is kept whole; a fresh read starts from the
//! newest server-computed checkpoint so replay cost stays bounded as a sheet ages.
use crate::OrgIntel;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest as _, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub const SHEET_ENGINE_VERSION: &str = "19.0.51";
pub const MAX_SHEET_BYTES: usize = 8 * 1024 * 1024;
/// Revisions a fresh read replays on top of its checkpoint, at least. Undo of a
/// revision older than this window is refused rather than replayed.
pub const SHEET_REPLAY_WINDOW: i64 = 200;
/// Revision JSON a fresh read replays, at most. A few very large pastes or
/// imports would otherwise push a sheet past the engine's 32 MiB input and it
/// could no longer open. Past this, the read starts from the newest checkpoint,
/// which narrows the undo window instead.
pub const SHEET_REPLAY_BYTES: usize = 16 * 1024 * 1024;
/// An accepted batch at least this large is checkpointed at once, so a read
/// over the replay budget always has a checkpoint just after it.
const LARGE_BATCH_BYTES: usize = 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum SheetError {
    #[error("sheet is unavailable")]
    Unavailable,
    #[error("sheet conflict: {0}")]
    Conflict(String),
    #[error("invalid sheet: {0}")]
    Invalid(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}
type Result<T> = std::result::Result<T, SheetError>;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SheetRow {
    pub id: Uuid,
    pub title: String,
    pub owner_actor_id: String,
    pub visibility: String,
    /// What every person in the company may do without being added: "private", "view",
    /// "comment" or "edit". Exec and Staff never gain access this way; they are added.
    pub general_access: String,
    pub engine_version: String,
    pub head_revision: String,
    pub sequence: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Set while the sheet is archived out of the Library. It still opens.
    pub archived_at: Option<DateTime<Utc>>,
    /// In a listing: whether this actor may archive, restore and rename the sheet.
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub may_manage: Option<bool>,
    /// In a listing: who made the newest change, for "Edited by" in the Library.
    #[sqlx(skip)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_editor_actor_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SheetMessage {
    pub sequence: i64,
    pub actor_id: String,
    pub message: Value,
}
#[derive(Debug, Clone, Serialize)]
pub struct SheetState {
    pub sheet: SheetRow,
    pub access: String,
    pub snapshot: Value,
    pub messages: Vec<SheetMessage>,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SheetCheckpoint {
    pub id: Uuid,
    pub sequence: i64,
    pub revision_id: String,
    pub title: Option<String>,
    pub actor_id: String,
    pub created_at: DateTime<Utc>,
}
/// Who can open a sheet, for its share panel: the general access, each person added with their
/// level, and whether the reader may change any of it.
#[derive(Debug, Clone, Serialize)]
pub struct SheetSharing {
    pub general_access: String,
    pub owner_actor_id: String,
    pub people: Vec<SheetParticipant>,
    pub may_manage: bool,
}
/// Someone asking for a sheet: to view (`read`), to comment or to edit.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SheetAccessRequest {
    pub id: Uuid,
    pub actor_id: String,
    pub access: String,
    pub requested_at: chrono::DateTime<chrono::Utc>,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SheetParticipant {
    pub actor_id: String,
    pub access: String,
    /// Who added them; unknown for people added before it was kept. A level change keeps it.
    pub added_by_actor_id: Option<String>,
    pub added_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Serialize)]
pub struct SheetClient {
    pub client_id: Uuid,
    pub reconnect_token: String,
    pub generation: Uuid,
}
const ROW: &str = "id,title,owner_actor_id,visibility,CASE WHEN visibility='participants' THEN 'private' WHEN company_access='read' THEN 'view' WHEN company_access='comment' THEN 'comment' ELSE 'edit' END AS general_access,engine_version,head_revision,sequence,created_at,updated_at,archived_at";

pub(crate) async fn access(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    actor: &str,
    edit: bool,
) -> Result<String> {
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT owner_actor_id,visibility,company_access FROM native_sheets WHERE id=$1 FOR SHARE",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((owner, visibility, company_access)) = row else {
        return Err(SheetError::Unavailable);
    };
    let class: Option<String> = sqlx::query_scalar(
        "SELECT actor_class FROM actors WHERE id=$1 AND retired_at IS NULL FOR SHARE",
    )
    .bind(actor)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(class) = class else {
        return Err(SheetError::Unavailable);
    };
    // A suspended external member loses access even if the durable Actor is
    // retained for attribution. Local humans have no external binding.
    let denied: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM human_principal_actor_bindings WHERE actor_id=$1 AND membership_status<>'active')")
        .bind(actor).fetch_one(&mut **tx).await?;
    if denied {
        return Err(SheetError::Unavailable);
    }
    let explicit: Option<String> = sqlx::query_scalar(
        "SELECT access FROM native_sheet_participants WHERE sheet_id=$1 AND actor_id=$2",
    )
    .bind(id)
    .bind(actor)
    .fetch_optional(&mut **tx)
    .await?;
    let grant = if owner == actor {
        Some("edit".to_string())
    } else {
        // People in the company get a company sheet's general access; an explicit grant still
        // wins, and Exec and Staff see a sheet only once it is shared with them.
        explicit.or_else(|| (visibility == "company" && class == "human").then_some(company_access))
    };
    let grant = grant.ok_or(SheetError::Unavailable)?;
    if edit && grant != "edit" {
        return Err(SheetError::Unavailable);
    }
    Ok(grant)
}
/// Archiving, restoring and renaming: whoever may edit the sheet may, and the
/// company's owner may tidy any sheet they can see, including one an agent made.
async fn may_manage(tx: &mut Transaction<'_, Postgres>, id: Uuid, actor: &str) -> Result<()> {
    if access(tx, id, actor, true).await.is_ok() {
        return Ok(());
    }
    access(tx, id, actor, false).await?;
    if !company_owner(tx, actor).await? {
        return Err(SheetError::Unavailable);
    }
    Ok(())
}
/// Who may decide who else sees or edits a sheet, as for documents: whoever made it, the
/// company's owner, and the Exec acting for that owner, except on a person's private sheet.
pub(crate) async fn may_share(tx: &mut Transaction<'_, Postgres>, id: Uuid, actor: &str) -> Result<bool> {
    let row: Option<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT s.owner_actor_id,s.visibility,a.actor_class FROM native_sheets s \
         LEFT JOIN actors a ON a.id=s.owner_actor_id WHERE s.id=$1",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((owner, visibility, owner_class)) = row else {
        return Err(SheetError::Unavailable);
    };
    // A suspended member shares nothing, as they open nothing.
    let denied: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM human_principal_actor_bindings WHERE actor_id=$1 AND membership_status<>'active')")
        .bind(actor).fetch_one(&mut **tx).await?;
    if denied {
        return Ok(false);
    }
    if owner == actor || company_owner(tx, actor).await? {
        return Ok(true);
    }
    Ok(actor == "exec"
        && (visibility == "company" || !matches!(owner_class.as_deref(), Some("human" | "owner"))))
}
async fn company_owner(tx: &mut Transaction<'_, Postgres>, actor: &str) -> Result<bool> {
    if actor == "owner" {
        return Ok(true);
    }
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM human_principal_actor_bindings WHERE actor_id=$1 AND membership_role='owner' AND membership_status='active')")
        .bind(actor)
        .fetch_one(&mut **tx)
        .await?)
}
/// The value's serialized size, refused past the 8 MiB limit.
fn bounded(value: &Value) -> Result<usize> {
    let size = serde_json::to_vec(value)
        .map_err(|e| SheetError::Invalid(e.to_string()))?
        .len();
    if size > MAX_SHEET_BYTES {
        return Err(SheetError::Invalid("workbook exceeds 8 MiB".into()));
    }
    Ok(size)
}
/// How much a level allows: viewing, then commenting, then editing.
pub(crate) fn level_rank(level: &str) -> u8 {
    match level {
        "edit" => 2,
        "comment" => 1,
        _ => 0,
    }
}
/// The same order in SQL, for a column or parameter holding a level.
fn level_rank_sql(expression: &str) -> String {
    format!("(CASE {expression} WHEN 'edit' THEN 2 WHEN 'comment' THEN 1 ELSE 0 END)")
}
const LEVELS: [&str; 3] = ["read", "comment", "edit"];
fn revision(message: &Value, field: &str) -> Result<String> {
    let value = message[field]
        .as_str()
        .filter(|v| !v.is_empty() && v.len() <= 100)
        .ok_or_else(|| SheetError::Invalid(format!("missing {field}")))?;
    Ok(value.into())
}

impl OrgIntel {
    pub async fn claim_sheet_client(
        &self,
        id: Uuid,
        actor: &str,
        resume: Option<(Uuid, &str)>,
    ) -> Result<SheetClient> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT id FROM native_sheets WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(SheetError::Unavailable)?;
        access(&mut tx, id, actor, false).await?;
        let generation = Uuid::new_v4();
        let (client_id, reconnect_token) = if let Some((client, token)) = resume {
            let claim:Option<(String,Vec<u8>)>=sqlx::query_as("SELECT actor_id,reconnect_hash FROM native_sheet_clients WHERE sheet_id=$1 AND client_id=$2")
                .bind(id).bind(client).fetch_optional(&mut *tx).await?;
            let Some((owner, hash)) = claim else {
                return Err(SheetError::Unavailable);
            };
            if owner != actor || hash != Sha256::digest(token.as_bytes()).to_vec() {
                return Err(SheetError::Unavailable);
            }
            sqlx::query(
                "UPDATE native_sheet_clients SET generation=$3 WHERE sheet_id=$1 AND client_id=$2",
            )
            .bind(id)
            .bind(client)
            .bind(generation)
            .execute(&mut *tx)
            .await?;
            (client, token.to_string())
        } else {
            let client = Uuid::new_v4();
            let token = Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO native_sheet_clients(sheet_id,client_id,actor_id,reconnect_hash,generation) VALUES($1,$2,$3,$4,$5)")
                .bind(id).bind(client).bind(actor).bind(Sha256::digest(token.as_bytes()).to_vec()).bind(generation).execute(&mut *tx).await?;
            (client, token)
        };
        tx.commit().await?;
        Ok(SheetClient {
            client_id,
            reconnect_token,
            generation,
        })
    }
    pub async fn sheet_client_active(
        &self,
        id: Uuid,
        actor: &str,
        client: Uuid,
        generation: Uuid,
    ) -> Result<bool> {
        Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM native_sheet_clients WHERE sheet_id=$1 AND actor_id=$2 AND client_id=$3 AND generation=$4)")
            .bind(id).bind(actor).bind(client).bind(generation).fetch_one(&self.pool).await?)
    }
    pub async fn create_sheet(
        &self,
        id: Uuid,
        actor: &str,
        title: &str,
        visibility: &str,
        workbook: &Value,
    ) -> Result<SheetRow> {
        if title.trim().is_empty()
            || title.chars().count() > 200
            || !["company", "participants"].contains(&visibility)
        {
            return Err(SheetError::Invalid("title or visibility is invalid".into()));
        }
        bounded(workbook)?;
        let head = revision(workbook, "revisionId")?;
        let mut tx = self.pool.begin().await?;
        let active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL)",
        )
        .bind(actor)
        .fetch_one(&mut *tx)
        .await?;
        if !active {
            return Err(SheetError::Unavailable);
        }
        let denied:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM human_principal_actor_bindings WHERE actor_id=$1 AND membership_status<>'active')")
            .bind(actor).fetch_one(&mut *tx).await?;
        if denied {
            return Err(SheetError::Unavailable);
        }
        sqlx::query("INSERT INTO native_sheets(id,title,owner_actor_id,visibility,engine_version,replay_base,head_revision) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(id) DO NOTHING")
            .bind(id).bind(title.trim()).bind(actor).bind(visibility).bind(SHEET_ENGINE_VERSION).bind(workbook).bind(head).execute(&mut *tx).await?;
        let row: SheetRow = sqlx::query_as(&format!("SELECT {ROW} FROM native_sheets WHERE id=$1"))
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
        if row.owner_actor_id != actor || row.title != title.trim() || row.visibility != visibility
        {
            return Err(SheetError::Conflict("creation ID already used".into()));
        }
        let original: Value =
            sqlx::query_scalar("SELECT replay_base FROM native_sheets WHERE id=$1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        if original != *workbook {
            return Err(SheetError::Conflict(
                "creation ID already used for different workbook".into(),
            ));
        }
        tx.commit().await?;
        Ok(row)
    }
    /// Archive a sheet out of the Library, or restore it. Whoever may edit it may.
    pub async fn set_sheet_archived(&self, id: Uuid, actor: &str, archived: bool) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        may_manage(&mut tx, id, actor).await?;
        sqlx::query(
            "UPDATE native_sheets SET archived_at = CASE WHEN $2 THEN now() ELSE NULL END WHERE id=$1",
        )
        .bind(id)
        .bind(archived)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
    /// Rename a sheet. The same people who may archive it may rename it.
    pub async fn rename_sheet(&self, id: Uuid, actor: &str, title: &str) -> Result<SheetRow> {
        let title = title.trim();
        if title.is_empty() || title.chars().count() > 200 {
            return Err(SheetError::Invalid(
                "a sheet title needs 1 to 200 characters".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        may_manage(&mut tx, id, actor).await?;
        let row: SheetRow = sqlx::query_as(&format!(
            "UPDATE native_sheets SET title=$2, updated_at=now() WHERE id=$1 RETURNING {ROW}"
        ))
        .bind(id)
        .bind(title)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(row)
    }

    /// Delete an archived sheet for good, with its history, checkpoints and shares. Its owner or
    /// the company's owner may, once it is archived.
    pub async fn delete_archived_sheet(&self, id: Uuid, actor: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        access(&mut tx, id, actor, false).await?;
        let (owner, archived): (String, bool) = sqlx::query_as(
            "SELECT owner_actor_id,archived_at IS NOT NULL FROM native_sheets WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(SheetError::Unavailable)?;
        if owner != actor {
            let company_owner: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM human_principal_actor_bindings WHERE actor_id=$1 AND membership_role='owner' AND membership_status='active')")
                .bind(actor)
                .fetch_one(&mut *tx)
                .await?;
            if !company_owner && actor != "owner" {
                return Err(SheetError::Unavailable);
            }
        }
        if !archived {
            return Err(SheetError::Conflict(
                "archive the sheet before deleting it permanently".into(),
            ));
        }
        sqlx::query("DELETE FROM native_sheets WHERE id=$1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO events (kind,actor_id,body) VALUES ($1,$2,$3)")
            .bind("owner.sheet.deleted.v1")
            .bind(actor)
            .bind(serde_json::json!({ "sheet_id": id }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_sheets(&self, actor: &str) -> Result<Vec<SheetRow>> {
        self.list_sheets_where(actor, false).await
    }

    /// The archived sheets an actor can see, for restoring.
    pub async fn list_archived_sheets(&self, actor: &str) -> Result<Vec<SheetRow>> {
        self.list_sheets_where(actor, true).await
    }

    /// The 100 most recently changed sheets this actor can see. Access is checked
    /// before the cap, so sheets shared with others never crowd out this actor's own.
    async fn list_sheets_where(&self, actor: &str, archived: bool) -> Result<Vec<SheetRow>> {
        const PAGE: i64 = 100;
        let mut tx = self.pool.begin().await?;
        let manages_all = company_owner(&mut tx, actor).await?;
        let mut visible = Vec::new();
        let mut offset = 0;
        while visible.len() < PAGE as usize {
            let rows: Vec<SheetRow> = sqlx::query_as(&format!(
                "SELECT {ROW} FROM native_sheets WHERE (archived_at IS NOT NULL) = $1 \
                 ORDER BY updated_at DESC, id LIMIT $2 OFFSET $3"
            ))
            .bind(archived)
            .bind(PAGE)
            .bind(offset)
            .fetch_all(&mut *tx)
            .await?;
            let last = (rows.len() as i64) < PAGE;
            offset += rows.len() as i64;
            for mut row in rows {
                if visible.len() >= PAGE as usize {
                    continue;
                }
                if let Ok(grant) = access(&mut tx, row.id, actor, false).await {
                    row.may_manage = Some(manages_all || grant == "edit");
                    visible.push(row);
                }
            }
            if last {
                break;
            }
        }
        // Who changed each one last: the newest accepted operation, else the newest checkpoint.
        let ids: Vec<Uuid> = visible.iter().map(|row| row.id).collect();
        let editors: Vec<(Uuid, Option<String>)> = sqlx::query_as(
            "SELECT sheet.id,COALESCE(\
               (SELECT m.actor_id FROM native_sheet_messages m WHERE m.sheet_id=sheet.id \
                  AND m.actor_id<>'daemon' ORDER BY m.sequence DESC LIMIT 1),\
               (SELECT c.actor_id FROM native_sheet_checkpoints c WHERE c.sheet_id=sheet.id \
                  AND c.actor_id<>'daemon' ORDER BY c.sequence DESC LIMIT 1)) \
             FROM unnest($1::uuid[]) AS sheet(id)",
        )
        .bind(&ids)
        .fetch_all(&mut *tx)
        .await?;
        for (id, editor) in editors {
            if let Some(row) = visible.iter_mut().find(|row| row.id == id) {
                row.last_editor_actor_id = editor;
            }
        }
        tx.commit().await?;
        Ok(visible)
    }
    pub async fn sheet_state(
        &self,
        id: Uuid,
        actor: &str,
        edit: bool,
        after: i64,
    ) -> Result<SheetState> {
        let mut tx = self.pool.begin().await?;
        let grant = access(&mut tx, id, actor, edit).await?;
        let sheet: SheetRow =
            sqlx::query_as(&format!("SELECT {ROW} FROM native_sheets WHERE id=$1"))
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
        // A catch-up read continues from its cursor. A fresh read starts from the
        // newest checkpoint that still leaves the replay window, so neither the
        // browser nor the model worker replays a long-lived sheet's whole history.
        let checkpoint_read = after == 0;
        let checkpoint: Option<(Value, i64)> = if checkpoint_read {
            sqlx::query_as("SELECT workbook,sequence FROM native_sheet_checkpoints WHERE sheet_id=$1 AND sequence<=$2 ORDER BY sequence DESC LIMIT 1")
                .bind(id).bind(sheet.sequence - SHEET_REPLAY_WINDOW).fetch_optional(&mut *tx).await?
        } else {
            None
        };
        let (mut snapshot, after) = match checkpoint {
            Some((workbook, sequence)) => (workbook, sequence),
            None => (
                sqlx::query_scalar("SELECT replay_base FROM native_sheets WHERE id=$1")
                    .bind(id)
                    .fetch_one(&mut *tx)
                    .await?,
                after,
            ),
        };
        let mut messages: Vec<SheetMessage> = sqlx::query_as("SELECT sequence,actor_id,message FROM native_sheet_messages WHERE sheet_id=$1 AND sequence>$2 ORDER BY sequence")
            .bind(id).bind(after).fetch_all(&mut *tx).await?;
        // Within the replay budget unless a few very large revisions are recent;
        // then the newest checkpoint, taken just after the latest of them.
        if checkpoint_read
            && messages
                .iter()
                .map(|m| serde_json::to_vec(&m.message).map_or(0, |v| v.len()))
                .sum::<usize>()
                > SHEET_REPLAY_BYTES
        {
            let newest: Option<(Value, i64)> = sqlx::query_as("SELECT workbook,sequence FROM native_sheet_checkpoints WHERE sheet_id=$1 AND sequence>$2 ORDER BY sequence DESC LIMIT 1")
                .bind(id).bind(after).fetch_optional(&mut *tx).await?;
            if let Some((workbook, sequence)) = newest {
                snapshot = workbook;
                messages.retain(|m| m.sequence > sequence);
            }
        }
        tx.commit().await?;
        Ok(SheetState {
            sheet,
            access: grant,
            snapshot,
            messages,
        })
    }
    pub async fn sheet_command_result(
        &self,
        id: Uuid,
        actor: &str,
        key: Uuid,
        request: &Value,
    ) -> Result<Option<Value>> {
        let mut tx = self.pool.begin().await?;
        access(&mut tx, id, actor, true).await?;
        let stored: Option<(String,Value,Value)> = sqlx::query_as("SELECT actor_id,request,result FROM native_sheet_commands WHERE sheet_id=$1 AND command_id=$2")
            .bind(id).bind(key).fetch_optional(&mut *tx).await?;
        tx.commit().await?;
        match stored {
            Some((owner, original, result)) if owner == actor && &original == request => {
                Ok(Some(result))
            }
            Some(_) => Err(SheetError::Conflict(
                "command key reused for different input".into(),
            )),
            None => Ok(None),
        }
    }
    /// Validated upstream messages enter the same ordered stream regardless of
    /// whether the writer is a browser or an agent. Commit precedes delivery.
    pub async fn accept_sheet_messages(
        &self,
        id: Uuid,
        actor: &str,
        expected: &str,
        messages: &[Value],
        workbook: &Value,
        command: Option<(Uuid, &Value, &Value)>,
        client: Option<(Uuid, Uuid)>,
    ) -> Result<i64> {
        bounded(workbook)?;
        let mut tx = self.pool.begin().await?;
        let (mut head, mut sequence): (String, i64) = sqlx::query_as(
            "SELECT head_revision,sequence FROM native_sheets WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(SheetError::Unavailable)?;
        access(&mut tx, id, actor, true).await?;
        if let Some((client, generation)) = client {
            let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM native_sheet_clients WHERE sheet_id=$1 AND actor_id=$2 AND client_id=$3 AND generation=$4)")
                .bind(id).bind(actor).bind(client).bind(generation).fetch_one(&mut *tx).await?;
            if !valid {
                return Err(SheetError::Unavailable);
            }
        }
        if let Some((key, request, _)) = command {
            let stored: Option<(String,Value)> = sqlx::query_as("SELECT actor_id,request FROM native_sheet_commands WHERE sheet_id=$1 AND command_id=$2")
                .bind(id).bind(key).fetch_optional(&mut *tx).await?;
            if let Some((owner, original)) = stored {
                if owner != actor || &original != request {
                    return Err(SheetError::Conflict("command key reused".into()));
                }
                tx.commit().await?;
                return Ok(sequence);
            }
        }
        // A lost acknowledgement may resend a previously accepted revision.
        if messages.len() == 1 {
            let next = revision(&messages[0], "nextRevisionId")?;
            let stored: Option<(String,Value)> = sqlx::query_as("SELECT actor_id,message FROM native_sheet_messages WHERE sheet_id=$1 AND revision_id=$2")
                .bind(id).bind(next).fetch_optional(&mut *tx).await?;
            if let Some((owner, original)) = stored {
                let mut submitted = messages[0].clone();
                submitted.as_object_mut().map(|o| o.remove("timestamp"));
                let mut original = original;
                original.as_object_mut().map(|o| o.remove("timestamp"));
                if owner != actor || submitted != original {
                    return Err(SheetError::Conflict("revision ID reused".into()));
                }
                tx.commit().await?;
                return Ok(sequence);
            }
        }
        if head != expected {
            return Err(SheetError::Conflict(
                "workbook changed; reread its current revision".into(),
            ));
        }
        let mut batch_bytes = 0;
        for message in messages {
            batch_bytes += bounded(message)?;
            if message["version"] != 1 || revision(message, "serverRevisionId")? != head {
                return Err(SheetError::Invalid("revision chain is invalid".into()));
            }
            let kind = message["type"].as_str().unwrap_or("");
            if !["REMOTE_REVISION", "REVISION_UNDONE", "REVISION_REDONE"].contains(&kind) {
                return Err(SheetError::Invalid(
                    "unsupported collaboration message".into(),
                ));
            }
            if kind != "REMOTE_REVISION" {
                let field = if kind == "REVISION_UNDONE" {
                    "undoneRevisionId"
                } else {
                    "redoneRevisionId"
                };
                let target = revision(message, field)?;
                let owner:Option<String>=sqlx::query_scalar("SELECT actor_id FROM native_sheet_messages WHERE sheet_id=$1 AND revision_id=$2 AND message->>'type'='REMOTE_REVISION'")
                    .bind(id).bind(target).fetch_optional(&mut *tx).await?;
                if owner.as_deref() != Some(actor) {
                    return Err(SheetError::Unavailable);
                }
            } else {
                let message_client = Uuid::parse_str(&revision(message, "clientId")?)
                    .map_err(|_| SheetError::Invalid("invalid client ID".into()))?;
                if let Some((bound, _)) = client {
                    if bound != message_client {
                        return Err(SheetError::Unavailable);
                    }
                }
                // Agent Models also reserve their IDs atomically. No two Actors
                // can cause upstream to mistake each other's operation for an ACK.
                let owner: Option<String> = sqlx::query_scalar(
                    "SELECT actor_id FROM native_sheet_clients WHERE sheet_id=$1 AND client_id=$2",
                )
                .bind(id)
                .bind(message_client)
                .fetch_optional(&mut *tx)
                .await?;
                if owner.as_deref().is_some_and(|o| o != actor) {
                    return Err(SheetError::Unavailable);
                }
                if owner.is_none() {
                    sqlx::query("INSERT INTO native_sheet_clients(sheet_id,client_id,actor_id,reconnect_hash,generation) VALUES($1,$2,$3,$4,$5)")
                        .bind(id).bind(message_client).bind(actor).bind(Vec::<u8>::new()).bind(Uuid::new_v4()).execute(&mut *tx).await?;
                }
            }
            let next = revision(message, "nextRevisionId")?;
            let next_uuid = Uuid::parse_str(&next)
                .map_err(|_| SheetError::Invalid("next revision must be a fresh UUID".into()))?;
            if next_uuid.is_nil() || next == head {
                return Err(SheetError::Invalid("next revision must be fresh".into()));
            }
            let reused:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM native_sheet_messages WHERE sheet_id=$1 AND revision_id=$2) OR EXISTS(SELECT 1 FROM native_sheets WHERE id=$1 AND replay_base->>'revisionId'=$2)")
                .bind(id).bind(&next).fetch_one(&mut *tx).await?;
            if reused {
                return Err(SheetError::Conflict("revision ID reused".into()));
            }
            head = next;
            sequence += 1;
            sqlx::query("INSERT INTO native_sheet_messages(sheet_id,sequence,revision_id,actor_id,message) VALUES($1,$2,$3,$4,$5)")
                .bind(id).bind(sequence).bind(&head).bind(actor).bind(message).execute(&mut *tx).await?;
        }
        if workbook["revisionId"].as_str() != Some(&head) {
            return Err(SheetError::Invalid(
                "checkpoint does not match accepted head".into(),
            ));
        }
        sqlx::query(
            "UPDATE native_sheets SET head_revision=$2,sequence=$3,updated_at=now() WHERE id=$1",
        )
        .bind(id)
        .bind(&head)
        .bind(sequence)
        .execute(&mut *tx)
        .await?;
        if !messages.is_empty()
            && (sequence / 50 > (sequence - messages.len() as i64) / 50
                || batch_bytes >= LARGE_BATCH_BYTES)
        {
            sqlx::query("INSERT INTO native_sheet_checkpoints(id,sheet_id,sequence,revision_id,workbook,actor_id) VALUES($1,$2,$3,$4,$5,$6)")
                .bind(Uuid::new_v4()).bind(id).bind(sequence).bind(&head).bind(workbook).bind(actor).execute(&mut *tx).await?;
        }
        if let Some((key, request, result)) = command {
            sqlx::query("INSERT INTO native_sheet_commands(sheet_id,command_id,actor_id,request,result) VALUES($1,$2,$3,$4,$5)")
                .bind(id).bind(key).bind(actor).bind(request).bind(result).execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(sequence)
    }
    /// Asks whoever may share a sheet for access to it: to view, to comment or to edit. Nothing
    /// happens for a sheet that is gone, or for an ask the person's access already answers. An
    /// ask set aside stays quiet for a day unless the person asks for more.
    pub async fn request_sheet_access(&self, id: Uuid, actor: &str, wanted: &str) -> Result<()> {
        if !LEVELS.contains(&wanted) {
            return Err(SheetError::Invalid("invalid access".into()));
        }
        let mut tx = self.pool.begin().await?;
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM native_sheets WHERE id=$1 AND archived_at IS NULL)",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let human: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL \
               AND actor_class IN ('human','owner'))",
        )
        .bind(actor)
        .fetch_one(&mut *tx)
        .await?;
        if !exists || !human {
            return Ok(());
        }
        match access(&mut tx, id, actor, false).await {
            Ok(grant) if level_rank(&grant) >= level_rank(wanted) => return Ok(()),
            Ok(_) | Err(SheetError::Unavailable) => {}
            Err(error) => return Err(error),
        }
        // An open ask keeps the most it asked for; one set aside wakes for more, or after a day.
        let old = level_rank_sql("native_sheet_access_requests.access");
        let new = level_rank_sql("EXCLUDED.access");
        sqlx::query(&format!(
            "INSERT INTO native_sheet_access_requests (id,sheet_id,actor_id,access) \
             VALUES ($1,$2,$3,$4) \
             ON CONFLICT (sheet_id,actor_id) DO UPDATE SET \
               access=CASE WHEN native_sheet_access_requests.closed_at IS NULL \
                 AND {old}>{new} THEN native_sheet_access_requests.access ELSE EXCLUDED.access END, \
               requested_at=CASE WHEN native_sheet_access_requests.closed_at IS NULL \
                 AND {old}>={new} \
                 THEN native_sheet_access_requests.requested_at ELSE now() END, \
               closed_at=NULL,closed_by_actor_id=NULL \
             WHERE native_sheet_access_requests.closed_at IS NULL \
               OR native_sheet_access_requests.closed_at<now()-interval '1 day' \
               OR {new}>{old}"
        ))
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(actor)
        .bind(wanted)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// The open asks for a sheet, for whoever may share it; an ask already answered is left out.
    pub async fn list_sheet_access_requests(
        &self,
        id: Uuid,
        actor: &str,
    ) -> Result<Vec<SheetAccessRequest>> {
        let mut tx = self.pool.begin().await?;
        if !may_share(&mut tx, id, actor).await? {
            return Err(SheetError::Unavailable);
        }
        let rows: Vec<SheetAccessRequest> = sqlx::query_as(
            "SELECT id,actor_id,access,requested_at FROM native_sheet_access_requests \
             WHERE sheet_id=$1 AND closed_at IS NULL ORDER BY requested_at LIMIT 50",
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        let mut open = Vec::with_capacity(rows.len());
        for row in rows {
            let answered = match access(&mut tx, id, &row.actor_id, false).await {
                Ok(grant) => level_rank(&grant) >= level_rank(&row.access),
                Err(SheetError::Unavailable) => false,
                Err(error) => return Err(error),
            };
            if !answered {
                open.push(row);
            }
        }
        tx.commit().await?;
        Ok(open)
    }

    /// Whoever may share a sheet sets an ask aside without granting it.
    pub async fn dismiss_sheet_access_request(
        &self,
        id: Uuid,
        request: Uuid,
        actor: &str,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        if !may_share(&mut tx, id, actor).await? {
            return Err(SheetError::Unavailable);
        }
        let changed = sqlx::query(
            "UPDATE native_sheet_access_requests SET closed_at=now(),closed_by_actor_id=$3 \
             WHERE sheet_id=$1 AND id=$2 AND closed_at IS NULL",
        )
        .bind(id)
        .bind(request)
        .bind(actor)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if changed != 1 {
            return Err(SheetError::Unavailable);
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn share_sheet(
        &self,
        id: Uuid,
        actor: &str,
        participant: &str,
        grant: Option<&str>,
    ) -> Result<()> {
        if grant.is_some_and(|g| !LEVELS.contains(&g)) {
            return Err(SheetError::Invalid("invalid access".into()));
        }
        let mut tx = self.pool.begin().await?;
        let owner: Option<String> =
            sqlx::query_scalar("SELECT owner_actor_id FROM native_sheets WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;
        if owner.is_none() || !may_share(&mut tx, id, actor).await? {
            return Err(SheetError::Unavailable);
        }
        if let Some(grant) = grant {
            let active: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL)",
            )
            .bind(participant)
            .fetch_one(&mut *tx)
            .await?;
            if !active {
                return Err(SheetError::Unavailable);
            }
            sqlx::query("INSERT INTO native_sheet_participants(sheet_id,actor_id,access,added_by_actor_id,added_at) VALUES($1,$2,$3,$4,now()) ON CONFLICT(sheet_id,actor_id) DO UPDATE SET access=EXCLUDED.access")
                .bind(id).bind(participant).bind(grant).bind(actor).execute(&mut *tx).await?;
            // Giving someone what they asked for answers their ask.
            sqlx::query(&format!("UPDATE native_sheet_access_requests SET closed_at=now(),closed_by_actor_id=$3 WHERE sheet_id=$1 AND actor_id=$2 AND closed_at IS NULL AND {}<={}", level_rank_sql("access"), level_rank_sql("$4")))
                .bind(id).bind(participant).bind(actor).bind(grant).execute(&mut *tx).await?;
        } else {
            sqlx::query("DELETE FROM native_sheet_participants WHERE sheet_id=$1 AND actor_id=$2")
                .bind(id)
                .bind(participant)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
    /// Set who in the company may open a sheet without being added: "private", "view",
    /// "comment" or "edit".
    /// Its managers may, as they may share it.
    pub async fn set_sheet_general_access(
        &self,
        id: Uuid,
        actor: &str,
        general_access: &str,
    ) -> Result<SheetRow> {
        let (visibility, company_access) = match general_access {
            "private" => ("participants", None),
            "view" => ("company", Some("read")),
            "comment" => ("company", Some("comment")),
            "edit" => ("company", Some("edit")),
            _ => return Err(SheetError::Invalid("invalid general access".into())),
        };
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT id FROM native_sheets WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(SheetError::Unavailable)?;
        if !may_share(&mut tx, id, actor).await? {
            return Err(SheetError::Unavailable);
        }
        let row: SheetRow = sqlx::query_as(&format!(
            "UPDATE native_sheets SET visibility=$2, company_access=COALESCE($3,company_access) \
             WHERE id=$1 RETURNING {ROW}"
        ))
        .bind(id)
        .bind(visibility)
        .bind(company_access)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO events (kind,actor_id,body) VALUES ($1,$2,$3)")
            .bind("sheet.general_access.updated.v1")
            .bind(actor)
            .bind(json!({ "sheet_id": id, "general_access": general_access }))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(row)
    }
    /// Whether this person may open the sheet now, for projections that recheck access on read.
    pub async fn sheet_readable(&self, id: Uuid, actor: &str) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let readable = match access(&mut tx, id, actor, false).await {
            Ok(_) => true,
            Err(SheetError::Unavailable) => false,
            Err(error) => return Err(error),
        };
        tx.commit().await?;
        Ok(readable)
    }
    /// The share panel's view of a sheet, for anyone who can open it or may manage it.
    pub async fn sheet_sharing(&self, id: Uuid, actor: &str) -> Result<SheetSharing> {
        let mut tx = self.pool.begin().await?;
        let may_manage = may_share(&mut tx, id, actor).await?;
        if !may_manage {
            access(&mut tx, id, actor, false).await?;
        }
        let (owner_actor_id, general_access): (String, String) = sqlx::query_as(
            "SELECT owner_actor_id,CASE WHEN visibility='participants' THEN 'private' \
               WHEN company_access='read' THEN 'view' WHEN company_access='comment' THEN 'comment' \
               ELSE 'edit' END \
             FROM native_sheets WHERE id=$1",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let people = sqlx::query_as(
            "SELECT p.actor_id,p.access,p.added_by_actor_id,p.added_at FROM native_sheet_participants p \
             JOIN actors a ON a.id=p.actor_id \
             WHERE p.sheet_id=$1 AND a.retired_at IS NULL ORDER BY p.actor_id",
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(SheetSharing {
            general_access,
            owner_actor_id,
            people,
            may_manage,
        })
    }
    pub async fn sheet_checkpoints(&self, id: Uuid, actor: &str) -> Result<Vec<SheetCheckpoint>> {
        let mut tx = self.pool.begin().await?;
        access(&mut tx, id, actor, false).await?;
        let rows=sqlx::query_as("SELECT id,sequence,revision_id,title,actor_id,created_at FROM native_sheet_checkpoints WHERE sheet_id=$1 ORDER BY sequence DESC,created_at DESC LIMIT 100")
            .bind(id).fetch_all(&mut *tx).await?;
        tx.commit().await?;
        Ok(rows)
    }
    pub async fn sheet_checkpoint(&self, id: Uuid, actor: &str, version: Uuid) -> Result<Value> {
        let mut tx = self.pool.begin().await?;
        access(&mut tx, id, actor, false).await?;
        let view:Option<Value>=sqlx::query_scalar("SELECT jsonb_build_object('id',id,'sheet_id',sheet_id,'sequence',sequence,'revision_id',revision_id,'title',title,'actor_id',actor_id,'created_at',created_at,'workbook',workbook) FROM native_sheet_checkpoints WHERE sheet_id=$1 AND id=$2")
            .bind(id).bind(version).fetch_optional(&mut *tx).await?;
        tx.commit().await?;
        view.ok_or(SheetError::Unavailable)
    }
    pub async fn checkpoint_sheet(
        &self,
        id: Uuid,
        actor: &str,
        key: Uuid,
        title: &str,
        expected: &str,
        workbook: &Value,
    ) -> Result<Value> {
        bounded(workbook)?;
        if title.trim().is_empty() || title.chars().count() > 200 {
            return Err(SheetError::Invalid("checkpoint title is invalid".into()));
        }
        let mut tx = self.pool.begin().await?;
        let (head, sequence): (String, i64) = sqlx::query_as(
            "SELECT head_revision,sequence FROM native_sheets WHERE id=$1 FOR UPDATE",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(SheetError::Unavailable)?;
        access(&mut tx, id, actor, true).await?;
        let existing:Option<(Uuid,String,Option<String>,String,i64)>=sqlx::query_as("SELECT sheet_id,actor_id,title,revision_id,sequence FROM native_sheet_checkpoints WHERE id=$1")
            .bind(key).fetch_optional(&mut *tx).await?;
        if let Some((sheet, owner, label, revision, sequence)) = existing {
            if sheet != id
                || owner != actor
                || label.as_deref() != Some(title.trim())
                || revision != expected
            {
                return Err(SheetError::Conflict(
                    "checkpoint ID reused for different input".into(),
                ));
            }
            tx.commit().await?;
            return Ok(json!({"id":key,"revision_id":revision,"sequence":sequence}));
        }
        if head != expected || workbook["revisionId"].as_str() != Some(&head) {
            return Err(SheetError::Conflict("checkpoint revision changed".into()));
        }
        sqlx::query("INSERT INTO native_sheet_checkpoints(id,sheet_id,sequence,revision_id,workbook,title,actor_id) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(id) DO NOTHING")
            .bind(key).bind(id).bind(sequence).bind(&head).bind(workbook).bind(title.trim()).bind(actor).execute(&mut *tx).await?;
        // The checkpoint key is global, so another sheet's transaction may
        // win its unique insertion while this sheet's lock is held.
        let stored:(Uuid,String,Option<String>,String,i64)=sqlx::query_as("SELECT sheet_id,actor_id,title,revision_id,sequence FROM native_sheet_checkpoints WHERE id=$1")
            .bind(key).fetch_one(&mut *tx).await?;
        if stored.0 != id
            || stored.1 != actor
            || stored.2.as_deref() != Some(title.trim())
            || stored.3 != expected
        {
            return Err(SheetError::Conflict(
                "checkpoint ID reused for different input".into(),
            ));
        }
        tx.commit().await?;
        Ok(json!({"id":key,"revision_id":head,"sequence":sequence}))
    }
    pub async fn sheet_checkpoint_receipt(
        &self,
        id: Uuid,
        actor: &str,
        key: Uuid,
        title: &str,
        expected: &str,
    ) -> Result<Option<Value>> {
        let mut tx = self.pool.begin().await?;
        access(&mut tx, id, actor, true).await?;
        let stored:Option<(Uuid,String,Option<String>,String,i64)>=sqlx::query_as("SELECT sheet_id,actor_id,title,revision_id,sequence FROM native_sheet_checkpoints WHERE id=$1")
            .bind(key).fetch_optional(&mut *tx).await?;
        let receipt = if let Some((sheet, owner, label, revision, sequence)) = stored {
            if sheet != id
                || owner != actor
                || label.as_deref() != Some(title.trim())
                || revision != expected
            {
                return Err(SheetError::Conflict(
                    "checkpoint ID reused for different input".into(),
                ));
            }
            Some(json!({"id":key,"revision_id":revision,"sequence":sequence}))
        } else {
            None
        };
        tx.commit().await?;
        Ok(receipt)
    }
}
