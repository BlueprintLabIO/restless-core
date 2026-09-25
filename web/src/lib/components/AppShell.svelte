<script module lang="ts">
	export type ShellTab = {
		key: string;
		label: string;
		badge?: number;
		href: string;
		on?: boolean;
	};
</script>

<script lang="ts">
	/* Bridge Light has one company shell. Owners receive the four owner surfaces
	 * and bounded Exec control; collaborators receive only Work and People. The
	 * executive transcript remains a persistent sibling of the owner workspace,
	 * never a collaboration control inferred from company membership. */

	import { tick, type Snippet } from 'svelte';
	import { goto } from '$app/navigation';
	import { dismissable } from '$lib/actions/dismissable';
	import { resizePane } from '$lib/actions/resize-pane';
	import Check from '@lucide/svelte/icons/check';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import SearchIcon from '@lucide/svelte/icons/search';
	import CommandMenu, { type Command } from '$lib/components/CommandMenu.svelte';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import type { CompanyCatalogEntry } from '$lib/model/cockpit';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';

	let {
		companyId,
		companyName,
		companies = [],
		tabs,
		homeHref = '/',
		canSwitchCompanies = true,
		execName = 'Exec',
		execHref = null,
		execLive = false,
		blocked = false,
		expandExec = false,
		railOpen = true,
		immersive = false,
		onexectoggle = null,
		commands = [],
		rail = null,
		children
	}: {
		companyId: string;
		companyName: string;
		companies?: CompanyCatalogEntry[];
		tabs: ShellTab[];
		/** Owner portfolio for owners; the current collaboration home for members. */
		homeHref?: string;
		/** Company discovery and lifecycle status are owner-only account-plane projections. */
		canSwitchCompanies?: boolean;
		execName?: string;
		execHref?: string | null;
		execLive?: boolean;
		/** Temporarily removes the underlying page from interaction while a blocking recovery prompt is open. */
		blocked?: boolean;
		expandExec?: boolean;
		railOpen?: boolean;
		/** Gives a prepared live outcome the full browser window while preserving one bounded Exec control. */
		immersive?: boolean;
		/** The one control for the rail: presence lamp and open/close in a single stable button. */
		onexectoggle?: (() => void) | null;
		/** Destinations and actions for the command menu (⌘K). */
		commands?: Command[];
		/**
		 * The persistent executive transcript. Omitted on surfaces that already
		 * hold a conversation with a specific actor — People carries its own, and
		 * a second permanent conversation with a different actor competes with it
		 * rather than supporting it (S06-T2).
		 */
		rail?: Snippet | null;
		children: Snippet;
	} = $props();

	const activeCompanies = $derived(
		companies.filter((company) => company.lifecycle_status === 'active')
	);
	const tabGlyphs: Record<string, readonly string[]> = {
		attention: GLYPHS.alert,
		work: GLYPHS.briefcase,
		people: GLYPHS.group,
		company: GLYPHS.key
	};
	const RUNTIME_LABEL: Record<string, string> = {
		running: 'Running',
		stopped: 'Asleep',
		absent: 'Not started',
		unavailable: 'Unavailable'
	};
	/* Linear-style two-key moves: G then the surface's initial. */
	const tabKeys: Record<string, string> = { attention: 'a', work: 'w', people: 'p', company: 'c' };
	const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform);
	const execShortcut = isMac ? '⌘J' : 'Ctrl+J';
	const menuShortcut = isMac ? '⌘K' : 'Ctrl+K';
	let commandMenuOpen = $state(false);
	const menuCommands = $derived<Command[]>([
		...tabs.map((tab) => ({
			id: `surface:${tab.key}`,
			group: 'Go to',
			label: tab.label,
			hint: tabKeys[tab.key] ? `G then ${tabKeys[tab.key].toUpperCase()}` : undefined,
			shortcut: true,
			href: tab.href
		})),
		...(rail && onexectoggle
			? [
					{
						id: 'exec:toggle',
						group: 'Go to',
						label: railOpen ? `Hide ${execName}` : `Talk to ${execName}`,
						hint: execShortcut,
						shortcut: true,
						run: () => onexectoggle?.()
					}
				]
			: []),
		...commands
	]);

	let tabNav: HTMLElement | undefined = $state();
	let indicator = $state<{ x: number; w: number; tone: string } | null>(null);
	let indicatorPlaced = $state(false);

	function placeIndicator(): boolean {
		const active = tabNav?.querySelector<HTMLElement>('.tb-tab.on');
		if (!active) {
			indicator = null;
			return false;
		}
		indicator = {
			x: active.offsetLeft,
			w: active.offsetWidth,
			tone: getComputedStyle(active).getPropertyValue('--tab-tone')
		};
		return true;
	}

	$effect(() => {
		void tabs.map((tab) => `${tab.key}:${tab.on}:${tab.badge ?? 0}`).join();
		/* The first placement lands without travel; only later moves animate. */
		if (placeIndicator()) requestAnimationFrame(() => (indicatorPlaced = true));
	});

	$effect(() => {
		if (!tabNav) return;
		const observer = new ResizeObserver(() => placeIndicator());
		observer.observe(tabNav);
		return () => observer.disconnect();
	});

	function focusRail() {
		const rail = document.getElementById('bridge-exrail');
		const target =
			rail?.querySelector<HTMLElement>('textarea:not([disabled])') ??
			rail?.querySelector<HTMLElement>('a[href], button:not([disabled])');
		target?.focus();
	}

	function typing(target: EventTarget | null) {
		const element = target as HTMLElement | null;
		return !!element?.closest(
			'input, textarea, select, [contenteditable=""], [contenteditable="true"]'
		);
	}

	let awaitingSurfaceKey = false;
	let surfaceKeyTimer: number | undefined;
	function onKeydown(event: KeyboardEvent) {
		if (event.defaultPrevented || blocked) return;
		if ((event.metaKey || event.ctrlKey) && !event.altKey && event.key.toLowerCase() === 'k') {
			event.preventDefault();
			commandMenuOpen = !commandMenuOpen;
			return;
		}
		if (commandMenuOpen) return;
		if ((event.metaKey || event.ctrlKey) && !event.altKey && event.key.toLowerCase() === 'j') {
			if (!rail || !onexectoggle) return;
			event.preventDefault();
			const opening = !railOpen;
			onexectoggle();
			/* A keyboard open should land the keyboard in the rail. */
			if (opening) void tick().then(() => focusRail());
			return;
		}
		if (
			event.key === 'Escape' &&
			railOpen &&
			onexectoggle &&
			(event.target as Element | null)?.closest?.('#bridge-exrail') &&
			!(event.target as Element).closest('dialog, [role="dialog"], [role="listbox"]')
		) {
			/* Decide after dispatch: a search or popover inside the rail that
			 * consumed Escape (preventDefault) keeps the rail open. */
			window.setTimeout(() => {
				if (event.defaultPrevented || !railOpen) return;
				onexectoggle?.();
				document.querySelector<HTMLElement>('.tb-exec')?.focus();
			});
			return;
		}
		if (event.metaKey || event.ctrlKey || event.altKey || typing(event.target)) return;
		const key = event.key.toLowerCase();
		if (awaitingSurfaceKey) {
			awaitingSurfaceKey = false;
			window.clearTimeout(surfaceKeyTimer);
			const tab = tabs.find((candidate) => tabKeys[candidate.key] === key);
			if (tab) {
				event.preventDefault();
				void goto(tab.href);
			}
			return;
		}
		if (key === 'g') {
			awaitingSurfaceKey = true;
			surfaceKeyTimer = window.setTimeout(() => (awaitingSurfaceKey = false), 1200);
		}
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="bridge-root" class:immersive inert={blocked}>
	<header class="bridge-topbar" aria-label="Global navigation">
		<div class="tb-brand">
			<a class="tb-brand-home" href={homeHref} aria-label={`${PRODUCT_NAME} company home`}>
				<span class="tb-mark"><MatrixGlyph rows={GLYPHS.r} size={13} glow /></span>
				<span class="tb-name">{PRODUCT_NAME}</span>
			</a>
			<span class="tb-company-slash" aria-hidden="true">/</span>
			{#if canSwitchCompanies}
				<details class="company-switcher" use:dismissable>
					<summary aria-label={`Switch company. Current company: ${companyName}`}>
						<span class="tb-co">{companyName}</span>
						<ChevronDown class="company-chevron" size={14} strokeWidth={2} aria-hidden="true" />
					</summary>
					<div class="company-switcher-menu">
						<a class="company-overview-link" href="/">
							<MatrixGlyph rows={GLYPHS.r} size={8} />
							<span><strong>All companies</strong></span>
						</a>
						<div class="company-switcher-rule" role="separator"></div>
						{#each activeCompanies as company (company.id)}
							<a
								class:current={company.id === companyId}
								href={`/${company.id}`}
								aria-current={company.id === companyId ? 'page' : undefined}
							>
								<i
									class="runtime-{company.unstartable_reason ? 'blocked' : company.runtime_status}"
									aria-hidden="true"
								></i>
								<span
									><strong>{company.name}</strong><small
										>{company.unstartable_reason
											? 'Can’t start'
											: (RUNTIME_LABEL[company.runtime_status] ?? company.runtime_status)}</small
									></span
								>
								{#if company.id === companyId}<Check
										class="switcher-current"
										size={14}
										strokeWidth={2.2}
										aria-label="Current company"
									/>{/if}
							</a>
						{:else}
							<a class="current" href={`/${companyId}`} aria-current="page">
								<i aria-hidden="true"></i><span><strong>{companyName}</strong></span>
								<Check
									class="switcher-current"
									size={14}
									strokeWidth={2.2}
									aria-label="Current company"
								/>
							</a>
						{/each}
					</div>
				</details>
			{:else}
				<span
					class="company-switcher company-switcher-static"
					aria-label={`Current company: ${companyName}`}
				>
					<span class="tb-co">{companyName}</span>
				</span>
			{/if}
		</div>

		<nav class="tb-tabs" aria-label="Company navigation" bind:this={tabNav}>
			{#if indicator}
				<span
					class="tb-indicator"
					class:placed={indicatorPlaced}
					style:--tb-indicator-x={`${indicator.x}px`}
					style:--tb-indicator-w={`${indicator.w}px`}
					style:--tab-tone={indicator.tone}
					aria-hidden="true"
				></span>
			{/if}
			{#each tabs as tab (tab.key)}
				{@render surfaceTab(tab)}
			{/each}
		</nav>

		<div class="tb-right">
			<button
				class="tb-search"
				type="button"
				aria-label="Search and jump"
				aria-keyshortcuts={isMac ? 'Meta+K' : 'Control+K'}
				title={`Search and jump · ${menuShortcut}`}
				onclick={() => (commandMenuOpen = true)}
			>
				<SearchIcon size={15} strokeWidth={2} aria-hidden="true" />
				<span class="tb-search-label">Search</span>
				<kbd aria-hidden="true">{menuShortcut}</kbd>
			</button>
			{#if execHref}
				<a class="tb-exec" class:live={execLive} href={execHref}>
					<span class="tb-exec-lamp" aria-hidden="true"></span>{execName}
				</a>
			{:else if rail}
				<button
					class="tb-exec"
					class:live={execLive}
					class:on={railOpen}
					type="button"
					aria-controls="bridge-exrail"
					aria-expanded={railOpen}
					aria-keyshortcuts={isMac ? 'Meta+J' : 'Control+J'}
					title={`${railOpen ? 'Hide' : 'Show'} ${execName} · ${execShortcut}`}
					onclick={() => onexectoggle?.()}
				>
					<span class="tb-exec-lamp" aria-hidden="true"></span>{execName}
				</button>
			{/if}
		</div>
	</header>

	{#snippet surfaceTab(tab: ShellTab)}
		<a
			class="tb-tab"
			class:on={tab.on}
			data-surface={tab.key}
			href={tab.href}
			aria-current={tab.on ? 'page' : undefined}
			aria-label={tab.badge ? `${tab.label}, ${tab.badge} items` : tab.label}
			title={tabKeys[tab.key]
				? `${tab.label} · G then ${tabKeys[tab.key].toUpperCase()}`
				: tab.label}
		>
			<span class="tb-tab-mark" aria-hidden="true">
				<MatrixGlyph rows={tabGlyphs[tab.key] ?? GLYPHS.square} size={12} />
			</span>
			<span class="tb-tab-label" aria-hidden="true">{tab.label}</span>
			{#if tab.badge}<span class="tb-badge" aria-hidden="true">{tab.badge}</span>{/if}
		</a>
	{/snippet}

	<CommandMenu bind:open={commandMenuOpen} commands={menuCommands} />

	{#if !immersive}
		<nav class="bridge-dock" aria-label="Company navigation">
			{#each tabs as tab (tab.key)}
				{@render surfaceTab(tab)}
			{/each}
		</nav>
	{/if}

	{#if immersive && rail}
		<button
			class="immersive-exec"
			class:live={execLive}
			class:on={railOpen}
			type="button"
			aria-controls="bridge-exrail"
			aria-expanded={railOpen}
			title={railOpen ? `Close ${execName}` : `Open ${execName}`}
			onclick={() => onexectoggle?.()}
		>
			<MessageSquare size={13} strokeWidth={2} aria-hidden="true" />
			{railOpen ? `Close ${execName}` : execName}
		</button>
	{/if}

	<div
		class="bridge-body"
		use:resizePane={{
			key: `${companyId}:exec`,
			label: 'Resize Exec pane',
			target: '.bridge-exrail',
			variable: '--exec-rail-w',
			side: 'end',
			min: 320,
			minOther: 320,
			defaultSize: (width) => (expandExec ? width * 0.62 : 380),
			enabled: !!rail && railOpen && !immersive
		}}
	>
		<div class="bridge-workspace">
			<main class="bridge-content">{@render children()}</main>
		</div>
		{#if rail}{@render rail()}{/if}
	</div>
</div>
