import assert from 'node:assert/strict';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { InboundService } from '../candidate/src/inbound-service.mjs';

test('a requirement change supersedes old work and rejects stale completion', () => {
  const directory = mkdtempSync(join(tmpdir(), 'lc2-hidden.'));
  try {
    const path = join(directory, 'state.json');
    let service = new InboundService(path);
    const oldWork = service.admitSignal({ signal_id: 'sig-v1', case_id: 'case-change', requirement_version: 1, facts: ['old'] }).work_id;
    service.beginWork(oldWork);
    service = new InboundService(path);
    const currentWork = service.admitSignal({ signal_id: 'sig-v2', case_id: 'case-change', requirement_version: 2, facts: ['new'] }).work_id;
    assert.equal(service.snapshot().work[oldWork].status, 'superseded');
    assert.deepEqual(service.completeWork({ work_id: oldWork, requirement_version: 1, outcome: 'stale' }), { accepted: false, reason: 'stale-work' });
    service.beginWork(currentWork);
    assert.equal(service.completeWork({ work_id: currentWork, requirement_version: 2, outcome: 'current' }).accepted, true);
    assert.equal(service.snapshot().receipts['case-change'].outcome, 'current');
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test('completion is idempotent across restart and cannot lie about its version', () => {
  const directory = mkdtempSync(join(tmpdir(), 'lc2-hidden.'));
  try {
    const path = join(directory, 'state.json');
    let service = new InboundService(path);
    const workId = service.admitSignal({ signal_id: 'sig-once', case_id: 'case-once', requirement_version: 3, facts: [] }).work_id;
    service.beginWork(workId);
    const wrongVersion = service.completeWork({ work_id: workId, requirement_version: 2, outcome: 'wrong' });
    assert.equal(wrongVersion.accepted, false);
    assert.deepEqual(service.snapshot().receipts, {});
    assert.equal(service.completeWork({ work_id: workId, requirement_version: 3, outcome: 'right' }).accepted, true);
    service = new InboundService(path);
    service.completeWork({ work_id: workId, requirement_version: 3, outcome: 'second' });
    assert.equal(service.snapshot().receipts['case-once'].outcome, 'right');
    assert.equal(service.snapshot().work[workId].status, 'completed');
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
