import assert from 'node:assert/strict';
import test from 'node:test';
import { emptyWorkTitle, matchesSearch, goalsWithFinishedWork } from './work-views.ts';

test('a filter says what it found nothing of; only a goal talks about goals', () => {
	assert.equal(emptyWorkTitle('waiting'), 'Nothing needs you.');
	assert.equal(emptyWorkTitle('active'), 'Nothing is in progress.');
	assert.equal(emptyWorkTitle('archived'), 'Nothing is archived.');
	assert.equal(emptyWorkTitle('other'), 'All work serves a goal.');
	assert.equal(emptyWorkTitle(''), 'No work yet');
	assert.equal(
		emptyWorkTitle('4f0c2d1e-0000-4000-8000-000000000000'),
		'No work under this goal yet'
	);
});

test('a search or filter that found nothing never says the company has no work', () => {
	assert.equal(emptyWorkTitle('', ' launch '), 'No work matches “launch”');
	assert.equal(emptyWorkTitle('active', '', true), 'No work matches these filters');
	assert.notEqual(emptyWorkTitle('', 'x'), emptyWorkTitle(''));
});

test('search reads every word on the row, not only the title', () => {
	assert.ok(matchesSearch('ada', ['Write the deck', 'brief', 'Ada Lovelace']));
	assert.ok(matchesSearch('LAUNCH', [null, 'Launch site goal']));
	assert.ok(matchesSearch('  ', ['anything']));
	assert.ok(!matchesSearch('pricing', ['Write the deck', undefined, 'Ada']));
});

test('only an open goal with completed work can be offered for explicit archiving', () => {
	const goals = [
		{ id: 'done' },
		{ id: 'active' },
		{ id: 'stopped' },
		{ id: 'empty' },
		{ id: 'closed', closed_at: '2026-10-01' }
	];
	const work = [
		{ goal_id: 'done', status: 'completed' },
		{ goal_id: 'done', status: 'completed' },
		{ goal_id: 'active', status: 'completed' },
		{ goal_id: 'active', status: 'blocked' },
		{ goal_id: 'stopped', status: 'abandoned' },
		{ goal_id: 'closed', status: 'completed' }
	];
	assert.deepEqual([...goalsWithFinishedWork(goals, work)], ['done']);
});
