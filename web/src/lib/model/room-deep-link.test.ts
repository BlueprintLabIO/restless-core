import assert from 'node:assert/strict';
import test from 'node:test';
// @ts-expect-error Node's strip-only test runner needs the explicit TypeScript suffix.
import { locateRoomMessageTarget, parseRoomMessageTarget } from './room-deep-link.ts';
import type { RoomMention, RoomMessage, RoomMessagePage } from './rooms';

const ROOM_ID = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
const MENTION_ID = 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb';

function message(id: number, deletedAt: string | null = null): RoomMessage {
	return {
		id,
		room_id: ROOM_ID,
		from_actor: 'owner',
		to_actor: null,
		body: deletedAt ? '' : `Message ${id}`,
		outcome_standard: null,
		parent_message_id: id === 10 ? null : 10,
		thread_root_message_id: id === 10 ? null : 10,
		client_command_id: `command-${id}`,
		created_at: '2026-09-10T00:00:00Z',
		revision_number: 0,
		edited_at: null,
		deleted_at: deletedAt,
		legacy_read_at: null
	};
}

function mention(messageId: number): RoomMention {
	return {
		id: MENTION_ID,
		room_id: ROOM_ID,
		message_id: messageId,
		thread_root_message_id: 10,
		mentioned_actor_id: 'alice',
		kind: 'direct',
		work_id: null,
		why_this_actor: null,
		expected_response: null,
		recommendation: null,
		alternatives: [],
		evidence: [],
		uncertainty: null,
		affected_scope: null,
		deadline_at: null,
		fallback: null,
		independent_work_can_continue: true,
		created_event_id: 1,
		resolution_message_id: null,
		resolved_event_id: null,
		cancelled_event_id: null,
		cancelled_by: null,
		cancellation_reason: null,
		created_at: '2026-09-10T00:00:00Z',
		resolved_at: null,
		cancelled_at: null
	};
}

function page(
	messages: RoomMessage[],
	mentions: RoomMention[],
	nextBeforeMessageId: number | null,
	hasMore: boolean
): RoomMessagePage {
	return {
		messages,
		mentions,
		next_before_message_id: nextBeforeMessageId,
		has_more: hasMore
	};
}

const target = {
	roomId: ROOM_ID,
	threadRootMessageId: 10,
	messageId: 75,
	mentionId: MENTION_ID
};

test('an exact Room target keyset-pages beyond the latest page and finds its mention', async () => {
	const latestReplies = Array.from({ length: 50 }, (_, index) => message(101 + index));
	const olderReplies = Array.from({ length: 50 }, (_, index) => message(51 + index));
	const pages = [page([message(10), ...latestReplies], [], 101, true)];
	let loads = 0;
	const located = await locateRoomMessageTarget(target, {
		pages: () => pages,
		hasMore: () => pages.at(-1)?.has_more ?? false,
		loadMore: async () => {
			loads += 1;
			pages.push(page([message(10), ...olderReplies], [mention(75)], null, false));
		}
	});

	assert.equal(loads, 1);
	assert.equal(located.state, 'found');
	if (located.state === 'found') assert.equal(located.message.id, 75);
});

test('a missing exact Room target exhausts finite history as unavailable', async () => {
	const pages = [page([message(10), message(101), message(150)], [], 101, true)];
	const located = await locateRoomMessageTarget(target, {
		pages: () => pages,
		hasMore: () => pages.at(-1)?.has_more ?? false,
		loadMore: async () => {
			pages.push(page([message(10), message(51), message(74), message(100)], [], null, false));
		}
	});

	assert.deepEqual(located, { state: 'unavailable', reason: 'missing' });
});

test('a deleted exact Room target is explicit and never treated as a focusable mention', async () => {
	const pages = [page([message(10), message(75, '2026-09-10T01:00:00Z')], [], null, false)];
	const located = await locateRoomMessageTarget(target, {
		pages: () => pages,
		hasMore: () => false,
		loadMore: async () => assert.fail('a loaded tombstone must stop paging')
	});

	assert.deepEqual(located, { state: 'unavailable', reason: 'deleted' });
});

test('malformed or partial exact Room coordinates are rejected before paging', () => {
	const invalid = [
		`room=not-a-uuid&thread=10&message=75&mention=${MENTION_ID}`,
		`room=${ROOM_ID}&thread=1e1&message=75&mention=${MENTION_ID}`,
		`room=${ROOM_ID}&thread=10&message=9&mention=${MENTION_ID}`,
		`room=${ROOM_ID}&thread=10&message=75&mention=orgintel%3Amention%3A${MENTION_ID}`,
		`room=${ROOM_ID}&thread=10&message=75`,
		`room=${ROOM_ID}&thread=10&message=75&message=76&mention=${MENTION_ID}`
	];
	for (const query of invalid) {
		assert.deepEqual(parseRoomMessageTarget(new URLSearchParams(query)), { kind: 'invalid' });
	}
});

test('an exact Room coordinate parses only source-owned UUIDs and decimal message ids', () => {
	assert.deepEqual(
		parseRoomMessageTarget(
			new URLSearchParams(
				`room=${ROOM_ID.toUpperCase()}&thread=10&message=75&mention=${MENTION_ID.toUpperCase()}`
			)
		),
		{ kind: 'target', target }
	);
});

test('ordinary Room and Thread navigation has no exact target', () => {
	assert.deepEqual(parseRoomMessageTarget(new URLSearchParams()), { kind: 'none' });
	assert.deepEqual(parseRoomMessageTarget(new URLSearchParams(`room=${ROOM_ID}&thread=10`)), {
		kind: 'none'
	});
});

test('exact-target paging has a hard page ceiling', async () => {
	const pages = [page([message(10), message(100)], [], 100, true)];
	let loads = 0;
	const located = await locateRoomMessageTarget(
		{ ...target, messageId: 25 },
		{
			pages: () => pages,
			hasMore: () => true,
			loadMore: async () => {
				loads += 1;
				pages.push(page([message(10), message(50)], [], 50, true));
			}
		},
		1
	);

	assert.equal(loads, 1);
	assert.deepEqual(located, { state: 'unavailable', reason: 'page-limit' });
});
