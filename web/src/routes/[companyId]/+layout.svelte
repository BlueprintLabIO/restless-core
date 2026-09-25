<script lang="ts">
	import { page } from '$app/state';
	import { setContext } from 'svelte';
	const setupDraft = $state<{ name: string | null }>({ name: null });
	setContext('company-setup-draft', setupDraft);
	import { goto } from '$app/navigation';
	import AppShell, { type ShellTab } from '$lib/components/AppShell.svelte';
	import type { Command } from '$lib/components/CommandMenu.svelte';
	import { COMPANY_PAGES, companyPageHref } from '$lib/model/company-pages';
	import { workStatusLabel } from '$lib/work/status';
	import { companyBrowserLinks } from '$lib/actions/company-browser-links';
	import CompanyQueryPersistence from '$lib/components/CompanyQueryPersistence.svelte';
	import ExecutiveRail from '$lib/components/ExecutiveRail.svelte';
	import { cockpitContextPath, reviewAction } from '$lib/model/attention';
	import { prepareCompanyBrowser } from '$lib/model/company-browser';
	import {
		collaboratorHome,
		companyShellTabs,
		hasOwnerSurfaceAccess,
		mayOpenCompanyRoute
	} from '$lib/model/company-access';
	import {
		attentionQuery,
		cockpitQuery,
		collaborationBootstrapQuery,
		companiesQuery,
		companyPrincipalQuery,
		conversationQuery
	} from '$lib/model/queries.svelte';
	import { intelligenceQuery } from '$lib/model/intelligence.svelte';
	import { actorCanReceive } from '$lib/model/cockpit';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';

	let { children } = $props();

	const companyId = $derived(page.params.companyId ?? 'aris');
	const principalProjection = $derived(companyPrincipalQuery(companyId));
	const principal = $derived(principalProjection.view);
	const ownerAccess = $derived(hasOwnerSurfaceAccess(principal));
	/* People owns its selected-person conversation, and immersive computer pages
	 * do not render the Exec rail. Do not keep shell-only rail state polling there. */
	const railVisible = $derived.by(() => {
		if (!ownerAccess) return false;
		const path = page.url.pathname;
		const people = `/${companyId}/people`;
		return !(
			path === people ||
			path.startsWith(`${people}/`) ||
			path === `/${companyId}/company/computer` ||
			(path === `/${companyId}` && page.url.searchParams.has('computer'))
		);
	});
	const intelligence = $derived(intelligenceQuery(companyId, () => railVisible));
	const collaboration = $derived(collaborationBootstrapQuery(companyId, () => principal));
	const companyCatalog = companiesQuery(() => ownerAccess);
	const companies = $derived(companyCatalog.view);
	let execRailOpen = $state(true);
	let focusRailRestore = $state<boolean | null>(null);
	let startupStalled = $state(false);
	let retryingStartup = $state(false);
	let startupRetryButton: HTMLButtonElement | undefined = $state();

	/* The shell and the Attention surface read one source rather than polling the
	 * same endpoint on two clocks. The badge can no longer disagree with the
	 * queue it is counting. */
	const attention = $derived(attentionQuery(companyId, () => ownerAccess));
	const cockpitProjection = cockpitQuery(
		() => companyId,
		() => ownerAccess
	);
	const cockpit = $derived(cockpitProjection.view);
	const startupReady = $derived.by(() => {
		if (!principal) return false;
		if (!ownerAccess) return Boolean(collaboration.view);
		return Boolean(attention.view && cockpit);
	});
	const startupFailure = $derived(
		principalProjection.failure ??
			(ownerAccess ? (attention.failure ?? cockpitProjection.failure) : collaboration.failure)
	);
	const startupBlocking = $derived(!startupReady && Boolean(startupFailure || startupStalled));
	$effect(() => {
		if (startupReady) {
			startupStalled = false;
			return;
		}
		startupStalled = false;
		const timeout = window.setTimeout(() => (startupStalled = true), 12_000);
		return () => window.clearTimeout(timeout);
	});
	$effect(() => {
		if (startupBlocking && startupRetryButton) startupRetryButton.focus();
	});

	const companyName = $derived(
		setupDraft.name ??
			(ownerAccess
				? (attention.view?.company.name ?? '')
				: (collaboration.view?.company.name ?? ''))
	);
	const liveNeedsYou = $derived(attention.view?.items ?? []);
	const focusedReviewId = $derived(page.url.searchParams.get('review'));
	const focusedReview = $derived(
		liveNeedsYou.find((item) => item.id === focusedReviewId && item.category === 'review') ?? null
	);
	const focusedConversationId = $derived(page.url.searchParams.get('conversation'));
	const focusedConversation = $derived(
		liveNeedsYou.find((item) => item.id === focusedConversationId) ?? null
	);
	const focusedAttention = $derived(focusedReview ?? focusedConversation);
	const railActorId = $derived(focusedAttention?.responsibleActor?.id ?? 'exec');
	const railConversation = $derived(
		conversationQuery(
			companyId,
			railActorId,
			focusedAttention?.workId,
			focusedAttention?.id,
			() => ownerAccess,
			principal?.actor_id ?? 'owner'
		)
	);
	const railActorName = $derived(
		railActorId === 'exec'
			? 'Exec'
			: (focusedAttention?.responsibleActor?.display ??
					railConversation.actor?.display ??
					railActorId)
	);
	const railActorRole = $derived(focusedAttention ? 'Responsible lead' : 'Executive');
	const railConnectionStatus = $derived.by(() => {
		if (!cockpit) return cockpitProjection.failure ? 'error' : 'unknown';
		if (cockpit.source_health.orgintel !== 'available') return 'error';
		const actorAvailable = actorCanReceive(cockpit, railActorId);
		if (cockpitProjection.status === 'stale' && !actorAvailable) return 'error';
		return actorAvailable ? 'available' : 'unavailable';
	});
	const railConnected = $derived(railConnectionStatus === 'available');
	const companyComputerSurface = $derived(page.url.pathname === `/${companyId}/company/computer`);
	const immersiveComputer = $derived(
		(companyComputerSurface && page.url.searchParams.get('focus') === 'desktop') ||
			(page.url.pathname === `/${companyId}` && page.url.searchParams.has('computer'))
	);
	$effect(() => railConversation.attach());
	$effect(() => {
		const authenticated = principal;
		if (!authenticated || mayOpenCompanyRoute(companyId, page.url.pathname, authenticated)) return;
		void goto(collaboratorHome(companyId), { replaceState: true });
	});
	$effect(() => {
		if (focusedAttention) execRailOpen = true;
	});
	/* Until intelligence is connected the rail can only say so, and the start
	 * blocker already says it on Attention. Start it closed once so the work
	 * surface keeps the width; the owner can still open it. */
	let railClosedForSetup = '';
	$effect(() => {
		if (railClosedForSetup === companyId || focusedAttention) return;
		if (intelligence.view?.has_connections === false) {
			railClosedForSetup = companyId;
			execRailOpen = false;
		}
	});
	$effect(() => {
		if (companyComputerSurface && focusRailRestore === null) {
			focusRailRestore = execRailOpen;
			execRailOpen = false;
		} else if (!companyComputerSurface && focusRailRestore !== null) {
			execRailOpen = focusRailRestore;
			focusRailRestore = null;
		}
	});

	$effect(() => {
		/* The executive is a persistent sibling on desktop, but its small-screen
		 * presentation is a full workspace overlay. Start that overlay closed unless
		 * a focused Attention item explicitly needs it. */
		if (
			typeof window !== 'undefined' &&
			window.matchMedia('(max-width: 980px)').matches &&
			!focusedAttention
		) {
			execRailOpen = false;
		}
	});

	async function askRail(
		text: string,
		files: File[],
		includeContext: boolean,
		newFocus: boolean,
		interrupt: boolean,
		outcomeStandard?: import('$lib/model/company').OutcomeStandard,
		skills: string[] = []
	): Promise<{ error?: string; notice?: string }> {
		if (!ownerAccess) return { error: 'This company membership cannot send owner directions.' };
		try {
			const contextPath = includeContext ? cockpitContextPath(companyId, page.url) : undefined;
			const result = await railConversation.send(
				text,
				files,
				contextPath,
				newFocus,
				interrupt,
				outcomeStandard,
				skills
			);
			if (interrupt) {
				return {
					notice: result.interrupted
						? `The current turn was interrupted; your direction is queued for ${railActorName}.`
						: `The turn ended before interruption; your direction is queued for ${railActorName}.`
				};
			}
			return includeContext && (!contextPath || result.contextOmitted)
				? { notice: 'Message sent without the current-screen link.' }
				: {};
		} catch (cause) {
			const status = (cause as { status?: unknown } | null)?.status;
			// The conversation query retains this exact send's command ID for a safe retry.
			if (typeof status !== 'number' || status >= 500 || [408, 425, 429].includes(status)) {
				return {
					error:
						'We could not confirm that your message was sent. Your draft is still here. Try sending it again.'
				};
			}
			return {
				error: cause instanceof Error ? cause.message : 'Your message was not delivered.'
			};
		}
	}

	async function decideFocusedReview(
		decision: 'accept' | 'request_changes',
		feedback: string
	): Promise<string | null> {
		if (!focusedReview) return 'This review is no longer outstanding.';
		try {
			await reviewAction(companyId, focusedReview.source.reference, decision, feedback);
			await attention.refresh();
			await goto(`/${companyId}`);
			return null;
		} catch (cause) {
			return cause instanceof Error ? cause.message : 'The review decision was not recorded.';
		}
	}

	function closeFocusedContext() {
		if (!focusedAttention) return;
		/* On small screens the executive rail is a full-workspace overlay. Closing
		 * it must reveal the focused outcome underneath, not also discard that
		 * outcome's URL context. The actor button can reopen the same lead and
		 * decision controls without rebuilding any review state. */
		if (typeof window !== 'undefined' && window.matchMedia('(max-width: 980px)').matches) {
			execRailOpen = false;
			return;
		}
		void goto(`/${companyId}?item=${encodeURIComponent(focusedAttention.id)}`);
	}

	const currentContext = $derived.by(() => {
		const path = page.url.pathname;
		const root = `/${companyId}`;
		if (path === `${root}/work` || path.startsWith(`${root}/work/`)) return 'Linked · Work';
		if (path === `${root}/people` || path.startsWith(`${root}/people/`)) return 'Linked · People';
		if (path === `${root}/company` || path.startsWith(`${root}/company/`))
			return 'Linked · Company';
		return 'Linked · Attention';
	});

	const tabs = $derived.by((): ShellTab[] => {
		return companyShellTabs(
			companyId,
			page.url.pathname,
			principal,
			attention.status === 'unknown' ? undefined : liveNeedsYou.length
		);
	});

	/* Owner-only destinations for the command menu. Collaborators keep the
	 * surface moves the shell adds for everyone. */
	const commands = $derived.by((): Command[] => {
		if (!ownerAccess) return [];
		const root = `/${encodeURIComponent(companyId)}`;
		const work = attention.view?.workGraph?.work ?? [];
		const goalTitle = new Map((cockpit?.goals ?? []).map((goal) => [goal.id, goal.title]));
		return [
			...(attention.view?.items ?? []).map((item) => ({
				id: `attention:${item.id}`,
				group: 'Needs you',
				label: item.title,
				hint: item.requestedAction,
				href: `${root}?item=${encodeURIComponent(item.id)}`
			})),
			...work
				.filter((item) => item.status !== 'abandoned')
				.map((item) => ({
					id: `work:${item.id}`,
					group: 'Work',
					label: item.title,
					hint: `${workStatusLabel(item.status)}${item.goal_id && goalTitle.has(item.goal_id) ? ` · ${goalTitle.get(item.goal_id)}` : ''}`,
					href: `${root}/work/${encodeURIComponent(item.id)}`
				})),
			...(cockpit?.goals ?? [])
				.filter((goal) => !goal.closed_at)
				.map((goal) => ({
					id: `goal:${goal.id}`,
					group: 'Goals',
					label: goal.title,
					href: `${root}/work?goal=${encodeURIComponent(goal.id)}`
				})),
			...(cockpit?.people ?? [])
				.filter((person) => !['system', 'owner'].includes(person.kind))
				.map((person) => ({
					id: `person:${person.actor_id}`,
					group: 'People',
					label: person.display,
					hint: person.kind === 'exec' ? 'Executive' : person.role,
					keywords: 'message talk chat',
					href: `${root}/people?person=${encodeURIComponent(person.actor_id)}`
				})),
			...COMPANY_PAGES.map((companyPage) => ({
				id: `company:${companyPage.key}`,
				group: 'Company',
				label: companyPage.label,
				hint: companyPage.section,
				keywords: `settings ${companyPage.keywords ?? ''}`,
				href: companyPageHref(encodeURIComponent(companyId), companyPage)
			})),
			...companies
				.filter((company) => company.lifecycle_status === 'active' && company.id !== companyId)
				.map((company) => ({
					id: `switch:${company.id}`,
					group: 'Switch company',
					label: company.name,
					href: `/${encodeURIComponent(company.id)}`
				})),
			{ id: 'portfolio', group: 'Switch company', label: 'All companies', href: '/' }
		];
	});

	const childAllowed = $derived(mayOpenCompanyRoute(companyId, page.url.pathname, principal));

	function openInCompanyBrowser(url: string) {
		prepareCompanyBrowser(companyId, url);
		void goto(`/${companyId}/company/computer?focus=browser`, { noScroll: true });
	}

	async function retryStartup() {
		if (retryingStartup) return;
		retryingStartup = true;
		try {
			await Promise.allSettled([principalProjection.refresh()]);
			if (ownerAccess) {
				await Promise.allSettled([attention.refresh(), cockpitProjection.refresh()]);
			} else {
				await Promise.allSettled([collaboration.refresh()]);
			}
		} finally {
			retryingStartup = false;
		}
	}
