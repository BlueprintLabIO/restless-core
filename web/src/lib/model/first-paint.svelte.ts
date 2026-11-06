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
 * A read that can take seconds holds the reveal only for its HOLDS_AT_MOST
 * (first-paint-hold.ts). Opening another company's Health held a blank frame of empty panes for
 * 8–10 s on Cloud (6 Nov) while the company read ran its live checks; now the shell and the
 * page's skeleton appear and that read fills them in.
 */

import { useQueryClient } from '@tanstack/svelte-query';
import { onMount } from 'svelte';
import { HOLDS_AT_MOST, holdsReveal, surfaceOf } from './first-paint-hold.ts';

export function firstPaint(companyId: () => string, pathname: () => string = () => '') {
	const client = useQueryClient();
	let ready = $state(false);
	let company = '';
	let armedAt = 0;

	/* Any read still waiting for its first answer holds the reveal, not only the company's own:
	 * the company list decides the setup notices and badges, and arriving late it moved them. */
	const waiting = () => {
		const waited = performance.now() - armedAt;
		return client
			.getQueryCache()
			.findAll()
			.some((query) => holdsReveal(query, companyId(), surfaceOf(pathname(), companyId()), waited));
	};

	onMount(() => {
		let frame = 0;
		let limits: ReturnType<typeof setTimeout>[] = [];
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
			armedAt = performance.now();
			for (const limit of limits) clearTimeout(limit);
			/* A slow read stops holding at its limit even when nothing else changes then. */
			limits = [...new Set(Object.values(HOLDS_AT_MOST))].map((ms) => setTimeout(check, ms));
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
			for (const limit of limits) clearTimeout(limit);
		};
	});

	return {
		get ready() {
			return ready && company === companyId();
		}
	};
}
