import assert from 'node:assert/strict';
import test from 'node:test';
import {
	directoryFilter,
	reviewVerdicts,
	statusWord,
	workPanel,
	type PanelWorkItem
} from './people-directory.ts';

const NOW = Date.parse('2026-10-14T12:00:00Z');
const day = (n: number) => new Date(NOW - n * 24 * 60 * 60 * 1000).toISOString();
const item = (
	id: string,
	status: PanelWorkItem['status'],
	updated: string,
	owner = 'elif'
): PanelWorkItem => ({ id, owner_id: owner, title: id, status, updated_at: updated });

test('the status word follows the row tone', () => {
	assert.equal(statusWord('work'), 'Working');
	assert.equal(statusWord('wait'), 'Waiting');
	assert.equal(statusWord('block'), 'Blocked');
	assert.equal(statusWord(''), 'Idle');
});

test('an unknown filter is Everyone', () => {
	assert.equal(directoryFilter('needs'), 'needs');
	assert.equal(directoryFilter('bogus'), 'everyone');
	assert.equal(directoryFilter(null), 'everyone');
});

test('Now is the Work in motion; Next and Done this week follow', () => {
	const work = [
		item('v5', 'completed', day(2)),
		item('v6', 'active', day(0)),
		item('pricing', 'proposed', day(1)),
		item('old', 'completed', day(20)),
		item('someone-else', 'active', day(0), 'emeka')
	];
	const panel = workPanel(work, new Set(['elif']), { now: NOW });
	assert.equal(panel.now?.id, 'v6');
	assert.equal(panel.nowDelivered, false);
	assert.deepEqual(
		panel.next.map((w) => w.id),
		['pricing']
	);
	assert.deepEqual(
		panel.done.map((w) => w.id),
		['v5']
	);
});

test('an idle agent shows their last delivered Work, so the panel is never empty', () => {
	const panel = workPanel(
		[item('v5', 'completed', day(2)), item('v4', 'completed', day(4))],
		new Set(['elif']),
		{ now: NOW }
	);
	assert.equal(panel.now?.id, 'v5');
	assert.equal(panel.nowDelivered, true);
	assert.deepEqual(
		panel.done.map((w) => w.id),
		['v4']
	);
});

test('a Work opened by address leads Now; a lead sees the whole team', () => {
	const work = [
		item('v6', 'active', day(0), 'elif'),
		item('copy', 'active', day(1), 'emeka'),
		item('v5', 'completed', day(2), 'elif')
	];
	const panel = workPanel(work, new Set(['elif', 'emeka']), { highlight: 'v5', now: NOW });
	assert.equal(panel.now?.id, 'v5');
	assert.equal(panel.nowDelivered, false);
	assert.deepEqual(
		panel.next.map((w) => w.id),
		['v6', 'copy']
	);
});

test("a review verdict is the reviewer's own outcome, read from its revises edge", () => {
	const work = [
		{ id: 'site', status: 'completed' },
		{ id: 'review', status: 'completed' },
		{ id: 'site-b', status: 'active' },
		{ id: 'review-b', status: 'active' },
		{ id: 'site-c', status: 'active' },
		{ id: 'review-c', status: 'active' }
	];
	const edges = [
		{ from_work_id: 'review', to_work_id: 'site', kind: 'revises' },
		{ from_work_id: 'review', to_work_id: 'site', kind: 'requires' },
		{ from_work_id: 'review-b', to_work_id: 'site-b', kind: 'revises' },
		{ from_work_id: 'review-c', to_work_id: 'site-c', kind: 'revises' }
	];
	const attempts = [
		{ work_id: 'review', revision: 1, attempt_no: 1, state: 'changes_requested' },
		{ work_id: 'review', revision: 2, attempt_no: 1, state: 'produced' },
		{ work_id: 'review-b', revision: 1, attempt_no: 2, state: 'changes_requested' },
		{ work_id: 'review-c', revision: 1, attempt_no: 1, state: 'running' }
	];
	const verdicts = reviewVerdicts(work, edges, attempts);
	assert.equal(verdicts.get('site')?.verdict, 'accepted');
	assert.equal(verdicts.get('site')?.reviewer.id, 'review');
	assert.equal(verdicts.get('site-b')?.verdict, 'changes');
	assert.equal(verdicts.get('site-c')?.verdict, 'pending');
	assert.equal(verdicts.has('review'), false);
});
