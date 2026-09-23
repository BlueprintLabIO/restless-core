// Issuer behaviour against a real PostgreSQL database and a signature-checking
// stand-in for Core's membership-control endpoint. Core's own verifier is
// exercised separately by test/run-core.mjs.
import assert from "node:assert/strict";
import { createServer } from "node:http";
import { generateKeyPairSync, randomUUID } from "node:crypto";
import { readFile } from "node:fs/promises";
import { createLocalJWKSet, jwtVerify } from "jose";
import pg from "pg";
import { validateConfig } from "../src/config.mjs";
import { createIssuer } from "../src/issuer.mjs";
import { staticPlacement } from "../src/placement-static.mjs";

const adminUrl = new URL(
  (await readFile(process.env.RESTLESS_TEST_DATABASE_URL_FILE, "utf8")).trim(),
);
const database = `identity_it_${randomUUID().slice(0, 8)}`;
const admin = new pg.Client({ connectionString: adminUrl.toString() });
await admin.connect();
await admin.query(`CREATE DATABASE ${database}`);
const databaseUrl = new URL(adminUrl);
databaseUrl.pathname = "/" + database;

const accountPort = 46000 + Math.floor(Math.random() * 1000),
  corePort = accountPort + 1000;
const origin = `http://127.0.0.1:${accountPort}`,
  coreOrigin = `http://plane.localhost:${corePort}`;
const { privateKey } = generateKeyPairSync("ed25519");
const config = validateConfig({
  origin,
  coreOrigin,
  companyName: "Issuer Test",
  ownerEmail: "owner@issuer.test",
  ownerId: randomUUID(),
  planeId: randomUUID(),
  companyId: randomUUID(),
  cellId: randomUUID(),
  secret: "issuer-test-secret-".repeat(4),
  signingKey: { ...privateKey.export({ format: "jwk" }), kid: "issuer-test" },
  databaseUrl: databaseUrl.toString(),
  port: accountPort,
});

// ---- stand-in Core: verifies each control and keeps monotonic state ----
const jwks = createLocalJWKSet({
  keys: [{ ...privateKey.export({ format: "jwk" }), d: undefined, kid: "issuer-test", alg: "EdDSA" }],
});
let coreUp = true;
const coreState = new Map();
const controls = [];
const core = createServer(async (req, res) => {
  if (!coreUp) {
    res.writeHead(503);
    return res.end();
  }
  const chunks = [];
  for await (const chunk of req) chunks.push(chunk);
  const { control } = JSON.parse(Buffer.concat(chunks).toString());
  const { payload, protectedHeader } = await jwtVerify(control, jwks, {
    issuer: origin,
    audience: "restless-core-membership-control",
    typ: "restless-membership-control+jwt",
  });
  assert.equal(protectedHeader.alg, "EdDSA");
  assert.equal(payload.company_id, config.companyId);
  assert.equal(payload.plane_hostname, "plane.localhost");
  controls.push(payload);
  const prior = coreState.get(payload.membership_id);
  const superseded = prior && prior.version > payload.membership_version;
  if (!superseded)
    coreState.set(payload.membership_id, {
      status: payload.membership_status,
      version: payload.membership_version,
    });
  const observed = coreState.get(payload.membership_id);
  res.writeHead(200, { "Content-Type": "application/json" });
  res.end(
    JSON.stringify({
      contract_version: 1,
      jti: payload.jti,
      company_id: payload.company_id,
      cell_id: payload.cell_id,
      membership_id: payload.membership_id,
      requested_status: payload.membership_status,
      requested_version: payload.membership_version,
      outcome: superseded
        ? "superseded"
        : prior?.version === payload.membership_version
          ? "already_applied"
          : "applied",
      observed_status: observed.status,
      observed_version: observed.version,
    }),
  );
});
await new Promise((resolve) => core.listen(corePort, "127.0.0.1", resolve));

const messages = [];
const issuer = await createIssuer(config, {
  placement: staticPlacement(config),
  mail: { send: async (message) => messages.push(message) },
  // plane.localhost resolves to loopback in browsers; route it for Node.
  fetchCore: (url, options) =>
    fetch(url.replace("plane.localhost", "127.0.0.1"), options),
  deliveryIntervalMs: 250,
});
const server = createServer(issuer.handle);
await new Promise((resolve) => server.listen(accountPort, "127.0.0.1", resolve));

