<script lang="ts">
	/* One vendor, expanded (UIR-012).
	 *
	 * The "app detail page" of a two-sided marketplace: who they are, what they sell, who works
	 * there, and what the company has actually bought from them. This is where
	 * `vendor-reputation.ts` finally renders — it was written, specified, tested, and referenced
	 * by nothing.
	 *
	 * There is no "hire" button, and that is not an omission. Nothing crosses from this company
	 * to a human vendor automatically. The honest action is to post a brief naming them. */

	import { page } from '$app/state';
	import PaneHeader from '$lib/primitives/PaneHeader.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import { composeVendorDetail, hasRole, statusMark } from '$lib/model/market-view';
	import { composeVendorReputations } from '$lib/model/vendor-reputation';
	import { cosmon, market, vendorEngagements, vendorCredentials } from '$lib/fixtures/cosmon';

	const desk = cosmon;
	const companyId = $derived(page.params.companyId ?? desk.company.id);
	const partyId = $derived(page.params.partyId ?? '');

	const detail = $derived(
		composeVendorDetail({
			parties: market.parties,
			offerings: market.offerings,
			vendorWorkers: market.vendorWorkers,
			partyId
		})
	);

	const marketHref = $derived(`/${companyId}/market`);

	/* Every brief this vendor was shortlisted on — the company's actual history with them, read
	 * off the sourcing record rather than asserted. */
	const engagements = $derived(
		market.sourcingRequests
			.filter((request) => request.candidates.some((candidate) => candidate.partyId === partyId))
			.map((request) => ({
				id: request.id,
				need: request.need,
				status: request.status,
				mark: statusMark(request.status),
				won: request.selectedPartyId === partyId
			}))
	);

	/**
	 * Reputation is only ever what THIS company has observed. There is no imported rating,
	 * no star average, and no opaque score — a transparent rule over visible counts, plus a
	 * plain statement of what it rests on. An unproven vendor says so.
	 */
	const reputation = $derived.by(() => {
		const profile = composeVendorReputations({
			vendorPartyIds: market.parties.filter((party) => hasRole(party, 'vendor')).map((p) => p.id),
			engagements: vendorEngagements,
			credentials: vendorCredentials
		}).find((entry) => entry.vendorPartyId === partyId);
		if (!profile) return { note: 'Nothing observed yet.', signals: [] as Array<{ label: string; value: string }> };
		return {
			note: profile.basis,
			signals: [
				{ label: 'Standing', value: profile.standing },
				{ label: 'Engagements', value: `${profile.engagements.completed} completed of ${profile.engagements.total}` },
				{ label: 'In progress', value: String(profile.engagements.inProgress) },
				{ label: 'Lost or cancelled', value: String(profile.engagements.lost + profile.engagements.cancelled) },
				{ label: 'Credentials', value: `${profile.credentials.valid} valid · ${profile.credentials.expiringSoon} expiring · ${profile.credentials.expired} expired` }
			]
		};
	});

	function money(cents: number | null, currency: string): string {
		if (cents == null) return 'price unrecorded';
		try {
			return new Intl.NumberFormat(undefined, { style: 'currency', currency }).format(cents / 100);
		} catch {
			return `${(cents / 100).toFixed(2)} ${currency}`;
		}
	}
</script>

<svelte:head><title>{detail?.party.name ?? 'Vendor'} — {desk.company.name}</title></svelte:head>

