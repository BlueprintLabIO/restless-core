<script lang="ts">
	import { onDestroy } from 'svelte';
	import type { Snippet } from 'svelte';
	import Check from '@lucide/svelte/icons/check';
	import Copy from '@lucide/svelte/icons/copy';
	import AttachmentList from './AttachmentList.svelte';
	import Markdown from './Markdown.svelte';
	import SemanticMark from './SemanticMark.svelte';
	import type { MessageAttachment, MessageIntentReceipt } from '$lib/model/view';

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
		headerExtra,
		actions,
		embedded = false
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
		headerExtra?: Snippet;
		actions?: Snippet;
		embedded?: boolean;
	} = $props();

	let copyState = $state<'idle' | 'copied' | 'failed'>('idle');
	let messageExpanded = $state(false);
	let copyTimer: number | undefined;

	function timeLabel(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		if (Number.isNaN(date.getTime())) return '';
		return date.toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: 'numeric',
			minute: '2-digit'
		});
	}
	const timestamp = $derived(timeLabel(createdAt));
	const messageDate = $derived(new Date(createdAt));
	const validDate = $derived(!Number.isNaN(messageDate.getTime()));
	const displayAuthor = $derived(author === 'The Exec' ? 'Exec' : author);
	const longOwnerMessage = $derived(
		sender === 'owner' && (text.length > 700 || (text.match(/\n/g)?.length ?? 0) >= 12)
	);
	const messagePreview = $derived(
		text
			.replace(/\s+/g, ' ')
			.trim()
			.slice(0, 200)
			.concat(text.trim().length > 200 ? '…' : '')
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
	class:pending
	class:embedded
	data-message-sender={sender}
>
	<header class="message-meta">
		<span class="message-avatar">
			<SemanticMark
				meaning={sender === 'agent'
					? 'executive'
					: sender === 'owner'
						? 'direction'
						: sender === 'human'
							? 'people'
							: 'work'}
				size="small"
				label={sender === 'owner' ? 'Your message' : `${displayAuthor} message`}
			/>
		</span>
		<strong>{displayAuthor}</strong>
		{#if timestamp}<time
				datetime={validDate ? messageDate.toISOString() : undefined}
				title={validDate ? messageDate.toLocaleString() : undefined}>{timestamp}</time
			>{/if}
		{@render headerExtra?.()}
	</header>

	<div class="message-body">
		{#if longOwnerMessage}
			<details class="message-fold" bind:open={messageExpanded}>
				<summary>
					<span class="message-preview">{messagePreview}</span>
					<span class="message-fold-label">
						{messageExpanded ? 'Show less' : 'Read full message'}
					</span>
				</summary>
				<div class="message-fold-content"><Markdown {text} /></div>
			</details>
		{:else}
			<Markdown {text} />
		{/if}
		{#if sender === 'agent' && (intent?.outcome || intent?.nextStep || intent?.ownerNeed)}
			<dl class="message-glance" aria-label="At a glance">
				{#if intent.outcome}
					<div>
						<dt>Outcome</dt>
						<dd>{intent.outcome}</dd>
					</div>
				{/if}
				{#if intent.nextStep}
					<div>
						<dt>Next</dt>
						<dd>{intent.nextStep}</dd>
					</div>
				{/if}
				{#if intent.ownerNeed}
					<div class="owner-need">
						<dt>Needs you</dt>
						<dd>{intent.ownerNeed}</dd>
					</div>
				{/if}
			</dl>
		{/if}
		<AttachmentList {attachments} {hrefFor} />
		{#if details && sender === 'agent'}
			<details class="work-details">
				<summary>Work details</summary>
				<div class="work-details-body"><Markdown text={details} /></div>
			</details>
		{/if}
	</div>

	{#if copyable && sender !== 'system'}
		<footer class="message-footer">
			<div class="message-actions" aria-label="Message actions">
				{@render actions?.()}
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
			</div>
		</footer>
	{/if}
	<span class="copy-status" aria-live="polite">
		{copyState === 'copied' ? 'Message copied' : copyState === 'failed' ? 'Copy failed' : ''}
	</span>
</article>

<style>
	.conversation-message {
		position: relative;
		width: 100%;
		min-width: 0;
		display: grid;
		gap: 3px;
		padding: 12px 14px 8px;
		border: 0;
		border-bottom: 1px solid var(--border);
		background: var(--chat-agent-bg);
	}

	/* The left edge is a signal, not a frame: it marks the two senders that are
	 * not the default one. An agent message carries no stripe, so its background
	 * starts flush with the rail header instead of looking inset by 2px. */

	.conversation-message.owner {
		background: var(--chat-owner-bg);
		box-shadow: inset 2px 0 0 var(--chat-owner-edge);
	}

	.conversation-message.system {
		background: var(--chat-context-bg);
		box-shadow: inset 2px 0 0 color-mix(in srgb, var(--intent-feedback) 42%, transparent);
	}

	.conversation-message.pending {
		border-top: 1px solid var(--border);
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
		font: 500 var(--t-label) var(--font-mono);
	}

	.message-footer {
		min-height: 18px;
		display: flex;
		align-items: center;
		justify-content: flex-start;
		margin-top: 1px;
	}

	.message-actions {
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 2px;
	}

	@media (pointer: coarse) {
		.copy-message {
			width: 44px;
			height: 44px;
		}
	}

	/* With a mouse, actions wait for the message you are pointing at; their
	 * space stays reserved so nothing moves. Touch keeps them visible. */
	@media (hover: hover) and (pointer: fine) {
		.message-actions {
			opacity: 0;
			transition: opacity var(--motion-state) var(--ease-standard);
		}

		.conversation-message:hover .message-actions,
		.conversation-message:focus-within .message-actions {
			opacity: 1;
		}
	}

	.copy-message {
		width: 18px;
		height: 18px;
		display: grid;
		place-items: center;
		padding: 0;
		border: 1px solid transparent;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
		transition:
			background var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard),
			box-shadow var(--motion-state) var(--ease-standard),
			transform var(--motion-press) var(--ease-standard);
	}

	.copy-message:hover,
	.copy-message:focus-visible {
		border-color: var(--border-strong);
		background: color-mix(in srgb, var(--surface) 78%, transparent);
		color: var(--ink);
	}

	.copy-message:focus-visible {
		outline: 2px solid color-mix(in srgb, var(--intent-conversation) 34%, transparent);
		outline-offset: 1px;
	}

	.copy-message.confirmed {
		color: var(--intent-feedback);
		animation: bridge-acknowledge var(--motion-punctuation) var(--ease-out) both;
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

	.message-fold summary {
		display: grid;
		gap: 5px;
		cursor: pointer;
		list-style: none;
	}

	.message-fold summary::-webkit-details-marker {
		display: none;
	}

	.message-preview {
		display: -webkit-box;
		overflow: hidden;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		color: var(--text-secondary);
	}

	.message-fold[open] .message-preview {
		display: none;
	}

	.message-fold-label {
		width: fit-content;
		color: var(--text-tertiary);
		font: 600 var(--t-label) var(--font-ui);
	}

	.message-fold summary:hover .message-fold-label,
	.message-fold summary:focus-visible .message-fold-label {
		color: var(--ink);
	}

	.message-fold summary:focus-visible {
		outline: 2px solid color-mix(in srgb, var(--intent-conversation) 34%, transparent);
		outline-offset: 2px;
	}

	.message-fold-content {
		padding-top: 8px;
	}

	.message-glance {
		display: grid;
		gap: 0;
		margin: 10px 0 2px;
		border-block: 1px solid var(--border-strong);
	}

	.message-glance > div {
		display: grid;
		grid-template-columns: 62px minmax(0, 1fr);
		gap: 8px;
		padding: 7px 0;
	}

	.message-glance > div + div {
		border-top: 1px solid var(--border);
	}

	.message-glance dt,
	.message-glance dd {
		margin: 0;
		font-size: var(--t-body);
		line-height: 1.42;
	}

	.message-glance dt {
		font-weight: 600;
		color: var(--text-tertiary);
	}

	.message-glance dd {
		color: var(--ink);
	}

	.message-glance .owner-need dt {
		color: var(--intent-authority);
	}

	.work-details {
		margin-top: 9px;
		border-top: 1px solid var(--border-strong);
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
