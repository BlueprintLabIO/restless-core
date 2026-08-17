<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import OwnerMenu from '$lib/components/OwnerMenu.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import SemanticMark from '$lib/primitives/SemanticMark.svelte';
	import { getAttention, signIn } from '$lib/model/attention';
	import { getCockpit, getCompanies, type CompanyCatalogEntry } from '$lib/model/cockpit';

	type PortfolioProjection = {
		attentionCount: number | null;
		nextProof: string | null;
		nextProofDetail: string;
		spendAccounted: number | null;
	};

	let companies = $state<CompanyCatalogEntry[]>([]);
	let projections = $state<Record<string, PortfolioProjection>>({});
	let ownerToken = $state('');
	let authRequired = $state(false);
	let signingIn = $state(false);
	let loaded = $state(false);
	let error = $state('');
	const activeCompanies = $derived(
		companies.filter((company) => company.lifecycle_status === 'active')
	);
	const archivedCompanies = $derived(
		companies.filter((company) => company.lifecycle_status === 'archived')
	);

	onMount(() => void loadCompanies());

	async function loadCompanies() {
		try {
			companies = await getCompanies();
			void loadProjections(companies.filter((company) => company.lifecycle_status === 'active'));
			authRequired = false;
			error = '';
			const next = safeNext(page.url.searchParams.get('next'));
			if (next) await goto(next, { replaceState: true });
		} catch (cause) {
			const typed = cause as Error & { status?: number };
			authRequired = typed.status === 401;
			error = authRequired ? '' : typed.message;
		} finally {
			loaded = true;
		}
	}

	async function loadProjections(catalog: CompanyCatalogEntry[]) {
		const entries = await Promise.all(
			catalog.map(async (company): Promise<[string, PortfolioProjection]> => {
				const [cockpitResult, attentionResult] = await Promise.allSettled([
					getCockpit(company.id),
					getAttention(company.id)
				]);
				const cockpit = cockpitResult.status === 'fulfilled' ? cockpitResult.value : null;
				const attention = attentionResult.status === 'fulfilled' ? attentionResult.value : null;
				const work = attention?.workGraph?.work ?? [];
				const next =
					work.find((item) => item.status === 'active') ??
					work.find((item) => item.status === 'blocked') ??
					work.find((item) => item.status === 'proposed') ??
					null;
				return [
					company.id,
					{
						attentionCount: attention ? attention.items.length : null,
						nextProof: next?.title ?? null,
						nextProofDetail: next
							? next.expected_artifact || next.outcome || workState(next.status)
							: attention
								? 'No open Work is recorded.'
								: 'Work projection unavailable.',
						spendAccounted: cockpit?.spend.accounted_usd ?? null
					}
				];
			})
		);
		projections = Object.fromEntries(entries);
	}

	async function submitSignIn(event: SubmitEvent) {
		event.preventDefault();
		if (!ownerToken || signingIn) return;
		signingIn = true;
		error = '';
		try {
			await signIn(ownerToken);
			ownerToken = '';
			await loadCompanies();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Sign-in failed.';
		} finally {
			signingIn = false;
		}
	}

	function safeNext(value: string | null): string {
		return value?.startsWith('/') && !value.startsWith('//') ? value : '';
	}

	function workState(value: string): string {
		return value.replaceAll('_', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase());
	}

	function money(value: number): string {
		return new Intl.NumberFormat(undefined, {
			style: 'currency',
			currency: 'USD',
			maximumFractionDigits: value < 100 ? 2 : 0
		}).format(value);
	}

	function attentionLabel(company: CompanyCatalogEntry): string {
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
		{#if !authRequired && loaded}
			<span class="portfolio-location">Companies</span>
		{/if}
		{#if !authRequired && loaded}
			<div class="tb-right">
				<OwnerMenu {companies} onchanged={loadCompanies} />
			</div>
		{/if}
	</header>

	{#if authRequired}
		<main class="portfolio-auth">
			<form class="portfolio-auth-card" onsubmit={submitSignIn}>
				<span class="portfolio-auth-mark"><MatrixGlyph rows={GLYPHS.r} size={16} glow /></span>
				<h1>Sign in to your companies</h1>
				<p>
					Use the owner credential created when this Restless installation was set up. One sign-in
					opens the portfolio and every company you own.
				</p>
				<input
					class="comp-input"
					type="password"
					bind:value={ownerToken}
					autocomplete="current-password"
					placeholder="Owner credential"
					aria-label="Owner credential"
					required
				/>
				<button class="btn primary" type="submit" disabled={signingIn}>
					{signingIn ? 'Checking…' : 'Open owner surface'}
				</button>
				{#if error}<p class="owner-error">{error}</p>{/if}
			</form>
		</main>
	{:else if loaded}
		<main class="portfolio-main">
			<header class="portfolio-head">
				<h1>Companies</h1>
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
								<a
									class="portfolio-company-row runtime-{company.runtime_status}"
									href={`/${company.id}`}
									aria-label={attentionLabel(company)}
								>
									<span class="portfolio-company-cell">
										<SemanticMark
											meaning={company.runtime_status === 'running'
												? 'presence'
												: company.runtime_status === 'unavailable'
													? 'unavailable'
													: 'waiting'}
											label={`${company.name} runtime: ${company.runtime_status}`}
										/>
										<span class="portfolio-company-copy">
											<strong>{company.name}</strong>
											<small>{company.runtime_status}</small>
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
											{spent === null || spent === undefined
												? `— of ${money(company.spend_ceiling_usd)}`
												: `${money(spent)} of ${money(company.spend_ceiling_usd)}`}
										</strong>
										<small>
											{spent === null || spent === undefined
												? 'Spend unavailable'
												: `${Math.round((spent / Math.max(company.spend_ceiling_usd, 0.01)) * 100)}% committed`}
										</small>
										{#if spent !== null && spent !== undefined}
											<i class="portfolio-spend-track" aria-hidden="true"
												><b
													style={`width: ${Math.min(100, (spent / Math.max(company.spend_ceiling_usd, 0.01)) * 100)}%`}
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
