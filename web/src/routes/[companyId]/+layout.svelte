<script lang="ts">
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import AppShell, { type ShellTab } from '$lib/components/AppShell.svelte';
	import ExecutiveRail from '$lib/components/ExecutiveRail.svelte';
	import { cockpitContextPath, reviewAction } from '$lib/model/attention';
	import {
		attentionQuery,
		cockpitQuery,
		companiesQuery,
		companyQuery,
		conversationQuery
	} from '$lib/model/queries.svelte';
	import { actorCanReceive } from '$lib/model/cockpit';

	let { children } = $props();

	const companyId = $derived(page.params.companyId ?? 'aris');
	const companyCatalog = companiesQuery();
	const companies = $derived(companyCatalog.view);
	let execRailOpen = $state(true);
	let focusRailRestore = $state<boolean | null>(null);
	let newFocusRequest = $state(0);

	/* The shell and the Attention surface read one source rather than polling the
	 * same endpoint on two clocks. The badge can no longer disagree with the
	 * queue it is counting. */
	const attention = $derived(attentionQuery(companyId));
	const cockpitProjection = $derived(cockpitQuery(companyId));
	const cockpit = $derived(cockpitProjection.view);
	const companyProjection = $derived(companyQuery(companyId));
	const companyDefaultStandard = $derived(
		companyProjection.view?.company.outcome_standard ??
			cockpit?.company.outcome_standard ??
			'exceptional'
	);
	$effect(() => companyProjection.attach());

	const companyName = $derived(attention.view?.company.name ?? '');
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
		conversationQuery(companyId, railActorId, focusedAttention?.workId, focusedAttention?.id)
	);
	const railActorName = $derived(
		railActorId === 'exec'
			? 'Exec'
			: (focusedAttention?.responsibleActor?.display ??
					railConversation.actor?.display ??
					railActorId)
	);
	const railActorRole = $derived(focusedAttention ? 'Responsible lead' : 'Executive');
	const railConnected = $derived(actorCanReceive(cockpit, railActorId));
	const companyComputerSurface = $derived(page.url.pathname === `/${companyId}/company/computer`);
	const immersiveComputer = $derived(
		(companyComputerSurface && page.url.searchParams.get('focus') === 'desktop') ||
			(page.url.pathname === `/${companyId}` && page.url.searchParams.has('computer'))
	);
	$effect(() => railConversation.attach());
	$effect(() => {
		if (focusedAttention) execRailOpen = true;
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
		outcomeStandard?: import('$lib/model/company').OutcomeStandard
	): Promise<{ error?: string; notice?: string }> {
		try {
			const contextPath = includeContext ? cockpitContextPath(companyId, page.url) : undefined;
			const result = await railConversation.send(
				text,
				files,
				contextPath,
				newFocus,
				interrupt,
				outcomeStandard
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
			return {
				error: cause instanceof Error ? cause.message : 'Your message was not delivered.'
			};
		}
	}

	function beginNewFocus() {
		if (railActorId !== 'exec' || !railConnected || railConversation.activeTurn) return;
		execRailOpen = true;
		newFocusRequest += 1;
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

	/* People holds its own conversation with the selected person, so a permanent
	 * rail there would render a second conversation with a different actor beside
	 * it — and duplicate itself outright when the Exec is the selection (S06-T2). */
	const railVisible = $derived.by(() => {
		const path = page.url.pathname;
		const people = `/${companyId}/people`;
		return !(
			path === people ||
			path.startsWith(`${people}/`) ||
			(path === `/${companyId}` && page.url.searchParams.has('computer'))
		);
	});

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
		const path = page.url.pathname;
		const root = `/${companyId}`;
		return [
			{
				key: 'attention',
				label: 'Attention',
				href: root,
				on: path === root,
				/* No badge until the source has answered once: a badge that is
				 * absent because nothing is waiting must not be confusable with a
				 * badge that is absent because nobody has asked yet. */
				badge: attention.status === 'unknown' ? undefined : liveNeedsYou.length || undefined
			},
			{
				key: 'work',
				label: 'Work',
				href: `${root}/work`,
				on: path === `${root}/work` || path.startsWith(`${root}/work/`)
			},
			{
				key: 'people',
				label: 'People',
				href: `${root}/people`,
				on: path === `${root}/people` || path.startsWith(`${root}/people/`)
			},
			{
				key: 'company',
				label: 'Company',
				href: `${root}/company`,
				on: path === `${root}/company` || path.startsWith(`${root}/company/`)
			}
		];
	});
</script>

{#snippet executiveRail()}
	<ExecutiveRail
		messages={railConversation.messages}
		participantName={railActorName}
		participantRole={railActorRole}
		turn={railConversation.activeTurn}
		{companyId}
		membershipRole="owner"
		connected={railConnected}
		defaultOutcomeStandard={companyDefaultStandard}
		contextLabel={currentContext}
		focusAfterMessageId={railConversation.focusAfterMessageId}
		focusStartedAt={railConversation.focusStartedAt}
		{newFocusRequest}
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

<AppShell
	{companyId}
	companyName={companyName || companyId.charAt(0).toUpperCase() + companyId.slice(1)}
	{companies}
	{tabs}
	execName={railActorName}
	execLive={railConnected}
	railOpen={execRailOpen}
	newFocusAvailable={railVisible && railActorId === 'exec' && !focusedAttention}
	newFocusDisabled={!railConnected || !!railConversation.activeTurn}
	immersive={immersiveComputer}
	onexectoggle={() => (execRailOpen = !execRailOpen)}
	onnewfocus={beginNewFocus}
	rail={railVisible ? executiveRail : null}
>
	{@render children()}
</AppShell>
