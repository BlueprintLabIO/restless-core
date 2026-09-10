-- External notification delivery must bind one immutable source epoch. A
-- content hash is not an epoch: A -> B -> A would otherwise reuse the first
-- delivery identity. Keep the monotonic counter on the canonical handoff row
-- and snapshot human presentation that lives outside the Mention/Handoff row
-- so later renames cannot mutate an already-emitted intent.

ALTER TABLE owner_handoffs
  ADD COLUMN notification_revision BIGINT NOT NULL DEFAULT 1
    CHECK (notification_revision > 0),
  ADD COLUMN notification_source_display TEXT,
  ADD COLUMN notification_source_user_id TEXT;

UPDATE owner_handoffs handoff
SET notification_source_display=actor.display,
    notification_source_user_id=(
      SELECT binding.subject
      FROM human_principal_actor_bindings binding
      WHERE binding.actor_id=handoff.requested_by
        AND binding.membership_status='active'
      ORDER BY binding.membership_id
      LIMIT 1
    )
FROM actors actor
WHERE actor.id=handoff.requested_by;

ALTER TABLE owner_handoffs
  ALTER COLUMN notification_source_display SET NOT NULL,
  ADD CONSTRAINT owner_handoff_notification_source_display_bounded
    CHECK (octet_length(notification_source_display) BETWEEN 1 AND 600),
  ADD CONSTRAINT owner_handoff_notification_source_user_bounded
    CHECK (notification_source_user_id IS NULL
      OR octet_length(notification_source_user_id) BETWEEN 1 AND 512);

CREATE FUNCTION orgintel_owner_handoff_notification_epoch() RETURNS trigger AS $$
BEGIN
  IF TG_OP = 'INSERT' THEN
    SELECT actor.display,
           (SELECT binding.subject
              FROM human_principal_actor_bindings binding
             WHERE binding.actor_id=NEW.requested_by
               AND binding.membership_status='active'
             ORDER BY binding.membership_id
             LIMIT 1)
      INTO NEW.notification_source_display,NEW.notification_source_user_id
      FROM actors actor WHERE actor.id=NEW.requested_by;
    NEW.notification_revision := 1;
    RETURN NEW;
  END IF;

  IF NEW.requested_by IS DISTINCT FROM OLD.requested_by
     OR NEW.notification_source_display IS DISTINCT FROM OLD.notification_source_display
     OR NEW.notification_source_user_id IS DISTINCT FROM OLD.notification_source_user_id THEN
    RAISE EXCEPTION 'owner handoff notification attribution is immutable'
      USING ERRCODE = 'check_violation';
  END IF;

  IF ROW(NEW.work_id,NEW.attempt_id,NEW.category,NEW.requested_action,
         NEW.prepared_state,NEW.resume_condition,NEW.state,NEW.resolution,
         NEW.assigned_to,NEW.escalated_from,NEW.escalated_at,NEW.owner_brief,
         NEW.briefed_by,NEW.briefed_at,NEW.brief_source_fingerprint,NEW.resolved_at)
     IS DISTINCT FROM
     ROW(OLD.work_id,OLD.attempt_id,OLD.category,OLD.requested_action,
         OLD.prepared_state,OLD.resume_condition,OLD.state,OLD.resolution,
         OLD.assigned_to,OLD.escalated_from,OLD.escalated_at,OLD.owner_brief,
         OLD.briefed_by,OLD.briefed_at,OLD.brief_source_fingerprint,OLD.resolved_at) THEN
    NEW.notification_revision := OLD.notification_revision + 1;
  ELSIF NEW.notification_revision IS DISTINCT FROM OLD.notification_revision THEN
    RAISE EXCEPTION 'owner handoff notification revision is database-maintained'
      USING ERRCODE = 'check_violation';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER owner_handoff_notification_epoch
  BEFORE INSERT OR UPDATE ON owner_handoffs
  FOR EACH ROW EXECUTE FUNCTION orgintel_owner_handoff_notification_epoch();

ALTER TABLE message_mentions
  ADD COLUMN notification_room_title TEXT,
  ADD COLUMN notification_source_display TEXT,
  ADD COLUMN notification_source_user_id TEXT;

UPDATE message_mentions mention
SET notification_room_title=room.title,
    notification_source_display=actor.display,
    notification_source_user_id=(
      SELECT binding.subject
      FROM human_principal_actor_bindings binding
      WHERE binding.actor_id=message.from_actor
        AND binding.membership_status='active'
      ORDER BY binding.membership_id
      LIMIT 1
    )
FROM messages message,rooms room,actors actor
WHERE message.id=mention.message_id
  AND room.id=mention.room_id
  AND actor.id=message.from_actor;

ALTER TABLE message_mentions
  ALTER COLUMN notification_room_title SET NOT NULL,
  ALTER COLUMN notification_source_display SET NOT NULL,
  ADD CONSTRAINT message_mention_notification_room_title_bounded
    CHECK (octet_length(notification_room_title) BETWEEN 1 AND 600),
  ADD CONSTRAINT message_mention_notification_source_display_bounded
    CHECK (octet_length(notification_source_display) BETWEEN 1 AND 600),
  ADD CONSTRAINT message_mention_notification_source_user_bounded
    CHECK (notification_source_user_id IS NULL
      OR octet_length(notification_source_user_id) BETWEEN 1 AND 512);

CREATE FUNCTION orgintel_message_mention_notification_snapshot() RETURNS trigger AS $$
BEGIN
  IF TG_OP = 'INSERT' THEN
    SELECT room.title,actor.display,
           (SELECT binding.subject
              FROM human_principal_actor_bindings binding
             WHERE binding.actor_id=message.from_actor
               AND binding.membership_status='active'
             ORDER BY binding.membership_id
             LIMIT 1)
      INTO NEW.notification_room_title,NEW.notification_source_display,
           NEW.notification_source_user_id
      FROM messages message
      JOIN rooms room ON room.id=message.room_id
      JOIN actors actor ON actor.id=message.from_actor
      WHERE message.id=NEW.message_id AND message.room_id=NEW.room_id;
    RETURN NEW;
  END IF;
  IF NEW.room_id IS DISTINCT FROM OLD.room_id
     OR NEW.message_id IS DISTINCT FROM OLD.message_id
     OR NEW.thread_root_message_id IS DISTINCT FROM OLD.thread_root_message_id
     OR NEW.mentioned_actor_id IS DISTINCT FROM OLD.mentioned_actor_id
     OR NEW.created_event_id IS DISTINCT FROM OLD.created_event_id
     OR NEW.notification_room_title IS DISTINCT FROM OLD.notification_room_title
     OR NEW.notification_source_display IS DISTINCT FROM OLD.notification_source_display
     OR NEW.notification_source_user_id IS DISTINCT FROM OLD.notification_source_user_id THEN
    RAISE EXCEPTION 'message mention notification source is immutable'
      USING ERRCODE = 'check_violation';
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER message_mention_notification_snapshot
  BEFORE INSERT OR UPDATE ON message_mentions
  FOR EACH ROW EXECUTE FUNCTION orgintel_message_mention_notification_snapshot();
