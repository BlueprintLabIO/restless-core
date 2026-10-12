<script lang="ts">
	/* For you: where someone mentioned, replied to, shared with or asked this person, and what
	 * others changed in what they follow. One row each, newest first, unread in ink. Opening a row
	 * reads it on every device; a row can be marked unread again or archived until something new
	 * happens there. J/K walk the rows, E reads one, Shift+U marks it unread. */
	import CheckCheck from '@lucide/svelte/icons/check-check';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { Item, Segmented } from '$lib/ui/page';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { forYouSentence, type ForYouFilter, type ForYouItem } from '$lib/model/for-you';
	import type { ForYouSource } from '$lib/model/for-you.svelte';

	let {
		source,
		heading = 'For you',
		onexit
	}: {
		source: ForYouSource;
		heading?: string;
		/** K on the first row leaves the list upwards, to whatever sits above it. */
		onexit?: () => void;
	} = $props();

	let section: HTMLElement;

	function fail(cause: unknown, fallback: string) {
		toast(failureSentence(cause, fallback), { tone: 'failure' });
	}
	function open(item: ForYouItem) {
		if (item.unread) void source.read([item.key]).catch(() => undefined);
	}
	async function readAll() {
		try {
			const was = await source.read('all');
			toast('Marked all read', {
				undo: was.length
					? () => source.unread(was).catch((cause) => fail(cause, 'Not undone.'))
					: undefined
			});
		} catch (cause) {
			fail(cause, 'Not marked read. Try again.');
		}
	}
	async function setRead(item: ForYouItem, read: boolean) {
		try {
			if (read) await source.read([item.key]);
			else await source.unread([item.key]);
		} catch (cause) {
			fail(cause, read ? 'Not marked read.' : 'Not marked unread.');
		}
	}
	async function bringBack(item: ForYouItem) {
		try {
			await source.archive([item.key], false);
			toast('Moved back to For you', {
				undo: () => source.archive([item.key]).catch((cause) => fail(cause, 'Not undone.'))
			});
		} catch (cause) {
			fail(cause, 'Not moved back. Try again.');
		}
	}
	async function archive(item: ForYouItem) {
		try {
			await source.archive([item.key]);
			toast('Archived', {
				undo: () =>
					source.archive([item.key], false).catch((cause) => fail(cause, 'Not brought back.'))
			});
		} catch (cause) {
			fail(cause, 'Not archived. Try again.');
		}
	}

	/** Focuses the first row; the Inbox moves here from its last item. */
	export function focusFirst(): boolean {
		const first = section?.querySelector<HTMLElement>('.item-hit');
		first?.focus();
		return Boolean(first);
	}
	function keyboard(event: KeyboardEvent) {
		if (event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey) return;
		const rows = [...section.querySelectorAll<HTMLElement>('.item-hit')];
		const index = rows.indexOf(document.activeElement as HTMLElement);
		if (index < 0) return;
		const item = source.items[index];
		const step = { j: 1, ArrowDown: 1, k: -1, ArrowUp: -1 }[event.key];
		if (step) {
			event.preventDefault();
			if (index + step < 0) onexit?.();
			else rows[Math.min(rows.length - 1, index + step)]?.focus();
		} else if (item && event.key === 'e') {
			event.preventDefault();
			if (item.unread) void setRead(item, true);
		} else if (item && event.key === 'U' && event.shiftKey) {
			event.preventDefault();
			if (!item.unread) void setRead(item, false);
		}
	}
	const filters: { value: ForYouFilter; label: string; title: string }[] = [
		{ value: 'all', label: 'All', title: 'Everything, newest first' },
		{ value: 'unread', label: 'Unread', title: 'Only what you have not opened' },
		{
			value: 'archived',
			label: 'Archived',
			title: 'What you archived; anything new there brings it back by itself'
		}
	];
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section class="for-you" aria-label={heading} bind:this={section} onkeydown={keyboard}>
	<header>
		<h3>{heading}</h3>
		{#if source.unreadCount}<span class="count" title="Unread">{source.unreadCount}</span>{/if}
		<span class="spacer"></span>
		{#if source.items.length || source.filter !== 'all'}
			<Segmented
				label="Show"
				options={filters}
				value={source.filter}
				onchange={(value) => source.setFilter(value)}
			/>
		{/if}
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
	{#each source.items as item (item.key)}
		<Item
			title={forYouSentence(item)}
			meta={item.excerpt || null}
			href={item.href}
			unread={item.unread}
			onclick={() => open(item)}
		>
			{#snippet trailing()}<RelativeTime value={item.at} />{/snippet}
			{#snippet actions()}
				<ActionMenu label={`Actions for ${forYouSentence(item)}`}>
					{#if item.unread}
						<button type="button" onclick={() => void setRead(item, true)}
							>Mark read <kbd>E</kbd></button
						>
					{:else}
						<button type="button" onclick={() => void setRead(item, false)}
							>Mark unread <kbd>⇧U</kbd></button
						>
					{/if}
					{#if source.filter === 'archived'}
						<button type="button" onclick={() => void bringBack(item)}>Move back</button>
					{:else}
						<button
							type="button"
							title="Off this list until something new happens there"
							onclick={() => void archive(item)}>Archive</button
						>
					{/if}
				</ActionMenu>
			{/snippet}
		</Item>
	{:else}
		{#if source.status === 'unknown' && source.failure}
			<p class="note">For you could not be read. It tries again shortly.</p>
		{:else if source.status !== 'unknown'}
			<p class="note">
				{source.filter === 'unread'
					? 'Nothing unread.'
					: source.filter === 'archived'
						? 'Nothing archived.'
						: 'When someone mentions you, replies to you, shares something with you, or changes what you follow, it shows here.'}
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
	kbd {
		margin-left: auto;
		padding: 0 4px;
		border: 1px solid var(--border-strong);
		border-radius: 3px;
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-mono);
	}
</style>
