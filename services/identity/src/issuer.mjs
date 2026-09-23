// The one Restless identity issuer (ADR 0012). A host supplies configuration,
// a Placement port and a Mail port; everything that must mean the same in every
// deployment lives here: accounts, membership versions, signed handoffs and
// controls, durable control delivery, the membership admin API and metadata.
import { readFile } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { betterAuth } from "better-auth";
import { getMigrations } from "better-auth/db/migration";
import { toNodeHandler } from "better-auth/node";
import { organization } from "better-auth/plugins";
import { importJWK, SignJWT } from "jose";
import pg from "pg";
import { coreRequest } from "./core-request.mjs";

export const ISSUER_CONTRACT_VERSION = 1;
const ADMIN_PREFIX = "/api/admin/v1/companies/";
const UUID =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const AUTH_ROUTES = new Set([
  "sign-up/email",
  "sign-in/email",
  "sign-out",
  "get-session",
  "verify-email",
  "send-verification-email",
  "request-password-reset",
  "reset-password",
]);
const ASSETS = new Map([
  ["/", ["index.html", "text/html; charset=utf-8"]],
  ["/app.js", ["app.js", "text/javascript; charset=utf-8"]],
  ["/style.css", ["style.css", "text/css; charset=utf-8"]],
]);
// Matches Fleet's delivery schedule, then retries every minute indefinitely.
const BACKOFF_SECONDS = [2, 5, 10, 30, 60];
const DELIVERY_LEASE_SECONDS = 30;

class Refusal extends Error {
  constructor(status, message) {
    super(message);
    this.status = status;
  }
}
const refuse = (status, message) => {
  throw new Refusal(status, message);
};

