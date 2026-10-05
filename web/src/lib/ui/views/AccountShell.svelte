<script lang="ts">
	import type { Snippet } from 'svelte';
	import Wordmark from '../glyph/Wordmark.svelte';
	import { dismissable } from '../controls/dismissable';
	import type { AccountTab } from '../account';

	/* The frame for every page outside one company: the same top bar the company app wears
	 * (wordmark, a few tabs, the owner's menu) and the page beneath it at full width. There is
	 * no sidebar; the account area is small enough that tabs and one Account page cover it. */
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

<div class="bridge-tokens account-shell">
	<header class="account-bar">
		<a class="account-brand" href={homeHref} aria-label={brandName + ': your companies'}>
			<Wordmark name={brandName} size={16} />
		</a>
		{#if tabs.length}
			<nav class="account-tabs" aria-label="Account navigation">
				{#each tabs as tab (tab.href ?? tab.label)}
					{#if tab.form}
						<form method="POST" action={tab.form.action}>
							{#each Object.entries(tab.form.fields ?? {}) as [name, value]}<input
									type="hidden"
									{name}
									{value}
								/>{/each}
							<button class="account-tab" class:active={tab.active} title={tab.tooltip}
								>{tab.label}</button
							>
						</form>
					{:else}
						<a
							class="account-tab"
							class:active={tab.active}
							href={tab.href}
							title={tab.tooltip}
							aria-current={tab.active ? 'page' : undefined}>{tab.label}</a
						>
					{/if}
				{/each}
			</nav>
		{/if}
		{#if account}
			<details class="account-menu" use:dismissable>
				<summary class="account-avatar" title={account.name} aria-label="Your account">
					{initials}
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
		align-items: center;
		gap: 20px;
		height: var(--topbar-h, 52px);
		padding: 0 max(14px, env(safe-area-inset-right)) 0 max(14px, env(safe-area-inset-left));
		border-bottom: 1px solid var(--border);
		background: var(--glass);
		backdrop-filter: blur(18px) saturate(1.4);
		-webkit-backdrop-filter: blur(18px) saturate(1.4);
	}
	.account-brand {
		display: inline-flex;
		align-items: center;
		color: inherit;
		text-decoration: none;
	}
	.account-tabs {
		display: flex;
		align-items: center;
		gap: 2px;
		margin-right: auto;
	}
	.account-tabs form {
		display: contents;
	}
	.account-tab {
		padding: 6px 10px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		font-size: var(--t-body);
		text-decoration: none;
		cursor: pointer;
		transition: background var(--motion-state) var(--ease-standard);
	}
	.account-tab:hover {
		background: var(--wash-hover, var(--surface-alt));
		color: var(--ink);
	}
	.account-tab.active {
		background: var(--surface-raised);
		box-shadow: 0 0 0 1px var(--border);
		color: var(--ink);
		font-weight: 500;
	}
	.account-tab:focus-visible,
	.account-avatar:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}
	.account-menu {
		position: relative;
		margin-left: auto;
	}
	.account-tabs + .account-menu {
		margin-left: 0;
	}
	.account-avatar {
		display: grid;
		place-items: center;
		width: 30px;
		height: 30px;
		border-radius: 50%;
		background: var(--ink);
		color: var(--surface-raised);
		font-size: var(--t-label);
		font-weight: 600;
		list-style: none;
		cursor: pointer;
	}
	.account-avatar {
		position: relative;
		min-width: 0;
		min-height: 0;
		aspect-ratio: 1;
	}
	/* A 30px circle, but a 44px target for a thumb. */
	.account-avatar::after {
		content: '';
		position: absolute;
		inset: -7px;
	}
	.account-avatar::-webkit-details-marker,
	.account-avatar::marker {
		display: none;
		content: none;
	}
	.account-avatar::before {
		display: none;
	}
	.account-menu-panel {
		position: absolute;
		top: calc(100% + 8px);
		right: 0;
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
	/* Phones give every control a 44px reach (primitives.css); keep that reach as an invisible hit
	 * area and the bar's proportions as drawn. */
	.account-bar .account-tab,
	.account-menu > .account-avatar {
		min-width: 0;
		min-height: 0;
	}
	.account-menu > .account-avatar {
		width: 30px;
		height: 30px;
		padding: 0;
	}
	.account-tab {
		position: relative;
	}
	.account-tab::after {
		content: '';
		position: absolute;
		inset: -8px 0;
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
