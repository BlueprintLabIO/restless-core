import assert from 'node:assert/strict';
import test from 'node:test';

import { FORMER_MEMBER, invitedFromShare, shareRows } from './share-people.ts';

const names: Record<string, string> = { alex: 'Alex', blair: 'Blair', exec: 'Exec' };

test('the share list names everyone, marks the viewer and says who added each person', () => {
	const rows = shareRows(
		[
			{ actor_id: 'alex', access: 'edit', added_by_actor_id: 'owner' },
			{ actor_id: 'blair', access: 'read', added_by_actor_id: 'alex' },
			{ actor_id: 'gone-7f3a', access: 'comment', added_by_actor_id: 'blair' },
			{ actor_id: 'exec', access: 'edit', added_by_actor_id: null }
		],
		{ owner: 'alex', viewer: 'blair', nameOf: (id) => names[id] }
	);
	// The owner is listed apart, so never twice.
	assert.deepEqual(
		rows.map((row) => [row.name, row.you, row.granted]),
		[
			['Blair', true, 'Added by Alex'],
			[FORMER_MEMBER, false, 'Added by you'],
			['Exec', false, '']
		]
	);
	assert(rows.every((row) => !row.name.includes('gone-7f3a')));
});

test('inviting from Share says when they will see it', () => {
	assert.match(invitedFromShare('a@example.com', 'private'), /Share this with them here/);
	assert.match(invitedFromShare('a@example.com', 'view'), /can open this once they join/);
});

test('a sheet or document that kept when someone was added says when', () => {
	const [row] = shareRows(
		[{ actor_id: 'blair', access: 'edit', added_by_actor_id: 'alex', added_at: '2026-10-03T09:00:00Z' }],
		{ owner: 'owner', viewer: 'alex', nameOf: (id) => names[id] }
	);
	assert.match(row.granted, /^Added by you, \S/);
});
