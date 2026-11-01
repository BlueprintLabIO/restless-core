//! FYI notices leave the Inbox (S109 B3). Someone changing a document or sheet a person follows,
//! or sharing one with them, is not something only that person can do, so it never counts in
//! their Inbox. Once a day Exec writes them one short message in their Exec chat naming what
//! happened, one line per document or sheet: a share and a change to the same thing read as one
//! line. A day with nothing in it writes nothing.

use super::*;
use crate::{DocumentError, ForYouItem, ForYouTarget};

/// The digest names this many documents and sheets; the rest are counted.
const DIGEST_LINES: usize = 12;

/// Who gets a digest: every active person. The owner's Exec chat is the owner conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DigestRecipient {
    actor_id: String,
    owner: bool,
}

fn document_error(error: DocumentError) -> OrgIntelError {
    match error {
        DocumentError::Database(error) => OrgIntelError::Db(error),
        other => OrgIntelError::RoomCommandConflict(other.to_string()),
    }
}

impl OrgIntel {
    /// Writes each person's digest once `every` has passed since their last one, covering
    /// everything since then. Returns how many digests were written. While Exec is in a turn
    /// nothing is written; the next pass catches up, and the window carries over.
    pub async fn post_notice_digests(&self, company: &str, every: chrono::Duration) -> Result<u64> {
        let exec_busy: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actor_cognitive_leases WHERE actor_id='exec')",
        )
        .fetch_one(&self.pool)
        .await?;
        if exec_busy {
            return Ok(0);
        }
        let now = Utc::now();
        let mut written = 0;
        for recipient in self.notice_digest_recipients().await? {
            // A person's first digest covers the day before it, never their whole history.
            sqlx::query(
                "INSERT INTO for_you_notice_digests (actor_id,through) VALUES ($1,$2) \
                 ON CONFLICT (actor_id) DO NOTHING",
            )
            .bind(&recipient.actor_id)
            .bind(now - every)
            .execute(&self.pool)
            .await?;
            let through: DateTime<Utc> =
                sqlx::query_scalar("SELECT through FROM for_you_notice_digests WHERE actor_id=$1")
                    .bind(&recipient.actor_id)
                    .fetch_one(&self.pool)
                    .await?;
            if now - through < every {
                continue;
            }
            let notices = self
                .for_you_notices(&recipient.actor_id, through, now)
                .await
                .map_err(document_error)?;
            let body = notice_digest_body(company, &notices);
            let mut tx = self.pool.begin().await?;
            // Another replica already wrote this window.
            let claimed: Option<String> = sqlx::query_scalar(
                "SELECT actor_id FROM for_you_notice_digests \
                 WHERE actor_id=$1 AND through=$2 FOR UPDATE",
            )
            .bind(&recipient.actor_id)
            .bind(through)
            .fetch_optional(&mut *tx)
            .await?;
            if claimed.is_none() {
                continue;
            }
            let message_id = match body {
                Some(body) => {
                    messages::validate_conversation_body(&body)?;
                    messages::reject_unfenced_owner_output_in_tx(&mut tx, "exec").await?;
                    let to = (!recipient.owner).then_some(recipient.actor_id.as_str());
                    let room_id = ensure_direct_message_room_in_tx(&mut tx, "exec", to).await?;
                    let id: i64 = sqlx::query_scalar(
                        "INSERT INTO messages (room_id,from_actor,to_actor,body) \
                         VALUES ($1,'exec',$2,$3) RETURNING id",
                    )
                    .bind(room_id)
                    .bind(to)
                    .bind(&body)
                    .fetch_one(&mut *tx)
                    .await?;
                    Some(id)
                }
                None => None,
            };
            sqlx::query(
                "UPDATE for_you_notice_digests SET through=$2, \
                   posted_at=CASE WHEN $3::bigint IS NULL THEN posted_at ELSE now() END, \
                   message_id=COALESCE($3,message_id) \
                 WHERE actor_id=$1",
            )
            .bind(&recipient.actor_id)
            .bind(now)
            .bind(message_id)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            written += u64::from(message_id.is_some());
        }
        Ok(written)
    }

    async fn notice_digest_recipients(&self) -> Result<Vec<DigestRecipient>> {
        let rows: Vec<(String, bool)> = sqlx::query_as(
            "SELECT b.actor_id, b.membership_role='owner' \
               FROM human_principal_actor_bindings b JOIN actors a ON a.id=b.actor_id \
              WHERE b.membership_status='active' AND a.retired_at IS NULL \
              ORDER BY b.membership_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut recipients: Vec<DigestRecipient> = rows
            .into_iter()
            .map(|(actor_id, owner)| DigestRecipient { actor_id, owner })
            .collect();
        // A company without a verified owner binding still has its owner, as `owner`.
        if !recipients.iter().any(|recipient| recipient.owner) {
            let owner: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM actors WHERE id='owner')")
                    .fetch_one(&self.pool)
                    .await?;
            if owner {
                recipients.push(DigestRecipient {
                    actor_id: "owner".into(),
                    owner: true,
                });
            }
        }
        Ok(recipients)
    }
}

