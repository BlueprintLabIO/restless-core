import assert from "node:assert/strict";
import { constants } from "node:fs";
import { chmod, mkdtemp, open, rm, writeFile } from "node:fs/promises";
import { generateKeyPairSync, randomBytes, randomUUID } from "node:crypto";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { isAbsolute, join } from "node:path";
import { spawn } from "node:child_process";
import pg from "pg";

const DATABASE_FILE = process.env.RESTLESS_TEST_DATABASE_URL_FILE;
const OWNER_EMAIL = "smtp-entrypoint-owner@restless.test";
const PASSWORD = "RestlessSmtpEntrypoint!123";
const TIMEOUT_MS = 15_000;
const MAX_SMTP_MESSAGE_BYTES = 64 * 1024;

const delay = (milliseconds) =>
  new Promise((resolve) => setTimeout(resolve, milliseconds));

async function privateDatabaseUrl(path) {
  if (!path || !isAbsolute(path))
    throw Error(
      "RESTLESS_TEST_DATABASE_URL_FILE must name an absolute private file",
    );
  const file = await open(path, constants.O_RDONLY | constants.O_NOFOLLOW);
  try {
    const stat = await file.stat();
    if (
      !stat.isFile() ||
      stat.nlink !== 1 ||
      stat.mode & 0o077 ||
      stat.size === 0 ||
      stat.size > 32_768
    )
      throw Error(
        "RESTLESS_TEST_DATABASE_URL_FILE must be one private regular file",
      );
    const raw = (await file.readFile("utf8")).trim();
    const url = new URL(raw);
    if (
      !["postgres:", "postgresql:"].includes(url.protocol) ||
      !url.hostname ||
      !url.username ||
      !url.password ||
      url.pathname === "/"
    )
      throw Error(
        "RESTLESS_TEST_DATABASE_URL_FILE must contain a password-authenticated PostgreSQL admin URL",
      );
    return url;
  } finally {
    await file.close();
  }
}

function quotedIdentifier(value) {
  assert.match(value, /^[a-z0-9_]+$/);
  return `"${value}"`;
}

function timeout(promise, label, milliseconds = TIMEOUT_MS) {
  let timer;
  return Promise.race([
    promise,
    new Promise((_, reject) => {
      timer = setTimeout(() => reject(Error(`${label} timed out`)), milliseconds);
    }),
  ]).finally(() => clearTimeout(timer));
}

