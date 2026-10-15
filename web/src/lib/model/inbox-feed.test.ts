import assert from 'node:assert/strict';
import test from 'node:test';
import { inboxFeed } from './inbox-feed.ts';
import type { AttentionItem } from './view';
import type { ForYouItem } from './for-you';

const action = (id: string, at: string, extra = {}) =>
	({ id, category: 'review', createdAt: at, canContinue: false, ...extra }) as AttentionItem;
const update = (key: string, at: string, extra = {}) =>
	({ key, kind: 'share', at, unread: true, ...extra }) as ForYouItem;
const day = (number: number) => '2026-10-' + String(number).padStart(2, '0') + 'T10:00:00Z';

test('actions keep a consistent order when attention and document reviews are interleaved', () => {
	const early = action('early', day(1)),
		late = action('late', day(3));
	const review = update('middle', day(2), { kind: 'review' });
	for (const attention of [
		[early, late],
		[late, early]
	]) {
		assert.deepEqual(
			inboxFeed(attention, [review]).map((row) => row.key),
			['attention:early', 'update:middle', 'attention:late']
		);
	}
});

test('blockers and deadlines precede ordinary actions; unread updates precede read updates', () => {
	const attention = [
		action('ordinary', day(1), { canContinue: true }),
		action('deadline', day(3), { deadline: day(4) }),
		action('blocker', day(5), { category: 'blocker' })
	];
	const updates = [
		update('read', day(7), { unread: false }),
		update('old', day(4)),
		update('new', day(6))
	];
	assert.deepEqual(
		inboxFeed(attention, updates).map((row) => row.key),
		[
			'attention:blocker',
			'attention:deadline',
			'attention:ordinary',
			'update:new',
			'update:old',
			'update:read'
		]
	);
	assert.deepEqual(
		attention.map((item) => item.id),
		['ordinary', 'deadline', 'blocker']
	);
	assert.deepEqual(
		updates.map((item) => item.key),
		['read', 'old', 'new']
	);
});
