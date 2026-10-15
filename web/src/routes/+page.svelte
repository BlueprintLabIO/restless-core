<script lang="ts">
	import { accountHomeDestination } from '$lib/model/account-home';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import {
		archiveCompany,
		companyExportHref,
		deleteCompany,
		restoreCompany
	} from '$lib/model/cockpit';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import CompanyPortfolio from '$lib/ui/views/CompanyPortfolio.svelte';
	import type { CompanyPortfolioEntry } from '$lib/ui/portfolio';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { tooltips } from '$lib/actions/tooltips';
	import { selectMenu } from '$lib/actions/select-menu';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import CreateCompany from '$lib/components/CreateCompany.svelte';
	import HomeAccount from '$lib/components/HomeAccount.svelte';
	import AccountFrame from '$lib/components/AccountFrame.svelte';
	import { planeLabel } from '$lib/model/appliance';
	import ApplianceRepair from '$lib/components/ApplianceRepair.svelte';
	import { applianceQuery, companiesQuery } from '$lib/model/queries.svelte';
	import {
		accountGrantFix,
		getAccountConnections,
		startFixHref,
		startGuidance,
		type AccountConnectionSummary
	} from '$lib/model/company-start';
	import type { CompanyCatalogEntry } from '$lib/model/cockpit';

	const companyCatalog = companiesQuery();
	const companies = $derived(companyCatalog.view);

	const RUN_STATE: Record<CompanyCatalogEntry['runtime_status'], string> = {
		running: 'Running',
		asleep: 'Asleep',
		stopped: 'Stopped',
		absent: 'Not started',
		unavailable: 'Unavailable'
	};
	let redirected = $state(false);
	const identity = applianceQuery();
	const appliance = $derived(identity.view);
	const destination = $derived(accountHomeDestination(page.url, appliance));
	const loaded = $derived(companyCatalog.status !== 'unknown' && identity.settled && !destination);
	$effect(() => {
		if (redirected || !identity.settled || !destination) return;
		redirected = true;
		if (destination.startsWith('/')) void goto(destination, { replaceState: true });
		else window.location.replace(destination);
	});
	const activeCompanies = $derived(
		companies.filter((company) => company.lifecycle_status === 'active')
	);
	const archivedCompanies = $derived(
		companies.filter((company) => company.lifecycle_status === 'archived')
	);

	/* Repair refreshes the same identity read the shell uses. */
	async function refreshAppliance() {
		await identity.refresh();
	}
	let accountConnections = $state<AccountConnectionSummary[]>([]);
	$effect(() => {
		const controller = new AbortController();
		// Only refines a blocked company's fix; without it the Company page link stays.
		void getAccountConnections(controller.signal)
			.then((rows) => (accountConnections = rows))
			.catch(() => {});
		return () => controller.abort();
	});

	/* Archiving stops the company, so it says what that means first; the toast then offers Undo.
	 * Restoring and failures say so on the toast too. */
	let busy = $state('');
	async function archive(id: string, name: string) {
		if (busy) return;
		if (
			!(await confirmAction({
				title: `Archive ${name}?`,
				body: 'Exec, Staff and schedules stop, and its computer is shut down. Files, history and settings are kept, and you can restore it at any time.',
				action: 'Archive company'
			}))
		)
			return;
		busy = id;
		try {
			await archiveCompany(id);
			await companyCatalog.refresh();
			toast('Company archived', { undo: () => restore(id) });
		} catch (cause) {
			toast(failureSentence(cause, 'Company could not be archived.'), { tone: 'failure' });
		} finally {
			busy = '';
		}
	}
	/* Deleting is for good, so it asks for the name and offers the export first. */
	async function remove(id: string, name: string) {
		if (busy) return;
		if (
			!(await confirmAction({
				title: `Delete ${name} permanently?`,
				body: 'Its computer, files, work, conversations and settings are removed and cannot be recovered. The record of outside actions and approvals is kept with your account. Download an export first if you may need anything.',
				action: 'Delete permanently',
				danger: true,
				typeToConfirm: name,
				offer: { label: 'Download export', href: companyExportHref(id), download: true }
			}))
		)
			return;
		busy = id;
		try {
			await deleteCompany(id, name);
			await companyCatalog.refresh();
			toast(`${name} deleted`);
		} catch (cause) {
			toast(failureSentence(cause, 'Company could not be deleted.'), { tone: 'failure' });
		} finally {
			busy = '';
		}
	}
	let showArchived = $state(false);
	async function restore(id: string) {
		if (busy) return;
		busy = id;
		try {
			await restoreCompany(id);
			await companyCatalog.refresh();
			toast('Company restored');
		} catch (cause) {
			toast(failureSentence(cause, 'Company could not be restored.'), { tone: 'failure' });
		} finally {
			busy = '';
		}
	}
	const archivedRows = $derived(
		showArchived
			? archivedCompanies.map((company): CompanyPortfolioEntry => ({
					id: company.id,
					name: company.name,
					logoUrl: company.logo_url,
					status: 'Archived',
					tone: 'waiting',
					/* Opens read-only: its charter and record, with Restore, Export and Delete. */
					entry: { href: `/archived/${encodeURIComponent(company.id)}` },
					dormant: true
				}))
			: []
	);
	const rows = $derived([
		...activeCompanies.map((company): CompanyPortfolioEntry => {
			const reason = company.unstartable_reason ?? '';
			const grant = reason ? accountGrantFix(reason, company.id, accountConnections) : null;
			const card = company.card;
			return {
				id: company.id,
				name: company.name,
				logoUrl: company.logo_url,
				status: reason ? 'Can’t start' : RUN_STATE[company.runtime_status],
				tone:
					reason || company.runtime_status === 'unavailable'
						? 'unavailable'
						: company.runtime_status === 'running'
							? 'presence'
							: 'waiting',
				card: card
					? {
							decisionsWaiting: card.decisions_waiting,
							peopleWorking: card.people_working,
							outcomesLastDay: card.outcomes_last_day,
							execReady: card.exec_ready,
							lastActivityAt: card.last_activity_at
						}
					: null,
				...(reason
					? {
							issue: grant
								? `Exec can’t start until you share your ${grant.connection.label} sign-in with it.`
								: `Exec can’t start. ${startGuidance(reason)}`,
							issueAction: grant ? 'Give access' : 'Fix setup'
						}
					: {}),
				entry: { href: grant?.href ?? (reason ? startFixHref(company.id) : `/${company.id}`) }
			};
		}),
		...archivedRows
	]);
	const archivedSet = $derived(new Set(archivedCompanies.map((company) => company.id)));
