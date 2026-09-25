<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { onMount, untrack } from 'svelte';
	import Activity from '@lucide/svelte/icons/activity';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';
	import Monitor from '@lucide/svelte/icons/monitor';
	import { desktopWindows, focusDesktopWindow, type DesktopWindow } from '$lib/model/desktop';
	import DesktopViewport from '$lib/components/DesktopViewport.svelte';
	import { browserControl, issueDesktopTicket } from '$lib/model/attention';
	import { browserTabClientId } from '$lib/model/browserTab';
	import {
		COMPANY_BROWSER_OPEN_EVENT,
		consumeCompanyBrowserIntent,
		lastCompanyBrowserDestination,
		openCompanyBrowser,
		rememberCompanyBrowserDestination
	} from '$lib/model/company-browser';
	import { attentionQuery, browserStatusQuery, companyQuery } from '$lib/model/queries.svelte';

	type TransitionDocument = Document & {
		startViewTransition?: (update: () => void | Promise<void>) => { finished: Promise<void> };
	};

	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(companyQuery(companyId));
	const attention = $derived(attentionQuery(companyId));
	const browserProjection = $derived(browserStatusQuery(companyId));
	const view = $derived(source.view);
	const preparedHandoffs = $derived(
		(attention.view?.items ?? []).filter((item) => item.runtimeAttach).slice(0, 4)
	);
	const focusMode = $derived(page.url.searchParams.get('focus'));
	const focus = $derived(focusMode === 'desktop' || focusMode === 'browser');
	const browserFocus = $derived(focusMode === 'browser');

	let clientId = $state('');
	let controlLeaseId = $state('');
	let desktopUrl = $state('');
	const browserStatus = $derived(browserProjection.view);
	let controller = $state<'observer' | 'owner'>('observer');
	let controlRequested = $state(false);
	let canResize = $state(false);
	let working = $state('');
	let error = $state('');
	let lastDesktopActivity = $state(0);
	let lastLeaseRenewal = $state(0);
	let activityRenewing = $state(false);
	let displaySizeTimer: ReturnType<typeof setTimeout> | undefined;
	let displaySizePending: { width: number; height: number } | undefined;
	let displaySizeRenewing = false;
	let attachmentRecoveryAttempted = false;
	let attachmentRecoveryInFlight = false;
	let attachmentRecoveryGoodLeases = 0;
	let browserDestination = $state('');
	let windows = $state<DesktopWindow[]>([]);
	let windowsLoading = $state(false);
	let windowsError = $state('');
	let windowCompany = $state('');
	let lastWindowRefresh = 0;
	const activeWindow = $derived(windows.find((window) => window.active)?.id ?? '');

	const runtimeBrowser = $derived(view?.computer.runtime?.browser ?? null);
	const canAttach = $derived(runtimeBrowser?.status === 'available');
	const controllerLabel = $derived.by(() => {
		if (working === 'control') return 'Taking control…';
		if (controller === 'owner') return 'You control';
		const control = browserStatus?.control;
		if (control?.controller === 'owner') {
			return control.client_id === clientId ? 'Viewing only' : 'Another owner tab controls';
		}
		if (control?.controller === 'agent') {
			return control.requesting_actor
				? `${control.requesting_actor} controls`
				: 'A company actor controls';
		}
		if (browserStatus) return 'Viewing only';
		return runtimeBrowser?.controller === 'unclaimed'
			? 'Viewing only'
			: (runtimeBrowser?.controller ?? 'Controller unknown');
	});

	onMount(() => {
		const browserRequest = (event: Event) => {
			const intent = (event as CustomEvent<{ url?: unknown }>).detail;
			if (typeof intent?.url === 'string') void openRequestedBrowser();
		};
		window.addEventListener(COMPANY_BROWSER_OPEN_EVENT, browserRequest);
		void browserTabClientId(companyId)
			.then((id) => {
				clientId = id;
				if (focusMode === 'desktop') void attachDesktop(false);
				if (focusMode === 'browser') void openRequestedBrowser();
			})
			.catch((cause) => {
				error = cause instanceof Error ? cause.message : 'The desktop session could not be opened.';
			});
		const idleRelease = window.setInterval(() => {
			if (
				controller === 'owner' &&
				lastDesktopActivity > 0 &&
				Date.now() - lastDesktopActivity >= 60_000
			) {
				void returnControl(true);
			}
		}, 5_000);
		return () => {
			window.clearInterval(idleRelease);
			clearTimeout(displaySizeTimer);
			window.removeEventListener(COMPANY_BROWSER_OPEN_EVENT, browserRequest);
		};
	});

	$effect(() => {
		const current = browserStatus;
		if (!current) return;
		// Apply a newly observed lease, not an old snapshot when a local action
		// changes the controller. That used to reconnect three times per takeover.
		untrack(() => {
			if (
				controlRequested &&
				current.control?.controller === 'owner' &&
				current.control.client_id === clientId
			) {
				controller = 'owner';
			} else if (controller === 'owner') {
				controller = 'observer';
				controlLeaseId = '';
				canResize = false;
			}
		});
	});

	async function morphTo(href: string) {
		const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
		const transitionDocument = document as TransitionDocument;
		if (!reduced && transitionDocument.startViewTransition) {
			await transitionDocument.startViewTransition(() => goto(href, { noScroll: true })).finished;
			return;
		}
		await goto(href, { noScroll: true });
	}

	async function attachDesktop(navigate = true) {
		if (!clientId || working) return;
		attachmentRecoveryAttempted = false;
		attachmentRecoveryGoodLeases = 0;
		working = 'attach';
		error = '';
		try {
			desktopUrl = await issueDesktopTicket(companyId, 'runtime-rescue', clientId);
			controller = 'observer';
			if (navigate) await morphTo(`/${companyId}/company/computer?focus=desktop`);
			await browserProjection.refresh();
		} catch (cause) {
			error =
				cause instanceof Error ? cause.message : 'The Company computer could not be attached.';
		} finally {
			working = '';
		}
	}

	async function openRequestedBrowser(requestedUrl?: string) {
		if (working) return;
		const intent = requestedUrl ? { url: requestedUrl } : consumeCompanyBrowserIntent(companyId);
		if (!intent?.url) {
			browserDestination = lastCompanyBrowserDestination(companyId);
			await attachDesktop(false);
			return;
		}
		browserDestination = intent.url;
		working = 'browser';
		error = '';
		try {
			desktopUrl = await openCompanyBrowser(companyId, intent.url);
			rememberCompanyBrowserDestination(companyId, intent.url);
			controller = 'observer';
			await browserProjection.refresh();
		} catch (cause) {
			error =
				cause instanceof Error ? cause.message : 'The Company browser could not open this page.';
		} finally {
			working = '';
		}
	}

	async function claimControl(): Promise<boolean> {
		if (!clientId) return false;
		if (controller === 'owner') return true;
		if (working) return false;
		working = 'control';
		error = '';
		try {
			const control = await browserControl(companyId, 'take', clientId);
			controlLeaseId = control.lease_id ?? '';
			controlRequested = true;
			controller = 'owner';
			lastDesktopActivity = Date.now();
			lastLeaseRenewal = Date.now();
			void browserProjection.refresh();
			void refreshWindows();
			return true;
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Control is held elsewhere.';
			void browserProjection.refresh();
			return false;
		} finally {
			working = '';
		}
	}

	async function returnControl(automatic = false) {
		if (!clientId || working) return;
		working = 'return';
		error = '';
		try {
			await browserControl(companyId, 'return', clientId, controlLeaseId);
			controlLeaseId = '';
			controlRequested = false;
			controller = 'observer';
			canResize = false;
			lastDesktopActivity = 0;
			await browserProjection.refresh();
		} catch (cause) {
			if (!automatic)
				error = cause instanceof Error ? cause.message : 'Control could not be returned.';
		} finally {
			working = '';
		}
	}

	function requestDisplayResize(width: number, height: number) {
		displaySizePending = { width, height };
		clearTimeout(displaySizeTimer);
		displaySizeTimer = setTimeout(() => void flushDisplayResize(), 180);
	}

	async function flushDisplayResize() {
		if (displaySizeRenewing || !displaySizePending || !clientId || !desktopUrl) return;
		displaySizeRenewing = true;
		try {
			while (displaySizePending && clientId && desktopUrl) {
				const size = displaySizePending;
				displaySizePending = undefined;
				const response = await fetch(
					`/api/companies/${encodeURIComponent(companyId)}/desktop/display-lease`,
					{
						method: 'POST',
						headers: { 'content-type': 'application/json' },
						body: JSON.stringify({
							client_id: clientId,
							width: size.width,
							height: size.height
						}),
						credentials: 'same-origin'
					}
				);
				if (response.status === 401) {
					canResize = false;
					displaySizePending = undefined;
					if (document.visibilityState === 'visible') {
						if (attachmentRecoveryAttempted && !attachmentRecoveryInFlight) {
							error =
								'The computer session is still unavailable. Reopen the computer to reconnect.';
						} else {
							await recoverExpiredAttachment();
						}
					}
					return;
				}
				if (!response.ok) throw new Error(`Display sizing returned ${response.status}.`);
				const result = (await response.json()) as { can_resize?: boolean };
				canResize = result.can_resize === true;
				if (attachmentRecoveryAttempted && ++attachmentRecoveryGoodLeases >= 2) {
					attachmentRecoveryAttempted = false;
					attachmentRecoveryGoodLeases = 0;
				}
			}
		} catch (cause) {
			canResize = false;
			if (document.visibilityState === 'visible')
				error = cause instanceof Error ? cause.message : 'Computer sizing could not be updated.';
		} finally {
			displaySizeRenewing = false;
		}
	}

	async function recoverExpiredAttachment() {
		if (attachmentRecoveryInFlight || attachmentRecoveryAttempted) return;
		attachmentRecoveryAttempted = true;
		attachmentRecoveryInFlight = true;
		attachmentRecoveryGoodLeases = 0;
		try {
			const freshUrl = await issueDesktopTicket(companyId, 'runtime-rescue', clientId);
			if (document.visibilityState !== 'visible') {
				attachmentRecoveryAttempted = false;
				return;
			}
			desktopUrl = freshUrl;
			error = '';
			void browserProjection.refresh();
		} catch (cause) {
			error =
				cause instanceof Error
					? `The computer session expired and could not be restored: ${cause.message}`
					: 'The computer session expired. Reopen the computer to reconnect.';
		} finally {
			attachmentRecoveryInFlight = false;
		}
	}

	async function closeFocus() {
		if (controller === 'owner') await returnControl();
		await morphTo(`/${companyId}/company/computer`);
	}

	async function desktopReady() {
		await Promise.all([browserProjection.refresh(), refreshWindows()]);
	}

	async function refreshWindows() {
		const requestedCompany = companyId;
		if (windowsLoading) return;
		windowsLoading = true;
		try {
			const result = await desktopWindows(requestedCompany);
			if (requestedCompany !== companyId) return;
			windows = result;
			windowCompany = requestedCompany;
			windowsError = '';
		} catch (cause) {
			if (requestedCompany === companyId)
				windowsError = cause instanceof Error ? cause.message : 'Applications are unavailable.';
		} finally {
			windowsLoading = false;
			lastWindowRefresh = Date.now();
		}
	}

	async function selectWindow(id: string) {
		if (!id || windowCompany !== companyId || controller !== 'owner' || working) return;
		working = 'window';
		error = '';
		try {
			await focusDesktopWindow(companyId, id, clientId, controlLeaseId);
			desktopActivity();
			await refreshWindows();
		} catch (cause) {
			error =
				cause instanceof Error ? cause.message : 'The application could not be brought forward.';
		} finally {
			working = '';
		}
	}

	function desktopActivity() {
		const now = Date.now();
		lastDesktopActivity = now;
		if (controller !== 'owner') return;
		if (now - lastWindowRefresh > 5_000) void refreshWindows();
		if (activityRenewing || now - lastLeaseRenewal < 8_000) return;
		activityRenewing = true;
		lastLeaseRenewal = now;
		void browserControl(companyId, 'heartbeat', clientId, controlLeaseId)
			.then(() => browserProjection.refresh())
			.catch((cause) => {
				controller = 'observer';
				controlRequested = false;
				controlLeaseId = '';
				canResize = false;
				error = cause instanceof Error ? cause.message : 'Desktop control expired.';
			})
			.finally(() => (activityRenewing = false));
	}

	function when(value?: string): string {
		if (!value) return 'Observation time unavailable';
		return new Date(value).toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: 'numeric',
			minute: '2-digit'
		});
	}
