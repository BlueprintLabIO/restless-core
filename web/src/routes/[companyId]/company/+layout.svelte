<script lang="ts">
	import { goto } from '$app/navigation';
	import { dismissable } from '$lib/actions/dismissable';
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
	import { companyPrincipalQuery, companiesQuery, companyQuery } from '$lib/model/queries.svelte';
	import { COMPANY_PAGES, companyPageHref } from '$lib/model/company-pages';

	let { children } = $props();
	const companyId = $derived(page.params.companyId ?? 'aris');
	const computerSurface = $derived(page.url.pathname === `/${companyId}/company/computer`);
	const catalog = companiesQuery();
	const diagnostics = $derived(companyQuery(companyId));
	$effect(() => {
		if (principal?.membership_role === 'owner') diagnostics.attach();
	});
	const setupIssue = $derived(
		catalog.view.find((company) => company.id === companyId)?.unstartable_reason
	);
	const unhealthy = $derived(
		setupIssue || (diagnostics.view && diagnostics.view.computer.doctor.status !== 'healthy')
	);
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
	function settingsKeyboard(event: KeyboardEvent) {
		if (
			(event.target as HTMLElement)?.closest(
				'input, textarea, select, [contenteditable], #bridge-exrail'
			) ||
			event.defaultPrevented ||
			document.querySelector('dialog[open]')
		)
			return;
		if (event.altKey && event.shiftKey && ['ArrowUp', 'ArrowDown'].includes(event.key)) {
			event.preventDefault();
			const index = routes.findIndex(active);
			const step = event.key === 'ArrowDown' ? 1 : -1;
			const route = routes[(index + step + routes.length) % routes.length];
			if (route) void goto(route.href);
		}
		if (event.key === 'e' && !event.metaKey && !event.ctrlKey && !event.altKey) {
			const button = [...document.querySelectorAll<HTMLButtonElement>('.company-page button')].find(
				(button) =>
					/^(Edit( charter| identity)?|Write identity)$/.test(button.textContent?.trim() ?? '')
			);
			if (button) {
				event.preventDefault();
				button.click();
			}
		}
	}
</script>

<svelte:window onkeydown={settingsKeyboard} />

{#if computerSurface && page.url.searchParams.get('focus') === 'desktop'}
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
			<details class="company-mobile-nav" use:dismissable>
				<summary
					>{routes.find((route) => active(route))?.label ?? 'Company'}
					<span aria-hidden="true">⌄</span></summary
				>
				<div>
					{#each routes as route}<a
							href={route.href}
							aria-current={active(route) ? 'page' : undefined}>{route.label}</a
						>{/each}
				</div>
			</details>
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
						<span>{route.label}</span
						>{#if (route.key === 'provider' && setupIssue) || (route.key === 'doctor' && unhealthy)}<span
								class="company-nav-alert"
								title="Needs attention"
								aria-label="Needs attention"
							></span>{/if}
					</a>
				{/each}
			</nav>
			<span
				class="company-keyboard-hint"
				title="Move between pages with Alt + Shift + ↑ / ↓. Press E to edit the charter or identity."
				>Alt ⇧ ↑ / ↓ <span aria-hidden="true">·</span> E edit</span
			>
		</aside>
		<section class="company-canvas">{@render children()}</section>
	</div>
{/if}

<style>
	.company-keyboard-hint {
		display: block;
		margin: auto 12px 12px;
		padding-top: 16px;
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}
	@media (max-width: 640px) {
		.company-keyboard-hint {
			display: none;
		}
	}
	.company-nav-alert {
		position: absolute;
		right: 8px;
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--state-danger);
	}
	@media (max-width: 640px) {
		.company-mobile-nav {
			position: relative;
		}
		.company-mobile-nav summary {
			display: flex;
			justify-content: space-between;
			gap: 16px;
			padding: 8px 12px;
			width: 100%;
			min-height: 36px;
			font-size: var(--t-body);
			color: var(--ink);
			cursor: pointer;
			list-style: none;
		}
		.company-mobile-nav summary::-webkit-details-marker {
			display: none;
		}
		.company-mobile-nav summary::before {
			content: none !important;
		}
		.company-mobile-nav > div {
			position: absolute;
			top: 100%;
			left: 0;
			z-index: 30;
			width: 100%;
			padding: 6px;
			border: 1px solid var(--border);
			background: var(--surface-raised);
			border-radius: 6px;
			box-shadow: var(--shadow-soft);
		}
		.company-mobile-nav a {
			display: block;
			padding: 10px;
			color: var(--text-secondary);
			text-decoration: none;
		}
		.company-mobile-nav a[aria-current] {
			background: var(--surface-alt);
			color: var(--ink);
		}
		:global(.bridge-root .company-spine-head) {
			display: none;
		}
		:global(.bridge-root .company-area .company-spine) {
			padding: 4px;
			min-height: 44px;
			height: auto;
			overflow: visible;
		}
	}

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
		:global(.bridge-root .company-area .company-spine nav) {
			display: none;
		}
	}
</style>
