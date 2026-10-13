//! Comments on sheets: a thread on one cell of one tab, its replies, mentions, and resolving.
//! Anyone who can open a sheet reads its threads; whoever may comment on it or edit it takes part.
//! A comment is plain text, and names the people it mentions so they hear about it in For you.
use crate::sheets::{access, level_rank, SheetError};
use crate::OrgIntel;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

type Result<T> = std::result::Result<T, SheetError>;

pub const MAX_SHEET_COMMENT_CHARS: usize = 8000;
const MAX_MENTIONS: usize = 32;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SheetComment {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub author_actor_id: String,
    pub body: String,
    pub mentioned_actor_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct SheetCommentThread {
    pub id: Uuid,
    /// The tab, by the grid's own id for it.
    pub worksheet_id: String,
    /// The cell, as the grid names it: `B3`.
    pub cell: String,
    pub created_by_actor_id: String,
    pub created_at: DateTime<Utc>,
    pub resolved_by_actor_id: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    #[sqlx(skip)]
    pub comments: Vec<SheetComment>,
}

/// A sheet's threads for one reader, and whether they may take part.
#[derive(Debug, Clone, Serialize)]
pub struct SheetComments {
    pub threads: Vec<SheetCommentThread>,
    pub may_comment: bool,
}

const COMMENT: &str = "id,thread_id,author_actor_id,body,mentioned_actor_ids,created_at,edited_at";
const THREAD: &str =
    "id,worksheet_id,cell,created_by_actor_id,created_at,resolved_by_actor_id,resolved_at";

fn cell_name(cell: &str) -> Result<String> {
    let cell = cell.trim().to_ascii_uppercase();
    let letters = cell.bytes().take_while(u8::is_ascii_uppercase).count();
    let digits = &cell[letters..];
    let valid = (1..=3).contains(&letters)
        && (1..=7).contains(&digits.len())
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && !digits.starts_with('0');
    if !valid {
        return Err(SheetError::Invalid("choose one cell to comment on".into()));
    }
    Ok(cell)
}

fn comment_body(body: &str) -> Result<String> {
    let body = body.trim();
    if body.is_empty() {
        return Err(SheetError::Invalid("write a comment first".into()));
    }
    if body.chars().count() > MAX_SHEET_COMMENT_CHARS {
        return Err(SheetError::Invalid(format!(
            "a comment holds up to {MAX_SHEET_COMMENT_CHARS} characters"
        )));
    }
    Ok(body.to_string())
}

/// Whoever may comment on the sheet or edit it.
async fn may_comment(tx: &mut Transaction<'_, Postgres>, id: Uuid, actor: &str) -> Result<()> {
    let grant = access(tx, id, actor, false).await?;
    if level_rank(&grant) < level_rank("comment") {
        return Err(SheetError::Unavailable);
    }
    Ok(())
}

/// Each person mentioned, once, in the order named. Everyone mentioned must be able to open the
/// sheet: a mention is never how someone learns of a sheet they cannot see.
async fn mentions(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    actor: &str,
    named: &[String],
) -> Result<Vec<String>> {
    let mut kept: Vec<String> = Vec::new();
    for person in named {
        if person != actor && !kept.contains(person) {
            kept.push(person.clone());
        }
    }
    if kept.len() > MAX_MENTIONS {
        return Err(SheetError::Invalid(format!(
            "mention up to {MAX_MENTIONS} people at once"
        )));
    }
    for person in &kept {
        match access(tx, id, person, false).await {
            Ok(_) => {}
            Err(SheetError::Unavailable) => {
                return Err(SheetError::Invalid(
                    "someone you mentioned cannot open this sheet; share it with them first".into(),
                ))
            }
            Err(error) => return Err(error),
        }
    }
    Ok(kept)
}

async fn thread_of(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    thread: Uuid,
) -> Result<SheetCommentThread> {
    sqlx::query_as(&format!(
        "SELECT {THREAD} FROM native_sheet_comment_threads WHERE sheet_id=$1 AND id=$2"
    ))
    .bind(id)
    .bind(thread)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(SheetError::Unavailable)
}

impl OrgIntel {
    /// Every thread on the sheet with a comment left in it, newest first, open and resolved.
    pub async fn sheet_comments(&self, id: Uuid, actor: &str) -> Result<SheetComments> {
        let mut tx = self.pool.begin().await?;
        let grant = access(&mut tx, id, actor, false).await?;
        let mut threads: Vec<SheetCommentThread> = sqlx::query_as(&format!(
            "SELECT {THREAD} FROM native_sheet_comment_threads WHERE sheet_id=$1 \
             ORDER BY created_at DESC,id DESC LIMIT 500"
        ))
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        let comments: Vec<SheetComment> = sqlx::query_as(&format!(
            "SELECT {COMMENT} FROM native_sheet_comments \
             WHERE sheet_id=$1 AND deleted_at IS NULL ORDER BY created_at,id"
        ))
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        for comment in comments {
            if let Some(thread) = threads.iter_mut().find(|t| t.id == comment.thread_id) {
                thread.comments.push(comment);
            }
        }
        threads.retain(|thread| !thread.comments.is_empty());
        Ok(SheetComments {
            threads,
            may_comment: level_rank(&grant) >= level_rank("comment"),
        })
    }

    /// Start a thread on a cell. The thread's id comes from the caller, so a retry after a lost
    /// answer finds the thread the first attempt made.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_sheet_comment_thread(
        &self,
        id: Uuid,
        actor: &str,
        thread: Uuid,
        worksheet_id: &str,
        cell: &str,
        body: &str,
        mentioned: &[String],
    ) -> Result<SheetCommentThread> {
        let cell = cell_name(cell)?;
        let body = comment_body(body)?;
        let worksheet_id = worksheet_id.trim();
        if worksheet_id.is_empty() || worksheet_id.chars().count() > 128 {
            return Err(SheetError::Invalid("choose a tab to comment on".into()));
        }
        let mut tx = self.pool.begin().await?;
        may_comment(&mut tx, id, actor).await?;
        let mentioned = mentions(&mut tx, id, actor, mentioned).await?;
        let made = sqlx::query(
            "INSERT INTO native_sheet_comment_threads (id,sheet_id,worksheet_id,cell,created_by_actor_id) \
             VALUES ($1,$2,$3,$4,$5) ON CONFLICT (id) DO NOTHING",
        )
        .bind(thread)
        .bind(id)
        .bind(worksheet_id)
        .bind(&cell)
        .bind(actor)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if made == 1 {
            sqlx::query(
                "INSERT INTO native_sheet_comments (id,sheet_id,thread_id,author_actor_id,body,mentioned_actor_ids) \
                 VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(thread)
            .bind(id)
            .bind(thread)
            .bind(actor)
            .bind(&body)
            .bind(&mentioned)
            .execute(&mut *tx)
            .await?;
        }
        let mut view = thread_of(&mut tx, id, thread)
            .await
            .map_err(|_| SheetError::Conflict("thread id reused".into()))?;
        if view.created_by_actor_id != actor
            || view.cell != cell
            || view.worksheet_id != worksheet_id
        {
            return Err(SheetError::Conflict("thread id reused".into()));
        }
        view.comments = sqlx::query_as(&format!(
            "SELECT {COMMENT} FROM native_sheet_comments WHERE thread_id=$1 AND deleted_at IS NULL \
             ORDER BY created_at,id"
        ))
        .bind(thread)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(view)
    }

    /// Reply in a thread. Replying to a resolved thread opens it again, as on documents.
    pub async fn reply_to_sheet_comment(
        &self,
        id: Uuid,
        actor: &str,
        thread: Uuid,
        comment: Uuid,
        body: &str,
        mentioned: &[String],
    ) -> Result<SheetComment> {
        let body = comment_body(body)?;
        let mut tx = self.pool.begin().await?;
        may_comment(&mut tx, id, actor).await?;
        thread_of(&mut tx, id, thread).await?;
        let mentioned = mentions(&mut tx, id, actor, mentioned).await?;
        let made = sqlx::query(
            "INSERT INTO native_sheet_comments (id,sheet_id,thread_id,author_actor_id,body,mentioned_actor_ids) \
             VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (id) DO NOTHING",
        )
        .bind(comment)
        .bind(id)
        .bind(thread)
        .bind(actor)
        .bind(&body)
        .bind(&mentioned)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if made == 1 {
            sqlx::query(
                "UPDATE native_sheet_comment_threads SET resolved_at=NULL,resolved_by_actor_id=NULL \
                 WHERE id=$1 AND resolved_at IS NOT NULL",
            )
            .bind(thread)
            .execute(&mut *tx)
            .await?;
        }
        let stored: SheetComment = sqlx::query_as(&format!(
            "SELECT {COMMENT} FROM native_sheet_comments WHERE id=$1 AND sheet_id=$2 \
               AND thread_id=$3 AND author_actor_id=$4"
        ))
        .bind(comment)
        .bind(id)
        .bind(thread)
        .bind(actor)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| SheetError::Conflict("comment id reused".into()))?;
        tx.commit().await?;
        Ok(stored)
    }

    /// Resolve a thread, or open it again. Anyone who may comment may.
    pub async fn set_sheet_comment_thread_resolved(
        &self,
        id: Uuid,
        actor: &str,
        thread: Uuid,
        resolved: bool,
    ) -> Result<SheetCommentThread> {
        let mut tx = self.pool.begin().await?;
        may_comment(&mut tx, id, actor).await?;
        thread_of(&mut tx, id, thread).await?;
        if resolved {
            sqlx::query(
                "UPDATE native_sheet_comment_threads SET resolved_at=now(),resolved_by_actor_id=$2 \
                 WHERE id=$1 AND resolved_at IS NULL",
            )
            .bind(thread)
            .bind(actor)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query(
                "UPDATE native_sheet_comment_threads SET resolved_at=NULL,resolved_by_actor_id=NULL \
                 WHERE id=$1",
            )
            .bind(thread)
            .execute(&mut *tx)
            .await?;
        }
        let view = thread_of(&mut tx, id, thread).await?;
        tx.commit().await?;
        Ok(view)
    }

    /// Change one's own comment. Whoever it newly mentions hears about it; whoever it already
    /// mentioned does not hear twice.
    pub async fn edit_sheet_comment(
        &self,
        id: Uuid,
        actor: &str,
        comment: Uuid,
        body: &str,
        mentioned: &[String],
    ) -> Result<SheetComment> {
        let body = comment_body(body)?;
        let mut tx = self.pool.begin().await?;
        may_comment(&mut tx, id, actor).await?;
        let mentioned = mentions(&mut tx, id, actor, mentioned).await?;
        let stored: SheetComment = sqlx::query_as(&format!(
            "UPDATE native_sheet_comments SET body=$4,mentioned_actor_ids=$5,edited_at=now() \
             WHERE id=$1 AND sheet_id=$2 AND author_actor_id=$3 AND deleted_at IS NULL \
             RETURNING {COMMENT}"
        ))
        .bind(comment)
        .bind(id)
        .bind(actor)
        .bind(&body)
        .bind(&mentioned)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(SheetError::Unavailable)?;
        tx.commit().await?;
        Ok(stored)
    }

    /// Take back one's own comment. It goes for everyone; a thread with nothing left goes too.
    pub async fn delete_sheet_comment(&self, id: Uuid, actor: &str, comment: Uuid) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        access(&mut tx, id, actor, false).await?;
        let changed = sqlx::query(
            "UPDATE native_sheet_comments SET deleted_at=now() \
             WHERE id=$1 AND sheet_id=$2 AND author_actor_id=$3 AND deleted_at IS NULL",
        )
        .bind(comment)
        .bind(id)
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
}
