import assert from 'node:assert/strict';
import { mkdtemp, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import { buildRegistration, parseConfiguration } from './runtime-agent.mjs';

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
  assert.deepEqual(registration.supported_features, ['registration.v1']);
  assert.equal(registration.capability, 'r1.payload.signature');
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
