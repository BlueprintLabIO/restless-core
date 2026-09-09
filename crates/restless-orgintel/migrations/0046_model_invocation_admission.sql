-- A model call is a consequential, scarce company action even when it was
-- triggered by ordinary collaboration instead of a Work Attempt.  The host
-- spend ledger still owns exact money accounting; this substrate separately
-- bounds how many provider invocations may be launched and survives daemon
-- restarts and competing replicas.

CREATE TYPE model_invocation_kind AS ENUM (
    'work',
    'owner_conversation',
    'room_mention'
);

CREATE TYPE model_invocation_state AS ENUM (
    'admitted',
    'settled',
    'expired'
);

CREATE TYPE model_invocation_outcome AS ENUM (
    'completed',
    'blocked',
    'failed',
    'cancelled'
);

-- One row is both the explicit policy and the company-wide serialization
-- fence.  Since each company has its own schema, locking this singleton makes
-- the company limit exact without a cross-company lock or shared counter.
CREATE TABLE model_invocation_policy (
    singleton                BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    window_seconds           INTEGER NOT NULL DEFAULT 3600
                             CHECK (window_seconds BETWEEN 60 AND 86400),
    company_limit            INTEGER NOT NULL DEFAULT 256
                             CHECK (company_limit BETWEEN 1 AND 10000),
    actor_limit              INTEGER NOT NULL DEFAULT 32
                             CHECK (actor_limit BETWEEN 1 AND company_limit),
    reclaim_after_seconds    INTEGER NOT NULL DEFAULT 1800
                             CHECK (reclaim_after_seconds BETWEEN 60 AND 86400),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO model_invocation_policy (singleton) VALUES (TRUE);

-- The command key is the idempotency boundary for one exact launch intent.
-- A token is returned only to the trusted Runtime and fences settlement.
-- Observability projections deliberately omit it.
CREATE TABLE model_invocation_admissions (
    id                          UUID PRIMARY KEY,
    admission_token             UUID NOT NULL UNIQUE,
    client_command_id           TEXT NOT NULL UNIQUE,
    client_payload_sha256       TEXT NOT NULL,
    actor_id                    TEXT NOT NULL REFERENCES actors(id),
    kind                        model_invocation_kind NOT NULL,
    subject_id                  UUID NOT NULL,
    model                       TEXT NOT NULL,
    harness                     TEXT NOT NULL,
    configured_effort           TEXT NOT NULL,
    work_id                     UUID REFERENCES work(id),
    attempt_id                  UUID REFERENCES work_attempts(id),
    cognitive_lease_token       UUID,
    mention_id                  UUID REFERENCES message_mentions(id),
    window_started_at           TIMESTAMPTZ NOT NULL,
    admitted_at                 TIMESTAMPTZ NOT NULL DEFAULT now(),
    reclaim_after               TIMESTAMPTZ NOT NULL,
    state                       model_invocation_state NOT NULL DEFAULT 'admitted',
    settlement_payload_sha256   TEXT,
    outcome                     model_invocation_outcome,
    evidence                    JSONB,
    settled_at                  TIMESTAMPTZ,
    expired_at                  TIMESTAMPTZ,
    expiry_reason               TEXT,
    CHECK (length(btrim(client_command_id)) BETWEEN 1 AND 200),
    CHECK (client_payload_sha256 ~ '^[0-9a-f]{64}$'),
    CHECK (length(btrim(model)) BETWEEN 1 AND 512),
    CHECK (length(btrim(harness)) BETWEEN 1 AND 64),
    CHECK (length(btrim(configured_effort)) BETWEEN 1 AND 64),
    CHECK (reclaim_after > admitted_at),
    CHECK (
      (kind = 'work'
       AND work_id IS NOT NULL AND attempt_id IS NOT NULL
       AND cognitive_lease_token IS NULL AND mention_id IS NULL
       AND subject_id = attempt_id)
      OR
      (kind = 'owner_conversation'
       AND work_id IS NULL AND attempt_id IS NULL
       AND cognitive_lease_token IS NOT NULL AND mention_id IS NULL
       AND subject_id = cognitive_lease_token)
      OR
      (kind = 'room_mention'
       AND work_id IS NULL AND attempt_id IS NULL
       AND cognitive_lease_token IS NOT NULL AND mention_id IS NOT NULL
       AND subject_id = cognitive_lease_token)
    ),
    CHECK (
      (state = 'admitted'
       AND settlement_payload_sha256 IS NULL AND outcome IS NULL
       AND evidence IS NULL AND settled_at IS NULL
       AND expired_at IS NULL AND expiry_reason IS NULL)
      OR
      (state = 'settled'
       AND settlement_payload_sha256 ~ '^[0-9a-f]{64}$'
       AND outcome IS NOT NULL AND evidence IS NOT NULL
       AND jsonb_typeof(evidence) = 'object'
       AND octet_length(evidence::text) <= 4096
       AND settled_at IS NOT NULL
       AND expired_at IS NULL AND expiry_reason IS NULL)
      OR
      (state = 'expired'
       AND settlement_payload_sha256 IS NULL AND outcome IS NULL
       AND evidence IS NULL AND settled_at IS NULL
       AND expired_at IS NOT NULL
       AND length(btrim(expiry_reason)) BETWEEN 1 AND 500)
    )
);

CREATE INDEX model_invocation_company_window
    ON model_invocation_admissions (window_started_at, admitted_at, id)
    WHERE state IN ('admitted', 'settled');
CREATE INDEX model_invocation_actor_window
    ON model_invocation_admissions (actor_id, window_started_at, admitted_at, id)
    WHERE state IN ('admitted', 'settled');
CREATE INDEX model_invocation_reconcile
    ON model_invocation_admissions (reclaim_after, admitted_at, id)
    WHERE state = 'admitted';
CREATE INDEX model_invocation_recent
    ON model_invocation_admissions (admitted_at DESC, id DESC);
