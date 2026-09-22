import {
  IncomingMessage as HocuspocusIncomingMessage,
  MessageType,
  Server,
  type Connection,
} from '@hocuspocus/server';
import * as Y from 'yjs';
import { bodyView, prepareBodyEdit, checkedBodyUpdate, BodyEditConflict, InvalidBodyEdit } from './body-edits.js';

import type { CollaborationConfig } from './config.js';
import { DocumentCheckpointChangedError } from './store-errors.js';
import {
  AUTHENTICATION_CLOSE_CODE,
  FORBIDDEN_CLOSE_CODE,
  LIVE_PATH,
  MAX_AWARENESS_MESSAGE_BYTES,
  MAX_YJS_STATE_BYTES,
  PROTOCOL_VERSION,
  READY_PATH,
  SCHEMA_VERSION,
  TOKEN_REFRESH_LEAD_SECONDS,
} from './constants.js';
import {
  collaborationDocumentName,
  parseCollaborationPath,
  parseCollaborationTarget,
  type CollaborationTarget,
} from './target.js';
import {
  CollaborationTokenError,
  CoreTokenVerifier,
  type CollaborationClaims,
} from './token-verifier.js';

export interface StoredDocumentInput extends CollaborationTarget {
  readonly state: Uint8Array;
}

export interface DocumentStore {
  initialize(): Promise<void>;
  ready(): Promise<boolean>;
  consumeSession(claims: CollaborationClaims): Promise<boolean>;
  load(target: CollaborationTarget): Promise<Uint8Array>;
  loadedCheckpoint(target: CollaborationTarget): string;
  store(input: StoredDocumentInput): Promise<void>;
  release?(target: CollaborationTarget): void | Promise<void>;
  close(): Promise<void>;
}

interface CollaborationLease {
  claims: CollaborationClaims;
  refreshTimer: NodeJS.Timeout | undefined;
  expiryTimer: NodeJS.Timeout | undefined;
  closed: boolean;
}

interface CollaborationContext {
  session?: {
    readonly target: CollaborationTarget;
    readonly lease: CollaborationLease;
  };
}

export interface CollaborationServerDependencies {
  readonly store: DocumentStore;
  readonly tokenVerifier?: CoreTokenVerifier;
  readonly now?: () => number;
}

export class SocketAuthorizationError extends Error {
  readonly code: number;
  readonly reason: string;

  constructor(code = AUTHENTICATION_CLOSE_CODE, reason = 'Unauthorized') {
    super(reason);
    this.name = 'SocketAuthorizationError';
    this.code = code;
    this.reason = reason;
  }
}

function stopHooks(): Promise<never> {
  return Promise.reject();
}

function requestPath(value: string | undefined): string {
  return value ?? '';
}

function writeJson(
  response: import('node:http').ServerResponse,
  method: string | undefined,
  status: number,
  value: Readonly<Record<string, unknown>>,
): void {
  const body = `${JSON.stringify(value)}\n`;
  response.writeHead(status, {
    'cache-control': 'no-store',
    'content-length': Buffer.byteLength(body),
    'content-type': 'application/json; charset=utf-8',
    'x-content-type-options': 'nosniff',
  });
  response.end(method === 'HEAD' ? undefined : body);
}

function rejectUpgrade(socket: { write(value: string): unknown; destroy(): unknown }): Promise<never> {
  socket.write('HTTP/1.1 404 Not Found\r\nConnection: close\r\nContent-Length: 0\r\n\r\n');
  socket.destroy();
  return stopHooks();
}

function documentTarget(companyId: string, documentName: string): CollaborationTarget {
  const parts = documentName.split(':');
  if (parts.length !== 2) throw new SocketAuthorizationError();
  const target = parseCollaborationPath(`/api/companies/${parts[0] ?? ''}/documents/${parts[1] ?? ''}/collaboration`);
  if (target.companyId !== companyId || collaborationDocumentName(target) !== documentName) {
    throw new SocketAuthorizationError();
  }
  return target;
}

