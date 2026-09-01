import assert from 'node:assert/strict';
import {
  mkdir, mkdtemp, readFile, symlink, writeFile,
} from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createConnection, createServer } from 'node:net';
import test from 'node:test';

import {
  buildRegistration, createCommandHandler, handleCommand, parseConfiguration,
} from './runtime-agent.mjs';

const UUIDS = [1, 2, 3, 4].map((value) => `00000000-0000-4000-8000-${String(value).padStart(12, '0')}`);
function environment(capabilityFile) {
  return {
    RESTLESS_RUNTIME_BRIDGE_URL: 'wss://plane.example.test/internal/v1/runtime-bridge',
    RESTLESS_RUNTIME_OWNER_ID: UUIDS[0],
    RESTLESS_RUNTIME_PLANE_ID: UUIDS[1],
    RESTLESS_RUNTIME_COMPANY_ID: UUIDS[2],
    RESTLESS_RUNTIME_CELL_ID: UUIDS[3],
    RESTLESS_COMPANY: 'hosted_test',
    RESTLESS_RUNTIME_ID: 'runtime-1',
    RESTLESS_RUNTIME_GENERATION: '2',
    RESTLESS_RUNTIME_DESIRED_REVISION: '3',
    RESTLESS_RUNTIME_IMAGE: `ghcr.io/example/runtime@sha256:${'a'.repeat(64)}`,
    RESTLESS_RUNTIME_VOLUME_NAME: 'cell-volume-1',
    RESTLESS_SOURCE_REVISION: 'b'.repeat(40),
    RESTLESS_RUNTIME_BRIDGE_CAPABILITY_FILE: capabilityFile,
  };
}

test('registration carries exact immutable identity and only implemented features', async () => {
  const root = await mkdtemp(join(tmpdir(), 'restless-runtime-agent-'));
  const capabilityFile = join(root, 'bridge.cap');
  await writeFile(capabilityFile, 'r1.payload.signature\n', { mode: 0o600 });
  const registration = await buildRegistration(parseConfiguration(environment(capabilityFile)));
  assert.equal(registration.protocol_version, 1);
  assert.equal(registration.runtime_generation, 2);
  assert.equal(registration.desired_revision, 3);
  assert.deepEqual(registration.supported_features, [
    'registration.v1', 'activity.v1', 'desktop.v1', 'files.v1', 'process.v1', 'streams.v1',
  ]);
  assert.equal(registration.capability, 'r1.payload.signature');
});

test('activity probe is exact, bounded and tied to the active Runtime generation', async () => {
  const config = parseConfiguration(environment('/run/secrets/bridge.cap'));
  const operationId = '00000000-0000-4000-8000-000000000099';
  const response = await handleCommand(JSON.stringify({
    type: 'activity.observe',
    protocol_version: 1,
    operation_id: operationId,
    runtime_id: 'runtime-1',
    runtime_generation: 2,
  }), config, new Date('2026-09-01T00:00:00.000Z'));
  assert.deepEqual(response, {
    type: 'activity.result',
    operation_id: operationId,
    runtime_id: 'runtime-1',
    runtime_generation: 2,
    active_processes: [],
    observed_at: '2026-09-01T00:00:00.000Z',
  });
  await assert.rejects(() => handleCommand(JSON.stringify({
    type: 'activity.observe',
    protocol_version: 1,
    operation_id: operationId,
    runtime_id: 'runtime-1',
    runtime_generation: 1,
  }), config));
});

test('file reads stay beneath the company volume and return bounded evidence', async () => {
  const root = await mkdtemp(join(tmpdir(), 'restless-runtime-files-'));
  await writeFile(join(root, 'mission.md'), 'Ship useful work.\n');
  await symlink('/etc/passwd', join(root, 'escape'));
  const config = parseConfiguration(environment('/run/secrets/bridge.cap'));
  const handle = createCommandHandler(config, { companyRoot: root });
  const base = {
    type: 'file.read', protocol_version: 1,
    operation_id: '00000000-0000-4000-8000-000000000101',
    runtime_id: 'runtime-1', runtime_generation: 2,
    path: '/company/mission.md', max_bytes: 4096,
  };
  const response = await handle(JSON.stringify(base));
  assert.equal(response.type, 'file.result');
  assert.equal(Buffer.from(response.bytes_base64, 'base64').toString(), 'Ship useful work.\n');
  assert.match(response.sha256, /^[0-9a-f]{64}$/);
  const chunk = await handle(JSON.stringify({
    type: 'file.read_chunk', protocol_version: 1,
    operation_id: '00000000-0000-4000-8000-000000000103',
    runtime_id: 'runtime-1', runtime_generation: 2,
    path: '/company/mission.md', offset: 5, max_bytes: 4,
  }));
  assert.equal(Buffer.from(chunk.bytes_base64, 'base64').toString(), 'usef');
  assert.equal(chunk.offset, 5);
  assert.equal(chunk.size_bytes, 18);
  assert.equal(chunk.eof, false);
  await assert.rejects(() => handle(JSON.stringify({
    ...base, operation_id: '00000000-0000-4000-8000-000000000102', path: '/company/escape',
  })), /escapes/);
});

