//! Human Rooms layered over the existing durable message path.
//!
//! A Room does not own another message body or delivery lifecycle:
//! `messages.id` remains authoritative and all legacy Work/inbox relations keep
//! pointing at that row. The Room tables add audience, reply-tree, retry and
//! participant-relative cursor semantics around it.

use super::*;
use crate::events::MAX_EVENT_REPLAY_LIMIT;
use sha2::Sha256;
use std::collections::{BTreeSet, HashMap};

const MAX_ROOM_PARTICIPANTS: usize = 100;
const MAX_ROOM_MESSAGE_BYTES: usize = 64 * 1024;
const MAX_ROOM_LIST_LIMIT: i64 = 100;
const MAX_ROOM_MESSAGE_REVISION_LIMIT: i64 = 10;
const MAX_ROOM_SEARCH_QUERY_BYTES: usize = 256;
const MAX_ROOM_SEARCH_LIMIT: i64 = 25;
const MAX_ROOM_SEARCH_SNIPPET_CHARACTERS: i32 = 480;
const MAX_ACTIVE_ROOMS_CREATED_PER_ACTOR: i64 = 128;
const MAX_MESSAGE_MENTIONS: usize = 16;
const MAX_PENDING_MENTIONS_AUTHORED_PER_ACTOR: i64 = 256;
const MAX_PENDING_MENTIONS_FOR_ACTOR: i64 = 256;
const MAX_MESSAGE_MENTION_CONTEXT_BYTES: usize = 64 * 1024;
const MAX_MENTION_LIST_ITEMS: usize = 16;
const MAX_MENTION_LIST_ITEM_BYTES: usize = 2_000;
const MAX_MESSAGE_MENTION_LEASE_SECONDS: u64 = 5 * 60;

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
    mentions: &[NewRoomMessageMention],
    resolves_mention_id: Option<Uuid>,
) -> String {
    let mut digest = Sha256::new();
    // Keep this first five-part spelling byte-for-byte compatible with the
    // pre-0044 Room command digest. An ordinary send retried after upgrading
    // must still find its original authoritative Message.
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
    if !mentions.is_empty() || resolves_mention_id.is_some() {
        // New semantic fields use an explicit domain so they cannot collide
        // with the legacy five-part shape or an eventual later extension.
        for part in [
            "restless.room-message.mentions-resolution.v1".to_string(),
            serde_json::to_string(mentions).expect("a Room mention command is serializable"),
            resolves_mention_id
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ] {
            digest.update((part.len() as u64).to_be_bytes());
            digest.update(part.as_bytes());
        }
    }
    format!("{:x}", digest.finalize())
}

fn room_creation_digest(kind: RoomKind, title: &str, participant_actor_ids: &[String]) -> String {
    let mut digest = Sha256::new();
    for part in [
        "restless.room-create.v1".to_string(),
        serde_json::to_string(&kind).expect("a Room kind is serializable"),
        title.to_string(),
        serde_json::to_string(participant_actor_ids)
            .expect("normalized Room participants are serializable"),
    ] {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn room_message_edit_digest(
    room_id: Uuid,
    message_id: i64,
    editor_actor: &str,
    expected_revision_number: i64,
    body: &str,
) -> String {
    let mut digest = Sha256::new();
    for part in [
        "restless.room-message.edit.v1".to_string(),
        room_id.to_string(),
        message_id.to_string(),
        editor_actor.to_string(),
        expected_revision_number.to_string(),
        body.to_string(),
    ] {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn room_message_delete_digest(room_id: Uuid, message_id: i64, deleting_actor: &str) -> String {
    let mut digest = Sha256::new();
    for part in [
        "restless.room-message.delete.v1".to_string(),
        room_id.to_string(),
        message_id.to_string(),
        deleting_actor.to_string(),
    ] {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn validated_room_command_id(client_command_id: &str) -> Result<&str> {
    let client_command_id = client_command_id.trim();
    if client_command_id.is_empty() || client_command_id.len() > 128 {
        return Err(OrgIntelError::InvalidRoom(
            "a retryable Room command needs a 1 to 128 byte client command id".into(),
        ));
    }
    Ok(client_command_id)
}

fn bounded_optional(
    label: &str,
    value: &Option<String>,
    maximum_bytes: usize,
) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() || value.len() > maximum_bytes {
        return Err(OrgIntelError::InvalidRoom(format!(
            "a mention {label} needs 1 to {maximum_bytes} bytes when supplied"
        )));
    }
    Ok(Some(value.to_string()))
}

fn bounded_list(label: &str, values: &[String]) -> Result<Vec<String>> {
    if values.len() > MAX_MENTION_LIST_ITEMS {
        return Err(OrgIntelError::InvalidRoom(format!(
            "a mention may include at most {MAX_MENTION_LIST_ITEMS} {label}"
        )));
    }
    values
        .iter()
        .map(|value| {
            let value = value.trim();
            if value.is_empty() || value.len() > MAX_MENTION_LIST_ITEM_BYTES {
                return Err(OrgIntelError::InvalidRoom(format!(
                    "each mention {label} item needs 1 to {MAX_MENTION_LIST_ITEM_BYTES} bytes"
                )));
            }
            Ok(value.to_string())
        })
        .collect()
}

fn normalized_mentions(
    author_actor: &str,
    mentions: &[NewRoomMessageMention],
) -> Result<Vec<NewRoomMessageMention>> {
    if mentions.len() > MAX_MESSAGE_MENTIONS {
        return Err(OrgIntelError::InvalidRoom(format!(
            "a Room message may mention at most {MAX_MESSAGE_MENTIONS} Actors"
        )));
    }
    let mut normalized = Vec::with_capacity(mentions.len());
    let mut actors = BTreeSet::new();
    for mention in mentions {
        let actor_id = mention.actor_id.trim();
        if actor_id.is_empty() || actor_id.len() > 255 {
            return Err(OrgIntelError::InvalidRoom(
                "a mention needs one bounded durable Actor id".into(),
            ));
        }
        if actor_id == author_actor {
            return Err(OrgIntelError::InvalidRoom(
                "a Room message cannot mention its own author".into(),
            ));
        }
        if !actors.insert(actor_id.to_string()) {
            return Err(OrgIntelError::InvalidRoom(
                "a Room message cannot mention the same Actor twice".into(),
            ));
        }
        let why_this_actor = bounded_optional("routing reason", &mention.why_this_actor, 2_000)?;
        let expected_response =
            bounded_optional("expected response", &mention.expected_response, 2_000)?;
        let recommendation = bounded_optional("recommendation", &mention.recommendation, 4_000)?;
        let uncertainty = bounded_optional("uncertainty", &mention.uncertainty, 2_000)?;
        let affected_scope = bounded_optional("affected scope", &mention.affected_scope, 1_000)?;
        let fallback = bounded_optional("fallback", &mention.fallback, 2_000)?;
        if !mention.independent_work_can_continue
            && (mention.work_id.is_none() || mention.deadline_at.is_none() || fallback.is_none())
        {
            return Err(OrgIntelError::InvalidRoom(
                "a blocking mention must name linked Work, a deadline, and a fallback".into(),
            ));
        }
        normalized.push(NewRoomMessageMention {
            actor_id: actor_id.to_string(),
            work_id: mention.work_id,
            why_this_actor,
            expected_response,
            recommendation,
            alternatives: bounded_list("alternative", &mention.alternatives)?,
            evidence: bounded_list("evidence", &mention.evidence)?,
            uncertainty,
            affected_scope,
            deadline_at: mention.deadline_at,
            fallback,
            independent_work_can_continue: mention.independent_work_can_continue,
        });
    }
    normalized.sort_by(|left, right| left.actor_id.cmp(&right.actor_id));
    if serde_json::to_vec(&normalized)
        .expect("normalized Room mentions serialize")
        .len()
        > MAX_MESSAGE_MENTION_CONTEXT_BYTES
    {
        return Err(OrgIntelError::InvalidRoom(format!(
            "a Room message may carry at most {MAX_MESSAGE_MENTION_CONTEXT_BYTES} bytes of mention context"
        )));
    }
    Ok(normalized)
}

fn mention_lease_seconds(lease_for: std::time::Duration) -> Result<i32> {
    let seconds = lease_for.as_secs();
    if lease_for.subsec_nanos() != 0 || !(1..=MAX_MESSAGE_MENTION_LEASE_SECONDS).contains(&seconds)
    {
        return Err(OrgIntelError::InvalidRoom(format!(
            "a mention claim lease must contain 1 to {MAX_MESSAGE_MENTION_LEASE_SECONDS} whole seconds"
        )));
    }
    Ok(seconds as i32)
}

async fn active_room_access(
    tx: &mut Transaction<'_, Postgres>,
    room_id: Uuid,
    actor_id: &str,
) -> Result<(RoomKind, RoomParticipantRole)> {
    // Actor -> Room is the one lifecycle order. Lock the active Actor first;
    // retirement takes its update lock before archiving/cancelling Room state.
    // Holding both predicates through the caller's transaction means access
    // cannot be revoked between authorization and the resulting read/write.
    sqlx::query("SELECT id FROM actors WHERE id=$1 AND retired_at IS NULL FOR SHARE")
        .bind(actor_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::RoomAccessDenied(
                "the active actor is not an active participant in this Room".into(),
            )
        })?;
    sqlx::query_as(
        "SELECT room.kind,participant.role FROM rooms room \
         JOIN room_participants participant ON participant.room_id=room.id \
         WHERE room.id=$1 AND participant.actor_id=$2 \
           AND participant.left_at IS NULL AND room.archived_at IS NULL \
         FOR SHARE OF room,participant",
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

/// Lock and validate the exact Actors to which Runtime can owe a persistent
/// free-form response. Team rows are always locked before target Actor rows,
/// matching lead replacement/disband and preventing a stale lead admission
/// from committing after its lifecycle cancellation scan.
///
/// Human Actors are addressable only when the caller explicitly permits them;
/// agent Actors must be the singleton Exec or a current live team lead. A
/// service Actor is never made addressable merely because legacy data names it
/// as a lead.
pub(crate) async fn lock_runtime_addressable_actors_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    actor_ids: &[String],
    allow_humans: bool,
    additional_active_actor_ids: &[String],
    actors_for_update: bool,
) -> Result<()> {
    let mut actor_ids = actor_ids
        .iter()
        .map(|actor_id| actor_id.trim().to_string())
        .collect::<Vec<_>>();
    actor_ids.sort();
    actor_ids.dedup();
    let mut all_actor_ids = actor_ids.clone();
    all_actor_ids.extend(
        additional_active_actor_ids
            .iter()
            .map(|actor_id| actor_id.trim().to_string()),
    );
    all_actor_ids.sort();
    all_actor_ids.dedup();
    if all_actor_ids.is_empty() {
        return Ok(());
    }

    // This first read discovers which Team rows may carry the current routing
    // responsibility. It is deliberately not authority: after locking all
    // candidate Teams, the Actor rows and exact relationship are reread.
    let snapshots = sqlx::query_as::<_, (String, String, Option<Uuid>)>(
        "SELECT id,actor_class,team_id FROM actors WHERE id=ANY($1) ORDER BY id",
    )
    .bind(&actor_ids)
    .fetch_all(&mut **tx)
    .await?;
    let candidate_team_ids = snapshots
        .iter()
        .filter_map(|(_, _, team_id)| *team_id)
        .collect::<Vec<_>>();
    let live_teams = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id,lead_actor_id FROM teams \
         WHERE disbanded_at IS NULL AND (id=ANY($1) OR lead_actor_id=ANY($2)) \
         ORDER BY id FOR SHARE",
    )
    .bind(&candidate_team_ids)
    .bind(&actor_ids)
    .fetch_all(&mut **tx)
    .await?;

    let active_actors = if actors_for_update {
        sqlx::query_as::<_, (String, String, Option<Uuid>)>(
            "SELECT id,actor_class,team_id FROM actors \
             WHERE id=ANY($1) AND retired_at IS NULL ORDER BY id FOR UPDATE",
        )
        .bind(&all_actor_ids)
        .fetch_all(&mut **tx)
        .await?
    } else {
        sqlx::query_as::<_, (String, String, Option<Uuid>)>(
            "SELECT id,actor_class,team_id FROM actors \
             WHERE id=ANY($1) AND retired_at IS NULL ORDER BY id FOR SHARE",
        )
        .bind(&all_actor_ids)
        .fetch_all(&mut **tx)
        .await?
    };
    if active_actors
        .iter()
        .map(|(actor_id, _, _)| actor_id)
        .ne(all_actor_ids.iter())
    {
        return Err(OrgIntelError::RoomAccessDenied(
            "every Room command Actor must currently be active".into(),
        ));
    }

    let team_leads = live_teams
        .iter()
        .map(|(_, lead_actor_id)| lead_actor_id.clone())
        .collect::<BTreeSet<_>>();
    let leads_by_team = live_teams.into_iter().collect::<HashMap<_, _>>();
    for (actor_id, actor_class, team_id) in active_actors {
        if actor_ids.binary_search(&actor_id).is_err() {
            continue;
        }
        if actor_class == "human" {
            if allow_humans {
                continue;
            }
            return Err(OrgIntelError::RoomAccessDenied(format!(
                "human Actor {actor_id:?} is not an agent conversation target"
            )));
        }
        if actor_class == "agent" && (actor_id == "exec" || team_leads.contains(&actor_id)) {
            continue;
        }
        if actor_class == "agent" {
            let route = team_id
                .and_then(|team_id| leads_by_team.get(&team_id))
                .map(|lead| {
                    format!(
                        "; route it through currently addressable accountable lead {lead:?} instead"
                    )
                })
                .unwrap_or_else(|| "; no active accountable lead is currently addressable".into());
            return Err(OrgIntelError::RoomAccessDenied(format!(
                "Actor {actor_id:?} is not a directly addressable persistent agent{route}"
            )));
        }
        return Err(OrgIntelError::RoomAccessDenied(format!(
            "service/system Actor {actor_id:?} cannot receive a persistent conversation"
        )));
    }
    Ok(())
}

async fn room_message_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    room_id: Uuid,
    message_id: i64,
) -> Result<RoomMessageRow> {
    sqlx::query_as(
        "SELECT message.id,message.room_id,message.from_actor,message.to_actor, \
                CASE WHEN message.deleted_at IS NULL \
                     THEN COALESCE(revision.body,message.body) ELSE '' END AS body, \
                message.outcome_standard,message.parent_message_id,message.thread_root_message_id, \
                message.client_command_id,message.created_at, \
                COALESCE(revision.revision_number,0) AS revision_number, \
                message.edited_at,message.deleted_at, \
                message.read_at AS legacy_read_at \
         FROM messages message \
         LEFT JOIN room_message_revisions revision ON revision.id=message.latest_revision_id \
         WHERE message.room_id=$1 AND message.id=$2",
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

async fn room_message_revision_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    revision_id: Uuid,
) -> Result<RoomMessageRevisionRow> {
    sqlx::query_as(
        "SELECT id,room_id,message_id,revision_number,editor_actor_id,body, \
                created_event_id,created_at \
         FROM room_message_revisions WHERE id=$1",
    )
    .bind(revision_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OrgIntelError::InvalidRoom("Room message revision does not exist".into()))
}

async fn room_message_tombstone_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    tombstone_id: Uuid,
) -> Result<RoomMessageTombstoneRow> {
    sqlx::query_as(
        "SELECT id,room_id,message_id,deleted_by_actor_id,created_event_id,created_at \
         FROM room_message_tombstones WHERE id=$1",
    )
    .bind(tombstone_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OrgIntelError::InvalidRoom("Room message tombstone does not exist".into()))
}

async fn replay_room_message_send_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    room_id: Uuid,
    author_actor: &str,
    client_command_id: &str,
    payload_sha256: &str,
) -> Result<Option<RoomMessageSendResult>> {
    let Some((message_id, prior_digest)) = sqlx::query_as::<_, (i64, String)>(
        "SELECT id,client_payload_sha256 FROM messages \
         WHERE room_id=$1 AND from_actor=$2 AND client_command_id=$3",
    )
    .bind(room_id)
    .bind(author_actor)
    .bind(client_command_id)
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(None);
    };
    if prior_digest != payload_sha256 {
        return Err(OrgIntelError::RoomCommandConflict(format!(
            "command {client_command_id:?} was already used for another message"
        )));
    }
    Ok(Some(RoomMessageSendResult {
        message: room_message_in_tx(tx, room_id, message_id).await?,
        event_id: durable_room_message_event_id(tx, room_id, message_id).await?,
        mentions: message_mentions_for_message_in_tx(tx, message_id).await?,
        resolved_mention: resolved_mention_for_message_in_tx(tx, message_id).await?,
        created: false,
    }))
}

async fn replay_room_message_edit_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    room_id: Uuid,
    editor_actor: &str,
    client_command_id: &str,
    payload_sha256: &str,
) -> Result<Option<RoomMessageEditResult>> {
    let Some((revision_id, prior_digest)) = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id,client_payload_sha256 FROM room_message_revisions \
         WHERE room_id=$1 AND editor_actor_id=$2 AND client_command_id=$3",
    )
    .bind(room_id)
    .bind(editor_actor)
    .bind(client_command_id)
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(None);
    };
    if prior_digest != payload_sha256 {
        return Err(OrgIntelError::RoomCommandConflict(format!(
            "command {client_command_id:?} was already used for another Message edit"
        )));
    }
    Ok(Some(RoomMessageEditResult {
        revision: room_message_revision_in_tx(tx, revision_id).await?,
        created: false,
    }))
}

