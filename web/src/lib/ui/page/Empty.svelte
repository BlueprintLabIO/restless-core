<script lang="ts">
	/* Nothing here yet: one quiet line and, when there is one, the action that
	 * changes that. The explanation, if any, is a tooltip. */
	import type { Component, Snippet } from 'svelte';
	import Inbox from '@lucide/svelte/icons/inbox';

	let {
		title,
		info,
		icon = Inbox,
		compact = false,
		action
	}: {
		title: string;
		info?: string;
		icon?: Component<{ size?: number; strokeWidth?: number }>;
		/** Inside a section card rather than filling a page. */
		compact?: boolean;
		action?: Snippet;
	} = $props();
	const Icon = $derived(icon);
</script>

<div class="empty" class:compact title={info}>
	<span class="empty-icon" aria-hidden="true"
		><Icon size={16} strokeWidth={1.6} /></span
	>
	<p>{title}</p>
	{#if action}<div class="empty-action">{@render action()}</div>{/if}
</div>

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
</style>
