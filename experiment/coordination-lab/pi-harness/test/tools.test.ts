import assert from "node:assert/strict";
import { mkdtemp, mkdir, realpath, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { EventLog } from "../src/event-log.js";
import type { LaunchContract } from "../src/launch.js";
import { buildTools } from "../src/tools.js";

async function fixture() {
  const root = await mkdtemp(join(tmpdir(), "restless-tools-test-"));
  const workspace = join(root, "workspace");
  await mkdir(join(workspace, "src"), { recursive: true });
  await mkdir(join(workspace, ".git", "objects"), { recursive: true });
  await writeFile(join(workspace, "README.md"), "verified creature roster\n");
  await writeFile(join(workspace, "src", "game.js"), "const creature = 'Mistray';\n");
  await writeFile(join(root, "outside.txt"), "must remain invisible\n");
  const contract: LaunchContract = {
    schemaVersion: 1,
    sessionId: "tools-test",
    actor: { id: "exec", kind: "exec", role: "Exec" },
    cwd: await realpath(workspace),
    systemPromptPath: join(root, "system.md"),
    model: { provider: "openrouter", id: "test/free" },
    tools: ["list", "search"],
    writeScope: "none",
    limits: { maxTurns: 2, timeoutMs: 10_000, commandTimeoutMs: 1_000, maxOutputBytes: 8_000 },
  };
  const tools = buildTools(contract, new EventLog(undefined, "tools-test", "exec"));
  return { tools };
}

test("list and search perceive the workspace but reject escapes", async () => {
  const { tools } = await fixture();
  const list = tools.find((tool) => tool.name === "list")!;
  const search = tools.find((tool) => tool.name === "search")!;
  const listed = await list.execute("list", { path: ".", depth: 2 }, undefined);
  const listText = listed.content.map((item) => item.type === "text" ? item.text : "").join("\n");
  assert.match(listText, /README\.md/);
  assert.match(listText, /src\/game\.js/);
  assert.doesNotMatch(listText, /\.git/);
  assert.doesNotMatch(listText, /outside\.txt/);
  const found = await search.execute("search", { query: "Mistray", path: "." }, undefined);
  const searchText = found.content.map((item) => item.type === "text" ? item.text : "").join("\n");
  assert.match(searchText, /src\/game\.js:1/);
  await assert.rejects(list.execute("escape", { path: ".." }, undefined), /path escapes workspace/);
  await assert.rejects(search.execute("escape", { query: "invisible", path: ".." }, undefined), /path escapes workspace/);
});
