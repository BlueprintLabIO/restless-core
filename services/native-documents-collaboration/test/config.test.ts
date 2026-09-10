import assert from 'node:assert/strict';
import { chmod, mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';

import { configFromEnvironment } from '../src/config.js';
import { COMPANY_ID, ISSUER, JWKS_URL } from './helpers.js';

test('configuration separates the public token issuer from an exact private JWKS service', async (context) => {
  const directory = await mkdtemp(join(tmpdir(), 'restless-native-documents-config-'));
  const credential = join(directory, 'database.url');
  context.after(() => rm(directory, { recursive: true, force: true }));
  await writeFile(credential, 'postgresql://sidecar:secret@127.0.0.1:55432/company\n', { mode: 0o600 });
  const environment: NodeJS.ProcessEnv = {
    RESTLESS_NATIVE_DOCUMENTS_COMPANY_ID: COMPANY_ID,
    RESTLESS_NATIVE_DOCUMENTS_TOKEN_ISSUER: ISSUER,
    RESTLESS_NATIVE_DOCUMENTS_JWKS_URL: JWKS_URL,
    RESTLESS_NATIVE_DOCUMENTS_STORE_CREDENTIAL_FILE: credential,
  };
  const config = await configFromEnvironment(environment);
  assert.equal(config.companyId, COMPANY_ID);
  assert.equal(config.databaseUrl, 'postgresql://sidecar:secret@127.0.0.1:55432/company');
  assert.equal(config.expectedIssuer, ISSUER);
  assert.equal(config.jwksUrl.toString(), JWKS_URL);

  for (const invalidJwksUrl of [
    'https://keys.invalid/.well-known/restless-native-documents-jwks.json',
    'http://user:password@restless-core:7788/.well-known/restless-native-documents-jwks.json',
    'http://restless-core:7788/not-jwks',
    'http://restless-core:7788/.well-known/restless-native-documents-jwks.json?company=other',
  ]) {
    await assert.rejects(configFromEnvironment({
      ...environment,
      RESTLESS_NATIVE_DOCUMENTS_JWKS_URL: invalidJwksUrl,
    }));
  }
  await chmod(credential, 0o644);
  await assert.rejects(configFromEnvironment(environment));
  await chmod(credential, 0o600);
  await writeFile(credential, 'postgresql://sidecar:secret@127.0.0.1:55432/company\nsecond-value\n', { mode: 0o600 });
  await assert.rejects(configFromEnvironment(environment));
});
