// One bounded, live, reply-only comparison for Sprint 109 D2. This is not an
// application end-to-end test or a production/provider-invoice savings claim.
// Run with the existing local OMP broker: node scripts/measure-owner-replies.mjs --run
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync, spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { join } from "node:path";

const root = fileURLToPath(new URL("../", import.meta.url));
assert.deepEqual(
  process.argv.slice(2),
  ["--run"],
  "Pass --run explicitly: this comparison makes 14 live model requests.",
);
const output = "/tmp/s109_ai_bench_test";
const profile = "s109-ai-bench";
const model = "gpt-5.6-sol";
const fixture = JSON.parse(
  await readFile(
    new URL(
      "../docs/sprints/sprint-109/evidence/owner-replies-seven-cases.json",
      import.meta.url,
    ),
    "utf8",
  ),
);
const old = execFileSync(
  "git",
  ["show", "811519c6^:crates/restless-engine/prompts/converse-with-owner.md"],
  { cwd: root, encoding: "utf8" },
);
const current = await readFile(
  join(root, "crates/restless-engine/prompts/converse-with-owner.md"),
  "utf8",
);
const token = (
  await readFile(
    "/home/yail/.omp/profiles/restless-model-broker/auth-broker.token",
    "utf8",
  )
).trim();
const env = {
  ...process.env,
  OMP_PROFILE: profile,
  OMP_AUTH_BROKER_URL: "http://127.0.0.1:7789",
  OMP_AUTH_BROKER_TOKEN: token,
};
await mkdir(output, { recursive: true, mode: 0o700 });
const sha = (text) => createHash("sha256").update(text).digest("hex");
const common = `You are Reno's Exec. This is a reply-only evaluation, with no tools or outside effects.
The supplied observations and completed tool receipts are frozen fixture facts; do not invent more actions.
Internal document edits and recoverable archiving are allowed. New-recipient email and public deployment require explicit approval.
As in the production context contract: when the owner wrote, make your final assistant response the complete reply.
Then end the response with exactly one machine-readable line in the existing hidden routing format:
<!--restless-intent:{"kind":"conversation|work_feedback|direction|authority","summary":"short takeaway addressed to you"}-->
Optional outcome, nextStep and ownerNeed describe actual facts only; they never grant approval.
ownerNeedHandoff is an optional exact existing handoff UUID for the same decision. It is never a URL; omit it when no exact UUID is supplied.
`;
const report = {
  scope: fixture.scope,
  route: { provider: "openai-codex", model, thinking: "medium" },
  qualification:
    "Exact route admitted live separately; tools, extensions, skills, title generation, session persistence and automatic fallback are disabled.",
  baseline: { ref: "811519c6^", sha256: sha(old), chars: old.length },
  current: {
    ref: execFileSync("git", ["rev-parse", "HEAD"], {
      cwd: root,
      encoding: "utf8",
    }).trim(),
    sha256: sha(current),
    chars: current.length,
  },
  fixtureSha256: sha(JSON.stringify(fixture)),
  startedAt: new Date().toISOString(),
  results: [],
};

