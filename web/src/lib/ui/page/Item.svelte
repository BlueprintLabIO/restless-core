<script lang="ts">
	import { truncatedTitle } from '../truncated-title';
	/* One row in any list: Inbox, Work, Library, schedules, skills, activity.
	 *
	 * Leading mark, a single-line title, quiet inline meta, and a trailing area
	 * for state and time. Row actions appear on hover and focus on a pointer
	 * device and stay visible on touch, so nothing is hover-only. */
	import type { Snippet } from 'svelte';

	let {
		title,
		label,
		meta,
		href,
		download = false,
		onclick,
		selected = false,
		unread = false,
		dim = false,
		stacked = false,
		leading,
		trailing,
		actions,
		children,
		role
	}: {
		title: string;
		/** What a screen reader hears for the row link, when the row says more than its title by
		 * look alone (unread, snoozed, its kind). Defaults to the title. */
		label?: string;
		meta?: string | null;
		href?: string;
		/** The link saves a file rather than opening a page. */
		download?: boolean;
		onclick?: (event: MouseEvent) => void;
		selected?: boolean;
		unread?: boolean;
		/** Finished or archived: present but quiet. */
		dim?: boolean;
		/** Title over meta, the title on up to two lines: for narrow lists such as the Inbox, where
		 * one line cut every title to a few words. */
		stacked?: boolean;
		leading?: Snippet;
		trailing?: Snippet;
		actions?: Snippet;
		/** Optional detail under the row, such as an expanded editor. */
		children?: Snippet;
		/** `listitem` when the row sits in a `role="list"`. */
		role?: 'listitem';
	} = $props();
</script>

<div
	class="item"
	{role}
	class:selected
	class:unread
	class:dim
	class:stacked
	class:interactive={!!href || !!onclick}
