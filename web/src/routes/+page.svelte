<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import CompanyPortfolio from '$lib/components/CompanyPortfolio.svelte';
	import { getApplianceStatus, type ApplianceStatus } from '$lib/model/appliance';
	import {
		archiveCompany,
		restoreCompany,
		type CompanyCatalogEntry
	} from '$lib/model/cockpit';
	import { portfolioQuery, type PortfolioProjection } from '$lib/model/queries.svelte';
	import type { ProductNotice } from '$lib/product/contracts';

	const portfolio = portfolioQuery();
	const companies = $derived(portfolio.view?.companies ?? []);
	const projections = $derived(
		portfolio.view?.projections ?? ({} as Record<string, PortfolioProjection>)
	);
	const loaded = $derived(portfolio.status !== 'unknown');
	const error = $derived(portfolio.failure?.message ?? '');
	let redirected = $state(false);
	let appliance = $state<ApplianceStatus | null>(null);
	const notice = $derived.by((): ProductNotice | null => {
		if (appliance?.state === 'degraded') {
			return {
				title: 'Schedule wake needs repair.',
				detail: appliance.repair ?? 'Run the appliance repair command and check again.'
			};
		}
		if (appliance?.model_gateway === 'starting') {
			return {
				title: 'Model access is starting.',
				detail: 'Companies will wake after provider access is ready. The owner surface remains available.'
			};
		}
		return null;
	});

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

	async function changed() {
		await portfolio.refresh();
	}

	async function archive(company: CompanyCatalogEntry) {
		await archiveCompany(company.id);
	}

	async function restore(company: CompanyCatalogEntry) {
		await restoreCompany(company.id);
	}
</script>

<CompanyPortfolio
	{companies}
	{projections}
	{loaded}
	{error}
	{notice}
	onarchive={archive}
	onrestore={restore}
	onchanged={changed}
/>