</script>

<CompanyQueryPersistence {companyId} />

{#snippet executiveRail()}
	<ExecutiveRail
		messages={railConversation.messages}
		participantName={railActorName}
		participantId={railActorId}
		participantRole={railActorRole}
		turn={railConversation.activeTurn}
		{companyId}
		membershipRole={principal?.membership_role ?? 'member'}
		connected={railConnected}
		connectionStatus={railConnectionStatus}
		conversationStatus={railConversation.status}
		conversationFailed={Boolean(railConversation.failure)}
		onrefreshConversation={() => void railConversation.refresh()}
		needsProvider={intelligence.view?.has_connections === false}
		contextLabel={currentContext}
		focusAfterMessageId={railConversation.focusAfterMessageId}
		focusStartedAt={railConversation.focusStartedAt}
		newFocusAvailable={railActorId === 'exec' &&
			!focusedAttention &&
			intelligence.view?.has_connections !== false}
		open={execRailOpen}
		onask={askRail}
		review={focusedReview
			? {
					onback: closeFocusedContext,
					ondecide: decideFocusedReview
				}
			: null}
		workContext={focusedAttention ? { onback: closeFocusedContext } : null}
	/>
{/snippet}

<div class="company-browser-link-capture" use:companyBrowserLinks={{ open: openInCompanyBrowser }}>
	<AppShell
		{companyId}
		companyName={companyName || companyId.charAt(0).toUpperCase() + companyId.slice(1)}
		{companies}
		{tabs}
		{commands}
		homeHref={ownerAccess ? '/' : collaboratorHome(companyId)}
		canSwitchCompanies={ownerAccess}
		execHref={ownerAccess &&
		(page.url.pathname === `/${companyId}/people` ||
			page.url.pathname.startsWith(`/${companyId}/people/`))
			? `/${companyId}/people?person=exec`
			: null}
		execName={railActorName}
		execLive={railConnected && intelligence.view?.has_connections !== false}
		railOpen={execRailOpen}
		expandExec={page.url.pathname === `/${companyId}` &&
			attention.status === 'live' &&
			liveNeedsYou.length === 0 &&
			intelligence.view?.has_connections !== false &&
			!page.url.searchParams.has('computer') &&
			!focusedAttention}
		immersive={immersiveComputer}
		blocked={startupBlocking}
		onexectoggle={() => (execRailOpen = !execRailOpen)}
		rail={railVisible ? executiveRail : null}
	>
		{#if childAllowed}
			{@render children()}
		{:else if principalProjection.failure}
			<section class="company-access-state cockpit-pane" role="alert">
				<h1>Company unavailable</h1>
				<p>{principalProjection.failure.message}</p>
			</section>
		{:else}
			<section class="company-access-state cockpit-pane" role="status" aria-live="polite">
				<p>{principal ? 'Opening your company workspace…' : 'Verifying company access…'}</p>
			</section>
		{/if}
	</AppShell>
	{#if startupBlocking}
		<div class="startup-error-scrim">
			<div
				class="startup-error cockpit-pane"
				role="alertdialog"
				aria-modal="true"
				aria-labelledby="startup-error-title"
				aria-describedby="startup-error-copy"
			>
				<div class="startup-error-mark" aria-hidden="true">
					<TriangleAlert size={18} strokeWidth={1.8} />
				</div>
				<h1 id="startup-error-title">We couldn’t open this company</h1>
				<p id="startup-error-copy">
					{startupFailure
						? 'We couldn’t complete the company check. Try again to continue.'
						: 'This is taking longer than expected. The page is still here, and you can try reconnecting.'}
				</p>
				<button
					bind:this={startupRetryButton}
					class="btn primary"
					type="button"
					disabled={retryingStartup}
					onclick={retryStartup}
				>
					{retryingStartup ? 'Reconnecting…' : 'Try again'}
				</button>
				<span class="startup-error-note">Your open page and drafts are being kept in place.</span>
			</div>
		</div>
	{/if}
</div>

<style>
	.company-access-state {
		width: 100%;
		display: grid;
		place-content: center;
		gap: var(--space-2);
		padding: var(--space-6);
		text-align: center;
	}
	.company-browser-link-capture {
		display: contents;
	}
	.startup-error-scrim {
		position: fixed;
		z-index: var(--z-overlay);
		inset: 0;
		display: grid;
		place-items: center;
		padding: 20px;
		background: rgba(26, 33, 47, 0.3);
		backdrop-filter: blur(7px) saturate(0.82);
		-webkit-backdrop-filter: blur(7px) saturate(0.82);
	}
	.startup-error {
		width: min(100%, 440px);
		display: grid;
		justify-items: center;
		gap: 12px;
		padding: clamp(24px, 5vw, 40px);
		border: 1px solid rgba(76, 88, 117, 0.18);
		border-radius: 6px;
		background: #fafbfe;
		box-shadow: 0 8px 28px rgba(65, 76, 104, 0.12);
		color: #293244;
		font: var(--t-body) / 1.55 var(--font-ui);
		text-align: center;
	}
	.startup-error-mark {
		width: 40px;
		height: 40px;
		display: grid;
		place-items: center;
		border: 1px solid rgba(155, 84, 91, 0.25);
		border-radius: 4px;
		background: #f7e9eb;
		color: #9b545b;
	}
	.startup-error h1,
	.startup-error p {
		margin: 0;
	}
	.startup-error h1 {
		font-size: var(--t-title);
		font-weight: 600;
	}
	.startup-error p {
		max-width: 34ch;
		color: #687487;
		line-height: 1.55;
	}
	.startup-error .btn {
		min-width: 140px;
		margin-top: 4px;
		padding: 9px 18px;
		border: 1px solid #cdd5e2;
		border-radius: 4px;
		background: #e8eff8;
		color: #456687;
		font: 600 var(--t-body) var(--font-ui);
		cursor: pointer;
	}
	.startup-error .btn:disabled {
		opacity: 0.55;
		cursor: wait;
	}
	.startup-error .btn:focus-visible {
		outline: 2px solid #456687;
		outline-offset: 2px;
	}
	.startup-error-note {
		color: #687487;
		font-size: var(--t-body);
	}

	.company-access-state h1,
	.company-access-state p {
		margin: 0;
	}

	.company-access-state h1 {
		font-size: var(--t-head);
	}

	.company-access-state p {
		color: var(--text-tertiary);
	}
</style>
