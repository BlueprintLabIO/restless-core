<script lang="ts">
	/* A settings row: what it is on the left, its value or control on the right.
	 * Explanations live in the tooltip, never as a second line of grey text. */
	import type { Snippet } from 'svelte';
	import InfoTip from '../controls/InfoTip.svelte';

	let {
		label,
		info,
		stack = false,
		children
	}: {
		label: string;
		info?: string;
		/** Puts the control under the label, for editors and long values. */
		stack?: boolean;
		children?: Snippet;
	} = $props();
</script>

<div class="row" class:stack>
	<div class="row-label">
		<span>{label}</span>
		{#if info}<InfoTip text={info} />{/if}
	</div>
	{#if children}<div class="row-control">{@render children()}</div>{/if}
</div>

<style>
	.row {
		display: grid;
		grid-template-columns: minmax(140px, 0.9fr) minmax(0, 1.6fr);
		align-items: center;
		gap: var(--space-4);
		min-height: 52px;
		padding: 10px 16px;
	}
	.row.stack {
		grid-template-columns: minmax(0, 1fr);
		gap: var(--space-2);
		padding-block: 14px;
	}
	.row-label {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		color: var(--ink);
		font-size: var(--t-body);
		font-weight: 500;
	}
	.row-control {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		flex-wrap: wrap;
		gap: var(--space-2);
		min-width: 0;
		color: var(--text-secondary);
		font-size: var(--t-body);
		text-align: right;
		font-variant-numeric: tabular-nums;
	}
	.stack .row-control {
		justify-content: flex-start;
		text-align: left;
	}
	@container page (max-width: 560px) {
		.row {
			grid-template-columns: minmax(0, 1fr);
			gap: 6px;
		}
		.row-control {
			justify-content: flex-start;
			text-align: left;
		}
	}
</style>
