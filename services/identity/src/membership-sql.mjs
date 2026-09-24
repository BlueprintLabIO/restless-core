// Core's membership Store for the identity issuer: versioned membership in
// PostgreSQL beside Better Auth, signed entry handoffs, and suspension and
// removal delivered to Core through a durable outbox.
import { randomUUID } from "node:crypto";
import { importJWK, SignJWT } from "jose";
import { coreRequest } from "./core-request.mjs";
import { refuse } from "./issuer.mjs";

// Matches Fleet's delivery schedule, then retries every minute indefinitely.
const BACKOFF_SECONDS = [2, 5, 10, 30, 60];
const DELIVERY_LEASE_SECONDS = 30;

/** Store tables. A host runs these once, after Better Auth's migrations. */
export const MEMBERSHIP_SCHEMA = `
CREATE TABLE IF NOT EXISTS restless_membership_state (
  membership_id text PRIMARY KEY,
  organization_id text NOT NULL,
  user_id text NOT NULL,
  company_id uuid NOT NULL,
  role text NOT NULL CHECK (role IN ('owner','admin','member')),
  status text NOT NULL CHECK (status IN ('active','suspended','removed')),
  version bigint NOT NULL CHECK (version >= 1),
  updated_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS restless_membership_controls (
  jti uuid PRIMARY KEY,
  membership_id text NOT NULL REFERENCES restless_membership_state(membership_id),
  company_id uuid NOT NULL,
  user_id text NOT NULL,
  role text NOT NULL CHECK (role IN ('owner','admin','member')),
  status text NOT NULL CHECK (status IN ('suspended','removed')),
  version bigint NOT NULL CHECK (version >= 1),
  created_at timestamptz NOT NULL DEFAULT now(),
  attempts integer NOT NULL DEFAULT 0,
  next_attempt_at timestamptz NOT NULL DEFAULT now(),
  delivered_at timestamptz,
  outcome text,
  last_error text,
  UNIQUE (membership_id, version)
);
CREATE INDEX IF NOT EXISTS restless_membership_controls_due
  ON restless_membership_controls (next_attempt_at) WHERE delivered_at IS NULL;
`;

