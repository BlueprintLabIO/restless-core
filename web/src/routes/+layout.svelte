<script lang="ts">
	/* The app frame outside every surface: the design language, and the two
	 * things that report on the app itself rather than on the company. */

	import '$lib/design/index.css';
	import { navigating } from '$app/state';

	let { children } = $props();

	let online = $state(true);
	const isLoading = $derived(navigating.to !== null);
</script>

<svelte:window bind:online />

{#if isLoading}
	<div class="app-loading-bar" role="status" aria-live="polite" aria-label="Working">
		<div class="app-loading-bar-fill"></div>
	</div>
{/if}

{#if !online}
	<div class="app-banner" role="status" aria-live="polite">You're offline.</div>
{/if}

{@render children()}

<style>
	.app-loading-bar {
		position: fixed;
		inset: 0 0 auto;
		height: 2px;
		z-index: var(--z-overlay);
		background: var(--surface-alt);
	}
	.app-loading-bar-fill {
		height: 100%;
		width: 40%;
		background: var(--accent);
		animation: slide 1.1s ease-in-out infinite;
	}
	@keyframes slide {
		0% {
			transform: translateX(-100%);
		}
		100% {
			transform: translateX(250%);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.app-loading-bar-fill {
			animation: none;
			width: 100%;
			opacity: 0.4;
		}
	}
	.app-banner {
		position: fixed;
		left: 50%;
		bottom: 18px;
		transform: translateX(-50%);
		z-index: var(--z-overlay);
		padding: 8px 14px;
		border-radius: var(--radius-md);
		background: var(--surface);
		border: 1px solid var(--border-strong);
		box-shadow: 0 12px 32px -12px var(--shadow-color);
		font-size: 12.5px;
	}
</style>
