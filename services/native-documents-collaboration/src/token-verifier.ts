import {
  base64url,
  decodeProtectedHeader,
  importJWK,
  jwtVerify,
  type CryptoKey,
  type JWK_OKP_Public,
} from 'jose';

import {
  JWKS_CACHE_SECONDS,
  MAX_JWKS_BYTES,
  MAX_TOKEN_BYTES,
  MAX_TOKEN_TTL_SECONDS,
  MIN_TOKEN_TTL_SECONDS,
  TOKEN_AUDIENCE,
  TOKEN_TYPE,
} from './constants.js';
import type { CollaborationTarget } from './target.js';
import { normalizedIssuerOrigin, normalizedPrivateJwksUrl } from './url-policy.js';

const CANONICAL_UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const KEY_ID = /^native-documents-[0-9a-f]{24}$/;
const HEADER_FIELDS = ['alg', 'kid', 'typ'] as const;
const CLAIM_FIELDS = [
  'access',
  'actor_id',
  'aud',
  'company_id',
  'document_id',
  'exp',
  'iat',
  'iss',
  'jti',
  'nbf',
  'sub',
] as const;
const JWK_FIELDS = ['alg', 'crv', 'kid', 'kty', 'use', 'x'] as const;

export type CollaborationAccess = 'read' | 'write';

export interface CollaborationClaims {
  readonly iss: string;
  readonly aud: typeof TOKEN_AUDIENCE;
  readonly sub: string;
  readonly company_id: string;
  readonly document_id: string;
  readonly actor_id: string;
  readonly access: CollaborationAccess;
  readonly iat: number;
  readonly nbf: number;
  readonly exp: number;
  readonly jti: string;
}

interface CachedJwks {
  readonly keys: ReadonlyMap<string, CryptoKey>;
  readonly expiresAtMs: number;
}

const UNKNOWN_KID_REFRESH_COOLDOWN_MILLISECONDS = 5_000;

export interface CoreTokenVerifierOptions {
  readonly issuer: string;
  readonly jwksUrl: URL;
  readonly fetch?: typeof fetch;
  readonly now?: () => number;
  readonly cacheSeconds?: number;
}

export class CollaborationTokenError extends Error {
  constructor() {
    super('native Documents collaboration token is invalid');
    this.name = 'CollaborationTokenError';
  }
}

function invalidToken(): never {
  throw new CollaborationTokenError();
}

function exactObject(value: unknown, fields: readonly string[]): value is Record<string, unknown> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return false;
  const keys = Object.keys(value).sort();
  const expected = [...fields].sort();
  return keys.length === expected.length && keys.every((key, index) => key === expected[index]);
}

function exactString(value: unknown, maximum: number): value is string {
  return (
    typeof value === 'string' &&
    value.length > 0 &&
    value.length <= maximum &&
    value.trim() === value &&
    !/[\u0000-\u001f\u007f]/u.test(value)
  );
}

function exactInteger(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value);
}

function parseHeader(token: string): { readonly kid: string } {
  let header: unknown;
  try {
    header = decodeProtectedHeader(token);
  } catch {
    invalidToken();
  }
  if (
    !exactObject(header, HEADER_FIELDS) ||
    header.alg !== 'EdDSA' ||
    header.typ !== TOKEN_TYPE ||
    typeof header.kid !== 'string' ||
    !KEY_ID.test(header.kid)
  ) {
    invalidToken();
  }
  return { kid: header.kid };
}

