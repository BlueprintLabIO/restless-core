<script lang="ts">
	/* Chats — the conversational surface. A list of direct lines and goal channels
	 * on the left, one thread on the right, the composer at the bottom.
	 *
	 * The composer is the one instrument (design-language pillar 5): it gets the
	 * bevel, the glass, the lit top edge. Everything else on this screen stays
	 * matte, because this is where the operator's hands are. */

	import { page } from '$app/state';
	import StartModal from '$lib/components/StartModal.svelte';
	import AttachmentList from '$lib/primitives/AttachmentList.svelte';
	import Composer from '$lib/primitives/Composer.svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import { initialsOf, type ThreadSummary } from '$lib/model/view';
	import { composerDisclosure } from '$lib/model/disclosure-state';
	import { rail } from '$lib/model/rail.svelte';
	import { cosmon } from '$lib/fixtures/cosmon';

	const desk = cosmon;
	const companyId = $derived(page.params.companyId ?? desk.company.id);
	const canOperate = $derived(['owner', 'operator'].includes(desk.membershipRole));

	const threads = desk.threads;

	/* On narrow screens the surface goes list-first: no ?t means the list,
	 * a selected thread means the thread — with a way back. */
	const threadSelected = $derived(page.url.searchParams.get('t') != null);
	const selectedKey = $derived(page.url.searchParams.get('t')?.trim() || 'executive');
	const selected = $derived(
		threads.find((thread) => thread.key === selectedKey) ?? threads[0] ?? null
	);
	const messages = $derived(selected ? (desk.messages[selected.key] ?? []) : []);

	let query = $state('');
	const visible = $derived(
		query.trim()
			? threads.filter((thread) =>
					`${thread.title} ${thread.subtitle}`.toLowerCase().includes(query.trim().toLowerCase())
				)
			: threads
	);
	const dmThreads = $derived(visible.filter((thread) => thread.kind !== 'goal'));
	const channelThreads = $derived(visible.filter((thread) => thread.kind === 'goal'));

	const pigVar = (pig: number) => `var(--pig-${pig})`;

	const baseHref = $derived(`/${companyId}/chats`);

	function threadHref(thread: ThreadSummary): string {
		/* Always carry ?t — the executive thread too, so list-first navigation
		 * on narrow screens can reach it. */
		return `${baseHref}?t=${encodeURIComponent(thread.key)}`;
	}

	function hhmm(value: Date | string | null): string {
		if (value == null) return '';
		const date = value instanceof Date ? value : new Date(value);
		if (Number.isNaN(date.getTime())) return '';
		return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	}

	/* Day separators ground the thread in time — computed from the record,
	 * so a replay and a live thread group the same way. */
	function dayOf(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		return Number.isNaN(date.getTime()) ? '' : date.toDateString();
	}

	function dayLabel(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		if (Number.isNaN(date.getTime())) return '';
		const today = new Date();
		if (date.toDateString() === today.toDateString()) return 'Today';
		const yesterday = new Date();
		yesterday.setDate(today.getDate() - 1);
		if (date.toDateString() === yesterday.toDateString()) return 'Yesterday';
		return date.toLocaleDateString(undefined, { month: 'long', day: 'numeric' });
	}

	function recordHref(assetId: string): string {
		return `/${companyId}/library?record=${assetId}`;
	}

	const composerName = $derived(
		selected?.kind === 'goal' ? `#${selected.title}` : (selected?.title ?? '')
	);

	/* Provider disclosure is a standing company setting, not a per-message choice —
	 * the composer shows the record-only state and carries no control. */
	const disclosureState = $derived(
		composerDisclosure({
			providerDisclosureEnabled: desk.providerDisclosureEnabled,
			hasResponder: desk.executiveConnected,
			companyId
		})
	);

	let composer = $state('');
	let composerFiles = $state<File[]>([]);

	const startOpen = $derived(page.url.searchParams.get('start') === '1');

	/* Unwired: sending travels the one governed path to an employee, which does not
	 * exist yet. The composer is live to type in and refuses to pretend it sent. */
	function inert(event: SubmitEvent) {
		event.preventDefault();
	}
</script>

<svelte:head><title>Chats — {desk.company.name}</title></svelte:head>

