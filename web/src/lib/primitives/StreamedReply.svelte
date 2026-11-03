<script lang="ts" module>
	import { replyIsError } from '$lib/model/turn-failure';
	import { turnWorking, type ActiveAgentTurn } from '$lib/model/queries.svelte';

	/** The reply text to draw while a turn streams it, or '' when there is none to draw: nothing
	 * written yet, the saved reply already in the conversation, or a failure's own words. */
	export function streamedReplyText(turn: ActiveAgentTurn | null | undefined): string {
		if (!turn || turn.transport === 'idle' || turn.replyLanded) return '';
		const reply = turn.live?.reply ?? '';
		if (!reply) return '';
		if (turn.live?.phase === 'failed' && replyIsError(reply, turn.live?.error)) return '';
		return reply;
	}
</script>

<script lang="ts">
	/* A reply as it streams, drawn as the message it becomes (S110). The host puts it where the
	 * saved reply will appear, the transcript's last message above the turn's line, so when the
	 * reply is saved it takes this one's place in the same frame: same header, same slot, same
	 * line breaks, and nothing below moves. */
	import ConversationMessage from './ConversationMessage.svelte';

	let {
		participantName,
		turn,
		continued = false,
		embedded = false
	}: {
		participantName: string;
		turn: ActiveAgentTurn;
		/** Follows a message of the same author moments earlier, as the saved reply will. */
		continued?: boolean;
		/** Inside a room, whose rows frame each message. */
		embedded?: boolean;
	} = $props();

	const text = $derived(streamedReplyText(turn));
	/* The header's time is when the reply started; it is saved within the same minute. */
	const startedAt = $derived(turn.live?.startedAt ?? turn.since);
</script>

{#if text}
	<ConversationMessage
		sender="agent"
		author={participantName}
		{text}
		createdAt={startedAt}
		streaming={turnWorking(turn)}
		{continued}
		{embedded}
	/>
{/if}