async fn replay_room_message_delete_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    room_id: Uuid,
    deleting_actor: &str,
    client_command_id: &str,
    payload_sha256: &str,
) -> Result<Option<RoomMessageDeleteResult>> {
    let Some((tombstone_id, prior_digest)) = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT id,client_payload_sha256 FROM room_message_tombstones \
         WHERE room_id=$1 AND deleted_by_actor_id=$2 AND client_command_id=$3",
    )
    .bind(room_id)
    .bind(deleting_actor)
    .bind(client_command_id)
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(None);
    };
    if prior_digest != payload_sha256 {
        return Err(OrgIntelError::RoomCommandConflict(format!(
            "command {client_command_id:?} was already used for another Message deletion"
        )));
    }
    Ok(Some(RoomMessageDeleteResult {
        tombstone: room_message_tombstone_in_tx(tx, tombstone_id).await?,
        created: false,
    }))
}

async fn message_mentions_for_messages_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    message_ids: &[i64],
) -> Result<Vec<MessageMentionRow>> {
    if message_ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(sqlx::query_as(
        "SELECT id,room_id,message_id,thread_root_message_id,mentioned_actor_id,kind,work_id, \
                why_this_actor,expected_response,recommendation,alternatives,evidence, \
                uncertainty,affected_scope,deadline_at,fallback,independent_work_can_continue, \
                created_event_id,resolution_message_id,resolved_event_id,cancelled_event_id, \
                cancelled_by,cancellation_reason,created_at,resolved_at,cancelled_at \
         FROM message_mentions mention \
         WHERE mention.message_id=ANY($1) \
           AND EXISTS (SELECT 1 FROM messages message \
                       WHERE message.id=mention.message_id AND message.deleted_at IS NULL) \
         ORDER BY message_id,mentioned_actor_id",
    )
    .bind(message_ids)
    .fetch_all(&mut **tx)
    .await?)
}

async fn message_mentions_for_message_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    message_id: i64,
) -> Result<Vec<MessageMentionRow>> {
    message_mentions_for_messages_in_tx(tx, &[message_id]).await
}

async fn resolved_mention_for_message_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    message_id: i64,
) -> Result<Option<MessageMentionRow>> {
    Ok(sqlx::query_as(
        "SELECT id,room_id,message_id,thread_root_message_id,mentioned_actor_id,kind,work_id, \
                why_this_actor,expected_response,recommendation,alternatives,evidence, \
                uncertainty,affected_scope,deadline_at,fallback,independent_work_can_continue, \
                created_event_id,resolution_message_id,resolved_event_id,cancelled_event_id, \
                cancelled_by,cancellation_reason,created_at,resolved_at,cancelled_at \
         FROM message_mentions WHERE resolution_message_id=$1",
    )
    .bind(message_id)
    .fetch_optional(&mut **tx)
    .await?)
}

