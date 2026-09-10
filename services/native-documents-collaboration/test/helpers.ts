import { exportJWK, generateKeyPair, SignJWT, type CryptoKey } from 'jose';

import { TOKEN_AUDIENCE, TOKEN_TYPE } from '../src/constants.js';
import type { CollaborationAccess } from '../src/token-verifier.js';

export const COMPANY_ID = 'a1111111-1111-4111-8111-111111111111';
export const DOCUMENT_ID = 'b2222222-2222-4222-8222-222222222222';
export const OTHER_DOCUMENT_ID = 'c3333333-3333-4333-8333-333333333333';
export const ACTOR_ID = 'human-founder';
export const SUBJECT = 'local-owner:founder-session';
export const ISSUER = 'http://127.0.0.1:7788';
export const JWKS_URL = 'http://restless-core:7788/.well-known/restless-native-documents-jwks.json';
export const KID = 'native-documents-aaaaaaaaaaaaaaaaaaaaaaaa';

export interface SigningFixture {
  readonly privateKey: CryptoKey;
  readonly jwks: Readonly<Record<string, unknown>>;
  sign(overrides?: Readonly<Record<string, unknown>>, headerOverrides?: Readonly<Record<string, unknown>>): Promise<string>;
}

export async function signingFixture(nowSeconds: number): Promise<SigningFixture> {
  const { privateKey, publicKey } = await generateKeyPair('EdDSA');
  const publicJwk = await exportJWK(publicKey);
  const jwks = {
    keys: [
      {
        kty: 'OKP',
        crv: 'Ed25519',
        alg: 'EdDSA',
        use: 'sig',
        kid: KID,
        x: publicJwk.x,
      },
    ],
  } as const;
  return {
    privateKey,
    jwks,
    async sign(overrides = {}, headerOverrides = {}) {
      const claims = {
        iss: ISSUER,
        aud: TOKEN_AUDIENCE,
        sub: SUBJECT,
        company_id: COMPANY_ID,
        document_id: DOCUMENT_ID,
        actor_id: ACTOR_ID,
        access: 'write' satisfies CollaborationAccess,
        iat: nowSeconds,
        nbf: nowSeconds,
        exp: nowSeconds + 60,
        jti: '44444444-4444-4444-8444-444444444444',
        ...overrides,
      };
      return new SignJWT(claims)
        .setProtectedHeader({ alg: 'EdDSA', typ: TOKEN_TYPE, kid: KID, ...headerOverrides })
        .sign(privateKey);
    },
  };
}

export function jwksResponse(jwks: Readonly<Record<string, unknown>>): Response {
  return new Response(JSON.stringify(jwks), {
    status: 200,
    headers: { 'content-type': 'application/json' },
  });
}
