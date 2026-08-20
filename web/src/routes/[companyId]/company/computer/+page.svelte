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
	import { attentionSource } from '$lib/model/attentionSource.svelte';
	import { browserTabClientId } from '$lib/model/browserTab';
	import { getBrowserStatus, type BrowserStatus } from '$lib/model/company';
	import { companySource } from '$lib/model/companySource.svelte';

	type TransitionDocument = Document & {
		startViewTransition?: (update: () => void | Promise<void>) => { finished: Promise<void> };
	};

	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(companySource(companyId));
	$effect(() => source.attach());
	const attention = $derived(attentionSource(companyId));
	$effect(() => attention.attach());
	const view = $derived(source.view);
	const preparedHandoffs = $derived(
		(attention.view?.items ?? []).filter((item) => item.runtimeAttach).slice(0, 4)
	);
	const focus = $derived(page.url.searchParams.get('focus') === 'desktop');

	let clientId = $state('');
	let desktopUrl = $state('');
	let browserStatus = $state<BrowserStatus | null>(null);
	let controller = $state<'observer' | 'owner'>('observer');
	let working = $state('');
	let error = $state('');

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
		return runtimeBrowser?.controller === 'unclaimed'
			? 'Ready for control'
			: (runtimeBrowser?.controller ?? 'Controller unknown');
	});

	onMount(() => {
		void browserTabClientId(companyId).then((id) => {
			clientId = id;
			void refreshBrowserStatus();
			if (focus) void attachDesktop(false);
		});
		const statusPoll = window.setInterval(() => void refreshBrowserStatus(), 8_000);
		const heartbeat = window.setInterval(() => {
			if (controller === 'owner') {
				void browserControl(companyId, 'heartbeat', clientId).catch((cause) => {
					controller = 'observer';
					error = cause instanceof Error ? cause.message : 'The controller lease ended.';
				});
			}
		}, 12_000);
		return () => {
			window.clearInterval(statusPoll);
			window.clearInterval(heartbeat);
		};
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

	async function refreshBrowserStatus() {
		try {
			browserStatus = await getBrowserStatus(companyId);
			if (
				browserStatus.control?.controller === 'owner' &&
				browserStatus.control.client_id === clientId
			) {
				controller = 'owner';
				if (focus) desktopUrl = controlledUrl();
			} else if (controller === 'owner') {
				controller = 'observer';
				if (focus) desktopUrl = observedUrl();
			}
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Browser state is unavailable.';
		}
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
			if (navigate) await morphTo(`/${companyId}/company/computer?focus=desktop`);
			await refreshBrowserStatus();
		} catch (cause) {
			error =
				cause instanceof Error ? cause.message : 'The Company computer could not be attached.';
		} finally {
			working = '';
		}
	}

	async function takeControl() {
		if (!clientId || working) return;
		working = 'control';
		error = '';
		try {
			await browserControl(companyId, 'take', clientId);
			controller = 'owner';
			desktopUrl = controlledUrl();
			await refreshBrowserStatus();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Control is held elsewhere.';
		} finally {
			working = '';
		}
	}

	async function returnControl() {
		if (!clientId || working) return;
		working = 'return';
		error = '';
		try {
			await browserControl(companyId, 'return', clientId);
			controller = 'observer';
			desktopUrl = observedUrl();
			await refreshBrowserStatus();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Control could not be returned.';
		} finally {
			working = '';
		}
	}

	async function closeFocus() {
		if (controller === 'owner') await returnControl();
		await morphTo(`/${companyId}/company/computer`);
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
						onclick={returnControl}>Return control</button
					>
				{:else}
					<button
						class="btn small primary"
						type="button"
						disabled={!!working || !desktopUrl}
						title="Pauses company automation and gives this browser tab sole keyboard and pointer control."
						onclick={takeControl}>Take control</button
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
		<DesktopViewport src={desktopUrl} title="Live Company computer" onload={refreshBrowserStatus} />
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
							? 'Opens the persistent desktop in observe-only mode. Take control only when you need to act.'
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
