import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = resolve(root, ".generated/mcp.launch.json");
await mkdir(dirname(output), { recursive: true });
const launch = {
  schemaVersion: 1,
  sessionId: `mcp-${Date.now()}`,
  actor: { id: "exec", kind: "exec", role: "MCP adapter probe" },
  cwd: resolve(root, "fixtures/mcp/workspace"),
  systemPromptPath: resolve(root, "fixtures/mcp/system.md"),
  model: {
    provider: "openrouter",
    id: process.env.PROBE_MODEL ?? "cohere/north-mini-code:free",
    reasoning: "low",
  },
  tools: ["read"],
  mcpServers: [{
    name: "coordination",
    command: process.execPath,
    args: [resolve(root, "dist/fixture-mcp-server.js")],
    envNames: [],
  }],
  writeScope: "none",
  limits: { maxTurns: 4, timeoutMs: 120_000, commandTimeoutMs: 10_000, maxOutputBytes: 32_000 },
  eventLog: resolve(root, ".generated/mcp.events.jsonl"),
};
await writeFile(output, `${JSON.stringify(launch, null, 2)}\n`);
process.stdout.write(`${output}\n`);
