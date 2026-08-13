<script module lang="ts">
	import { PRODUCT_NAME } from '$lib/brand/brand';
	export type ShellTab = {
		key: string;
		label: string;
		badge?: number;
		/** A real route — the tab renders as a link. */
		href?: string;
		/** Client-side state — the tab renders as a button. */
		onclick?: () => void;
		on?: boolean;
		disabled?: boolean;
	};
</script>

<script lang="ts">
	/* The one shell for every company surface: an instrument strip of a top
	 * navbar (Bridge: dark cockpit) with tabs, ONE brand dropdown on the left —
	 * the matrix mark opens company switching and the records (Library / Tape)
	 * in a single menu — and the Chief of Staff annunciator on the right.
	 * The executive rail renders as a flex sibling of the content — it takes
	 * real space and pushes the page aside instead of floating over it.
	 *
	 * The same shell frames the founding floor: a company that does not exist
	 * yet gets a static brand (nothing to switch to) and client-side tabs while
	 * its draft desk fills in — one chrome, live or forming. */

	import { page } from '$app/state';
	import type { Snippet } from 'svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';

	let {
		companyId = null,
		companyName,
		companies = [],
		tabs,
		execName = 'Chief of Staff',
		execLive = false,
		railOpen = false,
		onconversation,
		onRenameCompany,
		accountName = null,
		accountRole = null,
		accountDemo = false,
		rail,
		timeline,
		children
	}: {
		companyId?: string | null;
		companyName: string;
		companies?: { id: string; name: string }[];
		tabs: ShellTab[];
		execName?: string;
		execLive?: boolean;
		railOpen?: boolean;
		onconversation?: () => void;
		/** During founding the draft has no name yet — this lets the owner name it
		 * from the switcher so it is distinguishable while forming. Its presence is
		 * also what turns the brand into a dropdown when there are no other companies
		 * to switch to. */
		onRenameCompany?: (name: string) => void;
		/** The account slot: who you are and your role in this company. Absent
		 * where there is no membership to show. */
		accountName?: string | null;
		accountRole?: string | null;
		/** Local demo owner — honest label, no sign-out offered. */
		accountDemo?: boolean;
		rail?: Snippet;
		/** Optional center content in the top navbar — the founding timeline
		 * lives here while a company is being founded. */
		timeline?: Snippet;
		children: Snippet;
	} = $props();

	function markOf(name: string): string {
		return name.trim().slice(0, 1).toUpperCase() || PRODUCT_NAME.slice(0, 1).toUpperCase();
	}

	/* The brand menu opens whenever there is somewhere to go — another company
	 * to switch to, the records to consult, or (during founding) the draft to
	 * name. Only a truly lone, unnameable surface stays a static label. */
	const founding = $derived(companyId == null);
	const switchable = $derived(companyId != null || companies.length > 0 || !!onRenameCompany);

	/* Library and Tape — records and the audit trail — live in the brand menu,
	 * one click away but out of the daily tab row. The market joins them. */
	const libraryHref = $derived(companyId ? `/${companyId}/library` : null);
	const tapeHref = $derived(companyId ? `/${companyId}/tape` : null);
	const marketHref = $derived(companyId ? `/${companyId}/market` : null);
	const onLibrary = $derived(libraryHref != null && page.url.pathname.startsWith(libraryHref));
	const onTape = $derived(tapeHref != null && page.url.pathname.startsWith(tapeHref));
	const onMarket = $derived(marketHref != null && page.url.pathname.startsWith(marketHref));

	/* The single brand disclosure — button toggles, pointer outside or Escape
	 * closes, and every entry is a real link that works with middle click and
	 * keyboard alike. The menu holds destinations only; editing is not a
	 * destination, so naming the draft never lives here. */
	let brandOpen = $state(false);
	let brandWrap = $state<HTMLElement | undefined>();

	/* Naming the draft happens on the label itself, on request: the brand shows
	 * the same clean "<product> / company" as everywhere else — a small ghost
	 * affordance swaps the label for a field only when asked. Enter or blur
	 * commits; Escape cancels. */
	let renameValue = $state('');
	let renaming = $state(false);
	function startRename() {
		renameValue = '';
		renaming = true;
	}
	function submitRename() {
		const name = renameValue.trim();
		if (name && onRenameCompany) onRenameCompany(name);
		renameValue = '';
		renaming = false;
	}

	/* Focus belongs in the field once the ghost swaps the label for it — an
	 * action, not the autofocus attribute, so the intent is explicit. */
	function focusOnMount(el: HTMLElement) {
		el.focus();
	}

	/* The account slot — the same disclosure language as the brand menu. */
	let accountOpen = $state(false);
	let accountWrap = $state<HTMLElement | undefined>();

	/* The Records disclosure — a caret-tab in the strip, not a second identity
	 * dropdown: pill = identity, caret-tab = navigation. They can't be confused
	 * because they're different species. */
	let recordsOpen = $state(false);
	let recordsWrap = $state<HTMLElement | undefined>();
	const onRecords = $derived(onLibrary || onTape || onMarket);

	/* The menu stays bounded no matter how many companies exist: the current
	 * one pins as its own row, the list caps at a glance, and past that a
	 * filter finds the rest. An unbounded list is a wall, not a menu. */
	const MAX_VISIBLE = 6;
	let companyQuery = $state('');
	const currentCompany = $derived(companies.find((company) => company.id === companyId) ?? null);
	const visibleCompanies = $derived.by(() => {
		const query = companyQuery.trim().toLowerCase();
		const pool = query
			? companies.filter((company) => company.name.toLowerCase().includes(query))
			: companies;
		return pool.slice(0, query ? 20 : MAX_VISIBLE);
	});
	const hiddenCount = $derived(companies.length - visibleCompanies.length);

	function toggleBrand() {
		brandOpen = !brandOpen;
		if (!brandOpen) companyQuery = '';
	}

	async function signOut() {
		accountOpen = false;
		try {
			await fetch('/api/auth/sign-out', { method: 'POST' });
		} finally {
			window.location.href = '/sign-in';
		}
	}

	/* The light sibling (D2): persisted, applied pre-paint by the boot script in
	 * app.html, and inverted coherently — the machine's light becomes the
	 * darkest step, the operator's action stays solid. */
	let themeNow = $state<'dark' | 'light'>('dark');
	function toggleTheme() {
		themeNow = themeNow === 'light' ? 'dark' : 'light';
		document.documentElement.dataset.theme = themeNow;
		document.documentElement.style.background = themeNow === 'light' ? '#f3f4f6' : '#0a0b0d';
		try {
			localStorage.setItem('ph-theme', themeNow);
		} catch {
			/* private mode — the toggle still works for the session */
		}
	}
	$effect(() => {
		if (document.documentElement.dataset.theme === 'light') themeNow = 'light';
	});

	function onWindowPointerdown(event: PointerEvent) {
		if (
			brandOpen &&
			brandWrap &&
			event.target instanceof Node &&
			!brandWrap.contains(event.target)
		) {
			brandOpen = false;
		}
		if (
			accountOpen &&
			accountWrap &&
			event.target instanceof Node &&
			!accountWrap.contains(event.target)
		) {
			accountOpen = false;
		}
		if (
			recordsOpen &&
			recordsWrap &&
			event.target instanceof Node &&
			!recordsWrap.contains(event.target)
		) {
			recordsOpen = false;
		}
	}

	function onWindowKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			brandOpen = false;
			accountOpen = false;
			recordsOpen = false;
		}
	}
