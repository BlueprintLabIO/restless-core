// Real installed OpenClaw, isolated deterministic HTTP provider; no claim of live account authentication.
import fs from "node:fs";
import http from "node:http";
import assert from "node:assert/strict";
import readline from "node:readline";
import { spawn } from "node:child_process";
import { probe, environment, harnessRoot } from "./manage.mjs";
import { fileURLToPath } from "node:url";
assert(
  process.env.RESTLESS_COMPANY_ID?.endsWith("_test"),
  "Run only in a disposable _test company runtime",
);
const root = harnessRoot("openclaw");
const configFile = root + "/home/.openclaw/openclaw.json";
const hostFixture = process.argv.includes("--host-fixture");
assert(
  !fs.existsSync(configFile),
  "Smoke must not replace native owner settings",
);
fs.mkdirSync(root + "/home/.openclaw", { recursive: true, mode: 0o700 });
const calls = [];
const server = http.createServer(async (req, res) => {
  let data = "";
  for await (const part of req) data += part;
  if (req.method === "GET") {
    res.setHeader("content-type", "application/json");
    res.end(
      JSON.stringify({
        object: "list",
        data: [{ id: "fixture-model", object: "model" }],
      }),
    );
    return;
  }
  const input = JSON.parse(data);
  calls.push(input);
  if (hostFixture)
    fs.writeFileSync("/tmp/openclaw-host-calls.json", JSON.stringify(calls), {
      mode: 0o600,
    });
  res.writeHead(200, { "content-type": "text/event-stream" });
  const emit = (delta, finish = null) =>
    res.write(
      "data: " +
        JSON.stringify({
          id: "isolated-fixture",
          object: "chat.completion.chunk",
          model: "fixture-model",
          choices: [{ index: 0, delta, finish_reason: finish }],
        }) +
        "\n\n",
    );
  if (calls.length === 1) {
    emit({
      role: "assistant",
      tool_calls: [
        {
          index: 0,
          id: "fixture-scope",
          type: "function",
          function: {
            name: "exec",
            arguments: JSON.stringify({
              command: hostFixture
                ? "printf 'ACTOR=%s CAPABILITY_SHA256=%s' \"$RESTLESS_ACTOR\" \"$(printf '%s' \"$RESTLESS_SESSION_CAPABILITY\" | sha256sum | cut -d ' ' -f 1)\""
                : 'printf \'ACTOR=%s CAPABILITY=%s\' "$RESTLESS_ACTOR" "$RESTLESS_SESSION_CAPABILITY"',
              yieldMs: 1000,
            }),
          },
        },
      ],
    });
    emit({}, "tool_calls");
    res.end("data: [DONE]\n\n");
    return;
  }
  emit({ role: "assistant", content: "OpenClaw " });
  setTimeout(() => {
    emit({ content: "adapter stream verified." });
    emit({}, "stop");
    res.end("data: [DONE]\n\n");
  }, 200);
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
fs.writeFileSync(
  configFile,
  JSON.stringify({
    models: {
      providers: {
        "restless-fixture": {
          baseUrl: `http://127.0.0.1:${server.address().port}/v1`,
          apiKey: "isolated-fixture-key",
          api: "openai-completions",
          models: [
            {
              id: "fixture-model",
              name: "Isolated fixture",
              reasoning: false,
              input: ["text"],
              contextWindow: 32000,
              maxTokens: 1000,
              cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
            },
          ],
        },
      },
    },
    agents: {
      defaults: { model: { primary: "restless-fixture/fixture-model" } },
    },
  }),
  { mode: 0o600 },
);
const config = {
  adapter: "openclaw-gateway",
  command: [root + "/package/node_modules/.bin/openclaw", "acp"],
};
let child;
const pending = new Map();
const deadline = hostFixture
  ? null
  : setTimeout(() => {
      for (const waiter of pending.values())
        waiter.reject(new Error("OpenClaw smoke timed out"));
      pending.clear();
      if (child) {
        try {
          process.kill(-child.pid, "SIGKILL");
        } catch {}
      }
    }, 140000);
try {
  if (hostFixture) {
    fs.writeFileSync(
      "/tmp/openclaw-host-config.json",
      JSON.stringify({ config }),
      { mode: 0o600 },
    );
    await new Promise((resolve) => {
      process.once("SIGTERM", resolve);
      process.once("SIGINT", resolve);
    });
  } else {
    const discovered = await probe(
      root,
      { config, model: "restless-fixture/fixture-model" },
      { onStderr: console.error },
    );
    console.log(JSON.stringify({ step: "discovery", result: discovered }));
    assert.equal(discovered.selected_model, "restless-fixture/fixture-model");
    fs.writeFileSync(
      "/tmp/openclaw-system.md",
      "You are the isolated Restless test actor. Context marker: RESTLESS_SYSTEM_CONTEXT_591.",
    );
    child = spawn(
      "node",
      [
        fileURLToPath(new URL("./run.mjs", import.meta.url)),
        "openclaw",
        "/tmp/openclaw-system.md",
      ],
      {
        env: {
          ...environment(root),
          RESTLESS_CUSTOM_CONFIG: JSON.stringify(config),
          RESTLESS_ACTOR: "delivery-build",
          RESTLESS_SESSION_CAPABILITY: "isolated-test-capability",
        },
        stdio: ["pipe", "pipe", "pipe"],
        detached: true,
      },
    );
    child.stderr.on("data", (d) => console.error(String(d).slice(0, 500)));
    let id = 0;
    const updates = [];
    const lines = readline.createInterface({ input: child.stdout });
    lines.on("line", (line) => {
      const value = JSON.parse(line);
      if (value.method === "session/request_permission") {
        const option = value.params.options.find(
          (option) => option.kind === "allow_once",
        );
        child.stdin.write(
          JSON.stringify({
            jsonrpc: "2.0",
            id: value.id,
            result: {
              outcome: option
                ? { outcome: "selected", optionId: option.optionId }
                : { outcome: "cancelled" },
            },
          }) + "\n",
        );
        return;
      }
      if (value.method) {
        updates.push(value);
        return;
      }
      const p = pending.get(value.id);
      if (p) {
        pending.delete(value.id);
        value.error
          ? p.reject(new Error(JSON.stringify(value.error)))
          : p.resolve(value.result);
      }
    });
    const request = (method, params) =>
      new Promise((resolve, reject) => {
        const n = ++id;
        pending.set(n, { resolve, reject });
        child.stdin.write(
          JSON.stringify({ jsonrpc: "2.0", id: n, method, params }) + "\n",
        );
      });
    await request("initialize", {
      protocolVersion: 1,
      clientCapabilities: {},
      clientInfo: { name: "restless", version: "1" },
    });
    const session = await request("session/new", { cwd: root, mcpServers: [] });
    const option = session.configOptions.find((o) => o.category === "model");
    await request("session/set_config_option", {
      sessionId: session.sessionId,
      configId: option.id,
      value: "restless-fixture/fixture-model",
    });
    const result = await request("session/prompt", {
      sessionId: session.sessionId,
      prompt: [{ type: "text", text: "Reply with the verification phrase." }],
    });
    const chunks = updates.filter(
      (u) => u.params?.update?.sessionUpdate === "agent_message_chunk",
    );
    assert(chunks.length > 0, "Real OpenClaw must stream ACP message updates");
    assert(calls.length > 0, "Real OpenClaw must invoke selected API");
    assert.equal(calls[0].model, "fixture-model");
    const toolResult = calls
      .slice(1)
      .flatMap((call) => call.messages)
      .find(
        (message) =>
          message.role === "tool" &&
          JSON.stringify(message.content).includes(
            "ACTOR=delivery-build CAPABILITY=isolated-test-capability",
          ),
      );
    assert(
      toolResult,
      "Native shell must receive only this launch's actor and scoped capability",
    );
    const names = calls[0].tools.map((tool) => tool.function.name);
    assert(
      !names.some((name) =>
        [
          "subagents",
          "conversations_send",
          "create_goal",
          "secrets",
          "sessions_spawn",
        ].includes(name),
      ),
      "Private coordination and secret tools must not augment the Restless launch",
    );
    assert(
      calls[0].messages.some(
        (m) =>
          m.role === "system" &&
          JSON.stringify(m.content).includes("RESTLESS_SYSTEM_CONTEXT_591"),
      ),
      "Core context must be in the native system message",
    );
    console.log(
      JSON.stringify({
        step: "prompt",
        result,
        chunks: chunks.length,
        providerCalls: calls.length,
        systemContext: true,
        tools: calls[0].tools?.map((t) => t.function.name),
      }),
    );
    child.stdin.end();
    await new Promise((r) => child.once("close", r));
    child = null;
  }
} finally {
  if (deadline) clearTimeout(deadline);
  if (child) {
    try {
      process.kill(-child.pid, "SIGKILL");
    } catch {}
  }
  fs.rmSync(configFile, { force: true });
  fs.rmSync("/tmp/openclaw-system.md", { force: true });
  server.closeAllConnections();
  await new Promise((r) => server.close(r));
}
