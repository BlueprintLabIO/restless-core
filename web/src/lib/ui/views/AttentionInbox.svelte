<script lang="ts" module>
	import type { ActorKind } from '../glyph/ActorTag.svelte';

	export type AttentionEntry = {
		id: string;
		title: string;
		/** Decision, Review, Input, Sign-in… */
		category: string;
		from: string;
		fromKind: ActorKind;
		age: string;
		state: 'needs-you' | 'preparing' | 'done';
	};
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';
	import CircleDot from '@lucide/svelte/icons/circle-dot';
	import ActorTag from '../glyph/ActorTag.svelte';
	import { listFlip, listIn, listOut } from '../motion';

	/* Attention: only what needs a person, each item prepared by the agent that owns it. The selected
	 * item's detail (usually an OutcomeFolio) is the caller's snippet. */
	let {
		items,
		selected,
		detail,
		quiet = 0
	}: {
		items: AttentionEntry[];
		selected?: string;
		detail?: Snippet;
		/** Events handled today without reaching anyone, shown as context. */
		quiet?: number;
	} = $props();

	const waiting = $derived(items.filter((item) => item.state === 'needs-you').length);
	const LABEL = { 'needs-you': 'Needs you', preparing: 'Preparing', done: 'Done' } as const;
</script>

<div class="view-shell">
	<div class="attention-inbox" class:with-detail={!!detail}>
		<section class="ai-list" aria-label="Attention">
			<header class="ai-head">
				<CircleDot size={15} strokeWidth={2.2} />
				<strong>Attention</strong>
				<span class="ai-count">{waiting} {waiting === 1 ? 'needs you' : 'need you'}</span>
			</header>
			{#if quiet}<p class="ai-quiet">{quiet} events handled today without anyone</p>{/if}
			<ol>
				{#each items as item (item.id)}
					<li class="ai-item {item.state}" class:active={item.id === selected} animate:listFlip in:listIn out:listOut>
						<span class="ai-category">{item.category}</span>
						<strong>{item.title}</strong>
						<span class="ai-meta">{item.from} <ActorTag kind={item.fromKind} /> · {item.age}</span>
						<span class="ai-state">{LABEL[item.state]}</span>
					</li>
				{/each}
			</ol>
		</section>
		{#if detail}<section class="ai-detail">{@render detail()}</section>{/if}
	</div>
</div>

<style>
	/* Breakpoints follow this view's own width, not the window's: the same view sits in a full
	 * workspace pane, a narrow side panel or a phone. */
	.view-shell {
		container-type: inline-size;
		width: 100%;
		height: 100%;
		min-width: 0;
	}
	.attention-inbox {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		min-width: 0;
		height: 100%;
		background: var(--surface-pane);
		color: var(--ink);
		font: 400 var(--t-body) var(--font-ui);
	}
	.attention-inbox.with-detail {
		grid-template-columns: minmax(220px, 300px) minmax(0, 1fr);
	}
	.ai-list {
		display: grid;
		grid-template-rows: auto auto 1fr;
		align-content: start;
		min-width: 0;
		border-right: 1px solid var(--border);
	}
	.ai-head {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 12px 14px;
		border-bottom: 1px solid var(--border);
		color: var(--intent-direction);
	}
	.ai-head strong {
		font-size: var(--t-head);
		color: var(--ink);
	}
	.ai-count {
		margin-left: auto;
		padding: 1px 7px;
		border-radius: 999px;
		background: var(--intent-direction-soft);
		font: 500 var(--t-label) var(--font-mono);
	}
	.ai-quiet {
		margin: 0;
		padding: 7px 14px;
		border-bottom: 1px solid var(--border);
		font: 500 var(--t-label) var(--font-mono);
		color: var(--state-success);
	}
	ol {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.ai-item {
		display: grid;
		gap: 3px;
		padding: 10px 14px;
		border-bottom: 1px solid var(--border);
	}
	.ai-item.active {
		background: var(--surface-alt);
		box-shadow: inset 3px 0 0 var(--intent-direction);
	}
	.ai-item.done {
		opacity: 0.6;
	}
	.ai-category {
		font: 500 var(--t-label) var(--font-mono);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: var(--intent-direction);
	}
	.ai-item strong {
		font-weight: 620;
		line-height: 1.35;
	}
	.ai-meta {
		display: flex;
		align-items: center;
		gap: 5px;
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}
	.ai-state {
		justify-self: start;
		padding: 1px 6px;
		border-radius: var(--radius-control);
		background: var(--intent-direction-soft);
		color: var(--intent-direction);
		font: 500 var(--t-label) var(--font-mono);
	}
	.preparing .ai-state {
		background: var(--surface-alt);
		color: var(--text-tertiary);
	}
	.done .ai-state {
		background: var(--state-success-soft);
		color: var(--state-success);
	}
	.ai-detail {
		min-width: 0;
		min-height: 0;
		overflow: auto;
		padding: 14px;
	}
	.ai-detail :global(.owner-folio) {
		margin: 0;
	}
	@container (max-width: 760px) {
		.attention-inbox.with-detail {
			grid-template-columns: minmax(0, 1fr);
		}
		.ai-list {
			border-right: 0;
		}
	}
</style>
