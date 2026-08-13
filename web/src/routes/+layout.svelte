<script lang="ts">
	/* The app frame that sits OUTSIDE the design scope: the offline banner and the
	 * loading bar. They live here rather than inside `.bridge-root` because they
	 * report on the app itself, not on the company — which is also why the z-ladder
	 * that positions them is defined on :root (see design/base.css). */

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
	<div class="app-banner app-banner-offline" role="status" aria-live="polite">
		You're offline.
	</div>
{/if}

{@render children()}

<style>
	.app-loading-bar {
		position: fixed;
		inset: 0 0 auto;
		height: 2px;
		z-index: var(--z-app);
		background: rgba(255, 255, 255, 0.06);
	}
	.app-loading-bar-fill {
		height: 100%;
		width: 40%;
		background: #f4f5f6;
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
		z-index: var(--z-app);
		padding: 8px 14px;
		border-radius: 6px;
		font-size: 12.5px;
		background: #1e2227;
		color: #f4f5f6;
		border: 1px solid rgba(255, 255, 255, 0.12);
		box-shadow: 0 12px 32px rgba(0, 0, 0, 0.45);
	}
</style>
