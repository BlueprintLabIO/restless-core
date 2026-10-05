<script lang="ts">
	import type { Snippet } from 'svelte';
	import MatrixGlyph, { GLYPHS } from '../glyph/MatrixGlyph.svelte';
	import { dismissable } from '../controls/dismissable';
	import type { AccountNavGroup, AccountNavItem } from '../account';

	/* The account area outside any one company: a left sidebar that carries navigation, the owner's
	 * identity at its foot, and one raised pane for the page. Below 760px the sidebar becomes a
	 * drawer behind a compact bar, so the page keeps the whole width. Consumers supply the groups;
	 * an item is a link, or a native form POST where entry needs a signed handoff. */
	let {
		brandName = 'Restless',
		homeHref = '/',
		nav = [],
		primary = null,
		account = null,
		accountMenu = null,
		children
	}: {
		brandName?: string;
		homeHref?: string;
		nav?: AccountNavGroup[];
		/** The one creating action, at the top of the sidebar (e.g. New company). */
		primary?: Snippet | null;
		account?: { name: string; detail?: string } | null;
		/** Menu items shown above the account button when it is opened. */
		accountMenu?: Snippet | null;
		children: Snippet;
	} = $props();

	let drawerOpen = $state(false);
	const initials = $derived(
		(account?.name || '?')
			.split(/\s+/)
			.map((part) => part[0])
			.join('')
			.slice(0, 2)
			.toUpperCase()
	);
</script>

<svelte:window onkeydown={(event) => event.key === 'Escape' && (drawerOpen = false)} />

