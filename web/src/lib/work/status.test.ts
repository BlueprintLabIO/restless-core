import assert from 'node:assert/strict';
import test from 'node:test';
import { compareAttempts, newestAttempt, runStateLabel } from './status.ts';

const run = (revision: number, attempt_no: number, state: string, started_at: string) => ({
	revision,
	attempt_no,
	state,
	started_at
});

/* Reno, 25 October: concept-site Work failed three times in the Opus outage, was
 * resumed into a new revision whose first run finished, and still said "Failed". */
test('a resumed Work reads from its newest run, not its highest-numbered one', () => {
	const runs = [
		run(1, 1, 'failed', '2026-10-24T09:00:00Z'),
		run(1, 2, 'failed', '2026-10-24T09:10:00Z'),
		run(1, 3, 'failed', '2026-10-24T09:20:00Z'),
		run(2, 1, 'produced', '2026-10-24T11:00:00Z')
	];
	for (const order of [runs, runs.toReversed()]) {
		assert.equal(runStateLabel(newestAttempt(order)?.state), 'Finished');
		assert.equal(order.toSorted(compareAttempts).at(-1)?.state, 'produced');
	}
});

test('a run resumed within the same revision is newer than the one that failed', () => {
	const runs = [
		run(1, 2, 'produced', '2026-10-24T11:00:00Z'),
		run(1, 1, 'failed', '2026-10-24T09:00:00Z')
	];
	assert.equal(runStateLabel(newestAttempt(runs)?.state), 'Finished');
	assert.equal(newestAttempt([]), null);
});
