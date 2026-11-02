<script lang="ts" module>
	/** A comment being written in the margin, beside the words chosen for it. */
	export interface MarginDraft {
		blockId: string;
		quote: string | null;
		range: { from: string; to: string } | null;
		top: number;
	}
</script>

<script lang="ts">
	/* Comments in the margin (Sprint 109 E4): each open thread is a card beside the words it is
	 * about, level with them unless another card is already there. A card is one fixed-height row
	 * set until it is opened; opened, it shows the thread with a reply field and Resolve in place.
	 * The comments panel stays the place for history, reactions and edits. */
	import ExternalLink from '@lucide/svelte/icons/panel-right-open';
	import Check from '@lucide/svelte/icons/check';
	import { untrack } from 'svelte';
	import { composing } from '$lib/primitives/composer-keys';
	import { failureSentence } from '$lib/model/failure';
	import {
		commentContent,
		createDocumentCommentThread,
		replyToDocumentComment,
		resolveDocumentCommentThread,
		type DocumentCommentThreadView
	} from '$lib/model/documents';
	import {
		documentCommentThreadsQuery,
		documentCommentsQuery
	} from '$lib/model/document-queries.svelte';
	import { MARGIN_CARD_HEIGHT, stackMarginNotes } from '$lib/model/document-margin';

	interface Props {
		companyId: string;
		documentId: string;
		/** Open threads on blocks, in the order they began. */
		threads: readonly DocumentCommentThreadView[];
		/** How far down the page each thread's words are, by thread; a thread not here is not shown. */
		anchors: ReadonlyMap<string, number>;
		/** The card that is opened (pinned level with its words), or null. */
		active: string | null;
		/** A comment being written, or null. */
		draft: MarginDraft | null;
		canComment: boolean;
		online: boolean;
		nameOf: (actorId: string) => string;
		/** Open a card (or close it, with null). */
		onactivate: (threadId: string | null) => void;
		/** A card is hovered or focused: its words are marked more strongly. */
		onhover: (threadId: string | null) => void;
		/** The draft was sent (with the thread it began) or put away. */
		ondraftdone: (threadId: string | null) => void;
		/** The whole thread in the comments panel: history, reactions, edits. */
		onopenpanel: (threadId: string) => void;
	}

	let {
		companyId,
		documentId,
		threads,
		anchors,
		active,
		draft,
		canComment,
		online,
		nameOf,
		onactivate,
		onhover,
		ondraftdone,
		onopenpanel
	}: Props = $props();

	const DRAFT = 'margin-draft';
	const cache = documentCommentThreadsQuery(
		() => companyId,
		() => documentId
	);
	const activeThread = $derived(threads.find((item) => item.thread.id === active) ?? null);
	const comments = documentCommentsQuery(
		() => companyId,
		() => documentId,
		() => activeThread?.thread.id ?? ''
	);

	/* The opened card and the draft grow with what they hold; every other card is one height. */
	let openedHeight = $state(MARGIN_CARD_HEIGHT);
	let draftHeight = $state(MARGIN_CARD_HEIGHT);
	const shown = $derived(threads.filter((item) => anchors.has(item.thread.id)));
	const tops = $derived(
		stackMarginNotes(
			[
				...shown.map((item) => ({
					id: item.thread.id,
					anchor: anchors.get(item.thread.id) ?? 0,
					height: item.thread.id === active ? openedHeight : MARGIN_CARD_HEIGHT
				})),
				...(draft ? [{ id: DRAFT, anchor: draft.top, height: draftHeight }] : [])
			],
			{ gap: 8, active: draft ? DRAFT : active }
		)
	);

	/* Writing: a new thread from the draft, or a reply in the opened card. */
	let draftText = $state('');
	let replyText = $state('');
	let busy = $state(false);
	let failure = $state('');
	/* One command per thing being sent, so a retry after a dropped answer is not sent twice. */
	let draftCommand = '';
	let replyCommand = '';
	let draftInput = $state<HTMLTextAreaElement>();
	$effect(() => {
		if (!draft) return;
		untrack(() => {
			draftText = '';
			failure = '';
			draftCommand = crypto.randomUUID();
		});
		queueMicrotask(() => draftInput?.focus({ preventScroll: true }));
	});
	$effect(() => {
		void active;
		untrack(() => {
			replyText = '';
			failure = '';
			replyCommand = '';
		});
	});

	const content = (text: string) => commentContent(text, new Set());

	async function sendDraft(): Promise<void> {
		const body = draftText.trim();
		if (!draft || !body || busy || !online || !canComment) return;
		busy = true;
		failure = '';
		const target = { companyId, documentId };
		try {
			const created = await createDocumentCommentThread(
				companyId,
				documentId,
				draft.blockId,
				content(body),
				draftCommand || crypto.randomUUID(),
				draft.quote,
				draft.range
			);
			cache.acceptCreated(target, created);
			draftText = '';
			ondraftdone(created.thread.thread.id);
		} catch (cause) {
			failure = failureSentence(cause, 'The comment was not saved.');
		} finally {
			busy = false;
		}
	}

	async function sendReply(): Promise<void> {
		const body = replyText.trim();
		const thread = activeThread;
		if (!thread || !body || busy || !online || !canComment) return;
		busy = true;
		failure = '';
		replyCommand ||= crypto.randomUUID();
		try {
			const comment = await replyToDocumentComment(
				companyId,
				documentId,
				thread.thread.id,
				null,
				content(body),
				replyCommand
			);
			comments.accept({ companyId, documentId, threadId: thread.thread.id }, comment);
			replyText = '';
			replyCommand = '';
			void cache.refresh();
		} catch (cause) {
			failure = failureSentence(cause, 'The reply was not saved.');
		} finally {
			busy = false;
		}
	}

	async function resolve(): Promise<void> {
		const thread = activeThread;
		if (!thread || busy || !online || !canComment) return;
		busy = true;
		failure = '';
		try {
			const result = await resolveDocumentCommentThread(
				companyId,
				documentId,
				thread.thread.id,
				thread.thread.version,
				crypto.randomUUID()
			);
			cache.acceptResolved({ companyId, documentId }, result);
			onactivate(null);
		} catch (cause) {
			failure = failureSentence(cause, 'The thread was not resolved.');
		} finally {
			busy = false;
		}
	}

	function writingKeys(event: KeyboardEvent, send: () => void, cancel: () => void): void {
		if (event.key === 'Escape') {
			event.preventDefault();
			cancel();
		} else if (event.key === 'Enter' && (event.metaKey || event.ctrlKey) && !composing(event)) {
			event.preventDefault();
			send();
		}
	}

	const plural = (count: number) => (count === 1 ? '1 comment' : `${count} comments`);