/// "Bianca", "Bianca and Exec", "Bianca, Carol and 2 others".
fn names(names: &[String]) -> String {
    match names {
        [] => "Someone".into(),
        [one] => one.clone(),
        [one, two] => format!("{one} and {two}"),
        [one, two, three] => format!("{one}, {two} and {three}"),
        [one, two, rest @ ..] => format!("{one}, {two} and {} others", rest.len()),
    }
}

/// The digest message, or nothing when there is nothing to say. Notices about the same document
/// or sheet become one line, newest first.
pub fn notice_digest_body(company: &str, notices: &[ForYouItem]) -> Option<String> {
    struct Line {
        title: String,
        href: String,
        actors: Vec<String>,
        changed: bool,
        shared: Option<String>,
    }
    let mut lines: Vec<(Uuid, Line)> = Vec::new();
    for notice in notices {
        let (id, href, fallback) = match &notice.target {
            ForYouTarget::Document { document_id, .. } => (
                *document_id,
                format!("/{company}/library/documents?document={document_id}"),
                "a document",
            ),
            ForYouTarget::Sheet { sheet_id, .. } => (
                *sheet_id,
                format!("/{company}/library/sheets?sheet={sheet_id}"),
                "a sheet",
            ),
            ForYouTarget::Room { .. } => continue,
        };
        let index = match lines.iter().position(|(seen, _)| *seen == id) {
            Some(index) => index,
            None => {
                let title = notice.title.trim().replace(['[', ']'], "");
                lines.push((
                    id,
                    Line {
                        title: if title.is_empty() {
                            fallback.into()
                        } else {
                            title
                        },
                        href,
                        actors: Vec::new(),
                        changed: false,
                        shared: None,
                    },
                ));
                lines.len() - 1
            }
        };
        let line = &mut lines[index].1;
        for actor in &notice.actor_names {
            if !line.actors.contains(actor) {
                line.actors.push(actor.clone());
            }
        }
        match notice.kind.as_str() {
            "change" => line.changed = true,
            "share" => line.shared = Some(notice.excerpt.trim().to_lowercase()),
            _ => {}
        }
    }
    if lines.is_empty() {
        return None;
    }
    let mut body = String::from("A few updates from the last day. Nothing here needs you.\n");
    for (_, line) in lines.iter().take(DIGEST_LINES) {
        let who = names(&line.actors);
        let link = format!("[{}]({})", line.title, line.href);
        let sentence = match (&line.shared, line.changed) {
            (Some(access), true) if !access.is_empty() => {
                format!("{who} shared {link} with you ({access}) and changed it")
            }
            (Some(_), true) => format!("{who} shared {link} with you and changed it"),
            (Some(access), false) if !access.is_empty() => {
                format!("{who} shared {link} with you ({access})")
            }
            (Some(_), false) => format!("{who} shared {link} with you"),
            (None, _) => format!("{who} changed {link}"),
        };
        body.push_str(&format!("\n- {sentence}"));
    }
    if lines.len() > DIGEST_LINES {
        body.push_str(&format!(
            "\n- and {} more in the Library",
            lines.len() - DIGEST_LINES
        ));
    }
    Some(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notice(kind: &str, document: Uuid, title: &str, who: &str, excerpt: &str) -> ForYouItem {
        ForYouItem {
            key: format!("{kind}:{document}"),
            kind: kind.into(),
            title: title.into(),
            excerpt: excerpt.into(),
            target: ForYouTarget::Document {
                document_id: document,
                thread_id: None,
            },
            actor_ids: vec![who.to_lowercase()],
            actor_names: vec![who.into()],
            at: Utc::now(),
            unread: true,
        }
    }

    #[test]
    fn a_share_and_a_change_to_one_document_are_one_line() {
        let pricing = Uuid::new_v4();
        let plan = Uuid::new_v4();
        let body = notice_digest_body(
            "acme",
            &[
                notice("change", pricing, "Pricing", "Eames", ""),
                notice("share", pricing, "Pricing", "Eames", "Can edit"),
                notice("change", plan, "Plan [draft]", "Bianca", ""),
            ],
        )
        .unwrap();
        assert_eq!(
            body,
            format!(
                "A few updates from the last day. Nothing here needs you.\n\
                 \n- Eames shared [Pricing](/acme/library/documents?document={pricing}) with you (can edit) and changed it\
                 \n- Bianca changed [Plan draft](/acme/library/documents?document={plan})"
            )
        );
    }

    #[test]
    fn nothing_to_say_writes_nothing() {
        assert_eq!(notice_digest_body("acme", &[]), None);
    }

    #[test]
    fn a_long_day_names_the_first_dozen_and_counts_the_rest() {
        let notices: Vec<ForYouItem> = (0..15)
            .map(|n| notice("change", Uuid::new_v4(), &format!("Doc {n}"), "Exec", ""))
            .collect();
        let body = notice_digest_body("acme", &notices).unwrap();
        assert_eq!(body.matches("\n- Exec changed").count(), DIGEST_LINES);
        assert!(body.ends_with("\n- and 3 more in the Library"));
    }
}
