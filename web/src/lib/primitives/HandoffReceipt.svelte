<script lang="ts">
	/* A handoff between agents, where it happened in the conversation: one
	 * quiet line, the message itself a click away. */
	import Markdown from './Markdown.svelte';
	import type { AgentExchange } from '$lib/model/exchanges.svelte';

	let { exchange, name }: { exchange: AgentExchange; name: (actorId: string) => string } = $props();

	const line = $derived.by(() => {
		const first =
			exchange.body
				.split('\n')
				.map((part) => part.replace(/[#>*_`]/g, '').trim())
				.find(Boolean) ?? '';
		/* Record IDs belong to the message, not its one-line gist. */
		const gist = first
			.replace(/\b[0-9a-f]{8}(?:-[0-9a-f]{0,12})+…?/gi, '')
			.replace(/\s{2,}/g, ' ')
			.trim();
		return gist.length > 90 ? `${gist.slice(0, 90)}…` : gist;
	});
	const time = $derived(
		new Date(exchange.created_at).toLocaleTimeString(undefined, {
			hour: 'numeric',
			minute: '2-digit'
		})
	);
</script>

<details class="handoff">
	<summary
		><span class="handoff-who">{name(exchange.from_actor)} → {name(exchange.to_actor)}</span><span
			class="handoff-line">{line}</span
		><time datetime={exchange.created_at}>{time}</time></summary
	>
	<div class="handoff-body"><Markdown text={exchange.body} /></div>
</details>

<style>
	.handoff {
		margin: 2px 14px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.handoff summary {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		padding: 3px 6px;
		border-radius: var(--radius-control);
		cursor: pointer;
		list-style: none;
	}
	.handoff summary::-webkit-details-marker {
		display: none;
	}
	.handoff summary:hover {
		background: var(--wash-hover);
	}
	.handoff summary::before {
		flex: none;
		width: 14px;
		height: 1px;
		content: '';
		background: var(--border-strong);
	}
	.handoff-who {
		flex: none;
		color: var(--text-secondary);
		font-weight: 500;
	}
	.handoff-line {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.handoff time {
		flex: none;
	}
	.handoff-body {
		margin: 4px 0 8px 20px;
		padding: 8px 10px;
		border-left: 2px solid var(--border-strong);
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
</style>
