import assert from 'node:assert/strict';
import test from 'node:test';
import {
	forYouSentence,
	newlyUnread,
	unreadMarks,
	withRead,
	type ForYouItem,
	type ForYouPage
} from './for-you.ts';

function item(key: string, unread: boolean, at = '2026-10-13T09:00:00Z'): ForYouItem {
	return {
		key,
		kind: 'change',
		title: 'Pricing',
		excerpt: '',
		href: '/acme/library/documents?document=1',
		actor_id: 'bianca',
		actor_name: 'Bianca and Exec',
		at,
		unread
	};
}

test('each kind reads as one sentence, and a direct message names no room', () => {
	assert.equal(forYouSentence(item('changes:a', true)), 'Bianca and Exec changed Pricing');
	assert.equal(
		forYouSentence({ kind: 'mention', actor_name: 'Alex', title: '' }),
		'Alex mentioned you'
	);
	assert.equal(
		forYouSentence({ kind: 'mention', actor_name: 'Alex', title: 'Launch' }),
		'Alex mentioned you in Launch'
	);
	assert.equal(
		forYouSentence({ kind: 'reply', actor_name: 'Exec', title: 'Plan' }),
		'Exec replied in Plan'
	);
	assert.equal(
		forYouSentence({ kind: 'comment', actor_name: '', title: 'Plan' }),
		'Someone commented on Plan'
	);
});

test('reading an item lowers the count once, and mark all clears it', () => {
	const page: ForYouPage = {
		items: [item('a', true), item('b', true), item('c', false)],
		unread_count: 2,
		next_before: null
	};
	const once = withRead(page, ['a', 'c']);
	assert.equal(once.unread_count, 1);
	assert.deepEqual(
		once.items.map((row) => row.unread),
		[false, true, false]
	);
	assert.equal(withRead(once, ['a']).unread_count, 1, 'reading it twice changes nothing');
	assert.equal(withRead(page, 'all').unread_count, 0);
	assert.ok(withRead(page, 'all').items.every((row) => !row.unread));
});

test('a desktop notice is for news only: not the first reading, not what was already unread', () => {
	const first: ForYouPage = { items: [item('a', true)], unread_count: 1, next_before: null };
	assert.deepEqual(newlyUnread(null, first), []);
	const known = unreadMarks(first);
	assert.deepEqual(newlyUnread(known, first), []);
	const later: ForYouPage = {
		items: [item('a', true, '2026-10-13T10:00:00Z'), item('b', true), item('c', false)],
		unread_count: 2,
		next_before: null
	};
	assert.deepEqual(
		newlyUnread(known, later).map((row) => row.key),
		['a', 'b'],
		'new activity on a known item is news again'
	);
});
