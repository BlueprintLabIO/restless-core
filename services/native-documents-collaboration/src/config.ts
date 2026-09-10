import { constants } from 'node:fs';
import { open } from 'node:fs/promises';
import { isIP } from 'node:net';
import { resolve } from 'node:path';

import { normalizedIssuerOrigin, normalizedPrivateJwksUrl } from './url-policy.js';

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
const HOSTNAME = /^(?=.{1,253}$)(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)*[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/;

export interface CollaborationConfig {
  readonly address: string;
  readonly port: number;
  readonly companyId: string;
  readonly expectedIssuer: string;
  readonly jwksUrl: URL;
  readonly databaseUrl: string;
  readonly debounceMs: number;
  readonly maxDebounceMs: number;
}

function required(environment: NodeJS.ProcessEnv, name: string): string {
  const value = environment[name];
  if (!value || value.trim() !== value || /[\u0000-\u001f\u007f]/u.test(value)) {
    throw new Error(`${name} must be one non-empty normalized value`);
  }
  return value;
}

function positiveInteger(raw: string | undefined, name: string, fallback: number, maximum: number): number {
  const value = raw === undefined ? fallback : Number(raw);
  if (!Number.isSafeInteger(value) || value < 1 || value > maximum) {
    throw new Error(`${name} must be an integer between 1 and ${maximum}`);
  }
  return value;
}

function listenAddress(raw: string | undefined): string {
  const value = raw ?? '0.0.0.0';
  if (value === 'localhost' || isIP(value) !== 0 || HOSTNAME.test(value)) return value;
  throw new Error('RESTLESS_NATIVE_DOCUMENTS_LISTEN_ADDRESS is invalid');
}

async function readCredential(path: string): Promise<string> {
  if (!path.startsWith('/') || resolve(path) !== path) {
    throw new Error('RESTLESS_NATIVE_DOCUMENTS_STORE_CREDENTIAL_FILE must be an absolute normalized path');
  }
  const handle = await open(path, constants.O_RDONLY | constants.O_NOFOLLOW);
  try {
    const metadata = await handle.stat();
    if (!metadata.isFile() || metadata.nlink !== 1 || metadata.size < 1 || metadata.size > 4096) {
      throw new Error('native Documents store credential must be one bounded regular file');
    }
    if ((metadata.mode & 0o077) !== 0) {
      throw new Error('native Documents store credential must not be group- or world-accessible');
    }
    const raw = await handle.readFile('utf8');
    const value = raw.endsWith('\n') ? raw.slice(0, -1) : raw;
    if (
      !value ||
      value.trim() !== value ||
      value.includes('\n') ||
      value.includes('\r') ||
      /[\u0000-\u001f\u007f]/u.test(value)
    ) {
      throw new Error('native Documents store credential must contain exactly one normalized value');
    }
    return value;
  } finally {
    await handle.close();
  }
}

function postgresUrl(raw: string): string {
  let value: URL;
  try {
    value = new URL(raw);
  } catch {
    throw new Error('native Documents store credential must contain a PostgreSQL URL');
  }
  if (
    !['postgres:', 'postgresql:'].includes(value.protocol) ||
    !value.hostname ||
    !value.username ||
    !value.password ||
    value.pathname === '/' ||
    value.searchParams.has('options') ||
    value.hash
  ) {
    throw new Error('native Documents store credential must identify one password-authenticated PostgreSQL database');
  }
  return raw;
}

export async function configFromEnvironment(environment: NodeJS.ProcessEnv = process.env): Promise<CollaborationConfig> {
  const companyId = required(environment, 'RESTLESS_NATIVE_DOCUMENTS_COMPANY_ID').toLowerCase();
  if (!UUID.test(companyId) || companyId === '00000000-0000-0000-0000-000000000000') {
    throw new Error('RESTLESS_NATIVE_DOCUMENTS_COMPANY_ID must be a non-nil UUID');
  }
  const expectedIssuer = normalizedIssuerOrigin(required(environment, 'RESTLESS_NATIVE_DOCUMENTS_TOKEN_ISSUER'));
  const credentialPath = required(environment, 'RESTLESS_NATIVE_DOCUMENTS_STORE_CREDENTIAL_FILE');
  const configuredDatabaseUrl = postgresUrl(await readCredential(credentialPath));
  const debounceMs = positiveInteger(environment.RESTLESS_NATIVE_DOCUMENTS_DEBOUNCE_MS, 'RESTLESS_NATIVE_DOCUMENTS_DEBOUNCE_MS', 750, 10_000);
  const maxDebounceMs = positiveInteger(environment.RESTLESS_NATIVE_DOCUMENTS_MAX_DEBOUNCE_MS, 'RESTLESS_NATIVE_DOCUMENTS_MAX_DEBOUNCE_MS', 5_000, 30_000);
  if (maxDebounceMs < debounceMs) {
    throw new Error('RESTLESS_NATIVE_DOCUMENTS_MAX_DEBOUNCE_MS must be at least the debounce interval');
  }
  return Object.freeze({
    address: listenAddress(environment.RESTLESS_NATIVE_DOCUMENTS_LISTEN_ADDRESS),
    port: positiveInteger(environment.RESTLESS_NATIVE_DOCUMENTS_LISTEN_PORT, 'RESTLESS_NATIVE_DOCUMENTS_LISTEN_PORT', 6688, 65_535),
    companyId,
    expectedIssuer,
    jwksUrl: normalizedPrivateJwksUrl(required(environment, 'RESTLESS_NATIVE_DOCUMENTS_JWKS_URL')),
    databaseUrl: configuredDatabaseUrl,
    debounceMs,
    maxDebounceMs,
  });
}
