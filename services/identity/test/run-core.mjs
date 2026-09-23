import assert from "node:assert/strict";
import { randomBytes } from "node:crypto";
import { constants } from "node:fs";
import { access, lstat, mkdir, mkdtemp, open, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import pg from "pg";

const serviceDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repo = resolve(serviceDir, "..", "..");
const required = [
  "RESTLESS_TEST_DATABASE_URL_FILE",
  "RESTLESS_TEST_DAEMON",
  "RESTLESS_COMPANY_IMAGE",
  "RESTLESS_NATIVE_DOCUMENTS_IMAGE",
];

function fail(message) {
  throw Error(`self-hosted identity Core runner: ${message}`);
}
function quoteIdentifier(value) {
  return `"${value.replaceAll('"', '""')}"`;
}
function shell(command, args, options = {}) {
  return new Promise((resolveCommand, reject) => {
    const child = spawn(command, args, { stdio: ["ignore", "pipe", "pipe"], ...options });
    const chunks = [];
    child.stdout.on("data", (chunk) => chunks.push(chunk));
    child.stderr.on("data", (chunk) => chunks.push(chunk));
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      const output = Buffer.concat(chunks).toString("utf8").trim();
      if (code === 0) resolveCommand(output);
      else reject(Error(`${command} ${args[0] ?? ""} failed (${signal ?? code}): ${output}`));
    });
  });
}
async function privateText(path) {
  if (!isAbsolute(path)) fail("RESTLESS_TEST_DATABASE_URL_FILE must be absolute");
  const file = await open(path, constants.O_RDONLY | constants.O_NOFOLLOW);
  try {
    const stat = await file.stat();
    if (!stat.isFile() || stat.nlink !== 1 || stat.mode & 0o077 || stat.size > 32768)
      fail("RESTLESS_TEST_DATABASE_URL_FILE must be one private regular file (mode 0600)");
    return (await file.readFile("utf8")).trim();
  } finally {
    await file.close();
  }
}
async function freePort() {
  return await new Promise((resolvePort, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close((error) => (error ? reject(error) : resolvePort(port)));
    });
  });
}
async function freeOffset() {
  for (let offset = 13000; offset <= 15900; offset += 100) {
    const servers = [];
    try {
      for (const base of [...Array(10).keys()].map((n) => 7788 + n).concat(7833)) {
        const server = createServer();
        await new Promise((resolveListen, reject) => {
          server.once("error", reject);
          server.listen(base + offset, "127.0.0.1", resolveListen);
        });
        servers.push(server);
      }
      return offset;
    } catch {
      // A conflicting listener makes this offset unusable; try the next block.
    } finally {
      await Promise.all(servers.map((server) => new Promise((resolveClose) => server.close(resolveClose))));
    }
  }
  fail("no free Core port block in offsets 13000..15900");
}
function replaceDatabase(url, database) {
  const parsed = new URL(url);
  parsed.pathname = `/${database}`;
  return parsed.toString();
}
async function stopProcessGroup(child) {
  if (!child || !Number.isInteger(child.pid)) return;
  try { process.kill(-child.pid, "SIGTERM"); } catch (error) { if (error.code !== "ESRCH") throw error; }
  if (child.exitCode === null && child.signalCode === null) {
    await waitForExit(child, 15000);
  }
  // The journey's daemon shares its detached process group. Kill only that
  // group in case the Node parent exited before its own finally block ran.
  try { process.kill(-child.pid, "SIGKILL"); } catch (error) { if (error.code !== "ESRCH") throw error; }
}
function waitForExit(child, timeout) {
  if (child.exitCode !== null || child.signalCode !== null) return Promise.resolve(true);
  return new Promise((resolveExit, reject) => {
    let settled = false;
    const finish = (value, error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      child.removeListener("exit", onExit);
      child.removeListener("error", onError);
      if (error) reject(error); else resolveExit(value);
    };
    const onExit = () => finish(true);
    const onError = (error) => finish(undefined, error);
    const timer = setTimeout(() => finish(false), timeout);
    child.once("exit", onExit);
    child.once("error", onError);
  });
}
function waitForJourney(child) {
  return new Promise((resolveExit, reject) => {
    let settled = false;
    const finish = (value, error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      child.removeListener("exit", onExit);
      child.removeListener("error", onError);
      if (error) reject(error); else resolveExit(value);
    };
    const onExit = (code, signal) => finish({ code, signal });
    const onError = (error) => finish(undefined, error);
    const timer = setTimeout(() => finish({ timeout: true }), 240000);
    child.once("exit", onExit);
    child.once("error", onError);
  });
}
async function namedContainerIds(name) {
  const ids = (await shell("docker", ["ps", "-aq", "--filter", `name=${name}`]))
    .split(/\s+/).filter(Boolean);
  const exact = [];
  for (const id of ids) if ((await shell("docker", ["inspect", "--format", "{{.Name}}", id])) === `/${name}`) exact.push(id);
  return exact;
}
async function removeNamedContainer(name) {
  for (const id of await namedContainerIds(name)) await shell("docker", ["rm", "-f", id]);
}
async function namedVolumes(name) {
  return (await shell("docker", ["volume", "ls", "-q", "--filter", `name=^${name}$`]))
    .split(/\s+/).filter(Boolean).filter((found) => found === name);
}
async function removeNamedVolume(name) {
  for (const found of await namedVolumes(name)) await shell("docker", ["volume", "rm", found]);
}

