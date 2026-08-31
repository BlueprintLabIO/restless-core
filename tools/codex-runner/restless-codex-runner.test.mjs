import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { chmodSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createInterface } from 'node:readline';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const runner = fileURLToPath(new URL('./restless-codex-runner.mjs', import.meta.url));

function waitForExit(child) {
  if (child.exitCode !== null || child.signalCode !== null) {
    return Promise.resolve({ code: child.exitCode, signal: child.signalCode });
  }
  return new Promise((resolve, reject) => {
    child.once('error', reject);
    child.once('exit', (code, signal) => resolve({ code, signal }));
  });
}

function fakeCodex(directory) {
  const path = join(directory, 'fake-codex');
  writeFileSync(path, `#!/usr/bin/env node
const readline = require('node:readline');
const input = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
function out(value) { process.stdout.write(JSON.stringify(value) + '\\n'); }
const requiredDisabled = ['multi_agent', 'plugins', 'remote_plugin', 'plugin_sharing', 'skill_search', 'skill_mcp_dependency_install', 'apps', 'browser_use', 'browser_use_external', 'browser_use_full_cdp_access', 'computer_use', 'image_generation', 'in_app_browser'];
const disabled = process.argv.flatMap((value, index, all) => value === '--disable' ? [all[index + 1]] : []);
if (!requiredDisabled.every((feature) => disabled.includes(feature))) process.exit(91);
if (process.env.HTTP_PROXY !== 'http://127.0.0.1:9' || process.env.http_proxy !== 'http://127.0.0.1:9') process.exit(92);
if (process.env.NO_PROXY !== 'host.docker.internal,127.0.0.1,localhost') process.exit(93);
let pendingTurnStart = null;
input.on('line', (line) => {
  const message = JSON.parse(line);
  if (message.method === 'initialize') {
    out({ id: message.id, result: { userAgent: 'codex-cli 0.test', codexHome: process.env.CODEX_HOME } });
  } else if (message.method === 'initialized') {
    out({ method: 'remoteControl/status/changed', params: { status: 'disabled' } });
  } else if (message.method === 'thread/start') {
    out({ id: message.id, result: { thread: { id: 'thread-test' }, model: message.params.model, modelProvider: message.params.modelProvider, reasoningEffort: 'high', cwd: message.params.cwd, approvalPolicy: message.params.approvalPolicy, sandbox: message.params.sandbox } });
    out({ method: 'thread/started', params: { thread: { id: 'thread-test' } } });
  } else if (message.method === 'turn/start') {
    pendingTurnStart = message.id;
    out({ method: 'turn/started', params: { threadId: 'thread-test', turn: { id: 'turn-test' } } });
    out({ method: 'item/agentMessage/delta', params: { threadId: 'thread-test', turnId: 'turn-test', itemId: 'item-test', delta: 'working' } });
    out({ method: 'thread/tokenUsage/updated', params: { threadId: 'thread-test', turnId: 'turn-test', tokenUsage: { total: { totalTokens: 7 }, last: { totalTokens: 7 }, modelContextWindow: 100 } } });
  } else if (message.method === 'turn/steer') {
    out({ id: message.id, result: { turnId: 'turn-test' } });
  } else if (message.method === 'turn/interrupt') {
    out({ id: message.id, result: {} });
    out({ method: 'turn/completed', params: { threadId: 'thread-test', turn: { id: 'turn-test', status: 'interrupted', error: null, durationMs: 12 } } });
    out({ id: pendingTurnStart, result: { turn: { id: 'turn-test' } } });
    pendingTurnStart = null;
  }
});
`, { mode: 0o755 });
  chmodSync(path, 0o755);
  return path;
}

