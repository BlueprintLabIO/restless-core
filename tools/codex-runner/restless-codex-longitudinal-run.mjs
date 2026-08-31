#!/usr/bin/env node

// Frozen EXP-17 E-L controller. It delivers one causal event script to a
// durable Codex thread, kills the productive process group at the declared
// checkpoint, then resumes without semantic rescue.

import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { createInterface } from 'node:readline';

const workdir = process.env.RESTLESS_CODEX_TASK_WORKDIR;
const taskFile = process.env.RESTLESS_CODEX_TASK_FILE;
const materialFile = process.env.RESTLESS_CODEX_MATERIAL_EVENT_FILE;
const scheduledFile = process.env.RESTLESS_CODEX_SCHEDULED_EVENT_FILE;
const model = process.env.RESTLESS_CODEX_TASK_MODEL;
const effort = process.env.RESTLESS_CODEX_TASK_EFFORT;
const providerBaseUrl = process.env.RESTLESS_CODEX_TASK_BASE_URL;
const codexHome = process.env.CODEX_HOME;
for (const [name, value] of Object.entries({ workdir, taskFile, materialFile, scheduledFile, model, effort, providerBaseUrl, codexHome })) {
  if (!value) throw new Error(`missing ${name}`);
}
if (!process.env.RESTLESS_MODEL_CAPABILITY) throw new Error('missing scoped model capability');

const runner = '/usr/local/bin/restless-codex-runner';
const task = readFileSync(taskFile, 'utf8');
const material = readFileSync(materialFile, 'utf8');
const scheduled = readFileSync(scheduledFile, 'utf8');
const startedAt = Date.now();
const allEvents = [];
mkdirSync(codexHome, { recursive: true, mode: 0o700 });

function waitForExit(child) {
  if (child.exitCode !== null || child.signalCode !== null) return Promise.resolve({ code: child.exitCode, signal: child.signalCode });
  return new Promise((resolve, reject) => {
    child.once('error', reject);
    child.once('exit', (code, signal) => resolve({ code, signal }));
  });
}

function linuxProcessGroup(pid) {
  const stat = readFileSync(`/proc/${pid}/stat`, 'utf8');
  const close = stat.lastIndexOf(') ');
  assert(close > 0, `cannot parse /proc/${pid}/stat`);
  const fieldsAfterCommand = stat.slice(close + 2).trim().split(/\s+/);
  const processGroup = Number.parseInt(fieldsAfterCommand[2], 10);
  assert(Number.isSafeInteger(processGroup) && processGroup > 1, `invalid process group for pid ${pid}`);
  return processGroup;
}

