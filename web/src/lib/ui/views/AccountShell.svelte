<script lang="ts">
	import { onMount, type Snippet } from 'svelte';
	import PanelLeft from '@lucide/svelte/icons/panel-left';
	import Wordmark from '../glyph/Wordmark.svelte';
	import { dismissable } from '../controls/dismissable';
	import type { AccountTab } from '../account';

	/* The frame for every page outside one company: a quiet left rail (wordmark, Home, Account and
	 * whatever account controls a host adds, the owner at the foot) and the page beside it. The rail
	 * is mostly empty on purpose; it is where account-level controls grow. It collapses to icons
	 * (⌘B, the button, or a click on its edge), resizes by dragging its edge, and remembers both in
	 * this browser. On a phone it folds into the top bar the company app wears. */
	let {
		brandName = 'Restless',
		homeHref = '/',
		tabs = [],
		account = null,
		accountMenu = null,
		children
	}: {
		brandName?: string;
		homeHref?: string;
		/** The rail's items, in order. An item's `items` list beneath it, such as a page's sections. */
		tabs?: AccountTab[];
		account?: { name: string; detail?: string } | null;
		/** Items in the owner's menu, below their name. */
		accountMenu?: Snippet | null;
		children: Snippet;
	} = $props();

	const MIN = 200;
	const MAX = 340;
	const DEFAULT = 232;
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

	const initials = $derived(
		(account?.name || '?')
			.split(/\s+/)
			.map((part) => part[0])
			.join('')
			.slice(0, 2)
			.toUpperCase()
	);
</script>

<svelte:window onkeydown={keydown} />

