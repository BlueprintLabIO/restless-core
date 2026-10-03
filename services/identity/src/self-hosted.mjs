// The self-hosted host of the identity issuer: its own Better Auth instance
// and account pages for configured companies, composed with the shared library.
import { readFile } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { betterAuth } from "better-auth";
import { getMigrations } from "better-auth/db/migration";
import { toNodeHandler } from "better-auth/node";
import { organization } from "better-auth/plugins";
import pg from "pg";
import { createIssuer, failure, json, readJson, refuse } from "./issuer.mjs";
import { createSqlMembershipStore, MEMBERSHIP_SCHEMA } from "./membership-sql.mjs";
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
const UUID =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

export async function createSelfHostedIssuer(
  config,
  { placement, mail, fetchCore = coreRequest, deliveryIntervalMs } = {},
) {
  if (typeof mail?.send !== "function")
    throw Error("A Mail port is required for verification and invitations");
  const companies = config.companies ?? [{ companyId: config.companyId, cellId: config.cellId, companyName: config.companyName }];
  const defaultCompanyId = placement.defaultCompanyId;
  const configuredCompany = (id) => companies.find(company => company.companyId === id);
  const slugFor = (id) => `restless-${id}`;
  const origin = new URL(config.origin);
  const invitationLink = (id) =>
    `${config.origin}/?invite=${encodeURIComponent(id)}`;
  const pool = new pg.Pool({
    connectionString: config.databaseUrl,
    max: 8,
    connectionTimeoutMillis: 5000,
    statement_timeout: 10000,
  });
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
        organizationLimit: companies.length,
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
    await pool.query(MEMBERSHIP_SCHEMA);
    await migrateLegacyRemovals();
  } catch (error) {
    await pool.end();
    throw error;
  }
  const auth = betterAuth(options);
  const store = await createSqlMembershipStore({
    pool,
    origin: config.origin,
    signingKey: config.signingKey,
    placement,
    fetchCore,
    deliveryIntervalMs,
  });
  const issuer = createIssuer({
    auth,
    origin: config.origin,
    placement,
    store,
    invitationLink,
  });
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

  // Before the shared library, pending removals lived in a bare table and
  // entry signed constant versions (1, then 2 for removal). Carry any pending
  // removal into the durable outbox exactly once.
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
        const rowCompany = row.slug?.startsWith("restless-")
          ? row.slug.slice("restless-".length)
          : null;
        // A removal whose member row is gone was already confirmed by Core.
        if (!row.organization_id || !UUID.test(rowCompany ?? "")) continue;
        await client.query(
          `INSERT INTO restless_membership_state
             (membership_id, organization_id, user_id, company_id, role, status, version)
           VALUES ($1,$2,$3,$4,$5,'removed',2) ON CONFLICT (membership_id) DO NOTHING`,
          [row.membership_id, row.organization_id, row.user_id, rowCompany, row.role],
        );
        await client.query(
          `INSERT INTO restless_membership_controls
             (jti, membership_id, company_id, user_id, role, status, version)
           VALUES ($1,$2,$3,$4,$5,'removed',2) ON CONFLICT DO NOTHING`,
          [randomUUID(), row.membership_id, rowCompany, row.user_id, row.role],
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

  async function organizationRow(companyId, client = pool) {
    return (
      await client.query("SELECT id, name FROM organization WHERE slug=$1", [slugFor(companyId)])
    ).rows[0];
  }
  async function account(request) {
    const session = await auth.api.getSession({
      headers: new Headers({ cookie: request.headers.get("cookie") ?? "" }),
      query: { disableCookieCache: true },
    });
    if (!session?.user?.emailVerified)
      refuse(401, "Sign in with a verified email to continue.");
    return session.user;
  }
  async function bootstrap(user, companyId) {
    const company = configuredCompany(companyId);
    if (!company) refuse(404, "Company not found.");
    if (user.email.toLowerCase() !== config.ownerEmail)
      refuse(403, "Only the configured owner can open this company for the first time.");
    const client = await pool.connect();
    try {
      await client.query("BEGIN");
      await client.query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))", [slugFor(companyId)]);
      if (!(await organizationRow(companyId, client)))
        await auth.api.createOrganization({
          body: { userId: user.id, name: company.companyName, slug: slugFor(companyId) },
        });
      await client.query("COMMIT");
    } catch (error) {
      await client.query("ROLLBACK");
      throw error;
    } finally {
      client.release();
    }
  }
  const pageHeaders = {
    "X-Content-Type-Options": "nosniff",
    "Referrer-Policy": "no-referrer",
    "Cache-Control": "no-store",
    "Content-Security-Policy": `default-src 'self'; script-src 'self'; style-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self' ${config.coreOrigin}`,
  };
  const withHeaders = (response, headers) => {
    for (const [name, value] of Object.entries(headers))
      if (!response.headers.has(name)) response.headers.set(name, value);
    return response;
  };

  async function fetch(request) {
    if (request.headers.get("host")?.toLowerCase() !== origin.host.toLowerCase())
      return json(403, { message: "This hostname does not belong to this account service." });
    if (request.method === "GET" && new URL(request.url).pathname === "/.well-known/jwks.json")
      return json(200, store.jwks, { "Cache-Control": "public, max-age=60, must-revalidate" });
    const shared = await issuer.handle(request);
    if (shared) return withHeaders(shared, { "X-Content-Type-Options": "nosniff" });
    try {
      const requestUrl = new URL(request.url);
      const path = requestUrl.pathname;
      const companyId = requestUrl.searchParams.get("company") ?? defaultCompanyId;
      const selected = configuredCompany(companyId);
      if (request.method === "GET" && path === "/health")
        return json(200, { status: "ready" });
      if (request.method === "POST" && request.headers.get("origin") !== config.origin)
        refuse(403, "Open this account page directly before making changes.");
      if (path.startsWith("/api/auth/")) {
        const route = path.slice("/api/auth/".length);
        const resetCallback =
          request.method === "GET" && /^reset-password\/[A-Za-z0-9_-]{1,256}$/.test(route);
        if (!AUTH_ROUTES.has(route) && !resetCallback)
          refuse(404, "Account action not found.");
        return withHeaders(await auth.handler(request), pageHeaders);
      }
      if (request.method === "GET" && ASSETS.has(path)) {
        const asset = assets.get(path);
        return new Response(asset.body, {
          headers: { ...pageHeaders, "Content-Type": asset.type },
        });
      }
      if (request.method === "GET" && path === "/api/settings")
        return json(200, { companyName: selected?.companyName ?? config.companyName, coreOrigin: config.coreOrigin });
      const user = await account(request);
      if (request.method === "GET" && path === "/api/company") {
        if (!selected) refuse(404, "Company not found.");
        const member = await store.membership(companyId, user.id);
        const active = member?.status === "active" && !member.ending;
        const choices = [];
        for (const company of companies) {
          const membership = company.companyId === companyId ? member : await store.membership(company.companyId, user.id);
          const canBootstrap = user.email.toLowerCase() === config.ownerEmail && !(await organizationRow(company.companyId));
          if ((membership && membership.status !== "removed") || canBootstrap) choices.push({ id: company.companyId, name: company.companyName });
        }
        return json(200, {
          user: { name: user.name, email: user.email },
          companyId, companyName: selected.companyName, companies: choices,
          role: active ? member.role : null,
          status: member?.status ?? null,
          available: active && placement.checkReady ? await placement.checkReady(companyId) : null,
          canBootstrap:
            !(await organizationRow(companyId)) && user.email.toLowerCase() === config.ownerEmail,
        });
      }
      if (request.method !== "POST") refuse(404, "Page not found.");
      const input = await readJson(request);
      if (path === "/api/company/bootstrap") {
        await bootstrap(user, input.companyId ?? companyId);
        return json(200, { ready: true });
      }
      if (path === "/api/invitations/accept") {
        const invitation = (
          await pool.query('SELECT invitation."organizationId", organization.slug FROM invitation JOIN organization ON organization.id = invitation."organizationId" WHERE invitation.id=$1', [input.id])
        ).rows[0];
        const invitedCompany = invitation?.slug?.startsWith("restless-") ? invitation.slug.slice("restless-".length) : null;
        if (!configuredCompany(invitedCompany))
          refuse(404, "This invitation is no longer available.");
        await auth.api.acceptInvitation({
          headers: new Headers({ cookie: request.headers.get("cookie") ?? "" }),
          body: { invitationId: input.id },
        });
        return json(200, { accepted: true, companyId: invitedCompany });
      }
      if (path === "/api/enter") {
        const target = input.companyId ?? companyId;
        const member = await store.membership(target, user.id);
        if (!member || member.status !== "active" || member.ending)
          refuse(403, "You do not currently have access to this company.");
        if (placement.checkReady && !(await placement.checkReady(target)))
          refuse(503, "The company is offline. It will be available when its host reconnects. Your account and membership are unchanged.");
        return json(200, await store.enter(user, target));
      }
      refuse(404, "Account action not found.");
    } catch (error) {
      return failure(error);
    }
  }
  return {
    fetch,
    handle: toNodeHandler(fetch),
    deliverDue: store.deliverDue,
    metadata: issuer.metadata,
    async close() {
      await store.close();
      await pool.end();
    },
  };
}
