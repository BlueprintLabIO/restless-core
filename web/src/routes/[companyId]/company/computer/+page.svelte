<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import Activity from '@lucide/svelte/icons/activity';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';
	import Monitor from '@lucide/svelte/icons/monitor';
	import DesktopViewport from '$lib/components/DesktopViewport.svelte';
	import { browserControl, issueDesktopTicket } from '$lib/model/attention';
	import { browserTabClientId } from '$lib/model/browserTab';
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
	const focus = $derived(page.url.searchParams.get('focus') === 'desktop');

	let clientId = $state('');
	let desktopUrl = $state('');
	const browserStatus = $derived(browserProjection.view);
	let controller = $state<'observer' | 'owner'>('observer');
	let working = $state('');
	let error = $state('');
	let autoClaimPending = $state(false);
	let lastDesktopActivity = $state(0);
	let lastLeaseRenewal = $state(0);
	let activityRenewing = $state(false);

	const runtimeBrowser = $derived(view?.computer.runtime?.browser ?? null);
	const canAttach = $derived(runtimeBrowser?.status === 'available');
	const controllerLabel = $derived.by(() => {
		const control = browserStatus?.control;
		if (control?.controller === 'owner') {
			return control.client_id === clientId ? 'You control' : 'Another owner tab controls';
		}
		if (control?.controller === 'agent') {
			return control.requesting_actor
				? `${control.requesting_actor} controls`
				: 'A company actor controls';
		}
		if (browserStatus) return 'Ready for control';
		return runtimeBrowser?.controller === 'unclaimed'
			? 'Ready for control'
			: (runtimeBrowser?.controller ?? 'Controller unknown');
	});

	onMount(() => {
		void browserTabClientId(companyId).then((id) => {
			clientId = id;
			if (focus) void attachDesktop(false);
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
		};
	});

	$effect(() => {
		const current = browserStatus;
		if (!current) return;
		if (current.control?.controller === 'owner' && current.control.client_id === clientId) {
			controller = 'owner';
			if (focus) desktopUrl = controlledUrl();
		} else if (controller === 'owner') {
			controller = 'observer';
			if (focus) desktopUrl = observedUrl();
		}
		error = browserProjection.failure?.message ?? '';
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

	function observedUrl(): string {
		return `/desktop/${encodeURIComponent(companyId)}/observe`;
	}

	function controlledUrl(): string {
		return `/desktop/${encodeURIComponent(companyId)}/control?client_id=${encodeURIComponent(clientId)}`;
	}

	async function attachDesktop(navigate = true) {
		if (!clientId || working) return;
		working = 'attach';
		error = '';
		try {
			desktopUrl = await issueDesktopTicket(companyId, 'runtime-rescue', clientId);
			controller = 'observer';
			autoClaimPending = true;
			if (navigate) await morphTo(`/${companyId}/company/computer?focus=desktop`);
			await browserProjection.refresh();
		} catch (cause) {
			error =
				cause instanceof Error ? cause.message : 'The Company computer could not be attached.';
		} finally {
			working = '';
		}
	}

	async function takeControl(silent = false) {
		if (!clientId || working) return;
		working = 'control';
		error = '';
		try {
			await browserControl(companyId, 'take', clientId);
			controller = 'owner';
			desktopUrl = controlledUrl();
			lastDesktopActivity = Date.now();
			lastLeaseRenewal = Date.now();
			await browserProjection.refresh();
		} catch (cause) {
			if (!silent) error = cause instanceof Error ? cause.message : 'Control is held elsewhere.';
		} finally {
			working = '';
		}
	}

	async function returnControl(automatic = false) {
		if (!clientId || working) return;
		working = 'return';
		error = '';
		try {
			await browserControl(companyId, 'return', clientId);
			controller = 'observer';
			desktopUrl = observedUrl();
			lastDesktopActivity = 0;
			await browserProjection.refresh();
		} catch (cause) {
			if (!automatic)
				error = cause instanceof Error ? cause.message : 'Control could not be returned.';
		} finally {
			working = '';
		}
	}

	async function closeFocus() {
		if (controller === 'owner') await returnControl();
		await morphTo(`/${companyId}/company/computer`);
	}

	async function desktopReady() {
		await browserProjection.refresh();
		if (!autoClaimPending) return;
		autoClaimPending = false;
		await takeControl(true);
	}

	function desktopActivity() {
		const now = Date.now();
		lastDesktopActivity = now;
		if (controller !== 'owner') {
			void takeControl();
			return;
		}
		if (activityRenewing || now - lastLeaseRenewal < 8_000) return;
		activityRenewing = true;
		lastLeaseRenewal = now;
		void browserControl(companyId, 'heartbeat', clientId)
			.then(() => browserProjection.refresh())
			.catch((cause) => {
				controller = 'observer';
				desktopUrl = observedUrl();
				error = cause instanceof Error ? cause.message : 'Desktop control expired.';
			})
			.finally(() => (activityRenewing = false));
	}

	function when(value?: string): string {
		if (!value) return 'Observation time unavailable';
		return new Date(value).toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<svelte:head><title>Company computer — {view?.company.name ?? companyId}</title></svelte:head>

{#if focus}
	<div class="company-desktop-focus">
		<header class="computer-focus-toolbar">
			<div class="computer-focus-identity">
				<span class="computer-focus-icon"><Monitor size={15} strokeWidth={1.8} /></span>
				<div>
					<h1>Company computer</h1>
					<span
						><i
							class="source-lamp status-{runtimeBrowser?.status === 'available' ? 'live' : 'stale'}"
							aria-hidden="true"
						></i>{runtimeBrowser?.status ?? 'unknown'} · {controllerLabel}</span
					>
				</div>
			</div>
			<div class="desktop-focus-actions">
				{#if controller === 'owner'}
					<button
						class="btn small"
						type="button"
						disabled={!!working}
						title="Returns input to the company actor. It does not complete Work or an owner decision."
						onclick={() => returnControl()}>Return control</button
					>
				{:else}
					<button
						class="btn small primary"
						type="button"
						disabled={!!working || !desktopUrl}
						title="Claims input only if the computer is not held by another owner tab or company actor."
						onclick={() => takeControl()}>Try control</button
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
		{#if error}<div class="computer-error" role="alert">{error}</div>{/if}
		<DesktopViewport
			src={desktopUrl}
			title="Live Company computer"
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
					<h1>Company computer</h1>
					<div class="computer-portal-controller">
						<span class="source-lamp status-{canAttach ? 'live' : 'stale'}" aria-hidden="true"
						></span>
						{controllerLabel}
					</div>
					<button
						class="computer-enter"
						type="button"
						disabled={!canAttach || !!working || !clientId}
						onclick={() => attachDesktop()}
					>
						<span>{working === 'attach' ? 'Connecting…' : 'Enter computer'}</span>
						<ArrowUpRight size={16} strokeWidth={2} aria-hidden="true" />
					</button>
					<p>
						{canAttach
							? 'Opens with input when the computer is free. Control returns after one minute without desktop activity.'
							: 'The desktop has not passed its live probe. Open Doctor for the smallest available repair.'}
					</p>
				</div>
			</section>
		</main>

		<footer class="computer-portal-footer">
			<div>
				<span class="source-lamp status-{source.status}" aria-hidden="true"></span>
				<span>{source.status === 'live' ? 'Live observation' : 'Last observation'}</span>
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
