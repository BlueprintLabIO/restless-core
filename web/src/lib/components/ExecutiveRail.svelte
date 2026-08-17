<script lang="ts">
	/* The executive rail for v3: the Chief of Staff, one gesture away on any
	 * surface. It stays mounted and opens by taking real space — the page slides
	 * left as the rail grows to its full width, nothing floats on top. Everything
	 * it renders arrives as props; nothing is fetched here. Asking hands the text
	 * to `onask`, which owns the one governed path to the executive. */

	import { SvelteDate } from 'svelte/reactivity';
	import { EXEC_FALLBACK_NAME } from '$lib/brand/brand';
	import AttachmentList from '$lib/primitives/AttachmentList.svelte';
	import Composer from '$lib/primitives/Composer.svelte';
	import IntentReceipt from '$lib/primitives/IntentReceipt.svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import SemanticMark from '$lib/primitives/SemanticMark.svelte';
	import type { ThreadMessage } from '$lib/model/view';

	let {
		messages = [],
		execName = EXEC_FALLBACK_NAME,
		companyId,
		membershipRole,
		executiveConnected = false,
		contextLabel = 'Current screen',
		open = true,
		onask = null
	}: {
		messages?: ThreadMessage[];
		execName?: string;
		companyId: string;
		membershipRole: string;
		/**
		 * Whether the executive has a bound ACP runtime. This must come from a LIVE
		 * probe of the runtime, never from configuration — "probe, never guess". Until
		 * it is true the rail is glass-locked instead of implying that conversation is
		 * available. Runtime/provider administration stays outside the owner cockpit.
		 */
		executiveConnected?: boolean;
		contextLabel?: string;
		open?: boolean;
		/** Returns an error message, or null on success. Unwired: null, composer inert. */
		onask?:
			((text: string, files: File[], includeContext: boolean) => Promise<string | null>) | null;
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
</script>

<aside
	id="bridge-exrail"
	class="bridge-exrail"
	class:open
	aria-label="Executive conversation"
	aria-hidden={!open}
	inert={!open}
>
	{#if !executiveConnected}
		<div class="exr-lock">
			<div class="exr-lock-card">
				<span class="exr-lock-badge" aria-hidden="true"
					><MatrixGlyph rows={GLYPHS.p} size={18} glow /></span
				>
				<h2 class="exr-lock-h">Executive unavailable</h2>
				<p class="exr-lock-p">
					The company computer has not confirmed that {execName} is reachable. Conversation will open
					automatically when the live connection returns.
				</p>
				<p class="exr-lock-note">Connection is managed by the company computer.</p>
			</div>
		</div>
	{/if}
	<div class="exr-inner" inert={!executiveConnected}>
		<header class="exr-head">
			<div class="exr-who">
				<SemanticMark meaning="executive" />
				<span>
					<strong class="exr-name">Executive</strong>
					<small class="exr-sub">Context follows the focused work</small>
				</span>
			</div>
		</header>

		<div class="exr-msgs" bind:this={scrollEl}>
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
							<span>{message.from === 'you' ? 'You' : message.author || execName}</span>
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
					<p class="exr-empty-h">Ask anything.</p>
					<p class="exr-empty-p">
						The executive reads the whole company record before it answers — goals, staff, work in
						flight, and what needs your word.
					</p>
				</div>
			{/each}
			{#if waiting}
				<div class="exr-thinking" aria-label="The executive is preparing an answer">
					<i></i><i></i><i></i>
				</div>
			{/if}
		</div>

		<form class="exr-composer" onsubmit={submitAsk}>
			<Composer
				bind:value={composer}
				bind:files={composerFiles}
				disabled={!canOperate || !onask}
				placeholder="Ask, redirect, or make a judgement…"
				ariaLabel={`Ask ${execName}`}
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
				<span>Exec interprets intent and confirms consequential changes</span>
				<span>⌘ ↵ send</span>
			</div>
			{#if askError}
				<p class="exr-error" role="alert">{askError}</p>
			{/if}
		</form>
	</div>
</aside>
