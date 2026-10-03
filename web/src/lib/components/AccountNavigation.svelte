<script lang="ts">
	/* The account controls every account page shares, after Cloud's
	 * AccountNavigation: a way back to the companies and one menu for the
	 * owner's account. Locally the account is this computer, so the menu says
	 * which appliance it is rather than showing an email and a sign-out. */
	import { page } from '$app/state';
	import { dismissable } from '$lib/ui/controls/dismissable';
	import { getApplianceStatus, type ApplianceStatus } from '$lib/model/appliance';
	import Building2 from '@lucide/svelte/icons/building-2';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import Palette from '@lucide/svelte/icons/palette';
	import Plug from '@lucide/svelte/icons/plug';

	let { appliance = null }: { appliance?: ApplianceStatus | null } = $props();
	let observed = $state<ApplianceStatus | null>(null);
	const status = $derived(appliance ?? observed);
	const home = $derived(page.url.pathname === '/');
	const profile = $derived(
		status?.profile === 'dev'
			? 'Development profile'
			: status?.profile === 'test'
				? 'Test profile'
				: 'Local appliance'
	);

	$effect(() => {
		if (appliance) return;
		const controller = new AbortController();
		void getApplianceStatus(controller.signal)
			.then((value) => (observed = value))
			.catch(() => {
				// The menu still works without the profile name.
			});
		return () => controller.abort();
	});

	const items = [
		{ href: '/account/settings/connections', label: 'Connections', icon: Plug },
		{ href: '/account/settings/appearance', label: 'Appearance', icon: Palette }
	];
</script>

<nav class="account-navigation" aria-label="Account">
	{#if !home}<a class="btn account-companies" href="/"
			><Building2 size={15} strokeWidth={1.8} aria-hidden="true" /><span>Companies</span></a
		>{/if}
	<details class="account-menu" use:dismissable>
		<summary class="btn" aria-label="Account menu" title="Account">
			<span class="account-avatar" aria-hidden="true">Y</span><span class="account-label">You</span
			><ChevronDown size={14} strokeWidth={1.8} aria-hidden="true" />
		</summary>
		<div class="account-menu-panel">
			<div class="account-identity">
				<strong>You</strong>
				<span>{profile} · this computer</span>
			</div>
			{#each items as item (item.href)}
				{@const Icon = item.icon}
				<a href={item.href} aria-current={page.url.pathname === item.href ? 'page' : undefined}
					><Icon size={15} strokeWidth={1.8} aria-hidden="true" />{item.label}</a
				>
			{/each}
		</div>
	</details>
</nav>

<style>
	.account-navigation {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}
	.account-companies {
		gap: 7px;
	}
	.account-menu {
		position: relative;
	}
	.account-menu summary {
		gap: 7px;
		padding-inline: 6px 9px;
		list-style: none;
		cursor: pointer;
	}
	.account-menu summary::-webkit-details-marker {
		display: none;
	}
	.account-menu summary::before {
		content: none !important;
	}
	.account-avatar {
		width: 22px;
		height: 22px;
		display: grid;
		place-items: center;
		border-radius: 50%;
		background: var(--ink);
		color: var(--text-inverse);
		font: 600 var(--t-label)/1 var(--font-mono);
	}
	.account-menu-panel {
		position: absolute;
		top: calc(100% + 6px);
		right: 0;
		z-index: 40;
		display: grid;
		gap: 2px;
		min-width: 230px;
		max-width: calc(100vw - 24px);
		padding: 6px;
		border: 1px solid var(--border-strong);
		border-radius: 8px;
		background: var(--surface-raised);
		box-shadow: 0 8px 28px #18243a18;
		animation: bridge-popover-in var(--motion-disclosure) var(--ease-spring) both;
	}
	/* `dismissable` marks the closing menu at runtime. */
	.account-menu:global([data-closing]) .account-menu-panel {
		animation: account-menu-out var(--motion-state) var(--ease-standard) both;
	}
	@keyframes account-menu-out {
		to {
			opacity: 0;
			transform: translateY(-3px) scale(0.985);
		}
	}
	.account-identity {
		display: grid;
		gap: 2px;
		margin-bottom: 4px;
		padding: 8px 10px 10px;
		border-bottom: 1px solid var(--border);
	}
	.account-identity strong {
		font-size: var(--t-body);
	}
	.account-identity span {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.account-menu-panel a {
		display: flex;
		align-items: center;
		gap: 9px;
		min-height: 34px;
		padding: 7px 10px;
		border-radius: 4px;
		color: var(--ink);
		font-size: var(--t-body);
		text-decoration: none;
	}
	.account-menu-panel a:hover,
	.account-menu-panel a[aria-current='page'] {
		background: var(--surface-alt);
	}
	.account-menu-panel a:focus-visible {
		outline: 2px solid var(--intent-feedback);
		outline-offset: -2px;
	}
	@media (max-width: 640px) {
		.account-companies span,
		.account-label {
			display: none;
		}
	}
	@media (pointer: coarse) {
		.account-menu-panel a {
			min-height: 44px;
		}
	}
</style>
