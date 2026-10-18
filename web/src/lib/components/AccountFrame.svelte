<script lang="ts">
	/* The frame every non-company page shares, after Firecrawl's. The rail is the same on every
	 * page: Home, the owner's companies, then Settings and Help, with the owner's menu at the foot.
	 * A settings page lists settings' own sections in the page (`settings` receives them). On Cloud
	 * the account service owns Profile, Security, Plan and Support, so they sit in the same list
	 * (ADR 0007). Locally the account is this computer, so the menu says which appliance it is. */
	import { readAccountConnections } from '$lib/model/account-connections';
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import AccountShell from '$lib/ui/views/AccountShell.svelte';
	import House from '@lucide/svelte/icons/house';
	import Settings from '@lucide/svelte/icons/settings';
	import CircleHelp from '@lucide/svelte/icons/circle-help';
	import type { AccountGroup, AccountTab } from '$lib/ui/account';
	import { planeLabel, type ApplianceStatus } from '$lib/model/appliance';
	import { applianceQuery, companiesQuery } from '$lib/model/queries.svelte';
	import { forgetThisBrowser } from '$lib/model/sign-out';
	import CommandMenu, { type Command } from '$lib/components/CommandMenu.svelte';

	let {
		appliance = null,
		settings: settingsPage = null,
		loading = false,
		children = null
	}: {
		appliance?: ApplianceStatus | null;
		loading?: boolean;
		/** A settings page, given settings' own sections for its in-page list. */
		settings?: Snippet<[AccountGroup[]]> | null;
		children?: Snippet | null;
	} = $props();
	const identity = applianceQuery(() => !appliance);
	const status = $derived(appliance ?? identity.view);
	const profile = $derived(planeLabel(status));
	/* On a hosted plane, Home (the one company list) and the account service's sections are on the
	 * account issuer. */
	const home = $derived(status?.home_url ?? '/');
	const issuer = $derived(status?.home_url ? new URL(status.home_url).origin : null);

	/* The account pages the issuer returns to after entry (Cloud's accountReturn). */
	const ACCOUNT_RETURNS = new Set(['/account/connections', '/account/appearance']);
	$effect(() => {
		const accountHome = status?.home_url;
		const path = page.url.pathname;
		if (!status?.hosted || !accountHome || !ACCOUNT_RETURNS.has(path)) return;
		const controller = new AbortController();
		void readAccountConnections(controller.signal)
			.catch(() => null)
			.then((body) => {
				if (body?.scope !== 'company') return;
				// Once per page and minute, so a failed entry cannot loop.
				const key = `account-reentry:${path}`;
				try {
					const last = Number(sessionStorage.getItem(key) ?? 0);
					if (Date.now() - last < 60_000) return;
					sessionStorage.setItem(key, String(Date.now()));
				} catch {
					return;
				}
				const target = new URL(accountHome);
				target.searchParams.set('return', path);
				location.replace(target);
			})
			.catch(() => {
				// The page still works as the company's view.
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
			/* On Cloud a company is made on the account's own page; locally, Home makes it. */
			action: {
				label: 'New company',
				href: status?.hosted && status.home_url ? `${status.home_url}#new-company` : '/'
			},
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
					tooltip: 'Accounts and appearance',
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
	/* Search and jump (⌘K) outside a company: open any company, Home or a settings page. A
	 * company's own documents and Work are searched from inside it. */
	let searchOpen = $state(false);
	const commands = $derived<Command[]>([
		...catalog.view
			.filter((company) => company.lifecycle_status === 'active')
			.map((company) => ({
				id: `company:${company.id}`,
				group: 'Companies',
				label: company.name,
				hint: company.card?.decisions_waiting
					? `${company.card.decisions_waiting} waiting`
					: undefined,
				href: `/${encodeURIComponent(company.id)}`
			})),
		{ id: 'home', group: 'Go to', label: 'Home', hint: 'Your companies', href: home },
		{ id: 'connections', group: 'Settings', label: 'Accounts', href: '/account/connections' },
		{ id: 'appearance', group: 'Settings', label: 'Appearance', href: '/account/appearance' },
		...(status && !status.hosted
			? [
					{
						id: 'appliance',
						group: 'Settings',
						label: 'This computer',
						hint: 'Release and backup',
						href: '/account/appliance'
					}
				]
			: [])
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
			items: [own('Accounts', '/account/connections')]
		},
		...(status && !status.hosted
			? [{ label: 'This computer', items: [own('Release and backup', '/account/appliance')] }]
			: []),
		...(issuer ? [{ label: 'Help', items: issued('Support', 'support') }] : [])
	]);
</script>

<div class="account-frame" class:first-paint-pending={loading || (!appliance && !identity.settled)}>
	<AccountShell
		brandName={PRODUCT_NAME}
		homeHref={home}
		{tabs}
		{groups}
		onsearch={() => (searchOpen = !searchOpen)}
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
					<form method="POST" action="/account?/signOut" onsubmit={forgetThisBrowser}>
						<button>Sign out</button>
					</form>
				{/if}
			{/if}
		{/snippet}
		{#if settingsPage}{@render settingsPage(settingsNav)}{:else}{@render children?.()}{/if}
	</AccountShell>
</div>
<CommandMenu bind:open={searchOpen} {commands} />

<style>
	.account-frame {
		height: 100%;
		min-height: 0;
	}
</style>