async function unusedLoopbackPort() {
  const server = createServer();
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  assert(address && typeof address === "object");
  await new Promise((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
  return address.port;
}

function smtpCapture() {
  const sockets = new Set();
  let settleMessage;
  let rejectMessage;
  const message = new Promise((resolve, reject) => {
    settleMessage = resolve;
    rejectMessage = reject;
  });
  message.catch(() => {});
  const server = createServer((socket) => {
    sockets.add(socket);
    socket.setEncoding("utf8");
    socket.setTimeout(10_000, () => socket.destroy(Error("SMTP socket timed out")));
    socket.once("close", () => sockets.delete(socket));
    socket.once("error", (error) => rejectMessage(error));
    socket.write("220 restless.test SMTP capture ready\r\n");
    let buffer = "";
    let dataMode = false;
    let data = "";
    socket.on("data", (chunk) => {
      buffer += chunk;
      if (Buffer.byteLength(buffer) > MAX_SMTP_MESSAGE_BYTES)
        return socket.destroy(Error("SMTP command buffer exceeded its bound"));
      while (buffer.includes("\r\n")) {
        const boundary = buffer.indexOf("\r\n");
        let line = buffer.slice(0, boundary);
        buffer = buffer.slice(boundary + 2);
        if (dataMode) {
          if (line === ".") {
            dataMode = false;
            settleMessage(data);
            socket.write("250 2.0.0 captured\r\n");
            continue;
          }
          if (line.startsWith("..")) line = line.slice(1);
          data += line + "\r\n";
          if (Buffer.byteLength(data) > MAX_SMTP_MESSAGE_BYTES)
            return socket.destroy(Error("SMTP message exceeded its bound"));
          continue;
        }
        const command = line.split(/\s+/, 1)[0].toUpperCase();
        if (command === "EHLO" || command === "HELO")
          socket.write("250-restless.test\r\n250 SIZE 65536\r\n");
        else if (command === "MAIL" || command === "RCPT" || command === "RSET")
          socket.write("250 2.1.0 accepted\r\n");
        else if (command === "DATA") {
          dataMode = true;
          data = "";
          socket.write("354 End data with <CR><LF>.<CR><LF>\r\n");
        } else if (command === "QUIT") {
          socket.end("221 2.0.0 closing\r\n");
        } else {
          socket.write("500 5.5.2 unsupported command\r\n");
        }
      }
    });
  });
  return {
    server,
    message,
    async listen() {
      await new Promise((resolve, reject) => {
        server.once("error", reject);
        server.listen(0, "127.0.0.1", resolve);
      });
      const address = server.address();
      assert(address && typeof address === "object");
      return address.port;
    },
    async close() {
      for (const socket of sockets) socket.destroy();
      if (!server.listening) return;
      await new Promise((resolve) => server.close(resolve));
    },
  };
}

function verificationLink(message) {
  const separator = message.search(/\r?\n\r?\n/);
  assert.notEqual(separator, -1, "captured email has no body");
  const headers = message.slice(0, separator).toLowerCase();
  let body = message.slice(separator).replace(/^\r?\n\r?\n/, "");
  if (headers.includes("content-transfer-encoding: quoted-printable")) {
    body = body
      .replace(/=\r?\n/g, "")
      .replace(/=([0-9a-f]{2})/gi, (_, hex) =>
        String.fromCharCode(Number.parseInt(hex, 16)),
      );
  }
  const match = body.match(/https?:\/\/[^\s<>"']+/);
  assert(match, "verification email did not contain a link");
  const link = new URL(match[0]);
  assert(
    link.hostname === "127.0.0.1",
    "verification link did not target the isolated account service",
  );
  return link;
}

async function stopChild(child) {
  if (!child || child.exitCode !== null || child.signalCode !== null) return;
  const exited = new Promise((resolve) => child.once("exit", resolve));
  child.kill("SIGTERM");
  try {
    await timeout(exited, "identity service shutdown", 5_000);
  } catch {
    child.kill("SIGKILL");
    await timeout(exited, "identity service forced shutdown", 5_000);
  }
}

let admin;
let databaseName;
let databaseCreated = false;
let tempState;
let identityChild;
let smtp;
let failure;
try {
  const adminUrl = await privateDatabaseUrl(DATABASE_FILE);
  databaseName = `restless_identity_${randomUUID().replaceAll("-", "").slice(0, 20)}_test`;
  const databaseUrl = new URL(adminUrl);
  databaseUrl.pathname = `/${databaseName}`;
  admin = new pg.Client({
    connectionString: adminUrl.toString(),
    connectionTimeoutMillis: 5_000,
    statement_timeout: 10_000,
  });
  await admin.connect();
  await admin.query(`CREATE DATABASE ${quotedIdentifier(databaseName)}`);
  databaseCreated = true;

  tempState = await mkdtemp(join(tmpdir(), "restless-smtp-entrypoint-"));
  await chmod(tempState, 0o700);
  smtp = smtpCapture();
  const smtpPort = await smtp.listen();
  const accountPort = await unusedLoopbackPort();
  const origin = `http://127.0.0.1:${accountPort}`;
  const { privateKey } = generateKeyPairSync("ed25519");
  const configPath = join(tempState, "identity.json");
  await writeFile(
    configPath,
    JSON.stringify({
      origin,
      coreOrigin: "http://core.localhost:65535",
      companyName: "SMTP Entrypoint Test",
      ownerEmail: OWNER_EMAIL,
      ownerId: randomUUID(),
      planeId: randomUUID(),
      companyId: randomUUID(),
      cellId: randomUUID(),
      secret: randomBytes(48).toString("hex"),
      signingKey: {
        ...privateKey.export({ format: "jwk" }),
        kid: randomUUID(),
      },
      databaseUrl: databaseUrl.toString(),
      smtp: {
        host: "127.0.0.1",
        port: smtpPort,
        secure: false,
        ignoreTLS: true,
        from: "Restless SMTP test <accounts@restless.test>",
      },
      address: "127.0.0.1",
      port: accountPort,
    }),
    { mode: 0o600, flag: "wx" },
  );

  identityChild = spawn(process.execPath, ["src/main.mjs"], {
    cwd: new URL("..", import.meta.url),
    env: { ...process.env, RESTLESS_IDENTITY_CONFIG: configPath },
    stdio: ["ignore", "ignore", "ignore"],
  });
  let childSpawnError;
  identityChild.once("error", (error) => {
    childSpawnError = error;
  });
  const startupDeadline = Date.now() + TIMEOUT_MS;
  let ready = false;
  while (Date.now() < startupDeadline) {
    if (childSpawnError)
      throw Error("production identity entrypoint could not be started");
    if (identityChild.exitCode !== null || identityChild.signalCode !== null)
      throw Error("production identity entrypoint exited before becoming ready");
    try {
      const response = await fetch(origin + "/health", {
        signal: AbortSignal.timeout(1_000),
      });
      ready = response.ok;
      if (ready) break;
    } catch {}
    await delay(100);
  }
  assert(ready, "production identity entrypoint did not become ready");

  const signup = await fetch(origin + "/api/auth/sign-up/email", {
    method: "POST",
    headers: { Origin: origin, "Content-Type": "application/json" },
    body: JSON.stringify({
      name: "SMTP Entrypoint Owner",
      email: OWNER_EMAIL,
      password: PASSWORD,
      callbackURL: origin + "/",
    }),
    signal: AbortSignal.timeout(5_000),
  });
  assert.equal(signup.status, 200, "account sign-up failed");
  await signup.arrayBuffer();

  const captured = await timeout(smtp.message, "verification email delivery");
  const link = verificationLink(captured);
  const verification = await fetch(link, {
    redirect: "manual",
    signal: AbortSignal.timeout(5_000),
  });
  assert(
    verification.status === 200 ||
      (verification.status >= 300 && verification.status < 400),
    "verification link was not accepted",
  );
  await verification.arrayBuffer();

  const signIn = await fetch(origin + "/api/auth/sign-in/email", {
    method: "POST",
    headers: { Origin: origin, "Content-Type": "application/json" },
    body: JSON.stringify({ email: OWNER_EMAIL, password: PASSWORD }),
    redirect: "manual",
    signal: AbortSignal.timeout(5_000),
  });
  assert.equal(signIn.status, 200, "verified account could not sign in");
  const signedIn = await signIn.json();
  assert.equal(signedIn.user?.email, OWNER_EMAIL);
  console.log(
    "PASS production identity entrypoint delivered verification mail through captured SMTP and admitted verified sign-in",
  );
} catch (error) {
  failure = error;
} finally {
  try {
    await stopChild(identityChild);
  } catch (error) {
    failure ??= error;
  }
  try {
    await smtp?.close();
  } catch (error) {
    failure ??= error;
  }
  if (admin && databaseName && databaseCreated) {
    try {
      await admin.query(
        `DROP DATABASE IF EXISTS ${quotedIdentifier(databaseName)} WITH (FORCE)`,
      );
    } catch (error) {
      failure ??= error;
    }
  }
  try {
    await admin?.end();
  } catch (error) {
    failure ??= error;
  }
  if (tempState) {
    try {
      await rm(tempState, { recursive: true, force: true });
    } catch (error) {
      failure ??= error;
    }
  }
}

if (failure) {
  console.error(`FAIL ${failure.message}`);
  process.exitCode = 1;
}
