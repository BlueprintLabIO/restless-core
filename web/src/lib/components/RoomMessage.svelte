<script lang="ts">
	import { tick } from 'svelte';
	import MessageCircle from '@lucide/svelte/icons/message-circle';
	import ConversationMessage from '$lib/primitives/ConversationMessage.svelte';
	import SemanticMark from '$lib/primitives/SemanticMark.svelte';
	import type { MessageAttachment, ThreadMessage } from '$lib/model/view';
	import type {
		RoomMention,
		RoomMessage as RoomMessageRecord,
		RoomMessageRevision
	} from '$lib/model/rooms';

	let {
		message,
		presentation = null,
		hrefFor,
		author,
		isYou = false,
		isAgent = false,
		targeted = false,
		focusKey = '',
		mentions = [],
		thread = false,
		onthread = null,
		canEdit = false,
		canDelete = false,
		historyOpen = false,
		revisions = [],
		historyStatus = 'live',
		historyFailure = '',
		historyHasMore = false,
		historyLoadingMore = false,
		onhistory = null,
		onloadhistory = null,
		onedit = null,
		ondelete = null
	}: {
		message: RoomMessageRecord;
		presentation?: ThreadMessage | null;
		hrefFor?: (attachment: MessageAttachment) => string;
		author: string;
		isYou?: boolean;
		isAgent?: boolean;
		targeted?: boolean;
		focusKey?: string;
		mentions?: RoomMention[];
		thread?: boolean;
		onthread?: (() => void) | null;
		canEdit?: boolean;
		canDelete?: boolean;
		historyOpen?: boolean;
		revisions?: RoomMessageRevision[];
		historyStatus?: 'unknown' | 'live' | 'stale';
		historyFailure?: string;
		historyHasMore?: boolean;
		historyLoadingMore?: boolean;
		onhistory?: (() => void) | null;
		onloadhistory?: (() => void) | null;
		onedit?: ((body: string, commandId: string) => Promise<void>) | null;
		ondelete?: ((commandId: string) => Promise<void>) | null;
	} = $props();

	let editing = $state(false);
	let editBody = $state('');
	let editCommandId = $state<string | null>(null);
	let editCommandBody = $state('');
	let editError = $state('');
	let saving = $state(false);
	let confirmingDelete = $state(false);
	let deleteCommandId = $state<string | null>(null);
	let deleteError = $state('');
	let deleting = $state(false);
	let messageElement = $state<HTMLElement | undefined>();
	let focusedFor = '';

	$effect(() => {
		const key = targeted ? focusKey : '';
		const element = messageElement;
		if (!key) {
			focusedFor = '';
			return;
		}
		if (!element || focusedFor === key) return;
		focusedFor = key;
		void tick().then(() => {
			if (!targeted || focusKey !== key || messageElement !== element) return;
			const reducedMotion =
				typeof window !== 'undefined' &&
				window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
			element.focus({ preventScroll: true });
			element.scrollIntoView({ block: 'center', behavior: reducedMotion ? 'auto' : 'smooth' });
		});
	});

	function clearDeletedState() {
		// A server tombstone always wins over unsaved local UI. In particular,
		// never leave the previous body resident in an edit field after an SSE
		// deletion arrives while this component is awaiting a mutation.
		editing = false;
		editBody = '';
		editCommandId = null;
		editCommandBody = '';
		editError = '';
		saving = false;
		confirmingDelete = false;
		deleteCommandId = null;
		deleteError = '';
		deleting = false;
	}

	$effect(() => {
		if (message.deleted_at) clearDeletedState();
	});

	const time = $derived.by(() => {
		const value = new Date(message.created_at);
		return Number.isNaN(value.getTime())
			? ''
			: value.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' }).toLowerCase();
	});

	function retryable(cause: unknown): boolean {
		const status = (cause as { status?: unknown }).status;
		return (
			typeof status !== 'number' ||
			status === 408 ||
			status === 425 ||
			status === 429 ||
			status >= 500
		);
	}

	function beginEdit() {
		editBody = message.body;
		editCommandId = null;
		editCommandBody = '';
		editError = '';
		confirmingDelete = false;
		editing = true;
	}

	function cancelEdit() {
		editing = false;
		editError = '';
		editCommandId = null;
		editCommandBody = '';
	}

	async function saveEdit() {
		const body = editBody.trim();
		if (!body || body === message.body || !onedit || saving) return;
		const commandId =
			editCommandId && editCommandBody === body ? editCommandId : crypto.randomUUID();
		saving = true;
		editError = '';
		try {
			await onedit(body, commandId);
			cancelEdit();
		} catch (cause) {
			if (message.deleted_at) {
				clearDeletedState();
				return;
			}
			if (retryable(cause)) {
				editCommandId = commandId;
				editCommandBody = body;
			} else {
				editCommandId = null;
				editCommandBody = '';
			}
			editError = cause instanceof Error ? cause.message : 'The edit was not saved.';
		} finally {
			saving = false;
		}
	}

	async function confirmDelete() {
		if (!ondelete || deleting) return;
		const commandId = deleteCommandId ?? crypto.randomUUID();
		deleting = true;
		deleteError = '';
		try {
			await ondelete(commandId);
			confirmingDelete = false;
			deleteCommandId = null;
		} catch (cause) {
			if (message.deleted_at) {
				clearDeletedState();
				return;
			}
			deleteCommandId = retryable(cause) ? commandId : null;
			deleteError = cause instanceof Error ? cause.message : 'The message was not deleted.';
		} finally {
			deleting = false;
		}
	}
