#!/usr/bin/env node

// Neutral, no-rescue controller for EXP-17's first-party Codex parity gate.
// It runs inside the same Company image as Restless Staff but receives no
// organisational capability. The shared runner owns every Codex session.

import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { chmodSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { createInterface } from 'node:readline';

const workdir = process.env.RESTLESS_CODEX_PREFLIGHT_WORKDIR;
const model = process.env.RESTLESS_CODEX_PREFLIGHT_MODEL;
const effort = process.env.RESTLESS_CODEX_PREFLIGHT_EFFORT;
const providerBaseUrl = process.env.RESTLESS_CODEX_PREFLIGHT_BASE_URL;
const codexHome = process.env.CODEX_HOME;
for (const [name, value] of Object.entries({ workdir, model, effort, providerBaseUrl, codexHome })) {
  if (!value) throw new Error(`missing ${name}`);
}
if (!process.env.RESTLESS_MODEL_CAPABILITY) throw new Error('missing scoped model capability');

const runner = '/usr/local/bin/restless-codex-runner';
const runnerDigest = createHash('sha256').update(readFileSync(runner)).digest('hex');

function waitForExit(child) {
  if (child.exitCode !== null || child.signalCode !== null) {
    return Promise.resolve({ code: child.exitCode, signal: child.signalCode });
  }
  return new Promise((resolve, reject) => {
    child.once('error', reject);
    child.once('exit', (code, signal) => resolve({ code, signal }));
  });
}

function startRunner(threadId = null) {
  const child = spawn(process.execPath, [runner], {
    cwd: workdir,
    env: process.env,
    detached: true,
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  let stderr = '';
  child.stderr.setEncoding('utf8');
  child.stderr.on('data', (chunk) => { stderr += chunk; });
  const events = [];
  const waiters = new Set();
  createInterface({ input: child.stdout, crlfDelay: Infinity }).on('line', (line) => {
    const event = JSON.parse(line);
    events.push(event);
    for (const wake of waiters) wake();
  });
  const send = (value) => child.stdin.write(`${JSON.stringify(value)}\n`);
  const waitFor = (predicate, label, milliseconds = 180_000) => new Promise((resolve, reject) => {
    let timer = null;
    const check = () => {
      const failure = events.find((event) => ['runner_error', 'app_server_exited', 'runner_process_closed'].includes(event.type));
      if (failure) {
        cleanup();
        reject(new Error(`${label}: ${JSON.stringify(failure)} stderr=${stderr.slice(-1000)}`));
        return;
      }
      const match = events.find(predicate);
      if (match) {
        cleanup();
        resolve(match);
      }
    };
    const cleanup = () => {
      waiters.delete(check);
      if (timer) clearTimeout(timer);
    };
    waiters.add(check);
    timer = setTimeout(() => {
      cleanup();
      reject(new Error(`${label}: liveness envelope exhausted; events=${JSON.stringify(events.slice(-16))} stderr=${stderr.slice(-1000)}`));
    }, milliseconds);
    check();
  });
  send({
    op: 'launch',
    cwd: workdir,
    model,
    effort,
    provider_base_url: providerBaseUrl,
    developer_instructions: 'You are the neutral EXP-17 parity actor. Follow the exact task, use only task-local tools, do not inspect organisational state, and do not perform external effects.',
    thread_id: threadId,
  });
  return { child, events, send, waitFor };
}

async function shutdown(session, requestId) {
  session.send({ op: 'shutdown', request_id: requestId });
  session.child.stdin.end();
  assert.deepEqual(await waitForExit(session.child), { code: 0, signal: null });
}

let first = null;
let resumed = null;
let firstToolEvents = 0;
let firstUsageEvents = 0;
let resumedToolEvents = 0;
let resumedUsageEvents = 0;
try {
  mkdirSync(workdir, { recursive: true, mode: 0o770 });
  mkdirSync(codexHome, { recursive: true, mode: 0o700 });
  writeFileSync(join(workdir, 'fixture.txt'), 'PARITY_FIXTURE_V1\n', { mode: 0o440 });
  writeFileSync(join(workdir, 'gate.sh'), '#!/bin/sh\nset -eu\ntest "$(cat fixture.txt)" = "PARITY_FIXTURE_V1"\nprintf "GATE_OK\\n"\n', { mode: 0o550 });
  chmodSync(join(workdir, 'gate.sh'), 0o550);
  writeFileSync(join(workdir, 'long-safe.sh'), '#!/bin/sh\nset -eu\nprintf "%s\\n" "$$" > long.pid\nprintf "LONG_STARTED\\n"\nexec sleep 600\n', { mode: 0o550 });
  chmodSync(join(workdir, 'long-safe.sh'), 0o550);

  first = startRunner();
  const firstReady = await first.waitFor((event) => event.type === 'session_ready', 'cold readiness');
  assert.equal(firstReady.observed.effort_observed, effort);
  assert.equal(firstReady.observed.network_policy_observed, 'host-model-relay-only-v1');
  first.send({
    op: 'turn',
    request_id: 'artifact-event-v1',
    text: 'Use the shell to read fixture.txt and execute ./gate.sh. Write artifact.txt with exactly two lines: PARITY_FIXTURE_V1 and GATE_OK. Then reply exactly ARTIFACT_READY.',
  });
  const firstTerminal = await first.waitFor((event) => event.type === 'turn_completed', 'artifact turn');
  assert.equal(firstTerminal.status, 'completed');
  assert.equal(readFileSync(join(workdir, 'artifact.txt'), 'utf8'), 'PARITY_FIXTURE_V1\nGATE_OK\n');
  assert(first.events.some((event) => event.type === 'item_started'));
  assert(first.events.some((event) => event.type === 'usage'));

  const turnsBeforeDuplicate = first.events.filter((event) => event.type === 'turn_started').length;
  first.send({
    op: 'turn',
    request_id: 'artifact-event-v1',
    text: 'This is the same semantic event and must not execute twice.',
  });
  await first.waitFor((event) => event.type === 'operation_duplicate' && event.request_id === 'artifact-event-v1', 'duplicate suppression');
  assert.equal(first.events.filter((event) => event.type === 'turn_started').length, turnsBeforeDuplicate);
  firstToolEvents = first.events.filter((event) => event.type === 'item_started').length;
  firstUsageEvents = first.events.filter((event) => event.type === 'usage').length;
  await shutdown(first, 'cold-shutdown');
  first = null;

  resumed = startRunner(firstReady.thread_id);
  const resumedReady = await resumed.waitFor((event) => event.type === 'session_ready', 'resume readiness');
  assert.equal(resumedReady.resumed, true);
  assert.equal(resumedReady.thread_id, firstReady.thread_id);
  assert.equal(resumedReady.observed.network_policy_observed, 'host-model-relay-only-v1');
  resumed.send({
    op: 'turn',
    request_id: 'resume-event-v1',
    text: 'Use the shell to read artifact.txt, then reply exactly RESUMED.',
  });
  const resumeTerminal = await resumed.waitFor((event) => event.type === 'turn_completed', 'resume turn');
  assert.equal(resumeTerminal.status, 'completed');

  resumed.send({
    op: 'turn',
    request_id: 'cancel-event-v1',
    text: 'Run ./long-safe.sh now and wait for it to finish. Do not run it in the background and do not use another command.',
  });
  const cancelStarted = await resumed.waitFor(
    (event) => event.type === 'operation_complete' && event.request_id === 'cancel-event-v1',
    'long-tool turn admission',
  );
  await resumed.waitFor(
    (event) => event.type === 'item_started'
      && event.turn_id === cancelStarted.turn_id
      && event.item?.type === 'commandExecution',
    'long-tool start',
  );
  const longPidPath = join(workdir, 'long.pid');
  const longPid = await new Promise((resolve, reject) => {
    const startedAt = Date.now();
    const check = () => {
      try {
        const pid = Number.parseInt(readFileSync(longPidPath, 'utf8').trim(), 10);
        if (Number.isSafeInteger(pid) && pid > 1) {
          resolve(pid);
          return;
        }
      } catch (error) {
        if (error.code !== 'ENOENT') {
          reject(error);
          return;
        }
      }
      if (Date.now() - startedAt >= 10_000) {
        reject(new Error('long-tool process marker did not materialize after the command-start event'));
        return;
      }
      setTimeout(check, 10);
    };
    check();
  });
  assert(Number.isSafeInteger(longPid) && longPid > 1);
  resumed.send({ op: 'interrupt', request_id: 'cancel-request-v1' });
  const interrupted = await resumed.waitFor(
    (event) => event.type === 'turn_completed'
      && event.turn_id === cancelStarted.turn_id
      && ['interrupted', 'cancelled'].includes(event.status),
    'interrupted terminal',
  );
  assert(['interrupted', 'cancelled'].includes(interrupted.status));
  resumedToolEvents = resumed.events.filter((event) => event.type === 'item_started').length;
  resumedUsageEvents = resumed.events.filter((event) => event.type === 'usage').length;
  await shutdown(resumed, 'resumed-shutdown');
  resumed = null;
  assert.throws(() => process.kill(longPid, 0));

  const artifact = readFileSync(join(workdir, 'artifact.txt'));
  process.stdout.write(`${JSON.stringify({
    result: 'passed',
    codex_version: firstReady.observed.codex_version,
    runner_digest: runnerDigest,
    protocol_version: firstReady.observed.protocol_version,
    model_requested: firstReady.observed.model_requested,
    model_observed: firstReady.observed.model_observed,
    reasoning_effort: firstReady.observed.effort_observed,
    provider_observed: firstReady.observed.provider_observed,
    approval_policy: firstReady.observed.approval_policy_observed,
    sandbox_policy: firstReady.observed.sandbox_observed,
    network_policy: firstReady.observed.network_policy_observed,
    disabled_features: firstReady.observed.disabled_features_observed,
    artifact_sha256: createHash('sha256').update(artifact).digest('hex'),
    cold_thread_id: firstReady.thread_id,
    resumed: true,
    duplicate_semantic_deliveries: 1,
    cancellation_terminal: interrupted.status,
    long_process_reaped: true,
    tool_events: firstToolEvents + resumedToolEvents,
    usage_events: firstUsageEvents + resumedUsageEvents,
  })}\n`);
} finally {
  for (const session of [first, resumed]) {
    if (session?.child && session.child.exitCode === null && session.child.signalCode === null) {
      try { process.kill(-session.child.pid, 'SIGKILL'); } catch {}
      await waitForExit(session.child).catch(() => {});
    }
  }
  rmSync(workdir, { recursive: true, force: true, maxRetries: 20, retryDelay: 50 });
  rmSync(codexHome, { recursive: true, force: true, maxRetries: 20, retryDelay: 50 });
}
