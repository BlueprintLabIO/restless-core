<script lang="ts">
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
	import ExecutiveRail from '$lib/components/ExecutiveRail.svelte';
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
	const railActorId = $derived(
		focusedAttention?.responsibleActor?.id ?? chosenTopic?.actorId ?? 'exec'
	);
	const railWorkId = $derived(focusedAttention?.workId ?? chosenTopic?.workId);
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
	/* People already shows the Exec conversation full width; a second copy in
	 * the rail would only repeat it, so the rail steps aside while it is open. */
	const execConversationSurface = $derived(
		page.url.pathname === `/${companyId}/people` && page.url.searchParams.get('person') === 'exec'
	);
	const railYields = $derived(companyComputerSurface || execConversationSurface);
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

	const currentContext = $derived.by(() => {
		const path = page.url.pathname;
		const root = `/${companyId}`;
		if (path === `${root}/work` || path.startsWith(`${root}/work/`)) return 'Linked · Work';
		if (path === `${root}/library` || path.startsWith(`${root}/library/`))
			return 'Linked · Library';
		if (path === `${root}/people` || path.startsWith(`${root}/people/`)) return 'Linked · People';
		if (path === `${root}/company` || path.startsWith(`${root}/company/`))
			return 'Linked · Company';
		return 'Linked · Inbox';
	});

	const tabs = $derived.by((): ShellTab[] => {
		const routes = companyShellTabs(
			companyId,
			page.url.pathname,
			principal,
			attention.status === 'unknown' ? undefined : liveNeedsYou.length
		);
		return routes.map((tab) =>
			tab.key === 'company' && providerIssue ? { ...tab, badge: 1 } : tab
		);
	});

	/* Owner-only destinations for the command menu. Collaborators keep the
	 * surface moves the shell adds for everyone. */
	/* Others in this company's cockpit, and the signed-in person's circle. */
	const others = presence(() => companyId);
	let railFocusRequest = $state(0);
	$effect(() => {
		const ask = () => {
			chooseTopic(null);
			execRailOpen = true;
			railFocusRequest += 1;
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
		needsProvider={!!providerIssue ||
			railRouteState === 'needs_connection' ||
			railRouteState === 'unavailable'}
		providerLabel={providerIssue ? startLinkLabel(providerIssue) : 'Connect intelligence'}
		contextLabel={currentContext}
		focusAfterMessageId={railConversation.focusAfterMessageId}
		focusStartedAt={railConversation.focusStartedAt}
		newFocusAvailable={railActorId === 'exec' && !focusedAttention && !railWorkId && railConnected}
		topicLabel={railTopicLabel}
		topics={focusedAttention ? [] : railTopics}
		currentTopicKey={chosenTopic ? `${chosenTopic.actorId}:${chosenTopic.workId ?? ''}` : 'general'}
		ontopic={chooseTopic}
		viewerActorId={principal?.actor_id ?? 'owner'}
		onclose={() => (execRailOpen = false)}
		focusRequest={railFocusRequest}
		references={ownerAccess
			? referenceOptions(companyId, workRows, cockpit?.goals ?? [], cockpit?.people ?? [])
			: []}
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

<div
	class="company-browser-link-capture"
	use:companyBrowserLinks={{ open: openInCompanyBrowser }}
	use:prefetchOnIntent={{ client: queryClient, company: () => companyId }}
>
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
				<span class="access-mark" aria-hidden="true"><MatrixGlyph rows={GLYPHS.r} size={11} /></span
				>
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
