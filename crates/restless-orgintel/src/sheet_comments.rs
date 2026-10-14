//! Comments on sheets: a thread on a cell or range of one tab, its replies, mentions, and
//! resolving. A thread follows its cells when rows or columns are inserted or deleted.
//! Anyone who can open a sheet reads its threads; whoever may comment on it or edit it takes part.
//! A comment is plain text, and names the people it mentions so they hear about it in For you.
use crate::sheets::{access, level_rank, SheetError};
use crate::OrgIntel;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
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
    /// The cell or range, as the grid names it: `B3` or `B3:D3`.
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

/// A cell as the grid names it (`B3`), as 0-based column and row.
fn parse_cell(cell: &str) -> Option<(u32, u32)> {
    let letters = cell.bytes().take_while(u8::is_ascii_uppercase).count();
    let digits = &cell[letters..];
    let valid = (1..=3).contains(&letters)
        && (1..=7).contains(&digits.len())
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && !digits.starts_with('0');
    if !valid {
        return None;
    }
    let col = cell[..letters]
        .bytes()
        .fold(0u32, |col, letter| col * 26 + u32::from(letter - b'A' + 1));
    Some((col - 1, digits.parse::<u32>().ok()? - 1))
}

fn format_cell((col, row): (u32, u32)) -> String {
    let mut letters = Vec::new();
    let mut rest = col + 1;
    while rest > 0 {
        letters.push(b'A' + ((rest - 1) % 26) as u8);
        rest = (rest - 1) / 26;
    }
    letters.reverse();
    format!(
        "{}{}",
        String::from_utf8(letters).unwrap_or_default(),
        row + 1
    )
}

/// The corners of a thread's cell or range: top left, bottom right.
fn parse_anchor(anchor: &str) -> Option<((u32, u32), (u32, u32))> {
    let (start, end) = anchor.split_once(':').unwrap_or((anchor, anchor));
    let (a, b) = (parse_cell(start)?, parse_cell(end)?);
    Some(((a.0.min(b.0), a.1.min(b.1)), (a.0.max(b.0), a.1.max(b.1))))
}

fn format_anchor((start, end): ((u32, u32), (u32, u32))) -> String {
    if start == end {
        format_cell(start)
    } else {
        format!("{}:{}", format_cell(start), format_cell(end))
    }
}

/// A cell or a range, written top left to bottom right.
fn cell_name(cell: &str) -> Result<String> {
    let cell = cell.trim().to_ascii_uppercase();
    let corners = parse_anchor(&cell)
        .filter(|(_, (col, row))| *col < 18_278 && *row < 9_999_999)
        .ok_or_else(|| SheetError::Invalid("choose a cell or range to comment on".into()))?;
    Ok(format_anchor(corners))
}

/// Rows or columns inserted or deleted on one tab, as the grid's commands say.
#[derive(Debug, Clone, PartialEq)]
enum Shift {
    Insert {
        tab: String,
        rows: bool,
        at: u32,
        count: u32,
    },
    Remove {
        tab: String,
        rows: bool,
        elements: Vec<u32>,
    },
}

fn shifts_of(commands: &Value) -> Vec<Shift> {
    let mut shifts = Vec::new();
    for command in commands.as_array().into_iter().flatten() {
        let tab = command["sheetId"].as_str().unwrap_or_default().to_string();
        let rows = command["dimension"] == "ROW";
        match command["type"].as_str() {
            Some("ADD_COLUMNS_ROWS") => {
                let (Some(base), Some(count)) =
                    (command["base"].as_u64(), command["quantity"].as_u64())
                else {
                    continue;
                };
                let at = base + u64::from(command["position"] == "after");
                if let (Ok(at), Ok(count)) = (u32::try_from(at), u32::try_from(count)) {
                    shifts.push(Shift::Insert {
                        tab,
                        rows,
                        at,
                        count,
                    });
                }
            }
            Some("REMOVE_COLUMNS_ROWS") => {
                let mut elements: Vec<u32> = command["elements"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|element| u32::try_from(element.as_u64()?).ok())
                    .collect();
                elements.sort_unstable();
                elements.dedup();
                if !elements.is_empty() {
                    shifts.push(Shift::Remove {
                        tab,
                        rows,
                        elements,
                    });
                }
            }
            _ => {}
        }
    }
    shifts
}

/// What undoing these shifts does, in the order it happens.
fn inverse(shifts: Vec<Shift>) -> Vec<Shift> {
    let mut out = Vec::new();
    for shift in shifts.into_iter().rev() {
        match shift {
            Shift::Insert {
                tab,
                rows,
                at,
                count,
            } => out.push(Shift::Remove {
                tab,
                rows,
                elements: (at..at.saturating_add(count)).collect(),
            }),
            Shift::Remove {
                tab,
                rows,
                elements,
            } => {
                for at in elements {
                    out.push(Shift::Insert {
                        tab: tab.clone(),
                        rows,
                        at,
                        count: 1,
                    });
                }
            }
        }
    }
    out
}

