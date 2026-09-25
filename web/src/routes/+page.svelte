<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import CreateCompany from '$lib/components/CreateCompany.svelte';
	import { getApplianceStatus, type ApplianceStatus } from '$lib/model/appliance';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import SemanticMark from '$lib/primitives/SemanticMark.svelte';
	import {
		companiesQuery,
		portfolioQuery,
		type PortfolioProjection
	} from '$lib/model/queries.svelte';
	import type { CompanyCatalogEntry } from '$lib/model/cockpit';

	const companyCatalog = companiesQuery();
	const portfolio = portfolioQuery();
	const companies = $derived(companyCatalog.view);
	const projections = $derived(
		portfolio.view?.projections ?? ({} as Record<string, PortfolioProjection>)
	);
	const loaded = $derived(companyCatalog.status !== 'unknown');
	const error = $derived(companyCatalog.failure?.message ?? '');
	let redirected = $state(false);
	let appliance = $state<ApplianceStatus | null>(null);
	const activeCompanies = $derived(
		companies.filter((company) => company.lifecycle_status === 'active')
	);
	const archivedCompanies = $derived(
		companies.filter((company) => company.lifecycle_status === 'archived')
	);

	$effect(() => {
		if (redirected || !loaded) return;
		redirected = true;
		const next = safeNext(page.url.searchParams.get('next'));
		if (next) void goto(next, { replaceState: true });
	});

	$effect(() => {
		const controller = new AbortController();
		void getApplianceStatus(controller.signal)
			.then((value) => (appliance = value))
			.catch(() => {
				// The portfolio query already owns the global unavailable state.
			});
		return () => controller.abort();
	});

	function safeNext(value: string | null): string {
		return value?.startsWith('/') && !value.startsWith('//') ? value : '';
	}

	function startGuidance(reason: string): string {
		if (reason.startsWith('Choose an intelligence provider')) return reason;
		if (reason.startsWith('no usable host credential for model native-codex-oauth/')) {
			return 'Check Codex sign-in and select Codex in Company → Intelligence provider.';
		}
		if (
			reason.startsWith('no usable host credential') ||
			reason.startsWith('Claude Agent requires')
		) {
			return 'Connect the selected intelligence provider in Company → Intelligence provider.';
		}
		return 'Check the intelligence provider setup in Company → Intelligence provider.';
	}

	function attentionLabel(company: CompanyCatalogEntry, projection?: PortfolioProjection): string {
		// A company that cannot start is the one fact worth stating before
		// attention counts: nothing will happen in it until it is resolved.
		if (company.unstartable_reason) {
			return `Open ${company.name}. It cannot start: ${startGuidance(company.unstartable_reason)}`;
		}
		const next = projection?.nextProof ? ` Next item of value: ${projection.nextProof}.` : '';
		const count = projection?.attentionCount;
		if (count === null || count === undefined) return `Open ${company.name}.${next}`;
		if (count === 0) return `Open ${company.name}.${next} No owner attention is waiting.`;
		return `Open ${company.name}.${next} ${count} item${count === 1 ? '' : 's'} need owner attention.`;
	}
</script>

<svelte:head><title>Projects — {PRODUCT_NAME}</title></svelte:head>

<div class="bridge-root portfolio-root">
	<header class="bridge-topbar" aria-label="Portfolio navigation">
		<a class="tb-brand portfolio-brand" href="/" aria-label={`${PRODUCT_NAME} projects`}>
			<span class="tb-mark"><MatrixGlyph rows={GLYPHS.r} size={13} glow /></span>
			<span class="tb-name">{PRODUCT_NAME}</span>
		</a>
		{#if loaded}
			<div class="tb-right">
				<a class="connections-link" href="/connections">Connections</a>
				<CreateCompany />
			</div>
		{/if}
	</header>

	{#if error && !loaded}
		<main class="portfolio-main">
			<div class="portfolio-error" role="alert">{error}</div>
			<button class="btn small" type="button" onclick={() => void companyCatalog.refresh()}
				>Try again</button
			>
		</main>
	{:else if loaded}
		<main class="portfolio-main">
			{#if appliance?.state === 'recovering'}
				<div class="appliance-notice" role="status">
					<span>Restoring runtime safety.</span>
					<p>{appliance.repair}</p>
				</div>
			{:else if appliance?.state === 'draining'}
				<div class="appliance-notice" role="status">
					<span>Work admission is paused.</span>
					<p>{appliance.repair}</p>
				</div>
			{:else if appliance?.state === 'degraded'}
				<div class="appliance-notice" role="status">
					<span>Schedule wake needs repair.</span>
					<p>{appliance.repair}</p>
				</div>
			{/if}
			<header class="portfolio-head">
				<h1>Projects</h1>
			</header>

			{#if error}<div class="portfolio-error">{error}</div>{/if}
			<section class="portfolio-table" aria-label="Projects">
				{#if activeCompanies.length}
					<div class="portfolio-table-scroll">
						<div class="portfolio-grid">
							<div class="portfolio-grid-head" aria-hidden="true">
								<span></span>
								<span>Current focus</span>
								<span>Next item of value</span>
								<span>Needs you</span>
							</div>
							{#each activeCompanies as company (company.id)}
								{@const projection = projections[company.id]}
								{@const startIssue = company.unstartable_reason ? startGuidance(company.unstartable_reason) : ''}
								<a
									class="portfolio-company-row runtime-{company.runtime_status}"
									href={`/${company.id}`}
									aria-label={attentionLabel(company, projection)}
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
												? `${company.name} cannot start: ${startIssue}`
												: `${company.name} runtime: ${company.runtime_status}`}
										/>
										<span class="portfolio-company-copy">
											<strong>{company.name}</strong>
											{#if company.unstartable_reason}
												<small
													class="portfolio-company-unstartable"
														title={startIssue}>cannot start</small
												>
											{:else}
												<small>{company.runtime_status}</small>
											{/if}
										</span>
									</span>
									<span
										class="portfolio-metric portfolio-focus"
										title={company.mission || undefined}
									>
										<small class="portfolio-mobile-label">Current focus</small>
										<strong>{company.mission || 'Focus not set'}</strong>
									</span>
									<span class="portfolio-metric portfolio-proof">
										<small class="portfolio-mobile-label">Next item of value</small>
										<strong title={startIssue || undefined}
											>{startIssue || projection?.nextProof || 'Checking work…'}</strong
										>
									</span>
									<span class="portfolio-metric portfolio-attention">
										<small class="portfolio-mobile-label">Needs you</small>
										<strong
											>{company.unstartable_reason
												? 'Start blocked'
												: projection?.attentionCount == null
													? 'Checking…'
												: projection.attentionCount === 0
													? 'Nothing now'
													: `${projection.attentionCount} item${projection.attentionCount === 1 ? '' : 's'}`}</strong
										>
									</span>
								</a>
							{/each}
						</div>
					</div>
				{:else}
					<div class="portfolio-empty">
						<MatrixGlyph rows={GLYPHS.ring} size={14} />
						<h2>No projects yet</h2>
						<p>
							{archivedCompanies.length
								? 'Use + to create a project.'
								: 'Use + to create your first project.'}
						</p>
					</div>
				{/if}
			</section>
		</main>
	{:else}
		<main class="portfolio-loading">Loading projects…</main>
	{/if}
</div>

<style>
	.connections-link {
		padding: var(--space-2) var(--space-3);
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		font-size: var(--t-label);
		text-decoration: none;
	}
	.connections-link:hover { border-color: var(--intent-conversation); color: var(--intent-conversation); }

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
