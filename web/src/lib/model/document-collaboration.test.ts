import assert from 'node:assert/strict';
import test from 'node:test';
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

test('edits kept on this device merge back once and never duplicate what the service holds', async () => {
	const Y = await import('yjs');
	const store = new Map<string, string>();
	Object.assign(globalThis, {
		localStorage: {
			getItem: (key: string) => store.get(key) ?? null,
			setItem: (key: string, value: string) => store.set(key, value),
			removeItem: (key: string) => store.delete(key)
		}
	});
	const { savePendingDocumentEdits, loadPendingDocumentEdits, clearPendingDocumentEdits } =
		await import('./document-collaboration.ts');
	const server = new Y.Doc();
	server.getText('t').insert(0, 'Plan');
	const tab = new Y.Doc();
	Y.applyUpdate(tab, Y.encodeStateAsUpdate(server));
	const acknowledged = Y.encodeStateVector(tab);
	tab.getText('t').insert(4, ' for Q3');
	savePendingDocumentEdits('c', 'd', 'a', Y.encodeStateAsUpdate(tab, acknowledged));
	tab.destroy();

	// A new tab opens on the service's copy, which never received the edit, then merges it twice.
	const reopened = new Y.Doc();
	Y.applyUpdate(reopened, Y.encodeStateAsUpdate(server));
	const kept = loadPendingDocumentEdits('c', 'd', 'a');
	assert.ok(kept);
	Y.applyUpdate(reopened, kept);
	Y.applyUpdate(reopened, kept);
	assert.equal(reopened.getText('t').toString(), 'Plan for Q3');
	clearPendingDocumentEdits('c', 'd', 'a');
	assert.equal(loadPendingDocumentEdits('c', 'd', 'a'), null);
});

test('edits stranded by a restore are put back block by block, by id', () => {
	const p = (id: string, text: string) => ({
		type: 'paragraph',
		attrs: { block_id: id },
		content: [{ type: 'text', text }]
	});
	const doc = (...content: ReturnType<typeof p>[]) => ({ type: 'doc' as const, content });
	// Before the restore this device had a, b, c; it changed b, added n and d, removed c.
	const edits = collaboration.strandedBlockEdits(
		doc(p('a', 'A'), p('b', 'B'), p('c', 'C')),
		doc(p('a', 'A'), p('b', 'B edited'), p('n', 'New'), p('d', 'D'))
	);
	assert.deepEqual(
		edits.blocks.map((edit) => [edit.node.attrs?.block_id, edit.after]),
		[
			['b', 'a'],
			['n', 'b'],
			['d', 'n']
		]
	);
	assert.deepEqual(edits.removed, ['c']);
	// The restored version has a, c, and an older z; b is gone from it.
	const { content, applied } = collaboration.reapplyBlockEdits(
		doc(p('z', 'Z'), p('a', 'A old'), p('c', 'C')),
		edits
	);
	assert.deepEqual(
		content.content.map((node) => node.attrs?.block_id),
		['z', 'a', 'b', 'n', 'd']
	);
	// The restored a keeps its restored text: this device never changed a.
	assert.equal(content.content[1].content?.[0].text, 'A old');
	assert.equal(applied, 4);
	// Nothing stranded leaves the restored version as it is.
	const none = collaboration.reapplyBlockEdits(doc(p('a', 'A')), { blocks: [], removed: [] });
	assert.equal(none.applied, 0);
});
