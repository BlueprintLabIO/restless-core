<script lang="ts">
	/* The executive rail for v3: the Chief of Staff, one gesture away on any
	 * surface. It stays mounted and opens by taking real space — the page slides
	 * left as the rail grows to its full width, nothing floats on top. Everything
	 * it renders arrives as props; nothing is fetched here. Asking hands the text
	 * to `onask`, which owns the one governed path to the executive. */

	import { onMount } from 'svelte';
	import { SvelteDate } from 'svelte/reactivity';
	import { EXEC_FALLBACK_NAME } from '$lib/brand/brand';
	import AttachmentList from '$lib/primitives/AttachmentList.svelte';
	import Composer from '$lib/primitives/Composer.svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import type { ThreadSummary, ThreadMessage, NeedsYouItem } from '$lib/model/view';
	import { composerDisclosure } from '$lib/model/disclosure-state';

	let {
		thread = null,
		messages = [],
		needsYou = [],
		execName = EXEC_FALLBACK_NAME,
		companyId,
		membershipRole,
		providerDisclosureEnabled = false,
		executiveConnected = false,
		open,
		onclose,
		onask = null,
		onconnect = null
	}: {
		thread?: ThreadSummary | null;
		messages?: ThreadMessage[];
		needsYou?: NeedsYouItem[];
		execName?: string;
		companyId: string;
		membershipRole: string;
		providerDisclosureEnabled?: boolean;
		/**
		 * Whether the executive has a bound ACP runtime. This must come from a LIVE
		 * probe of the runtime, never from configuration — "probe, never guess". Until
		 * it is true the rail is glass-locked and offers the connect action instead of
		 * the conversation, which is the honest answer when nothing has been checked.
		 */
		executiveConnected?: boolean;
		open: boolean;
		onclose: () => void;
		/** Returns an error message, or null on success. Unwired: null, composer inert. */
		onask?: ((text: string, files: File[]) => Promise<string | null>) | null;
		onconnect?: ((provider: 'codex' | 'claude') => Promise<string | null>) | null;
	} = $props();

	const canOperate = $derived(['owner', 'operator'].includes(membershipRole ?? ''));

	/* Provider disclosure is a standing company setting, not a per-message question.
	 * Asking it again on every send meant the default send did not reach the provider
	 * at all unless the box was ticked first. */
	const disclosureState = $derived(
		composerDisclosure({
			providerDisclosureEnabled,
			hasResponder: executiveConnected,
			companyId
		})
	);

	/* Connecting the executive to a local ACP provider. */
	let connecting = $state<'codex' | 'claude' | null>(null);
	let connectError = $state('');
	async function connect(provider: 'codex' | 'claude') {
		if (connecting || !onconnect) return;
		connecting = provider;
		connectError = '';
		try {
			connectError = (await onconnect(provider)) ?? '';
		} finally {
			connecting = null;
		}
	}

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

	let composer = $state('');
	let composerFiles = $state<File[]>([]);
	let askError = $state('');
	let scrollEl = $state<HTMLDivElement | undefined>();

	/* keep the newest word in view — only while the rail is actually showing */
	$effect(() => {
		if (!open || messages.length === 0) return;
		scrollEl?.scrollTo({ top: scrollEl.scrollHeight });
	});

	onMount(() => {
		const onKey = (event: KeyboardEvent) => {
			if (event.key === 'Escape' && open) onclose();
		};
		window.addEventListener('keydown', onKey);
		return () => window.removeEventListener('keydown', onKey);
	});

	/* Asking from the rail posts to the chats surface (the ask action lives
	 * there) and lands on the executive thread; the inbox is the landing
	 * surface, so its link needs no query param. */
	const inboxHref = $derived(`/${companyId}`);

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
			const failure = await onask(text, files);
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
	aria-label="Executive chat"
	aria-hidden={!open}
	inert={!open}
>
	{#if !executiveConnected}
		<div class="exr-lock">
			<div class="exr-lock-card">
				<span class="exr-lock-badge" aria-hidden="true"
					><MatrixGlyph rows={GLYPHS.p} size={18} glow /></span
				>
				<h2 class="exr-lock-h">Connect your executive</h2>
				<p class="exr-lock-p">
					Bind {execName} to the company's configured ACP runtime to start the conversation.
				</p>
				{#if onconnect}
					<div class="exr-lock-providers">
						<button
							type="button"
							class="exr-lock-provider"
							disabled={!canOperate || connecting !== null}
							onclick={() => connect('codex')}
						>
							{connecting === 'codex' ? 'Connecting…' : 'Codex'}
						</button>
						<button
							type="button"
							class="exr-lock-provider"
							disabled={!canOperate || connecting !== null}
							onclick={() => connect('claude')}
						>
							{connecting === 'claude' ? 'Connecting…' : 'Claude'}
						</button>
					</div>
				{:else}
					<p class="exr-lock-note">Connection is managed by the company Runtime.</p>
				{/if}
				{#if !canOperate}
					<p class="exr-lock-note">Ask an owner or operator to connect the executive.</p>
				{/if}
				{#if connectError}
					<p class="exr-error" role="alert">{connectError}</p>
				{/if}
			</div>
		</div>
	{/if}
	<div class="exr-inner" inert={!executiveConnected}>
		<header class="exr-head">
			<div class="exr-who">
				<span class="exr-name">
					<span class="exr-lamp" class:live={thread?.live ?? false}
						><MatrixGlyph
							rows={(thread?.live ?? false) ? GLYPHS.dots : GLYPHS.ring}
							size={9}
							glow={thread?.live ?? false}
						/></span
					>{execName}
				</span>
				<span class="exr-sub">
					{executiveConnected ? 'answers from the company record' : 'no executive connected yet'}
				</span>
			</div>
			<button class="exr-close" aria-label="Close" onclick={onclose}>×</button>
		</header>

		{#if needsYou.length > 0}
			<a class="exr-needs" href={inboxHref} onclick={onclose}>
				<strong>{needsYou.length}</strong> need your word — open the inbox ▸
			</a>
		{/if}

		<div class="exr-msgs" bind:this={scrollEl}>
			{#each messages as message, i (message.id)}
				{#if i === 0 || dayOf(message.createdAt) !== dayOf(messages[i - 1].createdAt)}
					<div class="day-sep" aria-hidden="true"><span>{dayLabel(message.createdAt)}</span></div>
				{/if}
				<div class="exr-row {message.from === 'you' ? 'you' : 'agent'}">
					<div class="exr-b">
						{#if message.from === 'system'}
							<p class="exr-system">{message.text}</p>
						{:else if message.from === 'agent'}
							<!-- Employees write Markdown; the owner should read structure, not syntax. -->
							<Markdown text={message.text} />
							<AttachmentList attachments={message.attachments} />
						{:else}
							<p>{message.text}</p>
							<AttachmentList attachments={message.attachments} />
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
				{providerDisclosureEnabled}
				placeholder={`Ask ${execName}…`}
				ariaLabel={`Ask ${execName}`}
			>
				{#snippet disclosure()}
					{#if disclosureState.kind === 'record-only'}
						<p class="comp-recordonly">
							{disclosureState.message}
							<a href={disclosureState.settingsHref}>Change it in settings</a>
						</p>
					{/if}
				{/snippet}
			</Composer>
			{#if askError}
				<p class="exr-error" role="alert">{askError}</p>
			{/if}
		</form>
	</div>
</aside>
