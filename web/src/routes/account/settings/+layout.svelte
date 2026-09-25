<script lang="ts">
	import { tooltips } from '$lib/actions/tooltips';
	import { page } from '$app/state';
	import { PRODUCT_NAME } from '$lib/brand/brand';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	let { children } = $props();
	const section = $derived(page.url.pathname.split('/').at(-1));
</script>

<svelte:head><title>Account settings — {PRODUCT_NAME}</title></svelte:head>

<div class="bridge-root portfolio-root settings-root" use:tooltips>
	<header class="bridge-topbar" aria-label="Account navigation">
		<a class="tb-brand portfolio-brand" href="/" aria-label={`${PRODUCT_NAME} companies`}>
			<span class="tb-mark"><MatrixGlyph rows={GLYPHS.r} size={13} glow /></span>
			<span class="tb-name">{PRODUCT_NAME}</span>
		</a>
		<a class="back-link" href="/">Back to companies</a>
	</header>
	<div class="settings-frame">
		<aside class="settings-sidebar" aria-label="Account settings">
			<h1>Settings</h1>
			<nav>
				<a
					href="/account/settings/connections"
					class:active={section === 'connections'}
					aria-current={section === 'connections' ? 'page' : undefined}>Connections</a
				>
				<a
					href="/account/settings/appearance"
					class:active={section === 'appearance'}
					aria-current={section === 'appearance' ? 'page' : undefined}>Appearance</a
				>
			</nav>
		</aside>
		{@render children()}
	</div>
</div>

<style>
	.settings-root {
		min-height: 100svh;
	}
	.back-link {
		margin-left: auto;
		color: var(--text-secondary);
		font-size: var(--t-label);
		text-decoration: none;
	}
	.back-link:hover {
		color: var(--intent-conversation);
	}
	.settings-frame {
		display: grid;
		grid-template-columns: 220px minmax(0, 1fr);
		min-height: calc(100svh - 58px);
	}
	.settings-sidebar {
		padding: 42px 20px 28px max(24px, calc((100vw - 1200px) / 2));
		border-right: 1px solid var(--border);
	}
	.settings-sidebar h1 {
		margin: 0 0 22px;
		font-size: var(--t-head);
		font-weight: 600;
	}
	.settings-sidebar nav {
		display: grid;
		gap: 4px;
	}
	.settings-sidebar nav a {
		padding: 10px 12px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		font-size: var(--t-label);
		text-decoration: none;
	}
	.settings-sidebar nav a:hover {
		color: var(--text-primary);
		background: var(--surface-alt);
	}
	.settings-sidebar nav a.active {
		color: var(--text-primary);
		background: var(--surface-alt);
		box-shadow: inset 2px 0 var(--intent-conversation);
	}
	.settings-sidebar nav a:focus-visible,
	.back-link:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}
	@media (max-width: 700px) {
		.settings-frame {
			grid-template-columns: 1fr;
			align-content: start;
		}
		.settings-sidebar {
			padding: 16px 20px 0;
			border-right: 0;
		}
		.settings-sidebar h1 {
			margin: 0 0 10px;
		}
		.settings-sidebar nav {
			grid-template-columns: 1fr;
			border-bottom: 1px solid var(--border);
		}
		.settings-sidebar nav a {
			text-align: center;
		}
		.settings-sidebar nav a.active {
			box-shadow: inset 0 -2px var(--intent-conversation);
		}
	}
</style>
