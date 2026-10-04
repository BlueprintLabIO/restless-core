<script lang="ts">
	/* The one page frame every owner surface uses.
	 *
	 * `settings` is a single centred reading column for configuration: one title,
	 * then stacked sections. `list` is a full-width working surface with a compact
	 * sticky header for lists, boards and tables. Pages never draw their own
	 * header, width or scroll container; that drift is what this replaces. */
	import type { Snippet } from 'svelte';
	import InfoTip from '../controls/InfoTip.svelte';

	let {
		title,
		info,
		count,
		variant = 'settings',
		actions,
		toolbar,
		children
	}: {
		title: string;
		/** A hover explanation of what the page is for. Never a visible subtitle. */
		info?: string;
		count?: number | string | null;
		variant?: 'settings' | 'list';
		actions?: Snippet;
		/** List pages only: filters or view switches under the title row. */
		toolbar?: Snippet;
		children: Snippet;
	} = $props();
</script>

<div class="page {variant}">
	<header class="page-head">
		<div class="page-title">
			<h1>{title}</h1>
			{#if count != null && count !== ''}<span class="page-count">{count}</span>{/if}
			{#if info}<InfoTip text={info} />{/if}
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
	.settings .page-head {
		padding: 36px 0 24px;
	}
	.settings h1 {
		font-size: var(--t-title);
		line-height: 1.2;
	}
	.settings .page-body {
		display: grid;
		gap: 36px;
		padding-bottom: 64px;
	}

	/* A working surface: the header stays put while the list scrolls under it. */
	.list .page-head {
		position: sticky;
		top: 0;
		z-index: 4;
		min-height: 52px;
		padding: 8px 12px 8px 20px;
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
		padding: 8px 12px 8px 20px;
		border-bottom: 1px solid var(--border);
	}

	@container page (max-width: 560px) {
		.settings .page-head,
		.settings .page-body {
			width: calc(100% - 32px);
		}
		.settings .page-head {
			padding: 22px 0 18px;
		}
		.settings .page-body {
			gap: 28px;
		}
		.list .page-head,
		.list .page-toolbar {
			padding-left: 14px;
		}
	}
</style>