/// Where an anchor's cells are after a shift. A range grows over rows inserted inside it and
/// shrinks over rows deleted from it; a cell whose row is deleted moves to the row that takes
/// its place, so the thread stays near what it was about.
fn shifted(anchor: ((u32, u32), (u32, u32)), tab: &str, shift: &Shift) -> ((u32, u32), (u32, u32)) {
    let ((mut c0, mut r0), (mut c1, mut r1)) = anchor;
    match shift {
        Shift::Insert {
            tab: on,
            rows,
            at,
            count,
        } if on == tab => {
            let (start, end) = if *rows {
                (&mut r0, &mut r1)
            } else {
                (&mut c0, &mut c1)
            };
            if *start >= *at {
                *start = start.saturating_add(*count);
            }
            if *end >= *at {
                *end = end.saturating_add(*count);
            }
        }
        Shift::Remove {
            tab: on,
            rows,
            elements,
        } if on == tab => {
            let (start, end) = if *rows {
                (&mut r0, &mut r1)
            } else {
                (&mut c0, &mut c1)
            };
            let before = |index: u32| elements.iter().filter(|e| **e < index).count() as u32;
            let new_start = *start - before(*start);
            let new_end = if elements.binary_search(end).is_ok() {
                end.saturating_sub(before(*end) + 1)
            } else {
                *end - before(*end)
            };
            *start = new_start;
            *end = new_end.max(new_start);
        }
        _ => {}
    }
    ((c0, r0), (c1, r1))
}

/// Threads follow the rows and columns a change inserts or deletes, as formulas do. Called for
/// each message as it enters the sheet's log, in the same transaction.
pub(crate) async fn follow_structure(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    message: &Value,
) -> Result<()> {
    let original = |field: &str| message[field].as_str().map(str::to_string);
    let shifts = match message["type"].as_str() {
        Some("REMOTE_REVISION") => shifts_of(&message["commands"]),
        Some(kind @ ("REVISION_UNDONE" | "REVISION_REDONE")) => {
            let field = if kind == "REVISION_UNDONE" {
                "undoneRevisionId"
            } else {
                "redoneRevisionId"
            };
            let Some(target) = original(field) else {
                return Ok(());
            };
            let stored: Option<Value> = sqlx::query_scalar(
                "SELECT message FROM native_sheet_messages WHERE sheet_id=$1 AND revision_id=$2",
            )
            .bind(id)
            .bind(target)
            .fetch_optional(&mut **tx)
            .await?;
            let shifts = stored
                .map(|m| shifts_of(&m["commands"]))
                .unwrap_or_default();
            if kind == "REVISION_UNDONE" {
                inverse(shifts)
            } else {
                shifts
            }
        }
        _ => Vec::new(),
    };
    if shifts.is_empty() {
        return Ok(());
    }
    let threads: Vec<(Uuid, String, String)> = sqlx::query_as(
        "SELECT id,worksheet_id,cell FROM native_sheet_comment_threads WHERE sheet_id=$1",
    )
    .bind(id)
    .fetch_all(&mut **tx)
    .await?;
    for (thread, tab, cell) in threads {
        let Some(anchor) = parse_anchor(&cell) else {
            continue;
        };
        let moved = shifts
            .iter()
            .fold(anchor, |at, shift| shifted(at, &tab, shift));
        let ((_, _), (col, row)) = moved;
        if moved == anchor || col >= 18_278 || row >= 9_999_999 {
            continue;
        }
        sqlx::query("UPDATE native_sheet_comment_threads SET cell=$3 WHERE sheet_id=$1 AND id=$2")
            .bind(id)
            .bind(thread)
            .bind(format_anchor(moved))
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn moved(anchor: &str, commands: Value) -> String {
        let shifts = shifts_of(&commands);
        let at = parse_anchor(anchor).unwrap();
        format_anchor(
            shifts
                .iter()
                .fold(at, |at, shift| shifted(at, "tab", shift)),
        )
    }

    #[test]
    fn a_thread_follows_its_cells_when_rows_and_columns_move() {
        assert_eq!(cell_name(" d3:b1 ").unwrap(), "B1:D3");
        assert_eq!(cell_name("AA10:AA10").unwrap(), "AA10");
        assert!(cell_name("A0").is_err());
        let rows = |position: &str, base: u64, quantity: u64| json!([{"type":"ADD_COLUMNS_ROWS","sheetId":"tab","dimension":"ROW","position":position,"base":base,"quantity":quantity}]);
        // Two rows inserted above B5 carry it to B7; a row inserted below leaves it.
        assert_eq!(moved("B5", rows("before", 0, 2)), "B7");
        assert_eq!(moved("B5", rows("after", 4, 1)), "B5");
        // A range grows over rows inserted inside it.
        assert_eq!(moved("A2:C4", rows("after", 2, 3)), "A2:C7");
        // Another tab's insert moves nothing.
        assert_eq!(
            moved(
                "B5",
                json!([{"type":"ADD_COLUMNS_ROWS","sheetId":"other","dimension":"ROW","position":"before","base":0,"quantity":2}])
            ),
            "B5"
        );
        // Deleting rows above moves it up; deleting its own row leaves it on the row taking its place.
        let remove = |elements: Value| json!([{"type":"REMOVE_COLUMNS_ROWS","sheetId":"tab","dimension":"ROW","elements":elements}]);
        assert_eq!(moved("B5", remove(json!([0, 1]))), "B3");
        assert_eq!(moved("B5", remove(json!([4]))), "B5");
        assert_eq!(moved("A2:A6", remove(json!([5, 1]))), "A2:A4");
        // A column inserted before B moves B5 to C5.
        assert_eq!(
            moved(
                "B5",
                json!([{"type":"ADD_COLUMNS_ROWS","sheetId":"tab","dimension":"COL","position":"before","base":0,"quantity":1}])
            ),
            "C5"
        );
        // Undoing an insert puts the thread back.
        let at = parse_anchor("B5").unwrap();
        let there = shifts_of(&rows("before", 0, 2))
            .iter()
            .fold(at, |at, shift| shifted(at, "tab", shift));
        let back = inverse(shifts_of(&rows("before", 0, 2)))
            .iter()
            .fold(there, |at, shift| shifted(at, "tab", shift));
        assert_eq!(format_anchor(back), "B5");
    }
}
