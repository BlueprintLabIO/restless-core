<script lang="ts">
	import type { Snippet } from 'svelte';
	import InfoTip from '../controls/InfoTip.svelte';
	import type { AccountGroup, AccountSection } from '../account';

	/* One page for everything that belongs to the owner rather than to a company. The host decides
	 * which sections exist (locally: connections, AI apps, appearance; on Cloud: account, security,
	 * billing, support). A section a host lacks is simply absent, never a disabled imitation. */
	let {
		title = 'Account',
		sections,
		nav = null,
		section
	}: {
		title?: string;
		sections: AccountSection[];
		/** Settings' own list, after Firecrawl's: every section of settings, grouped, beside the one
		 * shown. The account rail stays as it is; this list is how settings moves between pages. */
		nav?: AccountGroup[] | null;
		/** Renders one section's body, given its id. */
		section: Snippet<[AccountSection]>;
	} = $props();
	/* One section is a page of its own: its title is the page's, with its explanation on hover. */
	const single = $derived(sections.length === 1 ? sections[0] : null);
</script>

{#if nav}
	<main class="account-page-main settings">
		<h1 class="settings-title">{title}</h1>
		<div class="settings-grid">
			<nav class="settings-nav" aria-label={title}>
				{#each nav as group (group.label)}
					<div class="settings-nav-group" role="group" aria-label={group.label}>
						<span class="settings-nav-label">{group.label}</span>
						{#each group.items as item (item.href ?? item.label)}
							{@const Icon = item.icon}
							<a
								href={item.href}
								class:active={item.active}
								aria-current={item.active ? 'page' : undefined}
								title={item.tooltip}
								>{#if Icon}<Icon size={15} strokeWidth={1.75} aria-hidden="true" />{/if}<span
									>{item.label}</span
								></a
							>
						{/each}
					</div>
				{/each}
			</nav>
			<div class="settings-body">
				{#each sections as item (item.id)}
					<section class="settings-section" id={item.id} aria-labelledby={`${item.id}-title`}>
						<h2 id={`${item.id}-title`}>
							{item.title}{#if item.tooltip}<InfoTip text={item.tooltip} />{/if}
						</h2>
						{@render section(item)}
					</section>
				{/each}
			</div>
		</div>
	</main>
{:else}
	<main class="account-page-main">
		<header class="account-page-head">
			<h1>
				{single?.title ?? title}{#if single?.tooltip}<InfoTip text={single.tooltip} />{/if}
			</h1>
			{#if sections.length >= 4}
				<nav class="account-page-index" aria-label="Sections">
					{#each sections as item (item.id)}<a href={`#${item.id}`}>{item.title}</a>{/each}
				</nav>
			{/if}
		</header>
		{#if single}
			<section class="account-page-single" id={single.id}>{@render section(single)}</section>
		{:else}{#each sections as item (item.id)}
				<section class="account-page-section" id={item.id} aria-labelledby={`${item.id}-title`}>
					<h2 id={`${item.id}-title`}>
						{item.title}{#if item.tooltip}<InfoTip text={item.tooltip} />{/if}
					</h2>
					{@render section(item)}
				</section>
			{/each}{/if}
	</main>
{/if}

<style>
	.account-page-main {
		width: min(820px, 100%);
		margin-inline: auto;
		padding: clamp(24px, 4vw, 48px) clamp(16px, 4vw, 32px) 72px;
	}
	.account-page-head {
		display: grid;
		gap: 14px;
		margin-bottom: 28px;
	}
	.account-page-head h1 {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 0;
		font-size: var(--t-title);
		font-weight: 600;
		line-height: 1.2;
		letter-spacing: -0.02em;
	}
	.account-page-index {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
	}
	.account-page-index a {
		padding: 4px 10px;
		border: 1px solid var(--border);
		border-radius: 999px;
		color: var(--text-secondary);
		font-size: var(--t-label);
		text-decoration: none;
	}
	/* Beside the account rail its sections are listed there already. */
	@media (min-width: 761px) {
		:global(.account-shell) .account-page-index {
			display: none;
		}
	}
	.account-page-index a:hover {
		border-color: var(--border-strong);
		color: var(--ink);
	}
	.account-page-section {
		padding: 24px 0;
		border-top: 1px solid var(--border);
		scroll-margin-top: calc(var(--topbar-h, 52px) + 12px);
	}
	/* Settings: its own list beside the section, as Firecrawl's settings are. */
	.account-page-main.settings {
		width: min(1040px, 100%);
	}
	.settings-title {
		margin: 0 0 24px;
		font-size: var(--t-title);
		font-weight: 600;
		line-height: 1.2;
		letter-spacing: -0.02em;
	}
	.settings-grid {
		display: grid;
		grid-template-columns: 188px minmax(0, 1fr);
		gap: 40px;
		align-items: start;
	}
	.settings-nav {
		position: sticky;
		top: 24px;
		display: grid;
		gap: 14px;
	}
	.settings-nav-group {
		display: grid;
		gap: 1px;
	}
	.settings-nav-label {
		padding: 0 8px 4px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 500;
	}
	.settings-nav a {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 30px;
		padding: 0 8px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		font-size: var(--t-body);
		text-decoration: none;
		transition:
			background var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard);
	}
	.settings-nav a :global(svg) {
		flex: none;
		color: var(--text-tertiary);
	}
	.settings-nav a:hover {
		background: var(--wash-hover, var(--surface-alt));
		color: var(--ink);
	}
	.settings-nav a.active {
		background: color-mix(in srgb, var(--ink) 6%, transparent);
		color: var(--ink);
		font-weight: 500;
	}
	.settings-nav a:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: -2px;
	}
	.settings-section h2 {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 0 0 16px;
		font-size: var(--t-head);
		letter-spacing: -0.01em;
	}
	/* A narrow page: the list becomes one row that scrolls sideways above the section. */
	@container account-page (max-width: 720px) {
		.settings-grid {
			grid-template-columns: minmax(0, 1fr);
			gap: 20px;
		}
		.settings-nav {
			position: static;
			display: flex;
			gap: 2px;
			overflow-x: auto;
			scrollbar-width: none;
		}
		.settings-nav-group {
			display: flex;
			gap: 2px;
		}
		.settings-nav-label {
			display: none;
		}
		.settings-nav a {
			flex: none;
			white-space: nowrap;
		}
	}
	.account-page-section h2 {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 0 0 16px;
		font-size: var(--t-head);
		letter-spacing: -0.01em;
	}
</style>