async function killAndProveProcessGroup(processGroup) {
  try { process.kill(-processGroup, 'SIGKILL'); } catch (error) {
    if (error.code !== 'ESRCH') throw error;
  }
  for (let attempt = 0; attempt < 100; attempt += 1) {
    try {
      process.kill(-processGroup, 0);
    } catch (error) {
      if (error.code === 'ESRCH') return;
      throw error;
    }
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  throw new Error(`productive process group ${processGroup} survived exact cleanup`);
}

function startRunner(threadId = null) {
  const child = spawn(process.execPath, [runner], { cwd: workdir, env: process.env, detached: true, stdio: ['pipe', 'pipe', 'pipe'] });
  let stderr = '';
  const events = [];
  const waiters = new Set();
  child.stderr.setEncoding('utf8');
  child.stderr.on('data', (chunk) => { stderr = `${stderr}${chunk}`.slice(-5000); });
  createInterface({ input: child.stdout, crlfDelay: Infinity }).on('line', (line) => {
    const event = JSON.parse(line);
    events.push(event);
    allEvents.push(event);
    for (const wake of waiters) wake();
  });
  const send = (value) => child.stdin.write(`${JSON.stringify(value)}\n`);
  const waitFor = (predicate, label, milliseconds = 900_000) => new Promise((resolve, reject) => {
    let timer;
    const cleanup = () => { waiters.delete(check); clearTimeout(timer); };
    const check = () => {
      const failure = events.find((event) => ['runner_error', 'app_server_exited', 'runner_process_closed'].includes(event.type));
      if (failure) { cleanup(); reject(new Error(`${label}: ${JSON.stringify(failure)} stderr=${stderr}`)); return; }
      const match = events.find(predicate);
      if (match) { cleanup(); resolve(match); }
    };
    waiters.add(check);
    timer = setTimeout(() => { cleanup(); reject(new Error(`${label}: safety envelope exhausted; events=${JSON.stringify(events.slice(-20))} stderr=${stderr}`)); }, milliseconds);
    check();
  });
  send({
    op: 'launch', cwd: workdir, model, effort, provider_base_url: providerBaseUrl, thread_id: threadId,
    developer_instructions: 'You are the neutral EXP-17 longitudinal producer. Maintain one frozen consumer artifact across causal events. Use only task-local files and tools, perform no external effect, preserve source identities and validate every material change.',
  });
  return { child, events, send, waitFor };
}

async function completeTurn(session, requestId, text) {
  session.send({ op: 'turn', request_id: requestId, text });
  const admitted = await session.waitFor((event) => event.type === 'operation_complete' && event.request_id === requestId, `${requestId} admission`);
  const terminal = await session.waitFor((event) => event.type === 'turn_completed' && event.turn_id === admitted.turn_id, `${requestId} terminal`);
  assert.equal(terminal.status, 'completed', `${requestId} ended ${terminal.status}`);
  return terminal;
}

async function shutdown(session, requestId) {
  session.send({ op: 'shutdown', request_id: requestId });
  session.child.stdin.end();
  assert.deepEqual(await waitForExit(session.child), { code: 0, signal: null });
}

let session;
let resumed;
let productiveProcessGroup = null;
try {
  session = startRunner();
  const ready = await session.waitFor((event) => event.type === 'session_ready', 'initial readiness');
  assert.equal(ready.observed.effort_observed, effort);
  assert.equal(ready.observed.network_policy_observed, 'host-model-relay-only-v1');
  await completeTurn(session, 'el2-initial-v1', task);
  assert(existsSync(join(workdir, 'DECISION_LEDGER.json')), 'initial ledger missing');
  await completeTurn(session, 'el2-material-sig004-v1', `${material}\n\nUpdate the same terminal artifact now and run its visible evaluator.`);
  const afterChange = createHash('sha256').update(readFileSync(join(workdir, 'DECISION_LEDGER.json'))).digest('hex');
  await completeTurn(session, 'el2-duplicate-sig004-v1', `${material}\n\nThis delivery has a distinct transport receipt but the exact same causal signal identity. Apply the contract and validate.`);
  const afterDuplicate = createHash('sha256').update(readFileSync(join(workdir, 'DECISION_LEDGER.json'))).digest('hex');
  assert.equal(afterDuplicate, afterChange, 'causal duplicate changed terminal ledger bytes');

  session.send({
    op: 'turn', request_id: 'el2-process-checkpoint-v1',
    text: 'Re-read and validate the current ledger. Then write PROCESS_CHECKPOINT.json containing the current ledger sha256 and the exact boolean field "ready_for_replacement": true. Finally run this exact foreground command and wait for it: sh -lc \'printf "%s\\n" "$$" > .exp17-long.pid; exec sleep 600\'. Do not do other work after creating the checkpoint.',
  });
  const admitted = await session.waitFor((event) => event.type === 'operation_complete' && event.request_id === 'el2-process-checkpoint-v1', 'checkpoint admission');
  await session.waitFor((event) => event.type === 'item_started' && event.turn_id === admitted.turn_id && event.item?.type === 'commandExecution', 'checkpoint command start');
  const marker = join(workdir, '.exp17-long.pid');
  const longPid = await new Promise((resolve, reject) => {
    const start = Date.now();
    const check = () => {
      try {
        const pid = Number.parseInt(readFileSync(marker, 'utf8').trim(), 10);
        if (Number.isSafeInteger(pid) && pid > 1) { resolve(pid); return; }
      } catch (error) { if (error.code !== 'ENOENT') { reject(error); return; } }
      if (Date.now() - start > 30_000) { reject(new Error('productive-process marker missing')); return; }
      setTimeout(check, 10);
    };
    check();
  });
  productiveProcessGroup = linuxProcessGroup(longPid);
  assert(existsSync(join(workdir, 'PROCESS_CHECKPOINT.json')), 'useful checkpoint missing before kill');
  await killAndProveProcessGroup(productiveProcessGroup);
  productiveProcessGroup = null;
  process.kill(-session.child.pid, 'SIGKILL');
  const killed = await waitForExit(session.child);
  assert.equal(killed.signal, 'SIGKILL');
  session = null;

  resumed = startRunner(ready.thread_id);
  const resumedReady = await resumed.waitFor((event) => event.type === 'session_ready', 'replacement readiness');
  assert.equal(resumedReady.resumed, true);
  assert.equal(resumedReady.observed.network_policy_observed, 'host-model-relay-only-v1');
  await completeTurn(resumed, 'el2-scheduled-sig005-v1', `${scheduled}\n\nThis is the frozen scheduled follow-up after process replacement. Recover from task-local state, update the same ledger and RESULT.md, and run the visible evaluator.`);
  await shutdown(resumed, 'el2-terminal-shutdown');
  resumed = null;

  const usage = allEvents.filter((event) => event.type === 'usage').at(-1)?.token_usage ?? null;
  process.stdout.write(`${JSON.stringify({
    schema: 'restless.exp17.longitudinal-run.v1', terminal_status: 'completed', thread_id: ready.thread_id,
    resumed_after_process_death: true, process_replacements: 1, causal_duplicate_changed_artifact: false,
    runner_digest: createHash('sha256').update(readFileSync(runner)).digest('hex'),
    model_requested: ready.observed.model_requested, model_observed: ready.observed.model_observed,
    reasoning_effort: ready.observed.effort_observed, task_sha256: createHash('sha256').update(task).digest('hex'),
    network_policy: ready.observed.network_policy_observed,
    disabled_features: ready.observed.disabled_features_observed,
    elapsed_ms: Date.now() - startedAt,
    tool_events: allEvents.filter((event) => event.type === 'item_started' && event.item?.type === 'commandExecution').length,
    usage,
  })}\n`);
} finally {
  if (productiveProcessGroup !== null) {
    await killAndProveProcessGroup(productiveProcessGroup).catch(() => {});
  }
  for (const active of [session, resumed]) {
    if (active?.child && active.child.exitCode === null && active.child.signalCode === null) {
      try { process.kill(-active.child.pid, 'SIGKILL'); } catch {}
      await waitForExit(active.child).catch(() => {});
    }
  }
  rmSync(join(workdir, '.exp17-long.pid'), { force: true });
  if (process.env.RESTLESS_CODEX_TASK_PRESERVE_SESSION !== '1') {
    rmSync(codexHome, { recursive: true, force: true, maxRetries: 20, retryDelay: 50 });
  }
}
