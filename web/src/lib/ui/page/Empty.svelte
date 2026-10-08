<script lang="ts">
	/* Nothing here yet. In a section it is one quiet row: the line, and the action that changes it.
	 * When a whole view is empty (`page`) it is Linear's empty view instead: centred a little above
	 * the middle, a soft icon, a short title, one line of what will appear here, and the action. */
	import type { Component, Snippet } from 'svelte';
	import Inbox from '@lucide/svelte/icons/inbox';

	let {
		title,
		info,
		icon = Inbox,
		compact = false,
		page = false,
		action
	}: {
		title: string;
		info?: string;
		icon?: Component<{ size?: number; strokeWidth?: number }>;
		/** Inside a section card rather than filling a page. */
		compact?: boolean;
		/** The whole view is empty: a centred empty view rather than a row. */
		page?: boolean;
		action?: Snippet;
	} = $props();
	const Icon = $derived(icon);
</script>

{#if page}
	<div class="empty-page">
		<span class="page-icon" aria-hidden="true"><Icon size={20} strokeWidth={1.6} /></span>
		<p class="page-title">{title}</p>
		{#if info}<p class="page-info">{info}</p>{/if}
		{#if action}<div class="page-action">{@render action()}</div>{/if}
	</div>
{:else}
	<div class="empty" class:compact title={info}>
		<span class="empty-icon" aria-hidden="true"><Icon size={16} strokeWidth={1.6} /></span>
		<p>{title}</p>
		{#if action}<div class="empty-action">{@render action()}</div>{/if}
	</div>
{/if}

<style>
	/* One quiet row everywhere: the line on the left, the action on the right. A big centred
	 * block made an empty corner of a page louder than its content. */
	.empty {
		display: flex;
		align-items: center;
		gap: 10px;
		min-height: 52px;
		padding: 10px 16px;
		text-align: left;
	}
	.empty-icon {
		display: inline-flex;
		color: var(--text-tertiary);
	}
	p {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.empty-action {
		margin-left: auto;
	}
	.empty-page {
		display: grid;
		justify-items: center;
		align-content: center;
		gap: 6px;
		min-height: min(420px, 60%);
		padding: 48px 24px 64px;
		text-align: center;
	}
	.page-icon {
		display: grid;
		place-items: center;
		width: 40px;
		height: 40px;
		margin-bottom: 6px;
		border: 1px solid var(--border);
		border-radius: 10px;
		background: var(--surface-alt, var(--surface));
		color: var(--text-tertiary);
	}
	.page-title {
		margin: 0;
		color: var(--ink);
		font-size: var(--t-head);
		font-weight: 500;
	}
	.page-info {
		max-width: 44ch;
		margin: 0;
		color: var(--text-tertiary);
		font-size: var(--t-body);
		line-height: 1.5;
	}
	.page-action {
		margin-top: 10px;
	}
</style>
