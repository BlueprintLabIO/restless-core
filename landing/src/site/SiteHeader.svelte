<script lang="ts">
	import Star from '@lucide/svelte/icons/star';
	import Wordmark from '$lib/ui/glyph/Wordmark.svelte';
	import { GITHUB_URL } from './config';
</script>

<header class="bar">
	<a class="brand" href="/" aria-label="Restless home"><Wordmark size={16} restless /></a>
	<nav aria-label="Primary">
		<a href="#inside">What’s inside</a>
		<a href="#start">Get started</a>
		<a class="gh" href={GITHUB_URL}><Star size={14} strokeWidth={2.4} /> GitHub</a>
	</nav>
</header>

<style>
	.bar {
		position: fixed;
		inset: max(12px, env(safe-area-inset-top)) clamp(12px, 3vw, 40px) auto;
		z-index: var(--z-sticky, 40);
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		padding: 6px 8px 6px 14px;
		border: 1px solid rgba(255, 255, 255, 0.22);
		border-radius: 8px;
		background: rgba(20, 24, 44, 0.62);
		backdrop-filter: blur(12px);
		color: #fff;
	}
	.brand {
		display: inline-flex;
		align-items: center;
		min-height: 40px;
		color: #fff;
		text-decoration: none;
	}
	nav {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	nav a {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		min-height: 36px;
		padding: 0 12px;
		border-radius: var(--radius-control);
		color: #fff;
		font: 600 13px var(--font-ui);
		text-decoration: none;
	}
	nav a:hover {
		background: rgba(255, 255, 255, 0.14);
	}
	nav a:focus-visible {
		outline: 2px solid #fff;
		outline-offset: 2px;
	}
	.gh {
		border: 1px solid rgba(255, 255, 255, 0.5);
	}
	@media (max-width: 560px) {
		nav a:not(.gh) {
			display: none;
		}
	}
</style>
