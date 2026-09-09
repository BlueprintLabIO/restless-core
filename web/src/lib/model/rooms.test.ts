import assert from 'node:assert/strict';
import test from 'node:test';
// @ts-expect-error Node's strip-only test runner needs the explicit TypeScript suffix.
import * as roomsModel from './rooms.ts';
import type { RoomEvent, RoomMention, RoomTransport } from './rooms';

const {
	deleteRoomMessage,
	editRoomMessage,
	editRoomMessagePages,
	getRoomMessageRevisions,
	getRoomMessages,
	getRoomReadCursor,
	getRoomThread,
	openRoomEventStream,
	roomDraftKey,
	removeRoomMessageSearchResult,
	searchRoomMessages,
	searchRooms,
	sendRoomMessage,
	tombstoneRoomMessagePages
} = roomsModel;

class FakeEventSource {
	static instances: FakeEventSource[] = [];
	onopen: (() => void) | null = null;
	onerror: (() => void) | null = null;
	closed = false;
	readonly url: string;
	readonly listeners = new Map<string, Array<(event: { data: string }) => void>>();

	constructor(url: string) {
		this.url = url;
		FakeEventSource.instances.push(this);
	}

	addEventListener(name: string, listener: EventListener) {
		const listeners = this.listeners.get(name) ?? [];
		listeners.push(listener as unknown as (event: { data: string }) => void);
		this.listeners.set(name, listeners);
	}

	emit(name: string, value: unknown = {}) {
		for (const listener of this.listeners.get(name) ?? []) {
			listener({ data: JSON.stringify(value) });
		}
	}

	close() {
		this.closed = true;
	}
}

test('Room stream advances events and turns compacted cursors into an explicit resync', () => {
	const original = globalThis.EventSource;
	globalThis.EventSource = FakeEventSource as unknown as typeof EventSource;
	FakeEventSource.instances = [];
	try {
		const transports: RoomTransport[] = [];
		const events: RoomEvent[] = [];
		const restarts: string[] = [];
		openRoomEventStream(
			'rooms_ui_test',
			'room id',
			41,
			(event) => events.push(event),
			(transport) => transports.push(transport),
			(reason) => restarts.push(reason)
		);
		const source = FakeEventSource.instances[0];
		assert.equal(
			source.url,
			'/api/companies/rooms_ui_test/rooms/room%20id/events/live?after_event_id=41'
		);

		source.onopen?.();
		source.emit('room-event', {
			id: 42,
			kind: 'message.created',
			room_id: 'room id',
			actor_id: 'exec',
			message_id: 9,
			created_at: '2026-09-09T00:00:00Z'
		});
		assert.equal(events[0]?.id, 42);

		source.emit('resync', { reason: 'cursor_unavailable', resume_after_event_id: 88 });
		assert.equal(source.closed, true);
		assert.deepEqual(restarts, ['resync']);
		assert.deepEqual(transports, ['connecting', 'live', 'reconnecting']);

		/* The owner server closes after resync. Its ensuing transport error must
		 * not schedule a second restart for the already-retired stream. */
		source.onerror?.();
		assert.deepEqual(restarts, ['resync']);
	} finally {
		globalThis.EventSource = original;
	}
});

test('Room stream retires native retry and reports a transport restart', () => {
	const original = globalThis.EventSource;
	globalThis.EventSource = FakeEventSource as unknown as typeof EventSource;
	FakeEventSource.instances = [];
	try {
		const restarts: string[] = [];
		openRoomEventStream(
			'acme',
			'room',
			7,
			() => {},
			() => {},
			(reason) => restarts.push(reason)
		);
		const source = FakeEventSource.instances[0];
		source.onerror?.();
		assert.equal(source.closed, true);
		assert.deepEqual(restarts, ['transport']);
	} finally {
		globalThis.EventSource = original;
	}
});