function validateLoadedState(state: Uint8Array): Uint8Array {
  if (!(state instanceof Uint8Array) || state.byteLength < 2 || state.byteLength > MAX_YJS_STATE_BYTES) {
    throw new Error('native Documents durable state is unavailable');
  }
  return state;
}

function inboundMessageType(update: Uint8Array): MessageType {
  try {
    const message = new HocuspocusIncomingMessage(update);
    message.readVarString();
    return message.readVarUint() as MessageType;
  } catch {
    throw new SocketAuthorizationError(FORBIDDEN_CLOSE_CODE, 'Forbidden');
  }
}

function clearLease(lease: CollaborationLease): void {
  lease.closed = true;
  if (lease.refreshTimer) clearTimeout(lease.refreshTimer);
  if (lease.expiryTimer) clearTimeout(lease.expiryTimer);
  lease.refreshTimer = undefined;
  lease.expiryTimer = undefined;
}

function samePrincipal(current: CollaborationClaims, replacement: CollaborationClaims): boolean {
  return (
    current.iss === replacement.iss &&
    current.sub === replacement.sub &&
    current.company_id === replacement.company_id &&
    current.document_id === replacement.document_id &&
    current.actor_id === replacement.actor_id &&
    replacement.exp > current.exp
  );
}

export class NativeDocumentsCollaborationServer {
  private readonly store: DocumentStore;
  private readonly tokenVerifier: CoreTokenVerifier;
  private readonly now: () => number;
  private readonly server: Server<CollaborationContext>;
  private listening = false;
  private initialized = false;
  private destroyed = false;

