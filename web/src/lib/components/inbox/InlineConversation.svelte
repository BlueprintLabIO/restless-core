<script lang="ts">
	/* Talking to anyone from their Inbox card (S101): the People tab's own conversation (the
	 * thread, threads, files, the composer) embedded in the card, so a question from Exec, a lead
	 * or a teammate is answered where it is asked. Answers the question itself offers are one tap
	 * in the footer; anything else is typed in the conversation. */
	import RoomConversation from '$lib/components/RoomConversation.svelte';
	import { sendActorMessage } from '$lib/model/attention';
	import { failureSentence } from '$lib/model/failure';
	import { quickAnswers, replyBody } from '$lib/model/inline-reply';
	import { cardActions } from './card-actions.svelte';

	let {
		companyId,
		actorId,
		actorName,
		question = '',
		workId,
		answers,
		ondone
	}: {
		companyId: string;
		actorId: string;
		actorName: string;
		/** What they asked, as the card states it; its choices become one-tap answers. */
		question?: string;
		workId?: string | null;
		/** Answers the card knows better than its question's wording. */
		answers?: string[];
		/** A one-tap answer went: the card shows the receipt. */
		ondone?: (receipt: string) => void;
	} = $props();

	const choices = $derived(answers ?? (question ? quickAnswers(question) : []));
	let error = $state('');

	cardActions(() =>
		choices.length && ondone
			? { secondary: choices.map((answer) => send(answer)) }
			: {}
	);

	function send(answer: string) {
		return {
			label: answer,
			busyLabel: 'Sending…',
			title: `Sends “${answer}” to ${actorName}`,
			run: async () => {
				error = '';
				try {
					await sendActorMessage(companyId, actorId, replyBody(question, answer), workId ?? undefined);
					ondone?.(`Sent to ${actorName}: “${answer}”. They carry on with your answer.`);
				} catch (cause) {
					error = failureSentence(cause, 'The reply was not sent. Try again.');
				}
			}
		};
	}
</script>

<div class="inline-conversation" aria-label={`Conversation with ${actorName}`}>
	<RoomConversation person={actorId} embedded />
</div>
{#if error}<p class="error" role="alert">{error}</p>{/if}

<style>
	.inline-conversation {
		display: flex;
		height: min(560px, 62dvh);
		min-height: 360px;
		margin-top: 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		overflow: hidden;
	}
	.inline-conversation > :global(*) {
		flex: 1;
		min-width: 0;
	}
	.error {
		margin: 6px 0 0;
		color: var(--state-danger);
		font-size: var(--t-label);
	}
</style>
