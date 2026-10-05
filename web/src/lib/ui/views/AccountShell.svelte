<script lang="ts">
	import type { Snippet } from 'svelte';
	import Wordmark from '../glyph/Wordmark.svelte';
	import { dismissable } from '../controls/dismissable';
	import type { AccountTab } from '../account';

	/* The frame for every page outside one company: a quiet left rail (wordmark, Home, Account and
	 * whatever account controls a host adds, the owner at the foot) and the page beside it. The rail
	 * is mostly empty on purpose; it is where account-level controls grow. On a phone it folds into
	 * the top bar the company app wears. */
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

	const initials = $derived(
		(account?.name || '?')
			.split(/\s+/)
			.map((part) => part[0])
			.join('')
			.slice(0, 2)
			.toUpperCase()
	);
</script>

{#snippet item(tab: AccountTab, sub: boolean)}
	{#if tab.form}
		<form method="POST" action={tab.form.action}>
			{#each Object.entries(tab.form.fields ?? {}) as [name, value]}<input
					type="hidden"
					{name}
					{value}
				/>{/each}
			<button class="account-tab" class:sub class:active={tab.active} title={tab.tooltip}
				>{tab.label}</button
			>
		</form>
	{:else}
		<a
			class="account-tab"
			class:sub
			class:active={tab.active}
			href={tab.href}
			title={tab.tooltip}
			aria-current={tab.active ? 'page' : undefined}>{tab.label}</a
		>
	{/if}
{/snippet}

<div class="bridge-tokens account-shell">
	<header class="account-bar">
		<a class="account-brand" href={homeHref} aria-label={brandName + ': your companies'}>
			<Wordmark name={brandName} size={16} />
		</a>
		{#if tabs.length}
			<nav class="account-tabs" aria-label="Account navigation">
				{#each tabs as tab (tab.href ?? tab.label)}
					{@render item(tab, false)}
					{#if tab.items?.length}
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
					title={account.detail ?? account.name}
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
	</header>
	<div class="account-page">{@render children()}</div>
</div>

<style>
	.account-shell {
		--account-rail-w: 232px;
		display: grid;
		grid-template-columns: var(--account-rail-w) minmax(0, 1fr);
		min-height: 100vh;
		min-height: 100dvh;
		background: var(--bg-app);
		color: var(--ink);
	}
	.account-bar {
		position: sticky;
		top: 0;
		z-index: var(--z-sticky, 20);
		display: flex;
		flex-direction: column;
		gap: 18px;
		height: 100vh;
		height: 100dvh;
		padding: 16px 10px 12px;
		border-right: 1px solid var(--border);
		background: var(--surface-rail, var(--bg-app));
	}
	.account-brand {
		display: inline-flex;
		align-items: center;
		align-self: flex-start;
		padding: 2px 8px;
		color: inherit;
		text-decoration: none;
	}
	.account-tabs {
		display: grid;
		gap: 1px;
		align-content: start;
		flex: 1 1 auto;
		min-height: 0;
		overflow-y: auto;
	}
	.account-tabs form {
		display: contents;
	}
	.account-subtabs {
		display: grid;
		gap: 1px;
		margin: 1px 0 6px;
	}
	.account-tab {
		position: relative;
		display: flex;
		align-items: center;
		min-height: 30px;
		padding: 0 10px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--t-body);
		text-align: left;
		text-decoration: none;
		cursor: pointer;
		transition:
			background var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard);
	}
	.account-tab.sub {
		min-height: 26px;
		padding-left: 22px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.account-tab:hover {
		background: var(--wash-hover, var(--surface-alt));
		color: var(--ink);
	}
	.account-tab.active {
		background: var(--surface-raised);
		box-shadow: var(--control-depth, 0 0 0 1px var(--border));
		color: var(--ink);
		font-weight: 500;
	}
	.account-tab:focus-visible,
	.account-who:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}
	.account-menu {
		position: relative;
	}
	.account-who {
		display: flex;
		align-items: center;
		gap: 10px;
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
		width: 26px;
		height: 26px;
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
		.account-brand {
			align-self: center;
			padding: 0;
		}
		.account-tabs {
			display: flex;
			gap: 2px;
			overflow: visible;
		}
		.account-subtabs,
		.account-name {
			display: none;
		}
		.account-tab {
			min-height: 0;
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