const SCHEMA = `
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

/**
 * @param config validated host configuration (origin, secret, signingKey, databaseUrl, companyName?, ownerEmail?)
 * @param ports.placement { resolve(companyId) -> coordinates | null, defaultCompanyId? }
 * @param ports.mail { send({to, subject, text}) }
 */
export async function createIssuer(
  config,
  { placement, mail, fetchCore = coreRequest, deliveryIntervalMs = 2000 } = {},
) {
  if (typeof placement?.resolve !== "function")
    throw Error("A Placement port is required");
  if (typeof mail?.send !== "function")
    throw Error("A Mail port is required for verification and invitations");
  const origin = new URL(config.origin);
  const pool = new pg.Pool({
    connectionString: config.databaseUrl,
    max: 8,
    connectionTimeoutMillis: 5000,
    statement_timeout: 10000,
  });
  const key = await importJWK(config.signingKey, "EdDSA");
  const { kty, crv, x } = config.signingKey;
  const jwks = {
    keys: [{ kty, crv, x, kid: config.signingKey.kid, use: "sig", alg: "EdDSA" }],
  };
  const metadata = {
    contract_version: ISSUER_CONTRACT_VERSION,
    issuer: config.origin,
    admin_api: config.origin + "/api/admin/v1",
    account_url: config.origin + "/",
    roles: ["owner", "admin", "member"],
    terminal_statuses: ["suspended", "removed"],
    invitation_delivery: ["email", "link"],
    role_changes: "owner",
  };
  const slugFor = (companyId) => `restless-${companyId}`;
  const invitationLink = (id) =>
    `${config.origin}/?invite=${encodeURIComponent(id)}`;

  // A secure deployment uses __Host- cookies so a sibling subdomain, such as a
  // company plane, can neither set nor overwrite the account session.
  const secure = origin.protocol === "https:";
  const options = {
    database: pool,
    baseURL: config.origin,
    secret: config.secret,
    trustedOrigins: [config.origin],
    logger: { disabled: true },
    advanced: secure
      ? {
          useSecureCookies: false,
          cookiePrefix: "__Host-restless-identity",
          defaultCookieAttributes: { secure: true },
        }
      : { cookiePrefix: "restless-identity" },
    rateLimit: { enabled: true, storage: "database" },
    emailAndPassword: {
      enabled: true,
      requireEmailVerification: true,
      minPasswordLength: 12,
      autoSignIn: false,
      sendResetPassword: async ({ user, url }) =>
        mail.send({
          to: user.email,
          subject: "Reset your Restless password",
          text: `Reset your password:\n\n${url}\n\nIf you did not request this, ignore this email.`,
        }),
    },
    emailVerification: {
      sendOnSignUp: true,
      sendVerificationEmail: async ({ user, url }) =>
        mail.send({
          to: user.email,
          subject: "Verify your Restless email",
          text: `Verify your email to sign in to Restless:\n\n${url}`,
        }),
    },
    plugins: [
      organization({
        allowUserToCreateOrganization: false,
        organizationLimit: 1,
        sendInvitationEmail: async ({ email, id, organization }) =>
          mail.send({
            to: email,
            subject: `Join ${organization.name} on Restless`,
            text: `You have been invited to ${organization.name}. Sign in with this email address to accept:\n\n${invitationLink(id)}`,
          }),
      }),
    ],
  };
  try {
    await (await getMigrations(options)).runMigrations();
    await pool.query(SCHEMA);
    await migrateLegacyRemovals();
  } catch (error) {
    await pool.end();
    throw error;
  }
  const auth = betterAuth(options),
    authHandler = toNodeHandler(auth);
  const assets = new Map(
    await Promise.all(
      [...ASSETS].map(async ([route, [file, type]]) => [
        route,
        {
          body: await readFile(new URL("../public/" + file, import.meta.url)),
          type,
        },
      ]),
    ),
  );

  // Before this library, the self-hosted service kept pending removals in a
  // bare table and signed constant versions (1 for entry, 2 for removal).
  // Carry any such removal into the durable outbox exactly once.
  async function migrateLegacyRemovals() {
    const legacy = await pool.query(
      "SELECT to_regclass('restless_membership_removals') IS NOT NULL AS present",
    );
    if (!legacy.rows[0].present) return;
    const client = await pool.connect();
    try {
      await client.query("BEGIN");
      const rows = (
        await client.query(
          `SELECT removal.membership_id, removal.user_id, member."organizationId" AS organization_id,
                  member.role, organization.slug
             FROM restless_membership_removals removal
             LEFT JOIN member ON member.id = removal.membership_id
             LEFT JOIN organization ON organization.id = member."organizationId"`,
        )
      ).rows;
      for (const row of rows) {
        const companyId = row.slug?.startsWith("restless-")
          ? row.slug.slice("restless-".length)
          : null;
        // A removal whose member row is gone was already confirmed by Core.
        if (!row.organization_id || !UUID.test(companyId ?? "")) continue;
        await client.query(
          `INSERT INTO restless_membership_state
             (membership_id, organization_id, user_id, company_id, role, status, version)
           VALUES ($1,$2,$3,$4,$5,'removed',2) ON CONFLICT (membership_id) DO NOTHING`,
          [row.membership_id, row.organization_id, row.user_id, companyId, row.role],
        );
        await client.query(
          `INSERT INTO restless_membership_controls
             (jti, membership_id, company_id, user_id, role, status, version)
           VALUES ($1,$2,$3,$4,$5,'removed',2) ON CONFLICT DO NOTHING`,
          [randomUUID(), row.membership_id, companyId, row.user_id, row.role],
        );
        await client.query("DELETE FROM member WHERE id=$1", [row.membership_id]);
      }
      await client.query("DROP TABLE restless_membership_removals");
      await client.query("COMMIT");
    } catch (error) {
      await client.query("ROLLBACK");
      throw error;
    } finally {
      client.release();
    }
  }

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

  async function coordinates(companyId) {
    if (!UUID.test(companyId ?? "")) refuse(404, "Company not found.");
    const placed = await placement.resolve(companyId);
    if (!placed) refuse(404, "Company not found.");
    return placed;
  }
  async function organizationRow(companyId, client = pool) {
    return (
      await client.query("SELECT id, name FROM organization WHERE slug=$1", [
        slugFor(companyId),
      ])
    ).rows[0];
  }

  /**
   * The versioned membership view: Better Auth's member row joined to its
   * Restless state. A member without state yet is version 1 and active, which
   * is exactly what the previous constant-version issuer signed.
   */
  async function membershipOf(companyId, userId, client = pool) {
    const row = (
      await client.query(
        `SELECT member.id AS membership_id, member."userId" AS user_id,
                member."organizationId" AS organization_id, member.role,
                state.status, state.version,
                EXISTS (SELECT 1 FROM restless_membership_controls control
                         WHERE control.membership_id = member.id
                           AND control.delivered_at IS NULL) AS ending
           FROM member
           JOIN organization ON organization.id = member."organizationId"
           LEFT JOIN restless_membership_state state ON state.membership_id = member.id
          WHERE organization.slug = $1 AND member."userId" = $2`,
        [slugFor(companyId), userId],
      )
    ).rows[0];
    if (!row) return null;
    return {
      ...row,
      status: row.status ?? "active",
      version: Number(row.version ?? 1),
    };
  }
  async function lockedState(client, companyId, membershipId) {
    const row = (
      await client.query(
        `SELECT member.id AS membership_id, member."userId" AS user_id,
                member."organizationId" AS organization_id, member.role
           FROM member JOIN organization ON organization.id = member."organizationId"
          WHERE organization.slug = $1 AND member.id = $2
          FOR UPDATE OF member`,
        [slugFor(companyId), membershipId],
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

  async function sign(claims, typ) {
    const now = Math.floor(Date.now() / 1000),
      kid = config.signingKey.kid;
    return new SignJWT({
      iss: config.origin,
      iat: now,
      exp: now + 45,
      kid,
      assertion_version: 1,
      ...claims,
    })
      .setProtectedHeader({ alg: "EdDSA", typ, kid })
      .sign(key);
  }
  async function entryAssertion(placed, member, displayName) {
    return sign(
      {
        aud: "restless-core-account-plane",
        sub: member.user_id,
        jti: randomUUID(),
        owner_id: placed.ownerId,
        plane_id: placed.planeId,
        company_id: placed.companyId,
        cell_id: placed.cellId,
        membership_id: member.membership_id,
        membership_role: member.role,
        membership_version: member.version,
        ...(displayName ? { display_name: displayName } : {}),
      },
      "JWT",
    );
  }

  // ---- durable membership-control delivery ----

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

  /** Deliver one claimed control. Returns true once Core confirms it. */
  async function deliver(control) {
    let receipt;
    try {
      const placed = await placement.resolve(control.company_id);
      if (!placed) throw Error("placement_unavailable");
      const token = await sign(
        {
          aud: "restless-core-membership-control",
          sub: control.user_id,
          // The stable outbox identity: a retry re-signs, never re-identifies.
          jti: control.jti,
          owner_id: placed.ownerId,
          plane_id: placed.planeId,
          plane_hostname: placed.coreHost,
          company_id: placed.companyId,
          cell_id: placed.cellId,
          membership_id: control.membership_id,
          membership_role: control.role,
          membership_status: control.status,
          membership_version: Number(control.version),
        },
        "restless-membership-control+jwt",
      );
      const response = await fetchCore(
        placed.coreOrigin + "/internal/v1/membership-controls",
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
        receipt.company_id === placed.companyId &&
        receipt.cell_id === placed.cellId &&
        receipt.membership_id === control.membership_id &&
        receipt.requested_status === control.status &&
        receipt.requested_version === requested;
      const confirmed =
        (["applied", "already_applied"].includes(receipt.outcome) &&
          receipt.observed_status === control.status &&
          receipt.observed_version === requested) ||
        (receipt.outcome === "superseded" &&
          receipt.observed_version > requested);
      if (!echoes || !confirmed) throw Error("receipt_mismatch");
    } catch (error) {
      const attempts = Number(control.attempts) + 1;
      const delay =
        BACKOFF_SECONDS[Math.min(attempts - 1, BACKOFF_SECONDS.length - 1)];
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
    for (const control of await claim("next_attempt_at <= now()", []))
      await deliver(control);
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

  // ---- request helpers ----

  async function account(req) {
    const session = await auth.api.getSession({
      headers: new Headers({ cookie: req.headers.cookie ?? "" }),
      query: { disableCookieCache: true },
    });
    if (!session?.user?.emailVerified)
      refuse(401, "Sign in with a verified email to continue.");
    return session.user;
  }
  async function body(req) {
    let length = 0;
    const chunks = [];
    for await (const chunk of req) {
      length += chunk.length;
      if (length > 16384) refuse(413, "Request is too large.");
      chunks.push(chunk);
    }
    try {
      return JSON.parse(Buffer.concat(chunks).toString() || "{}");
    } catch {
      refuse(400, "Request must contain valid JSON.");
    }
  }
  function json(res, status, data) {
    res.writeHead(status, { "Content-Type": "application/json; charset=utf-8" });
    res.end(JSON.stringify(data));
  }
  const sessionHeaders = (req) =>
    new Headers({ cookie: req.headers.cookie ?? "" });

  async function bootstrap(user, companyId) {
    if (!config.ownerEmail || user.email.toLowerCase() !== config.ownerEmail)
      refuse(403, "Only the configured owner can open this company for the first time.");
    return transaction(async (client) => {
      await client.query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))", [
        slugFor(companyId),
      ]);
      if (!(await organizationRow(companyId, client)))
        await auth.api.createOrganization({
          body: {
            userId: user.id,
            name: config.companyName,
            slug: slugFor(companyId),
          },
        });
    });
  }

  // ---- membership admin API (the cockpit's write side) ----

  function viewMember(row) {
    return {
      membership_id: row.membership_id,
      name: row.name,
      email: row.email,
      role: row.role,
      status: row.status ?? "active",
      version: Number(row.version ?? 1),
      ending: row.ending === true,
    };
  }
  async function adminView(companyId, viewer) {
    const members = (
      await pool.query(
        `SELECT member.id AS membership_id, member.role, "user".name, "user".email,
                state.status, state.version,
                EXISTS (SELECT 1 FROM restless_membership_controls control
                         WHERE control.membership_id = member.id AND control.delivered_at IS NULL) AS ending
           FROM member
           JOIN "user" ON "user".id = member."userId"
           JOIN organization ON organization.id = member."organizationId"
           LEFT JOIN restless_membership_state state ON state.membership_id = member.id
          WHERE organization.slug = $1
          UNION ALL
         SELECT state.membership_id, state.role, "user".name, "user".email,
                state.status, state.version, TRUE AS ending
           FROM restless_membership_state state
           JOIN "user" ON "user".id = state.user_id
          WHERE state.company_id = $2 AND state.status = 'removed'
            AND EXISTS (SELECT 1 FROM restless_membership_controls control
                         WHERE control.membership_id = state.membership_id AND control.delivered_at IS NULL)
          ORDER BY name`,
        [slugFor(companyId), companyId],
      )
    ).rows.map(viewMember);
    const invitations = (
      await pool.query(
        `SELECT invitation.id, invitation.email, invitation.role, invitation."expiresAt" AS expires_at
           FROM invitation JOIN organization ON organization.id = invitation."organizationId"
          WHERE organization.slug = $1 AND invitation.status = 'pending' AND invitation."expiresAt" > now()
          ORDER BY invitation."createdAt" DESC LIMIT 100`,
        [slugFor(companyId)],
      )
    ).rows.map((row) => ({ ...row, link: invitationLink(row.id) }));
    return {
      company_id: companyId,
      viewer: { membership_id: viewer.membership_id, role: viewer.role },
      members,
      invitations,
    };
  }
  /** Owners manage everyone but themselves; admins manage members only. */
  function mayManage(viewer, target) {
    if (target.role === "owner" || target.membership_id === viewer.membership_id)
      refuse(403, "You cannot change the company owner or yourself here.");
    if (viewer.role === "admin" && target.role !== "member")
      refuse(403, "Only the owner can change an administrator.");
  }
  async function adminAction(req, res, companyId, rest) {
    const user = await account(req);
    const viewer = await membershipOf(companyId, user.id);
    if (!viewer || viewer.status !== "active")
      refuse(403, "You do not currently have access to this company.");
    if (!["owner", "admin"].includes(viewer.role))
      refuse(403, "Only the company owner or an administrator can manage members.");
    if (req.method === "GET" && rest === "members")
      return json(res, 200, await adminView(companyId, viewer));
    if (req.method !== "POST") refuse(404, "Membership action not found.");
    if (!/^application\/json\b/i.test(req.headers["content-type"] ?? ""))
      refuse(415, "Membership changes must be JSON.");
    const input = await body(req);
    const organizationId = viewer.organization_id;

    if (rest === "invitations") {
      if (!["member", "admin"].includes(input.role))
        refuse(400, "Choose member or administrator access.");
      if (input.role === "admin" && viewer.role !== "owner")
        refuse(403, "Only the owner can invite an administrator.");
      const invitation = await auth.api.createInvitation({
        headers: sessionHeaders(req),
        body: {
          email: input.email,
          role: input.role,
          organizationId,
          resend: input.resend === true,
        },
      });
      return json(res, 200, {
        invitation: {
          id: invitation.id,
          email: invitation.email,
          role: invitation.role,
          expires_at: invitation.expiresAt,
          link: invitationLink(invitation.id),
        },
      });
    }
    let match = rest.match(/^invitations\/([^/]{1,128})\/cancel$/);
    if (match) {
      const invitation = (
        await pool.query(
          'SELECT "organizationId", role FROM invitation WHERE id=$1',
          [match[1]],
        )
      ).rows[0];
      if (invitation?.organizationId !== organizationId)
        refuse(404, "Invitation not found.");
      if (invitation.role !== "member" && viewer.role !== "owner")
        refuse(403, "Only the owner can cancel an administrator invitation.");
      await auth.api.cancelInvitation({
        headers: sessionHeaders(req),
        body: { invitationId: match[1] },
      });
      return json(res, 200, { cancelled: true });
    }
    match = rest.match(/^members\/([^/]{1,128})\/(role|suspend|reinstate|remove)$/);
    if (!match) refuse(404, "Membership action not found.");
    const [, membershipId, verb] = match;
    let jti = null;
    await transaction(async (client) => {
      const target = await lockedState(client, companyId, membershipId);
      mayManage(viewer, target);
      if (verb === "role") {
        if (viewer.role !== "owner") refuse(403, "Only the owner can change roles.");
        if (!["member", "admin"].includes(input.role))
          refuse(400, "Choose member or administrator access.");
        if (target.status !== "active")
          refuse(409, "Reinstate this person before changing their access.");
        if (input.role === target.role) return;
        // Role and version change together: the next handoff carries both.
        await client.query('UPDATE member SET role=$2 WHERE id=$1', [target.membership_id, input.role]);
        await client.query(
          "UPDATE restless_membership_state SET role=$2, version=version+1, updated_at=now() WHERE membership_id=$1",
          [target.membership_id, input.role],
        );
      } else if (verb === "suspend") {
        if (target.status === "suspended") return;
        jti = await enqueueControl(client, companyId, target, "suspended");
      } else if (verb === "reinstate") {
        if (target.status !== "suspended") refuse(409, "This person is not suspended.");
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
    return json(res, 200, { ending: !ended });
  }

  function corsFor(req, res, allowed) {
    const requestOrigin = req.headers.origin;
    if (!requestOrigin) return true;
    if (requestOrigin !== allowed) return false;
    res.setHeader("Access-Control-Allow-Origin", allowed);
    res.setHeader("Access-Control-Allow-Credentials", "true");
    res.setHeader("Vary", "Origin");
    return true;
  }

  async function handle(req, res) {
    res.setHeader("Cache-Control", "no-store");
    res.setHeader("X-Content-Type-Options", "nosniff");
    res.setHeader("Referrer-Policy", "no-referrer");
    try {
      if (req.headers.host?.toLowerCase() !== origin.host.toLowerCase())
        refuse(403, "This hostname does not belong to this account service.");
      const path = new URL(req.url, config.origin).pathname;
      if (req.method === "GET" && path === "/.well-known/jwks.json")
        return json(res, 200, jwks);
      if (req.method === "GET" && path === "/.well-known/restless-issuer")
        return json(res, 200, metadata);
      if (req.method === "GET" && path === "/health")
        return json(res, 200, { status: "ready" });

      if (path.startsWith(ADMIN_PREFIX)) {
        const [companyId, ...parts] = path.slice(ADMIN_PREFIX.length).split("/");
        const placed = await coordinates(companyId);
        // Only the company's own cockpit may call with the account session.
        if (!corsFor(req, res, placed.coreOrigin))
          refuse(403, "This origin cannot manage company members.");
        if (req.method === "OPTIONS") {
          res.writeHead(204, {
            "Access-Control-Allow-Methods": "GET, POST",
            "Access-Control-Allow-Headers": "Content-Type",
            "Access-Control-Max-Age": "600",
          });
          return res.end();
        }
        if (req.method === "POST" && req.headers.origin !== placed.coreOrigin)
          refuse(403, "Membership changes must come from the company.");
        return await adminAction(req, res, companyId, parts.join("/"));
      }

      res.setHeader(
        "Content-Security-Policy",
        `default-src 'self'; script-src 'self'; style-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self' ${
          (await placement.resolve(placement.defaultCompanyId))?.coreOrigin ?? ""
        }`,
      );
      if (req.method === "POST" && req.headers.origin !== config.origin)
        refuse(403, "Open this account page directly before making changes.");
      if (path.startsWith("/api/auth/")) {
        const route = path.slice("/api/auth/".length);
        const resetCallback =
          req.method === "GET" && /^reset-password\/[A-Za-z0-9_-]{1,256}$/.test(route);
        if (!AUTH_ROUTES.has(route) && !resetCallback)
          refuse(404, "Account action not found.");
        return authHandler(req, res);
      }
      if (req.method === "GET" && ASSETS.has(path)) {
        const asset = assets.get(path);
        res.writeHead(200, { "Content-Type": asset.type });
        return res.end(asset.body);
      }
      const companyId = placement.defaultCompanyId;
      const placed = await coordinates(companyId);
      if (req.method === "GET" && path === "/api/settings")
        return json(res, 200, {
          companyName: config.companyName,
          coreOrigin: placed.coreOrigin,
        });
      const user = await account(req);
      if (req.method === "GET" && path === "/api/company") {
        const member = await membershipOf(companyId, user.id);
        const active = member?.status === "active" && !member.ending;
        return json(res, 200, {
          user: { name: user.name, email: user.email },
          role: active ? member.role : null,
          status: member?.status ?? null,
          canBootstrap:
            !(await organizationRow(companyId)) &&
            !!config.ownerEmail &&
            user.email.toLowerCase() === config.ownerEmail,
        });
      }
      if (req.method !== "POST") refuse(404, "Page not found.");
      const input = await body(req);
      if (path === "/api/company/bootstrap") {
        await bootstrap(user, companyId);
        return json(res, 200, { ready: true });
      }
      if (path === "/api/invitations/accept") {
        const fixed = await organizationRow(companyId);
        const invitation = (
          await pool.query('SELECT "organizationId" FROM invitation WHERE id=$1', [input.id])
        ).rows[0];
        if (!fixed || invitation?.organizationId !== fixed.id)
          refuse(404, "This invitation is no longer available.");
        await auth.api.acceptInvitation({
          headers: sessionHeaders(req),
          body: { invitationId: input.id },
        });
        return json(res, 200, { accepted: true });
      }
      if (path === "/api/enter") {
        if (placed.ready === false)
          refuse(503, "The company is starting. Try again in a moment.");
        const member = await membershipOf(companyId, user.id);
        // A terminal control must reach Core before any fresh entry is issued.
        if (!member || member.status !== "active" || member.ending)
          refuse(403, "You do not currently have access to this company.");
        return json(res, 200, {
          action: placed.coreOrigin + "/entry",
          assertion: await entryAssertion(placed, member, user.name),
        });
      }
      refuse(404, "Account action not found.");
    } catch (error) {
      // Better Auth's public API errors contain a bounded user-facing message.
      const status =
        error instanceof Refusal ? error.status : Number(error.statusCode) || 500;
      const message =
        error instanceof Refusal
          ? error.message
          : (status < 500 ? error.body?.message : null) ||
            "The account service could not complete that request. Please try again.";
      if (!res.headersSent) json(res, status, { message });
      else res.end();
    }
  }
  async function close() {
    clearInterval(timer);
    await delivering;
    await pool.end();
  }
  return { handle, close, deliverDue, metadata };
}
