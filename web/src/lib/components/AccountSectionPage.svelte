<script lang="ts">
	/* One account section this plane owns (its credentials, or this browser's preferences), framed by
	 * the account rail. On Cloud the same address carries Fleet's Profile, Security, Plan and Support:
	 * each section lives once, with whoever owns its data (ADR 0007). */
	import { tooltips } from '$lib/actions/tooltips';
	import { selectMenu } from '$lib/actions/select-menu';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import AccountFrame from '$lib/components/AccountFrame.svelte';
	import AccountConnections from '$lib/components/AccountConnections.svelte';
	import OwnerAccounts from '$lib/components/OwnerAccounts.svelte';
	import { ownerAccounts } from '$lib/model/owner-accounts-client';
	import { companiesQuery } from '$lib/model/queries.svelte';
	import AccountAppearance from '$lib/components/AccountAppearance.svelte';
	import AccountAppliance from '$lib/components/AccountAppliance.svelte';
	import AccountPage from '$lib/ui/views/AccountPage.svelte';
	import type { AccountSection } from '$lib/ui/account';

	type Id = 'connections' | 'appearance' | 'appliance';
	let { id }: { id: Id } = $props();

	const SECTIONS: Record<Id, AccountSection> = {
		connections: {
			id: 'connections',
			title: 'Accounts',
			tooltip:
				'Accounts you hold once and let companies use: AI models, website logins, API keys and browser sign-ins. Each company gets only what you give it, and never sees a password or key.'
		},

		appliance: {
			id: 'appliance',
			title: 'This computer',
			tooltip:
				'The appliance this computer runs: its release, rolling back, and backing up every company on it.'
		},
		appearance: {
			id: 'appearance',
			title: 'Appearance',
			tooltip: 'Light, dark, or follow this device. Saved in this browser on this device only.'
		}
	};
	const current = $derived(SECTIONS[id]);
	/* The owner's companies, to give accounts to (S98-07). */
	const catalog = companiesQuery(() => id === 'connections');
	const grantable = $derived(
		catalog.view
			.filter((company) => company.lifecycle_status === 'active')
			.map((company) => ({ id: company.id, name: company.name }))
	);
</script>

<svelte:head><title>{current.title} — {PRODUCT_NAME}</title></svelte:head>

<div use:tooltips use:selectMenu>
	<AccountFrame>
		{#snippet settings(nav)}
			<AccountPage title="Settings" sections={[current]} {nav}>
				{#snippet section()}
					{#if id === 'connections'}<AccountConnections /><OwnerAccounts
							client={ownerAccounts}
							companies={grantable}
						/>{:else if id === 'appliance'}<AccountAppliance />{:else}<AccountAppearance />{/if}
				{/snippet}
			</AccountPage>
		{/snippet}
	</AccountFrame>
</div>
