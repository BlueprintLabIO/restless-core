import assert from 'node:assert/strict';
import test from 'node:test';

import { checkReadiness, healthcheckPort } from '../src/healthcheck.js';

test('the container healthcheck targets exact loopback readiness and fails closed', async () => {
  assert.equal(healthcheckPort({}), 6688);
  assert.equal(healthcheckPort({ RESTLESS_NATIVE_DOCUMENTS_LISTEN_PORT: '16688' }), 16688);
  assert.throws(() => healthcheckPort({ RESTLESS_NATIVE_DOCUMENTS_LISTEN_PORT: '0' }));
  assert.throws(() => healthcheckPort({ RESTLESS_NATIVE_DOCUMENTS_LISTEN_PORT: '65536' }));

  let requestedUrl = '';
  const ready = await checkReadiness(
    { RESTLESS_NATIVE_DOCUMENTS_LISTEN_PORT: '16688' },
    (async (input, init) => {
      requestedUrl = String(input);
      assert.equal(init?.redirect, 'error');
      assert.equal(new Headers(init?.headers).get('accept'), 'application/json');
      return new Response('{"status":"ready"}\n', {
        status: 200,
        headers: { 'content-type': 'application/json' },
      });
    }) as typeof fetch,
  );
  assert.equal(ready, true);
  assert.equal(requestedUrl, 'http://127.0.0.1:16688/internal/v1/native-documents/ready');

  assert.equal(
    await checkReadiness({}, (async () => new Response(null, { status: 503 })) as typeof fetch),
    false,
  );
  assert.equal(
    await checkReadiness({}, (async () => {
      throw new Error('unavailable');
    }) as typeof fetch),
    false,
  );
});
