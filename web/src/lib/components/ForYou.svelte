<script lang="ts">
	/* For you: where someone mentioned, replied to or commented to this person, and what others
	 * changed in documents they follow. One row each, newest first, unread in ink. Opening a row
	 * reads it on every device. */
	import CheckCheck from '@lucide/svelte/icons/check-check';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import { Item } from '$lib/ui/page';
	import { failureSentence } from '$lib/model/failure';
	import { forYouSentence, type ForYouItem } from '$lib/model/for-you';
	import type { ForYouSource } from '$lib/model/for-you.svelte';

	let {
		source,
		heading = 'For you'
	}: {
		source: ForYouSource;
		heading?: string;
	} = $props();

	let failure = $state('');

	function open(item: ForYouItem) {
		failure = '';
		if (item.unread) void source.read([item.key]).catch(() => undefined);
	}
	async function readAll() {
		failure = '';
		try {
			await source.read('all');
		} catch (cause) {
			failure = failureSentence(cause, 'Not marked read. Try again.');
		}
	}
</script>

<section class="for-you" aria-label={heading}>
	<header>
		<h3>{heading}</h3>
		{#if source.unreadCount}<span class="count" title="Unread">{source.unreadCount}</span>{/if}
		<span class="spacer"></span>
		{#if source.unreadCount}
			<button
				type="button"
				class="btn small ghost icon"
				aria-label="Mark all read"
				title="Mark all read"
				onclick={() => void readAll()}><CheckCheck size={15} strokeWidth={1.8} /></button
			>
		{/if}
	</header>
	{#if failure}<p class="note" role="alert">{failure}</p>{/if}
	{#each source.items as item (item.key)}
		<Item
			title={forYouSentence(item)}
			meta={item.excerpt || null}
			href={item.href}
			unread={item.unread}
			onclick={() => open(item)}
		>
			{#snippet trailing()}<RelativeTime value={item.at} />{/snippet}
		</Item>
	{:else}
		{#if source.status === 'unknown' && source.failure}
			<p class="note">For you could not be read. It tries again shortly.</p>
		{:else if source.status !== 'unknown'}
			<p class="note">
				When someone mentions you, replies to you, or changes a document you follow, it shows here.
			</p>
		{/if}
	{/each}
	{#if source.hasMore}
		<button
			type="button"
			class="btn small ghost more"
			disabled={source.loadingMore}
			onclick={() => source.loadMore()}>{source.loadingMore ? 'Loading…' : 'Show earlier'}</button
		>
	{/if}
</section>

<style>
	.for-you {
		display: grid;
		min-width: 0;
	}
	header {
		display: flex;
		align-items: center;
		gap: 8px;
		min-height: 36px;
		padding: 4px 8px 4px 16px;
	}
	h3 {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
		font-weight: 600;
	}
	.count {
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-variant-numeric: tabular-nums;
	}
	.spacer {
		flex: 1;
	}
	.note {
		margin: 0;
		padding: 8px 16px 12px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.more {
		justify-self: start;
		margin: 4px 8px 8px;
	}
</style>
