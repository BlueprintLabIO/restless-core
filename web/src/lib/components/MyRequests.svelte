<script lang="ts">
	/* What this person asked for and is waiting on (S80): who was asked and where it stands. Above
	 * the composer it is a strip of the latest three, gone a day after each is answered, with a
	 * link to the whole list; in For you it is the whole list, where each row unfolds to the exact
	 * change and a pending one can be withdrawn. */
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { decideDirectionRequest, directionRequests } from '$lib/model/direction';
	import type { DirectionRequest } from '$lib/model/generated/direction';

	let { companyId, all = false }: { companyId: string; all?: boolean } = $props();

	const client = useQueryClient();
	const DAY = 24 * 60 * 60_000;
	const key = $derived(['direction-requests', companyId, 'mine']);
	const mine = createQuery(() => ({
		queryKey: key,
		queryFn: () => directionRequests(companyId, 'mine'),
		enabled: Boolean(companyId),
		staleTime: 15_000,
		refetchInterval: 30_000,
		refetchIntervalInBackground: false,
		retry: false
	}));
	const recent = $derived(
		(mine.data?.items ?? []).filter(
			(request) =>
				request.state === 'open' ||
				Date.now() - Date.parse(request.decision?.decided_at ?? request.filed_at) < DAY
		)
	);
	const shown = $derived(all ? (mine.data?.items ?? []) : recent.slice(0, 3));
	const more = $derived((mine.data?.items.length ?? 0) > shown.length);
	const withdrawn = (request: DirectionRequest) =>
		request.state === 'declined' && request.decision?.by.actor_id === request.requester.actor_id;
	function status(request: DirectionRequest): string {
		switch (request.state) {
			case 'open':
				return 'Pending';
			case 'approved':
				return 'Approved';
			case 'done':
				return 'Done';
			case 'declined':
				if (withdrawn(request)) return 'Withdrawn';
				return request.decision?.reason ? `Declined: ${request.decision.reason}` : 'Declined';
			case 'failed':
				return `Approved, not done. Back with ${request.routed_to.name}`;
		}
	}
	let withdrawing = $state('');
	async function withdraw(request: DirectionRequest) {
		if (withdrawing) return;
		withdrawing = request.id;
		try {
			await decideDirectionRequest(companyId, request.id, {
				decision: 'decline',
				reason: 'Withdrawn by the person who asked'
			});
			toast(`Withdrawn. ${request.routed_to.name} no longer needs to answer it.`);
		} catch (cause) {
			toast(failureSentence(cause, 'That request was not withdrawn.'), { tone: 'failure' });
		} finally {
			withdrawing = '';
			await client.invalidateQueries({ queryKey: key });
		}
	}
</script>

{#if all}
	{#if shown.length}
		<section class="my-requests all" aria-label="Your requests">
			<h3>Changes you asked for</h3>
			<p class="explain">
				Changes you asked Exec to make that someone else in the company decides, and what they
				said.
			</p>
			{#each shown as request (request.id)}
				<details data-state={withdrawn(request) ? 'withdrawn' : request.state}>
					<summary>
						<span class="summary">{request.summary}</span>
						<span class="state">{status(request)}</span>
						<RelativeTime value={request.filed_at} />
					</summary>
					<dl>
						<dt>Asked</dt>
						<dd>
							{request.routed_to.name}{#if request.team}, for {request.team.name}{/if}
						</dd>
						<dt>The change</dt>
						<dd>{request.change}</dd>
						{#if request.reason}<dt>Why</dt>
							<dd>{request.reason}</dd>{/if}
						{#if request.failure}<dt>What went wrong</dt>
							<dd>{request.failure}</dd>{/if}
					</dl>
					{#if request.state === 'open'}<button
							type="button"
							class="btn small ghost"
							disabled={!!withdrawing}
							title={`${request.routed_to.name} stops being asked. You can ask again any time.`}
							onclick={() => void withdraw(request)}
							>{withdrawing === request.id ? 'Withdrawing…' : 'Withdraw'}</button
						>{/if}
				</details>
			{/each}
		</section>
	{/if}
{:else if shown.length}
	<ul class="my-requests" aria-label="Your requests">
		{#each shown as request (request.id)}
			<li data-state={withdrawn(request) ? 'withdrawn' : request.state} title={request.change}>
				<span class="asked">Asked {request.routed_to.name}:</span>
				<span class="summary">{request.summary}</span>
				<span class="state">{status(request)}</span>
			</li>
		{/each}
		{#if more || recent.length > shown.length}<li class="all-link">
				<a href={`/${encodeURIComponent(companyId)}/company/activity`}>All your requests</a>
			</li>{/if}
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
	li.all-link {
		background: none;
	}
	li.all-link a {
		color: var(--text-tertiary);
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
	[data-state='done'] .state,
	[data-state='approved'] .state {
		color: var(--state-success);
	}
	[data-state='declined'] .state,
	[data-state='failed'] .state {
		color: var(--state-danger);
	}
	.all {
		margin: 4px 8px 10px;
	}
	h3 {
		margin: 0;
		padding: 4px 8px;
		color: var(--text-secondary);
		font-size: var(--t-label);
		font-weight: 600;
	}
	.explain {
		margin: 0 0 6px;
		padding: 0 8px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	details {
		border-radius: var(--radius-control);
	}
	details[open] {
		background: var(--surface-alt);
	}
	summary {
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
		padding: 6px 8px;
		border-radius: var(--radius-control);
		color: var(--text-tertiary);
		cursor: pointer;
		list-style: none;
	}
	summary::-webkit-details-marker {
		display: none;
	}
	summary:hover {
		background: var(--wash-hover);
	}
	summary :global(time) {
		flex: none;
		color: inherit;
	}
	dl {
		display: grid;
		gap: 2px;
		margin: 0;
		padding: 2px 8px 6px;
	}
	dt {
		color: var(--text-tertiary);
	}
	dd {
		margin: 0 0 6px;
		color: var(--ink);
		font-size: var(--t-body);
		white-space: pre-wrap;
	}
	details .btn {
		margin: 0 8px 8px;
	}
</style>
