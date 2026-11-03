/**
 * Whether anyone is using this page right now, as its own input says: a key, a click, a scroll or
 * a touch. A page left open and untouched (a background pane, a second monitor, a tab behind
 * others that still reports itself visible) reads far less often; the moment someone touches it
 * again, whatever was held back reads at once (`onReturn`).
 */

import { IDLE_AFTER_MS } from './hint-pace.ts';

let lastInput = Date.now();
const returns = new Set<() => void>();
let listening = false;

function touched() {
	const wasIdle = Date.now() - lastInput > IDLE_AFTER_MS;
	lastInput = Date.now();
	if (wasIdle) for (const run of [...returns]) run();
}

function listen() {
	if (listening || typeof window === 'undefined') return;
	listening = true;
	for (const type of ['pointerdown', 'pointermove', 'keydown', 'wheel', 'touchstart'])
		window.addEventListener(type, touched, { capture: true, passive: true });
	/* Coming back to a hidden tab counts as using it. Its catch-up is the query client's own
	 * refetch on focus, so this only restarts the clock. */
	document.addEventListener('visibilitychange', () => {
		if (document.visibilityState === 'visible') lastInput = Date.now();
	});
}

/** Nobody has touched the page for a minute, or for `forMs`. */
export function untouched(now = Date.now(), forMs = IDLE_AFTER_MS): boolean {
	listen();
	return now - lastInput > forMs;
}

/** Run when someone touches the page after it sat untouched. Returns the unsubscribe. */
export function onReturn(run: () => void): () => void {
	listen();
	returns.add(run);
	return () => {
		returns.delete(run);
	};
}

/** How often a check repeats while nobody touches the page. */
const UNTOUCHED_CHECK_MS = 15 * 60_000;

/**
 * Repeat a check that no change hint covers (a file on the company computer, an app's sign-in)
 * every `everyMs` while the page is used, every fifteen minutes while it sits untouched, and never
 * while it is hidden. Shown or touched again after a check is due, it runs at once. The caller
 * runs the first check. Returns the stop.
 */
export function whileSeen(check: () => void, everyMs: number): () => void {
	listen();
	let last = Date.now();
	const due = (gap: number) => !document.hidden && Date.now() - last >= gap;
	const run = () => {
		last = Date.now();
		check();
	};
	const timer = setInterval(
		() => {
			if (due(untouched() ? UNTOUCHED_CHECK_MS : everyMs)) run();
		},
		Math.min(everyMs, 15_000)
	);
	const catchUp = () => {
		if (due(everyMs)) run();
	};
	document.addEventListener('visibilitychange', catchUp);
	const stopReturn = onReturn(catchUp);
	return () => {
		clearInterval(timer);
		document.removeEventListener('visibilitychange', catchUp);
		stopReturn();
	};
}
