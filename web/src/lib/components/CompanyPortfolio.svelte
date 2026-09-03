<script lang="ts">
	import type { Snippet } from 'svelte';
	import { PRODUCT_NAME } from '../brand/brand';
	import OwnerMenu from './OwnerMenu.svelte';
	import MatrixGlyph, { GLYPHS } from '../primitives/MatrixGlyph.svelte';
	import SemanticMark from '../primitives/SemanticMark.svelte';
	import type {
		CompanyCatalogEntry,
		PortfolioProjection,
		ProductNotice
	} from '../product/contracts';

	let {
		companies,
		projections,
		loaded,
		error = '',
		notice = null,
		companyHref = (company: CompanyCatalogEntry) => `/${company.id}`,
		manageHref = null,
		onarchive = null,
		onrestore = null,
		onopen = null,
		onchanged = null,
		ownerLabel = 'Owner',
		actions = null,
		ownerActions = null
	}: {
		companies: CompanyCatalogEntry[];
		projections: Record<string, PortfolioProjection>;
		loaded: boolean;
		error?: string;
		notice?: ProductNotice | null;
		companyHref?: (company: CompanyCatalogEntry) => string;
		manageHref?: ((company: CompanyCatalogEntry) => string) | null;
		onarchive?: ((company: CompanyCatalogEntry) => Promise<void>) | null;
		onrestore?: ((company: CompanyCatalogEntry) => Promise<void>) | null;
		onopen?: ((company: CompanyCatalogEntry) => void | Promise<void>) | null;
		onchanged?: (() => void | Promise<void>) | null;
		ownerLabel?: string;
		actions?: Snippet | null;
		ownerActions?: Snippet | null;
	} = $props();
	const activeCompanies = $derived(
		companies.filter((company) => company.lifecycle_status === 'active')
	);
	const archivedCompanies = $derived(
		companies.filter((company) => company.lifecycle_status === 'archived')
	);

	function money(value: number): string {
		return new Intl.NumberFormat(undefined, {
			style: 'currency',
			currency: 'USD',
			maximumFractionDigits: value < 100 ? 2 : 0
		}).format(value);
	}

	function attentionLabel(company: CompanyCatalogEntry): string {
		// A company that cannot start is the one fact worth stating before
		// attention counts: nothing will happen in it until it is resolved.
		if (company.unstartable_reason) {
			return `Open ${company.name}. It cannot start: ${company.unstartable_reason}`;
		}
		const count = projections[company.id]?.attentionCount;
		if (count === null || count === undefined) return `Open ${company.name}`;
		if (count === 0) return `Open ${company.name}. No owner attention is waiting.`;
		return `Open ${company.name}. ${count} item${count === 1 ? '' : 's'} need owner attention.`;
	}
</script>

<svelte:head><title>Companies — {PRODUCT_NAME}</title></svelte:head>

