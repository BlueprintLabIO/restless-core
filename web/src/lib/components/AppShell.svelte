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
	/* Bridge Light has one owner shell: company identity, four work surfaces, and
	 * a single Exec control at the right that carries live presence and opens or
	 * closes the transcript. The executive transcript is a persistent sibling of
	 * the workspace on desktop; it is not a chat mode or a disclosure hidden
	 * behind another navigation control. */

	import type { Snippet } from 'svelte';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import { PRODUCT_NAME } from '../brand/brand';
	import type { CompanyCatalogEntry } from '../product/contracts';
	import MatrixGlyph, { GLYPHS } from '../primitives/MatrixGlyph.svelte';

	let {
		companyId,
		companyName,
		companies = [],
		tabs,
		portfolioHref = '/',
		companyHref = (company: CompanyCatalogEntry) => `/${company.id}`,
		execName = 'Exec',
		execLive = false,
		railOpen = true,
		newFocusAvailable = false,
		newFocusDisabled = false,
		immersive = false,
		onexectoggle = null,
		onnewfocus = null,
		rail = null,
		children
	}: {
		companyId: string;
		companyName: string;
		companies?: CompanyCatalogEntry[];
		tabs: ShellTab[];
		/** Platform-owned route back to the authenticated company portfolio. */
		portfolioHref?: string;
		/** Platform-owned route or one-time entry URL for another company. */
		companyHref?: (company: CompanyCatalogEntry) => string;
		execName?: string;
		execLive?: boolean;
		railOpen?: boolean;
		newFocusAvailable?: boolean;
		newFocusDisabled?: boolean;
		/** Gives a prepared live outcome the full browser window while preserving one bounded Exec control. */
		immersive?: boolean;
		/** The one control for the rail: presence lamp and open/close in a single stable button. */
		onexectoggle?: (() => void) | null;
		/** Begins a clean working context without replacing the durable Exec relationship. */
		onnewfocus?: (() => void) | null;
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
			<a class="tb-brand-home" href={portfolioHref} aria-label={`${PRODUCT_NAME} companies`}>
				<span class="tb-mark"><MatrixGlyph rows={GLYPHS.r} size={13} glow /></span>
				<span class="tb-name">{PRODUCT_NAME}</span>
			</a>
			<span class="tb-company-slash" aria-hidden="true">/</span>
			<details class="company-switcher">
				<summary aria-label={`Switch company. Current company: ${companyName}`}>
					<span class="tb-co">{companyName}</span><span class="company-chevron" aria-hidden="true"
						>⌄</span
					>
				</summary>
				<div class="company-switcher-menu">
					<a class="company-overview-link" href={portfolioHref}>
						<MatrixGlyph rows={GLYPHS.r} size={8} />
						<span><strong>All companies</strong><small>Owner portfolio</small></span>
					</a>
					<div class="company-switcher-rule" role="separator"></div>
					{#each activeCompanies as company (company.id)}
						<a class:current={company.id === companyId} href={companyHref(company)}>
							<i class="runtime-{company.runtime_status}" aria-hidden="true"></i>
							<span><strong>{company.name}</strong><small>{company.runtime_status}</small></span>
							{#if company.id === companyId}<span class="switcher-current">Current</span>{/if}
						</a>
					{:else}
						<a class="current" href={companyHref({
							id: companyId,
							name: companyName,
							mission: '',
							model: '',
							spend_ceiling_usd: null,
							runtime_status: 'running',
							lifecycle_status: 'active'
						})}>
							<i aria-hidden="true"></i><span
								><strong>{companyName}</strong><small>Current company</small></span
							>
						</a>
					{/each}
				</div>
			</details>
		</div>

		<nav class="tb-tabs" aria-label="Owner surfaces">
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
					title={`${execName} ${execLive ? 'present' : 'unavailable'}`}
					onclick={() => onexectoggle?.()}
				>
					<!-- Shape says what the control does, colour says whether the Exec can
					     answer: the button tints and glows live, greys when unreachable. -->
					<MessageSquare size={13} strokeWidth={2} aria-hidden="true" />
					{execName}
				</button>
				{#if newFocusAvailable}
					<button
						class="tb-new-focus"
						type="button"
						disabled={newFocusDisabled}
						title={newFocusDisabled
							? 'Start a new focus when Exec finishes the current reply'
							: 'Begin with fresh working context; company memory is retained'}
						onclick={() => onnewfocus?.()}
					>
						New focus
					</button>
				{/if}
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

	<div class="bridge-body">
		<div class="bridge-workspace">
			<main class="bridge-content">{@render children()}</main>
		</div>
		{#if rail}{@render rail()}{/if}
	</div>
</div>
