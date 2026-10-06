<script lang="ts">
	/* A settings area as Linear's is: a quiet section list beside one pane. Where the area is too
	 * narrow for both, the list steps out; `narrow` is what stands in for it at the top of the
	 * pane, such as a way back or a row of the same sections. */
	import type { Snippet } from 'svelte';

	let {
		label,
		nav,
		narrow,
		children
	}: {
		/** What the list navigates, for assistive technology. */
		label: string;
		nav: Snippet;
		narrow?: Snippet;
		children: Snippet;
	} = $props();
</script>

<div class="sidebar-shell">
	<nav class="sidebar-nav" aria-label={label}>{@render nav()}</nav>
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

	/* Linear's settings sidebar: 13px rows at 30px, quiet group names. */
	.sidebar-nav {
		display: grid;
		flex: none;
		align-content: start;
		gap: 1px;
		width: 208px;
		padding: 6px 4px;
		overflow-y: auto;
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
