//! Library pins (S65): what a person pinned to the Library sidebar, kept per person so it follows
//! them to every device. A pin names a source object by key and grants nothing; the Library only
//! shows a pin it can still read for that person.

use crate::{OrgIntel, OrgIntelError, Result};

/// A person keeps at most this many pins; pinning one more lets the oldest go.
pub const MAX_LIBRARY_PINS: i64 = 100;

/// `kind:id`, as the Library names its rows: `doc:<uuid>`, `sheet:<uuid>`, `upload:<uuid>`, or a
/// Work file's kind and id. The table checks the same shape.
pub fn valid_library_pin(key: &str) -> bool {
    let Some((kind, id)) = key.split_once(':') else {
        return false;
    };
    (1..=16).contains(&kind.len())
        && kind.bytes().all(|byte| byte.is_ascii_lowercase())
        && (1..=200).contains(&id.len())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}

impl OrgIntel {
    /// This person's pins, newest first.
    pub async fn library_pins(&self, actor_id: &str) -> Result<Vec<String>> {
        Ok(sqlx::query_scalar(
            "SELECT item_key FROM library_pins WHERE actor_id=$1 \
             ORDER BY pinned_at DESC,item_key LIMIT $2",
        )
        .bind(actor_id)
        .bind(MAX_LIBRARY_PINS)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Pin or unpin one item for this person, and return their pins as they now stand. Pinning
    /// again moves an item to the top; unpinning what is not pinned changes nothing.
    pub async fn set_library_pin(
        &self,
        actor_id: &str,
        key: &str,
        pinned: bool,
    ) -> Result<Vec<String>> {
        if !valid_library_pin(key) {
            return Err(OrgIntelError::InvalidContext(format!(
                "{key:?} does not name a Library item"
            )));
        }
        let mut tx = self.pool.begin().await?;
        if pinned {
            sqlx::query(
                "INSERT INTO library_pins (actor_id,item_key) VALUES ($1,$2) \
                 ON CONFLICT (actor_id,item_key) DO UPDATE SET pinned_at=now()",
            )
            .bind(actor_id)
            .bind(key)
            .execute(&mut *tx)
            .await?;
            sqlx::query(
                "DELETE FROM library_pins WHERE actor_id=$1 AND item_key NOT IN ( \
                   SELECT item_key FROM library_pins WHERE actor_id=$1 \
                   ORDER BY pinned_at DESC,item_key LIMIT $2)",
            )
            .bind(actor_id)
            .bind(MAX_LIBRARY_PINS)
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query("DELETE FROM library_pins WHERE actor_id=$1 AND item_key=$2")
                .bind(actor_id)
                .bind(key)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        self.library_pins(actor_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::valid_library_pin;

    #[test]
    fn pin_keys_name_one_library_item() {
        for key in [
            "doc:0b9c3f0e-8a7e-4f39-9a51-6d1f0c4b2a11",
            "sheet:0b9c3f0e-8a7e-4f39-9a51-6d1f0c4b2a11",
            "upload:0b9c3f0e-8a7e-4f39-9a51-6d1f0c4b2a11",
            "site:art_42.v2",
        ] {
            assert!(valid_library_pin(key), "{key}");
        }
        for key in [
            "",
            "doc",
            "doc:",
            ":id",
            "Doc:id",
            "doc:a b",
            "doc:<script>",
            "doc:a/b",
        ] {
            assert!(!valid_library_pin(key), "{key}");
        }
        assert!(!valid_library_pin(&format!("doc:{}", "a".repeat(201))));
    }
}
