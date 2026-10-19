<script lang="ts">
	/* An agent's question answered inside its Inbox card (S101): the last few messages for context,
	 * the question, its own choices as one-tap answers, and a reply that goes to the agent as a
	 * message. Exec and every lead ask this way, so any chat that needs you is a card you can
	 * finish here. */
	import { onMount, tick } from 'svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import { getActorConversation, sendActorMessage } from '$lib/model/attention';
	import { failureSentence } from '$lib/model/failure';
	import { quickAnswers, replyBody } from '$lib/model/inline-reply';
	import type { ConversationMessageView } from '$lib/model/generated/conversation';

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
		/** One-tap answers the card knows better than its question's wording. */
		answers?: string[];
		ondone: (receipt: string) => void;
	} = $props();

	let recent = $state<ConversationMessageView[]>([]);
	let answer = $state('');
	let sending = $state(false);
	let error = $state('');
	let field = $state<HTMLTextAreaElement | null>(null);
	const choices = $derived(answers ?? quickAnswers(question));

	onMount(() => {
		void getActorConversation(companyId, actorId, workId ?? undefined)
			.then((view) => (recent = view.messages.slice(-4)))
			.catch(() => (recent = []));
	});

	async function pick(choice: string) {
		answer = choice;
		await tick();
		field?.focus();
	}

	async function send(event?: SubmitEvent) {
		event?.preventDefault();
		if (sending || !answer.trim()) return;
		sending = true;
		error = '';
		try {
			await sendActorMessage(companyId, actorId, replyBody(question, answer), workId ?? undefined);
			answer = '';
			ondone(`Sent to ${actorName}. They carry on with your answer.`);
		} catch (cause) {
			error = failureSentence(cause, 'The reply was not sent. Try again.');
		} finally {
			sending = false;
		}
	}
</script>

<section class="inline-reply" aria-label={`Reply to ${actorName}`}>
	{#if recent.length > 1}
		<ol class="thread" aria-label="Recent messages">
			{#each recent.slice(0, -1) as message (message.id)}
				<li class:mine={message.from_actor !== actorId}>
					<span class="who">{message.from_actor === actorId ? actorName : 'You'}</span>
					<RelativeTime value={message.created_at} />
					<div class="text"><Markdown text={message.body.length > 600 ? `${message.body.slice(0, 600)}…` : message.body} /></div>
				</li>
			{/each}
		</ol>
	{/if}
	{#if choices.length}
		<div class="choices" aria-label="Answers to choose">
			{#each choices as choice (choice)}
				<button type="button" class:on={answer === choice} onclick={() => void pick(choice)}>{choice}</button>
			{/each}
		</div>
	{/if}
	<form onsubmit={send}>
		<label class="sr-only" for={`reply-${actorId}`}>Your reply to {actorName}</label>
		<textarea
			id={`reply-${actorId}`}
			bind:this={field}
			bind:value={answer}
			rows="3"
			placeholder={`Reply to ${actorName}…`}
			onkeydown={(event) => {
				if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
					event.preventDefault();
					void send();
				}
			}}
		></textarea>
		<div class="foot">
			<span class="hint">⌘↵ to send · goes to {actorName} as a message</span>
			<button class="btn small primary" disabled={sending || !answer.trim()}>{sending ? 'Sending…' : 'Send'}</button>
		</div>
	</form>
	{#if error}<p class="error" role="alert">{error}</p>{/if}
</section>

<style>
	.inline-reply {
		display: grid;
		gap: 10px;
		margin-top: 12px;
	}
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
	.choices {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}
	.choices button {
		max-width: 100%;
		padding: 5px 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--ink);
		font-size: var(--t-label);
		text-align: left;
		cursor: pointer;
	}
	.choices button.on,
	.choices button:hover {
		border-color: var(--text-secondary);
	}
	form {
		display: grid;
		gap: 6px;
	}
	textarea {
		width: 100%;
		resize: vertical;
	}
	.foot {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
	}
	.hint {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.error {
		margin: 0;
		color: var(--state-danger);
		font-size: var(--t-label);
	}
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
	}
</style>
