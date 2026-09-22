import { readFile } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { betterAuth } from "better-auth";
import { getMigrations } from "better-auth/db/migration";
import { toNodeHandler } from "better-auth/node";
import { organization } from "better-auth/plugins";
import { importJWK, SignJWT } from "jose";
import pg from "pg";
import { validateConfig } from "./config.mjs";
import { coreRequest } from "./core-request.mjs";

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
class Refusal extends Error {
  constructor(status, message) {
    super(message);
    this.status = status;
  }
}
const refuse = (status, message) => {
  throw new Refusal(status, message);
};
export async function createIdentityService(
  rawConfig,
  { sendMail, fetchCore = coreRequest } = {},
) {
  const config = validateConfig(rawConfig);
  if (typeof sendMail !== "function")
    throw Error("A verification and invitation mail transport is required");
  const pool = new pg.Pool({
    connectionString: config.databaseUrl,
    max: 8,
    connectionTimeoutMillis: 5000,
    statement_timeout: 10000,
  });
  const key = await importJWK(config.signingKey, "EdDSA");
  const { kty, crv, x } = config.signingKey;
  const publicKey = { kty, crv, x };
  const jwks = {
    keys: [
      { ...publicKey, kid: config.signingKey.kid, use: "sig", alg: "EdDSA" },
    ],
  };
  const slug = `restless-${config.companyId}`;
  async function organizationRow() {
    return (
      await pool.query("SELECT id FROM organization WHERE slug=$1", [slug])
    ).rows[0];
  }
  async function signed(member, control = false, displayName) {
    const now = Math.floor(Date.now() / 1000),
      kid = config.signingKey.kid;
    return new SignJWT({
      iss: config.origin,
      aud: control
        ? "restless-core-membership-control"
        : "restless-core-account-plane",
      sub: member.userId,
      jti: randomUUID(),
      iat: now,
      exp: now + 55,
      kid,
      owner_id: config.ownerId,
      plane_id: config.planeId,
      company_id: config.companyId,
      cell_id: config.cellId,
      membership_id: member.id,
      membership_role: member.role,
      membership_version: control ? 2 : 1,
      assertion_version: 1,
      ...(control
        ? { plane_hostname: config.coreHost, membership_status: "removed" }
        : { display_name: displayName }),
    })
      .setProtectedHeader({
        alg: "EdDSA",
        typ: control ? "restless-membership-control+jwt" : "JWT",
        kid,
      })
      .sign(key);
  }
  async function revoke(member) {
    // A failed delivery must prevent fresh entry and remain retryable after restart.
    await pool.query(
      "INSERT INTO restless_membership_removals (membership_id,user_id) VALUES ($1,$2) ON CONFLICT (membership_id) DO NOTHING",
      [member.id, member.userId],
    );
    let response;
    try {
      response = await fetchCore(
        config.coreOrigin + "/internal/v1/membership-controls",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ control: await signed(member, true) }),
          redirect: "error",
          signal: AbortSignal.timeout(10000),
        },
      );
    } catch {
      refuse(
        503,
        "Core could not confirm removal. New entry is blocked; retry removal when Core is available.",
      );
    }
    if (!response.ok)
      refuse(
        503,
        "Core could not confirm removal. New entry is blocked; retry removal when Core is available.",
      );
    const receipt = await response.json();
    if (receipt.observed_status !== "removed" || receipt.observed_version < 2)
      refuse(503, "Core has not confirmed that access ended. Retry removal.");
  }
  const options = {
    database: pool,
    baseURL: config.origin,
    secret: config.secret,
    trustedOrigins: [config.origin],
    logger: { disabled: true },
    advanced: { cookiePrefix: `restless-identity-${config.companyId}` },
    rateLimit: { enabled: true, storage: "database" },
    emailAndPassword: {
      enabled: true,
      requireEmailVerification: true,
      minPasswordLength: 12,
      autoSignIn: false,
      sendResetPassword: async ({ user, url }) =>
        sendMail({
          to: user.email,
          subject: "Reset your Restless password",
          text: `Reset your password:\n\n${url}\n\nIf you did not request this, ignore this email.`,
        }),
    },
    emailVerification: {
      sendOnSignUp: true,
      sendVerificationEmail: async ({ user, url }) =>
        sendMail({
          to: user.email,
          subject: "Verify your Restless email",
          text: `Verify your email to sign in to Restless:\n\n${url}`,
        }),
    },
    plugins: [
      organization({
        allowUserToCreateOrganization: false,
        organizationLimit: 1,
        sendInvitationEmail: async ({ email, id }) =>
          sendMail({
            to: email,
            subject: `Join ${config.companyName} on Restless`,
            text: `You have been invited to ${config.companyName}. Sign in with this email address to accept:\n\n${config.origin}/?invite=${encodeURIComponent(id)}`,
          }),
        organizationHooks: {
          beforeRemoveMember: async ({ member, organization }) => {
            const fixed = await organizationRow();
            if (organization.id !== fixed?.id)
              refuse(403, "This company is not available.");
            if (member.role === "owner")
              refuse(403, "The company owner cannot be removed here.");
            await revoke(member);
          },
        },
      }),
    ],
  };
  try {
    await (await getMigrations(options)).runMigrations();
    await pool.query(
      "CREATE TABLE IF NOT EXISTS restless_membership_removals (membership_id text PRIMARY KEY,user_id text NOT NULL,requested_at timestamptz NOT NULL DEFAULT now())",
    );
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
  async function account(req) {
    const session = await auth.api.getSession({
      headers: new Headers({ cookie: req.headers.cookie ?? "" }),
      query: { disableCookieCache: true },
    });
    if (!session?.user?.emailVerified)
      refuse(401, "Sign in with a verified email to continue.");
    return session.user;
  }
  async function memberFor(user) {
    const fixed = await organizationRow();
    if (!fixed) return null;
    return (
      (
        await pool.query(
          'SELECT member.id,member.role,member."userId",member."organizationId",removal.membership_id AS removing FROM member LEFT JOIN restless_membership_removals removal ON removal.membership_id=member.id WHERE member."organizationId"=$1 AND member."userId"=$2',
          [fixed.id, user.id],
        )
      ).rows[0] ?? null
    );
  }
  async function membership(user) {
    const member = await memberFor(user);
    if (!member || member.removing)
      refuse(403, "You do not currently have access to this company.");
    return member;
  }
  function manager(member) {
    if (!["owner", "admin"].includes(member.role))
      refuse(
        403,
        "Only the company owner or an administrator can manage invitations.",
      );
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
    res.writeHead(status, {
      "Content-Type": "application/json; charset=utf-8",
    });
    res.end(JSON.stringify(data));
  }
  async function bootstrap(user) {
    if (user.email.toLowerCase() !== config.ownerEmail)
      refuse(
        403,
        "Only the configured owner can open this company for the first time.",
      );
    const connection = await pool.connect();
    try {
      await connection.query("BEGIN");
      await connection.query(
        "SELECT pg_advisory_xact_lock(hashtextextended($1,0))",
        [slug],
      );
      let fixed = await organizationRow();
      if (!fixed) {
        await auth.api.createOrganization({
          body: { userId: user.id, name: config.companyName, slug },
        });
        fixed = await organizationRow();
      }
      await connection.query("COMMIT");
      return fixed;
    } catch (error) {
      await connection.query("ROLLBACK");
      throw error;
    } finally {
      connection.release();
    }
  }
  async function handle(req, res) {
    res.setHeader("Cache-Control", "no-store");
    res.setHeader("X-Content-Type-Options", "nosniff");
    res.setHeader("Referrer-Policy", "no-referrer");
    res.setHeader(
      "Content-Security-Policy",
      `default-src 'self'; script-src 'self'; style-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self' ${config.coreOrigin}`,
    );
    try {
      if (req.headers.host?.toLowerCase() !== config.host.toLowerCase())
        refuse(403, "This hostname does not belong to this account service.");
      const path = new URL(req.url, config.origin).pathname;
      if (req.method === "GET" && path === "/.well-known/jwks.json")
        return json(res, 200, jwks);
      if (req.method === "GET" && path === "/health")
        return json(res, 200, { status: "ready" });
      if (req.method === "POST" && req.headers.origin !== config.origin)
        refuse(403, "Open this account page directly before making changes.");
      if (path.startsWith("/api/auth/")) {
        const route = path.slice("/api/auth/".length);
        const resetCallback =
          req.method === "GET" &&
          /^reset-password\/[A-Za-z0-9_-]{1,256}$/.test(route);
        if (!AUTH_ROUTES.has(route) && !resetCallback)
          refuse(404, "Account action not found.");
        return authHandler(req, res);
      }
      if (req.method === "GET" && ASSETS.has(path)) {
        const asset = assets.get(path);
        res.writeHead(200, { "Content-Type": asset.type });
        return res.end(asset.body);
      }
      if (req.method === "GET" && path === "/api/settings")
        return json(res, 200, {
          companyName: config.companyName,
          coreOrigin: config.coreOrigin,
        });
      const user = await account(req);
      if (req.method === "GET" && path === "/api/company") {
        const member = await memberFor(user);
        const fixed = await organizationRow();
        const manageable =
          member &&
          !member.removing &&
          ["owner", "admin"].includes(member.role);
        const members = manageable
          ? (
              await pool.query(
                'SELECT member.id,member.role,"user".name,"user".email,removal.membership_id IS NOT NULL AS removing FROM member JOIN "user" ON "user".id=member."userId" LEFT JOIN restless_membership_removals removal ON removal.membership_id=member.id WHERE member."organizationId"=$1 ORDER BY "user".name',
                [member.organizationId],
              )
            ).rows
          : [];
        const invitations = manageable
          ? (
              await pool.query(
                'SELECT id,email,role,status,"expiresAt" FROM invitation WHERE "organizationId"=$1 AND status=$2 AND "expiresAt">now() ORDER BY "createdAt" DESC LIMIT 100',
                [member.organizationId, "pending"],
              )
            ).rows
          : [];
        return json(res, 200, {
          user: { name: user.name, email: user.email },
          role: member?.removing ? null : (member?.role ?? null),
          canBootstrap:
            !fixed && user.email.toLowerCase() === config.ownerEmail,
          members,
          invitations,
        });
      }
      if (req.method !== "POST") refuse(404, "Page not found.");
      const input = await body(req);
      if (path === "/api/company/bootstrap") {
        await bootstrap(user);
        return json(res, 200, { ready: true });
      }
      if (path === "/api/invitations/accept") {
        const fixed = await organizationRow();
        const invitation = (
          await pool.query(
            'SELECT "organizationId" FROM invitation WHERE id=$1',
            [input.id],
          )
        ).rows[0];
        if (!fixed || invitation?.organizationId !== fixed.id)
          refuse(404, "This invitation is no longer available.");
        await auth.api.acceptInvitation({
          headers: new Headers({ cookie: req.headers.cookie ?? "" }),
          body: { invitationId: input.id },
        });
        return json(res, 200, { accepted: true });
      }
      const member = await membership(user);
      if (path === "/api/enter")
        return json(res, 200, {
          action: config.coreOrigin + "/entry",
          assertion: await signed(member, false, user.name),
        });
      manager(member);
      if (path === "/api/invitations") {
        if (!["member", "admin"].includes(input.role))
          refuse(400, "Choose member or administrator access.");
        await auth.api.createInvitation({
          headers: new Headers({ cookie: req.headers.cookie ?? "" }),
          body: {
            email: input.email,
            role: input.role,
            organizationId: member.organizationId,
            resend: input.resend === true,
          },
        });
        return json(res, 200, { sent: true });
      }
      if (path === "/api/invitations/cancel") {
        const invitation = (
          await pool.query(
            'SELECT "organizationId" FROM invitation WHERE id=$1',
            [input.id],
          )
        ).rows[0];
        if (invitation?.organizationId !== member.organizationId)
          refuse(404, "Invitation not found.");
        await auth.api.cancelInvitation({
          headers: new Headers({ cookie: req.headers.cookie ?? "" }),
          body: { invitationId: input.id },
        });
        return json(res, 200, { cancelled: true });
      }
      if (path === "/api/members/remove") {
        const target = (
          await pool.query(
            'SELECT id,role FROM member WHERE id=$1 AND "organizationId"=$2',
            [input.id, member.organizationId],
          )
        ).rows[0];
        if (!target) refuse(404, "Member not found.");
        if (target.role === "owner" || target.id === member.id)
          refuse(403, "You cannot remove the company owner or yourself here.");
        if (member.role === "admin" && target.role !== "member")
          refuse(403, "Only the owner can remove an administrator.");
        await auth.api.removeMember({
          headers: new Headers({ cookie: req.headers.cookie ?? "" }),
          body: {
            memberIdOrEmail: target.id,
            organizationId: member.organizationId,
          },
        });
        return json(res, 200, { removed: true });
      }
      refuse(404, "Account action not found.");
    } catch (error) {
      // Better Auth's public API errors contain a bounded user-facing message.
      const status =
        error instanceof Refusal
          ? error.status
          : Number(error.statusCode) || 500;
      const message =
        error instanceof Refusal
          ? error.message
          : (status < 500 ? error.body?.message : null) ||
            "The account service could not complete that request. Please try again.";
      if (!res.headersSent) json(res, status, { message });
      else res.end();
    }
  }
  return { handle, close: () => pool.end() };
}
