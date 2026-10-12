//! For you (S65): what needs a person's attention beside the decisions in their Inbox, assembled
//! on every read from objects that already exist: Room and document mentions of them, comment
//! threads and Room threads they are in or on documents they follow, documents and sheets someone
//! shared with them, reviews they were asked for, and changes others made to documents and sheets
//! they follow. Nothing here is an outbox; the only new state is per-person marks (read, marked
//! unread, archived), an "all read through" cursor, explicit follow choices and who was editing
//! when a live body was saved.
//!
//! Every item is rechecked against the source's own access rule on each read, so a person who loses
//! access to a document or sheet, or leaves a Room, stops seeing what came from it.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::Row as _;
use uuid::Uuid;

use crate::{DocumentError, DocumentResult, OrgIntel};

const MAX_FOR_YOU_PAGE: i64 = 50;
/// The unread badge counts this many at most; the cockpit shows more as "99+".
const MAX_FOR_YOU_UNREAD: i64 = 100;
const MAX_FOR_YOU_READ_KEYS: usize = 100;
const MAX_INBOX_MARKS: usize = 200;
const EXCERPT_CHARS: i32 = 280;

/// Documents a person follows: an explicit choice wins, otherwise creating, editing, commenting
/// on or being mentioned in a document follows it from the first time they did. `since` bounds the
/// changes and comments that reach them, so following an old document does not replay its history.
const FOLLOWED: &str = "followed AS ( \
   SELECT d.id AS document_id,d.title,d.status, \
     LEAST( \
       CASE WHEN f.following THEN f.updated_at END, \
       CASE WHEN d.created_by_actor_id=$1 THEN d.created_at END, \
       (SELECT min(v.created_at) FROM native_document_versions v \
         WHERE v.document_id=d.id AND v.created_by_actor_id=$1), \
       (SELECT min(v.created_at) FROM native_document_version_editors e \
          JOIN native_document_versions v ON v.id=e.version_id \
         WHERE e.document_id=d.id AND e.actor_id=$1), \
       (SELECT min(c.created_at) FROM native_document_comments c \
         WHERE c.document_id=d.id AND (c.author_actor_id=$1 OR $1=ANY(c.mentioned_actor_ids))) \
     ) AS since \
   FROM native_documents d \
   LEFT JOIN native_document_follows f ON f.document_id=d.id AND f.actor_id=$1 \
   WHERE f.following IS DISTINCT FROM FALSE AND ($6::uuid IS NULL OR d.id=$6) \
 ), \
 follows AS (SELECT * FROM followed WHERE since IS NOT NULL)";

/// Sheets a person follows, the same way: an explicit choice wins, otherwise owning or editing a
/// sheet follows it from the first time they did.
const SHEETS_FOLLOWED: &str = "sheet_follows AS ( \
   SELECT s.id AS sheet_id,s.title, \
     LEAST( \
       CASE WHEN f.following THEN f.updated_at END, \
       CASE WHEN s.owner_actor_id=$1 THEN s.created_at END, \
       (SELECT min(m.accepted_at) FROM native_sheet_messages m \
         WHERE m.sheet_id=s.id AND m.actor_id=$1) \
     ) AS since \
   FROM native_sheets s \
   LEFT JOIN native_sheet_follows f ON f.sheet_id=s.id AND f.actor_id=$1 \
   WHERE f.following IS DISTINCT FROM FALSE AND s.archived_at IS NULL \
     AND ($8::uuid IS NULL OR s.id=$8) \
 )";

