import assert from 'node:assert/strict';
import test from 'node:test';
// @ts-expect-error Node's strip-only test runner needs the explicit TypeScript suffix.
import * as collaboration from './document-collaboration.ts';

const COMPANY = '018f47f1-7b60-7c2e-8a2b-151fef2b7aa1';
const DOCUMENT = '018f47f1-7b60-7c2e-8a2b-151fef2b7aa2';

test('collaboration route is exact, same-origin, and UUID scoped', () => {
	assert.equal(
		collaboration.documentCollaborationWebSocketUrl(
			'https://cloud.restless.run',
			COMPANY,
			DOCUMENT
		),
		`wss://cloud.restless.run/api/companies/${COMPANY}/documents/${DOCUMENT}/collaboration`
	);
	assert.equal(
		collaboration.documentCollaborationWebSocketUrl('http://127.0.0.1:4321', COMPANY, DOCUMENT),
		`ws://127.0.0.1:4321/api/companies/${COMPANY}/documents/${DOCUMENT}/collaboration`
	);
	assert.equal(
		collaboration.documentCollaborationName(COMPANY, DOCUMENT),
		`${COMPANY}:${DOCUMENT}`
	);
	assert.throws(() =>
		collaboration.documentCollaborationWebSocketUrl(
			'https://cloud.restless.run',
			'company-slug',
			DOCUMENT
		)
	);
});

test('token resolver uses the route handle only for Core REST and never sends a body', async () => {
	const original = globalThis.fetch;
	const requests: Array<{ url: string; init?: RequestInit }> = [];
	globalThis.fetch = (async (input: string | URL | Request, init?: RequestInit) => {
		requests.push({ url: String(input), init });
		return new Response(
			JSON.stringify({
				token: 'short-lived-signed-token',
				token_type: 'Bearer',
				access: 'write',
				expires_at: '2030-01-01T00:00:00.000Z'
			}),
			{ status: 200, headers: { 'content-type': 'application/json' } }
		);
	}) as typeof fetch;
	try {
		const grant = await collaboration.requestDocumentCollaborationToken(
			'acme team',
			DOCUMENT,
			undefined,
			Date.parse('2029-01-01T00:00:00.000Z')
		);
		assert.equal(grant.access, 'write');
		const request = requests[0];
		assert.ok(request);
		assert.equal(
			request.url,
			`/api/companies/acme%20team/documents/${DOCUMENT}/collaboration/token`
		);
		assert.equal(request.init?.method, 'POST');
		assert.equal(request.init?.credentials, 'same-origin');
		assert.equal(request.init?.cache, 'no-store');
		assert.equal(request.init?.body, undefined);
	} finally {
		globalThis.fetch = original;
	}
});

test('token resolver rejects expired and malformed grants', async () => {
	const original = globalThis.fetch;
	try {
		globalThis.fetch = (async () =>
			new Response(
				JSON.stringify({
					token: 'expired',
					token_type: 'Bearer',
					access: 'read',
					expires_at: '2025-01-01T00:00:00.000Z'
				}),
				{ status: 200 }
			)) as typeof fetch;
		await assert.rejects(
			collaboration.requestDocumentCollaborationToken(
				'acme',
				DOCUMENT,
				undefined,
				Date.parse('2026-01-01T00:00:00.000Z')
			),
			/expired/u
		);
		globalThis.fetch = (async () =>
			new Response(JSON.stringify({ token: 'opaque', access: 'admin' }), {
				status: 200
			})) as typeof fetch;
		await assert.rejects(
			collaboration.requestDocumentCollaborationToken('acme', DOCUMENT),
			/invalid/u
		);
	} finally {
		globalThis.fetch = original;
	}
});

test('collaboration state tells initial connect, reconnect, degradation, and access apart', () => {
	const { transitionDocumentCollaborationState: move } = collaboration;
	assert.equal(move('degraded', 'start'), 'connecting');
	assert.equal(move('connecting', 'socket-connecting'), 'connecting');
	assert.equal(move('connecting', 'synced-write'), 'synced');
	assert.equal(move('synced', 'socket-disconnected'), 'reconnecting');
	assert.equal(move('reconnecting', 'synced-read'), 'read-only');
	assert.equal(move('read-only', 'socket-connecting'), 'reconnecting');
	assert.equal(move('reconnecting', 'failed'), 'degraded');
});
