import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { CallbackStore } from '../candidate/src/callback-store.mjs';

test('duplicate delivery remains idempotent after restart', () => {
  const directory = mkdtempSync(join(tmpdir(), 'sc1-hidden.'));
  try {
    const path = join(directory, 'state.json');
    const callback = {
      event_id: 'evt-restart', operation_id: 'op-restart', sequence: 4, kind: 'completed', detail: 'ok',
    };
    assert.equal(new CallbackStore(path).handle(callback).accepted, true);
    const restarted = new CallbackStore(path);
    assert.equal(restarted.handle(callback).accepted, false);
    assert.equal(restarted.snapshot().outbox.length, 1);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test('reordered and newer terminal evidence preserve one keyed outbox entry', () => {
  const directory = mkdtempSync(join(tmpdir(), 'sc1-hidden.'));
  try {
    const store = new CallbackStore(join(directory, 'state.json'));
    store.handle({ event_id: 'evt-7', operation_id: 'op-order', sequence: 7, kind: 'completed', detail: 'settled' });
    store.handle({ event_id: 'evt-5', operation_id: 'op-order', sequence: 5, kind: 'failed', detail: 'late-old-error' });
    assert.equal(store.snapshot().outcomes['op-order'].sequence, 7);
    assert.equal(store.snapshot().outcomes['op-order'].status, 'completed');
    store.handle({ event_id: 'evt-8', operation_id: 'op-order', sequence: 8, kind: 'failed', detail: 'reversed' });
    assert.equal(store.snapshot().outcomes['op-order'].sequence, 8);
    assert.equal(store.snapshot().outcomes['op-order'].status, 'failed');
    assert.equal(store.snapshot().outbox.length, 1);
    assert.equal(store.snapshot().outbox[0].outcome.sequence, 8);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
