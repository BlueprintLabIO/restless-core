<script lang="ts">
	/* Answering anyone from their Inbox card (S101). The conversation itself is the rail beside
	 * the cockpit, opened on that person while the card is up, so it stays where it always is.
	 * The card holds only the answer: what the asker offers is one tap in the footer, and the facts
	 * they ask for are fields here, sent with the chosen answer as one reply. Anything else is
	 * typed in the rail. */
	import { sendActorMessage } from '$lib/model/attention';
	import { failureSentence } from '$lib/model/failure';
	import { quickAnswers, replyBody } from '$lib/model/inline-reply';
	import InlinePanel from './InlinePanel.svelte';
	import InlineChoice from './InlineChoice.svelte';
	import Field from '$lib/ui/forms/Field.svelte';
	import { askForConversation } from '$lib/model/embedded-conversations.svelte';
	import { cardActions, type CardAction } from './card-actions.svelte';

	let {
		companyId,
		actorId,
		actorName,
		question = '',
		workId,
		answers,
		fields = [],
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
		/** A reply from the card went: the card shows the receipt. */
		ondone?: (receipt: string) => void;
	} = $props();

	/* The rail opens on this person while the card is up. */
	$effect(() => askForConversation(actorId));
	const choices = $derived(answers?.length ? answers : question ? quickAnswers(question) : []);
	let values = $state<Record<string, string>>({});
	let error = $state('');
	/* With facts to fill in, the offered answers are a choice beside them and one Send carries
	 * both, so "Tuesday" never goes without the supplier's name. */
	let picked = $state<string | null>(null);
	const filled = $derived(
		fields.every((field) => (values[field.id] ?? '').trim()) && (!choices.length || !!picked)
	);

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
				primary: {
					label: `Send to ${actorName}`,
					busyLabel: 'Sending…',
					disabled: !filled,
					run: () =>
						send(
							[
								...(picked ? [picked] : []),
								...fields.map((field) => `${field.label}: ${(values[field.id] ?? '').trim()}`)
							].join('\n'),
							`Sent to ${actorName}. They carry on with your answer.`
						)
				}
			};
		return choices.length ? { secondary: choices.map(answer) } : {};
	});
</script>

{#if fields.length && ondone}
	<InlinePanel label={`What ${actorName} asks for`} {error} form>
		{#if choices.length}
			<Field label="Your answer">
				<InlineChoice
					label="Your answer"
					options={choices.map((text) => ({ value: text, title: text }))}
					bind:value={picked}
				/>
			</Field>
		{/if}
		{#each fields as field (field.id)}
			<Field label={field.label}>
				<input bind:value={values[field.id]} aria-label={field.label} autocomplete="off" />
			</Field>
		{/each}
	</InlinePanel>
{:else if error}<p class="error" role="alert">{error}</p>{/if}

<style>
	.error {
		margin: 6px 0 0;
		color: var(--state-danger);
		font-size: var(--t-label);
	}
</style>
