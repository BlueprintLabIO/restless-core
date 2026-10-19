<script lang="ts">
	/* An agent's question answered in its card (S101): the last few messages for context on top,
	 * the question's own choices below with a way to write your own. The footer sends the answer
	 * to the agent as a message. Exec and every lead ask this way, so any chat that needs you is
	 * a card you finish here. */
	import { onMount, tick } from 'svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import { getActorConversation, sendActorMessage } from '$lib/model/attention';
	import { failureSentence } from '$lib/model/failure';
	import { quickAnswers, replyBody } from '$lib/model/inline-reply';
	import type { ConversationMessageView } from '$lib/model/generated/conversation';
	import InlinePanel from './InlinePanel.svelte';
	import InlineChoice from './InlineChoice.svelte';
	import { cardActions } from './card-actions.svelte';

	let {
		companyId,
		actorId,
		actorName,
		question,
		workId,
		answers,
		ondone
	}: {
		companyId: string;
		actorId: string;
		actorName: string;
		/** What the agent asked, as its card states it. */
		question: string;
		workId?: string | null;
		/** Answers the card knows better than its question's wording. */
		answers?: string[];
		ondone: (receipt: string) => void;
	} = $props();

	const OWN = '__own';
	let recent = $state<ConversationMessageView[]>([]);
	let own = $state('');
	let error = $state('');
	let field = $state<HTMLTextAreaElement | null>(null);
	const choices = $derived(answers ?? quickAnswers(question));
	let choice = $state<string | null>(null);
	$effect(() => {
		if (choice === null) choice = choices.length ? null : OWN;
	});
	$effect(() => {
		if (choice === OWN && choices.length) void tick().then(() => field?.focus());
	});
	const answer = $derived(choice === OWN ? own.trim() : (choice ?? ''));

	onMount(() => {
		void getActorConversation(companyId, actorId, workId ?? undefined)
			.then((view) => (recent = view.messages.slice(-4)))
			.catch(() => (recent = []));
	});

	async function send() {
		if (!answer) return;
		error = '';
		try {
			await sendActorMessage(companyId, actorId, replyBody(question, answer), workId ?? undefined);
			own = '';
			ondone(`Sent to ${actorName}. They carry on with your answer.`);
		} catch (cause) {
			error = failureSentence(cause, 'The reply was not sent. Try again.');
		}
	}

	cardActions(() => ({
		primary: { label: `Send to ${actorName}`, busyLabel: 'Sending…', disabled: !answer, run: send }
	}));
</script>

{#snippet thread()}
	<ol class="thread" aria-label="Recent messages">
		{#each recent.slice(0, -1) as message (message.id)}
			<li class:mine={message.from_actor !== actorId}>
				<span class="who">{message.from_actor === actorId ? actorName : 'You'}</span>
				<RelativeTime value={message.created_at} />
				<div class="text"><Markdown text={message.body.length > 600 ? `${message.body.slice(0, 600)}…` : message.body} /></div>
			</li>
		{/each}
	</ol>
{/snippet}

<InlinePanel label={`Reply to ${actorName}`} {error} status={recent.length > 1 ? thread : undefined}>
	{#if choices.length}
		<InlineChoice
			label="Your answer"
			options={[
				...choices.map((text) => ({ value: text, title: text })),
				{ value: OWN, title: 'Write my own' }
			]}
			bind:value={choice}
		/>
	{/if}
	{#if choice === OWN}
		<label class="sr-only" for={`reply-${actorId}`}>Your reply to {actorName}</label>
		<textarea
			id={`reply-${actorId}`}
			bind:this={field}
			bind:value={own}
			rows="3"
			placeholder={`Reply to ${actorName}…`}
			onkeydown={(event) => {
				if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
					event.preventDefault();
					void send();
				}
			}}
		></textarea>
	{/if}
</InlinePanel>

<style>
	.thread {
		display: grid;
		gap: 8px;
		margin: 0;
		padding: 0 0 0 10px;
		border-left: 2px solid var(--border);
		list-style: none;
	}
	.thread li {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 2px 8px;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.who {
		font-weight: 600;
		color: var(--text-secondary);
	}
	.thread li.mine .who {
		color: var(--ink);
	}
	.thread .text {
		grid-column: 1 / -1;
		max-height: 7.5em;
		overflow: hidden;
	}
	textarea {
		width: 100%;
		resize: vertical;
	}
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
	}
</style>
