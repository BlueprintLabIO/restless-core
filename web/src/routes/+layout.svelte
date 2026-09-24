<script lang="ts">
	/* The app frame that sits OUTSIDE the design scope: the offline banner and the
	 * loading bar. They live here rather than inside `.bridge-root` because they
	 * report on the app itself, not on the company — which is also why the z-ladder
	 * that positions them is defined on :root (see design/base.css). */

	import '$lib/design/index.css';
	import { navigating } from '$app/state';
	import { goto } from '$app/navigation';
	import { QueryClient, QueryClientProvider } from '@tanstack/svelte-query';
	import { onMount } from 'svelte';

	let { children } = $props();

	let online = $state(true);
	const isLoading = $derived(navigating.to !== null);
	const pendingDestination = $derived(navigating.to?.url.href ?? '');
	let navigationStalled = $state(false);
	let retryingNavigation = $state(false);
	let navigationDialog: HTMLDialogElement | undefined = $state();
	$effect(() => {
		if (!pendingDestination) {
			navigationStalled = false;
			return;
		}
		navigationStalled = false;
		const timeout = window.setTimeout(() => (navigationStalled = true), 12_000);
		return () => window.clearTimeout(timeout);
	});
	$effect(() => {
		if (navigationStalled && navigationDialog && !navigationDialog.open) {
			navigationDialog.showModal();
		}
	});
	async function retryNavigation() {
		if (!pendingDestination || retryingNavigation) return;
		retryingNavigation = true;
		try {
			await goto(pendingDestination);
		} finally {
			retryingNavigation = false;
		}
	}
	function stayOnCurrentPage() {
		navigationStalled = false;
		navigationDialog?.close();
	}
	const queryClient = new QueryClient({
		defaultOptions: {
			queries: {
				staleTime: 5_000,
				gcTime: 10 * 60_000,
				refetchOnWindowFocus: true,
				retry: 1
			}
		}
	});

	onMount(() => {
		let backgrounded = document.visibilityState === 'hidden';
		let lastReconciledAt = 0;

		const reconcile = () => {
			const now = performance.now();
			if (now - lastReconciledAt < 750) return;
			lastReconciledAt = now;
			/* Backgrounding is normal on mobile. Keep the current route and local
			 * interface state, then refresh only queries that still have observers. */
			void queryClient.invalidateQueries({ refetchType: 'active' });
		};

		const onVisibilityChange = () => {
			if (document.visibilityState === 'hidden') {
				backgrounded = true;
				return;
			}
			if (!backgrounded) return;
			backgrounded = false;
			reconcile();
		};

		const onPageShow = (event: PageTransitionEvent) => {
			if (event.persisted) reconcile();
		};

		document.addEventListener('visibilitychange', onVisibilityChange);
		window.addEventListener('pageshow', onPageShow);
		return () => {
			document.removeEventListener('visibilitychange', onVisibilityChange);
			window.removeEventListener('pageshow', onPageShow);
		};
	});
</script>

<svelte:window bind:online />

{#if isLoading}
	<div class="app-loading-bar" role="status" aria-live="polite" aria-label="Working">
		<div class="app-loading-bar-fill"></div>
	</div>
{/if}

{#if !online}
	<div class="app-banner app-banner-offline" role="status" aria-live="polite">You're offline.</div>
{/if}

{#if navigationStalled}
	<dialog
		bind:this={navigationDialog}
		class="app-navigation-error"
		aria-labelledby="navigation-error-title"
		aria-describedby="navigation-error-copy"
	>
		<div class="app-navigation-error-mark" aria-hidden="true">!</div>
		<h1 id="navigation-error-title">Restless is taking longer than expected</h1>
		<p id="navigation-error-copy">
			This page hasn’t opened yet. Retry the move or stay here with your current page and any unsent
			text.
		</p>
		<button type="button" disabled={retryingNavigation} onclick={retryNavigation}>
			{retryingNavigation ? 'Trying again…' : 'Try again'}
		</button>
		<button class="app-stay-button" type="button" onclick={stayOnCurrentPage}
			>Stay on this page</button
		>
	</dialog>
{/if}

<QueryClientProvider client={queryClient}>
	{@render children()}
</QueryClientProvider>

<style>
	.app-loading-bar {
		position: fixed;
		inset: 0 0 auto;
		height: 2px;
		z-index: var(--z-app);
		background: rgba(47, 108, 168, 0.12);
	}
	.app-loading-bar-fill {
		height: 100%;
		width: 40%;
		background: #2f6ca8;
		animation: slide var(--motion-working) var(--ease-standard) infinite;
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
		font-size: var(--t-body);
		background: rgba(255, 255, 255, 0.94);
		color: #171b24;
		border: 1px solid rgba(48, 57, 74, 0.16);
		box-shadow: 0 12px 32px rgba(43, 51, 66, 0.14);
	}
	.app-navigation-error {
		position: fixed;
		inset: 50% auto auto 50%;
		transform: translate(-50%, -50%);
		width: min(440px, calc(100vw - 32px));
		max-width: none;
		margin: 0;
		padding: clamp(24px, 5vw, 40px);
		border: 1px solid rgba(76, 88, 117, 0.18);
		border-radius: 6px;
		background: #fafbfe;
		box-shadow: 0 8px 28px rgba(65, 76, 104, 0.12);
		color: #293244;
		font: var(--t-body) / 1.55 var(--font-ui);
		text-align: center;
	}
	.app-navigation-error::backdrop {
		background: rgba(26, 33, 47, 0.3);
		backdrop-filter: blur(7px) saturate(0.82);
		-webkit-backdrop-filter: blur(7px) saturate(0.82);
	}
	.app-navigation-error-mark {
		width: 40px;
		height: 40px;
		display: grid;
		place-items: center;
		margin: 0 auto 12px;
		border: 1px solid rgba(155, 84, 91, 0.25);
		border-radius: 4px;
		background: #f7e9eb;
		color: #9b545b;
		font: 600 var(--t-head) var(--font-mono);
	}
	.app-navigation-error h1 {
		margin: 0;
		font-size: var(--t-title);
		font-weight: 600;
		line-height: 1.25;
	}
	.app-navigation-error p {
		max-width: 34ch;
		margin: 12px auto 20px;
		color: rgba(23, 27, 36, 0.66);
	}
	.app-navigation-error button {
		padding: 9px 18px;
		border: 1px solid #cdd5e2;
		border-radius: 4px;
		background: #e8eff8;
		box-shadow: 0 1px 2px rgba(65, 76, 104, 0.025);
		color: #456687;
		font: 600 var(--t-body) var(--font-ui);
		min-width: 140px;
		cursor: pointer;
	}
	.app-navigation-error button:disabled {
		opacity: 0.55;
		cursor: wait;
	}
	.app-navigation-error button:focus-visible {
		outline: 2px solid #456687;
		outline-offset: 2px;
	}
	.app-navigation-error .app-stay-button {
		margin: 12px 0 0 8px;
		border-color: transparent;
		background: transparent;
		box-shadow: none;
		color: #456687;
	}
</style>
