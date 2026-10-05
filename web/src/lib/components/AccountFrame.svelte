<script lang="ts">
	/* The frame every non-company page shares: the account rail (Home, your Account) and the page.
	 * Locally the account is this computer, so the menu says which appliance it is rather than an
	 * email. On Cloud, Account also lists the sections the account service owns (Profile, Security,
	 * Plan, Support), so the rail is the same whichever page you are on (ADR 0007). */
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import AccountShell from '$lib/ui/views/AccountShell.svelte';
	import House from '@lucide/svelte/icons/house';
	import UserRound from '@lucide/svelte/icons/user-round';
	import type { AccountTab } from '$lib/ui/account';
	import { getApplianceStatus, planeLabel, type ApplianceStatus } from '$lib/model/appliance';

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

	const path = $derived(page.url.pathname);
	const own = (label: string, href: string): AccountTab => ({ label, href, active: path === href });
	const tabs = $derived<AccountTab[]>([
		{ label: 'Home', href: home, active: path === '/', tooltip: 'Your companies', icon: House },
		{
			label: 'Account',
			href: '/account/connections',
			active: path.startsWith('/account'),
			tooltip: issuer
				? 'Your profile, security, plan, support, connections, AI apps and appearance'
				: 'Connections, AI apps and appearance, shared across your companies',
			icon: UserRound,
			items: [
				...(issuer
					? [
							{ label: 'Profile', href: `${issuer}/account/settings#account` },
							{ label: 'Security', href: `${issuer}/account/settings#security` },
							{ label: 'Plan', href: `${issuer}/account/settings#billing` },
							{ label: 'Support', href: `${issuer}/account/settings#support` }
						]
					: []),
				own('Connections', '/account/connections'),
				own('AI apps', '/account/ai-apps'),
				own('Appearance', '/account/appearance')
			]
		}
	]);
</script>

<AccountShell
	brandName={PRODUCT_NAME}
	homeHref={home}
	{tabs}
	account={{
		name: status?.viewer_name || 'You',
		detail: status?.hosted ? profile : `${profile} · this computer`
	}}
>
	{#snippet accountMenu()}
		{#if issuer}<a href={`${issuer}/account/settings#account`}>Profile</a>{/if}
		<a href="/account/connections">Connections</a>
	{/snippet}
	{@render children()}
</AccountShell>
