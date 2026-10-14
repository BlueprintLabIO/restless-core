<script lang="ts">
	import { settleQuietly } from '$lib/transition';
	import Toasts from '$lib/ui/feedback/Toasts.svelte';
	import ConfirmDialog from '$lib/ui/feedback/ConfirmDialog.svelte';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	/* The app frame that sits OUTSIDE the design scope: the offline banner and the
	 * loading bar. They live here rather than inside `.bridge-root` because they
	 * report on the app itself, not on the company — which is also why the z-ladder
	 * that positions them is defined on :root (see design/base.css). */

	import '$lib/design/index.css';
	import { navigating, page, updated } from '$app/state';
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
	import { installTripwire, resyncVisible, steady } from '$lib/model/steady';
	import { accountServicePath } from '$lib/model/account-service';
	import { planeAnswersAccountPaths } from '$lib/model/appliance';
	import { registerServiceWorkerWhereLocal } from '$lib/model/service-worker-registration';
	import { knownPlaneOwner } from '$lib/model/known-role';
	import { dev } from '$app/environment';
	import { PRODUCT_NAME } from '$lib/brand/brand';

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
			settleQuietly(
				document.startViewTransition(async () => {
					resolve();
					await navigation.complete;
				})
			);
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
	/* Staying must hold: a newer navigation to the page on screen supersedes the stalled one, so
	 * it can no longer finish later and move the person away mid-task. */
	function stayOnCurrentPage() {
		navigationStalled = false;
		navigationDialog?.close();
		void goto(`${page.url.pathname}${page.url.search}${page.url.hash}`, {
			replaceState: true,
			noScroll: true,
			keepFocus: true
		});
	}
	let restored = $state(false);
	let restoredTimer: ReturnType<typeof setTimeout> | undefined;
	function succeeded() {
		if (!observeSuccess()) return;
		/* Anything read during the outage may be behind: refresh what is on
		 * screen once, then confirm briefly. */
		void resyncVisible(queryClient);
		restored = true;
		clearTimeout(restoredTimer);
		restoredTimer = setTimeout(() => (restored = false), 2_400);
	}
	const queryClient = steady(
		new QueryClient({
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
		})
	);
	function reconnectNow() {
		void queryClient.refetchQueries({ type: 'active' });
	}

	/* A hosted session that ends mid-use re-enters through the account issuer. */
	onMount(() =>
		observeSessionEnd(
			() => page.params.companyId,
			(reason) =>
				toast(
					reason === 'signed_out'
						? 'You were signed out. Sign in again to keep working; your drafts are kept.'
						: 'Part of this page could not load after an update. Reload to finish; your drafts are kept.',
					{
						persist: true,
						action:
							reason === 'signed_out'
								? {
										label: 'Sign in',
										run: () =>
											window.location.assign(
												`/account?return=${encodeURIComponent(location.pathname + location.search)}`
											)
									}
								: { label: 'Reload', run: () => window.location.reload() }
					}
				)
		)
	);
	onMount(() => installTripwire(queryClient));
	/* A tab left open across a deploy is told once, at a moment the person chooses; drafts are
	 * already kept per conversation, so reloading loses nothing. Until then, the next move to
	 * another page loads the new version with a full load rather than a chunk the deploy removed. */
	let toldUpdated = false;
	$effect(() => {
		if (!updated.current || toldUpdated) return;
		toldUpdated = true;
		toast('A new version is ready.', {
			persist: true,
			action: { label: 'Reload', run: () => window.location.reload() }
		});
	});

	/* On Cloud's one address, Home, Profile, Security, Plan and Support are the account service's
	 * pages: a link to one leaves this app with a full load instead of being routed here. Whether
	 * this plane answers them is asked only when such a link is followed, so no other page pays a
	 * request for it. Only a plane that answered keeps the link here; a refused check (a member, a
	 * lapsed session) leaves with a full load too, so the address, not this app, decides. */
	let accountHere = false;
	beforeNavigate((navigation) => {
		const to = navigation.to?.url;
		if (
			updated.current &&
			to &&
			!navigation.willUnload &&
			to.origin === window.location.origin &&
			to.pathname !== window.location.pathname
		) {
			navigation.cancel();
			window.location.assign(to.href);
			return;
		}
		if (
			!to ||
			to.origin !== window.location.origin ||
			!accountServicePath(to.pathname) ||
			accountHere
		)
			return;
		navigation.cancel();
		void planeAnswersAccountPaths().then((here) => {
			accountHere = here;
			if (here) void goto(`${to.pathname}${to.search}${to.hash}`);
			else window.location.assign(to.href);
		});
	});
	onMount(() => {
		/* The worker serves a local appliance's owner; asking the plane whether it is hosted is the
		 * owner's read, refused to a member. */
		if (!dev && knownPlaneOwner()) void registerServiceWorkerWhereLocal().catch(() => {});
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
			void resyncVisible(queryClient);
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

	/* A file dropped where nothing takes it would make the browser open it in place of the
	 * cockpit. Drop targets (the composer, the Library) claim their own drops first; anything left
	 * over is refused here, so a stray drop does nothing. */
	const carriesFiles = (event: DragEvent) =>
		!event.defaultPrevented && !!event.dataTransfer?.types.includes('Files');
	function refuseStrayDragOver(event: DragEvent) {
		if (!carriesFiles(event)) return;
		event.preventDefault();
		event.dataTransfer!.dropEffect = 'none';
	}
	function refuseStrayDrop(event: DragEvent) {
		if (carriesFiles(event)) event.preventDefault();
	}
</script>

<svelte:head><meta name="application-name" content={PRODUCT_NAME} /></svelte:head>

<svelte:window bind:online ondragover={refuseStrayDragOver} ondrop={refuseStrayDrop} />

{#if isLoading}
	<div class="app-loading-bar" role="status" aria-live="polite" aria-label="Working">
		<div class="app-loading-bar-fill"></div>
	</div>
{/if}

{#if navigationStalled}
	<dialog
		bind:this={navigationDialog}
		class="app-navigation-error"
		aria-labelledby="navigation-error-title"
		aria-describedby="navigation-error-copy"
		oncancel={(event) => {
			/* Escape means staying, and staying cancels the move. */
			event.preventDefault();
			stayOnCurrentPage();
		}}
	>
		<div class="app-navigation-error-mark" aria-hidden="true">!</div>
		<h1 id="navigation-error-title">{PRODUCT_NAME} is taking longer than expected</h1>
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
	<Toasts>
		{#snippet below()}
			<!-- The connection banner shares the toasts' region: above the phone dock, never under
			     an Undo. -->
			{#if !online && connection.lost}
				<div
					class="app-banner"
					role="status"
					aria-live="polite"
					transition:fade={{ duration: 160 }}
				>
					You're offline.
				</div>
			{:else if connection.lost}
				<div
					class="app-banner"
					role="status"
					aria-live="polite"
					transition:fade={{ duration: 160 }}
				>
					<span class="app-banner-dot" aria-hidden="true"></span>
					Connection lost. Reconnecting…
					<button type="button" onclick={reconnectNow}>Retry now</button>
				</div>
			{:else if restored}
				<div
					class="app-banner"
					role="status"
					aria-live="polite"
					transition:fade={{ duration: 160 }}
				>
					Reconnected.
				</div>
			{/if}
		{/snippet}
	</Toasts>
	<ConfirmDialog />
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
