<script lang="ts">
	import MessageCircle from '@lucide/svelte/icons/message-circle';
	import SemanticMark from '$lib/primitives/SemanticMark.svelte';
	import type { RoomMention, RoomMessage as RoomMessageRecord } from '$lib/model/rooms';

	let {
		message,
		author,
		isYou = false,
		isAgent = false,
		mentions = [],
		thread = false,
		onthread = null
	}: {
		message: RoomMessageRecord;
		author: string;
		isYou?: boolean;
		isAgent?: boolean;
		mentions?: RoomMention[];
		thread?: boolean;
		onthread?: (() => void) | null;
	} = $props();

	const time = $derived.by(() => {
		const value = new Date(message.created_at);
		return Number.isNaN(value.getTime())
			? ''
			: value.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' }).toLowerCase();
	});
</script>

<article class="room-message" class:you={isYou} class:thread>
	<header>
		<SemanticMark
			meaning={isYou ? 'direction' : isAgent ? 'executive' : 'people'}
			size="small"
			label={`${author} message`}
		/>
		<strong>{author}</strong>
		{#if time}<time datetime={message.created_at}>{time}</time>{/if}
		{#if mentions.length}
			<span class="mention-receipt">
				{mentions.map((mention) => `@${mention.mentioned_actor_id}`).join(', ')}
			</span>
		{/if}
	</header>
	<p>{message.body}</p>
	{#if onthread}
		<button type="button" class="thread-action" onclick={() => onthread?.()}>
			<MessageCircle size={13} strokeWidth={2} aria-hidden="true" />
			Reply
		</button>
	{/if}
</article>

<style>
	.room-message {
		position: relative;
		min-width: 0;
		padding: 13px 16px 11px;
		border-bottom: 1px solid var(--border);
		background: var(--chat-agent-bg);
	}

	.room-message.you {
		background: var(--chat-owner-bg);
		box-shadow: inset 2px 0 0 var(--chat-owner-edge);
	}

	.room-message.thread {
		padding-inline: 13px;
	}

	header {
		display: flex;
		align-items: center;
		gap: 7px;
		min-width: 0;
	}

	header strong {
		min-width: 0;
		overflow: hidden;
		font-size: var(--t-label);
		font-weight: 600;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	time,
	.mention-receipt {
		flex: 0 0 auto;
		font: 500 var(--t-label) var(--font-mono);
		color: var(--text-tertiary);
	}

	.mention-receipt {
		margin-left: auto;
		padding: 2px 5px;
		border: 1px solid color-mix(in srgb, var(--intent-direction) 24%, var(--border));
		border-radius: var(--radius-control);
		background: var(--intent-direction-soft);
		color: var(--intent-direction);
	}

	p {
		margin: 7px 0 0 31px;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		font-size: var(--t-body);
		line-height: 1.55;
		color: var(--ink);
	}

	.thread-action {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		margin: 8px 0 0 31px;
		padding: 3px 6px;
		border: 1px solid transparent;
		border-radius: var(--radius-control);
		background: transparent;
		font-size: var(--t-label);
		font-weight: 500;
		color: var(--intent-conversation);
		cursor: pointer;
		transition:
			background var(--motion-state) var(--ease-standard),
			border-color var(--motion-state) var(--ease-standard),
			transform var(--motion-press) var(--ease-out);
	}

	.thread-action:hover,
	.thread-action:focus-visible {
		border-color: color-mix(in srgb, var(--intent-conversation) 24%, var(--border));
		background: var(--intent-conversation-soft);
	}

	.thread-action:active {
		transform: translateY(1px);
	}

	.thread-action:focus-visible {
		outline: 2px solid color-mix(in srgb, var(--intent-conversation) 32%, transparent);
		outline-offset: 2px;
	}
</style>
