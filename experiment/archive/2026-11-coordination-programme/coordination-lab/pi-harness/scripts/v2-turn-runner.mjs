#!/usr/bin/env node
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { spawn } from "node:child_process";

const required = (name) => {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
};
const actor = required("COORD_ACTOR");
const turnId = required("COORD_TURN_ID");
const turnDir = required("COORD_TURN_DIR");
const promptPath = required("COORD_PROMPT_PATH");
const systemPath = required("COORD_HOST_SYSTEM_PATH");
const workspace = required("COORD_HOST_WORKDIR");
const mcpServer = required("COORD_HOST_MCP_SERVER");
const python = required("COORD_PYTHON");
const endpoint = required("COORD_HOST_ENDPOINT");
const model = required("COORD_MODEL");
const readOnly = process.env.COORD_READ_ONLY === "1";
const actorLimitKey = `COORD_MAX_TURNS_${actor.toUpperCase().replaceAll("-", "_")}`;
const configuredTurns = Number(process.env[actorLimitKey] ?? (actor === "exec" ? 7 : 18));
if (!Number.isInteger(configuredTurns) || configuredTurns < 1 || configuredTurns > 100) {
  throw new Error(`${actorLimitKey} must be an integer from 1 to 100`);
}
await mkdir(turnDir, { recursive: true });
const launchPath = resolve(turnDir, `${turnId}.launch.json`);
const summaryPath = resolve(turnDir, `${turnId}.summary.json`);
const eventPath = resolve(turnDir, `${turnId}.events.jsonl`);
const mcpEnv = {
  COORD_ENDPOINT: endpoint,
  COORD_ACTOR: actor,
  COORD_ATTEMPT: process.env.COORD_ATTEMPT ?? "",
  COORD_LEASE_TOKEN: process.env.COORD_LEASE_TOKEN ?? "",
};
const launch = {
  schemaVersion: 1,
  sessionId: turnId,
  actor: { id: actor, kind: process.env.COORD_ACTOR_KIND ?? (readOnly ? "exec" : "staff"), role: actor },
  cwd: workspace,
  systemPromptPath: systemPath,
  model: { provider: "openrouter", id: model, reasoning: "medium" },
  tools: readOnly ? [] : ["read", "list", "search", "write", "edit", "run"],
  mcpServers: [{
    name: "orgintel",
    command: python,
    args: [mcpServer],
    envNames: Object.keys(mcpEnv).sort(),
  }],
  writeScope: readOnly ? "none" : "workspace",
  limits: {
    maxTurns: configuredTurns,
    timeoutMs: readOnly ? 180_000 : 480_000,
    commandTimeoutMs: 120_000,
    maxOutputBytes: 100_000,
  },
  eventLog: eventPath,
};
await writeFile(launchPath, `${JSON.stringify(launch, null, 2)}\n`);
const client = resolve(new URL("../dist/probe-client.js", import.meta.url).pathname);
const child = spawn(process.execPath, [client, launchPath, `@${promptPath}`], {
  env: { ...process.env, ...mcpEnv, PROBE_OUTPUT: summaryPath },
  stdio: ["ignore", "pipe", "pipe"],
});
let stdout = "";
let stderr = "";
child.stdout.on("data", (chunk) => { stdout += chunk.toString("utf8"); });
child.stderr.on("data", (chunk) => { stderr += chunk.toString("utf8"); });
const exitCode = await new Promise((accept, reject) => {
  child.once("error", reject);
  child.once("close", accept);
});
if (exitCode !== 0) throw new Error(`ACP/Pi turn failed (${exitCode}): ${stderr.slice(-4000)}`);
const summary = JSON.parse(await readFile(summaryPath, "utf8"));
const runtime = summary.response?._meta?.["restless.dev/result"];
if (!runtime) throw new Error(`turn result lacks Restless metadata: ${stdout.slice(-2000)}`);
process.stdout.write(`${JSON.stringify({
  text: summary.text ?? "",
  // The comparison coordinator predates the Pi adapter and stores only the
  // list length. Preserve that interface without teaching the harness Work
  // semantics or exposing unbounded raw call payloads here.
  tool_calls: Array.from({ length: Number(summary.updateCounts?.tool_call ?? 0) }, () => ({})),
  cost_usd: runtime.usage?.cost ?? 0,
  used_tokens: Number(runtime.usage?.input ?? 0) + Number(runtime.usage?.cacheRead ?? 0),
  output_tokens: runtime.usage?.output ?? 0,
  stop_reason: runtime.outcome,
  model: runtime.model,
  error: runtime.error,
})}\n`);