  constructor(
    private readonly config: CollaborationConfig,
    dependencies: CollaborationServerDependencies,
  ) {
    this.store = dependencies.store;
    this.now = dependencies.now ?? Date.now;
    this.tokenVerifier =
      dependencies.tokenVerifier ??
      new CoreTokenVerifier({ issuer: config.expectedIssuer, jwksUrl: config.jwksUrl, now: this.now });

    this.server = new Server<CollaborationContext>({
      name: 'native-documents-collaboration',
      address: config.address,
      port: config.port,
      quiet: true,
      stopOnSignals: false,
      debounce: config.debounceMs,
      maxDebounce: config.maxDebounceMs,
      unloadImmediately: true,
      timeout: 30_000,
      maxUnauthenticatedQueueSize: 512 * 1024,
      maxUnauthenticatedQueueMessages: 64,
      maxPendingDocuments: 1,
      flushDelay: 0,
      flushMaxBytes: 1024 * 1024,
      websocketOptions: { maxPayload: MAX_YJS_STATE_BYTES + 64 * 1024 },
      onListen: async () => {
        this.listening = true;
      },
      onUpgrade: async ({ request, socket }) => {
        try {
          const target = parseCollaborationPath(requestPath(request.url));
          if (target.companyId !== this.config.companyId) return rejectUpgrade(socket);
        } catch {
          return rejectUpgrade(socket);
        }
      },
      onRequest: async ({ request, response }) => {
        const method = request.method;
        const path = requestPath(request.url);
        if ((method === 'GET' || method === 'HEAD') && path === LIVE_PATH) {
          writeJson(response, method, 200, {
            status: 'live',
            protocol_version: PROTOCOL_VERSION,
            schema_version: SCHEMA_VERSION,
          });
          return stopHooks();
        }
        if ((method === 'GET' || method === 'HEAD') && path === READY_PATH) {
          let ready = false;
          try {
            ready =
              this.listening &&
              this.initialized &&
              (await this.tokenVerifier.probe()) &&
              (await this.store.ready());
          } catch {
            ready = false;
          }
          writeJson(response, method, ready ? 200 : 503, {
            status: ready ? 'ready' : 'not_ready',
            protocol_version: PROTOCOL_VERSION,
            schema_version: SCHEMA_VERSION,
          });
          return stopHooks();
        }
        if ((path === LIVE_PATH || path === READY_PATH) && method !== 'GET' && method !== 'HEAD') {
          writeJson(response, method, 405, { status: 'method_not_allowed' });
          return stopHooks();
        }
        if (method === 'POST' && /\/collaboration\/body$/.test(path)) {
          await this.handleBodyRequest(request, response);
          return stopHooks();
        }
        writeJson(response, method, 404, { status: 'not_found' });
        return stopHooks();
      },
      onConnect: async ({ documentName, request }) => {
        try {
          const target = parseCollaborationTarget(request.url, documentName);
          if (target.companyId !== this.config.companyId) throw new SocketAuthorizationError(FORBIDDEN_CLOSE_CODE, 'Forbidden');
        } catch (error) {
          if (error instanceof SocketAuthorizationError) throw error;
          throw new SocketAuthorizationError(FORBIDDEN_CLOSE_CODE, 'Forbidden');
        }
      },
      onAuthenticate: async ({ connectionConfig, documentName, request, token }) => {
        try {
          const target = parseCollaborationTarget(request.url, documentName);
          if (target.companyId !== this.config.companyId) throw new SocketAuthorizationError();
          const claims = await this.tokenVerifier.verify(token, target);
          if (!(await this.store.consumeSession(claims))) throw new SocketAuthorizationError();
          connectionConfig.readOnly = claims.access !== 'write';
          const lease: CollaborationLease = {
            claims,
            refreshTimer: undefined,
            expiryTimer: undefined,
            closed: false,
          };
          return { session: { target, lease } };
        } catch (error) {
          if (error instanceof SocketAuthorizationError) throw error;
          if (error instanceof CollaborationTokenError) throw new SocketAuthorizationError();
          throw new SocketAuthorizationError();
        }
      },
      connected: async ({ connection, context }) => {
        if (!context.session) {
          connection.close({ code: AUTHENTICATION_CLOSE_CODE, reason: 'Unauthorized' });
          return;
        }
        this.scheduleLease(context.session.lease, connection);
      },
      beforeHandleMessage: async ({ context, documentName, update }) => {
        if (!context.session || collaborationDocumentName(context.session.target) !== documentName) {
          throw new SocketAuthorizationError();
        }
        const type = inboundMessageType(update);
        if (type === MessageType.Stateless || type === MessageType.BroadcastStateless) {
          throw new SocketAuthorizationError(FORBIDDEN_CLOSE_CODE, 'Forbidden');
        }
        if (type === MessageType.Awareness && update.byteLength > MAX_AWARENESS_MESSAGE_BYTES) {
          throw new SocketAuthorizationError(FORBIDDEN_CLOSE_CODE, 'Forbidden');
        }
      },
      beforeHandleAwareness: async ({ connection, context, documentName, states }) => {
        if (
          !connection ||
          !context?.session ||
          collaborationDocumentName(context.session.target) !== documentName
        ) {
          throw new SocketAuthorizationError();
        }
        // Hocuspocus 4.6.0 decodes awareness through a scratch Awareness instance
        // before this hook. That representation cannot reliably distinguish its
        // synthetic local state from a client's first clock-0 state, so identity
        // cannot be rewritten without accepting spoofing or ghost clients. Keep
        // the protocol heartbeat but apply and broadcast no awareness state.
        states.clear();
      },
      onTokenSync: async ({ connection, connectionConfig, context, documentName, token }) => {
        try {
          const session = context.session;
          if (!session) throw new SocketAuthorizationError();
          const target = parseCollaborationTarget(connection.request.url, documentName);
          if (target.companyId !== this.config.companyId || target.documentId !== session.target.documentId) {
            throw new SocketAuthorizationError();
          }
          const claims = await this.tokenVerifier.verify(token, target);
          if (!samePrincipal(session.lease.claims, claims)) throw new SocketAuthorizationError();
          if (!(await this.store.consumeSession(claims))) throw new SocketAuthorizationError();
          session.lease.claims = claims;
          connectionConfig.readOnly = claims.access !== 'write';
          connection.readOnly = connectionConfig.readOnly;
          this.scheduleLease(session.lease, connection);
        } catch (error) {
          if (error instanceof SocketAuthorizationError) throw error;
          throw new SocketAuthorizationError();
        }
      },
      onLoadDocument: async ({ context, document, documentName }) => {
        try {
          if (!context.session || collaborationDocumentName(context.session.target) !== documentName) {
            throw new SocketAuthorizationError();
          }
          return validateLoadedState(await this.store.load(context.session.target));
        } catch (error) {
          // Hocuspocus 4.6.0 does not destroy a Document whose load hook
          // rejects before the instance enters its documents map. Destroy it
          // here so its awareness interval cannot survive a failed load.
          document.destroy();
          throw error;
        }
      },
      onStoreDocument: async ({ document, documentName }) => {
        const target = documentTarget(this.config.companyId, documentName);
        const state = Y.encodeStateAsUpdate(document);
        if (state.byteLength > MAX_YJS_STATE_BYTES) throw new Error('native Documents durable state exceeds the released limit');
        try {
          await this.store.store({ ...target, state });
        } catch (error) {
          if (error instanceof DocumentCheckpointChangedError) {
            // Core moved the immutable checkpoint while this Y.Doc was live.
            // Disconnect and unload this deliberately stale body after the
            // current store mutex releases. The next authenticated connection
            // will seed from Core's new checkpoint.
            const timer = setTimeout(() => {
              this.server.hocuspocus.closeConnections(documentName);
              void this.server.hocuspocus.unloadDocument(document);
            }, 0);
            timer.unref();
          }
          throw error;
        }
      },
      afterUnloadDocument: async ({ documentName }) => {
        const target = documentTarget(this.config.companyId, documentName);
        await this.store.release?.(target);
      },
      onDisconnect: async ({ context }) => {
        if (context.session) clearLease(context.session.lease);
      },
    });
  }

