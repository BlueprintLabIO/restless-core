import assert from 'node:assert/strict';
import test from 'node:test';
import { describeFailure, failureKind, isRetryable, responseFailure } from './failure.ts';

const json = (status: number, body: unknown) =>
	new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
const text = (status: number, statusText: string) =>
	new Response('upstream connect error', { status, statusText });

test('a private-network response stays authoritative when the browser reports no Internet', async () => {
	const previous = Object.getOwnPropertyDescriptor(globalThis, 'navigator');
	Object.defineProperty(globalThis, 'navigator', { configurable: true, value: { onLine: false } });
	try {
		assert.equal(
			failureKind(await responseFailure(json(403, { message: 'Owner only.' }))),
			'access'
		);
		assert.equal(
			failureKind(await responseFailure(json(401, { message: 'Sign in.' }))),
			'signed_out'
		);
		assert.equal(failureKind(new TypeError('Failed to fetch')), 'offline');
	} finally {
		if (previous) Object.defineProperty(globalThis, 'navigator', previous);
		else Reflect.deleteProperty(globalThis, 'navigator');
	}
});

test('transport and proxy failures never reach the owner as raw text', async () => {
	const seen = [
		describeFailure(new TypeError('Failed to fetch')),
		describeFailure(await responseFailure(text(502, 'Bad Gateway'))),
		describeFailure(await responseFailure(text(504, 'Gateway Timeout'))),
		describeFailure(await responseFailure(text(500, 'Internal Server Error')))
	];
	for (const view of seen) {
		const shown = `${view.title} ${view.detail}`;
		assert.doesNotMatch(shown, /fetch|gateway|upstream|\b50\d\b/i, shown);
		assert.equal(view.retryable, true);
	}
	assert.equal(seen[0].kind, 'unreachable');
	assert.equal(seen[1].kind, 'unreachable');
	assert.equal(seen[3].kind, 'server');
});

test("the daemon's own message is kept for request-level failures and hidden for 5xx", async () => {
	const rejected = describeFailure(
		await responseFailure(json(422, { error: 'charter', message: 'The charter needs a mission.' }))
	);
	assert.equal(rejected.detail, 'The charter needs a mission.');
	assert.equal(rejected.retryable, false);

	const internal = describeFailure(
		await responseFailure(json(500, { error: 'api', message: 'sqlx: pool timed out' }))
	);
	assert.doesNotMatch(internal.detail, /sqlx/);
	assert.equal(internal.technical, 'sqlx: pool timed out');

	// A 503 the daemon wrote is a dependency outage, not a lost connection.
	const vault = describeFailure(
		await responseFailure(
			json(503, { error: 'vault', message: 'Cannot read Infisical right now.' })
		)
	);
	assert.equal(vault.kind, 'unavailable');
	assert.equal(vault.detail, 'Cannot read Infisical right now.');
});

test('only failures a second attempt can fix are retried', async () => {
	assert.equal(isRetryable(await responseFailure(json(404, { message: 'No such Work.' }))), false);
	assert.equal(isRetryable(await responseFailure(json(403, { message: 'Owner only.' }))), false);
	assert.equal(isRetryable(await responseFailure(json(422, { message: 'Invalid.' }))), false);
	assert.equal(isRetryable(new Error('The company principal response is invalid.')), false);
	assert.equal(isRetryable(await responseFailure(json(429, { message: 'Slow down.' }))), true);
	assert.equal(isRetryable(new TypeError('NetworkError when attempting to fetch resource.')), true);
});

test('a route this server does not have yet says to restart, not "not found"', async () => {
	const stale = describeFailure(
		await responseFailure(json(404, { error: 'api', message: 'unknown owner API route' }))
	);
	assert.doesNotMatch(`${stale.title} ${stale.detail}`, /unknown|route|not found/i);
	assert.match(stale.detail, /restart/i);
	assert.equal(stale.technical, 'unknown owner API route');

	const gone = describeFailure(
		await responseFailure(json(404, { error: 'work', message: 'That Work no longer exists.' })),
		'Work'
	);
	assert.equal(gone.title, 'Work not found');
});
