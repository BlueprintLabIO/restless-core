import { test } from 'node:test';
import assert from 'node:assert/strict';
import { EventFrames } from './live-events.ts';
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
