<script lang="ts">
	import { createInfiniteQuery } from '@tanstack/svelte-query';
	import { fetchDecisionHistory } from '$lib/model/attention';
	import { Item } from '$lib/ui/page';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	let { companyId, version = 0 }: { companyId: string; version?: number } = $props();
	const history = createInfiniteQuery(() => ({
		queryKey: ['inbox-done', companyId, version],
		initialPageParam: undefined as string | undefined,
		queryFn: ({ pageParam }) => fetchDecisionHistory(companyId, pageParam),
		getNextPageParam: (page) => page.nextBefore,
		staleTime: 30_000
	}));
	const items = $derived(history.data?.pages.flatMap((page) => page.decisions) ?? []);
</script>

<section aria-label="Done items" class="inbox-history">
	<h3>Done</h3>
	{#if history.error}<FailureNotice error={history.error} subject="done items" onretry={() => history.refetch()} />{/if}
	{#each items as item (item.id)}
		<Item stacked title={item.title} meta={item.recordedDecision}
			href={`/${encodeURIComponent(companyId)}/company/activity?show=decisions&open=${encodeURIComponent(item.id)}`}>
			{#snippet trailing()}<RelativeTime value={item.observedAt} />{/snippet}
		</Item>
	{:else}
		{#if !history.error}<p>{history.isPending ? 'Reading completed decisions…' : 'No completed decisions.'}</p>{/if}
	{/each}
	{#if history.hasNextPage}<button class="btn small ghost" disabled={history.isFetchingNextPage} onclick={() => history.fetchNextPage()}>Show earlier decisions</button>{/if}
</section>

<style>
	.inbox-history { min-width: 0; padding-block: 4px 12px; }
	h3 { margin: 0; padding: 6px 12px; color: var(--text-tertiary); font: var(--t-label) var(--font-ui); }
	p { padding: 12px; color: var(--text-tertiary); }
</style>
