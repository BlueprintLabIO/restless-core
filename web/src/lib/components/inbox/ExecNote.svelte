<script lang="ts">
	/* What Exec did to this item from the owner's chat: one line under the title, with the owner's
	 * words a click away and Undo while its window is open. It arrives with the item, so the card
	 * renders once in its final form. */
	import { useQueryClient } from '@tanstack/svelte-query';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { refreshAttention } from '$lib/model/queries.svelte';
	import {
		execNoteLead,
		ownerSaid,
		showChatMessage,
		undoExecAction,
		type ExecInboxNote
	} from '$lib/model/exec-inbox';

	let { companyId, note }: { companyId: string; note: ExecInboxNote } = $props();

	const client = useQueryClient();
	let undoing = $state(false);

	async function undo() {
		if (undoing) return;
		undoing = true;
		try {
			await undoExecAction(companyId, note.id);
			toast('Undone. The item is as it was.');
		} catch (cause) {
			toast(failureSentence(cause, 'That was not undone.'), { tone: 'failure' });
		} finally {
			undoing = false;
			void refreshAttention(client, companyId);
		}
	}
</script>

<p class="exec-note" class:failed={note.state === 'failed'}>
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
			onclick={undo}>Undo</button
		>{/if}
</p>

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
