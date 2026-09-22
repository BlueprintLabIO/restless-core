import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { stateFromProjection, projectionFromState } from '../src/document-codec.js';
import test from 'node:test';

import { HocuspocusProvider, HocuspocusProviderWebsocket } from '@hocuspocus/provider';
import * as Y from 'yjs';

import type { CollaborationConfig } from '../src/config.js';
import { LIVE_PATH, READY_PATH } from '../src/constants.js';
import { DocumentCheckpointChangedError } from '../src/store-errors.js';
import {
  NativeDocumentsCollaborationServer,
  type DocumentStore,
  type StoredDocumentInput,
} from '../src/server.js';
import { collaborationDocumentName, type CollaborationTarget } from '../src/target.js';
import { CoreTokenVerifier, type CollaborationClaims } from '../src/token-verifier.js';
import {
  COMPANY_ID,
  DOCUMENT_ID,
  ISSUER,
  JWKS_URL,
  OTHER_DOCUMENT_ID,
  jwksResponse,
  signingFixture,
} from './helpers.js';

class MemoryDocumentStore implements DocumentStore {
  readonly sessions = new Set<string>();
  loadCount = 0;
  storeCount = 0;
  initialized = false;
  closed = false;

  constructor(public state: Uint8Array) {}

  async initialize(): Promise<void> {
    this.initialized = true;
  }

  async ready(): Promise<boolean> {
    return this.initialized && !this.closed;
  }

  async consumeSession(claims: CollaborationClaims): Promise<boolean> {
    if (this.sessions.has(claims.jti)) return false;
    this.sessions.add(claims.jti);
    return true;
  }

  async load(target: CollaborationTarget): Promise<Uint8Array> {
    assert.deepEqual(target, { companyId: COMPANY_ID, documentId: DOCUMENT_ID });
    this.loadCount += 1;
    return this.state.slice();
  }

  async store(input: StoredDocumentInput): Promise<void> {
    assert.equal(input.companyId, COMPANY_ID);
    assert.equal(input.documentId, DOCUMENT_ID);
    this.storeCount += 1;
    this.state = input.state.slice();
  }

  loadedCheckpoint(_target: CollaborationTarget): string { return DOCUMENT_ID; }

  async close(): Promise<void> {
    this.closed = true;
  }
}

class GatedDocumentStore extends MemoryDocumentStore {
  consumeCount = 0;

  constructor(state: Uint8Array, private readonly gate: Promise<void>) {
    super(state);
  }

  override async consumeSession(claims: CollaborationClaims): Promise<boolean> {
    this.consumeCount += 1;
    await this.gate;
    return super.consumeSession(claims);
  }
}

class MovingCheckpointStore extends MemoryDocumentStore {
  moved = false;

  override async store(input: StoredDocumentInput): Promise<void> {
    if (this.moved) return super.store(input);
    this.storeCount += 1;
    this.moved = true;
    const replacement = new Y.Doc();
    replacement.getText('content').insert(0, 'replacement');
    this.state = Y.encodeStateAsUpdate(replacement);
    replacement.destroy();
    throw new DocumentCheckpointChangedError();
  }
}

