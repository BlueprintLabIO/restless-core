<script module lang="ts">
	export type ShellTab = {
		key: string;
		label: string;
		badge?: number;
		/** Something on this surface needs the owner: a dot, never a count. */
		dot?: string;
		href: string;
		on?: boolean;
	};
</script>

<script lang="ts">
	import { tooltips } from '$lib/actions/tooltips';
	import { selectMenu } from '$lib/actions/select-menu';
	import { theme } from '$lib/theme.svelte';
	import ProfileMenu from './ProfileMenu.svelte';
	import ShortcutsDialog from './ShortcutsDialog.svelte';
	import type { PresentPerson } from '$lib/model/presence.svelte';
	/* Bridge Light has one company shell. Owners receive the four owner surfaces
	 * and bounded Exec control; collaborators receive only Work and People. The
	 * executive transcript remains a persistent sibling of the owner workspace,
	 * never a collaboration control inferred from company membership. */

	import { tick, type Snippet } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { dismissable } from '$lib/actions/dismissable';
	import { resizePane } from '$lib/actions/resize-pane';
	import Check from '@lucide/svelte/icons/check';
	import Plus from '@lucide/svelte/icons/plus';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import SearchIcon from '@lucide/svelte/icons/search';
	import CommandMenu, {
		type Command,
		type CommandSearch
	} from '$lib/components/CommandMenu.svelte';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import type { CompanyCatalogEntry } from '$lib/model/cockpit';
	import MatrixGlyph, { GLYPHS } from '$lib/ui/glyph/MatrixGlyph.svelte';
	import CompanyMark from '$lib/ui/glyph/CompanyMark.svelte';

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
		execUnavailable = false,
		execUnread = false,
		execWorking = false,
		blocked = false,
		railOpen = true,
		immersive = false,
		onexectoggle = null,
		commands = [],
		recents = [],
		search = null,
		ask = null,
		rail = null,
		viewerName = '',
		viewerRole = '',
		present = [],
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
		execUnavailable?: boolean;
		/** Exec wrote something the viewer has not read. */
		execUnread?: boolean;
		/** Exec is working on a turn right now. */
		execWorking?: boolean;
		/** Temporarily removes the underlying page from interaction while a blocking recovery prompt is open. */
		blocked?: boolean;
		railOpen?: boolean;
		/** Gives a prepared live outcome the full browser window while preserving one bounded Exec control. */
		immersive?: boolean;
		/** The one control for the rail: presence lamp and open/close in a single stable button. */
		onexectoggle?: (() => void) | null;
		/** Destinations and actions for the command menu (⌘K). */
		commands?: Command[];
		/** What the person opened lately, shown in the command menu before anything is typed. */
		recents?: Command[];
		/** What the command menu finds beyond its commands, such as text inside documents. */
		search?: CommandSearch | null;
		/** Hands what was typed in the command menu to Exec, when nothing else answers it. */
		ask?: ((query: string) => void) | null;
		/** The signed-in person, for the profile circle. */
		viewerName?: string;
		viewerRole?: string;
		/** Other people in this company's cockpit right now. */
		present?: PresentPerson[];
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
	/* Switching keeps the area: from Company › Limits to the other company's Limits. Only the
	 * surface (and a Company settings page) carries over; a Work item or a person does not. */
	const switchSuffix = $derived.by(() => {
		const parts = page.url.pathname.split('/').filter(Boolean);
		if (parts[0] !== companyId || parts.length < 2) return '';
		return `/${parts.slice(1, parts[1] === 'company' ? 3 : 2).join('/')}`;
	});
	let switcherQuery = $state('');
	const switchable = $derived.by(() => {
		const q = switcherQuery.trim().toLocaleLowerCase();
		return q
			? activeCompanies.filter((company) => company.name.toLocaleLowerCase().includes(q))
			: activeCompanies;
	});
	/* The bar names the company; an uploaded logo sits beside the name, a generated mark does not. */
	const currentLogo = $derived(
		companies.find((company) => company.id === companyId)?.logo_url ?? null
	);
	const tabGlyphs: Record<string, readonly string[]> = {
		attention: GLYPHS.alert,
		work: GLYPHS.briefcase,
		library: GLYPHS.rules,
		people: GLYPHS.group,
		apps: GLYPHS.apps,
		company: GLYPHS.key
	};
	const RUNTIME_LABEL: Record<string, string> = {
		running: 'Awake',
		asleep: 'Asleep',
		stopped: 'Stopped',
		absent: 'Not started',
		unavailable: 'Unavailable'
	};
	/* Linear-style two-key moves: G then the surface's initial. */
	const tabKeys: Record<string, string> = {
		attention: 'i',
		work: 'w',
		library: 'l',
		people: 'p',
		apps: 'a',
		company: 'c'
	};
	const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform);
	const execShortcut = isMac ? '⌘J' : 'Ctrl+J';
	/* The button says what the closed rail would: a reply waiting, or Exec at work. */
	const execLabel = $derived(
		[execName, execUnread ? 'new reply' : '', execWorking ? 'working' : '']
			.filter(Boolean)
			.join(', ')
	);
	const menuShortcut = isMac ? '⌘K' : 'Ctrl+K';
	let commandMenuOpen = $state(false);
	let shortcuts: ShortcutsDialog | undefined = $state();
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
		...commands,
		...(['system', 'light', 'dark'] as const).map((choice) => ({
			id: `appearance:${choice}`,
			group: 'Appearance',
			label: { system: 'Match system appearance', light: 'Light', dark: 'Dark' }[choice],
			hint: theme.preference === choice ? 'Current' : undefined,
			keywords: `theme appearance mode${choice === 'system' ? ' auto os device' : ''}`,
			run: () => theme.set(choice)
		})),
		{
			id: 'help:shortcuts',
			group: 'Help',
			label: 'Keyboard shortcuts',
			hint: '?',
			run: () => shortcuts?.open()
		}
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
			rail &&
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

