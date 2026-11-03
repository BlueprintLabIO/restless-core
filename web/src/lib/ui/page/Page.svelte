<script lang="ts">
	/* The one page frame every owner surface uses.
	 *
	 * `settings` is a single centred reading column for configuration: one title,
	 * then stacked sections. `list` is a full-width working surface with a compact
	 * sticky header for lists, boards and tables. Pages never draw their own
	 * header, width or scroll container; that drift is what this replaces. */
	import type { Snippet } from 'svelte';
	import InfoTip from '../controls/InfoTip.svelte';
	import WarningMark from '../feedback/WarningMark.svelte';

	let {
		title,
		info,
		count,
		variant = 'settings',
		leading,
		warning,
		actions,
		toolbar,
		children
	}: {
		title: string;
		/** A hover explanation of what the page is for. Never a visible subtitle. */
		info?: string;
		count?: number | string | null;
		variant?: 'settings' | 'list';
		/** A mark before the title, such as an app's logo. */
		leading?: Snippet;
		/** Pass it (even as null) to reserve a fixed "!" that reopens a degraded-state toast. */
		warning?: { label: string; show: () => void } | null;
		actions?: Snippet;
		/** List pages only: filters or view switches under the title row. */
		toolbar?: Snippet;
		children: Snippet;
	} = $props();
</script>

<div class="page {variant}">
	<header class="page-head">
		<div class="page-title">
			{@render leading?.()}
			<h1>{title}</h1>
			{#if count != null && count !== ''}<span class="page-count">{count}</span>{/if}
			{#if info}<InfoTip text={info} />{/if}
			{#if warning !== undefined}<WarningMark {warning} />{/if}
		</div>
		{#if actions}<div class="page-actions">{@render actions()}</div>{/if}
	</header>
	{#if toolbar}<div class="page-toolbar">{@render toolbar()}</div>{/if}
	<div class="page-body">{@render children()}</div>
</div>

<style>
	.page {
		container: page / inline-size;
		position: relative;
		width: 100%;
		height: 100%;
		min-width: 0;
		min-height: 0;
		overflow: auto;
		overscroll-behavior: contain;
	}
	.page-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		min-width: 0;
	}
	.page-title {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		min-width: 0;
	}
	h1 {
		margin: 0;
		overflow: hidden;
		color: var(--ink);
		font-weight: 600;
		letter-spacing: -0.02em;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.page-count {
		color: var(--text-tertiary);
		font-size: var(--t-body);
		font-variant-numeric: tabular-nums;
	}
	.page-actions {
		display: flex;
		align-items: center;
		flex: none;
		gap: var(--space-2);
	}

	/* A centred reading column, Apple Settings and Linear settings alike. */
	.settings .page-head,
	.settings .page-body {
		width: min(720px, calc(100% - 48px));
		margin-inline: auto;
	}
	/* The title sits in the pane's header row, on one line with the sidebar's and the Exec rail's
	 * headers (S109 F1); the column's air goes under it. */
	.settings .page-head {
		min-height: var(--pane-head-h);
		padding: var(--space-1) 0;
	}
	.settings h1 {
		font-size: var(--t-title);
		line-height: 1.2;
	}
	.settings .page-body {
		display: grid;
		/* One column no wider than the page: an implicit one sized to its longest unwrapped line
		 * (the charter on Company's overview) ran 180px off a phone (S110). */
		grid-template-columns: minmax(0, 1fr);
		gap: 36px;
		padding: var(--space-6) 0 64px;
	}

	/* A working surface: the header stays put while the list scrolls under it. */
	.list .page-head {
		position: sticky;
		top: 0;
		z-index: 4;
		min-height: var(--pane-head-h);
		padding: var(--space-1) 12px var(--space-1) var(--pane-pad-x);
		border-bottom: 1px solid var(--border);
		background: color-mix(in srgb, var(--surface-pane) 88%, transparent);
		backdrop-filter: blur(12px);
	}
	.list h1 {
		font-size: var(--t-head);
		line-height: 1.3;
	}
	.list .page-toolbar {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: var(--space-2);
		padding: 8px 12px 8px var(--pane-pad-x);
		border-bottom: 1px solid var(--border);
	}

	@container page (max-width: 560px) {
		.settings .page-head,
		.settings .page-body {
			width: calc(100% - 32px);
		}
		.settings .page-body {
			gap: 28px;
			padding-top: 18px;
		}
		.list .page-head,
		.list .page-toolbar {
			padding-left: 14px;
		}
		/* Actions that do not fit beside the title move under it, rather than squeezing the
		 * title to nothing and running off the edge. */
		.page-head {
			flex-wrap: wrap;
		}
		.page-actions {
			flex: 0 1 auto;
			flex-wrap: wrap;
			min-width: 0;
		}
	}
</style>
