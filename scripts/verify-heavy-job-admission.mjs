// Real heavy-job admission under an explicitly owned temporary workload pool.
// The accompanying fixture owns the slice and plane; never run against a real account.
import assert from "node:assert/strict";
import { randomBytes } from "node:crypto";
import { execFile } from "node:child_process";
import { readFile, writeFile, access, unlink } from "node:fs/promises";
import { promisify } from "node:util";
import { createConnection } from "node:net";

const run = promisify(execFile);
const company = process.env.RESTLESS_SMOKE_COMPANY;
const namespace = process.env.RESTLESS_RESOURCE_NAMESPACE;
const peer = process.env.RESTLESS_SMOKE_PEER_COMPANY;
const slice = process.env.RESTLESS_RESOURCE_TEST_SLICE;
const origin = new URL(process.env.RESTLESS_SMOKE_ORIGIN);
assert(/^[a-z0-9_]+_test$/.test(company ?? ""));
assert(/^[a-z0-9_]+_test$/.test(namespace ?? ""));
assert(/^[a-z0-9_]+_test$/.test(peer ?? "") && peer !== company);
assert(/^restless-s109job[a-f0-9]+\.slice$/.test(slice ?? ""));
assert(origin.hostname === "127.0.0.1" && origin.protocol === "http:");
const root = "/sys/fs/cgroup/restless.slice/" + slice;
const unit = (
  await run("systemctl", ["show", slice, "--property=FragmentPath", "--value"])
).stdout.trim();
assert.equal(unit, "/run/systemd/system/" + slice);
const co = "restless-" + namespace + "-co-" + company;
const peerCo = "restless-" + namespace + "-co-" + peer;
const parent = (
  await run("docker", ["inspect", "-f", "{{.HostConfig.CgroupParent}}", co])
).stdout.trim();
assert.equal(parent, slice);
assert.equal(
  (
    await run("docker", [
      "inspect",
      "-f",
      "{{.HostConfig.CgroupParent}}",
      peerCo,
    ])
  ).stdout.trim(),
  slice,
);
const savedHigh = (await readFile(root + "/memory.high", "utf8")).trim();
const setHigh = async (value) =>
  run("sudo", [
    "-n",
    "systemctl",
    "set-property",
    "--runtime",
    slice,
    "MemoryHigh=" + value,
  ]);
const getReading = async () => {
  const response = await fetch(
    new URL("/api/companies/" + company + "/company/compute", origin),
  );
  assert.equal(response.status, 200);
  return response.json();
};
const docker = async (args) =>
  (await run("docker", args, { timeout: 30000 })).stdout.trim();