{#snippet item(tab: AccountTab, sub: boolean)}
	{@const Icon = tab.icon}
	{#if tab.form}
		<form method="POST" action={tab.form.action}>
			{#each Object.entries(tab.form.fields ?? {}) as [name, value]}<input
					type="hidden"
					{name}
					{value}
				/>{/each}
			<button
				class="account-tab"
				class:sub
				class:active={tab.active}
				title={collapsed ? tab.label : tab.tooltip}
				aria-label={tab.label}
				>{#if Icon}<Icon size={16} strokeWidth={1.75} aria-hidden="true" />{/if}<span
					class="account-tab-label">{tab.label}</span
				></button
			>
		</form>
	{:else}
		<a
			class="account-tab"
			class:sub
			class:active={tab.active}
			href={tab.href}
			title={collapsed ? tab.label : tab.tooltip}
			aria-label={tab.label}
			aria-current={tab.active ? 'page' : undefined}
			>{#if Icon}<Icon size={16} strokeWidth={1.75} aria-hidden="true" />{/if}<span
				class="account-tab-label">{tab.label}</span
			></a
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
			<a class="account-brand" href={homeHref} aria-label={brandName + ': your companies'}>
				<Wordmark name={brandName} size={16} />
			</a>
			<button
				class="account-collapse"
				type="button"
				onclick={toggle}
				aria-expanded={!collapsed}
				aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
				title={collapsed ? 'Expand sidebar (⌘B)' : 'Collapse sidebar (⌘B)'}
				><PanelLeft size={16} strokeWidth={1.75} aria-hidden="true" /></button
			>
		</div>
		{#if tabs.length}
			<nav class="account-tabs" aria-label="Account navigation">
				{#each tabs as tab (tab.href ?? tab.label)}
					{@render item(tab, false)}
					{#if tab.items?.length && !collapsed}
						<div class="account-subtabs">
							{#each tab.items as subtab (subtab.href ?? subtab.label)}{@render item(
									subtab,
									true
								)}{/each}
						</div>
					{/if}
				{/each}
			</nav>
		{/if}
		{#if account}
			<details class="account-menu" use:dismissable>
				<summary
					class="account-who"
					title={collapsed ? account.name : (account.detail ?? account.name)}
					aria-label="Your account"
				>
					<span class="account-avatar" aria-hidden="true">{initials}</span>
					<span class="account-name">{account.name}</span>
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
		grid-template-columns: var(--account-rail-w, 232px) minmax(0, 1fr);
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
		gap: 14px;
		min-width: 0;
		height: 100vh;
		height: 100dvh;
		padding: 12px 8px 10px;
		border-right: 1px solid var(--border);
		background: var(--surface-rail, var(--bg-app));
	}
	.account-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 6px;
		min-height: 32px;
		padding-left: 8px;
	}
	.account-brand {
		display: inline-flex;
		align-items: center;
		min-width: 0;
		overflow: hidden;
		color: inherit;
		text-decoration: none;
	}
	.account-collapse {
		display: grid;
		flex: none;
		place-items: center;
		width: 28px;
		height: 28px;
		min-width: 0;
		min-height: 0;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
		opacity: 0;
		transition:
			opacity var(--motion-state) var(--ease-standard),
			background var(--motion-state) var(--ease-standard);
	}
	/* Quiet until wanted, like Linear: the toggle shows when the rail is hovered or focused. */
	.account-bar:hover .account-collapse,
	.account-collapse:focus-visible,
	.collapsed .account-collapse {
		opacity: 1;
	}
	.account-collapse:hover {
		background: var(--wash-hover, var(--surface-alt));
		color: var(--ink);
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
	.account-subtabs {
		display: grid;
		gap: 1px;
		margin: 2px 0 8px;
		padding-left: 17px;
	}
	.account-tab {
		position: relative;
		display: flex;
		align-items: center;
		gap: 9px;
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
	.account-tab :global(svg) {
		flex: none;
		color: var(--text-tertiary);
		transition: color var(--motion-state) var(--ease-standard);
	}
	.account-tab-label {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.account-tab.sub {
		height: 28px;
		padding-left: 12px;
		border-left: 1px solid var(--border);
		border-radius: 0 var(--radius-control) var(--radius-control) 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.account-tab:hover {
		background: var(--wash-hover, var(--surface-alt));
		color: var(--ink);
	}
	.account-tab:hover :global(svg),
	.account-tab.active :global(svg) {
		color: var(--ink);
	}
	.account-tab.active {
		background: var(--wash-active, var(--surface-alt));
		color: var(--ink);
		font-weight: 500;
	}
	.account-tab.sub.active {
		border-left-color: var(--ink);
		background: transparent;
	}
	.account-tab:focus-visible,
	.account-who:focus-visible,
	.account-collapse:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: -2px;
	}

	/* Collapsed: icons only, centred; labels, sections and the name step out. */
	.collapsed .account-head {
		flex-direction: column;
		gap: 8px;
		padding-left: 0;
	}
	.collapsed .account-brand :global(.wm-name),
	.collapsed .account-tab-label,
	.collapsed .account-name {
		display: none;
	}
	.collapsed .account-tab {
		justify-content: center;
		padding: 0;
	}
	.collapsed .account-who {
		justify-content: center;
		padding-inline: 0;
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

	.account-menu {
		position: relative;
	}
	.account-who {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
		padding: 6px 8px;
		border-radius: var(--radius-control);
		list-style: none;
		cursor: pointer;
	}
	.account-who:hover {
		background: var(--wash-hover, var(--surface-alt));
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
		width: 24px;
		height: 24px;
		border-radius: 50%;
		background: var(--ink);
		color: var(--surface-raised);
		font-size: var(--t-label);
		font-weight: 600;
	}
	.account-name {
		min-width: 0;
		overflow: hidden;
		font-size: var(--t-body);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	/* The rail's menu opens upward from its foot. */
	.account-menu-panel {
		position: absolute;
		bottom: calc(100% + 6px);
		left: 0;
		z-index: 30;
		min-width: 240px;
		padding: 6px;
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
		padding: 8px 10px 10px;
		border-bottom: 1px solid var(--border);
	}
	.account-menu-who small {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.account-menu-items {
		display: grid;
		padding-top: 6px;
	}
	.account-menu-items :global(:is(a, button)) {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		padding: 7px 10px;
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
	.account-page {
		min-width: 0;
	}
	@media (prefers-reduced-motion: reduce) {
		.account-shell {
			transition: none;
		}
	}

	/* A phone: the rail folds into a top bar (wordmark, items, avatar), as the company app wears it. */
	@media (max-width: 760px) {
		/* Phones give every control a 44px reach (primitives.css); the bar keeps its drawn
		 * proportions and carries the reach as an invisible hit area instead. */
		.account-bar .account-tab,
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
			gap: 14px;
			height: var(--topbar-h, 52px);
			padding: 0 max(14px, env(safe-area-inset-right)) 0 max(14px, env(safe-area-inset-left));
			border-right: 0;
			border-bottom: 1px solid var(--border);
			background: var(--glass);
			backdrop-filter: blur(18px) saturate(1.4);
			-webkit-backdrop-filter: blur(18px) saturate(1.4);
		}
		.account-head {
			padding: 0;
		}
		.account-collapse,
		.account-edge,
		.account-subtabs,
		.account-name,
		.account-tab :global(svg) {
			display: none;
		}
		.collapsed .account-brand,
		.collapsed .account-tab-label {
			display: initial;
		}
		.account-tabs {
			display: flex;
			gap: 2px;
			overflow: visible;
		}
		.account-tab {
			height: auto;
			padding: 6px 10px;
		}
		.account-tab::after {
			content: '';
			position: absolute;
			inset: -8px 0;
		}
		.account-who {
			position: relative;
			padding: 0;
		}
		.account-who:hover {
			background: transparent;
		}
		.account-avatar {
			width: 30px;
			height: 30px;
		}
		.account-who::after {
			content: '';
			position: absolute;
			inset: -7px;
		}
		.account-menu-panel {
			top: calc(100% + 8px);
			right: 0;
			bottom: auto;
			left: auto;
		}
	}
	@media (max-width: 520px) {
		.account-bar {
			gap: 10px;
		}
		.account-brand :global(.wm-name) {
			display: none;
		}
	}
</style>