test('file writes are atomic, confined and exactly replayable', async () => {
  const root = await mkdtemp(join(tmpdir(), 'restless-runtime-writes-'));
  await symlink('/tmp', join(root, 'escape'));
  const config = parseConfiguration(environment('/run/secrets/bridge.cap'));
  const handle = createCommandHandler(config, { companyRoot: root });
  const common = {
    protocol_version: 1,
    operation_id: '00000000-0000-4000-8000-000000000151',
    runtime_id: 'runtime-1', runtime_generation: 2,
  };
  const command = JSON.stringify({
    type: 'file.write', ...common,
    path: '/company/inbox/item/content',
    bytes_base64: Buffer.from('owner attachment').toString('base64'),
  });
  const written = await handle(command);
  assert.equal(written.type, 'file.written');
  assert.equal(written.size_bytes, 16);
  assert.equal(await readFile(join(root, 'inbox/item/content'), 'utf8'), 'owner attachment');
  assert.deepEqual(await handle(command), written);
  await assert.rejects(() => handle(JSON.stringify({
    type: 'file.write', ...common,
    operation_id: '00000000-0000-4000-8000-000000000152',
    path: '/company/escape/stolen', bytes_base64: '',
  })), /real directory/);
  const removed = await handle(JSON.stringify({
    type: 'file.remove', ...common,
    operation_id: '00000000-0000-4000-8000-000000000153',
    path: '/company/inbox/item/content',
  }));
  assert.equal(removed.type, 'file.removed');
  await assert.rejects(() => readFile(join(root, 'inbox/item/content')), /ENOENT/);
});

test('process execution has no shell, is bounded, and replays idempotently', async () => {
  const root = await mkdtemp(join(tmpdir(), 'restless-runtime-process-'));
  await mkdir(join(root, 'work'));
  const config = parseConfiguration(environment('/run/secrets/bridge.cap'));
  const handle = createCommandHandler(config, { companyRoot: root });
  const command = JSON.stringify({
    type: 'process.run', protocol_version: 1,
    operation_id: '00000000-0000-4000-8000-000000000201',
    runtime_id: 'runtime-1', runtime_generation: 2,
    program: '/usr/bin/true', args: [], cwd: '/company/work', environment: {},
    timeout_ms: 1000, max_output_bytes: 1024,
  });
  const first = await handle(command);
  const replay = await handle(command);
  assert.deepEqual(first, replay);
  assert.equal(first.exit_code, 0);
  assert.equal(first.timed_out, false);
  await assert.rejects(() => handle(JSON.stringify({
    ...JSON.parse(command), program: '/usr/bin/false',
  })), /replay changed shape/);
});

test('TCP streams are loopback-only, bounded and full duplex', async (context) => {
  const server = createServer((socket) => socket.on('data', (bytes) => socket.write(bytes)));
  await new Promise((resolvePromise) => server.listen(0, '127.0.0.1', resolvePromise));
  context.after(() => server.close());
  const address = server.address();
  const config = parseConfiguration(environment('/run/secrets/bridge.cap'));
  const events = [];
  const handle = createCommandHandler(config, { sendEvent: (event) => events.push(event) });
  const operationId = '00000000-0000-4000-8000-000000000301';
  const common = {
    protocol_version: 1, operation_id: operationId,
    runtime_id: 'runtime-1', runtime_generation: 2,
  };
  const opened = await handle(JSON.stringify({
    type: 'stream.open', ...common, host: '127.0.0.1', port: address.port,
  }));
  assert.equal(opened.type, 'stream.opened');
  await handle(JSON.stringify({
    type: 'stream.data', ...common, bytes_base64: Buffer.from('hello').toString('base64'),
  }));
  await new Promise((resolvePromise) => setTimeout(resolvePromise, 10));
  assert.equal(Buffer.from(events[0].bytes_base64, 'base64').toString(), 'hello');
  await handle(JSON.stringify({ type: 'stream.close', ...common }));
  await assert.rejects(() => handle(JSON.stringify({
    type: 'stream.open', ...common,
    operation_id: '00000000-0000-4000-8000-000000000302',
    host: '0.0.0.0', port: address.port,
  })), /loopback/);
});

test('desktop readiness proves the private web transport accepts TCP', async (context) => {
  const server = createServer();
  await new Promise((resolvePromise) => server.listen(0, '127.0.0.1', resolvePromise));
  context.after(() => server.close());
  const address = server.address();
  const config = parseConfiguration(environment('/run/secrets/bridge.cap'));
  const handle = createCommandHandler(config, {
    connectTcp: () => createConnection({ port: address.port, host: '127.0.0.1' }),
  });
  const response = await handle(JSON.stringify({
    type: 'desktop.probe', protocol_version: 1,
    operation_id: '00000000-0000-4000-8000-000000000401',
    runtime_id: 'runtime-1', runtime_generation: 2,
  }));
  assert.deepEqual(response, {
    type: 'desktop.result',
    operation_id: '00000000-0000-4000-8000-000000000401',
    runtime_id: 'runtime-1', runtime_generation: 2,
    status: 'available', host: '127.0.0.1', port: 6080,
  });
});

test('configuration refuses mutable images, plaintext remote transport and ambiguous URLs', () => {
  const base = environment('/run/secrets/bridge.cap');
  assert.throws(() => parseConfiguration({ ...base, RESTLESS_RUNTIME_IMAGE: 'runtime:latest' }));
  assert.throws(() => parseConfiguration({ ...base, RESTLESS_RUNTIME_BRIDGE_URL: 'ws://plane.example.test/internal/v1/runtime-bridge' }));
  assert.throws(() => parseConfiguration({ ...base, RESTLESS_RUNTIME_BRIDGE_URL: 'wss://plane.example.test/internal/v1/runtime-bridge?token=secret' }));
});

test('loopback plaintext is reserved for a real local integration probe', () => {
  const config = parseConfiguration({
    ...environment('/tmp/bridge.cap'),
    RESTLESS_RUNTIME_BRIDGE_URL: 'ws://127.0.0.1:7788/internal/v1/runtime-bridge',
  });
  assert.equal(config.bridgeUrl, 'ws://127.0.0.1:7788/internal/v1/runtime-bridge');
});
