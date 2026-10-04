<script lang="ts">
	/* Detail kept out of the way until asked for: passing checks, retired items,
	 * raw state, history. One row with a chevron that turns as it opens. */
	import type { Snippet } from 'svelte';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';

	let {
		label,
		count,
		hint,
		open = $bindable(false),
		children
	}: {
		label: string;
		count?: number | string | null;
		/** Quiet text at the end of the row, such as a date. */
		hint?: string | null;
		open?: boolean;
		children: Snippet;
	} = $props();
</script>

<details class="fold" bind:open>
	<summary>
		<ChevronRight size={14} strokeWidth={1.8} aria-hidden="true" />
		<span class="fold-label">{label}</span>
		{#if count != null && count !== ''}<span class="fold-count">{count}</span>{/if}
		{#if hint}<span class="fold-hint">{hint}</span>{/if}
	</summary>
	<div class="fold-body">{@render children()}</div>
</details>

<style>
	summary {
		display: flex;
		align-items: center;
		gap: 8px;
		min-height: 44px;
		padding: 0 16px;
		color: var(--text-secondary);
		font-size: var(--t-body);
		cursor: pointer;
		list-style: none;
		transition: background-color var(--motion-state) var(--ease-standard);
	}
	summary::-webkit-details-marker {
		display: none;
	}
	summary::before {
		content: none !important;
	}
	summary:hover {
		background: var(--surface-hover);
		color: var(--ink);
	}
	summary :global(svg) {
		flex: none;
		color: var(--text-tertiary);
		transition: transform var(--motion-state) var(--ease-standard);
	}
	.fold[open] > summary :global(svg) {
		transform: rotate(90deg);
	}
	.fold-label {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.fold-count {
		color: var(--text-tertiary);
		font-variant-numeric: tabular-nums;
	}
	.fold-hint {
		margin-left: auto;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		white-space: nowrap;
	}
	.fold-body > :global(* + *) {
		border-top: 1px solid var(--border);
	}
	.fold-body {
		border-top: 1px solid var(--border);
	}
</style>
