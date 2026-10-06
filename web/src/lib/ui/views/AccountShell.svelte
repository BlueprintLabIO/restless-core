<script lang="ts">
	import { onMount, type Snippet } from 'svelte';
	import ChevronsUpDown from '@lucide/svelte/icons/chevrons-up-down';
	import ChevronLeft from '@lucide/svelte/icons/chevron-left';
	import Plus from '@lucide/svelte/icons/plus';
	import { dismissable } from '../controls/dismissable';
	import type { AccountGroup, AccountTab } from '../account';

	/* The frame for every page outside one company, after Firecrawl's: the product at the top, the
	 * rail's rows (Home), labelled groups (the companies, then the account's own rows such as
	 * Settings and Help), and the owner at the foot with the one account menu, opening upward.
	 * The rail never changes shape between pages: a settings page lists its sections in the page,
	 * so the rail stays where the owner left it. It collapses to icons (⌘B, the Collapse row, or a
	 * click on its edge), resizes by dragging its edge, and remembers both in this browser. On a
	 * phone it folds into a top bar. */
	let {
		brandName = 'Restless',
		homeHref = '/',
		tabs = [],
		groups = [],
		account = null,
		accountMenu = null,
		footer = null,
		children
	}: {
		brandName?: string;
		homeHref?: string;
		/** The rail's top rows, in order, such as Home. */
		tabs?: AccountTab[];
		/** Labelled groups beneath them: the companies, then the account's rows (Settings, Help). */
		groups?: AccountGroup[];
		account?: { name: string; detail?: string } | null;
		/** Items in the owner's menu, below their name. */
		accountMenu?: Snippet | null;
		/** Pinned to the rail's foot when it is open, such as what is left of a monthly allowance.
		 * A collapsed rail and a phone's top bar leave it out; the same facts live on Home. */
		footer?: Snippet | null;
		children: Snippet;
	} = $props();

	const MIN = 200;
	const MAX = 340;
	const DEFAULT = 240;
	const KEY = 'restless:account-rail';
	let width = $state(DEFAULT);
	let collapsed = $state(false);
	let resizing = $state(false);

	/* Per-browser conveniences only: absent or blocked storage leaves the defaults. */
	onMount(() => {
		try {
			const saved = JSON.parse(localStorage.getItem(KEY) ?? 'null') as {
				width?: number;
				collapsed?: boolean;
			} | null;
			if (saved?.width) width = Math.min(MAX, Math.max(MIN, saved.width));
			if (saved?.collapsed) collapsed = true;
		} catch {
			// Defaults stand.
		}
	});
	function remember() {
		try {
			localStorage.setItem(KEY, JSON.stringify({ width, collapsed }));
		} catch {
			// Not remembered; the rail still works.
		}
	}
	function toggle() {
		collapsed = !collapsed;
		remember();
	}
	function keydown(event: KeyboardEvent) {
		if (event.key.toLowerCase() === 'b' && (event.metaKey || event.ctrlKey) && !event.altKey) {
			const target = event.target as HTMLElement | null;
			if (target?.closest('input, textarea, [contenteditable]')) return;
			event.preventDefault();
			toggle();
		}
	}

	/* The edge: drag to resize, click to collapse or expand. */
	let dragFrom: { x: number; width: number; moved: boolean } | null = null;
	function edgeDown(event: PointerEvent) {
		if (event.button !== 0) return;
		dragFrom = { x: event.clientX, width, moved: false };
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
	}
	function edgeMove(event: PointerEvent) {
		if (!dragFrom) return;
		const delta = event.clientX - dragFrom.x;
		if (Math.abs(delta) < 3 && !dragFrom.moved) return;
		dragFrom.moved = true;
		resizing = true;
		if (collapsed && delta > 24) collapsed = false;
		if (!collapsed) width = Math.min(MAX, Math.max(MIN, dragFrom.width + delta));
	}
	function edgeUp() {
		if (!dragFrom) return;
		if (!dragFrom.moved) collapsed = !collapsed;
		dragFrom = null;
		resizing = false;
		remember();
	}

	const initialsOf = (name: string) =>
		(name || '?')
			.split(/\s+/)
			.map((part) => part[0])
			.join('')
			.slice(0, 2)
			.toUpperCase();
	const initials = $derived(initialsOf(account?.name ?? ''));
