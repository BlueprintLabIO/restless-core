<script lang="ts">
	/* One sidebar row: an icon, a label, and what is true there. Rows are body size at weight 500 so
	 * they read at a glance. A second line (`sub`) says what the person or goal is doing; a reading
	 * on the right (`reading`) says a setting's value. Both take a semantic tone: working or live,
	 * waiting on the owner, blocked or broken, unread. A count that needs the owner is filled; a plain
	 * total stays quiet. The selected row is raised, like the selected top-navigation item. */
	import type { Component, Snippet } from 'svelte';

	export type SidebarTone = 'work' | 'wait' | 'block' | 'unread' | '';

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
		sub,
		subTone = '',
		reading,
		readingTone = '',
		strong = false,
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
		/** A second line: what this person or goal is doing. */
		sub?: string;
		subTone?: SidebarTone;
		/** The current value, at the end of the row: "Connected", "4 keys". */
		reading?: string;
		readingTone?: SidebarTone;
		/** Unread: the label is bold. */
		strong?: boolean;
		/** A secondary row, such as "12 more". */
		quiet?: boolean;
		/** Belongs to the row above it, outside a section (an archived goal under Archived). */
		indent?: boolean;
		/** Replaces the icon: an avatar, or a mark that is the item's state. */
		leading?: Snippet;
		/** Replaces the count; extra lines under the label go in `sub`. */
		trailing?: Snippet;
		title?: string;
	} = $props();
</script>

{#snippet content()}
	{#if leading}<i class="sidebar-icon">{@render leading()}</i>{:else if Icon}<i
			class="sidebar-icon"
			aria-hidden="true"><Icon size={16} strokeWidth={1.75} /></i
		>{/if}<span class="sidebar-text"
		><span class="sidebar-label" class:strong>{label}</span>{#if sub}<small
				class="sidebar-sub {subTone}"
				title={sub}>{sub}</small
			>{/if}</span
	>{#if trailing}{@render trailing()}{:else if reading}<span
			class="sidebar-reading {readingTone}"
			title={reading}>{reading}</span
		>{:else if count != null && count > 0}<b
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
		class:two={!!sub}
		{href}
		{title}
		aria-current={active ? 'page' : undefined}>{@render content()}</a
	>{:else}<button
		class="sidebar-row"
		class:active
		class:quiet
		class:indent
		class:two={!!sub}
		type="button"
		{title}
		aria-pressed={active}
		{onclick}>{@render content()}</button
	>{/if}

<style>
	.sidebar-row {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		min-height: 34px;
		padding: 0 8px;
		border: 0;
		border-radius: var(--radius-pane);
		background: transparent;
		color: var(--ink);
		font: inherit;
		font-size: var(--t-body);
		font-weight: 500;
		text-align: left;
		text-decoration: none;
		cursor: pointer;
		transition:
			background var(--motion-state) var(--ease-standard),
			box-shadow var(--motion-state) var(--ease-standard);
	}
	.sidebar-row.two {
		align-items: flex-start;
		min-height: 48px;
		padding-top: 6px;
		padding-bottom: 6px;
	}
	.sidebar-row.quiet {
		min-height: 28px;
		color: var(--text-tertiary);
		font-weight: 400;
	}
	.sidebar-row.indent {
		padding-left: 34px;
	}
	.sidebar-icon {
		display: inline-flex;
		flex: none;
		align-items: center;
		min-height: 18px;
	}
	.sidebar-row.two .sidebar-icon {
		margin-top: 1px;
	}
	.sidebar-text {
		display: grid;
		flex: 1 1 auto;
		min-width: 0;
	}
	.sidebar-label,
	.sidebar-sub {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.sidebar-label.strong {
		font-weight: 650;
	}
	.sidebar-sub {
		color: var(--text-tertiary);
		font-size: var(--t-body);
		font-weight: 400;
	}
	.sidebar-row :global(svg) {
		flex: none;
		color: var(--text-secondary);
	}
	.sidebar-row:hover {
		background: var(--wash-hover);
	}
	/* The current view: raised, as the selected top-navigation item is. */
	.sidebar-row.active {
		background: var(--surface-raised);
		box-shadow:
			var(--shadow-soft),
			var(--bevel),
			0 0 0 1px var(--border);
		font-weight: 600;
	}
	.sidebar-row.active :global(svg) {
		color: var(--ink);
	}
	.sidebar-row:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: -2px;
	}
	/* A reading gives way to the label: it truncates, with the whole value in its tooltip. */
	.sidebar-reading {
		flex: 0 1 auto;
		min-width: 0;
		max-width: 55%;
		margin-left: auto;
		overflow: hidden;
		color: var(--text-tertiary);
		font-weight: 400;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.sidebar-row:has(.sidebar-reading) .sidebar-text {
		flex: 1 0 auto;
		max-width: 60%;
	}
	.sidebar-count {
		flex: none;
		min-width: 18px;
		padding: 0 2px;
		color: var(--text-tertiary);
		font-weight: 400;
		font-variant-numeric: tabular-nums;
		line-height: 18px;
		text-align: right;
	}
	.sidebar-row.two .sidebar-count,
	.sidebar-row.two .sidebar-reading {
		margin-top: 1px;
	}
	.sidebar-count.todo {
		min-width: 18px;
		height: 18px;
		padding: 0 5px;
		border-radius: 9px;
		background: var(--intent-authority);
		color: var(--on-primary);
		font-size: var(--t-label);
		font-weight: 600;
		text-align: center;
	}
	.work {
		color: var(--intent-feedback);
	}
	.wait {
		color: var(--intent-authority);
	}
	.block {
		color: var(--state-danger);
	}
	.unread {
		color: var(--intent-conversation);
	}
	.sidebar-dot,
	.sidebar-problem {
		flex: none;
		align-self: center;
		width: 7px;
		height: 7px;
		margin-right: 4px;
		border-radius: 50%;
	}
	.sidebar-dot {
		background: var(--intent-authority);
	}
	.sidebar-problem {
		background: var(--state-danger);
	}
</style>
