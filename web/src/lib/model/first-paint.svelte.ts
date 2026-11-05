/**
 * Entering a company reveals the cockpit once, in its settled layout (Sprint 82).
 *
 * Each region reads its own query, so without a barrier the page arrives in four or five
 * visible steps: an empty shell, then the list, then the counts, then the avatar. Chrome's
 * layout-shift score reads 0 for that, because content swaps in place rather than moving, but
 * a person sees the page reshuffle. Here every region mounts and fetches at once while hidden,
 * and the page appears when its critical reads have answered or failed. Each critical
 * request has a deadline, so a failed source reveals its recovery state. After that the barrier never closes
 * again: later changes are the ordinary, animated kind.
 *
 * The barrier waits a short time at most (REVEAL_WITHIN_MS). Opening another company's Health
 * held a blank frame of empty panes for 8–10 s on Cloud (6 Nov) while its live checks ran: a read
 * that slow now arrives into the shell and the page's own skeletons, which hold its place.
 */

import { useQueryClient } from '@tanstack/svelte-query';
import { onMount } from 'svelte';
import { holdsReveal, surfaceOf } from './first-paint-hold.ts';

/** The longest the reveal waits on reads, from when the shell mounts. A load that settles sooner
 * appears once, settled; a slower one shows the shell and skeletons now, never a blank frame. */
export const REVEAL_WITHIN_MS = 300;

export function firstPaint(companyId: () => string, pathname: () => string = () => '') {
	const client = useQueryClient();
	let ready = $state(false);
	let company = '';

	/* Any read still waiting for its first answer holds the reveal, not only the company's own:
	 * the company list decides the setup notices and badges, and arriving late it moved them. */
	const waiting = () =>
		client
			.getQueryCache()
			.findAll()
			.some((query) => holdsReveal(query, companyId(), surfaceOf(pathname(), companyId())));

	onMount(() => {
		let frame = 0;
		let deadline: ReturnType<typeof setTimeout> | undefined;
		const check = () => {
			if (ready) return;
			cancelAnimationFrame(frame);
			// Two frames with nothing pending: anything that mounts on the first answer has had
			// its chance to start fetching too.
			frame = requestAnimationFrame(() => {
				frame = requestAnimationFrame(() => {
					if (!waiting()) ready = true;
				});
			});
		};
		const arm = () => {
			company = companyId();
			ready = false;
			clearTimeout(deadline);
			deadline = setTimeout(() => (ready = true), REVEAL_WITHIN_MS);
			check();
		};
		arm();
		let queued = false;
		let disposed = false;
		const unsubscribe = client.getQueryCache().subscribe(() => {
			// Changing a reactive query key can notify inside a derived calculation.
			// Reveal state belongs to the next update, outside that calculation.
			if (queued) return;
			queued = true;
			queueMicrotask(() => {
				queued = false;
				if (disposed) return;
				if (companyId() !== company) arm();
				else check();
			});
		});
		return () => {
			disposed = true;
			unsubscribe();
			cancelAnimationFrame(frame);
			clearTimeout(deadline);
		};
	});

	return {
		get ready() {
			return ready && company === companyId();
		}
	};
}
