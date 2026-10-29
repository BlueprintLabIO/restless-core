<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { hostPlace, planeStatus } from '$lib/model/appliance';
	import { Page, Section, Item, Notice, Empty } from '$lib/ui/page';
	import AppWindow from '@lucide/svelte/icons/app-window';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { failureSentence } from '$lib/model/failure';
	import CompanyLimits from '$lib/components/CompanyLimits.svelte';
	import CompanyCompute from '$lib/components/CompanyCompute.svelte';
	import CompanyAiUsage from '$lib/components/CompanyAiUsage.svelte';
	import Publications from '$lib/components/Publications.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { openCompanyResource, type CompanyResource } from '$lib/model/company';
	import { companyPrincipalQuery, companyQuery } from '$lib/model/queries.svelte';
	import { hasOwnerSurfaceAccess } from '$lib/model/company-access';
	import { holdQueries } from '$lib/model/hold-queries.svelte';

	const companyId = $derived(page.params.companyId ?? '');
	/* An admin reads what the company spends; the owner also sets what it may do, and only the
	 * owner reads the whole company view behind it. */
	const principal = $derived(companyPrincipalQuery(companyId));
	const owner = $derived(hasOwnerSurfaceAccess(principal.view));
	const source = $derived(companyQuery(companyId, () => owner));
	$effect(() => source.attach());
	const view = $derived(source.view);
	const launchable = $derived(view?.resources.items.filter((item) => item.launch) ?? []);
	let opening = $state<string | null>(null);
	let launchError = $state<string | null>(null);
	let embedded = $state<{ href: string; label: string } | null>(null);
	let nativeNotice = $state<string | null>(null);

	function words(value: string): string {
		return value.replaceAll('_', ' ');
	}

	function openLabel(item: CompanyResource): string {
		if (opening === item.id) return 'Preparing…';
		if (item.launch?.shape === 'native_client') return 'Launch';
		return 'Open';
	}

	async function openResource(item: CompanyResource) {
		if (!item.launch || item.launch.availability !== 'ready' || opening) return;
		opening = item.id;
		launchError = null;
		nativeNotice = null;
		try {
			const outcome = await openCompanyResource(companyId, item);
			if (outcome.kind === 'embedded') {
				embedded = { href: outcome.href, label: item.label };
			} else if (outcome.kind === 'company_computer') {
				await goto(outcome.href);
			} else if (outcome.kind === 'external') {
				window.location.assign(outcome.href);
			} else {
				nativeNotice = outcome.reused
					? `${item.label} is already running.`
					: `${item.label} launched ${hostPlace(await planeStatus())}.`;
			}
		} catch (error) {
			launchError = failureSentence(error, 'The resource could not be opened.');
		} finally {
			opening = null;
		}
	}
	holdQueries(() => [source, principal]);
	/* Telegram moved to Notifications; an old link to it still arrives. */
	$effect(() => {
		if (page.url.hash === '#telegram')
			void goto(`/${companyId}/company/notifications`, { replaceState: true });
	});
</script>

<CompanyTitle title="Resources" {companyId} />

<Page
	title="Resources"
	info="Compute, AI usage and the limits on what the company can spend and do."
>
	{#snippet actions()}
		{#if owner}<nav class="jump" aria-label="Sections on this page">
				<a href="#compute">Compute</a><a href="#ai-usage">AI</a><a href="#spend">Spend</a><a
					href="#authority">Authority</a
				><a href="#tools">Tools</a><a href="#computer">Computer</a>
			</nav>{/if}
	{/snippet}
	{#if principal.view && !owner}
		<CompanyLimits section="spend" />
	{:else if view}
		{#if source.status === 'stale'}<Notice tone="warning" title="Showing the last values read">
				{#snippet actions()}<button class="btn small" onclick={() => source.refresh()}>Retry</button
					>{/snippet}
			</Notice>{/if}
		{#if owner}<CompanyCompute {companyId} runtime={view.computer.runtime} />
			<CompanyAiUsage {companyId} />{/if}
		<CompanyLimits section="spend" />
		{#if owner}<CompanyLimits section="authority" />

			<Section
				id="tools"
				title="Tools"
				info="Applications the company computer can open for you. Connected services and their tools live in Apps."
				count={launchable.length || null}
			>
				{#if view.resources.status === 'unavailable'}
					<Empty compact title="Tools are unavailable right now" />
				{:else}
					{#each launchable as item (item.id)}
						<Item
							title={item.label}
							meta={item.launch?.detail ?? words(item.launch?.shape ?? item.kind)}
						>
							{#snippet leading()}<AppWindow size={15} strokeWidth={1.8} />{/snippet}
							{#snippet trailing()}<button
									class="btn small"
									type="button"
									disabled={item.launch?.availability !== 'ready' || opening !== null}
									title={item.launch?.detail}
									onclick={() => openResource(item)}>{openLabel(item)}</button
								>{/snippet}
						</Item>
					{:else}
						<Empty compact title="No tools to open" />
					{/each}
				{/if}
			</Section>
			<Publications {companyId} items={view.resources.items} onchanged={() => source.refresh()} />
			{#if launchError}<Notice tone="danger" title="The resource could not be opened"
					>{launchError}</Notice
				>{/if}
			{#if nativeNotice}<Notice tone="success" title={nativeNotice} />{/if}
			{#if embedded}
				<Section title={embedded.label} group={false}>
					{#snippet actions()}<button
							class="btn small"
							type="button"
							onclick={() => (embedded = null)}>Close</button
						>{/snippet}
					<iframe
						class="viewer"
						src={embedded.href}
						title={embedded.label}
						sandbox="allow-forms allow-pointer-lock allow-scripts"
					></iframe>
				</Section>
			{/if}

			<CompanyLimits section="computer" />{/if}
	{:else if source.failure}
		<FailureNotice
			error={source.failure}
			subject="limits"
			variant="block"
			onretry={source.refresh}
		/>
	{:else}<Skeleton label="Reading limits…" variant="page" count={4} />{/if}
</Page>

<style>
	.jump {
		display: flex;
		gap: 2px;
	}
	.jump a {
		padding: 4px 8px;
		border-radius: var(--radius-control);
		color: var(--text-tertiary);
		font-size: var(--t-body);
		text-decoration: none;
	}
	.jump a:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.viewer {
		width: 100%;
		height: min(70vh, 640px);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
	}
	@container page (max-width: 800px) {
		.jump {
			display: none;
		}
	}
</style>
