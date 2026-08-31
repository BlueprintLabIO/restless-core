import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { InboundService } from '../src/inbound-service.mjs';

function signal(id = 'sig-1', version = 1) {
  return { signal_id: id, case_id: 'case-a', requirement_version: version, facts: [`fact-${version}`] };
}

test('one signal creates durable work and one receipt', () => {
  const directory = mkdtempSync(join(tmpdir(), 'lc2-visible.'));
  try {
    const path = join(directory, 'state.json');
    const service = new InboundService(path);
    const admitted = service.admitSignal(signal());
    assert.equal(admitted.accepted, true);
    assert.equal(service.beginWork(admitted.work_id).accepted, true);
    assert.equal(service.completeWork({ work_id: admitted.work_id, requirement_version: 1, outcome: 'prepared' }).accepted, true);
    assert.equal(new InboundService(path).snapshot().receipts['case-a'].outcome, 'prepared');
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test('an exact repeated signal is ignored after restart', () => {
  const directory = mkdtempSync(join(tmpdir(), 'lc2-visible.'));
  try {
    const path = join(directory, 'state.json');
    assert.equal(new InboundService(path).admitSignal(signal()).accepted, true);
    assert.deepEqual(new InboundService(path).admitSignal(signal()), { accepted: false, reason: 'duplicate-signal' });
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
