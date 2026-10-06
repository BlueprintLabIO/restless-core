<script lang="ts">
	import type { Snippet } from 'svelte';
	import InfoTip from '../controls/InfoTip.svelte';
	import type { AccountSection } from '../account';

	/* One page for everything that belongs to the owner rather than to a company. The host decides
	 * which sections exist (locally: connections, AI apps, appearance; on Cloud: account, security,
	 * billing, support). A section a host lacks is simply absent, never a disabled imitation. */
	let {
		title = 'Account',
		sections,
		section
	}: {
		title?: string;
		sections: AccountSection[];
		/** Renders one section's body, given its id. */
		section: Snippet<[AccountSection]>;
	} = $props();
	/* One section is a page of its own: its title is the page's, with its explanation on hover. */
	const single = $derived(sections.length === 1 ? sections[0] : null);
</script>

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
	.account-page-section h2 {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 0 0 16px;
		font-size: var(--t-head);
		letter-spacing: -0.01em;
	}
</style>
