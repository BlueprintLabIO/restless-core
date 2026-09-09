<script lang="ts">
	import { onDestroy, tick } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import Hash from '@lucide/svelte/icons/hash';
	import MessageCircle from '@lucide/svelte/icons/message-circle';
	import Users from '@lucide/svelte/icons/users';
	import Wifi from '@lucide/svelte/icons/wifi';
	import WifiOff from '@lucide/svelte/icons/wifi-off';
	import RoomMessage from '$lib/components/RoomMessage.svelte';
	import { cockpitQuery } from '$lib/model/queries.svelte';
	import {
		roomActivityStream,
		roomMessagesQuery,
		roomParticipantsQuery,
		roomReadCursorQuery,
		roomsQuery,
		roomThreadQuery
	} from '$lib/model/room-queries.svelte';
	import {
		markRoomRead,
		readRoomDraft,
		roomDraftKey,
		sendRoomMessage,
		writeRoomDraft,
		type NewRoomMention,
		type Room,
		type RoomMessage as RoomMessageRecord
	} from '$lib/model/rooms';
	import Composer from '$lib/primitives/Composer.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const cockpitProjection = $derived(cockpitQuery(companyId));
	const roomList = $derived(roomsQuery(companyId));
	const requestedRoomId = $derived(page.url.searchParams.get('room') ?? '');
	const selectedRoomId = $derived(requestedRoomId || roomList.rooms[0]?.id || '');
	const selectedRoom = $derived(roomList.rooms.find((room) => room.id === selectedRoomId) ?? null);
	const requestedThread = $derived(Number(page.url.searchParams.get('thread')));
	const threadRootId = $derived(
		Number.isSafeInteger(requestedThread) && requestedThread > 0 ? requestedThread : null
	);

	const roomProjection = $derived(
		selectedRoomId ? roomMessagesQuery(companyId, selectedRoomId) : null
	);
	const participantProjection = $derived(
		selectedRoomId ? roomParticipantsQuery(companyId, selectedRoomId) : null
	);
	const readProjection = $derived(
		selectedRoomId ? roomReadCursorQuery(companyId, selectedRoomId) : null
	);
	const activity = $derived(selectedRoomId ? roomActivityStream(companyId, selectedRoomId) : null);
	const threadProjection = $derived(
		selectedRoomId && threadRootId ? roomThreadQuery(companyId, selectedRoomId, threadRootId) : null
	);
	$effect(() => activity?.attach());

	let online = $state(true);
	let composer = $state('');
	let activeDraftKey = $state('');
	let retryCommandId = $state<string | null>(null);
	let retryBody = $state('');
	let sending = $state(false);
	let sendError = $state('');
	let pendingMessage = $state<RoomMessageRecord | null>(null);
	let lastMarkedRoom = $state('');
	let lastMarkedMessage = $state(0);
	let locallyReadRoom = $state('');
	let locallyReadThrough = $state(0);
	let draftTimer: number | undefined;
	let roomScrollEl = $state<HTMLDivElement | undefined>();
	let threadScrollEl = $state<HTMLDivElement | undefined>();
	let roomOpenedFor = $state('');
	let threadOpenedFor = $state('');

	const currentActorId = $derived(readProjection?.actorId ?? '');
	const draftScopeKey = $derived(
		selectedRoomId && currentActorId
			? roomDraftKey(companyId, currentActorId, selectedRoomId, threadRootId)
			: ''
	);

	$effect(() => {
		const nextKey = draftScopeKey;
		if (nextKey === activeDraftKey) return;
		if (!nextKey) {
			/* A Room or authenticated principal changed before the new identity
			 * projection arrived. Retire the old scope immediately so fresh input
			 * can never be written into another principal's draft. */
			activeDraftKey = '';
			composer = '';
			retryCommandId = null;
			retryBody = '';
			sendError = '';
			pendingMessage = null;
			return;
		}
		const hadUnscopedInput = !activeDraftKey && composer.length > 0;
		activeDraftKey = nextKey;
		const stored = readRoomDraft(nextKey);
		/* Identity can arrive after the owner starts typing. Keep that ephemeral
		 * input rather than replacing it with a persisted principal-scoped draft. */
		if (!hadUnscopedInput) {
			composer = stored.body;
			retryCommandId = stored.commandId;
			retryBody = stored.commandId ? stored.body.trim() : '';
		}
		sendError = '';
		pendingMessage = null;
	});

	$effect(() => {
		const key = activeDraftKey;
		const body = composer;
		const commandId = retryBody === body.trim() ? retryCommandId : null;
		if (!key || typeof window === 'undefined') return;
		const draft = { body, commandId, updatedAt: new Date().toISOString() };
		window.clearTimeout(draftTimer);
		draftTimer = window.setTimeout(() => {
			writeRoomDraft(key, draft);
		}, 120);
		return () => {
			window.clearTimeout(draftTimer);
			/* Switching Room or Thread must not strand the final keystroke inside a
			 * cancelled debounce. Flush the captured scope before it disappears. */
			writeRoomDraft(key, draft);
		};
	});

	onDestroy(() => {
		if (typeof window === 'undefined') return;
		window.clearTimeout(draftTimer);
		if (activeDraftKey) {
			writeRoomDraft(activeDraftKey, {
				body: composer,
				commandId: retryBody === composer.trim() ? retryCommandId : null,
				updatedAt: new Date().toISOString()
			});
		}
	});

	const roomMessages = $derived(roomProjection?.messages ?? []);
	const roots = $derived(roomMessages.filter((message) => message.parent_message_id === null));
	const visibleRoots = $derived(
		pendingMessage && pendingMessage.parent_message_id === null ? [...roots, pendingMessage] : roots
	);
	const threadMessages = $derived(threadProjection?.messages ?? []);
	const visibleThread = $derived(
		pendingMessage && pendingMessage.parent_message_id !== null
			? [...threadMessages, pendingMessage]
			: threadMessages
	);
	const lastLoadedMessageId = $derived.by(() => {
		const loaded = [roomMessages.at(-1)?.id, threadMessages.at(-1)?.id].filter(
			(value): value is number => typeof value === 'number'
		);
		return loaded.length ? Math.max(...loaded) : null;
	});
	const unreadFrom = $derived.by(() => {
		const server = readProjection?.cursor?.last_read_message_id ?? null;
		const local = locallyReadRoom === selectedRoomId ? locallyReadThrough : 0;
		return Math.max(server ?? 0, local) || server;
	});
	const people = $derived(cockpitProjection.view?.people ?? []);
	const participants = $derived(participantProjection?.participants ?? []);
	const participantSummary = $derived.by(() => {
		const names = participants.map((participant) => actorName(participant.actor_id));
		if (names.length <= 3) return names.join(', ');
		return `${names.slice(0, 2).join(', ')} and ${names.length - 2} more`;
	});

	$effect(() => {
		const room = selectedRoomId;
		const scroller = roomScrollEl;
		if (!room || !scroller || !roomMessages.length || roomOpenedFor === room) return;
		roomOpenedFor = room;
		void tick().then(() => scroller.scrollTo({ top: scroller.scrollHeight }));
	});

	$effect(() => {
		const scope = threadRootId ? `${selectedRoomId}:${threadRootId}` : '';
		const scroller = threadScrollEl;
		if (!scope) {
			threadOpenedFor = '';
			return;
		}
		if (!scroller || !threadMessages.length || threadOpenedFor === scope) return;
		threadOpenedFor = scope;
		void tick().then(() => scroller.scrollTo({ top: scroller.scrollHeight }));
	});

	$effect(() => {
		const room = selectedRoomId;
		const through = lastLoadedMessageId;
		if (!online || !room || through === null) return;
		if (room === lastMarkedRoom && through <= lastMarkedMessage) return;
		if (readProjection?.cursor?.last_read_message_id === through) return;
		lastMarkedRoom = room;
		lastMarkedMessage = through;
		void markRoomRead(companyId, room, through)
			.then((cursor) => {
				readProjection?.accept(cursor);
				locallyReadRoom = room;
				locallyReadThrough = cursor.last_read_message_id ?? through;
			})
			.catch(() => {
				// Read state is a recoverable projection; a later page/event refresh retries.
				if (lastMarkedRoom === room && lastMarkedMessage === through) lastMarkedMessage = 0;
			});
	});

	function actorName(actorId: string): string {
		if (actorId === currentActorId) return 'You';
		return people.find((person) => person.actor_id === actorId)?.display ?? actorId;
	}

	function actorIsAgent(actorId: string): boolean {
		const kind = people.find((person) => person.actor_id === actorId)?.kind;
		return kind === 'exec' || kind === 'staff';
	}

	function mentionsFor(messageId: number) {
		const mentions = [...(roomProjection?.mentions ?? []), ...(threadProjection?.mentions ?? [])];
		return [...new Map(mentions.map((mention) => [mention.id, mention])).values()].filter(
			(mention) => mention.message_id === messageId
		);
	}

	function roomIcon(room: Room): typeof Hash {
		return room.kind === 'direct' ? MessageCircle : room.kind === 'group' ? Users : Hash;
	}

	function roomKind(room: Room): string {
		if (room.kind === 'company') return 'Company Room';
		if (room.kind === 'direct') return 'Direct Room';
		return 'Group Room';
	}

	function roomHref(roomId: string): string {
		return `/${encodeURIComponent(companyId)}/people/rooms?room=${encodeURIComponent(roomId)}`;
	}

	function threadHref(messageId: number): string {
		return `${roomHref(selectedRoomId)}&thread=${encodeURIComponent(messageId)}`;
	}

	function openThread(messageId: number) {
		void goto(threadHref(messageId), { keepFocus: true, noScroll: true });
	}

	function closeThread() {
		void goto(roomHref(selectedRoomId), { keepFocus: true, noScroll: true });
	}

	async function loadOlderRoomMessages() {
		if (!roomProjection || !roomScrollEl) return;
		const before = roomScrollEl.scrollHeight;
		await roomProjection.loadMore();
		await tick();
		roomScrollEl.scrollTop += roomScrollEl.scrollHeight - before;
	}

	async function loadOlderThreadMessages() {
		if (!threadProjection || !threadScrollEl) return;
		const before = threadScrollEl.scrollHeight;
		await threadProjection.loadMore();
		await tick();
		threadScrollEl.scrollTop += threadScrollEl.scrollHeight - before;
	}

	function knownMentions(body: string): NewRoomMention[] {
		const tokens = new Set(
			body
				.split(/\s+/)
				.filter((token) => token.startsWith('@'))
				.map((token) => token.slice(1).replace(/[.,!?;]+$/u, ''))
				.filter(Boolean)
		);
		const known = new Set(participants.map((participant) => participant.actor_id));
		if (tokens.has('exec')) known.add('exec');
		return [...tokens].filter((actorId) => known.has(actorId)).map((actor_id) => ({ actor_id }));
	}

	function insertExecMention() {
		if (composer.split(/\s+/).includes('@exec')) return;
		composer = `${composer.trimEnd()}${composer.trim() ? ' ' : ''}@exec `;
	}

	async function submitMessage(event: SubmitEvent) {
		event.preventDefault();
		const body = composer.trim();
		if (!body || !selectedRoomId || !currentActorId || sending || !online) return;

		const commandId = retryCommandId && retryBody === body ? retryCommandId : crypto.randomUUID();
		const parent = threadRootId;
		sending = true;
		sendError = '';
		pendingMessage = {
			id: -Date.now(),
			room_id: selectedRoomId,
			from_actor: currentActorId,
			to_actor: null,
			body,
			outcome_standard: null,
			parent_message_id: parent,
			thread_root_message_id: parent,
			client_command_id: commandId,
			created_at: new Date().toISOString(),
			legacy_read_at: null
		};
		try {
			const result = await sendRoomMessage(
				companyId,
				selectedRoomId,
				body,
				commandId,
				parent,
				knownMentions(body)
			);
			if (parent === null) roomProjection?.accept(result);
			else threadProjection?.accept(result);
			composer = '';
			retryCommandId = null;
			retryBody = '';
			pendingMessage = null;
			if (activeDraftKey) {
				writeRoomDraft(activeDraftKey, { body: '', commandId: null, updatedAt: '' });
			}
			await tick();
			const scroller = parent === null ? roomScrollEl : threadScrollEl;
			scroller?.scrollTo({ top: scroller.scrollHeight, behavior: 'smooth' });
		} catch (cause) {
			const status = (cause as { status?: unknown }).status;
			const retryable =
				typeof status !== 'number' ||
				status === 408 ||
				status === 425 ||
				status === 429 ||
				status >= 500;
			retryCommandId = retryable ? commandId : null;
			retryBody = retryable ? body : '';
			pendingMessage = null;
			sendError = cause instanceof Error ? cause.message : 'This message was not delivered.';
		} finally {
			sending = false;
		}
	}

	function dayLabel(value: string): string {
		const date = new Date(value);
		if (Number.isNaN(date.getTime())) return '';
		const today = new Date();
		if (date.toDateString() === today.toDateString()) return 'Today';
		const yesterday = new Date(today);
		yesterday.setDate(today.getDate() - 1);
		if (date.toDateString() === yesterday.toDateString()) return 'Yesterday';
		return date.toLocaleDateString(undefined, { month: 'long', day: 'numeric' });
	}

	function dayKey(value: string): string {
		const date = new Date(value);
		return Number.isNaN(date.getTime()) ? value : date.toDateString();
	}
