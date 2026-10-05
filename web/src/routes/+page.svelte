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
	import { page } from '$app/state';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import CreateCompany from '$lib/components/CreateCompany.svelte';
	import AccountFrame from '$lib/components/AccountFrame.svelte';
	import { getApplianceStatus, type ApplianceStatus } from '$lib/model/appliance';
	import {
		companiesQuery,
		portfolioQuery,
		type PortfolioProjection
	} from '$lib/model/queries.svelte';
	import {
		accountGrantFix,
		getAccountConnections,
		startFixHref,
		startLinkLabel,
		type AccountConnectionSummary
	} from '$lib/model/company-start';
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

	let accountConnections = $state<AccountConnectionSummary[]>([]);
	$effect(() => {
		const controller = new AbortController();
		void getApplianceStatus(controller.signal)
			.then((value) => (appliance = value))
			.catch(() => {
				// The portfolio query already owns the global unavailable state.
			});
		// Only refines a blocked company's fix; without it the Company page link stays.
		void getAccountConnections(controller.signal)
			.then((rows) => (accountConnections = rows))
			.catch(() => {});
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
	let showArchived = $state(false);
	async function restore(id: string) {
		if (busy) return;
		busy = id;
		actionError = '';
		try {
			await restoreCompany(id);
			await companyCatalog.refresh();
			notice = 'Company restored';
			archivedId = '';
		} catch (cause) {
			actionError = failureSentence(cause, 'Company could not be restored.');
		} finally {
			busy = '';
		}
	}
	const archivedRows = $derived(
		showArchived
			? archivedCompanies.map((company): CompanyPortfolioEntry => ({
					id: company.id,
					name: company.name,
					status: 'Archived',
					tone: 'waiting',
					focus: plainText(company.mission, { dropTitle: true }) || 'Focus not set',
					next: 'Restore it to open it again',
					entry: null,
					dormant: true
				}))
			: []
	);
	const rows = $derived([
		...activeCompanies.map((company): CompanyPortfolioEntry => {
			const projection = projections[company.id];
			const reason = company.unstartable_reason ?? '';
			const grant = reason ? accountGrantFix(reason, company.id, accountConnections) : null;
			const issue = grant?.label ?? (reason ? startLinkLabel(reason) : '');
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
				entry: {
					href: grant?.href ?? (issue ? startFixHref(company.id) : `/${company.id}`)
				}
			};
		}),
		...archivedRows
	]);
	const archivedSet = $derived(new Set(archivedCompanies.map((company) => company.id)));
</script>

<svelte:head><title>Companies — {PRODUCT_NAME}</title></svelte:head>

{#snippet feedback()}
	{#if companyCatalog.failure}
		<FailureNotice
			error={companyCatalog.failure}
			subject="your companies"
			stale={loaded}
			variant={loaded ? 'inline' : 'page'}
			onretry={companyCatalog.refresh}
		/>
	{/if}
	{#if actionError}<p class="portfolio-toast failure" role="alert">{actionError}</p>{/if}
	{#if notice}<p class="portfolio-toast" role="status">
			<span>{notice}</span>{#if archivedId}<button
					class="btn small"
					disabled={!!busy}
					onclick={undoArchive}>Undo</button
				>{/if}
		</p>{/if}
{/snippet}

<div use:tooltips use:selectMenu>
	<AccountFrame {appliance}>
		<CompanyPortfolio
			companies={rows}
			{loaded}
			feedback={companyCatalog.failure || actionError || notice ? feedback : null}
		>
			{#snippet actions()}
				{#if loaded}<CreateCompany />{/if}
			{/snippet}
			{#snippet rowActions(company)}<ActionMenu label={`${company.name} options`}
					>{#if archivedSet.has(company.id)}<button
							disabled={!!busy}
							onclick={() => restore(company.id)}>Restore company</button
						>{:else}<a href={`/${company.id}`}>Open</a><a href={`/${company.id}/company`}>Rename</a
						><a href={`/${company.id}/company/provider`}>Intelligence</a><button
							disabled={!!busy}
							onclick={() => archive(company.id)}>Archive company</button
						>{/if}</ActionMenu
				>{/snippet}
			{#snippet footer()}
				<span
					>{activeCompanies.length}
					{activeCompanies.length === 1 ? 'company' : 'companies'} · {appliance?.profile === 'dev'
						? 'Development profile'
						: appliance?.profile === 'test'
							? 'Test profile'
							: 'Local appliance'}</span
				>
				{#if archivedCompanies.length}<button
						class="portfolio-archived-toggle"
						type="button"
						aria-pressed={showArchived}
						onclick={() => (showArchived = !showArchived)}
						>{showArchived
							? 'Hide archived'
							: `Show archived (${archivedCompanies.length})`}</button
					>{/if}
			{/snippet}
			{#snippet before()}
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
						? 'Use New company to start one.'
						: 'Tell Exec what to build with New company.'}
				</p>
			{/snippet}
		</CompanyPortfolio>
	</AccountFrame>
</div>

<style>
	.portfolio-toast {
		display: inline-flex;
		align-items: center;
		gap: var(--space-3);
		margin: 0;
		padding: 6px 6px 6px 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		box-shadow: var(--shadow-soft);
		font-size: var(--t-body);
		animation: bridge-popover-in var(--motion-disclosure) var(--ease-spring) both;
	}
	.portfolio-toast.failure {
		color: var(--state-danger);
		padding-right: 12px;
	}
	.portfolio-archived-toggle {
		padding: 4px 6px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		cursor: pointer;
	}
	.portfolio-archived-toggle:hover,
	.portfolio-archived-toggle:focus-visible {
		background: var(--surface-alt);
		color: var(--ink);
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
