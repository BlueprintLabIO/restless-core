//! Finding a comment (S65): the text of document comments, across every document one person may
//! read, newest first, so ⌘K reaches a discussion as it reaches the document. The access rule is
//! the one document search uses; query text is data and never widens it.

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::{DocumentError, DocumentResult, OrgIntel};

const MAX_COMMENT_SEARCH_QUERY_BYTES: usize = 256;
const MAX_COMMENT_SEARCH_PAGE: i64 = 20;
const COMMENT_SNIPPET_CHARS: i32 = 160;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DocumentCommentSearchHit {
    pub document_id: Uuid,
    pub document_title: String,
    pub thread_id: Uuid,
    pub comment_id: Uuid,
    pub author_actor_id: String,
    pub snippet: String,
    pub created_at: DateTime<Utc>,
}

impl OrgIntel {
    /// Comments whose text matches, on documents this person can read and that are not archived.
    pub async fn search_document_comments_for_actor(
        &self,
        actor_id: &str,
        query: &str,
        limit: i64,
    ) -> DocumentResult<Vec<DocumentCommentSearchHit>> {
        let query = query.trim();
        if query.is_empty() || query.len() > MAX_COMMENT_SEARCH_QUERY_BYTES {
            return Err(DocumentError::Invalid(format!(
                "a comment search needs 1 to {MAX_COMMENT_SEARCH_QUERY_BYTES} bytes"
            )));
        }
        if !(1..=MAX_COMMENT_SEARCH_PAGE).contains(&limit) {
            return Err(DocumentError::Invalid(format!(
                "a comment search returns 1 to {MAX_COMMENT_SEARCH_PAGE} comments"
            )));
        }
        let hits = sqlx::query_as::<_, DocumentCommentSearchHit>(
            "SELECT comment.document_id,document.title AS document_title,comment.thread_id,\
                    comment.id AS comment_id,comment.author_actor_id,\
                    substr(comment.plain_text,\
                      GREATEST(position(lower($2) in lower(comment.plain_text))-40,1),$4) AS snippet,\
                    comment.created_at \
             FROM native_document_comments comment \
             JOIN native_documents document ON document.id=comment.document_id \
             JOIN actors viewer ON viewer.id=$1 AND viewer.retired_at IS NULL \
             LEFT JOIN native_document_participants participant \
               ON participant.document_id=document.id AND participant.actor_id=$1 \
              AND participant.removed_at IS NULL \
             WHERE document.status<>'archived' \
               AND (\
                 document.owner_actor_id=$1 \
                 OR participant.actor_id IS NOT NULL \
                 OR (document.visibility='company' AND viewer.actor_class='human' \
                     AND NOT EXISTS(SELECT 1 FROM human_principal_actor_bindings guest \
                       WHERE guest.actor_id=viewer.id AND guest.membership_role='guest')) \
                 OR (\
                   document.visibility='participants' \
                   AND document.inherit_room_visibility \
                   AND EXISTS (\
                     SELECT 1 FROM rooms room \
                     JOIN room_participants member ON member.room_id=room.id \
                     WHERE room.id=document.linked_room_id \
                       AND room.archived_at IS NULL \
                       AND member.actor_id=$1 AND member.left_at IS NULL\
                   )\
                 )\
               ) \
               AND (\
                 position(lower($2) in lower(comment.plain_text))>0 \
                 OR to_tsvector('simple',comment.plain_text) @@ websearch_to_tsquery('simple',$2)\
               ) \
             ORDER BY comment.created_at DESC,comment.id DESC LIMIT $3",
        )
        .bind(actor_id)
        .bind(query)
        .bind(limit)
        .bind(COMMENT_SNIPPET_CHARS)
        .fetch_all(&self.pool)
        .await?;
        Ok(hits)
    }
}
