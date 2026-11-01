import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	nextFollow,
	queuedMessages,
	queuedRowState,
	queuedScheduleLabel,
	turnEnded,
	turnUnderWay
} from './queued-sends.ts';

test('F7: only a begun, unfinished turn makes a new message wait', () => {
	for (const phase of ['thinking', 'acting', 'responding'] as const)
		assert.equal(turnUnderWay(phase), true);
	for (const phase of ['queued', 'complete', 'failed', 'unavailable', null, undefined] as const)
		assert.equal(turnUnderWay(phase), false);
	assert.equal(turnEnded('complete'), true);
	assert.equal(turnEnded('acting'), false);
});

test('F7: the reply under way stays followed while messages queue behind it', () => {
	assert.equal(
		nextFollow({ lastFromViewer: 12, queued: [12], following: 10, phase: 'acting' }),
		null
	);
	assert.equal(
		nextFollow({ lastFromViewer: null, queued: [12, 13], following: 10, phase: 'responding' }),
		null
	);
});

test('F7: when the reply ends, the earliest queued message is followed next', () => {
	assert.equal(
		nextFollow({ lastFromViewer: null, queued: [13, 12], following: 10, phase: 'complete' }),
		12
	);
	assert.equal(
		nextFollow({ lastFromViewer: 13, queued: [12, 13], following: 10, phase: 'failed' }),
		12
	);
	assert.equal(
		nextFollow({ lastFromViewer: null, queued: [12], following: null, phase: null }),
		12
	);
});

test('F7: without a queue, the newest message of yours is followed as before', () => {
	assert.equal(nextFollow({ lastFromViewer: 14, queued: [], following: 10, phase: 'acting' }), 14);
	assert.equal(
		nextFollow({ lastFromViewer: null, queued: [], following: 10, phase: 'complete' }),
		null
	);
});

const message = (id: string, extra: Record<string, unknown> = {}) => ({
	id,
	from: 'you',
	readAt: null,
	...extra
});

test('F7: a queued message is yours, unread, and after the message being answered', () => {
	const messages = [
		message('9'),
		message('10'),
		{ id: '11', from: 'agent', readAt: null },
		message('12'),
		message('13', { readAt: '2026-11-02T00:00:00Z' }),
		message('optimistic:abc', { queued: true })
	];
	const turn = { triggerMessageId: 10 };
	assert.deepEqual(
		queuedMessages(messages, turn, true).map((entry) => entry.id),
		['12', 'optimistic:abc']
	);
	assert.deepEqual(queuedMessages(messages, turn, false), []);
	assert.deepEqual(queuedMessages(messages, null, true), []);
});

test('F7: a just-sent message is not queued unless it was sent behind a reply', () => {
	const turn = { triggerMessageId: 0 };
	assert.deepEqual(
		queuedMessages([message('optimistic:abc')], turn, true).map((entry) => entry.id),
		[]
	);
});

test('F7: each row names its state in one fixed slot', () => {
	const none = new Set<string>();
	assert.equal(queuedRowState({ id: 'optimistic:abc' }, none, none), 'queuing');
	assert.equal(queuedRowState({ id: '12' }, new Set(['12']), none), 'cancelling');
	assert.equal(queuedRowState({ id: '12' }, none, new Set(['12'])), 'failed');
	assert.equal(queuedRowState({ id: '12' }, none, none), 'queued');
	assert.equal(queuedScheduleLabel('queued'), 'After this reply');
	assert.equal(queuedScheduleLabel('queuing'), 'Queuing…');
	assert.equal(queuedScheduleLabel('cancelling'), 'Cancelling…');
	assert.equal(queuedScheduleLabel('failed'), 'Still queued');
});
