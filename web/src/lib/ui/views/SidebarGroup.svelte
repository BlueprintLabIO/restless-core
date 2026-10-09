<script lang="ts">
	/* A sidebar section: a title a step larger than its rows, with the rows indented beneath it.
	 * The title can itself be a view (Documents shows every document), carry a count, and fold.
	 * An explanation lives in the help mark's tooltip, never as text in the list; an action (add,
	 * propose) is an icon on the title, shown on hover. */
	import type { Component, Snippet } from 'svelte';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import InfoTip from '../controls/InfoTip.svelte';

	let {
		label,
		href,
		active = false,
		count = null,
		help,
		action,
		children
	}: {
		label: string;
		/** The title is a view of everything in the section. */
		href?: string;
		active?: boolean;
		count?: number | null;
		/** What the section is, shown as a tooltip on the help mark. */
		help?: string;
		action?: {
			label: string;
			icon: Component<{ size?: number; strokeWidth?: number }>;
			onclick: () => void;
		};
		children?: Snippet;
	} = $props();

	/* Folding is a per-viewer convenience, remembered where the browser allows. */
	const storageKey = $derived(`sidebar-fold:${label}`);
	let open = $state(true);
	$effect(() => {
		try {
			open = localStorage.getItem(storageKey) !== 'closed';
		} catch {
			open = true;
		}
	});
	function toggle() {
		open = !open;
		try {
			if (open) localStorage.removeItem(storageKey);
			else localStorage.setItem(storageKey, 'closed');
		} catch {
			/* Folding still works for this visit. */
		}
	}
</script>

<div class="sidebar-group" role="group" aria-label={label}>
	<div class="sidebar-title" class:active>
		{#if children}<button
				class="sidebar-fold"
				type="button"
				aria-expanded={open}
				aria-label={open ? `Fold ${label}` : `Unfold ${label}`}
				onclick={toggle}
				><ChevronRight
					size={14}
					strokeWidth={2}
					aria-hidden="true"
					class={open ? 'open' : ''}
				/></button
			>{/if}
		{#if href}<a class="sidebar-title-label" {href} aria-current={active ? 'page' : undefined}
				>{label}</a
			>{:else}<span class="sidebar-title-label">{label}</span>{/if}
		{#if help}<span class="sidebar-help"><InfoTip text={help} /></span>{/if}
		{#if action}{@const Icon = action.icon}<button
				class="sidebar-action"
				type="button"
				title={action.label}
				aria-label={action.label}
				onclick={action.onclick}><Icon size={14} strokeWidth={2} /></button
			>{/if}
		{#if count != null && count > 0}<b class="sidebar-title-count">{count}</b>{/if}
	</div>
	{#if children && open}<div class="sidebar-items">{@render children()}</div>{/if}
</div>

<style>
	.sidebar-group {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: 1px;
		margin-top: 10px;
	}
	.sidebar-title {
		display: flex;
		align-items: center;
		gap: 4px;
		height: 30px;
		padding: 0 6px 0 2px;
		border-radius: var(--radius-control);
		color: var(--ink);
		font-size: var(--t-head);
		font-weight: 600;
	}
	.sidebar-title.active {
		background: var(--surface-raised, var(--wash-active, var(--wash-hover)));
		box-shadow: inset 0 0 0 1px var(--border);
	}
	.sidebar-fold {
		display: inline-grid;
		place-items: center;
		width: 20px;
		height: 20px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
	}
	.sidebar-fold:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.sidebar-fold :global(svg) {
		transition: transform var(--motion-disclosure) var(--ease-out);
	}
	.sidebar-fold :global(svg.open) {
		transform: rotate(90deg);
	}
	.sidebar-title-label {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		color: inherit;
		text-decoration: none;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	a.sidebar-title-label:hover {
		text-decoration: underline;
		text-decoration-color: var(--border-strong);
		text-underline-offset: 3px;
	}
	.sidebar-group:not(:has(.sidebar-fold)) .sidebar-title-label {
		padding-left: 6px;
	}
	.sidebar-help,
	.sidebar-action {
		display: inline-grid;
		flex: none;
		place-items: center;
		width: 22px;
		height: 22px;
		color: var(--text-tertiary);
	}
	.sidebar-action {
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		opacity: 0;
		cursor: pointer;
		transition: opacity var(--motion-state) var(--ease-standard);
	}
	.sidebar-title:hover .sidebar-action,
	.sidebar-action:focus-visible {
		opacity: 1;
	}
	.sidebar-action:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.sidebar-title-count {
		flex: none;
		min-width: 18px;
		padding: 0 5px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 500;
		font-variant-numeric: tabular-nums;
		text-align: center;
	}
	.sidebar-items {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: 1px;
	}
	/* Rows belong to their section: indented past the fold mark. */
	.sidebar-items :global(.sidebar-row) {
		padding-left: 26px;
	}
	.sidebar-fold:focus-visible,
	.sidebar-action:focus-visible,
	a.sidebar-title-label:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: -2px;
	}
</style>
