//! For you (S65): what needs a person's attention beside the decisions in their Inbox, assembled
//! on every read from objects that already exist: Room and document mentions of them, comment
//! threads they are in or on documents they follow, and changes others made to documents they
//! follow. Nothing here is an outbox; the only new state is per-person read marks, an "all read
//! through" cursor, explicit follow choices and who was editing when a live body was saved.
//!
//! Every item is rechecked against the source's own access rule on each read, so a person who loses
//! access to a document or leaves a Room stops seeing what came from it.

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

/// One row per item. `$1` viewer, `$2`/`$3` the (at,key) page cursor, `$4` page size, `$5` unread
/// only, `$6` one document. Activity by the viewer never counts; neither does an automatic save
/// they were editing in, since they saw it happen.
const ITEMS: &str = "viewer AS ( \
   SELECT COALESCE((SELECT read_through FROM for_you_read_cursors WHERE actor_id=$1), \
                   '-infinity'::timestamptz) AS through \
 ), \
 room_mentions AS ( \
   SELECT 'room-mention:'||m.id AS key,'mention'::text AS kind,NULL::uuid AS document_id, \
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
   WHERE $6::uuid IS NULL AND m.mentioned_actor_id=$1 AND m.kind='direct' \
     AND m.cancelled_at IS NULL AND message.from_actor<>$1 \
 ), \
 doc_mentions AS ( \
   SELECT 'doc-mention:'||c.id,'mention',c.document_id,NULL::uuid,NULL::text,NULL::bigint, \
     NULL::bigint,NULL::uuid,d.title,left(c.plain_text,$7),ARRAY[c.author_actor_id],c.created_at, \
     (c.created_at>GREATEST(viewer.through,COALESCE(r.read_at,'-infinity')) \
      AND NOT EXISTS(SELECT 1 FROM native_document_comments answer \
        WHERE answer.thread_id=c.thread_id AND answer.author_actor_id=$1 \
          AND answer.created_at>c.created_at)) \
   FROM native_document_comments c CROSS JOIN viewer \
   JOIN native_documents d ON d.id=c.document_id AND d.status<>'archived' \
   LEFT JOIN for_you_reads r ON r.actor_id=$1 AND r.item_key='doc-mention:'||c.id \
   WHERE $1=ANY(c.mentioned_actor_ids) AND c.author_actor_id<>$1 \
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
   WHERE ($6::uuid IS NULL OR t.document_id=$6) \
 ), \
 threads AS ( \
   SELECT 'thread:'||tr.thread_id, \
     CASE WHEN tr.joined_at IS NULL THEN 'comment' ELSE 'reply' END, \
     tr.document_id,NULL::uuid,NULL::text,NULL::bigint,NULL::bigint,NULL::uuid,tr.title, \
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
 change_rows AS ( \
   SELECT v.document_id,v.created_at,v.created_by_actor_id AS actor \
   FROM native_document_versions v JOIN follows fo ON fo.document_id=v.document_id \
   WHERE v.version_number>1 AND v.created_at>fo.since \
     AND v.created_by_actor_id<>$1 AND v.created_by_actor_id<>'daemon' \
   UNION ALL \
   SELECT v.document_id,v.created_at,e.actor_id \
   FROM native_document_versions v JOIN follows fo ON fo.document_id=v.document_id \
   JOIN native_document_version_editors e ON e.version_id=v.id \
   WHERE v.created_by_actor_id='daemon' AND v.created_at>fo.since AND e.actor_id<>$1 \
     AND NOT EXISTS(SELECT 1 FROM native_document_version_editors own \
       WHERE own.version_id=v.id AND own.actor_id=$1) \
 ), \
 changes AS ( \
   SELECT 'changes:'||fo.document_id,'change',fo.document_id,NULL::uuid,NULL::text,NULL::bigint, \
     NULL::bigint,NULL::uuid,fo.title,''::text,COALESCE(fresh.actors,latest.actors),last.at, \
     fresh.actors IS NOT NULL \
   FROM follows fo CROSS JOIN viewer \
   LEFT JOIN for_you_reads r ON r.actor_id=$1 AND r.item_key='changes:'||fo.document_id \
   JOIN LATERAL (SELECT max(cr.created_at) AS at FROM change_rows cr \
     WHERE cr.document_id=fo.document_id) last ON last.at IS NOT NULL \
   CROSS JOIN LATERAL (SELECT array_agg(DISTINCT cr.actor) AS actors FROM change_rows cr \
     WHERE cr.document_id=fo.document_id AND cr.created_at=last.at) latest \
   LEFT JOIN LATERAL (SELECT array_agg(DISTINCT cr.actor) AS actors FROM change_rows cr \
     WHERE cr.document_id=fo.document_id \
       AND cr.created_at>GREATEST(viewer.through,COALESCE(r.read_at,'-infinity'))) fresh ON TRUE \
   WHERE fo.status<>'archived' \
 ), \
 items AS ( \
   SELECT * FROM room_mentions UNION ALL SELECT * FROM doc_mentions \
   UNION ALL SELECT * FROM threads UNION ALL SELECT * FROM changes \
 )";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum ForYouTarget {
    Document {
        document_id: Uuid,
    },
    Room {
        room_id: Uuid,
        room_kind: String,
        thread_root_message_id: i64,
        message_id: i64,
        mention_id: Uuid,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct ForYouItem {
    /// Stable per source object; the read marks use it.
    pub key: String,
    /// `mention`, `reply` (a thread the person is in), `comment` (a new thread on a document they
    /// follow) or `change` (others changed a document they follow).
    pub kind: String,
    /// The source's own title: the document or the Room.
    pub title: String,
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

/// A read mark names one source object by kind and id; anything else is refused.
fn valid_key(key: &str) -> bool {
    let Some((kind, id)) = key.split_once(':') else {
        return false;
    };
    matches!(kind, "room-mention" | "doc-mention" | "thread" | "changes")
        && Uuid::parse_str(id).is_ok_and(|parsed| parsed.hyphenated().to_string() == id)
}

impl OrgIntel {
    async fn for_you_rows(
        &self,
        viewer: &str,
        before: Option<&ForYouCursor>,
        limit: i64,
        unread_only: bool,
        document: Option<Uuid>,
    ) -> DocumentResult<Vec<ForYouItem>> {
        let rows = sqlx::query(&format!(
            "WITH {FOLLOWED}, {ITEMS} \
             SELECT items.*, ARRAY(SELECT COALESCE(NULLIF(btrim(a.display),''),a.id) \
                 FROM unnest(items.actors) WITH ORDINALITY u(id,n) JOIN actors a ON a.id=u.id \
                 ORDER BY u.n) AS actor_names \
             FROM items \
             WHERE ($2::timestamptz IS NULL OR (items.at,items.key)<($2,$3)) \
               AND (NOT $5 OR items.unread) \
             ORDER BY items.at DESC,items.key DESC LIMIT $4"
        ))
        .bind(viewer)
        .bind(before.map(|cursor| cursor.at))
        .bind(before.map(|cursor| cursor.key.as_str()).unwrap_or(""))
        .bind(limit)
        .bind(unread_only)
        .bind(document)
        .bind(EXCERPT_CHARS)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                let document_id: Option<Uuid> = row.try_get("document_id")?;
                let target = match document_id {
                    Some(document_id) => ForYouTarget::Document { document_id },
                    None => ForYouTarget::Room {
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
    /// Rooms the viewer is in; documents answer to their own access rule, asked once each.
    async fn readable_for_you(
        &self,
        viewer: &str,
        items: Vec<ForYouItem>,
        readable: &mut HashMap<Uuid, bool>,
    ) -> DocumentResult<Vec<ForYouItem>> {
        let mut kept = Vec::with_capacity(items.len());
        for item in items {
            if let ForYouTarget::Document { document_id } = item.target {
                let allowed = match readable.get(&document_id) {
                    Some(allowed) => *allowed,
                    None => {
                        let allowed = self
                            .document_access_for_actor(document_id, viewer)
                            .await?
                            .is_some();
                        readable.insert(document_id, allowed);
                        allowed
                    }
                };
                if !allowed {
                    continue;
                }
            }
            kept.push(item);
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
                .for_you_rows(viewer, cursor.as_ref(), limit + 1, false, None)
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
        let unread = self
            .for_you_rows(viewer, None, MAX_FOR_YOU_UNREAD, true, None)
            .await?;
        let unread_count = self
            .readable_for_you(viewer, unread, &mut readable)
            .await?
            .len() as i64;
        Ok(ForYouPage {
            items,
            unread_count,
            next_before,
        })
    }

    /// Marks items read for this person on every device. Unknown keys are harmless; malformed ones
    /// are refused.
    pub async fn mark_for_you_read(&self, viewer: &str, keys: &[String]) -> DocumentResult<()> {
        if keys.is_empty() || keys.len() > MAX_FOR_YOU_READ_KEYS {
            return Err(DocumentError::Invalid(format!(
                "mark between 1 and {MAX_FOR_YOU_READ_KEYS} items read at once"
            )));
        }
        if let Some(key) = keys.iter().find(|key| !valid_key(key)) {
            return Err(DocumentError::Invalid(format!("unknown For you item {key:?}")));
        }
        sqlx::query(
            "INSERT INTO for_you_reads (actor_id,item_key,read_at) \
             SELECT $1,key,now() FROM unnest($2::text[]) AS key \
             ON CONFLICT (actor_id,item_key) DO UPDATE SET read_at=EXCLUDED.read_at",
        )
        .bind(viewer)
        .bind(keys)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Everything up to now is read for this person. Per-item marks before it say nothing more.
    pub async fn mark_all_for_you_read(&self, viewer: &str) -> DocumentResult<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO for_you_read_cursors (actor_id,read_through) VALUES ($1,now()) \
             ON CONFLICT (actor_id) DO UPDATE SET read_through=EXCLUDED.read_through",
        )
        .bind(viewer)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM for_you_reads WHERE actor_id=$1")
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
            .for_you_rows(viewer, None, MAX_FOR_YOU_PAGE, true, Some(document_id))
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
}
