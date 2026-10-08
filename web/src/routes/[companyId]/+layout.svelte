<script lang="ts">
	import WelcomeTour from '$lib/components/WelcomeTour.svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { prefetchOnIntent } from '$lib/model/prefetch';
	import { describeFailure, failureSentence } from '$lib/model/failure';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/ui/glyph/MatrixGlyph.svelte';
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
	import { referenceOptions } from '$lib/model/composer-options.svelte';
	import type ExecutiveRailView from '$lib/components/ExecutiveRail.svelte';
	import { presence } from '$lib/model/presence.svelte';
	import { desktopNotify } from '$lib/model/desktop-notify.svelte';
	import { ASK_EXEC_EVENT } from '$lib/model/ask-exec';
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
		companyChangeStream,
		companyPrincipalQuery,
		conversationQuery
	} from '$lib/model/queries.svelte';
	import { agentRouteState, intelligenceQuery } from '$lib/model/intelligence.svelte';
	import { affectsCompany, watchIntelligenceChanges } from '$lib/model/intelligence-events';
	import { onMount } from 'svelte';
	import { startLinkLabel } from '$lib/model/company-start';
	import { actorCanReceive } from '$lib/model/cockpit';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
	import { inPlace } from '$lib/transition';
	import { pageContext } from '$lib/model/page-context.svelte';
	import { provideReferencePreview } from '$lib/model/reference-preview';
	import { documentsQuery } from '$lib/model/document-queries.svelte';
	import { fetchConnections } from '$lib/model/connections';
	import { fetchAppRequests } from '$lib/model/app-requests';
	import { CATALOGUE, buildApps } from '$lib/model/apps';

	let { children } = $props();

	const companyId = $derived(page.params.companyId ?? 'aris');
	const principalProjection = $derived(companyPrincipalQuery(companyId));
	const principal = $derived(principalProjection.view);
	const ownerAccess = $derived(hasOwnerSurfaceAccess(principal));
	/* The rail's conversation, composer and Markdown are a large share of a
	 * company page's code. Phones start with the rail closed, so it loads when
	 * first opened, or as soon as the owner starts interacting. */
	let ExecutiveRail = $state<typeof ExecutiveRailView | null>(null);
	let railLoading: Promise<void> | null = null;
	function loadRail() {
		railLoading ??= import('$lib/components/ExecutiveRail.svelte').then(
			(module) => void (ExecutiveRail = module.default)
		);
		return railLoading;
	}
	$effect(() => {
		/* People shows the Exec and leads in the rail at full width. */
		if (execRailOpen || wideActor) void loadRail();
	});
	onMount(() => {
		const early = () => void loadRail();
		const events = ['pointerdown', 'keydown'] as const;
		for (const name of events) window.addEventListener(name, early, { once: true, passive: true });
		return () => {
			for (const name of events) window.removeEventListener(name, early);
		};
	});

	/* People owns its selected-person conversation, and the immersive desktop does
	 * not render the Exec rail. The Company browser keeps it: a page Exec brings to
	 * the owner is discussed beside it, as an Attention item's live browser is.
	 * Do not keep shell-only rail state polling where it is hidden. */
	const peopleConversation = $derived(
		ownerAccess &&
			page.url.pathname === `/${companyId}/people` &&
			page.url.searchParams.has('person')
	);
	const railVisible = $derived.by(() => {
		if (!ownerAccess) return false;
		const path = page.url.pathname;
		const people = `/${companyId}/people`;
		return !(
			path === people ||
			path.startsWith(`${people}/`) ||
			(path === `/${companyId}/company/computer` &&
				page.url.searchParams.get('focus') === 'desktop') ||
			(path === `/${companyId}` && page.url.searchParams.has('computer'))
		);
	});
	const intelligence = $derived(
		intelligenceQuery(companyId, () => railVisible || peopleConversation)
	);
	onMount(() =>
		watchIntelligenceChanges((changed) => {
			if (affectsCompany(changed, companyId)) void intelligence.refresh();
		})
	);
	const queryClient = useQueryClient();
	const collaboration = $derived(
		collaborationBootstrapQuery(companyId, () => (ownerAccess ? null : principal))
	);
	const companyCatalog = companiesQuery(() => ownerAccess);
	const companies = $derived(companyCatalog.view);
	/* The rail keeps the owner's last choice per company, so a reload does not
	 * open it only to close it again. Phones always start with the overlay
	 * closed. "setup" marks a rail closed because intelligence was missing;
	 * it reopens once when intelligence is connected. */
	const railKey = (id: string) => `restless:exec-rail:${id}`;
	function readRail(id: string): string | null {
		try {
			return localStorage.getItem(railKey(id));
		} catch {
			return null;
		}
	}
	function writeRail(id: string, value: 'open' | 'closed' | 'setup') {
		try {
			localStorage.setItem(railKey(id), value);
		} catch {
			/* Storage is a convenience; the rail still works without it. */
		}
	}
	function initialRail(id: string): boolean {
		if (window.matchMedia('(max-width: 980px)').matches) return false;
		const stored = readRail(id);
		return stored === null || stored === 'open';
	}
	let execRailOpen = $state(initialRail(page.params.companyId ?? ''));
	let railCompany = page.params.companyId ?? '';
	let focusRailRestore = $state<boolean | null>(null);
	let startupStalled = $state(false);
	let retryingStartup = $state(false);
	let startupRetryButton: HTMLButtonElement | undefined = $state();

	/* The shell and the Attention surface read one source rather than polling the
	 * same endpoint on two clocks. The badge can no longer disagree with the
	 * queue it is counting. */
	companyChangeStream(
		() => companyId,
		() => ownerAccess
	);
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
				? (attention.view?.company.name ??
					companies.find((company) => company.id === companyId)?.name ??
					'')
				: (collaboration.view?.company.name ?? ''))
	);
	/* The last name this browser saw for the company stands in until the source
	 * answers, so the topbar does not re-flow from an id-shaped placeholder. */
	const nameKey = (id: string) => `restless:company-name:${id}`;
	function rememberedName(id: string): string {
		try {
			return localStorage.getItem(nameKey(id)) ?? '';
		} catch {
			return '';
		}
	}
	$effect(() => {
		if (!companyName) return;
		try {
			localStorage.setItem(nameKey(companyId), companyName);
		} catch {
			/* A convenience only. */
		}
	});
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

	/* Who the rail talks to, and about what. The Exec and a lead each hold one
	 * direct conversation, with the current page linked; a piece of Work has
	 * its own thread only with the person producing it (OrgIntel keeps Work
	 * feedback with its owner). The owner picks from the rail header; an Inbox
	 * focus decides while that focus is open. */
	type RailTopic = { actorId: string; workId?: string };
	let chosenTopic = $state<RailTopic | null>(null);
	let topicCompany = '';
	$effect(() => {
		if (companyId === topicCompany) return;
		topicCompany = companyId;
		chosenTopic = null;
	});
	const topicsKey = $derived(`restless:${companyId}:rail-topics`);
	function readTopics(): RailTopic[] {
		try {
			const stored = JSON.parse(localStorage.getItem(topicsKey) ?? '[]');
			return Array.isArray(stored) ? stored.slice(0, 6) : [];
		} catch {
			return [];
		}
	}
	let recentTopics = $state<RailTopic[]>([]);
	$effect(() => {
		topicsKey;
		recentTopics = readTopics();
	});
	function chooseTopic(topic: RailTopic | null) {
		chosenTopic = topic && (topic.actorId !== 'exec' || topic.workId) ? topic : null;
		if (!chosenTopic) return;
		const next = [
			chosenTopic,
			...recentTopics.filter(
				(t) => !(t.actorId === chosenTopic!.actorId && t.workId === chosenTopic!.workId)
			)
		].slice(0, 6);
		recentTopics = next;
		try {
			localStorage.setItem(topicsKey, JSON.stringify(next));
		} catch {
			/* Recents are a convenience. */
		}
	}
	const workRows = $derived(attention.view?.workGraph?.work ?? []);
	/* Documents the composer offers after `#`. */
	const libraryDocuments = $derived(documentsQuery(ownerAccess ? companyId : ''));
	const workTitle = (id: string) => workRows.find((work) => work.id === id)?.title ?? 'Work';
	function personName(actorId: string): string {
		if (actorId === 'exec') return 'Exec';
		return cockpit?.people.find((person) => person.actor_id === actorId)?.display ?? actorId;
	}
	/* The lead accountable for a piece of Work: its team's lead, or its owner. */
	function leadFor(workId: string): string {
		const owner = workRows.find((work) => work.id === workId)?.owner_id ?? '';
		const person = cockpit?.people.find((candidate) => candidate.actor_id === owner);
		const team = cockpit?.teams.find(
			(candidate) => candidate.lead_actor_id === owner || candidate.id === person?.team_id
		);
		return team?.lead_actor_id ?? owner;
	}
	/* People renders the wide conversation where its selected person goes. */
	setContext('wide-conversation', {
		get actor() {
			return wideActor;
		},
		render: () => executiveRail
	});
	/* What a Work, Goal or person chip in a message names, for its hover card. */
	provideReferencePreview((href) => {
		let url: URL;
		try {
			url = new URL(href, window.location.origin);
		} catch {
			return null;
		}
		const root = `/${companyId}`;
		const workId = url.pathname.match(new RegExp(`^${root}/work/([^/]+)$`))?.[1];
		if (workId) {
			const work = workRows.find((row) => row.id === decodeURIComponent(workId));
			if (!work) return null;
			const goal = cockpit?.goals.find((candidate) => candidate.id === work.goal_id)?.title;
			return {
				title: work.title,
				lines: [
					`${workStatusLabel(work.status)} · ${personName(work.owner_id)}`,
					...(goal ? [`Goal: ${goal}`] : [])
				]
			};
		}
		const goalId = url.pathname === `${root}/work` ? url.searchParams.get('goal') : null;
		if (goalId) {
			const goal = cockpit?.goals.find((candidate) => candidate.id === goalId);
			if (!goal) return null;
			const open = workRows.filter(
				(row) => row.goal_id === goalId && !['completed', 'abandoned'].includes(row.status)
			).length;
			return { title: goal.title, lines: [`Goal · ${open} open Work`] };
		}
		const personId = url.pathname === `${root}/people` ? url.searchParams.get('person') : null;
		if (personId) {
			const person = cockpit?.people.find((candidate) => candidate.actor_id === personId);
			if (!person) return null;
			const team = cockpit?.teams.find((candidate) => candidate.id === person.team_id)?.name;
			return {
				title: person.display,
				lines: [
					person.kind === 'exec'
						? 'Executive'
						: `${person.role.charAt(0).toUpperCase()}${person.role.slice(1)}${team ? ` · ${team}` : ''}`
				]
			};
		}
		return null;
	});
	const pageWorkId = $derived(
		page.url.pathname.match(new RegExp(`^/${companyId}/work/([^/]+)$`))?.[1] ?? ''
	);
	const railTopics = $derived.by(() => {
		const entries: { key: string; label: string; hint: string; topic: RailTopic | null }[] = [
			{ key: 'general', label: 'General', hint: 'Exec', topic: null }
		];
		const add = (topic: RailTopic, hint?: string) => {
			const key = `${topic.actorId}:${topic.workId ?? ''}`;
			if (entries.some((entry) => entry.key === key) || topic.actorId === 'exec') return;
			entries.push(
				topic.workId
					? { key, label: workTitle(topic.workId), hint: hint ?? personName(topic.actorId), topic }
					: { key, label: personName(topic.actorId), hint: hint ?? 'Direct conversation', topic }
			);
		};
		if (pageWorkId) {
			const lead = leadFor(pageWorkId);
			const owner = workRows.find((work) => work.id === pageWorkId)?.owner_id ?? '';
			if (lead) add({ actorId: lead }, 'Leads this Work');
			if (owner && owner !== lead)
				add({ actorId: owner, workId: pageWorkId }, `${personName(owner)} · working on it`);
		}
		for (const topic of recentTopics)
			if (!topic.workId || workRows.some((work) => work.id === topic.workId)) add(topic);
		return entries;
	});
	/* A page can start a conversation about its Work with ?talk=<actor>; the
	 * layout takes it as the chosen topic and drops it from the address. */
	$effect(() => {
		const talk = page.url.searchParams.get('talk');
		if (!talk || !pageWorkId) return;
		const owner = workRows.find((work) => work.id === pageWorkId)?.owner_id;
		chooseTopic(
			talk === owner && talk !== leadFor(pageWorkId)
				? { actorId: talk, workId: pageWorkId }
				: { actorId: talk }
		);
		execRailOpen = true;
		const url = new URL(page.url);
		url.searchParams.delete('talk');
		void goto(url, { replaceState: true, keepFocus: true, noScroll: true });
	});
	/* People opens the Exec and team leads in this same conversation, full
	 * width, instead of a second copy of it: one conversation, two sizes. */
	const wideActor = $derived.by(() => {
		if (!ownerAccess || page.url.searchParams.has('room')) return '';
		const path = page.url.pathname;
		if (path !== `/${companyId}/people`) return '';
		const person = page.url.searchParams.get('person') ?? '';
		if (person === 'exec') return person;
		return cockpit?.teams.some((team) => team.lead_actor_id === person) ? person : '';
	});
	const railActorId = $derived(
		wideActor || (focusedAttention?.responsibleActor?.id ?? chosenTopic?.actorId ?? 'exec')
	);
	const railWorkId = $derived(
		wideActor ? undefined : (focusedAttention?.workId ?? chosenTopic?.workId)
	);
	const railTopicKey = $derived(
		wideActor
			? wideActor === 'exec'
				? 'general'
				: `${wideActor}:`
			: chosenTopic
				? `${chosenTopic.actorId}:${chosenTopic.workId ?? ''}`
				: 'general'
	);
	const railTopicLabel = $derived(
		focusedAttention
			? ''
			: chosenTopic?.workId
				? workTitle(chosenTopic.workId)
				: chosenTopic
					? ''
					: 'General'
	);
	const railConversation = $derived(
		conversationQuery(
			companyId,
			railActorId,
			railWorkId,
			wideActor ? undefined : focusedAttention?.id,
			() => ownerAccess,
			principal?.actor_id ?? 'owner'
		)
	);
	const railActorName = $derived(
		railActorId === 'exec'
			? 'Exec'
			: (focusedAttention?.responsibleActor?.display ??
					railConversation.actor?.display ??
					personName(railActorId))
	);
	const railActorRole = $derived(
		focusedAttention || railActorId !== 'exec' ? 'Responsible lead' : 'Executive'
	);
	const providerIssue = $derived(
		companies.find((company) => company.id === companyId)?.unstartable_reason ?? ''
	);
	const railRouteState = $derived(agentRouteState(intelligence.view, railActorId));
	const railConnectionStatus = $derived.by(() => {
		if (providerIssue) return 'unavailable';
		if (!cockpit) return cockpitProjection.failure ? 'error' : 'unknown';
		if (cockpit.source_health.orgintel !== 'available') return 'error';
		const actorAvailable = actorCanReceive(cockpit, railActorId);
		if (cockpitProjection.status === 'stale' && !actorAvailable) return 'error';
		if (!actorAvailable) return 'unavailable';
		if (intelligence.error) return 'error';
		if (railRouteState === 'checking' || railRouteState === 'starting') return 'unknown';
		return railRouteState === 'ready' ? 'available' : 'unavailable';
	});
	const railConnected = $derived(railConnectionStatus === 'available');
	const companyComputerSurface = $derived(page.url.pathname === `/${companyId}/company/computer`);
	const immersiveComputer = $derived(
		(companyComputerSurface && page.url.searchParams.get('focus') === 'desktop') ||
			(page.url.pathname === `/${companyId}` && page.url.searchParams.has('computer'))
	);
	const railYields = $derived(immersiveComputer);
	/* Meeting a new owner is a page of its own: no top bar, no rail, nothing else on screen. */
	const welcoming = $derived(page.url.pathname === `/${companyId}/welcome`);
	/* Straight after meeting Exec: the conversation is now the rail, and a short tour shows the
	 * owner around. */
	const touring = $derived(page.url.searchParams.get('tour') === '1');
	$effect(() => {
		if (touring) execRailOpen = true;
	});
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
		const route = agentRouteState(intelligence.view, 'exec');
		if (route === 'needs_connection' || route === 'unavailable') {
			railClosedForSetup = companyId;
			// Only the default gives way to setup; an owner's own choice stands.
			if (readRail(companyId) === null) {
				execRailOpen = false;
				writeRail(companyId, 'setup');
			}
		} else if (route === 'ready' && readRail(companyId) === 'setup') {
			railClosedForSetup = companyId;
			execRailOpen = true;
		}
	});
	$effect(() => {
		if (companyId === railCompany) return;
		railCompany = companyId;
		execRailOpen = initialRail(companyId);
	});
	$effect(() => {
		// Remember deliberate desktop choices only, not temporary closes.
		const open = execRailOpen;
		if (
			focusRailRestore !== null ||
			focusedAttention ||
			companyId !== railCompany ||
			window.matchMedia('(max-width: 980px)').matches
		)
			return;
		if (open) writeRail(companyId, 'open');
		else if (readRail(companyId) !== 'setup') writeRail(companyId, 'closed');
	});
	$effect(() => {
		if (railYields && focusRailRestore === null) {
			focusRailRestore = execRailOpen;
			execRailOpen = false;
		} else if (!railYields && focusRailRestore !== null) {
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
			const busy = Boolean(railConversation.activeTurn);
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
			if (includeContext && (!contextPath || result.contextOmitted))
				return { notice: 'Message sent without the current-screen link.' };
			// A message sent mid-reply says "Queued" on itself, not under the composer.
			return {};
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
				error: failureSentence(cause, 'Your message was not delivered.')
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
			return failureSentence(cause, 'The review decision was not recorded.');
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

	/* What the next rail message will be linked to, by name: the Work, Goal,
	 * document or person on screen rather than the section it sits in. */
	const currentContext = $derived.by((): { kind: string; label: string } => {
		const url = page.url;
		const path = url.pathname;
		const root = `/${companyId}`;
		const workId = path.match(new RegExp(`^${root}/work/([^/]+)$`))?.[1];
		if (workId) return { kind: 'work', label: workTitle(decodeURIComponent(workId)) };
		if (path === `${root}/work` || path.startsWith(`${root}/work/`)) {
			const goal = url.searchParams.get('goal');
			const title = goal ? cockpit?.goals.find((candidate) => candidate.id === goal)?.title : '';
			return { kind: 'work', label: title || 'Work' };
		}
		if (path === `${root}/library` || path.startsWith(`${root}/library/`)) {
			const fallback = path.endsWith('/documents')
				? url.searchParams.has('document')
					? 'Document'
					: 'Documents'
				: path.endsWith('/sheets')
					? url.searchParams.has('sheet')
						? 'Sheet'
						: 'Sheets'
					: 'Library';
			return { kind: 'library', label: pageContext.title || fallback };
		}
		if (path === `${root}/people` || path.startsWith(`${root}/people/`)) {
			const person = url.searchParams.get('person');
			return { kind: 'people', label: person ? personName(person) : 'People' };
		}
		if (path === `${root}/company` || path.startsWith(`${root}/company/`)) {
			const companyPage = COMPANY_PAGES.find(
				(candidate) => path === `${root}/company${candidate.path}`
			);
			return { kind: 'company', label: companyPage?.label ?? 'Company' };
		}
		const item = url.searchParams.get('item');
		const title = item ? liveNeedsYou.find((candidate) => candidate.id === item)?.title : '';
		return { kind: 'inbox', label: title || 'Inbox' };
	});

	const tabs = $derived.by((): ShellTab[] => {
		const routes = companyShellTabs(
			companyId,
			page.url.pathname,
			principal,
			attention.status === 'unknown' ? undefined : liveNeedsYou.length
		);
		return routes.map((tab) =>
			tab.key === 'company' && providerIssue
				? { ...tab, badge: 1 }
				: tab.key === 'apps' && appsAttention
					? { ...tab, dot: appsAttention }
					: tab
		);
	});

	/* The Apps dot: a sign-in to renew, a tool to review, or an app Exec asked
	 * for. Checked on arrival, on every move and once a minute. Know-how
	 * candidates are counted on the Apps page itself, not here, because reading
	 * the skill library scans the company computer. */
	let appsAttention = $state('');
	$effect(() => {
		if (!ownerAccess) return;
		const id = companyId;
		void page.url.pathname;
		let stopped = false;
		const check = async () => {
			try {
				const [tools, asked] = await Promise.all([
					fetchConnections(id),
					fetchAppRequests(id).catch(() => [])
				]);
				if (stopped) return;
				const count =
					buildApps(tools, null).mine.filter((app) => app.state === 'needs_you').length +
					asked.length;
				appsAttention = !count ? '' : count === 1 ? '1 app needs you' : `${count} apps need you`;
			} catch {
				/* A failed check leaves the dot as it was; Apps shows the failure. */
			}
		};
		void check();
		const timer = setInterval(() => void check(), 60_000);
		return () => {
			stopped = true;
			clearInterval(timer);
		};
	});

	/* Owner-only destinations for the command menu. Collaborators keep the
	 * surface moves the shell adds for everyone. */
	/* Others in this company's cockpit, and the signed-in person's circle. */
	const others = presence(() => companyId);
	let railFocusRequest = $state(0);
	let railDraft = $state<{ text: string; key: number } | null>(null);
	$effect(() => {
		const ask = (event: Event) => {
			chooseTopic(null);
			execRailOpen = true;
			railFocusRequest += 1;
			const draft = (event as CustomEvent<{ draft?: string }>).detail?.draft;
			if (draft) railDraft = { text: draft, key: railFocusRequest };
			/* People has no side rail; the Exec opens there at full width. */
			if (page.url.pathname.startsWith(`/${companyId}/people`) && wideActor !== 'exec')
				void goto(`/${encodeURIComponent(companyId)}/people?person=exec`);
		};
		window.addEventListener(ASK_EXEC_EVENT, ask);
		return () => window.removeEventListener(ASK_EXEC_EVENT, ask);
	});
	const viewerName = $derived(
		(ownerAccess ? cockpit?.people : collaboration.view?.people)?.find(
			(person) => person.actor_id === principal?.actor_id
		)?.display ?? (ownerAccess ? 'Owner' : 'Member')
	);
	const viewerRole = $derived(
		`${principal?.membership_role === 'owner' ? 'Owner' : 'Member'} · ${companyName || companyId}`
	);

	/* Desktop notifications: tell this device about anything new that needs
	 * the owner while the cockpit is in the background. The first projection
	 * only primes what is already known. */
	let knownNeeds: Set<string> | null = null;
	$effect(() => {
		if (!ownerAccess || attention.status !== 'live') return;
		const items = attention.view?.items ?? [];
		const ids = new Set(items.map((item) => item.id));
		if (knownNeeds) {
			for (const item of items) {
				if (knownNeeds.has(item.id) || item.preparing) continue;
				desktopNotify.show(
					item.title,
					item.requestedAction || item.whatHappened,
					() => void goto(`/${encodeURIComponent(companyId)}?item=${encodeURIComponent(item.id)}`)
				);
			}
		}
		knownNeeds = ids;
	});

	const commands = $derived.by((): Command[] => {
		if (!ownerAccess) return [];
		const root = `/${encodeURIComponent(companyId)}`;
		const work = attention.view?.workGraph?.work ?? [];
		const goalTitle = new Map((cockpit?.goals ?? []).map((goal) => [goal.id, goal.title]));
		const leads = (cockpit?.teams ?? [])
			.map((team) => cockpit?.people.find((person) => person.actor_id === team.lead_actor_id))
			.filter(
				(person): person is NonNullable<typeof person> => !!person && person.actor_id !== 'exec'
			);
		return [
			{
				id: 'action:new-document',
				group: 'Actions',
				label: 'New document',
				keywords: 'create write doc',
				href: `${root}/library?create=doc`
			},
			{
				id: 'action:new-sheet',
				group: 'Actions',
				label: 'New sheet',
				keywords: 'create spreadsheet table',
				href: `${root}/library?create=sheet`
			},
			{
				id: 'action:ask-exec',
				group: 'Actions',
				label: 'Ask the Exec…',
				keywords: 'talk message chat',
				run: () => {
					chooseTopic(null);
					execRailOpen = true;
				}
			},
			{
				id: 'action:add-app',
				group: 'Actions',
				label: 'Add an app',
				keywords: 'connect connector integration plugin skill mcp tool',
				href: `${root}/apps`
			},
			...CATALOGUE.map((entry) => ({
				id: `app:${entry.key}`,
				group: 'Apps',
				label: `Add ${entry.name}`,
				hint: entry.category,
				keywords: `connect ${entry.key} ${entry.description}`,
				href: `${root}/apps/${encodeURIComponent(entry.key)}`
			})),
			...leads.map((lead) => ({
				id: `action:ask:${lead.actor_id}`,
				group: 'Actions',
				label: `Ask ${lead.display}…`,
				keywords: `talk message lead ${lead.role}`,
				run: () => {
					chooseTopic({ actorId: lead.actor_id });
					execRailOpen = true;
				}
			})),
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
					hint:
						person.kind === 'exec'
							? 'Executive'
							: person.role.charAt(0).toUpperCase() + person.role.slice(1),
					keywords: 'message talk chat',
					href: `${root}/people?person=${encodeURIComponent(person.actor_id)}`
				})),
			...COMPANY_PAGES.map((companyPage) => ({
				id: `company:${companyPage.key}`,
				group: 'Company',
				label: companyPage.label,
				hint: companyPage.group,
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

{#snippet executiveRail(wide = false)}
	{#if ExecutiveRail}
		<ExecutiveRail
			{wide}
			focusMessage={wide ? Number(page.url.searchParams.get('focus')) || 0 : 0}
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
			needsProvider={!!providerIssue ||
				railRouteState === 'needs_connection' ||
				railRouteState === 'unavailable'}
			providerLabel={providerIssue ? startLinkLabel(providerIssue) : 'Connect intelligence'}
			contextLabel={currentContext.label}
			contextKind={currentContext.kind}
			focusAfterMessageId={railConversation.focusAfterMessageId}
			focusStartedAt={railConversation.focusStartedAt}
			newFocusAvailable={railActorId === 'exec' &&
				!focusedAttention &&
				!railWorkId &&
				railConnected}
			topicLabel={railTopicLabel}
			topics={focusedAttention || wide ? [] : railTopics}
			currentTopicKey={railTopicKey}
			ontopic={chooseTopic}
			viewerActorId={principal?.actor_id ?? 'owner'}
			onclose={wide ? null : () => (execRailOpen = false)}
			focusRequest={railFocusRequest}
			draftRequest={railDraft}
			references={ownerAccess
				? referenceOptions(
						companyId,
						workRows,
						cockpit?.goals ?? [],
						cockpit?.people ?? [],
						libraryDocuments.documents
					)
				: []}
			open={wide || execRailOpen}
			onask={askRail}
			review={focusedReview && !wide
				? {
						onback: closeFocusedContext,
						ondecide: decideFocusedReview
					}
				: null}
			workContext={focusedAttention && !wide ? { onback: closeFocusedContext } : null}
		/>{/if}
{/snippet}

<div
	class="company-browser-link-capture"
	use:companyBrowserLinks={{ open: openInCompanyBrowser }}
	use:prefetchOnIntent={{ client: queryClient, company: () => companyId }}
>
	{#if welcoming && childAllowed}
		{@render children()}
	{:else}
		<AppShell
			{companyId}
			companyName={companyName ||
				rememberedName(companyId) ||
				companyId.charAt(0).toUpperCase() + companyId.slice(1)}
			{companies}
			{tabs}
			{commands}
			{viewerName}
			{viewerRole}
			present={others.present}
			homeHref={ownerAccess ? '/' : collaboratorHome(companyId)}
			canSwitchCompanies={ownerAccess}
			execHref={ownerAccess &&
			(page.url.pathname === `/${companyId}/people` ||
				page.url.pathname.startsWith(`/${companyId}/people/`))
				? `/${companyId}/people?person=exec`
				: null}
			execName={railActorName}
			execLive={railConnected}
			execUnavailable={!!providerIssue}
			railOpen={execRailOpen}
			expandExec={page.url.pathname === `/${companyId}` &&
				attention.status === 'live' &&
				liveNeedsYou.length === 0 &&
				railConnected &&
				!page.url.searchParams.has('computer') &&
				!focusedAttention}
			immersive={immersiveComputer}
			blocked={startupBlocking}
			onexectoggle={() => inPlace(() => (execRailOpen = !execRailOpen))}
			rail={railVisible ? executiveRail : null}
		>
			{#if childAllowed}
				{@render children()}
			{:else if principalProjection.failure}
				<section class="company-access-state cockpit-pane">
					<FailureNotice
						error={principalProjection.failure}
						subject="this company"
						variant="page"
						onretry={principalProjection.refresh}
					/>
				</section>
			{:else}
				<section class="company-access-state cockpit-pane" role="status" aria-live="polite">
					<span class="access-mark" aria-hidden="true"
						><MatrixGlyph rows={GLYPHS.r} size={11} /></span
					>
					<p>{principal ? 'Opening your company workspace…' : 'Verifying company access…'}</p>
				</section>
			{/if}
		</AppShell>
		{#if touring && childAllowed}<WelcomeTour {companyId} />{/if}
	{/if}
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
						? describeFailure(startupFailure).detail
						: 'This is taking longer than expected. Your page is still here; try reconnecting.'}
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
		border: 1px solid var(--border-strong);
		border-radius: 6px;
		background: var(--surface-pane);
		box-shadow: var(--shadow-lift);
		color: var(--ink);
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
		background: var(--state-danger-soft);
		color: var(--state-danger);
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
		color: var(--text-tertiary);
		line-height: 1.55;
	}
	.startup-error .btn {
		min-width: 140px;
		margin-top: 4px;
		padding: 9px 18px;
		border: 1px solid var(--control-edge);
		border-radius: 4px;
		background: var(--intent-conversation-soft);
		color: var(--intent-conversation);
		font: 600 var(--t-body) var(--font-ui);
		cursor: pointer;
	}
	.startup-error .btn:disabled {
		opacity: 0.55;
		cursor: wait;
	}
	.startup-error .btn:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}
	.startup-error-note {
		color: var(--text-tertiary);
		font-size: var(--t-body);
	}

	.company-access-state p {
		margin: 0;
		color: var(--text-tertiary);
		/* A quick check shows only the calm mark; words appear if it lingers. */
		animation: access-words-in var(--motion-disclosure) var(--ease-out) 600ms backwards;
	}
	.access-mark {
		justify-self: center;
		margin-bottom: var(--space-2);
		color: var(--intent-direction);
		animation: bridge-skeleton-breathe var(--motion-working) ease-in-out infinite;
	}
	@keyframes access-words-in {
		from {
			opacity: 0;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.company-access-state p,
		.access-mark {
			animation: none;
		}
	}
</style>
