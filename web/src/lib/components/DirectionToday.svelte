<script lang="ts">
	/* Today, at the top of the Inbox (S80): what Exec applied for others and what waits on this
	 * person, in one line, instead of an interruption for each. Nothing on a quiet day. */
	import { createQuery } from '@tanstack/svelte-query';
	import { directionDigest } from '$lib/model/direction';

	let { companyId }: { companyId: string } = $props();

	const digest = createQuery(() => ({
		queryKey: ['direction-digest', companyId],
		queryFn: () => directionDigest(companyId),
		enabled: Boolean(companyId),
		staleTime: 60_000,
		refetchInterval: 5 * 60_000,
		refetchIntervalInBackground: false,
		retry: false
	}));
	const summary = $derived(digest.data?.summary.trim() ?? '');
</script>

{#if summary}
	<p class="direction-today" role="status">
		<span class="label">Today</span>
		<span class="summary">{summary}</span>
	</p>
{/if}

<style>
	.direction-today {
		display: flex;
		align-items: baseline;
		gap: 8px;
		min-width: 0;
		margin: 0;
		padding: 8px 16px;
		border-bottom: 1px solid var(--border);
		font-size: var(--t-label);
	}
	.label {
		flex: none;
		color: var(--text-tertiary);
		font-weight: 600;
	}
	.summary {
		min-width: 0;
		color: var(--text-secondary);
	}
</style>
