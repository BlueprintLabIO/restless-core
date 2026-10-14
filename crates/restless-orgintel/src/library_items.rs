//! How the company organises its Library (Sprint 83): collections, a display name for a kept
//! file, a new name for a Work file, and removing an archived Work file from the Library for good.
//! Collections are one flat label per item, not a folder tree.

use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::library_pins::valid_library_pin;
use crate::{DocumentAccess, OrgIntel, OrgIntelError, Result};

/// One request organises at most this many items, as a bulk move from a selection does.
pub const MAX_LIBRARY_ITEMS_PER_CHANGE: usize = 200;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct LibraryItemRow {
    pub item_key: String,
    pub collection: Option<String>,
    pub label: Option<String>,
}

fn clean(value: Option<&str>, what: &str, maximum: usize) -> Result<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if value.chars().count() > maximum || value.chars().any(char::is_control) {
        return Err(OrgIntelError::InvalidContext(format!(
            "a {what} has 1 to {maximum} characters and no control characters"
        )));
    }
    Ok(Some(value.to_string()))
}

impl OrgIntel {
    /// Every organised item. Labels name the owner's kept files, so only the owner reads them.
    pub async fn library_items(&self, owner: bool) -> Result<Vec<LibraryItemRow>> {
        Ok(sqlx::query_as(
            "SELECT item_key,collection,CASE WHEN $1 THEN label END AS label FROM library_items \
             WHERE collection IS NOT NULL OR $1 ORDER BY item_key",
        )
        .bind(owner)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Put items into a collection, or take them out of any with `None`. The owner organises
    /// anything; anyone else only the documents and sheets they may edit.
    pub async fn set_library_collection(
        &self,
        actor_id: &str,
        owner: bool,
        keys: &[String],
        collection: Option<&str>,
    ) -> Result<Vec<LibraryItemRow>> {
        if keys.is_empty() || keys.len() > MAX_LIBRARY_ITEMS_PER_CHANGE {
            return Err(OrgIntelError::InvalidContext(format!(
                "name 1 to {MAX_LIBRARY_ITEMS_PER_CHANGE} Library items"
            )));
        }
        let collection = clean(collection, "collection name", 80)?;
        for key in keys {
            if !valid_library_pin(key) {
                return Err(OrgIntelError::InvalidContext(format!(
                    "{key:?} does not name a Library item"
                )));
            }
            if !owner && !self.may_edit_library_item(actor_id, key).await? {
                return Err(OrgIntelError::InvalidContext(
                    "only the owner, or someone who may edit it, can move this item".into(),
                ));
            }
        }
        let mut tx = self.pool.begin().await?;
        for key in keys {
            sqlx::query(
                "INSERT INTO library_items (item_key,collection,updated_by) VALUES ($1,$2,$3) \
                 ON CONFLICT (item_key) DO UPDATE SET collection=EXCLUDED.collection, \
                   updated_by=EXCLUDED.updated_by, updated_at=now()",
            )
            .bind(key)
            .bind(&collection)
            .bind(actor_id)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query("DELETE FROM library_items WHERE collection IS NULL AND label IS NULL")
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        self.library_items(owner).await
    }

    async fn may_edit_library_item(&self, actor_id: &str, key: &str) -> Result<bool> {
        let Some((kind, id)) = key.split_once(':') else {
            return Ok(false);
        };
        let Ok(id) = Uuid::parse_str(id) else {
            return Ok(false);
        };
        match kind {
            "doc" => Ok(matches!(
                self.document_access_for_actor(id, actor_id).await,
                Ok(Some(DocumentAccess::Edit))
            )),
            "sheet" => {
                let mut tx = self.pool.begin().await?;
                let editable = crate::sheets::access(&mut tx, id, actor_id, true).await.is_ok();
                tx.commit().await?;
                Ok(editable)
            }
            _ => Ok(false),
        }
    }

    /// The name a kept file shows in the Library, or its own name again with `None`.
    pub async fn set_library_label(
        &self,
        actor_id: &str,
        key: &str,
        label: Option<&str>,
    ) -> Result<()> {
        if !key.starts_with("upload:") || !valid_library_pin(key) {
            return Err(OrgIntelError::InvalidContext(
                "only a kept file takes a Library name; rename a Work file by its id".into(),
            ));
        }
        let label = clean(label, "file name", 200)?;
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO library_items (item_key,label,updated_by) VALUES ($1,$2,$3) \
             ON CONFLICT (item_key) DO UPDATE SET label=EXCLUDED.label, \
               updated_by=EXCLUDED.updated_by, updated_at=now()",
        )
        .bind(key)
        .bind(&label)
        .bind(actor_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM library_items WHERE collection IS NULL AND label IS NULL")
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Give a Work file a new name, everywhere it is shown. The file itself is unchanged.
    pub async fn rename_work_artifact(&self, artifact_id: Uuid, label: &str) -> Result<bool> {
        let label = clean(Some(label), "file name", 200)?.ok_or_else(|| {
            OrgIntelError::InvalidContext("a file name has 1 to 200 characters".into())
        })?;
        Ok(sqlx::query(
            "UPDATE artifact_refs SET label=$2 WHERE id=$1 AND work_id IS NOT NULL",
        )
        .bind(artifact_id)
        .bind(label)
        .execute(&self.pool)
        .await?
        .rows_affected()
            == 1)
    }

    /// Take a Work file the owner archived out of the Library for good. Only an owner-archived
    /// file qualifies; its reference stays as the provenance other records point at.
    pub async fn remove_archived_work_artifact(
        &self,
        artifact_id: Uuid,
        removed_by: &str,
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        let changed = sqlx::query(
            "UPDATE artifact_refs a SET library_removed_at=now() \
             WHERE a.id=$1 AND a.state='superseded' AND a.library_removed_at IS NULL \
               AND a.work_id IS NOT NULL AND (a.retired_by='owner' OR EXISTS( \
                 SELECT 1 FROM human_principal_actor_bindings b WHERE b.actor_id=a.retired_by \
                   AND b.membership_role='owner' AND b.membership_status='active'))",
        )
        .bind(artifact_id)
        .execute(&mut *tx)
        .await?
        .rows_affected()
            == 1;
        if changed {
            sqlx::query(
                "INSERT INTO events(kind,actor_id,body) VALUES ('artifact_removed_from_library',$1,$2)",
            )
            .bind(removed_by)
            .bind(serde_json::json!({"artifact_ref_id": artifact_id}))
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(changed)
    }
}
