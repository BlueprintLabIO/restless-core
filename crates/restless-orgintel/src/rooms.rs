//! Human Rooms layered over the existing durable message path.
//!
//! A Room does not own another message body or delivery lifecycle:
//! `messages.id` remains authoritative and all legacy Work/inbox relations keep
//! pointing at that row. The Room tables add audience, reply-tree, retry and
//! participant-relative cursor semantics around it.

use super::*;
use sha2::Sha256;
use std::collections::BTreeSet;

const MAX_ROOM_PARTICIPANTS: usize = 100;
const MAX_ROOM_MESSAGE_BYTES: usize = 64 * 1024;

/// Length-prefix both Actor ids so future identity adapters may use arbitrary
/// stable identifiers without creating delimiter collisions (for example,
/// `a:b` + `c` must not alias `a` + `b:c`).
pub(crate) fn direct_room_canonical_key(first_actor: &str, second_actor: &str) -> String {
    let (first_actor, second_actor) = if first_actor <= second_actor {
        (first_actor, second_actor)
    } else {
        (second_actor, first_actor)
    };
    format!(
        "direct:{}:{first_actor}:{}:{second_actor}",
        first_actor.len(),
        second_actor.len()
    )
}

fn room_message_digest(
    room_id: Uuid,
    author_actor: &str,
    parent_message_id: Option<i64>,
    outcome_standard: Option<OutcomeStandard>,
    body: &str,
) -> String {
    let mut digest = Sha256::new();
    for part in [
        room_id.to_string(),
        author_actor.to_string(),
        parent_message_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        outcome_standard
            .map(|standard| standard.as_str().to_string())
            .unwrap_or_default(),
        body.to_string(),
    ] {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

async fn active_room_access(
    tx: &mut Transaction<'_, Postgres>,
    room_id: Uuid,
    actor_id: &str,
) -> Result<(RoomKind, RoomParticipantRole)> {
    sqlx::query_as(
        "SELECT room.kind,participant.role FROM rooms room \
         JOIN room_participants participant ON participant.room_id=room.id \
         JOIN actors actor ON actor.id=participant.actor_id \
         WHERE room.id=$1 AND participant.actor_id=$2 \
           AND participant.left_at IS NULL AND actor.retired_at IS NULL \
           AND room.archived_at IS NULL \
         FOR KEY SHARE OF room,participant,actor",
    )
    .bind(room_id)
    .bind(actor_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| {
        OrgIntelError::RoomAccessDenied(
            "the active actor is not an active participant in this Room".into(),
        )
    })
}

pub(crate) async fn active_room_kind(
    tx: &mut Transaction<'_, Postgres>,
    room_id: Uuid,
    actor_id: &str,
) -> Result<RoomKind> {
    Ok(active_room_access(tx, room_id, actor_id).await?.0)
}

async fn room_message_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    room_id: Uuid,
    message_id: i64,
) -> Result<RoomMessageRow> {
    sqlx::query_as(
        "SELECT id,room_id,from_actor,to_actor,body,outcome_standard, \
                parent_message_id,thread_root_message_id,client_command_id,created_at, \
                read_at AS legacy_read_at \
         FROM messages WHERE room_id=$1 AND id=$2",
    )
    .bind(room_id)
    .bind(message_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OrgIntelError::InvalidRoom("Room message does not exist".into()))
}

async fn durable_room_message_event_id(
    tx: &mut Transaction<'_, Postgres>,
    room_id: Uuid,
    message_id: i64,
) -> Result<i64> {
    sqlx::query_scalar(
        "SELECT room_created_event_id FROM messages \
         WHERE room_id=$1 AND id=$2",
    )
    .bind(room_id)
    .bind(message_id)
    .fetch_optional(&mut **tx)
    .await?
    .flatten()
    .ok_or_else(|| {
        OrgIntelError::InvalidRoom(
            "Room message committed without its durable transactional event cursor".into(),
        )
    })
}

async fn append_room_event(
    tx: &mut Transaction<'_, Postgres>,
    event_kind: &str,
    room_id: Uuid,
    actor_id: &str,
    message_id: Option<i64>,
    body: serde_json::Value,
) -> Result<i64> {
    Ok(sqlx::query_scalar(
        "INSERT INTO events \
         (kind,room_id,actor_id,message_id,body) \
         VALUES ($1,$2,$3,$4,$5) RETURNING id",
    )
    .bind(event_kind)
    .bind(room_id)
    .bind(actor_id)
    .bind(message_id)
    .bind(body)
    .fetch_one(&mut **tx)
    .await?)
}

impl OrgIntel {
    /// Create one Room and its initial participant set atomically. Company and
    /// direct Rooms are idempotent by their canonical audience; a group Room
    /// is always a new intentional space. The authenticated principal is
    /// expected to supply only its server-derived `created_by` Actor id.
    pub async fn create_room(
        &self,
        created_by: &str,
        kind: RoomKind,
        title: &str,
        participant_actor_ids: &[&str],
    ) -> Result<RoomRow> {
        let created_by = created_by.trim();
        let title = title.trim();
        if title.is_empty() || title.chars().count() > 160 {
            return Err(OrgIntelError::InvalidRoom(
                "a Room title must contain between 1 and 160 characters".into(),
            ));
        }

        let mut participants = participant_actor_ids
            .iter()
            .map(|actor| actor.trim().to_string())
            .collect::<BTreeSet<_>>();
        participants.insert(created_by.to_string());
        if participants.iter().any(String::is_empty) || participants.len() > MAX_ROOM_PARTICIPANTS {
            return Err(OrgIntelError::InvalidRoom(format!(
                "a Room needs 1 to {MAX_ROOM_PARTICIPANTS} stable participant Actors"
            )));
        }
        if kind == RoomKind::Direct && participants.len() != 2 {
            return Err(OrgIntelError::InvalidRoom(
                "a direct Room needs exactly two distinct participant Actors".into(),
            ));
        }

        let canonical_key = match kind {
            RoomKind::Company => Some("company:default".to_string()),
            RoomKind::Group => None,
            RoomKind::Direct => {
                let mut actors = participants.iter();
                let first = actors.next().expect("direct Room first participant");
                let second = actors.next().expect("direct Room second participant");
                Some(direct_room_canonical_key(first, second))
            }
        };

        let participant_values = participants.iter().cloned().collect::<Vec<_>>();
        let mut tx = self.pool.begin().await?;
        let active_actor_ids = sqlx::query_scalar::<_, String>(
            "SELECT id FROM actors WHERE id=ANY($1) AND retired_at IS NULL ORDER BY id",
        )
        .bind(&participant_values)
        .fetch_all(&mut *tx)
        .await?;
        if active_actor_ids != participant_values {
            return Err(OrgIntelError::InvalidRoom(
                "every Room participant must be an active durable Actor".into(),
            ));
        }

        let candidate_id = Uuid::new_v4();
        let inserted: Option<RoomRow> = sqlx::query_as(
            "INSERT INTO rooms (id,kind,title,created_by,canonical_key) \
             VALUES ($1,$2,$3,$4,$5) \
             ON CONFLICT (canonical_key) DO NOTHING \
             RETURNING id,kind,title,created_by,canonical_key,created_at,archived_at",
        )
        .bind(candidate_id)
        .bind(kind)
        .bind(title)
        .bind(created_by)
        .bind(&canonical_key)
        .fetch_optional(&mut *tx)
        .await?;
        let created = inserted.is_some();
        let room = match inserted {
            Some(room) => room,
            None => {
                let canonical_key = canonical_key.as_deref().ok_or_else(|| {
                    OrgIntelError::InvalidRoom(
                        "an ordinary group Room unexpectedly conflicted".into(),
                    )
                })?;
                sqlx::query_as(
                    "SELECT id,kind,title,created_by,canonical_key,created_at,archived_at \
                     FROM rooms WHERE canonical_key=$1 AND archived_at IS NULL FOR UPDATE",
                )
                .bind(canonical_key)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| {
                    OrgIntelError::InvalidRoom("the canonical Room exists but is not active".into())
                })?
            }
        };
        if room.kind != kind {
            return Err(OrgIntelError::InvalidRoom(
                "canonical Room kind does not match the requested audience".into(),
            ));
        }

        // Creation of an ordinary group Room is always a fresh insert. An
        // existing canonical Room is a mutation target, so serialize changes
        // and require the caller to already belong to it. This prevents a
        // valid company Actor from adding itself to the singleton company Room
        // merely by guessing its canonical identity.
        let existing_participants = if created {
            BTreeSet::new()
        } else {
            let (_, requester_role) = active_room_access(&mut tx, room.id, created_by).await?;
            let current = sqlx::query_scalar::<_, String>(
                "SELECT participant.actor_id FROM room_participants participant \
                 JOIN actors actor ON actor.id=participant.actor_id \
                 WHERE participant.room_id=$1 AND participant.left_at IS NULL \
                   AND actor.retired_at IS NULL ORDER BY participant.actor_id",
            )
            .bind(room.id)
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .collect::<BTreeSet<_>>();

            if kind == RoomKind::Direct && current != participants {
                return Err(OrgIntelError::InvalidRoom(
                    "a canonical direct Room has an immutable two-Actor audience".into(),
                ));
            }
            let additions = participants.difference(&current).count();
            if additions > 0 && requester_role != RoomParticipantRole::Owner {
                return Err(OrgIntelError::RoomAccessDenied(
                    "only the Room owner may add participants".into(),
                ));
            }
            if current.len() + additions > MAX_ROOM_PARTICIPANTS {
                return Err(OrgIntelError::InvalidRoom(format!(
                    "a Room may have at most {MAX_ROOM_PARTICIPANTS} active participants"
                )));
            }
            current
        };

        let mut changed_participants = Vec::new();
        for actor_id in &participant_values {
            if existing_participants.contains(actor_id) {
                continue;
            }
            let role = if actor_id == &room.created_by {
                RoomParticipantRole::Owner
            } else {
                RoomParticipantRole::Member
            };
            let changed: Option<String> = sqlx::query_scalar(
                "INSERT INTO room_participants (room_id,actor_id,role) \
                 VALUES ($1,$2,$3) \
                 ON CONFLICT (room_id,actor_id) DO UPDATE \
                   SET role=EXCLUDED.role,joined_at=now(),left_at=NULL \
                   WHERE room_participants.left_at IS NOT NULL \
                      OR room_participants.role<>EXCLUDED.role \
                 RETURNING actor_id",
            )
            .bind(room.id)
            .bind(actor_id)
            .bind(role)
            .fetch_optional(&mut *tx)
            .await?;
            if let Some(actor_id) = changed {
                changed_participants.push(actor_id);
            }
        }

        if created {
            append_room_event(
                &mut tx,
                "room.created.v1",
                room.id,
                created_by,
                None,
                serde_json::json!({
                    "room_id": room.id,
                    "kind": room.kind,
                    "participants": participant_values,
                }),
            )
            .await?;
        }
        for actor_id in changed_participants {
            append_room_event(
                &mut tx,
                "room.participant.joined.v1",
                room.id,
                created_by,
                None,
                serde_json::json!({
                    "room_id": room.id,
                    "participant_actor_id": actor_id,
                }),
            )
            .await?;
        }
        tx.commit().await?;
        Ok(room)
    }

    /// The one default company Room. Every call remains explicit about the
    /// active Actors admitted to it; no membership role is inferred here.
    pub async fn ensure_company_room(
        &self,
        created_by: &str,
        participant_actor_ids: &[&str],
    ) -> Result<RoomRow> {
        self.create_room(
            created_by,
            RoomKind::Company,
            "Company",
            participant_actor_ids,
        )
        .await
    }

    /// Active Rooms visible to one server-derived Actor.
    pub async fn list_rooms_for_actor(&self, actor_id: &str) -> Result<Vec<RoomRow>> {
        Ok(sqlx::query_as(
            "SELECT room.id,room.kind,room.title,room.created_by,room.canonical_key, \
                    room.created_at,room.archived_at \
             FROM rooms room \
             JOIN room_participants participant ON participant.room_id=room.id \
             JOIN actors actor ON actor.id=participant.actor_id \
             WHERE participant.actor_id=$1 AND participant.left_at IS NULL \
               AND actor.retired_at IS NULL AND room.archived_at IS NULL \
             ORDER BY room.created_at,room.id",
        )
        .bind(actor_id)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Current participants, disclosed only to another active participant.
    pub async fn room_participants(
        &self,
        requesting_actor: &str,
        room_id: Uuid,
    ) -> Result<Vec<RoomParticipantRow>> {
        let mut tx = self.pool.begin().await?;
        active_room_kind(&mut tx, room_id, requesting_actor).await?;
        let rows = sqlx::query_as(
            "SELECT participant.room_id,participant.actor_id,participant.role, \
                    participant.joined_at,participant.left_at \
             FROM room_participants participant \
             JOIN actors actor ON actor.id=participant.actor_id \
             WHERE participant.room_id=$1 AND participant.left_at IS NULL \
               AND actor.retired_at IS NULL \
             ORDER BY participant.joined_at,participant.actor_id",
        )
        .bind(room_id)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(rows)
    }

    /// Add or reactivate one participant. Direct-Room audiences are immutable;
    /// callers create/find the canonical pair instead. For company/group Rooms
    /// the durable Room owner is the only participant allowed to change the
    /// audience. Higher-level membership/admin policy may be stricter, but can
    /// never use this method to bypass this floor.
    pub async fn add_room_participant(
        &self,
        requesting_actor: &str,
        room_id: Uuid,
        actor_id: &str,
    ) -> Result<RoomParticipantRow> {
        let actor_id = actor_id.trim();
        if actor_id.is_empty() {
            return Err(OrgIntelError::InvalidRoom(
                "a Room participant needs a stable Actor id".into(),
            ));
        }

        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT id FROM rooms WHERE id=$1 FOR UPDATE")
            .bind(room_id)
            .fetch_optional(&mut *tx)
            .await?;
        let (kind, requester_role) = active_room_access(&mut tx, room_id, requesting_actor).await?;
        if kind == RoomKind::Direct {
            return Err(OrgIntelError::InvalidRoom(
                "a direct Room audience is immutable".into(),
            ));
        }
        if requester_role != RoomParticipantRole::Owner {
            return Err(OrgIntelError::RoomAccessDenied(
                "only the Room owner may add participants".into(),
            ));
        }
        let actor_is_active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors WHERE id=$1 AND retired_at IS NULL)",
        )
        .bind(actor_id)
        .fetch_one(&mut *tx)
        .await?;
        if !actor_is_active {
            return Err(OrgIntelError::InvalidRoom(
                "the new participant must be an active durable Actor".into(),
            ));
        }

        if let Some(existing) = sqlx::query_as::<_, RoomParticipantRow>(
            "SELECT room_id,actor_id,role,joined_at,left_at FROM room_participants \
             WHERE room_id=$1 AND actor_id=$2 AND left_at IS NULL",
        )
        .bind(room_id)
        .bind(actor_id)
        .fetch_optional(&mut *tx)
        .await?
        {
            tx.commit().await?;
            return Ok(existing);
        }
        let active_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM room_participants WHERE room_id=$1 AND left_at IS NULL",
        )
        .bind(room_id)
        .fetch_one(&mut *tx)
        .await?;
        if active_count >= MAX_ROOM_PARTICIPANTS as i64 {
            return Err(OrgIntelError::InvalidRoom(format!(
                "a Room may have at most {MAX_ROOM_PARTICIPANTS} active participants"
            )));
        }

        let participant = sqlx::query_as(
            "INSERT INTO room_participants (room_id,actor_id,role) \
             VALUES ($1,$2,'member') \
             ON CONFLICT (room_id,actor_id) DO UPDATE \
               SET role='member',joined_at=now(),left_at=NULL \
             RETURNING room_id,actor_id,role,joined_at,left_at",
        )
        .bind(room_id)
        .bind(actor_id)
        .fetch_one(&mut *tx)
        .await?;
        append_room_event(
            &mut tx,
            "room.participant.joined.v1",
            room_id,
            requesting_actor,
            None,
            serde_json::json!({
                "room_id": room_id,
                "participant_actor_id": actor_id,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(participant)
    }

    /// Remove one ordinary participant without deleting their attributable
    /// history. A Room owner cannot remove itself through this primitive and a
    /// direct pair cannot be mutated into a one-sided pseudo-conversation.
    pub async fn remove_room_participant(
        &self,
        requesting_actor: &str,
        room_id: Uuid,
        actor_id: &str,
    ) -> Result<RoomParticipantRow> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT id FROM rooms WHERE id=$1 FOR UPDATE")
            .bind(room_id)
            .fetch_optional(&mut *tx)
            .await?;
        let (kind, requester_role) = active_room_access(&mut tx, room_id, requesting_actor).await?;
        if kind == RoomKind::Direct {
            return Err(OrgIntelError::InvalidRoom(
                "a direct Room audience is immutable".into(),
            ));
        }
        if requester_role != RoomParticipantRole::Owner {
            return Err(OrgIntelError::RoomAccessDenied(
                "only the Room owner may remove participants".into(),
            ));
        }
        if actor_id == requesting_actor {
            return Err(OrgIntelError::InvalidRoom(
                "the Room owner cannot remove itself".into(),
            ));
        }
        let removed = sqlx::query_as(
            "UPDATE room_participants SET left_at=now() \
             WHERE room_id=$1 AND actor_id=$2 AND role='member' AND left_at IS NULL \
             RETURNING room_id,actor_id,role,joined_at,left_at",
        )
        .bind(room_id)
        .bind(actor_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidRoom(
                "the selected Actor is not an active removable participant".into(),
            )
        })?;
        append_room_event(
            &mut tx,
            "room.participant.left.v1",
            room_id,
            requesting_actor,
            None,
            serde_json::json!({
                "room_id": room_id,
                "participant_actor_id": actor_id,
            }),
        )
        .await?;
        tx.commit().await?;
        Ok(removed)
    }

    /// Send one idempotent Room message. The Actor id is an authorization
    /// parameter, not client-authored attribution: the HTTP adapter must derive
    /// it from CompanyAccessContext. A reused command id returns the original
    /// row only when the complete semantic payload is identical.
    pub async fn send_room_message(
        &self,
        room_id: Uuid,
        author_actor: &str,
        body: &str,
        parent_message_id: Option<i64>,
        client_command_id: &str,
    ) -> Result<RoomMessageSendResult> {
        self.send_room_message_with_standard(
            room_id,
            author_actor,
            body,
            parent_message_id,
            client_command_id,
            None,
        )
        .await
    }

    /// The same retry-safe send with the optional owner-composer outcome
    /// standard preserved on the single authoritative Message row.
    pub async fn send_room_message_with_standard(
        &self,
        room_id: Uuid,
        author_actor: &str,
        body: &str,
        parent_message_id: Option<i64>,
        client_command_id: &str,
        outcome_standard: Option<OutcomeStandard>,
    ) -> Result<RoomMessageSendResult> {
        if body.trim().is_empty() || body.len() > MAX_ROOM_MESSAGE_BYTES {
            return Err(OrgIntelError::InvalidRoom(format!(
                "a Room message needs 1 to {MAX_ROOM_MESSAGE_BYTES} bytes"
            )));
        }
        let client_command_id = client_command_id.trim();
        if client_command_id.is_empty() || client_command_id.len() > 128 {
            return Err(OrgIntelError::InvalidRoom(
                "a retryable Room send needs a 1 to 128 byte client command id".into(),
            ));
        }
        let payload_sha256 = room_message_digest(
            room_id,
            author_actor,
            parent_message_id,
            outcome_standard,
            body,
        );

        let mut tx = self.pool.begin().await?;
        let kind = active_room_kind(&mut tx, room_id, author_actor).await?;

        if let Some((message_id, prior_digest)) = sqlx::query_as::<_, (i64, String)>(
            "SELECT id,client_payload_sha256 FROM messages \
             WHERE room_id=$1 AND from_actor=$2 AND client_command_id=$3",
        )
        .bind(room_id)
        .bind(author_actor)
        .bind(client_command_id)
        .fetch_optional(&mut *tx)
        .await?
        {
            if prior_digest != payload_sha256 {
                return Err(OrgIntelError::RoomCommandConflict(format!(
                    "command {client_command_id:?} was already used for another message"
                )));
            }
            let message = room_message_in_tx(&mut tx, room_id, message_id).await?;
            let event_id = durable_room_message_event_id(&mut tx, room_id, message_id).await?;
            tx.commit().await?;
            return Ok(RoomMessageSendResult {
                message,
                event_id,
                created: false,
            });
        }

        let thread_root_message_id = match parent_message_id {
            Some(parent_message_id) => Some(
                sqlx::query_scalar::<_, i64>(
                    "SELECT COALESCE(thread_root_message_id,id) FROM messages \
                     WHERE room_id=$1 AND id=$2",
                )
                .bind(room_id)
                .bind(parent_message_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| {
                    OrgIntelError::InvalidRoom(
                        "a reply parent must be an existing message in the same Room".into(),
                    )
                })?,
            ),
            None => None,
        };

        // `to_actor` stays populated only as a compatibility projection for a
        // direct Room. Replies to the stable owner retain the historical NULL
        // owner-inbox spelling; company/group delivery uses participants and
        // future mentions, never the global legacy inbox.
        let to_actor = if kind == RoomKind::Direct {
            let active_participants = sqlx::query_scalar::<_, String>(
                "SELECT participant.actor_id FROM room_participants participant \
                 JOIN actors actor ON actor.id=participant.actor_id \
                 WHERE participant.room_id=$1 AND participant.left_at IS NULL \
                   AND actor.retired_at IS NULL ORDER BY participant.actor_id",
            )
            .bind(room_id)
            .fetch_all(&mut *tx)
            .await?;
            match active_participants.as_slice() {
                [only] if only == author_actor => Some(author_actor.to_string()),
                [first, second] if first == author_actor => Some(second.clone()),
                [first, second] if second == author_actor => Some(first.clone()),
                _ => {
                    return Err(OrgIntelError::InvalidRoom(
                        "an active direct Room must contain its author and exactly one peer".into(),
                    ))
                }
            }
            .filter(|recipient| recipient != "owner")
        } else {
            None
        };

        let inserted_id: Option<i64> = sqlx::query_scalar(
            "INSERT INTO messages \
             (room_id,from_actor,to_actor,body,parent_message_id,thread_root_message_id, \
              client_command_id,client_payload_sha256,outcome_standard) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) \
             ON CONFLICT (room_id,from_actor,client_command_id) \
               WHERE client_command_id IS NOT NULL DO NOTHING \
             RETURNING id",
        )
        .bind(room_id)
        .bind(author_actor)
        .bind(to_actor.as_deref())
        .bind(body)
        .bind(parent_message_id)
        .bind(thread_root_message_id)
        .bind(client_command_id)
        .bind(&payload_sha256)
        .bind(outcome_standard)
        .fetch_optional(&mut *tx)
        .await?;

        let (message_id, created) = match inserted_id {
            Some(message_id) => (message_id, true),
            None => {
                let (message_id, prior_digest) = sqlx::query_as::<_, (i64, String)>(
                    "SELECT id,client_payload_sha256 FROM messages \
                     WHERE room_id=$1 AND from_actor=$2 AND client_command_id=$3",
                )
                .bind(room_id)
                .bind(author_actor)
                .bind(client_command_id)
                .fetch_one(&mut *tx)
                .await?;
                if prior_digest != payload_sha256 {
                    return Err(OrgIntelError::RoomCommandConflict(format!(
                        "command {client_command_id:?} was concurrently used for another message"
                    )));
                }
                (message_id, false)
            }
        };
        let message = room_message_in_tx(&mut tx, room_id, message_id).await?;
        let event_id = durable_room_message_event_id(&mut tx, room_id, message_id).await?;
        tx.commit().await?;
        Ok(RoomMessageSendResult {
            message,
            event_id,
            created,
        })
    }

    /// Keyset pagination over Room messages, oldest first. A cursor is only a
    /// watermark; it grants no access and is safe to replay after reconnect.
    pub async fn room_messages_after(
        &self,
        requesting_actor: &str,
        room_id: Uuid,
        after_message_id: Option<i64>,
        limit: i64,
    ) -> Result<RoomMessagePage> {
        self.room_message_page(requesting_actor, room_id, after_message_id, limit, None)
            .await
    }

    /// One reply tree. Passing any member of the tree resolves its canonical
    /// root before the page is read.
    pub async fn room_thread_after(
        &self,
        requesting_actor: &str,
        room_id: Uuid,
        thread_message_id: i64,
        after_message_id: Option<i64>,
        limit: i64,
    ) -> Result<RoomMessagePage> {
        let mut tx = self.pool.begin().await?;
        active_room_kind(&mut tx, room_id, requesting_actor).await?;
        let root_message_id = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(thread_root_message_id,id) FROM messages \
             WHERE room_id=$1 AND id=$2",
        )
        .bind(room_id)
        .bind(thread_message_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidRoom("Room thread does not exist".into()))?;
        tx.commit().await?;
        self.room_message_page(
            requesting_actor,
            room_id,
            after_message_id,
            limit,
            Some(root_message_id),
        )
        .await
    }

    async fn room_message_page(
        &self,
        requesting_actor: &str,
        room_id: Uuid,
        after_message_id: Option<i64>,
        limit: i64,
        thread_root_message_id: Option<i64>,
    ) -> Result<RoomMessagePage> {
        let limit = limit.clamp(1, 100);
        let mut tx = self.pool.begin().await?;
        active_room_kind(&mut tx, room_id, requesting_actor).await?;
        let mut messages: Vec<RoomMessageRow> =
            if let Some(thread_root_message_id) = thread_root_message_id {
                sqlx::query_as(
                    "SELECT id,room_id,from_actor,to_actor,body,outcome_standard, \
                        parent_message_id,thread_root_message_id,client_command_id,created_at, \
                        read_at AS legacy_read_at \
                 FROM messages \
                 WHERE room_id=$1 AND id>COALESCE($2,0) \
                   AND (id=$3 OR thread_root_message_id=$3) \
                 ORDER BY id LIMIT $4",
                )
                .bind(room_id)
                .bind(after_message_id)
                .bind(thread_root_message_id)
                .bind(limit + 1)
                .fetch_all(&mut *tx)
                .await?
            } else {
                sqlx::query_as(
                    "SELECT id,room_id,from_actor,to_actor,body,outcome_standard, \
                        parent_message_id,thread_root_message_id,client_command_id,created_at, \
                        read_at AS legacy_read_at \
                 FROM messages WHERE room_id=$1 AND id>COALESCE($2,0) \
                 ORDER BY id LIMIT $3",
                )
                .bind(room_id)
                .bind(after_message_id)
                .bind(limit + 1)
                .fetch_all(&mut *tx)
                .await?
            };
        tx.commit().await?;
        let has_more = messages.len() as i64 > limit;
        if has_more {
            messages.truncate(limit as usize);
        }
        let next_after_message_id = messages.last().map(|message| message.id);
        Ok(RoomMessagePage {
            messages,
            next_after_message_id,
            has_more,
        })
    }

    /// Advance one participant's cursor monotonically. Retrying or presenting
    /// an older message is a no-op and returns the already authoritative row.
    pub async fn mark_room_read_through(
        &self,
        room_id: Uuid,
        actor_id: &str,
        through_message_id: i64,
    ) -> Result<RoomReadCursorRow> {
        let mut tx = self.pool.begin().await?;
        active_room_kind(&mut tx, room_id, actor_id).await?;
        let valid_message: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM messages WHERE room_id=$1 AND id=$2)")
                .bind(room_id)
                .bind(through_message_id)
                .fetch_one(&mut *tx)
                .await?;
        if !valid_message {
            return Err(OrgIntelError::InvalidRoom(
                "a Room read cursor must name a message in that Room".into(),
            ));
        }

        // The monotonic predicate belongs in the upsert itself. A separate
        // read-then-write check can regress from message 10 to message 5 when
        // two first-use requests race before either cursor row exists.
        let advanced: Option<RoomReadCursorRow> = sqlx::query_as(
            "INSERT INTO room_read_cursors (room_id,actor_id,last_read_message_id) \
             VALUES ($1,$2,$3) \
             ON CONFLICT (room_id,actor_id) DO UPDATE \
               SET last_read_message_id=EXCLUDED.last_read_message_id,updated_at=now() \
               WHERE room_read_cursors.last_read_message_id IS NULL \
                  OR room_read_cursors.last_read_message_id < EXCLUDED.last_read_message_id \
             RETURNING room_id,actor_id,last_read_message_id,updated_at",
        )
        .bind(room_id)
        .bind(actor_id)
        .bind(through_message_id)
        .fetch_optional(&mut *tx)
        .await?;
        let cursor = if let Some(cursor) = advanced {
            append_room_event(
                &mut tx,
                "room.read_cursor.advanced.v1",
                room_id,
                actor_id,
                Some(through_message_id),
                serde_json::json!({
                    "room_id": room_id,
                    "through_message_id": through_message_id,
                }),
            )
            .await?;
            cursor
        } else {
            sqlx::query_as(
                "SELECT room_id,actor_id,last_read_message_id,updated_at \
                 FROM room_read_cursors WHERE room_id=$1 AND actor_id=$2",
            )
            .bind(room_id)
            .bind(actor_id)
            .fetch_one(&mut *tx)
            .await?
        };
        tx.commit().await?;
        Ok(cursor)
    }

    pub async fn room_read_cursor(
        &self,
        requesting_actor: &str,
        room_id: Uuid,
    ) -> Result<Option<RoomReadCursorRow>> {
        let mut tx = self.pool.begin().await?;
        active_room_kind(&mut tx, room_id, requesting_actor).await?;
        let cursor = sqlx::query_as(
            "SELECT room_id,actor_id,last_read_message_id,updated_at \
             FROM room_read_cursors WHERE room_id=$1 AND actor_id=$2",
        )
        .bind(room_id)
        .bind(requesting_actor)
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(cursor)
    }

    /// Durable collaboration events after a caller-held cursor. Current Room
    /// participation is checked at read time so an event cursor never becomes
    /// a capability or leaks the existence of another Room.
    pub async fn collaboration_events_after(
        &self,
        requesting_actor: &str,
        after_event_id: i64,
        limit: i64,
    ) -> Result<Vec<CollaborationEventRow>> {
        Ok(sqlx::query_as(
            "SELECT event.id,event.kind,event.room_id,event.actor_id, \
                    event.message_id,event.body,event.created_at \
             FROM events event \
             JOIN room_participants participant ON participant.room_id=event.room_id \
             JOIN actors actor ON actor.id=participant.actor_id \
             JOIN rooms room ON room.id=event.room_id \
             WHERE participant.actor_id=$1 AND participant.left_at IS NULL \
               AND actor.retired_at IS NULL AND room.archived_at IS NULL \
               AND (event.kind<>'room.read_cursor.advanced.v1' OR event.actor_id=$1) \
               AND event.id>$2 ORDER BY event.id LIMIT $3",
        )
        .bind(requesting_actor)
        .bind(after_event_id.max(0))
        .bind(limit.clamp(1, 500))
        .fetch_all(&self.pool)
        .await?)
    }
}
