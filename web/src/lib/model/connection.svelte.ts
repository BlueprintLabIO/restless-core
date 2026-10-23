/**
 * Whether the owner API is reachable, as observed by the requests the cockpit
 * already makes. No extra probe: a transport failure marks the connection lost
 * and the next successful response marks it restored.
 *
 * One app-level notice then speaks for the outage, so individual panes can keep
 * their last good data instead of each reporting the same fault.
 */

import { failureKind } from './failure';

export const connection = $state<{
	lost: boolean;
	lostAt: number | null;
	restoredAt: number | null;
}>({ lost: false, lostAt: null, restoredAt: null });

/* One endpoint failing while others answer is that endpoint's failure, shown
 * where it happened. The connection is lost only when nothing has answered for
 * a moment; the query client's retries run first, so a real outage is already
 * several seconds old when its last failure lands here. */
const QUIET_BEFORE_LOST_MS = 4_000;
let lastSuccessAt = 0;

export function observeFailure(error: unknown): void {
	if (!['unreachable', 'offline'].includes(failureKind(error)) || connection.lost) return;
	if (Date.now() - lastSuccessAt < QUIET_BEFORE_LOST_MS) return;
	connection.lost = true;
	connection.lostAt = Date.now();
}

/** Returns true when this success ends an outage. */
export function observeSuccess(): boolean {
	lastSuccessAt = Date.now();
	if (!connection.lost) return false;
	connection.lost = false;
	connection.lostAt = null;
	connection.restoredAt = Date.now();
	return true;
}

/**
 * Whether the company change stream is connected. While it is, reads that
 * OrgIntel notifies about refresh on a hint and poll only as a slow repair
 * path; when it drops, they return to their ordinary interval.
 */
export const changes = $state<{ live: boolean }>({ live: false });

/* Every poll is a request through the edge router, which is billed per request (23 Oct: an idle
 * tab made about 36 a minute and the free daily cap ran out). While change hints arrive, a poll
 * only repairs a missed hint, so it waits at least this long. */
const LIVE_REPAIR_MS = 180_000;
/* While the connection is lost, the app-level notice and its Retry speak for the outage; a poll
 * every few seconds from every pane only adds failed requests. */
const OUTAGE_POLL_MS = 60_000;

/** A refetch interval: `ordinary` while hints are down, at most every `whenLive` (never under
 * three minutes) while they arrive, and slow while the connection is lost. */
export function pollEvery(ordinary: number, whenLive = LIVE_REPAIR_MS): () => number {
	return () =>
		connection.lost
			? Math.max(ordinary, OUTAGE_POLL_MS)
			: changes.live
				? Math.max(whenLive, LIVE_REPAIR_MS)
				: ordinary;
}