</script>

<svelte:window onkeydown={keydown} />

{#snippet row(tab: AccountTab, kind: 'tab' | 'sub' | 'company')}
	{@const Icon = tab.icon}
	{@const label = tab.count ? `${tab.label}, ${tab.count} waiting` : tab.label}
	{#snippet inner()}
		{#if Icon}<Icon
				size={16}
				strokeWidth={1.75}
				aria-hidden="true"
			/>{:else if kind === 'company'}<span class="account-mark" aria-hidden="true"
				>{initialsOf(tab.label).slice(0, 1)}</span
			>{/if}<span class="account-tab-label">{tab.label}</span>{#if tab.tone}<i
				class="account-dot"
				data-tone={tab.tone}
				aria-hidden="true"
			></i>{/if}{#if tab.count}<span class="account-count" aria-hidden="true">{tab.count}</span
			>{/if}
	{/snippet}
	{#if tab.form}
		<form method="POST" action={tab.form.action}>
			{#each Object.entries(tab.form.fields ?? {}) as [name, value]}<input
					type="hidden"
					{name}
					{value}
				/>{/each}
			<button
				class="account-tab"
				class:sub={kind === 'sub'}
				class:active={tab.active}
				title={collapsed ? tab.label : tab.tooltip}
				aria-label={label}>{@render inner()}</button
			>
		</form>
	{:else}
		<a
			class="account-tab"
			class:sub={kind === 'sub'}
			class:active={tab.active}
			href={tab.href}
			title={collapsed ? tab.label : tab.tooltip}
			aria-label={label}
			aria-current={tab.active ? 'page' : undefined}>{@render inner()}</a
		>
	{/if}
{/snippet}

<div
	class="bridge-tokens account-shell"
	class:collapsed
	class:resizing
	style:--account-rail-w={collapsed ? '56px' : `${width}px`}
>
	<header class="account-bar">
		<div class="account-head">
			<a class="account-brand" href={homeHref} title={collapsed ? brandName : undefined}
				><span class="account-brand-mark" aria-hidden="true">{brandName.slice(0, 1)}</span><span
					class="account-tab-label">{brandName}</span
				></a
			>
		</div>
		<nav class="account-tabs" aria-label="Account navigation">
			{#each tabs as tab (tab.href ?? tab.label)}
				{@render row(tab, 'tab')}
				{#if tab.items?.length && !collapsed}
					<div class="account-subtabs">
						{#each tab.items as subtab (subtab.href ?? subtab.label)}{@render row(
								subtab,
								'sub'
							)}{/each}
					</div>
				{/if}
			{/each}
			{#each groups as group (group.label)}
				<div class="account-group" role="group" aria-label={group.label}>
					<div class="account-group-head">
						<span>{group.label}</span>
						{#if group.action}
							{@const action = group.action}
							{#if action.form}
								<form method="POST" action={action.form.action}>
									<button
										class="account-group-add"
										title={action.tooltip ?? action.label}
										aria-label={action.label}
										><Plus size={14} strokeWidth={2} aria-hidden="true" /></button
									>
								</form>
							{:else}
								<a
									class="account-group-add"
									href={action.href}
									title={action.tooltip ?? action.label}
									aria-label={action.label}><Plus size={14} strokeWidth={2} aria-hidden="true" /></a
								>
							{/if}
						{/if}
					</div>
					{#each group.items as item (item.href ?? item.label + (item.form?.fields?.organization_id ?? ''))}{@render row(
							item,
							'company'
						)}{/each}
				</div>
			{/each}
		</nav>
		{#if footer && !collapsed}<div class="account-foot">{@render footer()}</div>{/if}
		{#if account}
			<details class="account-menu" use:dismissable>
				<summary
					class="account-who"
					title={collapsed ? account.name : account.detail}
					aria-label={`${account.name}: account menu`}
				>
					<span class="account-avatar" aria-hidden="true">{initials}</span>
					<span class="account-name">{account.name}</span>
					<ChevronsUpDown size={14} strokeWidth={1.75} aria-hidden="true" />
				</summary>
				<div class="account-menu-panel" role="menu">
					<div class="account-menu-who">
						<strong>{account.name}</strong>
						{#if account.detail}<small>{account.detail}</small>{/if}
					</div>
					{#if accountMenu}<div class="account-menu-items">{@render accountMenu()}</div>{/if}
				</div>
			</details>
		{/if}
		<button
			class="account-collapse"
			type="button"
			onclick={toggle}
			aria-expanded={!collapsed}
			title={collapsed ? 'Expand (⌘B)' : '⌘B'}
			><ChevronLeft size={16} strokeWidth={1.75} aria-hidden="true" /><span
				class="account-tab-label">Collapse</span
			></button
		>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="account-edge"
			title={collapsed ? 'Click to expand' : 'Drag to resize · click to collapse'}
			onpointerdown={edgeDown}
			onpointermove={edgeMove}
			onpointerup={edgeUp}
			onpointercancel={edgeUp}
		></div>
	</header>
	<div class="account-page">{@render children()}</div>
</div>

<style>
	.account-shell {
		display: grid;
		grid-template-columns: var(--account-rail-w, 240px) minmax(0, 1fr);
		min-height: 100vh;
		min-height: 100dvh;
		background: var(--bg-app);
		color: var(--ink);
		transition: grid-template-columns var(--motion-disclosure, 180ms) var(--ease-standard, ease);
	}
	.account-shell.resizing {
		transition: none;
		cursor: col-resize;
		user-select: none;
	}
	.account-bar {
		position: sticky;
		top: 0;
		z-index: var(--z-sticky, 20);
		display: flex;
		flex-direction: column;
		gap: 10px;
		min-width: 0;
		height: 100vh;
		height: 100dvh;
		padding: 10px 8px;
		border-right: 1px solid var(--border);
		background: var(--surface-rail, var(--bg-app));
	}
	.account-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 4px;
		min-height: 32px;
	}

	/* One row height, one size, one weight ramp: 13px labels at 30px, like Linear's sidebar. */
	.account-tab,
	.account-brand,
	.account-collapse,
	.account-who {
		position: relative;
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
		height: 30px;
		padding: 0 8px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--t-body);
		text-align: left;
		text-decoration: none;
		white-space: nowrap;
		cursor: pointer;
		transition:
			background var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard);
	}
	.account-tab :global(svg),
	.account-collapse :global(svg) {
		flex: none;
		color: var(--text-tertiary);
		transition: color var(--motion-state) var(--ease-standard);
	}
	.account-tab-label {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.account-tab:hover,
	.account-brand:hover,
	.account-collapse:hover,
	.account-who:hover,
	.account-menu[open] > .account-who {
		background: var(--wash-hover, var(--surface-alt));
		color: var(--ink);
	}
	.account-tab:hover :global(svg),
	.account-collapse:hover :global(svg),
	.account-tab.active :global(svg) {
		color: var(--ink);
	}
	.account-tab.active {
		background: var(--wash-active, var(--surface-alt));
		color: var(--ink);
		font-weight: 500;
	}
	.account-tab:focus-visible,
	.account-brand:focus-visible,
	.account-who:focus-visible,
	.account-collapse:focus-visible,
	.account-group-add:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: -2px;
	}
	.account-brand {
		flex: 1 1 auto;
		color: var(--ink);
		font-weight: 600;
		letter-spacing: -0.01em;
	}
	.account-brand-mark {
		display: grid;
		flex: none;
		place-items: center;
		width: 18px;
		height: 18px;
		border-radius: 5px;
		background: var(--ink);
		color: var(--surface-raised);
		font-size: var(--t-label);
		font-weight: 700;
	}

	.account-subtabs {
		display: grid;
		gap: 1px;
		margin: 1px 0 4px;
	}
	.account-tab.sub {
		padding-left: 32px;
	}

	.account-foot {
		flex: none;
		animation: account-foot-in var(--motion-disclosure) var(--ease-out) both;
	}
	@keyframes account-foot-in {
		from {
			opacity: 0;
			transform: translateY(4px);
		}
	}
	.account-tabs {
		display: grid;
		gap: 1px;
		align-content: start;
		flex: 1 1 auto;
		min-height: 0;
		overflow-x: hidden;
		overflow-y: auto;
	}
	.account-tabs form {
		display: contents;
	}
	.account-group {
		display: grid;
		gap: 1px;
		margin-top: 14px;
	}
	.account-group-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 26px;
		padding: 0 4px 0 8px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 500;
	}
	.account-group-head form {
		display: contents;
	}
	.account-group-add {
		display: grid;
		place-items: center;
		width: 22px;
		height: 22px;
		min-width: 0;
		min-height: 0;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
		opacity: 0;
		transition: opacity var(--motion-state) var(--ease-standard);
	}
	.account-group:hover .account-group-add,
	.account-group-add:focus-visible {
		opacity: 1;
	}
	.account-group-add:hover {
		background: var(--wash-hover, var(--surface-alt));
		color: var(--ink);
	}
	/* A company's mark: its initial on a quiet square, where a section would carry an icon. */
	.account-mark {
		display: grid;
		flex: none;
		place-items: center;
		width: 18px;
		height: 18px;
		border-radius: 5px;
		background: var(--surface-alt);
		color: var(--text-secondary);
		font-size: var(--t-label);
		font-weight: 600;
	}
	.account-dot {
		flex: none;
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--text-tertiary);
	}
	.account-dot[data-tone='presence'] {
		background: var(--state-success);
	}
	.account-dot[data-tone='unavailable'] {
		background: var(--state-danger);
	}
	.account-count {
		flex: none;
		min-width: 18px;
		padding: 0 5px;
		border-radius: 9px;
		background: var(--surface-alt);
		color: var(--text-secondary);
		font-size: var(--t-label);
		font-variant-numeric: tabular-nums;
		line-height: 18px;
		text-align: center;
	}

	.account-collapse {
		flex: none;
		width: 100%;
		color: var(--text-tertiary);
	}
	.account-collapse :global(svg) {
		transition: transform var(--motion-disclosure, 180ms) var(--ease-standard, ease);
	}
	.collapsed .account-collapse :global(svg) {
		transform: rotate(180deg);
	}

	/* The owner's menu: the one place for the account, at the rail's foot, opening upward. */
	.account-menu {
		position: relative;
		flex: none;
		min-width: 0;
		margin-top: auto;
	}
	.account-foot + .account-menu {
		margin-top: 0;
	}
	.account-who {
		color: var(--ink);
		list-style: none;
	}
	.account-who > :global(svg) {
		margin-left: auto;
	}
	.account-who :global(svg) {
		flex: none;
		color: var(--text-tertiary);
	}
	.account-who::-webkit-details-marker,
	.account-who::marker {
		display: none;
		content: none;
	}
	.account-who::before {
		display: none;
	}
	.account-avatar {
		display: grid;
		flex: none;
		place-items: center;
		width: 20px;
		height: 20px;
		border-radius: 50%;
		background: var(--ink);
		color: var(--surface-raised);
		font-size: var(--t-label);
		font-weight: 600;
	}
	.account-name {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	/* As wide as the rail and no wider: the menu belongs to the row it opens from. */
	.account-menu-panel {
		position: absolute;
		bottom: calc(100% + 4px);
		left: 0;
		z-index: 30;
		width: 100%;
		min-width: 188px;
		max-width: 232px;
		padding: 4px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-float);
		animation: bridge-popover-in var(--motion-disclosure, 160ms) var(--ease-spring, ease-out) both;
	}
	.account-menu:global([data-closing]) .account-menu-panel {
		animation: bridge-popover-out var(--motion-state, 120ms) var(--ease-standard, ease-in) both;
	}
	.account-menu-who {
		display: grid;
		gap: 2px;
		min-width: 0;
		padding: 8px 10px;
		border-bottom: 1px solid var(--border);
	}
	.account-menu-who > * {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.account-menu-who strong {
		font-size: var(--t-body);
		font-weight: 500;
	}
	.account-menu-who small {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.account-menu-items {
		display: grid;
		padding-top: 4px;
	}
	.account-menu-items :global(form) {
		display: contents;
	}
	.account-menu-items :global(hr) {
		height: 1px;
		margin: 4px 0;
		border: 0;
		background: var(--border);
	}
	.account-menu-items :global(:is(a, button)) {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		height: 28px;
		min-height: 0;
		padding: 0 10px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--ink);
		font: inherit;
		font-size: var(--t-body);
		text-align: left;
		text-decoration: none;
		cursor: pointer;
	}
	.account-menu-items :global(:is(a, button):hover) {
		background: var(--wash-hover, var(--surface-alt));
	}

	/* Collapsed: icons only, centred; labels, groups' names and the owner's name step out. */
	.collapsed .account-tab-label,
	.collapsed .account-name,
	.collapsed .account-who > :global(svg),
	.collapsed .account-count,
	.collapsed .account-dot,
	.collapsed .account-group-head,
	.collapsed .account-subtabs {
		display: none;
	}
	.collapsed .account-tab,
	.collapsed .account-brand,
	.collapsed .account-collapse,
	.collapsed .account-who {
		justify-content: center;
		padding: 0;
	}
	.collapsed .account-menu-panel {
		bottom: 0;
		left: calc(100% + 8px);
		width: 220px;
	}
	.collapsed .account-group {
		margin-top: 8px;
		padding-top: 8px;
		border-top: 1px solid var(--border);
	}

	/* The edge: a hairline that thickens under the pointer. */
	.account-edge {
		position: absolute;
		top: 0;
		right: -4px;
		bottom: 0;
		width: 8px;
		cursor: col-resize;
		touch-action: none;
	}
	.account-edge::after {
		content: '';
		position: absolute;
		top: 0;
		bottom: 0;
		left: 3px;
		width: 2px;
		background: transparent;
		transition: background var(--motion-state) var(--ease-standard);
	}
	.account-edge:hover::after,
	.resizing .account-edge::after {
		background: var(--border-strong);
	}
	.collapsed .account-edge {
		cursor: e-resize;
	}
	.account-page {
		min-width: 0;
		container: account-page / inline-size;
	}
	@media (prefers-reduced-motion: reduce) {
		.account-shell {
			transition: none;
		}
	}

	/* A phone: the rail folds into a top bar (the owner's menu or the way back, then the rows).
	 * Companies stay on Home's list; settings sections scroll sideways. */
	@media (max-width: 760px) {
		/* Phones give every control a 44px reach (primitives.css); the bar keeps its drawn
		 * proportions and carries the reach as an invisible hit area instead. */
		.account-bar .account-tab,
		.account-bar .account-brand,
		.account-menu > .account-who {
			min-width: 0;
			min-height: 0;
		}
		.account-shell {
			display: block;
		}
		.account-bar {
			flex-direction: row;
			align-items: center;
			gap: 10px;
			height: var(--topbar-h, 52px);
			padding: 0 max(12px, env(safe-area-inset-right)) 0 max(12px, env(safe-area-inset-left));
			border-right: 0;
			border-bottom: 1px solid var(--border);
			background: var(--glass);
			backdrop-filter: blur(18px) saturate(1.4);
			-webkit-backdrop-filter: blur(18px) saturate(1.4);
		}
		.account-head {
			flex: none;
		}
		.account-brand .account-tab-label {
			display: none;
		}
		.account-menu {
			flex: none;
			order: 9;
			margin-left: auto;
		}
		.account-menu-panel {
			top: calc(100% + 6px);
			right: 0;
			bottom: auto;
			left: auto;
			width: 220px;
		}
		.account-collapse,
		.account-edge,
		.account-subtabs,
		.account-name,
		.account-tab :global(svg),
		.account-group-head {
			display: none;
		}
		/* Companies stay on Home's list; Settings and Help are in the owner's menu. */
		.account-group {
			display: none !important;
		}
		.collapsed .account-tab-label {
			display: initial;
		}
		.account-foot {
			display: none;
		}
		.account-tabs {
			display: flex;
			gap: 2px;
			overflow-x: auto;
			overflow-y: visible;
			scrollbar-width: none;
		}
		.account-group {
			display: flex;
			gap: 2px;
			margin: 0;
			padding: 0;
			border: 0;
		}
		.account-tab {
			flex: none;
			height: 30px;
			padding: 0 10px;
		}
		.account-tab::after,
		.account-who::after {
			content: '';
			position: absolute;
			inset: -7px 0;
		}
		.account-avatar {
			width: 26px;
			height: 26px;
			font-size: var(--t-label);
		}
		.account-who {
			padding: 0 4px;
		}
		.account-who > :global(svg) {
			display: none;
		}
	}
</style>
