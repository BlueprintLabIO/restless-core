//! Duplicating a document (S67), as Notion's Duplicate does: a new document the person owns, with
//! the body as it reads now (live edits included), its kind and its general access. Its people,
//! comments, reviews and history stay with the original.

use serde_json::Value;
use uuid::Uuid;

use crate::{
    DocumentCommandResult, DocumentError, DocumentResult, GeneralAccess, NewDocument, OrgIntel,
    SetDocumentGeneralAccess,
};

/// The copy's title: "Copy of …", within the title limit.
pub fn copy_title(title: &str) -> String {
    const MAX_TITLE_CHARS: usize = 200;
    let title = title.trim();
    let named = if title.is_empty() {
        "Copy".to_string()
    } else {
        format!("Copy of {title}")
    };
    named.chars().take(MAX_TITLE_CHARS).collect()
}

impl OrgIntel {
    /// Copies a document this person can read into a new one they own. Replaying the same
    /// command answers the same copy.
    pub async fn duplicate_document(
        &self,
        source_id: Uuid,
        actor_id: &str,
        command_id: Uuid,
    ) -> DocumentResult<DocumentCommandResult> {
        let source = self.get_document_for_actor(source_id, actor_id).await?;
        // What people see now: the live body when there is one, else the current version.
        let live: Option<Value> = sqlx::query_scalar(
            "SELECT projection_json FROM native_document_yjs_state WHERE document_id=$1",
        )
        .bind(source_id)
        .fetch_optional(&self.pool)
        .await?;
        let content = live.unwrap_or_else(|| source.current_version.version.content_json.clone());
        let title = copy_title(&source.document.title);
        let created = self
            .create_document(NewDocument {
                command_id,
                title: &title,
                kind: source.document.kind,
                visibility: source.document.visibility,
                linked_room_id: None,
                inherit_room_visibility: false,
                owner_actor_id: actor_id,
                created_by_actor_id: actor_id,
                content_json: &content,
                reason: "Duplicated",
            })
            .await?;
        // Where it came from, so the images it was made with still show in it.
        sqlx::query(
            "UPDATE native_documents SET duplicated_from=$2 WHERE id=$1 AND duplicated_from IS NULL",
        )
        .bind(created.document_id)
        .bind(source_id)
        .execute(&self.pool)
        .await?;
        let copy = self.get_document_for_actor(created.document_id, actor_id).await?;
        let wanted = source.document.general_access;
        if copy.document.general_access != wanted && wanted != GeneralAccess::Private {
            match self
                .set_document_general_access(SetDocumentGeneralAccess {
                    command_id: Uuid::new_v4(),
                    document_id: created.document_id,
                    actor_id,
                    expected_version: copy.document.version,
                    general_access: wanted,
                })
                .await
            {
                // The copy is theirs either way; at worst it keeps the default access.
                Ok(_) | Err(DocumentError::Conflict(_)) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(created)
    }
}

#[cfg(test)]
mod tests {
    use super::copy_title;

    #[test]
    fn a_copy_is_named_for_its_original() {
        assert_eq!(copy_title("Pricing"), "Copy of Pricing");
        assert_eq!(copy_title("  "), "Copy");
        assert_eq!(copy_title(&"x".repeat(300)).chars().count(), 200);
    }
}
