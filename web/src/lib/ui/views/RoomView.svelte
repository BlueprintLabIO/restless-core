<script lang="ts" module>
	import type { ActorKind } from '../glyph/ActorTag.svelte';

	export type RoomSummary = { id: string; name: string; kind: 'group' | 'direct'; unread?: number };
	export type RoomParticipant = { name: string; kind: ActorKind };
	export type RoomMessage = {
		id: string;
		author: string;
		kind: ActorKind;
		time: string;
		text: string;
		/** The message this one answers. */
		replyTo?: string;
	};
</script>

<script lang="ts">
	import AtSign from '@lucide/svelte/icons/at-sign';
	import MessageCircle from '@lucide/svelte/icons/message-circle';
	import Mic from '@lucide/svelte/icons/mic';
	import Search from '@lucide/svelte/icons/search';
	import SendHorizontal from '@lucide/svelte/icons/send-horizontal';
	import Users from '@lucide/svelte/icons/users';
	import Wifi from '@lucide/svelte/icons/wifi';
	import ActorTag from '../glyph/ActorTag.svelte';
	import { listIn } from '../motion';

	/* A room as the owner workspace shows it: the list of rooms, an explicit audience of people and
	 * agents, the conversation and the composer. Read-only; add a message to the array to post it. */
	let {
		name,
		participants,
		messages,
		rooms = [],
		activeRoom,
		typing = null,
		live = true,
		draft = '',
		showList = true
	}: {
		name: string;
		participants: RoomParticipant[];
		messages: RoomMessage[];
		rooms?: RoomSummary[];
		activeRoom?: string;
		/** Who is writing right now, if anyone. */
		typing?: RoomParticipant | null;
		live?: boolean;
		/** Text sitting in the composer. */
		draft?: string;
		showList?: boolean;
	} = $props();

	const byId = $derived(new Map(messages.map((message) => [message.id, message])));
	const summary = $derived(
		participants.length > 2
			? `${participants.slice(0, 2).map((person) => person.name).join(', ')} and ${participants.length - 2} more`
			: participants.map((person) => person.name).join(' and ')
	);
</script>

