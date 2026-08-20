<script lang="ts">
	/* One contextual conversation rail. It normally belongs to the Exec; while
	 * the owner focuses a review it belongs to that Work's accountable lead.
	 * The rail stays mounted and takes real space rather than nesting another
	 * chat inside the outcome surface. */

	import { tick } from 'svelte';
	import { SvelteDate } from 'svelte/reactivity';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import AttentionRailCard from '$lib/components/AttentionRailCard.svelte';
	import Composer from '$lib/primitives/Composer.svelte';
	import ConversationHistoryTools from '$lib/primitives/ConversationHistoryTools.svelte';
	import ConversationMessage from '$lib/primitives/ConversationMessage.svelte';
	import ConversationTurnDock from '$lib/primitives/ConversationTurnDock.svelte';
	import HoldApprove from '$lib/primitives/HoldApprove.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import SemanticMark from '$lib/primitives/SemanticMark.svelte';
	import type { ActiveConversationTurn } from '$lib/model/conversationSource.svelte';
	import {
		mergeAdjacentAgentMessages,
		type AttentionItem,
		type ThreadMessage
	} from '$lib/model/view';

	let {
		messages = [],
		participantName = 'Exec',
		participantRole = 'Executive',
		turn = null,
		companyId,
		membershipRole,
		connected = false,
		contextLabel = 'Current screen',
		open = true,
		onask = null,
		review = null,
		workContext = null,
		attention = null
	}: {
		messages?: ThreadMessage[];
		participantName?: string;
		participantRole?: string;
		turn?: ActiveConversationTurn | null;
		companyId: string;
		membershipRole: string;
		/**
		 * Whether the executive has a bound ACP runtime. This must come from a LIVE
		 * probe of the runtime, never from configuration — "probe, never guess". Until
		 * it is true the rail is glass-locked instead of implying that conversation is
		 * available. Runtime/provider administration stays outside the owner cockpit.
		 */
		connected?: boolean;
		contextLabel?: string;
		open?: boolean;
		/** Returns delivery feedback. Unwired ordinary chat is inert. */
		onask?:
			| ((
					text: string,
					files: File[],
					includeContext: boolean
			  ) => Promise<{ error?: string; notice?: string }>)
			| null;
		review?: {
			onback: () => void;
			ondecide: (
				decision: 'accept' | 'request_changes',
				feedback: string
			) => Promise<string | null>;
		} | null;
		workContext?: { onback: () => void } | null;
		attention?: AttentionItem | null;
	} = $props();

	const canOperate = $derived(['owner', 'operator'].includes(membershipRole ?? ''));
	const visibleMessages = $derived(mergeAdjacentAgentMessages(messages));

	/* Day separators, same as the thread — computed from the record, so the
	 * rail and the page group the same way. */
	function dayOf(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		return Number.isNaN(date.getTime()) ? '' : date.toDateString();
	}

	function dayLabel(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		if (Number.isNaN(date.getTime())) return '';
		const today = new SvelteDate();
		if (date.toDateString() === today.toDateString()) return 'Today';
		const yesterday = new SvelteDate();
		yesterday.setDate(today.getDate() - 1);
		if (date.toDateString() === yesterday.toDateString()) return 'Yesterday';
		return date.toLocaleDateString(undefined, { month: 'long', day: 'numeric' });
	}

	function messageDomId(messageId: string): string {
		return `rail-message-${companyId}-${messageId.replaceAll(':', '-')}`;
	}

	function jumpToMessage(messageId: string) {
		const message = document.getElementById(messageDomId(messageId));
		if (!message) return;
		const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
		message.scrollIntoView({ behavior: reduceMotion ? 'auto' : 'smooth', block: 'center' });
		if (!reduceMotion) {
			message.animate(
				[
					{
						boxShadow:
							'inset 2px 0 0 color-mix(in srgb, var(--intent-conversation) 0%, transparent)'
					},
					{ boxShadow: 'inset 2px 0 0 var(--intent-conversation)' },
					{
						boxShadow:
							'inset 2px 0 0 color-mix(in srgb, var(--intent-conversation) 0%, transparent)'
					}
				],
				{ duration: 900, easing: 'cubic-bezier(0.23, 1, 0.32, 1)' }
			);
		}
	}

	let composer = $state('');
	let composerFiles = $state<File[]>([]);
	let includeContext = $state(true);
	let contextFlare = $state(0);
	let askError = $state('');
	let askNotice = $state('');
	let reviewError = $state('');
	let deciding = $state(false);
	let scrollEl = $state<HTMLDivElement | undefined>();
	let anchoredTurnId = $state<number | null>(null);
	let transcriptTailHeight = $state(0);
	let initiallyScrolledFor = $state('');
	let railView = $state<'chat' | 'attention'>('chat');
	let attentionId = $state('');

	function toggleContext() {
		includeContext = !includeContext;
		if (includeContext) contextFlare += 1;
	}

	/* Opening a different review starts with its conversation. Switching the live
	 * projection underneath the same review does not steal the owner's chosen tab. */
	$effect(() => {
		const nextAttentionId = attention?.id ?? '';
		if (attentionId === nextAttentionId) return;
		attentionId = nextAttentionId;
		railView = 'chat';
	});

	async function anchorSubmittedMessage(messageId: number) {
		await tick();
		const scroller = scrollEl;
		const message = document.getElementById(messageDomId(String(messageId)));
		if (!scroller || !message || anchoredTurnId !== messageId) return;

		/* Leave one transcript-height of runway beneath the owner's message. The
		 * live turn and durable reply can then grow below it without moving the
		 * question away from the top of the reading area. */
		transcriptTailHeight = Math.max(0, scroller.clientHeight - message.offsetHeight);
		await tick();
		const top =
			message.getBoundingClientRect().top -
			scroller.getBoundingClientRect().top +
			scroller.scrollTop;
		const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
		scroller.scrollTo({ top, behavior: reduceMotion ? 'auto' : 'smooth' });
	}

	/* A send creates a new reading viewport. Do this once per turn; subsequent
	 * stream updates must not fight the owner's scroll position. */
	$effect(() => {
		const messageId = turn?.triggerMessageId ?? null;
		if (!open || railView !== 'chat' || messageId === null || anchoredTurnId === messageId) return;
		const firstMessageId = messages[0]?.id;
		if (firstMessageId) initiallyScrolledFor = `${companyId}:${participantName}:${firstMessageId}`;
		anchoredTurnId = messageId;
		void anchorSubmittedMessage(messageId);
	});

	/* Existing conversations open at their latest message. This is hydration
	 * behavior only; it deliberately does not run for every new message. */
	$effect(() => {
		const firstMessageId = messages[0]?.id;
		if (!open || railView !== 'chat' || !firstMessageId || turn) return;
		const conversationKey = `${companyId}:${participantName}:${firstMessageId}`;
		if (initiallyScrolledFor === conversationKey) return;
		initiallyScrolledFor = conversationKey;
		anchoredTurnId = null;
		transcriptTailHeight = 0;
		void tick().then(() => scrollEl?.scrollTo({ top: scrollEl.scrollHeight }));
	});

	const attachmentHref = (attachment: { uploadId: string }) =>
		`/api/companies/${encodeURIComponent(companyId)}/attachments/${encodeURIComponent(attachment.uploadId)}`;

	let sending = $state(false);
	async function submitAsk(event: SubmitEvent) {
		event.preventDefault();
		const text = composer.trim();
		if (!text || sending || deciding || !onask) return;
		sending = true;
		askError = '';
		askNotice = '';
		const sent = composer;
		const files = composerFiles;
		composer = '';
		try {
			const outcome = await onask(text, files, includeContext);
			if (outcome.error) {
				composer = sent;
				askError = outcome.error;
			} else {
				composerFiles = [];
				askNotice = outcome.notice ?? '';
			}
		} catch (cause) {
			composer = sent;
			askError = cause instanceof Error ? cause.message : 'Your message was not delivered.';
		} finally {
			sending = false;
		}
	}

	async function acceptReview() {
		if (!review || deciding) return;
		deciding = true;
		reviewError = '';
		try {
			const failure = await review.ondecide('accept', '');
			if (failure) reviewError = failure;
			else composer = '';
		} finally {
			deciding = false;
		}
	}
