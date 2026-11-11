import { mkdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';

function emptyState() {
  return { processed_signal_ids: [], cases: {}, work: {}, receipts: {} };
}

export class InboundService {
  constructor(path) {
    this.path = path;
    this.state = this.#load();
  }

  #load() {
    try {
      return JSON.parse(readFileSync(this.path, 'utf8'));
    } catch (error) {
      if (error.code === 'ENOENT') return emptyState();
      throw error;
    }
  }

  #save() {
    mkdirSync(dirname(this.path), { recursive: true });
    const temporary = `${this.path}.next`;
    writeFileSync(temporary, `${JSON.stringify(this.state)}\n`);
    renameSync(temporary, this.path);
  }

  admitSignal(signal) {
    if (this.state.processed_signal_ids.includes(signal.signal_id)) {
      return { accepted: false, reason: 'duplicate-signal' };
    }
    this.state.processed_signal_ids.push(signal.signal_id);
    this.state.cases[signal.case_id] = {
      case_id: signal.case_id,
      requirement_version: signal.requirement_version,
      facts: signal.facts,
    };
    const workId = `${signal.case_id}:v${signal.requirement_version}`;
    this.state.work[workId] = {
      work_id: workId,
      case_id: signal.case_id,
      requirement_version: signal.requirement_version,
      status: 'ready',
    };
    this.#save();
    return { accepted: true, work_id: workId };
  }

  beginWork(workId) {
    const work = this.state.work[workId];
    if (!work) return { accepted: false, reason: 'unknown-work' };
    work.status = 'running';
    this.#save();
    return { accepted: true, work: structuredClone(work) };
  }

  completeWork(completion) {
    const work = this.state.work[completion.work_id];
    if (!work) return { accepted: false, reason: 'unknown-work' };
    work.status = 'completed';
    const receipt = {
      key: work.case_id,
      work_id: work.work_id,
      requirement_version: completion.requirement_version,
      outcome: completion.outcome,
    };
    this.state.receipts[work.case_id] = receipt;
    this.#save();
    return { accepted: true, receipt: structuredClone(receipt) };
  }

  snapshot() {
    return structuredClone(this.state);
  }
}