</script>

<article
	bind:this={messageElement}
	class="room-message"
	class:you={isYou}
	class:thread
	class:targeted
	tabindex="-1"
>
	{#if targeted}<span class="sr-only">Linked mention message.</span>{/if}
	{#if message.deleted_at || editing}
		<header>
			<SemanticMark
				meaning={isYou ? 'direction' : isAgent ? 'executive' : 'people'}
				size="small"
				label={`${author} message`}
			/>
			<strong>{author}</strong>
			{#if time}<time datetime={message.created_at}>{time}</time>{/if}
			{#if message.edited_at && !message.deleted_at}<span class="lifecycle">edited</span>{/if}
			{#if mentions.length}
				<span class="mention-receipt">
					{mentions.map((mention) => `@${mention.mentioned_actor_id}`).join(', ')}
				</span>
			{/if}
		</header>
		{#if message.deleted_at}
			<p class="deleted">Message deleted</p>
		{:else}
			<div class="edit-panel">
				<label>
					<span class="sr-only">Edit message</span>
					<textarea bind:value={editBody} maxlength={65536} rows={3}></textarea>
				</label>
				<div class="edit-controls">
					<button type="button" class="primary-action" disabled={saving} onclick={saveEdit}>
						{saving ? 'Saving…' : editCommandId ? 'Retry save' : 'Save'}
					</button>
					<button type="button" disabled={saving} onclick={cancelEdit}>Cancel</button>
				</div>
				{#if editError}<p class="action-error" role="alert">{editError}</p>{/if}
			</div>
		{/if}
	{:else}
		<ConversationMessage
			sender={isYou ? 'owner' : isAgent ? 'agent' : 'human'}
			{author}
			text={message.edited_at ? message.body : (presentation?.text ?? message.body)}
			createdAt={message.created_at}
			details={message.edited_at ? null : presentation?.details}
			attachments={message.edited_at ? [] : (presentation?.attachments ?? [])}
			intent={message.edited_at ? null : (presentation?.intent ?? null)}
			{hrefFor}
			embedded
		>
			{#snippet headerExtra()}
				{#if message.edited_at}<span class="lifecycle">edited</span>{/if}
				{#if mentions.length}
					<span class="mention-receipt">
						{mentions.map((mention) => `@${mention.mentioned_actor_id}`).join(', ')}
					</span>
				{/if}
			{/snippet}
			{#snippet actions()}
				<span class="inline-actions">
					{#if onthread}
						<button type="button" onclick={() => onthread?.()}>
							<MessageCircle size={13} strokeWidth={2} aria-hidden="true" /> Reply
						</button>
					{/if}
					{#if canEdit}<button type="button" onclick={beginEdit}>Edit</button>{/if}
					{#if onhistory}
						<button type="button" aria-expanded={historyOpen} onclick={() => onhistory?.()}>
							{historyOpen ? 'Hide edits' : 'Edits'}
						</button>
					{/if}
					{#if canDelete}
						<button
							type="button"
							class="delete-action"
							aria-expanded={confirmingDelete}
							onclick={() => {
								confirmingDelete = !confirmingDelete;
								deleteError = '';
							}}>Delete</button
						>
					{/if}
				</span>
			{/snippet}
		</ConversationMessage>
	{/if}
	{#if (message.deleted_at || editing) && onthread}
		<div class="message-actions">
			<button type="button" onclick={() => onthread?.()}>
				<MessageCircle size={13} strokeWidth={2} aria-hidden="true" />
				{message.deleted_at ? 'View thread' : 'Reply'}
			</button>
		</div>
	{/if}
	{#if confirmingDelete && !message.deleted_at}
		<div class="delete-confirm" role="group" aria-label="Confirm message deletion">
			<span>Delete this message?</span>
			<button type="button" class="danger-action" disabled={deleting} onclick={confirmDelete}>
				{deleting ? 'Deleting…' : deleteCommandId ? 'Retry delete' : 'Delete'}
			</button>
			<button
				type="button"
				disabled={deleting}
				onclick={() => {
					confirmingDelete = false;
					deleteError = '';
					deleteCommandId = null;
				}}>Cancel</button
			>
		</div>
		{#if deleteError}<p class="action-error" role="alert">{deleteError}</p>{/if}
	{/if}
	{#if historyOpen && !message.deleted_at}
		<section class="revision-history" aria-label="Message edits">
			{#if historyStatus === 'unknown'}
				<p class="history-state">Loading edits…</p>
			{:else if historyFailure}
				<p class="action-error" role="alert">{historyFailure}</p>
			{:else if revisions.length}
				{#each revisions as revision (revision.id)}
					<article>
						<header>
							<strong>Edit {revision.revision_number}</strong>
							<time datetime={revision.created_at}>
								{new Date(revision.created_at).toLocaleString()}
							</time>
						</header>
						<p>{revision.body}</p>
					</article>
				{/each}
				{#if historyHasMore}
					<button
						type="button"
						class="load-history"
						disabled={historyLoadingMore}
						onclick={() => onloadhistory?.()}
					>
						{historyLoadingMore ? 'Loading…' : 'Older edits'}
					</button>
				{/if}
			{:else}
				<p class="history-state">No earlier edits.</p>
			{/if}
		</section>
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

	.room-message.targeted,
	.room-message.you.targeted {
		z-index: 1;
		background: color-mix(in srgb, var(--intent-direction) 9%, var(--surface));
		box-shadow:
			inset 3px 0 0 var(--intent-direction),
			inset 0 0 0 1px color-mix(in srgb, var(--intent-direction) 34%, transparent);
	}

	.room-message.targeted:focus {
		outline: 2px solid color-mix(in srgb, var(--intent-direction) 58%, transparent);
		outline-offset: -3px;
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
	.mention-receipt,
	.lifecycle {
		flex: 0 0 auto;
		font: 500 var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
		color: var(--text-tertiary);
	}

	.lifecycle {
		font-style: italic;
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

	p.deleted {
		font-style: italic;
		color: var(--text-tertiary);
	}

	.inline-actions {
		display: inline-flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 2px;
	}

	.message-actions {
		display: inline-flex;
		align-items: center;
		gap: 2px;
		margin: 8px 0 0 31px;
	}

	:is(.message-actions, .inline-actions) button,
	.edit-controls button,
	.delete-confirm button,
	.load-history {
		display: inline-flex;
		align-items: center;
		gap: 5px;
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

	:is(.message-actions, .inline-actions) button:hover,
	:is(.message-actions, .inline-actions) button:focus-visible,
	.edit-controls button:hover,
	.edit-controls button:focus-visible,
	.delete-confirm button:hover,
	.delete-confirm button:focus-visible,
	.load-history:hover,
	.load-history:focus-visible {
		border-color: color-mix(in srgb, var(--intent-conversation) 24%, var(--border));
		background: var(--intent-conversation-soft);
	}

	:is(.message-actions, .inline-actions) button:active,
	.edit-controls button:active,
	.delete-confirm button:active,
	.load-history:active {
		transform: translateY(1px);
	}

	:is(.message-actions, .inline-actions) button:focus-visible,
	.edit-controls button:focus-visible,
	.delete-confirm button:focus-visible,
	.load-history:focus-visible {
		outline: 2px solid color-mix(in srgb, var(--intent-conversation) 32%, transparent);
		outline-offset: 2px;
	}

	.inline-actions .delete-action {
		color: var(--text-tertiary);
	}

	.inline-actions .delete-action:hover,
	.inline-actions .delete-action:focus-visible,
	.inline-actions .delete-action[aria-expanded='true'],
	.delete-confirm .danger-action {
		color: var(--state-danger);
	}

	.edit-panel,
	.delete-confirm,
	.revision-history {
		margin: 8px 0 0 31px;
	}

	.edit-panel textarea {
		width: min(100%, 680px);
		min-height: 76px;
		resize: vertical;
		padding: 8px 9px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface);
		font: inherit;
		line-height: 1.5;
		color: var(--ink);
	}

	.edit-panel textarea:focus-visible {
		outline: 2px solid color-mix(in srgb, var(--intent-conversation) 32%, transparent);
		outline-offset: 2px;
	}

	.edit-controls,
	.delete-confirm {
		display: flex;
		align-items: center;
		gap: 6px;
	}

	.edit-controls {
		margin-top: 6px;
	}

	.edit-controls .primary-action {
		border-color: color-mix(in srgb, var(--intent-conversation) 24%, var(--border));
		background: var(--intent-conversation-soft);
	}

	.delete-confirm {
		width: fit-content;
		padding: 6px 8px;
		border: 1px solid color-mix(in srgb, var(--state-danger) 20%, var(--border));
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--state-danger) 5%, var(--surface));
		font-size: var(--t-label);
	}

	.action-error,
	.history-state {
		margin: 6px 0 0 31px;
		font-size: var(--t-label);
		color: var(--state-danger);
	}

	.edit-panel .action-error,
	.revision-history .action-error,
	.revision-history .history-state {
		margin-left: 0;
	}

	.history-state {
		color: var(--text-tertiary);
	}

	.revision-history {
		max-height: 280px;
		overflow: auto;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-alt);
	}

	.revision-history article {
		padding: 8px 10px;
		border-bottom: 1px solid var(--border);
	}

	.revision-history article header {
		gap: 8px;
	}

	.revision-history article header time {
		margin-left: auto;
	}

	.revision-history article p {
		margin: 5px 0 0;
		font-size: var(--t-label);
		line-height: 1.5;
	}

	.load-history {
		width: 100%;
		justify-content: center;
		border-radius: 0;
	}

	button:disabled {
		cursor: wait;
		opacity: 0.62;
	}

	@media (max-width: 760px) {
		:is(.message-actions, .inline-actions) button,
		.edit-controls button,
		.delete-confirm button,
		.load-history {
			min-height: 40px;
			padding-inline: 9px;
		}

		.message-actions,
		.edit-panel,
		.delete-confirm,
		.revision-history {
			margin-left: 0;
		}
	}
</style>
