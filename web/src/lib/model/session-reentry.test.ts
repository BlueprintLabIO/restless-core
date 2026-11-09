import assert from 'node:assert/strict';
import { test } from 'node:test';
import { observeSessionEnd, type ReentryGaveUp } from './session-reentry.ts';

function installBrowser(company: string) {
	const reloads: number[] = [];
	const responses = new Map<string, () => Response>([
		['/health', () => new Response('ok', { status: 200 })]
	]);
	const store = new Map<string, string>();
	(globalThis as Record<string, unknown>).sessionStorage = {
		getItem: (key: string) => store.get(key) ?? null,
		setItem: (key: string, value: string) => void store.set(key, value),
		removeItem: (key: string) => void store.delete(key)
	};
	(globalThis as Record<string, unknown>).window = {
		fetch: async (input: string | URL) => {
			const path = new URL(String(input), 'http://plane.test').pathname;
			return (responses.get(path) ?? (() => new Response('{}', { status: 404 })))();
		},
		location: { href: `http://plane.test/${company}`, origin: 'http://plane.test', reload: () => reloads.push(1) },
		addEventListener: () => {},
		removeEventListener: () => {}
	};
	return { reloads, respond: responses, store };
}
const outOfScope = () =>
	new Response(JSON.stringify({ code: 'company_out_of_scope' }), {
		status: 403,
		headers: { 'content-type': 'application/json' }
	});

test('a session without this company is told no access, not signed out', async () => {
	const { respond, store, reloads } = installBrowser('acme');
	// The tab already tried going Home to enter this company once.
	store.set('restless:reentry-company:acme', '1');
	respond.set('/api/companies/acme/principal', outOfScope);
	const told: ReentryGaveUp[] = [];
	const stop = observeSessionEnd(
		() => 'acme',
		(reason) => told.push(reason)
	);
	await window.fetch('/api/companies/acme/principal');
	await new Promise((resolve) => setTimeout(resolve, 20));
	stop();
	assert.deepEqual(told, ['no_access']);
	assert.equal(reloads.length, 0, 'it does not reload into the same refusal');
});
