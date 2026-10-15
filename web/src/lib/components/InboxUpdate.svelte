<script lang="ts">
	/* One update in the shared Inbox queue. Its server source owns read/archive state. */
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { Item } from '$lib/ui/page';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { forYouSentence, type ForYouItem } from '$lib/model/for-you';
	import type { ForYouSource } from '$lib/model/for-you.svelte';
	import AppliedChanges from './AppliedChanges.svelte';
	import { targetHref } from '$lib/model/direction';
	import Bell from '@lucide/svelte/icons/bell';
	let { companyId, source, item }: { companyId: string; source: ForYouSource; item: ForYouItem } =
		$props();
	function fail(cause: unknown, fallback: string) {
		toast(failureSentence(cause, fallback), { tone: 'failure' });
	}
	function open(item: ForYouItem) {
		if (item.unread) void source.read([item.key]).catch((cause) => fail(cause, 'Not marked read.'));
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
			toast('Moved back to Inbox');
		} catch (cause) {
			fail(cause, 'Not moved back.');
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
			fail(cause, 'Not archived.');
		}
	}
</script>

<Item
	role="listitem"
	stacked
	title={forYouSentence(item)}
	meta={item.kind === 'applied' ? null : item.excerpt || null}
	href={item.kind === 'applied'
		? item.direction_applied?.length === 1
			? (targetHref(
					companyId,
					item.direction_applied[0].target,
					item.direction_applied[0].applied_at
				) ?? undefined)
			: undefined
		: item.href}
	unread={item.unread}
	onclick={() => open(item)}
>
	{#snippet leading()}<span class="inbox-update-mark" title={item.kind}
			><Bell size={15} strokeWidth={1.8} aria-hidden="true" /></span
		>{/snippet}
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
{#if item.kind === 'applied' && item.direction_applied?.length}<AppliedChanges
		{companyId}
		changes={item.direction_applied}
		onchanged={() => source.refresh()}
	/>{/if}

<style>
	:global(.inbox-update-mark) {
		display: grid;
		color: var(--intent-conversation);
	}
	kbd {
		margin-left: auto;
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-mono);
	}
</style>
