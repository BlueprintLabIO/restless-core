<script lang="ts">
	/* The app frame that sits OUTSIDE the design scope: the offline banner and the
	 * loading bar. They live here rather than inside `.bridge-root` because they
	 * report on the app itself, not on the company — which is also why the z-ladder
	 * that positions them is defined on :root (see design/base.css). */

	import '$lib/design/index.css';
	import { navigating } from '$app/state';
	import { beforeNavigate, goto, onNavigate } from '$app/navigation';
	import {
		MutationCache,
		QueryCache,
		QueryClient,
		QueryClientProvider
	} from '@tanstack/svelte-query';
	import { onMount } from 'svelte';
	import { fade } from 'svelte/transition';
	import { connection, observeFailure, observeSuccess } from '$lib/model/connection.svelte';
	import { isRetryable } from '$lib/model/failure';
	import { observeSessionEnd } from '$lib/model/session-reentry';
	import { accountServicePath } from '$lib/model/account-service';
	import { accountServiceOrigin } from '$lib/model/appliance';
	import { registerServiceWorkerWhereLocal } from '$lib/model/service-worker-registration';
	import { dev } from '$app/environment';

	let { children } = $props();

	/* Moving between surfaces crossfades the work area while the chrome stays
	 * put (their view-transition names live in design/motion.css). Query-only
	 * changes — selecting an item, switching a lens — update in place. */
	onNavigate((navigation) => {
		if (
			!document.startViewTransition ||
			window.matchMedia('(prefers-reduced-motion: reduce)').matches ||
			navigation.from?.url.pathname === navigation.to?.url.pathname
		)
			return;
		return new Promise((resolve) => {
			document.startViewTransition(async () => {
				resolve();
				await navigation.complete;
			});
		});
	});

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
	let restored = $state(false);
	let restoredTimer: ReturnType<typeof setTimeout> | undefined;
	function succeeded() {
		if (!observeSuccess()) return;
		/* Anything read during the outage may be behind: refresh what is on
		 * screen once, then confirm briefly. */
		void queryClient.invalidateQueries({ refetchType: 'active' });
		restored = true;
		clearTimeout(restoredTimer);
		restoredTimer = setTimeout(() => (restored = false), 2_400);
	}
	const queryClient = new QueryClient({
		queryCache: new QueryCache({ onError: observeFailure, onSuccess: succeeded }),
		mutationCache: new MutationCache({ onError: observeFailure, onSuccess: succeeded }),
		defaultOptions: {
			queries: {
				// Private networks and localhost can work while the OS reports no Internet.
				// Attempt the real endpoint; browser online state is not access evidence.
				networkMode: 'always',
				staleTime: 5_000,
				gcTime: 10 * 60_000,
				refetchOnWindowFocus: true,
				/* Retry only what a second attempt can fix. A 404 or a rejected
				 * request is shown at once rather than after a pointless wait. */
				retry: (failureCount, error) => failureCount < 3 && isRetryable(error),
				retryDelay: (attempt) => Math.min(500 * 2 ** attempt, 4_000)
			}
		}
	});
	function reconnectNow() {
		void queryClient.refetchQueries({ type: 'active' });
	}

	/* A hosted session that ends mid-use re-enters through the account issuer. */
	onMount(observeSessionEnd);

	/* On Cloud's one address, Home, Profile, Security, Plan and Support are the account service's
	 * pages: a link to one leaves this app with a full load instead of being routed here. Whether
	 * this plane is under that address is asked only when such a link is followed, so no other page
	 * pays a request for it. */
	let accountService: string | null | undefined;
	beforeNavigate((navigation) => {
		const to = navigation.to?.url;
		if (
			!to ||
			to.origin !== window.location.origin ||
			!accountServicePath(to.pathname) ||
			accountService === null
		)
			return;
		navigation.cancel();
		if (accountService) {
			window.location.assign(to.href);
			return;
		}
		void accountServiceOrigin().then((origin) => {
			accountService = origin;
			if (origin) window.location.assign(to.href);
			else void goto(`${to.pathname}${to.search}${to.hash}`);
		});
	});
	onMount(() => {
		if (!dev) void registerServiceWorkerWhereLocal().catch(() => {});
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

{#if !online && connection.lost}
	<div class="app-banner" role="status" aria-live="polite" transition:fade={{ duration: 160 }}>
		You're offline.
	</div>
{:else if connection.lost}
	<div class="app-banner" role="status" aria-live="polite" transition:fade={{ duration: 160 }}>
		<span class="app-banner-dot" aria-hidden="true"></span>
		Connection lost. Reconnecting…
		<button type="button" onclick={reconnectNow}>Retry now</button>
	</div>
{:else if restored}
	<div class="app-banner" role="status" aria-live="polite" transition:fade={{ duration: 160 }}>
		Reconnected.
	</div>
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
		background: var(--app-surface);
		color: var(--app-ink);
		border: 1px solid var(--app-edge);
		box-shadow: 0 12px 32px rgba(43, 51, 66, 0.14);
		display: flex;
		align-items: center;
		gap: 10px;
		white-space: nowrap;
	}
	.app-banner button {
		margin: -4px -8px -4px 2px;
		padding: 4px 8px;
		border: 0;
		border-radius: 4px;
		background: transparent;
		color: var(--app-accent);
		font: 600 var(--t-body) var(--font-ui);
		cursor: pointer;
	}
	.app-banner button:hover {
		background: var(--app-accent-soft);
	}
	.app-banner button:focus-visible {
		outline: 2px solid var(--app-accent);
		outline-offset: 1px;
	}
	.app-banner-dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		background: var(--app-danger);
		animation: app-banner-pulse var(--motion-working) ease-in-out infinite;
	}
	@keyframes app-banner-pulse {
		50% {
			opacity: 0.35;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.app-banner-dot {
			animation: none;
		}
	}
	.app-navigation-error {
		position: fixed;
		inset: 50% auto auto 50%;
		transform: translate(-50%, -50%);
		width: min(440px, calc(100vw - 32px));
		max-width: none;
		margin: 0;
		padding: clamp(24px, 5vw, 40px);
		border: 1px solid var(--app-edge);
		border-radius: 6px;
		background: var(--app-surface);
		box-shadow: 0 8px 28px rgba(65, 76, 104, 0.12);
		color: var(--app-ink);
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
		background: var(--app-danger-soft);
		color: var(--app-danger);
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
		color: var(--app-muted);
	}
	.app-navigation-error button {
		padding: 9px 18px;
		border: 1px solid var(--app-edge);
		border-radius: 4px;
		background: var(--app-accent-soft);
		box-shadow: 0 1px 2px rgba(65, 76, 104, 0.025);
		color: var(--app-accent);
		font: 600 var(--t-body) var(--font-ui);
		min-width: 140px;
		cursor: pointer;
	}
	.app-navigation-error button:disabled {
		opacity: 0.55;
		cursor: wait;
	}
	.app-navigation-error button:focus-visible {
		outline: 2px solid var(--app-accent);
		outline-offset: 2px;
	}
	.app-navigation-error .app-stay-button {
		margin: 12px 0 0 8px;
		border-color: transparent;
		background: transparent;
		box-shadow: none;
		color: var(--app-accent);
	}
</style>
