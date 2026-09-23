// A Gateway belongs to one Restless launch. Native account state is shared; actor histories are separate.
import fs from "node:fs";
import path from "node:path";
import net from "node:net";
import { createRequire } from "node:module";
import { spawn } from "node:child_process";
import { randomUUID, createHash } from "node:crypto";

const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const optionId = "restless-openclaw-model";
function privateJson(file, value) {
  fs.writeFileSync(file, JSON.stringify(value), { mode: 0o600 });
}
async function freePort() {
  const server = net.createServer();
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const port = server.address().port;
  await new Promise((resolve) => server.close(resolve));
  return port;
}
async function listening(port) {
  return new Promise((resolve) => {
    const socket = net.connect({ host: "127.0.0.1", port });
    const finish = (value) => {
      socket.destroy();
      resolve(value);
    };
    socket.setTimeout(200);
    socket.once("connect", () => finish(true));
    socket.once("error", () => finish(false));
    socket.once("timeout", () => finish(false));
  });
}

export async function openGateway({
  root,
  command,
  env,
  context = "",
  cwd = process.cwd(),
}) {
  const executable = command[0];
  const profile = env.OPENCLAW_STATE_DIR;
  const directory = fs.mkdtempSync(path.join(root, "gateway-"));
  fs.chmodSync(directory, 0o700);
  const state = path.join(directory, "profile");
  const workspace = path.join(directory, "workspace");
  fs.mkdirSync(state, { mode: 0o700 });
  fs.mkdirSync(workspace, { mode: 0o700 });
  const actor = env.RESTLESS_ACTOR ?? "discovery";
  const agent = `restless-${createHash("sha256").update(actor).digest("hex").slice(0, 24)}`;
  const sessionKey = `agent:${agent}:restless-${randomUUID()}`;
  let gateway;
  const children = new Set();
  let closed = false;
  const close = async () => {
    if (closed) return;
    closed = true;
    const closing = [...children];
    const waits = closing.map((child) =>
      child.exitCode !== null || child.signalCode !== null
        ? Promise.resolve()
        : new Promise((resolve) => child.once("close", resolve)),
    );
    // Let the native Gateway release its shared profile lock before a probe
    // is followed immediately by a productive launch. SIGKILL left the next
    // Gateway alive but unable to bind until its readiness deadline expired.
    for (const child of closing) {
      try {
        child.kill("SIGTERM");
      } catch {}
    }
    await Promise.race([Promise.all(waits), delay(3000)]);
    for (const child of closing) {
      if (child.exitCode !== null || child.signalCode !== null) continue;
      try {
        child.kill("SIGKILL");
      } catch {}
    }
    await Promise.all(waits);
    fs.rmSync(directory, { recursive: true, force: true });
  };
  try {
    // Native auth refresh must retain a single store and lock owner. Never copy rotating OAuth tokens.
    for (const name of ["state", "locks", "extensions"]) {
      fs.mkdirSync(path.join(profile, name), { recursive: true, mode: 0o700 });
      fs.symlinkSync(path.join(profile, name), path.join(state, name));
    }
    let saved = {};
    const configPath = path.join(profile, "openclaw.json");
    if (fs.existsSync(configPath)) {
      const require = createRequire(fs.realpathSync(executable));
      saved = require("json5").parse(fs.readFileSync(configPath, "utf8"));
    }
    const port = await freePort();
    const token = randomUUID();
    const tokenFile = path.join(directory, "token");
    fs.writeFileSync(tokenFile, token, { mode: 0o600 });
    fs.writeFileSync(path.join(workspace, "AGENTS.md"), context, {
      mode: 0o600,
    });
    const launchConfig = {
      ...(saved.models ? { models: saved.models } : {}),
      ...(saved.auth ? { auth: saved.auth } : {}),
      ...(saved.secrets ? { secrets: saved.secrets } : {}),
      ...(saved.plugins ? { plugins: saved.plugins } : {}),
      agents: {
        defaults: {
          ...(saved.agents?.defaults?.model
            ? { model: saved.agents.defaults.model }
            : {}),
          ...(saved.agents?.defaults?.models
            ? { models: saved.agents.defaults.models }
            : {}),
          workspace,
          cwd,
          skipBootstrap: true,
          skills: [],
          startupContext: { enabled: false },
          bootstrapMaxChars: Math.max(20000, context.length + 2000),
          bootstrapTotalMaxChars: Math.max(24000, context.length + 4000),
        },
        entries: {
          [agent]: { agentDir: path.join(root, "actors", agent, "agent") },
        },
      },
      tools: {
        allow: [
          "exec",
          "process",
          "read",
          "write",
          "edit",
          "apply_patch",
          "ls",
          "web_search",
          "web_fetch",
          "browser",
          "file_fetch",
          "file_write",
          "dir_list",
          "dir_fetch",
        ],
      },
      gateway: {
        mode: "local",
        bind: "loopback",
        port,
        auth: { mode: "token" },
      },
    };
    const launchConfigPath = path.join(directory, "openclaw.json");
    privateJson(launchConfigPath, launchConfig);
    const launchEnv = {
      ...env,
      OPENCLAW_STATE_DIR: state,
      OPENCLAW_CONFIG_PATH: launchConfigPath,
      OPENCLAW_AGENT_DIR: path.join(profile, "agents", "main", "agent"),
      OPENCLAW_GATEWAY_PORT: String(port),
      OPENCLAW_GATEWAY_TOKEN: token,
    };
    const launch = (args, stdio) => {
      const child = spawn(executable, args, { env: launchEnv, cwd, stdio });
      children.add(child);
      child.once("close", () => children.delete(child));
      return child;
    };
    gateway = launch(
      [
        "gateway",
        "run",
        "--allow-unconfigured",
        "--bind",
        "loopback",
        "--port",
        String(port),
        "--auth",
        "token",
      ],
      ["ignore", "ignore", "pipe"],
    );
    let startupError;
    gateway.on("error", (error) => {
      startupError = error;
    });
    // Keep native diagnostics out of ACP stdout and owner-facing probe errors.
    gateway.stderr.resume();
    for (let i = 0; ; i++) {
      if (
        startupError ||
        gateway.exitCode !== null ||
        gateway.signalCode !== null
      )
        throw new Error(
          "OpenClaw Gateway exited before readiness. Check its native provider setup.",
        );
      if (await listening(port)) break;
      if (i === 149)
        throw new Error(
          "OpenClaw Gateway did not become ready within 30 seconds.",
        );
      await delay(200);
    }
    const call = (method, params) =>
      new Promise((resolve, reject) => {
        const child = launch(
          [
            "gateway",
            "call",
            method,
            "--json",
            "--params",
            JSON.stringify(params),
          ],
          ["ignore", "pipe", "ignore"],
        );
        let output = "";
        const timer = setTimeout(() => {
          try {
            child.kill("SIGKILL");
          } catch {}
        }, 20000);
        child.on("error", reject);
        child.stdout.setEncoding("utf8");
        child.stdout.on("data", (chunk) => {
          output += chunk;
          if (Buffer.byteLength(output) > 16 * 1024 * 1024) {
            try {
              child.kill("SIGKILL");
            } catch {}
          }
        });
        child.once("close", (code) => {
          clearTimeout(timer);
          if (code !== 0)
            return reject(
              new Error(
                `OpenClaw ${method} failed. Check model availability and native setup.`,
              ),
            );
          try {
            resolve(JSON.parse(output));
          } catch {
            reject(new Error("OpenClaw returned invalid Gateway JSON"));
          }
        });
      });
    const catalog = await call("models.list", {
      agentId: agent,
      view: "configured",
      includeDetails: true,
    });
    const models = (catalog.models ?? [])
      .filter(
        (model) =>
          typeof model.id === "string" && typeof model.provider === "string",
      )
      .map((model) => ({
        id: `${model.provider}/${model.id}`,
        name: model.name ?? model.id,
      }));
    const bridge = new GatewayModels({ call, models, sessionKey });
    return {
      command: [
        executable,
        "acp",
        "--url",
        `ws://127.0.0.1:${port}`,
        "--token-file",
        tokenFile,
        "--session",
        sessionKey,
        "--no-prefix-cwd",
      ],
      env: launchEnv,
      bridge,
      close,
    };
  } catch (error) {
    await close();
    throw error;
  }
}

