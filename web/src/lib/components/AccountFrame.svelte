<script lang="ts">
	/* The frame every non-company page shares, after Firecrawl's, when this plane draws its own
	 * account pages: a computer of its own, or an account service at an address of its own. The rail
	 * is the same on every page: Home, the owner's companies, then Settings, with the owner's menu at
	 * the foot. A settings page lists settings' own sections in the page (`settings` receives them).
	 * On Cloud's one address the account service draws every account page instead (Cloud ADR 0008). */
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import AccountShell from '$lib/ui/views/AccountShell.svelte';
	import House from '@lucide/svelte/icons/house';
	import Settings from '@lucide/svelte/icons/settings';
	import type { AccountGroup, AccountTab } from '$lib/ui/account';
	import { planeLabel, type ApplianceStatus } from '$lib/model/appliance';
	import { applianceQuery, companiesQuery } from '$lib/model/queries.svelte';
	import { ACCOUNT_SECTIONS, type AccountSectionId } from '$lib/model/account-sections';
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
	/* An account service at an address of its own keeps Home, the one company list. */
	const home = $derived(status?.home_url ?? '/');
	/* Release and backup are this computer's own; a hosted plane has none to show. */
	const sectionIds = $derived<AccountSectionId[]>([
		'accounts',
		'computers',
		'appearance',
		...(status && !status.hosted ? (['this-computer'] as const) : [])
	]);

	const catalog = companiesQuery();
	const path = $derived(page.url.pathname);
	const own = (id: AccountSectionId): AccountTab => ({
		label: ACCOUNT_SECTIONS[id].title,
		href: ACCOUNT_SECTIONS[id].href,
		active: path === ACCOUNT_SECTIONS[id].href
	});

	const tabs = $derived<AccountTab[]>([
		{ label: 'Home', href: home, active: path === '/', tooltip: 'Your companies', icon: House }
	]);
	const groups = $derived<AccountGroup[]>([
		{
			label: 'Companies',
			/* With an account service a company is made on its Home; locally, Home makes it. */
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
					href: ACCOUNT_SECTIONS.accounts.href,
					active: path.startsWith('/account/settings'),
					tooltip: 'Accounts, computers and appearance',
					icon: Settings
				}
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
		...sectionIds.map((id) => ({
			id,
			group: 'Settings',
			label: ACCOUNT_SECTIONS[id].title,
			href: ACCOUNT_SECTIONS[id].href
		}))
	]);
	/* Settings' own list, shown in the page beside the section. */
	const settingsNav = $derived<AccountGroup[]>([
		{ label: 'Account', items: [own('appearance')] },
		{ label: 'Integrations', items: [own('accounts'), own('computers')] },
		...(sectionIds.includes('this-computer')
			? [{ label: 'This computer', items: [own('this-computer')] }]
			: [])
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
			<a href={ACCOUNT_SECTIONS.accounts.href}>Settings</a>
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
