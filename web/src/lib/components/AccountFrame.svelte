<script lang="ts">
	/* The frame every non-company page shares: the account bar (your companies, your account) and
	 * the page. Locally the account is this computer, so the menu says which appliance it is
	 * rather than an email. */
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import AccountShell from '$lib/ui/views/AccountShell.svelte';
	import type { AccountTab } from '$lib/ui/account';
	import { getApplianceStatus, planeLabel, type ApplianceStatus } from '$lib/model/appliance';

	let { appliance = null, children }: { appliance?: ApplianceStatus | null; children: Snippet } =
		$props();
	let observed = $state<ApplianceStatus | null>(null);
	const status = $derived(appliance ?? observed);
	const profile = $derived(planeLabel(status));
	/* On a hosted plane, Home (the one company list) is on the account issuer. */
	const home = $derived(status?.home_url ?? '/');

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
	const tabs = $derived<AccountTab[]>([
		{ label: 'Companies', href: home, active: path === '/' },
		{
			label: 'Account',
			href: '/account',
			active: path === '/account',
			tooltip: 'Connections, AI apps and appearance, shared across your companies'
		}
	]);
</script>

<AccountShell
	brandName={PRODUCT_NAME}
	homeHref={home}
	{tabs}
	account={{ name: 'You', detail: status?.hosted ? profile : `${profile} · this computer` }}
>
	{#snippet accountMenu()}
		<a href="/account#connections">Connections</a>
		<a href="/account#ai-apps">AI apps</a>
		<a href="/account#appearance">Appearance</a>
	{/snippet}
	{@render children()}
</AccountShell>
