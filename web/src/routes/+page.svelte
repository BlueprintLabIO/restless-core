<script lang="ts">
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { archiveCompany, restoreCompany } from '$lib/model/cockpit';
	import { failureSentence } from '$lib/model/failure';
	import CompanyPortfolio from '$lib/ui/views/CompanyPortfolio.svelte';
	import type { CompanyPortfolioEntry } from '$lib/ui/portfolio';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { tooltips } from '$lib/actions/tooltips';
	import { selectMenu } from '$lib/actions/select-menu';
	import { goto } from '$app/navigation';
	import { Settings2 } from '@lucide/svelte';
	import { page } from '$app/state';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import CreateCompany from '$lib/components/CreateCompany.svelte';
	import { getApplianceStatus, type ApplianceStatus } from '$lib/model/appliance';
	import {
		companiesQuery,
		portfolioQuery,
		type PortfolioProjection
	} from '$lib/model/queries.svelte';
	import { startFixHref, startLinkLabel } from '$lib/model/company-start';
	import { plainText } from '$lib/ui/text';

	const companyCatalog = companiesQuery();
	const portfolio = portfolioQuery();
	const companies = $derived(companyCatalog.view);
	const projections = $derived(
		portfolio.view?.projections ?? ({} as Record<string, PortfolioProjection>)
	);
	const loaded = $derived(companyCatalog.status !== 'unknown');
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

	let notice = $state(''),
		actionError = $state(''),
		archivedId = $state(''),
		busy = $state('');
	async function archive(id: string) {
		if (busy) return;
		busy = id;
		actionError = '';
		try {
			await archiveCompany(id);
			await companyCatalog.refresh();
			archivedId = id;
			notice = 'Company archived';
		} catch (cause) {
			actionError = failureSentence(cause, 'Company could not be archived.');
		} finally {
			busy = '';
		}
	}
	async function undoArchive() {
		if (busy || !archivedId) return;
		busy = archivedId;
		try {
			await restoreCompany(archivedId);
			await companyCatalog.refresh();
			notice = 'Company restored';
			archivedId = '';
		} catch (cause) {
			actionError = failureSentence(cause, 'Company could not be restored.');
		} finally {
			busy = '';
		}
	}
	const rows = $derived(
		activeCompanies.map((company): CompanyPortfolioEntry => {
			const projection = projections[company.id];
			const issue = company.unstartable_reason ? startLinkLabel(company.unstartable_reason) : '';
			return {
				id: company.id,
				name: company.name,
				status: issue ? 'Can’t start' : company.runtime_status,
				tone:
					issue || company.runtime_status === 'unavailable'
						? 'unavailable'
						: company.runtime_status === 'running'
							? 'presence'
							: 'waiting',
				focus: plainText(company.mission, { dropTitle: true }) || 'Focus not set',
				next: projection?.nextProof || 'Checking work…',
				nextHint: projection?.nextProofDetail,
				attentionCount: projection?.attentionCount,
				needsYou: projection?.attentionCount == null && !issue ? 'Checking…' : undefined,
				issue,
				entry: { href: issue ? startFixHref(company.id) : `/${company.id}` }
			};
		})
	);
</script>

<svelte:head><title>Companies — {PRODUCT_NAME}</title></svelte:head>

{#snippet failures()}
	{#if companyCatalog.failure}
		<FailureNotice
			error={companyCatalog.failure}
			subject="your companies"
			stale={loaded}
			variant={loaded ? 'inline' : 'page'}
			onretry={companyCatalog.refresh}
		/>
	{/if}
{/snippet}

<div use:tooltips use:selectMenu>
	<CompanyPortfolio
		companies={rows}
		{loaded}
		brandName={PRODUCT_NAME}
		feedback={companyCatalog.failure ? failures : null}
	>
		{#snippet headerActions()}
			{#if loaded}<CreateCompany />{/if}
			<a
				class="btn settings-link"
				href="/account/settings"
				aria-label="Account settings"
				title="Account settings"
			>
				<Settings2 size={17} strokeWidth={1.8} aria-hidden="true" /><span>Settings</span>
			</a>
		{/snippet}
		{#snippet rowActions(company)}<ActionMenu label={`${company.name} options`}
				><a href={`/${company.id}/company`}>Rename</a><button
					disabled={!!busy}
					onclick={() => archive(company.id)}>Archive company</button
				></ActionMenu
			>{/snippet}
		{#snippet before()}
			{#if actionError}<p role="alert">{actionError}</p>{/if}
			{#if notice}<p role="status">
					{notice}{#if archivedId}<button class="btn small" disabled={!!busy} onclick={undoArchive}
							>Undo</button
						>{/if}
				</p>{/if}
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
		{/snippet}

		{#snippet empty()}
			<h2>No companies yet</h2>
			<p>
				{archivedCompanies.length
					? 'Use + to start a company.'
					: 'Use + to start your first company.'}
			</p>
		{/snippet}
	</CompanyPortfolio>
</div>

<style>
	.settings-link {
		gap: var(--space-2);
	}

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
		font-weight: 600;
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
