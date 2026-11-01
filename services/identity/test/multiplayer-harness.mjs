// Shared harness for the multiplayer journeys (started by run-core.mjs).
//
// It owns the processes a journey starts: a loopback SMTP catcher, the production account
// entrypoint (src/main.mjs) and Core in network entry, one hostname on two ports as a
// Tailscale-shared host uses. Every person is a separate browser context with their own
// verified account. run-core.mjs owns the isolated state, databases and containers.
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


export const {
  RESTLESS_HOME: root,
  RESTLESS_AUTH_TEST_PORT: authPort,
  RESTLESS_PORT_OFFSET: offset,
  RESTLESS_TEST_COMPANY: company,
  RESTLESS_TEST_REPO: repo,
} = process.env;
assert(company.endsWith("_test"), "the smoke only ever creates _test companies");
export const serviceDir = join(dirname(fileURLToPath(import.meta.url)), "..");
export const evidence = dirname(process.env.RESTLESS_TEST_RESULT);
export const host = "plane.localhost";
export const corePort = Number(offset) + 7788;
export const core = `http://${host}:${corePort}`;
export const accounts = `http://${host}:${authPort}`;
export const base = `/api/companies/${company}`;
export const password = "RestlessMultiplayer!123";
export const people = {
  owner: { name: "Mara Owner", email: "owner@multiplayer.test" },
  alice: { name: "Alice Member", email: "alice@multiplayer.test" },
  bob: { name: "Bob Member", email: "bob@multiplayer.test" },
  carol: { name: "Carol Member", email: "carol@multiplayer.test" },
};
export const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
export const steps = [];
// Defects the smoke observes but does not fail on; each names what a person sees.
export const notes = [];
export function note(text) {
  notes.push(text);
  console.log(`NOTE ${text}`);
}
export function pass(step) {
  steps.push(step);
  console.log(`PASS ${step}`);
}
export async function until(check, label, seconds = 60) {
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
export function decodeQuotedPrintable(text) {
  return text
    .replace(/=\r?\n/g, "")
    .replace(/=([0-9A-F]{2})/g, (_, hex) => String.fromCharCode(parseInt(hex, 16)));
}
export function parseMail(raw) {
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
export async function smtpCatcher() {
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
export const logFd = openSync(process.env.RESTLESS_TEST_DAEMON_LOG, "w");
export const identityLogFd = openSync(join(evidence, "accounts.log"), "w");
export let smtp, daemon, identity, browser, cellPool;
export const contexts = [];
/** Core's network-entry environment, kept so a journey can restart Core as a plane update does. */
export let coreEntry;

export function startDaemon(network, entry = {}) {
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
export async function stopChild(child) {
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
export async function health() {
  const response = await fetch(`${core}/health`).catch(() => null);
  return response?.ok ?? false;
}

// ---------------------------------------------------------------- browser
export let lastAuthAction = 0;
// Better Auth limits sign-up and sign-in to three per ten seconds per address;
// every person here shares loopback, so space those actions out.
export async function authGate() {
  const wait = lastAuthAction + 3600 - Date.now();
  if (wait > 0) await pause(wait);
  lastAuthAction = Date.now();
}
export async function person(key) {
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  // A failed script or API load explains a page stuck "Opening…"; kept for the failure report.
  const failedRequests = [];
  page.on("requestfailed", (request) => {
    const url = new URL(request.url());
    if (url.hostname === "plane.localhost") failedRequests.push(`${request.method()} ${url.pathname} ${request.failure()?.errorText ?? ""}`);
  });
  const sockets = [];
  page.on("websocket", (socket) => sockets.push({ url: socket.url(), socket, closed: false }));
  page.on("websocket", (socket) =>
    socket.on("close", () => {
      const entry = sockets.find((item) => item.socket === socket);
      if (entry) entry.closed = true;
    }),
  );
  const who = { key, ...people[key], context, page, errors, failedRequests, sockets };
  contexts.push(who);
  return who;
}
export async function shot(who, name) {
  await who.page.screenshot({ path: join(evidence, `${who.key}-${name}.png`), fullPage: false }).catch(() => {});
}
export async function coreApi(who, path, { method = "GET", body, expected = 200 } = {}) {
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
export async function daemonCommand(request) {
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
export const direction = (operation) => daemonCommand({ cmd: "direction-operation", company, direction_operation: operation });

/* After a role change ends a session, that person's open cockpit tab re-enters through the
 * account by itself (session-reentry.ts). The test's own visit to the account can land in the
 * middle of that navigation and be cancelled by it; it is retried, and every attempt is logged so
 * a page that is truly stuck still fails here. */
export async function enterAccounts(who) {
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
export async function steadyEntry(who, role) {
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
export async function createAccount(who, start = "/") {
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
export async function openCompany(who) {
  await who.page.locator("#enter-company").click();
  await who.page.waitForURL((url) => url.origin === core, { timeout: 30000 });
  who.principal = await coreApi(who, `${base}/principal`);
}


/** Stop Core and start it again on the same state and entry, as a plane update does. Open
 * pages keep their tabs; whatever they show must come back by itself. */
export async function restartCore() {
  await stopChild(daemon);
  startDaemon(true, coreEntry);
  await until(async () => {
    if (daemon.exitCode !== null) throw Error("Core exited on restart; see core.log");
    return health();
  }, "Core back after a restart", 120);
}

/** Start the account service and Core, and sign the owner up into the _test company. */
export async function boot() {
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
  coreEntry = entry;
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
  const owner = await person("owner");

  // 4. Owner signs up, claims the company and opens it.
  await createAccount(owner);
  await owner.page.locator("#bootstrap-company").click();
  await owner.page.locator("#enter-company").waitFor({ state: "visible" });
  await openCompany(owner);
  assert.equal(owner.principal.actor_id, "owner", "the first network owner keeps the local owner Actor");
  await shot(owner, "entered");
  pass("owner verified by email, claimed the company and entered it");
  return owner;
}

/** The owner invites each person from Company › Members; each accepts by the emailed link,
 * creates and verifies an account and enters as a member. */
export async function invite(owner, members) {
  await owner.page.goto(`${core}/${company}/company/members`);
  for (const who of members) {
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
  pass(`owner invited ${members.length} colleagues from Members; invitations reached the SMTP catcher`);
  // 6. Each colleague follows the emailed link, creates and verifies an account,
  //    accepts and opens the company.
  for (const who of members) {
    const invite = new URL(who.invitation.link);
    await createAccount(who, invite.pathname + invite.search);
    await who.page.locator("#accept-invitation").click();
    await who.page.getByText("You’ve joined the company.").waitFor();
    await openCompany(who);
    assert.equal(who.principal.membership_role, "member");
    await shot(who, "entered");
  }
  const actors = new Set([owner, ...members].map((who) => who.principal.actor_id));
  assert.equal(actors.size, members.length + 1, "every person is a distinct Actor");
  pass(`${members.length} colleagues accepted by email link and entered as distinct member Actors`);
}

/** Run one journey: its steps, then every page free of errors, then the result file.
 * On failure, what each person was looking at is kept. Processes always stop. */
export async function run(journey, body) {
  try {
    await body();
    for (const who of contexts) assert.deepEqual(who.errors, [], `${who.key} page errors`);
    await writeFile(
      process.env.RESTLESS_TEST_RESULT,
      JSON.stringify({ journey, people: contexts.length, steps, notes }, null, 2),
    );
  } catch (error) {
    for (const who of contexts) {
      await shot(who, "failure");
      console.error(`${who.key} at ${who.page.url()}; page errors: ${JSON.stringify(who.errors)}`);
      if (who.failedRequests.length) console.error(`${who.key} failed requests: ${JSON.stringify(who.failedRequests.slice(-20))}`);
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
    console.log(`PASS ${journey} journey processes stopped`);
  }
}
