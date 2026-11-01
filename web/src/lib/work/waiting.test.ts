import assert from 'node:assert/strict';
import test from 'node:test';
import type { AttentionItem } from '../model/view.ts';
import { actionPhrase, inboxWaitingLine, workWaitingLine, type WaitingSources } from './waiting.ts';

const names: Record<string, string> = { baloo: 'Baloo', aang: 'Aang', exec: 'Exec' };
const base: WaitingSources = {
	viewer: 'owner',
	viewerIsOwner: true,
	name: (id) => names[id] ?? id
};
const work = (status: string, extra: Partial<{ id: string; resolution: string }> = {}) => ({
	id: 'w1',
	owner_id: 'baloo',
	status,
	resolution: '',
	...extra
});
const item = (extra: Partial<AttentionItem>) =>
	({
		id: 'i1',
		workId: 'w1',
		category: 'approval',
		title: 'Brand rules',
		requestedAction: 'Approve the brand rules.',
		responsibleActor: { id: 'baloo', display: 'Baloo', role: 'lead' },
		...extra
	}) as AttentionItem;
const attempt = (state: string, actor_id = 'baloo', attempt_no = 1) => ({
	work_id: 'w1',
	actor_id,
	state,
	revision: 1,
	attempt_no,
	started_at: `2026-11-0${attempt_no}T00:00:00Z`,
	summary: ''
});

test('an authored action reads as the tail of the line, names kept', () => {
	assert.equal(actionPhrase('Approve the brand rules.'), 'approve the brand rules');
	assert.equal(
		actionPhrase('Needs you: announce 3 November or 10 November?'),
		'announce 3 November or 10 November'
	);
	assert.equal(actionPhrase('Exec needs a login'), 'Exec needs a login');
	assert.equal(actionPhrase('DNS record at GoDaddy'), 'DNS record at GoDaddy');
	assert.equal(actionPhrase('  '), '');
});

test('finished and stopped Work waits on nobody', () => {
	assert.equal(workWaitingLine(work('completed'), base), null);
	assert.equal(workWaitingLine(work('abandoned'), base), null);
});

test('a decision in the owner queue waits on you, ahead of everything else', () => {
	const sources = { ...base, items: [item({})], attempts: [attempt('running')] };
	assert.equal(workWaitingLine(work('active'), sources), 'Waiting on you: approve the brand rules');
	assert.equal(
		workWaitingLine(work('active'), {
			...base,
			items: [item({ requestedAction: '', category: 'review' })]
		}),
		'Waiting on you: your review'
	);
});

test('a step still being prepared waits on whoever prepares it', () => {
	assert.equal(
		workWaitingLine(work('active'), { ...base, items: [item({ preparing: true })] }),
		'Waiting on Baloo: preparing your step'
	);
});

test('a handoff passed to a team owner waits on them', () => {
	assert.equal(
		workWaitingLine(work('active'), {
			...base,
			viewer: 'member',
			viewerIsOwner: false,
			viewerHandoffs: [{ work_id: 'w1', requested_action: 'Choose a supplier.' }]
		}),
		'Waiting on you: choose a supplier'
	);
});

test('a judgement a lead holds names the lead; one owed to the owner names them', () => {
	const held = (assigned_to: string | null) => ({
		...base,
		handoffs: [{ work_id: 'w1', state: 'pending', assigned_to, requested_action: 'Pick the logo.' }]
	});
	assert.equal(workWaitingLine(work('active'), held('aang')), 'Waiting on Aang: pick the logo');
	assert.equal(workWaitingLine(work('active'), held(null)), 'Waiting on you: pick the logo');
	assert.equal(
		workWaitingLine(work('active'), { ...held(null), viewer: 'm', viewerIsOwner: false }),
		'Waiting on the owner: pick the logo'
	);
	const resolved = {
		...base,
		handoffs: [{ work_id: 'w1', state: 'resolved', assigned_to: 'aang', requested_action: 'x' }]
	};
	assert.equal(workWaitingLine(work('proposed'), resolved), 'Waiting on Baloo: not started yet');
});

