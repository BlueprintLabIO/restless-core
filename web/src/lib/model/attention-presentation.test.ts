import assert from 'node:assert/strict';
import test from 'node:test';
import {
	coordinationFailure,
	judgementHolders,
	workWaitingOnOwner
} from './attention-presentation.ts';
import { describeFailure } from './failure.ts';
import type { AttentionItem } from './view';

const item = (fields: Partial<AttentionItem>) => fields as AttentionItem;

test('Work waits on the owner only for what the Inbox shows, once prepared', () => {
	const waiting = workWaitingOnOwner([
		item({ id: 'a', workId: 'w1' }),
		item({ id: 'b', workId: 'w2', preparing: true }),
		item({ id: 'c' })
	]);
	assert.deepEqual([...waiting], ['w1']);
});

test('judgement a lead still holds is attributed to them, not the owner', () => {
	const held = judgementHolders([
		{ work_id: 'w1', state: 'pending', assigned_to: 'lead-a' },
		{ work_id: 'w2', state: 'pending', assigned_to: null },
		{ work_id: 'w3', state: 'resolved', assigned_to: 'lead-b' }
	]);
	assert.deepEqual([...held], [['w1', 'lead-a']]);
});

test('unavailable coordination reads as a failure, never as an empty company', () => {
	assert.equal(coordinationFailure({ sourceHealth: { orgintel: 'available' } }), null);
	assert.equal(coordinationFailure(null), null);
	const failure = coordinationFailure({ sourceHealth: { orgintel: 'unavailable' } });
	assert.ok(failure);
	assert.equal(describeFailure(failure).kind, 'unavailable');
	assert.match(describeFailure(failure).detail, /coordination is unavailable/);
});
