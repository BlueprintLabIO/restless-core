<script lang="ts">
	/* A handoff between agents, where it happened in the conversation: one
	 * quiet line, the message itself a click away. On the page of the person it
	 * concerns, it opens as the message itself: the brief they got, the reply
	 * they sent. */
	import Markdown from './Markdown.svelte';
	import { page } from '$app/state';
	import type { AgentExchange } from '$lib/model/exchanges.svelte';

	let {
		exchange,
		name,
		host = 'exec',
		expanded = false
	}: {
		exchange: AgentExchange;
		name: (actorId: string) => string;
		/** Whose conversation this sits in. */
		host?: string;
		expanded?: boolean;
	} = $props();

	/* The other side has its own conversation, where this message also lives. */
	const other = $derived(exchange.from_actor === host ? exchange.to_actor : exchange.from_actor);
	const otherHref = $derived(
		other && other !== host && other !== 'owner'
			? `/${encodeURIComponent(page.params.companyId ?? '')}/people?person=${encodeURIComponent(other)}`
			: ''
	);
	const heading = $derived(
		exchange.to_actor === host
			? `From ${name(exchange.from_actor)}`
			: exchange.from_actor === host
				? `To ${name(exchange.to_actor)}`
				: `${name(exchange.from_actor)} → ${name(exchange.to_actor)}`
	);

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

{#if expanded}
	<article class="handoff-card">
		<header>
			<span class="handoff-who">{heading}</span><time datetime={exchange.created_at}>{time}</time>
		</header>
		<Markdown text={exchange.body} />
	</article>
{:else}
	<details class="handoff">
		<summary
			><span class="handoff-who">{name(exchange.from_actor)} → {name(exchange.to_actor)}</span><span
				class="handoff-line">{line}</span
			><time datetime={exchange.created_at}>{time}</time></summary
		>
		<div class="handoff-body">
			<Markdown text={exchange.body} />
			{#if otherHref}<a class="handoff-open" href={otherHref}>Open {name(other)}'s conversation</a
				>{/if}
		</div>
	</details>
{/if}

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
	.handoff-open {
		display: inline-block;
		margin-top: 6px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.handoff-open:hover {
		color: var(--ink);
	}
	.handoff-card {
		margin: 6px 14px;
		padding: 10px 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-card, 10px);
		background: var(--surface);
		color: var(--text-primary, var(--ink));
		font-size: var(--t-body);
	}
	.handoff-card header {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 8px;
		margin-bottom: 4px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.handoff-card .handoff-who {
		color: var(--text-secondary);
	}
</style>
