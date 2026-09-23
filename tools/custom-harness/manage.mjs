#!/usr/bin/env node
// Runtime-side installation and ACP discovery. Never infer usable model access from installation.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";

const self = fileURLToPath(import.meta.url);
const now = () => new Date().toISOString();
const emit = (value) => process.stdout.write(`${JSON.stringify(value)}\n`);
export function harnessRoot(id) {
  if (!/^[a-z][a-z0-9-]{0,47}$/.test(id ?? ""))
    throw new Error("Invalid harness identifier");
  return `/company/home/.restless/custom-harnesses/${id}`;
}
function prepare(root) {
  fs.mkdirSync(root, { recursive: true, mode: 0o700 });
}
function writeState(root, state) {
  const dest = path.join(root, "install.json");
  const temp = `${dest}.${randomUUID()}.tmp`;
  fs.writeFileSync(temp, JSON.stringify(state), { mode: 0o600 });
  fs.renameSync(temp, dest);
}
function processStart(pid) {
  // comm can itself contain spaces and parentheses; fields after the final ')' start at field 3.
  const stat = fs.readFileSync(`/proc/${pid}/stat`, "utf8");
  return stat.slice(stat.lastIndexOf(")") + 2).split(" ")[19];
}
export function installationStatus(root) {
  let state;
  try {
    state = JSON.parse(
      fs.readFileSync(path.join(root, "install.json"), "utf8"),
    );
  } catch (e) {
    return { state: e.code === "ENOENT" ? "not_installed" : "unavailable" };
  }
  if (state.state === "installing") {
    let live = false;
    try {
      live = processStart(state.pid) === state.process_start;
    } catch {}
    if (!live)
      return {
        ...state,
        state: "interrupted",
        message: "Installation was interrupted. Retry to continue.",
      };
  }
  return state;
}
export function environment(root, values = {}) {
  const home = path.join(root, "home");
  fs.mkdirSync(home, { recursive: true, mode: 0o700 });
  // Setup/probes do not inherit actor grants or unrelated provider credentials from their caller.
  const inherited = {};
  for (const key of [
    "PATH",
    "LANG",
    "LC_ALL",
    "TERM",
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
  ]) {
    if (process.env[key]) inherited[key] = process.env[key];
  }
  for (const [key, value] of Object.entries(values)) {
    if (
      !/^[A-Z_][A-Z0-9_]*$/.test(key) ||
      typeof value !== "string" ||
      value.includes("\0")
    ) {
      throw new Error("Invalid harness environment");
    }
  }
  return {
    ...inherited,
    ...values,
    HOME: home,
    XDG_CONFIG_HOME: path.join(home, ".config"),
    XDG_CACHE_HOME: path.join(home, ".cache"),
    HERMES_HOME: path.join(home, ".hermes"),
    OPENCLAW_STATE_DIR: path.join(home, ".openclaw"),
    RESTLESS_HARNESS_DIR: root,
  };
}
function argvFor(root, argv, systemPromptPath) {
  if (
    !Array.isArray(argv) ||
    !argv.length ||
    argv.some((a) => typeof a !== "string" || a.includes("\0"))
  ) {
    throw new Error("A harness launch command is required");
  }
  return argv.map((a) =>
    a
      .replaceAll("${HARNESS_DIR}", root)
      .replaceAll("${SYSTEM_PROMPT_FILE}", systemPromptPath),
  );
}
function stop(child) {
  if (!child.pid) return;
  try {
    process.kill(-child.pid, "SIGKILL");
  } catch {}
}
function redactor(values) {
  const secrets = Object.values(values)
    .filter((v) => typeof v === "string" && v.length >= 4)
    .sort((a, b) => b.length - a.length);
  return (text) =>
    secrets.reduce(
      (result, value) => result.replaceAll(value, "[redacted]"),
      String(text),
    );
}

