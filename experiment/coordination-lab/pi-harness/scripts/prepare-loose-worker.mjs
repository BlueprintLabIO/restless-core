import { readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const harnessRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const runRoot = resolve(harnessRoot, "workdir/v07");
const workspace = resolve(runRoot, "workspace");
const execSummary = JSON.parse(await readFile(resolve(runRoot, "exec-summary.json"), "utf8"));
if (!execSummary.text?.trim()) throw new Error("Exec produced no worker brief");
const promptPath = resolve(runRoot, "worker-prompt.md");
await writeFile(promptPath, `# Exec brief\n\n${execSummary.text}\n\nExecute this outcome now.`);
const launchPath = resolve(runRoot, "worker-launch.json");
await writeFile(launchPath, `${JSON.stringify({
  schemaVersion: 1,
  sessionId: `v07-worker-${Date.now()}`,
  actor: { id: "gameplay-worker", kind: "staff", role: "Loose-team game worker" },
  cwd: workspace,
  systemPromptPath: resolve(harnessRoot, "fixtures/game/loose-worker.md"),
  model: { provider: "openrouter", id: "poolside/laguna-s-2.1:free", reasoning: "medium" },
  tools: ["read", "write", "edit", "run"],
  writeScope: "workspace",
  limits: { maxTurns: 14, timeoutMs: 360_000, commandTimeoutMs: 90_000, maxOutputBytes: 80_000 },
  eventLog: resolve(runRoot, "worker-events.jsonl"),
}, null, 2)}\n`);
process.stdout.write(`${JSON.stringify({ runRoot, promptPath, launchPath })}\n`);
