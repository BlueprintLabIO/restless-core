<script lang="ts">
	/* One contextual conversation rail. It normally belongs to the Exec; while
	 * the owner focuses a review it belongs to that Work's accountable lead.
	 * The rail stays mounted and takes real space rather than nesting another
	 * chat inside the outcome surface. */

	import { followChat } from '$lib/actions/follow-chat';
	import IntelligencePopover from './IntelligencePopover.svelte';
	import { SvelteDate } from 'svelte/reactivity';
	import Plus from '@lucide/svelte/icons/plus';
	import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import Composer from '$lib/primitives/Composer.svelte';
	import ConversationHistoryTools from '$lib/primitives/ConversationHistoryTools.svelte';
	import ConversationMessage from '$lib/primitives/ConversationMessage.svelte';
	import ConversationTurnDock from '$lib/primitives/ConversationTurnDock.svelte';
	import HoldApprove from '$lib/primitives/HoldApprove.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import SemanticMark from '$lib/primitives/SemanticMark.svelte';
	import type { ActiveAgentTurn } from '$lib/model/queries.svelte';
	import type { OutcomeStandard } from '$lib/model/company';
	import type { ThreadMessage } from '$lib/model/view';
	import {
		EXEC_COMMANDS,
		addGoal,
		addLoop,
		cancelLoop,
		closeGoal,
		describeInterval,
		fetchSkillLibrary,
		listGoals,
		listLoops,
		parseComposerCommand,
		skillOptions,
		type ComposerOption
	} from '$lib/model/skills';

	let {
		messages = [],
		participantName = 'Exec',
		participantId = 'exec',
		participantRole = 'Executive',
		turn = null,
		companyId,
		membershipRole,
		connected = false,
		needsProvider = false,
		contextLabel = 'Current screen',
		focusAfterMessageId = 0,
		focusStartedAt = null,
		newFocusAvailable = false,
		open = true,
		onask = null,
		review = null,
		workContext = null
	}: {
		messages?: ThreadMessage[];
		participantName?: string;
		participantId?: string;
		participantRole?: string;
		turn?: ActiveAgentTurn | null;
		companyId: string;
		membershipRole: string;
		/**
		 * Whether the executive has a bound ACP runtime. This must come from a LIVE
		 * probe of the runtime, never from configuration — "probe, never guess". Until
		 * it is true the rail is glass-locked instead of implying that conversation is
		 * available. Runtime/provider administration stays outside the owner cockpit.
		 */
		connected?: boolean;
		needsProvider?: boolean;
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
	} = $props();

	const canOperate = $derived(['owner', 'operator'].includes(membershipRole ?? ''));

	/* `$skill` selection and the `/goal` and `/loop` commands. Skills are the
	 * company library; the commands are Restless primitives offered only in the
	 * Exec conversation, where they have one accountable meaning. */
	let composerSkills = $state<string[]>([]);
	let skillChoices = $state<ComposerOption[]>([]);
	const execConversation = $derived(participantId === 'exec' && !review && !workContext);
	const composerOptions = $derived(
		execConversation && membershipRole === 'owner'
			? [...EXEC_COMMANDS, ...skillChoices]
			: skillChoices
	);
	$effect(() => {
		if (!open || !canOperate || !companyId) return;
		let cancelled = false;
		fetchSkillLibrary(companyId)
			.then((library) => {
				if (!cancelled) skillChoices = skillOptions(library);
			})
			.catch(() => {
				/* The composer still works without skills; the library page reports why. */
			});
		return () => {
			cancelled = true;
		};
	});

	/* Returns a notice for commands handled without a message, or null to send. */
	async function runCommand(text: string): Promise<{ notice?: string; error?: string; send: boolean }> {
		const command = execConversation && membershipRole === 'owner' ? parseComposerCommand(text) : null;
		if (!command) return { send: true };
		switch (command.kind) {
			case 'invalid':
				return { error: command.message, send: false };
			case 'goal-set':
				await addGoal(companyId, command.objective);
				return { send: true };
			case 'goal-show': {
				const open = (await listGoals(companyId)).filter((goal) => !goal.closed_at);
				return {
					notice: open.length
						? `Open goals: ${open.map((goal) => goal.title).join('; ')}`
						: 'No open goals. Set one with /goal <objective>.',
					send: false
				};
			}
			case 'goal-clear': {
				const open = (await listGoals(companyId)).filter((goal) => !goal.closed_at);
				const latest = open.at(-1);
				if (!latest) return { notice: 'There is no open goal to clear.', send: false };
				await closeGoal(companyId, latest.id);
				return { notice: `Closed the goal “${latest.title}”. Its work is unchanged.`, send: false };
			}
			case 'loop-set': {
				const loop = await addLoop(companyId, command.every, command.prompt);
				return {
					notice: loop.created
						? `Exec will check in every ${describeInterval(loop.interval_seconds)}: ${command.prompt}`
						: `That loop is already running every ${describeInterval(loop.interval_seconds)}.`,
					send: false
				};
			}
			case 'loop-show': {
				const loops = await listLoops(companyId);
				return {
					notice: loops.length
						? loops
								.map((loop) => `Every ${describeInterval(loop.interval_seconds ?? 0)}: ${loop.reason}`)
								.join(' · ')
						: 'No loops are running. Start one with /loop 30m <what to check>.',
					send: false
				};
			}
			case 'loop-clear': {
				const loops = await listLoops(companyId);
				await Promise.all(loops.map((loop) => cancelLoop(companyId, loop.id)));
				return {
					notice: loops.length ? `Stopped ${loops.length} loop${loops.length === 1 ? '' : 's'}.` : 'No loops were running.',
					send: false
				};
			}
		}
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

	const activeFocusAfterMessageId = $derived(
		newFocusPending ? pendingFocusAfterMessageId : focusAfterMessageId
	);
	const focusActive = $derived(newFocusPending || focusStartedAt !== null);
	// Each durable reply keeps its own timestamp, intent, and navigation target.
	const visibleMessages = $derived(messages);
	const hasMessagesAfterFocus = $derived(
		focusActive &&
			messages.some((message) => messageNumericId(message.id) > activeFocusAfterMessageId)
	);
	const capabilityHint = $derived.by(() => {
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
		if (!text || sending || deciding || !onask || needsProvider) return;
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
			askError = cause instanceof Error ? cause.message : 'Your message was not delivered.';
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
						aria-label="Back to Attention"
						title="Back to Attention"
						onclick={(review ?? workContext)!.onback}
					>
						<ArrowLeft size={18} aria-hidden="true" />
					</button>
				{/if}
				<IntelligencePopover
					{companyId}
					actorId={participantId}
					label={participantName}
					align="start"
				>
					{#snippet children(tooltipId)}
						<button
							class="exr-who intelligence-trigger"
							type="button"
							aria-label={`${participantName} intelligence settings`}
							aria-describedby={tooltipId}
						>
							<SemanticMark meaning={review || workContext ? 'work' : 'executive'} />
							<strong class="exr-name">{participantName}</strong>
						</button>
					{/snippet}
				</IntelligencePopover>
				{#if newFocusAvailable}
					<button
						class="exr-new-focus"
						type="button"
						disabled={!connected || !!turn || sending}
						title={turn || sending
							? 'Start a new focus when Exec finishes the current reply'
							: 'Begin with fresh working context; company memory is retained'}
						onclick={beginNewFocus}
					>
						New focus
					</button>
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
			{#if !connected && !needsProvider}
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
			<div class="exr-chat" inert={!connected && !needsProvider}>
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
							intent={message.intent}
							attachments={message.attachments}
							hrefFor={attachmentHref}
						/>
					{:else}
						{#if focusActive}
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
									{needsProvider ? 'Connect intelligence' : 'Ask anything.'}
								</p>
								<p class="exr-empty-p">
									{needsProvider
										? `Add a connection to start talking with ${participantName}.`
										: capabilityHint}
								</p>
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
					{#if turn}<ConversationTurnDock {participantName} {turn} />{/if}
				</div>

				{#if needsProvider}
					<a class="provider-connect" href={`/${companyId}/company/provider`}
						><Plus size={15} strokeWidth={1.8} aria-hidden="true" /><span
							>Add intelligence provider</span
						><ArrowUpRight size={14} strokeWidth={1.8} aria-hidden="true" /></a
					>
				{:else}
					<form class="exr-composer" onsubmit={submitAsk}>
						<Composer
							bind:value={composer}
							bind:files={composerFiles}
							bind:selectedSkills={composerSkills}
							options={composerOptions}
							actionLabel={turn ? 'Queue direction' : 'Send'}
							disabled={!canOperate || sending || deciding || !onask}
							minlength={1}
							placeholder={review || workContext
								? 'Message the lead…'
								: 'Ask, redirect, or make a judgement…'}
							ariaLabel={review || workContext
								? `Message ${participantName}`
								: `Ask ${participantName}`}
							flareKey={contextFlare}
							focusKey={composerFocusKey}
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
				{/if}
			</div>
		</div>
	</div>
</aside>

<style>
	.intelligence-trigger {
		border: 0;
		padding: 0;
		background: transparent;
		color: inherit;
		cursor: help;
	}
	.intelligence-trigger:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 4px;
		border-radius: var(--radius-control);
	}

	.provider-connect {
		display: flex;
		align-items: center;
		justify-content: flex-start;
		gap: var(--space-3);
		min-height: 44px;
		margin: var(--space-3);
		padding: var(--space-3);
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-pane);
		color: var(--intent-conversation);
		box-shadow: var(--bevel);
		font-size: var(--t-label);
		font-weight: 500;
		text-decoration: none;
	}
	.provider-connect span {
		flex: 1;
	}
	.provider-connect:hover {
		background: var(--surface-alt);
		border-color: var(--intent-conversation);
	}
	.provider-connect:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 3px;
	}

	.exr-head-primary {
		min-width: 0;
		flex: 1 1 auto;
		display: flex;
		align-items: center;
		gap: 10px;
	}
	.exr-head-primary > :global(.intelligence-hover) {
		min-width: 0;
		margin-right: auto;
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
	.conversation-focus-boundary {
		width: calc(100% - 28px);
		display: grid;
		grid-template-columns: auto minmax(16px, 1fr) auto;
		align-items: center;
		gap: 8px;
		margin: 18px 14px 8px;
		color: color-mix(in srgb, var(--intent-conversation) 58%, var(--text-tertiary));
		font: 500 var(--t-label) var(--font-mono);
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
		box-shadow: var(--shadow-soft);
	}
	.review-empty-mark {
		width: 38px;
		height: 38px;
		display: grid;
		place-items: center;
		border: 1px solid color-mix(in srgb, var(--intent-feedback) 28%, var(--border));
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--intent-feedback-soft) 76%, white);
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