test('Room reads use only the latest-first before cursor contract', async () => {
	const original = globalThis.fetch;
	const urls: string[] = [];
	globalThis.fetch = (async (input: string | URL | Request) => {
		urls.push(String(input));
		return new Response(
			JSON.stringify({
				messages: [],
				mentions: [],
				next_before_message_id: null,
				has_more: false
			}),
			{ status: 200, headers: { 'content-type': 'application/json' } }
		);
	}) as typeof fetch;
	try {
		await getRoomMessages('acme group', 'room/id', 41, 17);
		await getRoomThread('acme group', 'room/id', 9, 23, 11);
		assert.deepEqual(urls, [
			'/api/companies/acme%20group/rooms/room%2Fid/messages?limit=17&before_message_id=41',
			'/api/companies/acme%20group/rooms/room%2Fid/threads/9?limit=11&before_message_id=23'
		]);
	} finally {
		globalThis.fetch = original;
	}
});

test('Room identity is explicit before a cursor exists and draft keys isolate principals', async () => {
	const original = globalThis.fetch;
	globalThis.fetch = (async () =>
		new Response(JSON.stringify({ actor_id: 'alice', cursor: null }), {
			status: 200,
			headers: { 'content-type': 'application/json' }
		})) as typeof fetch;
	try {
		assert.deepEqual(await getRoomReadCursor('acme', 'room'), {
			actor_id: 'alice',
			cursor: null
		});
		assert.notEqual(
			roomDraftKey('acme', 'alice', 'room', null),
			roomDraftKey('acme', 'bob', 'room', null)
		);
	} finally {
		globalThis.fetch = original;
	}
});

test('Thread sends preserve the caller command id and exact current write shape', async () => {
	const original = globalThis.fetch;
	let observedUrl = '';
	let observedInit: RequestInit | undefined;
	globalThis.fetch = (async (input: string | URL | Request, init?: RequestInit) => {
		observedUrl = String(input);
		observedInit = init;
		return new Response(
			JSON.stringify({
				message: { id: 12 },
				event_id: 21,
				created: true,
				mentions: [],
				resolved_mention: null
			}),
			{ status: 200, headers: { 'content-type': 'application/json' } }
		);
	}) as typeof fetch;
	try {
		await sendRoomMessage('acme', 'room', 'Evidence attached.', 'command-7', 9, [
			{ actor_id: 'exec' }
		]);
		assert.equal(observedUrl, '/api/companies/acme/rooms/room/messages/9/replies');
		assert.equal(observedInit?.method, 'POST');
		assert.deepEqual(JSON.parse(String(observedInit?.body)), {
			body: 'Evidence attached.',
			command_id: 'command-7',
			mentions: [{ actor_id: 'exec' }]
		});
	} finally {
		globalThis.fetch = original;
	}
});

test('Room lifecycle transports preserve strict cursors and idempotency headers', async () => {
	const original = globalThis.fetch;
	const calls: Array<{ url: string; init?: RequestInit }> = [];
	const controller = new AbortController();
	globalThis.fetch = (async (input: string | URL | Request, init?: RequestInit) => {
		calls.push({ url: String(input), init });
		return new Response(JSON.stringify({ revisions: [], messages: [], rooms: [], created: true }), {
			status: 200,
			headers: { 'content-type': 'application/json' }
		});
	}) as typeof fetch;
	try {
		await searchRooms(
			'acme group',
			'launch review',
			{ beforeCreatedAt: '2026-09-09T00:00:00Z', beforeRoomId: 'room/id' },
			12,
			controller.signal
		);
		await searchRoomMessages('acme group', 'hard evidence', 41, 13, controller.signal);
		await getRoomMessageRevisions('acme group', 'room/id', 9, 3, 7, controller.signal);
		await editRoomMessage('acme group', 'room/id', 9, 'Revised', 2, 'edit-command');
		await deleteRoomMessage('acme group', 'room/id', 9, 'delete-command');

		assert.equal(
			calls[0]?.url,
			'/api/companies/acme%20group/rooms/search?q=launch+review&limit=12&before_created_at=2026-09-09T00%3A00%3A00Z&before_room_id=room%2Fid'
		);
		assert.equal(
			calls[1]?.url,
			'/api/companies/acme%20group/room-messages/search?q=hard+evidence&limit=13&before_message_id=41'
		);
		assert.equal(
			calls[2]?.url,
			'/api/companies/acme%20group/rooms/room%2Fid/messages/9/revisions?limit=7&before_revision_number=3'
		);
		assert.equal(calls[3]?.init?.method, 'PATCH');
		assert.equal(
			(calls[3]?.init?.headers as Record<string, string>)['idempotency-key'],
			'edit-command'
		);
		assert.deepEqual(JSON.parse(String(calls[3]?.init?.body)), {
			body: 'Revised',
			expected_revision_number: 2
		});
		assert.equal(calls[4]?.init?.method, 'DELETE');
		assert.equal(
			(calls[4]?.init?.headers as Record<string, string>)['idempotency-key'],
			'delete-command'
		);
		assert.equal(calls[4]?.init?.body, undefined);
		assert.equal(calls[0]?.init?.signal, controller.signal);
		assert.equal(calls[1]?.init?.signal, controller.signal);
		assert.equal(calls[2]?.init?.signal, controller.signal);
	} finally {
		globalThis.fetch = original;
	}
});