  private async handleBodyRequest(
    request: import('node:http').IncomingMessage,
    response: import('node:http').ServerResponse,
  ): Promise<void> {
    try {
      const target = parseCollaborationPath(requestPath(request.url).slice(0, -5));
      if (target.companyId !== this.config.companyId) throw new SocketAuthorizationError();
      const authorization = request.headers.authorization;
      if (!authorization?.startsWith('Bearer ')) throw new SocketAuthorizationError();
      const claims = await this.tokenVerifier.verify(authorization.slice(7), target);
      // Read the bounded payload before consuming the capability, so a slow upload
      // cannot extend the authorization period or hold a live document connection.
      let size = 0;
      const chunks: Buffer[] = [];
      for await (const chunk of request) {
        size += chunk.length;
        if (size > 12 * 1024 * 1024) throw new InvalidBodyEdit('request is too large');
        chunks.push(Buffer.from(chunk));
      }
      let value: Record<string, unknown>;
      try {
        value = JSON.parse(Buffer.concat(chunks).toString('utf8'));
        if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error();
      } catch { throw new InvalidBodyEdit('expected a JSON object'); }
      const fields = value.action === 'read' ? ['action'] : value.action === 'prepare'
        ? ['action', 'operations'] : value.action === 'apply'
          ? ['action', 'checkpoint_id', 'update_base64'] : [];
      if (!fields.length || Object.keys(value).length !== fields.length || fields.some(key => !(key in value))) {
        throw new InvalidBodyEdit('unknown action or unexpected fields');
      }
      if (value.action !== 'read' && claims.access !== 'write') throw new SocketAuthorizationError(FORBIDDEN_CLOSE_CODE, 'Forbidden');
      if (claims.exp * 1000 <= this.now() || !(await this.store.consumeSession(claims))) throw new SocketAuthorizationError();
      const lease: CollaborationLease = { claims, refreshTimer: undefined, expiryTimer: undefined, closed: false };
      const connection = await this.server.hocuspocus.openDirectConnection(collaborationDocumentName(target), { session: { target, lease } });
      try {
        const document = connection.document!;
        let result: Record<string, unknown> = {};
        await document.saveMutex.runExclusive(async () => {
          if (claims.exp * 1000 <= this.now()) throw new SocketAuthorizationError();
          const checkpoint = this.store.loadedCheckpoint(target);
          if (value.action === 'apply') {
            if (value.checkpoint_id !== checkpoint) throw new BodyEditConflict('document checkpoint changed; read it again');
            const update = checkedBodyUpdate(document, value.update_base64);
            // Validation and mutation share one synchronous turn; no browser update
            // can land between the prospective-body check and application.
            Y.applyUpdate(document, update);
            result = { status: 'applied', checkpoint_id: checkpoint };
          } else {
            const state = Y.encodeStateAsUpdate(document);
            result = { checkpoint_id: checkpoint, ...(value.action === 'read'
              ? bodyView(state) : prepareBodyEdit(state, { operations: value.operations })) };
          }
          // Hocuspocus's disconnect store hook logs/swallow failures. Explicitly
          // await persistence under its mutex before acknowledging a command.
          // This also stabilizes a freshly seeded Yjs body before returning a delta.
          await this.store.store({ ...target, state: Y.encodeStateAsUpdate(document) });
        });
        writeJson(response, request.method, 200, result);
      } finally {
        await connection.disconnect();
      }
    } catch (error) {
      if (response.headersSent) return;
      if (error instanceof SocketAuthorizationError || error instanceof CollaborationTokenError) {
        writeJson(response, request.method, 403, { error: 'forbidden' });
      } else if (error instanceof BodyEditConflict || error instanceof DocumentCheckpointChangedError) {
        writeJson(response, request.method, 409, { error: 'body_conflict' });
      } else if (error instanceof InvalidBodyEdit) {
        writeJson(response, request.method, 400, { error: 'invalid_body_edit' });
      } else {
        writeJson(response, request.method, 503, { error: 'body_unavailable' });
      }
    }
  }

