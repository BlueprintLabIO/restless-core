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
	if (failureKind(error) !== 'unreachable' || connection.lost) return;
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

/** A refetch interval that relaxes to `whenLive` while change hints arrive. */
export function pollEvery(ordinary: number, whenLive: number): () => number {
	return () => (changes.live ? whenLive : ordinary);
}
