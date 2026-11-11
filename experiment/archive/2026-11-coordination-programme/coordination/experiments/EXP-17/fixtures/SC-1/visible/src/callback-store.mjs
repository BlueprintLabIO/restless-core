import { mkdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';

function emptyState() {
  return { outcomes: {}, processed_event_ids: [], outbox: [] };
}

export class CallbackStore {
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

  handle(callback) {
    if (this.state.processed_event_ids.includes(callback.event_id)) {
      return { accepted: false, reason: 'duplicate-event' };
    }

    this.state.processed_event_ids.push(callback.event_id);
    const outcome = {
      operation_id: callback.operation_id,
      sequence: callback.sequence,
      status: callback.kind === 'completed' ? 'completed' : 'failed',
      detail: callback.detail,
    };
    this.state.outcomes[callback.operation_id] = outcome;
    this.state.outbox.push({
      key: callback.operation_id,
      type: 'operation-terminal',
      outcome,
    });
    this.#save();
    return { accepted: true, outcome };
  }

  snapshot() {
    return structuredClone(this.state);
  }
}
