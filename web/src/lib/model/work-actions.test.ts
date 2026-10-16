import assert from 'node:assert/strict';
import test from 'node:test';
import { transitionGoals } from './work-actions.ts';

const json = (body: unknown, status = 200) =>
	new Response(JSON.stringify(body), {
		status,
		headers: { 'content-type': 'application/json' }
	});

test('partial goal archive keeps every applied result available to Undo', async () => {
	const previous = globalThis.fetch;
	globalThis.fetch = async (url, init) => {
		if (init?.method === 'POST')
			return String(url).includes('/second/')
				? json({ message: 'Cannot archive this goal yet.' }, 409)
				: json({ closed: true });
		return json({
			goals: [
				{ id: 'first', closed_at: '2026-10-16' },
				{ id: 'second', closed_at: null }
			]
		});
	};
	try {
		const result = await transitionGoals('fixture_test', ['first', 'second'], 'archive');
		assert.deepEqual(result.changed, ['first']);
		assert.deepEqual(
			result.failed.map((goal) => goal.id),
			['second']
		);
	} finally {
		globalThis.fetch = previous;
	}
});

test('a lost archive response is reconciled before offering Retry', async () => {
	const previous = globalThis.fetch;
	globalThis.fetch = async (_url, init) => {
		if (init?.method === 'POST') throw new TypeError('Connection closed after commit');
		return json({
			goals: [
				{ id: 'committed', closed_at: '2026-10-16' },
				{ id: 'pending', closed_at: null }
			]
		});
	};
	try {
		const result = await transitionGoals('fixture_test', ['committed', 'pending'], 'archive');
		assert.deepEqual(result.changed, ['committed']);
		assert.deepEqual(
			result.failed.map((goal) => goal.id),
			['pending']
		);
	} finally {
		globalThis.fetch = previous;
	}
});

test('partial goal Undo retries only goals that remain archived', async () => {
	const previous = globalThis.fetch;
	globalThis.fetch = async (_url, init) => {
		if (init?.method === 'POST') return json({ message: 'Response unavailable' }, 503);
		return json({
			goals: [
				{ id: 'reopened', closed_at: null },
				{ id: 'closed', closed_at: '2026-10-16' }
			]
		});
	};
	try {
		const result = await transitionGoals('fixture_test', ['reopened', 'closed'], 'reopen');
		assert.deepEqual(result.changed, ['reopened']);
		assert.deepEqual(
			result.failed.map((goal) => goal.id),
			['closed']
		);
	} finally {
		globalThis.fetch = previous;
	}
});

test('an unavailable reconciliation never fabricates a successful goal change', async () => {
	const previous = globalThis.fetch;
	globalThis.fetch = async () => json({ message: 'Service unavailable' }, 503);
	try {
		const result = await transitionGoals('fixture_test', ['unknown'], 'archive');
		assert.deepEqual(result.changed, []);
		assert.deepEqual(
			result.failed.map((goal) => goal.id),
			['unknown']
		);
	} finally {
		globalThis.fetch = previous;
	}
});