// flock owns the whole installer lifetime; a status read never unlocks or overwrites a live job.
export async function install(root, config) {
  prepare(root);
  if (
    typeof config.install !== "string" ||
    !config.install.trim() ||
    config.install.length > 32768
  ) {
    throw new Error("An install command is required (maximum 32 KiB)");
  }
  const child = spawn(
    "flock",
    [
      "--nonblock",
      "--conflict-exit-code",
      "75",
      "--no-fork",
      path.join(root, "install.lock"),
      process.execPath,
      self,
      "--install-worker",
      root,
    ],
    { stdio: ["pipe", "ignore", "pipe"] },
  );
  child.stderr.resume();
  child.stdin.on("error", () => {});
  child.stdin.end(JSON.stringify(config));
  await new Promise((resolve, reject) => {
    child.on("error", reject);
    child.on("close", (code) =>
      code === 0
        ? resolve()
        : reject(
            new Error(
              code === 75
                ? "Installation is already running"
                : "Installer failed; inspect installation status",
            ),
          ),
    );
  });
  return installationStatus(root);
}
async function installWorker(root, config) {
  const env = environment(root, config.env ?? {});
  const redact = redactor(config.env ?? {});
  const state = {
    state: "installing",
    job: randomUUID(),
    pid: process.pid,
    process_start: processStart(process.pid),
    started_at: now(),
  };
  writeState(root, state);
  const child = spawn("sh", ["-c", config.install], {
    cwd: root,
    env,
    detached: true,
    stdio: ["ignore", "pipe", "pipe"],
  });
  let tail = "",
    stopped = null;
  for (const stream of [child.stdout, child.stderr]) {
    stream.setEncoding("utf8");
    stream.on("data", (chunk) => {
      tail = (tail + chunk).slice(-16000);
    });
  }
  const timer = setTimeout(() => {
    stopped = "Installation timed out after 15 minutes.";
    stop(child);
  }, 900000);
  const interrupted = () => {
    stopped = "Installation interrupted.";
    stop(child);
  };
  process.once("SIGTERM", interrupted);
  process.once("SIGINT", interrupted);
  const error = await new Promise((resolve) => {
    child.on("error", (e) => resolve(e.message));
    child.on("exit", () => stop(child));
    child.on("close", (code, signal) =>
      resolve(
        stopped ??
          (code === 0
            ? null
            : `Installer ended ${signal ?? `with exit ${code}`}.`),
      ),
    );
  });
  clearTimeout(timer);
  process.removeListener("SIGTERM", interrupted);
  process.removeListener("SIGINT", interrupted);
  // Do not leave a successful installer's background processes around either.
  stop(child);
  writeState(root, {
    ...state,
    state: error ? "failed" : "installed",
    finished_at: now(),
    message: redact(error ?? "Installation completed."),
    log: redact(tail),
  });
  if (error) process.exitCode = 1;
}

