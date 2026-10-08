<script lang="ts">
	/* The frame every non-company page shares, after Firecrawl's. The rail is the same on every
	 * page: Home, the owner's companies, then Settings and Help, with the owner's menu at the foot.
	 * A settings page lists settings' own sections in the page (`settings` receives them). On Cloud
	 * the account service owns Profile, Security, Plan and Support, so they sit in the same list
	 * (ADR 0007). Locally the account is this computer, so the menu says which appliance it is. */
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import AccountShell from '$lib/ui/views/AccountShell.svelte';
	import House from '@lucide/svelte/icons/house';
	import Settings from '@lucide/svelte/icons/settings';
	import CircleHelp from '@lucide/svelte/icons/circle-help';
	import type { AccountGroup, AccountTab } from '$lib/ui/account';
	import { getApplianceStatus, planeLabel, type ApplianceStatus } from '$lib/model/appliance';
	import { companiesQuery } from '$lib/model/queries.svelte';

	let {
		appliance = null,
		settings: settingsPage = null,
		children = null
	}: {
		appliance?: ApplianceStatus | null;
		/** A settings page, given settings' own sections for its in-page list. */
		settings?: Snippet<[AccountGroup[]]> | null;
		children?: Snippet | null;
	} = $props();
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

	const tabs = $derived<AccountTab[]>([
		{
			label: 'Home',
			href: home,
			active: path === '/',
			tooltip: 'Your companies',
			icon: House
		}
	]);
	const groups = $derived<AccountGroup[]>([
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
		},
		{
			label: 'Account',
			items: [
				{
					label: 'Settings',
					href: '/account/connections',
					active: settings,
					tooltip: 'Connections, AI apps and appearance',
					icon: Settings
				},
				...(issuer
					? [
							{
								label: 'Help',
								href: `${issuer}/account/settings#support`,
								tooltip: 'Support for entry and service health',
								icon: CircleHelp
							}
						]
					: [])
			]
		}
	]);
	/* Settings' own list, shown in the page beside the section. */
	const settingsNav = $derived<AccountGroup[]>([
		{
			label: 'Account',
			items: [
				...issued('Profile', 'account'),
				...issued('Security', 'security'),
				own('Appearance', '/account/appearance')
			]
		},
		...(issuer ? [{ label: 'Billing', items: issued('Plan & credit', 'billing') }] : []),
		{
			label: 'Integrations',
			items: [own('Connections', '/account/connections'), own('AI apps', '/account/ai-apps')]
		},
		...(issuer ? [{ label: 'Help', items: issued('Support', 'support') }] : [])
	]);
</script>

<AccountShell
	brandName={PRODUCT_NAME}
	homeHref={home}
	{tabs}
	{groups}
	account={{
		name: status?.viewer_name || 'You',
		detail: status?.hosted ? profile : `${profile} · this computer`
	}}
>
	{#snippet accountMenu()}
		<a href="/account/connections">Settings</a>
		{#if issuer}
			<!-- The account's own page: a full load, so the edge sends it to the account, not this plane. -->
			<a href={`${issuer}/account/settings#support`} data-sveltekit-reload>Support</a>
			{#if issuer === page.url.origin}
				<hr />
				<form method="POST" action="/account?/signOut"><button>Sign out</button></form>
			{/if}
		{/if}
	{/snippet}
	{#if settingsPage}{@render settingsPage(settingsNav)}{:else}{@render children?.()}{/if}
</AccountShell>
