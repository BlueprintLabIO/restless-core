import { test } from 'node:test';
import assert from 'node:assert/strict';
import { EventFrames, isLoopbackHost } from './live-events.ts';

test('every loopback name takes the socket, so streams never hold the six HTTP/1 connections', () => {
	for (const host of [
		'localhost',
		'127.0.0.1',
		'[::1]',
		'plane.localhost',
		'acme.plane.localhost'
	]) {
		assert.equal(isLoopbackHost(host), true, host);
	}
	for (const host of ['restless.run', 'localhost.example.com', 'notlocalhost', '10.0.0.2']) {
		assert.equal(isLoopbackHost(host), false, host);
	}
});
test('socket framing retains fragmented events, comments and multiline data', () => {
	const frames = new EventFrames();
	assert.deepEqual(frames.feed(':keep-alive\r\n\r\nevent: acti'), []);
	assert.deepEqual(frames.feed('vity\r\ndata: {"phase":"working"}\r\n\r'), []);
	assert.deepEqual(frames.feed('\nevent: change\ndata: work_changed\ndata: message\n\n'), [
		{ type: 'activity', data: '{"phase":"working"}' },
		{ type: 'change', data: 'work_changed\nmessage' }
	]);
	assert.deepEqual(frames.feed('data: done\n\n'), [{ type: 'message', data: 'done' }]);
});

test('socket reconnects once and close cancels future retries', async () => {
	const originalSocket = globalThis.WebSocket;
	const originalLocation = Object.getOwnPropertyDescriptor(globalThis, 'location');
	class FakeSocket {
		static instances: FakeSocket[] = [];
		binaryType = '';
		onopen: ((e: Event) => void) | null = null;
		onclose: ((e: Event) => void) | null = null;
		onmessage: ((e: MessageEvent) => void) | null = null;
		onerror: ((e: Event) => void) | null = null;
		constructor(_url: string) {
			FakeSocket.instances.push(this);
		}
		close() {
			this.onclose?.(new Event('close'));
		}
	}
	try {
		Object.defineProperty(globalThis, 'location', {
			value: { protocol: 'http:', hostname: '127.0.0.1', origin: 'http://127.0.0.1:1234' },
			configurable: true
		});
		globalThis.WebSocket = FakeSocket as unknown as typeof WebSocket;
		const { openLiveEvents } = await import('./live-events.ts');
		const source = openLiveEvents('/api/test');
		let failures = 0;
		source.onerror = () => failures++;
		const received: string[] = [];
		source.addEventListener('activity', (event) => received.push((event as MessageEvent).data));
		FakeSocket.instances[0].onmessage?.({
			data: new TextEncoder().encode('event: activity\ndata: café\n\n').buffer
		} as MessageEvent);
		assert.deepEqual(received, ['café']);
		FakeSocket.instances[0].close();
		assert.equal(failures, 1);
		await new Promise((resolve) => setTimeout(resolve, 1100));
		assert.equal(FakeSocket.instances.length, 2);
		FakeSocket.instances[1].close();
		source.close();
		await new Promise((resolve) => setTimeout(resolve, 1100));
		assert.equal(FakeSocket.instances.length, 2);
	} finally {
		globalThis.WebSocket = originalSocket;
		if (originalLocation) Object.defineProperty(globalThis, 'location', originalLocation);
		else Reflect.deleteProperty(globalThis, 'location');
	}
});

/* Hosted, a refused change stream (429 when the company's streams are full) closed for good and
 * the page polled at its ordinary pace for as long as it stayed open (3 Nov 2026). */
test('a refused change stream opens again, later each time, until it is closed', async (t) => {
	const { RetryingEventSource } = await import('./live-events.ts');
	t.mock.timers.enable({ apis: ['setTimeout'] });
	class FakeSource {
		static instances: FakeSource[] = [];
		readyState = 0;
		onopen: ((e: Event) => void) | null = null;
		onerror: ((e: Event) => void) | null = null;
		listeners = new Map<string, EventListener>();
		constructor() {
			FakeSource.instances.push(this);
		}
		addEventListener(type: string, listener: EventListener) {
			this.listeners.set(type, listener);
		}
		close() {
			this.readyState = 2;
		}
		refuse() {
			this.readyState = 2;
			this.onerror?.(new Event('error'));
		}
	}
	const source = new RetryingEventSource(
		'/api/companies/acme/changes',
		() => new FakeSource() as unknown as EventSource
	);
	const hints: string[] = [];
	source.addEventListener('change', (event) => hints.push((event as MessageEvent).data));
	FakeSource.instances[0].refuse();
	t.mock.timers.tick(4_999);
	assert.equal(FakeSource.instances.length, 1);
	t.mock.timers.tick(1);
	assert.equal(FakeSource.instances.length, 2);
	FakeSource.instances[1].listeners.get('change')?.(
		new MessageEvent('change', { data: 'work_changed' })
	);
	assert.deepEqual(hints, ['work_changed']);
	FakeSource.instances[1].refuse();
	t.mock.timers.tick(9_999);
	assert.equal(FakeSource.instances.length, 2);
	t.mock.timers.tick(1);
	assert.equal(FakeSource.instances.length, 3);
	/* A dropped connection the browser retries itself (still connecting) is left to it. */
	FakeSource.instances[2].onerror?.(new Event('error'));
	t.mock.timers.tick(60_000);
	assert.equal(FakeSource.instances.length, 3);
	source.close();
	FakeSource.instances[2].refuse();
	t.mock.timers.tick(60_000);
	assert.equal(FakeSource.instances.length, 3);
});
