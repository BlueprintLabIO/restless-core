#!/usr/bin/env node

// Neutral one-turn task controller for EXP-17 solo arms. It supplies frozen
// task bytes to the shared runner and records transport/process evidence only.
// It does not plan, review, repair, gate or interpret the artifact.

import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, rmSync } from 'node:fs';
import { createInterface } from 'node:readline';

const workdir = process.env.RESTLESS_CODEX_TASK_WORKDIR;
const taskFile = process.env.RESTLESS_CODEX_TASK_FILE;
const model = process.env.RESTLESS_CODEX_TASK_MODEL;
const effort = process.env.RESTLESS_CODEX_TASK_EFFORT;
const providerBaseUrl = process.env.RESTLESS_CODEX_TASK_BASE_URL;
const codexHome = process.env.CODEX_HOME;
const priorThreadId = process.env.RESTLESS_CODEX_TASK_THREAD_ID || null;
const requestId = process.env.RESTLESS_CODEX_TASK_REQUEST_ID || 'frozen-task-v1';
const preserveSession = process.env.RESTLESS_CODEX_TASK_PRESERVE_SESSION === '1';
for (const [name, value] of Object.entries({ workdir, taskFile, model, effort, providerBaseUrl, codexHome })) {
  if (!value) throw new Error(`missing ${name}`);
}
if (!process.env.RESTLESS_MODEL_CAPABILITY) throw new Error('missing scoped model capability');

const runner = '/usr/local/bin/restless-codex-runner';
const runnerDigest = createHash('sha256').update(readFileSync(runner)).digest('hex');
const task = readFileSync(taskFile, 'utf8');
assert(task.trim(), 'frozen task file is empty');
mkdirSync(codexHome, { recursive: true, mode: 0o700 });

function waitForExit(child) {
  if (child.exitCode !== null || child.signalCode !== null) {
    return Promise.resolve({ code: child.exitCode, signal: child.signalCode });
  }
  return new Promise((resolve, reject) => {
    child.once('error', reject);
    child.once('exit', (code, signal) => resolve({ code, signal }));
  });
}

const startedAt = Date.now();
const child = spawn(process.execPath, [runner], {
  cwd: workdir,
  env: process.env,
  detached: true,
  stdio: ['pipe', 'pipe', 'pipe'],
});
let stderr = '';
child.stderr.setEncoding('utf8');
child.stderr.on('data', (chunk) => { stderr = `${stderr}${chunk}`.slice(-4000); });
const events = [];
const waiters = new Set();
createInterface({ input: child.stdout, crlfDelay: Infinity }).on('line', (line) => {
  const event = JSON.parse(line);
  events.push(event);
  for (const wake of waiters) wake();
});
const send = (value) => child.stdin.write(`${JSON.stringify(value)}\n`);
const waitFor = (predicate, label, milliseconds = 900_000) => new Promise((resolve, reject) => {
  let timer = null;
  const cleanup = () => {
    waiters.delete(check);
    if (timer) clearTimeout(timer);
  };
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
  waiters.add(check);
  timer = setTimeout(() => {
    cleanup();
    reject(new Error(`${label}: safety envelope exhausted; events=${JSON.stringify(events.slice(-20))} stderr=${stderr.slice(-1000)}`));
  }, milliseconds);
  check();
});

try {
  send({
    op: 'launch',
    cwd: workdir,
    model,
    effort,
    provider_base_url: providerBaseUrl,
    developer_instructions: 'You are the neutral EXP-17 solo producer. Own the frozen task end to end. Use only task-local files and tools, perform no external effect, do not inspect organisational state, and stop only after creating and verifying the requested terminal artifact.',
    thread_id: priorThreadId,
  });
  const ready = await waitFor((event) => event.type === 'session_ready', 'session readiness');
  assert.equal(ready.observed.effort_observed, effort);
  assert.equal(ready.observed.network_policy_observed, 'host-model-relay-only-v1');
  send({ op: 'turn', request_id: requestId, text: task });
  const admitted = await waitFor(
    (event) => event.type === 'operation_complete' && event.request_id === requestId,
    'turn admission',
  );
  const terminal = await waitFor(
    (event) => event.type === 'turn_completed' && event.turn_id === admitted.turn_id,
    'turn terminal',
  );
  assert.equal(terminal.status, 'completed', `Codex task terminal was ${terminal.status}`);
  send({ op: 'shutdown', request_id: `${requestId}-shutdown` });
  child.stdin.end();
  assert.deepEqual(await waitForExit(child), { code: 0, signal: null });

  const usage = events.filter((event) => event.type === 'usage').at(-1)?.token_usage ?? null;
  process.stdout.write(`${JSON.stringify({
    schema: 'restless.exp17.solo-run.v1',
    terminal_status: terminal.status,
    thread_id: ready.thread_id,
    resumed: ready.resumed,
    turn_id: terminal.turn_id,
    runner_digest: runnerDigest,
    codex_version: ready.observed.codex_version,
    model_requested: ready.observed.model_requested,
    model_observed: ready.observed.model_observed,
    reasoning_effort: ready.observed.effort_observed,
    provider_observed: ready.observed.provider_observed,
    network_policy: ready.observed.network_policy_observed,
    disabled_features: ready.observed.disabled_features_observed,
    task_sha256: createHash('sha256').update(task).digest('hex'),
    elapsed_ms: Date.now() - startedAt,
    tool_events: events.filter((event) => event.type === 'item_started' && event.item?.type === 'commandExecution').length,
    usage,
  })}\n`);
} finally {
  if (child.exitCode === null && child.signalCode === null) {
    try { process.kill(-child.pid, 'SIGKILL'); } catch {}
    await waitForExit(child).catch(() => {});
  }
  if (!preserveSession) {
    rmSync(codexHome, { recursive: true, force: true, maxRetries: 20, retryDelay: 50 });
  }
}
