import assert from 'node:assert/strict';
import test from 'node:test';
import { decidedNotice, leaksInternals, ownerLine, RESOLVED_ELSEWHERE } from './owner-copy.ts';
import { blockerSentence } from './work-actions.ts';

/* The record that reached the owner on 3 Nov 2026, word for word. */
const ABANDONED =
	'Withdrawn because Work was abandoned by reno-supervisor: Owner msg 1513 … supersedes the v3 digests this delivery was bound to; replaced by rewrite 44b3016c, check 4 a81cc8b2 and a new delivery node.';
const TITLE = 'Approve sending the four prospect emails once sending is switched on';

/* The guard: nothing the owner reads carries a hex id, a message number or an agent handle. */
const HEX = /\b[0-9a-f]{8,}\b/i;
const MSG = /\bmsg\s*\d+/i;
const HANDLE = /\b[a-z][a-z0-9]*(?:-[a-z0-9]+)+\b/;
function assertOwnerSafe(text: string) {
	assert.doesNotMatch(text, HEX, `hex id in ${JSON.stringify(text)}`);
	assert.doesNotMatch(text, MSG, `message number in ${JSON.stringify(text)}`);
	assert.doesNotMatch(text, HANDLE, `agent handle in ${JSON.stringify(text)}`);
	assert.doesNotMatch(text, /delivery node|digests|bound to|check \d/i);
}

const OUTCOMES = [
	'approved',
	'declined',
	'changes_requested',
	'completed',
	'revoked',
	'withdrawn',
	'replaced',
	'expired',
	undefined
] as const;

test('every decided notice is the platform’s words, never the record’s reason', () => {
	for (const outcome of OUTCOMES)
		for (const viewerIsOwner of [true, false])
			for (const replacementInInbox of [true, false])
				for (const who of [
					{},
					{ byOwner: true },
					{ byOwner: true, viaChat: true, decidedBy: 'Exec' },
					{ decidedBy: 'Avery Holt' }
				]) {
					const notice = decidedNotice(
						{ title: TITLE, outcome, ...who },
						{ viewerIsOwner, replacementInInbox }
					);
					assertOwnerSafe(notice.text);
					assertOwnerSafe(notice.title);
					assert.ok(notice.text.length > 0);
				}
	assertOwnerSafe(RESOLVED_ELSEWHERE);
});

test('a replaced item says so plainly, and where the new one is', () => {
	const decision = { title: TITLE, outcome: 'replaced' as const };
	assert.deepEqual(decidedNotice(decision, { viewerIsOwner: true, replacementInInbox: true }), {
		title: TITLE,
		text: 'This was replaced by an updated version. The new one is in your Inbox.'
	});
	assert.equal(
		decidedNotice(decision, { viewerIsOwner: true }).text,
		'This was replaced by an updated version. If it needs you, it will come to your Inbox.'
	);
});

test('each state reads as one plain sentence', () => {
	const say = (decision: Parameters<typeof decidedNotice>[0], viewerIsOwner = true) =>
		decidedNotice(decision, { viewerIsOwner }).text;
	assert.equal(say({ title: TITLE, outcome: 'withdrawn' }), 'Withdrawn: this is no longer needed.');
	assert.equal(say({ title: TITLE, outcome: 'expired' }), 'This expired before it was decided.');
	assert.equal(
		say({ title: TITLE, outcome: 'approved', byOwner: true }),
		'You already approved this.'
	);
	assert.equal(
		say({ title: TITLE, outcome: 'declined', byOwner: true }),
		'You already declined this.'
	);
	assert.equal(
		say({ title: TITLE, outcome: 'approved', byOwner: true, viaChat: true, decidedBy: 'Exec' }),
		'Exec approved this from your chat.'
	);
	assert.equal(
		say({ title: TITLE, outcome: 'declined', decidedBy: 'Avery Holt' }),
		'Avery Holt declined this.'
	);
	assert.equal(say({ title: TITLE, outcome: 'approved' }), 'This was already approved.');
	assert.equal(
		say({ title: TITLE, outcome: 'approved', byOwner: true }, false),
		'The owner approved this.'
	);
});

test('a title that leaks internals is left out rather than quoted', () => {
	assert.equal(
		decidedNotice({ title: 'Ship rewrite 44b3016c', outcome: 'withdrawn' }, { viewerIsOwner: true })
			.title,
		''
	);
});

test('leaksInternals catches ids, hashes, message numbers, handles and engine nouns', () => {
	assert.ok(leaksInternals(ABANDONED));
	assert.ok(leaksInternals('waiting on 44b3016c'));
	assert.ok(leaksInternals('see msg 1513'));
	assert.ok(leaksInternals('held by reno-supervisor'));
	assert.ok(leaksInternals('a new delivery node'));
	assert.ok(leaksInternals('Paused for bodhi_ops', ['bodhi_ops']));
	assert.ok(!leaksInternals('Waiting for the client to send the signed contract.'));
	assert.ok(!leaksInternals('Paused: the follow-up email waits for Monday.'));
	assert.equal(ownerLine(ABANDONED, 'Stopped: no longer needed.'), 'Stopped: no longer needed.');
	assert.equal(
		ownerLine('Waiting for the signed contract.', 'x'),
		'Waiting for the signed contract.'
	);
});

test('a blocker line never quotes a leaky reason', () => {
	assert.equal(
		blockerSentence('paused by reno-supervisor: superseded by 44b3016c', '', 'Blocked.'),
		'Blocked.'
	);
	assert.equal(blockerSentence('paused by ops-lead: waiting on 9f3e21aa77'), '');
	assert.equal(
		blockerSentence(
			'awaiting accountable-lead outcome review 3f2b8c1e-1111-4222-8333-444455556666'
		),
		'Waiting for its lead to review the result.'
	);
	assert.equal(
		blockerSentence('paused by ops-lead: waiting for the signed contract'),
		'Paused: waiting for the signed contract'
	);
	assert.equal(blockerSentence('', ''), '');
});
