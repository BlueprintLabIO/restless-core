<script lang="ts">
	/* The frame every non-company page shares, after Linear. Home is the rail's one row, then the
	 * owner's companies; the owner's menu at the top-left is the one account menu. The account's
	 * pages are settings mode: a way back and the sections, grouped. On Cloud the account service
	 * owns Profile, Security, Plan and Support, so they sit in the same groups (ADR 0007). Locally
	 * the account is this computer, so the menu says which appliance it is rather than an email. */
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import AccountShell from '$lib/ui/views/AccountShell.svelte';
	import House from '@lucide/svelte/icons/house';
	import type { AccountGroup, AccountTab } from '$lib/ui/account';
	import { getApplianceStatus, planeLabel, type ApplianceStatus } from '$lib/model/appliance';
	import { companiesQuery } from '$lib/model/queries.svelte';

	let { appliance = null, children }: { appliance?: ApplianceStatus | null; children: Snippet } =
		$props();
	let observed = $state<ApplianceStatus | null>(null);
	const status = $derived(appliance ?? observed);
	const profile = $derived(planeLabel(status));
	/* On a hosted plane, Home (the one company list) and the account service's sections are on the
	 * account issuer. */
	const home = $derived(status?.home_url ?? '/');
	const issuer = $derived(status?.home_url ? new URL(status.home_url).origin : null);

	$effect(() => {
		if (appliance) return;
		const controller = new AbortController();
		void getApplianceStatus(controller.signal)
			.then((value) => (observed = value))
			.catch(() => {
				// The bar still works without the profile name.
			});
		return () => controller.abort();
	});

	const catalog = companiesQuery();
	const path = $derived(page.url.pathname);
	const settings = $derived(path.startsWith('/account'));
	const own = (label: string, href: string): AccountTab => ({ label, href, active: path === href });
	const issued = (label: string, hash: string): AccountTab[] =>
		issuer ? [{ label, href: `${issuer}/account/settings#${hash}` }] : [];

	const tabs = $derived<AccountTab[]>(
		settings
			? []
			: [
					{
						label: 'Home',
						href: home,
						active: path === '/',
						tooltip: 'Your companies',
						icon: House
					}
				]
	);
	const groups = $derived<AccountGroup[]>(
		settings
			? [
					{
						label: 'Account',
						items: [
							...issued('Profile', 'account'),
							...issued('Security', 'security'),
							own('Appearance', '/account/appearance')
						]
					},
					...(issuer ? [{ label: 'Billing', items: issued('Plan', 'billing') }] : []),
					{
						label: 'Integrations',
						items: [own('Connections', '/account/connections'), own('AI apps', '/account/ai-apps')]
					},
					...(issuer ? [{ label: 'Help', items: issued('Support', 'support') }] : [])
				]
			: [
					{
						label: 'Companies',
						items: catalog.view
							.filter((company) => company.lifecycle_status === 'active')
							.map((company) => ({
								label: company.name,
								href: `/${company.id}`,
								count: company.card?.decisions_waiting,
								tone:
									company.runtime_status === 'unavailable' || company.unstartable_reason
										? 'unavailable'
										: undefined
							}))
					}
				]
	);
</script>

<AccountShell
	brandName={PRODUCT_NAME}
	homeHref={home}
	{tabs}
	{groups}
	back={settings ? { label: 'Back', href: home, tooltip: 'Back to your companies' } : null}
	account={{
		name: status?.viewer_name || 'You',
		detail: status?.hosted ? profile : `${profile} · this computer`
	}}
>
	{#snippet accountMenu()}
		<a href="/account/connections">Settings</a>
		{#if issuer}
			<a href={`${issuer}/account/settings#support`}>Support</a>
			{#if issuer === page.url.origin}
				<hr />
				<form method="POST" action="/account?/signOut"><button>Sign out</button></form>
			{/if}
		{/if}
	{/snippet}
	{@render children()}
</AccountShell>
