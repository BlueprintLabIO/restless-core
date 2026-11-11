import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const harnessRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const labRoot = resolve(harnessRoot, "..");
const version = process.argv[2];
const model = process.argv[3];
if (!version || !/^v\d\d$/.test(version) || !model) {
  throw new Error("usage: prepare-game-run <vNN> <model-id>");
}
const runRoot = resolve(harnessRoot, `workdir/${version}`);
const workspace = resolve(runRoot, "workspace");
const missionPath = resolve(labRoot, "v2/workdir/v2/context/scenario.md");
const mission = await readFile(missionPath, "utf8");
const focus = [
  "# Bounded experiment instruction",
  "",
  "Make one material, coherent, playable improvement toward this mandate in the available turn envelope.",
  "The existing seed already has Sunleaf exploration, 12 creatures, combat, bonding, and evolution.",
  "Prefer one missing loop link that can be implemented and independently checked now.",
  "",
  mission,
].join("\n");
await mkdir(runRoot, { recursive: true });
const promptPath = resolve(runRoot, "prompt.md");
await writeFile(promptPath, focus);
const launchPath = resolve(runRoot, "launch.json");
const launch = {
  schemaVersion: 1,
  sessionId: `${version}-single-${Date.now()}`,
  actor: { id: "single-agent", kind: "staff", role: "Single-agent comparison baseline" },
  cwd: workspace,
  systemPromptPath: resolve(harnessRoot, "fixtures/game/single-agent.md"),
  model: { provider: "openrouter", id: model, reasoning: "medium" },
  tools: ["read", "write", "edit", "run"],
  writeScope: "workspace",
  limits: { maxTurns: 10, timeoutMs: 300_000, commandTimeoutMs: 90_000, maxOutputBytes: 80_000 },
  eventLog: resolve(runRoot, "events.jsonl"),
};
await writeFile(launchPath, `${JSON.stringify(launch, null, 2)}\n`);
await writeFile(resolve(runRoot, "input.json"), `${JSON.stringify({
  version,
  mode: "single_agent",
  model,
  seed: "514b7b3d0a65e093af608b08ca142344412181f4",
  missionSha256: createHash("sha256").update(mission).digest("hex"),
}, null, 2)}\n`);
process.stdout.write(`${JSON.stringify({ launchPath, promptPath, runRoot, workspace })}\n`);