</script>

<svelte:head><title>Companies — {PRODUCT_NAME}</title></svelte:head>

{#snippet feedback()}
	{#if companyCatalog.failure || identity.failure}
		<FailureNotice
			error={companyCatalog.failure ?? identity.failure!}
			subject="your companies"
			stale={loaded}
			variant={loaded ? 'inline' : 'page'}
			onretry={() => {
				void companyCatalog.refresh();
				void identity.refresh();
			}}
		/>
	{/if}
{/snippet}

<div use:tooltips use:selectMenu>
	<AccountFrame {appliance} loading={!loaded && !companyCatalog.failure && !identity.failure}>
		{#if identity.failure || (companyCatalog.failure && !companies.length)}
			<FailureNotice
				error={identity.failure ?? companyCatalog.failure!}
				subject={identity.failure ? 'account navigation' : 'your companies'}
				variant="page"
				onretry={() => Promise.all([identity.refresh(), companyCatalog.refresh()])}
			/>
		{:else}
			<CompanyPortfolio
				companies={rows}
				{loaded}
				feedback={companyCatalog.failure || identity.failure ? feedback : null}
			>
				{#snippet actions()}
					{#if loaded}<CreateCompany />{/if}
				{/snippet}
				{#snippet rowActions(company)}<ActionMenu label={`${company.name} options`}
						>{#if archivedSet.has(company.id)}<a
								href={`/archived/${encodeURIComponent(company.id)}`}>Open read-only</a
							><button disabled={!!busy} onclick={() => restore(company.id)}>Restore company</button
							><a href={companyExportHref(company.id)} download>Download export</a><button
								disabled={!!busy}
								onclick={() => remove(company.id, company.name)}>Delete permanently</button
							>{:else}<a href={`/${company.id}`}>Open</a><a href={`/${company.id}/company`}
								>Company</a
							><a href={`/${company.id}/company/provider`}>Intelligence</a><button
								disabled={!!busy}
								onclick={() => archive(company.id, company.name)}>Archive company</button
							>{/if}</ActionMenu
					>{/snippet}
				{#snippet footer()}
					<span
						>{activeCompanies.length}
						{activeCompanies.length === 1 ? 'company' : 'companies'} · {planeLabel(appliance)}</span
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
				{#snippet after()}
					<HomeAccount companies={activeCompanies} {appliance} />
				{/snippet}
				{#snippet before()}
					{#if appliance?.state === 'recovering'}
						<div class="appliance-notice" role="status">
							<span>Restoring runtime safety.</span>
							<p>{appliance.repair}</p>
							<ApplianceRepair
								actions={appliance.repair_actions ?? []}
								ondone={() => void refreshAppliance()}
							/>
						</div>
					{:else if appliance?.state === 'draining'}
						<div class="appliance-notice" role="status">
							<span>Work admission is paused.</span>
							<p>{appliance.repair}</p>
							<ApplianceRepair
								actions={appliance.repair_actions ?? []}
								ondone={() => void refreshAppliance()}
							/>
						</div>
					{:else if appliance?.state === 'degraded'}
						<div class="appliance-notice" role="status">
							<span>Schedule wake needs repair.</span>
							<p>{appliance.repair}</p>
							<ApplianceRepair
								actions={appliance.repair_actions ?? []}
								ondone={() => void refreshAppliance()}
							/>
						</div>
					{/if}
				{/snippet}

				{#snippet empty()}
					<h2>No companies yet</h2>
					<p>
						{archivedCompanies.length
							? 'Use Start a company to begin a new one.'
							: 'Use Start a company and tell Exec what to build.'}
					</p>
				{/snippet}
			</CompanyPortfolio>
		{/if}
	</AccountFrame>
</div>

<style>
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
