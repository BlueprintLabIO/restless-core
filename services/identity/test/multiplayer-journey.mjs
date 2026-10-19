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
import { createConnection, createServer } from "node:net";
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

// Exec's side, over the plane's own socket as the owner, exactly as the CLI sends it.
async function daemonCommand(request) {
  const line = await new Promise((resolve, reject) => {
    const socket = createConnection(join(root, "restlessd.sock"));
    let buffer = "";
    socket.setEncoding("utf8");
    socket.on("connect", () => socket.write(JSON.stringify({ principal: "owner", ...request }) + "\n"));
    socket.on("data", (chunk) => {
      buffer += chunk;
      const end = buffer.indexOf("\n");
      if (end >= 0) {
        socket.end();
        resolve(buffer.slice(0, end));
      }
    });
    socket.on("error", reject);
  });
  const response = JSON.parse(line);
  assert(response.ok, `${request.cmd}: ${JSON.stringify(response.error)}`);
  return response.data;
}
const direction = (operation) => daemonCommand({ cmd: "direction-operation", company, direction_operation: operation });

/* After a role change ends a session, that person's open cockpit tab re-enters through the
 * account by itself (session-reentry.ts). The test's own visit to the account can land in the
 * middle of that navigation and be cancelled by it; it is retried, and every attempt is logged so
 * a page that is truly stuck still fails here. */
async function enterAccounts(who) {
  for (let attempt = 1; ; attempt += 1) {
    try {
      await who.page.goto(`${accounts}/`, { timeout: 10000 });
      if (attempt > 1) console.log(`${who.key}: reached the account on attempt ${attempt}`);
      return;
    } catch (error) {
      console.log(`${who.key}: visit to the account interrupted (attempt ${attempt}) at ${who.page.url()}`);
      if (attempt >= 3) throw error;
      await pause(1000);
    }
  }
}

/* Sprint 82: entering the cockpit reveals once, settled, and no message that was shown vanishes.
 * Each role enters its company and the visible regions are watched from the reveal on. */
