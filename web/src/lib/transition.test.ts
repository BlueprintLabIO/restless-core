import assert from 'node:assert/strict';
import test from 'node:test';
import { pressThroughTransitions, settleQuietly } from './transition.ts';

/* 6 Nov: arriving on Health from the sidebar and clicking "Rebuild company computer" at once did
 * nothing; the second click opened the confirm. The surface crossfade was still running, and
 * while one runs Chrome hit-tests the whole page as the root element. */

type Listener = (event: never) => void;

function fakeWindow(aimed: { click: () => void; closest: () => { focus: () => void } | null }) {
	const listeners: Record<string, Listener[]> = {};
	const root = { id: 'root' };
	const body = { id: 'body' };
	return {
		root,
		listeners,
		window: {
			document: { documentElement: root, body, elementFromPoint: () => aimed },
			addEventListener: (type: string, listener: Listener) =>
				(listeners[type] ??= []).push(listener),
			removeEventListener: (type: string, listener: Listener) =>
				(listeners[type] = (listeners[type] ?? []).filter((entry) => entry !== listener))
		},
		fire(type: string, event: Record<string, unknown>) {
			for (const listener of listeners[type] ?? []) listener(event as never);
		}
	};
}

function fakeTransition() {
	let skipped = 0;
	let finish!: () => void;
	const finished = new Promise<void>((resolve) => (finish = resolve));
	return {
		get skipped() {
			return skipped;
		},
		finish,
		transition: {
			ready: Promise.resolve(),
			finished,
			updateCallbackDone: Promise.resolve(),
			skipTransition: () => void skipped++,
			types: new Set()
		} as unknown as ViewTransition
	};
}

function clickEvent(target: unknown) {
	const event = {
		target,
		clientX: 10,
		clientY: 10,
		stopped: false,
		prevented: false,
		stopImmediatePropagation: () => (event.stopped = true),
		preventDefault: () => (event.prevented = true)
	};
	return event;
}

test('a press during a crossfade skips it and the click reaches the element under the pointer', () => {
	let clicks = 0;
	let focused = 0;
	const page = fakeWindow({ click: () => clicks++, closest: () => ({ focus: () => focused++ }) });
	const stop = pressThroughTransitions(page.window as unknown as Window);
	const running = fakeTransition();
	settleQuietly(running.transition);

	page.fire('pointerdown', { button: 0 });
	assert.equal(running.skipped, 1, 'the press ends the crossfade');
	const event = clickEvent(page.root);
	page.fire('click', event);
	assert.equal(clicks, 1, 'the aimed-at control receives the click');
	assert.equal(focused, 1);
	assert.ok(event.stopped && event.prevented, 'the root never sees the stray click');
	stop();
	assert.equal(page.listeners.pointerdown.length + page.listeners.click.length, 0);
});

test('with no crossfade running, presses and clicks are left alone', async () => {
	let clicks = 0;
	const page = fakeWindow({ click: () => clicks++, closest: () => null });
	const stop = pressThroughTransitions(page.window as unknown as Window);
	const ended = fakeTransition();
	settleQuietly(ended.transition);
	ended.finish();
	await ended.transition.finished;

	page.fire('pointerdown', { button: 0 });
	assert.equal(ended.skipped, 0);
	const event = clickEvent(page.root);
	page.fire('click', event);
	assert.equal(clicks, 0);
	assert.equal(event.stopped, false);
	stop();
});

test('a click that already reached a real element is not sent twice', () => {
	let clicks = 0;
	const page = fakeWindow({ click: () => clicks++, closest: () => null });
	const stop = pressThroughTransitions(page.window as unknown as Window);
	settleQuietly(fakeTransition().transition);

	page.fire('pointerdown', { button: 0 });
	const event = clickEvent({ id: 'button' });
	page.fire('click', event);
	assert.equal(clicks, 0);
	assert.equal(event.stopped, false);
	stop();
});
