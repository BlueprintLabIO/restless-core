<script lang="ts">
	import { page } from '$app/state';
	import Activity from '@lucide/svelte/icons/activity';
	import BookOpen from '@lucide/svelte/icons/book-open';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import ListChecks from '@lucide/svelte/icons/list-checks';
	import Monitor from '@lucide/svelte/icons/monitor';
	import RadioTower from '@lucide/svelte/icons/radio-tower';
	import ShieldCheck from '@lucide/svelte/icons/shield-check';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import { companyQuery } from '$lib/model/queries.svelte';

	let { children } = $props();
	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(companyQuery(companyId));
	$effect(() => source.attach());
	const computerSurface = $derived(page.url.pathname === `/${companyId}/company/computer`);
	const routes = $derived([
		{ label: 'Company charter', href: `/${companyId}/company`, exact: true, icon: BookOpen },
		{ label: 'Decision history', href: `/${companyId}/company/decisions`, icon: ListChecks },
		{
			label: 'Authority & limits',
			href: `/${companyId}/company/authority`,
			icon: ShieldCheck
		},
		{ label: 'Resources & access', href: `/${companyId}/company/resources`, icon: KeyRound },
		{ label: 'External actions', href: `/${companyId}/company/actions`, icon: RadioTower },
		{ label: 'Company computer', href: `/${companyId}/company/computer`, icon: Monitor },
		{ label: 'Company doctor', href: `/${companyId}/company/doctor`, icon: Activity }
	]);

	function active(route: { href: string; exact?: boolean }): boolean {
		return route.exact
			? page.url.pathname === route.href
			: page.url.pathname.startsWith(route.href);
	}
</script>

{#if computerSurface}
	<div class="company-focus-shell">{@render children()}</div>
{:else}
	<div class="company-area">
		<aside class="company-spine">
			<div class="company-spine-head">
				<h2>Company</h2>
				<InfoTip
					text="Durable owner concepts stay in the same order even when a company's tools and providers differ."
				/>
			</div>
			<nav aria-label="Company">
				{#each routes as route (route.href)}
					{@const RouteIcon = route.icon}
					<a
						class:active={active(route)}
						href={route.href}
						title={route.label}
						aria-current={active(route) ? 'page' : undefined}
					>
						<i aria-hidden="true"><RouteIcon size={15} strokeWidth={1.8} /></i>
						<span>{route.label}</span>
					</a>
				{/each}
			</nav>
			<div class="company-source-summary" class:stale={source.status === 'stale'}>
				<span class="source-lamp status-{source.status}" aria-hidden="true"></span>
				<span
					>{source.status === 'live'
						? 'Live sources'
						: source.status === 'stale'
							? 'Last observation'
							: 'Reading sources'}</span
				>
				<InfoTip
					text="Company pages preserve the last observation when a source drops and label it stale rather than replacing it with an empty list."
				/>
			</div>
		</aside>
		<section class="company-canvas">{@render children()}</section>
	</div>
{/if}
