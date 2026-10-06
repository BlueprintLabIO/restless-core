<script lang="ts">
	/* One sidebar row: an icon, a label, and what waits there. A count that needs the owner is
	 * marked; a plain total stays quiet; a problem is a dot. */
	import type { Component } from 'svelte';

	let {
		href,
		label,
		icon: Icon,
		active = false,
		count = null,
		todo = false,
		problem = false,
		title
	}: {
		href: string;
		label: string;
		icon?: Component<{ size?: number; strokeWidth?: number }>;
		active?: boolean;
		/** A number shown at the end of the row. */
		count?: number | null;
		/** The count is waiting on the owner rather than a plain total. */
		todo?: boolean;
		problem?: boolean;
		title?: string;
	} = $props();
</script>

<a class="sidebar-row" class:active {href} {title} aria-current={active ? 'page' : undefined}
	>{#if Icon}<i class="sidebar-icon" aria-hidden="true"><Icon size={15} strokeWidth={1.8} /></i
		>{/if}<span>{label}</span>{#if count != null && count > 0}<b
			class="sidebar-count"
			class:todo
			aria-label={todo ? `${count} to do` : `${count}`}>{count}</b
		>{:else if problem}<i class="sidebar-problem" aria-label="Needs attention"></i>{/if}</a
>

<style>
	.sidebar-row {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 30px;
		padding: 0 8px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		font-size: var(--t-body);
		text-decoration: none;
		white-space: nowrap;
		transition:
			background var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard);
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
	.sidebar-row.active {
		background: var(--wash-active, var(--wash-hover));
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
	.sidebar-problem {
		flex: none;
		width: 6px;
		height: 6px;
		margin-right: 6px;
		border-radius: 50%;
		background: var(--state-danger);
	}
</style>