test('Next Work behind unfinished Work names what it waits for', () => {
	const sources = {
		...base,
		work: [
			{ id: 'a', title: 'Draft the copy', status: 'active' },
			{ id: 'b', title: 'Pick photos', status: 'proposed' },
			{ id: 'c', title: 'Old', status: 'completed' }
		],
		edges: [
			{ from_work_id: 'a', to_work_id: 'w1', kind: 'requires' },
			{ from_work_id: 'b', to_work_id: 'w1', kind: 'requires' },
			{ from_work_id: 'c', to_work_id: 'w1', kind: 'requires' },
			{ from_work_id: 'w1', to_work_id: 'a', kind: 'revises' }
		]
	};
	assert.equal(
		workWaitingLine(work('proposed'), sources),
		'Waiting on other Work: Draft the copy and 1 more'
	);
	assert.equal(
		workWaitingLine(work('proposed'), { ...sources, edges: sources.edges.slice(2) }),
		'Waiting on Baloo: not started yet'
	);
});

test('blocked Work waits on whoever can resume it, with its reason unless shown elsewhere', () => {
	const blocked = work('blocked', { resolution: 'The site host refused the key.' });
	assert.equal(
		workWaitingLine(blocked, base),
		'Waiting on you: resume it. The site host refused the key'
	);
	assert.equal(workWaitingLine(blocked, base, { reason: false }), 'Waiting on you: resume it');
	assert.equal(
		workWaitingLine(work('blocked'), { ...base, viewer: 'm', viewerIsOwner: false }),
		'Waiting on the owner: resume it'
	);
});

test('a lead without a usable model waits on a model, in the rail’s words', () => {
	const route = (state: 'needs_connection' | 'unavailable' | 'ready' | 'checking') => ({
		...base,
		route: () => state
	});
	assert.equal(
		workWaitingLine(work('active'), route('needs_connection')),
		'Waiting on a model: connect a model for Baloo'
	);
	assert.equal(
		workWaitingLine(work('proposed'), route('unavailable')),
		'Waiting on a model: check model connection for Baloo'
	);
	assert.equal(
		workWaitingLine(work('proposed'), route('checking')),
		'Waiting on Baloo: not started yet'
	);
	/* The run's actor, not the Work's owner, is the one who needs the model. */
	const staffed = {
		...base,
		attempts: [attempt('running', 'aang')],
		route: (id: string) => (id === 'aang' ? ('needs_connection' as const) : ('ready' as const))
	};
	assert.equal(
		workWaitingLine(work('active'), staffed),
		'Waiting on a model: connect a model for Aang'
	);
});

test('Work in motion waits on whoever runs it, by the newest run', () => {
	const run = (...attempts: ReturnType<typeof attempt>[]) => ({ ...base, attempts });
	assert.equal(
		workWaitingLine(work('active'), run(attempt('running', 'aang'))),
		'Waiting on Aang: running'
	);
	assert.equal(
		workWaitingLine(
			work('active'),
			run(attempt('running', 'aang', 1), attempt('failed', 'baloo', 2))
		),
		'Waiting on Baloo: a new run after one failed'
	);
	assert.equal(
		workWaitingLine(work('active'), run(attempt('changes_requested'))),
		'Waiting on Baloo: the changes asked for'
	);
	assert.equal(workWaitingLine(work('active'), run()), 'Waiting on Baloo: the next run');
	assert.equal(
		workWaitingLine(work('active'), { ...run(attempt('running', 'owner')) }),
		'Waiting on you: running'
	);
});

test('an Inbox row says it waits on you, or on whoever prepares it', () => {
	assert.equal(inboxWaitingLine(item({})), 'Waiting on you: approve the brand rules · from Baloo');
	assert.equal(inboxWaitingLine(item({ preparing: true })), 'Waiting on Baloo: preparing it');
	assert.equal(
		inboxWaitingLine(
			item({ requestedAction: '', category: 'human_step', responsibleActor: undefined })
		),
		'Waiting on you: your step · from Exec'
	);
});
