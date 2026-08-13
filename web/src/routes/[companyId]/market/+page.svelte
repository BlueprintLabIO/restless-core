<script lang="ts">
	/* The market: who sells to this company, what they sell, and the governed
	 * way to ask for more. Read side is the desk (parties, offerings, vendor
	 * workers); the write side is sourcing.request.open.v1 — a sourcing request
	 * is a decision, so it lands on the tape. Nothing here is invented: a vendor
	 * with no recorded price simply has no price shown. */

	import { page } from '$app/state';
	import { composeOfferBoards, hasRole, statusMark } from '$lib/model/market-view';
	import Hint from '$lib/primitives/Hint.svelte';
	import PaneHeader from '$lib/primitives/PaneHeader.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import { cosmon, market } from '$lib/fixtures/cosmon';

	const desk = cosmon;
	const companyId = $derived(page.params.companyId ?? desk.company.id);
	const canOperate = $derived(['owner', 'operator'].includes(desk.membershipRole));

	const vendors = market.parties.filter((party) => hasRole(party, 'vendor'));
	const offerings = market.offerings;

	/* One board per brief, its shortlist as rows — `candidates` is the only genuinely
	 * two-sided content on the surface, so it is not flattened into a joined string. */
	const boards = composeOfferBoards(market.sourcingRequests);

	const vendorHref = (partyId: string) => `/${companyId}/market/${partyId}`;

	function day(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		return Number.isNaN(date.getTime())
			? '—'
			: date.toLocaleDateString(undefined, { day: 'numeric', month: 'short', year: 'numeric' });
	}

	function partyName(partyId: string | null): string {
		if (!partyId) return '';
		return market.parties.find((party) => party.id === partyId)?.name ?? '';
	}
	function workersOf(partyId: string) {
		return market.vendorWorkers.filter((worker) => worker.vendorPartyId === partyId);
	}
	function textOf(value: unknown): string {
		if (Array.isArray(value))
			return value.filter((v): v is string => typeof v === 'string').join(' · ');
		return typeof value === 'string' ? value : '';
	}
	function money(cents: number | null, currency: string): string {
		if (cents == null) return '';
		try {
			return new Intl.NumberFormat(undefined, {
				style: 'currency',
				currency: currency || 'USD'
			}).format(cents / 100);
		} catch {
			return `${(cents / 100).toFixed(2)} ${currency}`;
		}
	}
	/* `statusMark` classifies the recorded status. A naive three-way check reads `sourced`
	 * inside `not_sourced` and renders a bench vendor as available, and it has no way to say
	 * `inconsistent` — which is a conclusion reconciliation genuinely has to be able to reach. */

	/* The brief — a job posting, not a search box. Category alone told a vendor nothing about
	 * whether they could take the work; a cap, a date, and the credentials required are what
	 * separate "I need legal review" from something biddable. All optional, so an operator in a
	 * hurry still just types the need. */
	const SOURCING_CATEGORIES = ['buy', 'hire', 'make', 'automate'] as const;
	let sourcingNeed = $state('');
	let sourcingCategory = $state<(typeof SOURCING_CATEGORIES)[number]>('buy');
	let sourcingBudget = $state('');
	let sourcingDeadline = $state('');
	let sourcingRequirements = $state('');
	let sourcingSending = $state(false);
	let sourcingNote = $state('');
	let sourcingError = $state('');

	/** Pre-fill the brief from an offering — the store idea that carries over honestly. */
	function requestOffering(offering: (typeof offerings)[number]) {
		const provider = partyName(offering.providerPartyId);
		sourcingNeed = provider ? `${offering.name} — from ${provider}` : offering.name;
		sourcingCategory = 'buy';
		if (offering.priceCents != null) sourcingBudget = (offering.priceCents / 100).toFixed(2);
		document.getElementById('mk-need')?.focus();
	}

	/**
	 * Unwired. Opening a sourcing request is a governed decision that lands on the record —
	 * it needs the kernel's write path, not a fetch from a page. The brief is fully typeable
	 * so the shape of the ask can be reviewed; it just does not go anywhere yet.
	 */
	function sendSourcingRequest() {
		sourcingError = '';
		sourcingNote = 'Not wired yet — a sourcing request is a governed decision, and there is nothing to record it with.';
	}
</script>

<svelte:head><title>Market — {desk.company.name}</title></svelte:head>