test('a delivered deletion event fails closed across cached body and mention projections', () => {
	const pages = tombstoneRoomMessagePages(
		[
			{
				messages: [
					{
						id: 9,
						room_id: 'room',
						from_actor: 'owner',
						to_actor: 'exec',
						body: 'sensitive body',
						outcome_standard: null,
						parent_message_id: null,
						thread_root_message_id: null,
						client_command_id: 'command',
						created_at: '2026-09-09T00:00:00Z',
						revision_number: 0,
						edited_at: null,
						deleted_at: null,
						legacy_read_at: null
					}
				],
				mentions: [{ id: 'mention', message_id: 9 } as RoomMention],
				next_before_message_id: null,
				has_more: false
			}
		],
		9,
		'2026-09-09T00:01:00Z'
	);
	assert.equal(pages[0]?.messages[0]?.body, '');
	assert.equal(pages[0]?.messages[0]?.deleted_at, '2026-09-09T00:01:00Z');
	assert.deepEqual(pages[0]?.mentions, []);
});

test('a confirmed edit updates live bodies but never resurrects a tombstone', () => {
	const pages = [
		{
			messages: [
				{
					id: 9,
					room_id: 'room',
					from_actor: 'owner',
					to_actor: null,
					body: 'Old',
					outcome_standard: null,
					parent_message_id: null,
					thread_root_message_id: null,
					client_command_id: 'command',
					created_at: '2026-09-09T00:00:00Z',
					revision_number: 0,
					edited_at: null,
					deleted_at: null,
					legacy_read_at: null
				},
				{
					id: 10,
					room_id: 'room',
					from_actor: 'owner',
					to_actor: null,
					body: '',
					outcome_standard: null,
					parent_message_id: null,
					thread_root_message_id: null,
					client_command_id: 'command-2',
					created_at: '2026-09-09T00:00:00Z',
					revision_number: 0,
					edited_at: null,
					deleted_at: '2026-09-09T00:01:00Z',
					legacy_read_at: null
				}
			],
			mentions: [],
			next_before_message_id: null,
			has_more: false
		}
	];
	const edited = editRoomMessagePages(pages, 9, 'New', '2026-09-09T00:02:00Z', 1);
	const tombstone = editRoomMessagePages(edited, 10, 'Secret', '2026-09-09T00:03:00Z', 1);
	assert.equal(edited[0]?.messages[0]?.body, 'New');
	assert.equal(edited[0]?.messages[0]?.edited_at, '2026-09-09T00:02:00Z');
	assert.equal(edited[0]?.messages[0]?.revision_number, 1);
	assert.equal(tombstone[0]?.messages[1]?.body, '');

	const searchPages = [
		{
			messages: [
				{
					id: 9,
					room_id: 'room',
					from_actor: 'owner',
					thread_root_message_id: null,
					snippet: 'Old'
				}
			],
			next_before_message_id: null,
			has_more: false
		}
	];
	const deletedSearch = removeRoomMessageSearchResult(searchPages, 9);
	assert.equal(
		deletedSearch[0]?.messages.some((message) => message.id === 9),
		false
	);
});
