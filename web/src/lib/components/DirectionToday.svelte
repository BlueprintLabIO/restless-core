<script lang="ts">
	/* Today, at the top of the Inbox (S80): what Exec applied for others and what waits on this
	 * person, in one line, instead of an interruption for each. Each clause opens its list: the
	 * applied changes unfold here with their Undo, and the waiting requests filter the Inbox.
	 * Nothing on a quiet day. */
	import { createQuery } from '@tanstack/svelte-query';
	import { directionDigest } from '$lib/model/direction';
	import AppliedChanges from './AppliedChanges.svelte';

	let {
		companyId,
		requestsOnly = false,
		weekly = false,
		onrequests
	}: {
		companyId: string;
		/** Offer last week's review on Mondays: the owner's weekly look back. */
		weekly?: boolean;
		/** The Inbox is showing only the waiting requests. */
		requestsOnly?: boolean;
		/** Filter the Inbox to the waiting requests, or back to everything. */
		onrequests?: (only: boolean) => void;
	} = $props();

	const digest = createQuery(() => ({
		queryKey: ['direction-digest', companyId],
		queryFn: () => directionDigest(companyId),
		enabled: Boolean(companyId),
		staleTime: 60_000,
		refetchInterval: 5 * 60_000,
		refetchIntervalInBackground: false,
		retry: false
	}));
	const applied = $derived(digest.data?.applied ?? []);
	const waiting = $derived(digest.data?.requests_waiting.length ?? 0);
	const plural = (count: number, one: string, many: string) =>
		`${count} ${count === 1 ? one : many}`;
	let showApplied = $state(false);
	const monday = $derived(weekly && new Date().getDay() === 1);
</script>

{#if applied.length || waiting || monday}
	<div class="direction-today">
		<p role="status">
			<span class="label">Today</span>
			{#if applied.length}<button
					type="button"
					class="clause"
					aria-expanded={showApplied}
					title="See each change, open what it touched, or undo it"
					onclick={() => (showApplied = !showApplied)}
					>Exec applied {plural(applied.length, 'change', 'changes')} for others</button
				>{/if}
			{#if applied.length && waiting}<span aria-hidden="true">·</span>{/if}
			{#if waiting}<button
					type="button"
					class="clause"
					aria-pressed={requestsOnly}
					title={requestsOnly ? 'Show everything in the Inbox' : 'Show only these requests'}
					onclick={() => onrequests?.(!requestsOnly)}
					>{plural(waiting, 'request', 'requests')} waiting</button
				>{/if}
			{#if monday}{#if applied.length || waiting}<span aria-hidden="true">·</span>{/if}<a
					class="clause"
					href={`/${encodeURIComponent(companyId)}/company/week`}
					title="What the company finished, decided, sent and spent last week; Exec can write it up"
					>Last week’s review</a
				>{/if}
		</p>
		{#if showApplied && applied.length}<AppliedChanges
				{companyId}
				changes={applied}
				onchanged={() => void digest.refetch()}
			/>{/if}
	</div>
{/if}

<style>
	.direction-today {
		border-bottom: 1px solid var(--border);
	}
	p {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 4px 8px;
		min-width: 0;
		margin: 0;
		padding: 8px 16px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.label {
		flex: none;
		font-weight: 600;
	}
	.clause {
		padding: 0;
		border: 0;
		background: none;
		color: var(--text-secondary);
		font: inherit;
		text-decoration: underline;
		text-decoration-color: var(--border-strong);
		text-underline-offset: 3px;
		cursor: pointer;
	}
	.clause:hover,
	.clause[aria-expanded='true'],
	.clause[aria-pressed='true'] {
		color: var(--ink);
		text-decoration-color: currentColor;
	}
	.direction-today :global(.applied-changes) {
		margin: 0 8px 8px;
	}
</style>