<div class="bridge-page bridge-bleed bridge-market">
	<div class="page-head">
		<div>
			<h1>
				Market — vendors and what they sell<Hint
					text="The bench outside the building: who they are, what they offer, and how to ask for more."
					label="What the market is"
				/>
			</h1>
		</div>
	</div>

	<div class="pane-frame">
		<!-- Asking is the whole point of this page, so it leads instead of sitting
		     under three read-only lists at the bottom of a mostly empty screen. -->
		{#if canOperate}
			<form
				class="pane mk-pane mk-p-ask"
				onsubmit={(event) => {
					event.preventDefault();
					sendSourcingRequest();
				}}
			>
				<PaneHeader title="Source something" />
				<div class="mk-ask-grid">
					<input
						id="mk-need"
						class="mk-need"
						type="text"
						minlength="3"
						maxlength="500"
						required
						placeholder="What do you need? (e.g. credentialed Australian legal review)"
						aria-label="What do you need?"
						bind:value={sourcingNeed}
					/>
					<select aria-label="Category" bind:value={sourcingCategory}>
						{#each SOURCING_CATEGORIES as category (category)}
							<option value={category}>{category}</option>
						{/each}
					</select>
					<button class="btn primary small" type="submit" disabled={sourcingSending}>
						Open a sourcing request
					</button>
				</div>
				<div class="mk-brief-grid">
					<label class="mk-brief-field">
						<span class="f-label">Budget cap<span class="mk-opt">optional</span></span>
						<input
							type="number"
							min="0"
							step="0.01"
							inputmode="decimal"
							placeholder={desk.company.currency}
							bind:value={sourcingBudget}
						/>
					</label>
					<label class="mk-brief-field">
						<span class="f-label">Wanted by<span class="mk-opt">optional</span></span>
						<input type="date" bind:value={sourcingDeadline} />
					</label>
					<label class="mk-brief-field mk-brief-wide">
						<span class="f-label">
							Credentials or jurisdiction<span class="mk-opt">optional</span>
						</span>
						<input
							type="text"
							maxlength="500"
							placeholder="Admitted in NSW, holds PI insurance — comma separated"
							bind:value={sourcingRequirements}
						/>
					</label>
				</div>
				<span class="tape-note mono">a sourcing request lands on the record as its own decision</span>
				{#if sourcingNote}<p class="caption" style="margin: 6px 0 0">{sourcingNote}</p>{/if}
				{#if sourcingError}<p class="form-error">{sourcingError}</p>{/if}
			</form>
		{/if}

		<div class="pane-row mk-body">
			<!-- vendors are rows in one region, not narrow cards adrift in a grid:
			     one vendor used to leave a 300px tile beside 1500px of nothing -->
			<section class="pane mk-pane mk-p-bench">
				<PaneHeader title="The bench — known vendors" />
				{#each vendors as party (party.id)}
					<div class="mk-row">
						<div class="mk-row-top">
							<a class="mk-vendor-link" href={vendorHref(party.id)}>{party.name}</a>
							<span class="mk-status mono" data-tone={statusMark(party.status).tone}>
								<MatrixGlyph rows={GLYPHS[statusMark(party.status).glyph]} size={9} />
								{party.status}
							</span>
						</div>
						{#if textOf(party.serviceAreas)}<p class="caption mk-sub">
								{textOf(party.serviceAreas)}
							</p>{/if}
						{#if textOf(party.jurisdictions)}<p class="caption mk-sub">
								{textOf(party.jurisdictions)}
							</p>{/if}
						{#if party.availabilityNote}<p class="caption mk-sub">{party.availabilityNote}</p>{/if}
						{#if party.website || party.email}
							<p class="mk-contact mono">
								{[party.website, party.email].filter(Boolean).join(' · ')}
							</p>
						{/if}
						{#if workersOf(party.id).length > 0}
							<div class="mk-workers">
								{#each workersOf(party.id) as worker (worker.id)}
									<span class="mk-worker"
										>{worker.name}{worker.role ? ` · ${worker.role}` : ''}</span
									>
								{/each}
							</div>
						{/if}
					</div>
				{/each}
				{#if vendors.length === 0}
					<p class="mk-empty">No vendors on the bench yet.</p>
				{/if}
			</section>

			<section class="pane mk-pane mk-p-offer">
				<PaneHeader title="Offerings — what's for sale" />
				{#each offerings as offering (offering.id)}
					<div class="mk-row">
						<div class="mk-row-top">
							<b>{offering.name}</b>
							<span class="mk-price mono">
								{#if offering.priceCents != null}
									{money(
										offering.priceCents,
										offering.currency
									)}{#if offering.billing && offering.billing !== 'once'}
										/ {offering.billing}{/if}
								{:else}
									price unrecorded
								{/if}
							</span>
						</div>
						<p class="caption mk-sub">
							{offering.kind}{offering.description ? ` · ${offering.description}` : ''}
							{#if partyName(offering.providerPartyId)}· from {partyName(
									offering.providerPartyId
								)}{/if}
						</p>
						{#if canOperate}
							<!-- The one App Store idea that carries over without a false promise: this fills
							     the brief, it does not buy anything. A sourcing request is still a decision. -->
							<button
								class="btn small mk-request"
								type="button"
								onclick={() => requestOffering(offering)}
							>
								Request this
							</button>
						{/if}
					</div>
				{:else}
					<p class="mk-empty">
						Nothing for sale yet — the first offering lands here when a vendor lists one.
					</p>
				{/each}
			</section>

			<!-- the region stays put when empty: a section that vanishes makes the
			     page jump, and "nothing asked yet" is itself worth reading -->
			<section class="pane mk-pane mk-p-open">
				<PaneHeader
					title="Sourcing — briefs and offers"
					hint="A sourcing request is a decision, not a search: it lands on the tape, and the executive works the shortlist."
					hintLabel="What happens after you post a brief"
				/>
				{#each boards as board (board.requestId)}
					<div class="mk-board" class:open={board.open}>
						<div class="mk-row-top">
							<b>{board.need}</b>
							<span class="mk-status mono" data-tone={board.mark.tone}>
								<MatrixGlyph rows={GLYPHS[board.mark.glyph]} size={9} />
								{board.status.replaceAll('_', ' ')}
							</span>
						</div>
						<p class="caption mk-sub">
							{board.category}
							{#if board.budgetCapCents != null}
								· up to {money(board.budgetCapCents, desk.company.currency)}
							{/if}
							{#if board.deadline}· wanted by {day(board.deadline)}{/if}
						</p>
						{#if board.requirements.length > 0}
							<p class="caption mk-sub">Requires: {board.requirements.join(' · ')}</p>
						{/if}

						{#if board.offers.length > 0}
							<div class="mk-offers">
								{#each board.offers as offer (offer.partyId)}
									<a
										class="mk-offer"
										class:selected={offer.selected}
										href={vendorHref(offer.partyId)}
									>
										<span class="mk-offer-name">{offer.partyName}</span>
										{#if offer.note}<span class="caption mk-offer-note">{offer.note}</span>{/if}
										{#if offer.selected}<span class="mi-chip">selected</span>{/if}
									</a>
								{/each}
							</div>
						{:else if board.open}
							<p class="caption mk-sub">No offers yet — the executive is working the shortlist.</p>
						{/if}
					</div>
				{:else}
					<p class="mk-empty">Nothing asked for yet.</p>
				{/each}
			</section>
		</div>
	</div>
</div>

<style>
	/* These four selectors are absent from the migrated stylesheet — the source build had no
	   offerings and no sourcing requests, so the offer board and the optional-field markers
	   were never actually rendered and the gap never showed. Added here, in the page that
	   owns the markup. */
	.mk-opt {
		margin-left: 6px;
		font-family: 'IBM Plex Mono', monospace;
		font-size: 9.5px;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--text-tertiary);
	}
	.mk-offers {
		display: flex;
		flex-direction: column;
		gap: 6px;
		margin-top: 8px;
	}
	.mk-offer {
		display: flex;
		flex-direction: column;
		gap: 2px;
		align-items: flex-start;
		padding: 8px 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		background: var(--surface-alt);
		text-decoration: none;
	}
	.mk-offer:hover {
		border-color: var(--border-strong);
	}
	.mk-offer.selected {
		border-color: var(--accent-strong);
	}
	.mk-offer-name {
		font-size: 12.5px;
		font-weight: 600;
	}
	.mk-offer-note {
		font-size: 11.5px;
		line-height: 1.5;
	}
	/* the bench is widest — vendor rows carry the most text; the ask leads as a
	   masthead, same shape the mission surface uses */
	.mk-body {
		grid-template-columns: 5fr 4fr 3fr;
	}
	:global(.bridge-root) .mk-pane {
		padding: 20px 24px 22px;
	}
	:global(.bridge-root) .mk-p-ask {
		padding: 22px 24px 20px;
	}
	.mk-ask-grid {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		gap: 8px;
	}
	.mk-row {
		padding: 10px 0;
		border-bottom: 1px solid var(--border);
	}
	.mk-row:last-child {
		border-bottom: 0;
	}
	.mk-row-top {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 10px;
	}
	.mk-sub {
		margin: 2px 0 0;
	}
	.mk-status {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 10.5px;
		letter-spacing: 0.06em;
		color: var(--text-tertiary);
		flex: 0 0 auto;
	}
	.mk-contact {
		margin: 6px 0 0;
		font-size: 10.5px;
		color: var(--text-tertiary);
	}
	.mk-workers {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin-top: 8px;
		padding-top: 8px;
		border-top: 1px solid var(--border);
	}
	.mk-worker {
		font-size: 11px;
		color: var(--text-secondary);
	}
	.mk-price {
		flex: 0 0 auto;
		font-size: 11.5px;
		color: var(--text-secondary);
		font-variant-numeric: tabular-nums;
	}
	.mk-empty {
		font-size: 12.5px;
		line-height: 1.6;
		color: var(--text-tertiary);
		font-style: italic;
		margin: 4px 0;
	}
	.mk-need,
	.mk-p-ask select {
		padding: 9px 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-sm);
		background: var(--surface-alt);
		color: var(--ink);
		font: inherit;
		font-size: 13px;
	}
	.mk-need:focus,
	.mk-p-ask select:focus {
		outline: 2px solid color-mix(in srgb, var(--accent) 35%, transparent);
		border-color: var(--accent-strong);
	}
	@media (max-width: 1200px) {
		.mk-body {
			grid-template-columns: 1fr 1fr;
		}
		.mk-p-open {
			grid-column: 1 / -1;
		}
	}
	@media (max-width: 900px) {
		.mk-body {
			grid-template-columns: 1fr;
		}
		:global(.bridge-root) .mk-pane {
			padding: 18px 18px 20px;
		}
	}
	@media (max-width: 720px) {
		.mk-ask-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