const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
async function until(check, label, seconds = 20) {
  const end = Date.now() + seconds * 1000;
  while (Date.now() < end) {
    if (await check()) return;
    await pause(200);
  }
  throw Error(`Timed out: ${label}`);
}
async function call(path, { body, cookie = "", requestOrigin = origin, method, expected = 200 } = {}) {
  const response = await fetch(origin + path, {
    method: method ?? (body === undefined ? "GET" : "POST"),
    headers: {
      ...(requestOrigin ? { Origin: requestOrigin } : {}),
      "Content-Type": "application/json",
      Cookie: cookie,
    },
    body: body === undefined ? undefined : JSON.stringify(body),
    redirect: "manual",
  });
  const text = await response.text();
  assert.equal(response.status, expected, `${path}: ${text.slice(0, 300)}`);
  return {
    data: text ? JSON.parse(text) : null,
    headers: response.headers,
    cookie: response.headers
      .getSetCookie()
      .map((item) => item.split(";")[0])
      .join("; "),
  };
}
const adminPath = (rest) => `/api/admin/v1/companies/${config.companyId}/${rest}`;
const cockpit = (rest, options = {}) =>
  call(adminPath(rest), { requestOrigin: coreOrigin, ...options });
async function signup(name, email) {
  const password = "IssuerIntegration!123";
  await call("/api/auth/sign-up/email", { body: { name, email, password } });
  const mail = messages.find((m) => m.to === email && m.subject === "Verify your Restless email");
  const verified = await fetch(mail.text.match(/https?:\/\/\S+/)[0], { redirect: "manual" });
  assert([200, 302].includes(verified.status));
  await pause(3500);
  const session = await call("/api/auth/sign-in/email", { body: { email, password } });
  return { email, cookie: session.cookie };
}
const decode = (token) =>
  JSON.parse(Buffer.from(token.split(".")[1], "base64url").toString());

