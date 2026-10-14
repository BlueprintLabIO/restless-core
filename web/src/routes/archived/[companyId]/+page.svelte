<script lang="ts">
	/* An archived company, read-only: its charter and its record of decisions and outside actions,
	 * with the three things an owner does with it. Its computer stays off; everything else it held
	 * is in the export. */
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { tooltips } from '$lib/actions/tooltips';
	import { selectMenu } from '$lib/actions/select-menu';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import AccountFrame from '$lib/components/AccountFrame.svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { Page, Section, Item, Empty } from '$lib/ui/page';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { formatMoment } from '$lib/ui/time';
	import { failureSentence } from '$lib/model/failure';
	import {
		companyExportHref,
		deleteCompany,
		getArchivedCompany,
		restoreCompany,
		type ArchivedCompany
	} from '$lib/model/cockpit';
	import { getGovernanceActivity, type CompanyExternalAction } from '$lib/model/company';

	const companyId = $derived(page.params.companyId ?? '');
	let company = $state<ArchivedCompany | null>(null);
	let failure = $state<unknown>(null);
	let record = $state<CompanyExternalAction[]>([]);
	let next = $state<number | undefined>(undefined);
	let reading = $state(false);
	let busy = $state('');

	async function load() {
		failure = null;
		try {
			company = await getArchivedCompany(companyId);
			const first = await getGovernanceActivity(companyId);
			record = first.items;
			next = first.next_before;
		} catch (cause) {
			failure = cause;
		}
	}
	$effect(() => {
		companyId;
		void load();
	});
	async function earlier() {
		if (next == null || reading) return;
		reading = true;
		try {
			const more = await getGovernanceActivity(companyId, next);
			record = [...record, ...more.items];
			next = more.next_before;
		} catch (cause) {
			toast(failureSentence(cause, 'Earlier activity could not be read.'), { tone: 'failure' });
		} finally {
			reading = false;
		}
	}
	async function restore() {
		if (busy || !company) return;
		busy = 'restore';
		try {
			await restoreCompany(companyId);
			toast(`${company.name} restored`);
			await goto(`/${encodeURIComponent(companyId)}/company`);
		} catch (cause) {
			toast(failureSentence(cause, 'Company could not be restored.'), { tone: 'failure' });
		} finally {
			busy = '';
		}
	}
	async function remove() {
		if (busy || !company) return;
		const name = company.name;
		if (
			!(await confirmAction({
				title: `Delete ${name} permanently?`,
				body: 'Its computer, files, work, conversations and settings are removed and cannot be recovered. The record of outside actions and approvals is kept with your account. Download an export first if you may need anything.',
				action: 'Delete permanently',
				danger: true,
				typeToConfirm: name,
				offer: { label: 'Download export', href: companyExportHref(companyId), download: true }
			}))
		)
			return;
		busy = 'delete';
		try {
			await deleteCompany(companyId, name);
			toast(`${name} deleted`);
			await goto('/');
		} catch (cause) {
			toast(failureSentence(cause, 'Company could not be deleted.'), { tone: 'failure' });
		} finally {
			busy = '';
		}
	}
</script>

<svelte:head><title>{company?.name ?? 'Archived company'} — {PRODUCT_NAME}</title></svelte:head>

<div use:tooltips use:selectMenu>
	<AccountFrame>
		<Page
			title={company?.name ?? 'Archived company'}
			info="Archived: its computer is off and nothing runs. Restore it to work again, or download everything it holds."
		>
			{#snippet actions()}
				<a class="btn small" href={companyExportHref(companyId)} download>Download export</a>
				<button class="btn small" disabled={!!busy} onclick={restore}
					>{busy === 'restore' ? 'Restoring…' : 'Restore'}</button
				>
				<button class="btn small danger" disabled={!!busy} onclick={remove}
					>{busy === 'delete' ? 'Deleting…' : 'Delete permanently'}</button
				>
			{/snippet}
			{#if company}
				<Section
					title="Charter"
					info={company.archived_at
						? `As it stood when archived, ${formatMoment(company.archived_at)}.`
						: 'As it stood when archived.'}
				>
					<div class="charter">
						{#if company.charter.trim()}<Markdown text={company.charter} />{:else}<Empty
								compact
								title="No charter was written"
							/>{/if}
					</div>
				</Section>
				<Section
					title="Activity"
					info="Outside actions, approvals and setting changes, newest first."
					count={record.length || null}
				>
					{#each record as item (item.id)}
						<Item
							title={item.title}
							meta={`${formatMoment(item.observed_at)}${item.party ? ` · ${item.party}` : ''}`}
						/>
					{:else}
						<Empty compact title="Nothing was recorded" />
					{/each}
				</Section>
				{#if next != null}<div class="earlier">
						<button class="btn small" disabled={reading} onclick={earlier}
							>{reading ? 'Reading…' : 'Show earlier'}</button
						>
					</div>{/if}
			{:else if failure}
				<FailureNotice error={failure} subject="this company" variant="block" onretry={load} />
			{:else}
				<Skeleton label="Reading the company…" variant="page" count={3} />
			{/if}
		</Page>
	</AccountFrame>
</div>

<style>
	.charter {
		padding: 16px 20px;
	}
	.earlier {
		display: flex;
		justify-content: center;
		padding: var(--space-3) 0;
	}
</style>
