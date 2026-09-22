import assert from 'node:assert/strict';
import test from 'node:test';

import * as Y from 'yjs';

import { TOKEN_AUDIENCE } from '../src/constants.js';
import { projectionFromState, stateFromProjection } from '../src/document-codec.js';
import {
  DocumentCheckpointChangedError,
  DocumentPersistenceError,
  PostgresDocumentStore,
  type DatabasePool,
} from '../src/postgres-store.js';
import type { CollaborationClaims } from '../src/token-verifier.js';
import { COMPANY_ID, DOCUMENT_ID, ISSUER } from './helpers.js';

const CHECKPOINT_ID = 'c3333333-3333-4333-8333-333333333333';
const STORE_ID = 'd4444444-4444-4444-8444-444444444444';
const projection = {
  type: 'doc',
  content: [{
    type: 'paragraph',
    attrs: { block_id: 'opening' },
    content: [{ type: 'text', text: 'A shared plan' }],
  }],
};

interface QueryCall {
  readonly text: string;
  readonly values: unknown[];
}

class FakePool implements DatabasePool {
  readonly calls: QueryCall[] = [];
  closed = false;

  constructor(private readonly responses: unknown[]) {}

  async query(text: string, values: unknown[] = []): Promise<{ rows: unknown[] }> {
    this.calls.push({ text, values });
    const response = this.responses.shift();
    if (response instanceof Error) throw response;
    if (!Array.isArray(response)) throw new Error('test response queue is empty');
    return { rows: response };
  }

  async end(): Promise<void> {
    this.closed = true;
  }
}

function capability(boundary = true): unknown[] {
  return [{ boundary, consume: true, load: true, store: true }];
}

function claims(): CollaborationClaims {
  return {
    iss: ISSUER,
    aud: TOKEN_AUDIENCE,
    sub: 'human-user-1',
    company_id: COMPANY_ID,
    document_id: DOCUMENT_ID,
    actor_id: 'owner',
    access: 'write',
    iat: 1_800_000_000,
    nbf: 1_800_000_000,
    exp: 1_800_000_060,
    jti: 'e5555555-5555-4555-8555-555555555555',
  };
}

function seedRow(): unknown[] {
  return [{
    source_kind: 'seed',
    state_revision: '0',
    yjs_state: null,
    projection_json: projection,
    checkpoint_named_version_id: CHECKPOINT_ID,
      seeded_from_named_version_id: CHECKPOINT_ID,
  }];
}

function stateRow(state: Uint8Array): unknown[] {
  return [{
    source_kind: 'state',
    state_revision: '3',
    yjs_state: Buffer.from(state),
    projection_json: projectionFromState(state),
    checkpoint_named_version_id: CHECKPOINT_ID,
      seeded_from_named_version_id: CHECKPOINT_ID,
  }];
}

function connectionError(): Error {
  const error = new Error('simulated transport loss') as Error & { code: string };
  error.code = '08006';
  return error;
}

function appendText(state: Uint8Array, suffix: string): Uint8Array {
  const document = new Y.Doc();
  Y.applyUpdate(document, state);
  const paragraph = document.getXmlFragment('default').get(0);
  assert.ok(paragraph instanceof Y.XmlElement);
  const text = paragraph.get(0);
  assert.ok(text instanceof Y.XmlText);
  text.insert(text.length, suffix);
  const updated = Y.encodeStateAsUpdate(document);
  document.destroy();
  return updated;
}

test('the Postgres store requires the narrow role boundary and calls only the three capability functions', async () => {
  const pool = new FakePool([
    capability(),
    [{ ready: 1 }],
    [{ consumed: true }],
    seedRow(),
    [{ outcome: 'stored' }],
    [{ ...stateRow(stateFromProjection(projection))[0] as object, state_revision: '1' }],
  ]);
  const store = new PostgresDocumentStore({
    companyId: COMPANY_ID,
    databaseUrl: 'postgresql://unused:unused@127.0.0.1/unused',
    pool,
    makeStoreId: () => STORE_ID,
    sleep: async () => undefined,
  });

  await store.initialize();
  assert.equal(await store.ready(), true);
  assert.equal(await store.consumeSession(claims()), true);
  const state = await store.load({ companyId: COMPANY_ID, documentId: DOCUMENT_ID });
  assert.deepEqual(projectionFromState(state), projectionFromState(stateFromProjection(projection)));
  await store.close();
  assert.equal(pool.closed, true);

  assert.match(pool.calls[0]?.text ?? '', /NOT role\.rolsuper/u);
  assert.match(pool.calls[0]?.text ?? '', /NOT EXISTS/u);
  assert.match(pool.calls[2]?.text ?? '', /orgintel_native_document_collaboration_consume/u);
  assert.match(pool.calls[3]?.text ?? '', /orgintel_native_document_yjs_load/u);
  assert.match(pool.calls[4]?.text ?? '', /orgintel_native_document_yjs_store/u);
  const storeValues = pool.calls[4]?.values;
  assert.equal(storeValues?.[0], COMPANY_ID);
  assert.equal(storeValues?.[1], DOCUMENT_ID);
  assert.equal(storeValues?.[2], 0n);
  assert.equal(storeValues?.[3], CHECKPOINT_ID);
  assert.equal(storeValues?.[4], STORE_ID);
  assert.ok(Buffer.isBuffer(storeValues?.[5]));
  assert.deepEqual(storeValues?.[6], projectionFromState(state));
});

