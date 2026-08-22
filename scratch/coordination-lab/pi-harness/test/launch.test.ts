import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { loadLaunch } from "../src/launch.js";

async function fixture(overrides: Record<string, unknown> = {}) {
  const root = await mkdtemp(join(tmpdir(), "restless-launch-test-"));
  const workspace = join(root, "workspace");
  await mkdir(workspace);
  const prompt = join(root, "system.md");
  await writeFile(prompt, "Exact system prompt\n");
  const launch = join(root, "launch.json");
  const value = {
    schemaVersion: 1,
    sessionId: "session-test",
    actor: { id: "exec", kind: "exec", role: "Exec" },
    cwd: workspace,
    systemPromptPath: prompt,
    model: { provider: "openrouter", id: "example/free", reasoning: "medium" },
    tools: ["read"],
    writeScope: "none",
    limits: { maxTurns: 3, timeoutMs: 10_000, commandTimeoutMs: 1_000, maxOutputBytes: 8_000 },
    ...overrides,
  };
  await writeFile(launch, JSON.stringify(value));
  return { root, workspace, prompt, launch };
}

test("loads and hashes the exact system prompt and contract", async () => {
  const item = await fixture();
  const loaded = await loadLaunch(item.launch);
  assert.equal(loaded.systemPrompt, "Exact system prompt\n");
  assert.equal(loaded.contract.actor.id, "exec");
  assert.match(loaded.systemPromptSha256, /^[a-f0-9]{64}$/);
  assert.match(loaded.contractSha256, /^[a-f0-9]{64}$/);
});

test("a read-only launch cannot smuggle a writable shell", async () => {
  const item = await fixture({ tools: ["read", "run"] });
  await assert.rejects(loadLaunch(item.launch), /read-only launch cannot expose/);
});

test("a read-only launch may expose scoped perception", async () => {
  const item = await fixture({ tools: ["read", "list", "search"] });
  const loaded = await loadLaunch(item.launch);
  assert.deepEqual(loaded.contract.tools, ["read", "list", "search"]);
});

test("rejects a relative workspace", async () => {
  const item = await fixture({ cwd: "relative" });
  await assert.rejects(loadLaunch(item.launch), /must be absolute/);
});
