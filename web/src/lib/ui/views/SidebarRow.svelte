<script lang="ts">
	/* One sidebar row: an icon, a label, and what waits there. A count that needs the owner is
	 * marked; a plain total stays quiet; a problem is a red dot; an item waiting on the owner is an
	 * amber dot whose tooltip says why. Detail belongs in the tooltip, never a second line. */
	import type { Component, Snippet } from 'svelte';

	let {
		href,
		onclick,
		label,
		icon: Icon,
		active = false,
		count = null,
		todo = false,
		problem = false,
		dot,
		quiet = false,
		indent = false,
		leading,
		trailing,
		title
	}: {
		href?: string;
		/** A row that changes the view in place rather than navigating. */
		onclick?: () => void;
		label: string;
		icon?: Component<{ size?: number; strokeWidth?: number }>;
		active?: boolean;
		/** A number shown at the end of the row. */
		count?: number | null;
		/** The count is waiting on the owner rather than a plain total. */
		todo?: boolean;
		problem?: boolean;
		/** This item waits on the owner; the text is the dot's tooltip. */
		dot?: string;
		/** A secondary row, such as "12 more". */
		quiet?: boolean;
		/** Belongs to the row above it, outside a section (an archived goal under Archived). */
		indent?: boolean;
		/** Replaces the icon, for a mark that is the item's state, such as a goal's progress ring. */
		leading?: Snippet;
		/** Replaces the count. */
		trailing?: Snippet;
		title?: string;
	} = $props();
</script>

{#snippet content()}
	{#if leading}<i class="sidebar-icon">{@render leading()}</i>{:else if Icon}<i
			class="sidebar-icon"
			aria-hidden="true"><Icon size={15} strokeWidth={1.8} /></i
		>{/if}<span>{label}</span
	>{#if trailing}{@render trailing()}{:else if count != null && count > 0}<b
			class="sidebar-count"
			class:todo
			aria-label={todo ? `${count} to do` : `${count}`}>{count}</b
		>{/if}{#if dot}<i class="sidebar-dot" title={dot} aria-label={dot} role="img"
		></i>{:else if problem}<i class="sidebar-problem" aria-label="Needs attention"></i>{/if}
{/snippet}

{#if href}<a
		class="sidebar-row"
		class:active
		class:quiet
		class:indent
		{href}
		{title}
		aria-current={active ? 'page' : undefined}>{@render content()}</a
	>{:else}<button
		class="sidebar-row"
		class:active
		class:quiet
		class:indent
		type="button"
		{title}
		aria-pressed={active}
		{onclick}>{@render content()}</button
	>{/if}

<style>
	.sidebar-row {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		height: 30px;
		padding: 0 8px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--t-body);
		text-align: left;
		text-decoration: none;
		white-space: nowrap;
		cursor: pointer;
		transition:
			background var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard);
	}
	.sidebar-row.quiet {
		height: 26px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.sidebar-row.indent {
		padding-left: 26px;
	}
	.sidebar-icon {
		display: inline-flex;
		flex: none;
	}
	.sidebar-row > span {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.sidebar-row :global(svg) {
		flex: none;
		color: var(--text-tertiary);
	}
	.sidebar-row:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	/* The current view is raised, not just washed, so it reads at a glance. */
	.sidebar-row.active {
		background: var(--surface-pane);
		box-shadow:
			inset 0 0 0 1px var(--border),
			var(--shadow-soft);
		color: var(--ink);
		font-weight: 500;
	}
	.sidebar-row:hover :global(svg),
	.sidebar-row.active :global(svg) {
		color: var(--ink);
	}
	.sidebar-row:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: -2px;
	}
	.sidebar-count {
		flex: none;
		min-width: 18px;
		padding: 0 5px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 500;
		font-variant-numeric: tabular-nums;
		line-height: 18px;
		text-align: center;
	}
	.sidebar-count.todo {
		border-radius: 9px;
		background: color-mix(in srgb, var(--surface-attention) 22%, transparent);
		color: var(--ink);
	}
	.sidebar-dot,
	.sidebar-problem {
		flex: none;
		width: 6px;
		height: 6px;
		margin-right: 6px;
		border-radius: 50%;
	}
	.sidebar-dot {
		background: var(--surface-attention, var(--state-warning));
	}
	.sidebar-problem {
		background: var(--state-danger);
	}
</style>
