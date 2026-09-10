import assert from 'node:assert/strict';
import test from 'node:test';

import { MAX_JWKS_BYTES, TOKEN_AUDIENCE } from '../src/constants.js';
import { CollaborationTokenError, CoreTokenVerifier } from '../src/token-verifier.js';
import {
  COMPANY_ID,
  DOCUMENT_ID,
  ISSUER,
  JWKS_URL,
  KID,
  OTHER_DOCUMENT_ID,
  jwksResponse,
  signingFixture,
} from './helpers.js';

test('only an exact Core token bound to the requested company and document is accepted', async () => {
  const nowSeconds = 2_000_000_000;
  const fixture = await signingFixture(nowSeconds);
  let fetches = 0;
  const verifier = new CoreTokenVerifier({
    issuer: ISSUER,
    jwksUrl: new URL(JWKS_URL),
    now: () => nowSeconds * 1_000,
    fetch: (async () => {
      fetches += 1;
      return jwksResponse(fixture.jwks);
    }) as typeof fetch,
  });
  const token = await fixture.sign();
  const claims = await verifier.verify(token, { companyId: COMPANY_ID, documentId: DOCUMENT_ID });
  assert.equal(claims.actor_id, 'human-founder');
  assert.equal(claims.access, 'write');
  assert.equal(fetches, 1);
  await verifier.verify(token, { companyId: COMPANY_ID, documentId: DOCUMENT_ID });
  assert.equal(fetches, 1, 'fresh JWKS must be cached');

  await assert.rejects(
    verifier.verify(token, { companyId: COMPANY_ID, documentId: OTHER_DOCUMENT_ID }),
    CollaborationTokenError,
  );
  await assert.rejects(
    verifier.verify(await fixture.sign({ aud: `${TOKEN_AUDIENCE}-wrong` }), {
      companyId: COMPANY_ID,
      documentId: DOCUMENT_ID,
    }),
    CollaborationTokenError,
  );
  await assert.rejects(
    verifier.verify(await fixture.sign({ unexpected: true }), { companyId: COMPANY_ID, documentId: DOCUMENT_ID }),
    CollaborationTokenError,
  );
  await assert.rejects(
    verifier.verify(await fixture.sign({ exp: nowSeconds }), { companyId: COMPANY_ID, documentId: DOCUMENT_ID }),
    CollaborationTokenError,
  );
});

test('JWKS retrieval is bounded, redirect-free, and accepts public Ed25519 material only', async () => {
  const nowSeconds = 2_000_000_000;
  const fixture = await signingFixture(nowSeconds);
  const token = await fixture.sign();
  let observedRedirect: RequestRedirect | undefined;
  const malformed = structuredClone(fixture.jwks) as { keys: Array<Record<string, unknown>> };
  malformed.keys[0] = { ...malformed.keys[0], d: 'private-material' };
  const malformedVerifier = new CoreTokenVerifier({
    issuer: ISSUER,
    jwksUrl: new URL(JWKS_URL),
    now: () => nowSeconds * 1_000,
    fetch: (async (_input, init) => {
      observedRedirect = init?.redirect;
      return jwksResponse(malformed);
    }) as typeof fetch,
  });
  await assert.rejects(malformedVerifier.initialize());
  assert.equal(observedRedirect, 'error');

  const oversizedVerifier = new CoreTokenVerifier({
    issuer: ISSUER,
    jwksUrl: new URL(JWKS_URL),
    now: () => nowSeconds * 1_000,
    fetch: (async () =>
      new Response('x'.repeat(MAX_JWKS_BYTES + 1), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      })) as typeof fetch,
  });
  await assert.rejects(oversizedVerifier.initialize());

  const replacementKid = `native-documents-${'b'.repeat(24)}`;
  const replacementToken = await fixture.sign({}, { kid: replacementKid });
  let refreshes = 0;
  const replacementJwks = structuredClone(fixture.jwks) as { keys: Array<Record<string, unknown>> };
  replacementJwks.keys[0] = { ...replacementJwks.keys[0], kid: replacementKid };
  const refreshingVerifier = new CoreTokenVerifier({
    issuer: ISSUER,
    jwksUrl: new URL(JWKS_URL),
    now: () => nowSeconds * 1_000,
    fetch: (async () => {
      refreshes += 1;
      return jwksResponse(refreshes === 1 ? fixture.jwks : replacementJwks);
    }) as typeof fetch,
  });
  await refreshingVerifier.initialize();
  await refreshingVerifier.verify(replacementToken, { companyId: COMPANY_ID, documentId: DOCUMENT_ID });
  assert.equal(refreshes, 2, 'a key absent from a fresh cache must cause one bounded rotation refresh');

  const attackerKid = await fixture.sign({}, { kid: `native-documents-${'c'.repeat(24)}` });
  await assert.rejects(
    refreshingVerifier.verify(attackerKid, { companyId: COMPANY_ID, documentId: DOCUMENT_ID }),
    CollaborationTokenError,
  );
  assert.equal(refreshes, 2, 'unknown key ids must not turn token verification into an unbounded fetch path');
  assert.match(KID, /^native-documents-[0-9a-f]{24}$/);
  assert.notEqual(token, replacementToken);
});