>
	<div class="item-row">
		{#if href}<a
				class="item-hit"
				{href}
				download={download ? '' : undefined}
				aria-current={selected ? 'true' : undefined}
				{onclick}><span class="sr-only">{label ?? title}</span></a
			>{:else if onclick}<button class="item-hit" type="button" {onclick}
				><span class="sr-only">{label ?? title}</span></button
			>{/if}
		{#if leading}<span class="item-leading">{@render leading()}</span>{/if}
		<span class="item-main">
			<span class="item-title" use:truncatedTitle={title}>{title}</span>
			{#if meta}<span class="item-meta" use:truncatedTitle={meta}>{meta}</span>{/if}
		</span>
		{#if trailing}<span class="item-trailing">{@render trailing()}</span>{/if}
		{#if actions}<span class="item-actions">{@render actions()}</span>{/if}
	</div>
	{#if children}<div class="item-detail">{@render children()}</div>{/if}
</div>

<style>
	.item {
		position: relative;
		min-width: 0;
	}
	.item-row {
		position: relative;
		display: flex;
		align-items: center;
		gap: 10px;
		min-height: 44px;
		padding: 6px 12px 6px 16px;
		transition: background-color var(--motion-state) var(--ease-standard);
	}
	.interactive > .item-row:hover {
		background: var(--surface-hover);
	}
	.selected > .item-row {
		background: var(--surface-alt);
	}
	.selected > .item-row::before {
		content: '';
		position: absolute;
		left: 0;
		top: 8px;
		bottom: 8px;
		width: 2px;
		border-radius: 2px;
		background: var(--intent-conversation);
	}
	.item-hit {
		position: absolute;
		inset: 0;
		z-index: 1;
		padding: 0;
		border: 0;
		background: transparent;
		cursor: pointer;
	}
	.item-hit:focus-visible {
		outline-offset: -2px;
	}
	.item-leading {
		display: grid;
		place-items: center;
		flex: none;
		width: 18px;
		color: var(--text-tertiary);
	}
	.item-main {
		display: flex;
		align-items: baseline;
		gap: 10px;
		flex: 1;
		min-width: 0;
	}
	.item-title {
		flex: 0 1 auto;
		min-width: 0;
		overflow: hidden;
		color: var(--ink);
		font-size: var(--t-body);
		font-weight: 500;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.unread .item-title {
		font-weight: 600;
	}
	.item-meta {
		flex: 1 1 0;
		/* A crowded row shortens the title first; a name never collapses to one letter. */
		min-width: min(7ch, 30%);
		overflow: hidden;
		color: var(--text-tertiary);
		font-size: var(--t-body);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.item-trailing {
		display: flex;
		align-items: center;
		flex: none;
		gap: 10px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-variant-numeric: tabular-nums;
		white-space: nowrap;
	}
	/* Controls sit above the row's link so they stay clickable. */
	.item-actions,
	.item-trailing :global(:is(a, button, details, input, label)) {
		position: relative;
		z-index: 2;
	}
	.item-actions {
		display: flex;
		align-items: center;
		flex: none;
		gap: 4px;
	}
	@media (hover: hover) and (pointer: fine) {
		/* Hidden actions take no room: they float over the row's end on hover, so every row's time
		 * lines up whatever actions it has. */
		.item-actions {
			position: absolute;
			top: 50%;
			right: 8px;
			padding-left: 8px;
			border-radius: var(--radius-control);
			background: var(--surface-hover);
			transform: translateY(-50%);
			opacity: 0;
			/* Unseen is unclickable: a hidden Resume must never take a click meant for the row. */
			pointer-events: none;
			transition: opacity var(--motion-state) var(--ease-standard);
		}
		.selected .item-actions {
			background: var(--surface-alt);
		}
		.item:not(.stacked):has(.item-actions:not(:empty)):is(:hover, :focus-within)
			> .item-row
			> .item-trailing:not(:has(:global(a, button, details, input, label))) {
			visibility: hidden;
		}
		.item:hover .item-actions,
		.item:focus-within .item-actions,
		.item-actions:has(:global(details[open])) {
			opacity: 1;
			pointer-events: auto;
		}
	}
	.dim .item-title,
	.dim .item-leading {
		color: var(--text-tertiary);
	}
	.item-detail {
		padding: 0 16px 14px 44px;
	}
	/* A detail snippet that renders nothing leaves no gap under the row. */
	.item-detail:empty {
		display: none;
	}
	@container page (max-width: 560px) {
		.item-main {
			flex-direction: column;
			gap: 1px;
			align-items: stretch;
		}
		.item-row {
			padding-left: 14px;
		}
		.item-detail {
			padding-left: 14px;
		}
	}
	/* Two-line rows: the title wraps to two lines, who and when sit under it, and the row's
	 * actions take the time's place on hover rather than squeezing the title. */
	.stacked > .item-row {
		align-items: flex-start;
		min-height: 56px;
		padding-top: 10px;
		padding-bottom: 10px;
	}
	.stacked .item-main {
		flex-direction: column;
		align-items: stretch;
		gap: 2px;
	}
	.stacked .item-title {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		line-height: 1.35;
		white-space: normal;
		overflow-wrap: anywhere;
	}
	.stacked .item-meta {
		flex: none;
		min-width: 0;
		font-size: var(--t-label);
	}
	.stacked .item-leading {
		margin-top: 1px;
	}
	.stacked .item-trailing {
		padding-top: 1px;
		font-size: var(--t-label);
	}
	.stacked .item-actions {
		position: absolute;
		top: 6px;
		right: 8px;
		border-radius: var(--radius-control);
		background: var(--surface-hover);
		transform: none;
	}
	@media (hover: hover) and (pointer: fine) {
		.stacked:has(.item-actions:not(:empty)):is(:hover, :focus-within)
			.item-trailing:not(:has(:global(a, button, details, input, label))) {
			visibility: hidden;
		}
	}
	@media not ((hover: hover) and (pointer: fine)) {
		.stacked .item-actions {
			position: static;
			background: transparent;
		}
	}
</style>
