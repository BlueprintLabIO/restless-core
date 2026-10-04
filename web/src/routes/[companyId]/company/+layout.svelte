<script lang="ts">
	/* Company is the settings area: a quiet navigation list and one content pane.
	 * Every page inside renders through the shared Page frame, so this layout owns
	 * no headers, widths or section styles of its own. */
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { dismissable } from '$lib/actions/dismissable';
	import Activity from '@lucide/svelte/icons/activity';
	import BookOpen from '@lucide/svelte/icons/book-open';
	import Brain from '@lucide/svelte/icons/brain';
	import CalendarClock from '@lucide/svelte/icons/calendar-clock';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import Fingerprint from '@lucide/svelte/icons/fingerprint';
	import Gauge from '@lucide/svelte/icons/gauge';
	import History from '@lucide/svelte/icons/history';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import Monitor from '@lucide/svelte/icons/monitor';
	import Sparkles from '@lucide/svelte/icons/sparkles';
	import Users from '@lucide/svelte/icons/users';
	import { companyPrincipalQuery, companiesQuery, companyQuery } from '$lib/model/queries.svelte';
	import { COMPANY_PAGES, companyPageHref } from '$lib/model/company-pages';

	let { children } = $props();
	const companyId = $derived(page.params.companyId ?? 'aris');
	const principal = $derived(companyPrincipalQuery(companyId).view);
	const owner = $derived(principal?.membership_role === 'owner');
	const catalog = companiesQuery();
	const diagnostics = $derived(companyQuery(companyId));
	$effect(() => {
		if (owner) diagnostics.attach();
	});
	const setupIssue = $derived(
		catalog.view.find((company) => company.id === companyId)?.unstartable_reason
	);
	const unhealthy = $derived(
		!!diagnostics.view && diagnostics.view.computer.doctor.status !== 'healthy'
	);
	const icons = {
		general: BookOpen,
		identity: Fingerprint,
		members: Users,
		provider: Brain,
		skills: Sparkles,
		vault: KeyRound,
		schedules: CalendarClock,
		limits: Gauge,
		computer: Monitor,
		activity: History,
		health: Activity
	} as const;
	const routes = $derived(
		COMPANY_PAGES.filter((route) => owner || route.key === 'members').map((route) => ({
			...route,
			href: companyPageHref(companyId, route),
			icon: icons[route.key as keyof typeof icons],
			alert: (route.key === 'provider' && !!setupIssue) || (route.key === 'health' && unhealthy)
		}))
	);
	const current = $derived(routes.find((route) => active(route)));
	const desktopFocus = $derived(
		page.url.pathname === `/${companyId}/company/computer` &&
			page.url.searchParams.get('focus') === 'desktop'
	);

	function active(route: { href: string; exact?: boolean }): boolean {
		return route.exact
			? page.url.pathname === route.href
			: page.url.pathname.startsWith(route.href);
	}

	/* Alt+Shift+↑/↓ steps through pages; E edits where a page offers it. */
	function keyboard(event: KeyboardEvent) {
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
			const next =
				routes[(index + (event.key === 'ArrowDown' ? 1 : -1) + routes.length) % routes.length];
			if (next) void goto(next.href);
		}
		if (event.key === 'e' && !event.metaKey && !event.ctrlKey && !event.altKey) {
			const edit = [...document.querySelectorAll<HTMLButtonElement>('.company-main button')].find(
				(button) => /^(Edit|Write charter|Write identity)$/.test(button.textContent?.trim() ?? '')
			);
			if (edit) {
				event.preventDefault();
				edit.click();
			}
		}
	}
</script>

<svelte:window onkeydown={keyboard} />