async function eventually(check: () => boolean, label: string, timeoutMilliseconds = 4_000): Promise<void> {
  const deadline = Date.now() + timeoutMilliseconds;
  while (Date.now() < deadline) {
    if (check()) return;
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  assert.fail(`timed out waiting for ${label}`);
}

function documentText(state: Uint8Array): string {
  const document = new Y.Doc();
  Y.applyUpdate(document, state);
  return document.getText('content').toString();
}

test('the real Hocuspocus boundary authenticates before load and enforces read-only updates', async (context) => {
  const nowSeconds = Math.floor(Date.now() / 1_000);
  const signer = await signingFixture(nowSeconds);
  const seed = new Y.Doc();
  seed.getText('content').insert(0, 'seed');
  const store = new MemoryDocumentStore(Y.encodeStateAsUpdate(seed));
  const verifier = new CoreTokenVerifier({
    issuer: ISSUER,
    jwksUrl: new URL(JWKS_URL),
    fetch: (async () => jwksResponse(signer.jwks)) as typeof fetch,
  });
  const config: CollaborationConfig = {
    address: '127.0.0.1',
    port: 0,
    companyId: COMPANY_ID,
    expectedIssuer: ISSUER,
    jwksUrl: new URL(JWKS_URL),
    databaseUrl: 'postgresql://unused:unused@127.0.0.1/unused',
    debounceMs: 25,
    maxDebounceMs: 100,
  };
  const server = new NativeDocumentsCollaborationServer(config, { store, tokenVerifier: verifier });
  await server.listen();
  const providers: HocuspocusProvider[] = [];
  context.after(async () => {
    for (const provider of providers) provider.destroy();
    await server.destroy();
  });

  const live = await fetch(`${server.httpUrl}${LIVE_PATH}`);
  assert.equal(live.status, 200);
  assert.deepEqual(await live.json(), { status: 'live', protocol_version: 1, schema_version: 1 });
  const ready = await fetch(`${server.httpUrl}${READY_PATH}`);
  assert.equal(ready.status, 200);
  assert.equal((await ready.json() as { status: string }).status, 'ready');
  assert.equal((await fetch(`${server.httpUrl}/`)).status, 404);

  const target = { companyId: COMPANY_ID, documentId: DOCUMENT_ID } as const;
  const url = `${server.webSocketUrl}/api/companies/${COMPANY_ID}/documents/${DOCUMENT_ID}/collaboration`;
  const name = collaborationDocumentName(target);
  const wrongDocumentToken = await signer.sign({ document_id: OTHER_DOCUMENT_ID });
  let wrongDocumentRejected = false;
  const unauthorized = new HocuspocusProvider({
    url,
    name,
    token: wrongDocumentToken,
    document: new Y.Doc(),
    onAuthenticationFailed: () => {
      wrongDocumentRejected = true;
    },
  });
  providers.push(unauthorized);
  await eventually(() => wrongDocumentRejected, 'wrong-document rejection');
  assert.equal(store.loadCount, 0, 'authorization must finish before any durable content load');
  unauthorized.destroy();

  const readToken = await signer.sign({ access: 'read' });
  const readDocument = new Y.Doc();
  let statelessDenied = false;
  const reader = new HocuspocusProvider({
    url,
    name,
    token: readToken,
    document: readDocument,
    onClose: ({ event }) => {
      if (event.reason === 'Forbidden') statelessDenied = true;
    },
  });
  providers.push(reader);
  await eventually(() => reader.isAuthenticated && reader.isSynced, 'read-only client sync');
  assert.equal(readDocument.getText('content').toString(), 'seed');
  assert.equal(store.loadCount, 1);

  readDocument.getText('content').insert(4, '-forbidden');
  await new Promise((resolve) => setTimeout(resolve, 75));

  const writeToken = await signer.sign({
    access: 'write',
    jti: '55555555-5555-4555-8555-555555555555',
  });
  const writeDocument = new Y.Doc();
  const writer = new HocuspocusProvider({ url, name, token: writeToken, document: writeDocument });
  providers.push(writer);
  await eventually(() => writer.isAuthenticated && writer.isSynced, 'writer sync');
  assert.equal(writeDocument.getText('content').toString(), 'seed', 'reader updates must never enter the server document');

  reader.awareness?.setLocalState({
    actor_id: 'forged-actor',
    access: 'write',
    user: { id: 'forged-user', name: 'Mallory', color: '#000000' },
    cursor: { anchor: 3, head: 4 },
    arbitrary: { identity: 'must-not-cross-the-boundary' },
  });
  await new Promise((resolve) => setTimeout(resolve, 75));
  assert.equal(
    writer.awareness?.getStates().has(readDocument.clientID),
    false,
    'untrusted awareness identity must not cross the server boundary',
  );
  assert.equal(reader.isAuthenticated, true, 'discarded awareness must not break document synchronization');

  writeDocument.getText('content').insert(4, '-writer');
  await eventually(() => readDocument.getText('content').toString().includes('-writer'), 'writer fan-out');
  await eventually(() => store.storeCount > 0, 'durable store hook');
  assert.equal(documentText(store.state), 'seed-writer');

  let replayRejected = false;
  const replay = new HocuspocusProvider({
    url,
    name,
    token: writeToken,
    document: new Y.Doc(),
    sessionAwareness: true,
    onAuthenticationFailed: () => {
      replayRejected = true;
    },
  });
  providers.push(replay);
  await eventually(() => replayRejected, 'replayed session rejection');
  assert.equal(replay.isAuthenticated, false);
  assert.equal(store.sessions.size, 2);

  reader.sendStateless('this channel is deliberately disabled');
  await eventually(() => statelessDenied, 'stateless message rejection');
  assert.equal(reader.isAuthenticated, false);
});

test('a persisted state below the database minimum is rejected before document sync', async (context) => {
  const nowSeconds = Math.floor(Date.now() / 1_000);
  const signer = await signingFixture(nowSeconds);
  const store = new MemoryDocumentStore(Uint8Array.of(0));
  const verifier = new CoreTokenVerifier({
    issuer: ISSUER,
    jwksUrl: new URL(JWKS_URL),
    fetch: (async () => jwksResponse(signer.jwks)) as typeof fetch,
  });
  const config: CollaborationConfig = {
    address: '127.0.0.1',
    port: 0,
    companyId: COMPANY_ID,
    expectedIssuer: ISSUER,
    jwksUrl: new URL(JWKS_URL),
    databaseUrl: 'postgresql://unused:unused@127.0.0.1/unused',
    debounceMs: 25,
    maxDebounceMs: 100,
  };
  const server = new NativeDocumentsCollaborationServer(config, { store, tokenVerifier: verifier });
  await server.listen();
  let rejected = false;
  const provider = new HocuspocusProvider({
    url: `${server.webSocketUrl}/api/companies/${COMPANY_ID}/documents/${DOCUMENT_ID}/collaboration`,
    name: collaborationDocumentName({ companyId: COMPANY_ID, documentId: DOCUMENT_ID }),
    token: await signer.sign(),
    document: new Y.Doc(),
    onAuthenticationFailed: () => {
      rejected = true;
    },
  });
  context.after(async () => {
    provider.destroy();
    await server.destroy();
  });

  await eventually(() => rejected, 'invalid durable-state rejection');
  assert.equal(store.loadCount, 1);
  assert.equal(store.storeCount, 0);
  assert.equal(provider.isSynced, false);
});

test('an immutable checkpoint move evicts the stale live document before the next connection', async (context) => {
  const nowSeconds = Math.floor(Date.now() / 1_000);
  const signer = await signingFixture(nowSeconds);
  const seed = new Y.Doc();
  seed.getText('content').insert(0, 'seed');
  const store = new MovingCheckpointStore(Y.encodeStateAsUpdate(seed));
  const verifier = new CoreTokenVerifier({
    issuer: ISSUER,
    jwksUrl: new URL(JWKS_URL),
    fetch: (async () => jwksResponse(signer.jwks)) as typeof fetch,
  });
  const config: CollaborationConfig = {
    address: '127.0.0.1',
    port: 0,
    companyId: COMPANY_ID,
    expectedIssuer: ISSUER,
    jwksUrl: new URL(JWKS_URL),
    databaseUrl: 'postgresql://unused:unused@127.0.0.1/unused',
    debounceMs: 10,
    maxDebounceMs: 25,
  };
  const server = new NativeDocumentsCollaborationServer(config, { store, tokenVerifier: verifier });
  await server.listen();
  const providers: HocuspocusProvider[] = [];
  context.after(async () => {
    for (const provider of providers) provider.destroy();
    await server.destroy();
  });

  const target = { companyId: COMPANY_ID, documentId: DOCUMENT_ID } as const;
  const url = `${server.webSocketUrl}/api/companies/${COMPANY_ID}/documents/${DOCUMENT_ID}/collaboration`;
  const firstDocument = new Y.Doc();
  let checkpointClosed = false;
  let first!: HocuspocusProvider;
  first = new HocuspocusProvider({
    url,
    name: collaborationDocumentName(target),
    token: await signer.sign(),
    document: firstDocument,
    onClose: () => {
      if (store.moved) {
        checkpointClosed = true;
        first.destroy();
      }
    },
  });
  providers.push(first);
  await eventually(() => first.isAuthenticated && first.isSynced, 'initial checkpoint sync');
  firstDocument.getText('content').insert(4, '-stale');
  await eventually(() => checkpointClosed, 'stale checkpoint eviction');

  const replacementDocument = new Y.Doc();
  const second = new HocuspocusProvider({
    url,
    name: collaborationDocumentName(target),
    token: await signer.sign({ jti: 'a7777777-7777-4777-8777-777777777777' }),
    document: replacementDocument,
  });
  providers.push(second);
  await eventually(() => second.isAuthenticated && second.isSynced, 'replacement checkpoint sync');
  assert.equal(replacementDocument.getText('content').toString(), 'replacement');
  assert.equal(store.loadCount, 2);
});

test('one socket cannot fan out across multiple pending document sessions', async (context) => {
  const nowSeconds = Math.floor(Date.now() / 1_000);
  const signer = await signingFixture(nowSeconds);
  const seed = new Y.Doc();
  seed.getText('content').insert(0, 'seed');
  let releaseConsume!: () => void;
  const consumeGate = new Promise<void>((resolve) => {
    releaseConsume = resolve;
  });
  const store = new GatedDocumentStore(Y.encodeStateAsUpdate(seed), consumeGate);
  const verifier = new CoreTokenVerifier({
    issuer: ISSUER,
    jwksUrl: new URL(JWKS_URL),
    fetch: (async () => jwksResponse(signer.jwks)) as typeof fetch,
  });
  const config: CollaborationConfig = {
    address: '127.0.0.1',
    port: 0,
    companyId: COMPANY_ID,
    expectedIssuer: ISSUER,
    jwksUrl: new URL(JWKS_URL),
    databaseUrl: 'postgresql://unused:unused@127.0.0.1/unused',
    debounceMs: 25,
    maxDebounceMs: 100,
  };
  const server = new NativeDocumentsCollaborationServer(config, { store, tokenVerifier: verifier });
  await server.listen();

  let closedForFanout = false;
  let socket!: HocuspocusProviderWebsocket;
  socket = new HocuspocusProviderWebsocket({
    url: `${server.webSocketUrl}/api/companies/${COMPANY_ID}/documents/${DOCUMENT_ID}/collaboration`,
    autoConnect: false,
    maxAttempts: 1,
    onClose: ({ event }) => {
      if (event.code === 4205) {
        closedForFanout = true;
        socket.disconnect();
      }
    },
  });
  const name = collaborationDocumentName({ companyId: COMPANY_ID, documentId: DOCUMENT_ID });
  const first = new HocuspocusProvider({
    websocketProvider: socket,
    sessionAwareness: true,
    name,
    token: await signer.sign({ jti: '88888888-8888-4888-8888-888888888888' }),
    document: new Y.Doc(),
  });
  const second = new HocuspocusProvider({
    websocketProvider: socket,
    sessionAwareness: true,
    name,
    token: await signer.sign({ jti: '99999999-9999-4999-8999-999999999999' }),
    document: new Y.Doc(),
  });
  first.attach();
  second.attach();
  context.after(async () => {
    releaseConsume();
    first.destroy();
    second.destroy();
    socket.destroy();
    await server.destroy();
  });

  void socket.connect().catch(() => undefined);
  await eventually(() => closedForFanout, 'pending-document fan-out rejection');
  assert.equal(store.loadCount, 0, 'pending fan-out must close before content load');
  assert.ok(store.consumeCount <= 1, 'at most one pending document may enter authentication');
  releaseConsume();
});

test('an active connection refreshes before expiry and an authoritative downgrade takes effect', async (context) => {
  const nowSeconds = Math.floor(Date.now() / 1_000);
  const signer = await signingFixture(nowSeconds);
  const seed = new Y.Doc();
  seed.getText('content').insert(0, 'seed');
  const store = new MemoryDocumentStore(Y.encodeStateAsUpdate(seed));
  const now = () => nowSeconds * 1_000;
  const verifier = new CoreTokenVerifier({
    issuer: ISSUER,
    jwksUrl: new URL(JWKS_URL),
    now,
    fetch: (async () => jwksResponse(signer.jwks)) as typeof fetch,
  });
  const config: CollaborationConfig = {
    address: '127.0.0.1',
    port: 0,
    companyId: COMPANY_ID,
    expectedIssuer: ISSUER,
    jwksUrl: new URL(JWKS_URL),
    databaseUrl: 'postgresql://unused:unused@127.0.0.1/unused',
    debounceMs: 25,
    maxDebounceMs: 100,
  };
  const server = new NativeDocumentsCollaborationServer(config, { store, tokenVerifier: verifier, now });
  await server.listen();
  const providers: HocuspocusProvider[] = [];
  context.after(async () => {
    for (const provider of providers) provider.destroy();
    await server.destroy();
  });

  const initial = await signer.sign({ exp: nowSeconds + 15 });
  const refreshed = await signer.sign({
    access: 'read',
    exp: nowSeconds + 60,
    jti: '66666666-6666-4666-8666-666666666666',
  });
  let tokenRequests = 0;
  const target = { companyId: COMPANY_ID, documentId: DOCUMENT_ID } as const;
  const url = `${server.webSocketUrl}/api/companies/${COMPANY_ID}/documents/${DOCUMENT_ID}/collaboration`;
  const clientDocument = new Y.Doc();
  const client = new HocuspocusProvider({
    url,
    name: collaborationDocumentName(target),
    document: clientDocument,
    token: async () => {
      tokenRequests += 1;
      return tokenRequests === 1 ? initial : refreshed;
    },
  });
  providers.push(client);
  await eventually(() => client.isAuthenticated && client.isSynced && store.sessions.size === 2, 'lease refresh');
  assert.equal(tokenRequests, 2);

  clientDocument.getText('content').insert(4, '-blocked-after-downgrade');
  await new Promise((resolve) => setTimeout(resolve, 75));
  const observerToken = await signer.sign({
    access: 'read',
    jti: '77777777-7777-4777-8777-777777777777',
  });
  const observerDocument = new Y.Doc();
  const observer = new HocuspocusProvider({
    url,
    name: collaborationDocumentName(target),
    token: observerToken,
    document: observerDocument,
  });
  providers.push(observer);
  await eventually(() => observer.isAuthenticated && observer.isSynced, 'observer sync');
  assert.equal(observerDocument.getText('content').toString(), 'seed');
});


test('body HTTP commands share the browser Y.Doc, enforce capabilities and acknowledge durable edits only', async (context) => {
  const paragraph = (text: string) => ({ type: 'paragraph', attrs: { block_id: 'opening' }, content: [{ type: 'text', text }] });
  class BodyStore extends MemoryDocumentStore {
    fail = false;
    override async store(input: StoredDocumentInput): Promise<void> {
      if (this.fail) throw new Error('simulated persistence failure');
      await super.store(input);
    }
  }
  const store = new BodyStore(stateFromProjection({ type: 'doc', content: [paragraph('Opening')] }));
  const signer = await signingFixture(Math.floor(Date.now() / 1000));
  const verifier = new CoreTokenVerifier({ issuer: ISSUER, jwksUrl: new URL(JWKS_URL), fetch: (async () => jwksResponse(signer.jwks)) as typeof fetch });
  const server = new NativeDocumentsCollaborationServer({ address: '127.0.0.1', port: 0, companyId: COMPANY_ID, expectedIssuer: ISSUER, jwksUrl: new URL(JWKS_URL), databaseUrl: 'postgresql://unused', debounceMs: 25, maxDebounceMs: 100 }, { store, tokenVerifier: verifier });
  await server.listen();
  const browserDoc = new Y.Doc();
  const provider = new HocuspocusProvider({ url: `${server.webSocketUrl}/api/companies/${COMPANY_ID}/documents/${DOCUMENT_ID}/collaboration`, name: collaborationDocumentName({ companyId: COMPANY_ID, documentId: DOCUMENT_ID }), document: browserDoc, token: await signer.sign({ jti: randomUUID() }) });
  context.after(async () => { store.fail = false; provider.destroy(); browserDoc.destroy(); await server.destroy(); });
  await eventually(() => provider.isSynced, 'browser document');
  const url = `${server.httpUrl}/api/companies/${COMPANY_ID}/documents/${DOCUMENT_ID}/collaboration/body`;
  const command = async (body: unknown, overrides: Record<string, unknown> = {}) => fetch(url, { method: 'POST', headers: { authorization: `Bearer ${await signer.sign({ jti: randomUUID(), ...overrides })}` }, body: JSON.stringify(body) });
  assert.equal((await fetch(url, { method: 'POST', body: '{}' })).status, 403);
  assert.equal((await command({ action: 'read' }, { document_id: OTHER_DOCUMENT_ID })).status, 403);
  const read = await command({ action: 'read' }, { access: 'read' });
  assert.equal(read.status, 200);
  const view = await read.json() as { blocks: { hash: string }[]; checkpoint_id: string };
  const edit = { action: 'prepare', operations: [{ op: 'replace', block_id: 'opening', expected_hash: view.blocks[0]!.hash, block: paragraph('Improved opening') }] };
  assert.equal((await command(edit, { access: 'read' })).status, 403);
  assert.equal((await command({ action: 'apply', checkpoint_id: DOCUMENT_ID, update_base64: '!!!' })).status, 400);
  assert.equal((await command({ action: 'prepare', operations: [{ op: 'insert', after: null, block: { type: 'script', attrs: { block_id: 'bad' } } }] })).status, 400);
  const preparedResponse = await command(edit);
  assert.equal(preparedResponse.status, 200);
  const prepared = await preparedResponse.json() as { update_base64: string; checkpoint_id: string };
  const browserText = () => (browserDoc.getXmlFragment('default').get(0) as Y.XmlElement).get(0) as Y.XmlText;
  browserText().insert(browserText().length, ' plus human text');
  await eventually(() => JSON.stringify(projectionFromState(store.state)).includes('plus human text'), 'human persistence');
  const apply = { action: 'apply', checkpoint_id: prepared.checkpoint_id, update_base64: prepared.update_base64 };
  assert.equal((await command({ ...apply, checkpoint_id: OTHER_DOCUMENT_ID })).status, 409);
  assert.equal((await command(apply)).status, 200);
  await eventually(() => browserText().toString().includes('Improved opening'), 'agent fan-out');
  assert.match(browserText().toString(), /plus human text/);
  assert.match(JSON.stringify(projectionFromState(store.state)), /Improved opening/);
  const before = Y.encodeStateAsUpdate(browserDoc);
  assert.equal((await command(apply)).status, 200);
  assert.deepEqual(Y.encodeStateAsUpdate(browserDoc), before, 'replay must not duplicate the edit');
  assert.equal((await command(edit)).status, 409, 'stale observed block must be rejected');
  const jti = randomUUID();
  assert.equal((await command({ action: 'read' }, { jti })).status, 200);
  assert.equal((await command({ action: 'read' }, { jti })).status, 403);
  store.fail = true;
  assert.equal((await command(apply)).status, 503, 'a failed store must never receive a successful acknowledgement');
  store.fail = false;
  assert.equal((await command(apply)).status, 200, 'the exact edit can recover after persistence failure');
  provider.destroy();
  await new Promise(resolve => setTimeout(resolve, 150));
  const reload = await command({ action: 'read' });
  assert.equal(reload.status, 200);
  const reloaded = await reload.json() as { blocks: { hash: string }[] };
  const headless = await command({ action: 'prepare', operations: [{ op: 'replace', block_id: 'opening', expected_hash: reloaded.blocks[0]!.hash, block: paragraph('Edited with no browser connected') }] });
  assert.equal(headless.status, 200);
  const delta = await headless.json() as { checkpoint_id: string; update_base64: string };
  await new Promise(resolve => setTimeout(resolve, 100));
  assert.equal((await command({ action: 'apply', checkpoint_id: delta.checkpoint_id, update_base64: delta.update_base64 })).status, 200);
  assert.match(JSON.stringify(projectionFromState(store.state)), /Edited with no browser connected/);

});