test('normalises app-server session, turn, steer, usage and interrupt events', async () => {
  const temporary = mkdtempSync(join(tmpdir(), 'restless-codex-runner-test.'));
  let child = null;
  let stderr = '';
  try {
    child = spawn(process.execPath, [runner], {
      env: {
        ...process.env,
        CODEX_HOME: temporary,
        RESTLESS_MODEL_CAPABILITY: 'scoped-test-value',
      },
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    child.stderr.setEncoding('utf8');
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    const events = [];
    createInterface({ input: child.stdout, crlfDelay: Infinity }).on('line', (line) => events.push(JSON.parse(line)));
    const send = (value) => child.stdin.write(`${JSON.stringify(value)}\n`);
    send({
      op: 'launch',
      cwd: temporary,
      model: 'litellm/gpt-5.6-sol',
      effort: 'high',
      provider_base_url: 'http://host.docker.internal:7790',
      developer_instructions: 'Test instructions.',
      codex_bin: fakeCodex(temporary),
    });
    await new Promise((resolve, reject) => {
      const interval = setInterval(() => {
        if (events.some((event) => event.type === 'session_ready')) {
          clearTimeout(deadline);
          clearInterval(interval);
          resolve();
        }
      }, 5);
      const deadline = setTimeout(() => {
        clearInterval(interval);
        reject(new Error(`runner did not become ready: ${JSON.stringify(events)} stderr=${stderr}`));
      }, 2000);
    });
    send({ op: 'turn', request_id: 'one', text: 'Do the work.' });
    await new Promise((resolve) => setTimeout(resolve, 25));
    send({ op: 'turn', request_id: 'one', text: 'Do the work.' });
    await new Promise((resolve) => setTimeout(resolve, 25));
    send({ op: 'steer', request_id: 'two', text: 'Use the retained checkpoint.' });
    await new Promise((resolve) => setTimeout(resolve, 25));
    send({ op: 'interrupt', request_id: 'three' });
    await new Promise((resolve) => setTimeout(resolve, 25));
    send({ op: 'shutdown', request_id: 'four' });
    child.stdin.end();
    const exit = await waitForExit(child);
    assert.deepEqual(exit, { code: 0, signal: null });
    const ready = events.find((event) => event.type === 'session_ready');
    assert.equal(ready.thread_id, 'thread-test');
    assert.equal(ready.observed.model_observed, 'gpt-5.6-sol');
    assert.equal(ready.observed.provider_observed, 'restless');
    assert.equal(ready.observed.network_policy_observed, 'host-model-relay-only-v1');
    assert.deepEqual(ready.observed.disabled_features_observed, ['multi_agent', 'plugins', 'remote_plugin', 'plugin_sharing', 'skill_search', 'skill_mcp_dependency_install', 'apps', 'browser_use', 'browser_use_external', 'browser_use_full_cdp_access', 'computer_use', 'image_generation', 'in_app_browser']);
    assert.match(ready.observed.runner_digest, /^[a-f0-9]{64}$/);
    assert(events.some((event) => event.type === 'agent_text_delta' && event.text === 'working'));
    assert.equal(events.filter((event) => event.type === 'turn_started').length, 1);
    assert(events.some((event) => event.type === 'operation_duplicate' && event.request_id === 'one'));
    assert(events.some((event) => event.type === 'usage' && event.token_usage.total.totalTokens === 7));
    assert(events.some((event) => event.type === 'operation_complete' && event.op === 'steer'));
    assert(events.some((event) => event.type === 'turn_completed' && event.status === 'interrupted'));
    assert(!events.some((event) => event.type === 'runner_error'));
  } finally {
    if (child && child.exitCode === null && child.signalCode === null) child.kill('SIGKILL');
    rmSync(temporary, { recursive: true, force: true });
  }
});

test('live first-party app-server admits the exact route and completes event-driven', {
  skip: !process.env.RESTLESS_CODEX_LIVE_BASE_URL || !process.env.RESTLESS_MODEL_CAPABILITY,
}, async () => {
  const temporary = mkdtempSync(join(tmpdir(), 'restless-codex-live.'));
  let child = null;
  let stderr = '';
  try {
    mkdirSync(join(temporary, 'codex-home'), { recursive: true });
    child = spawn(process.execPath, [runner], {
      env: { ...process.env, CODEX_HOME: join(temporary, 'codex-home') },
      cwd: temporary,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    child.stderr.setEncoding('utf8');
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    const events = [];
    createInterface({ input: child.stdout, crlfDelay: Infinity }).on('line', (line) => events.push(JSON.parse(line)));
    const waitFor = (predicate, label, milliseconds = 120_000) => new Promise((resolve, reject) => {
      const interval = setInterval(() => {
        const failure = events.find((event) => ['runner_error', 'app_server_exited'].includes(event.type));
        if (failure) {
          clearTimeout(deadline);
          clearInterval(interval);
          reject(new Error(`${label}: ${JSON.stringify(failure)} stderr=${stderr.slice(-1000)}`));
          return;
        }
        const match = events.find(predicate);
        if (match) {
          clearTimeout(deadline);
          clearInterval(interval);
          resolve(match);
        }
      }, 20);
      const deadline = setTimeout(() => {
        clearInterval(interval);
        reject(new Error(`${label}: ${JSON.stringify(events.slice(-12))} stderr=${stderr.slice(-1000)}`));
      }, milliseconds);
    });
    const send = (value) => child.stdin.write(`${JSON.stringify(value)}\n`);
    send({
      op: 'launch',
      cwd: temporary,
      model: process.env.RESTLESS_CODEX_LIVE_MODEL || 'litellm/gpt-5.6-sol',
      effort: process.env.RESTLESS_CODEX_LIVE_EFFORT || 'high',
      provider_base_url: process.env.RESTLESS_CODEX_LIVE_BASE_URL,
      developer_instructions: 'This is a no-artifact transport admission probe. Do not use tools.',
    });
    const ready = await waitFor((event) => event.type === 'session_ready', 'session readiness');
    assert.equal(ready.observed.effort_observed, process.env.RESTLESS_CODEX_LIVE_EFFORT || 'high');
    send({ op: 'turn', request_id: 'live-one', text: 'Reply with exactly READY and nothing else.' });
    const terminal = await waitFor((event) => event.type === 'turn_completed', 'turn completion');
    assert.equal(terminal.status, 'completed');
    assert.equal(
      events.filter((event) => event.type === 'agent_text_delta').map((event) => event.text).join('').trim(),
      'READY',
    );
    assert(events.some((event) => event.type === 'usage'));
    send({ op: 'shutdown', request_id: 'live-shutdown' });
    child.stdin.end();
    const exit = await waitForExit(child);
    assert.deepEqual(exit, { code: 0, signal: null });

    // A hot worker may be reconstructed after a daemon or transport process
    // restart. Prove that the durable Codex thread remains resumable from the
    // same actor-scoped CODEX_HOME without replaying or guessing its context.
    child = spawn(process.execPath, [runner], {
      env: { ...process.env, CODEX_HOME: join(temporary, 'codex-home') },
      cwd: temporary,
      stdio: ['pipe', 'pipe', 'pipe'],
    });
    stderr = '';
    child.stderr.setEncoding('utf8');
    child.stderr.on('data', (chunk) => { stderr += chunk; });
    const resumedEvents = [];
    createInterface({ input: child.stdout, crlfDelay: Infinity }).on('line', (line) => resumedEvents.push(JSON.parse(line)));
    const waitForResumed = (predicate, label, milliseconds = 120_000) => new Promise((resolve, reject) => {
      const interval = setInterval(() => {
        const failure = resumedEvents.find((event) => ['runner_error', 'app_server_exited'].includes(event.type));
        if (failure) {
          clearTimeout(deadline);
          clearInterval(interval);
          reject(new Error(`${label}: ${JSON.stringify(failure)} stderr=${stderr.slice(-1000)}`));
          return;
        }
        const match = resumedEvents.find(predicate);
        if (match) {
          clearTimeout(deadline);
          clearInterval(interval);
          resolve(match);
        }
      }, 20);
      const deadline = setTimeout(() => {
        clearInterval(interval);
        reject(new Error(`${label}: ${JSON.stringify(resumedEvents.slice(-12))} stderr=${stderr.slice(-1000)}`));
      }, milliseconds);
    });
    const sendResumed = (value) => child.stdin.write(`${JSON.stringify(value)}\n`);
    sendResumed({
      op: 'launch',
      cwd: temporary,
      model: process.env.RESTLESS_CODEX_LIVE_MODEL || 'litellm/gpt-5.6-sol',
      effort: process.env.RESTLESS_CODEX_LIVE_EFFORT || 'high',
      provider_base_url: process.env.RESTLESS_CODEX_LIVE_BASE_URL,
      developer_instructions: 'This is a no-artifact transport resume probe. Do not use tools.',
      thread_id: ready.thread_id,
    });
    const resumed = await waitForResumed((event) => event.type === 'session_ready', 'resumed session readiness');
    assert.equal(resumed.thread_id, ready.thread_id);
    assert.equal(resumed.resumed, true);
    assert.equal(resumed.observed.effort_observed, process.env.RESTLESS_CODEX_LIVE_EFFORT || 'high');
    sendResumed({ op: 'turn', request_id: 'live-two', text: 'Reply with exactly READY2 and nothing else.' });
    const resumedTerminal = await waitForResumed((event) => event.type === 'turn_completed', 'resumed turn completion');
    assert.equal(resumedTerminal.status, 'completed');
    assert.equal(
      resumedEvents.filter((event) => event.type === 'agent_text_delta').map((event) => event.text).join('').trim(),
      'READY2',
    );
    assert(resumedEvents.some((event) => event.type === 'usage'));
    sendResumed({ op: 'shutdown', request_id: 'live-resumed-shutdown' });
    child.stdin.end();
    assert.deepEqual(await waitForExit(child), { code: 0, signal: null });
  } finally {
    if (child && child.exitCode === null && child.signalCode === null) child.kill('SIGKILL');
    rmSync(temporary, { recursive: true, force: true });
  }
});
