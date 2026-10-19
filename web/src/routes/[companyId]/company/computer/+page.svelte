<script lang="ts">
	import { requestDisplayLease } from '$lib/model/company-actions';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { formatRelative, formatMoment } from '$lib/ui/time';
	import { failureSentence } from '$lib/model/failure';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { onMount, untrack } from 'svelte';
	import Activity from '@lucide/svelte/icons/activity';
	import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';
	import Monitor from '@lucide/svelte/icons/monitor';
	import { desktopWindows, focusDesktopWindow, type DesktopWindow } from '$lib/model/desktop';
	import DesktopViewport from '$lib/components/DesktopViewport.svelte';
	import ComputerFiles from '$lib/components/ComputerFiles.svelte';
	import CompanyMachines from '$lib/components/CompanyMachines.svelte';
	import { dropComputerFile } from '$lib/model/computer-files';
	import { issueDesktopTicket } from '$lib/model/attention';
	import { DesktopControl } from '$lib/model/desktop-control.svelte';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import { recoverCompany, sleepCompany } from '$lib/model/company';
	import { browserTabClientId } from '$lib/model/browserTab';
	import {
		COMPANY_BROWSER_OPEN_EVENT,
		consumeCompanyBrowserIntent,
		lastCompanyBrowserDestination,
		openCompanyBrowser,
		rememberCompanyBrowserDestination
	} from '$lib/model/company-browser';
	import { attentionQuery, browserStatusQuery, companyQuery } from '$lib/model/queries.svelte';
	import { holdQueries } from '$lib/model/hold-queries.svelte';

	type TransitionDocument = Document & {
		startViewTransition?: (update: () => void | Promise<void>) => { finished: Promise<void> };
	};

	const companyId = $derived(page.params.companyId ?? '');
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
	const filesView = $derived(focusMode === 'files');
	/* A harness sign-in opened in the company browser: bring it forward and hand this tab control. */
	const signIn = $derived(page.url.searchParams.get('task') === 'harness-sign-in');

	let clientId = $state('');
	let desktopUrl = $state('');
	const browserStatus = $derived(browserProjection.view);
	const control = new DesktopControl(
		() => companyId,
		() => clientId,
		() => void browserProjection.refresh()
	);
	let canResize = $state(false);
	let working = $state('');
	let error = $state('');
	let signInPending = false;
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
	/* Asleep is designed, not broken: asking to enter wakes it. */
	const asleep = $derived(!canAttach && view?.limits.runtime.asleep === true);
	const container = $derived(view?.computer.runtime?.container ?? null);
	/* Stopped by someone, not by the sleep policy: it starts only when asked. */
	const stopped = $derived(container === 'Stopped' && !asleep);
	/* A used-up monthly limit keeps it asleep: no wake can start it until the limit changes. */
	const hoursUsedUp = $derived.by(() => {
		const runtime = view?.limits.runtime;
		return (
			!canAttach &&
			runtime?.monthly_runtime_cap_hours != null &&
			runtime.usage?.remaining_seconds === 0
		);
	});
	const limitsHref = $derived(`/${companyId}/company/resources#computer`);
	/* Waking usually takes under a minute; this page keeps asking until then rather than failing. */
	let wakeStarted = $state(0);
	let wakeNow = $state(0);
	const enterLabel = $derived.by(() => {
		if (working === 'attach') {
			if (!asleep) return 'Connecting…';
			const seconds = wakeStarted ? Math.round((wakeNow - wakeStarted) / 1000) : 0;
			return seconds >= 5 ? `Waking… ${seconds}s` : 'Waking…';
		}
		return asleep ? 'Wake and enter' : 'Enter computer';
	});
	/* The agent driving the shared browser right now: taking control pauses it. */
	const agentName = $derived.by(() => {
		const agent = browserStatus?.agent;
		return agent ? agent.display || agent.actor : '';
	});
	const controllerLabel = $derived.by(() => {
		if (control.working === 'control') return 'Taking control…';
		if (control.owner) return 'You control';
		const lease = browserStatus?.control;
		if (lease?.controller === 'owner' && lease.client_id !== clientId) {
			return 'Another owner tab controls';
		}
		if (agentName) return `${agentName} is working`;
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
				error = failureSentence(cause, 'The desktop session could not be opened.');
			});
		const stopWatch = control.watch();
		return () => {
			stopWatch();
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
			const held =
				current.control?.controller === 'owner' && current.control.client_id === clientId;
			if (control.owner && !held && !control.working) {
				control.lose();
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

	/* A desktop still starting answers 503; keep asking for a bounded while instead of failing. */
	async function ticketWhileWaking(): Promise<string> {
		const deadline = Date.now() + 90_000;
		wakeStarted = Date.now();
		wakeNow = wakeStarted;
		const tickTimer = setInterval(() => (wakeNow = Date.now()), 1_000);
		try {
			for (;;) {
				try {
					return await issueDesktopTicket(companyId, 'runtime-rescue', clientId);
				} catch (cause) {
					const failure = cause as { status?: number; code?: string | null };
					const starting = failure.status === 503 && failure.code === 'desktop_starting';
					if (!starting || Date.now() > deadline) throw cause;
					await new Promise((resolve) => setTimeout(resolve, 3_000));
				}
			}
		} finally {
			clearInterval(tickTimer);
			wakeStarted = 0;
		}
	}

	async function attachDesktop(navigate = true) {
		if (!clientId || working) return;
		attachmentRecoveryAttempted = false;
		attachmentRecoveryGoodLeases = 0;
		working = 'attach';
		error = '';
		try {
			desktopUrl = await ticketWhileWaking();
			control.lose();
			signInPending = signIn;
			if (navigate) await morphTo(`/${companyId}/company/computer?focus=desktop`);
			await browserProjection.refresh();
		} catch (cause) {
			error = failureSentence(cause, 'The Company computer could not be attached.');
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
			control.lose();
			await browserProjection.refresh();
		} catch (cause) {
			error = failureSentence(cause, 'The Company browser could not open this page.');
		} finally {
			working = '';
		}
	}

	async function claimControl(): Promise<boolean> {
		if (!clientId) return false;
		if (control.owner) return true;
		if (working || control.working) return false;
		/* Joining in while an agent is mid-task pauses it: ask once, not on every click. */
		if (agentName) {
			const go = await confirmAction({
				title: `Pause ${agentName}?`,
				body: `${agentName} is using the computer. Taking control pauses it until you hand control back.`,
				action: 'Take control'
			});
			if (!go) return false;
		}
		error = '';
		const granted = await control.take();
		if (granted) void refreshWindows();
		return granted;
	}

	async function returnControl() {
		await control.release();
		if (!control.owner) canResize = false;
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
				const response = await requestDisplayLease(companyId, {
					client_id: clientId,
					width: size.width,
					height: size.height
				});
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
				error = failureSentence(cause, 'Computer sizing could not be updated.');
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
			error = failureSentence(cause, 'The computer session expired and couldn’t be restored.');
		} finally {
			attachmentRecoveryInFlight = false;
		}
	}

	/* Restart is always the owner's to ask for: probes can pass while a desktop is frozen or an
	 * app is hung. Files and the browser profile are kept. */
	async function restartComputer() {
		if (working) return;
		const go = await confirmAction({
			title: 'Restart the company computer?',
			body: 'Running work pauses briefly and open windows close. Files and the browser profile are kept.',
			action: 'Restart'
		});
		if (!go) return;
		await runLifecycle('restart');
	}

	async function runLifecycle(action: 'start' | 'restart') {
		working = action;
		error = '';
		const wasFocused = focus;
		try {
			if (control.owner) await control.release();
			desktopUrl = '';
			await recoverCompany(companyId, action);
			toast(action === 'restart' ? 'The computer restarted.' : 'The computer started.');
			await source.refresh();
		} catch (cause) {
			error = failureSentence(
				cause,
				action === 'restart' ? 'The computer could not restart.' : 'The computer could not start.'
			);
		} finally {
			working = '';
		}
		if (wasFocused && !error) await attachDesktop(false);
	}

	async function sleepComputer() {
		if (working) return;
		working = 'sleep';
		error = '';
		try {
			if (control.owner) await control.release();
			const outcome = await sleepCompany(companyId);
			toast(
				outcome.message.endsWith('not running')
					? 'The computer is not running.'
					: 'The computer is asleep.'
			);
			desktopUrl = '';
			await source.refresh();
			if (focus) await morphTo(`/${companyId}/company/computer`);
		} catch (cause) {
			error = failureSentence(cause, 'The computer could not sleep now.');
		} finally {
			working = '';
		}
	}

	async function closeFocus() {
		if (control.owner) await returnControl();
		// Entering may have woken the computer: read it again so the entrance does not say asleep.
		void source.refresh();
		await morphTo(`/${companyId}/company/computer`);
	}

	async function desktopReady() {
		await Promise.all([browserProjection.refresh(), refreshWindows()]);
		if (signInPending) {
			signInPending = false;
			await bringSignInForward();
		}
	}

	/* The sign-in page opened in the company browser. Take control for this tab and put that
	 * window in front, so the owner lands on the step rather than hunting for it. */
	async function bringSignInForward() {
		if (!(await control.take())) return;
		await refreshWindows();
		const browserWindow = windows.find((window) => /chrom/i.test(`${window.app} ${window.title}`));
		if (browserWindow && !browserWindow.active) await selectWindow(browserWindow.id);
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
				windowsError = failureSentence(cause, 'Applications are unavailable.');
		} finally {
			windowsLoading = false;
			lastWindowRefresh = Date.now();
		}
	}

	async function selectWindow(id: string) {
		if (!id || windowCompany !== companyId || !control.owner || working) return;
		working = 'window';
		error = '';
		try {
			await focusDesktopWindow(companyId, id, clientId, control.leaseId);
			desktopActivity();
			await refreshWindows();
		} catch (cause) {
			error = failureSentence(cause, 'The application could not be brought forward.');
		} finally {
			working = '';
		}
	}

	function desktopActivity() {
		control.activity();
		if (control.owner && Date.now() - lastWindowRefresh > 5_000) void refreshWindows();
	}

	const when = (value?: Date | string) => formatRelative(value, 'Not yet');
	holdQueries(() => [source, attention, browserProjection]);
