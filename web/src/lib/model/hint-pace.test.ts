import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	HINT_ACTIVE_CEILING_MS,
	HINT_IDLE_CEILING_MS,
	HINT_QUIET_MS,
	HintPace
} from './hint-pace.ts';

/* Reads one query makes while a hint arrives every `every` ms for `span` ms. */
function reads(every: number, span: number, idle: boolean) {
	const pace = new HintPace();
	let held: number | null = null;
	let count = 0;
	for (let now = 0; now <= span; now += 250) {
		if (now % every === 0) held = held ?? now;
		if (held !== null && pace.wait('q', now, idle) === 0) {
			pace.read('q', now);
			held = null;
			count++;
		}
	}
	return count;
}

test('the first hint after a quiet spell reads at once', () => {
	const pace = new HintPace();
	assert.equal(pace.wait('q', 0, true), 0);
	pace.read('q', 0);
	assert.ok(pace.wait('q', 1_000, true) > 0);
	assert.equal(pace.wait('q', HINT_QUIET_MS, true), 0);
});

/* A busy company (a hint every 4 s, 3 Nov 2026) used to read each query on screen 15 times a
 * minute. An untouched page now reads it about once a minute; one in use, every ten seconds. */
test('an untouched page reads a busy query about once a minute', () => {
	const tenMinutes = 10 * 60_000;
	const idle = reads(4_000, tenMinutes, true);
	assert.ok(idle <= tenMinutes / HINT_IDLE_CEILING_MS + 6, `${idle} reads`);
	const active = reads(4_000, tenMinutes, false);
	assert.ok(active <= tenMinutes / HINT_ACTIVE_CEILING_MS + 3, `${active} reads`);
	assert.ok(active >= tenMinutes / HINT_ACTIVE_CEILING_MS - 3, `${active} reads`);
});

test('a hint is never held longer than the ceiling', () => {
	const pace = new HintPace();
	for (let now = 0; now < 30 * 60_000; now += 1_000) {
		const wait = pace.wait('q', now, true);
		assert.ok(wait <= HINT_IDLE_CEILING_MS);
		if (wait === 0) pace.read('q', now);
	}
});
