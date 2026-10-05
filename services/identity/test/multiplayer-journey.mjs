// Multiplayer smoke: three real human accounts in one company, over network entry.
//
// Started by run-core.mjs (RESTLESS_TEST_JOURNEY=multiplayer-journey.mjs), which owns
// the isolated state, databases, containers and their removal. This journey owns the
// processes it starts: a loopback SMTP catcher, the production account entrypoint
// (src/main.mjs) and Core. Every person uses a separate browser context and their
// own verified account. The account service and Core share one hostname on two
// ports, the same shape a Tailscale-shared host uses.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { closeSync, openSync } from "node:fs";
import { readFile, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { lookup } from "node:dns/promises";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import pg from "pg";
import { setup } from "../scripts/setup.mjs";

const {
  RESTLESS_HOME: root,
  RESTLESS_AUTH_TEST_PORT: authPort,
  RESTLESS_PORT_OFFSET: offset,
  RESTLESS_TEST_COMPANY: company,
  RESTLESS_TEST_REPO: repo,
} = process.env;
assert(company.endsWith("_test"), "the smoke only ever creates _test companies");
const serviceDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const evidence = dirname(process.env.RESTLESS_TEST_RESULT);
const host = "plane.localhost";
const corePort = Number(offset) + 7788;
const core = `http://${host}:${corePort}`;
const accounts = `http://${host}:${authPort}`;
const base = `/api/companies/${company}`;
const password = "RestlessMultiplayer!123";
const people = {
  owner: { name: "Mara Owner", email: "owner@multiplayer.test" },
  alice: { name: "Alice Member", email: "alice@multiplayer.test" },
  bob: { name: "Bob Member", email: "bob@multiplayer.test" },
};
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const steps = [];
// Defects the smoke observes but does not fail on; each names what a person sees.
const notes = [];
function note(text) {
  notes.push(text);
  console.log(`NOTE ${text}`);
}
function pass(step) {
  steps.push(step);
  console.log(`PASS ${step}`);
}
async function until(check, label, seconds = 60) {
  const end = Date.now() + seconds * 1000;
  let last;
  while (Date.now() < end) {
    try {
      if (await check()) return;
    } catch (error) {
      last = error;
    }
    await pause(500);
  }
  throw Error(`Timed out: ${label}${last ? ` (${last.message})` : ""}`);
}

// ---------------------------------------------------------------- SMTP catcher
// A loopback SMTP server that keeps every message. Nothing leaves the host.
function decodeQuotedPrintable(text) {
  return text
    .replace(/=\r?\n/g, "")
    .replace(/=([0-9A-F]{2})/g, (_, hex) => String.fromCharCode(parseInt(hex, 16)));
}
function parseMail(raw) {
  const split = raw.indexOf("\r\n\r\n");
  const head = raw.slice(0, split).replace(/\r\n[ \t]+/g, " ");
  let body = raw.slice(split + 4);
  const header = (name) =>
    head.match(new RegExp(`^${name}:\\s*(.*)$`, "im"))?.[1]?.trim() ?? "";
  if (/quoted-printable/i.test(header("Content-Transfer-Encoding"))) body = decodeQuotedPrintable(body);
  if (/base64/i.test(header("Content-Transfer-Encoding")))
    body = Buffer.from(body.replace(/\s+/g, ""), "base64").toString("utf8");
  const to = header("To").replace(/^.*</, "").replace(/>.*$/, "").toLowerCase();
  return { to, subject: header("Subject"), text: body, link: body.match(/https?:\/\/\S+/)?.[0] };
}
async function smtpCatcher() {
  const messages = [];
  const sockets = new Set();
  const server = createServer((socket) => {
    sockets.add(socket);
    socket.once("close", () => sockets.delete(socket));
    socket.on("error", () => {});
    socket.setEncoding("utf8");
    socket.write("220 multiplayer.test capture\r\n");
    let buffer = "",
      data = "",
      dataMode = false;
    socket.on("data", (chunk) => {
      buffer += chunk;
      if (buffer.length > 262144) return socket.destroy();
      let at;
      while ((at = buffer.indexOf("\r\n")) >= 0) {
        let line = buffer.slice(0, at);
        buffer = buffer.slice(at + 2);
        if (dataMode) {
          if (line === ".") {
            dataMode = false;
            messages.push(parseMail(data));
            socket.write("250 2.0.0 captured\r\n");
          } else data += (line.startsWith("..") ? line.slice(1) : line) + "\r\n";
          continue;
        }
        const command = line.split(/\s+/, 1)[0].toUpperCase();
        if (command === "EHLO" || command === "HELO") socket.write("250-multiplayer.test\r\n250 SIZE 262144\r\n");
        else if (["MAIL", "RCPT", "RSET", "NOOP"].includes(command)) socket.write("250 2.1.0 ok\r\n");
        else if (command === "DATA") {
          dataMode = true;
          data = "";
          socket.write("354 go ahead\r\n");
        } else if (command === "QUIT") socket.end("221 2.0.0 bye\r\n");
        else socket.write("502 5.5.2 unsupported\r\n");
      }
    });
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  return {
    port: server.address().port,
    messages,
    async waitFor(match, label) {
      let found;
      await until(() => (found = messages.findLast(match)), `mail: ${label}`, 30);
      return found;
    },
    async close() {
      for (const socket of sockets) socket.destroy();
      await new Promise((resolve) => server.close(resolve));
    },
  };
}

// ---------------------------------------------------------------- processes
const logFd = openSync(process.env.RESTLESS_TEST_DAEMON_LOG, "w");
const identityLogFd = openSync(join(evidence, "accounts.log"), "w");
let smtp, daemon, identity, browser, cellPool;
const contexts = [];

function startDaemon(network, entry = {}) {
  daemon = spawn(process.env.RESTLESS_TEST_DAEMON, [], {
    cwd: repo,
    env: {
      ...process.env,
      RESTLESS_OWNER_ADDR: `127.0.0.1:${corePort}`,
      RESTLESS_ENTRY_MODE: network ? "network" : "local",
      RESTLESS_RUNTIME_MODE: "local",
      ...entry,
    },
    stdio: ["ignore", logFd, logFd],
  });
}
async function stopChild(child) {
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  await new Promise((resolve) => {
    const timer = setTimeout(() => {
      child.kill("SIGKILL");
      resolve();
    }, 10000);
    child.once("exit", () => {
      clearTimeout(timer);
      resolve();
    });
    child.kill("SIGTERM");
  });
}
async function health() {
  const response = await fetch(`${core}/health`).catch(() => null);
  return response?.ok ?? false;
}

// ---------------------------------------------------------------- browser
let lastAuthAction = 0;
// Better Auth limits sign-up and sign-in to three per ten seconds per address;
// every person here shares loopback, so space those actions out.
async function authGate() {
  const wait = lastAuthAction + 3600 - Date.now();
  if (wait > 0) await pause(wait);
  lastAuthAction = Date.now();
}
async function person(key) {
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const sockets = [];
  page.on("websocket", (socket) => sockets.push({ url: socket.url(), socket, closed: false }));
  page.on("websocket", (socket) =>
    socket.on("close", () => {
      const entry = sockets.find((item) => item.socket === socket);
      if (entry) entry.closed = true;
    }),
  );
  const who = { key, ...people[key], context, page, errors, sockets };
  contexts.push(who);
  return who;
}
async function shot(who, name) {
  await who.page.screenshot({ path: join(evidence, `${who.key}-${name}.png`), fullPage: false }).catch(() => {});
}
async function coreApi(who, path, { method = "GET", body, expected = 200 } = {}) {
  const response = await who.context.request.fetch(core + path, {
    method,
    headers: { Origin: core, "Content-Type": "application/json", "Idempotency-Key": randomUUID() },
    data: body === undefined ? undefined : JSON.stringify(body),
    failOnStatusCode: false,
  });
  const text = await response.text();
  assert.equal(response.status(), expected, `${method} ${path} as ${who.key}: ${text.slice(0, 300)}`);
  return text ? JSON.parse(text) : null;
}

// Account site, driven exactly as a person would: create, verify by email, sign in.
async function createAccount(who, start = "/") {
  const { page } = who;
  await page.goto(accounts + start);
  await page.locator("#switch-mode").click();
  await page.locator("#name").fill(who.name);
  await page.locator("#email").fill(who.email);
  await page.locator("#password").fill(password);
  await authGate();
  await page.locator("#submit-account").click();
  await page.getByText("Check your email to verify your account").waitFor();
  const mail = await smtp.waitFor(
    (m) => m.to === who.email && m.subject === "Verify your Restless email",
    `${who.key} verification`,
  );
  await page.goto(mail.link);
  await page.locator("#email").waitFor();
  await page.locator("#email").fill(who.email);
  await page.locator("#password").fill(password);
  await authGate();
  await page.locator("#submit-account").click();
  await page.locator("#company").waitFor({ state: "visible" });
}
async function openCompany(who) {
  await who.page.locator("#enter-company").click();
  await who.page.waitForURL((url) => url.origin === core, { timeout: 30000 });
  who.principal = await coreApi(who, `${base}/principal`);
}

// ---------------------------------------------------------------- journey
try {
  // Core and Node resolve the shared hostname through the OS, not the browser.
  const resolved = await lookup(host).catch(() => null);
  assert(
    resolved && ["127.0.0.1", "::1"].includes(resolved.address),
    `${host} must resolve to loopback on this host (systemd-resolved does; otherwise add "127.0.0.1 ${host}" to /etc/hosts)`,
  );
  smtp = await smtpCatcher();

  // 1. A local company exists first, as on any owner's host.
  startDaemon(false);
  await until(async () => {
    if (daemon.exitCode !== null) throw Error("Core exited during local start; see core.log");
    cellPool ??= new pg.Pool({
      connectionString: (await readFile(`${root}/cells/${company}/database.url`, "utf8")).trim(),
      max: 2,
    });
    const rows = (await cellPool.query(`SELECT company_id, cell_id FROM "${company}".company_access_identity`)).rows;
    if (!rows[0]) return false;
    await readFile(`${root}/native-documents/${rows[0].cell_id}.json`);
    return true;
  }, "local company bootstrap", 120);
  pass("local Core created the _test company");

  // 2. Members → Enable sharing prepares the plan; the account installer runs on it.
  const prepared = await fetch(`http://127.0.0.1:${corePort}${base}/sharing/setup`, {
    method: "POST",
    headers: { Origin: `http://127.0.0.1:${corePort}`, "Content-Type": "application/json" },
    body: JSON.stringify({ account_origin: accounts, core_origin: core, owner_email: people.owner.email, access: "private" }),
  });
  assert.equal(prepared.status, 200, await prepared.clone().text());
  await writeFile(`${root}/sharing-plan.json`, JSON.stringify(await prepared.json()), { mode: 0o600 });
  await stopChild(daemon);
  await writeFile(`${root}/account-database.url`, process.env.RESTLESS_AUTH_TEST_DATABASE, { mode: 0o600 });
  await writeFile(
    `${root}/smtp.json`,
    JSON.stringify({ host: "127.0.0.1", port: smtp.port, secure: false, ignoreTLS: true, from: "Restless smoke <accounts@multiplayer.test>" }),
    { mode: 0o600 },
  );
  const installed = await setup({
    plan: `${root}/sharing-plan.json`,
    databaseUrlFile: `${root}/account-database.url`,
    smtpFile: `${root}/smtp.json`,
    output: `${root}/accounts`,
    companyImage: process.env.RESTLESS_COMPANY_IMAGE,
    port: authPort,
    corePort: String(corePort),
  });
  const entry = Object.fromEntries(
    (await readFile(installed.coreEnvironment, "utf8"))
      .trim()
      .split("\n")
      .map((line) => line.replace(/^export /, "").split("="))
      .map(([name, ...value]) => [name, value.join("=").replace(/^'|'$/g, "")]),
  );
  assert.equal(entry.RESTLESS_ENTRY_HOST, host);
  pass("sharing plan and account installer prepared one host on two ports");

  // 3. The production account entrypoint with SMTP mail, then Core in network entry.
  identity = spawn(process.execPath, [join(serviceDir, "src/main.mjs")], {
    cwd: serviceDir,
    env: { PATH: process.env.PATH, HOME: process.env.HOME, RESTLESS_IDENTITY_CONFIG: installed.configPath },
    stdio: ["ignore", identityLogFd, identityLogFd],
  });
  await until(async () => (await fetch(`${accounts}/.well-known/restless-issuer`).catch(() => null))?.ok, "account service", 60);
  startDaemon(true, entry);
  await until(async () => {
    if (daemon.exitCode !== null) throw Error("Core exited in network mode; see core.log");
    return health();
  }, "Core network entry", 120);
  pass("account service (SMTP) and Core network entry with local runtime are up");

  const { chromium } = createRequire(join(repo, "web/package.json"))("playwright");
  browser = await chromium.launch({ executablePath: process.env.RESTLESS_BROWSER_EXECUTABLE });
  const owner = await person("owner"),
    alice = await person("alice"),
    bob = await person("bob");

  // 4. Owner signs up, claims the company and opens it.
  await createAccount(owner);
  await owner.page.locator("#bootstrap-company").click();
  await owner.page.locator("#enter-company").waitFor({ state: "visible" });
  await openCompany(owner);
  assert.equal(owner.principal.actor_id, "owner", "the first network owner keeps the local owner Actor");
  await shot(owner, "entered");
  pass("owner verified by email, claimed the company and entered it");

  // 5. Owner invites both colleagues from Company → Members.
  await owner.page.goto(`${core}/${company}/company/members`);
  for (const who of [alice, bob]) {
    const field = owner.page.getByLabel("Email to invite");
    if (!(await field.isVisible())) await owner.page.getByRole("button", { name: "Invite", exact: true }).click();
    await field.fill(who.email);
    await owner.page.getByRole("button", { name: "Send invite" }).click();
    await owner.page.getByText(`Invitation sent to ${who.email}.`).waitFor();
    // The form clears only after the list reloads; typing earlier is discarded.
    await until(async () => (await field.inputValue()) === "", "invite form cleared", 15);
    who.invitation = await smtp.waitFor((m) => m.to === who.email && m.subject.startsWith("Join "), `${who.key} invitation`);
  }
  await shot(owner, "invited");
  pass("owner invited two colleagues from Members; invitations reached the SMTP catcher");

  // 6. Each colleague follows the emailed link, creates and verifies an account,
  //    accepts and opens the company.
  for (const who of [alice, bob]) {
    const invite = new URL(who.invitation.link);
    await createAccount(who, invite.pathname + invite.search);
    await who.page.locator("#accept-invitation").click();
    await who.page.getByText("You’ve joined the company.").waitFor();
    await openCompany(who);
    assert.equal(who.principal.membership_role, "member");
    await shot(who, "entered");
  }
  const actors = new Set([owner, alice, bob].map((who) => who.principal.actor_id));
  assert.equal(actors.size, 3, "three people are three distinct Actors");
  pass("both colleagues accepted by email link and entered as distinct member Actors");

  // 7. One native document, three editors at once; every edit survives reload.
  const created = await coreApi(owner, `${base}/documents`, {
    method: "POST",
    expected: 201,
    body: {
      title: "Multiplayer plan",
      kind: "brief",
      visibility: "participants",
      content_json: { type: "doc", content: [{ type: "paragraph", attrs: { block_id: randomUUID() }, content: [{ type: "text", text: "Shared plan." }] }] },
      reason: "Multiplayer smoke",
    },
  });
  const documentId = created.document_id;
  for (const who of [alice, bob]) {
    const current = (await coreApi(owner, `${base}/documents/${documentId}`)).document;
    await coreApi(owner, `${base}/documents/${documentId}/participants/${who.principal.actor_id}`, {
      method: "PUT",
      body: { expected_document_version: current.version, access: "edit" },
    });
  }
  const documentUrl = `${core}/documents/${company}/${documentId}`;
  const editors = [owner, alice, bob];
  for (const who of editors) {
    await who.page.goto(documentUrl);
    who.body = who.page.getByLabel("Collaborative document body");
    await who.body.waitFor({ timeout: 30000 });
    // Type only into a synced document, as a person reading it first would.
    await who.body.getByText("Shared plan.").waitFor({ timeout: 30000 });
  }
  const line = (who) => `${who.name} wrote this line live.`;
  await Promise.all(
    editors.map(async (who) => {
      await who.body.click();
      await who.page.keyboard.press("Control+End");
      await who.page.keyboard.press("Enter");
      // Human-speed keystrokes, so remote edits arrive while each person types.
      await who.page.keyboard.type(line(who), { delay: 40 });
    }),
  );
  for (const who of editors) for (const author of editors) await who.page.getByText(line(author)).waitFor({ timeout: 20000 });
  for (const who of editors) await shot(who, "coediting");
  for (const who of editors) {
    await who.page.reload();
    await who.page.getByLabel("Collaborative document body").waitFor({ timeout: 30000 });
    for (const author of editors) await who.page.getByText(line(author)).waitFor({ timeout: 20000 });
  }
  pass("owner and both members typed into one document at once; all three edits survive reload");

  // 8. Room chat: a group room with all three. Owner and Alice type in the
  //    conversation; every participant's browser shows both messages.
  const room = await coreApi(owner, `${base}/rooms`, {
    method: "POST",
    body: { kind: "group", title: "Launch room", command_id: randomUUID(), participant_actor_ids: [alice.principal.actor_id, bob.principal.actor_id] },
  });
  const roomUrl = `${core}/${company}/people?room=${room.id}`;
  for (const who of editors) {
    await who.page.goto(roomUrl);
    await who.page.getByRole("textbox", { name: "Conversation message" }).waitFor({ timeout: 30000 });
  }
  const chat = ["Owner: kickoff at nine.", "Alice: I will bring the draft."];
  for (const [who, text] of [[owner, chat[0]], [alice, chat[1]]]) {
    const composer = who.page.getByRole("textbox", { name: "Conversation message" });
    await composer.fill(text);
    await composer.press("Enter");
  }
  for (const who of editors) for (const text of chat) await who.page.getByText(text).first().waitFor({ timeout: 20000 });
  for (const who of editors) await shot(who, "room");
  pass("room chat typed by owner and a member appears live for all three people");

  // 9. Attention: the owner asks Alice by name. It reaches only Alice's attention;
  //    she answers in the thread from her browser. The cockpit has no control that
  //    resolves a mention, so resolution uses the product API a client would call.
  const question = "Alice, can you confirm the budget?";
  const asked = await coreApi(owner, `${base}/rooms/${room.id}/messages`, {
    method: "POST",
    expected: 201,
    body: {
      body: question,
      command_id: randomUUID(),
      mentions: [{ actor_id: alice.principal.actor_id, expected_response: "Confirm the budget" }],
    },
  });
  const askedId = asked.message?.id ?? asked.id;
  let mention;
  await until(async () => {
    const bootstrap = await coreApi(alice, `${base}/collaboration/bootstrap`);
    mention = bootstrap.attention.mentions.find((item) => item.room_id === room.id);
    return mention;
  }, "Alice's attention shows the mention", 30);
  assert.equal(
    (await coreApi(bob, `${base}/collaboration/bootstrap`)).attention.mentions.length,
    0,
    "a mention reaches only the person named",
  );
  const asking = alice.page.locator("article.room-message").filter({ hasText: question }).first();
  await asking.waitFor({ timeout: 30000 });
  await asking.hover();
  await asking.getByRole("button", { name: "Reply" }).click();
  const reply = alice.page.getByRole("textbox", { name: "Thread reply" });
  await reply.fill("Confirmed: the budget holds.");
  await reply.press("Enter");
  await alice.page.getByText("Confirmed: the budget holds.").first().waitFor({ timeout: 20000 });
  await shot(alice, "attention-reply");
  await until(
    async () => JSON.stringify(await coreApi(owner, `${base}/rooms/${room.id}/threads/${askedId}`)).includes("Confirmed: the budget holds."),
    "the owner can read Alice's thread answer",
    20,
  );
  const aliceView = await coreApi(alice, `${base}/collaboration/bootstrap`);
  const stillOpen = aliceView.attention.mentions.some((item) => item.id === mention.id);
  if (stillOpen) note("answering a mention from the cockpit thread does not resolve it; no cockpit control sends resolves_mention_id");
  const ownerSeen = aliceView.people.find((item) => item.actor_id === owner.principal.actor_id);
  if (ownerSeen?.display !== people.owner.name)
    note(`members see the owner as ${JSON.stringify(ownerSeen?.display ?? null)}, not ${JSON.stringify(people.owner.name)}`);
  if (await asking.getByText(alice.principal.actor_id).count())
    note("the mention receipt shows the raw actor id instead of the person's name");
  await coreApi(alice, `${base}/rooms/${room.id}/messages/${askedId}/replies`, {
    method: "POST",
    expected: 201,
    body: { body: "Resolving the request.", command_id: randomUUID(), resolves_mention_id: mention.id },
  });
  await until(async () => {
    const bootstrap = await coreApi(alice, `${base}/collaboration/bootstrap`);
    return !bootstrap.attention.mentions.some((item) => item.id === mention.id);
  }, "Alice's resolving answer clears her attention item", 30);
  pass(
    `a mention reached only Alice; she answered in the thread from her browser${stillOpen ? " (the browser answer left it open)" : ""} and resolving it cleared her attention`,
  );

  // 10. Removal from Members closes Bob's live document connection and session.
  await bob.page.goto(documentUrl);
  await bob.page.getByLabel("Collaborative document body").waitFor({ timeout: 30000 });
  await until(() => bob.sockets.some((item) => item.url.includes("/collaboration") && !item.closed), "Bob's live document connection", 30);
  await owner.page.goto(`${core}/${company}/company/members`);
  await owner.page.locator(`summary[aria-label="Access for ${people.bob.name}"]`).click();
  await owner.page.getByRole("button", { name: "Remove…" }).click();
  await owner.page.getByRole("button", { name: "Remove access" }).click();
  await owner.page.getByText(`${people.bob.name} no longer has access.`).waitFor();
  await until(() => bob.sockets.filter((item) => item.url.includes("/collaboration")).every((item) => item.closed), "removal closes Bob's document connection", 30);
  await coreApi(bob, `${base}/principal`, { expected: 401 });
  await pause(5000);
  if (await bob.page.getByText("Reconnecting").count())
    note("a removed member's open document says Reconnecting, not that access ended");
  await coreApi(alice, `${base}/principal`);
  await shot(bob, "removed");
  await bob.page.goto(`${accounts}/`);
  await bob.page.locator("#company").waitFor({ state: "visible" });
  assert(await bob.page.locator("#enter-company").isHidden(), "a removed member cannot open the company again");
  pass("owner removed Bob from Members; his live document connection and Core session closed, re-entry is refused");

  for (const who of contexts) assert.deepEqual(who.errors, [], `${who.key} page errors`);
  await writeFile(process.env.RESTLESS_TEST_RESULT, JSON.stringify({ journey: "multiplayer", people: 3, steps, notes }, null, 2));
} catch (error) {
  // Keep what each person was looking at when the smoke failed.
  for (const who of contexts) {
    await shot(who, "failure");
    console.error(`${who.key} at ${who.page.url()}; page errors: ${JSON.stringify(who.errors)}`);
  }
  throw error;
} finally {
  if (browser) await browser.close().catch(() => {});
  await stopChild(daemon);
  await stopChild(identity);
  if (cellPool) await cellPool.end().catch(() => {});
  if (smtp) await smtp.close();
  closeSync(logFd);
  closeSync(identityLogFd);
  console.log("PASS multiplayer journey processes stopped");
}