test('an uncertain store response retries the exact operation and accepts its replay acknowledgement', async () => {
  const state = stateFromProjection(projection);
  const pool = new FakePool([
    capability(),
    stateRow(state),
    connectionError(),
    [{
      outcome: 'replayed',
      state_revision: '4',
      checkpoint_named_version_id: CHECKPOINT_ID,
      seeded_from_named_version_id: CHECKPOINT_ID,
      current_yjs_state: null,
      current_projection_json: null,
    }],
  ]);
  const store = new PostgresDocumentStore({
    companyId: COMPANY_ID,
    databaseUrl: 'postgresql://unused:unused@127.0.0.1/unused',
    pool,
    makeStoreId: () => STORE_ID,
    sleep: async () => undefined,
  });
  await store.initialize();
  await store.load({ companyId: COMPANY_ID, documentId: DOCUMENT_ID });
  await store.store({ companyId: COMPANY_ID, documentId: DOCUMENT_ID, state });

  const stores = pool.calls.filter((call) => call.text.includes('FROM orgintel_native_document_yjs_store('));
  assert.equal(stores.length, 2);
  assert.deepEqual(stores[0]?.values, stores[1]?.values, 'transport retries must preserve the idempotency payload exactly');
});

test('a live CAS conflict merges the authoritative state and retries from its returned cursor', async () => {
  const base = stateFromProjection(projection);
  const client = appendText(base, ' from client');
  const current = appendText(base, ' from server');
  const storeIds = [STORE_ID, 'f6666666-6666-4666-8666-666666666666'];
  const pool = new FakePool([
    capability(),
    stateRow(base),
    [{
      outcome: 'conflict',
      state_revision: '4',
      checkpoint_named_version_id: CHECKPOINT_ID,
      seeded_from_named_version_id: CHECKPOINT_ID,
      current_yjs_state: Buffer.from(current),
      current_projection_json: projectionFromState(current),
    }],
    [{
      outcome: 'stored',
      state_revision: '5',
      checkpoint_named_version_id: CHECKPOINT_ID,
      seeded_from_named_version_id: CHECKPOINT_ID,
      current_yjs_state: null,
      current_projection_json: null,
    }],
  ]);
  const store = new PostgresDocumentStore({
    companyId: COMPANY_ID,
    databaseUrl: 'postgresql://unused:unused@127.0.0.1/unused',
    pool,
    makeStoreId: () => storeIds.shift() ?? assert.fail('unexpected store id request'),
    sleep: async () => undefined,
  });
  await store.initialize();
  await store.load({ companyId: COMPANY_ID, documentId: DOCUMENT_ID });
  await store.store({ companyId: COMPANY_ID, documentId: DOCUMENT_ID, state: client });

  const stores = pool.calls.filter((call) => call.text.includes('FROM orgintel_native_document_yjs_store('));
  assert.equal(stores.length, 2);
  assert.equal(stores[1]?.values[2], 4n);
  assert.equal(stores[1]?.values[4], 'f6666666-6666-4666-8666-666666666666');
  const merged = projectionFromState(new Uint8Array(stores[1]?.values[5] as Buffer));
  const text = JSON.stringify(merged);
  assert.match(text, /from client/u);
  assert.match(text, /from server/u);
});

