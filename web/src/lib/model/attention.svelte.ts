/**
 * The attention queue, held once for the whole window.
 *
 * The count belongs in the top nav, which is on every surface, and the queue
 * itself belongs on the Inbox. Fetching it in both places would put two answers
 * to "how many things need me" on screen at the same time, and they would
 * disagree for as long as one of them was stale. So there is one read and one
 * store, and the Inbox refreshes it after acting.
 *
 * Deliberately not polled. The queue changes when the company acts or when you
 * do, and a background poll that quietly re-marks the badge is how a surface
 * starts lying about a decision you already made.
 */

import { getAttention, type AttentionOutcome } from '$lib/api/attention';
import { company } from '$lib/api/client';

let queue = $state<AttentionOutcome>({ state: 'failed', message: 'not loaded yet' });
let loading = $state(false);

export function attention(): AttentionOutcome {
	return queue;
}

/**
 * How many things are waiting on you, or `null` when that is not known —
 * signed out, unreachable, or not yet read.
 *
 * `null` and `0` must stay distinguishable all the way to the badge. Zero means
 * the company asked and nothing came back; null means nobody asked. Rendering
 * them the same way is how a signed-out cockpit tells you that you are free.
 */
export function waiting(): number | null {
	return queue.state === 'ok' ? queue.view.items.length : null;
}

export async function refreshAttention(): Promise<void> {
	if (loading) return;
	loading = true;
	try {
		queue = await getAttention(company());
	} finally {
		loading = false;
	}
}

/** After a failed write, so the surface can show why without a stale queue. */
export function reportAttentionFailure(message: string): void {
	queue = { state: 'failed', message };
}