export async function createSqlMembershipStore({
  pool,
  origin,
  signingKey,
  placement,
  fetchCore = coreRequest,
  deliveryIntervalMs = 2000,
}) {
  const key = await importJWK(signingKey, "EdDSA");
  const { kty, crv, x } = signingKey;
  const jwks = {
    keys: [{ kty, crv, x, kid: signingKey.kid, use: "sig", alg: "EdDSA" }],
  };

  async function transaction(work) {
    const client = await pool.connect();
    try {
      await client.query("BEGIN");
      const result = await work(client);
      await client.query("COMMIT");
      return result;
    } catch (error) {
      await client.query("ROLLBACK");
      throw error;
    } finally {
      client.release();
    }
  }
  async function placed(companyId) {
    const coordinates = await placement.resolve(companyId);
    if (!coordinates) refuse(404, "Company not found.");
    return coordinates;
  }
  /** The organisation holding a company's memberships. */
  async function organizationId(companyId, client = pool) {
    const coordinates = await placed(companyId);
    if (coordinates.organizationId) return coordinates.organizationId;
    return (
      await client.query("SELECT id FROM organization WHERE slug=$1", [
        `restless-${companyId}`,
      ])
    ).rows[0]?.id;
  }

  /** A member without state yet is version 1 and active. */
  async function membership(companyId, userId, client = pool) {
    const organization = await organizationId(companyId, client);
    if (!organization) return null;
    const row = (
      await client.query(
        `SELECT member.id AS membership_id, member."userId" AS user_id,
                member."organizationId" AS organization_id, member.role,
                state.status, state.version,
                EXISTS (SELECT 1 FROM restless_membership_controls control
                         WHERE control.membership_id = member.id
                           AND control.delivered_at IS NULL) AS ending
           FROM member
           LEFT JOIN restless_membership_state state ON state.membership_id = member.id
          WHERE member."organizationId" = $1 AND member."userId" = $2`,
        [organization, userId],
      )
    ).rows[0];
    if (!row) return null;
    return { ...row, status: row.status ?? "active", version: Number(row.version ?? 1) };
  }

  async function members(companyId, organization) {
    return (
      await pool.query(
        `SELECT member.id AS membership_id, member.role, "user".name, "user".email,
                state.status, state.version,
                EXISTS (SELECT 1 FROM restless_membership_controls control
                         WHERE control.membership_id = member.id AND control.delivered_at IS NULL) AS ending
           FROM member
           JOIN "user" ON "user".id = member."userId"
           LEFT JOIN restless_membership_state state ON state.membership_id = member.id
          WHERE member."organizationId" = $1
          UNION ALL
         SELECT state.membership_id, state.role, "user".name, "user".email,
                state.status, state.version, TRUE AS ending
           FROM restless_membership_state state
           JOIN "user" ON "user".id = state.user_id
          WHERE state.company_id = $2 AND state.status = 'removed'
            AND EXISTS (SELECT 1 FROM restless_membership_controls control
                         WHERE control.membership_id = state.membership_id AND control.delivered_at IS NULL)
          ORDER BY name`,
        [organization, companyId],
      )
    ).rows.map((row) => ({
      membership_id: row.membership_id,
      name: row.name,
      email: row.email,
      role: row.role,
      status: row.status ?? "active",
      version: Number(row.version ?? 1),
      ending: row.ending === true,
    }));
  }

  async function lockedState(client, companyId, membershipId) {
    const organization = await organizationId(companyId, client);
    const row = (
      await client.query(
        `SELECT id AS membership_id, "userId" AS user_id,
                "organizationId" AS organization_id, role
           FROM member WHERE "organizationId" = $1 AND id = $2 FOR UPDATE`,
        [organization, membershipId],
      )
    ).rows[0];
    if (!row) refuse(404, "Member not found.");
    await client.query(
      `INSERT INTO restless_membership_state
         (membership_id, organization_id, user_id, company_id, role, status, version)
       VALUES ($1,$2,$3,$4,$5,'active',1) ON CONFLICT (membership_id) DO NOTHING`,
      [row.membership_id, row.organization_id, row.user_id, companyId, row.role],
    );
    const state = (
      await client.query(
        "SELECT status, version FROM restless_membership_state WHERE membership_id=$1 FOR UPDATE",
        [row.membership_id],
      )
    ).rows[0];
    return { ...row, status: state.status, version: Number(state.version) };
  }
  async function enqueueControl(client, companyId, state, status) {
    const version = state.version + 1;
    await client.query(
      "UPDATE restless_membership_state SET status=$2, version=$3, role=$4, updated_at=now() WHERE membership_id=$1",
      [state.membership_id, status, version, state.role],
    );
    const jti = randomUUID();
    await client.query(
      `INSERT INTO restless_membership_controls
         (jti, membership_id, company_id, user_id, role, status, version)
       VALUES ($1,$2,$3,$4,$5,$6,$7)`,
      [jti, state.membership_id, companyId, state.user_id, state.role, status, version],
    );
    return jti;
  }

  /** Apply one authorised change atomically with its version and control. */
  async function change(companyId, membershipId, requested, { authorize }) {
    let jti = null;
    await transaction(async (client) => {
      const target = await lockedState(client, companyId, membershipId);
      if (!authorize(target)) return;
      if (requested.verb === "role") {
        // Role and version change together: the next handoff carries both.
        await client.query("UPDATE member SET role=$2 WHERE id=$1", [
          target.membership_id,
          requested.role,
        ]);
        await client.query(
          "UPDATE restless_membership_state SET role=$2, version=version+1, updated_at=now() WHERE membership_id=$1",
          [target.membership_id, requested.role],
        );
      } else if (requested.verb === "suspend") {
        jti = await enqueueControl(client, companyId, target, "suspended");
      } else if (requested.verb === "reinstate") {
        await client.query(
          "UPDATE restless_membership_state SET status='active', version=version+1, updated_at=now() WHERE membership_id=$1",
          [target.membership_id],
        );
      } else {
        jti = await enqueueControl(client, companyId, target, "removed");
        await client.query("DELETE FROM member WHERE id=$1", [target.membership_id]);
      }
    });
    // Try at once; the outbox keeps retrying if Core is unavailable.
    const ended = jti ? await deliverNow(jti) : true;
    return { ending: !ended };
  }

  async function sign(claims, typ) {
    const now = Math.floor(Date.now() / 1000);
    return new SignJWT({
      iss: origin,
      iat: now,
      exp: now + 45,
      kid: signingKey.kid,
      assertion_version: 1,
      ...claims,
    })
      .setProtectedHeader({ alg: "EdDSA", typ, kid: signingKey.kid })
      .sign(key);
  }

  /**
   * A single-use entry handoff for one verified user into one company. A
   * terminal control must reach Core before any fresh entry is issued.
   */
  async function enter(user, companyId) {
    const coordinates = await placed(companyId);
    // Access first: someone without entry learns that, not the plane's state.
    const member = await membership(companyId, user.id);
    if (!member || member.status !== "active" || member.ending)
      refuse(403, "You do not currently have access to this company.");
    if (coordinates.ready === false)
      refuse(503, "The company is starting. Try again in a moment.");
    return {
      action: coordinates.coreOrigin + "/entry",
      assertion: await sign(
        {
          aud: "restless-core-account-plane",
          sub: member.user_id,
          jti: randomUUID(),
          owner_id: coordinates.ownerId,
          plane_id: coordinates.planeId,
          company_id: coordinates.companyId,
          cell_id: coordinates.cellId,
          membership_id: member.membership_id,
          membership_role: member.role,
          membership_version: member.version,
          ...(user.name ? { display_name: user.name } : {}),
        },
        "JWT",
      ),
    };
  }

  /** Deliver one claimed control. Returns true once Core confirms it. */
  async function deliver(control) {
    let receipt;
    try {
      const coordinates = await placement.resolve(control.company_id);
      if (!coordinates) throw Error("placement_unavailable");
      const token = await sign(
        {
          aud: "restless-core-membership-control",
          sub: control.user_id,
          // The stable outbox identity: a retry re-signs, never re-identifies.
          jti: control.jti,
          owner_id: coordinates.ownerId,
          plane_id: coordinates.planeId,
          plane_hostname: coordinates.coreHost,
          company_id: coordinates.companyId,
          cell_id: coordinates.cellId,
          membership_id: control.membership_id,
          membership_role: control.role,
          membership_status: control.status,
          membership_version: Number(control.version),
        },
        "restless-membership-control+jwt",
      );
      const response = await fetchCore(
        coordinates.coreOrigin + "/internal/v1/membership-controls",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ control: token }),
          redirect: "error",
          signal: AbortSignal.timeout(10000),
        },
      );
      if (!response.ok) throw Error("core_refused");
      receipt = await response.json();
      const requested = Number(control.version);
      const echoes =
        receipt.jti === control.jti &&
        receipt.company_id === coordinates.companyId &&
        receipt.cell_id === coordinates.cellId &&
        receipt.membership_id === control.membership_id &&
        receipt.requested_status === control.status &&
        receipt.requested_version === requested;
      const confirmed =
        (["applied", "already_applied"].includes(receipt.outcome) &&
          receipt.observed_status === control.status &&
          receipt.observed_version === requested) ||
        (receipt.outcome === "superseded" && receipt.observed_version > requested);
      if (!echoes || !confirmed) throw Error("receipt_mismatch");
    } catch (error) {
      const attempts = Number(control.attempts) + 1;
      const delay = BACKOFF_SECONDS[Math.min(attempts - 1, BACKOFF_SECONDS.length - 1)];
      await pool.query(
        `UPDATE restless_membership_controls
            SET attempts=$2, next_attempt_at=now() + make_interval(secs => $3), last_error=$4
          WHERE jti=$1 AND delivered_at IS NULL`,
        [control.jti, attempts, delay, String(error?.message ?? "delivery_failed").slice(0, 64)],
      );
      return false;
    }
    await pool.query(
      `UPDATE restless_membership_controls
          SET delivered_at=now(), outcome=$2, attempts=attempts+1, last_error=NULL
        WHERE jti=$1`,
      [control.jti, receipt.outcome],
    );
    return true;
  }
  /** Claim due controls with a lease, so concurrent hosts never double-send. */
  async function claim(where, parameters) {
    return (
      await pool.query(
        `UPDATE restless_membership_controls control
            SET next_attempt_at = now() + make_interval(secs => ${DELIVERY_LEASE_SECONDS})
          WHERE control.jti IN (
            SELECT jti FROM restless_membership_controls
             WHERE delivered_at IS NULL AND ${where}
             ORDER BY version ASC LIMIT 10 FOR UPDATE SKIP LOCKED)
        RETURNING control.*`,
        parameters,
      )
    ).rows;
  }
  async function deliverDue() {
    for (const control of await claim("next_attempt_at <= now()", [])) await deliver(control);
  }
  async function deliverNow(jti) {
    const [control] = await claim("jti = $1", [jti]);
    return control ? deliver(control) : false;
  }
  let delivering = Promise.resolve();
  const timer = setInterval(() => {
    delivering = delivering.then(deliverDue).catch(() => {});
  }, deliveryIntervalMs);
  timer.unref?.();

  return {
    membership,
    members,
    change,
    enter,
    jwks,
    deliverDue,
    async close() {
      clearInterval(timer);
      await delivering;
    },
  };
}
