<script lang="ts">
	/* One account section's content, the same wherever it is drawn: in this plane's own settings
	 * page on a computer of its own, or mounted into the account service's settings on Cloud's one
	 * address (Cloud ADR 0008). It brings no frame; whoever draws the page brings that. */
	import { tooltips } from '$lib/actions/tooltips';
	import { selectMenu } from '$lib/actions/select-menu';
	import AccountConnections from '$lib/components/AccountConnections.svelte';
	import OwnerAccounts from '$lib/components/OwnerAccounts.svelte';
	import AccountUsage from '$lib/components/AccountUsage.svelte';
	import AccountAppearance from '$lib/components/AccountAppearance.svelte';
	import AccountAppliance from '$lib/components/AccountAppliance.svelte';
	import AccountComputers from '$lib/components/AccountComputers.svelte';
	import { ownerAccounts } from '$lib/model/owner-accounts-client';
	import { activityView } from '$lib/model/account-activity-view.svelte';
	import { companiesQuery } from '$lib/model/queries.svelte';
	import type { AccountSectionId } from '$lib/model/account-sections';

	let { id }: { id: AccountSectionId } = $props();

	/* The owner's companies, to give accounts and computers to (S98-07). */
	const catalog = companiesQuery(() => id === 'accounts' || id === 'computers');
	const grantable = $derived(
		catalog.view
			.filter((company) => company.lifecycle_status === 'active')
			.map((company) => ({ id: company.id, name: company.name }))
	);
</script>

<div class="account-section-body" use:tooltips use:selectMenu>
	{#if id === 'accounts'}{#if !activityView.id}<AccountConnections /><AccountUsage
				companies={grantable}
			/>{/if}<OwnerAccounts
			client={ownerAccounts}
			companies={grantable}
		/>{:else if id === 'computers'}<AccountComputers
			companies={grantable}
		/>{:else if id === 'this-computer'}<AccountAppliance />{:else}<AccountAppearance />{/if}
</div>

<style>
	.account-section-body {
		position: relative;
	}
	/* The sections under Accounts wait until its list has loaded, then appear in place, instead of
	 * showing first and being pushed down a few hundred pixels under the reader's eye. */
	.account-section-body :global(.account-connections[aria-busy='true'] ~ *) {
		visibility: hidden;
	}
</style>
