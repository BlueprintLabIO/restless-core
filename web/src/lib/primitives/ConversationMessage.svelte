<script lang="ts">
	import { onDestroy } from 'svelte';
	import Check from '@lucide/svelte/icons/check';
	import Copy from '@lucide/svelte/icons/copy';
	import AttachmentList from './AttachmentList.svelte';
	import Markdown from './Markdown.svelte';
	import SemanticMark from './SemanticMark.svelte';
	import type { MessageAttachment } from '$lib/model/view';

	let {
		sender,
		author,
		text,
		createdAt,
		details = null,
		attachments = [],
		hrefFor,
		copyable = true,
		pending = false,
		domId
	}: {
		sender: 'owner' | 'agent' | 'system';
		author: string;
		text: string;
		createdAt: Date | string;
		details?: string | null;
		attachments?: MessageAttachment[];
		hrefFor?: (attachment: MessageAttachment) => string;
		copyable?: boolean;
		pending?: boolean;
		domId?: string;
	} = $props();

	let copyState = $state<'idle' | 'copied' | 'failed'>('idle');
	let copyTimer: number | undefined;

	function timeLabel(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		if (Number.isNaN(date.getTime())) return '';
		return date.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' }).toLowerCase();
	}
	const timestamp = $derived(timeLabel(createdAt));
	const displayAuthor = $derived(author === 'The Exec' ? 'Exec' : author);

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
	data-message-sender={sender}
>
	<header class="message-meta">
		<span class="message-avatar">
			<SemanticMark
				meaning={sender === 'agent' ? 'executive' : sender === 'owner' ? 'direction' : 'work'}
				size="small"
				label={sender === 'owner' ? 'Your message' : `${displayAuthor} message`}
			/>
		</span>
		<strong>{displayAuthor}</strong>
		{#if timestamp}<time>{timestamp}</time>{/if}
	</header>

	<div class="message-body">
		{#if sender === 'agent'}
			<Markdown {text} />
		{:else}
			<p>{text}</p>
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

	.message-body > p {
		margin: 0;
		white-space: pre-wrap;
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
