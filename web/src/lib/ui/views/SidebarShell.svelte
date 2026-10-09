<script lang="ts">
	/* A sidebar area as Linear's is: a quiet section list beside one pane. Every left sidebar has
	 * the same structure: views at the top, named sections with their rows indented beneath, and
	 * what is finished (Archived) pinned to the bottom in `foot`. Where the area is too narrow for
	 * both, the list steps out; `narrow` is what stands in for it at the top of the pane, such as a
	 * way back or a row of the same sections. */
	import type { Snippet } from 'svelte';

	let {
		label,
		nav,
		foot,
		narrow,
		children
	}: {
		/** What the list navigates, for assistive technology. */
		label: string;
		nav: Snippet;
		/** Pinned below a rule at the bottom of the list: Archived. */
		foot?: Snippet;
		narrow?: Snippet;
		children: Snippet;
	} = $props();
</script>

<div class="sidebar-shell">
	<nav class="sidebar-nav" aria-label={label}>
		<div class="sidebar-list">{@render nav()}</div>
		{#if foot}<div class="sidebar-foot">{@render foot()}</div>{/if}
	</nav>
	<main class="sidebar-main">
		{#if narrow}<div class="sidebar-narrow">{@render narrow()}</div>{/if}
		{@render children()}
	</main>
</div>

<style>
	.sidebar-shell {
		container: sidebar / inline-size;
		display: flex;
		flex: 1 1 auto;
		gap: 8px;
		width: 100%;
		min-width: 0;
		min-height: 0;
	}
	.sidebar-main {
		position: relative;
		display: flex;
		flex: 1 1 auto;
		flex-direction: column;
		width: 100%;
		min-width: 0;
		min-height: 0;
		overflow: hidden;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		background: var(--surface-pane);
		box-shadow: var(--bevel), var(--shadow-soft);
	}
	.sidebar-main > :global(:not(.sidebar-narrow)) {
		flex: 1 1 auto;
		min-height: 0;
	}

	/* Linear's sidebar: 13px rows at 30px; section titles a step larger. */
	.sidebar-nav {
		display: flex;
		flex: none;
		flex-direction: column;
		width: 208px;
		min-height: 0;
		padding: 6px 4px;
	}
	.sidebar-list {
		display: grid;
		flex: 1 1 auto;
		grid-template-columns: minmax(0, 1fr);
		align-content: start;
		gap: 1px;
		min-height: 0;
		overflow-x: hidden;
		overflow-y: auto;
	}
	/* A rule between the top views and the first section, so the two read apart at a glance. */
	.sidebar-list > :global(.sidebar-row + .sidebar-group) {
		margin-top: 8px;
		padding-top: 8px;
		border-top: 1px solid var(--border-strong);
	}
	.sidebar-foot {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		flex: none;
		gap: 1px;
		margin-top: 8px;
		padding-top: 6px;
		border-top: 1px solid var(--border-strong);
	}
	.sidebar-narrow {
		display: none;
	}

	@container sidebar (max-width: 760px) {
		.sidebar-nav {
			display: none;
		}
		.sidebar-narrow {
			display: block;
			flex: none;
		}
	}
</style>
