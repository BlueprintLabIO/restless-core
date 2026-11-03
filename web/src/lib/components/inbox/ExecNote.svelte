<script lang="ts">
	/* What Exec did to this item from the owner's chat: one line under the title, with the owner's
	 * words a click away and Undo while its window is open. The note normally arrives with the
	 * item; a change Exec makes while the card is open shows from the small read of Exec's recent
	 * changes, so the owner can still undo it inside its window. */
	import { createQuery, useQueryClient } from '@tanstack/svelte-query';
	import { pollEvery } from '$lib/model/connection.svelte';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import TimeLeft from '$lib/ui/feedback/TimeLeft.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { refreshAttention } from '$lib/model/queries.svelte';
	import {
		execActionsKey,
		execNoteLead,
		localDeadline,
		ownerSaid,
		readExecActions,
		showChatMessage,
		undoExecAction,
		type ExecInboxNote
	} from '$lib/model/exec-inbox';
	import type { AttentionItem } from '$lib/model/view';

	let {
		companyId,
		item,
		owner,
		flush = false
	}: {
		companyId: string;
		item: AttentionItem;
		owner: boolean;
		/** Inside a surface that sets its own inset (a request, a charter draft). */
		flush?: boolean;
	} = $props();

	const client = useQueryClient();
	const recent = createQuery(() => ({
		queryKey: execActionsKey(companyId),
		queryFn: ({ signal }) => readExecActions(companyId, signal),
		enabled: owner && Boolean(companyId),
		staleTime: 10_000,
		refetchInterval: pollEvery(30_000)
	}));
	/* The newest of: what came with the item, or a change still waiting or refused since. What
	 * came with the item takes the state the small read has for it since (applied, undone). */
	const note = $derived.by((): ExecInboxNote | undefined => {
		const recentItems = recent.data?.items ?? [];
		const live = recentItems.find(
			(action) =>
				action.item_id === item.id && (action.state === 'pending' || action.state === 'failed')
		);
		const since =
			item.execAction && recentItems.find((action) => action.id === item.execAction?.id);
		const carried = since ? { ...item.execAction!, state: since.state } : item.execAction;
		if (live && (!carried || Date.parse(live.at) >= Date.parse(carried.at))) return live;
		if (carried?.state === 'canceled') return undefined;
		/* A decision that has landed leaves the card with the item; until then it says what Exec did. */
		return carried;
	});
	/* When Undo closes on this browser's clock: the small read's deadline is the current one (the
	 * owner's next message shortens it), corrected for clock skew; what came with the item is the
	 * fallback. */
	const deadline = $derived.by((): number | undefined => {
		if (note?.state !== 'pending') return undefined;
		const read = recent.data;
		const current = read?.items.find((action) => action.id === note.id);
		if (read && current?.undo_until) return localDeadline(current.undo_until, read);
		return note.undo_until ? Date.parse(note.undo_until) : undefined;
	});
	let undoing = $state(false);

	async function undo(id: string) {
		if (undoing) return;
		undoing = true;
		try {
			await undoExecAction(companyId, id);
			toast('Undone. The item is as it was.');
		} catch (cause) {
			toast(failureSentence(cause, 'That was not undone.'), { tone: 'failure' });
		} finally {
			undoing = false;
			void client.invalidateQueries({ queryKey: execActionsKey(companyId) });
			void refreshAttention(client, companyId);
		}
	}
</script>

{#if note}<p class="exec-note" class:flush class:failed={note.state === 'failed'}>
		<span class="lead">{execNoteLead(note)}</span>
		<span>from</span>
		<button
			type="button"
			class="link"
			title={ownerSaid(note.owner_words)}
			onclick={() => showChatMessage(note.source_message_id)}>your chat</button
		>{#if note.state === 'failed' && note.problem}<span class="problem" title={note.problem}
				>: {note.problem}</span
			>{/if}{#if note.state === 'pending'}<span aria-hidden="true">·</span><button
				type="button"
				class="link"
				disabled={undoing}
				onclick={() => undo(note.id)}>Undo</button
			>{#if deadline !== undefined}<span aria-hidden="true">·</span><TimeLeft
					until={deadline}
				/>{/if}{/if}
	</p>{/if}

<style>
	.exec-note {
		display: flex;
		align-items: baseline;
		gap: 4px;
		min-width: 0;
		margin: 0;
		padding: 6px 24px 0;
		overflow: hidden;
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.4;
		white-space: nowrap;
	}
	.exec-note.flush {
		padding: 4px 0 0;
	}
	/* The same inset as the card's title on a narrow pane. */
	@container page (max-width: 560px) {
		.exec-note:not(.flush) {
			padding-inline: 14px;
		}
	}
	.exec-note > * {
		flex: none;
	}
	.exec-note .problem {
		flex: 0 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.exec-note.failed .lead {
		color: var(--state-danger);
	}
	.link {
		padding: 0;
		border: 0;
		background: none;
		color: var(--ink);
		font: inherit;
		text-decoration: underline;
		text-underline-offset: 2px;
		cursor: pointer;
	}
	.link:disabled {
		cursor: default;
		opacity: 0.6;
	}
	.link:hover:not(:disabled) {
		color: var(--accent-strong);
	}
</style>