for (const name of required) if (!process.env[name]) fail(`missing ${name}`);
const adminFile = process.env.RESTLESS_TEST_DATABASE_URL_FILE;
const daemon = process.env.RESTLESS_TEST_DAEMON;
if (!isAbsolute(daemon)) fail("RESTLESS_TEST_DAEMON must be absolute");
const daemonStat = await lstat(daemon).catch(() => null);
if (!daemonStat?.isFile()) fail("RESTLESS_TEST_DAEMON must name a regular file");
await access(daemon, constants.X_OK).catch(() => fail("RESTLESS_TEST_DAEMON must be executable"));
if (!/^\S+@sha256:[a-f0-9]{64}$/.test(process.env.RESTLESS_COMPANY_IMAGE))
  fail("RESTLESS_COMPANY_IMAGE must be a pinned repository@sha256 digest");
await access(join(serviceDir, "test", "core-journey.mjs"));
const adminUrl = await privateText(adminFile);
try { new URL(adminUrl); } catch { fail("RESTLESS_TEST_DATABASE_URL_FILE does not contain a URL"); }
await shell("docker", ["image", "inspect", "--format", "{{.Id}}", process.env.RESTLESS_COMPANY_IMAGE, process.env.RESTLESS_NATIVE_DOCUMENTS_IMAGE]);

const token = randomBytes(6).toString("hex");
const namespace = `id_${token}_test`;
const company = `identity_${token}_test`;
const databasePrefix = `restless_${namespace.replaceAll("-", "_")}_`;
const planeDatabase = `${databasePrefix}plane_test`;
const accountsDatabase = `${databasePrefix}accounts_test`;
const offset = await freeOffset();
const authPort = await freePort();
const explicitOutput = process.env.RESTLESS_IDENTITY_TEST_OUTPUT;
if (explicitOutput && !isAbsolute(explicitOutput)) fail("RESTLESS_IDENTITY_TEST_OUTPUT must be absolute");
if (explicitOutput) {
  try { await mkdir(explicitOutput, { mode: 0o700 }); }
  catch (error) { if (error.code === "EEXIST") fail("RESTLESS_IDENTITY_TEST_OUTPUT must be a new directory"); throw error; }
}
const output = explicitOutput ?? await mkdtemp(join(tmpdir(), "restless-identity-core-evidence-"));
const state = await mkdtemp(join(tmpdir(), "restless-identity-core-state-"));
const journeyLog = join(output, "journey.log");
const coreLog = join(output, "core.log");
const result = join(output, "identity-service-result.json");
const stateCompany = join(state, "companies", `${company}.toml`);
let admin;
let journey;

async function query(sql, values) { return admin.query(sql, values); }
async function cleanup() {
  const errors = [];
  const attempt = async (action) => { try { await action(); } catch (error) { errors.push(error); } };
  await attempt(() => stopProcessGroup(journey));
  await attempt(async () => {
    const ids = (await shell("docker", ["ps", "-aq", "--filter", `label=com.restless.local-documents-root=${state}`])).split(/\s+/).filter(Boolean);
    for (const id of ids) await shell("docker", ["rm", "-f", id]);
  });
  await attempt(() => removeNamedContainer(`restless-${namespace}-co-${company}`));
  await attempt(() => removeNamedVolume(`restless-${namespace}-vol-${company}`));
  await attempt(async () => {
    const docs = (await shell("docker", ["ps", "-aq", "--filter", `label=com.restless.local-documents-root=${state}`])).split(/\s+/).filter(Boolean);
    assert.equal(docs.length, 0, "namespace Documents containers remain after cleanup");
    assert.equal((await namedContainerIds(`restless-${namespace}-co-${company}`)).length, 0, "namespace company container remains after cleanup");
    assert.equal((await namedVolumes(`restless-${namespace}-vol-${company}`)).length, 0, "namespace company volume remains after cleanup");
  });
  if (admin) {
    await attempt(async () => {
      const named = await query("SELECT datname FROM pg_database WHERE starts_with(datname, $1)", [databasePrefix]);
      for (const { datname } of named.rows) await query(`DROP DATABASE IF EXISTS ${quoteIdentifier(datname)} WITH (FORCE)`);
    });
    await attempt(async () => {
      const roles = await query("SELECT rolname FROM pg_roles WHERE starts_with(rolname, $1)", [databasePrefix]);
      for (const { rolname } of roles.rows) await query(`DROP ROLE IF EXISTS ${quoteIdentifier(rolname)}`);
    });
    await attempt(async () => {
      const databases = await query("SELECT datname FROM pg_database WHERE starts_with(datname, $1)", [databasePrefix]);
      const roles = await query("SELECT rolname FROM pg_roles WHERE starts_with(rolname, $1)", [databasePrefix]);
      assert.equal(databases.rows.length, 0, "namespace databases remain after cleanup");
      assert.equal(roles.rows.length, 0, "namespace roles remain after cleanup");
    });
    await admin.end().catch((error) => errors.push(error));
    admin = undefined;
  }
  await rm(state, { recursive: true, force: true }).catch((error) => errors.push(error));
  if (errors.length) throw new AggregateError(errors, "isolated Core cleanup failed");
  console.log("PASS isolated identity Core containers, volume, databases and roles removed");
}