async function steadyEntry(who, role) {
  await who.page.addInitScript(() => { window.__restlessStrict = true; });
  await who.page.goto("about:blank");
  await who.page.goto(`${core}/${company}`);
  await who.page.waitForSelector(".company-browser-link-capture:not(.first-paint-pending)", { timeout: 20000 });
  const states = await who.page.evaluate(async () => {
    const text = () => [...document.querySelectorAll("main, aside, nav, header")]
      .map((node) => node.innerText.replace(/\s+/g, " ").trim()).join(" || ");
    const seen = [text()];
    const until = performance.now() + 4000;
    while (performance.now() < until) {
      await new Promise((resolve) => requestAnimationFrame(resolve));
      const now = text();
      if (now !== seen.at(-1)) seen.push(now);
    }
    return seen;
  });
  const violations = await who.page.evaluate(() => window.__restlessViolations ?? []);
  assert.deepEqual(violations, [], `${role}: no shown message vanished`);
  // One change is allowed: the office canvas replacing its own loading line.
  assert(states.length <= 2, `${role}: the cockpit changed ${states.length - 1} times after it appeared:\n${states.join("\n---\n")}`);
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

  // 5. Before anyone else is in the company, the owner shares a document with Bob's address; it
  //    waits for him (S66). Then the owner invites both colleagues from Company → Members.
  const welcome = await coreApi(owner, `${base}/documents`, {
    method: "POST",
    expected: 201,
    body: {
      title: "Welcome pack",
      kind: "brief",
      visibility: "participants",
      content_json: { type: "doc", content: [{ type: "paragraph", attrs: { block_id: randomUUID() }, content: [{ type: "text", text: "Read me first." }] }] },
      reason: "Multiplayer smoke",
    },
  });
  await coreApi(owner, `${base}/pending-shares`, {
    method: "POST",
    expected: 204,
    body: { document: welcome.document_id, email: bob.email.toUpperCase(), access: "comment" },
  });
  const waiting = await coreApi(owner, `${base}/pending-shares?document=${welcome.document_id}`);
  assert.deepEqual(waiting.items.map((item) => [item.email, item.access]), [[bob.email.toLowerCase(), "comment"]]);
  await owner.page.goto(`${core}/${company}/company/members`);
  for (const who of [alice, bob]) {
    const field = owner.page.getByLabel("Emails", { exact: true });
    if (!(await field.isVisible())) await owner.page.getByRole("button", { name: "Invite", exact: true }).click();
    await field.fill(who.email);
    await owner.page.getByRole("button", { name: "Send invite" }).click();
    await owner.page.getByText(`Invitation sent to ${who.email}.`).waitFor();
    // The form closes (or clears) only after the list reloads; typing earlier is discarded.
    await until(
      async () => !(await field.isVisible()) || (await field.inputValue()) === "",
      "invite form done",
      15,
    );
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

  // 6b. What waited for Bob's address is his now, shared by the owner; Alice was never given it.
  const given = await coreApi(bob, `${base}/documents/${welcome.document_id}`);
  assert.equal(given.access, "comment");
  await coreApi(alice, `${base}/documents/${welcome.document_id}`, { expected: 404 });
  assert.deepEqual((await coreApi(owner, `${base}/pending-shares?document=${welcome.document_id}`)).items, []);
  const bobForYou = await coreApi(bob, `${base}/for-you?limit=50`);
  assert.ok(
    bobForYou.items.some((item) => item.kind === "share" && item.title === "Welcome pack"),
    "Bob's For you says the owner shared Welcome pack with him",
  );
  pass("a document shared with an invited address was waiting for Bob when he joined, and only for him");

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
      // Into the end of the last line, as a person clicks after what is there.
      await who.body.locator("p").last().click();
      // The editor learns where the caret went from the browser's selectionchange, which comes a
      // moment after the click; a person never presses the next key within that moment.
      await who.page.waitForTimeout(150);
      await who.page.keyboard.press("End");
      await who.page.waitForTimeout(150);
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

  // 8b. A file dropped into a group room reaches everyone in it.
  const picture = join(root, "room-picture.png");
  // A 1x1 PNG is enough for the browser to decode and show.
  await writeFile(picture, Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==", "base64"));
  await alice.page.locator('input[type="file"]').first().setInputFiles(picture);
  const aliceComposer = alice.page.getByRole("textbox", { name: "Conversation message" });
  await aliceComposer.fill("Alice: here is the photo.");
  await aliceComposer.press("Enter");
  for (const who of editors) {
    const sent = who.page.locator("article.room-message").filter({ hasText: "here is the photo" }).first();
    await sent.waitFor({ timeout: 30000 });
    await until(async () => (await sent.locator("img").count()) > 0, `${who.key} sees the attached picture`, 30);
  }
  await shot(owner, "room-attachment");
  pass("a picture attached in a group room appears for all three people");

  // 8c. Typing says so to the others in the room, and sending ends it.
  const typingPosts = [];
  alice.page.on("response", (response) => {
    if (response.url().includes("/typing")) typingPosts.push(response.status());
  });
  const aliceTyping = alice.page.getByRole("textbox", { name: "Conversation message" });
  await aliceTyping.click();
  await aliceTyping.pressSequentially("Thinking about it", { delay: 60 });
  await owner.page
    .getByText(`${alice.name} is typing…`)
    .waitFor({ timeout: 5000 })
    .catch(async (error) => {
      const line = await owner.page.locator(".room-typing").allInnerTexts();
      throw Error(`typing did not reach the owner: POSTs ${JSON.stringify(typingPosts)}, owner line ${JSON.stringify(line)}: ${error.message}`);
    });
  assert.equal(await alice.page.getByText(`${alice.name} is typing…`).count(), 0, "nobody is told they are typing themselves");
  await aliceTyping.fill("");
  await owner.page.getByText(`${alice.name} is typing…`).waitFor({ state: "detached", timeout: 10000 });
  pass("a member typing in a room shows to the others as it happens, and the line goes once she stops");

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
  // The mention reaches only the person named, in their For you.
  const forYou = (who) => coreApi(who, `${base}/for-you?limit=50`);
  let mention;
  await until(async () => {
    mention = (await forYou(alice)).items.find((item) => item.kind === "mention" && item.excerpt.includes("budget"));
    return mention?.unread;
  }, "Alice's For you shows the unread mention", 30);
  assert(
    !(await forYou(bob)).items.some((item) => item.kind === "mention"),
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
  await coreApi(alice, `${base}/for-you/read`, { method: "POST", expected: 204, body: { keys: [mention.key] } });
  assert(
    !(await forYou(alice)).items.find((item) => item.key === mention.key)?.unread,
    "reading a For you item marks it read for that person",
  );
  pass("a mention reached only Alice's For you; she answered in the thread and reading it marked it read");

  // 9b. Working together on a page, as in Notion: faces, comments that reach the person,
  //     replies that come back, reactions, view-only access and the / menu.
  const mentionNode = (who) => ({ type: "mention", attrs: { actor_id: who.principal.actor_id, label: who.name } });
  const commentBody = (text, who) => ({
    type: "doc",
    content: [{ type: "paragraph", attrs: { block_id: randomUUID() }, content: [{ type: "text", text }, ...(who ? [mentionNode(who)] : [])] }],
  });
  for (const who of [owner, alice]) {
    await who.page.goto(documentUrl);
    await who.page.getByLabel("Collaborative document body").waitFor({ timeout: 30000 });
  }
  for (const [who, other] of [[owner, alice], [alice, owner]]) {
    await until(
      // Each face is a button named for the person and what they are doing there.
      async () => (await who.page.locator(".page-faces [aria-label]").evaluateAll((faces) => faces.map((face) => face.getAttribute("aria-label")).join(" "))).includes(other.name),
      `${who.key} sees ${other.key}'s face on the page`,
      30,
    );
  }
  await shot(owner, "faces");
  pass("owner and a member each see the other's face on the document they share");

  const commented = await coreApi(owner, `${base}/documents/${documentId}/comments`, {
    method: "POST",
    expected: 201,
    body: { content_json: commentBody("Can you check the numbers ", alice) },
  });
  const threadId = commented.thread.thread.id;
  await until(
    async () => (await forYou(alice)).items.some((item) => item.kind === "mention" && item.title === "Multiplayer plan" && item.unread),
    "a comment mention reaches Alice's For you",
    30,
  );
  assert(
    !(await forYou(bob)).items.some((item) => item.kind === "mention" && item.title === "Multiplayer plan"),
    "a comment mention reaches only the person named",
  );
  await coreApi(alice, `${base}/documents/${documentId}/comments/${threadId}/replies`, {
    method: "POST",
    expected: 201,
    body: { content_json: commentBody("Checked, they add up.") },
  });
  await until(
    async () => (await forYou(owner)).items.some((item) => item.kind === "reply" && item.title === "Multiplayer plan" && item.unread),
    "Alice's reply reaches the owner's For you",
    30,
  );
  await coreApi(owner, `${base}/documents/${documentId}/comment-reactions`, {
    method: "POST",
    body: { comment_id: commented.first_comment.id, emoji: "👍", on: true },
  });
  assert(
    JSON.stringify(await coreApi(alice, `${base}/documents/${documentId}/comment-reactions`)).includes("👍"),
    "a reaction on a comment is visible to the other person",
  );
  pass("a comment mention reached only Alice, her reply came back to the owner, and a reaction is shared");

  const viewOnly = await coreApi(owner, `${base}/documents`, {
    method: "POST",
    expected: 201,
    body: { title: "Read me", kind: "brief", visibility: "company", content_json: commentBody("Policy text."), reason: "Multiplayer smoke" },
  });
  const viewDocument = (await coreApi(owner, `${base}/documents/${viewOnly.document_id}`)).document;
  await coreApi(owner, `${base}/documents/${viewOnly.document_id}`, {
    method: "PATCH",
    body: { expected_version: viewDocument.version, general_access: "view" },
  });
  assert.equal((await coreApi(alice, `${base}/documents/${viewOnly.document_id}`)).access, "read", "company view gives a member read access");
  await alice.page.goto(`${core}/${company}/library/documents?document=${viewOnly.document_id}`);
  const readBody = alice.page.getByLabel("Document body", { exact: true });
  await readBody.waitFor({ timeout: 30000 });
  await alice.page.getByText("Policy text.").waitFor({ timeout: 30000 });
  assert.equal(await readBody.getAttribute("contenteditable"), "false", "a view-only document cannot be typed into");
  await readBody.locator("p").first().hover();
  assert.equal(await alice.page.locator(".block-handle").isVisible(), false, "a reader sees no block grip");
  pass("general access 'company can view' gives a member a read-only page");

  await owner.page.goto(documentUrl);
  const ownerBody = owner.page.getByLabel("Collaborative document body");
  await ownerBody.waitFor({ timeout: 30000 });
  // Start a new line after the last paragraph, then type the slash there.
  await ownerBody.locator("p").last().click();
  await owner.page.waitForTimeout(150);
  await owner.page.keyboard.press("End");
  await owner.page.waitForTimeout(150);
  await owner.page.keyboard.press("Enter");
  await owner.page.keyboard.type("/", { delay: 50 });
  await owner.page.getByRole("listbox", { name: "Blocks" }).waitFor({ timeout: 10000 });
  await owner.page.keyboard.press("Escape");
  await until(async () => !(await owner.page.getByRole("listbox", { name: "Blocks" }).isVisible()), "the / menu closes on Escape", 10);
  pass("typing / at the start of a line opens the block menu, Escape closes it");

  // 9c. Alice asks for a sheet she cannot open; the owner hears of it in For you, opens its
  //     sharing from there and lets her view it.
  const sheetId = randomUUID();
  await coreApi(owner, `${base}/sheets`, {
    method: "POST",
    body: { id: sheetId, title: "Budget", visibility: "participants" },
  });
  await alice.page.goto(`${core}/${company}/library/sheets?sheet=${sheetId}`);
  await alice.page.getByRole("button", { name: "Ask for access" }).click();
  await alice.page.getByRole("button", { name: "Asked" }).waitFor({ timeout: 15000 });
  const asks = await coreApi(owner, `${base}/sheets/${sheetId}/access-requests`);
  assert.deepEqual(asks.items.map((ask) => [ask.actor_id, ask.access]), [[alice.principal.actor_id, "read"]]);
  const askItem = (await coreApi(owner, `${base}/for-you?limit=50`)).items.find((item) => item.key.startsWith("sheet-access:"));
  assert.ok(askItem?.href.includes("inspect=share"), "the ask opens the sheet's sharing");
  await owner.page.goto(core + askItem.href);
  await owner.page.getByRole("button", { name: "Let them view" }).click();
  await until(
    async () => (await alice.context.request.fetch(`${core}${base}/sheets/${sheetId}/participants`, { failOnStatusCode: false })).status() === 200,
    "Alice can open the sheet she asked for",
    20,
  );
  pass("a member asked for a private sheet; the owner granted it from For you and she can open it");

  // 9c2. Comments on a sheet. The owner lets Alice comment; she comments on a cell and mentions
  //      the owner, whose open sheet shows it at once and marks the cell. The owner hears of it in
  //      For you, replies from the thread it opens, and resolves it; Alice sees both as they happen.
  await coreApi(owner, `${base}/sheets/${sheetId}/participants/${alice.principal.actor_id}`, {
    method: "PUT",
    body: { access: "comment" },
  });
  await owner.page.goto(`${core}/${company}/library/sheets?sheet=${sheetId}&inspect=comments`);
  await owner.page.getByText("No open comments").waitFor({ timeout: 30000 });
  await alice.page.goto(`${core}/${company}/library/sheets?sheet=${sheetId}&inspect=comments`);
  const aliceComment = alice.page.getByRole("combobox", { name: "Comment on A1" });
  await aliceComment.waitFor({ timeout: 30000 });
  await aliceComment.click();
  await aliceComment.pressSequentially("Is this right, @Mar", { delay: 30 });
  await alice.page.getByRole("option", { name: owner.name }).waitFor({ timeout: 5000 });
  await alice.page.keyboard.press("Enter");
  await aliceComment.pressSequentially("?", { delay: 30 });
  await alice.page.getByRole("button", { name: "Comment", exact: true }).click();
  await alice.page.getByText(`Is this right, @${owner.name} ?`).waitFor({ timeout: 10000 });
  await owner.page.getByText(`Is this right, @${owner.name} ?`).waitFor({ timeout: 8000 });
  await until(async () => (await owner.page.locator(".sheet-comment-marks span").count()) === 1, "the commented cell is marked", 10);
  const sheetMention = (await coreApi(owner, `${base}/for-you?limit=50`)).items.find((item) => item.key.startsWith("sheet-mention:"));
  assert.ok(sheetMention?.href.includes("inspect=comments&thread="), "the mention opens the sheet on its thread");
  await owner.page.goto(core + sheetMention.href);
  const ownerReply = owner.page.getByRole("combobox", { name: "Reply on A1" });
  await ownerReply.waitFor({ timeout: 30000 });
  await ownerReply.fill("Yes, it matches the board pack.");
  await owner.page.getByRole("button", { name: "Reply", exact: true }).click();
  await alice.page.getByText("Yes, it matches the board pack.").waitFor({ timeout: 8000 });
  await shot(owner, "sheet-comments");
  await owner.page.getByRole("button", { name: "Resolve the thread on A1" }).click();
  await alice.page.getByText("No open comments").waitFor({ timeout: 8000 });
  // By keyboard alone: Tab moves between cells, and Escape then Tab leaves the grid for the panel.
  const cell = alice.page.locator(".o-grid-composer .o-composer").first();
  await cell.focus();
  await alice.page.keyboard.press("Tab");
  assert.ok(await alice.page.evaluate(() => !!document.activeElement?.closest(".o-spreadsheet")), "Tab moves between cells");
  await alice.page.keyboard.press("Escape");
  await alice.page.keyboard.press("Tab");
  assert.ok(
    await alice.page.evaluate(() => !document.activeElement?.closest(".o-spreadsheet") && document.activeElement !== document.body),
    "Escape, then Tab, leaves the grid",
  );
  // Losing the connection keeps the page and the draft: coming back online must not reload it.
  let reloaded = false;
  const onLoad = () => (reloaded = true);
  alice.page.on("load", onLoad);
  await alice.context.setOffline(true);
  await alice.page.mouse.click(5, 300); // the first pointer loads the rail's code, which fails offline
  const offlineDraft = alice.page.getByRole("combobox", { name: /^Comment on / });
  await offlineDraft.fill("Drafted while offline");
  await alice.page.getByRole("button", { name: "Comment", exact: true }).click();
  await alice.page.getByText("You're offline").first().waitFor({ timeout: 5000 });
  await alice.context.setOffline(false);
  await pause(4000);
  alice.page.off("load", onLoad);
  assert.equal(reloaded, false, "coming back online does not reload the page");
  assert.equal(await offlineDraft.inputValue(), "Drafted while offline", "the draft survives the lost connection");
  await alice.page.getByRole("button", { name: "Comment", exact: true }).click();
  await owner.page.getByText("Drafted while offline").waitFor({ timeout: 8000 });
  pass("a member commented on a sheet's cell and mentioned the owner; the reply and resolve reached her live; the keyboard leaves the grid; a draft outlives a lost connection");

  // 9d. A page made for the company reaches a member's open Library as it happens. The list polls
  //     every 15 seconds; arriving well inside that is the change stream, which members now hear.
  await alice.page.goto(`${core}/${company}/library`);
  await alice.page.getByRole("heading", { name: "All files" }).first().waitFor({ timeout: 30000 }).catch(() => undefined);
  await pause(2000);
  await coreApi(owner, `${base}/documents`, {
    method: "POST",
    expected: 201,
    body: { title: "Launch checklist", kind: "brief", visibility: "company", content_json: commentBody("Ship it."), reason: "Multiplayer smoke" },
  });
  await alice.page.getByText("Launch checklist").first().waitFor({ timeout: 6000 });
  pass("a page the owner made for the company appeared in a member's open Library within seconds");

  // 9e. A document a member cannot open says so, once, rather than loading forever.
  let documentReads = 0;
  const countReads = (request) => {
    if (/\/api\/companies\/[^/]+\/documents\/[0-9a-f-]{36}$/.test(new URL(request.url()).pathname)) documentReads += 1;
  };
  alice.page.on("request", countReads);
  await alice.page.goto(`${core}/${company}/library/documents?document=${randomUUID()}`);
  await alice.page.getByText("You can’t open this document").waitFor({ timeout: 15000 });
  await pause(5000);
  alice.page.off("request", countReads);
  assert.ok(documentReads <= 2, `a refused document is read ${documentReads} times, not once`);
  pass("a document a member cannot open says so at once, and is not asked for again and again");

  // 9f. A member keeps a page of her own as an owner would: she duplicates a company page, and
  //     renames, archives, restores and deletes the copy; the original is untouched.
  const launch = (await coreApi(alice, `${base}/documents?limit=50`)).items.find(
    (item) => item.document.title === "Launch checklist",
  );
  assert.ok(launch, "the company page is in Alice's list");
  const copy = await coreApi(alice, `${base}/documents/${launch.document.id}/duplicate`, { method: "POST", expected: 201 });
  let mine = (await coreApi(alice, `${base}/documents/${copy.document_id}`)).document;
  assert.equal(mine.owner_actor_id, alice.principal.actor_id, "the copy is Alice's");
  assert.equal(mine.title, "Copy of Launch checklist");
  await coreApi(alice, `${base}/documents/${copy.document_id}`, {
    method: "PATCH",
    body: { expected_version: mine.version, title: "Alice's checklist", kind: mine.kind, visibility: mine.visibility },
  });
  mine = (await coreApi(alice, `${base}/documents/${copy.document_id}`)).document;
  assert.equal(mine.title, "Alice's checklist");
  for (const archived of [true, false, true]) {
    await coreApi(alice, `${base}/documents/${copy.document_id}/archive`, { method: "POST", body: { archived } });
  }
  await coreApi(alice, `${base}/documents/${copy.document_id}`, { method: "DELETE" });
  await coreApi(owner, `${base}/documents/${copy.document_id}`, { expected: 404 });
  assert.equal((await coreApi(owner, `${base}/documents/${launch.document.id}`)).document.title, "Launch checklist");
  pass("a member duplicated a company page, then renamed, archived, restored and deleted her copy");

  // 9g. A member's everyday screens ask only for what a member may read: no request is refused
  //     on the way. An owner-only read that fails for her is a console error, and leaves part of
  //     her screen empty.
  const refused = [];
  // Only requests a screen under test made count: a page left behind (step 9e's missing document)
  // may still answer a retry after the next screen's watcher is attached.
  let screen = null;
  const madeHere = new WeakSet();
  const noteRequest = (request) => {
    if (screen !== null) madeHere.add(request);
  };
  const watchRefusals = (response) => {
    if (!madeHere.has(response.request())) return;
    const url = new URL(response.url());
    if (url.origin === core && response.status() >= 400 && url.pathname.startsWith("/api/"))
      refused.push(`${response.status()} ${response.request().method()} ${url.pathname} (on ${screen || "Inbox"})`);
  };
  alice.page.on("request", noteRequest);
  alice.page.on("response", watchRefusals);
  for (const path of ["", "/library", "/people", "/work", "/company", "/company/charter", "/company/identity", `/library/sheets?sheet=${sheetId}`]) {
    await alice.page.goto("about:blank");
    screen = path;
    await alice.page.goto(`${core}/${company}${path}`);
    await pause(3500);
    if (!path) {
      // The company floor is hers too, read from what a member may see.
      const floor = alice.page.locator(".company-office");
      await floor.waitFor({ timeout: 15000 });
      await until(async () => Number(await floor.getAttribute("data-member-count")) > 0, "the floor shows who is at work", 15);
      await shot(alice, "member-inbox");
    }
  }
  alice.page.off("response", watchRefusals);
  alice.page.off("request", noteRequest);
  assert.deepEqual([...new Set(refused)], [], "a member's screens make no refused requests");
  pass("a member sees the company floor, and her everyday screens make no request she is refused");
  await steadyEntry(alice, "member");
  await steadyEntry(owner, "owner");
  pass("the cockpit appears once, settled, for the owner and a member");

  // 9h. An admin runs the day to day: the owner makes Bob an admin from Members. His screens,
  //     Company settings included, make no request he is refused; a member who tries what an
  //     admin may do is told who may.
  await owner.page.goto(`${core}/${company}/company/members`);
  const bobAccess = owner.page.locator(`select[aria-label="Access for ${people.bob.name}"]`);
  await bobAccess.waitFor({ timeout: 30000 });
  await bobAccess.selectOption("admin");
  await owner.page.getByText(`${people.bob.name} is now an admin.`).waitFor({ timeout: 15000 });
  // The change reaches Core at once: his session from before ends, and he comes back an admin.
  await until(
    async () =>
      (await bob.context.request.fetch(`${core}${base}/principal`, { failOnStatusCode: false })).status() === 401,
    "Bob's session from before the change ends",
    30,
  );
  await enterAccounts(bob);
  await bob.page.locator("#company").waitFor({ state: "visible" });
  await openCompany(bob);
  assert.equal(bob.principal.membership_role, "admin", "Bob comes back an admin");
  const adminRefused = [];
  let adminScreen = "";
  const watchAdmin = (response) => {
    const url = new URL(response.url());
    if (url.origin === core && response.status() >= 400 && url.pathname.startsWith("/api/"))
      adminRefused.push(`${response.status()} ${response.request().method()} ${url.pathname} (on ${adminScreen || "Inbox"})`);
  };
  bob.page.on("response", watchAdmin);
  for (const path of ["", "/library", "/people", "/work", "/company", "/company/members", "/company/identity", "/company/schedules"]) {
    adminScreen = path;
    await bob.page.goto(`${core}/${company}${path}`);
    await pause(3500);
  }
  bob.page.off("response", watchAdmin);
  assert.deepEqual([...new Set(adminRefused)], [], "an admin's screens make no refused requests");
  await steadyEntry(bob, "admin");
  const memberTries = await alice.context.request.fetch(`${core}${base}/company/logo`, {
    method: "DELETE",
    headers: { Origin: core },
    failOnStatusCode: false,
  });
  assert.equal(memberTries.status(), 403);
  assert.match(await memberTries.text(), /owner or an admin/, "a member is told an admin may do it too");
  // 9i. Direction and permission (S80), with Bob an admin. Exec's half runs over the plane's own
  //     socket exactly as `restless direction` and `restless request` send it; each person's half
  //     runs through their own session.
  {
    const alicePerson = alice.principal.actor_id;
    const website = `Website ${randomUUID().slice(0, 6)}`;
    await daemonCommand({ cmd: "actor-create", company, as_actor: "copy-critic", role: "Copywriter", name: "Wren", actor: "exec", reason: "Website copy for the direction smoke" });
    await daemonCommand({ cmd: "team-create", company, name: website, to: "copy-critic", body: "Copy is plain.", actor: "exec" });
    const brief = async () => (await daemonCommand({ cmd: "teams", company })).teams.find((team) => team.name === website)?.brief;
    const teamId = (await daemonCommand({ cmd: "teams", company })).teams.find((team) => team.name === website).id;
    const directionPath = async (requester, cls, team) =>
      (await direction({ operation: "path", requester, class: cls ?? null, team: team ?? null })).path;

    // 1. A member's content rule on an unowned team applies at once; the owner can undo it, and Alice is told.
    assert.equal(await directionPath(alicePerson, "content", website), "act_then_tell");
    await daemonCommand({ cmd: "team-update", company, name: website, body: "Copy speaks in first person as the business.", actor: "exec", reason: "Alice asked for first person" });
    const applied = await direction({
      operation: "applied", requester: alicePerson, class: "content", team: website,
      summary: "Website copy speaks in first person", target: { kind: "team_brief", team: { id: teamId, name: website } },
      undo: { kind: "content", before: { team_id: teamId, brief: "Copy is plain." }, after: { brief: "Copy speaks in first person as the business." } },
    });
    const forYou = await coreApi(owner, `${base}/for-you`);
    const day = forYou.items.find((item) => item.kind === "applied");
    assert(day?.direction_applied?.some((entry) => entry.id === applied.id), "the owner's For you shows the change applied for Alice");
    const undone = await coreApi(owner, `${base}/direction/applied/${applied.id}/undo`, { method: "POST" });
    assert.equal(undone.undo.state, "undone");
    assert.equal(await brief(), "Copy is plain.", "Undo restores the brief");

    // 2. A spend change asks first; Approve reaches Exec, and the request turns Done for Alice.
    assert.equal(await directionPath(alicePerson, null, null), "ask_first");
    const spend = (await direction({ operation: "request_file", requester: alicePerson, class: null, team: null, summary: "Raise the ads budget to $200", change: "Monthly ads envelope from $100 to $200", reason: "Launch week" })).request;
    const inbox = await coreApi(owner, `${base}/attention`);
    assert(inbox.items.some((item) => item.direction_request?.id === spend.id), "the request is in the owner's Inbox");
    const approved = await coreApi(owner, `${base}/direction/requests/${spend.id}/decision`, { method: "POST", body: { decision: "approve" } });
    assert.equal(approved.state, "approved");
    await direction({ operation: "request_done", id: spend.id, target: null, failure: null });
    const mine = await coreApi(alice, `${base}/direction/requests?state=mine`);
    assert.equal(mine.items.find((item) => item.id === spend.id)?.state, "done", "Alice's request turns Done");

    // 3. Approve, and always allow: her next staffing change applies; after Revoke it asks again.
    assert.equal(await directionPath(alicePerson, "staffing", website), "ask_first");
    const staffing = (await direction({ operation: "request_file", requester: alicePerson, class: "staffing", team: website, summary: "Give the copywriter a stronger model", change: "The copywriter on the frontier tier", reason: "" })).request;
    await coreApi(owner, `${base}/direction/requests/${staffing.id}/decision`, { method: "POST", body: { decision: "approve", always_allow: true } });
    assert.equal(await directionPath(alicePerson, "staffing", website), "act_then_tell", "a standing grant lets her next staffing change apply");
    const grant = (await coreApi(owner, `${base}/direction/grants`)).items.find((item) => item.class === "staffing");
    await coreApi(owner, `${base}/direction/grants/${grant.id}/revoke`, { method: "POST", expected: 204 });
    assert.equal(await directionPath(alicePerson, "staffing", website), "ask_first", "after Revoke it asks again");

    // 4. "Make Alice owner" makes her next direction for that team Direct.
    const own = (await direction({ operation: "request_file", requester: alicePerson, class: "schedule", team: website, summary: "Post twice a week", change: "A Tuesday and Friday posting schedule", reason: "" })).request;
    await coreApi(owner, `${base}/direction/requests/${own.id}/decision`, { method: "POST", body: { decision: "approve", make_team_owner: true } });
    assert.equal(await directionPath(alicePerson, "schedule", website), "direct");

    // 5. The owner and an admin approve one request at once: one wins, the other is told who did.
    const race = (await direction({ operation: "request_file", requester: alicePerson, class: null, team: null, summary: "Invite a contractor", change: "Invite sam@multiplayer.test as a member", reason: "" })).request;
    const answers = await Promise.all(
      [owner, bob].map((who) =>
        who.context.request.fetch(`${core}${base}/direction/requests/${race.id}/decision`, {
          method: "POST",
          headers: { Origin: core, "Content-Type": "application/json", "Idempotency-Key": randomUUID() },
          data: JSON.stringify({ decision: "approve" }),
          failOnStatusCode: false,
        }),
      ),
    );
    assert.deepEqual(answers.map((answer) => answer.status()).sort(), [200, 409], "one approval wins");
    const loser = answers.find((answer) => answer.status() === 409);
    assert.match((await loser.json()).message, /already approved/);

    // 6. One change applied and one request open: the digest counts both.
    await direction({
      operation: "applied", requester: bob.principal.actor_id, class: "content", team: null,
      summary: "Charter mentions the launch", target: { kind: "charter" }, undo: { kind: "content", before: {} },
    }).catch(() => null);
    const open = (await direction({ operation: "request_file", requester: alicePerson, class: null, team: null, summary: "Open a second bank account", change: "Business savings account", reason: "" })).request;
    const digest = await coreApi(owner, `${base}/direction/digest`);
    assert.match(digest.summary, /applied .* for others/, "the digest counts what was applied");
    assert(digest.requests_waiting.some((item) => item.id === open.id), "the digest lists the waiting request");

    // 7. A member never sees another's requests and is refused grants and Undo.
    await direction({ operation: "request_file", requester: bob.principal.actor_id, class: null, team: null, summary: "Bob's private ask", change: "Something only Bob asked", reason: "" });
    const aliceOpen = await coreApi(alice, `${base}/direction/requests?state=open`);
    assert(!aliceOpen.items.some((item) => item.requester.actor_id === bob.principal.actor_id), "Alice never sees Bob's requests");
    await coreApi(alice, `${base}/direction/grants`, { expected: 403 });
    await coreApi(alice, `${base}/direction/applied/${applied.id}/undo`, { method: "POST", expected: 403 });
    pass("direction: act then tell with Undo, ask first with Approve and Done, always allow until revoked, make owner, one answer wins, the digest counts, and a member sees only her own");
  }


  // 9j. The work is the surface (S82): Exec works on the real charter. The owner's own direction
  //     saves it; anyone else's becomes a draft only the owner sees, on Company › Charter and as
  //     the Inbox item itself; publishing it is the owner's save. A request for one's own
  //     direction is refused: it is Direct.
  {
    const alicePerson = alice.principal.actor_id;
    let refusedApply = "";
    await direction({ operation: "charter_apply", requester: alicePerson, markdown: "# Alice's charter", source_message: null })
      .catch((error) => { refusedApply = String(error.message); });
    assert.match(refusedApply, /Only .* sets the charter/, "a member's direction cannot set the charter");
    const draftText = `# Multiplayer\n\nWe build games together (${randomUUID().slice(0, 6)}).\n`;
    const proposed = await direction({ operation: "charter_propose", requester: alicePerson, markdown: draftText, reason: "Alice suggested it", source_message: null });
    assert(proposed.proposal_id, "a draft is recorded");
    const ownerCharter = await coreApi(owner, `${base}/company/charter`);
    assert.equal(ownerCharter.proposal?.id, proposed.proposal_id, "the owner sees the draft on the charter");
    assert.equal((await coreApi(alice, `${base}/company/charter`)).proposal, null, "a member never sees an unpublished draft");
    assert.equal((await coreApi(bob, `${base}/company/charter`)).proposal, null, "nor does an admin");
    const ownerInbox = await coreApi(owner, `${base}/attention`);
    assert(ownerInbox.items.some((item) => item.charter_proposal?.id === proposed.proposal_id), "the draft is an item in the owner's Inbox");
    await coreApi(owner, `${base}/company/charter`, {
      method: "POST",
      body: { markdown: draftText, base_revision: ownerCharter.current_revision, proposal_id: proposed.proposal_id },
    });
    const published = await coreApi(owner, `${base}/company/charter`);
    assert.equal(published.proposal, null, "publishing closes the draft");
    assert(published.revisions.some((revision) => revision.markdown === draftText && revision.revision === published.current_revision), "the draft is the charter");
    const after = await coreApi(owner, `${base}/attention`);
    assert(!after.items.some((item) => item.charter_proposal), "the draft leaves the Inbox");
    let selfRequest = "";
    await direction({ operation: "request_file", requester: owner.principal.actor_id, class: null, team: null, summary: "Set the charter", change: "The charter text", reason: "" })
      .catch((error) => { selfRequest = String(error.message); });
    assert.match(selfRequest, /Direct/, "the owner is never asked to approve their own direction");
    pass("the charter: a member's direction becomes a draft only the owner sees and publishes from the Inbox, and the owner's own direction never becomes a request");
  }

  // Taking it back is as immediate: his admin session ends, and he is a member again.
  await owner.page.goto(`${core}/${company}/company/members`);
  await bobAccess.waitFor({ timeout: 30000 });
  await bobAccess.selectOption("member");
  await owner.page.getByText(`${people.bob.name} is now a member.`).waitFor({ timeout: 15000 });
  await until(
    async () =>
      (await bob.context.request.fetch(`${core}${base}/principal`, { failOnStatusCode: false })).status() === 401,
    "Bob's admin session ends when the role is taken back",
    30,
  );
  await enterAccounts(bob);
  await bob.page.locator("#company").waitFor({ state: "visible" });
  await openCompany(bob);
  assert.equal(bob.principal.membership_role, "member", "Bob comes back a member");
  pass("a role change reaches Core at once both ways; as an admin Bob's screens make no refused request, and a member is told who may act");

  // 9k. The owner's accounts (S98): a website login added once at the owner level, given to the
  //     company, read by its people without its secret; its own browser starts for the owner's
  //     sign-in and goes when the account is removed. Members cannot add or read accounts.
  // A company session cannot manage the owner's accounts and is told why; the owner opens
  // "Logins and keys" on the account site, which enters account-wide and lands on Accounts.
  const scoped = await owner.context.request.fetch(`${core}/api/accounts`, { failOnStatusCode: false });
  assert.equal(scoped.status(), 403, "a company session does not manage the owner's accounts");
  assert.match(await scoped.text(), /account owner/);
  await enterAccounts(owner);
  await owner.page.locator("#enter-account").click();
  await owner.page.waitForURL((url) => url.origin === core && url.pathname === "/account/connections", { timeout: 30000 });
  await owner.page.getByRole("heading", { name: "Accounts" }).first().waitFor({ timeout: 30000 });
  await shot(owner, "accounts");
  {
    const login = await coreApi(owner, "/api/accounts", {
      method: "POST",
      expected: 201,
      body: {
        kind: "browser_profile",
        label: "Smoke shop",
        // A browser sign-in holds no secret, so it needs no vault on this test plane; a website
        // login goes through the same grant, browser and removal paths with its password vaulted.
        credential: { kind: "browser_profile", url: "https://example.com/" },
      },
    });
    const refused = async (path, method = "GET") => {
      const response = await alice.context.request.fetch(core + path, {
        method,
        headers: { Origin: core, "Content-Type": "application/json" },
        data: method === "GET" ? undefined : JSON.stringify({ company }),
        failOnStatusCode: false,
      });
      assert([401, 403, 404].includes(response.status()), `${method} ${path} as a member answered ${response.status()}`);
    };
    await refused("/api/accounts");
    await refused(`/api/accounts/${login.id}/grants`, "POST");
    await coreApi(owner, `/api/accounts/${login.id}/grants`, { method: "POST", expected: 201, body: { company } });
    const seen = await coreApi(alice, `${base}/accounts`);
    assert.deepEqual(seen.grants.map((grant) => grant.account.label), ["Smoke shop"], "a member sees the account the company may use");

    const signIn = await coreApi(owner, `/api/accounts/${login.id}/sign-in`, { method: "POST" });
    assert.match(signIn.desktop_url, /^\/api\/accounts\/[^/]+\/desktop\/vnc\.html/);
    const desktop = await owner.context.request.get(core + signIn.desktop_url, { failOnStatusCode: false });
    assert.equal(desktop.status(), 200, "the account's own browser desktop answers the owner");
    await refused(signIn.desktop_url);
    await coreApi(owner, `/api/accounts/${login.id}/sign-in/${signIn.session}`, { method: "DELETE", expected: 204 });
    await coreApi(owner, `/api/accounts/${login.id}`, { method: "DELETE", expected: 204 });
    assert.deepEqual((await coreApi(alice, `${base}/accounts`)).grants, [], "a removed account leaves the company");
    pass("the owner enters Accounts from the account site; a browser sign-in is given to the company, its browser opens for the owner alone, and removing it removes it everywhere");

    // 9m. Computers (S99): a code and one line, through the plane's real routes. The machine side
    // here is a stand-in speaking the helper's protocol; the helper itself is proven against a
    // Docker host in the engine's machines e2e.
    const pairing = await coreApi(owner, "/api/machines/pairings", { method: "POST", body: { label: "Smoke PC" }, expected: 201 });
    assert.match(pairing.code, /^[2-9A-Z]{4}-[2-9A-Z]{4}$/);
    assert.ok(pairing.commands.unix.includes(`${core}/connect`), pairing.commands.unix);
    const script = await fetch(`${core}/connect`);
    assert.equal(script.status, 200);
    assert.ok((await script.text()).includes(`PLANE="${core}"`), "the install script names this plane");
    const facts = { hostname: "smoke-pc", os: "Ubuntu 24.04", arch: "x86_64", cpus: 4, memory_bytes: 8 * 1024 ** 3, docker_version: "27.0", helper_version: "smoke" };
    const paired = await fetch(`${core}/machines/pair`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ code: pairing.code, facts }) });
    assert.equal(paired.status, 200, await paired.clone().text());
    const { machine_id: machineId, credential } = await paired.json();
    const again = await fetch(`${core}/machines/pair`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ code: pairing.code, facts }) });
    assert.equal(again.status, 403, "a code works once");
    const WebSocket = createRequire(join(repo, "web/package.json"))("ws");
    const strangerLink = new WebSocket(`${core.replace(/^http/, "ws")}/machines/link`, { headers: { Authorization: "Bearer not-a-credential-not-a-credential-0000" } });
    await new Promise((resolve) => strangerLink.once("unexpected-response", (_, response) => { assert.equal(response.statusCode, 401); resolve(); }));
    const link = new WebSocket(`${core.replace(/^http/, "ws")}/machines/link`, { headers: { Authorization: `Bearer ${credential}` } });
    const settings = await new Promise((resolve, reject) => {
      link.once("message", (data, binary) => (binary ? reject(new Error("binary first")) : resolve(JSON.parse(String(data)))));
      link.once("error", reject);
    });
    assert.equal(settings.type, "settings");
    assert.equal(settings.machine_id, machineId);
    link.send(JSON.stringify({ type: "heartbeat", facts }));
    await until(async () => {
      const { machines } = await coreApi(owner, "/api/machines");
      return machines.some((machine) => machine.id === machineId && machine.online && machine.facts.os === facts.os);
    }, "the paired computer shows as connected");
    await coreApi(alice, "/api/machines", { expected: 403 });
    await coreApi(alice, "/api/machines/pairings", { method: "POST", body: {}, expected: 403 });
    const closed = new Promise((resolve) => link.once("close", resolve));
    await coreApi(owner, `/api/machines/${machineId}`, { method: "DELETE", expected: 204 });
    await closed;
    const afterRemoval = await fetch(`${core}/machines/pair`, { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ code: pairing.code, facts }) });
    assert.equal(afterRemoval.status, 403);
    const { machines: left } = await coreApi(owner, "/api/machines");
    assert.ok(!left.some((machine) => machine.id === machineId), "a removed computer is gone");
    pass("the owner added a computer with a code and one line; it linked, showed as connected, members were refused, and removing it closed the link");

    // Back into the company, as the owner would from the account site.
    await enterAccounts(owner);
    await owner.page.locator("#company").waitFor({ state: "visible" });
    await openCompany(owner);
  }

  // 10. Removal from Members closes Bob's live document connection and session.
  await bob.page.goto(documentUrl);
  await bob.page.getByLabel("Collaborative document body").waitFor({ timeout: 30000 });
  await until(() => bob.sockets.some((item) => item.url.includes("/collaboration") && !item.closed), "Bob's live document connection", 30);
  // The connections open now are the ones removal must close; a retry the page makes on its way
  // to the account site is not Bob's access.
  const openAtRemoval = bob.sockets.filter((item) => item.url.includes("/collaboration") && !item.closed);
  await owner.page.goto(`${core}/${company}/company/members`);
  await owner.page.locator(`summary[aria-label="Access for ${people.bob.name}"]`).click();
  await owner.page.getByRole("button", { name: "Remove…" }).click();
  await owner.page.getByRole("button", { name: "Remove access" }).click();
  await owner.page.getByText(`${people.bob.name} no longer has access.`).waitFor();
  // Closed by Core, or gone with the page when it left for the account site (the browser reports no
  // close for a socket torn down with its page).
  await until(
    () => openAtRemoval.every((item) => item.closed) || !bob.page.url().startsWith(core),
    "removal closes Bob's document connection",
    30,
  );
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
