<script lang="ts">
	/* One row in any list: Inbox, Work, Library, schedules, skills, activity.
	 *
	 * Leading mark, a single-line title, quiet inline meta, and a trailing area
	 * for state and time. Row actions appear on hover and focus on a pointer
	 * device and stay visible on touch, so nothing is hover-only. */
	import type { Snippet } from 'svelte';

	let {
		title,
		meta,
		href,
		onclick,
		selected = false,
		unread = false,
		dim = false,
		leading,
		trailing,
		actions,
		children
	}: {
		title: string;
		meta?: string | null;
		href?: string;
		onclick?: (event: MouseEvent) => void;
		selected?: boolean;
		unread?: boolean;
		/** Finished or archived: present but quiet. */
		dim?: boolean;
		leading?: Snippet;
		trailing?: Snippet;
		actions?: Snippet;
		/** Optional detail under the row, such as an expanded editor. */
		children?: Snippet;
	} = $props();
</script>

<div class="item" class:selected class:unread class:dim class:interactive={!!href || !!onclick}>
	<div class="item-row">
		{#if href}<a class="item-hit" {href} aria-current={selected ? 'true' : undefined} {onclick}
				><span class="sr-only">{title}</span></a
			>{:else if onclick}<button class="item-hit" type="button" {onclick}
				><span class="sr-only">{title}</span></button
			>{/if}
		{#if leading}<span class="item-leading">{@render leading()}</span>{/if}
		<span class="item-main">
			<span class="item-title" {title}>{title}</span>
			{#if meta}<span class="item-meta" title={meta}>{meta}</span>{/if}
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
		min-width: 0;
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
		.item-actions {
			opacity: 0;
			transition: opacity var(--motion-state) var(--ease-standard);
		}
		.item:hover .item-actions,
		.item:focus-within .item-actions,
		.item-actions:has(:global(details[open])) {
			opacity: 1;
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
</style>
