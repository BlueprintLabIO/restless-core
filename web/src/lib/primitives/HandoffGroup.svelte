<script lang="ts">
	/* A run of handoffs between agents, folded to one quiet line: who coordinated and how much.
	 * The owner sees outcomes first; each message is a click away, never a wall of rows. */
	import HandoffReceipt from './HandoffReceipt.svelte';
	import type { AgentExchange } from '$lib/model/exchanges.svelte';

	let {
		exchanges,
		name,
		host = 'exec'
	}: {
		exchanges: AgentExchange[];
		name: (actorId: string) => string;
		/** Whose conversation this is: the line names the others. */
		host?: string;
	} = $props();

	function line(items: AgentExchange[]): string {
		const seen = new Set<string>();
		for (const exchange of items)
			for (const actor of [exchange.from_actor, exchange.to_actor])
				if (actor && actor !== host) seen.add(name(actor));
		const others = [...seen];
		return `${name(host)} coordinated with ${
			others.length > 2 ? `${others.slice(0, 2).join(', ')} and others` : others.join(' and ')
		}`;
	}
	/* Consecutive runs, in time order: a brief to or from Exec on someone else's page stands alone
	 * and open; everything else folds together. */
	const runs = $derived.by(() => {
		const out: { open: boolean; items: AgentExchange[] }[] = [];
		for (const exchange of exchanges) {
			const open =
				host !== 'exec' && (exchange.from_actor === 'exec' || exchange.to_actor === 'exec');
			const last = out.at(-1);
			if (!open && last && !last.open) last.items.push(exchange);
			else out.push({ open, items: [exchange] });
		}
		return out;
	});
	function span(items: AgentExchange[]): string {
		const time = (value: string) =>
			new Date(value).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
		const first = time(items[0].created_at);
		const last = time(items[items.length - 1].created_at);
		return first === last ? first : `${first}–${last}`;
	}
</script>

{#each runs as run, index (index)}
	{#if run.open || run.items.length === 1}
		{#each run.items as exchange (exchange.id)}<HandoffReceipt
				{exchange}
				{name}
				{host}
				expanded={run.open}
			/>{/each}
	{:else}
		<details class="handoff-group">
			<summary
				><span class="group-line">{line(run.items)}</span><span class="group-count"
					>{run.items.length} messages</span
				><time>{span(run.items)}</time></summary
			>
			<div class="group-items">
				{#each run.items as exchange (exchange.id)}<HandoffReceipt {exchange} {name} {host} />{/each}
			</div>
		</details>
	{/if}
{/each}

<style>
	.handoff-group {
		margin: 2px 14px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.handoff-group > summary {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		padding: 3px 6px;
		border-radius: var(--radius-control);
		cursor: pointer;
		list-style: none;
	}
	.handoff-group > summary::-webkit-details-marker {
		display: none;
	}
	.handoff-group > summary:hover {
		background: var(--wash-hover);
	}
	.handoff-group > summary::before {
		flex: none;
		width: 14px;
		height: 1px;
		content: '';
		background: var(--border-strong);
	}
	.group-line {
		min-width: 0;
		overflow: hidden;
		color: var(--text-secondary);
		font-weight: 500;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.group-count {
		flex: 1;
		white-space: nowrap;
	}
	.handoff-group > summary time {
		flex: none;
	}
	.group-items {
		margin-left: 8px;
	}
	.group-items :global(.handoff) {
		margin-inline: 0;
	}
</style>
