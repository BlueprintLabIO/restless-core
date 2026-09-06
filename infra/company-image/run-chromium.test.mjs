import test from 'node:test';
import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { setTimeout as delay } from 'node:timers/promises';
import { closeBrowser, runBrowser } from './run-chromium.mjs';

const endpoint = 'ws://127.0.0.1:9222/devtools/browser/test';
const discovery = url => async () => ({ ok: true, json: async () => ({ webSocketDebuggerUrl: url }) });
function socketWith(result) {
  return class extends EventTarget {
    constructor(url) { super(); assert.equal(url, endpoint); queueMicrotask(() => this.dispatchEvent(new Event('open'))); }
    send(text) {
      assert.deepEqual(JSON.parse(text), { id: 1, method: 'Browser.close' });
      queueMicrotask(() => this.dispatchEvent(result()));
    }
    close() {}
  };
}

test('requests normal browser close through the fixed local endpoint', async () => {
  const Socket = socketWith(() => new MessageEvent('message', { data: '{"id":1,"result":{}}' }));
  await closeBrowser({ fetchImpl: discovery(endpoint), Socket });
});

test('accepts transport shutdown after the close request, not before it', async () => {
  await closeBrowser({ fetchImpl: discovery(endpoint), Socket: socketWith(() => new Event('error')) });
  class FailedSocket extends EventTarget {
    constructor() { super(); queueMicrotask(() => this.dispatchEvent(new Event('error'))); }
    close() {}
  }
  await assert.rejects(closeBrowser({ fetchImpl: discovery(endpoint), Socket: FailedSocket }), /before close request/);
});

test('refuses remote, credentialed and non-browser discovery endpoints', async () => {
  for (const url of ['ws://example.com:9222/devtools/browser/test', 'ws://user@127.0.0.1:9222/devtools/browser/test', 'ws://127.0.0.1:9223/devtools/browser/test', 'ws://127.0.0.1:9222/devtools/page/test']) {
    await assert.rejects(closeBrowser({ fetchImpl: discovery(url) }), /unexpected endpoint/);
  }
});

test('browser refusal is not a successful close receipt', async () => {
  const Socket = socketWith(() => new MessageEvent('message', { data: '{"id":1,"error":{"message":"no"}}' }));
  await assert.rejects(closeBrowser({ fetchImpl: discovery(endpoint), Socket }), /refused close/);
});

function fakeChild() {
  const child = new EventEmitter();
  child.kills = [];
  child.kill = signal => child.kills.push(signal);
  return child;
}

test('TERM requests one browser close and waits for actual child exit', async () => {
  const child = fakeChild(), signals = new EventEmitter();
  let closes = 0, finished = false;
  const result = runBrowser('chromium', [], { spawnImpl: () => child, signals, close: async () => { closes++; }, termAfterMs: 50, killAfterMs: 100 });
  result.then(() => finished = true);
  signals.emit('SIGTERM'); signals.emit('SIGINT');
  await delay(5);
  assert.equal(closes, 1); assert.equal(finished, false); assert.deepEqual(child.kills, []);
  child.emit('exit', 0, null);
  assert.deepEqual(await result, { code: 0, signal: null, stopping: true });
  assert.equal(signals.listenerCount('SIGTERM'), 0);
  await delay(110);
  assert.deepEqual(child.kills, []);
});

test('unresponsive close has bounded TERM/KILL fallbacks', async () => {
  const child = fakeChild(), signals = new EventEmitter();
  const result = runBrowser('chromium', [], { spawnImpl: () => child, signals, close: () => new Promise(() => {}), termAfterMs: 5, killAfterMs: 10, log: () => {} });
  signals.emit('SIGTERM');
  await delay(25);
  assert.deepEqual(child.kills, ['SIGTERM', 'SIGKILL']);
  child.emit('exit', null, 'SIGKILL');
  assert.equal((await result).signal, 'SIGKILL');
});

test('ordinary browser exit and spawn failure clean up signal listeners', async () => {
  for (const event of ['exit', 'error']) {
    const child = fakeChild(), signals = new EventEmitter();
    const result = runBrowser('chromium', [], { spawnImpl: () => child, signals });
    if (event === 'exit') { child.emit('exit', 3, null); assert.equal((await result).code, 3); }
    else { child.emit('error', new Error('spawn failed')); await assert.rejects(result, /spawn failed/); }
    assert.equal(signals.listenerCount('SIGTERM'), 0);
    assert.equal(signals.listenerCount('SIGINT'), 0);
  }
});