{#if desktopFocus}
	<div class="company-focus">{@render children()}</div>
{:else}
	<div class="company-shell">
		<nav class="company-nav" aria-label="Company settings">
			<details class="company-nav-mobile" use:dismissable>
				<summary
					><span>{current?.label ?? 'Company'}</span><ChevronDown
						size={14}
						strokeWidth={1.8}
						aria-hidden="true"
					/></summary
				>
				<div class="company-nav-menu">
					{#each routes as route (route.href)}<a
							href={route.href}
							aria-current={active(route) ? 'page' : undefined}>{route.label}</a
						>{/each}
				</div>
			</details>
			<div class="company-nav-list">
				{#each routes as route, index (route.href)}
					{@const Icon = route.icon}
					{#if index === 0 || routes[index - 1].group !== route.group}<span
							class="company-nav-group">{route.group}</span
						>{/if}
					<a
						href={route.href}
						class:active={active(route)}
						aria-current={active(route) ? 'page' : undefined}
						title={route.alert ? `${route.label}: needs attention` : undefined}
					>
						<Icon size={15} strokeWidth={1.8} aria-hidden="true" />
						<span>{route.label}</span>
						{#if route.alert}<i class="company-nav-alert" aria-label="Needs attention"></i>{/if}
					</a>
				{/each}
			</div>
		</nav>
		<main class="company-main">{@render children()}</main>
	</div>
{/if}

<style>
	.company-shell,
	.company-focus {
		display: flex;
		flex: 1 1 auto;
		gap: var(--pane-gap);
		width: 100%;
		min-width: 0;
		min-height: 0;
		overflow: hidden;
	}
	.company-nav,
	.company-main {
		min-height: 0;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		box-shadow: var(--bevel), var(--shadow-soft);
	}
	.company-nav {
		display: flex;
		flex-direction: column;
		flex: none;
		width: 220px;
		padding: 14px 8px 8px;
		overflow: auto;
		background: var(--surface-rail);
	}
	.company-main {
		display: flex;
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		background: var(--surface-pane);
	}
	.company-nav-list {
		display: grid;
		gap: 1px;
	}
	.company-nav-group {
		margin: 14px 8px 4px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 500;
	}
	.company-nav-group:first-child {
		margin-top: 2px;
	}
	.company-nav-list a {
		position: relative;
		display: flex;
		align-items: center;
		gap: 9px;
		min-height: 30px;
		padding: 0 8px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		font-size: var(--t-body);
		text-decoration: none;
		transition:
			background-color var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard);
	}
	.company-nav-list a :global(svg) {
		flex: none;
		color: var(--text-tertiary);
	}
	.company-nav-list a:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.company-nav-list a.active {
		background: var(--surface-raised);
		box-shadow: var(--control-depth);
		color: var(--ink);
		font-weight: 500;
	}
	.company-nav-list a.active :global(svg) {
		color: var(--ink);
	}
	.company-nav-alert {
		width: 6px;
		height: 6px;
		margin-left: auto;
		border-radius: 50%;
		background: var(--state-danger);
	}
	.company-nav-mobile {
		display: none;
	}
	@media (max-width: 760px) {
		.company-shell {
			flex-direction: column;
		}
		.company-nav {
			width: auto;
			padding: 4px;
			overflow: visible;
		}
		.company-nav-list {
			display: none;
		}
		.company-nav-mobile {
			display: block;
			position: relative;
		}
		.company-nav-mobile summary {
			display: flex;
			align-items: center;
			justify-content: space-between;
			min-height: 40px;
			padding: 0 12px;
			color: var(--ink);
			font-size: var(--t-body);
			font-weight: 500;
			cursor: pointer;
			list-style: none;
		}
		.company-nav-mobile summary::-webkit-details-marker {
			display: none;
		}
		.company-nav-mobile summary::before {
			content: none !important;
		}
		.company-nav-menu {
			position: absolute;
			top: calc(100% + 4px);
			left: 0;
			right: 0;
			z-index: 30;
			display: grid;
			padding: 6px;
			border: 1px solid var(--border-strong);
			border-radius: var(--radius-lg);
			background: var(--surface-raised);
			box-shadow: var(--shadow-lift);
		}
		.company-nav-menu a {
			padding: 11px 10px;
			border-radius: var(--radius-control);
			color: var(--text-secondary);
			text-decoration: none;
		}
		.company-nav-menu a[aria-current] {
			background: var(--surface-alt);
			color: var(--ink);
		}
	}
</style>
