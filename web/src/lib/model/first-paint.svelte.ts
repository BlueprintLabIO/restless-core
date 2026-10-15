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
 */

import { useQueryClient } from '@tanstack/svelte-query';
import { onMount } from 'svelte';

const CRITICAL = new Set([
	'companies',
	'appliance',
	'company-principal',
	'attention',
	'cockpit',
	'company-collaboration',
	'company'
]);

export function firstPaint(companyId: () => string) {
	const client = useQueryClient();
	let ready = $state(false);
	let company = '';

	/* Any read still waiting for its first answer holds the reveal, not only the company's own:
	 * the company list decides the setup notices and badges, and arriving late it moved them. */
	const waiting = () =>
		client
			.getQueryCache()
			.findAll()
			.some(
				(query) =>
					CRITICAL.has(String(query.queryKey[0])) &&
					(['companies', 'appliance'].includes(String(query.queryKey[0])) ||
						query.queryKey[1] === companyId()) &&
					query.state.data === undefined &&
					query.state.status === 'pending' &&
					query.state.fetchStatus === 'fetching'
			);

	onMount(() => {
		let frame = 0;
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
			check();
		};
		arm();
		const unsubscribe = client.getQueryCache().subscribe(() => {
			if (companyId() !== company) arm();
			else check();
		});
		return () => {
			unsubscribe();
			cancelAnimationFrame(frame);
		};
	});

	return {
		get ready() {
			return ready;
		}
	};
}
