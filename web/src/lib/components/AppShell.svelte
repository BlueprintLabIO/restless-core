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

	import type { Snippet } from 'svelte';
	import { resizePane } from '$lib/actions/resize-pane';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import MessageSquare from '@lucide/svelte/icons/message-square';
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
		execLive = false,
		expandExec = false,
		railOpen = true,
		immersive = false,
		onexectoggle = null,
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
		execLive?: boolean;
		expandExec?: boolean;
		railOpen?: boolean;
		/** Gives a prepared live outcome the full browser window while preserving one bounded Exec control. */
		immersive?: boolean;
		/** The one control for the rail: presence lamp and open/close in a single stable button. */
		onexectoggle?: (() => void) | null;
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
</script>

<div class="bridge-root" class:immersive>
	<header class="bridge-topbar" aria-label="Global navigation">
		<div class="tb-brand">
			<a class="tb-brand-home" href={homeHref} aria-label={`${PRODUCT_NAME} company home`}>
				<span class="tb-mark"><MatrixGlyph rows={GLYPHS.r} size={13} glow /></span>
				<span class="tb-name">{PRODUCT_NAME}</span>
			</a>
			<span class="tb-company-slash" aria-hidden="true">/</span>
			{#if canSwitchCompanies}
				<details class="company-switcher">
					<summary aria-label={`Switch company. Current company: ${companyName}`}>
						<span class="tb-co">{companyName}</span>
						<ChevronDown class="company-chevron" size={14} strokeWidth={2} aria-hidden="true" />
					</summary>
					<div class="company-switcher-menu">
						<a class="company-overview-link" href="/">
							<MatrixGlyph rows={GLYPHS.r} size={8} />
							<span><strong>All companies</strong><small>Owner portfolio</small></span>
						</a>
						<div class="company-switcher-rule" role="separator"></div>
						{#each activeCompanies as company (company.id)}
							<a class:current={company.id === companyId} href={`/${company.id}`}>
								<i class="runtime-{company.runtime_status}" aria-hidden="true"></i>
								<span><strong>{company.name}</strong><small>{company.runtime_status}</small></span>
								{#if company.id === companyId}<span class="switcher-current">Current</span>{/if}
							</a>
						{:else}
							<a class="current" href={`/${companyId}`}>
								<i aria-hidden="true"></i><span
									><strong>{companyName}</strong><small>Current company</small></span
								>
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

		<nav class="tb-tabs" aria-label="Company navigation">
			{#each tabs as tab (tab.key)}
				<a
					class="tb-tab"
					class:on={tab.on}
					data-surface={tab.key}
					href={tab.href}
					aria-current={tab.on ? 'page' : undefined}
					aria-label={tab.badge ? `${tab.label}, ${tab.badge} items` : tab.label}
				>
					<span class="tb-tab-mark" aria-hidden="true">
						<MatrixGlyph rows={tabGlyphs[tab.key] ?? GLYPHS.square} size={12} />
					</span>
					<span class="tb-tab-label" aria-hidden="true">{tab.label}</span>
					{#if tab.badge}<span class="tb-badge">{tab.badge}</span>{/if}
				</a>
			{/each}
		</nav>

		<div class="tb-right">
			{#if rail}
				<button
					class="tb-exec"
					class:live={execLive}
					class:on={railOpen}
					type="button"
					aria-controls="bridge-exrail"
					aria-expanded={railOpen}
					onclick={() => onexectoggle?.()}
				>
					<MessageSquare size={13} strokeWidth={2} aria-hidden="true" />{execName}
				</button>
			{/if}
		</div>
	</header>

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
