import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = resolve(root, ".generated/cancel.launch.json");
await mkdir(dirname(output), { recursive: true });
const launch = {
  schemaVersion: 1,
  sessionId: `cancel-${Date.now()}`,
  actor: { id: "probe-worker", kind: "staff", role: "Cancellation probe worker" },
  cwd: resolve(root, "fixtures/cancel/workspace"),
  systemPromptPath: resolve(root, "fixtures/cancel/system.md"),
  model: {
    provider: "openrouter",
    id: process.env.PROBE_MODEL ?? "nvidia/nemotron-3.5-lightning:free",
    reasoning: "low",
  },
  tools: ["run"],
  writeScope: "workspace",
  limits: { maxTurns: 3, timeoutMs: 60_000, commandTimeoutMs: 45_000, maxOutputBytes: 32_000 },
  eventLog: resolve(root, ".generated/cancel.events.jsonl"),
};
await writeFile(output, `${JSON.stringify(launch, null, 2)}\n`);
process.stdout.write(`${output}\n`);
