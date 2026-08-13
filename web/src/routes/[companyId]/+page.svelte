<script lang="ts">
	/* The inbox — the landing surface. Two questions, answered in order: what needs
	 * your word, and what happened while you were away.
	 *
	 * The brief at the top is the morning in one read. When the executive can
	 * compose, its prose slots into this same frame; until then the counts carry it,
	 * and they are honest because they are the record rather than a summary of it. */

	import { page } from '$app/state';
	import Hint from '$lib/primitives/Hint.svelte';
	import PaneHeader from '$lib/primitives/PaneHeader.svelte';
	import HoldApprove from '$lib/primitives/HoldApprove.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import type { NeedsYouItem } from '$lib/model/view';
	import { cosmon } from '$lib/fixtures/cosmon';

	const desk = cosmon;
	const companyId = $derived(page.params.companyId ?? desk.company.id);
	const canOperate = $derived(['owner', 'operator'].includes(desk.membershipRole));
	const canAdminister = $derived(desk.membershipRole === 'owner');

	const needsYou = desk.needsYou;
	const updates = desk.updates;

	const dawnMs = (() => {
		const dawn = new Date();
		dawn.setHours(6, 0, 0, 0);
		return dawn.getTime();
	})();
	const sinceDawn = desk.tape.filter((entry) => new Date(entry.at).getTime() >= dawnMs);
	const inFlight = desk.hq.activeRuns;
	const briefDate = new Date()
		.toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long' })
		.toLowerCase();

	const selectedItemId = $derived(page.url.searchParams.get('item'));
	const selectedItem = $derived(
		needsYou.find((item) => item.id === selectedItemId) ??
			(selectedItemId ? null : (needsYou[0] ?? null))
	);

	const baseHref = $derived(`/${companyId}`);

	function itemHref(id: string): string {
		return `${baseHref}?item=${encodeURIComponent(id)}`;
	}

	function updateHref(update: (typeof updates.items)[number]): string {
		return update.target.kind === 'record'
			? `${baseHref}/library?record=${update.target.assetId}`
			: `${baseHref}/chats?t=${encodeURIComponent(update.target.thread)}`;
	}

	function when(value: Date | string | null): string {
		if (value == null) return '';
		const date = value instanceof Date ? value : new Date(value);
		return date.toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function kicker(item: NeedsYouItem): string {
		if (item.kind === 'decision') return 'A decision waits on your word';
		if (item.kind === 'email-approval') return 'An email sends on your word';
		if (item.kind === 'promotion-approval') return 'A change promotes on your word';
		return 'A human step is needed';
	}

	/* Unwired: approving, declining and responding all travel a governed path that
	 * does not exist yet. The affordances stay — they are the surface — but they
	 * resolve to nothing rather than posting into a void. */
	function inert(event: SubmitEvent) {
		event.preventDefault();
	}
</script>

<svelte:head><title>Inbox — {desk.company.name}</title></svelte:head>

<div class="bridge-page bridge-bleed bridge-inbox">
	<div class="page-head">
		<div>
			<h1>
				Inbox<Hint
					text="What needs your word, and what happened while you were away."
					label="What the inbox is"
				/>
			</h1>
		</div>
		{#if updates.unreadCount > 0}
			<form onsubmit={inert}>
				<button class="btn small" type="submit">Mark all read</button>
			</form>
		{/if}
	</div>

	<div class="pane-frame">
		<section class="pane brief in-p-brief">
			<PaneHeader title="The brief — {briefDate}">
				{#snippet action()}
					<span class="brief-src mono">from the record · the executive reads the same tape</span>
				{/snippet}
			</PaneHeader>
			<div class="brief-segs">
				<span class="brief-seg" class:lit={needsYou.length > 0}>
					<MatrixGlyph rows={GLYPHS.ring} size={10} glow={needsYou.length > 0} />
					<b>{needsYou.length}</b> need your word
				</span>
				<span class="brief-seg" class:lit={inFlight > 0}>
					<MatrixGlyph rows={GLYPHS.dots} size={10} glow={inFlight > 0} />
					<b>{inFlight}</b> in flight
				</span>
				<span class="brief-seg">
					<MatrixGlyph rows={GLYPHS.work} size={10} />
					<b>{sinceDawn.length}</b> on the tape since 06:00
				</span>
			</div>
			{#if sinceDawn.length > 0}
				<div class="brief-rows">
					{#each sinceDawn.slice(0, 3) as entry (`${entry.kind}:${entry.id}`)}
						<div class="brief-row">
							<span class="brief-time mono">{when(entry.at)}</span>
							<span class="brief-entry">{entry.summary}</span>
						</div>
					{/each}
				</div>
			{/if}
		</section>

		<div class="pane-row inbox-body">
			<div class="pane-rail">
				<!-- Two regions, two panes. A heading floating over rows it has no structural
				     relationship to is not a heading, it is a caption pretending. -->
				<section class="pane inbox-list in-p-list">
					<PaneHeader title="Needs your word · {needsYou.length}" />
					{#each needsYou as item (item.id)}
						<a
							class="ib-row"
							class:selected={selectedItem?.id === item.id}
							href={itemHref(item.id)}
							aria-current={selectedItem?.id === item.id ? 'true' : undefined}
						>
							<span class="ib-dot waiting"></span>
							<span class="ib-col">
								<span class="ib-title">{item.title}</span>
								<span class="ib-meta">{item.kind.replaceAll('-', ' ')} · {item.detail}</span>
							</span>
							{#if item.createdAt}<span class="ib-when">{when(item.createdAt)}</span>{/if}
						</a>
					{:else}
						<!-- the empty state lives in the detail pane — saying it twice reads as a bug -->
					{/each}
				</section>

				<section class="pane inbox-list in-p-list">
					<PaneHeader title="Updates" />
					{#each updates.items as update (update.id)}
						<a class="ib-row" href={updateHref(update)}>
							<span class="ib-dot" class:unread={update.unread}></span>
							<span class="ib-col">
								<span class="ib-title" class:read={!update.unread}>{update.title}</span>
								<span class="ib-meta">{update.detail}</span>
							</span>
							<span class="ib-when">{when(update.at)}</span>
						</a>
					{:else}
						<p class="caption" style="padding: 0 16px 8px">
							Nothing yet — everything the company does lands here, and on the tape.
						</p>
					{/each}
					{#if updates.total > updates.items.length}
						<p class="caption" style="padding: 8px 16px">
							Showing the newest {updates.items.length} of {updates.total} — the full history is on the
							tape.
						</p>
					{/if}
				</section>
			</div>

			<div class="inbox-detail">
				{#if selectedItem}
					{@render inboxDetail(selectedItem)}
				{:else}
					<div class="pane inbox-emptycard">
						<p class="inbox-empty-h">Nothing needs your word.</p>
						<p class="caption" style="margin: 0">
							Decisions, approvals, and escalations land here the moment they need a human.
						</p>
					</div>
				{/if}
			</div>
		</div>
	</div>
</div>

{#snippet inboxDetail(item: NeedsYouItem)}
	<div class="inbox-pane">
		<div class="needs-card">
			<div class="nc-kicker">{kicker(item)}</div>
			<div class="nc-title">{item.title}</div>
			<div class="nc-detail">{item.detail}</div>

			{#if item.context.kind === 'email-approval' && item.context.draft}
				<blockquote class="ib-quote">{item.context.draft.textBody}</blockquote>
			{:else if item.context.kind === 'decision' && item.context.proposalFacts.length > 0}
				<div class="ib-facts">
					{#each item.context.proposalFacts as [label, value] (label)}
						<div class="kv"><span>{label}</span><b>{value}</b></div>
					{/each}
				</div>
			{:else if item.context.kind === 'escalation' && item.context.why}
				<p class="nc-detail">{item.context.why}</p>
			{/if}

			{#if item.kind === 'decision'}
				<div class="nc-actions">
					{#if canAdminister}
						<form onsubmit={inert}>
							<HoldApprove small label="Hold to approve" />
						</form>
						<form onsubmit={inert}>
							<button class="btn small danger" type="submit">Decline</button>
						</form>
					{:else}
						<span class="caption">Needs an owner's hand.</span>
					{/if}
				</div>
			{:else if item.kind === 'email-approval' || item.kind === 'promotion-approval'}
				<div class="nc-actions">
					{#if canAdminister}
						<form onsubmit={inert}>
							<HoldApprove small label="Hold to sign" />
						</form>
						<form onsubmit={inert}>
							<button class="btn small danger" type="submit">Decline</button>
						</form>
					{:else}
						<span class="caption">Needs an owner's hand.</span>
					{/if}
				</div>
			{:else if item.kind === 'escalation'}
				<form onsubmit={inert}>
					<div class="nc-actions" style="flex-direction: column; align-items: stretch">
						<input
							class="comp-input"
							style="min-height: 0; padding: 8px 12px"
							name="note"
							minlength="3"
							maxlength="1000"
							required
							placeholder="A line for the record — what did you do or decide?"
							aria-label="A line for the record — what did you do or decide?"
							disabled={!canOperate}
						/>
						<div style="display: flex; gap: 8px">
							{#if canOperate}
								<HoldApprove small label="Hold to mark done" />
								<button class="btn small" type="submit">Hand back</button>
							{:else}
								<span class="caption">Needs an operator's hand.</span>
							{/if}
						</div>
					</div>
				</form>
			{/if}
		</div>
	</div>
{/snippet}