let jobName;
let legacyName;
const concurrentNames = [];
let pausedName;
const queueFixtures = [];
const marker = "/company/run/s109-heavy-job-" + namespace;
try {
  // A job larger than the hard account ceiling can never make progress. Refuse
  // it before creating an intent, with a useful explanation rather than a silent queue.
  let oversized;
  try {
    await docker([
      "exec",
      "-u",
      "company",
      co,
      "restless",
      "job",
      "run",
      "--memory",
      "32g",
      "--timeout",
      "30",
      "--",
      "sleep 3",
    ]);
  } catch (error) {
    oversized = error;
  }
  assert(oversized, "a job above the hard account ceiling must be refused");
  assert.match(
    String(oversized.stderr) + String(oversized.stdout),
    /more memory than the account can admit/,
  );
  const existing = await docker([
    "ps",
    "-a",
    "--filter",
    "label=restless.job=1",
    "--format",
    "{{.Names}}",
  ]);
  assert(
    !existing.split("\n").some((name) => name.startsWith(co + "-job-")),
    "oversized request must leave no job",
  );
  // Simulate a container stranded in created state by an older release. It has
  // the old labels but no queue marker; recovery must never start its command.
  legacyName = co + "-job-" + randomBytes(6).toString("hex");
  await docker([
    "create",
    "--name",
    legacyName,
    "--label",
    "restless.job=1",
    "--label",
    "restless.company=" + company,
    "--entrypoint",
    "sh",
    process.env.RESTLESS_COMPANY_IMAGE ??
      "restless-company-image:release-connections",
    "-c",
    "exit 0",
  ]);
  if (process.env.RESTLESS_JOB_RESTART_DIR)
    await writeFile(
      process.env.RESTLESS_JOB_RESTART_DIR + "/legacy.json",
      JSON.stringify({ legacyName }),
    );
  const maximum = Number((await readFile(root + "/memory.max", "utf8")).trim());
  const baseline = await getReading();
  const working = baseline.sample.work_admission.working_set_bytes;
  const high = working + Math.min(128 * 1024 * 1024, Math.floor(working / 2));
  assert(working > 0 && high < maximum);
  // Set the application reserve below the working set while retaining headroom
  // above it. Reclaimable file cache can exceed this temporary high threshold.
  await setHigh(String(high));
  let sample = await getReading();
  assert(
    sample.sample.work_admission.waiting_for_memory,
    "fixture must really be under admission pressure",
  );
  for (const name of [company, peer]) {
    for (const path of [
      "/health",
      "/api/companies/" + name + "/principal",
      "/api/companies/" + name + "/attention",
      "/api/companies/" + name + "/work-graph",
    ]) {
      const response = await fetch(new URL(path, origin), {
        signal: AbortSignal.timeout(10000),
      });
      assert.equal(
        response.status,
        200,
        "owner entry remains available under account pressure: " + path,
      );
    }
  }
  // Fill all but one queue slot, then race two real requests. The shared
  // authority lock must allow only one to claim the last slot.
  for (let i = 0; i < 63; i++) {
    const name = co + "-job-" + randomBytes(6).toString("hex");
    await docker([
      "create",
      "--name",
      name,
      "--label",
      "restless.job=1",
      "--label",
      "restless.job.queue=1",
      "--label",
      "restless.job.scope=namespace:" + namespace,
      "--label",
      "restless.company=" + company,
      "--memory",
      "128m",
      "--cgroup-parent",
      slice,
      "--entrypoint",
      "sh",
      process.env.RESTLESS_COMPANY_IMAGE ??
        "restless-company-image:release-connections",
      "-c",
      "exit 0",
    ]);
    queueFixtures.push(name);
    if (process.env.RESTLESS_JOB_RESTART_DIR)
      await writeFile(
        process.env.RESTLESS_JOB_RESTART_DIR + "/queue.json",
        JSON.stringify(queueFixtures),
      );
  }
  const submitLast = () =>
    docker([
      "exec",
      "-u",
      "company",
      co,
      "restless",
      "job",
      "run",
      "--cpus",
      "0.2",
      "--memory",
      "128m",
      "--timeout",
      "30",
      "--",
      "printf x >> " + marker,
    ]);
  const race = await Promise.allSettled([submitLast(), submitLast()]);
  const accepted = race.filter((result) => result.status === "fulfilled");
  const refused = race.filter((result) => result.status === "rejected");
  for (const item of accepted) {
    const id = JSON.parse(item.value).job;
    assert.match(id, /^[a-f0-9]{12}$/);
    concurrentNames.push(co + "-job-" + id);
  }
  if (process.env.RESTLESS_JOB_RESTART_DIR)
    await writeFile(
      process.env.RESTLESS_JOB_RESTART_DIR + "/queue.json",
      JSON.stringify([...queueFixtures, ...concurrentNames]),
    );
  assert.equal(
    accepted.length,
    1,
    "only one request may claim the last queue slot",
  );
  assert.equal(refused.length, 1);
  assert.match(
    String(refused[0].reason.stderr) + String(refused[0].reason.stdout),
    /job queue is full/i,
  );
  const launched = JSON.parse(accepted[0].value);
  assert.match(launched.job, /^[a-f0-9]{12}$/);
  for (const name of queueFixtures) await docker(["rm", "-f", name]);
  queueFixtures.length = 0;
  if (process.env.RESTLESS_JOB_RESTART_DIR)
    await writeFile(
      process.env.RESTLESS_JOB_RESTART_DIR + "/queue.json",
      JSON.stringify(concurrentNames),
    );
  jobName = co + "-job-" + launched.job;
  concurrentNames.length = 0;
  const state = await docker(["inspect", "-f", "{{.State.Status}}", jobName]);
  const status = JSON.parse(
    await docker([
      "exec",
      "-u",
      "company",
      co,
      "restless",
      "job",
      "status",
      launched.job,
    ]),
  );
  assert.equal(
    status.status,
    "waiting_for_capacity",
    "a new job must wait while its account lacks capacity",
  );
  assert.notEqual(state, "running", "waiting must not start the job container");
  assert(status.reason, "the waiting reason must be visible");
  if (process.env.RESTLESS_JOB_RESTART_DIR) {
    const flag = process.env.RESTLESS_JOB_RESTART_DIR + "/docker-fail-inspect";
    await writeFile(flag, jobName);
    try {
      let unavailable;
      try {
        await docker([
          "exec",
          "-u",
          "company",
          co,
          "restless",
          "job",
          "status",
          launched.job,
        ]);
      } catch (error) {
        unavailable = error;
      }
      assert(
        unavailable,
        "an inspect failure must not be reported as a missing job",
      );
      assert.match(
        String(unavailable.stderr) + String(unavailable.stdout),
        /temporarily unavailable/i,
      );
    } finally {
      await unlink(flag);
    }
  }
  if (process.env.RESTLESS_JOB_RESTART_DIR) {
    const directory = process.env.RESTLESS_JOB_RESTART_DIR;
    await writeFile(
      directory + "/ready.json",
      JSON.stringify({ jobName, legacyName }),
    );
    let resumed = false;
    for (let i = 0; i < 360; i++) {
      if (
        await access(directory + "/resumed").then(
          () => true,
          () => false,
        )
      ) {
        resumed = true;
        break;
      }
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
    assert(resumed, "the isolated daemon restart must complete");
  }
  if (!process.env.RESTLESS_JOB_RESTART_DIR) {
    const before = await run("docker", [
      "exec",
      "-u",
      "company",
      co,
      "test",
      "-e",
      marker,
    ]).then(
      () => true,
      () => false,
    );
    assert.equal(
      before,
      false,
      "waiting job must have no effect before admission",
    );
    await setHigh(savedHigh);
  }
  let final;
  for (let attempt = 0; attempt < 80; attempt++) {
    await new Promise((resolve) => setTimeout(resolve, 500));
    final = JSON.parse(
      await docker([
        "exec",
        "-u",
        "company",
        co,
        "restless",
        "job",
        "status",
        launched.job,
      ]),
    );
    if (["succeeded", "failed", "timed_out", "gone"].includes(final.status))
      break;
  }
  assert.equal(
    final?.status,
    "succeeded",
    "the waiting job must resume and finish after capacity returns",
  );
  await new Promise((resolve) => setTimeout(resolve, 6000));
  assert.equal(
    await docker(["exec", "-u", "company", co, "cat", marker]),
    "x",
    "job effect occurs exactly once",
  );
  assert.equal(
    await docker(["inspect", "-f", "{{.State.Status}}", legacyName]),
    "created",
    "legacy created containers must never be started",
  );

  // Give the pool room for one 256 MiB job, but not two concurrent reservations.
  const stable = await getReading();
  const secondHigh =
    stable.sample.work_admission.working_set_bytes +
    256 * 1024 * 1024 +
    256 * 1024 * 1024 +
    96 * 1024 * 1024;
  assert(
    secondHigh < maximum,
    "isolated pool must have room for the concurrency check",
  );
  await setHigh(String(secondHigh));
  const submit = async (target) => ({
    ...JSON.parse(
      await docker([
        "exec",
        "-u",
        "company",
        target,
        "restless",
        "job",
        "run",
        "--cpus",
        "0.2",
        "--memory",
        "256m",
        "--timeout",
        "40",
        "--",
        "sleep 14",
      ]),
    ),
    target,
  });
  const concurrent = await Promise.all([submit(co), submit(peerCo)]);
  for (const item of concurrent) {
    assert.match(item.job, /^[a-f0-9]{12}$/);
    concurrentNames.push(item.target + "-job-" + item.job);
  }
  if (process.env.RESTLESS_JOB_RESTART_DIR)
    await writeFile(
      process.env.RESTLESS_JOB_RESTART_DIR + "/queue.json",
      JSON.stringify(concurrentNames),
    );
  await new Promise((resolve) => setTimeout(resolve, 8000));
  const states = await Promise.all(
    concurrentNames.map((name) =>
      docker(["inspect", "-f", "{{.State.Status}}", name]),
    ),
  );
  assert.deepEqual(
    states.toSorted(),
    ["created", "running"],
    "account capacity admits one reserved job at a time",
  );
  pausedName = concurrentNames[states.indexOf("running")];
  await docker(["pause", pausedName]);
  await new Promise((resolve) => setTimeout(resolve, 6500));
  const pausedStates = await Promise.all(
    concurrentNames.map((name) =>
      docker(["inspect", "-f", "{{.State.Status}}", name]),
    ),
  );
  assert.deepEqual(
    pausedStates.toSorted(),
    ["created", "paused"],
    "a paused job keeps its memory reservation",
  );
  await docker(["unpause", pausedName]);
  pausedName = undefined;
  const completed = new Set();
  for (
    let attempt = 0;
    attempt < 100 && completed.size < concurrent.length;
    attempt++
  ) {
    for (const item of concurrent)
      if (!completed.has(item.job)) {
        const current = JSON.parse(
          await docker([
            "exec",
            "-u",
            "company",
            item.target,
            "restless",
            "job",
            "status",
            item.job,
          ]),
        );
        if (current.status === "succeeded") completed.add(item.job);
        else
          assert(
            !["failed", "timed_out", "gone"].includes(current.status),
            JSON.stringify(current),
          );
      }
    if (completed.size < concurrent.length)
      await new Promise((resolve) => setTimeout(resolve, 500));
  }
  assert.equal(
    completed.size,
    2,
    "both jobs must finish as capacity becomes available",
  );
  // Docker can accept a create and lose its reply. The unconfirmed name must
  // remain inert even if the existing scheduler is explicitly woken.
  if (process.env.RESTLESS_JOB_RESTART_DIR) {
    const directory = process.env.RESTLESS_JOB_RESTART_DIR;
    const delayFlag = directory + "/docker-delay-create";
    const acceptedFlag = directory + "/docker-create-accepted";
    const timeoutMarker = "/company/run/s109-timeout-" + namespace;
    await setHigh(savedHigh);
    await writeFile(delayFlag, "1");
    try {
      const attempt = run(
        "docker",
        [
          "exec",
          "-u",
          "company",
          co,
          "restless",
          "job",
          "run",
          "--memory",
          "128m",
          "--timeout",
          "30",
          "--",
          "printf x > " + timeoutMarker,
        ],
        { timeout: 90000 },
      ).then(
        (value) => ({ value }),
        (error) => ({ error }),
      );
      let acceptedCreate = false;
      for (let i = 0; i < 80; i++) {
        if (
          await access(acceptedFlag).then(
            () => true,
            () => false,
          )
        ) {
          acceptedCreate = true;
          break;
        }
        await new Promise((resolve) => setTimeout(resolve, 250));
      }
      assert(
        acceptedCreate,
        "the test Docker create must actually be accepted",
      );
      const names = (
        await docker([
          "ps",
          "-a",
          "--filter",
          "label=restless.job.scope=namespace:" + namespace,
          "--format",
          "{{.Names}}",
        ])
      )
        .split("\n")
        .filter(
          (name) =>
            name.startsWith(co + "-job-") && name.endsWith("-preparing"),
        );
      assert.equal(
        names.length,
        1,
        "the unconfirmed job is held under its inert name",
      );
      const staged = names[0];
      concurrentNames.push(staged);
      await writeFile(
        directory + "/queue.json",
        JSON.stringify(concurrentNames),
      );
      const wake = await new Promise((resolve, reject) => {
        const socket = createConnection(directory + "/plane/restlessd.sock");
        let received = "";
        socket.setTimeout(5000, () =>
          socket.destroy(new Error("schedule wake timed out")),
        );
        socket.once("error", reject);
        socket.once("connect", () =>
          socket.write(
            JSON.stringify({ cmd: "schedule-wake", principal: "owner" }) + "\n",
          ),
        );
        socket.on("data", (data) => {
          received += data.toString();
          if (received.includes("\n")) {
            socket.end();
            resolve(JSON.parse(received.split("\n")[0]));
          }
        });
      });
      assert.equal(wake.ok, true);
      await new Promise((resolve) => setTimeout(resolve, 7000));
      assert.equal(
        await docker(["inspect", "-f", "{{.State.Status}}", staged]),
        "created",
      );
      assert.equal(
        await run("docker", [
          "exec",
          "-u",
          "company",
          co,
          "test",
          "-e",
          timeoutMarker,
        ]).then(
          () => true,
          () => false,
        ),
        false,
      );
      const result = await attempt;
      assert(result.error, "the delayed create confirmation must time out");
      assert.match(
        String(result.error.stderr) + String(result.error.stdout),
        /preparation was not confirmed/i,
      );
      await new Promise((resolve) => setTimeout(resolve, 6000));
      assert.equal(
        await docker(["inspect", "-f", "{{.State.Status}}", staged]),
        "created",
      );
      assert.equal(
        await run("docker", [
          "exec",
          "-u",
          "company",
          co,
          "test",
          "-e",
          timeoutMarker,
        ]).then(
          () => true,
          () => false,
        ),
        false,
      );
      await docker(["rm", "-f", staged]);
    } finally {
      await unlink(delayFlag).catch(() => {});
      await unlink(acceptedFlag).catch(() => {});
      await run("docker", [
        "exec",
        "-u",
        "company",
        co,
        "rm",
        "-f",
        timeoutMarker,
      ]).catch(() => {});
    }
  }
  if (process.env.RESTLESS_JOB_RESTART_DIR) {
    const directory = process.env.RESTLESS_JOB_RESTART_DIR;
    const renameFlag = directory + "/docker-fail-rename";
    const uncertainMarker = "/company/run/s109-rename-" + namespace;
    await writeFile(renameFlag, "1");
    try {
      const uncertain = JSON.parse(
        await docker([
          "exec",
          "-u",
          "company",
          co,
          "restless",
          "job",
          "run",
          "--memory",
          "128m",
          "--timeout",
          "30",
          "--",
          "printf x > " + uncertainMarker,
        ]),
      );
      assert.match(uncertain.job, /^[a-f0-9]{12}$/);
      assert.equal(uncertain.status, "confirmation_unknown");
      const staged = co + "-job-" + uncertain.job + "-preparing";
      concurrentNames.push(staged);
      await writeFile(
        directory + "/queue.json",
        JSON.stringify(concurrentNames),
      );
      const status = JSON.parse(
        await docker([
          "exec",
          "-u",
          "company",
          co,
          "restless",
          "job",
          "status",
          uncertain.job,
        ]),
      );
      assert.equal(status.status, "confirmation_unknown");
      assert.match(status.reason, /check its status/i);
      await new Promise((resolve) => setTimeout(resolve, 7000));
      assert.equal(
        await docker(["inspect", "-f", "{{.State.Status}}", staged]),
        "created",
      );
      assert.equal(
        await run("docker", [
          "exec",
          "-u",
          "company",
          co,
          "test",
          "-e",
          uncertainMarker,
        ]).then(
          () => true,
          () => false,
        ),
        false,
      );
      // An abandoned preparation must be reclaimed without ever executing.
      // Use the real 120-second age threshold and let the scheduler retry.
      let reaped = false;
      for (let i = 0; i < 150; i++) {
        reaped = !(await run("docker", ["inspect", staged]).then(
          () => true,
          () => false,
        ));
        if (reaped) break;
        await new Promise((resolve) => setTimeout(resolve, 1000));
      }
      assert(reaped, "an abandoned preparation must be reaped");
      assert.equal(
        await run("docker", [
          "exec",
          "-u",
          "company",
          co,
          "test",
          "-e",
          uncertainMarker,
        ]).then(
          () => true,
          () => false,
        ),
        false,
      );
    } finally {
      await unlink(renameFlag).catch(() => {});
      await run("docker", [
        "exec",
        "-u",
        "company",
        co,
        "rm",
        "-f",
        uncertainMarker,
      ]).catch(() => {});
    }
  }
  // The opposite ambiguity: Docker accepted the rename but its reply was
  // lost. The returned ID must let the caller observe the single accepted job.
  if (process.env.RESTLESS_JOB_RESTART_DIR) {
    const directory = process.env.RESTLESS_JOB_RESTART_DIR;
    const flag = directory + "/docker-delay-rename";
    const effect = "/company/run/s109-rename-accepted-" + namespace;
    await writeFile(flag, "1");
    try {
      const result = JSON.parse(
        await docker([
          "exec",
          "-u",
          "company",
          co,
          "restless",
          "job",
          "run",
          "--memory",
          "128m",
          "--timeout",
          "30",
          "--",
          "printf x >> " + effect,
        ]),
      );
      assert.equal(result.status, "confirmation_unknown");
      assert.match(result.job, /^[a-f0-9]{12}$/);
      concurrentNames.push(co + "-job-" + result.job);
      await writeFile(
        directory + "/queue.json",
        JSON.stringify(concurrentNames),
      );
      let completed = false;
      for (let i = 0; i < 90; i++) {
        const observed = JSON.parse(
          await docker([
            "exec",
            "-u",
            "company",
            co,
            "restless",
            "job",
            "status",
            result.job,
          ]),
        );
        if (observed.status === "succeeded") {
          completed = true;
          break;
        }
        assert(
          !["failed", "gone", "timed_out"].includes(observed.status),
          JSON.stringify(observed),
        );
        await new Promise((resolve) => setTimeout(resolve, 1000));
      }
      assert(
        completed,
        "the accepted job must be observable by its returned ID",
      );
      assert.equal(
        await docker(["exec", "-u", "company", co, "cat", effect]),
        "x",
      );
    } finally {
      await unlink(flag).catch(() => {});
      await run("docker", [
        "exec",
        "-u",
        "company",
        co,
        "rm",
        "-f",
        effect,
      ]).catch(() => {});
    }
  }
  console.log(
    JSON.stringify({
      scenario: "heavy job waits under pressure and resumes",
      state,
      waiting: status.status,
      final: final.status,
      concurrentStates: states,
      concurrentCompleted: completed.size,
      oversizedRejected: true,
      queueLimitRacePassed: true,
      pausedReservationPassed: true,
      inspectFailureHandled: true,
      createTimeoutStayedInert: true,
      renameUncertaintyStayedInert: true,
      abandonedPreparationReaped: true,
      acceptedRenameObservedOnce: true,
      companiesSharingPool: 2,
      ownerEntryChecks: 8,
    }),
  );
} finally {
  if (pausedName) await run("docker", ["unpause", pausedName]).catch(() => {});
  if (jobName) await run("docker", ["rm", "-f", jobName]).catch(() => {});
  if (legacyName) await run("docker", ["rm", "-f", legacyName]).catch(() => {});
  for (const name of concurrentNames)
    await run("docker", ["rm", "-f", name]).catch(() => {});
  for (const name of queueFixtures)
    await run("docker", ["rm", "-f", name]).catch(() => {});
  await run("docker", ["exec", "-u", "company", co, "rm", "-f", marker]).catch(
    () => {},
  );
  await setHigh(savedHigh);
}
