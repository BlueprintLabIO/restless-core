import assert from 'node:assert/strict';
import test from 'node:test';
import { goalRollup, partsDone, workEstimate, type ProgressRow } from './progress.ts';

const now = Date.parse('2026-11-03T12:00:00Z');
let next = 1;
const row = (
	work_id: string,
	percent: number | null,
	text: string,
	state: string = percent === null ? 'blocked' : percent === 100 ? 'done' : 'estimate',
	recorded_at = '2026-11-03T10:00:00Z'
): ProgressRow => ({
	id: next++,
	work_id,
	state,
	percent,
	next: text,
	estimated_by: 'print-direction',
	recorded_at
});
const name = (id: string) => (id === 'print-direction' ? 'Kai' : id);

test('no estimate shows nothing, not 0%', () => {
	assert.equal(workEstimate({ id: 'w', status: 'active' }, []), null);
	assert.equal(workEstimate({ id: 'w', status: 'active' }, undefined), null);
});

test('the newest estimate is current, with who and when and its trend', () => {
	const rows = [
		row('w', 20, 'sketch'),
		row('w', 60, 'export print files', 'estimate', '2026-11-03T11:00:00Z'),
		row('other', 90, 'x')
	];
	const estimate = workEstimate({ id: 'w', status: 'active' }, rows.toReversed(), name, now);
	assert.deepEqual(
		{ ...estimate, title: undefined },
		{
			fill: 60,
			percent: 60,
			tone: 'progress',
			text: '~60% · next: export print files',
			title: undefined,
			trend: [20, 60],
			recordedAt: '2026-11-03T11:00:00Z'
		}
	);
	assert.equal(estimate?.title, 'Rough estimate by Kai, 1 h ago');
});

test('blocked drops the number in words and keeps the bar where it was', () => {
	const rows = [row('w', 40, 'pick paper'), row('w', null, "waiting on the printer's spec")];
	const estimate = workEstimate({ id: 'w', status: 'blocked' }, rows, name, now);
	assert.equal(estimate?.percent, null);
	assert.equal(estimate?.fill, 40);
	assert.equal(estimate?.tone, 'blocked');
	assert.equal(estimate?.text, "Blocked: waiting on the printer's spec");
});

test('done is 100% even when nobody estimated it', () => {
	const done = workEstimate({ id: 'w', status: 'completed' }, []);
	assert.equal(done?.percent, 100);
	assert.equal(done?.text, '100% · done');
	const estimated = workEstimate(
		{ id: 'w', status: 'completed' },
		[row('w', 60, 'x'), row('w', 100, '')],
		name,
		now
	);
	assert.deepEqual(estimated?.trend, [60, 100]);
	assert.match(estimated?.title ?? '', /^Recorded when it was done/);
});

test('stopped Work keeps its last estimate, marked stopped', () => {
	const stopped = workEstimate(
		{ id: 'w', status: 'abandoned' },
		[row('w', 30, 'pick paper')],
		name,
		now
	);
	assert.equal(stopped?.text, '~30% · stopped');
	assert.equal(stopped?.tone, 'stopped');
	assert.equal(workEstimate({ id: 'w', status: 'abandoned' }, []), null);
});

test('a goal rolls up its parts equally, leaving stopped Work out', () => {
	const work = [
		{ id: 'a', status: 'completed', goal_id: 'g' },
		{ id: 'b', status: 'active', goal_id: 'g' },
		{ id: 'c', status: 'blocked', goal_id: 'g' },
		{ id: 'd', status: 'proposed', goal_id: 'g' },
		{ id: 'e', status: 'abandoned', goal_id: 'g' },
		{ id: 'f', status: 'active', goal_id: 'other' }
	];
	const rows = [row('b', 40, 'x'), row('c', 20, 'y'), row('c', null, 'no van'), row('e', 90, 'z')];
	const rollup = goalRollup('g', work, rows);
	assert.deepEqual(rollup, { percent: Math.round((100 + 40 + 20 + 0) / 4), done: 1, total: 4 });
	assert.equal(partsDone(rollup), '1 of 4 parts done');
	assert.equal(partsDone({ percent: null, done: 1, total: 1 }), '1 of 1 part done');
});

test('a goal nobody has estimated shows nothing', () => {
	const work = [
		{ id: 'a', status: 'active', goal_id: 'g' },
		{ id: 'b', status: 'proposed', goal_id: 'g' }
	];
	assert.equal(goalRollup('g', work, []).percent, null);
	assert.equal(goalRollup('empty', work, []).percent, null);
});
