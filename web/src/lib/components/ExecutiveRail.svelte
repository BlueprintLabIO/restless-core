<script lang="ts">
	/* One contextual conversation rail. It normally belongs to the Exec; while
	 * the owner focuses a review it belongs to that Work's accountable lead.
	 * The rail stays mounted and takes real space rather than nesting another
	 * chat inside the outcome surface. */

	import { SvelteDate } from 'svelte/reactivity';
	import AttachmentList from '$lib/primitives/AttachmentList.svelte';
	import Composer from '$lib/primitives/Composer.svelte';
	import IntentReceipt from '$lib/primitives/IntentReceipt.svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import SemanticMark from '$lib/primitives/SemanticMark.svelte';
	import type { ThreadMessage } from '$lib/model/view';

	let {
		messages = [],
		participantName = 'Exec',
		participantRole = 'Executive',
		companyId,
		membershipRole,
		connected = false,
		contextLabel = 'Current screen',
		open = true,
		onask = null,
		review = null
	}: {
		messages?: ThreadMessage[];
		participantName?: string;
		participantRole?: string;
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
		/** Returns an error message, or null on success. Unwired: null, composer inert. */
		onask?:
			((text: string, files: File[], includeContext: boolean) => Promise<string | null>) | null;
		review?: {
			title: string;
			recommendation: string;
			onback: () => void;
			ondecide: (
				decision: 'accept' | 'request_changes',
				feedback: string
			) => Promise<string | null>;
		} | null;
	} = $props();

	const canOperate = $derived(['owner', 'operator'].includes(membershipRole ?? ''));

	/* "Thinking" is read straight from the record: the last word is yours, so an
	 * answer is owed. No local flags to disagree with it. */
	const waiting = $derived(messages.length > 0 && messages.at(-1)!.from === 'you');

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

	function timeLabel(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		if (Number.isNaN(date.getTime())) return '';
		return date.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
	}

	let composer = $state('');
	let composerFiles = $state<File[]>([]);
	let includeContext = $state(true);
	let askError = $state('');
	let reviewError = $state('');
	let deciding = $state(false);
	let scrollEl = $state<HTMLDivElement | undefined>();

	/* keep the newest word in view — only while the rail is actually showing */
	$effect(() => {
		if (!open || messages.length === 0) return;
		scrollEl?.scrollTo({ top: scrollEl.scrollHeight });
	});

	const attachmentHref = (attachment: { uploadId: string }) =>
		`/api/companies/${encodeURIComponent(companyId)}/attachments/${encodeURIComponent(attachment.uploadId)}`;

	let sending = $state(false);
	async function submitAsk(event: SubmitEvent) {
		event.preventDefault();
		const text = composer.trim();
		if (!text || sending || !onask) return;
		sending = true;
		askError = '';
		const sent = composer;
		const files = composerFiles;
		composer = '';
		try {
			const failure = await onask(text, files, includeContext);
			if (failure) {
				composer = sent;
				askError = failure;
			} else {
				composerFiles = [];
			}
		} finally {
			sending = false;
		}
	}

	const reviewFeedback = $derived(
		composer.trim() || messages.findLast((message) => message.from === 'you')?.text.trim() || ''
	);

	async function decideReview(decision: 'accept' | 'request_changes') {
		if (!review || deciding) return;
		const feedback = decision === 'request_changes' ? reviewFeedback : '';
		if (decision === 'request_changes' && !feedback) {
			reviewError = 'Write the exact change you want first.';
			return;
		}
		deciding = true;
		reviewError = '';
		try {
			const failure = await review.ondecide(decision, feedback);
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
	aria-label={`${participantRole} conversation`}
	aria-hidden={!open}
	inert={!open}
>
	{#if !connected}
		<div class="exr-lock">
			<div class="exr-lock-card">
				<span class="exr-lock-badge" aria-hidden="true"
					><MatrixGlyph rows={GLYPHS.p} size={18} glow /></span
				>
				<h2 class="exr-lock-h">{participantRole} unavailable</h2>
				<p class="exr-lock-p">
					The company computer has not confirmed that {participantName} is reachable. Conversation will
					open automatically when the live connection returns.
				</p>
				<p class="exr-lock-note">Connection is managed by the company computer.</p>
			</div>
		</div>
	{/if}
	<div class="exr-inner" inert={!connected}>
		<header class="exr-head">
			<div class="exr-who">
				<SemanticMark meaning={review ? 'work' : 'executive'} />
				<span>
					<strong class="exr-name">{participantName}</strong>
					<small class="exr-sub">{participantRole}</small>
				</span>
			</div>
			{#if review}
				<button class="rail-back" type="button" onclick={review.onback}>Back to inbox</button>
			{/if}
		</header>

		<div class="exr-msgs" bind:this={scrollEl}>
			{#if review}
				<div class="review-context">
					<span>Outcome under review</span>
					<strong>{review.title}</strong>
					<p>{review.recommendation}</p>
					<small>Discussion stays open until you make an explicit decision below.</small>
				</div>
			{/if}
			{#each messages as message, i (message.id)}
				{#if i === 0 || dayOf(message.createdAt) !== dayOf(messages[i - 1].createdAt)}
					<div class="day-sep" aria-hidden="true"><span>{dayLabel(message.createdAt)}</span></div>
				{/if}
				<div class="exr-row {message.from === 'you' ? 'you' : 'agent'}">
					<div class="exr-b">
						<div class="transcript-meta">
							{#if message.from === 'agent'}
								<MatrixGlyph rows={GLYPHS.p} size={7} />
							{/if}
							<span>{message.from === 'you' ? 'You' : message.author || participantName}</span>
							<time>{timeLabel(message.createdAt)}</time>
						</div>
						{#if message.from === 'system'}
							<p class="exr-system">{message.text}</p>
						{:else if message.from === 'agent'}
							<!-- Employees write Markdown; the owner should read structure, not syntax. -->
							<Markdown text={message.text} />
							<AttachmentList attachments={message.attachments} hrefFor={attachmentHref} />
							{#if message.intent}
								<IntentReceipt intent={message.intent} />
							{/if}
						{:else}
							<p>{message.text}</p>
							<AttachmentList attachments={message.attachments} hrefFor={attachmentHref} />
							{#if message.contextPath}
								<span class="message-context-ref">
									<MatrixGlyph rows={GLYPHS.work} size={7} />
									Sent from {message.contextPath}
								</span>
							{/if}
						{/if}
					</div>
				</div>
			{:else}
				<div class="exr-empty">
					<p class="exr-empty-h">{review ? `Talk to ${participantName}.` : 'Ask anything.'}</p>
					<p class="exr-empty-p">
						{review
							? 'This conversation is scoped to the reviewed Work and its evidence.'
							: 'The executive reads the whole company record before it answers — goals, staff, work in flight, and what needs your word.'}
					</p>
				</div>
			{/each}
			{#if waiting}
				<div class="exr-thinking" aria-label={`${participantName} is preparing an answer`}>
					<i></i><i></i><i></i>
				</div>
			{/if}
		</div>

		<form class="exr-composer" onsubmit={submitAsk}>
			<Composer
				bind:value={composer}
				bind:files={composerFiles}
				disabled={!canOperate || !onask}
				placeholder={review
					? `Ask ${participantName}, or write exact revision feedback…`
					: 'Ask, redirect, or make a judgement…'}
				ariaLabel={`Ask ${participantName}`}
			>
				{#snippet controls()}
					<div class="exec-context-line">
						<button
							type="button"
							class="exec-context-chip"
							class:off={!includeContext}
							aria-pressed={includeContext}
							title="Link this message to the current screen"
							onclick={() => (includeContext = !includeContext)}
						>
							<MatrixGlyph rows={GLYPHS.work} size={8} />
							{includeContext ? contextLabel : 'Link current screen'}
						</button>
					</div>
				{/snippet}
			</Composer>
			<div class="composer-foot">
				<span
					>{review
						? 'Sending is discussion only'
						: 'Exec interprets intent and confirms consequential changes'}</span
				>
				<span>⌘ ↵ send</span>
			</div>
			{#if askError}
				<p class="exr-error" role="alert">{askError}</p>
			{/if}
		</form>
		{#if review}
			<section class="rail-review-gate" aria-label="Review decision">
				<span>Your decision</span>
				<div>
					<button
						class="btn small primary"
						type="button"
						disabled={deciding}
						onclick={() => decideReview('accept')}
					>
						{deciding ? 'Recording…' : 'Accept outcome'}
					</button>
					<button
						class="btn small danger"
						type="button"
						disabled={deciding || !reviewFeedback}
						onclick={() => decideReview('request_changes')}
					>
						Request changes
					</button>
				</div>
				<small>Your typed note, or latest delivered message, becomes the next revision brief.</small
				>
				{#if reviewError}<p role="alert">{reviewError}</p>{/if}
			</section>
		{/if}
	</div>
</aside>

<style>
	.rail-back {
		flex: none;
		padding: 5px 8px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: rgba(255, 255, 255, 0.58);
		color: var(--text-secondary);
		font: 500 var(--t-label) var(--font-mono);
		cursor: pointer;
	}
	.review-context {
		display: grid;
		gap: 5px;
		padding: 14px 15px;
		border-bottom: 1px solid var(--border);
		background: var(--intent-feedback-soft);
	}
	.review-context > span,
	.rail-review-gate > span {
		font: 600 var(--t-label) var(--font-mono);
		letter-spacing: 0.07em;
		text-transform: uppercase;
		color: var(--intent-feedback);
	}
	.review-context strong {
		font-size: var(--t-body);
	}
	.review-context p,
	.review-context small {
		margin: 0;
		line-height: 1.45;
		color: var(--text-secondary);
	}
	.review-context small {
		color: var(--text-tertiary);
	}
	.rail-review-gate {
		display: grid;
		gap: 8px;
		padding: 12px 14px 14px;
		border-top: 1px solid var(--border-strong);
		background: color-mix(in srgb, var(--intent-feedback-soft) 60%, white);
	}
	.rail-review-gate > div {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
	}
	.rail-review-gate small,
	.rail-review-gate p {
		margin: 0;
		font-size: var(--t-label);
		line-height: 1.4;
		color: var(--text-tertiary);
	}
	.rail-review-gate p {
		color: var(--danger);
	}
</style>