try {
  const metadata = (await call("/.well-known/restless-issuer", { requestOrigin: null })).data;
  assert.equal(metadata.issuer, origin);
  assert.equal(metadata.admin_api, origin + "/api/admin/v1");
  assert.deepEqual(metadata.terminal_statuses, ["suspended", "removed"]);
  console.log("PASS issuer publishes its metadata document");

  const owner = await signup("Owner", "owner@issuer.test");
  await call("/api/company/bootstrap", { body: {}, cookie: owner.cookie });
  const ownerEntry = decode((await call("/api/enter", { body: {}, cookie: owner.cookie })).data.assertion);
  assert.equal(ownerEntry.membership_role, "owner");
  assert.equal(ownerEntry.membership_version, 1);

  // CORS: only this company's cockpit origin may use the account session.
  const preflight = await call(adminPath("members"), {
    method: "OPTIONS",
    requestOrigin: coreOrigin,
    expected: 204,
  });
  assert.equal(preflight.headers.get("access-control-allow-origin"), coreOrigin);
  assert.equal(preflight.headers.get("access-control-allow-credentials"), "true");
  await call(adminPath("members"), { requestOrigin: "https://elsewhere.example", cookie: owner.cookie, expected: 403 });
  await call(adminPath("invitations"), {
    requestOrigin: origin,
    cookie: owner.cookie,
    body: { email: "x@issuer.test", role: "member" },
    expected: 403,
  });
  const listed = await cockpit("members", { cookie: owner.cookie });
  assert.equal(listed.headers.get("access-control-allow-origin"), coreOrigin);
  assert.equal(listed.data.viewer.role, "owner");
  console.log("PASS admin API answers only the company cockpit origin");

  // Invitations carry a copyable link as well as email delivery.
  const invited = (await cockpit("invitations", {
    cookie: owner.cookie,
    body: { email: "colleague@issuer.test", role: "member" },
  })).data.invitation;
  assert.match(invited.link, /\?invite=/);
  assert(messages.some((m) => m.to === "colleague@issuer.test" && m.text.includes(invited.link)));
  const colleague = await signup("Colleague", "colleague@issuer.test");
  await call("/api/invitations/accept", { body: { id: invited.id }, cookie: colleague.cookie });
  let entry = decode((await call("/api/enter", { body: {}, cookie: colleague.cookie })).data.assertion);
  assert.equal(entry.membership_role, "member");
  assert.equal(entry.membership_version, 1);
  const membershipId = entry.membership_id;
  await cockpit("members", { cookie: colleague.cookie, expected: 403 });
  console.log("PASS invitation link and email; members cannot manage members");

  // Role changes advance the version so Core accepts the new role.
  await cockpit(`members/${membershipId}/role`, { cookie: owner.cookie, body: { role: "admin" } });
  entry = decode((await call("/api/enter", { body: {}, cookie: colleague.cookie })).data.assertion);
  assert.equal(entry.membership_role, "admin");
  assert.equal(entry.membership_version, 2);
  // Admins invite members, never administrators, and cannot change roles.
  await cockpit("invitations", { cookie: colleague.cookie, body: { email: "a2@issuer.test", role: "admin" }, expected: 403 });
  const memberInvite = (await cockpit("invitations", { cookie: colleague.cookie, body: { email: "m2@issuer.test", role: "member" } })).data.invitation;
  await cockpit(`invitations/${memberInvite.id}/cancel`, { cookie: colleague.cookie, body: {} });
  await cockpit(`members/${membershipId}/role`, { cookie: colleague.cookie, body: { role: "member" }, expected: 403 });
  await cockpit(`members/${membershipId}/role`, { cookie: owner.cookie, body: { role: "member" } });
  console.log("PASS role changes are owner-only and advance the membership version");

  // Suspension is delivered to Core, blocks entry, and reinstatement is newer.
  let result = await cockpit(`members/${membershipId}/suspend`, { cookie: owner.cookie, body: {} });
  assert.equal(result.data.ending, false);
  assert.deepEqual(coreState.get(membershipId), { status: "suspended", version: 4 });
  await call("/api/enter", { body: {}, cookie: colleague.cookie, expected: 403 });
  assert.equal((await call("/api/company", { cookie: colleague.cookie })).data.status, "suspended");
  await cockpit(`members/${membershipId}/reinstate`, { cookie: owner.cookie, body: {} });
  entry = decode((await call("/api/enter", { body: {}, cookie: colleague.cookie })).data.assertion);
  assert.equal(entry.membership_version, 5);
  console.log("PASS suspension reaches Core, blocks entry, and reinstatement carries a newer version");

  // Removal while Core is down: entry is blocked at once and delivery completes
  // later from the outbox, across an issuer restart, with no retry click.
  coreUp = false;
  result = await cockpit(`members/${membershipId}/remove`, { cookie: owner.cookie, body: {} });
  assert.equal(result.data.ending, true);
  await call("/api/enter", { body: {}, cookie: colleague.cookie, expected: 403 });
  const pending = (await cockpit("members", { cookie: owner.cookie })).data.members.find((m) => m.membership_id === membershipId);
  assert.equal(pending.status, "removed");
  assert.equal(pending.ending, true);
  await new Promise((resolve) => server.close(resolve));
  await issuer.close();
  const restarted = await createIssuer(config, {
    placement: staticPlacement(config),
    mail: { send: async (message) => messages.push(message) },
    fetchCore: (url, options) => fetch(url.replace("plane.localhost", "127.0.0.1"), options),
    deliveryIntervalMs: 250,
  });
  const restartedServer = createServer(restarted.handle);
  await new Promise((resolve) => restartedServer.listen(accountPort, "127.0.0.1", resolve));
  try {
    await call("/api/enter", { body: {}, cookie: colleague.cookie, expected: 403 });
    coreUp = true;
    await until(() => coreState.get(membershipId)?.status === "removed", "outbox delivers removal after Core returns");
    await until(
      async () => !(await cockpit("members", { cookie: owner.cookie })).data.members.some((m) => m.membership_id === membershipId),
      "removed member leaves the list once Core confirms",
    );
    assert.deepEqual(coreState.get(membershipId), { status: "removed", version: 6 });
    const jtis = controls.filter((c) => c.membership_version === 6).map((c) => c.jti);
    assert.equal(new Set(jtis).size, 1, "retries keep one stable control identity");
    await call("/api/enter", { body: {}, cookie: colleague.cookie, expected: 403 });
    await cockpit(`members/${ownerEntry.membership_id}/remove`, { cookie: owner.cookie, body: {}, expected: 403 });
    console.log("PASS removal during a Core outage completes from the outbox across an issuer restart");
  } finally {
    await new Promise((resolve) => restartedServer.close(resolve));
    await restarted.close();
  }
} finally {
  if (server.listening) {
    await new Promise((resolve) => server.close(resolve));
    await issuer.close().catch(() => {});
  }
  await new Promise((resolve) => core.close(resolve));
  await admin.query(`DROP DATABASE IF EXISTS ${database} WITH (FORCE)`);
  await admin.end();
}