<div class="view-shell">
	<div class="room-view" class:no-list={!showList || !rooms.length}>
		{#if showList && rooms.length}
			<nav class="room-list" aria-label="Rooms">
				<p class="room-list-head">Rooms</p>
				{#each rooms as room (room.id)}
					<span class="room-entry" class:active={room.id === activeRoom}>
						<span class="room-icon">{#if room.kind === 'group'}<Users size={14} />{:else}<MessageCircle size={14} />{/if}</span>
						<span class="room-name"><strong>{room.name}</strong><span>{room.kind === 'group' ? 'Group room' : 'Direct room'}</span></span>
						{#if room.unread}<b>{room.unread}</b>{/if}
					</span>
				{/each}
			</nav>
		{/if}

		<section class="room-main" aria-label={name}>
			<header class="room-head">
				<div>
					<strong>{name}</strong>
					<span class="room-who">{summary}</span>
				</div>
				<span class="room-tags" aria-label="Who is in this room">
					{#each participants as person (person.name)}<span>{person.name} <ActorTag kind={person.kind} /></span>{/each}
				</span>
				<span class="room-tools" aria-hidden="true"><Search size={15} /></span>
				{#if live}<span class="room-live"><Wifi size={12} strokeWidth={2.4} /> Live</span>{/if}
			</header>
			<ol class="room-log">
				{#each messages as message (message.id)}
					{@const answer = message.replyTo ? byId.get(message.replyTo) : null}
					<li class="room-message" class:agent={message.kind === 'agent'} in:listIn>
						<p class="room-meta"><b>{message.author}</b> <ActorTag kind={message.kind} /> <time>{message.time}</time></p>
						{#if answer}<p class="room-reply">↳ {answer.author}: {answer.text}</p>{/if}
						<p class="room-text">{message.text}</p>
					</li>
				{/each}
				{#if typing}
					<li class="room-typing" aria-label="{typing.name} is writing">
						<b>{typing.name}</b> <ActorTag kind={typing.kind} /> <span><i></i><i></i><i></i></span>
					</li>
				{/if}
			</ol>
			<footer class="room-composer">
				<span class="room-input" class:filled={!!draft}>{draft || `Message ${name}…`}</span>
				<span class="room-composer-tools" aria-hidden="true"><AtSign size={14} /> <Mic size={14} /> <SendHorizontal size={14} /></span>
			</footer>
		</section>
	</div>
</div>

<style>
	/* Breakpoints follow this view's own width, not the window's: the same view sits in a full
	 * workspace pane, a narrow side panel or a phone. */
	.view-shell {
		container-type: inline-size;
		width: 100%;
		height: 100%;
		min-width: 0;
	}
	.room-view {
		display: grid;
		grid-template-columns: minmax(150px, 220px) minmax(0, 1fr);
		min-width: 0;
		height: 100%;
		background: var(--surface-pane);
		color: var(--ink);
		font: 400 var(--t-body) var(--font-ui);
	}
	.room-view.no-list {
		grid-template-columns: minmax(0, 1fr);
	}
	.room-list {
		display: grid;
		align-content: start;
		border-right: 1px solid var(--border);
	}
	.room-list-head {
		margin: 0;
		padding: 12px 14px;
		border-bottom: 1px solid var(--border);
		font-weight: 650;
		font-size: var(--t-head);
	}
	.room-entry {
		display: grid;
		grid-template-columns: auto 1fr auto;
		align-items: center;
		gap: 9px;
		padding: 9px 12px;
		border-bottom: 1px solid var(--border);
	}
	.room-entry.active {
		box-shadow: inset 3px 0 0 var(--intent-conversation);
		background: var(--surface-alt);
	}
	.room-icon {
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		color: var(--intent-conversation);
	}
	.room-name {
		display: grid;
		min-width: 0;
	}
	.room-name strong {
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}
	.room-name span {
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}
	.room-entry b {
		padding: 0 6px;
		border-radius: 999px;
		background: var(--intent-conversation);
		color: #fff;
		font: 600 var(--t-label)/16px var(--font-ui);
	}
	.room-main {
		display: grid;
		grid-template-rows: auto 1fr auto;
		min-width: 0;
		min-height: 0;
	}
	.room-head {
		display: flex;
		align-items: center;
		gap: 12px;
		min-width: 0;
		padding: 10px 14px;
		border-bottom: 1px solid var(--border);
	}
	.room-head > div {
		display: grid;
		min-width: 0;
	}
	.room-head strong {
		font-size: var(--t-head);
		font-weight: 650;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.room-who {
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}
	.room-tags {
		display: flex;
		flex-wrap: wrap;
		gap: 4px 10px;
		margin-left: auto;
		font-size: var(--t-label);
		color: var(--text-secondary);
	}
	.room-tags span {
		display: inline-flex;
		align-items: center;
		gap: 4px;
	}
	.room-tools {
		display: inline-flex;
		color: var(--text-tertiary);
	}
	.room-live {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		padding: 2px 7px;
		border: 1px solid color-mix(in srgb, var(--state-success) 40%, transparent);
		border-radius: var(--radius-control);
		background: var(--state-success-soft);
		color: var(--state-success);
		font: 500 var(--t-label) var(--font-mono);
	}
	.room-log {
		display: grid;
		align-content: start;
		gap: 0;
		min-height: 0;
		margin: 0;
		padding: 0;
		overflow: auto;
		list-style: none;
	}
	.room-message {
		display: grid;
		gap: 4px;
		padding: 10px 14px;
		border-bottom: 1px solid var(--border);
		box-shadow: inset 3px 0 0 color-mix(in srgb, var(--intent-conversation) 55%, transparent);
		background: color-mix(in srgb, var(--chat-owner-bg) 55%, transparent);
	}
	.room-message.agent {
		box-shadow: inset 3px 0 0 color-mix(in srgb, var(--intent-direction) 45%, transparent);
		background: transparent;
	}
	.room-message p {
		margin: 0;
	}
	.room-meta {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.room-meta time {
		font: 500 var(--t-label) var(--font-mono);
		color: var(--text-tertiary);
	}
	.room-reply {
		font-size: var(--t-label);
		color: var(--text-tertiary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.room-text {
		line-height: 1.5;
	}
	.room-typing {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 10px 14px;
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}
	.room-typing span {
		display: inline-flex;
		gap: 3px;
	}
	.room-typing i {
		width: 4px;
		height: 4px;
		background: currentColor;
		animation: typing 0.9s steps(2) infinite;
	}
	.room-typing i:nth-child(2) {
		animation-delay: 0.15s;
	}
	.room-typing i:nth-child(3) {
		animation-delay: 0.3s;
	}
	@keyframes typing {
		50% {
			transform: translateY(-3px);
		}
	}
	.room-composer {
		display: flex;
		align-items: center;
		gap: 10px;
		margin: 8px;
		padding: 10px 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		background: var(--surface-raised);
	}
	.room-input {
		flex: 1;
		min-width: 0;
		color: var(--text-tertiary);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.room-input.filled {
		color: var(--ink);
	}
	.room-composer-tools {
		display: inline-flex;
		gap: 10px;
		color: var(--text-tertiary);
	}
	@container (max-width: 720px) {
		.room-view {
			grid-template-columns: minmax(0, 1fr);
		}
		.room-list,
		.room-tags {
			display: none;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.room-typing i {
			animation: none;
		}
	}
</style>