<aside class="bridge-side" class:mobile-hidden={threadSelected}>
	<div class="side-head">
		<h1>Chats</h1>
		<a
			class="new-chat"
			href={`${baseHref}?start=1`}
			title="New: message, group, or hire"
			aria-label="Start something new"><MatrixGlyph rows={GLYPHS.plus} size={11} /></a
		>
	</div>
	<input
		class="side-search"
		type="search"
		placeholder="Search people and channels"
		aria-label="Search people and channels"
		bind:value={query}
	/>
	<div class="side-scroll">
		<div class="side-section">Direct messages</div>
		{#each dmThreads as thread (thread.key)}
			<a class="conv-row" class:on={selected?.key === thread.key} href={threadHref(thread)}>
				{#if thread.kind === 'executive'}
					<span class="avatar avatar-glyph">
						<MatrixGlyph rows={GLYPHS.p} size={13} glow={thread.live} />
						<span class="dot" class:working={thread.live} class:offline={!thread.live}></span>
					</span>
				{:else}
					<span class="avatar" style={`background: ${pigVar(thread.pig)}`}>
						{initialsOf(thread.title)}
						<span class="dot" class:working={thread.live} class:offline={!thread.live}></span>
					</span>
				{/if}
				<span class="c-main">
					<span class="c-top">
						<span class="c-name">{thread.title}</span>
						<span class="c-time">{hhmm(thread.lastAt)}</span>
					</span>
					<span class="c-preview">{thread.preview || thread.subtitle}</span>
				</span>
			</a>
		{:else}
			<p class="caption" style="padding: 0 16px">No employees yet — hire one from ✎.</p>
		{/each}
		{#if channelThreads.length > 0}
			<div class="side-section">Channels</div>
			{#each channelThreads as thread (thread.key)}
				<a class="conv-row" class:on={selected?.key === thread.key} href={threadHref(thread)}>
					<span class="avatar avatar-chan">#</span>
					<span class="c-main">
						<span class="c-top">
							<span class="c-name">{thread.title}</span>
							<span class="c-time">{hhmm(thread.lastAt)}</span>
						</span>
						<span class="c-preview">{thread.preview || thread.subtitle}</span>
					</span>
				</a>
			{/each}
		{/if}
	</div>
</aside>

<section class="bridge-thread" class:mobile-hidden={!threadSelected}>
	{#if selected}
		<header class="t-head">
			<a class="btn small mobile-back" href={baseHref}>‹ Chats</a>
			{#if selected.kind === 'executive'}
				<span class="avatar avatar-glyph">
					<MatrixGlyph rows={GLYPHS.p} size={13} glow={selected.live} />
					<span class="dot" class:working={selected.live} class:offline={!selected.live}></span>
				</span>
			{:else if selected.kind === 'goal'}
				<span class="avatar avatar-chan">#</span>
			{:else}
				<span class="avatar" style={`background: ${pigVar(selected.pig)}`}>
					{initialsOf(selected.title)}
					<span class="dot" class:working={selected.live} class:offline={!selected.live}></span>
				</span>
			{/if}
			<div style="min-width: 0; flex: 1">
				<div class="t-name">
					{selected.kind === 'goal' ? `#${selected.title}` : selected.title}
				</div>
				<div class="t-sub">{selected.subtitle}</div>
			</div>
			{#if selected.kind === 'agent' && selected.subjectId}
				<a class="btn small" href="/{companyId}/staff/{selected.subjectId}">View profile</a>
			{/if}
		</header>
		<div class="t-scroll">
			{#if messages.length === 0}
				<div class="thread-empty">
					{#if selected.kind === 'agent'}
						<p class="thread-empty-h">The start of your direct line.</p>
						<p class="caption">Every message with {selected.title} lands on the tape.</p>
					{:else if selected.kind === 'goal'}
						<p class="thread-empty-h">The channel for “{selected.title}”.</p>
						<p class="caption">The conversation around this goal lands on the tape.</p>
					{:else}
						<p class="thread-empty-h">Ask anything.</p>
						<p class="caption">
							The executive reads the whole company record before it answers — every word lands on
							the tape.
						</p>
					{/if}
				</div>
			{/if}
			{#each messages as message, i (message.id)}
				{#if i === 0 || dayOf(message.createdAt) !== dayOf(messages[i - 1].createdAt)}
					<div class="day-sep" aria-hidden="true"><span>{dayLabel(message.createdAt)}</span></div>
				{/if}
				{#if message.from === 'system'}
					<p class="msg system">{message.text}</p>
				{:else}
					<div class="msg" class:you={message.from === 'you'}>
						<div style="min-width: 0">
							<div class="m-meta" style={message.from === 'you' ? 'justify-content: flex-end' : ''}>
								<span class="m-from">{message.from === 'you' ? 'You' : message.author}</span>
								<span class="m-time">{hhmm(message.createdAt)}</span>
							</div>
							<div class="bubble">
								{#if message.from === 'agent'}
									<!-- Employees write Markdown; the owner should read structure, not syntax.
								     Your own words and system statuses stay literal. -->
									<Markdown text={message.text} />
								{:else}
									{message.text}
								{/if}
							</div>
							<AttachmentList attachments={message.attachments} />
							{#if message.assetId}
								<a class="artifact-card" href={recordHref(message.assetId)}>
									<span class="a-kind">rec</span>
									<span style="min-width: 0">
										<span class="a-name" style="display: block">versioned record</span>
										<span class="a-meta">the reply's evidence — open it in the library</span>
									</span>
								</a>
							{/if}
							{#if message.runId || message.assetId}
								<div
									class="ref-row"
									style={message.from === 'you' ? 'justify-content: flex-end' : ''}
								>
									{#if message.runId}<span class="ref-tag" title={message.runId}>run</span>{/if}
									{#if message.assetId}<a class="ref-tag" href={recordHref(message.assetId)}
											>record</a
										>{/if}
								</div>
							{/if}
						</div>
					</div>
				{/if}
			{/each}
		</div>

		<form class="bridge-composer" onsubmit={inert}>
			<Composer
				bind:value={composer}
				bind:files={composerFiles}
				disabled={!canOperate}
				providerDisclosureEnabled={desk.providerDisclosureEnabled}
				placeholder={`Message ${composerName}`}
				ariaLabel={`Message ${composerName}`}
			>
				{#snippet disclosure()}
					<!-- Two states about an answer that will not arrive, in the one place where you
					     are about to ask for one. They are mutually exclusive: with no runtime
					     connected there is no provider question to raise yet. -->
					{#if !desk.executiveConnected}
						<p class="comp-connect">
							<span>Connect a runtime to talk to your executive.</span>
							<button type="button" class="comp-connect-cta" onclick={() => (rail.open = true)}>
								Connect →
							</button>
						</p>
					{:else if disclosureState.kind === 'record-only'}
						<p class="comp-recordonly">
							{disclosureState.message}
							<a href={disclosureState.settingsHref}>Change it in settings</a>
						</p>
					{/if}
				{/snippet}
			</Composer>
		</form>
	{/if}
</section>

{#if startOpen}
	<StartModal team={desk.hq.team} {baseHref} {canOperate} />
{/if}