</script>

<svelte:window onpointerdown={onWindowPointerdown} onkeydown={onWindowKeydown} />

<div class="bridge-root">
	<header class="bridge-topbar">
		<div class="tb-brandwrap" bind:this={brandWrap}>
			{#if founding && onRenameCompany && renaming}
				<span class="tb-brand static">
					<span class="tb-mark"><MatrixGlyph rows={GLYPHS.p} size={13} glow /></span>
					<span class="tb-name">{PRODUCT_NAME}</span>
				</span>
				<input
					class="tb-co-edit"
					type="text"
					placeholder={companyName}
					bind:value={renameValue}
					aria-label="Name this company"
					size={Math.max(10, companyName.length)}
					use:focusOnMount
					onkeydown={(event) => {
						if (event.key === 'Enter') submitRename();
						if (event.key === 'Escape') {
							renameValue = '';
							renaming = false;
						}
					}}
					onblur={submitRename}
				/>
			{:else if switchable}
				<button
					class="tb-brand"
					type="button"
					aria-expanded={brandOpen}
					aria-label="Helm menu"
					title={companyName}
					onclick={toggleBrand}
				>
					<span class="tb-mark"><MatrixGlyph rows={GLYPHS.p} size={13} glow /></span>
					<span class="tb-name">{PRODUCT_NAME}</span>
					<span class="tb-co">/ {companyName}</span>
					<span class="tb-caret" class:open={brandOpen}>▾</span>
				</button>
				{#if founding && onRenameCompany && !renaming}
					<button class="tb-rename-ghost" type="button" onclick={startRename}>rename</button>
				{/if}
				{#if brandOpen}
					<div class="tb-menu">
						{#if companies.length > 0}
							{#if currentCompany}
								<div class="tb-menu-label">This company</div>
								<a
									class="tb-menu-item current"
									href={`/${currentCompany.id}`}
									onclick={() => (brandOpen = false)}
								>
									<span class="tb-menu-mark">{markOf(currentCompany.name)}</span>
									<span class="tb-menu-name">{currentCompany.name}</span>
									<span class="tb-menu-check"><MatrixGlyph rows={GLYPHS.check} size={8} /></span>
								</a>
								<!-- Settings lives here, not as a sixth tab: a tab strip of five operating
								     surfaces is the product, and a sixth for administration would dilute it.
								     The account menu keeps personal things (theme, sign out) — company vs
								     person is the split. -->
								<a
									class="tb-menu-item"
									href={`/${currentCompany.id}/settings/company`}
									onclick={() => (brandOpen = false)}
								>
									<span class="tb-menu-name">Settings</span>
								</a>
							{/if}
							<div class="tb-menu-label">Companies</div>
							{#if companies.length > MAX_VISIBLE}
								<div class="tb-menu-filter">
									<input
										class="tb-filter-input"
										type="text"
										placeholder="filter companies…"
										aria-label="Filter companies"
										bind:value={companyQuery}
									/>
								</div>
							{/if}
							<div class="tb-menu-list">
								{#each visibleCompanies as company (company.id)}
									<a
										class="tb-menu-item"
										class:current={company.id === companyId}
										href={`/${company.id}`}
										onclick={() => (brandOpen = false)}
									>
										<span class="tb-menu-mark">{markOf(company.name)}</span>
										<span class="tb-menu-name">{company.name}</span>
										{#if company.id === companyId}
											<span class="tb-menu-check"><MatrixGlyph rows={GLYPHS.check} size={8} /></span
											>
										{/if}
									</a>
								{:else}
									<p class="tb-menu-empty">No companies match.</p>
								{/each}
								{#if hiddenCount > 0 && !companyQuery.trim()}
									<p class="tb-menu-empty">{hiddenCount} more — filter to find them.</p>
								{/if}
							</div>
							<a class="tb-menu-new" href={'/onboarding'}>
								<span class="tb-menu-new-mark"><MatrixGlyph rows={GLYPHS.plus} size={9} /></span>
								<span class="tb-menu-new-label">New company</span>
							</a>
						{/if}
					</div>
				{/if}
			{:else}
				<span class="tb-brand static" title={companyName}>
					<span class="tb-mark"><MatrixGlyph rows={GLYPHS.p} size={13} glow /></span>
					<span class="tb-name">{PRODUCT_NAME}</span>
					<span class="tb-co">/ {companyName}</span>
				</span>
			{/if}
		</div>

		<!-- The nav group carries the centering so Records stays welded to the
		     strip while sitting outside its scroll box. -->
		<div class="tb-navgroup">
			<nav class="tb-tabs" aria-label="primary">
				{#each tabs as tab (tab.key)}
					{#if tab.href}
						<!-- Tab hrefs are built by the caller (the layout), which owns the
					     route shape for this surface. -->
						<a class="tb-tab" class:on={tab.on} href={tab.href}>
							{tab.label}
							{#if tab.badge}<span class="tb-badge">{tab.badge}</span>{/if}
						</a>
					{:else}
						<button
							class="tb-tab"
							class:on={tab.on}
							type="button"
							disabled={tab.disabled}
							onclick={tab.onclick}
						>
							{tab.label}
							{#if tab.badge}<span class="tb-badge">{tab.badge}</span>{/if}
						</button>
					{/if}
				{/each}
			</nav>

			<!-- Records lives beside the strip, not inside it: the strip scrolls
		     (overflow-x), and an overflow container clips an absolutely
		     positioned menu on both axes. Outside it, the menu is free. -->
			{#if libraryHref && tapeHref}
				<div class="tb-recordswrap" bind:this={recordsWrap}>
					<button
						class="tb-tab tb-records"
						class:on={onRecords}
						type="button"
						aria-expanded={recordsOpen}
						aria-label="Records"
						onclick={() => (recordsOpen = !recordsOpen)}
					>
						Records <span class="tb-caret" class:open={recordsOpen}>▾</span>
					</button>
					{#if recordsOpen}
						<div class="tb-menu tb-records-menu">
							<a
								class="tb-menu-item"
								class:current={onLibrary}
								href={libraryHref}
								onclick={() => (recordsOpen = false)}
							>
								<span class="tb-menu-name">Library</span>
								{#if onLibrary}
									<span class="tb-menu-check"><MatrixGlyph rows={GLYPHS.check} size={8} /></span>
								{/if}
							</a>
							<a
								class="tb-menu-item"
								class:current={onTape}
								href={tapeHref}
								onclick={() => (recordsOpen = false)}
							>
								<span class="tb-menu-name">Tape</span>
								{#if onTape}
									<span class="tb-menu-check"><MatrixGlyph rows={GLYPHS.check} size={8} /></span>
								{/if}
							</a>
							{#if marketHref}
								<a
									class="tb-menu-item"
									class:current={onMarket}
									href={marketHref}
									onclick={() => (recordsOpen = false)}
								>
									<span class="tb-menu-name">Market</span>
									{#if onMarket}
										<span class="tb-menu-check"><MatrixGlyph rows={GLYPHS.check} size={8} /></span>
									{/if}
								</a>
							{/if}
						</div>
					{/if}
				</div>
			{/if}
		</div>

		{#if timeline}
			<div class="tb-timeline">
				{@render timeline()}
			</div>
		{/if}

		<div class="tb-right">
			{#if onconversation}
				<button
					class="tb-cos"
					class:on={railOpen}
					type="button"
					onclick={onconversation}
					aria-expanded={railOpen}
					aria-controls="bridge-exrail"
					title="talk to {execName}"
				>
					<span class="tb-cos-lamp" class:live={execLive}>
						<MatrixGlyph rows={execLive ? GLYPHS.dots : GLYPHS.ring} size={9} glow={execLive} />
					</span>
					<span class="tb-cos-name">{execName}</span>
				</button>
			{/if}
			{#if accountRole}
				<div class="tb-accountwrap" bind:this={accountWrap}>
					<button
						class="tb-account"
						type="button"
						aria-expanded={accountOpen}
						aria-label="Account menu"
						title={accountName ?? 'You'}
						onclick={() => (accountOpen = !accountOpen)}
					>
						<span class="tb-account-mark">{markOf(accountName ?? 'You')}</span>
					</button>
					{#if accountOpen}
						<div class="tb-menu tb-account-menu">
							<div class="tb-account-head">
								<span class="tb-account-name">{accountName ?? 'You'}</span>
								<span class="tb-account-sub mono">
									{accountRole}{accountDemo ? ' · local demo' : ''}
								</span>
							</div>
							<button class="tb-menu-item tb-menu-button" type="button" onclick={toggleTheme}>
								<span class="tb-menu-name"
									>{themeNow === 'light' ? 'Dark theme' : 'Light theme'}</span
								>
							</button>
							{#if !accountDemo}
								<button class="tb-menu-item tb-menu-button" type="button" onclick={signOut}>
									<span class="tb-menu-name">Sign out</span>
								</button>
							{/if}
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</header>

	<div class="bridge-body">
		<main class="bridge-content">
			{@render children()}
		</main>
		{#if rail}{@render rail()}{/if}
	</div>
</div>
