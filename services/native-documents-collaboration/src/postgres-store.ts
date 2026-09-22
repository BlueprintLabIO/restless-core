import { randomUUID } from 'node:crypto';
import { isDeepStrictEqual } from 'node:util';

import { Pool, type PoolConfig } from 'pg';
import * as Y from 'yjs';

import { MAX_YJS_STATE_BYTES } from './constants.js';
import { projectionFromState, stateFromProjection } from './document-codec.js';
import type { DocumentStore, StoredDocumentInput } from './server.js';
import {
  DocumentCheckpointChangedError,
  DocumentPersistenceError,
} from './store-errors.js';
import { collaborationDocumentName, type CollaborationTarget } from './target.js';
import type { CollaborationClaims } from './token-verifier.js';

const MAX_STORE_ATTEMPTS = 3;
const RETRY_BASE_MILLISECONDS = 25;
const STATEMENT_TIMEOUT_MILLISECONDS = 5_000;
const CONNECTION_TIMEOUT_MILLISECONDS = 5_000;
const POOL_FAILURE = 'pool';
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

interface QueryResultLike {
  readonly rows: unknown[];
}

export interface DatabasePool {
  query(text: string, values?: unknown[]): Promise<QueryResultLike>;
  end(): Promise<void>;
  on?(event: 'error', listener: (error: Error) => void): unknown;
}

export interface PostgresDocumentStoreOptions {
  readonly companyId: string;
  readonly databaseUrl: string;
  readonly pool?: DatabasePool;
  readonly makeStoreId?: () => string;
  readonly sleep?: (milliseconds: number) => Promise<void>;
}

interface DocumentRevision {
  readonly checkpointId: string;
  readonly seedId: string;
  readonly revision: bigint;
}

interface LoadRow extends Record<string, unknown> {
  checkpoint_named_version_id: unknown;
  seeded_from_named_version_id: unknown;
  projection_json: unknown;
  source_kind: unknown;
  state_revision: unknown;
  yjs_state: unknown;
}

interface StoreRow extends Record<string, unknown> {
  checkpoint_named_version_id: unknown;
  seeded_from_named_version_id: unknown;
  current_projection_json: unknown;
  current_yjs_state: unknown;
  outcome: unknown;
  state_revision: unknown;
}

export { DocumentCheckpointChangedError, DocumentPersistenceError } from './store-errors.js';

function poolConfiguration(connectionString: string): PoolConfig {
  return {
    application_name: 'restless-native-documents-collaboration',
    connectionString,
    connectionTimeoutMillis: CONNECTION_TIMEOUT_MILLISECONDS,
    idleTimeoutMillis: 30_000,
    keepAlive: true,
    max: 4,
    maxLifetimeSeconds: 300,
    query_timeout: STATEMENT_TIMEOUT_MILLISECONDS + 1_000,
    statement_timeout: STATEMENT_TIMEOUT_MILLISECONDS,
  };
}

function exactRow(value: unknown, fields: readonly string[]): Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) throw new DocumentPersistenceError();
  const row = value as Record<string, unknown>;
  const keys = Object.keys(row).sort();
  const expected = [...fields].sort();
  if (keys.length !== expected.length || keys.some((key, index) => key !== expected[index])) {
    throw new DocumentPersistenceError();
  }
  return row;
}

function oneRow(result: QueryResultLike, fields: readonly string[]): Record<string, unknown> {
  if (result.rows.length !== 1) throw new DocumentPersistenceError();
  return exactRow(result.rows[0], fields);
}

function revision(value: unknown, minimum: bigint): bigint {
  let parsed: bigint;
  try {
    if (typeof value === 'bigint') parsed = value;
    else if (typeof value === 'number' && Number.isSafeInteger(value)) parsed = BigInt(value);
    else if (typeof value === 'string' && /^(?:0|[1-9][0-9]*)$/u.test(value)) parsed = BigInt(value);
    else throw new DocumentPersistenceError();
  } catch {
    throw new DocumentPersistenceError();
  }
  if (parsed < minimum || parsed > 9_223_372_036_854_775_807n) throw new DocumentPersistenceError();
  return parsed;
}

function uuid(value: unknown): string {
  if (typeof value !== 'string' || !UUID.test(value) || value === '00000000-0000-0000-0000-000000000000') {
    throw new DocumentPersistenceError();
  }
  return value;
}

function binaryState(value: unknown): Uint8Array {
  if (!Buffer.isBuffer(value) || value.byteLength < 2 || value.byteLength > MAX_YJS_STATE_BYTES) {
    throw new DocumentPersistenceError();
  }
  return new Uint8Array(value);
}