</script>

<CompanyTitle title="Computer" {companyId} />

{#if filesView}
	<div class="computer-files-view">
		<a class="btn small ghost" href={`/${companyId}/company/computer`}>← Computer</a>
		<ComputerFiles {companyId} />
	</div>
{:else if focus}
	<div class="company-desktop-focus">
		<header class="computer-focus-toolbar">
			<div class="computer-focus-identity">
				<span class="computer-focus-icon"><Monitor size={17} strokeWidth={1.8} /></span>
				<h1>{browserFocus ? 'Company browser' : 'Computer'}</h1>
				<span
					class="computer-control-state"
					class:controlled={control.owner}
					title="One person or agent controls the shared computer at a time. Inactive control is released after one minute."
				>
					<i class="source-lamp status-{control.owner ? 'live' : 'stale'}" aria-hidden="true"></i>
					{controllerLabel}
				</span>
			</div>
			{#if control.owner && windowCompany === companyId && windows.length > 1}
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
				{#if control.idleSeconds}
					<span class="computer-idle" role="status">Releasing in {control.idleSeconds}s</span>
					<button
						class="btn small primary"
						type="button"
						title="Keeps control with you. Quiet control is released after a minute without telling the agent to resume."
						onclick={() => control.stillHere()}>Still here</button
					>
				{/if}
				{#if control.owner}
					<button
						class="btn small"
						type="button"
						disabled={!!working || !!control.working}
						title="Returns input to the company actor. It does not complete Work or an owner decision."
						onclick={() => returnControl()}
						>{control.working === 'return' ? 'Releasing…' : 'Release control'}</button
					>
				{/if}
				<button
					class="btn small"
					type="button"
					disabled={!!working}
					title="Restarts a frozen desktop or hung app. Files and the browser profile are kept."
					onclick={() => void restartComputer()}
					>{working === 'restart' ? 'Restarting…' : 'Restart'}</button
				>
				<button
					class="btn small"
					type="button"
					title="Leaves the live computer and restores the Company surface."
					onclick={closeFocus}>Leave computer</button
				>
			</div>
		</header>
		<div class="desktop-notices">
			{#if error || control.error}<div class="computer-error" role="alert">
					{error || control.error}
				</div>{/if}
			{#if signIn}<div class="computer-task" role="status">
					Finish the Claude sign-in in the company browser. Settings shows the account once it
					completes.
				</div>{/if}
		</div>
		<DesktopViewport
			src={desktopUrl}
			title="Live Company computer"
			interactive={control.owner}
			canResize={canResize || control.owner}
			onclaim={claimControl}
			ondropfile={(file) => dropComputerFile(companyId, file)}
			ondisplayresize={requestDisplayResize}
			onload={() => void desktopReady()}
			onactivity={desktopActivity}
		/>
	</div>
{:else}
	<div class="company-computer-portal">
		{#if error}<div class="computer-error portal-error" role="alert">{error}</div>{/if}

		<section class="computer-portal-stage">
			<section class:unavailable={!canAttach} class="computer-portal-machine">
				<div class="computer-portal-bezel">
					<div class="computer-portal-glyph" aria-hidden="true">
						<Monitor size={28} strokeWidth={1.45} />
					</div>
					<h1>Computer</h1>
					<div
						class="computer-portal-controller"
						title={hoursUsedUp
							? 'The monthly hours limit stops it waking for messages, work or schedules until the limit is raised or the month ends.'
							: asleep
								? 'It sleeps when nothing needs it, and wakes for messages, ready work and due schedules.'
								: undefined}
					>
						<span class="source-lamp status-{canAttach ? 'live' : 'stale'}" aria-hidden="true"
						></span>
						{hoursUsedUp
							? 'Asleep · monthly hours used up'
							: asleep
								? 'Asleep'
								: stopped
									? 'Stopped'
									: controllerLabel}
					</div>
					{#if hoursUsedUp}
						<a class="btn primary" href={limitsHref}>Raise the hours limit</a>
						<p>It wakes again once the limit is raised or cleared, or next month.</p>
					{:else if stopped}
						<button
							class="computer-enter"
							type="button"
							disabled={!!working}
							onclick={() => void runLifecycle('start')}
						>
							<span>{working === 'start' ? 'Starting…' : 'Start computer'}</span>
						</button>
						<p>It keeps its files and browser profile while stopped.</p>
					{:else if canAttach || asleep}
						<button
							class="computer-enter"
							type="button"
							disabled={!!working || !clientId}
							title={asleep ? 'Waking usually takes under a minute.' : undefined}
							onclick={() => attachDesktop()}
						>
							<span>{enterLabel}</span>
							<ArrowUpRight size={16} strokeWidth={2} aria-hidden="true" />
						</button>
						<p>
							Your team’s shared browser, <a href={`/${companyId}/company/computer?focus=files`}
								>files</a
							> and applications. Click or type on the desktop to join in.
						</p>
					{:else}
						<p class="computer-unavailable-copy">
							{container === 'Running'
								? 'The desktop isn’t answering, so it can’t be opened.'
								: 'The computer isn’t ready, so it can’t be opened.'}
						</p>
						<a class="btn primary" href={`/${companyId}/company/doctor`}>
							<Activity size={14} strokeWidth={1.8} aria-hidden="true" /> See what needs fixing
						</a>
					{/if}
					{#if container === 'Running'}
						<div class="computer-portal-actions">
							<button
								class="btn small ghost"
								type="button"
								disabled={!!working}
								title="Restarts a frozen desktop or hung app. Files and the browser profile are kept."
								onclick={() => void restartComputer()}
								>{working === 'restart' ? 'Restarting…' : 'Restart'}</button
							>
							<button
								class="btn small ghost"
								type="button"
								disabled={!!working}
								title="Sleeps now if nothing is running. It wakes for messages, ready work and due schedules."
								onclick={() => void sleepComputer()}
								>{working === 'sleep' ? 'Going to sleep…' : 'Sleep now'}</button
							>
						</div>
					{/if}
				</div>
			</section>
		</section>

		<CompanyMachines {companyId} companyName={view?.company.name ?? 'This company'} />

		<footer class="computer-portal-footer">
			<div>
				<span class="source-lamp status-{source.status}" aria-hidden="true"></span>
				<span>{source.status === 'live' ? 'Live' : 'Out of date'}</span>
				<time title={formatMoment(view?.computer.doctor.observed_at)}
					>{when(view?.computer.doctor.observed_at)}</time
				>
			</div>
			{#if preparedHandoffs.length}
				<a href={`/${companyId}`}
					>{preparedHandoffs.length} prepared {preparedHandoffs.length === 1
						? 'handoff'
						: 'handoffs'} in the Inbox</a
				>
			{:else}
				<span>No prepared handoff is waiting.</span>
			{/if}
		</footer>
	</div>
{/if}
