import assert from 'node:assert/strict';
import test from 'node:test';
// @ts-expect-error Node's strip-only test runner needs the explicit TypeScript suffix.
import * as roomsModel from './rooms.ts';
import type { RoomEvent, RoomTransport } from './rooms';

const {
	getRoomMessages,
	getRoomReadCursor,
	getRoomThread,
	openRoomEventStream,
	roomDraftKey,
	sendRoomMessage
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
