<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { tick } from 'svelte';
	import CompanyPortfolio from '$lib/components/CompanyPortfolio.svelte';
	import { getApplianceStatus, type ApplianceStatus } from '$lib/model/appliance';
	import { archiveCompany, restoreCompany, type CompanyCatalogEntry } from '$lib/model/cockpit';
	import {
		platformQuery,
		portfolioQuery,
		type PortfolioProjection
	} from '$lib/model/queries.svelte';
	import { createPlatformCompany, enterPlatformCompany } from '$lib/platform';
	import type { ProductNotice } from '$lib/product/contracts';

	const platform = platformQuery();
	const portfolio = portfolioQuery();
	const companies = $derived(portfolio.view?.companies ?? []);
	const projections = $derived(
		portfolio.view?.projections ?? ({} as Record<string, PortfolioProjection>)
	);
	const authRequired = $derived(
		platform.failure?.status === 401 || portfolio.failure?.status === 401
	);
	const loaded = $derived(portfolio.status !== 'unknown' || portfolio.failure !== null);
	const error = $derived(
		authRequired ? 'Sign in to see your companies.' : (portfolio.failure?.message ?? '')
	);
	const ownerLabel = $derived(platform.view?.identity.displayName ?? 'Owner');
	const supportHref = $derived(
		platform.view?.capabilities.includes('account.support')
			? (platform.view.navigation.supportHref ?? null)
			: null
	);
	const signOutHref = $derived(
		platform.view?.capabilities.includes('account.sign_out')
			? (platform.view.navigation.signOutHref ?? null)
			: null
	);
	const archiveAction = $derived(
		platform.view?.capabilities.includes('company.archive') ? archive : null
	);
	const restoreAction = $derived(
		platform.view?.capabilities.includes('company.restore') ? restore : null
	);
	const openAction = $derived(
		platform.view?.mode === 'cloud_fleet' && platform.view.capabilities.includes('company.open')
			? openCloudCompany
			: null
	);
	const canCreate = $derived(platform.view?.capabilities.includes('company.create') ?? false);
	let redirected = $state(false);
	let appliance = $state<ApplianceStatus | null>(null);
	let entryError = $state('');
	let enteringCompany = $state<string | null>(null);
	let createDialog = $state<HTMLDialogElement | null>(null);
	let companyNameInput = $state<HTMLInputElement | null>(null);
	let companyName = $state('');
	let creatingCompany = $state(false);
	let creationError = $state('');
	let actionNotice = $state<ProductNotice | null>(null);
	const notice = $derived.by((): ProductNotice | null => {
		if (entryError) return { title: 'The company did not open.', detail: entryError };
		if (enteringCompany) {
			return { title: 'Opening company.', detail: 'Creating a secure Core session.' };
		}
		if (actionNotice) return actionNotice;
		if (appliance?.state === 'degraded') {
			return {
				title: 'Schedule wake needs repair.',
				detail: appliance.repair ?? 'Run the appliance repair command and check again.'
			};
		}
		if (appliance?.model_gateway === 'starting') {
			return {
				title: 'Model access is starting.',
				detail:
					'Companies will wake after provider access is ready. The owner surface remains available.'
			};
		}
		return null;
	});

	$effect(() => {
		// Keep an unauthenticated Fleet visit on the canonical portfolio long
		// enough to present Cloud's sign-in door instead of bouncing through the
		// protected `next` route forever. Self-hosted local-owner mode never
		// enters this branch and retains its existing post-load continuation.
		if (redirected || !loaded || authRequired) return;
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
		await Promise.all([platform.refresh(), portfolio.refresh()]);
	}

	async function openCreateCompany() {
		companyName = '';
		creationError = '';
		actionNotice = null;
		createDialog?.showModal();
		await tick();
		companyNameInput?.focus();
	}

	async function createCompany(event: SubmitEvent) {
		event.preventDefault();
		if (creatingCompany) return;
		const name = companyName.trim();
		if (name.length < 2 || name.length > 80) {
			creationError = 'Use a company name between 2 and 80 characters.';
			return;
		}
		creatingCompany = true;
		creationError = '';
		try {
			const created = await createPlatformCompany(name);
			createDialog?.close();
			actionNotice = {
				title: `${created.name} is being prepared.`,
				detail: 'It will become available here after its private Core environment passes readiness.'
			};
			await changed();
		} catch (cause) {
			creationError = cause instanceof Error ? cause.message : 'The company could not be created.';
		} finally {
			creatingCompany = false;
		}
	}

	function openCloudCompany(company: CompanyCatalogEntry) {
		/* Company entry targets the always-on Core account plane, not the
		 * company's Runtime. A sleeping or replacing Runtime is therefore still
		 * enterable: wake/recovery belongs inside the real company cockpit. The
		 * platform endpoint remains the authority and refuses a plane that is not
		 * ready or a membership that is no longer active. */
		if (enteringCompany) return;
		entryError = '';
		enteringCompany = company.id;
		try {
			enterPlatformCompany(company.id);
		} catch (cause) {
			entryError = cause instanceof Error ? cause.message : 'Core entry failed.';
			enteringCompany = null;
		}
	}

	async function archive(company: CompanyCatalogEntry) {
		await archiveCompany(company.id);
	}

	async function restore(company: CompanyCatalogEntry) {
		await restoreCompany(company.id);
	}
</script>

{#snippet ownerActions()}
	{#if supportHref}<a href={supportHref} data-sveltekit-reload>Help &amp; status</a>{/if}
	{#if signOutHref}
		<form method="POST" action={signOutHref}>
			<button type="submit">Sign out</button>
		</form>
	{/if}
{/snippet}

{#snippet portfolioActions()}
	{#if canCreate}<button class="portfolio-create" type="button" onclick={openCreateCompany}
			>New company</button
		>{/if}
	{#if authRequired}<a class="portfolio-sign-in" href="/auth/sign-in" data-sveltekit-reload
			>Sign in</a
		>{/if}
{/snippet}

<CompanyPortfolio
	{companies}
	{projections}
	{loaded}
	{error}
	{notice}
	{ownerLabel}
	ownerActions={supportHref || signOutHref ? ownerActions : null}
	actions={canCreate || authRequired ? portfolioActions : null}
	onopen={openAction}
	onarchive={archiveAction}
	onrestore={restoreAction}
	onchanged={changed}
/>

<dialog
	class="bridge-root company-create-dialog"
	bind:this={createDialog}
	aria-labelledby="company-create-title"
>
	<div class="company-create-head">
		<div>
			<h2 id="company-create-title">Name the company</h2>
			<p>Restless will prepare its private Core environment after you create it.</p>
		</div>
		<form method="dialog"><button class="company-create-close" aria-label="Close">×</button></form>
	</div>
	<form class="company-create-form" onsubmit={createCompany}>
		<label for="company-name">Company name</label>
		<input
			id="company-name"
			name="company_name"
			bind:this={companyNameInput}
			bind:value={companyName}
			minlength="2"
			maxlength="80"
			autocomplete="organization"
			required
		/>
		{#if creationError}<p class="company-create-error" role="alert">{creationError}</p>{/if}
		<div class="company-create-actions">
			<button type="button" onclick={() => createDialog?.close()}>Cancel</button>
			<button class="company-create-submit" disabled={creatingCompany}>
				{creatingCompany ? 'Creating…' : 'Create company'}
			</button>
		</div>
	</form>
</dialog>

<style>
	:global(.portfolio-sign-in),
	:global(.portfolio-create),
	.company-create-submit {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-height: 34px;
		padding: 0 13px;
		border: 1px solid var(--ink);
		border-radius: var(--radius-control);
		background: var(--ink);
		box-shadow: var(--bevel-subtle);
		font: 600 var(--t-label) var(--font-mono);
		color: var(--surface);
		text-decoration: none;
	}

	:global(.portfolio-create) {
		cursor: pointer;
	}

	.company-create-dialog {
		width: min(480px, calc(100vw - 32px));
		padding: 0;
		border: 1px solid var(--ink);
		border-radius: var(--radius-pane);
		background: var(--surface);
		box-shadow: var(--shadow-lift);
		color: var(--ink);
	}

	.company-create-dialog::backdrop {
		background: color-mix(in srgb, var(--ink) 38%, transparent);
	}

	.company-create-head {
		display: flex;
		justify-content: space-between;
		gap: 24px;
		padding: 24px 24px 19px;
		border-bottom: 1px solid var(--border-strong);
	}

	.company-create-head h2,
	.company-create-head p,
	.company-create-error {
		margin: 0;
	}

	.company-create-head h2 {
		font-size: var(--t-title);
	}

	.company-create-head p {
		max-width: 39ch;
		margin-top: 5px;
		color: var(--text-tertiary);
	}

	.company-create-close {
		width: 32px;
		height: 32px;
		padding: 0;
		border: 0;
		background: transparent;
		font-size: var(--t-title);
		line-height: 1;
		color: var(--text-tertiary);
		cursor: pointer;
	}

	.company-create-form {
		display: grid;
		gap: 9px;
		padding: 22px 24px 24px;
	}

	.company-create-form label {
		font-weight: 650;
	}

	.company-create-form input {
		min-height: 42px;
		padding: 0 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface);
		font: inherit;
		color: inherit;
	}

	.company-create-form input:focus-visible,
	.company-create-close:focus-visible,
	.company-create-actions button:focus-visible,
	:global(.portfolio-create:focus-visible) {
		outline: 2px solid var(--ink);
		outline-offset: 2px;
	}

	.company-create-error {
		color: var(--danger, #a22828);
	}

	.company-create-actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 13px;
	}

	.company-create-actions button {
		min-height: 34px;
		padding: 0 13px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface);
		font: 600 var(--t-label) var(--font-mono);
		color: var(--ink);
		cursor: pointer;
	}

	.company-create-actions .company-create-submit {
		border-color: var(--ink);
		background: var(--ink);
		color: var(--surface);
	}

	.company-create-submit:disabled {
		cursor: wait;
		opacity: 0.62;
	}
</style>