function parseClaims(value: unknown, expected: CollaborationTarget, issuer: string, nowSeconds: number): CollaborationClaims {
  if (!exactObject(value, CLAIM_FIELDS)) invalidToken();
  const {
    iss,
    aud,
    sub,
    company_id: companyId,
    document_id: documentId,
    actor_id: actorId,
    access,
    iat,
    nbf,
    exp,
    jti,
  } = value;
  if (
    iss !== issuer ||
    aud !== TOKEN_AUDIENCE ||
    !exactString(sub, 255) ||
    !exactString(actorId, 200) ||
    (access !== 'read' && access !== 'write') ||
    typeof companyId !== 'string' ||
    typeof documentId !== 'string' ||
    typeof jti !== 'string' ||
    !CANONICAL_UUID.test(companyId) ||
    !CANONICAL_UUID.test(documentId) ||
    !CANONICAL_UUID.test(jti) ||
    companyId === '00000000-0000-0000-0000-000000000000' ||
    documentId === '00000000-0000-0000-0000-000000000000' ||
    jti === '00000000-0000-0000-0000-000000000000' ||
    companyId !== expected.companyId ||
    documentId !== expected.documentId ||
    !exactInteger(iat) ||
    !exactInteger(nbf) ||
    !exactInteger(exp)
  ) {
    invalidToken();
  }
  const lifetime = exp - iat;
  if (
    iat <= 0 ||
    nbf !== iat ||
    lifetime < MIN_TOKEN_TTL_SECONDS ||
    lifetime > MAX_TOKEN_TTL_SECONDS ||
    iat > nowSeconds ||
    nbf > nowSeconds ||
    exp <= nowSeconds
  ) {
    invalidToken();
  }
  return Object.freeze({
    iss,
    aud,
    sub,
    company_id: companyId,
    document_id: documentId,
    actor_id: actorId,
    access,
    iat,
    nbf,
    exp,
    jti,
  });
}

async function boundedJson(response: Response): Promise<unknown> {
  if (response.status !== 200 || response.redirected) throw new Error('Core JWKS request failed');
  const contentType = response.headers.get('content-type')?.split(';', 1)[0]?.trim().toLowerCase();
  if (contentType !== 'application/json') throw new Error('Core JWKS response is not JSON');
  const contentLength = response.headers.get('content-length');
  if (contentLength !== null) {
    const length = Number(contentLength);
    if (!Number.isSafeInteger(length) || length < 1 || length > MAX_JWKS_BYTES) {
      throw new Error('Core JWKS response is not bounded');
    }
  }
  if (!response.body) throw new Error('Core JWKS response has no body');
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let length = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    length += value.byteLength;
    if (length > MAX_JWKS_BYTES) {
      await reader.cancel();
      throw new Error('Core JWKS response is not bounded');
    }
    chunks.push(value);
  }
  if (length === 0) throw new Error('Core JWKS response is empty');
  const raw = new Uint8Array(length);
  let offset = 0;
  for (const chunk of chunks) {
    raw.set(chunk, offset);
    offset += chunk.byteLength;
  }
  let text: string;
  try {
    text = new TextDecoder('utf-8', { fatal: true }).decode(raw);
  } catch {
    throw new Error('Core JWKS response is not UTF-8');
  }
  try {
    return JSON.parse(text) as unknown;
  } catch {
    throw new Error('Core JWKS response is not valid JSON');
  }
}

async function parseJwks(value: unknown): Promise<ReadonlyMap<string, CryptoKey>> {
  if (!exactObject(value, ['keys']) || !Array.isArray(value.keys) || value.keys.length < 1 || value.keys.length > 3) {
    throw new Error('Core JWKS has an invalid shape');
  }
  const keys = new Map<string, CryptoKey>();
  for (const candidate of value.keys) {
    if (!exactObject(candidate, JWK_FIELDS)) throw new Error('Core JWKS key has an invalid shape');
    const { alg, crv, kid, kty, use, x } = candidate;
    if (
      alg !== 'EdDSA' ||
      crv !== 'Ed25519' ||
      kty !== 'OKP' ||
      use !== 'sig' ||
      typeof kid !== 'string' ||
      !KEY_ID.test(kid) ||
      typeof x !== 'string' ||
      x.includes('=')
    ) {
      throw new Error('Core JWKS key is invalid');
    }
    let publicBytes: Uint8Array;
    try {
      publicBytes = base64url.decode(x);
    } catch {
      throw new Error('Core JWKS key is invalid');
    }
    if (publicBytes.byteLength !== 32 || base64url.encode(publicBytes) !== x || keys.has(kid)) {
      throw new Error('Core JWKS key is invalid');
    }
    const jwk: JWK_OKP_Public = { alg: 'EdDSA', crv: 'Ed25519', kid, kty: 'OKP', use: 'sig', x };
    const imported = await importJWK(jwk, 'EdDSA');
    if (imported instanceof Uint8Array) throw new Error('Core JWKS key is invalid');
    keys.set(kid, imported);
  }
  return keys;
}

