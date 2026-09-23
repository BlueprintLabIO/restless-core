import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import { spawn } from "node:child_process";
import readline from "node:readline";
test("productive proxy preserves actor scope and passes system context outside user messages", async (t) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "restless-proxy_test-"));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const fixture = path.join(root, "agent.mjs");
  fs.writeFileSync(
    fixture,
    `import readline from 'node:readline';
 import fs from 'node:fs';
 for await(const line of readline.createInterface({input:process.stdin})) {
  const r=JSON.parse(line);let result={};
  if(r.method==='session/new')result={sessionId:'s',models:{currentModelId:'old',availableModels:[{modelId:'new'}]}};
  if(r.method==='session/prompt')result={system:fs.readFileSync(process.argv[2],'utf8'),prompt:r.params.prompt,actor:process.env.RESTLESS_ACTOR,capability:process.env.RESTLESS_SESSION_CAPABILITY,home:process.env.HOME};
  console.log(JSON.stringify({jsonrpc:'2.0',id:r.id,result}));
 }`,
  );
  const system = path.join(root, "system.md");
  fs.writeFileSync(system, "Restless operating context");
  const config = {
    adapter: "acp",
    command: [process.execPath, fixture, "${SYSTEM_PROMPT_FILE}"],
    env: { RESTLESS_ACTOR: "wrong", HOME: "/wrong" },
  };
  const script = `import {run} from ${JSON.stringify(new URL("./run.mjs", import.meta.url).href)};await run(${JSON.stringify({ root, systemPromptPath: system, config })});`;
  const child = spawn(process.execPath, ["--input-type=module", "-e", script], {
    env: {
      ...process.env,
      RESTLESS_ACTOR: "alice",
      RESTLESS_SESSION_CAPABILITY: "scoped-test-capability",
    },
    stdio: ["pipe", "pipe", "pipe"],
  });
  t.after(() => child.kill());
  const lines = readline
    .createInterface({ input: child.stdout })
    [Symbol.asyncIterator]();
  const request = async (id, method, params) => {
    child.stdin.write(
      JSON.stringify({ jsonrpc: "2.0", id, method, params }) + "\n",
    );
    const next = await lines.next();
    assert.equal(next.done, false);
    return JSON.parse(next.value);
  };
  const session = await request(1, "session/new", {});
  assert.equal(session.result.configOptions[0].id, "restless-legacy-model");
  const selected = await request(2, "session/set_config_option", {
    sessionId: "s",
    configId: "restless-legacy-model",
    value: "new",
  });
  assert.equal(selected.result.configOptions[0].currentValue, "new");
  const first = await request(3, "session/prompt", {
    sessionId: "s",
    prompt: [{ type: "text", text: "hello" }],
  });
  assert.deepEqual(
    first.result.prompt.map((p) => p.text),
    ["hello"],
  );
  assert.equal(first.result.system, "Restless operating context");
  assert.equal(first.result.actor, "alice");
  assert.equal(first.result.capability, "scoped-test-capability");
  assert.equal(first.result.home, path.join(root, "home"));
  const second = await request(4, "session/prompt", {
    sessionId: "s",
    prompt: [{ type: "text", text: "again" }],
  });
  assert.deepEqual(
    second.result.prompt.map((p) => p.text),
    ["again"],
  );
  child.stdin.end();
  await new Promise((resolve) => child.on("close", resolve));
  assert.equal(child.exitCode, 0);
});
