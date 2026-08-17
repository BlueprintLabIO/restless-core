-- S06-T4/T5. A resolver between a staff member and the human.
--
-- The audit that produced this migration: exactly two things reach the owner.
-- Authority `approval_required` rows, which are a real authority boundary and
-- stay with the owner forever; and pending `owner_handoffs`. Of the six handoff
-- categories, five (identity, captcha, mfa, legal_attestation,
-- payment_confirmation) are irreducibly human and have never been used by a live
-- company. Every handoff live Aris has ever raised is `owner_judgement` — 2 of 2.
--
-- And a pending handoff blocks its Work ("awaiting owner handoff <uuid>"). So
-- today every judgement the company cannot make itself stops a Work node until
-- one human answers. At three staff that is a nuisance. At thirty it is the
-- company halting on a person, which is the opposite of the product.
--
-- Nothing in the schema could express "someone other than the owner should
-- answer this", because `request_owner_handoff` had exactly one destination.
-- These three changes give it a second one.

-- A team is coordination state: recoverable, overridable, repairable. It is NOT
-- an authority, budget, credential or approval boundary, and no kernel record
-- gains a team column. A lead cannot approve what its members could not.
CREATE TABLE teams (
    id            UUID PRIMARY KEY,
    name          TEXT NOT NULL,
    brief         TEXT NOT NULL DEFAULT '',
    -- The actor accountable for this team: absorbs coordination and repair
    -- below the Exec, and answers for the team to the owner.
    lead_actor_id TEXT NOT NULL REFERENCES actors(id),
    created_by    TEXT NOT NULL REFERENCES actors(id),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    disbanded_at  TIMESTAMPTZ
);

-- Names are unique among live teams only. A disbanded team keeps its record and
-- stops reserving its name.
CREATE UNIQUE INDEX teams_live_name ON teams (name) WHERE disbanded_at IS NULL;

-- One team per actor, so "who is accountable" has exactly one answer. NULL is a
-- normal, displayable state — unassigned, not a default team. ON DELETE SET NULL
-- because disbanding a team must release its members, never orphan them.
ALTER TABLE actors ADD COLUMN team_id UUID REFERENCES teams(id) ON DELETE SET NULL;
CREATE INDEX actors_team ON actors (team_id) WHERE team_id IS NOT NULL;

-- Who owes this judgement. NULL means the owner, which is what every existing
-- row means and why the column is nullable rather than defaulted to an actor.
--
-- `escalated_from` records the actor that had it before it reached the owner, so
-- a fall-through is visible rather than silent. A lead that quietly swallows
-- escalations is worse than no lead: it is the S05-T7 single-point-of-failure
-- one level down, with the evidence removed.
ALTER TABLE owner_handoffs
  ADD COLUMN assigned_to     TEXT REFERENCES actors(id),
  ADD COLUMN escalated_from  TEXT REFERENCES actors(id),
  ADD COLUMN escalated_at    TIMESTAMPTZ;

CREATE INDEX owner_handoffs_assigned
  ON owner_handoffs (assigned_to) WHERE state = 'pending';

-- The owner queue is now "pending and nobody below me owes it".
CREATE INDEX owner_handoffs_owner_queue
  ON owner_handoffs (created_at) WHERE state = 'pending' AND assigned_to IS NULL;
