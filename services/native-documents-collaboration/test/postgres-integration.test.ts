import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

import { Client } from 'pg';
import * as Y from 'yjs';
import { blockHash, prepareBodyEdit, checkedBodyUpdate } from '../src/body-edits.js';

import { TOKEN_AUDIENCE } from '../src/constants.js';
import { projectionFromState } from '../src/document-codec.js';
import { DocumentCheckpointChangedError, PostgresDocumentStore } from '../src/postgres-store.js';
import type { CollaborationClaims } from '../src/token-verifier.js';

const migrationUrl = new URL(
  '../../../crates/restless-orgintel/migrations/0053_native_document_live_content.sql',
  import.meta.url,
);

function identifier(value: string): string {
  assert.match(value, /^[a-z][a-z0-9_]{0,62}$/u);
  return value;
}

function databaseUrl(base: string, database: string, username?: string, password?: string): string {
  const value = new URL(base);
  value.pathname = `/${database}`;
  if (username !== undefined) value.username = username;
  if (password !== undefined) value.password = password;
  return value.toString();
}

test('the real Postgres adapter crosses only the migration 0053 capability boundary', async (context) => {
  const adminUrl = process.env.RESTLESS_TEST_DATABASE_URL;
  if (!adminUrl) {
    context.skip('RESTLESS_TEST_DATABASE_URL is unset');
    return;
  }

  const suffix = randomUUID().replaceAll('-', '').slice(0, 16);
  const database = identifier(`sidecar_${suffix}_test`);
  const company = identifier(`docs_${suffix}_test`);
  const role = identifier(`sidecar_${suffix}_role`);
  const password = `Sidecar${suffix}Password`;
  const companyId = randomUUID();
  const documentId = randomUUID();
  const versionId = randomUUID();
  const actorId = 'owner';
  const projection = {
    type: 'doc',
    content: [{
      type: 'paragraph',
      attrs: { block_id: 'opening' },
      content: [{ type: 'text', text: 'Persisted through the narrow capability' }],
    }],
  };

  const admin = new Client({ connectionString: adminUrl });
  let databaseCreated = false;
  let roleCreated = false;
  await admin.connect();
  try {
    await admin.query(
      `CREATE ROLE ${role} WITH LOGIN NOINHERIT NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS PASSWORD '${password}'`,
    );
    roleCreated = true;
    await admin.query(`CREATE DATABASE ${database}`);
    databaseCreated = true;
    await admin.query(`REVOKE CONNECT, TEMPORARY ON DATABASE ${database} FROM PUBLIC`);
    await admin.query(`REVOKE ALL PRIVILEGES ON DATABASE ${database} FROM ${role}`);
    await admin.query(`GRANT CONNECT ON DATABASE ${database} TO ${role}`);
    await admin.query(`ALTER ROLE ${role} IN DATABASE ${database} SET search_path TO ${company}, pg_catalog`);

    const cellAdmin = new Client({ connectionString: databaseUrl(adminUrl, database) });
    await cellAdmin.connect();
    try {
      await cellAdmin.query(`CREATE SCHEMA ${company}`);
      await cellAdmin.query(`SET search_path TO ${company}, pg_catalog`);
      await cellAdmin.query(`
        CREATE TABLE actors (id TEXT PRIMARY KEY, retired_at TIMESTAMPTZ);
        CREATE TABLE company_access_identity (company_id UUID PRIMARY KEY);
        CREATE TABLE native_documents (id UUID PRIMARY KEY, current_named_version_id UUID NOT NULL);
        CREATE TABLE native_document_versions (
          document_id UUID NOT NULL,
          id UUID NOT NULL,
          content_json JSONB NOT NULL,
          content_hash TEXT NOT NULL,
          created_at TIMESTAMPTZ NOT NULL DEFAULT pg_catalog.now(),
          UNIQUE (document_id, id)
        );
      `);
      await cellAdmin.query('INSERT INTO actors (id) VALUES ($1)', [actorId]);
      await cellAdmin.query('INSERT INTO company_access_identity (company_id) VALUES ($1)', [companyId]);
      await cellAdmin.query(
        'INSERT INTO native_documents (id,current_named_version_id) VALUES ($1,$2)',
        [documentId, versionId],
      );
      await cellAdmin.query(
        'INSERT INTO native_document_versions (document_id,id,content_json,content_hash) VALUES ($1,$2,$3,$4)',
        [documentId, versionId, projection, 'a'.repeat(64)],
      );
      await cellAdmin.query(await readFile(migrationUrl, 'utf8'));
      for (const statement of [
        `REVOKE ALL ON SCHEMA ${company} FROM PUBLIC`,
        'REVOKE ALL ON SCHEMA public FROM PUBLIC',
        `GRANT USAGE ON SCHEMA ${company} TO ${role}`,
        `REVOKE ALL ON ALL TABLES IN SCHEMA ${company} FROM ${role}`,
        `REVOKE EXECUTE ON ALL FUNCTIONS IN SCHEMA ${company} FROM PUBLIC`,
        `REVOKE EXECUTE ON ALL FUNCTIONS IN SCHEMA ${company} FROM ${role}`,
        `GRANT EXECUTE ON FUNCTION ${company}.orgintel_native_document_collaboration_consume(UUID,UUID,TEXT,UUID,TEXT,TIMESTAMPTZ,TIMESTAMPTZ) TO ${role}`,
        `GRANT EXECUTE ON FUNCTION ${company}.orgintel_native_document_yjs_load(UUID,UUID) TO ${role}`,
        `GRANT EXECUTE ON FUNCTION ${company}.orgintel_native_document_yjs_store(UUID,UUID,BIGINT,UUID,UUID,BYTEA,JSONB) TO ${role}`,
      ]) await cellAdmin.query(statement);
    } finally {
      await cellAdmin.end();
    }

    const sidecarUrl = databaseUrl(adminUrl, database, role, password);
    const now = Math.floor(Date.now() / 1_000);
    const claims: CollaborationClaims = {
      iss: 'https://restless.run',
      aud: TOKEN_AUDIENCE,
      sub: 'owner-session',
      company_id: companyId,
      document_id: documentId,
      actor_id: actorId,
      access: 'write',
      iat: now,
      nbf: now,
      exp: now + 60,
      jti: randomUUID(),
    };
    const store = new PostgresDocumentStore({ companyId, databaseUrl: sidecarUrl });
    await store.initialize();
    assert.equal(await store.ready(), true);
    assert.equal(await store.consumeSession(claims), true);
    assert.equal(await store.consumeSession(claims), false);
    const concurrent = new PostgresDocumentStore({ companyId, databaseUrl: sidecarUrl });
    await concurrent.initialize();
    const [seed, competingSeed] = await Promise.all([
      store.load({ companyId, documentId }), concurrent.load({ companyId, documentId }),
    ]);
    assert.deepEqual(seed, competingSeed, 'first opens must share the same durable CRDT identities');
    await concurrent.close();
    assert.deepEqual(projectionFromState(seed).type, 'doc');
    assert.equal(store.loadedCheckpoint({ companyId, documentId }), versionId);
    const observed = projectionFromState(seed).content as Record<string, unknown>[];
    const prepared = prepareBodyEdit(seed, { operations: [{ op: 'replace', block_id: 'opening', expected_hash: blockHash(observed[0]), block: { ...projection.content[0], content: [{ type: 'text', text: 'Agent edit survives service restart' }] } }] });
    await store.store({ companyId, documentId, state: seed });
    await store.close();

    const restarted = new PostgresDocumentStore({ companyId, databaseUrl: sidecarUrl });
    await restarted.initialize();
    const persisted = await restarted.load({ companyId, documentId });
    assert.deepEqual(projectionFromState(persisted), projectionFromState(seed));
    assert.equal(restarted.loadedCheckpoint({ companyId, documentId }), versionId);
    const live = new Y.Doc();
    try {
      Y.applyUpdate(live, persisted);
      Y.applyUpdate(live, checkedBodyUpdate(live, prepared.update_base64));
      const transition = new Client({ connectionString: databaseUrl(adminUrl, database) });
      await transition.connect();
      const advancedId = randomUUID();
      try {
        await transition.query(`SET search_path TO ${company}, pg_catalog`);
        await transition.query('BEGIN');
        await transition.query('INSERT INTO native_document_versions (document_id,id,content_json,content_hash) VALUES ($1,$2,$3,$4)', [documentId, advancedId, projection, 'b'.repeat(64)]);
        await transition.query('UPDATE native_documents SET current_named_version_id=$2 WHERE id=$1', [documentId, advancedId]);
        await transition.query('UPDATE native_document_yjs_state SET checkpoint_named_version_id=$2,checkpoint_state_revision=state_revision WHERE document_id=$1', [documentId, advancedId]);
        await transition.query('COMMIT');
      } finally { await transition.end(); }
      await restarted.store({ companyId, documentId, state: Y.encodeStateAsUpdate(live) });
      restarted.release({ companyId, documentId });
      const saved = await restarted.load({ companyId, documentId });
      assert.match(JSON.stringify(projectionFromState(saved)), /Agent edit survives service restart/);
      Y.applyUpdate(live, checkedBodyUpdate(live, prepared.update_base64));
      assert.deepEqual(projectionFromState(Y.encodeStateAsUpdate(live)), projectionFromState(saved));
      assert.deepEqual(Y.encodeStateVectorFromUpdate(Y.encodeStateAsUpdate(live)), Y.encodeStateVectorFromUpdate(saved));
      assert.equal(restarted.loadedCheckpoint({ companyId, documentId }), advancedId);
      // A restore creates a new seed. Even after another client persists that
      // replacement, an old writer must not merge its previous document back.
      const replacementId = randomUUID();
      const restore = new Client({ connectionString: databaseUrl(adminUrl, database) });
      await restore.connect();
      try {
        await restore.query(`SET search_path TO ${company}, pg_catalog`);
        await restore.query('BEGIN');
        await restore.query('INSERT INTO native_document_versions (document_id,id,content_json,content_hash) VALUES ($1,$2,$3,$4)', [documentId, replacementId, projection, 'c'.repeat(64)]);
        await restore.query('UPDATE native_documents SET current_named_version_id=$2 WHERE id=$1', [documentId, replacementId]);
        await restore.query('DELETE FROM native_document_yjs_state WHERE document_id=$1', [documentId]);
        await restore.query('COMMIT');
      } finally { await restore.end(); }
      const replacement = new PostgresDocumentStore({ companyId, databaseUrl: sidecarUrl });
      await replacement.initialize();
      try {
        const replacementBody = await replacement.load({ companyId, documentId });
        await replacement.store({ companyId, documentId, state: replacementBody });
        await assert.rejects(restarted.store({ companyId, documentId, state: saved }), DocumentCheckpointChangedError);
        assert.deepEqual(projectionFromState(await replacement.load({ companyId, documentId })), projection);
      } finally { await replacement.close(); }

    } finally { live.destroy(); await restarted.close(); }

    const denied = new Client({ connectionString: sidecarUrl });
    await denied.connect();
    try {
      await assert.rejects(
        denied.query(`SELECT yjs_state FROM ${company}.native_document_yjs_state`),
        (error: unknown) => typeof error === 'object' && error !== null && 'code' in error && error.code === '42501',
      );
    } finally {
      await denied.end();
    }
  } finally {
    if (databaseCreated) {
      await admin.query('SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname=$1 AND pid<>pg_backend_pid()', [database]);
      await admin.query(`DROP DATABASE ${database}`);
    }
    if (roleCreated) await admin.query(`DROP ROLE ${role}`);
    await admin.end();
  }
});