test('a same-lineage checkpoint advance preserves pending edits and adopts the new checkpoint', async () => {
  const base = stateFromProjection(projection);
  const client = appendText(base, ' from client');
  const current = appendText(base, ' from server');
  const storeIds = [STORE_ID, 'f6666666-6666-4666-8666-666666666666'];
  const pool = new FakePool([
    capability(),
    stateRow(base),
    [{
      outcome: 'conflict',
      state_revision: '4',
      checkpoint_named_version_id: 'a7777777-7777-4777-8777-777777777777',
      seeded_from_named_version_id: CHECKPOINT_ID,
      current_yjs_state: Buffer.from(current),
      current_projection_json: projectionFromState(current),
    }],
    [{
      outcome: 'stored',
      state_revision: '5',
      checkpoint_named_version_id: 'a7777777-7777-4777-8777-777777777777',
      seeded_from_named_version_id: CHECKPOINT_ID,
      current_yjs_state: null,
      current_projection_json: null,
    }],
  ]);
  const store = new PostgresDocumentStore({
    companyId: COMPANY_ID,
    databaseUrl: 'postgresql://unused:unused@127.0.0.1/unused',
    pool,
    makeStoreId: () => storeIds.shift() ?? assert.fail('unexpected store id request'),
    sleep: async () => undefined,
  });
  await store.initialize();
  await store.load({ companyId: COMPANY_ID, documentId: DOCUMENT_ID });
  await store.store({ companyId: COMPANY_ID, documentId: DOCUMENT_ID, state: client });

  const stores = pool.calls.filter((call) => call.text.includes('FROM orgintel_native_document_yjs_store('));
  assert.equal(stores.length, 2);
  assert.equal(stores[1]?.values[3], 'a7777777-7777-4777-8777-777777777777');
  assert.equal(stores[1]?.values[2], 4n);
  assert.equal(stores[1]?.values[4], 'f6666666-6666-4666-8666-666666666666');
  const merged = projectionFromState(new Uint8Array(stores[1]?.values[5] as Buffer));
  const text = JSON.stringify(merged);
  assert.match(text, /from client/u);
  assert.match(text, /from server/u);
});

test('a replacement seed is surfaced distinctly so the server can evict its stale Y.Doc', async () => {
  const state = stateFromProjection(projection);
  const nextCheckpoint = 'a7777777-7777-4777-8777-777777777777';
  const pool = new FakePool([
    capability(),
    stateRow(state),
    [{
      outcome: 'conflict',
      state_revision: '0',
      checkpoint_named_version_id: nextCheckpoint,
      seeded_from_named_version_id: nextCheckpoint,
      current_yjs_state: null,
      current_projection_json: { type: 'doc', content: [] },
    }],
  ]);
  const store = new PostgresDocumentStore({
    companyId: COMPANY_ID,
    databaseUrl: 'postgresql://unused:unused@127.0.0.1/unused',
    pool,
    makeStoreId: () => STORE_ID,
  });
  await store.initialize();
  await store.load({ companyId: COMPANY_ID, documentId: DOCUMENT_ID });

  await assert.rejects(
    store.store({ companyId: COMPANY_ID, documentId: DOCUMENT_ID, state }),
    DocumentCheckpointChangedError,
  );
  assert.equal(await store.ready(), false);
});

test('persistence or least-privilege failures degrade readiness and disclose no database detail', async () => {
  const broadPool = new FakePool([capability(false)]);
  const broad = new PostgresDocumentStore({
    companyId: COMPANY_ID,
    databaseUrl: 'postgresql://sidecar:top-secret@database.invalid/company',
    pool: broadPool,
  });
  await assert.rejects(broad.initialize(), (error: unknown) => {
    assert.ok(error instanceof DocumentPersistenceError);
    assert.doesNotMatch(error.message, /top-secret|database\.invalid/u);
    return true;
  });

  const state = stateFromProjection(projection);
  const invalid = new Error('simulated invalid write') as Error & { code: string };
  invalid.code = '22023';
  const pool = new FakePool([capability(), stateRow(state), invalid]);
  const store = new PostgresDocumentStore({
    companyId: COMPANY_ID,
    databaseUrl: 'postgresql://sidecar:top-secret@database.invalid/company',
    pool,
    makeStoreId: () => STORE_ID,
  });
  await store.initialize();
  await store.load({ companyId: COMPANY_ID, documentId: DOCUMENT_ID });
  await assert.rejects(
    store.store({ companyId: COMPANY_ID, documentId: DOCUMENT_ID, state }),
    DocumentPersistenceError,
  );
  assert.equal(await store.ready(), false, 'a failed durable store must remain visibly degraded');
});
