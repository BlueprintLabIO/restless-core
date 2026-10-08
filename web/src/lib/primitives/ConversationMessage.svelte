<script lang="ts">
	import { formatMoment } from '$lib/ui/time';
	import { onDestroy, untrack } from 'svelte';
	import type { Snippet } from 'svelte';
	import Check from '@lucide/svelte/icons/check';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import Copy from '@lucide/svelte/icons/copy';
	import AttachmentList from './AttachmentList.svelte';
	import Markdown from './Markdown.svelte';
	import ReferencePreview from './ReferencePreview.svelte';
	import MessageReactions from './MessageReactions.svelte';
	import ReactionAdd from './ReactionAdd.svelte';
	import type { ReactionSummary } from '$lib/model/reactions.svelte';
	import SemanticMark from '$lib/ui/glyph/SemanticMark.svelte';
	import type { MessageAttachment, MessageIntentReceipt } from '$lib/model/view';
	import { initials } from '$lib/model/initials';

	let {
		sender,
		author,
		text,
		createdAt,
		details = null,
		attachments = [],
		intent = null,
		hrefFor,
		copyable = true,
		pending = false,
		domId,
		messageId = '',
		companyId = '',
		reactions = [],
		onreact,
		headerExtra,
		actions,
		embedded = false,
		continued = false
	}: {
		sender: 'owner' | 'agent' | 'human' | 'system';
		author: string;
		text: string;
		createdAt: Date | string;
		details?: string | null;
		attachments?: MessageAttachment[];
		intent?: MessageIntentReceipt | null;
		hrefFor?: (attachment: MessageAttachment) => string;
		copyable?: boolean;
		pending?: boolean;
		domId?: string;
		/** The stored message, so files it names can be opened in place. */
		messageId?: string;
		companyId?: string;
		reactions?: ReactionSummary[];
		/** Present when the viewer may react; reacting never starts a turn. */
		onreact?: (emoji: string, on: boolean) => void;
		headerExtra?: Snippet;
		actions?: Snippet;
		embedded?: boolean;
		/** Follows a message from the same author moments earlier: no header. */
		continued?: boolean;
	} = $props();

	let copyState = $state<'idle' | 'copied' | 'failed'>('idle');
	let copyTimer: number | undefined;

	function timeLabel(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		if (Number.isNaN(date.getTime())) return '';
		// Day separators carry the date; a message shows its time of day.
		return date.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
	}
	/* A room message that is seconds old when it first renders has just
	 * arrived: it settles in. History loading on open and the owner's own
	 * words (shown optimistically, then confirmed) stay still, as do Exec
	 * replies, which already streamed in their live turn. */
	const fresh = untrack(
		() => embedded && sender !== 'owner' && Date.now() - new Date(createdAt).getTime() < 15_000
	);
	const timestamp = $derived(timeLabel(createdAt));
	/* A message in a run has no header; its time waits in the gutter until pointed at. */
	const gutterTime = $derived.by(() => {
		const date = createdAt instanceof Date ? createdAt : new Date(createdAt);
		return Number.isNaN(date.getTime())
			? ''
			: date.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit', hour12: false });
	});
	const messageDate = $derived(new Date(createdAt));
	const validDate = $derived(!Number.isNaN(messageDate.getTime()));
	const displayAuthor = $derived(author === 'The Exec' ? 'Exec' : author);
	/* The Exec keeps its mark; every other person or agent is the same initial
	 * avatar the directory shows, so a message is attributable at a glance. */
	const personInitial = $derived(
		(sender === 'agent' || sender === 'human') && displayAuthor !== 'Exec'
			? initials(displayAuthor)
			: ''
	);

	/* Takeaway first: an agent's own one-line reading of the message leads. An agent's reply is
	 * never folded; only the owner's own long paste (a report, a log) folds, and then late. */
	const takeaway = $derived(sender === 'agent' ? (intent?.summary?.trim() ?? '') : '');
	const long = $derived(sender !== 'agent' && text.split(/\s+/).length > 450);
	let expanded = $state(false);
	const folded = $derived(long && !expanded);
	let preview: ReferencePreview | undefined = $state();
	const referable = $derived(
		!!companyId && !!messageId && /^\d+$/.test(messageId) && sender !== 'owner'
	);

	async function copyMessage() {
		try {
			await navigator.clipboard.writeText(text);
			copyState = 'copied';
		} catch {
			copyState = 'failed';
		}
		window.clearTimeout(copyTimer);
		copyTimer = window.setTimeout(() => (copyState = 'idle'), 1_800);
	}

	onDestroy(() => {
		if (typeof window !== 'undefined') window.clearTimeout(copyTimer);
	});
</script>

