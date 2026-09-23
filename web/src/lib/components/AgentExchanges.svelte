<script lang="ts">
	import { createInfiniteQuery } from '@tanstack/svelte-query';
	import Markdown from '$lib/primitives/Markdown.svelte';
	let { companyId, actorId }: { companyId: string; actorId: string } = $props();
	type Exchange = {
		id: number;
		from_actor: string;
		to_actor: string;
		body: string;
		created_at: string;
	};
	type ExchangePage = {
		messages: Exchange[];
		names: Record<string, string>;
		next_before: number | null;
	};
	let open = $state(false);
	let expanded = $state<number[]>([]);
	const query = createInfiniteQuery(() => ({
		queryKey: ['agent-exchanges', companyId, actorId],
		initialPageParam: null as number | null,
		queryFn: async ({ pageParam, signal }): Promise<ExchangePage> => {
			const suffix = pageParam === null ? '' : `?before=${pageParam}`;
			const response = await fetch(
				`/api/companies/${encodeURIComponent(companyId)}/actors/${encodeURIComponent(actorId)}/exchanges${suffix}`,
				{ signal }
			);
			if (!response.ok) throw new Error('Could not load team exchanges.');
			return response.json();
		},
		getNextPageParam: (lastPage) => lastPage.next_before ?? undefined,
		refetchInterval: open ? 10000 : 30000,
		staleTime: 5000,
		retry: 1
	}));
	const messages = $derived(query.data?.pages.flatMap((p) => p.messages) ?? []);
	const names = $derived(
		Object.assign({}, ...(query.data?.pages.map((p) => p.names) ?? [])) as Record<string, string>
	);
	const latest = $derived(messages[0]);
	function name(actor: string) {
		return actor === 'exec' ? 'Exec' : (names[actor] ?? actor);
	}
	function timestamp(value: string) {
		return new Date(value).toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: 'numeric',
			minute: '2-digit'
		});
	}
</script>

<details class="agent-exchanges" bind:open>
	<summary>
		<strong>Team exchanges</strong>
		{#if latest}<span class="latest">{name(latest.from_actor)} → {name(latest.to_actor)}</span>{/if}
	</summary>
	{#if query.error}
		<p role="alert">
			Could not load team exchanges. <button type="button" onclick={() => query.refetch()}
				>Retry</button
			>
		</p>
	{:else if query.isPending}
		<p role="status">Loading exchanges…</p>
	{:else if !messages.length}
		<p>No internal messages recorded for this person yet.</p>
	{/if}
	{#each messages as message (message.id)}
		<details
			class="exchange"
			ontoggle={(event) => {
				expanded = event.currentTarget.open
					? [...new Set([...expanded, message.id])]
					: expanded.filter((id) => id !== message.id);
			}}
		>
			<summary>
				<span class="exchange-description"
					><strong>{name(message.from_actor)} → {name(message.to_actor)}</strong><span
						class="preview">{message.body}</span
					></span
				>
				<time datetime={message.created_at}>{timestamp(message.created_at)}</time>
			</summary>
			{#if expanded.includes(message.id)}<div class="exchange-body">
					<Markdown text={message.body} />
				</div>{/if}
		</details>
	{/each}
	{#if query.hasNextPage}<button
			class="older"
			type="button"
			disabled={query.isFetchingNextPage}
			onclick={() => query.fetchNextPage()}
			>{query.isFetchingNextPage ? 'Loading…' : 'Older exchanges'}</button
		>{/if}
</details>

<style>
	.agent-exchanges {
		flex: 0 0 auto;
		min-width: 0;
		max-height: 35vh;
		overflow-y: auto;
		margin: var(--space-2) var(--space-4);
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-pane);
		font-size: var(--t-label);
	}
	summary {
		cursor: pointer;
		padding: var(--space-3);
	}
	.latest {
		margin-left: var(--space-3);
		color: var(--text-secondary);
		overflow-wrap: anywhere;
	}
	.exchange {
		border-top: 1px solid var(--border);
	}
	.exchange > summary {
		display: flex;
		align-items: baseline;
		gap: var(--space-3);
	}
	.exchange-description {
		flex: 1;
		min-width: 0;
	}
	.exchange-description strong {
		overflow-wrap: anywhere;
	}
	.preview {
		display: block;
		white-space: nowrap;
		text-overflow: ellipsis;
		overflow: hidden;
		color: var(--text-secondary);
		margin-top: var(--space-1);
	}
	time {
		color: var(--text-tertiary);
		flex-shrink: 0;
	}
	.exchange-body {
		padding: 0 var(--space-3) var(--space-3);
		overflow-wrap: anywhere;
		min-width: 0;
	}
	.exchange-body :global(pre),
	.exchange-body :global(table) {
		max-width: 100%;
		overflow-x: auto;
	}
	p,
	.older {
		margin: var(--space-3);
	}
	@media (max-width: 640px) {
		.exchange > summary {
			flex-wrap: wrap;
		}
		time {
			width: 100%;
		}
		.latest {
			display: block;
			margin: var(--space-1) 0 0;
		}
	}
</style>