function errorCode(value: unknown): string | undefined {
  if (typeof value !== 'object' || value === null || !('code' in value)) return undefined;
  return typeof value.code === 'string' ? value.code : undefined;
}

function retryable(value: unknown): boolean {
  const code = errorCode(value);
  return code !== undefined && (
    code.startsWith('08') ||
    code.startsWith('40') ||
    code.startsWith('53') ||
    code === '55P03' ||
    code === '57014' ||
    ['ECONNREFUSED', 'ECONNRESET', 'EPIPE', 'ETIMEDOUT'].includes(code)
  );
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function checkedState(value: Uint8Array): Uint8Array {
  if (!(value instanceof Uint8Array) || value.byteLength < 2 || value.byteLength > MAX_YJS_STATE_BYTES) {
    throw new DocumentPersistenceError();
  }
  return value.slice();
}

export class PostgresDocumentStore implements DocumentStore {
  private readonly companyId: string;
  private readonly pool: DatabasePool;
  private readonly makeStoreId: () => string;
  private readonly sleep: (milliseconds: number) => Promise<void>;
  private readonly documents = new Map<string, DocumentRevision>();
  private readonly failures = new Set<string>();
  private initialized = false;
  private closed = false;

  constructor(options: PostgresDocumentStoreOptions) {
    this.companyId = options.companyId;
    this.pool = options.pool ?? (new Pool(poolConfiguration(options.databaseUrl)) as unknown as DatabasePool);
    this.makeStoreId = options.makeStoreId ?? randomUUID;
    this.sleep = options.sleep ?? delay;
    this.pool.on?.('error', () => {
      this.failures.add(POOL_FAILURE);
    });
  }

  async initialize(): Promise<void> {
    if (this.closed || this.initialized) throw new DocumentPersistenceError();
    try {
      const result = await this.pool.query(
        `SELECT
           current_schema() IS NOT NULL
             AND current_schema() NOT IN ('public', 'pg_catalog', 'information_schema')
             AND current_schema() !~ '^pg_'
             AND current_schemas(false) = ARRAY[current_schema(), 'pg_catalog']::name[]
             AND NOT role.rolsuper
             AND NOT role.rolinherit
             AND NOT role.rolcreaterole
             AND NOT role.rolcreatedb
             AND NOT role.rolreplication
             AND NOT role.rolbypassrls
             AND database.datdba <> role.oid
             AND schema.nspowner <> role.oid
             AND NOT has_database_privilege(current_user, current_database(), 'TEMP')
             AND has_schema_privilege(current_user, current_schema(), 'USAGE')
             AND NOT has_schema_privilege(current_user, current_schema(), 'CREATE')
             AND NOT EXISTS (
               SELECT 1 FROM information_schema.table_privileges grant_row
               WHERE grant_row.table_schema = current_schema()
                 AND grant_row.grantee IN (current_user, 'PUBLIC')
             ) AS boundary,
           to_regprocedure('orgintel_native_document_collaboration_consume(uuid,uuid,text,uuid,text,timestamp with time zone,timestamp with time zone)') IS NOT NULL
             AND has_function_privilege(current_user, to_regprocedure('orgintel_native_document_collaboration_consume(uuid,uuid,text,uuid,text,timestamp with time zone,timestamp with time zone)'), 'EXECUTE') AS consume,
           to_regprocedure('orgintel_native_document_yjs_load(uuid,uuid)') IS NOT NULL
             AND has_function_privilege(current_user, to_regprocedure('orgintel_native_document_yjs_load(uuid,uuid)'), 'EXECUTE') AS load,
           to_regprocedure('orgintel_native_document_yjs_store(uuid,uuid,bigint,uuid,uuid,bytea,jsonb)') IS NOT NULL
             AND has_function_privilege(current_user, to_regprocedure('orgintel_native_document_yjs_store(uuid,uuid,bigint,uuid,uuid,bytea,jsonb)'), 'EXECUTE') AS store
         FROM pg_catalog.pg_roles role
         JOIN pg_catalog.pg_database database ON database.datname = current_database()
         JOIN pg_catalog.pg_namespace schema ON schema.nspname = current_schema()
         WHERE role.rolname = current_user`,
        [],
      );
      const row = oneRow(result, ['boundary', 'consume', 'load', 'store']);
      if (row.boundary !== true || row.consume !== true || row.load !== true || row.store !== true) {
        throw new DocumentPersistenceError();
      }
      this.initialized = true;
      this.failures.delete(POOL_FAILURE);
    } catch {
      this.failures.add(POOL_FAILURE);
      throw new DocumentPersistenceError();
    }
  }

  async ready(): Promise<boolean> {
    if (!this.initialized || this.closed || [...this.failures].some((key) => key !== POOL_FAILURE)) return false;
    try {
      const row = oneRow(await this.pool.query('SELECT 1 AS ready'), ['ready']);
      if (row.ready !== 1) return false;
      this.failures.delete(POOL_FAILURE);
      return true;
    } catch {
      this.failures.add(POOL_FAILURE);
      return false;
    }
  }

  async consumeSession(claims: CollaborationClaims): Promise<boolean> {
    this.assertCompany(claims.company_id);
    try {
      const row = oneRow(
        await this.pool.query(
          'SELECT orgintel_native_document_collaboration_consume($1,$2,$3,$4,$5,$6,$7) AS consumed',
          [
            claims.company_id,
            claims.document_id,
            claims.actor_id,
            claims.jti,
            claims.access,
            new Date(claims.iat * 1_000),
            new Date(claims.exp * 1_000),
          ],
        ),
        ['consumed'],
      );
      this.failures.delete(POOL_FAILURE);
      return row.consumed === true;
    } catch {
      this.failures.add(POOL_FAILURE);
      throw new DocumentPersistenceError();
    }
  }

  async load(target: CollaborationTarget): Promise<Uint8Array> {
    this.assertCompany(target.companyId);
    const key = collaborationDocumentName(target);
    try {
      for (let attempt = 0; attempt < MAX_STORE_ATTEMPTS; attempt += 1) {
        const row = await this.withRetries(async () => oneRow(
          await this.pool.query(
            'SELECT source_kind,state_revision,yjs_state,projection_json,checkpoint_named_version_id,seeded_from_named_version_id FROM orgintel_native_document_yjs_load($1,$2)',
            [target.companyId, target.documentId],
          ),
          ['source_kind', 'state_revision', 'yjs_state', 'projection_json', 'checkpoint_named_version_id', 'seeded_from_named_version_id'],
        ));
        const typed = row as LoadRow;
        const checkpointId = uuid(typed.checkpoint_named_version_id);
        const seedId = uuid(typed.seeded_from_named_version_id);
        let state: Uint8Array;
        let stateRevision: bigint;
        if (typed.source_kind === 'seed') {
          stateRevision = revision(typed.state_revision, 0n);
          if (stateRevision !== 0n || typed.yjs_state !== null) throw new DocumentPersistenceError();
          state = stateFromProjection(typed.projection_json);
          // Establish one durable CRDT seed before any client can observe it.
          // Independently regenerated seeds use different Yjs identities and merge
          // into duplicate blocks on reconnect or concurrent first opens.
          const storeId = this.checkedStoreId();
          const result = await this.withRetries(async () => oneRow(await this.pool.query(
            'SELECT outcome FROM orgintel_native_document_yjs_store($1,$2,$3,$4,$5,$6,$7)',
            [target.companyId, target.documentId, 0n, checkpointId, storeId, Buffer.from(state), projectionFromState(state)],
          ), ['outcome']));
          if (!['stored', 'replayed', 'conflict'].includes(String(result.outcome))) throw new DocumentPersistenceError();
          // Read the winner after CAS, including a concurrent seeder or restore.
          // Never merge a losing seed with the winning CRDT.
          continue;
        } else if (typed.source_kind === 'state') {
          stateRevision = revision(typed.state_revision, 1n);
          state = binaryState(typed.yjs_state);
          if (!isDeepStrictEqual(projectionFromState(state), typed.projection_json)) {
            throw new DocumentPersistenceError('native Documents persisted projections disagree');
          }
        } else {
          throw new DocumentPersistenceError();
        }
        this.documents.set(key, { checkpointId, seedId, revision: stateRevision });
        this.failures.delete(key);
        this.failures.delete(POOL_FAILURE);
        return state;
      }
      throw new DocumentPersistenceError();
    } catch {
      this.failures.add(key);
      throw new DocumentPersistenceError();
    }
  }

  async store(input: StoredDocumentInput): Promise<void> {
    this.assertCompany(input.companyId);
    const key = collaborationDocumentName(input);
    let metadata = this.documents.get(key);
    if (!metadata) {
      this.failures.add(key);
      throw new DocumentPersistenceError();
    }
    let candidate = checkedState(input.state);
    let storeId = this.checkedStoreId();

    try {
      for (let attempt = 0; attempt < MAX_STORE_ATTEMPTS; attempt += 1) {
        const projection = projectionFromState(candidate);
        let row: StoreRow;
        try {
          row = oneRow(
            await this.pool.query(
              'SELECT stored.outcome,stored.state_revision,stored.checkpoint_named_version_id,stored.current_yjs_state,stored.current_projection_json,(SELECT seeded_from_named_version_id FROM orgintel_native_document_yjs_load($1,$2)) AS seeded_from_named_version_id FROM orgintel_native_document_yjs_store($1,$2,$3,$4,$5,$6,$7) stored',
              [
                input.companyId,
                input.documentId,
                metadata.revision,
                metadata.checkpointId,
                storeId,
                Buffer.from(candidate),
                projection,
              ],
            ),
            ['outcome', 'state_revision', 'checkpoint_named_version_id', 'current_yjs_state', 'current_projection_json', 'seeded_from_named_version_id'],
          ) as StoreRow;
        } catch (error) {
          if (!retryable(error) || attempt === MAX_STORE_ATTEMPTS - 1) throw error;
          await this.sleep(RETRY_BASE_MILLISECONDS * (2 ** attempt));
          continue;
        }

        const returnedRevision = revision(row.state_revision, 0n);
        const returnedCheckpoint = uuid(row.checkpoint_named_version_id);
        // Both functions hold the document lock until this SQL statement ends.
        // A new immutable checkpoint may retain the same Yjs lineage; a restore
        // creates a new seed and must never receive these pending old edits.
        const returnedSeed = uuid(row.seeded_from_named_version_id);
        if (returnedSeed !== metadata.seedId) throw new DocumentCheckpointChangedError();
        if (row.outcome === 'stored' || row.outcome === 'replayed') {
          if (
            returnedRevision !== metadata.revision + 1n ||
            returnedCheckpoint !== metadata.checkpointId ||
            row.current_yjs_state !== null ||
            row.current_projection_json !== null
          ) {
            throw new DocumentPersistenceError();
          }
          this.documents.set(key, { checkpointId: returnedCheckpoint, seedId: returnedSeed, revision: returnedRevision });
          this.failures.delete(key);
          this.failures.delete(POOL_FAILURE);
          return;
        }
        if (row.outcome !== 'conflict' || row.current_yjs_state === null) {
          throw new DocumentCheckpointChangedError();
        }
        if (attempt === MAX_STORE_ATTEMPTS - 1) throw new DocumentPersistenceError();
        const current = binaryState(row.current_yjs_state);
        if (!isDeepStrictEqual(projectionFromState(current), row.current_projection_json)) {
          throw new DocumentPersistenceError('native Documents conflict projections disagree');
        }
        candidate = checkedState(Y.mergeUpdates([current, candidate]));
        metadata = { checkpointId: returnedCheckpoint, seedId: returnedSeed, revision: returnedRevision };
        this.documents.set(key, metadata);
        storeId = this.checkedStoreId();
        await this.sleep(RETRY_BASE_MILLISECONDS * (2 ** attempt));
      }
      throw new DocumentPersistenceError();
    } catch (error) {
      this.failures.add(key);
      if (error instanceof DocumentCheckpointChangedError) throw error;
      throw new DocumentPersistenceError();
    }
  }

  loadedCheckpoint(target: CollaborationTarget): string {
    this.assertCompany(target.companyId);
    const revision = this.documents.get(collaborationDocumentName(target));
    if (!revision) throw new DocumentPersistenceError();
    return revision.checkpointId;
  }

  release(target: CollaborationTarget): void {
    const key = collaborationDocumentName(target);
    if (!this.failures.has(key)) this.documents.delete(key);
  }

  async close(): Promise<void> {
    if (this.closed) return;
    this.closed = true;
    this.initialized = false;
    this.documents.clear();
    try {
      await this.pool.end();
    } catch {
      throw new DocumentPersistenceError();
    }
  }

  private assertCompany(companyId: string): void {
    if (companyId !== this.companyId) throw new DocumentPersistenceError();
  }

  private checkedStoreId(): string {
    const id = this.makeStoreId();
    if (!UUID.test(id) || id === '00000000-0000-0000-0000-000000000000') throw new DocumentPersistenceError();
    return id;
  }

  private async withRetries<T>(operation: () => Promise<T>): Promise<T> {
    for (let attempt = 0; attempt < MAX_STORE_ATTEMPTS; attempt += 1) {
      try {
        return await operation();
      } catch (error) {
        if (!retryable(error) || attempt === MAX_STORE_ATTEMPTS - 1) throw error;
        await this.sleep(RETRY_BASE_MILLISECONDS * (2 ** attempt));
      }
    }
    throw new DocumentPersistenceError();
  }
}
