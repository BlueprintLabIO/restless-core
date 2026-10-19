<script lang="ts">
	/* Talking to anyone from their Inbox card (S101): the People tab's own conversation (the
	 * thread, threads, files, the composer) embedded in the card, so a question from Exec, a lead
	 * or a teammate is answered where it is asked. The answers the asker offers are one tap in the
	 * footer; the facts they ask for are fields above the conversation, sent as one reply; anything
	 * else is typed in the conversation. */
	import RoomConversation from '$lib/components/RoomConversation.svelte';
	import { sendActorMessage } from '$lib/model/attention';
	import { failureSentence } from '$lib/model/failure';
	import { quickAnswers, replyBody } from '$lib/model/inline-reply';
	import InlinePanel from './InlinePanel.svelte';
	import { showingConversation } from '$lib/model/embedded-conversations.svelte';
	import { cardActions, type CardAction } from './card-actions.svelte';

	let {
		companyId,
		actorId,
		actorName,
		question = '',
		workId,
		answers,
		fields = [],
		compact = false,
		ondone
	}: {
		companyId: string;
		actorId: string;
		actorName: string;
		/** What they asked, as the card states it; its choices become one-tap answers. */
		question?: string;
		workId?: string | null;
		/** The answers the asker offers, or the card knows better than the question's wording. */
		answers?: string[];
		/** The facts the asker wants, a field each. */
		fields?: { id: string; label: string }[];
		/** Shorter, beside a computer the card also shows. */
		compact?: boolean;
		/** A reply from the card went: the card shows the receipt. */
		ondone?: (receipt: string) => void;
	} = $props();

	/* The rail steps aside while this card shows the same conversation. */
	$effect(() => showingConversation(actorId));
	const choices = $derived(answers?.length ? answers : question ? quickAnswers(question) : []);
	let values = $state<Record<string, string>>({});
	let error = $state('');
	const filled = $derived(fields.every((field) => (values[field.id] ?? '').trim()));

	async function send(text: string, receipt: string) {
		error = '';
		try {
			await sendActorMessage(companyId, actorId, replyBody(question, text), workId ?? undefined);
			ondone?.(receipt);
		} catch (cause) {
			error = failureSentence(cause, 'The reply was not sent. Try again.');
		}
	}

	const answer = (text: string): CardAction => ({
		label: text,
		busyLabel: 'Sending…',
		title: `Sends “${text}” to ${actorName}`,
		run: () => send(text, `Sent to ${actorName}: “${text}”. They carry on with your answer.`)
	});

	cardActions(() => {
		if (!ondone) return {};
		if (fields.length)
			return {
				secondary: choices.map(answer),
				primary: {
					label: `Send to ${actorName}`,
					busyLabel: 'Sending…',
					disabled: !filled,
					run: () =>
						send(
							fields.map((field) => `${field.label}: ${(values[field.id] ?? '').trim()}`).join('\n'),
							`Sent to ${actorName}. They carry on with your answer.`
						)
				}
			};
		return choices.length ? { secondary: choices.map(answer) } : {};
	});
</script>

{#if fields.length && ondone}
	<InlinePanel label={`What ${actorName} asks for`} {error}>
		<div class="pair">
			{#each fields as field (field.id)}
				<label class="field">{field.label}<input bind:value={values[field.id]} autocomplete="off" /></label>
			{/each}
		</div>
	</InlinePanel>
{/if}
<div class="inline-conversation" class:compact aria-label={`Conversation with ${actorName}`}>
	<RoomConversation person={actorId} embedded />
</div>
{#if error && !fields.length}<p class="error" role="alert">{error}</p>{/if}

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
	.inline-conversation.compact {
		height: min(380px, 45dvh);
		min-height: 280px;
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