  get httpUrl(): string {
    return this.server.httpURL;
  }

  get webSocketUrl(): string {
    return this.server.webSocketURL;
  }

  async listen(): Promise<void> {
    if (this.destroyed || this.listening) throw new Error('native Documents collaboration server cannot be started');
    try {
      await this.tokenVerifier.initialize();
      await this.store.initialize();
      this.initialized = true;
      await this.server.listen();
    } catch (error) {
      this.destroyed = true;
      await this.store.close().catch(() => undefined);
      throw error;
    }
  }

  async destroy(): Promise<void> {
    if (this.destroyed) return;
    this.destroyed = true;
    this.listening = false;
    try {
      await this.server.destroy();
    } finally {
      await this.store.close();
    }
  }

  private scheduleLease(lease: CollaborationLease, connection: Connection<CollaborationContext>): void {
    if (lease.closed) return;
    if (lease.refreshTimer) clearTimeout(lease.refreshTimer);
    if (lease.expiryTimer) clearTimeout(lease.expiryTimer);
    const remainingMilliseconds = Math.max(0, lease.claims.exp * 1_000 - this.now());
    const refreshMilliseconds = Math.max(0, remainingMilliseconds - TOKEN_REFRESH_LEAD_SECONDS * 1_000);
    lease.refreshTimer = setTimeout(() => {
      if (!lease.closed) connection.requestToken();
    }, refreshMilliseconds);
    lease.expiryTimer = setTimeout(() => {
      if (!lease.closed) {
        clearLease(lease);
        connection.close({ code: AUTHENTICATION_CLOSE_CODE, reason: 'Authorization expired' });
      }
    }, remainingMilliseconds);
    lease.refreshTimer.unref();
    lease.expiryTimer.unref();
  }
}
