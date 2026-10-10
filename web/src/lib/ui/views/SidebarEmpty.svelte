<script lang="ts">
	/* What a section says when it has nothing yet: one sentence on why, then the real actions that
	 * fill it, as buttons. */
	import type { Snippet } from 'svelte';

	let { text, children }: { text: string; children?: Snippet } = $props();
</script>

<div class="sidebar-empty">
	<p>{text}</p>
	{#if children}<div class="sidebar-empty-actions">{@render children()}</div>{/if}
</div>

<style>
	.sidebar-empty {
		margin: 6px 4px 0;
		padding: 14px;
		border: 1px dashed var(--border-strong);
		border-radius: var(--radius-lg);
		color: var(--text-secondary);
	}
	p {
		margin: 0;
		line-height: 1.5;
	}
	.sidebar-empty-actions {
		display: grid;
		gap: 6px;
		margin-top: 12px;
	}
	.sidebar-empty-actions :global(:is(a, button)) {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
		height: 32px;
		padding: 0 12px;
		border: 1px solid var(--edge-control);
		border-radius: var(--radius-pane);
		background: var(--surface-raised);
		box-shadow: var(--control-depth), var(--bevel);
		color: var(--ink);
		font: inherit;
		font-weight: 500;
		text-decoration: none;
		cursor: pointer;
	}
	.sidebar-empty-actions :global(:is(a, button):hover) {
		border-color: var(--edge-control-hover);
	}
	.sidebar-empty-actions :global(:is(a, button).primary) {
		border-color: var(--primary-edge);
		background: var(--primary);
		color: var(--on-primary);
	}
</style>