<div class="bridge-root" class:immersive inert={blocked} use:tooltips use:selectMenu>
	<a class="skip-link" href="#main-content">Skip to content</a>
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
						{#if currentLogo}<span class="tb-co-mark" aria-hidden="true"
								><CompanyMark id={companyId} logoUrl={currentLogo} size={8} /></span
							>{/if}
						<span class="tb-co">{companyName}</span>
						<ChevronDown class="company-chevron" size={14} strokeWidth={2} aria-hidden="true" />
					</summary>
					<div class="company-switcher-menu">
						<a class="company-overview-link" href={homeHref}>
							<MatrixGlyph rows={GLYPHS.r} size={8} />
							<span><strong>All companies</strong></span>
						</a>
						<div class="company-switcher-rule" role="separator"></div>
						{#if activeCompanies.length > 6}<input
								class="switcher-search"
								type="search"
								placeholder="Find a company"
								aria-label="Find a company"
								bind:value={switcherQuery}
							/>{/if}
						{#each switchable as company (company.id)}
							<a
								class:current={company.id === companyId}
								href={`/${company.id}${company.id === companyId ? '' : switchSuffix}`}
								aria-current={company.id === companyId ? 'page' : undefined}
							>
								<span class="switcher-mark" class:logo={!!company.logo_url} aria-hidden="true"
									><CompanyMark id={company.id} logoUrl={company.logo_url} size={8} /><i
										class="runtime-{company.unstartable_reason
											? 'blocked'
											: company.runtime_status}"
									></i></span
								>
								<span
									><strong>{company.name}</strong><small
										>{company.unstartable_reason
											? 'Can’t start'
											: (RUNTIME_LABEL[company.runtime_status] ?? company.runtime_status)}{company
											.card?.decisions_waiting
											? ` · ${company.card.decisions_waiting} waiting`
											: ''}</small
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
						<div class="company-switcher-rule" role="separator"></div>
						<a
							class="company-overview-link"
							href={homeHref + (homeHref.includes('?') ? '&' : '?') + 'start=1'}
						>
							<Plus size={14} strokeWidth={2} aria-hidden="true" />
							<span><strong>Start a company</strong></span>
						</a>
					</div>
				</details>
			{:else}
				<span
					class="company-switcher company-switcher-static"
					aria-label={`Current company: ${companyName}`}
				>
					{#if currentLogo}<span class="tb-co-mark" aria-hidden="true"
							><CompanyMark id={companyId} logoUrl={currentLogo} size={8} /></span
						>{/if}
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
			<!-- The avatar keeps its place before the name is known, and shows the right letter
			     once, rather than a placeholder letter that changes. -->
			{#if viewerName}<ProfileMenu
					name={viewerName}
					role={viewerRole}
					{present}
					owner={canSwitchCompanies}
					{companyId}
					onshortcuts={() => shortcuts?.open()}
				/>{:else}<span class="tb-profile-slot" aria-hidden="true"></span>{/if}
			{#if execHref}
				<a
					class="tb-exec"
					class:live={execLive}
					class:unavailable={execUnavailable}
					class:working={execWorking}
					href={execHref}
					aria-label={execLabel}
					title={execLabel}
				>
					<span class="tb-exec-lamp" aria-hidden="true"></span><span class="tb-exec-name"
						>{execName}</span
					>{#if execUnread}<span class="tb-dot" aria-hidden="true"></span>{/if}
				</a>
			{:else if rail}
				<button
					class="tb-exec"
					class:live={execLive}
					class:unavailable={execUnavailable}
					class:on={railOpen}
					class:working={execWorking}
					type="button"
					aria-expanded={railOpen}
					aria-keyshortcuts={isMac ? 'Meta+J' : 'Control+J'}
					aria-label={execLabel}
					title={`${railOpen ? 'Hide' : 'Show'} ${execLabel} · ${execShortcut}`}
					onclick={() => onexectoggle?.()}
				>
					<span class="tb-exec-lamp" aria-hidden="true"></span><span class="tb-exec-name"
						>{execName}</span
					>{#if execUnread}<span class="tb-dot" aria-hidden="true"></span>{/if}
				</button>
			{/if}
		</div>
	</header>

	<ShortcutsDialog bind:this={shortcuts} />

	{#snippet surfaceTab(tab: ShellTab)}
		<a
			class="tb-tab"
			class:on={tab.on}
			data-surface={tab.key}
			href={tab.href}
			aria-current={tab.on ? 'page' : undefined}
			aria-label={tab.badge
				? `${tab.label}, ${tab.badge} ${tab.badge === 1 ? 'item' : 'items'}`
				: tab.dot
					? `${tab.label}, ${tab.dot}`
					: tab.label}
			title={tab.dot
				? `${tab.label}: ${tab.dot}`
				: tabKeys[tab.key]
					? `${tab.label} · G then ${tabKeys[tab.key].toUpperCase()}`
					: tab.label}
		>
			<span class="tb-tab-mark" aria-hidden="true">
				<MatrixGlyph rows={tabGlyphs[tab.key] ?? GLYPHS.square} size={12} />
			</span>
			<span class="tb-tab-label" aria-hidden="true">{tab.label}</span>
			{#if tab.badge}<span class="tb-badge" aria-hidden="true">{tab.badge}</span>
			{:else if tab.dot}<span class="tb-dot" aria-hidden="true"></span>{/if}
		</a>
	{/snippet}

	<CommandMenu bind:open={commandMenuOpen} commands={menuCommands} {recents} {search} {ask} />

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
			class:unread={execUnread}
			class:live={execLive}
			class:unavailable={execUnavailable}
			class:on={railOpen}
			type="button"
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
			defaultSize: 380,
			enabled: !!rail && railOpen && !immersive
		}}
	>
		<div class="bridge-workspace">
			<main class="bridge-content" id="main-content" tabindex="-1">{@render children()}</main>
		</div>
		{#if rail}{@render rail()}{/if}
	</div>
</div>
