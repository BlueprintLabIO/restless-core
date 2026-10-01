<script lang="ts">
	import { goto } from '$app/navigation';
	import { resizePane } from '$lib/actions/resize-pane';
	import { page } from '$app/state';
	import Activity from '@lucide/svelte/icons/activity';
	import CalendarClock from '@lucide/svelte/icons/calendar-clock';
	import Settings from '@lucide/svelte/icons/settings';
	import BookOpen from '@lucide/svelte/icons/book-open';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import ListChecks from '@lucide/svelte/icons/list-checks';
	import Monitor from '@lucide/svelte/icons/monitor';
	import RadioTower from '@lucide/svelte/icons/radio-tower';
	import ShieldCheck from '@lucide/svelte/icons/shield-check';
	import Fingerprint from '@lucide/svelte/icons/fingerprint';
	import Users from '@lucide/svelte/icons/users';
	import Sparkles from '@lucide/svelte/icons/sparkles';
	import { companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { COMPANY_PAGES, companyPageHref } from '$lib/model/company-pages';

	let { children } = $props();
	const companyId = $derived(page.params.companyId ?? 'aris');
	const computerSurface = $derived(page.url.pathname === `/${companyId}/company/computer`);
	const principal = $derived(companyPrincipalQuery(companyId).view);
	const icons = {
		charter: BookOpen,
		identity: Fingerprint,
		members: Users,
		skills: Sparkles,
		provider: Settings,
		vault: KeyRound,
		schedules: CalendarClock,
		resources: ShieldCheck,
		computer: Monitor,
		doctor: Activity,
		decisions: ListChecks,
		actions: RadioTower
	} as const;
	const allRoutes = $derived(
		COMPANY_PAGES.map((route) => ({
			...route,
			href: companyPageHref(companyId, route),
			icon: icons[route.key as keyof typeof icons]
		}))
	);
	// Administrators see only the access they manage; the rest is the owner's.
	const routes = $derived(
		principal?.membership_role === 'owner'
			? allRoutes
			: allRoutes.filter((route) => route.label === 'Members')
	);

	function active(route: { href: string; exact?: boolean }): boolean {
		return route.exact
			? page.url.pathname === route.href
			: page.url.pathname.startsWith(route.href);
	}
</script>

{#if computerSurface}
	<div class="company-focus-shell">{@render children()}</div>
{:else}
	<div
		class="company-area"
		use:resizePane={{
			key: `${companyId}:company`,
			label: 'Resize company panes',
			target: '.company-spine',
			variable: '--company-index-w',
			min: 170,
			minOther: 280,
			defaultSize: 230,
			enabled: true
		}}
	>
		<aside class="company-spine">
			<div class="company-spine-head">
				<h2>Company</h2>
			</div>
			<label class="company-mobile-nav"
				><span class="sr-only">Company page</span>
				<select
					aria-label="Company page"
					value={page.url.pathname}
					onchange={(event) => void goto(event.currentTarget.value)}
				>
					{#each routes as route}<option value={route.href}>{route.label}</option>{/each}
				</select>
			</label>
			<nav aria-label="Company">
				{#each routes as route, index (route.href)}
					{@const RouteIcon = route.icon}
					{#if index === 0 || routes[index - 1].section !== route.section}<div
							class="company-nav-group"
						>
							{route.section}
						</div>{/if}
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
		</aside>
		<section class="company-canvas">{@render children()}</section>
	</div>
{/if}

<style>
	.company-mobile-nav {
		display: none;
	}
	:global(.company-spine nav .company-nav-group) {
		padding: var(--space-3) var(--space-3) var(--space-1);
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 600;
		letter-spacing: 0.02em;
	}
	:global(.company-spine nav .company-nav-group:not(:first-child)) {
		margin-top: var(--space-2);
		border-top: 1px solid var(--border);
	}
	@media (max-width: 640px) {
		.company-mobile-nav {
			display: flex;
			align-items: center;
			gap: var(--space-3);
			width: 100%;
			font-size: var(--t-label);
			color: var(--text-secondary);
		}
		.company-mobile-nav select {
			flex: 1;
			min-width: 0;
			min-height: 44px;
			padding: var(--space-2) var(--space-3);
			font: 600 var(--t-body) var(--font-ui);
			color: var(--ink);
			background: var(--surface);
			border: 1px solid var(--border-strong);
			border-radius: var(--radius-control);
		}
		:global(.bridge-root .company-area .company-spine nav) {
			display: none;
		}
	}
</style>