export class CoreTokenVerifier {
  readonly issuer: string;
  readonly jwksUrl: URL;
  private readonly fetchImplementation: typeof fetch;
  private readonly now: () => number;
  private readonly cacheMilliseconds: number;
  private cached: CachedJwks | undefined;
  private refreshPromise: Promise<CachedJwks> | undefined;
  private lastUnknownKidRefreshMs = Number.NEGATIVE_INFINITY;

  constructor(options: CoreTokenVerifierOptions) {
    this.issuer = normalizedIssuerOrigin(options.issuer);
    this.jwksUrl = normalizedPrivateJwksUrl(options.jwksUrl.toString());
    this.fetchImplementation = options.fetch ?? fetch;
    this.now = options.now ?? Date.now;
    const cacheSeconds = options.cacheSeconds ?? JWKS_CACHE_SECONDS;
    if (!Number.isSafeInteger(cacheSeconds) || cacheSeconds < 1 || cacheSeconds > JWKS_CACHE_SECONDS) {
      throw new Error(`JWKS cache must be between 1 and ${JWKS_CACHE_SECONDS} seconds`);
    }
    this.cacheMilliseconds = cacheSeconds * 1_000;
  }

  async initialize(): Promise<void> {
    await this.keys(false);
  }

  async probe(): Promise<boolean> {
    try {
      await this.keys(false);
      return true;
    } catch {
      return false;
    }
  }

  async verify(token: string, expected: CollaborationTarget): Promise<CollaborationClaims> {
    try {
      if (
        typeof token !== 'string' ||
        token.length === 0 ||
        Buffer.byteLength(token, 'utf8') > MAX_TOKEN_BYTES ||
        token.trim() !== token
      ) {
        invalidToken();
      }
      const { kid } = parseHeader(token);
      const hadFreshCache = this.cached !== undefined && this.cached.expiresAtMs > this.now();
      let keys = await this.keys(false);
      let key = keys.get(kid);
      const refreshNow = this.now();
      if (
        !key &&
        hadFreshCache &&
        refreshNow - this.lastUnknownKidRefreshMs >= UNKNOWN_KID_REFRESH_COOLDOWN_MILLISECONDS
      ) {
        this.lastUnknownKidRefreshMs = refreshNow;
        keys = await this.keys(true);
        key = keys.get(kid);
      }
      if (!key) invalidToken();
      const now = this.now();
      const result = await jwtVerify(token, key, {
        algorithms: ['EdDSA'],
        audience: TOKEN_AUDIENCE,
        issuer: this.issuer,
        typ: TOKEN_TYPE,
        currentDate: new Date(now),
        clockTolerance: 0,
        requiredClaims: [...CLAIM_FIELDS],
      });
      return parseClaims(result.payload, expected, this.issuer, Math.floor(now / 1_000));
    } catch (error) {
      if (error instanceof CollaborationTokenError) throw error;
      throw new CollaborationTokenError();
    }
  }

  private async keys(forceRefresh: boolean): Promise<ReadonlyMap<string, CryptoKey>> {
    if (!forceRefresh && this.cached && this.cached.expiresAtMs > this.now()) return this.cached.keys;
    if (!this.refreshPromise) {
      this.refreshPromise = this.refresh().finally(() => {
        this.refreshPromise = undefined;
      });
    }
    return (await this.refreshPromise).keys;
  }

  private async refresh(): Promise<CachedJwks> {
    const response = await this.fetchImplementation(this.jwksUrl, {
      method: 'GET',
      headers: { accept: 'application/json' },
      redirect: 'error',
      signal: AbortSignal.timeout(5_000),
    });
    const keys = await parseJwks(await boundedJson(response));
    const loadedAtMs = this.now();
    const cached = Object.freeze({ keys, expiresAtMs: loadedAtMs + this.cacheMilliseconds });
    this.cached = cached;
    return cached;
  }
}
