import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const harnessRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const labRoot = resolve(harnessRoot, "..");
const runRoot = resolve(harnessRoot, "workdir/v07");
const workspace = resolve(runRoot, "workspace");
const mission = await readFile(resolve(labRoot, "v2/workdir/v2/context/scenario.md"), "utf8");
await mkdir(runRoot, { recursive: true });
const promptPath = resolve(runRoot, "exec-prompt.md");
await writeFile(promptPath, `Frame one next worker outcome from this mandate and current seed.\n\n${mission}`);
const launchPath = resolve(runRoot, "exec-launch.json");
await writeFile(launchPath, `${JSON.stringify({
  schemaVersion: 1,
  sessionId: `v07-exec-${Date.now()}`,
  actor: { id: "exec", kind: "exec", role: "Loose-team Exec" },
  cwd: workspace,
  systemPromptPath: resolve(harnessRoot, "fixtures/game/loose-exec.md"),
  model: { provider: "openrouter", id: "nvidia/nemotron-3-super-120b-a12b:free", reasoning: "medium" },
  tools: ["read"],
  writeScope: "none",
  limits: { maxTurns: 5, timeoutMs: 180_000, commandTimeoutMs: 10_000, maxOutputBytes: 40_000 },
  eventLog: resolve(runRoot, "exec-events.jsonl"),
}, null, 2)}\n`);
await writeFile(resolve(runRoot, "input.json"), `${JSON.stringify({
  version: "v07",
  mode: "loose_team",
  seed: "514b7b3d0a65e093af608b08ca142344412181f4",
  execModel: "nvidia/nemotron-3-super-120b-a12b:free",
  workerModel: "poolside/laguna-s-2.1:free",
  missionSha256: createHash("sha256").update(mission).digest("hex"),
}, null, 2)}\n`);
process.stdout.write(`${JSON.stringify({ runRoot, workspace, promptPath, launchPath })}\n`);