<div class="bridge-page bridge-bleed bridge-market">
	<div class="page-head">
		<div style="display: flex; align-items: center; gap: 10px">
			<a class="btn small" href={marketHref}>‹ Market</a>
			<h1>{detail?.party.name ?? 'Vendor'}</h1>
		</div>
	</div>

	<div class="pane-frame">
		{#if !detail}
			<section class="pane mk-pane">
				<p class="mk-empty">
					No vendor by that id is on this company's bench. <a href={marketHref}
						>Back to the market</a
					>.
				</p>
			</section>
		{:else}
			<div class="pane-row mk-detail-body">
				<section class="pane mk-pane">
					<PaneHeader
						title="Who they are"
						hint="Recorded from what the vendor and the company have stated. Helm does not verify these against any registry."
						hintLabel="How vendor identity is recorded"
					/>
					<div class="kv">
						<span>Status</span>
						<b class="mk-status mono" data-tone={detail.mark.tone}>
							<MatrixGlyph rows={GLYPHS[detail.mark.glyph]} size={9} />
							{detail.party.status.replaceAll('_', ' ')}
						</b>
					</div>
					<div class="kv">
						<span>Service areas</span><b>{detail.serviceAreas.join(' · ') || '—'}</b>
					</div>
					<div class="kv">
						<span>Jurisdictions</span><b>{detail.jurisdictions.join(' · ') || '—'}</b>
					</div>
					{#if detail.party.availabilityNote}
						<div class="kv">
							<span>Availability</span><b>{detail.party.availabilityNote}</b>
						</div>
					{/if}
					{#if detail.party.website || detail.party.email}
						<p class="mk-contact mono">
							{[detail.party.website, detail.party.email].filter(Boolean).join(' · ')}
						</p>
					{/if}
				</section>

				<div class="pane-rail">
					<section class="pane mk-pane">
						<PaneHeader title="What they sell" />
						{#each detail.offerings as offering (offering.id)}
							<div class="mk-row">
								<div class="mk-row-top">
									<b>{offering.name}</b>
									<span class="mk-price mono">
										{money(
											offering.priceCents,
											offering.currency
										)}{#if offering.billing && offering.billing !== 'once'}
											/ {offering.billing}{/if}
									</span>
								</div>
								<p class="caption mk-sub">
									{offering.kind}{offering.description ? ` · ${offering.description}` : ''}
								</p>
							</div>
						{:else}
							<p class="mk-empty">
								Nothing listed. The company has recorded no offerings for them.
							</p>
						{/each}
					</section>

					<section class="pane mk-pane">
						<PaneHeader title="Who works there" />
						{#each detail.workers as worker (worker.id)}
							<div class="kv">
								<span>{worker.name}</span><b>{worker.role || '—'}</b>
							</div>
						{:else}
							<p class="mk-empty">No named people recorded.</p>
						{/each}
					</section>
				</div>
			</div>

			<section class="pane mk-pane">
				<PaneHeader
					title="Reputation"
					hint={reputation.note}
					hintLabel="How reputation is computed"
				/>
				{#each reputation.signals as signal (signal.label)}
					<div class="kv"><span>{signal.label}</span><b>{signal.value}</b></div>
				{:else}
					<p class="mk-empty">
						Nothing recorded yet. Reputation here is only ever what this company has observed —
						there is no imported rating.
					</p>
				{/each}
			</section>

			<section class="pane mk-pane">
				<PaneHeader title="What we've asked them for" />
				{#each engagements as engagement (engagement.id)}
					<div class="mk-row">
						<div class="mk-row-top">
							<b>{engagement.need}</b>
							<span class="mk-status mono" data-tone={engagement.mark.tone}>
								<MatrixGlyph rows={GLYPHS[engagement.mark.glyph]} size={9} />
								{engagement.won ? 'selected' : engagement.status.replaceAll('_', ' ')}
							</span>
						</div>
					</div>
				{:else}
					<p class="mk-empty">Never shortlisted on a brief yet.</p>
				{/each}
			</section>

			<section class="pane mk-pane">
				<PaneHeader title="Engaging them" />
				<p class="caption">
					There is no button here that hires anyone, because nothing can yet cross to a human
					vendor: Helm Port defines a <span class="mono">service</span> profile class, but no
					service profile is implemented (<span class="mono">server/port/profiles/</span> holds web-read,
					email-send, browser-session, and computer-session). The honest action is to post a brief naming
					them, which the executive works and which lands on the tape.
				</p>
				<a class="btn small primary" href={marketHref}>Post a brief</a>
			</section>
		{/if}
	</div>
</div>