/// One row per item. `$1` viewer, `$2`/`$3` the (at,key) page cursor, `$4` page size, `$5` unread
/// only, `$6` one document, `$7` excerpt length, `$8` one sheet. Activity by the viewer never
/// counts; neither does an automatic save they were editing in, since they saw it happen. A save
/// of the live body (by the daemon, or first thing when someone comments, restores or asks for
/// review) is the editors' change, not the change of whoever happened to trigger the save.
///
/// `$6` and `$8` narrow the list to one document or one sheet; each kind that cannot come from
/// that source drops out.
const ITEMS: &str = "viewer AS ( \
   SELECT COALESCE((SELECT read_through FROM for_you_read_cursors WHERE actor_id=$1), \
                   '-infinity'::timestamptz) AS through \
 ), \
 room_mentions AS ( \
   SELECT 'room-mention:'||m.id AS key,'mention'::text AS kind,NULL::uuid AS document_id, \
     NULL::uuid AS sheet_id, \
     m.room_id,room.kind::text AS room_kind,m.thread_root_message_id,m.message_id,m.id AS mention_id, \
     room.title,left(message.current_plain_text,$7) AS excerpt,ARRAY[message.from_actor] AS actors, \
     m.created_at AS at, \
     (m.resolved_at IS NULL \
      AND m.created_at>GREATEST(viewer.through,COALESCE(r.read_at,'-infinity'))) AS unread \
   FROM message_mentions m CROSS JOIN viewer \
   JOIN messages message ON message.id=m.message_id AND message.deleted_at IS NULL \
   JOIN rooms room ON room.id=m.room_id AND room.archived_at IS NULL \
   JOIN room_participants p ON p.room_id=m.room_id AND p.actor_id=$1 AND p.left_at IS NULL \
   LEFT JOIN for_you_reads r ON r.actor_id=$1 AND r.item_key='room-mention:'||m.id \
   WHERE $6::uuid IS NULL AND $8::uuid IS NULL AND m.mentioned_actor_id=$1 AND m.kind='direct' \
     AND m.cancelled_at IS NULL AND message.from_actor<>$1 \
 ), \
 room_thread_rows AS ( \
   SELECT m.room_id,COALESCE(m.thread_root_message_id,m.id) AS root_id, \
     room.title,room.kind::text AS room_kind,min(m.created_at) AS joined_at \
   FROM messages m \
   JOIN rooms room ON room.id=m.room_id AND room.archived_at IS NULL AND room.kind<>'direct' \
   JOIN room_participants p ON p.room_id=m.room_id AND p.actor_id=$1 AND p.left_at IS NULL \
   WHERE $6::uuid IS NULL AND $8::uuid IS NULL AND m.from_actor=$1 AND m.deleted_at IS NULL \
   GROUP BY m.room_id,COALESCE(m.thread_root_message_id,m.id),room.title,room.kind \
 ), \
 room_threads AS ( \
   SELECT 'room-thread:'||rt.room_id||':'||rt.root_id,'reply',NULL::uuid,NULL::uuid,rt.room_id, \
     rt.room_kind,rt.root_id,latest.id,NULL::uuid,rt.title,left(latest.current_plain_text,$7), \
     COALESCE(fresh.actors,ARRAY[latest.from_actor]),latest.created_at,fresh.actors IS NOT NULL \
   FROM room_thread_rows rt CROSS JOIN viewer \
   LEFT JOIN for_you_reads r ON r.actor_id=$1 \
     AND r.item_key='room-thread:'||rt.room_id||':'||rt.root_id \
   LEFT JOIN room_read_cursors rc ON rc.room_id=rt.room_id AND rc.actor_id=$1 \
   JOIN LATERAL ( \
     SELECT m.id,m.from_actor,m.current_plain_text,m.created_at FROM messages m \
     WHERE m.room_id=rt.room_id AND m.thread_root_message_id=rt.root_id \
       AND m.from_actor<>$1 AND m.deleted_at IS NULL AND m.created_at>rt.joined_at \
       AND NOT EXISTS(SELECT 1 FROM message_mentions mm WHERE mm.message_id=m.id \
         AND mm.mentioned_actor_id=$1 AND mm.kind='direct' AND mm.cancelled_at IS NULL) \
     ORDER BY m.id DESC LIMIT 1 \
   ) latest ON TRUE \
   LEFT JOIN LATERAL ( \
     SELECT array_agg(DISTINCT m.from_actor) AS actors FROM messages m \
     WHERE m.room_id=rt.room_id AND m.thread_root_message_id=rt.root_id \
       AND m.from_actor<>$1 AND m.deleted_at IS NULL AND m.created_at>rt.joined_at \
       AND NOT EXISTS(SELECT 1 FROM message_mentions mm WHERE mm.message_id=m.id \
         AND mm.mentioned_actor_id=$1 AND mm.kind='direct' AND mm.cancelled_at IS NULL) \
       AND m.created_at>GREATEST(viewer.through,COALESCE(r.read_at,'-infinity')) \
       AND m.id>COALESCE(rc.last_read_message_id,0) \
   ) fresh ON TRUE \
 ), \
 doc_mentions AS ( \
   SELECT 'doc-mention:'||c.id,'mention',c.document_id,NULL::uuid,NULL::uuid,NULL::text, \
     NULL::bigint,NULL::bigint,NULL::uuid,d.title,left(c.plain_text,$7),ARRAY[c.author_actor_id], \
     c.created_at, \
     (c.created_at>GREATEST(viewer.through,COALESCE(r.read_at,'-infinity')) \
      AND NOT EXISTS(SELECT 1 FROM native_document_comments answer \
        WHERE answer.thread_id=c.thread_id AND answer.author_actor_id=$1 \
          AND answer.created_at>c.created_at)) \
   FROM native_document_comments c CROSS JOIN viewer \
   JOIN native_documents d ON d.id=c.document_id AND d.status<>'archived' \
   LEFT JOIN for_you_reads r ON r.actor_id=$1 AND r.item_key='doc-mention:'||c.id \
   WHERE $1=ANY(c.mentioned_actor_ids) AND c.author_actor_id<>$1 AND $8::uuid IS NULL \
     AND ($6::uuid IS NULL OR c.document_id=$6) \
 ), \
 thread_rows AS ( \
   SELECT t.id AS thread_id,t.document_id,d.title, \
     (SELECT min(c.created_at) FROM native_document_comments c \
       WHERE c.thread_id=t.id AND c.author_actor_id=$1) AS joined_at, \
     fo.since AS followed_since \
   FROM native_document_comment_threads t \
   JOIN native_documents d ON d.id=t.document_id AND d.status<>'archived' \
   LEFT JOIN follows fo ON fo.document_id=t.document_id \
   WHERE ($6::uuid IS NULL OR t.document_id=$6) AND $8::uuid IS NULL \
 ), \
 threads AS ( \
   SELECT 'thread:'||tr.thread_id, \
     CASE WHEN tr.joined_at IS NULL THEN 'comment' ELSE 'reply' END, \
     tr.document_id,NULL::uuid,NULL::uuid,NULL::text,NULL::bigint,NULL::bigint,NULL::uuid,tr.title, \
     left(latest.plain_text,$7),COALESCE(fresh.actors,ARRAY[latest.author_actor_id]), \
     latest.created_at,fresh.actors IS NOT NULL \
   FROM thread_rows tr CROSS JOIN viewer \
   LEFT JOIN for_you_reads r ON r.actor_id=$1 AND r.item_key='thread:'||tr.thread_id \
   JOIN LATERAL ( \
     SELECT c.author_actor_id,c.plain_text,c.created_at FROM native_document_comments c \
     WHERE c.thread_id=tr.thread_id AND c.author_actor_id<>$1 \
       AND NOT ($1=ANY(c.mentioned_actor_ids)) \
       AND c.created_at>LEAST(tr.joined_at,tr.followed_since) \
     ORDER BY c.created_at DESC,c.id DESC LIMIT 1 \
   ) latest ON TRUE \
   LEFT JOIN LATERAL ( \
     SELECT array_agg(DISTINCT c.author_actor_id) AS actors FROM native_document_comments c \
     WHERE c.thread_id=tr.thread_id AND c.author_actor_id<>$1 \
       AND NOT ($1=ANY(c.mentioned_actor_ids)) \
       AND c.created_at>LEAST(tr.joined_at,tr.followed_since) \
       AND c.created_at>GREATEST(viewer.through,COALESCE(r.read_at,'-infinity')) \
   ) fresh ON TRUE \
   WHERE tr.joined_at IS NOT NULL OR tr.followed_since IS NOT NULL \
 ), \
 doc_shares AS ( \
   SELECT 'share:'||p.document_id,'share',p.document_id,NULL::uuid,NULL::uuid,NULL::text, \
     NULL::bigint,NULL::bigint,NULL::uuid,d.title, \
     CASE p.access::text WHEN 'edit' THEN 'Can edit' WHEN 'comment' THEN 'Can comment' \
       ELSE 'Can view' END,ARRAY[p.added_by_actor_id],p.added_at, \
     p.added_at>GREATEST(viewer.through,COALESCE(r.read_at,'-infinity')) \
   FROM native_document_participants p CROSS JOIN viewer \
   JOIN native_documents d ON d.id=p.document_id AND d.status<>'archived' \
   LEFT JOIN for_you_reads r ON r.actor_id=$1 AND r.item_key='share:'||p.document_id \
   WHERE p.actor_id=$1 AND p.removed_at IS NULL AND p.added_by_actor_id<>$1 \
     AND d.owner_actor_id<>$1 AND $8::uuid IS NULL AND ($6::uuid IS NULL OR p.document_id=$6) \
 ), \
 sheet_shares AS ( \
   SELECT 'sheet-share:'||p.sheet_id,'share',NULL::uuid,p.sheet_id,NULL::uuid,NULL::text, \
     NULL::bigint,NULL::bigint,NULL::uuid,s.title, \
     CASE p.access WHEN 'edit' THEN 'Can edit' ELSE 'Can view' END, \
     ARRAY[p.added_by_actor_id],p.added_at, \
     p.added_at>GREATEST(viewer.through,COALESCE(r.read_at,'-infinity')) \
   FROM native_sheet_participants p CROSS JOIN viewer \
   JOIN native_sheets s ON s.id=p.sheet_id AND s.archived_at IS NULL \
   LEFT JOIN for_you_reads r ON r.actor_id=$1 AND r.item_key='sheet-share:'||p.sheet_id \
   WHERE p.actor_id=$1 AND p.added_at IS NOT NULL AND p.added_by_actor_id<>$1 \
     AND s.owner_actor_id<>$1 AND $6::uuid IS NULL AND ($8::uuid IS NULL OR p.sheet_id=$8) \
 ), \
 reviews AS ( \
   SELECT 'review:'||w.review_id,'review',w.document_id,NULL::uuid,NULL::uuid,NULL::text, \
     NULL::bigint,NULL::bigint,NULL::uuid,d.title,left(review.summary,$7), \
     ARRAY[review.requested_by_actor_id],w.created_at, \
     w.created_at>GREATEST(viewer.through,COALESCE(r.read_at,'-infinity')) \
   FROM native_document_work_review_dependencies w CROSS JOIN viewer \
   JOIN native_document_reviews review ON review.id=w.review_id AND review.status='requested' \
   JOIN native_documents d ON d.id=w.document_id AND d.status<>'archived' \
   LEFT JOIN for_you_reads r ON r.actor_id=$1 AND r.item_key='review:'||w.review_id \
   WHERE w.reviewer_actor_id=$1 AND w.status='pending' AND review.requested_by_actor_id<>$1 \
     AND $8::uuid IS NULL AND ($6::uuid IS NULL OR w.document_id=$6) \
     AND $1<>COALESCE((SELECT b.actor_id FROM human_principal_actor_bindings b \
       WHERE b.membership_role='owner' AND b.membership_status='active' \
       ORDER BY b.first_verified_at ASC LIMIT 1),'owner') \
 ), \
 change_rows AS ( \
   SELECT v.document_id,v.created_at,v.created_by_actor_id AS actor \
   FROM native_document_versions v JOIN follows fo ON fo.document_id=v.document_id \
   WHERE v.version_number>1 AND v.created_at>fo.since \
     AND v.created_by_actor_id<>$1 AND v.created_by_actor_id<>'daemon' \
     AND NOT EXISTS(SELECT 1 FROM native_document_version_editors e WHERE e.version_id=v.id) \
   UNION ALL \
   SELECT v.document_id,v.created_at,e.actor_id \
   FROM native_document_versions v JOIN follows fo ON fo.document_id=v.document_id \
   JOIN native_document_version_editors e ON e.version_id=v.id \
   WHERE v.created_at>fo.since AND e.actor_id<>$1 \
     AND NOT EXISTS(SELECT 1 FROM native_document_version_editors own \
       WHERE own.version_id=v.id AND own.actor_id=$1) \
 ), \
 changes AS ( \
   SELECT 'changes:'||fo.document_id,'change',fo.document_id,NULL::uuid,NULL::uuid,NULL::text, \
     NULL::bigint,NULL::bigint,NULL::uuid,fo.title,''::text,COALESCE(fresh.actors,latest.actors), \
     last.at,fresh.actors IS NOT NULL \
   FROM follows fo CROSS JOIN viewer \
   LEFT JOIN for_you_reads r ON r.actor_id=$1 AND r.item_key='changes:'||fo.document_id \
   JOIN LATERAL (SELECT max(cr.created_at) AS at FROM change_rows cr \
     WHERE cr.document_id=fo.document_id) last ON last.at IS NOT NULL \
   CROSS JOIN LATERAL (SELECT array_agg(DISTINCT cr.actor) AS actors FROM change_rows cr \
     WHERE cr.document_id=fo.document_id AND cr.created_at=last.at) latest \
   LEFT JOIN LATERAL (SELECT array_agg(DISTINCT cr.actor) AS actors FROM change_rows cr \
     WHERE cr.document_id=fo.document_id \
       AND cr.created_at>GREATEST(viewer.through,COALESCE(r.read_at,'-infinity'))) fresh ON TRUE \
   WHERE fo.status<>'archived' AND $8::uuid IS NULL \
 ), \
 sheet_changes AS ( \
   SELECT 'sheet-changes:'||sf.sheet_id,'change',NULL::uuid,sf.sheet_id,NULL::uuid,NULL::text, \
     NULL::bigint,NULL::bigint,NULL::uuid,sf.title,''::text,COALESCE(fresh.actors,latest.actors), \
     last.at,fresh.actors IS NOT NULL \
   FROM sheet_follows sf CROSS JOIN viewer \
   LEFT JOIN for_you_reads r ON r.actor_id=$1 AND r.item_key='sheet-changes:'||sf.sheet_id \
   JOIN LATERAL (SELECT max(m.accepted_at) AS at FROM native_sheet_messages m \
     WHERE m.sheet_id=sf.sheet_id AND m.actor_id<>$1 AND m.accepted_at>sf.since) last \
     ON last.at IS NOT NULL \
   CROSS JOIN LATERAL (SELECT array_agg(DISTINCT m.actor_id) AS actors \
     FROM native_sheet_messages m \
     WHERE m.sheet_id=sf.sheet_id AND m.actor_id<>$1 AND m.accepted_at=last.at) latest \
   LEFT JOIN LATERAL (SELECT array_agg(DISTINCT m.actor_id) AS actors \
     FROM native_sheet_messages m \
     WHERE m.sheet_id=sf.sheet_id AND m.actor_id<>$1 AND m.accepted_at>sf.since \
       AND m.accepted_at>GREATEST(viewer.through,COALESCE(r.read_at,'-infinity'))) fresh ON TRUE \
   WHERE sf.since IS NOT NULL AND $6::uuid IS NULL \
 ), \
 items AS ( \
   SELECT * FROM room_mentions UNION ALL SELECT * FROM room_threads \
   UNION ALL SELECT * FROM doc_mentions UNION ALL SELECT * FROM threads \
   UNION ALL SELECT * FROM doc_shares UNION ALL SELECT * FROM sheet_shares \
   UNION ALL SELECT * FROM reviews UNION ALL SELECT * FROM changes \
   UNION ALL SELECT * FROM sheet_changes \
 )";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum ForYouTarget {
    Document {
        document_id: Uuid,
    },
    Sheet {
        sheet_id: Uuid,
    },
    Room {
        room_id: Uuid,
        room_kind: String,
        thread_root_message_id: i64,
        message_id: i64,
        /// The mention this item is, or none for a reply in a thread the person is in.
        mention_id: Option<Uuid>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct ForYouItem {
    /// Stable per source object; the read marks use it.
    pub key: String,
    /// `mention`, `reply` (a document or Room thread the person is in), `comment` (a new thread on
    /// a document they follow), `share` (someone gave them a document or sheet), `review` (they
    /// were asked to review a document) or `change` (others changed something they follow).
    pub kind: String,
    /// The source's own title: the document, sheet or Room.
    pub title: String,
    /// What was said, or for a share the access it gave.
    pub excerpt: String,
    pub target: ForYouTarget,
    /// Who acted since the person last read the item, or the latest actor when it is read.
    pub actor_ids: Vec<String>,
    pub actor_names: Vec<String>,
    pub at: DateTime<Utc>,
    pub unread: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForYouCursor {
    pub at: DateTime<Utc>,
    pub key: String,
}

impl ForYouCursor {
    pub fn encode(&self) -> String {
        format!("{}~{}", self.at.timestamp_micros(), self.key)
    }

    pub fn parse(value: &str) -> Option<Self> {
        let (micros, key) = value.split_once('~')?;
        let at = DateTime::from_timestamp_micros(micros.parse().ok()?)?;
        valid_key(key).then(|| Self {
            at,
            key: key.to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ForYouPage {
    pub items: Vec<ForYouItem>,
    pub unread_count: i64,
    pub next_before: Option<ForYouCursor>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentChangeNotice {
    pub actor_ids: Vec<String>,
    pub actor_names: Vec<String>,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentFollowState {
    pub following: bool,
    /// Others changed it since this person last opened it.
    pub changed_since_you_looked: Option<DocumentChangeNotice>,
}

/// A person's own mark on one owner Inbox item: the brief they last opened, and until when it is
/// snoozed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
pub struct InboxMark {
    pub item_id: String,
    pub seen_stamp: Option<String>,
    pub snoozed_until: Option<DateTime<Utc>>,
}

/// A mark names one source object by kind and id; anything else is refused. A Room thread is
/// named by its Room and root message.
fn valid_key(key: &str) -> bool {
    let Some((kind, id)) = key.split_once(':') else {
        return false;
    };
    let canonical = |id: &str| Uuid::parse_str(id).is_ok_and(|parsed| parsed.hyphenated().to_string() == id);
    match kind {
        "room-mention" | "doc-mention" | "thread" | "changes" | "share" | "sheet-share"
        | "review" | "sheet-changes" => canonical(id),
        "room-thread" => id.split_once(':').is_some_and(|(room, root)| {
            canonical(room)
                && !root.is_empty()
                && root.len() <= 19
                && root.bytes().all(|byte| byte.is_ascii_digit())
                && !root.starts_with('0')
        }),
        _ => false,
    }
}

fn checked_keys(keys: &[String]) -> DocumentResult<()> {
    if keys.is_empty() || keys.len() > MAX_FOR_YOU_READ_KEYS {
        return Err(DocumentError::Invalid(format!(
            "mark between 1 and {MAX_FOR_YOU_READ_KEYS} items at once"
        )));
    }
    if let Some(key) = keys.iter().find(|key| !valid_key(key)) {
        return Err(DocumentError::Invalid(format!("unknown For you item {key:?}")));
    }
    Ok(())
}

/// Narrows the list to one document or one sheet.
#[derive(Debug, Clone, Copy, Default)]
struct Source {
    document: Option<Uuid>,
    sheet: Option<Uuid>,
}

impl OrgIntel {
    async fn for_you_rows(
        &self,
        viewer: &str,
        before: Option<&ForYouCursor>,
        limit: i64,
        unread_only: bool,
        source: Source,
    ) -> DocumentResult<Vec<ForYouItem>> {
        // A person's own mark can make an item unread again or archive it until something new
        // happens there.
        let rows = sqlx::query(&format!(
            "WITH {FOLLOWED}, {SHEETS_FOLLOWED}, {ITEMS}, \
             marked AS ( \
               SELECT items.key,items.kind,items.document_id,items.sheet_id,items.room_id, \
                 items.room_kind,items.thread_root_message_id,items.message_id,items.mention_id, \
                 items.title,items.excerpt,items.actors,items.at, \
                 (items.unread OR COALESCE(mark.marked_unread,false)) AS unread \
               FROM items \
               LEFT JOIN for_you_reads mark ON mark.actor_id=$1 AND mark.item_key=items.key \
               WHERE mark.archived_at IS NULL OR items.at>mark.archived_at \
             ) \
             SELECT marked.*, ARRAY(SELECT COALESCE(NULLIF(btrim(a.display),''),a.id) \
                 FROM unnest(marked.actors) WITH ORDINALITY u(id,n) JOIN actors a ON a.id=u.id \
                 ORDER BY u.n) AS actor_names \
             FROM marked \
             WHERE ($2::timestamptz IS NULL OR (marked.at,marked.key)<($2,$3)) \
               AND (NOT $5 OR marked.unread) \
             ORDER BY marked.at DESC,marked.key DESC LIMIT $4"
        ))
        .bind(viewer)
        .bind(before.map(|cursor| cursor.at))
        .bind(before.map(|cursor| cursor.key.as_str()).unwrap_or(""))
        .bind(limit)
        .bind(unread_only)
        .bind(source.document)
        .bind(EXCERPT_CHARS)
        .bind(source.sheet)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                let document_id: Option<Uuid> = row.try_get("document_id")?;
                let sheet_id: Option<Uuid> = row.try_get("sheet_id")?;
                let target = match (document_id, sheet_id) {
                    (Some(document_id), _) => ForYouTarget::Document { document_id },
                    (None, Some(sheet_id)) => ForYouTarget::Sheet { sheet_id },
                    (None, None) => ForYouTarget::Room {
                        room_id: row.try_get("room_id")?,
                        room_kind: row.try_get("room_kind")?,
                        thread_root_message_id: row.try_get("thread_root_message_id")?,
                        message_id: row.try_get("message_id")?,
                        mention_id: row.try_get("mention_id")?,
                    },
                };
                Ok(ForYouItem {
                    key: row.try_get("key")?,
                    kind: row.try_get("kind")?,
                    title: row.try_get("title")?,
                    excerpt: row.try_get::<Option<String>, _>("excerpt")?.unwrap_or_default(),
                    target,
                    actor_ids: row.try_get("actors")?,
                    actor_names: row.try_get("actor_names")?,
                    at: row.try_get("at")?,
                    unread: row.try_get("unread")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(DocumentError::from)
    }

    /// Keeps the items whose source the viewer may still read. Room items are already limited to
    /// Rooms the viewer is in; documents and sheets answer to their own access rule, asked once
    /// each.
    async fn readable_for_you(
        &self,
        viewer: &str,
        items: Vec<ForYouItem>,
        readable: &mut HashMap<Uuid, bool>,
    ) -> DocumentResult<Vec<ForYouItem>> {
        let mut kept = Vec::with_capacity(items.len());
        for item in items {
            let allowed = match item.target {
                ForYouTarget::Document { document_id } => match readable.get(&document_id) {
                    Some(allowed) => *allowed,
                    None => {
                        let allowed = self
                            .document_access_for_actor(document_id, viewer)
                            .await?
                            .is_some();
                        readable.insert(document_id, allowed);
                        allowed
                    }
                },
                ForYouTarget::Sheet { sheet_id } => match readable.get(&sheet_id) {
                    Some(allowed) => *allowed,
                    None => {
                        let allowed = self
                            .sheet_readable(sheet_id, viewer)
                            .await
                            .map_err(|error| DocumentError::Conflict(error.to_string()))?;
                        readable.insert(sheet_id, allowed);
                        allowed
                    }
                },
                ForYouTarget::Room { .. } => true,
            };
            if allowed {
                kept.push(item);
            }
        }
        Ok(kept)
    }

    /// One page of For you, newest first, plus how many items are unread.
    pub async fn for_you_page(
        &self,
        viewer: &str,
        before: Option<&ForYouCursor>,
        limit: i64,
    ) -> DocumentResult<ForYouPage> {
        self.for_you_page_filtered(viewer, before, limit, false)
            .await
    }

    /// One page of For you, or of only its unread items, newest first, plus how many are unread.
    pub async fn for_you_page_filtered(
        &self,
        viewer: &str,
        before: Option<&ForYouCursor>,
        limit: i64,
        unread_only: bool,
    ) -> DocumentResult<ForYouPage> {
        if !(1..=MAX_FOR_YOU_PAGE).contains(&limit) {
            return Err(DocumentError::Invalid(format!(
                "limit must be between 1 and {MAX_FOR_YOU_PAGE}"
            )));
        }
        let mut readable = HashMap::new();
        let mut items = Vec::new();
        let mut cursor = before.cloned();
        let mut exhausted = false;
        // Items whose source the viewer can no longer read are skipped, so a page may need a
        // second look further back to fill.
        for _ in 0..4 {
            let rows = self
                .for_you_rows(
                    viewer,
                    cursor.as_ref(),
                    limit + 1,
                    unread_only,
                    Source::default(),
                )
                .await?;
            exhausted = rows.len() as i64 <= limit;
            cursor = rows.last().map(|row| ForYouCursor {
                at: row.at,
                key: row.key.clone(),
            });
            items.extend(self.readable_for_you(viewer, rows, &mut readable).await?);
            if exhausted || items.len() as i64 > limit {
                break;
            }
        }
        let next_before = if items.len() as i64 > limit {
            items.truncate(limit as usize);
            items.last().map(|item| ForYouCursor {
                at: item.at,
                key: item.key.clone(),
            })
        } else if exhausted {
            None
        } else {
            cursor
        };
        let unread_count = self.unread_for_you(viewer, &mut readable).await?.len() as i64;
        Ok(ForYouPage {
            items,
            unread_count,
            next_before,
        })
    }

    async fn unread_for_you(
        &self,
        viewer: &str,
        readable: &mut HashMap<Uuid, bool>,
    ) -> DocumentResult<Vec<ForYouItem>> {
        let unread = self
            .for_you_rows(viewer, None, MAX_FOR_YOU_UNREAD, true, Source::default())
            .await?;
        self.readable_for_you(viewer, unread, readable).await
    }

    /// What is unread for this person right now, newest first, each still readable by them. The
    /// hosted email projection reads this, so an item opened anywhere is never emailed.
    pub async fn unread_for_you_items(&self, viewer: &str) -> DocumentResult<Vec<ForYouItem>> {
        self.unread_for_you(viewer, &mut HashMap::new()).await
    }

    /// Marks items read for this person on every device. Unknown keys are harmless; malformed ones
    /// are refused.
    pub async fn mark_for_you_read(&self, viewer: &str, keys: &[String]) -> DocumentResult<()> {
        checked_keys(keys)?;
        sqlx::query(
            "INSERT INTO for_you_reads (actor_id,item_key,read_at) \
             SELECT $1,key,now() FROM unnest($2::text[]) AS key \
             ON CONFLICT (actor_id,item_key) \
             DO UPDATE SET read_at=EXCLUDED.read_at,marked_unread=false",
        )
        .bind(viewer)
        .bind(keys)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Marks items unread again for this person: they stay unread, even past "mark all read",
    /// until opened. Marking unread also brings an archived item back.
    pub async fn mark_for_you_unread(&self, viewer: &str, keys: &[String]) -> DocumentResult<()> {
        checked_keys(keys)?;
        sqlx::query(
            "INSERT INTO for_you_reads (actor_id,item_key,read_at,marked_unread) \
             SELECT $1,key,'-infinity',true FROM unnest($2::text[]) AS key \
             ON CONFLICT (actor_id,item_key) \
             DO UPDATE SET marked_unread=true,archived_at=NULL",
        )
        .bind(viewer)
        .bind(keys)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Archives items for this person: each is read and leaves their list until something new
    /// happens there. Unarchiving puts them back as they were read.
    pub async fn archive_for_you(
        &self,
        viewer: &str,
        keys: &[String],
        archived: bool,
    ) -> DocumentResult<()> {
        checked_keys(keys)?;
        if archived {
            sqlx::query(
                "INSERT INTO for_you_reads (actor_id,item_key,read_at,archived_at) \
                 SELECT $1,key,now(),now() FROM unnest($2::text[]) AS key \
                 ON CONFLICT (actor_id,item_key) DO UPDATE \
                 SET read_at=EXCLUDED.read_at,archived_at=EXCLUDED.archived_at,marked_unread=false",
            )
        } else {
            sqlx::query(
                "UPDATE for_you_reads SET archived_at=NULL \
                 WHERE actor_id=$1 AND item_key=ANY($2::text[])",
            )
        }
        .bind(viewer)
        .bind(keys)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Everything up to now is read for this person. Per-item read marks before it say nothing
    /// more; archived items stay archived.
    pub async fn mark_all_for_you_read(&self, viewer: &str) -> DocumentResult<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO for_you_read_cursors (actor_id,read_through) VALUES ($1,now()) \
             ON CONFLICT (actor_id) DO UPDATE SET read_through=EXCLUDED.read_through",
        )
        .bind(viewer)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM for_you_reads WHERE actor_id=$1 AND archived_at IS NULL")
            .bind(viewer)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE for_you_reads SET marked_unread=false WHERE actor_id=$1")
            .bind(viewer)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// An explicit Follow or Unfollow by someone who can read the document. It wins over the
    /// implicit follow that creating, editing, commenting or a mention gave them.
    pub async fn set_document_follow(
        &self,
        document_id: Uuid,
        viewer: &str,
        following: bool,
    ) -> DocumentResult<DocumentFollowState> {
        if self
            .document_access_for_actor(document_id, viewer)
            .await?
            .is_none()
        {
            return Err(DocumentError::Unavailable);
        }
        sqlx::query(
            "INSERT INTO native_document_follows (document_id,actor_id,following,updated_at) \
             VALUES ($1,$2,$3,now()) \
             ON CONFLICT (document_id,actor_id) \
             DO UPDATE SET following=EXCLUDED.following,updated_at=EXCLUDED.updated_at",
        )
        .bind(document_id)
        .bind(viewer)
        .bind(following)
        .execute(&self.pool)
        .await?;
        self.document_follow_state(document_id, viewer).await
    }

    /// Whether this person follows the document, and who else changed it since they last opened it.
    pub async fn document_follow_state(
        &self,
        document_id: Uuid,
        viewer: &str,
    ) -> DocumentResult<DocumentFollowState> {
        if self
            .document_access_for_actor(document_id, viewer)
            .await?
            .is_none()
        {
            return Err(DocumentError::Unavailable);
        }
        let following: bool = sqlx::query_scalar(&format!(
            "WITH {} SELECT EXISTS(SELECT 1 FROM follows)",
            FOLLOWED.replace("$6", "$2")
        ))
        .bind(viewer)
        .bind(document_id)
        .fetch_one(&self.pool)
        .await?;
        let changed_since_you_looked = self
            .for_you_rows(
                viewer,
                None,
                MAX_FOR_YOU_PAGE,
                true,
                Source {
                    document: Some(document_id),
                    sheet: None,
                },
            )
            .await?
            .into_iter()
            .find(|item| item.kind == "change")
            .map(|item| DocumentChangeNotice {
                actor_ids: item.actor_ids,
                actor_names: item.actor_names,
                at: item.at,
            });
        Ok(DocumentFollowState {
            following,
            changed_since_you_looked,
        })
    }

    /// An explicit Follow or Unfollow of a sheet by someone who can open it; it wins over the
    /// implicit follow that owning or editing it gave them. Answers whether they now follow it.
    pub async fn set_sheet_follow(
        &self,
        sheet_id: Uuid,
        viewer: &str,
        following: bool,
    ) -> DocumentResult<bool> {
        self.readable_sheet(sheet_id, viewer).await?;
        sqlx::query(
            "INSERT INTO native_sheet_follows (sheet_id,actor_id,following,updated_at) \
             VALUES ($1,$2,$3,now()) \
             ON CONFLICT (sheet_id,actor_id) \
             DO UPDATE SET following=EXCLUDED.following,updated_at=EXCLUDED.updated_at",
        )
        .bind(sheet_id)
        .bind(viewer)
        .bind(following)
        .execute(&self.pool)
        .await?;
        self.sheet_following(sheet_id, viewer).await
    }

    /// Whether this person follows the sheet, explicitly or by owning or editing it.
    pub async fn sheet_following(&self, sheet_id: Uuid, viewer: &str) -> DocumentResult<bool> {
        self.readable_sheet(sheet_id, viewer).await?;
        Ok(sqlx::query_scalar(&format!(
            "WITH {} SELECT EXISTS(SELECT 1 FROM sheet_follows WHERE since IS NOT NULL)",
            SHEETS_FOLLOWED.replace("$8", "$2")
        ))
        .bind(viewer)
        .bind(sheet_id)
        .fetch_one(&self.pool)
        .await?)
    }

    async fn readable_sheet(&self, sheet_id: Uuid, viewer: &str) -> DocumentResult<()> {
        match self.sheet_readable(sheet_id, viewer).await {
            Ok(true) => Ok(()),
            Ok(false) => Err(DocumentError::Unavailable),
            Err(error) => Err(DocumentError::Conflict(error.to_string())),
        }
    }

    /// This person's own seen and snooze marks on their owner Inbox, for the items still in it.
    pub async fn inbox_marks(&self, viewer: &str) -> DocumentResult<Vec<InboxMark>> {
        Ok(sqlx::query_as(
            "SELECT item_id,seen_stamp,snoozed_until FROM inbox_marks \
             WHERE actor_id=$1 ORDER BY updated_at DESC LIMIT 500",
        )
        .bind(viewer)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Records what this person opened or snoozed in their Inbox, on every device. A field left
    /// out keeps its value; marks untouched for 90 days are dropped, since their items are gone.
    pub async fn set_inbox_marks(
        &self,
        viewer: &str,
        marks: &[InboxMarkChange],
    ) -> DocumentResult<()> {
        if marks.is_empty() || marks.len() > MAX_INBOX_MARKS {
            return Err(DocumentError::Invalid(format!(
                "mark between 1 and {MAX_INBOX_MARKS} Inbox items at once"
            )));
        }
        if marks.iter().any(|mark| {
            mark.item_id.is_empty()
                || mark.item_id.chars().count() > 512
                || mark
                    .seen_stamp
                    .as_deref()
                    .is_some_and(|stamp| stamp.chars().count() > 64)
        }) {
            return Err(DocumentError::Invalid("an Inbox mark is malformed".into()));
        }
        let mut tx = self.pool.begin().await?;
        for mark in marks {
            sqlx::query(
                "INSERT INTO inbox_marks (actor_id,item_id,seen_stamp,snoozed_until,updated_at) \
                 VALUES ($1,$2,$3,$5,now()) \
                 ON CONFLICT (actor_id,item_id) DO UPDATE SET \
                   seen_stamp=COALESCE(EXCLUDED.seen_stamp,inbox_marks.seen_stamp), \
                   snoozed_until=CASE WHEN $4 THEN EXCLUDED.snoozed_until \
                     ELSE inbox_marks.snoozed_until END, \
                   updated_at=now()",
            )
            .bind(viewer)
            .bind(&mark.item_id)
            .bind(&mark.seen_stamp)
            .bind(mark.snoozed_until.is_some())
            .bind(mark.snoozed_until.flatten())
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            "DELETE FROM inbox_marks WHERE actor_id=$1 AND updated_at<now()-interval '90 days'",
        )
        .bind(viewer)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
}

/// One change to a person's Inbox mark. `snoozed_until`: absent keeps the snooze, `Some(None)`
/// wakes the item, `Some(Some(t))` snoozes it until `t`.
#[derive(Debug, Clone, Default)]
pub struct InboxMarkChange {
    pub item_id: String,
    pub seen_stamp: Option<String>,
    pub snoozed_until: Option<Option<DateTime<Utc>>>,
}

#[cfg(test)]
mod tests {
    use super::valid_key;

    #[test]
    fn marks_name_one_source_object() {
        let id = "00000000-0000-4000-8000-00000000000a";
        for kind in ["share", "sheet-share", "review", "sheet-changes", "thread"] {
            assert!(valid_key(&format!("{kind}:{id}")));
        }
        assert!(valid_key(&format!("room-thread:{id}:42")));
        assert!(!valid_key(&format!("room-thread:{id}")));
        assert!(!valid_key(&format!("room-thread:{id}:042")));
        assert!(!valid_key(&format!("room-thread:{id}:4x")));
        assert!(!valid_key(&format!("share:{id}:1")));
        assert!(!valid_key(&format!("attention:{id}")));
        assert!(!valid_key(&format!("share:{}", id.to_uppercase())));
    }
}
