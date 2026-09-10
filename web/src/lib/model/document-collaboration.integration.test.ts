import assert from 'node:assert/strict';
import test from 'node:test';
import { HocuspocusProvider } from '@hocuspocus/provider';
import { Server } from '@hocuspocus/server';
import * as Y from 'yjs';
// @ts-expect-error Node's strip-only test runner needs the explicit TypeScript suffix.
import * as collaboration from './document-collaboration.ts';

const COMPANY = '018f47f1-7b60-7c2e-8a2b-151fef2b7aa1';
const DOCUMENT = '018f47f1-7b60-7c2e-8a2b-151fef2b7aa2';

async function eventually(
	check: () => boolean,
	label: string,
	timeoutMilliseconds = 4_000
): Promise<void> {
	const deadline = Date.now() + timeoutMilliseconds;
	while (Date.now() < deadline) {
		if (check()) return;
		await new Promise((resolve) => setTimeout(resolve, 10));
	}
	assert.fail(`timed out waiting for ${label}`);
}

test('two providers converge on the exact route and refetch a token when the server requests one', async (context) => {
	const originalFetch = globalThis.fetch;
	const tokenRequests: string[] = [];
	const authenticatedTokens: string[] = [];
	const refreshedTokens: string[] = [];
	const connectedPaths: string[] = [];
	globalThis.fetch = (async (input: string | URL | Request) => {
		const url = String(input);
		tokenRequests.push(url);
		return new Response(
			JSON.stringify({
				token: `ephemeral-${tokenRequests.length}`,
				token_type: 'Bearer',
				access: 'write',
				expires_at: new Date(Date.now() + 60_000).toISOString()
			}),
			{ status: 200, headers: { 'content-type': 'application/json' } }
		);
	}) as typeof fetch;

	const server = new Server({
		address: '127.0.0.1',
		port: 0,
		quiet: true,
		onConnect: async ({ request }) => {
			connectedPaths.push(new URL(request.url).pathname);
		},
		onAuthenticate: async ({ documentName, token }) => {
			assert.equal(documentName, collaboration.documentCollaborationName(COMPANY, DOCUMENT));
			assert.match(token, /^ephemeral-\d+$/u);
			authenticatedTokens.push(token);
		},
		onTokenSync: async ({ token }) => {
			assert.match(token, /^ephemeral-\d+$/u);
			refreshedTokens.push(token);
		}
	});
	await server.listen();

	const documents = [new Y.Doc(), new Y.Doc()];
	const providers = documents.map(
		(document) =>
			new HocuspocusProvider({
				url: collaboration.documentCollaborationWebSocketUrl(server.httpURL, COMPANY, DOCUMENT),
				name: collaboration.documentCollaborationName(COMPANY, DOCUMENT),
				document,
				token: async () =>
					(await collaboration.requestDocumentCollaborationToken('company-handle', DOCUMENT)).token
			})
	);

	context.after(async () => {
		for (const provider of providers) provider.destroy();
		for (const document of documents) document.destroy();
		await server.destroy();
		globalThis.fetch = originalFetch;
	});

	await eventually(
		() => providers.every((provider) => provider.isAuthenticated && provider.isSynced),
		'both providers to authenticate and sync'
	);
	assert.deepEqual(connectedPaths, [
		`/api/companies/${COMPANY}/documents/${DOCUMENT}/collaboration`,
		`/api/companies/${COMPANY}/documents/${DOCUMENT}/collaboration`
	]);
	assert.deepEqual(authenticatedTokens.sort(), ['ephemeral-1', 'ephemeral-2']);

	documents[0].getText('body').insert(0, 'evidence');
	await eventually(
		() => documents[1].getText('body').toString() === 'evidence',
		'first edit to reach the second provider'
	);
	documents[1].getText('body').insert(8, ' first');
	await eventually(
		() => documents[0].getText('body').toString() === 'evidence first',
		'second edit to reach the first provider'
	);
	providers[1].disconnect();
	await eventually(() => !providers[1].isAuthenticated, 'second provider to disconnect');
	documents[0].getText('body').insert(14, ', durable');
	assert.equal(documents[1].getText('body').toString(), 'evidence first');
	await providers[1].connect();
	await eventually(
		() =>
			providers[1].isAuthenticated &&
			providers[1].isSynced &&
			documents[1].getText('body').toString() === 'evidence first, durable',
		'second provider to refetch access and converge after reconnect'
	);
	assert.equal(tokenRequests.length, 3);

	const liveDocument = server.hocuspocus.documents.get(
		collaboration.documentCollaborationName(COMPANY, DOCUMENT)
	);
	assert.ok(liveDocument);
	const firstConnection = [...liveDocument.connections.keys()][0];
	assert.ok(firstConnection);
	firstConnection.requestToken();
	await eventually(
		() => tokenRequests.length === 4 && refreshedTokens.length === 1,
		'a fresh Core token after the server token request'
	);
	assert.equal(refreshedTokens[0], 'ephemeral-4');
	assert.ok(
		tokenRequests.every(
			(url) => url === `/api/companies/company-handle/documents/${DOCUMENT}/collaboration/token`
		)
	);
});
