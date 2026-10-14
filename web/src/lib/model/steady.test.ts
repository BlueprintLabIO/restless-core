import { test } from 'node:test';
import assert from 'node:assert/strict';
import { QueryClient } from '@tanstack/query-core';
import { expectRemoval, hintTouches, parseHint, steady, vanished } from './steady.ts';

test('a message never reloads settings, the principal or model connections', () => {
	for (const key of ['company-principal', 'intelligence', 'company-identity', 'connected-models']) {
		assert.equal(hintTouches(['message'], [key, 'acme']), false);
		assert.equal(hintTouches(['work_changed'], [key, 'acme']), false);
	}
	assert.equal(hintTouches(['message'], ['conversation', 'acme', 'exec', null]), true);
	assert.equal(hintTouches(['room_event'], ['attention', 'acme']), true);
	assert.equal(hintTouches(['message'], ['documents', 'acme']), false);
});

test('Work changes and unknown kinds refresh everything but settings', () => {
	assert.equal(hintTouches(['work_changed'], ['documents', 'acme']), true);
	assert.equal(hintTouches(['something_new'], ['documents', 'acme']), true);
	assert.equal(hintTouches(['message', 'library_changed'], ['documents', 'acme']), true);
	assert.equal(hintTouches(['resync'], ['intelligence', 'acme']), false);
	assert.deepEqual(parseHint(''), ['resync']);
	assert.equal(hintTouches(['work_changed'], ['schedules', 'acme']), false);
	assert.equal(hintTouches(['schedule_changed'], ['schedules', 'acme']), true);
});

test('the tripwire reports a shown message that disappears, not older history or a retract', () => {
	const before = [{ id: 10 }, { id: 11 }, { id: 12 }];
	assert.deepEqual(vanished(before, [{ id: 10 }, { id: 11 }]), ['12']);
	assert.deepEqual(vanished(before, [{ id: 11 }, { id: 12 }, { id: 13 }]), []);
	expectRemoval(12);
	assert.deepEqual(vanished(before, [{ id: 10 }, { id: 11 }]), []);
});

test('a read that started before a local write cannot land over it', async () => {
	const client = steady(new QueryClient());
	const key = ['conversation', 'acme', 'exec', null];
	client.setQueryData(key, { messages: [{ id: 1 }] });
	let answer: (value: unknown) => void = () => {};
	const stale = client.fetchQuery({
		queryKey: key,
		queryFn: () => new Promise((resolve) => (answer = resolve)),
		staleTime: 0
	});
	// The send's own answer: the message that a slow refresh would otherwise erase.
	client.setQueryData(key, { messages: [{ id: 1 }, { id: 2 }] });
	answer({ messages: [{ id: 1 }] });
	await stale.catch(() => undefined);
	assert.deepEqual(client.getQueryData(key), { messages: [{ id: 1 }, { id: 2 }] });
});