</script>

<svelte:head><title>Computer — {view?.company.name ?? companyId}</title></svelte:head>

{#if focus}
	<div class="company-desktop-focus">
		<header class="computer-focus-toolbar">
			<div class="computer-focus-identity">
				<span class="computer-focus-icon"><Monitor size={17} strokeWidth={1.8} /></span>
				<h1>{browserFocus ? 'Company browser' : 'Computer'}</h1>
				<span
					class="computer-control-state"
					class:controlled={controller === 'owner'}
					title="One person or agent controls the shared computer at a time. Inactive control is released after one minute."
				>
					<i
						class="source-lamp status-{controller === 'owner' ? 'live' : 'stale'}"
						aria-hidden="true"
					></i>
					{controllerLabel}
				</span>
			</div>
			{#if controller === 'owner' && windowCompany === companyId && windows.length > 1}
				<div
					class="computer-app-switcher"
					title={windowsError || 'Bring an open application to the front'}
				>
					<select
						aria-label="Switch application"
						value={activeWindow}
						disabled={!!working}
						onfocus={() => void refreshWindows()}
						onpointerdown={() => void refreshWindows()}
						onchange={(event) => void selectWindow(event.currentTarget.value)}
					>
						<option value="" disabled
							>{windowsError ? 'Applications unavailable' : 'Open applications'}</option
						>
						{#each windowCompany === companyId ? windows : [] as window (window.id)}
							<option value={window.id}>{window.title || window.app}</option>
						{/each}
					</select>
				</div>
			{/if}
			<div class="desktop-focus-actions">
				{#if browserFocus && browserDestination}
					<a
						class="btn small"
						href={browserDestination}
						target="_blank"
						rel="noreferrer"
						data-open-externally>Open externally</a
					>
				{/if}
				{#if controller === 'owner'}
					<button
						class="btn small"
						type="button"
						disabled={!!working}
						title="Returns input to the company actor. It does not complete Work or an owner decision."
						onclick={() => returnControl()}
						>{working === 'return' ? 'Releasing…' : 'Release control'}</button
					>
				{/if}
				<button
					class="btn small"
					type="button"
					title="Leaves the live computer and restores the Company surface."
					onclick={closeFocus}>Leave computer</button
				>
			</div>
		</header>
		<div class="desktop-notices">
			{#if error}<div class="computer-error" role="alert">{error}</div>{/if}
		</div>
		<DesktopViewport
			src={desktopUrl}
			title="Live Company computer"
			interactive={controller === 'owner'}
			canResize={canResize || controller === 'owner'}
			onclaim={claimControl}
			ondisplayresize={requestDisplayResize}
			onload={() => void desktopReady()}
			onactivity={desktopActivity}
		/>
	</div>
{:else}
	<div class="company-computer-portal">
		<header class="computer-portal-nav">
			<a href={`/${companyId}/company`} title="Back to Company" aria-label="Back to Company"
				><ArrowLeft size={14} strokeWidth={1.8} /> <span>Company</span></a
			>
			<a href={`/${companyId}/company/doctor`}>
				<Activity size={14} strokeWidth={1.8} />
				Doctor
				{#if view}<i class="doctor-dot check-{view.computer.doctor.status}" aria-hidden="true"
					></i>{/if}
			</a>
		</header>

		{#if error}<div class="computer-error portal-error" role="alert">{error}</div>{/if}

		<main class="computer-portal-stage">
			<section class:unavailable={!canAttach} class="computer-portal-machine">
				<div class="computer-portal-bezel">
					<div class="computer-portal-glyph" aria-hidden="true">
						<Monitor size={28} strokeWidth={1.45} />
					</div>
					<h1>Computer</h1>
					<div class="computer-portal-controller">
						<span class="source-lamp status-{canAttach ? 'live' : 'stale'}" aria-hidden="true"
						></span>
						{controllerLabel}
					</div>
					{#if canAttach}
						<button
							class="computer-enter"
							type="button"
							disabled={!!working || !clientId}
							onclick={() => attachDesktop()}
						>
							<span>{working === 'attach' ? 'Connecting…' : 'Enter computer'}</span>
							<ArrowUpRight size={16} strokeWidth={2} aria-hidden="true" />
						</button>
						<p>
							Your team’s shared browser, files and applications. Click or type on the desktop to
							join in.
						</p>
					{:else}
						<p class="computer-unavailable-copy">
							The desktop hasn’t passed its live check yet, so it can’t be opened.
						</p>
						<a class="btn primary" href={`/${companyId}/company/doctor`}>
							<Activity size={14} strokeWidth={1.8} aria-hidden="true" /> See what needs fixing
						</a>
					{/if}
				</div>
			</section>
		</main>

		<footer class="computer-portal-footer">
			<div>
				<span class="source-lamp status-{source.status}" aria-hidden="true"></span>
				<span>{source.status === 'live' ? 'Live' : 'Last checked'}</span>
				<time>{when(view?.computer.doctor.observed_at)}</time>
			</div>
			{#if preparedHandoffs.length}
				<a href={`/${companyId}`}
					>{preparedHandoffs.length} prepared {preparedHandoffs.length === 1
						? 'handoff'
						: 'handoffs'} in Attention</a
				>
			{:else}
				<span>No prepared handoff is waiting.</span>
			{/if}
		</footer>
	</div>
{/if}