try {
  await mkdir(dirname(stateCompany), { recursive: true, mode: 0o700 });
  await writeFile(stateCompany, `name = "${company}"\nmodel = "unconfigured/pending"\nspend_ceiling_usd = 0\n`, { mode: 0o600 });
  await writeFile(join(state, "orgintel.toml"), `database_url = ${JSON.stringify(replaceDatabase(adminUrl, planeDatabase))}\n`, { mode: 0o600 });
  admin = new pg.Client({ connectionString: adminUrl, connectionTimeoutMillis: 5000, statement_timeout: 10000 });
  await admin.connect();
  await query(`CREATE DATABASE ${quoteIdentifier(planeDatabase)}`);
  await query(`CREATE DATABASE ${quoteIdentifier(accountsDatabase)}`);

  const environment = {};
  for (const name of ["PATH", "HOME", "USER", "LANG", "LC_ALL", "TMPDIR"])
    if (process.env[name]) environment[name] = process.env[name];
  if (process.env.RESTLESS_COCKPIT_DIR) environment.RESTLESS_COCKPIT_DIR = process.env.RESTLESS_COCKPIT_DIR;
  if (process.env.RESTLESS_BROWSER_EXECUTABLE) environment.RESTLESS_BROWSER_EXECUTABLE = process.env.RESTLESS_BROWSER_EXECUTABLE;
  Object.assign(environment, {
    RESTLESS_PROFILE: "dev",
    RESTLESS_HOME: state,
    RESTLESS_AUTH_TEST_PORT: String(authPort),
    RESTLESS_PORT_OFFSET: String(offset),
    RESTLESS_TEST_COMPANY: company,
    RESTLESS_TEST_REPO: repo,
    RESTLESS_TEST_DAEMON: daemon,
    RESTLESS_TEST_DAEMON_LOG: coreLog,
    RESTLESS_TEST_RESULT: result,
    RESTLESS_AUTH_TEST_DATABASE: replaceDatabase(adminUrl, accountsDatabase),
    RESTLESS_RESOURCE_NAMESPACE: namespace,
    RESTLESS_COMPANY_IMAGE: process.env.RESTLESS_COMPANY_IMAGE,
    RESTLESS_NATIVE_DOCUMENTS_IMAGE: process.env.RESTLESS_NATIVE_DOCUMENTS_IMAGE,
  });
  const log = await open(journeyLog, "w", 0o600);
  try {
    journey = spawn(process.execPath, [join(serviceDir, "test", "core-journey.mjs")], {
      cwd: serviceDir, env: environment, detached: true, stdio: ["ignore", log.fd, log.fd],
    });
  } finally {
    await log.close();
  }
  const exit = await waitForJourney(journey);
  if (exit.timeout) fail(`journey exceeded 240 seconds; evidence: ${output}`);
  if (exit.code !== 0) fail(`journey failed (${exit.signal ?? exit.code}); evidence: ${output}`);
  await access(result);
  console.log(`PASS isolated self-hosted identity Core journey; evidence retained at ${output}`);
} catch (error) {
  await writeFile(join(output, "runner-error.txt"), `${error.stack ?? error}\n`, { mode: 0o600 }).catch(() => {});
  console.error(error.stack ?? error);
  process.exitCode = 1;
} finally {
  try { await cleanup(); }
  catch (error) { console.error(error.stack ?? error); process.exitCode = 1; }
}