</script>

<aside
	id="bridge-exrail"
	class="bridge-exrail"
	class:open
	aria-label={railView === 'attention' && attention
		? `Attention: ${attention.title}`
		: `${participantName} conversation`}
	aria-hidden={!open}
	inert={!open}
>
	<div class="exr-inner">
		<header class="exr-head" class:contextual={!!attention}>
			<div class="exr-head-primary">
				{#if review || workContext}
					<button
						class="rail-back"
						type="button"
						aria-label="Back to Attention"
						title="Back to Attention"
						onclick={(review ?? workContext)!.onback}
					>
						<ArrowLeft size={18} aria-hidden="true" />
					</button>
				{/if}
				<div class="exr-who">
					<SemanticMark meaning={attention ? 'work' : 'executive'} />
					<span>
						<strong class="exr-name">{participantName}</strong>
					</span>
				</div>
				{#if visibleMessages.length}
					<ConversationHistoryTools
						messages={visibleMessages}
						{participantName}
						onjump={jumpToMessage}
					/>
				{/if}
			</div>
			{#if attention}
				<div class="review-controls">
					<div
						class="rail-view-switch"
						class:attention-active={railView === 'attention'}
						role="group"
						aria-label="Right sidebar view"
					>
						<button
							type="button"
							class:active={railView === 'chat'}
							aria-pressed={railView === 'chat'}
							onclick={() => (railView = 'chat')}>Chat</button
						>
						<button
							type="button"
							class:active={railView === 'attention'}
							aria-pressed={railView === 'attention'}
							onclick={() => (railView = 'attention')}>Attention</button
						>
					</div>
					{#if review}
						<HoldApprove
							small
							label={deciding ? 'Recording…' : 'Hold to accept'}
							title="Hold to accept outcome"
							completeLabel="accepted ✓"
							disabled={deciding || sending}
							onapprove={() => void acceptReview()}
						/>
					{/if}
				</div>
			{/if}
		</header>
		{#if reviewError}<p class="review-error" role="alert">{reviewError}</p>{/if}

		<div class="exr-panel">
			{#if attention && railView === 'attention'}
				<AttentionRailCard item={attention} />
			{:else}
				{#if !connected}
					<div class="exr-lock">
						<div class="exr-lock-card">
							<span class="exr-lock-badge" aria-hidden="true"
								><MatrixGlyph rows={GLYPHS.e} size={18} glow /></span
							>
							<h2 class="exr-lock-h">{participantRole} unavailable</h2>
							<p class="exr-lock-p">
								The company computer has not confirmed that {participantName} is reachable. Conversation
								will open automatically when the live connection returns.
							</p>
							<p class="exr-lock-note">Connection is managed by the company computer.</p>
						</div>
					</div>
				{/if}
				<div class="exr-chat" inert={!connected}>
					<div class="exr-msgs" bind:this={scrollEl}>
						{#each visibleMessages as message, i (message.id)}
							{#if i === 0 || dayOf(message.createdAt) !== dayOf(visibleMessages[i - 1].createdAt)}
								<div class="day-sep" aria-hidden="true">
									<span>{dayLabel(message.createdAt)}</span>
								</div>
							{/if}
							<ConversationMessage
								domId={messageDomId(message.id)}
								sender={message.from === 'you' ? 'owner' : message.from}
								author={message.from === 'you' ? 'You' : message.author || participantName}
								text={message.text}
								createdAt={message.createdAt}
								details={message.details}
								attachments={message.attachments}
								hrefFor={attachmentHref}
							/>
						{:else}
							{#if review || workContext}
								<div class="exr-empty review-empty">
									<div class="review-empty-card">
										<span class="review-empty-mark">
											<MatrixGlyph rows={GLYPHS.work} size={12} />
										</span>
										<div>
											<strong>Talk to the lead</strong>
											<p>Ask questions, discuss evidence, or share revision feedback.</p>
										</div>
									</div>
								</div>
							{:else}
								<div class="exr-empty">
									<p class="exr-empty-h">Ask anything.</p>
									<p class="exr-empty-p">
										The executive reads the whole company record before it answers — goals, staff,
										work in flight, and what needs your word.
									</p>
								</div>
							{/if}
						{/each}
						{#if turn}<ConversationTurnDock {participantName} {turn} />{/if}
						{#if anchoredTurnId !== null}
							<div
								class="conversation-tail"
								style:height={`${transcriptTailHeight}px`}
								aria-hidden="true"
							></div>
						{/if}
					</div>

					<form class="exr-composer" onsubmit={submitAsk}>
						<Composer
							bind:value={composer}
							bind:files={composerFiles}
							disabled={!canOperate || sending || deciding || !onask}
							minlength={1}
							placeholder={review || workContext
								? 'Message the lead…'
								: 'Ask, redirect, or make a judgement…'}
							ariaLabel={review || workContext
								? `Message ${participantName}`
								: `Ask ${participantName}`}
							flareKey={contextFlare}
						>
							{#snippet controls()}
								{#if !review && !workContext}
									<div class="exec-context-line">
										<button
											type="button"
											class="exec-context-chip"
											class:off={!includeContext}
											aria-pressed={includeContext}
											title="Link this message to the current screen"
											onclick={toggleContext}
										>
											<MatrixGlyph rows={GLYPHS.work} size={8} />
											<span>{includeContext ? contextLabel : 'Link current screen'}</span>
										</button>
									</div>
								{/if}
							{/snippet}
						</Composer>
						{#if askError}
							<p class="exr-error" role="alert">{askError}</p>
						{/if}
						{#if askNotice}
							<p class="exr-notice" role="status">{askNotice}</p>
						{/if}
					</form>
				</div>
			{/if}
		</div>
	</div>
</aside>

<style>
	/* One column that fills the rail. The explicit track is load-bearing:
	 * cockpit.css sets `justify-content: space-between` on .exr-head, which in a
	 * grid would size an implicit auto track to max-content and leave the header
	 * visibly narrower than the transcript beneath it. */
	.exr-head.contextual {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: 10px;
	}
	.exr-head-primary {
		width: 100%;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 10px;
	}
	.exr-head-primary .exr-who {
		min-width: 0;
		flex: 1 1 auto;
	}
	.exr-panel {
		position: relative;
		min-height: 0;
		flex: 1 1 auto;
		display: flex;
		flex-direction: column;
	}
	.exr-chat {
		min-height: 0;
		flex: 1 1 auto;
		display: flex;
		flex-direction: column;
	}
	.conversation-tail {
		width: 1px;
		flex: 0 0 auto;
		pointer-events: none;
	}
	.exr-panel > .exr-lock {
		width: 100%;
	}
	.rail-back {
		width: 34px;
		height: 34px;
		flex: none;
		display: grid;
		place-items: center;
		padding: 0;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: rgba(255, 255, 255, 0.58);
		color: var(--text-secondary);
		cursor: pointer;
	}
	.rail-back:focus-visible {
		outline: 3px solid color-mix(in srgb, var(--intent-conversation) 30%, transparent);
		outline-offset: 2px;
	}
	.review-controls {
		flex: none;
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.contextual .review-controls {
		width: 100%;
		justify-content: space-between;
	}
	.review-controls :global(.hold-approve) {
		min-width: 122px;
		white-space: nowrap;
	}
	.rail-view-switch {
		position: relative;
		isolation: isolate;
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		padding: 2px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--surface-alt) 82%, white);
		box-shadow: var(--control-depth-pressed);
	}
	.rail-view-switch::before {
		content: '';
		position: absolute;
		z-index: 0;
		top: 2px;
		bottom: 2px;
		left: 2px;
		width: calc((100% - 4px) / 2);
		border: 1px solid color-mix(in srgb, var(--intent-conversation) 16%, var(--control-edge));
		border-radius: calc(var(--radius-control) - 2px);
		background: var(--surface);
		box-shadow: var(--control-depth);
		transform: translateX(0);
		transition: transform var(--motion-disclosure) var(--ease-spring);
	}
	.rail-view-switch.attention-active::before {
		transform: translateX(100%);
	}
	.rail-view-switch button {
		position: relative;
		z-index: 1;
		min-width: 68px;
		padding: 4px 9px;
		border: 0;
		border-radius: calc(var(--radius-control) - 2px);
		background: transparent;
		color: var(--text-secondary);
		font: 600 var(--t-label) var(--font-ui);
		cursor: pointer;
		transition:
			color var(--motion-state) var(--ease-standard),
			transform var(--motion-press) var(--ease-standard);
	}
	.rail-view-switch button.active {
		background: transparent;
		box-shadow: none;
		color: var(--ink);
	}
	.rail-view-switch button:active {
		transform: translateY(1px);
	}
	.rail-view-switch button:focus-visible {
		outline: 3px solid color-mix(in srgb, var(--intent-conversation) 30%, transparent);
		outline-offset: 2px;
	}
	.exr-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.review-error {
		margin: 0;
		padding: 9px 14px;
		border-bottom: 1px solid color-mix(in srgb, var(--danger) 28%, var(--border));
		background: color-mix(in srgb, var(--danger) 6%, white);
		font-size: var(--t-label);
		line-height: 1.4;
		color: var(--danger);
	}
	.review-empty {
		width: calc(100% - 36px);
		max-width: 360px;
		padding: 0;
		text-align: left;
	}
	.review-empty-card {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		align-items: center;
		gap: 12px;
		padding: 14px 15px;
		border: 1px solid color-mix(in srgb, var(--intent-feedback) 34%, var(--border));
		border-left: 3px solid var(--intent-feedback);
		border-radius: var(--radius-pane);
		background: linear-gradient(
			145deg,
			color-mix(in srgb, var(--intent-feedback-soft) 82%, white),
			color-mix(in srgb, var(--intent-feedback-soft) 58%, var(--surface-alt))
		);
		box-shadow:
			var(--bevel),
			0 2px 0 color-mix(in srgb, var(--intent-feedback) 10%, transparent),
			0 12px 28px rgba(35, 117, 99, 0.12);
	}
	.review-empty-mark {
		width: 38px;
		height: 38px;
		display: grid;
		place-items: center;
		border: 1px solid color-mix(in srgb, var(--intent-feedback) 28%, var(--border));
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--intent-feedback-soft) 76%, white);
		box-shadow:
			inset 0 1px 0 rgba(255, 255, 255, 0.88),
			0 3px 8px rgba(35, 117, 99, 0.09);
		color: var(--intent-feedback);
	}
	.review-empty-card strong {
		display: block;
		color: var(--ink);
		font-size: var(--t-body);
		font-weight: 600;
		line-height: 1.3;
	}
	.review-empty-card p {
		margin: 3px 0 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
		line-height: 1.45;
	}
</style>
