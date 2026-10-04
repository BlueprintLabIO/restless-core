<script lang="ts">
	import { failureSentence } from '$lib/model/failure';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	/* One contextual conversation rail. It normally belongs to the Exec; while
	 * the owner focuses a review it belongs to that Work's accountable lead.
	 * The rail stays mounted and takes real space rather than nesting another
	 * chat inside the outcome surface. */

	import { followChat } from '$lib/actions/follow-chat';
	import IntelligenceChip from './IntelligenceChip.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import Check from '@lucide/svelte/icons/check';
	import { SvelteDate } from 'svelte/reactivity';
	import Plus from '@lucide/svelte/icons/plus';
	import RotateCw from '@lucide/svelte/icons/rotate-cw';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import Composer from '$lib/primitives/Composer.svelte';
	import ConversationHistoryTools from '$lib/primitives/ConversationHistoryTools.svelte';
	import ConversationMessage from '$lib/primitives/ConversationMessage.svelte';
	import ConversationTurnDock from '$lib/primitives/ConversationTurnDock.svelte';
	import HoldApprove from '$lib/ui/controls/HoldApprove.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/ui/glyph/MatrixGlyph.svelte';
	import SemanticMark from '$lib/ui/glyph/SemanticMark.svelte';
	import type { ActiveAgentTurn, QuerySourceStatus } from '$lib/model/queries.svelte';
	import type { OutcomeStandard } from '$lib/model/company';
	import type { ThreadMessage } from '$lib/model/view';
	import { runExecCommand } from '$lib/model/skills';
	import { reactionsQuery } from '$lib/model/reactions.svelte';
	import { composerOptions, readDraft, writeDraft } from '$lib/model/composer-options.svelte';

	let {
		messages = [],
		participantName = 'Exec',
		participantId = 'exec',
		participantRole = 'Executive',
		turn = null,
		companyId,
		membershipRole,
		connected = false,
		connectionStatus = 'unknown',
		conversationStatus = 'unknown',
		conversationFailed = false,
		onrefreshConversation = null,
		needsProvider = false,
		providerLabel = 'Connect intelligence',
		contextLabel = 'Current screen',
		focusAfterMessageId = 0,
		focusStartedAt = null,
		newFocusAvailable = false,
		open = true,
		onask = null,
		review = null,
		workContext = null,
		topicLabel = '',
		topics = [],
		currentTopicKey = 'general',
		ontopic = null,
		viewerActorId = 'owner',
		references = []
	}: {
		messages?: ThreadMessage[];
		participantName?: string;
		participantId?: string;
		participantRole?: string;
		turn?: ActiveAgentTurn | null;
		companyId: string;
		membershipRole: string;
		/** Whether this actor has a usable conversation route in the latest cockpit view. */
		connected?: boolean;
		/** Separates confirmed unavailability from an unknown or failed status read. */
		connectionStatus?: 'unknown' | 'error' | 'unavailable' | 'available';
		conversationStatus?: QuerySourceStatus;
		conversationFailed?: boolean;
		onrefreshConversation?: (() => void) | null;
		needsProvider?: boolean;
		providerLabel?: string;
		contextLabel?: string;
		focusAfterMessageId?: number;
		focusStartedAt?: string | null;
		newFocusAvailable?: boolean;
		open?: boolean;
		/** Returns delivery feedback. Unwired ordinary chat is inert. */
		onask?:
			| ((
					text: string,
					files: File[],
					includeContext: boolean,
					newFocus: boolean,
					interrupt: boolean,
					outcomeStandard?: OutcomeStandard,
					skills?: string[]
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
		/** What this conversation is about: General, or one piece of Work. */
		topicLabel?: string;
		topics?: {
			key: string;
			label: string;
			hint: string;
			topic: { actorId: string; workId?: string } | null;
		}[];
		currentTopicKey?: string;
		ontopic?: ((topic: { actorId: string; workId?: string } | null) => void) | null;
		viewerActorId?: string;
		/** Work, Goals and people the composer offers after `#` and `@`. */
		references?: import('$lib/model/skills').ComposerOption[];
	} = $props();
	const reactions = reactionsQuery(
		() => companyId,
		() =>
			messages
				.map((message) => messageNumericId(message.id))
				.filter((id) => id > 0)
				.slice(-60),
		() => viewerActorId
	);

	const canOperate = $derived(['owner', 'operator'].includes(membershipRole ?? ''));

	/* `$skill` selection and the `/goal` and `/loop` commands. Skills are the
	 * company library; the commands are Restless primitives offered only in the
	 * Exec conversation, where they have one accountable meaning. */
	let composerSkills = $state<string[]>([]);
	const execConversation = $derived(participantId === 'exec' && !review && !workContext);
	const commandsAllowed = $derived(execConversation && membershipRole === 'owner');
	const options = composerOptions(
		() => companyId,
		() => open && canOperate,
		() => commandsAllowed
	);

	/* Returns a notice for commands handled without a message, or send: true. */
	async function runCommand(
		text: string
	): Promise<{ notice?: string; error?: string; send: boolean }> {
		return commandsAllowed ? runExecCommand(companyId, text) : { send: true };
	}

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
		scrollEl?.dispatchEvent(new Event('chat-scroll-pause'));
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
	/* The draft belongs to this company and this recipient, and survives a reload. */
	const draftKey = $derived(`restless:${companyId}:rail-draft:${participantId}:${currentTopicKey}`);
	let loadedDraftKey = '';
	$effect(() => {
		const key = draftKey;
		if (key === loadedDraftKey) return;
		loadedDraftKey = key;
		composer = readDraft(key);
	});
	$effect(() => {
		const key = draftKey,
			body = composer;
		if (key !== loadedDraftKey) return;
		const timer = setTimeout(() => writeDraft(key, body), 150);
		return () => clearTimeout(timer);
	});
	let composerFiles = $state<File[]>([]);
	let includeContext = $state(true);
	let contextFlare = $state(0);
	let askError = $state('');
	let askNotice = $state('');
	let reviewError = $state('');
	let reviewFeedback = $state('');
	let deciding = $state(false);
	let scrollEl = $state<HTMLDivElement | undefined>();
	let scrollReset = $state(0);
	let newFocusPending = $state(false);
	let pendingFocusAfterMessageId = $state(0);
	let composerFocusKey = $state(0);

	/* What the owner has already seen here, per company and recipient, so new
	 * replies start under a "New" rule. Read once per conversation; marked as
	 * seen while the rail is open. */
	const seenKey = $derived(`restless:${companyId}:rail-seen:${participantId}:${currentTopicKey}`);
	let seenThrough = $state<number | null>(null);
	let seenLoadedFor = '';
	$effect(() => {
		const key = seenKey;
		if (key === seenLoadedFor || !messages.length) return;
		seenLoadedFor = key;
		let stored = 0;
		try {
			stored = Number(localStorage.getItem(key) ?? 0) || 0;
		} catch {
			/* Without storage there is no rule; the conversation still reads. */
		}
		seenThrough = stored;
	});
	$effect(() => {
		const key = seenKey,
			latest = messages.reduce((max, m) => Math.max(max, messageNumericId(m.id)), 0);
		if (!open || !latest || key !== seenLoadedFor) return;
		try {
			localStorage.setItem(key, String(latest));
		} catch {
			/* See above. */
		}
	});
	function unreadRuleBefore(index: number): boolean {
		if (!seenThrough) return false;
		const current = visibleMessages[index];
		if (!current || current.from === 'you' || messageNumericId(current.id) <= seenThrough)
			return false;
		const previous = visibleMessages[index - 1];
		return !previous || messageNumericId(previous.id) <= seenThrough;
	}
	function shortTime(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		return Number.isNaN(date.getTime())
			? ''
			: date.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
	}
	/* The agent's open question, when its latest word asks for one. Replying
	 * answers it; the Inbox lists the same question until then. */
	const openQuestion = $derived.by(() => {
		const last = visibleMessages.at(-1);
		return last && last.from !== 'you' ? (last.intent?.ownerNeed?.trim() ?? '') : '';
	});

	const activeFocusAfterMessageId = $derived(
		newFocusPending ? pendingFocusAfterMessageId : focusAfterMessageId
	);
	const focusActive = $derived(newFocusPending || focusStartedAt !== null);
	// Each durable reply keeps its own timestamp, intent, and navigation target.
	const visibleMessages = $derived(messages);
	/* Mirrors the transcript's empty-state branch: only there does the
	 * connect action appear beside its explanation. */
	const providerPromptInline = $derived(
		visibleMessages.length === 0 &&
			conversationStatus !== 'unknown' &&
			!focusActive &&
			!review &&
			!workContext
	);
	const hasMessagesAfterFocus = $derived(
		focusActive &&
			messages.some((message) => messageNumericId(message.id) > activeFocusAfterMessageId)
	);
	const capabilityHint = $derived.by(() => {
		if (participantId !== 'exec')
			return `Ask ${participantName} about the work they own; the page you are on is linked.`;
		if (contextLabel.includes('Work')) return 'Exec can inspect current Work before answering.';
		if (contextLabel.includes('People'))
			return 'Exec can route a question through the accountable lead.';
		if (contextLabel.includes('Company'))
			return 'Exec can compare a proposal with current company direction.';
		return 'Exec can inspect Work, compare options, or bring in a fresh critic.';
	});

	function messageNumericId(messageId: string): number {
		const value = Number(messageId.split(':').at(-1));
		return Number.isFinite(value) ? value : 0;
	}

	function firstMessageNumericId(messageId: string): number {
		const value = Number(messageId.split(':')[0]);
		return Number.isFinite(value) ? value : 0;
	}

	/* One header for a run: same author, same day, within five minutes, and
	 * no focus boundary between them. */
	function continuesRun(index: number): boolean {
		if (index === 0 || focusDividerBefore(index)) return false;
		const previous = visibleMessages[index - 1];
		const current = visibleMessages[index];
		return (
			previous.from === current.from &&
			dayOf(previous.createdAt) === dayOf(current.createdAt) &&
			new Date(current.createdAt).getTime() - new Date(previous.createdAt).getTime() < 5 * 60_000
		);
	}

	function focusDividerBefore(index: number): boolean {
		if (!focusActive) return false;
		const current = visibleMessages[index];
		if (!current || firstMessageNumericId(current.id) <= activeFocusAfterMessageId) return false;
		const previous = visibleMessages[index - 1];
		return !previous || messageNumericId(previous.id) <= activeFocusAfterMessageId;
	}

	function beginNewFocus() {
		if (!newFocusAvailable || !connected || turn || sending || review || workContext) return;
		pendingFocusAfterMessageId = messages.reduce(
			(maximum, message) => Math.max(maximum, messageNumericId(message.id)),
			0
		);
		newFocusPending = true;
		askError = '';
		askNotice = '';
		scrollReset += 1;
		composerFocusKey += 1;
	}

	function toggleContext() {
		includeContext = !includeContext;
		if (includeContext) contextFlare += 1;
	}

	const attachmentHref = (attachment: { uploadId: string }) =>
		`/api/companies/${encodeURIComponent(companyId)}/attachments/${encodeURIComponent(attachment.uploadId)}`;

	let sending = $state(false);
	async function submitAsk(event: SubmitEvent) {
		event.preventDefault();
		const text = composer.trim();
		if (!text || sending || deciding || !onask || needsProvider || connectionStatus !== 'available')
			return;
		scrollReset += 1;
		sending = true;
		askError = '';
		askNotice = '';
		const sent = composer;
		const files = composerFiles;
		const skills = composerSkills;
		composer = '';
		try {
			const command = await runCommand(text);
			if (!command.send) {
				if (command.error) {
					composer = sent;
					askError = command.error;
				} else {
					askNotice = command.notice ?? '';
				}
				return;
			}
			const outcome = await onask(
				text,
				files,
				includeContext,
				newFocusPending,
				!!turn,
				undefined,
				skills
			);
			if (outcome.error) {
				composer = sent;
				askError = outcome.error;
			} else {
				composerFiles = [];
				composerSkills = [];
				newFocusPending = false;
				askNotice = outcome.notice ?? '';
			}
		} catch (cause) {
			composer = sent;
			askError = failureSentence(cause, 'Your message was not delivered.');
		} finally {
			sending = false;
		}
	}

	async function requestChanges() {
		const feedback = reviewFeedback.trim();
		if (!review || !feedback || deciding) return;
		deciding = true;
		reviewError = '';
		try {
			const failure = await review.ondecide('request_changes', feedback);
			if (failure) reviewError = failure;
		} finally {
			deciding = false;
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
	aria-label={`${participantName} conversation`}
	aria-hidden={!open}
	inert={!open}
>
	<div class="exr-inner">
		<header class="exr-head">
			<div class="exr-head-primary">
				{#if review || workContext}
					<button
						class="rail-back"
						type="button"
						aria-label="Back to the Inbox"
						title="Back to the Inbox"
						onclick={(review ?? workContext)!.onback}
					>
						<ArrowLeft size={18} aria-hidden="true" />
					</button>
				{/if}
				{#if topics.length && ontopic}
					<!-- Who you are talking to, and about what. One menu switches both. -->
					<div class="exr-who topic-switch">
						<ActionMenu label="Switch conversation">
							{#snippet trigger()}<span class="topic-face"
									><SemanticMark meaning={participantId === 'exec' ? 'executive' : 'work'} /><strong
										class="exr-name">{participantName}</strong
									>{#if topicLabel}<span class="topic-label">{topicLabel}</span>{/if}<ChevronDown
										size={14}
										aria-hidden="true"
									/></span
								>{/snippet}
							{#each topics as entry (entry.key)}
								<button type="button" onclick={() => ontopic(entry.topic)}
									><span class="topic-option"
										><span>{entry.label}</span><small>{entry.hint}</small></span
									>{#if entry.key === currentTopicKey}<Check
											size={14}
											aria-label="Current"
										/>{/if}</button
								>
							{/each}
							{#if newFocusAvailable}
								<button
									type="button"
									class="topic-fresh"
									disabled={!connected || !!turn || sending}
									title="Begin with fresh working context; company memory is retained"
									onclick={beginNewFocus}>Start fresh</button
								>
							{/if}
						</ActionMenu>
					</div>
				{:else}
					<div class="exr-who">
						<SemanticMark meaning={review || workContext ? 'work' : 'executive'} />
						<strong class="exr-name">{participantName}</strong>
					</div>
				{/if}
				{#if visibleMessages.length}
					<ConversationHistoryTools
						messages={visibleMessages}
						{participantName}
						onjump={jumpToMessage}
					/>
				{/if}
			</div>
			{#if review}
				<div class="review-controls">
					<HoldApprove
						small
						label={deciding ? 'Recording…' : 'Hold to accept'}
						title="Hold to accept outcome"
						completeLabel="accepted ✓"
						disabled={deciding || sending}
						onapprove={() => void acceptReview()}
					/>
				</div>
			{/if}
		</header>
		{#if reviewError}<p class="review-error" role="alert">{reviewError}</p>{/if}
		{#if review}
			<form
				class="review-feedback"
				onsubmit={(event) => {
					event.preventDefault();
					void requestChanges();
				}}
			>
				<label
					>Request changes<textarea
						bind:value={reviewFeedback}
						rows="3"
						placeholder="Describe the changes needed…"
						disabled={!canOperate || deciding}></textarea></label
				>
				<button class="btn small" disabled={!canOperate || deciding || !reviewFeedback.trim()}
					>{deciding ? 'Recording…' : 'Request changes'}</button
				>
			</form>
		{/if}

		<div class="exr-panel">
			{#if !needsProvider && connectionStatus === 'unknown'}
				<!-- Unknown is the ordinary first moment; the transcript skeleton below
				     already says so without a sentence about it. -->
				<p class="sr-only" role="status">Checking {participantName}'s conversation status…</p>
			{:else if !needsProvider && connectionStatus === 'error'}
				<p class="exr-connection-notice" role="status">
					Connection status could not be refreshed. Try again shortly.
				</p>
			{:else if !needsProvider && connectionStatus === 'unavailable'}
				<p class="exr-connection-notice" role="status">
					A conversation route for {participantName} is not available in the latest company status.
				</p>
			{/if}
			{#if conversationFailed && conversationStatus === 'stale'}
				<p class="exr-connection-notice" role="status">
					Recent messages could not be refreshed. Showing the last loaded conversation.
					<button type="button" class="exr-retry" onclick={() => onrefreshConversation?.()}
						>Try again</button
					>
				</p>
			{/if}
			<div class="exr-chat">
				<div
					class="exr-msgs"
					bind:this={scrollEl}
					use:followChat={`${companyId}:${participantId}:${scrollReset}`}
				>
					{#each visibleMessages as message, i (message.id)}
						{#if focusDividerBefore(i)}
							<div class="conversation-focus-boundary">
								<span>New focus</span><i aria-hidden="true"></i><span>Company memory retained</span>
							</div>
						{/if}
						{#if unreadRuleBefore(i)}
							<div class="unread-rule" role="separator">
								<span
									>New since {shortTime(
										visibleMessages[i - 1]?.createdAt ?? message.createdAt
									)}</span
								>
							</div>
						{/if}
						{#if i === 0 || dayOf(message.createdAt) !== dayOf(visibleMessages[i - 1].createdAt)}
							<div class="day-sep" aria-hidden="true">
								<span>{dayLabel(message.createdAt)}</span>
							</div>
						{/if}
						<ConversationMessage
							continued={continuesRun(i)}
							domId={messageDomId(message.id)}
							messageId={String(messageNumericId(message.id) || '')}
							{companyId}
							reactions={reactions.summaryFor(messageNumericId(message.id))}
							onreact={canOperate && messageNumericId(message.id) > 0
								? (emoji, on) => void reactions.react(messageNumericId(message.id), emoji, on)
								: undefined}
							sender={message.from === 'you' ? 'owner' : message.from}
							author={message.from === 'you' ? 'You' : message.author || participantName}
							text={message.text}
							createdAt={message.createdAt}
							details={message.details}
							intent={message.intent}
							attachments={message.attachments}
							hrefFor={attachmentHref}
						/>
					{:else}
						{#if conversationFailed && conversationStatus === 'unknown'}
							<div class="exr-empty" role="alert">
								<p class="exr-empty-h">Conversation unavailable</p>
								<p class="exr-empty-p">Recent messages could not be loaded.</p>
								<button type="button" class="exr-retry" onclick={() => onrefreshConversation?.()}
									>Try again</button
								>
							</div>
						{:else if conversationStatus === 'unknown' || (!needsProvider && connectionStatus === 'unknown')}
							<!-- Whether intelligence is connected is not known yet: neither
							     "Ask anything" nor "Connect intelligence" would be true. -->
							<Skeleton label="Loading conversation" variant="messages" count={3} />
						{:else if focusActive}
							<!-- The focus boundary below is the empty transcript state. -->
						{:else if review || workContext}
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
								<p class="exr-empty-h">
									{needsProvider ? providerLabel : 'Ask anything.'}
								</p>
								<p class="exr-empty-p">
									{needsProvider
										? `Restore intelligence access so ${participantName} can reply.`
										: capabilityHint}
								</p>
								{#if needsProvider}
									<a
										class="btn primary small provider-connect"
										href={`/${companyId}/company/provider`}
										><Plus size={14} strokeWidth={2} aria-hidden="true" /><span
											>{providerLabel}</span
										></a
									>
								{/if}
							</div>
						{/if}
					{/each}
					{#if focusActive && !hasMessagesAfterFocus}
						<div class="conversation-focus-boundary" role={newFocusPending ? 'status' : undefined}>
							<span>New focus</span><i aria-hidden="true"></i><span>Company memory retained</span>
						</div>
						{#if !composer.trim() && !turn}
							<p class="conversation-capability-hint">{capabilityHint}</p>
						{/if}
					{/if}
					{#if turn && needsProvider && (turn.live?.phase ?? 'queued') === 'queued'}
						<p class="exr-setup-pending" role="status">
							Message saved. {participantName} can reply after intelligence access is restored.
						</p>
					{:else if turn}<ConversationTurnDock {participantName} {turn} />{/if}
				</div>

				{#if needsProvider}
					<!-- In the empty state the connect action sits with its explanation
					     above; otherwise it takes the composer's place. -->
					{#if !providerPromptInline}
						<a
							class="btn primary small provider-connect-slot"
							href={`/${companyId}/company/provider`}
							>{#if providerLabel.startsWith('Reconnect')}<RotateCw
									size={14}
									strokeWidth={2}
									aria-hidden="true"
								/>{:else}<Plus size={14} strokeWidth={2} aria-hidden="true" />{/if}<span
								>{providerLabel}</span
							></a
						>
					{/if}
				{:else}
					{#if openQuestion}
						<div class="open-question" role="note">
							<span>{participantName} is asking</span>
							<p>{openQuestion}</p>
							<button type="button" class="btn small" onclick={() => (composerFocusKey += 1)}
								>Answer</button
							>
						</div>
					{/if}
					<form class="exr-composer" onsubmit={submitAsk}>
						<Composer
							bind:value={composer}
							bind:files={composerFiles}
							bind:selectedSkills={composerSkills}
							options={[...options.value, ...references]}
							actionLabel={turn ? 'Queue direction' : 'Send'}
							disabled={!canOperate ||
								sending ||
								deciding ||
								!onask ||
								connectionStatus !== 'available'}
							minlength={1}
							placeholder={review || workContext || participantId !== 'exec'
								? `Message ${participantName}…`
								: 'Ask, redirect, or make a judgement…'}
							ariaLabel={review || workContext
								? `Message ${participantName}`
								: `Ask ${participantName}`}
							flareKey={contextFlare}
							focusKey={composerFocusKey}
						>
							{#snippet controls()}
								<IntelligenceChip {companyId} actorId={participantId} name={participantName} />
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
				{/if}
			</div>
		</div>
	</div>
</aside>

<style>
	.unread-rule {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 10px 14px 2px;
		color: var(--intent-conversation);
		font: 500 var(--t-label) var(--font-ui);
	}
	.unread-rule::after {
		flex: 1;
		height: 1px;
		content: '';
		background: color-mix(in srgb, var(--intent-conversation) 30%, transparent);
	}
	.open-question {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: 2px 10px;
		margin: 0 12px 8px;
		padding: 10px 12px;
		border: 1px solid color-mix(in srgb, var(--state-warning) 40%, var(--border));
		border-radius: var(--radius-lg);
		background: color-mix(in srgb, var(--state-warning) 8%, var(--surface-raised));
	}
	.open-question span {
		grid-column: 1;
		color: var(--text-secondary);
		font: 500 var(--t-label) var(--font-ui);
	}
	.open-question p {
		grid-column: 1;
		margin: 0;
		color: var(--ink);
		line-height: 1.4;
	}
	.open-question .btn {
		grid-column: 2;
		grid-row: 1 / 3;
	}
	.provider-connect-slot {
		margin: var(--space-3);
	}
	.provider-connect {
		margin-top: var(--space-4);
	}

	.exr-head-primary {
		min-width: 0;
		flex: 1 1 auto;
		display: flex;
		align-items: center;
		gap: 10px;
	}
	.topic-switch {
		overflow: hidden;
	}
	.topic-switch :global(.action-menu) {
		flex: 1 1 auto;
		min-width: 0;
	}
	.topic-switch :global(summary) {
		display: flex;
		justify-content: flex-start;
		width: auto;
		max-width: 100%;
		min-width: 0;
		height: 34px;
		padding: 0 8px 0 4px;
		overflow: hidden;
	}
	.topic-face {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
		color: var(--ink);
	}
	.topic-face :global(svg) {
		flex: none;
		color: var(--text-tertiary);
	}
	.topic-face .exr-name {
		flex: none;
		max-width: 60%;
	}
	.topic-label {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		color: var(--text-secondary);
		font-size: var(--t-label);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.topic-label::before {
		content: '· ';
		color: var(--text-tertiary);
	}
	.topic-option {
		display: grid;
		flex: 1;
		min-width: 0;
	}
	.topic-option span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.topic-option small {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.topic-fresh {
		border-top: 1px solid var(--border) !important;
		border-radius: 0 !important;
		color: var(--text-secondary) !important;
	}
	.exr-head-primary .exr-who {
		min-width: 0;
		flex: 1 1 auto;
		display: flex;
		align-items: center;
		gap: 8px;
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
	.conversation-focus-boundary {
		width: calc(100% - 28px);
		display: grid;
		grid-template-columns: auto minmax(16px, 1fr) auto;
		align-items: center;
		gap: 8px;
		margin: 18px 14px 8px;
		color: color-mix(in srgb, var(--intent-conversation) 58%, var(--text-tertiary));
		font: 500 var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
		animation: focus-arrive var(--motion-disclosure) var(--ease-spring) both;
	}
	.conversation-focus-boundary i {
		height: 1px;
		background: linear-gradient(
			90deg,
			color-mix(in srgb, var(--intent-conversation) 35%, transparent),
			color-mix(in srgb, var(--intent-conversation) 10%, transparent)
		);
	}
	.conversation-capability-hint {
		max-width: 270px;
		margin: 3px auto 20px;
		padding-inline: 18px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		line-height: 1.55;
		text-align: center;
		animation: focus-arrive var(--motion-disclosure) var(--ease-standard) both;
	}
	.exr-setup-pending {
		margin: var(--space-3);
		padding: var(--space-3);
		border: 1px solid var(--border-soft);
		border-radius: var(--radius-control);
		background: var(--surface-pane);
		color: var(--text-secondary);
		font-size: var(--t-label);
		line-height: 1.5;
	}
	@keyframes focus-arrive {
		from {
			opacity: 0;
			transform: translateY(5px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.conversation-focus-boundary,
		.conversation-capability-hint {
			animation: none;
		}
	}
	.exr-connection-notice {
		flex: none;
		margin: 0;
		padding: 9px 14px;
		border-bottom: 1px solid var(--border-soft);
		color: var(--text-secondary);
		font-size: var(--t-label);
		line-height: 1.45;
	}
	.exr-retry {
		display: inline-block;
		margin-top: 8px;
		border: 0;
		padding: 0;
		background: none;
		color: var(--intent-conversation);
		font: inherit;
		font-weight: 600;
		cursor: pointer;
	}
	.exr-retry:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 3px;
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
		background: color-mix(in srgb, var(--highlight) 58%, transparent);
		color: var(--text-secondary);
		cursor: pointer;
	}
	.rail-back:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}
	.review-controls {
		flex: none;
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.review-feedback {
		display: grid;
		gap: 8px;
		margin: 0 16px 12px;
		padding: 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface);
	}
	.review-feedback label {
		display: grid;
		gap: 6px;
		font-size: var(--t-label);
		font-weight: 500;
	}
	.review-feedback textarea {
		box-sizing: border-box;
		width: 100%;
		resize: vertical;
		padding: 8px 10px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--ink);
		font: inherit;
	}
	.review-feedback button {
		justify-self: start;
	}
	.review-controls :global(.hold-approve) {
		min-width: 122px;
		white-space: nowrap;
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
		background: color-mix(in srgb, var(--danger) 6%, var(--surface-raised));
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
			color-mix(in srgb, var(--intent-feedback-soft) 82%, var(--surface-raised)),
			color-mix(in srgb, var(--intent-feedback-soft) 58%, var(--surface-alt))
		);
		box-shadow: var(--shadow-soft);
	}
	.review-empty-mark {
		width: 38px;
		height: 38px;
		display: grid;
		place-items: center;
		border: 1px solid color-mix(in srgb, var(--intent-feedback) 28%, var(--border));
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--intent-feedback-soft) 76%, var(--surface-raised));
		box-shadow: var(--control-depth);
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