<div class="bridge-root portfolio-root">
	<header class="bridge-topbar" aria-label="Portfolio navigation">
		<a class="tb-brand portfolio-brand" href="/" aria-label={`${PRODUCT_NAME} companies`}>
			<span class="tb-mark"><MatrixGlyph rows={GLYPHS.r} size={13} glow /></span>
			<span class="tb-name">{PRODUCT_NAME}</span>
		</a>
		{#if loaded}
			<span class="portfolio-location">Companies</span>
		{/if}
		{#if loaded}
			<div class="tb-right">
				<OwnerMenu {companies} {manageHref} {onarchive} {onrestore} {onchanged} label={ownerLabel}>
					{#snippet footer()}{#if ownerActions}{@render ownerActions()}{/if}{/snippet}
				</OwnerMenu>
			</div>
		{/if}
	</header>

	{#if loaded}
		<main class="portfolio-main">
			{#if notice}
				<div class="appliance-notice" role="status">
					<span>{notice.title}</span>
					<p>{notice.detail}</p>
				</div>
			{/if}
			<header class="portfolio-head">
				<h1>Companies</h1>
				{#if actions}<div class="portfolio-actions">{@render actions()}</div>{/if}
			</header>

			{#if error}<div class="portfolio-error">{error}</div>{/if}
			<section class="portfolio-table" aria-label="Company portfolio">
				<header class="portfolio-table-head">
					<h2>Portfolio</h2>
					<div class="portfolio-totals" aria-label="Portfolio financial totals">
						<span><small>30-day revenue</small><strong>—</strong></span>
						<span><small>30-day profit</small><strong>—</strong></span>
					</div>
				</header>
				{#if activeCompanies.length}
					<div class="portfolio-table-scroll">
						<div class="portfolio-grid">
							<div class="portfolio-grid-head" aria-hidden="true">
								<span>Company</span>
								<span>Revenue · 30 days</span>
								<span>Profit · 30 days</span>
								<span>Next proof of value</span>
								<span>Spend / envelope</span>
							</div>
							{#each activeCompanies as company (company.id)}
								{@const projection = projections[company.id]}
								{@const spent = projection?.spendAccounted}
								{@const ceiling = company.spend_ceiling_usd}
								<a
									class="portfolio-company-row runtime-{company.runtime_status}"
									href={companyHref(company)}
									data-sveltekit-preload-data={onopen ? 'off' : undefined}
									onclick={(event) => {
										if (!onopen) return;
										event.preventDefault();
										void onopen(company);
									}}
									aria-label={attentionLabel(company)}
								>
									<span class="portfolio-company-cell">
										<SemanticMark
											meaning={company.unstartable_reason
												? 'unavailable'
												: company.runtime_status === 'running'
													? 'presence'
													: company.runtime_status === 'unavailable'
														? 'unavailable'
														: 'waiting'}
											label={company.unstartable_reason
												? `${company.name} cannot start: ${company.unstartable_reason}`
												: `${company.name} runtime: ${company.runtime_status}`}
										/>
										<span class="portfolio-company-copy">
											<strong>{company.name}</strong>
											{#if company.unstartable_reason}
												<!-- The exact reason is the hover explanation; the row stays
												     one short phrase rather than growing a second line. -->
												<small
													class="portfolio-company-unstartable"
													title={company.unstartable_reason}>cannot start</small
												>
											{:else}
												<small>{company.runtime_status}</small>
											{/if}
										</span>
										<span class="portfolio-company-open" aria-hidden="true">
											→
											{#if (projection?.attentionCount ?? 0) > 0}
												<i>{projection.attentionCount}</i>
											{/if}
										</span>
									</span>
									<span class="portfolio-metric financial-unavailable">
										<strong>—</strong><small>No verified figure</small>
									</span>
									<span class="portfolio-metric financial-unavailable">
										<strong>—</strong><small>No verified figure</small>
									</span>
									<span class="portfolio-metric portfolio-proof">
										<strong>{projection?.nextProof ?? 'Checking Work…'}</strong>
										<small>{projection?.nextProofDetail ?? 'Loading live projection.'}</small>
									</span>
									<span class="portfolio-metric portfolio-spend">
										<strong>
											{ceiling === null
												? '—'
												: spent === null || spent === undefined
													? `— of ${money(ceiling)}`
													: `${money(spent)} of ${money(ceiling)}`}
										</strong>
										<small>
											{ceiling === null
												? 'Allowance unavailable'
												: spent === null || spent === undefined
													? 'Spend unavailable'
													: `${Math.round((spent / Math.max(ceiling, 0.01)) * 100)}% committed`}
										</small>
										{#if ceiling !== null && spent !== null && spent !== undefined}
											<i class="portfolio-spend-track" aria-hidden="true"
												><b
													style={`width: ${Math.min(100, (spent / Math.max(ceiling, 0.01)) * 100)}%`}
												></b></i
											>
										{/if}
									</span>
								</a>
							{/each}
						</div>
					</div>
					<footer class="portfolio-table-foot">
						<span>Revenue and profit remain blank until a verified source reports them.</span>
						<span
							>{activeCompanies.length} operating compan{activeCompanies.length === 1
								? 'y'
								: 'ies'}</span
						>
					</footer>
				{:else}
					<div class="portfolio-empty">
						<MatrixGlyph rows={GLYPHS.ring} size={14} />
						<h2>No active companies</h2>
						<p>
							{archivedCompanies.length
								? 'Restore an archived company from Owner settings.'
								: 'This owner installation has no configured companies.'}
						</p>
					</div>
				{/if}
			</section>
		</main>
	{:else}
		<main class="portfolio-loading">Loading companies…</main>
	{/if}
</div>

<style>
	.appliance-notice {
		display: grid;
		grid-template-columns: max-content 1fr;
		gap: 8px 18px;
		align-items: baseline;
		margin: 0 0 32px;
		padding: 12px 0;
		border-block: 1px solid color-mix(in srgb, var(--ink, #171b24) 18%, transparent);
		color: var(--ink, #171b24);
	}

	.appliance-notice span {
		font-weight: 650;
	}

	.appliance-notice p {
		margin: 0;
		color: var(--ink-muted, #596170);
	}

	@media (max-width: 640px) {
		.appliance-notice {
			grid-template-columns: 1fr;
		}
	}
</style>
