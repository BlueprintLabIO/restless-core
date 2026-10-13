<script lang="ts">
	/* What this person asked for and is waiting on (S80), above the composer: who was asked and
	 * where it stands, instead of paragraphs explaining whose call it is. A request leaves the strip
	 * a day after it was answered. */
	import { createQuery } from '@tanstack/svelte-query';
	import { directionRequests } from '$lib/model/direction';
	import type { DirectionRequest } from '$lib/model/generated/direction';

	let { companyId }: { companyId: string } = $props();

	const DAY = 24 * 60 * 60_000;
	const mine = createQuery(() => ({
		queryKey: ['direction-requests', companyId, 'mine'],
		queryFn: () => directionRequests(companyId, 'mine'),
		enabled: Boolean(companyId),
		staleTime: 15_000,
		refetchInterval: 30_000,
		refetchIntervalInBackground: false,
		retry: false
	}));
	const shown = $derived(
		(mine.data?.items ?? [])
			.filter(
				(request) =>
					request.state === 'open' ||
					Date.now() - Date.parse(request.decision?.decided_at ?? request.filed_at) < DAY
			)
			.slice(0, 3)
	);
	function status(request: DirectionRequest): string {
		switch (request.state) {
			case 'open':
				return 'Pending';
			case 'approved':
				return 'Approved';
			case 'done':
				return 'Done';
			case 'declined':
				return request.decision?.reason ? `Declined: ${request.decision.reason}` : 'Declined';
			case 'failed':
				return 'Approved, not done';
		}
	}
</script>

{#if shown.length}
	<ul class="my-requests" aria-label="Your requests">
		{#each shown as request (request.id)}
			<li data-state={request.state} title={request.change}>
				<span class="asked">Asked {request.routed_to.name}:</span>
				<span class="summary">{request.summary}</span>
				<span class="state">{status(request)}</span>
			</li>
		{/each}
	</ul>
{/if}

<style>
	.my-requests {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: 2px;
		margin: 0 10px 6px;
		padding: 0;
		list-style: none;
		font-size: var(--t-label);
	}
	li {
		display: flex;
		align-items: baseline;
		gap: 6px;
		min-width: 0;
		padding: 4px 8px;
		border-radius: var(--radius-control);
		background: var(--surface-alt);
	}
	.asked {
		flex: none;
		color: var(--text-tertiary);
	}
	.summary {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		color: var(--ink);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.state {
		flex: none;
		max-width: 45%;
		overflow: hidden;
		color: var(--text-tertiary);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	li[data-state='done'] .state,
	li[data-state='approved'] .state {
		color: var(--state-success);
	}
	li[data-state='declined'] .state,
	li[data-state='failed'] .state {
		color: var(--state-danger);
	}
</style>