function modelChoices(options) {
  if (!Array.isArray(options)) return [];
  return options.flatMap((o) =>
    o.options
      ? modelChoices(o.options)
      : typeof o.value === "string" && o.value
        ? [{ id: o.value, name: o.name ?? o.value }]
        : [],
  );
}
export async function probe(
  root,
  { config, env = {}, model, secret_names },
  { timeoutMs = 25000, maxFrameBytes = 16 * 1024 * 1024, onStderr } = {},
) {
  prepare(root);
  const gateway = config.adapter === "openclaw-gateway";
  const wrapped = gateway || config.adapter === "hermes-acp";
  const systemPromptPath = path.join(root, `.probe-context-${randomUUID()}.md`);
  fs.writeFileSync(
    systemPromptPath,
    "You are a Restless capability-discovery probe. Do not run tools or perform work.",
    { mode: 0o600 },
  );
  const argv = wrapped
    ? [
        process.execPath,
        path.join(path.dirname(self), "run.mjs"),
        path.basename(root),
        systemPromptPath,
      ]
    : argvFor(root, config.command, systemPromptPath);
  const launchEnvironment = environment(root, env);
  if (wrapped)
    launchEnvironment.RESTLESS_CUSTOM_CONFIG = JSON.stringify({
      ...config,
      env,
    });
  const redact = redactor(
    secret_names === undefined
      ? env
      : Object.fromEntries(
          secret_names
            .filter((name) => typeof name === "string" && name in env)
            .map((name) => [name, env[name]]),
        ),
  );
  const clean = (value) =>
    typeof value === "string"
      ? redact(value)
      : Array.isArray(value)
        ? value.map(clean)
        : value && typeof value === "object"
          ? Object.fromEntries(
              Object.entries(value).map(([k, v]) => [k, clean(v)]),
            )
          : value;
  const child = spawn(argv[0], argv.slice(1), {
    cwd: root,
    env: launchEnvironment,
    detached: true,
    stdio: ["pipe", "pipe", "pipe"],
  });
  const pending = new Map();
  let buffer = "",
    next = 0,
    failure = null,
    initialized = null;
  const fail = (message) => {
    failure ??= new Error(message);
    for (const waiter of pending.values()) waiter.reject(failure);
    pending.clear();
  };
  const send = (message) => child.stdin.write(`${JSON.stringify(message)}\n`);
  // Diagnostics are opt-in for local tests; owner discovery never returns raw process output.
  if (onStderr) {
    child.stderr.setEncoding("utf8");
    child.stderr.on("data", (chunk) => onStderr(redact(chunk)));
  } else child.stderr.resume();
  child.stdin.on("error", (e) =>
    fail(`ACP input closed: ${e.code ?? "write failed"}`),
  );
  child.on("error", (e) =>
    fail(`Could not launch harness: ${e.code ?? "process error"}`),
  );
  const closed = new Promise((resolve) =>
    child.on("close", () => {
      fail("Harness exited during ACP discovery");
      resolve();
    }),
  );
  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (chunk) => {
    buffer += chunk;
    let newline;
    while ((newline = buffer.indexOf("\n")) >= 0) {
      const line = buffer.slice(0, newline);
      buffer = buffer.slice(newline + 1);
      if (Buffer.byteLength(line) > maxFrameBytes) {
        fail("ACP response exceeded the frame limit");
        stop(child);
        return;
      }
      if (!line.trim()) continue;
      let message;
      try {
        message = JSON.parse(line);
      } catch {
        fail("Harness wrote invalid JSON to ACP stdout");
        stop(child);
        return;
      }
      if (!message || typeof message !== "object") {
        fail("Harness wrote an invalid ACP message");
        stop(child);
        return;
      }
      if (message.method && message.id !== undefined) {
        send({
          jsonrpc: "2.0",
          id: message.id,
          error: {
            code: -32601,
            message: "Discovery does not authorize tools or filesystem access",
          },
        });
        continue;
      }
      const waiter = pending.get(message.id);
      if (!waiter) continue;
      pending.delete(message.id);
      message.error
        ? waiter.reject(
            new Error(message.error.message ?? "ACP request failed"),
          )
        : waiter.resolve(message.result);
    }
    if (Buffer.byteLength(buffer) > maxFrameBytes) {
      fail("ACP response exceeded the frame limit");
      stop(child);
    }
  });
  const request = (method, params) =>
    new Promise((resolve, reject) => {
      if (failure) return reject(failure);
      const id = ++next;
      pending.set(id, { resolve, reject });
      send({ jsonrpc: "2.0", id, method, params });
    });
  const timer = setTimeout(
    () => {
      fail("ACP discovery timed out. Complete harness setup and retry.");
      stop(child);
    },
    gateway ? Math.max(timeoutMs, 90000) : timeoutMs,
  );
  try {
    const init = (initialized = await request("initialize", {
      protocolVersion: 1,
      clientCapabilities: {},
      clientInfo: { name: "restless", version: "1" },
    }));
    if (init?.protocolVersion !== 1)
      throw new Error("Harness does not support ACP protocol version 1");
    const session = await request("session/new", { cwd: root, mcpServers: [] });
    if (typeof session?.sessionId !== "string" || !session.sessionId)
      throw new Error("Harness did not return an ACP session");
    const option = session.configOptions?.find(
      (o) => o.category === "model" || o.id === "model",
    );
    const models = option
      ? modelChoices(option.options)
      : (session.models?.availableModels ?? [])
          .filter((m) => typeof m.modelId === "string")
          .map((m) => ({ id: m.modelId, name: m.name ?? m.modelId }));
    const selection = option
      ? "config_option"
      : session.models
        ? "legacy"
        : "unavailable";
    let selected = null;
    if (model) {
      if (!models.some((m) => m.id === model))
        throw new Error("Harness did not advertise the selected model");
      if (option) {
        const result = await request("session/set_config_option", {
          sessionId: session.sessionId,
          configId: option.id,
          value: model,
        });
        if (
          !result?.configOptions?.some(
            (o) => o.id === option.id && o.currentValue === model,
          )
        ) {
          throw new Error("Harness did not confirm the selected model");
        }
      } else {
        // Legacy ACP acknowledges set_model with an empty response. Preserve that weaker evidence explicitly.
        await request("session/set_model", {
          sessionId: session.sessionId,
          modelId: model,
        });
      }
      selected = model;
    }
    return clean({
      state:
        models.length && selection !== "unavailable"
          ? "compatible"
          : "setup_required",
      authentication: "unverified",
      message: models.length
        ? undefined
        : "No selectable models reported. Configure this harness and check again.",
      agent: init.agentInfo ?? null,
      auth_methods: init.authMethods ?? [],
      models,
      model_selection: selection,
      model_config_id: option?.id ?? null,
      selected_model: selected,
      capabilities: init.agentCapabilities ?? {},
      checked_at: now(),
    });
  } catch (e) {
    return clean({
      state: "setup_required",
      message: redact(e.message),
      agent: initialized?.agentInfo ?? null,
      auth_methods: initialized?.authMethods ?? [],
      authentication: "unverified",
      checked_at: now(),
    });
  } finally {
    clearTimeout(timer);
    child.stdin.end();
    const killTimer = setTimeout(() => stop(child), 2000);
    await closed;
    clearTimeout(killTimer);
    fs.rmSync(systemPromptPath, { force: true });
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === self) {
  try {
    const [action, id] = process.argv.slice(2);
    if (action === "--install-worker") {
      await installWorker(id, JSON.parse(fs.readFileSync(0, "utf8")));
    } else {
      const root = harnessRoot(id);
      if (action === "status") emit(installationStatus(root));
      else if (action === "install")
        emit(await install(root, JSON.parse(fs.readFileSync(0, "utf8"))));
      else if (action === "probe")
        emit(await probe(root, JSON.parse(fs.readFileSync(0, "utf8"))));
      else throw new Error("Unsupported harness operation");
    }
  } catch (e) {
    emit({ state: "failed", message: e.message });
    process.exitCode = 1;
  }
}