{#snippet itemBody(item: AccountNavItem)}
	{#if item.mark}<span class="account-nav-dot" data-tone={item.mark} aria-hidden="true"
		></span>{:else if item.icon}{@const Icon = item.icon}<Icon
			size={15}
			strokeWidth={1.8}
			aria-hidden="true"
		/>{/if}
	<span class="account-nav-label">{item.label}</span>
	{#if item.badge != null}<span class="account-nav-badge">{item.badge}</span>{/if}
{/snippet}

{#snippet navItem(item: AccountNavItem, sub: boolean)}
	{#if item.form}
		<form method="POST" action={item.form.action}>
			{#each Object.entries(item.form.fields ?? {}) as [name, value]}<input
					type="hidden"
					{name}
					{value}
				/>{/each}
			<button class="account-nav-item" class:sub title={item.tooltip}
				>{@render itemBody(item)}</button
			>
		</form>
	{:else}
		<a
			class="account-nav-item"
			class:sub
			href={item.href}
			title={item.tooltip}
			aria-current={item.active ? 'page' : undefined}
			onclick={() => (drawerOpen = false)}>{@render itemBody(item)}</a
		>
	{/if}
{/snippet}

<div class="bridge-tokens bridge-root portfolio-root account-shell" class:drawer-open={drawerOpen}>
	<header class="account-bar">
		<button
			class="account-bar-toggle"
			type="button"
			aria-label="Open navigation"
			aria-expanded={drawerOpen}
			onclick={() => (drawerOpen = true)}
			><svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true"
				><path d="M2.5 4h11M2.5 8h11M2.5 12h11" stroke="currentColor" stroke-width="1.5" /></svg
			></button
		>
		<a class="account-brand" href={homeHref} aria-label={brandName + ' companies'}>
			<span class="account-mark"><MatrixGlyph rows={GLYPHS.r} size={12} glow /></span>
			<span class="account-name">{brandName}</span>
		</a>
	</header>

	<button
		class="account-scrim"
		type="button"
		tabindex="-1"
		aria-label="Close navigation"
		onclick={() => (drawerOpen = false)}
	></button>

	<aside class="account-sidebar" aria-label="Account navigation">
		<a class="account-brand" href={homeHref} aria-label={brandName + ' companies'}>
			<span class="account-mark"><MatrixGlyph rows={GLYPHS.r} size={12} glow /></span>
			<span class="account-name">{brandName}</span>
		</a>
		{#if primary}<div class="account-primary">{@render primary()}</div>{/if}

		<nav class="account-nav">
			{#each nav as group, index (group.label ?? index)}
				<section class="account-nav-group">
					{#if group.label}<h2 title={group.tooltip}>{group.label}</h2>{/if}
					<ul>
						{#each group.items as item (item.key ?? item.href ?? item.label)}
							<li class:active={item.active}>
								{@render navItem(item, false)}
								{#if item.children?.length}
									<ul class="account-nav-sub">
										{#each item.children as child (child.key ?? child.href ?? child.label)}
											<li class:active={child.active}>{@render navItem(child, true)}</li>
										{/each}
									</ul>
								{/if}
							</li>
						{/each}
					</ul>
				</section>
			{/each}
		</nav>

		{#if account}
			<div class="account-foot">
				{#if accountMenu}
					<details class="account-menu" use:dismissable>
						<summary class="account-button" title={account.detail}>
							<span class="account-avatar" aria-hidden="true">{initials}</span>
							<span class="account-copy"
								><strong>{account.name}</strong>{#if account.detail}<small>{account.detail}</small
									>{/if}</span
							>
						</summary>
						<div class="account-menu-panel">{@render accountMenu()}</div>
					</details>
				{:else}
					<div class="account-button static" title={account.detail}>
						<span class="account-avatar" aria-hidden="true">{initials}</span>
						<span class="account-copy"
							><strong>{account.name}</strong>{#if account.detail}<small>{account.detail}</small
								>{/if}</span
						>
					</div>
				{/if}
			</div>
		{/if}
	</aside>

	<div class="account-content">{@render children()}</div>
</div>

<style>
	.account-shell {
		--account-sidebar-w: 236px;
		display: grid;
		grid-template-columns: var(--account-sidebar-w) minmax(0, 1fr);
		grid-template-rows: minmax(0, 1fr);
		height: 100vh;
		height: 100dvh;
		padding: max(var(--app-gutter, 8px), env(safe-area-inset-top))
			max(var(--app-gutter, 8px), env(safe-area-inset-right)) var(--app-gutter, 8px) 0;
		box-sizing: border-box;
		isolation: isolate;
	}
	.account-bar,
	.account-scrim {
		display: none;
	}

	.account-sidebar {
		position: sticky;
		top: var(--app-gutter, 8px);
		display: flex;
		flex-direction: column;
		gap: 14px;
		height: calc(100vh - 2 * var(--app-gutter, 8px));
		min-height: 0;
		padding: 6px 10px 4px max(10px, env(safe-area-inset-left));
		box-sizing: border-box;
	}
	.account-brand {
		display: inline-flex;
		align-items: center;
		gap: 9px;
		min-height: 32px;
		padding: 0 8px;
		color: inherit;
		text-decoration: none;
	}
	.account-mark {
		color: var(--intent-direction);
	}
	.account-name {
		font: 400 var(--t-head) var(--font-mark);
		letter-spacing: 0.02em;
	}
	.account-primary {
		display: grid;
	}

	.account-nav {
		display: grid;
		align-content: start;
		gap: 18px;
		flex: 1;
		min-width: 0;
		min-height: 0;
		overflow: hidden auto;
		scrollbar-width: thin;
	}
	.account-nav-group h2 {
		margin: 0 0 4px;
		padding: 0 8px;
		color: var(--text-tertiary);
		font: 500 var(--t-label) / 1.4 var(--font-ui);
	}
	.account-nav ul {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: 1px;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.account-nav form {
		display: contents;
	}
	.account-nav-item {
		display: flex;
		align-items: center;
		gap: 9px;
		width: 100%;
		min-height: 30px;
		padding: 0 8px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		font: 450 var(--t-body) / 1.2 var(--font-ui);
		text-align: left;
		text-decoration: none;
		cursor: pointer;
		box-sizing: border-box;
		transition:
			background-color var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard);
	}
	.account-nav-item :global(svg) {
		flex: none;
		color: var(--text-tertiary);
	}
	.account-nav-item:hover {
		background: var(--wash-hover, var(--surface-hover));
		color: var(--ink);
	}
	.active > .account-nav-item,
	.active > form > .account-nav-item {
		background: var(--surface-raised);
		color: var(--ink);
		box-shadow:
			0 0 0 1px var(--border),
			0 1px 2px rgba(41, 50, 68, 0.06);
	}
	.active > .account-nav-item :global(svg) {
		color: var(--ink);
	}
	.account-nav-item:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 1px;
	}
	.account-nav-label {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.account-nav-badge {
		color: var(--text-tertiary);
		font: 500 var(--t-label) var(--font-mono);
	}
	.account-nav-sub {
		margin: 1px 0 4px 15px !important;
		padding-left: 9px !important;
		border-left: 1px solid var(--border);
	}
	.account-nav-item.sub {
		min-height: 27px;
		font-size: var(--t-label);
	}
	.account-nav-dot {
		width: 7px;
		height: 7px;
		flex: none;
		margin: 0 4px;
		border-radius: 50%;
		background: var(--intent-authority);
	}
	.account-nav-dot[data-tone='presence'] {
		background: var(--state-success);
	}
	.account-nav-dot[data-tone='unavailable'] {
		background: var(--state-danger);
	}

	.account-foot {
		padding-top: 8px;
		border-top: 1px solid var(--border);
	}
	.account-menu {
		position: relative;
	}
	.account-button {
		display: flex;
		align-items: center;
		gap: 9px;
		min-width: 0;
		padding: 6px 8px;
		border-radius: var(--radius-control);
		list-style: none;
		cursor: pointer;
		transition: background-color var(--motion-state) var(--ease-standard);
	}
	.account-button.static {
		cursor: default;
	}
	summary.account-button::-webkit-details-marker {
		display: none;
	}
	summary.account-button::before {
		content: none !important;
	}
	summary.account-button:hover,
	.account-menu[open] > summary.account-button {
		background: var(--wash-hover, var(--surface-hover));
	}
	.account-avatar {
		width: 26px;
		height: 26px;
		display: grid;
		flex: none;
		place-items: center;
		border-radius: 50%;
		background: var(--ink);
		color: var(--text-inverse);
		font: 600 var(--t-label)/1 var(--font-mono);
	}
	.account-copy {
		display: grid;
		min-width: 0;
		line-height: 1.25;
	}
	.account-copy strong,
	.account-copy small {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.account-copy strong {
		font-size: var(--t-body);
		font-weight: 550;
	}
	.account-copy small {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.account-menu-panel {
		position: absolute;
		bottom: calc(100% + 6px);
		left: 0;
		right: 0;
		z-index: 40;
		display: grid;
		gap: 1px;
		padding: 5px;
		border: 1px solid var(--border-strong);
		border-radius: 8px;
		background: var(--surface-raised);
		box-shadow: 0 8px 28px #18243a18;
		transform-origin: bottom left;
		animation: account-menu-in var(--motion-disclosure, 320ms) var(--ease-spring, ease) both;
	}
	.account-menu:global([data-closing]) .account-menu-panel {
		animation: account-menu-out var(--motion-state, 180ms) var(--ease-standard, ease) both;
		pointer-events: none;
	}
	.account-menu-panel :global(:is(a, button)) {
		display: flex;
		align-items: center;
		gap: 9px;
		width: 100%;
		min-height: 30px;
		padding: 0 8px;
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
	.account-menu-panel :global(:is(a, button):hover) {
		background: var(--wash-hover, var(--surface-hover));
	}
	.account-menu-panel :global(form) {
		display: contents;
	}
	.account-menu-panel :global(hr) {
		width: 100%;
		margin: 4px 0;
		border: 0;
		border-top: 1px solid var(--border);
	}
	@keyframes account-menu-in {
		from {
			opacity: 0;
			transform: translateY(4px) scale(0.985);
		}
	}
	@keyframes account-menu-out {
		to {
			opacity: 0;
			transform: translateY(3px) scale(0.985);
		}
	}

	.account-content {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		grid-template-rows: minmax(0, 1fr);
		min-width: 0;
		min-height: 0;
		overflow: auto;
		background: color-mix(in srgb, var(--highlight) 70%, transparent);
		border: 1px solid rgba(57, 66, 84, 0.14);
		border-radius: var(--radius-pane);
		box-shadow: var(--shadow-soft);
	}

	@media (max-width: 760px) {
		.account-shell {
			--app-gutter: 4px;
			grid-template-columns: minmax(0, 1fr);
			grid-template-rows: auto minmax(0, 1fr);
			gap: 4px;
			padding-left: max(var(--app-gutter), env(safe-area-inset-left));
		}
		.account-bar {
			display: flex;
			align-items: center;
			gap: 4px;
			min-height: 44px;
			padding: 0 4px;
		}
		.account-bar-toggle {
			display: grid;
			width: 36px;
			height: 36px;
			place-items: center;
			border: 0;
			border-radius: var(--radius-control);
			background: transparent;
			color: var(--ink);
			cursor: pointer;
		}
		.account-bar .account-brand {
			padding-left: 2px;
		}
		.account-sidebar {
			position: fixed;
			inset: 0 auto 0 0;
			z-index: 60;
			width: min(280px, 86vw);
			height: 100%;
			padding: max(12px, env(safe-area-inset-top)) 10px max(10px, env(safe-area-inset-bottom))
				max(10px, env(safe-area-inset-left));
			background: var(--surface-raised);
			border-right: 1px solid var(--border);
			box-shadow: var(--shadow-float, 0 18px 48px #18243a29);
			transform: translateX(-104%);
			transition: transform var(--motion-disclosure, 320ms) var(--ease-spring, ease);
		}
		.drawer-open .account-sidebar {
			transform: none;
		}
		.account-scrim {
			display: block;
			position: fixed;
			inset: 0;
			z-index: 59;
			border: 0;
			background: rgba(24, 36, 58, 0.22);
			opacity: 0;
			pointer-events: none;
			transition: opacity var(--motion-state, 180ms) var(--ease-standard, ease);
		}
		.drawer-open .account-scrim {
			opacity: 1;
			pointer-events: auto;
		}
		.account-nav-item {
			min-height: 38px;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.account-sidebar,
		.account-menu-panel {
			transition: none;
			animation: none;
		}
	}
</style>