async function run(item, variant) {
  const system = common + "\n" + (variant === "before" ? old : current);
  const prompt = `# Authoritative fixture observations and completed tool receipts\n${item.facts}\n\n# Last message from you\n${item.message}`;
  const args = [
    "--provider=openai-codex",
    "--model=" + model,
    "--thinking=medium",
    "--mode=json",
    "--print",
    "--no-tools",
    "--no-lsp",
    "--no-session",
    "--no-title",
    "--no-extensions",
    "--no-skills",
    "--no-rules",
    "--no-prewalk",
    "--config=" + join(root, "crates/restless-engine/omp-runtime.yml"),
    "--max-time=120",
    "--system-prompt=" + system,
    prompt,
  ];
  const started = Date.now();
  const child = spawn(join(root, "infra/host-tools/omp-source"), args, {
    cwd: output,
    env,
    detached: true,
    stdio: ["ignore", "pipe", "pipe"],
  });
  const stdout = [],
    stderr = [];
  child.stdout.on("data", (b) => stdout.push(b));
  child.stderr.on("data", (b) => stderr.push(b));
  const timer = setTimeout(() => {
    try {
      process.kill(-child.pid, "SIGTERM");
    } catch {}
  }, 135000);
  let status;
  try {
    status = await new Promise((resolve, reject) => {
      child.on("error", reject);
      child.on("exit", resolve);
    });
  } finally {
    clearTimeout(timer);
  }
  const events = Buffer.concat(stdout)
    .toString()
    .split("\n")
    .flatMap((line) => {
      try {
        return [JSON.parse(line)];
      } catch {
        return [];
      }
    });
  const messages = events
    .filter((e) => e.type === "message_end" && e.message?.role === "assistant")
    .map((e) => e.message);
  await writeFile(
    join(output, `${item.id}-${variant}.stderr`),
    Buffer.concat(stderr),
    { mode: 0o600 },
  );
  assert.equal(
    status,
    0,
    `${item.id}/${variant}: process failed; inspect the private stderr`,
  );
  assert.equal(
    messages.length,
    1,
    `${item.id}/${variant}: must be exactly one assistant completion`,
  );
  const message = messages[0];
  assert.equal(message.provider, "openai-codex");
  assert.equal(message.model, model);
  assert.equal(message.stopReason, "stop");
  const text = message.content
    .filter((c) => c.type === "text")
    .map((c) => c.text)
    .join("");
  await writeFile(
    join(output, `${item.id}-${variant}.completion.json`),
    JSON.stringify(
      {
        text,
        usage: message.usage,
        model: message.model,
        provider: message.provider,
      },
      null,
      2,
    ) + "\n",
    { mode: 0o600 },
  );
  const markers = [...text.matchAll(/<!--restless-intent:([\s\S]*?)-->/g)];
  const protocolErrors = [];
  if (markers.length !== 1)
    protocolErrors.push("Expected exactly one routing marker");
  let marker = null;
  if (markers.length === 1) {
    try {
      marker = JSON.parse(markers[0][1]);
    } catch {
      protocolErrors.push("Invalid routing JSON");
    }
  }
  if (
    marker &&
    !["conversation", "work_feedback", "direction", "authority"].includes(
      marker.kind,
    )
  )
    protocolErrors.push("Invalid routing kind");
  if (
    marker?.ownerNeedHandoff !== undefined &&
    marker.ownerNeedHandoff !== item.handoffId
  )
    protocolErrors.push("Handoff does not match the supplied exact UUID");
  const visible = text.replace(/<!--restless-intent:[\s\S]*?-->/g, "").trim();
  const result = {
    case: item.id,
    variant,
    ms: Date.now() - started,
    visible,
    marker,
    protocolErrors,
    visibleWords: visible.match(/\S+/g)?.length ?? 0,
    usage: message.usage,
    qualityRubric: item.quality,
  };
  report.results.push(result);
  await writeFile(
    join(output, "results.json"),
    JSON.stringify(report, null, 2) + "\n",
    { mode: 0o600 },
  );
  console.log(
    JSON.stringify({
      case: item.id,
      variant,
      ms: result.ms,
      words: result.visibleWords,
      protocolErrors,
      usage: message.usage,
    }),
  );
}
for (const [index, item] of fixture.cases.entries()) {
  for (const variant of index % 2 === 0
    ? ["before", "after"]
    : ["after", "before"])
    await run(item, variant);
}
report.finishedAt = new Date().toISOString();
await writeFile(
  join(output, "results.json"),
  JSON.stringify(report, null, 2) + "\n",
  { mode: 0o600 },
);
console.log(
  `Finished ${report.results.length} live completions; independently review each reply against its frozen rubric before drawing quality conclusions.`,
);
if (report.results.some((r) => r.protocolErrors.length)) process.exitCode = 1;
