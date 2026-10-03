<script lang="ts">
	import { tooltips } from '$lib/actions/tooltips';
	import { selectMenu } from '$lib/actions/select-menu';
	import { page } from '$app/state';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import AccountShell from '$lib/ui/views/AccountShell.svelte';
	import AccountNavigation from '$lib/components/AccountNavigation.svelte';
	import Palette from '@lucide/svelte/icons/palette';
	import Plug from '@lucide/svelte/icons/plug';
	let { children } = $props();
	const section = $derived(page.url.pathname.split('/').at(-1));
	/* The same account shell as the Companies page and Cloud's account pages:
	 * one header with the account menu, then this section's own navigation. */
	const sections = [
		{ key: 'connections', label: 'Connections', icon: Plug },
		{ key: 'appearance', label: 'Appearance', icon: Palette }
	];
</script>

<svelte:head><title>Account settings — {PRODUCT_NAME}</title></svelte:head>

<div class="settings-root" use:tooltips use:selectMenu>
	<AccountShell brandName={PRODUCT_NAME} homeHref="/">
		{#snippet actions()}<AccountNavigation />{/snippet}
		<div class="settings-frame">
			<aside class="settings-sidebar" aria-label="Account settings">
				<h1>Settings</h1>
				<nav>
					{#each sections as item (item.key)}
						{@const Icon = item.icon}
						<a
							href={`/account/settings/${item.key}`}
							class:active={section === item.key}
							aria-current={section === item.key ? 'page' : undefined}
							><Icon size={15} strokeWidth={1.8} aria-hidden="true" />{item.label}</a
						>
					{/each}
				</nav>
			</aside>
			<div class="settings-content">{@render children()}</div>
		</div>
	</AccountShell>
</div>

<style>
	.settings-root {
		min-height: 100svh;
	}
	.settings-frame {
		display: grid;
		grid-template-columns: 220px minmax(0, 1fr);
		min-height: 0;
		overflow: hidden;
		border: 1px solid rgba(57, 66, 84, 0.14);
		border-radius: var(--radius-pane);
		background: color-mix(in srgb, var(--highlight) 60%, transparent);
		box-shadow: var(--shadow-soft);
	}
	.settings-content {
		min-width: 0;
		min-height: 0;
		overflow: auto;
	}
	.settings-sidebar {
		padding: 22px 12px;
		border-right: 1px solid var(--border);
	}
	.settings-sidebar h1 {
		margin: 0 10px 14px;
		font-size: var(--t-head);
		font-weight: 600;
	}
	.settings-sidebar nav {
		display: grid;
		gap: 2px;
	}
	.settings-sidebar nav a {
		display: flex;
		align-items: center;
		gap: 9px;
		padding: 8px 10px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		font-size: var(--t-body);
		text-decoration: none;
		transition:
			background-color var(--motion-state) var(--ease-standard),
			color var(--motion-state) var(--ease-standard);
	}
	.settings-sidebar nav a:hover {
		color: var(--ink);
		background: var(--surface-alt);
	}
	.settings-sidebar nav a.active {
		color: var(--ink);
		background: var(--surface-alt);
	}
	.settings-sidebar nav a:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}
	@media (max-width: 700px) {
		.settings-frame {
			grid-template-columns: 1fr;
			grid-template-rows: auto minmax(0, 1fr);
		}
		.settings-sidebar {
			padding: 10px 10px 0;
			border-right: 0;
			border-bottom: 1px solid var(--border);
		}
		.settings-sidebar h1 {
			display: none;
		}
		.settings-sidebar nav {
			grid-auto-flow: column;
			grid-auto-columns: 1fr;
		}
		.settings-sidebar nav a {
			justify-content: center;
			border-radius: var(--radius-control) var(--radius-control) 0 0;
		}
		.settings-sidebar nav a.active {
			box-shadow: inset 0 -2px var(--intent-conversation);
		}
	}
</style>
