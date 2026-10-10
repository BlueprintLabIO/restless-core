<script lang="ts">
	/* A sidebar section: a small muted label over rows that share the views' icon column. The label
	 * can itself be a view (Documents shows every document), carry a count, and fold.
	 * An explanation lives in the help mark's tooltip, never as text in the list; an action (add,
	 * propose) is an icon on the title, shown on hover. */
	import type { Component, Snippet } from 'svelte';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
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
	<div class="sidebar-title" class:active class:folded={!open}>
		{#if href}<a class="sidebar-title-label" {href} aria-current={active ? 'page' : undefined}
				>{label}</a
			>{:else if children}<button
				class="sidebar-title-label"
				type="button"
				aria-expanded={open}
				onclick={toggle}>{label}</button
			>{:else}<span class="sidebar-title-label">{label}</span>{/if}
		{#if count != null && count > 0}<b class="sidebar-title-count">{count}</b>{/if}
		{#if children}<button
				class="sidebar-fold"
				type="button"
				aria-expanded={open}
				aria-label={open ? `Fold ${label}` : `Unfold ${label}`}
				onclick={toggle}><ChevronDown size={12} strokeWidth={2.2} aria-hidden="true" /></button
			>{/if}
		<span class="sidebar-title-gap"></span>
		{#if help}<span class="sidebar-help"><InfoTip text={help} /></span>{/if}
		{#if action}{@const Icon = action.icon}<button
				class="sidebar-action"
				type="button"
				title={action.label}
				aria-label={action.label}
				onclick={action.onclick}><Icon size={14} strokeWidth={2} /></button
			>{/if}
	</div>
	{#if children && open}<div class="sidebar-items">{@render children()}</div>{/if}
</div>

<style>
	/* A section label you can read: body size at weight 600 in secondary ink, the caret after it on
	 * hover; the rows keep the same icon column as the views above them, so nothing is indented. */
	.sidebar-group {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: 1px;
		margin-top: 12px;
	}
	.sidebar-title {
		display: flex;
		align-items: center;
		gap: 4px;
		height: 30px;
		padding: 0 4px 0 8px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		font-size: var(--t-body);
		font-weight: 600;
	}
	.sidebar-title.active {
		color: var(--ink);
	}
	.sidebar-title-label {
		min-width: 0;
		overflow: hidden;
		padding: 0;
		border: 0;
		background: transparent;
		color: inherit;
		font: inherit;
		text-decoration: none;
		text-overflow: ellipsis;
		white-space: nowrap;
		cursor: pointer;
	}
	span.sidebar-title-label {
		cursor: default;
	}
	.sidebar-title:hover .sidebar-title-label {
		color: var(--text-secondary);
	}
	.sidebar-title-gap {
		flex: 1 1 auto;
	}
	.sidebar-fold {
		display: inline-grid;
		place-items: center;
		width: 16px;
		height: 16px;
		padding: 0;
		border: 0;
		border-radius: 4px;
		background: transparent;
		color: var(--text-tertiary);
		opacity: 0;
		cursor: pointer;
		transition:
			opacity var(--motion-state) var(--ease-standard),
			transform var(--motion-disclosure) var(--ease-out);
	}
	.sidebar-title:hover .sidebar-fold,
	.sidebar-title.folded .sidebar-fold,
	.sidebar-fold:focus-visible {
		opacity: 1;
	}
	.sidebar-title.folded .sidebar-fold {
		transform: rotate(-90deg);
	}
	.sidebar-fold:hover {
		color: var(--ink);
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
	.sidebar-help {
		opacity: 0;
		transition: opacity var(--motion-state) var(--ease-standard);
	}
	.sidebar-title:hover .sidebar-help,
	.sidebar-help:focus-within {
		opacity: 1;
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
		padding: 0 2px;
		color: var(--text-tertiary);
		font-size: var(--t-body);
		font-weight: 400;
		font-variant-numeric: tabular-nums;
		text-align: center;
	}
	.sidebar-items {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: 1px;
	}
	.sidebar-fold:focus-visible,
	.sidebar-action:focus-visible,
	.sidebar-title-label:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: -2px;
	}
</style>
