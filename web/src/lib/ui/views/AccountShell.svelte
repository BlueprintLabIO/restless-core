<script lang="ts">
	import type { Snippet } from 'svelte';
	import MatrixGlyph, { GLYPHS } from '../glyph/MatrixGlyph.svelte';

	let {
		brandName = 'Restless',
		homeHref = '/',
		actions = null,
		children
	}: {
		brandName?: string;
		homeHref?: string;
		actions?: Snippet | null;
		children: Snippet;
	} = $props();
</script>

<div class="bridge-tokens bridge-root portfolio-root account-shell">
	<header class="account-header" aria-label="Account navigation">
		<a class="account-brand" href={homeHref} aria-label={brandName + ' companies'}>
			<span class="account-mark"><MatrixGlyph rows={GLYPHS.r} size={13} glow /></span>
			<span class="account-name">{brandName}</span>
		</a>
		{#if actions}<div class="account-actions">{@render actions()}</div>{/if}
	</header>
	<div class="account-content">{@render children()}</div>
</div>

<style>
	.account-shell {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		grid-template-rows: var(--topbar-h, 52px) minmax(0, 1fr);
		gap: 6px;
		min-height: 100vh;
		padding: max(var(--app-gutter, 8px), env(safe-area-inset-top))
			max(var(--app-gutter, 8px), env(safe-area-inset-right)) var(--app-gutter, 8px)
			max(var(--app-gutter, 8px), env(safe-area-inset-left));
		box-sizing: border-box;
		isolation: isolate;
	}
	.account-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		min-width: 0;
		padding: 0 16px;
		background: color-mix(in srgb, var(--highlight) 74%, transparent);
		border: 1px solid rgba(57, 66, 84, 0.14);
		border-radius: var(--radius-pane);
		box-shadow: var(--shadow-soft);
	}
	.account-brand {
		display: inline-flex;
		align-items: center;
		gap: 9px;
		min-height: 40px;
		color: inherit;
		text-decoration: none;
		flex: none;
	}
	.account-mark {
		color: var(--intent-direction);
	}
	.account-name {
		font: 400 var(--t-head) var(--font-mark);
		letter-spacing: 0.02em;
	}
	.account-actions {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 10px;
		min-width: 0;
	}
	.account-content {
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		grid-template-rows: minmax(0, 1fr);
		min-width: 0;
		min-height: 0;
	}
	@media (max-width: 640px) {
		.account-shell {
			--app-gutter: 4px;
			gap: 4px;
		}
		.account-header {
			padding-inline: 10px;
			gap: 10px;
		}
		.account-actions {
			gap: 6px;
		}
	}
</style>
