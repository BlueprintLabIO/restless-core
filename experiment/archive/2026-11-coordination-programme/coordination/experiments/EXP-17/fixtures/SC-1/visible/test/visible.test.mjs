import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { CallbackStore } from '../src/callback-store.mjs';

test('one ordinary callback creates one terminal outcome', () => {
  const directory = mkdtempSync(join(tmpdir(), 'sc1-visible.'));
  try {
    const store = new CallbackStore(join(directory, 'state.json'));
    assert.equal(store.handle({
      event_id: 'evt-1', operation_id: 'op-1', sequence: 1, kind: 'completed', detail: 'receipt-1',
    }).accepted, true);
    assert.equal(store.snapshot().outcomes['op-1'].status, 'completed');
    assert.equal(store.snapshot().outbox.length, 1);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test('an immediate duplicate event is ignored', () => {
  const directory = mkdtempSync(join(tmpdir(), 'sc1-visible.'));
  try {
    const path = join(directory, 'state.json');
    const store = new CallbackStore(path);
    const callback = {
      event_id: 'evt-2', operation_id: 'op-2', sequence: 3, kind: 'failed', detail: 'declined',
    };
    assert.equal(store.handle(callback).accepted, true);
    assert.deepEqual(store.handle(callback), { accepted: false, reason: 'duplicate-event' });
    assert.equal(store.snapshot().outbox.length, 1);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
