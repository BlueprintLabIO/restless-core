// Actual Restless job admission and host enforcement on an explicitly owned temporary slice.
// Shrinks only that slice, exercises memory and CPU separately, and restores its original limits.

import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { readFile, writeFile } from "node:fs/promises";
const run = promisify(execFile);
const company = process.argv[2],
  namespace = process.env.RESTLESS_RESOURCE_NAMESPACE;
assert(
  /^[a-z0-9]+$/.test(namespace ?? "") &&
    /^[a-z0-9_]+_test$/.test(company ?? ""),
  "Use an explicitly owned test namespace and company",
);
const slice = "restless-" + namespace + ".slice",
  root = "/sys/fs/cgroup/restless.slice/" + slice;
assert.equal(
  process.env.RESTLESS_RESOURCE_TEST_SLICE,
  slice,
  "A temporary test-owned slice is required",
);
assert.notEqual(
  slice,
  "restless-workloads.slice",
  "Never change the production pool",
);
const origin = new URL(process.env.RESTLESS_SMOKE_ORIGIN);
assert(
  origin.protocol === "http:" && origin.hostname === "127.0.0.1",
  "Use the isolated local owner API",
);
const co = "restless-" + namespace + "-co-" + company;
const output = process.env.RESTLESS_VERIFY_OUTPUT;
assert(output, "Choose a report file");
// Refuse to shrink a slice that contains any other fixture or owner's workload.
const names = (await run("docker", ["ps", "--format", "{{.Names}}"])).stdout
  .trim()
  .split("\n")
  .filter(Boolean);
for (const name of names) {
  const parent = (
    await run("docker", ["inspect", "-f", "{{.HostConfig.CgroupParent}}", name])
  ).stdout.trim();
  if (parent === slice)
    assert(
      [co, "restless-" + namespace + "-pc-" + company].includes(name),
      "Another workload shares this test slice",
    );
}
const unit = (
  await run("systemctl", ["show", slice, "--property=FragmentPath", "--value"])
).stdout.trim();
assert.equal(
  unit,
  "/run/systemd/system/" + slice,
  "Only a temporary runtime unit may be stressed",
);
const jobs = [];
const execute = async (cmd, args, timeout = 30000) =>
  (await run(cmd, args, { timeout, maxBuffer: 2e6 })).stdout;
const docker = async (args) => execute("docker", args);
const set = async (memory, high, cpu) =>
  execute("sudo", [
    "-n",
    "systemctl",
    "set-property",
    "--runtime",
    slice,
    "MemoryMax=" + memory,
    "MemoryHigh=" + high,
    "CPUQuota=" + cpu,
  ]);
const usage = async () => {
  const r = await fetch(
    new URL("/api/companies/" + company + "/company/compute", origin),
    { signal: AbortSignal.timeout(15000) },
  );
  assert.equal(r.status, 200);
  return r.json();
};
const health = async () => {
  const r = await fetch(new URL("/health", origin), {
    signal: AbortSignal.timeout(5000),
  });
  assert.equal(r.status, 200);
};
const original = {
  memory: (await readFile(root + "/memory.max", "utf8")).trim(),
  high: (await readFile(root + "/memory.high", "utf8")).trim(),
  cpu: (await readFile(root + "/cpu.max", "utf8")).trim(),
};
const cpu = async () =>
  Number(
    (await readFile(root + "/cpu.stat", "utf8")).match(
      /^usage_usec (\d+)$/m,
    )[1],
  );
const events = async () =>
  Object.fromEntries(
    (await readFile(root + "/memory.events", "utf8"))
      .trim()
      .split("\n")
      .map((x) => {
        const [k, v] = x.split(" ");
        return [k, Number(v)];
      }),
  );
try {
  const before = await usage();
  assert.equal(before.status, "available");
  assert.equal(before.sample.placement_current, true);
  await set("768M", "691M", "50%");
  const program =
    "const memory=Buffer.alloc(320*1024*1024,1); setInterval(()=>memory[0]++,1000)";
  const initialEvents = await events();
  for (let i = 0; i < 2; i++) {
    const result = JSON.parse(
      await docker([
        "exec",
        "-u",
        "company",
        co,
        "restless",
        "job",
        "run",
        "--cpus",
        "1",
        "--memory",
        "512m",
        "--timeout",
        "25",
        "--",
        "node -e " + JSON.stringify(program),
      ]),
    );
    assert.match(result.job, /^[a-f0-9]+$/);
    jobs.push(co + "-job-" + result.job);
    const parent = (
      await docker([
        "inspect",
        "-f",
        "{{.HostConfig.CgroupParent}}",
        jobs.at(-1),
      ])
    ).trim();
    assert.equal(parent, slice);
  }
  const at = performance.now();
  const controls = [];
  for (let i = 0; i < 8; i++) {
    await health();
    controls.push(true);
    await new Promise((r) => setTimeout(r, 500));
  }

  const during = await usage();
  assert.equal(during.status, "available");
  assert.equal(during.sample.pool.memory_limit_bytes, 768 * 1024 * 1024);
  assert.equal(during.sample.pool.cpu_limit_cores, 0.5);
  const memory = Number(
    (await readFile(root + "/memory.current", "utf8")).trim(),
  );
  assert(memory <= 768 * 1024 * 1024);
  const endEvents = await events();
  assert(
    endEvents.high > initialEvents.high ||
      endEvents.oom_kill > initialEvents.oom_kill,
    "actual shared memory pressure",
  );
  assert(
    during.sample.work.memory_limit_bytes <= 768 * 1024 * 1024,
    "company combined ceilings respect pool",
  );
  await Promise.all(jobs.map((name) => docker(["rm", "-f", name])));
  jobs.length = 0;
  for (let i = 0; i < 2; i++) {
    const result = JSON.parse(
      await docker([
        "exec",
        "-u",
        "company",
        co,
        "restless",
        "job",
        "run",
        "--cpus",
        "1",
        "--memory",
        "128m",
        "--timeout",
        "15",
        "--",
        'node -e "while(true) Math.sqrt(Math.random())"',
      ]),
    );
    jobs.push(co + "-job-" + result.job);
  }
  const cpuBefore = await cpu(),
    cpuAt = performance.now();
  for (let i = 0; i < 8; i++) {
    await health();
    controls.push(true);
    await new Promise((r) => setTimeout(r, 500));
  }
  const seconds = (performance.now() - cpuAt) / 1000,
    used = ((await cpu()) - cpuBefore) / 1e6;
  assert(used > seconds * 0.15, "actual busy jobs");
  assert(used < seconds * 0.7 + 0.15, "shared CPU limit");
  const report = {
    actualRestlessJobs: 4,
    placement: "shared parent",
    capBytes: 768 * 1024 * 1024,
    memoryBytes: memory,
    memoryHighEvents: endEvents.high - initialEvents.high,
    oomKills: endEvents.oom_kill - initialEvents.oom_kill,
    cpuQuota: 0.5,
    cpuSeconds: used,
    wallSeconds: seconds,
    ownerHealthChecks: controls.length,
    computeApi: "available",
  };
  await writeFile(output, JSON.stringify(report, null, 2) + "\n");
  console.log(JSON.stringify(report));
} finally {
  await Promise.allSettled(jobs.map((name) => docker(["rm", "-f", name])));
  const [quota, period] = original.cpu.split(" ").map(Number);
  await set(original.memory, original.high, (100 * quota) / period + "%");
  await health();
}
