<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Row, Item, Notice, Empty } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { failureSentence } from '$lib/model/failure';
	import { page } from '$app/state';
	import CompanyNameField from '$lib/components/CompanyNameField.svelte';
	import CompanyLogoField from '$lib/components/CompanyLogoField.svelte';
	import CopyCompanySetting from '$lib/components/CopyCompanySetting.svelte';
	import CharterPurpose from '$lib/components/CharterPurpose.svelte';
	import { companyPrincipalQuery, companyQuery } from '$lib/model/queries.svelte';
	import SharedCharter from '$lib/components/SharedCharter.svelte';
	import LegalProfile from '$lib/components/LegalProfile.svelte';
	import ArrowRight from '@lucide/svelte/icons/arrow-right';
	import Target from '@lucide/svelte/icons/target';
	import { holdQueries } from '$lib/model/hold-queries.svelte';

	const companyId = $derived(page.params.companyId ?? '');
	/* Everyone reads the charter; the owner edits it, from the owner's company view, which only
	 * the owner may read. */
	const principal = $derived(companyPrincipalQuery(companyId));
	const owner = $derived(principal.view?.membership_role === 'owner');
	const source = $derived(companyQuery(companyId, () => owner));
	$effect(() => {
		if (owner) source.attach();
	});
	const view = $derived(source.view);
	let nameVersion = $state(0);
	holdQueries(() => [source, principal]);
</script>

<CompanyTitle title="Charter" {companyId} />

<Page
	title="Charter"
	info="Who the company is and why it exists. The charter is its purpose and operating rules, not its legal constitution or its current plan."
>
	{#if !owner}
		<SharedCharter {companyId} />
	{:else if view}
		{#if source.status === 'stale'}<Notice
				tone="warning"
				title="Showing the last saved charter"
				details={source.failure ? failureSentence(source.failure, '') : null}
			>
				{#snippet actions()}<button class="btn small" onclick={() => source.refresh()}>Retry</button
					>{/snippet}
			</Notice>{/if}
		<Section title="Company">
			<Row label="Name">
				{#key `${companyId}:${nameVersion}`}<CompanyNameField {companyId} />{/key}
				<CopyCompanySetting
					{companyId}
					setting="name"
					label="Company name"
					oncopied={async () => {
						nameVersion += 1;
						await source.refresh();
					}}
				/>
			</Row>
			<Row
				label="Logo"
				info="Shown beside the company name across the cockpit, and kept in /company/brand for Exec and Staff to use in sites, decks and email. PNG, JPEG, WebP or SVG."
			>
				<CompanyLogoField {companyId} />
			</Row>
		</Section>

		<CharterPurpose
			{companyId}
			{view}
			onsaved={(company) => {
				source.accept(company);
				nameVersion += 1;
			}}
		/>

		<Section
			title="Current direction"
			info="What the company is working toward now. It changes as work moves; the charter does not."
		>
			{#if view.charter.current_direction}
				<Item
					title={view.charter.current_direction.title}
					meta={view.charter.current_direction.body}
					href={view.charter.current_direction.href}
				>
					{#snippet leading()}<Target size={15} strokeWidth={1.8} />{/snippet}
					{#snippet trailing()}<ArrowRight
							size={14}
							strokeWidth={1.8}
							aria-hidden="true"
						/>{/snippet}
				</Item>
			{:else if view.charter.current_direction_status !== 'available'}
				<Empty compact title="Current direction is unavailable right now" />
			{:else}
				<Empty compact title="No open company goal" />
			{/if}
		</Section>

		<LegalProfile {companyId} {view} onsaved={(company) => source.accept(company)} />
	{:else if source.failure}
		<FailureNotice
			error={source.failure}
			subject="the charter"
			variant="block"
			onretry={source.refresh}
		/>
	{:else}
		<Skeleton label="Reading the charter…" variant="page" count={4} />
	{/if}
</Page>
