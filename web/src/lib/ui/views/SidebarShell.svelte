<script lang="ts">
	/* A sidebar area: a titled list beside one pane. Every left sidebar has the same structure: a
	 * title with the area's main action, a search field or switch under it (`top`), views, then
	 * sections, every row's icon in one column, and what is finished (Archived) pinned to the bottom
	 * in `foot`. The list sits on the canvas, not on a card; it gets its presence from legible type,
	 * real controls and a raised selected row. Where the area is too narrow for both, the list steps
	 * out; `narrow` is what stands in for it at the top of the pane. */
	import type { Snippet } from 'svelte';

	let {
		label,
		title,
		action,
		top,
		nav,
		foot,
		narrow,
		children
	}: {
		/** What the list navigates, for assistive technology. */
		label: string;
		/** The area's name, shown over the list. */
		title?: string;
		/** The area's main action, beside the title: a "+" or "Ask Exec". */
		action?: Snippet;
		/** Under the title: a search field, a view switch. */
		top?: Snippet;
		nav: Snippet;
		/** Pinned at the bottom of the list: Archived. */
		foot?: Snippet;
		narrow?: Snippet;
		children: Snippet;
	} = $props();
</script>

<div class="sidebar-shell">
	<nav class="sidebar-nav" aria-label={label}>
		{#if title || action}<div class="sidebar-head">
				{#if title}<h2>{title}</h2>{/if}
				{#if action}<span class="sidebar-head-action">{@render action()}</span>{/if}
			</div>{/if}
		{#if top}<div class="sidebar-top">{@render top()}</div>{/if}
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

	.sidebar-nav {
		display: flex;
		flex: none;
		flex-direction: column;
		width: var(--sidebar-width, 264px);
		min-height: 0;
		padding: 4px 8px 8px 6px;
	}
	.sidebar-head {
		display: flex;
		flex: none;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		min-height: 40px;
		padding: 0 2px 0 8px;
	}
	.sidebar-head h2 {
		margin: 0;
		overflow: hidden;
		color: var(--ink);
		font-size: var(--t-head);
		font-weight: 600;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.sidebar-head-action {
		display: inline-flex;
		flex: none;
		gap: 4px;
	}
	.sidebar-top {
		display: grid;
		flex: none;
		grid-template-columns: minmax(0, 1fr);
		gap: 10px;
		min-width: 0;
		padding: 6px 2px 10px;
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
		/* room for the raised selected row's shadow */
		padding: 1px 2px 2px;
	}
	.sidebar-foot {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		flex: none;
		gap: 1px;
		margin-top: 8px;
		padding: 0 2px;
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