<article
	id={domId}
	class="conversation-message {sender}"
	class:fresh
	class:continued
	class:pending
	class:embedded
	tabindex="-1"
	data-message-sender={sender}
>
	<header class="message-meta" class:sr-only={continued}>
		<span class="message-avatar">
			{#if personInitial}<span
					class="message-initial"
					role="img"
					aria-label={`${displayAuthor} message`}>{personInitial}</span
				>{:else}
				<SemanticMark
					meaning={sender === 'agent' ? 'executive' : sender === 'owner' ? 'direction' : 'work'}
					size="small"
					label={sender === 'owner' ? 'Your message' : `${displayAuthor} message`}
				/>
			{/if}
		</span>
		<strong>{displayAuthor}</strong>
		{#if timestamp}<time
				datetime={validDate ? messageDate.toISOString() : undefined}
				title={validDate ? formatMoment(messageDate) : undefined}>{timestamp}</time
			>{/if}
		{@render headerExtra?.()}
	</header>

	{#if continued && sender !== 'owner' && gutterTime}<time
			class="gutter-time"
			datetime={validDate ? messageDate.toISOString() : undefined}
			title={validDate ? formatMoment(messageDate) : undefined}>{gutterTime}</time
		>{/if}
	<div class="message-body">
		{#if takeaway}<p class="takeaway">{takeaway}</p>{/if}
		<div class="message-text" class:folded class:after-takeaway={!!takeaway}>
			<Markdown
				{text}
				onreference={referable
					? (path, name) => void preview?.open(messageId, path, name)
					: undefined}
			/>
		</div>
		{#if long}<button
				type="button"
				class="fold-toggle"
				class:expanded
				aria-expanded={expanded}
				onclick={() => (expanded = !expanded)}
				>{expanded ? 'Show less' : 'Show more'}<ChevronDown
					size={12}
					strokeWidth={2}
					aria-hidden="true"
				/></button
			>{/if}
		<AttachmentList {attachments} {hrefFor} />
		{#if onreact && sender !== 'system'}<MessageReactions {reactions} {onreact} />{/if}
		{#if details && sender === 'agent'}
			<!-- One disclosure: the evidence stays folded under the message until asked for. -->
			<details class="work-details">
				<summary>Work details</summary>
				<div class="work-details-body"><Markdown text={details} /></div>
			</details>
		{/if}
	</div>

	{#if (copyable || onreact) && sender !== 'system'}
		<footer class="message-footer">
			<div class="message-actions" role="toolbar" aria-label="Message actions">
				{#if onreact}<ReactionAdd {reactions} {onreact} />{/if}
				{@render actions?.()}
				{#if copyable}
					<button
						type="button"
						class="copy-message"
						class:confirmed={copyState === 'copied'}
						aria-label={copyState === 'copied'
							? 'Message copied'
							: copyState === 'failed'
								? 'Could not copy message'
								: 'Copy message'}
						title={copyState === 'copied'
							? 'Copied'
							: copyState === 'failed'
								? 'Could not copy'
								: 'Copy message'}
						onclick={copyMessage}
					>
						{#if copyState === 'copied'}
							<Check size={11} strokeWidth={2} aria-hidden="true" />
						{:else}
							<Copy size={11} strokeWidth={2} aria-hidden="true" />
						{/if}
					</button>
				{/if}
			</div>
		</footer>
	{/if}
	{#if referable}<ReferencePreview bind:this={preview} {companyId} />{/if}
	<span class="copy-status" aria-live="polite">
		{copyState === 'copied' ? 'Message copied' : copyState === 'failed' ? 'Copy failed' : ''}
	</span>
</article>

<style>
	.conversation-message.fresh {
		animation: message-arrive var(--motion-disclosure) var(--ease-out) both;
	}
	@keyframes message-arrive {
		from {
			opacity: 0;
			transform: translateY(6px);
		}
	}

	/* After Svelte AI Elements: space separates turns, not rules. Others speak
	 * as avatar, name and plain text; the owner's own words are a compact
	 * bubble on the right, with no name to read. A run of messages from one
	 * author shares a single header. */
	.conversation-message {
		position: relative;
		width: 100%;
		min-width: 0;
		display: grid;
		gap: 4px;
		padding: 10px 14px 4px;
		border: 0;
		background: transparent;
	}

	.conversation-message.continued {
		padding-top: 0;
	}

	/* Everyone else's text lines up under their name, and a run reads as one
	 * block. Actions float at the top right on hover instead of reserving a
	 * row under every message. */
	.conversation-message:not(.owner) :is(.message-body, .message-footer) {
		padding-left: 31px;
	}

	/* Touch has no hover: a message's actions appear when it is tapped
	 * (focused), instead of a Reply row under every message. */
	@media (hover: none) {
		.conversation-message:not(:focus-within) .message-footer {
			display: none;
		}
	}

	.conversation-message:focus {
		outline: none;
	}

	/* With a mouse, someone else's message shows its toolbar riding its top edge, as Slack and Linear
	 * do, clear of the text. */
	@media (hover: hover) and (pointer: fine) {
		.conversation-message:not(.owner) .message-footer {
			position: absolute;
			z-index: 2;
			top: -10px;
			right: 12px;
			min-height: 0;
			margin: 0;
			padding: 0;
		}
	}

	/* The owner's bubble sits right; its actions wait beside it rather than
	 * reserving a row beneath, so a run of bubbles stays tight. Their column is
	 * never narrower than the actions: a wide bubble gives way instead of
	 * pushing them past the rail's edge. */
	.conversation-message.owner {
		grid-template-columns: minmax(max-content, 1fr) minmax(0, auto);
		column-gap: 6px;
	}

	.conversation-message.owner .message-meta {
		grid-row: 1;
		grid-column: 1 / -1;
		justify-self: end;
	}

	.conversation-message.owner .message-body {
		grid-row: 2;
		grid-column: 2;
	}

	/* The owner's toolbar waits beside the bubble's top corner, clear of its time above. */
	.conversation-message.owner .message-footer {
		grid-row: 2;
		grid-column: 1;
		align-self: start;
		justify-self: end;
		min-height: 0;
		margin: 0;
	}

	.conversation-message.owner .message-meta :is(.message-avatar, strong) {
		display: none;
	}

	/* The owner's words: a soft bubble whose tail corner points at the rail's edge they came from. */
	.conversation-message.owner .message-body {
		max-width: 72ch;
		padding: 8px 12px;
		border-radius: 14px 14px 4px 14px;
		background: var(--chat-owner-bg);
		box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--ink) 5%, transparent);
		color: var(--ink);
	}
	.conversation-message.owner.continued .message-body {
		border-top-right-radius: 4px;
	}

	/* Pointing at someone else's message washes its row, as Linear and Slack do, so the floating
	 * actions read as belonging to that message. */
	@media (hover: hover) and (pointer: fine) {
		.conversation-message:not(.owner):not(.embedded):not(.system) {
			border-radius: var(--radius-md);
			transition: background var(--motion-state) var(--ease-standard);
		}
		.conversation-message:not(.owner):not(.embedded):not(.system):hover {
			background: color-mix(in srgb, var(--ink) 2.5%, transparent);
		}
	}

	.conversation-message.system {
		background: var(--chat-context-bg);
		box-shadow: inset 2px 0 0 color-mix(in srgb, var(--intent-feedback) 42%, transparent);
	}

	.conversation-message.pending .message-body {
		opacity: 0.72;
	}

	.conversation-message.embedded {
		width: auto;
		padding: 0;
		border: 0;
		background: transparent;
		box-shadow: none;
	}

	.message-avatar {
		display: inline-flex;
		flex: none;
	}

	.message-initial {
		display: grid;
		width: 24px;
		height: 24px;
		place-items: center;
		border: 1px solid var(--border);
		border-radius: 6px;
		background: var(--surface-alt);
		color: var(--text-secondary);
		font: 600 var(--t-label) var(--font-ui);
	}

	.message-meta {
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 7px;
		min-height: 24px;
	}

	.message-meta strong {
		overflow: hidden;
		color: var(--ink);
		font: 600 var(--t-label) var(--font-ui);
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.message-meta time {
		flex: none;
		color: var(--text-tertiary);
		font: 500 var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
	}

	.message-footer {
		min-height: 18px;
		display: flex;
		align-items: center;
		justify-content: flex-start;
		margin-top: 1px;
	}

	/* One toolbar per message: a small raised pill of borderless icon buttons. Its buttons come from
	 * here and from the hosting surface alike, so every one is styled here, not by its owner. */
	.message-actions {
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 1px;
		padding: 3px;
		border-radius: 9px;
		background: var(--surface-raised);
		box-shadow:
			0 0 0 1px var(--border),
			0 4px 14px rgba(43, 51, 66, 0.1);
	}

	.message-actions :global(:is(button, summary)) {
		width: 26px;
		height: 26px;
		display: grid;
		place-items: center;
		margin: 0;
		padding: 0;
		border: 0;
		border-radius: 6px;
		background: transparent;
		box-shadow: none;
		color: var(--text-tertiary);
		cursor: pointer;
		list-style: none;
		transition:
			background var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard),
			transform var(--motion-press) var(--ease-standard);
	}
	.message-actions :global(summary::-webkit-details-marker) {
		display: none;
	}
	/* A finger needs a bigger target than a pointer; every button in the toolbar grows alike. */
	@media (pointer: coarse) {
		.message-actions :global(:is(button, summary)) {
			width: 44px;
			height: 44px;
		}
	}
	.message-actions :global(:is(button, summary) svg) {
		width: 14px;
		height: 14px;
	}
	.message-actions :global(:is(button, summary):hover) {
		background: color-mix(in srgb, var(--ink) 7%, transparent);
		color: var(--ink);
	}
	.message-actions :global(:is(button, summary):active) {
		transform: scale(0.9);
	}
	.message-actions :global(:is(button, summary):focus-visible) {
		outline: 2px solid color-mix(in srgb, var(--intent-conversation) 40%, transparent);
		outline-offset: 0;
	}
	.message-actions :global(button.confirmed) {
		color: var(--intent-feedback);
		animation: bridge-acknowledge var(--motion-punctuation) var(--ease-out) both;
	}

	/* With a mouse, the toolbar waits for the message you are pointing at and settles in from just
	 * below; nothing reserves room for it. Touch shows it on the tapped message. */
	@media (hover: hover) and (pointer: fine) {
		.message-actions {
			opacity: 0;
			transform: translateY(3px);
			pointer-events: none;
			transition:
				opacity var(--motion-state) var(--ease-standard),
				transform var(--motion-state) var(--ease-out);
		}

		.conversation-message:hover .message-actions,
		.conversation-message:focus-within .message-actions {
			opacity: 1;
			transform: none;
			pointer-events: auto;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.message-actions {
			transform: none !important;
		}
	}

	.message-body {
		min-width: 0;
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.5;
		overflow-wrap: anywhere;
	}

	/* Prose keeps a readable measure in a wide pane; tables and code still use
	 * the full width they need. */
	.message-body :global(.md > :is(p, ul, ol, blockquote, h1, h2, h3, h4, h5, h6)),
	.message-body :global(.message-preview) {
		max-width: 80ch;
	}

	.message-body :global(.md > :first-child) {
		margin-top: 0;
	}

	.message-body :global(.md > :last-child) {
		margin-bottom: 0;
	}

	.message-body :global(:is(strong, h1, h2, h3, h4, h5, h6)) {
		color: var(--ink);
	}

	.takeaway {
		max-width: 80ch;
		margin: 0 0 4px;
		color: var(--ink);
		font-weight: 500;
	}
	.message-text.folded {
		max-height: 30em;
		overflow: hidden;
		mask-image: linear-gradient(to bottom, black 70%, transparent);
	}
	.message-text.folded.after-takeaway {
		max-height: 30em;
	}
	/* Folding is a small pill with a chevron that turns, so a long message reads as folded on
	 * purpose rather than cut off. */
	.fold-toggle {
		display: inline-flex;
		align-items: center;
		gap: 3px;
		margin: 4px 0 0 -7px;
		padding: 2px 7px;
		border: 0;
		border-radius: 999px;
		background: transparent;
		color: var(--text-tertiary);
		font: 500 var(--t-label) var(--font-ui);
		cursor: pointer;
		transition:
			background var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard);
	}
	.fold-toggle:hover {
		background: color-mix(in srgb, var(--ink) 6%, transparent);
		color: var(--ink);
	}
	.fold-toggle :global(svg) {
		transition: transform var(--motion-state) var(--ease-out);
	}
	.fold-toggle.expanded :global(svg) {
		transform: rotate(180deg);
	}
	.gutter-time {
		position: absolute;
		top: 3px;
		left: 6px;
		width: 34px;
		color: var(--text-tertiary);
		font: 500 var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
		text-align: right;
		opacity: 0;
		transition: opacity var(--motion-state) var(--ease-standard);
	}
	.conversation-message:hover .gutter-time,
	.conversation-message:focus-within .gutter-time {
		opacity: 1;
	}
	@media (hover: none) {
		.gutter-time {
			display: none;
		}
	}
	.work-details {
		margin-top: 9px;
	}

	.work-details summary {
		padding: 8px 0 0;
		color: var(--text-tertiary);
		font: 600 var(--t-label) var(--font-ui);
		cursor: pointer;
		list-style: none;
	}

	.work-details summary::-webkit-details-marker {
		display: none;
	}

	.work-details summary::before {
		content: '›';
		display: inline-block;
		width: 13px;
		transition: transform var(--motion-state) var(--ease-out);
	}

	.work-details[open] summary::before {
		transform: rotate(90deg);
	}

	.work-details[open] .work-details-body {
		animation: bridge-disclosure-in var(--motion-disclosure) var(--ease-out) both;
	}

	.work-details-body {
		padding: 7px 0 2px 13px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}

	.copy-status {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border: 0;
	}
</style>