async fn message_mention_context_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    mention_id: Uuid,
) -> Result<MessageMentionContext> {
    let mention: MessageMentionRow = sqlx::query_as(
        "SELECT id,room_id,message_id,thread_root_message_id,mentioned_actor_id,kind,work_id, \
                why_this_actor,expected_response,recommendation,alternatives,evidence, \
                uncertainty,affected_scope,deadline_at,fallback,independent_work_can_continue, \
                created_event_id,resolution_message_id,resolved_event_id,cancelled_event_id, \
                cancelled_by,cancellation_reason,created_at,resolved_at,cancelled_at \
         FROM message_mentions WHERE id=$1",
    )
    .bind(mention_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OrgIntelError::InvalidRoom("message mention does not exist".into()))?;
    let message = room_message_in_tx(tx, mention.room_id, mention.message_id).await?;
    let room_title: String = sqlx::query_scalar("SELECT title FROM rooms WHERE id=$1")
        .bind(mention.room_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidRoom("a durable mention lost its Room".into()))?;
    Ok(MessageMentionContext {
        mention,
        room_title,
        message,
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
    Ok(
        sqlx::query_scalar("SELECT orgintel_append_room_event($1,$2,$3,$4,$5)")
            .bind(event_kind)
            .bind(room_id)
            .bind(actor_id)
            .bind(message_id)
            .bind(body)
            .fetch_one(&mut **tx)
            .await?,
    )
}

/// Fence an Actor's current free-form conversation and terminally consume
/// only ordinary unread inputs that no longer have a runtime route. Named
/// Room mentions are deliberately not cancelled here: they address the
/// durable Actor rather than the Actor's current team office.
pub(crate) async fn revoke_actor_conversation_and_cancel_unread_inputs(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: &str,
    revoked_by: &str,
    reason: &str,
) -> Result<u64> {
    let reason = reason.trim();
    if reason.is_empty() || reason.len() > 2_000 {
        return Err(OrgIntelError::InvalidRoom(
            "revoking a conversation needs a 1 to 2000 byte reason".into(),
        ));
    }
    sqlx::query("SELECT id FROM actors WHERE id=$1 FOR UPDATE")
        .bind(actor_id)
        .fetch_optional(&mut **tx)
        .await?;
    let lease_token: Option<Uuid> = sqlx::query_scalar(
        "UPDATE actor_cognitive_leases \
         SET revoked_at=COALESCE(revoked_at,now()),revoked_by=$2,revocation_reason=$3 \
         WHERE actor_id=$1 AND claimed_until>now() RETURNING lease_token",
    )
    .bind(actor_id)
    .bind(revoked_by)
    .bind(reason)
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(lease_token) = lease_token {
        sqlx::query(
            "UPDATE message_mentions SET claim_token=NULL,claimed_at=NULL,claimed_until=NULL \
             WHERE mentioned_actor_id=$1 AND claim_token=$2 \
               AND resolution_message_id IS NULL AND cancelled_event_id IS NULL",
        )
        .bind(actor_id)
        .bind(lease_token)
        .execute(&mut **tx)
        .await?;
    }
    let abandoned_inputs: Vec<(i64, String)> = sqlx::query_as(
        "SELECT message.id,message.from_actor FROM messages message \
         WHERE message.to_actor=$1 AND message.from_actor<>$1 \
           AND message.read_at IS NULL \
           AND NOT EXISTS (SELECT 1 FROM work_feedback feedback \
                           WHERE feedback.message_id=message.id) \
           AND NOT EXISTS (SELECT 1 FROM message_mentions mention \
                           WHERE mention.message_id=message.id) \
         ORDER BY message.id FOR UPDATE",
    )
    .bind(actor_id)
    .fetch_all(&mut **tx)
    .await?;
    if abandoned_inputs.is_empty() {
        return Ok(0);
    }
    let message_ids = abandoned_inputs
        .iter()
        .map(|(message_id, _)| *message_id)
        .collect::<Vec<_>>();
    sqlx::query("UPDATE messages SET read_at=now() WHERE id=ANY($1) AND read_at IS NULL")
        .bind(&message_ids)
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO events (kind,actor_id,body) VALUES ($1,$2,$3)")
        .bind("actor.conversation.cancelled.v1")
        .bind(revoked_by)
        .bind(serde_json::json!({
            "former_actor_id": actor_id,
            "messages": abandoned_inputs
                .iter()
                .map(|(message_id, from_actor)| serde_json::json!({
                    "message_id": message_id,
                    "from_actor": from_actor,
                }))
                .collect::<Vec<_>>(),
            "reason": reason,
        }))
        .execute(&mut **tx)
        .await?;
    Ok(message_ids.len() as u64)
}

/// End every still-pending mention that became impossible to serve. This is
/// called from the same transaction as roster/Room lifecycle changes so a
/// sender never sees an unresolved mention whose recipient can no longer
/// answer it. Cancellation is terminal and recorded in the existing Room
/// event stream; it never silently retargets the question.
pub(crate) async fn cancel_pending_message_mentions(
    tx: &mut Transaction<'_, Postgres>,
    mentioned_actor_id: &str,
    room_id: Option<Uuid>,
    cancelled_by: &str,
    reason: &str,
) -> Result<u64> {
    let reason = reason.trim();
    if reason.is_empty() || reason.len() > 2_000 {
        return Err(OrgIntelError::InvalidRoom(
            "cancelling a mention needs a 1 to 2000 byte reason".into(),
        ));
    }

    // The Actor row is also the serialization boundary for cognitive leases
    // and Work claims. A lifecycle change therefore cannot race a renewal and
    // leave the superseded process able to persist a reply.
    sqlx::query("SELECT id FROM actors WHERE id=$1 FOR UPDATE")
        .bind(mentioned_actor_id)
        .fetch_optional(&mut **tx)
        .await?;
    let pending: Vec<(Uuid, Uuid, i64)> = sqlx::query_as(
        "SELECT id,room_id,message_id FROM message_mentions \
         WHERE mentioned_actor_id=$1 AND resolution_message_id IS NULL \
           AND cancelled_event_id IS NULL AND ($2::uuid IS NULL OR room_id=$2) \
         ORDER BY created_event_id FOR UPDATE",
    )
    .bind(mentioned_actor_id)
    .bind(room_id)
    .fetch_all(&mut **tx)
    .await?;
    let mention_ids = pending
        .iter()
        .map(|(mention_id, _, _)| *mention_id)
        .collect::<Vec<_>>();
    if room_id.is_none() {
        revoke_actor_conversation_and_cancel_unread_inputs(
            tx,
            mentioned_actor_id,
            cancelled_by,
            reason,
        )
        .await?;
    } else if !mention_ids.is_empty() {
        sqlx::query(
            "UPDATE actor_cognitive_leases \
             SET revoked_at=COALESCE(revoked_at,now()),revoked_by=$3,revocation_reason=$4 \
             WHERE actor_id=$1 AND focused_mention_id=ANY($2) AND claimed_until>now()",
        )
        .bind(mentioned_actor_id)
        .bind(&mention_ids)
        .bind(cancelled_by)
        .bind(reason)
        .execute(&mut **tx)
        .await?;
    }

    if pending.is_empty() {
        return Ok(0);
    }

    for (mention_id, mention_room_id, message_id) in &pending {
        let cancelled_event_id = append_room_event(
            tx,
            "room.mention.cancelled.v1",
            *mention_room_id,
            cancelled_by,
            Some(*message_id),
            serde_json::json!({
                "mention_id": mention_id,
                "mentioned_actor_id": mentioned_actor_id,
                "reason": reason,
            }),
        )
        .await?;
        sqlx::query(
            "UPDATE message_mentions SET cancelled_event_id=$2,cancelled_by=$3, \
                    cancellation_reason=$4,cancelled_at=now(),claim_token=NULL, \
                    claimed_at=NULL,claimed_until=NULL \
             WHERE id=$1 AND resolution_message_id IS NULL AND cancelled_event_id IS NULL",
        )
        .bind(mention_id)
        .bind(cancelled_event_id)
        .bind(cancelled_by)
        .bind(reason)
        .execute(&mut **tx)
        .await?;
    }
    Ok(pending.len() as u64)
}

/// Archive every active group Room whose durable owner is leaving the
/// company. A group Room has exactly one owner and no silent ownership
/// transfer policy, so retirement terminates the collaboration space rather
/// than leaving an unmanageable audience behind. The immutable transcript is
/// retained and every still-pending mention in the archived Room receives an
/// explicit terminal receipt.
///
/// The caller holds the retiring Actor's update lock. All Room commands use
/// Actor -> Room ordering, so taking owned Room locks here preserves the
/// lifecycle order. Focused cognitive leases are fenced by their mention row
/// and Room state; no unrelated Actor row is needed to terminally archive the
/// Room.
pub(crate) async fn archive_group_rooms_owned_by_retiring_actor(
    tx: &mut Transaction<'_, Postgres>,
    actor_id: &str,
    archived_by: &str,
    reason: &str,
) -> Result<u64> {
    let room_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM rooms \
         WHERE created_by=$1 AND kind='group' AND archived_at IS NULL \
         ORDER BY id FOR UPDATE",
    )
    .bind(actor_id)
    .fetch_all(&mut **tx)
    .await?;
    if room_ids.is_empty() {
        return Ok(0);
    }

    for room_id in &room_ids {
        let pending: Vec<(Uuid, String, i64)> = sqlx::query_as(
            "SELECT id,mentioned_actor_id,message_id FROM message_mentions \
             WHERE room_id=$1 AND resolution_message_id IS NULL \
               AND cancelled_event_id IS NULL \
             ORDER BY created_event_id FOR UPDATE",
        )
        .bind(room_id)
        .fetch_all(&mut **tx)
        .await?;
        let mention_ids = pending
            .iter()
            .map(|(mention_id, _, _)| *mention_id)
            .collect::<Vec<_>>();
        if !mention_ids.is_empty() {
            sqlx::query(
                "UPDATE actor_cognitive_leases \
                 SET revoked_at=COALESCE(revoked_at,now()),revoked_by=$2,revocation_reason=$3 \
                 WHERE focused_mention_id=ANY($1) AND claimed_until>now()",
            )
            .bind(&mention_ids)
            .bind(archived_by)
            .bind("the focused Room was archived when its owner retired")
            .execute(&mut **tx)
            .await?;
        }
        for (mention_id, mentioned_actor_id, message_id) in pending {
            let cancelled_event_id = append_room_event(
                tx,
                "room.mention.cancelled.v1",
                *room_id,
                archived_by,
                Some(message_id),
                serde_json::json!({
                    "mention_id": mention_id,
                    "mentioned_actor_id": mentioned_actor_id,
                    "reason": "the Room was archived when its owner retired",
                }),
            )
            .await?;
            sqlx::query(
                "UPDATE message_mentions SET cancelled_event_id=$2,cancelled_by=$3, \
                        cancellation_reason=$4,cancelled_at=now(),claim_token=NULL, \
                        claimed_at=NULL,claimed_until=NULL \
                 WHERE id=$1 AND resolution_message_id IS NULL AND cancelled_event_id IS NULL",
            )
            .bind(mention_id)
            .bind(cancelled_event_id)
            .bind(archived_by)
            .bind("the Room was archived when its owner retired")
            .execute(&mut **tx)
            .await?;
        }

        append_room_event(
            tx,
            "room.archived.v1",
            *room_id,
            archived_by,
            None,
            serde_json::json!({
                "room_id": room_id,
                "former_owner_actor_id": actor_id,
                "reason": reason,
            }),
        )
        .await?;
        sqlx::query("UPDATE rooms SET archived_at=now() WHERE id=$1 AND archived_at IS NULL")
            .bind(room_id)
            .execute(&mut **tx)
            .await?;
    }
    Ok(room_ids.len() as u64)
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
        client_command_id: &str,
    ) -> Result<RoomRow> {
        let created_by = created_by.trim();
        let title = title.trim();
        let client_command_id = client_command_id.trim();
        if client_command_id.is_empty() || client_command_id.len() > 128 {
            return Err(OrgIntelError::InvalidRoom(
                "Room creation needs a stable 1 to 128 byte client command id".into(),
            ));
        }
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
        let payload_sha256 = room_creation_digest(kind, title, &participant_values);
        let mut tx = self.pool.begin().await?;
        // Serialize the command key before looking up its receipt. Two
        // replicas that receive the same group-Room command may both begin
        // before either commits; the loser waits here and then replays the
        // winner instead of attempting a second Room and failing on the
        // receipt primary key.
        sqlx::query(
            "SELECT pg_advisory_xact_lock(\
               hashtextextended('restless-room-create:' || current_schema() || ':' || $1 || ':' || $2,0)\
             )",
        )
        .bind(created_by)
        .bind(client_command_id)
        .execute(&mut *tx)
        .await?;
        if let Some((prior_digest, room_id)) = sqlx::query_as::<_, (String, Uuid)>(
            "SELECT command.client_payload_sha256,command.room_id \
             FROM room_creation_commands command \
             WHERE command.created_by=$1 AND command.client_command_id=$2 \
             FOR SHARE OF command",
        )
        .bind(created_by)
        .bind(client_command_id)
        .fetch_optional(&mut *tx)
        .await?
        {
            if prior_digest != payload_sha256 {
                return Err(OrgIntelError::RoomCommandConflict(format!(
                    "Room command {client_command_id:?} was already used with different semantics"
                )));
            }
            let room = sqlx::query_as(
                "SELECT id,kind,title,created_by,canonical_key,created_at,archived_at \
                 FROM rooms WHERE id=$1 FOR SHARE",
            )
            .bind(room_id)
            .fetch_one(&mut *tx)
            .await?;
            tx.commit().await?;
            return Ok(room);
        }
        // Mutable lifecycle and participant admission applies only to a new
        // command. An exact committed receipt remains recoverable after its
        // creator retires, another participant leaves, or the Room archives.
        sqlx::query("SELECT id FROM actors WHERE id=$1 AND retired_at IS NULL FOR SHARE")
            .bind(created_by)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                OrgIntelError::RoomAccessDenied(
                    "the Room creator must be an active durable Actor".into(),
                )
            })?;
        let active_actor_ids = sqlx::query_scalar::<_, String>(
            "SELECT id FROM actors WHERE id=ANY($1) AND retired_at IS NULL ORDER BY id FOR SHARE",
        )
        .bind(&participant_values)
        .fetch_all(&mut *tx)
        .await?;
        if active_actor_ids != participant_values {
            return Err(OrgIntelError::InvalidRoom(
                "every Room participant must be an active durable Actor".into(),
            ));
        }
        // One durable database fence bounds resource creation across every
        // daemon replica. Archived Rooms no longer consume the active quota;
        // their history remains immutable and available to audit paths.
        sqlx::query("LOCK TABLE rooms IN SHARE ROW EXCLUSIVE MODE")
            .execute(&mut *tx)
            .await?;

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
        if created {
            let active_owned_rooms: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM rooms WHERE created_by=$1 AND archived_at IS NULL",
            )
            .bind(created_by)
            .fetch_one(&mut *tx)
            .await?;
            if active_owned_rooms > MAX_ACTIVE_ROOMS_CREATED_PER_ACTOR {
                return Err(OrgIntelError::InvalidRoom(format!(
                    "an Actor may own at most {MAX_ACTIVE_ROOMS_CREATED_PER_ACTOR} active Rooms"
                )));
            }
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
        sqlx::query(
            "INSERT INTO room_creation_commands \
             (created_by,client_command_id,client_payload_sha256,room_id) \
             VALUES ($1,$2,$3,$4)",
        )
        .bind(created_by)
        .bind(client_command_id)
        .bind(payload_sha256)
        .bind(room.id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(room)
    }

    /// The one default company Room. Every call remains explicit about the
    /// active Actors admitted to it; no membership role is inferred here.
    pub async fn ensure_company_room(
        &self,
        created_by: &str,
        participant_actor_ids: &[&str],
        client_command_id: &str,
    ) -> Result<RoomRow> {
        self.create_room(
            created_by,
            RoomKind::Company,
            "Company",
            participant_actor_ids,
            client_command_id,
        )
        .await
    }

    /// One bounded, newest-first keyset page of active Rooms visible to one
    /// server-derived Actor. Room creation coordinates are immutable, so an
    /// unchanged view never duplicates a Room across pages; newly committed
    /// Rooms naturally appear on a subsequent refresh of the first page.
    pub async fn room_page_for_actor(
        &self,
        actor_id: &str,
        before: Option<(DateTime<Utc>, Uuid)>,
        limit: i64,
    ) -> Result<RoomListPage> {
        if !(1..=MAX_ROOM_LIST_LIMIT).contains(&limit) {
            return Err(OrgIntelError::InvalidRoom(format!(
                "Room list limit must be between 1 and {MAX_ROOM_LIST_LIMIT}"
            )));
        }
        let mut rooms: Vec<RoomRow> = match before {
            Some((created_at, room_id)) => {
                sqlx::query_as(
                    "SELECT room.id,room.kind,room.title,room.created_by,room.canonical_key, \
                            room.created_at,room.archived_at \
                     FROM rooms room \
                     JOIN room_participants participant ON participant.room_id=room.id \
                     JOIN actors actor ON actor.id=participant.actor_id \
                     WHERE participant.actor_id=$1 AND participant.left_at IS NULL \
                       AND actor.retired_at IS NULL AND room.archived_at IS NULL \
                       AND (room.created_at,room.id)<($2,$3) \
                     ORDER BY room.created_at DESC,room.id DESC LIMIT $4",
                )
                .bind(actor_id)
                .bind(created_at)
                .bind(room_id)
                .bind(limit + 1)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as(
                    "SELECT room.id,room.kind,room.title,room.created_by,room.canonical_key, \
                            room.created_at,room.archived_at \
                     FROM rooms room \
                     JOIN room_participants participant ON participant.room_id=room.id \
                     JOIN actors actor ON actor.id=participant.actor_id \
                     WHERE participant.actor_id=$1 AND participant.left_at IS NULL \
                       AND actor.retired_at IS NULL AND room.archived_at IS NULL \
                     ORDER BY room.created_at DESC,room.id DESC LIMIT $2",
                )
                .bind(actor_id)
                .bind(limit + 1)
                .fetch_all(&self.pool)
                .await?
            }
        };
        let has_more = rooms.len() as i64 > limit;
        if has_more {
            rooms.truncate(limit as usize);
        }
        let next = has_more
            .then(|| rooms.last().map(|room| (room.created_at, room.id)))
            .flatten();
        Ok(RoomListPage {
            rooms,
            next_before_created_at: next.map(|cursor| cursor.0),
            next_before_room_id: next.map(|cursor| cursor.1),
            has_more,
        })
    }

    /// Fuzzy, bounded Room-title search over only the active Rooms visible to
    /// one Actor. Title matches remain a Room query; they never broaden a
    /// Message-content search into every Message in a similarly named Room.
    pub async fn search_rooms_for_actor(
        &self,
        actor_id: &str,
        query: &str,
        before: Option<(DateTime<Utc>, Uuid)>,
        limit: i64,
    ) -> Result<RoomListPage> {
        let query = query.trim();
        if query.is_empty() || query.len() > MAX_ROOM_SEARCH_QUERY_BYTES {
            return Err(OrgIntelError::InvalidRoom(format!(
                "a Room search query needs 1 to {MAX_ROOM_SEARCH_QUERY_BYTES} bytes"
            )));
        }
        if !(1..=MAX_ROOM_LIST_LIMIT).contains(&limit) {
            return Err(OrgIntelError::InvalidRoom(format!(
                "Room search limit must be between 1 and {MAX_ROOM_LIST_LIMIT}"
            )));
        }
        let mut rooms: Vec<RoomRow> = match before {
            Some((created_at, room_id)) => {
                sqlx::query_as(
                    "SELECT room.id,room.kind,room.title,room.created_by,room.canonical_key, \
                            room.created_at,room.archived_at \
                     FROM rooms room \
                     JOIN room_participants participant ON participant.room_id=room.id \
                     JOIN actors actor ON actor.id=participant.actor_id \
                     WHERE participant.actor_id=$1 AND participant.left_at IS NULL \
                       AND actor.retired_at IS NULL AND room.archived_at IS NULL \
                       AND (lower(room.title) LIKE '%'||lower($2)||'%' \
                            OR room.title OPERATOR(public.%) $2) \
                       AND (room.created_at,room.id)<($3,$4) \
                     ORDER BY room.created_at DESC,room.id DESC LIMIT $5",
                )
                .bind(actor_id)
                .bind(query)
                .bind(created_at)
                .bind(room_id)
                .bind(limit + 1)
                .fetch_all(&self.pool)
                .await?
            }
            None => {
                sqlx::query_as(
                    "SELECT room.id,room.kind,room.title,room.created_by,room.canonical_key, \
                            room.created_at,room.archived_at \
                     FROM rooms room \
                     JOIN room_participants participant ON participant.room_id=room.id \
                     JOIN actors actor ON actor.id=participant.actor_id \
                     WHERE participant.actor_id=$1 AND participant.left_at IS NULL \
                       AND actor.retired_at IS NULL AND room.archived_at IS NULL \
                       AND (lower(room.title) LIKE '%'||lower($2)||'%' \
                            OR room.title OPERATOR(public.%) $2) \
                     ORDER BY room.created_at DESC,room.id DESC LIMIT $3",
                )
                .bind(actor_id)
                .bind(query)
                .bind(limit + 1)
                .fetch_all(&self.pool)
                .await?
            }
        };
        let has_more = rooms.len() as i64 > limit;
        if has_more {
            rooms.truncate(limit as usize);
        }
        let next = has_more
            .then(|| rooms.last().map(|room| (room.created_at, room.id)))
            .flatten();
        Ok(RoomListPage {
            rooms,
            next_before_created_at: next.map(|cursor| cursor.0),
            next_before_room_id: next.map(|cursor| cursor.1),
            has_more,
        })
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
        let mut command_actors = vec![requesting_actor.to_string(), actor_id.to_string()];
        command_actors.sort();
        command_actors.dedup();
        let active_actors = sqlx::query_scalar::<_, String>(
            "SELECT id FROM actors WHERE id=ANY($1) AND retired_at IS NULL \
             ORDER BY id FOR SHARE",
        )
        .bind(&command_actors)
        .fetch_all(&mut *tx)
        .await?;
        if active_actors != command_actors {
            return Err(OrgIntelError::InvalidRoom(
                "the Room owner and new participant must be active durable Actors".into(),
            ));
        }
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
        // Actor lifecycle owns the outer serialization boundary. Lock both
        // requester and target in stable order before the Room so concurrent
        // retirement, participant mutation, and Room sends cannot form an
        // Actor/Room cycle.
        let mut command_actors = vec![requesting_actor.to_string(), actor_id.to_string()];
        command_actors.sort();
        command_actors.dedup();
        let durable_actors = sqlx::query_scalar::<_, String>(
            "SELECT id FROM actors WHERE id=ANY($1) ORDER BY id FOR UPDATE",
        )
        .bind(&command_actors)
        .fetch_all(&mut *tx)
        .await?;
        if durable_actors != command_actors {
            return Err(OrgIntelError::InvalidRoom(
                "the Room owner and selected participant must be durable Actors".into(),
            ));
        }
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
        cancel_pending_message_mentions(
            &mut tx,
            actor_id,
            Some(room_id),
            requesting_actor,
            "the mentioned Actor was removed from this Room",
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
        self.send_room_message_with_mentions(
            room_id,
            author_actor,
            body,
            parent_message_id,
            client_command_id,
            outcome_standard,
            &[],
            None,
        )
        .await
    }

    /// Retry-safe Room send with structured, explicit Actor mentions and an
    /// optional exact mention resolution. Mention creation, Message creation,
    /// resolution and their existing-stream event receipts share one database
    /// transaction. Reusing the command id with any semantic drift conflicts.
    #[expect(
        clippy::too_many_arguments,
        reason = "the Room command keeps attribution, thread, retry, optional standard and mention semantics explicit"
    )]
    pub async fn send_room_message_with_mentions(
        &self,
        room_id: Uuid,
        author_actor: &str,
        body: &str,
        parent_message_id: Option<i64>,
        client_command_id: &str,
        outcome_standard: Option<OutcomeStandard>,
        mentions: &[NewRoomMessageMention],
        resolves_mention_id: Option<Uuid>,
    ) -> Result<RoomMessageSendResult> {
        self.send_room_message_with_mention_claim(
            room_id,
            author_actor,
            body,
            parent_message_id,
            client_command_id,
            outcome_standard,
            mentions,
            resolves_mention_id,
            None,
        )
        .await
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "the internal Room command adds only the non-client Runtime claim proof"
    )]
    async fn send_room_message_with_mention_claim(
        &self,
        room_id: Uuid,
        author_actor: &str,
        body: &str,
        parent_message_id: Option<i64>,
        client_command_id: &str,
        outcome_standard: Option<OutcomeStandard>,
        mentions: &[NewRoomMessageMention],
        resolves_mention_id: Option<Uuid>,
        resolution_claim_token: Option<Uuid>,
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
        let mentions = normalized_mentions(author_actor, mentions)?;
        let linked_work_ids = mentions
            .iter()
            .filter_map(|mention| mention.work_id)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let payload_sha256 = room_message_digest(
            room_id,
            author_actor,
            parent_message_id,
            outcome_standard,
            body,
            &mentions,
            resolves_mention_id,
        );

        let target_ids = mentions
            .iter()
            .map(|mention| mention.actor_id.clone())
            .collect::<Vec<_>>();
        let mut tx = self.pool.begin().await?;
        // A committed send receipt is read before mutable Actor/Room state.
        // This keeps a lost response replayable after removal, retirement or
        // archival and prevents current policy from rewriting history.
        if let Some(result) = replay_room_message_send_in_tx(
            &mut tx,
            room_id,
            author_actor,
            client_command_id,
            &payload_sha256,
        )
        .await?
        {
            tx.commit().await?;
            return Ok(result);
        }
        // Serialize only this semantic command. After waiting, re-read the
        // receipt before any mutable validation so concurrent duplicates
        // deterministically replay instead of surfacing a unique-key error.
        sqlx::query(
            "SELECT pg_advisory_xact_lock(\
               hashtextextended('restless-room-send:' || current_schema() || ':' || \
                                $1::text || ':' || $2 || ':' || $3,0)\
             )",
        )
        .bind(room_id)
        .bind(author_actor)
        .bind(client_command_id)
        .execute(&mut *tx)
        .await?;
        if let Some(result) = replay_room_message_send_in_tx(
            &mut tx,
            room_id,
            author_actor,
            client_command_id,
            &payload_sha256,
        )
        .await?
        {
            tx.commit().await?;
            return Ok(result);
        }
        // Team responsibility and every participating Actor are locked before
        // the Room, matching retirement/removal and preventing Actor↔Room
        // cycles. The author is only required to be active; mention targets
        // additionally pass the runtime-addressability policy.
        lock_runtime_addressable_actors_in_tx(
            &mut tx,
            &target_ids,
            true,
            &[author_actor.to_string()],
            resolution_claim_token.is_some(),
        )
        .await?;
        let kind = active_room_kind(&mut tx, room_id, author_actor).await?;

        if !linked_work_ids.is_empty() {
            // Work references are collaboration capabilities, not decorative
            // metadata. Lock and admit only company-visible Work or Work
            // explicitly scoped to this same Room. Every mention recipient is
            // checked as an active Room participant below, so the same test
            // protects both author and recipients without an object-wide ACL.
            let visible_work_ids = sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM work \
                 WHERE id=ANY($1) AND (collaboration_visibility='company' \
                    OR (collaboration_visibility='room' AND collaboration_room_id=$2)) \
                 ORDER BY id FOR SHARE",
            )
            .bind(&linked_work_ids)
            .bind(room_id)
            .fetch_all(&mut *tx)
            .await?;
            if visible_work_ids != linked_work_ids {
                return Err(OrgIntelError::RoomAccessDenied(
                    "a Work-scoped mention must reference company Work or Work shared with this exact Room"
                        .into(),
                ));
            }
        }

        if !mentions.is_empty() {
            // Quota locks are semantic and transaction-scoped: every process
            // contending for the same author or recipient serializes before
            // it counts and creates mention rows. They never replace the
            // durable mention rows and cannot be lost on daemon restart.
            let mut quota_keys = vec![format!("out:{author_actor}")];
            quota_keys.extend(target_ids.iter().map(|actor_id| format!("in:{actor_id}")));
            quota_keys.sort();
            quota_keys.dedup();
            for quota_key in quota_keys {
                sqlx::query(
                    "SELECT pg_advisory_xact_lock(\
                       hashtextextended('restless-room-mention-quota:' || current_schema() || ':' || $1,0)\
                     )",
                )
                .bind(quota_key)
                .execute(&mut *tx)
                .await?;
            }
            let authored_pending: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM message_mentions mention \
                 JOIN messages message ON message.id=mention.message_id \
                 WHERE message.from_actor=$1 AND mention.resolution_message_id IS NULL \
                   AND mention.cancelled_event_id IS NULL",
            )
            .bind(author_actor)
            .fetch_one(&mut *tx)
            .await?;
            if authored_pending.saturating_add(mentions.len() as i64)
                > MAX_PENDING_MENTIONS_AUTHORED_PER_ACTOR
            {
                return Err(OrgIntelError::InvalidRoom(format!(
                    "an Actor may author at most {MAX_PENDING_MENTIONS_AUTHORED_PER_ACTOR} pending mentions"
                )));
            }
            let saturated_recipient: Option<String> = sqlx::query_scalar(
                "SELECT mentioned_actor_id FROM message_mentions \
                 WHERE mentioned_actor_id=ANY($1) AND resolution_message_id IS NULL \
                   AND cancelled_event_id IS NULL \
                 GROUP BY mentioned_actor_id HAVING COUNT(*) >= $2 \
                 ORDER BY mentioned_actor_id LIMIT 1",
            )
            .bind(&target_ids)
            .bind(MAX_PENDING_MENTIONS_FOR_ACTOR)
            .fetch_optional(&mut *tx)
            .await?;
            if let Some(actor_id) = saturated_recipient {
                return Err(OrgIntelError::InvalidRoom(format!(
                    "Actor {actor_id:?} already has the maximum {MAX_PENDING_MENTIONS_FOR_ACTOR} pending mentions"
                )));
            }
        }

        if !mentions.is_empty() {
            let active_participants = sqlx::query_scalar::<_, String>(
                "SELECT participant.actor_id FROM room_participants participant \
                 WHERE participant.room_id=$1 AND participant.actor_id=ANY($2) \
                   AND participant.left_at IS NULL ORDER BY participant.actor_id \
                 FOR SHARE OF participant",
            )
            .bind(room_id)
            .bind(&target_ids)
            .fetch_all(&mut *tx)
            .await?;
            if active_participants.iter().ne(target_ids.iter()) {
                return Err(OrgIntelError::RoomAccessDenied(
                    "every mentioned Actor must be a current active Room participant".into(),
                ));
            }
        }

        if let Some(claim_token) = resolution_claim_token {
            let Some(mention_id) = resolves_mention_id else {
                return Err(OrgIntelError::RoomAccessDenied(
                    "a Runtime mention claim may only authorize its exact resolving reply".into(),
                ));
            };
            // The claimed path selected UPDATE strength for every Actor in
            // canonical order above. Never upgrade a shared Actor lock here:
            // identical concurrent retries would otherwise deadlock.
            let lease_live: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM actor_cognitive_leases \
                 WHERE actor_id=$1 AND lease_token=$2 AND focused_mention_id=$3 \
                   AND claimed_until>now() AND revoked_at IS NULL)",
            )
            .bind(author_actor)
            .bind(claim_token)
            .bind(mention_id)
            .fetch_one(&mut *tx)
            .await?;
            if !lease_live {
                return Err(OrgIntelError::RoomAccessDenied(
                    "the Actor cognitive-session lease no longer owns this mention".into(),
                ));
            }
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
        let to_actor = if kind == RoomKind::Direct && mentions.is_empty() {
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
              client_command_id,client_payload_sha256,outcome_standard,current_plain_text) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$4) \
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

        // A concurrent identical command owns mention creation and resolution.
        // Once its Message won the idempotency key, replay its exact receipts
        // rather than reapplying either side effect.
        if !created {
            let message = room_message_in_tx(&mut tx, room_id, message_id).await?;
            let event_id = durable_room_message_event_id(&mut tx, room_id, message_id).await?;
            let mentions = message_mentions_for_message_in_tx(&mut tx, message_id).await?;
            let resolved_mention = resolved_mention_for_message_in_tx(&mut tx, message_id).await?;
            tx.commit().await?;
            return Ok(RoomMessageSendResult {
                message,
                event_id,
                created: false,
                mentions,
                resolved_mention,
            });
        }

        for mention in &mentions {
            let mention_id = Uuid::new_v4();
            let mention_kind = if mention.actor_id == "exec" {
                MessageMentionKind::Exec
            } else {
                MessageMentionKind::Direct
            };
            let created_event_id = append_room_event(
                &mut tx,
                "room.mention.created.v1",
                room_id,
                author_actor,
                Some(message_id),
                serde_json::json!({
                    "mention_id": mention_id,
                    "mentioned_actor_id": mention.actor_id,
                }),
            )
            .await?;
            sqlx::query(
                "INSERT INTO message_mentions \
                 (id,room_id,message_id,thread_root_message_id,mentioned_actor_id,kind,work_id, \
                  why_this_actor,expected_response,recommendation,alternatives,evidence, \
                  uncertainty,affected_scope,deadline_at,fallback,independent_work_can_continue, \
                  created_event_id) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)",
            )
            .bind(mention_id)
            .bind(room_id)
            .bind(message_id)
            .bind(thread_root_message_id.unwrap_or(message_id))
            .bind(&mention.actor_id)
            .bind(mention_kind)
            .bind(mention.work_id)
            .bind(mention.why_this_actor.as_deref())
            .bind(mention.expected_response.as_deref())
            .bind(mention.recommendation.as_deref())
            .bind(
                serde_json::to_value(&mention.alternatives)
                    .expect("mention alternatives serialize"),
            )
            .bind(serde_json::to_value(&mention.evidence).expect("mention evidence serializes"))
            .bind(mention.uncertainty.as_deref())
            .bind(mention.affected_scope.as_deref())
            .bind(mention.deadline_at)
            .bind(mention.fallback.as_deref())
            .bind(mention.independent_work_can_continue)
            .bind(created_event_id)
            .execute(&mut *tx)
            .await?;
            if let Some(work_id) = mention
                .work_id
                .filter(|_| !mention.independent_work_can_continue)
            {
                // A blocking human/agent question pauses only the Work it
                // names. The marker lets the resolving transaction distinguish
                // this wait from an unrelated gate or handoff block.
                sqlx::query(
                    "UPDATE work SET status='blocked',resolution=$2,updated_at=now() \
                     WHERE id=$1 AND status IN ('proposed','active')",
                )
                .bind(work_id)
                .bind(format!("awaiting Room input {mention_id}"))
                .execute(&mut *tx)
                .await?;
            }
        }

        if let Some(mention_id) = resolves_mention_id {
            let Some(parent_message_id) = parent_message_id else {
                return Err(OrgIntelError::InvalidRoom(
                    "a mention can only be resolved by an explicit reply in its Thread".into(),
                ));
            };
            let target = sqlx::query(
                "SELECT mention.room_id,mention.mentioned_actor_id, \
                        mention.thread_root_message_id,mention.resolution_message_id, \
                        mention.cancelled_event_id,actor.actor_class,mention.claim_token, \
                        mention.work_id,mention.independent_work_can_continue, \
                        COALESCE(mention.claimed_until>now(),FALSE) AS claim_live, \
                        EXISTS(SELECT 1 FROM actor_cognitive_leases lease \
                               WHERE lease.actor_id=mention.mentioned_actor_id \
                                 AND lease.lease_token=mention.claim_token \
                                 AND lease.focused_mention_id=mention.id \
                                 AND lease.claimed_until>now() \
                                 AND lease.revoked_at IS NULL) AS lease_live \
                 FROM message_mentions mention \
                 JOIN actors actor ON actor.id=mention.mentioned_actor_id \
                 WHERE mention.id=$1 FOR UPDATE OF mention",
            )
            .bind(mention_id)
            .fetch_optional(&mut *tx)
            .await?;
            let Some(target) = target else {
                return Err(OrgIntelError::InvalidRoom(
                    "the mention selected for resolution does not exist".into(),
                ));
            };
            let mention_room_id: Uuid = target.get("room_id");
            let mentioned_actor_id: String = target.get("mentioned_actor_id");
            let mention_thread_root: i64 = target.get("thread_root_message_id");
            let prior_resolution: Option<i64> = target.get("resolution_message_id");
            if mention_room_id != room_id
                || mentioned_actor_id != author_actor
                || mention_thread_root != thread_root_message_id.unwrap_or(parent_message_id)
            {
                return Err(OrgIntelError::RoomAccessDenied(
                    "only the mentioned Actor may resolve it with a reply in the exact Room and Thread"
                        .into(),
                ));
            }
            if prior_resolution.is_some() {
                return Err(OrgIntelError::RoomCommandConflict(
                    "the selected mention was already resolved by another reply".into(),
                ));
            }
            if target.get::<Option<i64>, _>("cancelled_event_id").is_some() {
                return Err(OrgIntelError::RoomCommandConflict(
                    "the selected mention was cancelled and cannot be resolved".into(),
                ));
            }
            let actor_class: String = target.get("actor_class");
            let durable_claim_token: Option<Uuid> = target.get("claim_token");
            if actor_class == "human" {
                if resolution_claim_token.is_some() {
                    return Err(OrgIntelError::RoomAccessDenied(
                        "a human mention is resolved by its authenticated human, not a Runtime lease"
                            .into(),
                    ));
                }
            } else if resolution_claim_token.is_none()
                || durable_claim_token != resolution_claim_token
                || !target.get::<bool, _>("claim_live")
                || !target.get::<bool, _>("lease_live")
            {
                return Err(OrgIntelError::RoomAccessDenied(
                    "an agent mention reply requires its exact live Actor cognitive-session lease"
                        .into(),
                ));
            }
            let resolved_event_id = append_room_event(
                &mut tx,
                "room.mention.resolved.v1",
                room_id,
                author_actor,
                Some(message_id),
                serde_json::json!({
                    "mention_id": mention_id,
                    "resolution_message_id": message_id,
                }),
            )
            .await?;
            sqlx::query(
                "UPDATE message_mentions \
                 SET resolution_message_id=$2,resolved_event_id=$3,resolved_at=now(), \
                     resolution_claim_token=$4,claim_token=NULL,claimed_at=NULL,claimed_until=NULL \
                 WHERE id=$1 AND resolution_message_id IS NULL \
                   AND cancelled_event_id IS NULL",
            )
            .bind(mention_id)
            .bind(message_id)
            .bind(resolved_event_id)
            .bind(resolution_claim_token)
            .execute(&mut *tx)
            .await?;
            if let Some(work_id) = target.get::<Option<Uuid>, _>("work_id") {
                // The answer is ordinary attributed Work feedback. Linking it
                // here, in the same transaction that resolves Attention,
                // gives the next Attempt the exact response without copying
                // the message body into another store.
                sqlx::query(
                    "INSERT INTO work_feedback (work_id,message_id,linked_by) \
                     VALUES ($1,$2,$3) ON CONFLICT (work_id,message_id) DO NOTHING",
                )
                .bind(work_id)
                .bind(message_id)
                .bind(author_actor)
                .execute(&mut *tx)
                .await?;

                if !target.get::<bool, _>("independent_work_can_continue") {
                    let another_blocking_input: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM message_mentions \
                         WHERE work_id=$1 AND independent_work_can_continue=FALSE \
                           AND resolution_message_id IS NULL AND cancelled_event_id IS NULL)",
                    )
                    .bind(work_id)
                    .fetch_one(&mut *tx)
                    .await?;
                    let pending_handoff: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM owner_handoffs \
                         WHERE work_id=$1 AND state='pending')",
                    )
                    .bind(work_id)
                    .fetch_one(&mut *tx)
                    .await?;
                    if !another_blocking_input && !pending_handoff {
                        sqlx::query(
                            "UPDATE work SET status='active',resolution=$2,updated_at=now(), \
                                    attempt_limit=CASE \
                                      WHEN attempt_limit IS NOT NULL \
                                        AND NOT EXISTS (SELECT 1 FROM work_attempts attempt \
                                                        WHERE attempt.work_id=work.id AND attempt.state='running') \
                                        AND (SELECT count(*) FROM work_attempts attempt \
                                             WHERE attempt.work_id=work.id AND attempt.revision=work.revision \
                                               AND attempt.state <> 'superseded') >= attempt_limit \
                                        AND attempt_limit < 2147483647 \
                                      THEN attempt_limit + 1 ELSE attempt_limit END \
                             WHERE id=$1 AND status='blocked' \
                               AND resolution LIKE 'awaiting Room input %'",
                        )
                        .bind(work_id)
                        .bind(format!("Room input returned in message {message_id}"))
                        .execute(&mut *tx)
                        .await?;
                    }
                }
            }
            if let Some(claim_token) = resolution_claim_token {
                sqlx::query(
                    "UPDATE actor_cognitive_leases SET focused_mention_id=NULL \
                     WHERE actor_id=$1 AND lease_token=$2 AND focused_mention_id=$3",
                )
                .bind(author_actor)
                .bind(claim_token)
                .bind(mention_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        let message = room_message_in_tx(&mut tx, room_id, message_id).await?;
        let event_id = durable_room_message_event_id(&mut tx, room_id, message_id).await?;
        let mentions = message_mentions_for_message_in_tx(&mut tx, message_id).await?;
        let resolved_mention = resolved_mention_for_message_in_tx(&mut tx, message_id).await?;
        tx.commit().await?;
        Ok(RoomMessageSendResult {
            message,
            event_id,
            created,
            mentions,
            resolved_mention,
        })
    }

    /// Append one immutable revision to a Message authored by the requesting
    /// Actor. `expected_revision_number=0` names the original Message body;
    /// later values fence concurrent editors without rewriting history.
    pub async fn edit_room_message(
        &self,
        room_id: Uuid,
        editor_actor: &str,
        message_id: i64,
        expected_revision_number: i64,
        body: &str,
        client_command_id: &str,
    ) -> Result<RoomMessageEditResult> {
        if body.trim().is_empty() || body.len() > MAX_ROOM_MESSAGE_BYTES {
            return Err(OrgIntelError::InvalidRoom(format!(
                "a Room message revision needs 1 to {MAX_ROOM_MESSAGE_BYTES} bytes"
            )));
        }
        if expected_revision_number < 0 {
            return Err(OrgIntelError::InvalidRoom(
                "a Room message revision number cannot be negative".into(),
            ));
        }
        let client_command_id = validated_room_command_id(client_command_id)?;
        let payload_sha256 = room_message_edit_digest(
            room_id,
            message_id,
            editor_actor,
            expected_revision_number,
            body,
        );

        let mut tx = self.pool.begin().await?;
        // A receipt is an immutable fact. Recover it before consulting mutable
        // actor, participant, or Room lifecycle so a lost response remains
        // replayable after retirement, removal, or archival.
        if let Some(result) = replay_room_message_edit_in_tx(
            &mut tx,
            room_id,
            editor_actor,
            client_command_id,
            &payload_sha256,
        )
        .await?
        {
            tx.commit().await?;
            return Ok(result);
        }
        // The Actor is the command-id serialization boundary and is locked
        // before the Room, matching every other Room lifecycle operation.
        sqlx::query("SELECT id FROM actors WHERE id=$1 FOR UPDATE")
            .bind(editor_actor)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                OrgIntelError::RoomAccessDenied(
                    "the active actor is not an active participant in this Room".into(),
                )
            })?;
        if let Some(result) = replay_room_message_edit_in_tx(
            &mut tx,
            room_id,
            editor_actor,
            client_command_id,
            &payload_sha256,
        )
        .await?
        {
            tx.commit().await?;
            return Ok(result);
        }
        active_room_access(&mut tx, room_id, editor_actor).await?;

        let message = sqlx::query(
            "SELECT message.from_actor,message.deleted_at,message.current_plain_text, \
                    COALESCE(revision.revision_number,0) AS revision_number \
             FROM messages message \
             LEFT JOIN room_message_revisions revision ON revision.id=message.latest_revision_id \
             WHERE message.room_id=$1 AND message.id=$2 FOR UPDATE OF message",
        )
        .bind(room_id)
        .bind(message_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidRoom("Room message does not exist".into()))?;
        let author_actor: String = message.get("from_actor");
        if author_actor != editor_actor {
            return Err(OrgIntelError::RoomAccessDenied(
                "only the original Message author may append a revision".into(),
            ));
        }
        if message
            .get::<Option<DateTime<Utc>>, _>("deleted_at")
            .is_some()
        {
            return Err(OrgIntelError::RoomCommandConflict(
                "a deleted Room Message cannot be edited".into(),
            ));
        }
        let current_revision_number: i64 = message.get("revision_number");
        if current_revision_number != expected_revision_number {
            return Err(OrgIntelError::RoomCommandConflict(format!(
                "expected Message revision {expected_revision_number}, current revision is {current_revision_number}"
            )));
        }
        if message
            .get::<Option<String>, _>("current_plain_text")
            .as_deref()
            == Some(body)
        {
            return Err(OrgIntelError::InvalidRoom(
                "a Message edit must change the visible body".into(),
            ));
        }

        let revision_id = Uuid::new_v4();
        let revision_number = current_revision_number + 1;
        let created_event_id = append_room_event(
            &mut tx,
            "room.message.edited.v1",
            room_id,
            editor_actor,
            Some(message_id),
            serde_json::json!({
                "message_id": message_id,
                "revision_id": revision_id,
                "revision_number": revision_number,
            }),
        )
        .await?;
        let revision: RoomMessageRevisionRow = sqlx::query_as(
            "INSERT INTO room_message_revisions \
             (id,room_id,message_id,revision_number,editor_actor_id,body,plain_text, \
              client_command_id,client_payload_sha256,created_event_id) \
             VALUES ($1,$2,$3,$4,$5,$6,$6,$7,$8,$9) \
             RETURNING id,room_id,message_id,revision_number,editor_actor_id,body, \
                       created_event_id,created_at",
        )
        .bind(revision_id)
        .bind(room_id)
        .bind(message_id)
        .bind(revision_number)
        .bind(editor_actor)
        .bind(body)
        .bind(client_command_id)
        .bind(&payload_sha256)
        .bind(created_event_id)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE messages SET latest_revision_id=$3,current_plain_text=$4,edited_at=$5 \
             WHERE room_id=$1 AND id=$2",
        )
        .bind(room_id)
        .bind(message_id)
        .bind(revision_id)
        .bind(body)
        .bind(revision.created_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(RoomMessageEditResult {
            revision,
            created: true,
        })
    }

    /// Preserve one Message as a body-free tombstone. Authors may delete their
    /// own Messages; a non-direct Room's durable owner may also moderate it.
    /// Direct peers never gain moderation power from canonical owner shape.
    /// Pending mentions from the deleted source are cancelled explicitly.
    pub async fn delete_room_message(
        &self,
        room_id: Uuid,
        deleting_actor: &str,
        message_id: i64,
        client_command_id: &str,
    ) -> Result<RoomMessageDeleteResult> {
        let client_command_id = validated_room_command_id(client_command_id)?;
        let payload_sha256 = room_message_delete_digest(room_id, message_id, deleting_actor);
        let mut tx = self.pool.begin().await?;

        if let Some(result) = replay_room_message_delete_in_tx(
            &mut tx,
            room_id,
            deleting_actor,
            client_command_id,
            &payload_sha256,
        )
        .await?
        {
            tx.commit().await?;
            return Ok(result);
        }

        // Discover only lock coordinates first. No data leaves this method
        // until exact Room access is proven below. Lock every affected Actor
        // in stable order before the Room to preserve the lifecycle order.
        let mut actor_ids = sqlx::query_scalar::<_, String>(
            "SELECT mentioned_actor_id FROM message_mentions \
             WHERE room_id=$1 AND message_id=$2 AND resolution_message_id IS NULL \
               AND cancelled_event_id IS NULL ORDER BY mentioned_actor_id",
        )
        .bind(room_id)
        .bind(message_id)
        .fetch_all(&mut *tx)
        .await?;
        actor_ids.push(deleting_actor.to_string());
        actor_ids.sort();
        actor_ids.dedup();
        sqlx::query("SELECT id FROM actors WHERE id=ANY($1) ORDER BY id FOR UPDATE")
            .bind(&actor_ids)
            .fetch_all(&mut *tx)
            .await?;
        if let Some(result) = replay_room_message_delete_in_tx(
            &mut tx,
            room_id,
            deleting_actor,
            client_command_id,
            &payload_sha256,
        )
        .await?
        {
            tx.commit().await?;
            return Ok(result);
        }
        let (room_kind, requester_role) =
            active_room_access(&mut tx, room_id, deleting_actor).await?;

        let message = sqlx::query(
            "SELECT from_actor,deleted_at FROM messages \
             WHERE room_id=$1 AND id=$2 FOR UPDATE",
        )
        .bind(room_id)
        .bind(message_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| OrgIntelError::InvalidRoom("Room message does not exist".into()))?;
        let author_actor: String = message.get("from_actor");
        let may_moderate =
            room_kind != RoomKind::Direct && requester_role == RoomParticipantRole::Owner;
        if author_actor != deleting_actor && !may_moderate {
            return Err(OrgIntelError::RoomAccessDenied(
                "only the Message author may delete it in a direct Room; other Rooms also permit their owner to moderate".into(),
            ));
        }
        if message
            .get::<Option<DateTime<Utc>>, _>("deleted_at")
            .is_some()
        {
            return Err(OrgIntelError::RoomCommandConflict(
                "the Room Message already has a deletion tombstone".into(),
            ));
        }

        let tombstone_id = Uuid::new_v4();
        let created_event_id = append_room_event(
            &mut tx,
            "room.message.deleted.v1",
            room_id,
            deleting_actor,
            Some(message_id),
            serde_json::json!({
                "message_id": message_id,
                "tombstone_id": tombstone_id,
            }),
        )
        .await?;
        let tombstone: RoomMessageTombstoneRow = sqlx::query_as(
            "INSERT INTO room_message_tombstones \
             (id,room_id,message_id,deleted_by_actor_id,client_command_id, \
              client_payload_sha256,created_event_id) \
             VALUES ($1,$2,$3,$4,$5,$6,$7) \
             RETURNING id,room_id,message_id,deleted_by_actor_id,created_event_id,created_at",
        )
        .bind(tombstone_id)
        .bind(room_id)
        .bind(message_id)
        .bind(deleting_actor)
        .bind(client_command_id)
        .bind(&payload_sha256)
        .bind(created_event_id)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE messages SET current_plain_text='',deleted_at=$3,deleted_by_actor_id=$4 \
             WHERE room_id=$1 AND id=$2",
        )
        .bind(room_id)
        .bind(message_id)
        .bind(tombstone.created_at)
        .bind(deleting_actor)
        .execute(&mut *tx)
        .await?;

        let pending_mentions: Vec<(Uuid, String)> = sqlx::query_as(
            "SELECT id,mentioned_actor_id FROM message_mentions \
             WHERE room_id=$1 AND message_id=$2 AND resolution_message_id IS NULL \
               AND cancelled_event_id IS NULL ORDER BY mentioned_actor_id FOR UPDATE",
        )
        .bind(room_id)
        .bind(message_id)
        .fetch_all(&mut *tx)
        .await?;
        for (mention_id, mentioned_actor_id) in pending_mentions {
            sqlx::query(
                "UPDATE actor_cognitive_leases \
                 SET revoked_at=COALESCE(revoked_at,now()),revoked_by=$3,revocation_reason=$4 \
                 WHERE actor_id=$1 AND focused_mention_id=$2 AND claimed_until>now()",
            )
            .bind(&mentioned_actor_id)
            .bind(mention_id)
            .bind(deleting_actor)
            .bind("the source Room Message was deleted")
            .execute(&mut *tx)
            .await?;
            let cancelled_event_id = append_room_event(
                &mut tx,
                "room.mention.cancelled.v1",
                room_id,
                deleting_actor,
                Some(message_id),
                serde_json::json!({
                    "mention_id": mention_id,
                    "mentioned_actor_id": mentioned_actor_id,
                    "reason": "the source Room Message was deleted",
                }),
            )
            .await?;
            sqlx::query(
                "UPDATE message_mentions SET cancelled_event_id=$2,cancelled_by=$3, \
                        cancellation_reason=$4,cancelled_at=$5,claim_token=NULL, \
                        claimed_at=NULL,claimed_until=NULL \
                 WHERE id=$1 AND resolution_message_id IS NULL AND cancelled_event_id IS NULL",
            )
            .bind(mention_id)
            .bind(cancelled_event_id)
            .bind(deleting_actor)
            .bind("the source Room Message was deleted")
            .bind(tombstone.created_at)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(RoomMessageDeleteResult {
            tombstone,
            created: true,
        })
    }

    /// Bounded immutable revision history for one accessible, live Message.
    /// A tombstone removes every body-bearing normal projection; immutable
    /// rows remain available only to future explicit audit authority.
    pub async fn room_message_revisions_before(
        &self,
        requesting_actor: &str,
        room_id: Uuid,
        message_id: i64,
        before_revision_number: Option<i64>,
        limit: i64,
    ) -> Result<RoomMessageRevisionPage> {
        if !(1..=MAX_ROOM_MESSAGE_REVISION_LIMIT).contains(&limit) {
            return Err(OrgIntelError::InvalidRoom(format!(
                "Room Message revision limit must be between 1 and {MAX_ROOM_MESSAGE_REVISION_LIMIT}"
            )));
        }
        if before_revision_number.is_some_and(|value| value <= 0) {
            return Err(OrgIntelError::InvalidRoom(
                "a Room Message revision cursor must be positive".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        active_room_access(&mut tx, room_id, requesting_actor).await?;
        let message = room_message_in_tx(&mut tx, room_id, message_id).await?;
        if message.deleted_at.is_some() {
            return Err(OrgIntelError::InvalidRoom(
                "revision history is unavailable for a deleted Room Message".into(),
            ));
        }
        let mut revisions: Vec<RoomMessageRevisionRow> = sqlx::query_as(
            "SELECT id,room_id,message_id,revision_number,editor_actor_id,body, \
                    created_event_id,created_at \
             FROM room_message_revisions \
             WHERE room_id=$1 AND message_id=$2 \
               AND revision_number<COALESCE($3,9223372036854775807) \
             ORDER BY revision_number DESC LIMIT $4",
        )
        .bind(room_id)
        .bind(message_id)
        .bind(before_revision_number)
        .bind(limit + 1)
        .fetch_all(&mut *tx)
        .await?;
        let has_more = revisions.len() as i64 > limit;
        if has_more {
            revisions.truncate(limit as usize);
        }
        let next_before_revision_number = has_more.then(|| {
            revisions
                .last()
                .expect("a page with more revisions is non-empty")
                .revision_number
        });
        tx.commit().await?;
        Ok(RoomMessageRevisionPage {
            revisions,
            next_before_revision_number,
            has_more,
        })
    }

    /// Search current, non-deleted Message text across only the Rooms visible
    /// to one active Actor. The immutable Message id is the newest-first keyset
    /// cursor; query text never grants Room access.
    pub async fn search_room_messages(
        &self,
        requesting_actor: &str,
        query: &str,
        before_message_id: Option<i64>,
        limit: i64,
    ) -> Result<RoomMessageSearchPage> {
        let query = query.trim();
        if query.is_empty() || query.len() > MAX_ROOM_SEARCH_QUERY_BYTES {
            return Err(OrgIntelError::InvalidRoom(format!(
                "a Room search query needs 1 to {MAX_ROOM_SEARCH_QUERY_BYTES} bytes"
            )));
        }
        if !(1..=MAX_ROOM_SEARCH_LIMIT).contains(&limit) {
            return Err(OrgIntelError::InvalidRoom(format!(
                "Room search limit must be between 1 and {MAX_ROOM_SEARCH_LIMIT}"
            )));
        }
        if before_message_id.is_some_and(|value| value <= 0) {
            return Err(OrgIntelError::InvalidRoom(
                "a Room search cursor must be positive".into(),
            ));
        }

        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT id FROM actors WHERE id=$1 AND retired_at IS NULL FOR SHARE")
            .bind(requesting_actor)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                OrgIntelError::RoomAccessDenied("Room search requires one active Actor".into())
            })?;
        let mut messages: Vec<RoomMessageRow> = sqlx::query_as(
            "SELECT message.id,message.room_id,message.from_actor,message.to_actor, \
                    LEFT(COALESCE(revision.body,message.body),$5) AS body,message.outcome_standard, \
                    message.parent_message_id,message.thread_root_message_id, \
                    message.client_command_id,message.created_at, \
                    COALESCE(revision.revision_number,0) AS revision_number,message.edited_at, \
                    message.deleted_at,message.read_at AS legacy_read_at \
             FROM messages message \
             JOIN rooms room ON room.id=message.room_id AND room.archived_at IS NULL \
             JOIN room_participants participant ON participant.room_id=room.id \
               AND participant.actor_id=$1 AND participant.left_at IS NULL \
             LEFT JOIN room_message_revisions revision ON revision.id=message.latest_revision_id \
             WHERE message.deleted_at IS NULL \
               AND message.id<COALESCE($3,9223372036854775807) \
               AND to_tsvector('simple',message.current_plain_text) \
                     @@ websearch_to_tsquery('simple',$2) \
             ORDER BY message.id DESC LIMIT $4",
        )
        .bind(requesting_actor)
        .bind(query)
        .bind(before_message_id)
        .bind(limit + 1)
        .bind(MAX_ROOM_SEARCH_SNIPPET_CHARACTERS)
        .fetch_all(&mut *tx)
        .await?;
        let has_more = messages.len() as i64 > limit;
        if has_more {
            messages.truncate(limit as usize);
        }
        let next_before_message_id = has_more.then(|| {
            messages
                .last()
                .expect("a search page with more results is non-empty")
                .id
        });
        tx.commit().await?;
        Ok(RoomMessageSearchPage {
            messages,
            next_before_message_id,
            has_more,
        })
    }

    /// Keyset pagination over Room messages. A normal read opens on the most
    /// recent bounded page; `before_message_id` walks backwards through older
    /// history. Each returned page remains chronological for direct rendering.
    /// A cursor is only a watermark; it grants no access.
    pub async fn room_messages_before(
        &self,
        requesting_actor: &str,
        room_id: Uuid,
        before_message_id: Option<i64>,
        limit: i64,
    ) -> Result<RoomMessagePage> {
        self.room_message_page(requesting_actor, room_id, before_message_id, limit, None)
            .await
    }

    /// One reply tree. Passing any member of the tree resolves its canonical
    /// root before the page is read.
    pub async fn room_thread_before(
        &self,
        requesting_actor: &str,
        room_id: Uuid,
        thread_message_id: i64,
        before_message_id: Option<i64>,
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
            before_message_id,
            limit,
            Some(root_message_id),
        )
        .await
    }

    async fn room_message_page(
        &self,
        requesting_actor: &str,
        room_id: Uuid,
        before_message_id: Option<i64>,
        limit: i64,
        thread_root_message_id: Option<i64>,
    ) -> Result<RoomMessagePage> {
        let limit = limit.clamp(1, 100);
        let mut tx = self.pool.begin().await?;
        active_room_kind(&mut tx, room_id, requesting_actor).await?;
        let (messages, has_more, next_before_message_id) = if let Some(thread_root_message_id) =
            thread_root_message_id
        {
            // The root is a context anchor, not part of the reply page. It
            // is therefore returned on every page while the cursor walks
            // only the replies. Clients may deduplicate it by stable id.
            let root = room_message_in_tx(&mut tx, room_id, thread_root_message_id).await?;
            let mut replies: Vec<RoomMessageRow> = sqlx::query_as(
                    "SELECT message.id,message.room_id,message.from_actor,message.to_actor, \
                        CASE WHEN message.deleted_at IS NULL \
                             THEN COALESCE(revision.body,message.body) ELSE '' END AS body, \
                        message.outcome_standard,message.parent_message_id, \
                        message.thread_root_message_id,message.client_command_id, \
                        message.created_at,COALESCE(revision.revision_number,0) AS revision_number, \
                        message.edited_at,message.deleted_at, \
                        message.read_at AS legacy_read_at \
                     FROM messages message \
                     LEFT JOIN room_message_revisions revision \
                       ON revision.id=message.latest_revision_id \
                     WHERE message.room_id=$1 \
                       AND message.id<COALESCE($2,9223372036854775807) \
                       AND message.thread_root_message_id=$3 \
                     ORDER BY message.id DESC LIMIT $4",
                )
                .bind(room_id)
                .bind(before_message_id)
                .bind(thread_root_message_id)
                .bind(limit + 1)
                .fetch_all(&mut *tx)
                .await?;
            let has_more = replies.len() as i64 > limit;
            if has_more {
                replies.truncate(limit as usize);
            }
            replies.reverse();
            let next_before_message_id = has_more.then(|| replies[0].id);
            let mut messages = Vec::with_capacity(replies.len() + 1);
            messages.push(root);
            messages.extend(replies);
            (messages, has_more, next_before_message_id)
        } else {
            // A Room opens on recent top-level conversation. Replies are
            // read through their Thread so a page containing only replies
            // can never render as an apparently empty Room.
            let mut roots: Vec<RoomMessageRow> = sqlx::query_as(
                    "SELECT message.id,message.room_id,message.from_actor,message.to_actor, \
                        CASE WHEN message.deleted_at IS NULL \
                             THEN COALESCE(revision.body,message.body) ELSE '' END AS body, \
                        message.outcome_standard,message.parent_message_id, \
                        message.thread_root_message_id,message.client_command_id, \
                        message.created_at,COALESCE(revision.revision_number,0) AS revision_number, \
                        message.edited_at,message.deleted_at, \
                        message.read_at AS legacy_read_at \
                     FROM messages message \
                     LEFT JOIN room_message_revisions revision \
                       ON revision.id=message.latest_revision_id \
                     WHERE message.room_id=$1 \
                       AND message.id<COALESCE($2,9223372036854775807) \
                       AND message.parent_message_id IS NULL \
                     ORDER BY message.id DESC LIMIT $3",
                )
                .bind(room_id)
                .bind(before_message_id)
                .bind(limit + 1)
                .fetch_all(&mut *tx)
                .await?;
            let has_more = roots.len() as i64 > limit;
            if has_more {
                roots.truncate(limit as usize);
            }
            roots.reverse();
            let next_before_message_id = has_more.then(|| roots[0].id);
            (roots, has_more, next_before_message_id)
        };
        let message_ids = messages
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>();
        let mentions = message_mentions_for_messages_in_tx(&mut tx, &message_ids).await?;
        tx.commit().await?;
        Ok(RoomMessagePage {
            messages,
            mentions,
            next_before_message_id,
            has_more,
        })
    }

    /// Unresolved recipient-relative Attention for one durable Actor. Current
    /// Room participation is joined on every read: mention ids and old event
    /// cursors never become access capabilities after a participant leaves.
    pub async fn pending_message_mentions_for_actor(
        &self,
        actor_id: &str,
        after_created_event_id: i64,
        limit: i64,
    ) -> Result<Vec<MessageMentionContext>> {
        if after_created_event_id < 0 || !(1..=100).contains(&limit) {
            return Err(OrgIntelError::InvalidEventCursor(
                "mention cursor must be non-negative and limit must be between 1 and 100".into(),
            ));
        }
        let mut tx = self.pool.begin().await?;
        let mention_rows: Vec<MessageMentionRow> = sqlx::query_as(
            "SELECT mention.id,mention.room_id,mention.message_id,mention.thread_root_message_id, \
                    mention.mentioned_actor_id,mention.kind,mention.work_id,mention.why_this_actor, \
                    mention.expected_response,mention.recommendation,mention.alternatives, \
                    mention.evidence,mention.uncertainty,mention.affected_scope,mention.deadline_at, \
                    mention.fallback,mention.independent_work_can_continue,mention.created_event_id, \
                    mention.resolution_message_id,mention.resolved_event_id, \
                    mention.cancelled_event_id,mention.cancelled_by,mention.cancellation_reason, \
                    mention.created_at,mention.resolved_at,mention.cancelled_at \
             FROM message_mentions mention \
             JOIN room_participants participant \
               ON participant.room_id=mention.room_id \
              AND participant.actor_id=mention.mentioned_actor_id \
             JOIN actors actor ON actor.id=mention.mentioned_actor_id \
             JOIN rooms room ON room.id=mention.room_id \
             WHERE mention.mentioned_actor_id=$1 \
               AND mention.resolution_message_id IS NULL \
               AND mention.cancelled_event_id IS NULL \
               AND mention.created_event_id>$2 \
               AND participant.left_at IS NULL AND actor.retired_at IS NULL \
               AND room.archived_at IS NULL \
             ORDER BY mention.created_event_id LIMIT $3 \
             FOR SHARE OF participant,actor,room",
        )
        .bind(actor_id)
        .bind(after_created_event_id)
        .bind(limit)
        .fetch_all(&mut *tx)
        .await?;

        if mention_rows.is_empty() {
            tx.commit().await?;
            return Ok(Vec::new());
        }
        let message_ids = mention_rows
            .iter()
            .map(|mention| mention.message_id)
            .collect::<Vec<_>>();
        let room_ids = mention_rows
            .iter()
            .map(|mention| mention.room_id)
            .collect::<Vec<_>>();
        let messages: Vec<RoomMessageRow> = sqlx::query_as(
            "SELECT message.id,message.room_id,message.from_actor,message.to_actor, \
                    CASE WHEN message.deleted_at IS NULL \
                         THEN COALESCE(revision.body,message.body) ELSE '' END AS body, \
                    message.outcome_standard,message.parent_message_id, \
                    message.thread_root_message_id,message.client_command_id, \
                    message.created_at,COALESCE(revision.revision_number,0) AS revision_number, \
                    message.edited_at,message.deleted_at, \
                    message.read_at AS legacy_read_at \
             FROM messages message \
             LEFT JOIN room_message_revisions revision ON revision.id=message.latest_revision_id \
             WHERE message.id=ANY($1)",
        )
        .bind(&message_ids)
        .fetch_all(&mut *tx)
        .await?;
        let rooms: Vec<(Uuid, String)> =
            sqlx::query_as("SELECT id,title FROM rooms WHERE id=ANY($1)")
                .bind(&room_ids)
                .fetch_all(&mut *tx)
                .await?;
        tx.commit().await?;

        let mut messages = messages
            .into_iter()
            .map(|message| (message.id, message))
            .collect::<HashMap<_, _>>();
        let rooms = rooms.into_iter().collect::<HashMap<_, _>>();
        mention_rows
            .into_iter()
            .map(|mention| {
                let message = messages.remove(&mention.message_id).ok_or_else(|| {
                    OrgIntelError::InvalidRoom(
                        "a durable mention lost its authoritative Message".into(),
                    )
                })?;
                let room_title = rooms.get(&mention.room_id).cloned().ok_or_else(|| {
                    OrgIntelError::InvalidRoom("a durable mention lost its Room".into())
                })?;
                Ok(MessageMentionContext {
                    mention,
                    room_title,
                    message,
                })
            })
            .collect()
    }

    pub async fn next_pending_message_mention(
        &self,
        actor_id: &str,
    ) -> Result<Option<MessageMentionContext>> {
        Ok(self
            .pending_message_mentions_for_actor(actor_id, 0, 1)
            .await?
            .into_iter()
            .next())
    }

    /// Active agent Actors that still owe at least one explicit named-Actor
    /// Room mention. This includes a former lead: a lead change changes an
    /// office, not the durable identity named by an already-accepted mention.
    /// The runtime still claims the Actor-wide lease before reading or acting.
    pub async fn actors_owing_message_mentions(&self, limit: i64) -> Result<Vec<String>> {
        let limit = limit.clamp(1, 128);
        Ok(sqlx::query_scalar(
            "SELECT mention.mentioned_actor_id \
             FROM message_mentions mention \
             JOIN actors actor ON actor.id=mention.mentioned_actor_id \
             JOIN room_participants participant \
               ON participant.room_id=mention.room_id \
              AND participant.actor_id=mention.mentioned_actor_id \
             JOIN rooms room ON room.id=mention.room_id \
             WHERE mention.resolution_message_id IS NULL \
               AND mention.cancelled_event_id IS NULL \
               AND actor.actor_class='agent' AND actor.retired_at IS NULL \
               AND participant.left_at IS NULL AND room.archived_at IS NULL \
             GROUP BY mention.mentioned_actor_id \
             ORDER BY MIN(mention.created_event_id),mention.mentioned_actor_id LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?)
    }

    /// Claim the Actor-wide conversation boundary. This row is deliberately a
    /// lease, not audit history or another inbox: the one authoritative inputs
    /// remain Messages, mentions and handoffs. Work claims and this method lock
    /// the same Actor row before checking the other's durable ownership fact.
    pub async fn claim_actor_cognitive_session(
        &self,
        actor_id: &str,
        lease_for: std::time::Duration,
    ) -> Result<Option<ActorCognitiveLease>> {
        self.claim_actor_cognitive_session_internal(actor_id, None, lease_for)
            .await
    }

    /// Team-lead conversation claim. The team row is locked before the Actor,
    /// matching replacement/disband lifecycle order, so a dispatcher that
    /// observed an old lead cannot acquire a lease after that role changed.
    pub async fn claim_team_lead_cognitive_session(
        &self,
        actor_id: &str,
        team_id: Uuid,
        lease_for: std::time::Duration,
    ) -> Result<Option<ActorCognitiveLease>> {
        self.claim_actor_cognitive_session_internal(actor_id, Some(team_id), lease_for)
            .await
    }

    async fn claim_actor_cognitive_session_internal(
        &self,
        actor_id: &str,
        expected_lead_team_id: Option<Uuid>,
        lease_for: std::time::Duration,
    ) -> Result<Option<ActorCognitiveLease>> {
        let lease_seconds = mention_lease_seconds(lease_for)?;
        let mut tx = self.pool.begin().await?;
        if let Some(team_id) = expected_lead_team_id {
            let current_lead: Option<String> = sqlx::query_scalar(
                "SELECT lead_actor_id FROM teams \
                 WHERE id=$1 AND disbanded_at IS NULL FOR SHARE",
            )
            .bind(team_id)
            .fetch_optional(&mut *tx)
            .await?;
            if current_lead.as_deref() != Some(actor_id) {
                tx.commit().await?;
                return Ok(None);
            }
        }
        let actor_available: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors \
             WHERE id=$1 AND retired_at IS NULL AND actor_class='agent' FOR UPDATE)",
        )
        .bind(actor_id)
        .fetch_one(&mut *tx)
        .await?;
        if !actor_available {
            tx.commit().await?;
            return Ok(None);
        }

        // A revoked token is already unable to renew or persist. Reclaim it
        // immediately under the Actor lock so an owner interrupt or role
        // transition does not impose the old lease's remaining TTL on the
        // next valid conversation. Expired rows follow the same path.
        let expired_or_revoked: Option<(Uuid, Option<Uuid>)> = sqlx::query_as(
            "DELETE FROM actor_cognitive_leases \
             WHERE actor_id=$1 AND (claimed_until<=now() OR revoked_at IS NOT NULL) \
             RETURNING lease_token,focused_mention_id",
        )
        .bind(actor_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((prior_token, _)) = expired_or_revoked {
            sqlx::query(
                "UPDATE message_mentions SET claim_token=NULL,claimed_at=NULL,claimed_until=NULL \
                 WHERE mentioned_actor_id=$1 AND claim_token=$2 \
                   AND resolution_message_id IS NULL AND cancelled_event_id IS NULL",
            )
            .bind(actor_id)
            .bind(prior_token)
            .execute(&mut *tx)
            .await?;
        }

        let attempt_running: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM work_attempts \
             WHERE actor_id=$1 AND state='running')",
        )
        .bind(actor_id)
        .fetch_one(&mut *tx)
        .await?;
        if attempt_running {
            tx.commit().await?;
            return Ok(None);
        }
        let token = Uuid::new_v4();
        let claimed_until: Option<DateTime<Utc>> = sqlx::query_scalar(
            "INSERT INTO actor_cognitive_leases \
             (actor_id,lease_token,claimed_at,claimed_until) \
             VALUES ($1,$2,now(),now()+make_interval(secs=>$3)) \
             ON CONFLICT (actor_id) DO NOTHING RETURNING claimed_until",
        )
        .bind(actor_id)
        .bind(token)
        .bind(lease_seconds as f64)
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(claimed_until.map(|claimed_until| ActorCognitiveLease {
            actor_id: actor_id.to_string(),
            token,
            claimed_until,
        }))
    }

    /// Extend an exact live Actor lease. Losing the token, expiry, retirement,
    /// Room participation, or the Work exclusion makes renewal fail closed;
    /// the caller must cancel its cognitive process and may not persist output.
    pub async fn renew_actor_cognitive_session(
        &self,
        lease: &ActorCognitiveLease,
        lease_for: std::time::Duration,
    ) -> Result<ActorCognitiveLease> {
        let lease_seconds = mention_lease_seconds(lease_for)?;
        let mut tx = self.pool.begin().await?;
        // Native document operations lock Document before Actor. Take the same
        // order before extending an Actor lease focused on a document comment.
        sqlx::query("SELECT d.id FROM native_documents d JOIN native_document_mentions m ON m.document_id=d.id JOIN actor_cognitive_leases l ON l.focused_document_mention_id=m.id WHERE l.actor_id=$1 AND l.lease_token=$2 FOR SHARE OF d")
            .bind(&lease.actor_id).bind(lease.token).fetch_optional(&mut *tx).await?;
        let actor_available: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actors \
             WHERE id=$1 AND retired_at IS NULL AND actor_class='agent' FOR UPDATE)",
        )
        .bind(&lease.actor_id)
        .fetch_one(&mut *tx)
        .await?;
        let focused_mention_id: Option<Uuid> = if actor_available {
            sqlx::query_scalar::<_, Option<Uuid>>(
                "SELECT focused_mention_id FROM actor_cognitive_leases \
                 WHERE actor_id=$1 AND lease_token=$2 AND claimed_until>now() \
                   AND revoked_at IS NULL FOR UPDATE",
            )
            .bind(&lease.actor_id)
            .bind(lease.token)
            .fetch_optional(&mut *tx)
            .await?
            .flatten()
        } else {
            None
        };
        let owns_live_lease: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM actor_cognitive_leases \
             WHERE actor_id=$1 AND lease_token=$2 AND claimed_until>now() \
               AND revoked_at IS NULL)",
        )
        .bind(&lease.actor_id)
        .bind(lease.token)
        .fetch_one(&mut *tx)
        .await?;
        let attempt_running: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM work_attempts \
             WHERE actor_id=$1 AND state='running')",
        )
        .bind(&lease.actor_id)
        .fetch_one(&mut *tx)
        .await?;
        let focus_is_current = match focused_mention_id {
            Some(mention_id) => {
                sqlx::query_scalar::<_, bool>(
                    "SELECT EXISTS( \
                       SELECT 1 FROM message_mentions mention \
                       JOIN room_participants participant \
                         ON participant.room_id=mention.room_id \
                        AND participant.actor_id=mention.mentioned_actor_id \
                       JOIN rooms room ON room.id=mention.room_id \
                       WHERE mention.id=$1 AND mention.mentioned_actor_id=$2 \
                         AND mention.claim_token=$3 AND mention.resolution_message_id IS NULL \
                         AND mention.cancelled_event_id IS NULL AND participant.left_at IS NULL \
                         AND room.archived_at IS NULL)",
                )
                .bind(mention_id)
                .bind(&lease.actor_id)
                .bind(lease.token)
                .fetch_one(&mut *tx)
                .await?
            }
            None => true,
        };
        let document_focus_current =
            crate::documents::document_mention_lease_current(&mut tx, lease)
                .await
                .map_err(|error| OrgIntelError::RoomCommandConflict(error.to_string()))?;
        if !actor_available
            || !owns_live_lease
            || attempt_running
            || !focus_is_current
            || !document_focus_current
        {
            tx.commit().await?;
            return Err(OrgIntelError::RoomCommandConflict(
                "the Actor cognitive-session lease is no longer current".into(),
            ));
        }

        let claimed_until: DateTime<Utc> = sqlx::query_scalar(
            "UPDATE actor_cognitive_leases \
             SET claimed_until=now()+make_interval(secs=>$3) \
             WHERE actor_id=$1 AND lease_token=$2 AND revoked_at IS NULL \
             RETURNING claimed_until",
        )
        .bind(&lease.actor_id)
        .bind(lease.token)
        .bind(lease_seconds as f64)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE message_mentions SET claimed_until=$3 \
             WHERE mentioned_actor_id=$1 AND claim_token=$2 \
               AND resolution_message_id IS NULL AND cancelled_event_id IS NULL",
        )
        .bind(&lease.actor_id)
        .bind(lease.token)
        .bind(claimed_until)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(ActorCognitiveLease {
            actor_id: lease.actor_id.clone(),
            token: lease.token,
            claimed_until,
        })
    }

    /// Release only the exact process token. A crashed process may call this
    /// after a replacement has reclaimed the Actor; it cannot delete the new
    /// owner's lease or claim.
    pub async fn release_actor_cognitive_session(
        &self,
        lease: &ActorCognitiveLease,
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT id FROM actors WHERE id=$1 FOR UPDATE")
            .bind(&lease.actor_id)
            .fetch_optional(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE message_mentions SET claim_token=NULL,claimed_at=NULL,claimed_until=NULL \
             WHERE mentioned_actor_id=$1 AND claim_token=$2 \
               AND resolution_message_id IS NULL AND cancelled_event_id IS NULL",
        )
        .bind(&lease.actor_id)
        .bind(lease.token)
        .execute(&mut *tx)
        .await?;
        let removed =
            sqlx::query("DELETE FROM actor_cognitive_leases WHERE actor_id=$1 AND lease_token=$2")
                .bind(&lease.actor_id)
                .bind(lease.token)
                .execute(&mut *tx)
                .await?;
        tx.commit().await?;
        Ok(removed.rows_affected() == 1)
    }

    /// Fence a live cognitive session after its supervising daemon established
    /// that the session's wake was interrupted. The old token can no longer
    /// renew or commit a reply, allowing the durable unread input to be
    /// admitted by the recovery wake without waiting for its normal lease TTL.
    pub async fn revoke_interrupted_actor_cognitive_session(&self, actor_id: &str) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE actor_cognitive_leases \
             SET revoked_at=now(),revoked_by='exec', \
                 revocation_reason='daemon recovery fenced an interrupted Exec wake' \
             WHERE actor_id=$1 AND claimed_until>now() AND revoked_at IS NULL",
        )
        .bind(actor_id)
        .execute(&self.pool)
        .await?
        .rows_affected()
            > 0)
    }

    /// Bind the oldest currently serviceable mention to an already-owned
    /// Actor lease. Holding the Actor row prevents another replica from
    /// claiming a different mention for the same Actor at the same time.
    pub async fn claim_next_pending_message_mention(
        &self,
        lease: &ActorCognitiveLease,
    ) -> Result<Option<MessageMentionClaim>> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT id FROM actors WHERE id=$1 AND retired_at IS NULL FOR UPDATE")
            .bind(&lease.actor_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| {
                OrgIntelError::RoomCommandConflict(
                    "the mentioned Actor is no longer available".into(),
                )
            })?;
        let live_lease: Option<(DateTime<Utc>, Option<Uuid>)> = sqlx::query_as(
            "SELECT claimed_until,focused_mention_id FROM actor_cognitive_leases \
             WHERE actor_id=$1 AND lease_token=$2 AND claimed_until>now() \
               AND revoked_at IS NULL FOR UPDATE",
        )
        .bind(&lease.actor_id)
        .bind(lease.token)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((claimed_until, prior_focus)) = live_lease else {
            return Err(OrgIntelError::RoomCommandConflict(
                "the Actor cognitive-session lease is no longer current".into(),
            ));
        };
        let document_focus: Option<Uuid> = sqlx::query_scalar("SELECT focused_document_mention_id FROM actor_cognitive_leases WHERE actor_id=$1 AND lease_token=$2")
            .bind(&lease.actor_id).bind(lease.token).fetch_one(&mut *tx).await?;
        if document_focus.is_some() {
            tx.commit().await?;
            return Ok(None);
        }
        if let Some(mention_id) = prior_focus {
            let still_current: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM message_mentions \
                 WHERE id=$1 AND mentioned_actor_id=$2 AND claim_token=$3 \
                   AND resolution_message_id IS NULL AND cancelled_event_id IS NULL \
                   AND claimed_until>now())",
            )
            .bind(mention_id)
            .bind(&lease.actor_id)
            .bind(lease.token)
            .fetch_one(&mut *tx)
            .await?;
            if still_current {
                let context = message_mention_context_in_tx(&mut tx, mention_id).await?;
                tx.commit().await?;
                return Ok(Some(MessageMentionClaim {
                    context,
                    lease: ActorCognitiveLease {
                        actor_id: lease.actor_id.clone(),
                        token: lease.token,
                        claimed_until,
                    },
                }));
            }
            sqlx::query(
                "UPDATE actor_cognitive_leases SET focused_mention_id=NULL \
                 WHERE actor_id=$1 AND lease_token=$2",
            )
            .bind(&lease.actor_id)
            .bind(lease.token)
            .execute(&mut *tx)
            .await?;
        }

        let mention_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT mention.id FROM message_mentions mention \
             JOIN room_participants participant \
               ON participant.room_id=mention.room_id \
              AND participant.actor_id=mention.mentioned_actor_id \
             JOIN rooms room ON room.id=mention.room_id \
             WHERE mention.mentioned_actor_id=$1 \
               AND mention.resolution_message_id IS NULL AND mention.cancelled_event_id IS NULL \
               AND (mention.claim_token IS NULL OR mention.claimed_until<=now()) \
               AND participant.left_at IS NULL AND room.archived_at IS NULL \
             ORDER BY mention.created_event_id FOR UPDATE OF mention SKIP LOCKED LIMIT 1",
        )
        .bind(&lease.actor_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(mention_id) = mention_id else {
            tx.commit().await?;
            return Ok(None);
        };
        sqlx::query(
            "UPDATE message_mentions SET claim_token=$2,claimed_at=now(),claimed_until=$3 \
             WHERE id=$1",
        )
        .bind(mention_id)
        .bind(lease.token)
        .bind(claimed_until)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "UPDATE actor_cognitive_leases SET focused_mention_id=$3 \
             WHERE actor_id=$1 AND lease_token=$2",
        )
        .bind(&lease.actor_id)
        .bind(lease.token)
        .bind(mention_id)
        .execute(&mut *tx)
        .await?;
        let context = message_mention_context_in_tx(&mut tx, mention_id).await?;
        tx.commit().await?;
        Ok(Some(MessageMentionClaim {
            context,
            lease: ActorCognitiveLease {
                actor_id: lease.actor_id.clone(),
                token: lease.token,
                claimed_until,
            },
        }))
    }

    /// Runtime bridge convenience for the one-at-a-time mention path. The
    /// stable command id makes a lost in-process receipt safe to retry; the
    /// ordinary send invariant still rechecks current participation and exact
    /// recipient/Room/Thread resolution inside one transaction.
    pub async fn reply_to_message_mention(
        &self,
        actor_id: &str,
        mention: &MessageMentionContext,
        body: &str,
    ) -> Result<RoomMessageSendResult> {
        self.send_room_message_with_mentions(
            mention.mention.room_id,
            actor_id,
            body,
            Some(mention.message.id),
            &format!("runtime-mention-reply:{}", mention.mention.id),
            None,
            &[],
            Some(mention.mention.id),
        )
        .await
    }

    /// Runtime-only exact reply path. The Actor-wide lease token is checked in
    /// the same transaction that creates the Message and resolves its mention.
    pub async fn reply_to_claimed_message_mention(
        &self,
        claim: &MessageMentionClaim,
        body: &str,
    ) -> Result<RoomMessageSendResult> {
        self.send_room_message_with_mention_claim(
            claim.context.mention.room_id,
            &claim.lease.actor_id,
            body,
            Some(claim.context.message.id),
            &format!("runtime-mention-reply:{}", claim.context.mention.id),
            None,
            &[],
            Some(claim.context.mention.id),
            Some(claim.lease.token),
        )
        .await
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

    /// Oldest-first, bounded replay of body-free event hints for one Room.
    ///
    /// The cursor belongs to the existing company-wide operational stream,
    /// but the returned rows never do: active participation is proved before
    /// the stream is read and every selected event is constrained to the exact
    /// Room. Delivery remains at-least-once; consumers deduplicate stable event
    /// ids and refetch authoritative projections for every hint.
    pub async fn room_events_after(
        &self,
        requesting_actor: &str,
        room_id: Uuid,
        after_event_id: i64,
        limit: i64,
    ) -> Result<RoomEventReplayPage> {
        if after_event_id < 0 {
            return Err(OrgIntelError::InvalidEventCursor(
                "after_event_id must be non-negative".into(),
            ));
        }
        if !(1..=MAX_EVENT_REPLAY_LIMIT).contains(&limit) {
            return Err(OrgIntelError::InvalidEventCursor(format!(
                "Room event replay limit must be between 1 and {MAX_EVENT_REPLAY_LIMIT}"
            )));
        }

        let mut tx = self.pool.begin().await?;
        // Match Room writer lock order before capturing a stable committed
        // prefix. A guessed id and a former participant fail here and learn no
        // cursor, retention or event metadata about the Room.
        active_room_kind(&mut tx, room_id, requesting_actor).await?;
        // Every Room writer takes this row FOR UPDATE before its event identity
        // is allocated. Holding it FOR SHARE therefore exposes a committed
        // prefix for this Room without stopping writers in any other Room.
        let (snapshot_cursor, compacted_through_event_id): (i64, i64) = sqlx::query_as(
            "SELECT last_event_id,compacted_through_event_id \
             FROM room_event_streams WHERE room_id=$1 FOR SHARE",
        )
        .bind(room_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            OrgIntelError::InvalidRoom("an active Room lost its event stream watermark".into())
        })?;
        let oldest_available_event_id: Option<i64> = sqlx::query_scalar(
            "SELECT MIN(id) FROM events \
             WHERE room_id=$1 AND id>$2 \
               AND (kind<>'room.read_cursor.advanced.v1' OR actor_id=$3)",
        )
        .bind(room_id)
        .bind(compacted_through_event_id)
        .bind(requesting_actor)
        .fetch_one(&mut *tx)
        .await?;
        let resync_required =
            after_event_id < compacted_through_event_id || after_event_id > snapshot_cursor;

        let mut events = if resync_required {
            Vec::new()
        } else {
            sqlx::query_as::<_, CollaborationEventRow>(
                "SELECT event.id,event.kind,event.room_id,event.actor_id, \
                        event.message_id,event.created_at \
                 FROM events event \
                 WHERE event.room_id=$1 AND event.id>$2 AND event.id>$3 AND event.id<=$4 \
                   AND (event.kind<>'room.read_cursor.advanced.v1' OR event.actor_id=$5) \
                 ORDER BY event.id LIMIT $6",
            )
            .bind(room_id)
            .bind(after_event_id)
            .bind(compacted_through_event_id)
            .bind(snapshot_cursor)
            .bind(requesting_actor)
            .bind(limit + 1)
            .fetch_all(&mut *tx)
            .await?
        };
        tx.commit().await?;

        let has_more = events.len() as i64 > limit;
        if has_more {
            events.truncate(limit as usize);
        }
        let next_after_event_id = if resync_required {
            after_event_id
        } else if has_more {
            events
                .last()
                .expect("a page with another row contains the requested prefix")
                .id
        } else {
            // The Room watermark proves there is no omitted visible event at
            // or below this Room's committed prefix. Hidden read-cursor hints
            // can be skipped because they can never become visible later.
            snapshot_cursor
        };
        Ok(RoomEventReplayPage {
            events,
            requested_after_event_id: after_event_id,
            next_after_event_id,
            snapshot_cursor,
            compacted_through_event_id,
            oldest_available_event_id,
            has_more,
            resync_required,
        })
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
                    event.message_id,event.created_at \
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