export class GatewayModels {
  constructor({ call, models, sessionKey }) {
    this.call = call;
    this.models = models;
    this.sessionKey = sessionKey;
    this.newRequests = new Set();
    this.sessions = new Map();
  }
  option(value = "") {
    return {
      id: optionId,
      category: "model",
      name: "Model",
      type: "select",
      currentValue: value,
      options: this.models.map((model) => ({
        value: model.id,
        name: model.name,
      })),
    };
  }
  async request(message) {
    if (message.method === "session/new") {
      if (this.sessions.size || this.newRequests.size)
        return {
          rejected: {
            jsonrpc: "2.0",
            id: message.id,
            error: {
              code: -32602,
              message: "One scoped session is supported per Restless launch",
            },
          },
        };
      this.newRequests.add(message.id);
    }
    if (
      message.method === "session/prompt" &&
      !this.sessions.get(message.params?.sessionId)
    )
      return {
        rejected: {
          jsonrpc: "2.0",
          id: message.id,
          error: {
            code: -32602,
            message: "Select and confirm a model before starting this session",
          },
        },
      };
    if (
      message.method !== "session/set_config_option" ||
      message.params?.configId !== optionId
    )
      return message;
    const { sessionId, value } = message.params;
    if (
      !this.sessions.has(sessionId) ||
      !this.models.some((model) => model.id === value)
    )
      return {
        rejected: {
          jsonrpc: "2.0",
          id: message.id,
          error: { code: -32602, message: "Unknown session or model" },
        },
      };
    try {
      const result = await this.call("sessions.patch", {
        key: this.sessionKey,
        model: value,
      });
      if (
        result.key !== this.sessionKey ||
        `${result.resolved?.modelProvider}/${result.resolved?.model}` !== value
      )
        throw new Error("OpenClaw did not confirm the exact selected model");
      this.sessions.set(sessionId, value);
      return {
        rejected: {
          jsonrpc: "2.0",
          id: message.id,
          result: { configOptions: [this.option(value)] },
        },
      };
    } catch (error) {
      return {
        rejected: {
          jsonrpc: "2.0",
          id: message.id,
          error: { code: -32603, message: error.message },
        },
      };
    }
  }
  response(message) {
    if (this.newRequests.delete(message.id) && message.result?.sessionId) {
      this.sessions.set(message.result.sessionId, "");
      return {
        ...message,
        result: {
          ...message.result,
          configOptions: [
            ...(message.result.configOptions ?? []),
            this.option(),
          ],
        },
      };
    }
    return message;
  }
}
