import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const tag = (process.env.PROBE_TAG ?? "probe").replace(/[^a-zA-Z0-9_.-]/g, "-");
const output = resolve(root, `.generated/${tag}.launch.json`);
await mkdir(dirname(output), { recursive: true });
const launch = {
  schemaVersion: 1,
  sessionId: `${tag}-${Date.now()}`,
  actor: { id: "exec", kind: "exec", role: "Harness conformance probe" },
  cwd: resolve(root, "fixtures/probe/workspace"),
  systemPromptPath: resolve(root, "fixtures/probe/system.md"),
  model: {
    provider: "openrouter",
    id: process.env.PROBE_MODEL ?? "nvidia/nemotron-3.5-lightning:free",
    reasoning: "low",
  },
  tools: ["read"],
  writeScope: "none",
  limits: { maxTurns: Number(process.env.PROBE_MAX_TURNS ?? 4), timeoutMs: 120_000, commandTimeoutMs: 10_000, maxOutputBytes: 32_000 },
  eventLog: resolve(root, `.generated/${tag}.events.jsonl`),
};
await writeFile(output, `${JSON.stringify(launch, null, 2)}\n`);
process.stdout.write(`${output}\n`);