</script>

<aside class="comment-margin" aria-label="Comments in the margin">
	{#each shown as item (item.thread.id)}
		{@const id = item.thread.id}
		{@const opening = item.opening}
		{@const opened = id === active}
		{@const top = tops.get(id)}
		{#if top !== undefined}
			<div
				class="margin-card"
				class:opened
				style:top={`${top}px`}
				data-margin-thread={id}
				role="group"
				aria-label={`Comment by ${opening ? nameOf(opening.author_actor_id) : 'someone'}`}
				onmouseenter={() => onhover(id)}
				onmouseleave={() => onhover(null)}
				onfocusin={() => onhover(id)}
				onfocusout={() => onhover(null)}
			>
				{#if opened}
					<div class="opened-body" bind:offsetHeight={openedHeight}>
						<div class="card-head">
							<span class="count"
								>{plural(Math.max(comments.comments.length, opening?.comment_count ?? 1))}</span
							>
							<span class="card-tools">
								{#if canComment}
									<button
										type="button"
										class="icon"
										aria-label="Resolve"
										title="Resolve this thread"
										disabled={busy || !online}
										onclick={() => void resolve()}><Check size={14} strokeWidth={2} /></button
									>
								{/if}
								<button
									type="button"
									class="icon"
									aria-label="Open in the comments panel"
									title="History, reactions and edits in the comments panel"
									onclick={() => onopenpanel(id)}
									><ExternalLink size={14} strokeWidth={1.8} /></button
								>
							</span>
						</div>
						<ol class="thread" aria-label="Comments">
							{#each comments.comments.toReversed() as comment (comment.id)}
								<li>
									<span class="who">{nameOf(comment.author_actor_id)}</span>
									<p>{comment.plain_text}</p>
								</li>
							{:else}
								<li>
									<span class="who">{opening ? nameOf(opening.author_actor_id) : ''}</span>
									<p>{opening?.excerpt ?? ''}</p>
								</li>
							{/each}
						</ol>
						{#if canComment}
							<form
								class="reply"
								onsubmit={(event) => {
									event.preventDefault();
									void sendReply();
								}}
							>
								<textarea
									aria-label="Reply"
									placeholder="Reply"
									rows="2"
									bind:value={replyText}
									onkeydown={(event) =>
										writingKeys(
											event,
											() => void sendReply(),
											() => onactivate(null)
										)}></textarea>
								<div class="actions">
									<button type="button" class="btn small" onclick={() => onactivate(null)}
										>Close</button
									>
									<button
										type="submit"
										class="btn small primary"
										disabled={busy || !online || !replyText.trim()}>Reply</button
									>
								</div>
							</form>
						{:else}
							<p class="read-only">You can read comments. Ask for comment access to reply.</p>
						{/if}
						{#if failure}<p class="failure" role="alert">{failure}</p>{/if}
					</div>
				{:else}
					<button
						type="button"
						class="card-summary"
						aria-expanded="false"
						onclick={() => onactivate(id)}
					>
						<span class="card-head">
							<span class="author">{opening ? nameOf(opening.author_actor_id) : 'Comment'}</span>
							{#if (opening?.comment_count ?? 1) > 1}
								<span class="count">{plural(opening?.comment_count ?? 1)}</span>
							{/if}
						</span>
						<span class="excerpt">{opening?.excerpt ?? ''}</span>
					</button>
				{/if}
			</div>
		{/if}
	{/each}
	{#if draft && tops.get(DRAFT) !== undefined}
		<form
			class="margin-card opened draft"
			style:top={`${tops.get(DRAFT)}px`}
			aria-label="New comment"
			bind:offsetHeight={draftHeight}
			onsubmit={(event) => {
				event.preventDefault();
				void sendDraft();
			}}
		>
			{#if draft.quote}<p class="quoted" title={draft.quote}>{draft.quote}</p>{/if}
			<textarea
				aria-label="Comment"
				placeholder="Comment, then Ctrl+Enter"
				rows="3"
				bind:this={draftInput}
				bind:value={draftText}
				onkeydown={(event) =>
					writingKeys(
						event,
						() => void sendDraft(),
						() => ondraftdone(null)
					)}></textarea>
			<div class="actions">
				<button type="button" class="btn small" onclick={() => ondraftdone(null)}>Cancel</button>
				<button
					type="submit"
					class="btn small primary"
					disabled={busy || !online || !draftText.trim()}>Comment</button
				>
			</div>
			{#if failure}<p class="failure" role="alert">{failure}</p>{/if}
		</form>
	{/if}
</aside>

<style>
	.comment-margin {
		position: absolute;
		top: 0;
		right: var(--doc-margin-inset);
		width: var(--doc-margin-width);
		height: 0;
		z-index: 2;
	}
	.margin-card {
		position: absolute;
		left: 0;
		width: 100%;
		box-sizing: border-box;
		border: 1px solid var(--border);
		border-radius: var(--radius-md, 8px);
		background: var(--surface-raised);
		color: var(--ink);
		font: 400 var(--t-body) / 1.4 var(--font-ui);
		transition: top 160ms ease;
	}
	.margin-card:hover,
	.margin-card:focus-within {
		border-color: color-mix(in srgb, var(--highlight) 90%, var(--surface-attention));
	}
	.margin-card.opened {
		box-shadow: var(--shadow-popover, 0 4px 16px rgba(0, 0, 0, 0.12));
		z-index: 1;
	}
	/* Rows never grow: a closed card is one head row and two lines of what was said. */
	.card-summary {
		display: grid;
		grid-template-rows: 20px 40px;
		gap: 4px;
		width: 100%;
		height: 82px;
		padding: 8px 10px;
		border: 0;
		background: transparent;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.card-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 6px;
		min-width: 0;
		height: 20px;
	}
	.author {
		overflow: hidden;
		font-weight: 600;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.count {
		flex: none;
		color: var(--text-tertiary);
		font: 500 var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
	}
	.excerpt {
		display: -webkit-box;
		overflow: hidden;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		color: var(--text-secondary);
		overflow-wrap: anywhere;
	}
	.opened-body,
	.draft {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 8px 10px;
	}
	.card-tools {
		display: inline-flex;
		gap: 2px;
	}
	.icon {
		display: inline-grid;
		place-items: center;
		width: 24px;
		height: 24px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
	}
	.icon:hover,
	.icon:focus-visible {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.thread {
		display: flex;
		flex-direction: column;
		gap: 8px;
		max-height: 320px;
		margin: 0;
		padding: 0;
		overflow: auto;
		list-style: none;
	}
	.thread .who {
		font-weight: 600;
	}
	.thread p,
	.read-only,
	.failure,
	.quoted {
		margin: 0;
		overflow-wrap: anywhere;
	}
	.read-only {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.failure {
		color: var(--state-danger);
		font-size: var(--t-label);
	}
	.quoted {
		overflow: hidden;
		padding-left: 8px;
		border-left: 2px solid color-mix(in srgb, var(--highlight) 90%, var(--surface-attention));
		color: var(--text-secondary);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.reply {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	textarea {
		width: 100%;
		box-sizing: border-box;
		resize: vertical;
		padding: 6px 8px;
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		background: var(--surface-pane);
		color: var(--ink);
		font: inherit;
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 6px;
	}
	@media (prefers-reduced-motion: reduce) {
		.margin-card {
			transition: none;
		}
	}
	@media print {
		.comment-margin {
			display: none;
		}
	}
</style>
