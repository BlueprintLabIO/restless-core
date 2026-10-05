<script lang="ts">
	/* The account area every non-company page shares: the left sidebar with the
	 * companies and account settings, then the page. Locally the account is this
	 * computer, so its foot says which appliance it is rather than an email. */
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import AccountShell from '$lib/ui/views/AccountShell.svelte';
	import type { AccountNavGroup } from '$lib/ui/account';
	import { getApplianceStatus, type ApplianceStatus } from '$lib/model/appliance';
	import Building2 from '@lucide/svelte/icons/building-2';
	import Palette from '@lucide/svelte/icons/palette';
	import Plug from '@lucide/svelte/icons/plug';
	import Bot from '@lucide/svelte/icons/bot';

	let { appliance = null, children }: { appliance?: ApplianceStatus | null; children: Snippet } =
		$props();
	let observed = $state<ApplianceStatus | null>(null);
	const status = $derived(appliance ?? observed);
	const profile = $derived(
		status?.profile === 'dev'
			? 'Development profile'
			: status?.profile === 'test'
				? 'Test profile'
				: 'Local appliance'
	);

	$effect(() => {
		if (appliance) return;
		const controller = new AbortController();
		void getApplianceStatus(controller.signal)
			.then((value) => (observed = value))
			.catch(() => {
				// The sidebar still works without the profile name.
			});
		return () => controller.abort();
	});

	const path = $derived(page.url.pathname);
	const nav = $derived<AccountNavGroup[]>([
		{ items: [{ label: 'Companies', href: '/', icon: Building2, active: path === '/' }] },
		{
			label: 'Settings',
			items: [
				{
					label: 'Connections',
					href: '/account/settings/connections',
					icon: Plug,
					tooltip: 'Model sign-ins and connections shared across your companies',
					active: path === '/account/settings/connections'
				},
				{
					label: 'AI apps',
					href: '/account/settings/ai-apps',
					icon: Bot,
					tooltip: 'Use Restless from Claude Code, Claude Desktop or Codex',
					active: path === '/account/settings/ai-apps'
				},
				{
					label: 'Appearance',
					href: '/account/settings/appearance',
					icon: Palette,
					active: path === '/account/settings/appearance'
				}
			]
		}
	]);
</script>

<AccountShell
	brandName={PRODUCT_NAME}
	homeHref="/"
	{nav}
	account={{ name: 'You', detail: `${profile} · this computer` }}
>
	{@render children()}
</AccountShell>