</script>

<svelte:window bind:online />
<svelte:head><title>Rooms — {cockpitProjection.view?.company.name ?? companyId}</title></svelte:head
>

<div
	class="cockpit-screen rooms-screen"
	class:room-selected={requestedRoomId !== ''}
	class:thread-selected={threadRootId !== null}
>
	<section class="room-list-pane cockpit-pane" aria-label="Rooms">
		<header class="cockpit-pane-head rooms-index-head">
			<div>
				<a href={`/${companyId}/people`} aria-label="Back to People" title="Back to People">
					<ArrowLeft size={15} strokeWidth={2} aria-hidden="true" />
				</a>
				<h1>Rooms</h1>
			</div>
			<span class="pane-count">{roomList.rooms.length}</span>
		</header>

		<div class="room-list">
			{#each roomList.rooms as room (room.id)}
				{@const Icon = roomIcon(room)}
				<a
					class="room-row"
					class:selected={room.id === selectedRoomId}
					href={roomHref(room.id)}
					aria-current={room.id === selectedRoomId ? 'page' : undefined}
				>
					<span class="room-row-icon"><Icon size={15} strokeWidth={1.8} aria-hidden="true" /></span>
					<span><strong>{room.title}</strong><small>{roomKind(room)}</small></span>
					<ChevronRight size={14} strokeWidth={1.8} aria-hidden="true" />
				</a>
			{:else}
				{#if roomList.status === 'unknown'}
					<p class="room-empty">Loading Rooms…</p>
				{:else}
					<div class="room-empty">
						<strong>No Rooms yet.</strong>
						<p>The company Room appears here when collaboration is ready.</p>
					</div>
				{/if}
			{/each}
		</div>

		{#if roomList.hasMore}
			<button
				type="button"
				class="load-room-page"
				disabled={roomList.loadingMore}
				onclick={() => void roomList.loadMore()}
			>
				{roomList.loadingMore ? 'Loading…' : 'Load older Rooms'}
			</button>
		{/if}
		{#if roomList.failure}
			<p class="room-source-error" role="status">{roomList.failure.message}</p>
		{/if}
	</section>

	<section class="room-conversation cockpit-pane">
		{#if selectedRoomId}
			<header class="room-head">
				<a class="mobile-room-back" href={`/${companyId}/people/rooms`} aria-label="All Rooms">
					<ArrowLeft size={15} strokeWidth={2} aria-hidden="true" />
				</a>
				<div class="room-head-copy">
					<strong>{selectedRoom?.title ?? 'Room'}</strong>
					{#if participantSummary}<small>{participantSummary}</small>{/if}
				</div>
				<div
					class="room-transport"
					class:degraded={!online || activity?.transport === 'reconnecting'}
					role="status"
					aria-live="polite"
				>
					{#if online && activity?.transport === 'live'}
						<Wifi size={13} strokeWidth={2} aria-hidden="true" /> Live
					{:else if online}
						<WifiOff size={13} strokeWidth={2} aria-hidden="true" /> Reconnecting
					{:else}
						<WifiOff size={13} strokeWidth={2} aria-hidden="true" /> Offline
					{/if}
				</div>
			</header>

			{#if (!online || activity?.transport === 'reconnecting') && roomMessages.length}
				<div class="degraded-strip" role="status">
					Showing the last loaded messages. Your draft stays on this device.
				</div>
			{/if}

			<div class="room-message-list" bind:this={roomScrollEl}>
				{#if roomProjection?.hasMore}
					<button
						type="button"
						class="load-message-page"
						disabled={roomProjection.loadingMore}
						onclick={() => void loadOlderRoomMessages()}
					>
						{roomProjection.loadingMore ? 'Loading…' : 'Load older messages'}
					</button>
				{/if}
				{#each visibleRoots as message, index (message.id)}
					{#if index === 0 || dayKey(message.created_at) !== dayKey(visibleRoots[index - 1].created_at)}
						<div class="room-day"><span>{dayLabel(message.created_at)}</span></div>
					{/if}
					{#if unreadFrom !== null && message.id > unreadFrom && (index === 0 || visibleRoots[index - 1].id <= unreadFrom)}
						<div class="unread-rule"><span>New since your last read</span></div>
					{/if}
					<RoomMessage
						{message}
						author={message.id < 0 ? 'You' : actorName(message.from_actor)}
						isYou={message.id < 0 || message.from_actor === currentActorId}
						isAgent={actorIsAgent(message.from_actor)}
						mentions={mentionsFor(message.id)}
						onthread={message.id > 0 ? () => openThread(message.id) : null}
					/>
				{:else}
					{#if roomProjection?.status === 'unknown'}
						<div class="conversation-empty">Loading conversation…</div>
					{:else if roomProjection?.failure && !roomMessages.length}
						<div class="conversation-empty failure">
							<strong>Conversation unavailable.</strong>
							<p>{roomProjection.failure.message}</p>
							<button type="button" onclick={() => void roomProjection?.refresh()}>Try again</button
							>
						</div>
					{:else}
						<div class="conversation-empty">
							<strong>Nothing said yet.</strong>
							<p>Start the shared conversation. It remains available while the Runtime sleeps.</p>
						</div>
					{/if}
				{/each}
			</div>

			{#if threadRootId === null}{@render messageComposer()}{/if}
		{:else}
			<div class="conversation-empty choose-room">
				<Users size={24} strokeWidth={1.6} aria-hidden="true" />
				<strong>Choose a Room.</strong>
			</div>
		{/if}
	</section>

	{#if threadRootId}
		<aside class="thread-pane cockpit-pane" aria-label="Thread">
			<header class="thread-head">
				<button type="button" onclick={closeThread} aria-label="Close Thread">
					<ArrowLeft size={15} strokeWidth={2} aria-hidden="true" />
				</button>
				<div>
					<strong>Thread</strong><small>{Math.max(0, visibleThread.length - 1)} replies</small>
				</div>
			</header>
			<div class="thread-messages" bind:this={threadScrollEl}>
				{#if threadProjection?.hasMore}
					<button
						type="button"
						class="load-message-page"
						disabled={threadProjection.loadingMore}
						onclick={() => void loadOlderThreadMessages()}
					>
						{threadProjection.loadingMore ? 'Loading…' : 'Load older replies'}
					</button>
				{/if}
				{#each visibleThread as message (message.id)}
					<RoomMessage
						{message}
						author={message.id < 0 ? 'You' : actorName(message.from_actor)}
						isYou={message.id < 0 || message.from_actor === currentActorId}
						isAgent={actorIsAgent(message.from_actor)}
						mentions={mentionsFor(message.id)}
						thread
					/>
				{:else}
					{#if threadProjection?.status === 'unknown'}
						<div class="conversation-empty">Loading Thread…</div>
					{:else}
						<div class="conversation-empty failure">
							<strong>Thread unavailable.</strong>
							<p>{threadProjection?.failure?.message ?? 'This Thread could not be loaded.'}</p>
						</div>
					{/if}
				{/each}
			</div>
			{@render messageComposer()}
		</aside>
	{/if}
</div>

{#snippet messageComposer()}
	<form class="room-composer" onsubmit={submitMessage}>
		<Composer
			bind:value={composer}
			actionLabel={retryCommandId && retryBody === composer.trim() ? 'Retry send' : 'Send'}
			disabled={sending || !online}
			minlength={1}
			allowAttachments={false}
			placeholder={threadRootId
				? 'Reply in this Thread…'
				: `Message ${selectedRoom?.title ?? 'Room'}…`}
			ariaLabel={threadRootId ? 'Thread reply' : 'Room message'}
		>
			{#snippet controls()}
				<button type="button" class="mention-exec" onclick={insertExecMention}>@exec</button>
			{/snippet}
		</Composer>
		{#if sendError}<p class="room-send-error" role="alert">{sendError}</p>{/if}
		{#if !online}<p class="room-draft-state">Draft kept locally until you reconnect.</p>{/if}
	</form>
{/snippet}

<style>
	.rooms-screen {
		width: 100%;
		height: 100%;
		display: grid;
		grid-template-columns: 240px minmax(0, 1fr);
		gap: var(--pane-gap);
		overflow: hidden;
	}

	.rooms-screen.thread-selected {
		grid-template-columns: 240px minmax(320px, 1fr) minmax(300px, 360px);
	}

	.room-list-pane,
	.room-conversation,
	.thread-pane {
		min-width: 0;
		min-height: 0;
		overflow: hidden;
	}

	.room-list-pane,
	.room-conversation,
	.thread-pane {
		display: flex;
		flex-direction: column;
	}

	.rooms-index-head > div,
	.room-head,
	.thread-head {
		display: flex;
		align-items: center;
	}

	.rooms-index-head > div {
		min-width: 0;
		gap: 8px;
	}

	.rooms-index-head a,
	.mobile-room-back,
	.thread-head button {
		width: 28px;
		height: 28px;
		display: grid;
		place-items: center;
		flex: 0 0 auto;
		padding: 0;
		border: 1px solid transparent;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		cursor: pointer;
	}

	.rooms-index-head a:hover,
	.mobile-room-back:hover,
	.thread-head button:hover {
		border-color: var(--border-strong);
		background: var(--surface-alt);
		color: var(--ink);
	}

	.room-list {
		min-height: 0;
		flex: 1;
		overflow: auto;
	}

	.room-row {
		position: relative;
		display: grid;
		grid-template-columns: 30px minmax(0, 1fr) auto;
		align-items: center;
		gap: 9px;
		padding: 10px 11px;
		border-bottom: 1px solid var(--border);
		background: transparent;
		color: var(--text-secondary);
		text-decoration: none;
		transition:
			background var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard);
	}

	.room-row::before {
		position: absolute;
		inset: 7px auto 7px 0;
		width: 2px;
		background: transparent;
		content: '';
		transition: background var(--motion-state) var(--ease-standard);
	}

	.room-row:hover,
	.room-row.selected {
		background: rgba(255, 255, 255, 0.6);
		color: var(--ink);
	}

	.room-row.selected::before {
		background: var(--intent-conversation);
	}

	.room-row-icon {
		width: 30px;
		height: 30px;
		display: grid;
		place-items: center;
		border: 1px solid color-mix(in srgb, var(--intent-conversation) 20%, var(--border));
		border-radius: var(--radius-control);
		background: var(--intent-conversation-soft);
		color: var(--intent-conversation);
	}

	.room-row strong,
	.room-row small {
		display: block;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.room-row strong {
		font-size: var(--t-body);
		font-weight: 600;
	}

	.room-row small {
		margin-top: 2px;
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}

	.load-room-page,
	.load-message-page {
		flex: 0 0 auto;
		padding: 8px 12px;
		border: 0;
		border-top: 1px solid var(--border);
		background: var(--surface-alt);
		font-size: var(--t-label);
		font-weight: 600;
		color: var(--intent-conversation);
		cursor: pointer;
	}

	.load-message-page {
		width: 100%;
		border-bottom: 1px solid var(--border);
	}

	.room-empty,
	.room-source-error {
		margin: 0;
		padding: 14px 12px;
		font-size: var(--t-body);
		color: var(--text-secondary);
	}

	.room-empty p {
		margin: 4px 0 0;
	}

	.room-source-error {
		border-top: 1px solid color-mix(in srgb, var(--state-danger) 24%, var(--border));
		color: var(--state-danger);
	}

	.room-head,
	.thread-head {
		min-height: var(--pane-head-h);
		gap: 9px;
		padding: 9px 14px;
		border-bottom: 1px solid var(--border);
		background: rgba(255, 255, 255, 0.48);
		box-shadow: var(--bevel-subtle);
	}

	.mobile-room-back {
		display: none;
	}

	.room-head-copy,
	.thread-head > div {
		min-width: 0;
		flex: 1;
	}

	.room-head-copy strong,
	.room-head-copy small,
	.thread-head strong,
	.thread-head small {
		display: block;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.room-head-copy strong,
	.thread-head strong {
		font-size: var(--t-head);
		font-weight: 650;
	}

	.room-head-copy small,
	.thread-head small {
		margin-top: 2px;
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}

	.room-transport {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		flex: 0 0 auto;
		padding: 3px 7px;
		border: 1px solid color-mix(in srgb, var(--state-success) 24%, var(--border));
		border-radius: var(--radius-control);
		background: var(--state-success-soft);
		font: 500 var(--t-label) var(--font-mono);
		color: var(--state-success);
	}

	.room-transport.degraded {
		border-color: color-mix(in srgb, var(--intent-authority) 24%, var(--border));
		background: var(--intent-authority-soft);
		color: var(--intent-authority);
	}

	.degraded-strip {
		flex: 0 0 auto;
		padding: 6px 14px;
		border-bottom: 1px solid color-mix(in srgb, var(--intent-authority) 20%, var(--border));
		background: color-mix(in srgb, var(--intent-authority-soft) 58%, var(--surface));
		font-size: var(--t-label);
		color: var(--intent-authority);
	}

	.room-message-list,
	.thread-messages {
		min-height: 0;
		flex: 1;
		overflow: auto;
		overscroll-behavior: contain;
		background: var(--surface-pane);
	}

	.room-day,
	.unread-rule {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 9px 14px 5px;
		font: 500 var(--t-label) var(--font-mono);
		color: var(--text-tertiary);
	}

	.room-day::before,
	.room-day::after,
	.unread-rule::before,
	.unread-rule::after {
		height: 1px;
		flex: 1;
		background: var(--border);
		content: '';
	}

	.unread-rule {
		color: var(--intent-conversation);
	}

	.unread-rule::before,
	.unread-rule::after {
		background: color-mix(in srgb, var(--intent-conversation) 30%, transparent);
	}

	.conversation-empty {
		min-height: 100%;
		display: grid;
		place-content: center;
		justify-items: center;
		padding: 28px;
		text-align: center;
		color: var(--text-secondary);
	}

	.conversation-empty strong {
		font-size: var(--t-head);
		color: var(--ink);
	}

	.conversation-empty p {
		max-width: 420px;
		margin: 6px 0 0;
		font-size: var(--t-body);
	}

	.conversation-empty button {
		margin-top: 12px;
		padding: 6px 10px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface);
		box-shadow: var(--control-depth);
		cursor: pointer;
	}

	.conversation-empty.failure strong,
	.conversation-empty.failure p {
		color: var(--state-danger);
	}

	.room-composer {
		flex: 0 0 auto;
		padding: 9px 16px max(11px, env(safe-area-inset-bottom));
		background: rgba(255, 255, 255, 0.66);
		box-shadow: 0 -10px 30px rgba(43, 51, 66, 0.035);
	}

	.mention-exec {
		padding: 3px 6px;
		border: 1px solid transparent;
		border-radius: var(--radius-control);
		background: transparent;
		font: 500 var(--t-label) var(--font-mono);
		color: var(--intent-direction);
		cursor: pointer;
	}

	.mention-exec:hover,
	.mention-exec:focus-visible {
		border-color: color-mix(in srgb, var(--intent-direction) 22%, var(--border));
		background: var(--intent-direction-soft);
	}

	.room-send-error,
	.room-draft-state {
		margin: 6px 2px 0;
		font-size: var(--t-label);
		color: var(--state-danger);
	}

	.room-draft-state {
		color: var(--intent-authority);
	}

	.thread-pane {
		border-left: 1px solid var(--border);
		background: var(--surface-rail);
	}

	@media (max-width: 1180px) {
		.rooms-screen.thread-selected {
			grid-template-columns: 208px minmax(300px, 1fr) minmax(280px, 330px);
		}
		.rooms-screen {
			grid-template-columns: 208px minmax(0, 1fr);
		}
	}

	@media (max-width: 760px) {
		.rooms-screen,
		.rooms-screen.thread-selected {
			grid-template-columns: 1fr;
			grid-template-rows: minmax(0, 1fr);
		}

		.room-conversation,
		.thread-pane,
		.rooms-screen.room-selected .room-list-pane {
			display: none;
		}

		.rooms-screen.room-selected .room-conversation {
			display: flex;
		}

		.rooms-screen.thread-selected .room-conversation {
			display: none;
		}

		.rooms-screen.thread-selected .thread-pane {
			display: flex;
		}

		.mobile-room-back {
			display: grid;
		}

		.room-head,
		.thread-head {
			padding-inline: 9px;
		}

		.room-composer {
			padding-inline: 8px;
		}

		.room-message-list,
		.thread-messages {
			scroll-padding-bottom: 92px;
		}
	}
</style>
