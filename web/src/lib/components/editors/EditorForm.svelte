<script lang="ts">
	/* A shared editor on its Settings page (S101): the same editor an Inbox card shows, framed as
	 * one of the page's forms. The editor hands its actions to the slot, as it does in a card, and
	 * this footer renders them: a note, the secondary actions, Cancel, then the one primary. Enter
	 * runs the primary. */
	import type { Snippet } from 'svelte';
	import HoldApprove from '$lib/ui/controls/HoldApprove.svelte';
	import { provideCardActions } from '../inbox/card-actions.svelte';

	let {
		title = '',
		framed = false,
		head = undefined,
		note = '',
		oncancel = undefined,
		children
	}: {
		/** The form's head, such as "Model budget"; none when the section already names it. */
		title?: string;
		/** A card of its own, for a form standing alone on the page (Add account). */
		framed?: boolean;
		/** More in the form's head beside the title, such as the kind of account. */
		head?: Snippet;
		note?: string;
		oncancel?: () => void;
		children: Snippet;
	} = $props();

	const slot = provideCardActions();
	const primary = $derived(slot.primary);
</script>

<form
	class="form"
	class:plain={!framed}
	onsubmit={(event) => {
		event.preventDefault();
		if (primary && !primary.hold) void slot.invoke(primary);
	}}
>
	{#if title || head}<header class="form-head">
			{#if title}<strong>{title}</strong>{/if}{@render head?.()}
		</header>{/if}
	{@render children()}
	<footer class="form-foot">
		{#if note}<small class="form-note">{note}</small>{/if}
		{#each slot.secondary as extra (extra.label)}<button
				type="button"
				class="btn small"
				title={extra.title}
				disabled={!!slot.busy || extra.disabled}
				onclick={() => void slot.invoke(extra)}
				>{slot.busy === extra.label ? (extra.busyLabel ?? extra.label) : extra.label}</button
			>{/each}
		{#if oncancel}<button type="button" class="btn small" disabled={!!slot.busy} onclick={oncancel}
				>Cancel</button
			>{/if}
		{#if primary?.hold}<HoldApprove
				small
				label={primary.label}
				title={primary.title}
				disabled={!!slot.busy || primary.disabled}
				onapprove={() => void slot.invoke(primary)}
			/>{:else if primary}<button
				class="btn small primary"
				title={primary.title}
				disabled={!!slot.busy || primary.disabled}
				aria-busy={slot.busy === primary.label}
				>{slot.busy === primary.label
					? (primary.busyLabel ?? primary.label)
					: primary.label}</button
			>{/if}
	</footer>
</form>
