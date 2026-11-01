// Invoked by the Rust ignored integration test with the actual generated overlays.
// Uses only the local scripted provider, fake credentials and disposable profiles.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { homedir, tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

let input = "";
for await (const part of process.stdin) input += part;
const overlays = JSON.parse(input);
assert.equal(typeof overlays.metered, "string");
assert.equal(typeof overlays.subscription, "string");
const root = fileURLToPath(new URL("../../../", import.meta.url));
const work = await mkdtemp(join(tmpdir(), "restless-cache-retention-"));
const log = join(work, "requests.jsonl");
const listener = createServer();
await new Promise((resolve) => listener.listen(0, "127.0.0.1", resolve));
const port = listener.address().port;
await new Promise((resolve) => listener.close(resolve));
let mock;
const profiles = [];
async function stop(child) {
  if (!child?.pid || child.exitCode !== null || child.signalCode !== null)
    return;
  const ended = new Promise((resolve) => child.once("exit", resolve));
  try {
    process.kill(-child.pid, "SIGTERM");
  } catch {}
  const kill = setTimeout(() => {
    try {
      process.kill(-child.pid, "SIGKILL");
    } catch {}
  }, 2000);
  await ended;
  clearTimeout(kill);
}
function markers(node, out = []) {
  if (Array.isArray(node)) {
    for (const value of node) markers(value, out);
  } else if (node && typeof node === "object") {
    if (node.cache_control) out.push(node.cache_control);
    for (const [key, value] of Object.entries(node))
      if (key !== "cache_control") markers(value, out);
  }
  return out;
}
try {
  mock = spawn(
    process.execPath,
    [
      join(root, "tools/mock-model/server.mjs"),
      "--port",
      String(port),
      "--log",
      log,
    ],
    { detached: true, stdio: ["ignore", "pipe", "pipe"] },
  );
  await new Promise((resolve, reject) => {
    let text = "";
    const timer = setTimeout(
      () => reject(Error("Mock provider did not start")),
      5000,
    );
    mock.stdout.on("data", (part) => {
      text += part;
      if (text.includes("mock-model listening")) {
        clearTimeout(timer);
        resolve();
      }
    });
    mock.once("error", reject);
    mock.once("exit", () =>
      reject(Error("Mock provider exited during startup")),
    );
  });
  for (const mode of ["subscription", "metered"]) {
    const profile = "s109-cache-" + randomUUID().slice(0, 8) + "_test";
    const profileDir = join(homedir(), ".omp/profiles", profile);
    await mkdir(profileDir, { mode: 0o700 });
    profiles.push(profileDir);
    await mkdir(join(profileDir, "agent"), { mode: 0o700 });
    await writeFile(
      join(profileDir, "agent/models.yml"),
      `providers:
  anthropic:
    baseUrl: http://127.0.0.1:${port}/v1
    apiKey: RESTLESS_CACHE_PROBE_KEY
    api: anthropic-messages
    models:
      - id: claude-opus-5-5
        name: Claude cache probe
        compat:
          supportsLongCacheRetention: true
`,
      { mode: 0o600 },
    );
    const config = join(work, mode + ".yml");
    await writeFile(config, overlays[mode], { mode: 0o600 });
    const env = {
      ...process.env,
      OMP_PROFILE: profile,
      OMP_AUTH_BROKER_URL: "",
      OMP_AUTH_BROKER_TOKEN: "",
      PI_CACHE_RETENTION: "long",
      RESTLESS_CACHE_PROBE_KEY: "synthetic-cache-probe-not-a-provider-key",
    };
    for (const key of [
      "ANTHROPIC_API_KEY",
      "ANTHROPIC_AUTH_TOKEN",
      "CLAUDE_CODE_OAUTH_TOKEN",
      "OPENAI_API_KEY",
    ])
      delete env[key];
    const child = spawn(
      join(root, "infra/host-tools/omp-source"),
      [
        "--provider=anthropic",
        "--model=claude-opus-5-5",
        "--thinking=off",
        "--print",
        "--mode=json",
        "--no-tools",
        "--no-session",
        "--no-title",
        "--no-extensions",
        "--no-skills",
        "--no-rules",
        "--no-lsp",
        "--max-time=30",
        "--config=" + config,
        "--system-prompt=Bounded cache wire probe.",
        "cache wire probe " + mode,
      ],
      { cwd: work, env, detached: true, stdio: ["ignore", "pipe", "pipe"] },
    );
    const stdout = [],
      stderr = [];
    child.stdout.on("data", (x) => stdout.push(x));
    child.stderr.on("data", (x) => stderr.push(x));
    const deadline = setTimeout(() => stop(child), 35000);
    let status;
    try {
      status = await new Promise((resolve, reject) => {
        child.once("error", reject);
        child.once("exit", resolve);
      });
    } finally {
      clearTimeout(deadline);
      await stop(child);
    }
    assert.equal(status, 0, mode + ": " + Buffer.concat(stderr).toString());
    const requests = (await readFile(log, "utf8"))
      .trim()
      .split("\n")
      .map((x) => JSON.parse(x));
    const last = requests.findLast(
      (x) => x.method === "POST" && x.path.endsWith("/messages"),
    );
    assert.ok(last, mode + ": no request reached the local provider");
    const controls = markers(last.body);
    assert.ok(controls.length > 0, mode + ": cache controls missing");
    assert.ok(controls.every((c) => c.type === "ephemeral"));
    if (mode === "subscription")
      assert.ok(
        controls.some((c) => c.ttl === "1h"),
        "control must request a long cache",
      );
    else
      assert.ok(
        controls.every((c) => c.ttl === undefined || c.ttl === "5m"),
        "metered overlay must override the long environment hint",
      );
    console.log(JSON.stringify({ mode, cacheControls: controls }));
  }
} finally {
  await stop(mock);
  for (const path of profiles) await rm(path, { recursive: true, force: true });
  await rm(work, { recursive: true, force: true });
}
