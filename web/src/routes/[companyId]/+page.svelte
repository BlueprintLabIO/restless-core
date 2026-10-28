<script lang="ts">
	import { askExec } from '$lib/model/ask-exec';
	import { failureSentence } from '$lib/model/failure';
	import { readDraft, writeDraft } from '$lib/model/composer-options.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { listFlip, listIn, listOut } from '$lib/ui/motion';
	import { resizePane } from '$lib/actions/resize-pane';
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { goto, beforeNavigate } from '$app/navigation';
	import InboxPreview from '$lib/components/InboxPreview.svelte';
	import OutcomePreview from '$lib/components/OutcomePreview.svelte';
	import AttentionFooter from '$lib/components/AttentionFooter.svelte';
	import { inboxPreviewTarget, inboxPreviewHref, inboxPreviewBack } from '$lib/model/inbox-preview';
	import AttentionDocument from '$lib/components/AttentionDocument.svelte';
	import DesktopViewport from '$lib/components/DesktopViewport.svelte';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
	import Composer, { type StopOutcome } from '$lib/primitives/Composer.svelte';
	import ConversationMessage from '$lib/primitives/ConversationMessage.svelte';
	import InboxList from '$lib/components/InboxList.svelte';
	import { useForYou } from '$lib/model/for-you.svelte';
	import InboxDetail from '$lib/components/InboxDetail.svelte';
	import { celebrate } from '$lib/ui/celebrate';
	import DirectionRequestDetail from '$lib/components/DirectionRequestDetail.svelte';
	import CharterPurpose from '$lib/components/CharterPurpose.svelte';
	import {
		attentionTitle,
		coordinationFailure,
		isOutcomeReview
	} from '$lib/model/attention-presentation';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/ui/glyph/MatrixGlyph.svelte';
	import ConversationTurnDock from '$lib/primitives/ConversationTurnDock.svelte';
	import CompanyOffice, { preloadOffice } from '$lib/office/LazyCompanyOffice.svelte';
	import type { AttentionItem } from '$lib/model/view';
	import {
		attentionQuery,
		collaborationBootstrapQuery,
		companiesQuery,
		companyPrincipalQuery,
		companyQuery,
		conversationQuery,
		identityQuery,
		turnWorking
	} from '$lib/model/queries.svelte';
	import { hasOwnerSurfaceAccess } from '$lib/model/company-access';
	import { startFixHref, startLinkLabel } from '$lib/model/company-start';
	import { getOwnershipHandover, handoverNotice, type OwnershipHandover } from '$lib/model/members';
	import { browserTabClientId } from '$lib/model/browserTab';
	import { fetchDecisionHistory, issueDesktopTicket, stopNotice } from '$lib/model/attention';
	import { DesktopControl } from '$lib/model/desktop-control.svelte';
	import { dropComputerFile } from '$lib/model/computer-files';
	import type { LibraryArtifact } from '$lib/model/library-files';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { markDoneSoon } from '$lib/components/inbox-actions';

	const companyId = $derived(page.params.companyId ?? '');

	/* The shared source, not a second poller. `loaded` is the distinction this
	 * surface was missing: before it existed, `items` was `[]` both when the
	 * queue was genuinely clear and when nobody had asked yet, so first paint
	 * asserted "Nothing needs your judgement" and took it back a moment later. */
	/* Everyone's Inbox is this page. The owner's /attention is the company queue; anyone else's is
	 * the same projection of what is assigned to them, and their floor is the Work they may see. */
	const principal = $derived(companyPrincipalQuery(companyId).view);
	const owner = $derived(hasOwnerSurfaceAccess(principal));
	const source = attentionQuery(
		() => companyId,
		() => !!principal,
		() => owner
	);
	const collaboration = $derived(
		collaborationBootstrapQuery(companyId, () => (owner ? null : principal))
	);
	const view = $derived(source.view);
	const loaded = $derived(!!view && source.status !== 'unknown');

	let error = $state('');
	let focusItem = $state<AttentionItem | null>(null);
	let desktopUrl = $state('');
	let autoClaimPending = $state(false);
	let conversationError = $state('');
	let messageDraft = $state('');
	/* A direction typed to the person on a handover survives a reload, per company and item. */
	const handoverDraftKey = (itemId: string) => `restless:${companyId}:handover-draft:${itemId}`;
	$effect(() => {
		const id = focusItem?.id;
		const body = messageDraft;
		if (!id) return;
		const timer = setTimeout(() => writeDraft(handoverDraftKey(id), body), 150);
		return () => clearTimeout(timer);
	});
	let messageFiles = $state<File[]>([]);
	let sendingMessage = $state(false);
	let interruptMessage = $state(false);
	let clientId = $state('');
	const control = new DesktopControl(
		() => companyId,
		() => clientId
	);
	let focusAttachKey = $state('');

	const items = $derived(view?.items ?? []);
	let visibleItems = $state<AttentionItem[] | null>(null);
	const graph = $derived(
		owner
			? (view?.workGraph ?? null)
			: collaboration.view
				? ({
						gates: [],
						gate_runs: [],
						handoffs: [],
						attempt_inputs: [],
						attempt_feedback: [],
						...collaboration.view.work_graph
					} as unknown as NonNullable<typeof view>['workGraph'])
				: null
	);
	/* The Work graph's file references, so files named by an item open in the Library viewer. */
	const artifacts = $derived((graph?.artifacts ?? []) as unknown as LibraryArtifact[]);
	const selectedUpdateId = $derived(page.url.searchParams.get('update'));
	const inboxFilter = $derived(page.url.searchParams.get('filter') ?? 'open');
	const selectedItemId = $derived(
		page.url.searchParams.get('item') ?? page.url.searchParams.get('review')
	);
	/* Brand not set yet: the clear Inbox offers to set it up with Exec. */
	const identity = $derived(identityQuery(companyId));
	const brandMissing = $derived(
		hasOwnerSurfaceAccess(companyPrincipalQuery(companyId).view) &&
			!!identity.view &&
			!identity.view.current_release &&
			!identity.view.pending_proposals.length
	);
	const setUpBrand = () =>
		askExec(
			"Let's set up our brand and voice. Ask me what you need: which brands feel like us, how we should sound, and anything we must never say."
		);
	const companyCatalog = companiesQuery(() =>
		hasOwnerSurfaceAccess(companyPrincipalQuery(companyId).view)
	);
	/* A company that cannot start is not clear, whatever the queue says: say
	 * the blocker in the portfolio's words and link straight to its fix. */
	const startBlocker = $derived.by(() => {
		const reason = companyCatalog.view.find(
			(company) => company.id === companyId
		)?.unstartable_reason;
		return reason ? startLinkLabel(reason) : '';
	});
	/* Coordination down is not a clear queue: what it holds may be missing. Nothing needs the
	 * owner when Attention is empty; the list still stays open while For you has anything in it,
	 * read or not, so reading an item changes its dot and nothing else moves. */
	const coordination = $derived(coordinationFailure(view));
	const forYou = useForYou();
	/* The charter an Inbox draft opens: read only while such an item is on screen. */
	const charterCompany = $derived(companyQuery(companyId, () => !!selectedItem?.charterProposal));
	const attentionClear = $derived(loaded && items.length === 0 && !coordination);
	/* A pending ownership handover (ADR 0012 decision 7) is said to both people in it, here rather
	 * than as a refusal when someone tries to approve. Reread whenever Attention changes. */
	let handover = $state<OwnershipHandover | null>(null);
	$effect(() => {
		void view;
		const requested = companyId;
		getOwnershipHandover(requested)
			.then((next) => {
				if (requested === companyId) handover = next;
			})
			.catch(() => {});
	});
	const handoverStatus = $derived(handoverNotice(handover, principal?.actor_id));
	const queueClear = $derived(
		inboxFilter === 'open' && attentionClear && !forYou?.items.length && forYou?.filter === 'all' && !handoverStatus
	);
	/* Preload office assets on desktop; a clear phone Inbox can still open the floor. */
	let compactScreen = $state(false);
	$effect(() => {
		const query = window.matchMedia('(max-width: 760px)');
		compactScreen = query.matches;
		const change = () => (compactScreen = query.matches);
		query.addEventListener('change', change);
		return () => query.removeEventListener('change', change);
	});
	/* The office is the likely answer on a desktop; fetch its code and art
	 * alongside Attention rather than after it. */
	$effect(() => {
		if (!compactScreen && !(loaded && items.length > 0)) void preloadOffice();
	});
	/* The queue opening or clearing while the owner watches is animated; the
	 * page arriving in its first known layout is not. Two frames after the
	 * first answer, later changes may move. */
	let settled = $state(false);
	$effect(() => {
		if (!loaded || settled) return;
		const frame = requestAnimationFrame(() =>
			requestAnimationFrame(() => {
				settled = true;
			})
		);
		return () => cancelAnimationFrame(frame);
	});
	/* The company catalogue is the owner's; nobody else waits on it. */
	const catalogPending = $derived(owner && companyCatalog.status === 'unknown');
	/* The pill speaks only when the list is folded away; with For you open beside it, the list
	 * already says there are no decisions (S85-22). */
	const showClear = $derived(queueClear && !catalogPending && !startBlocker);
	/* The queue starts collapsed until its source answers, so an idle
	 * company's office is never squeezed and released; when items do arrive,
	 * the queue opens with the same motion as any later change. */
	const selectedItem = $derived.by(() => {
		if (selectedUpdateId) return null;
		if (selectedItemId) return items.find((item) => item.id === selectedItemId) ?? null;
		if (['skipped', 'done'].includes(inboxFilter)) return null;
		return (visibleItems ?? items)[0] ?? null;
	});
	let missingNotice = $state('');
	let missingDecisionHref = $state('');
	let explaining = '';
	async function explainMissing(company: string, itemId: string) {
		explaining = itemId;
		const history = await fetchDecisionHistory(company).catch(() => null);
		const decision = history?.decisions.find((row) => row.itemId === itemId);
		if (!decision || explaining !== itemId || !missingNotice) return;
		const what = decision.recordedDecision.trim() || 'It was decided.';
		missingNotice = decision.decidedBy
			? `${decision.decidedBy} already decided “${decision.title}”: ${what}`
			: `“${decision.title}” was already decided: ${what}`;
		missingDecisionHref = `/${encodeURIComponent(company)}/company/activity?show=decisions&open=${encodeURIComponent(decision.id)}`;
	}
	let missingItemRedirect = $state('');
	let resolvedHere = '';
	let sent = $state<{
		text: string;
		href?: string;
		result?: { href: string; label: string };
		/** Where an answer given in a card landed (S101). */
		home?: { href: string; label: string };
	} | null>(null);
	$effect(() => {
		if (!sent) return;
		/* An accepted result stays until dismissed: it is the way to what was approved. */
		if (sent.result) return;
		const timer = setTimeout(() => (sent = null), 6000);
		return () => clearTimeout(timer);
	});
	let previousItemOrder: string[] = [];
	$effect(() => {
		if (
			loaded &&
			view?.sourceHealth.authority === 'available' &&
			view.sourceHealth.orgintel === 'available' &&
			selectedItemId &&
			missingItemRedirect !== selectedItemId &&
			!items.some((item) => item.id === selectedItemId)
		) {
			missingItemRedirect = selectedItemId;
			missingNotice =
				resolvedHere === selectedItemId
					? ''
					: 'That item has been resolved or is no longer in the Inbox.';
			missingDecisionHref = '';
			/* An old link to an item someone already answered says who decided and what. */
			if (resolvedHere !== selectedItemId) void explainMissing(companyId, selectedItemId);
			const previousIndex = Math.max(0, previousItemOrder.indexOf(selectedItemId));
			const next = (visibleItems ?? items)[
				Math.min(previousIndex, (visibleItems ?? items).length - 1)
			];
			void goto(next ? itemHref(next.id) : baseHref, {
				replaceState: true,
				noScroll: true
			});
		}
		if (!selectedItemId || items.some((item) => item.id === selectedItemId))
			missingItemRedirect = '';
		if (loaded) previousItemOrder = (visibleItems ?? items).map((item) => item.id);
	});
	const focusedComputerId = $derived(page.url.searchParams.get('computer'));
	const baseHref = $derived(`/${companyId}`);
	const requestingActor = $derived(
		focusItem?.responsibleActor?.id ?? focusItem?.runtimeAttach?.requestingActor ?? ''
	);
	const leadConversation = $derived(
		requestingActor && focusItem
			? conversationQuery(companyId, requestingActor, focusItem.workId, focusItem.id)
			: null
	);
	$effect(() => leadConversation?.attach());
	const requestingActorName = $derived(
		leadConversation?.actor?.display ||
			focusItem?.responsibleActor?.display ||
			focusItem?.runtimeAttach?.requestingActorDisplay ||
			(requestingActor === 'exec' ? 'Exec' : requestingActor || 'Company context')
	);
	const visibleConversationMessages = $derived(leadConversation?.messages ?? []);
	const conversationTurn = $derived(leadConversation?.activeTurn ?? null);
	/* A finished reply stays on the dock; only a turn still running is interrupted. */
	const conversationWorking = $derived(turnWorking(conversationTurn));
	const attachmentHref = (attachment: { uploadId: string }) =>
		`/api/companies/${encodeURIComponent(companyId)}/attachments/${encodeURIComponent(attachment.uploadId)}`;

	$effect(() => {
		/* A discussion (?conversation) keeps the Inbox detail; the rail carries that conversation. */
		const focusId = focusedComputerId;
		if (!focusId) {
			focusAttachKey = '';
			focusItem = null;
			desktopUrl = '';
			control.lose();
			/* A handover failure belongs to the computer view, not to the Inbox left behind. */
			error = '';
			return;
		}
		const item = items.find((candidate) => candidate.id === focusId) ?? null;
		if (!item) {
			if (loaded) {
				focusAttachKey = '';
				focusItem = null;
				desktopUrl = '';
				control.lose();
			}
			return;
		}
		if (!item.runtimeAttach) {
			// The source can withdraw a prepared step while this URL is open.
			// Return to its current card rather than showing an unrelated desktop.
			void closePreparedComputer(item);
			return;
		}
		if (!clientId) return;
		const key = `computer:${item.id}:${item.runtimeAttach.generation}`;
		if (focusAttachKey === key) return;
		focusAttachKey = key;
		void attachPreparedComputer(item);
	});

	onMount(() => {
		void browserTabClientId(companyId).then((id) => (clientId = id));
		return control.watch();
	});

	/* Actions still refresh immediately after a write — the poll is a floor, not
	 * the only path. The source coalesces this with any poll already running. */
	async function refresh() {
		await source.refresh();
	}

	function itemHref(id: string): string {
		return `${baseHref}?item=${encodeURIComponent(id)}`;
	}

	const requestedPreview = $derived(page.url.searchParams.get('preview'));
	let hiddenPreview = $state<string | null>(null);
	const previewTarget = $derived(
		requestedPreview && requestedPreview !== hiddenPreview
			? inboxPreviewTarget(companyId, requestedPreview, page.url.origin)
			: null
	);
	$effect(() => {
		if (!requestedPreview) hiddenPreview = null;
	});
	/* The empty queue's marks belong to the empty pane: over an open item or preview they covered
	 * its header and left a bar along its foot (20 October). */
	const paneEmpty = $derived(!selectedItem && !previewTarget);
	async function openPreview(href: string | URL, options?: Parameters<typeof goto>[1]) {
		const target = inboxPreviewTarget(companyId, href, page.url.origin);
		if (!target) return goto(href, options);
		const current = new URL(page.url);
		if (!selectedUpdateId && selectedItem) current.searchParams.set('item', selectedItem.id);
		return goto(inboxPreviewHref(current, target.href), { noScroll: true, ...options });
	}
	/* Only a link followed inside the Inbox's own pane opens in that pane. "Open in the Library"
	 * from Exec's rail or a file dialog means the Library, and used to stay in the Inbox, framed
	 * into its preview (21 October). */
	let paneClickAt = 0;
	beforeNavigate((navigation) => {
		if (!navigation.to || navigation.from?.url.pathname !== baseHref) return;
		if (navigation.type === 'link' && performance.now() - paneClickAt > 1000) return;
		if (!inboxPreviewTarget(companyId, navigation.to.url, page.url.origin)) return;
		navigation.cancel();
		void openPreview(navigation.to.url);
	});
	function restoreItem() {
		if (!previewTarget) return;
		hiddenPreview = requestedPreview;
		void goto(inboxPreviewBack(page.url), { replaceState: true, noScroll: true, keepFocus: true });
	}
	function revealItemEditor(node: HTMLElement) {
		const click = (event: MouseEvent) => {
			if (
				selectedItem &&
				event.target instanceof Element &&
				event.target.closest('[data-inbox-actions] [data-inbox-reveal]')
			)
				restoreItem();
		};
		node.addEventListener('click', click, true);
		return { destroy: () => node.removeEventListener('click', click, true) };
	}
	async function archiveUpdate() {
		if (!selectedUpdateId || !forYou) return;
		try {
			await forYou.archive([selectedUpdateId]);
			await goto(baseHref);
		} catch (cause) {
			error = failureSentence(cause, 'The update was not archived.');
		}
	}

	/* The prepared step is the owner's to do: once the desktop connects, this tab takes control
	 * so the first keystroke lands. The connection never changes afterwards; control only flips who
	 * may type, so returning control keeps the agent's resumption in view. */
	async function desktopConnected() {
		if (!focusItem || !desktopUrl || !clientId || !autoClaimPending) return;
		autoClaimPending = false;
		await control.take(true);
	}

	async function attachPreparedComputer(item: AttentionItem) {
		error = '';
		focusItem = item;
		control.lose();
		conversationError = '';
		messageDraft = readDraft(handoverDraftKey(item.id));
		messageFiles = [];
		desktopUrl = '';
		if (!item.runtimeAttach) {
			error = 'This handoff no longer has a live Company computer attachment.';
			return;
		}
		try {
			const url = await issueDesktopTicket(companyId, item.id, clientId);
			/* The owner may have left the computer view while the ticket was in flight. */
			if (focusItem?.id !== item.id) return;
			desktopUrl = url;
			autoClaimPending = true;
		} catch (cause) {
			if (focusItem?.id === item.id)
				error = failureSentence(cause, 'The live browser is unavailable.');
		}
	}

	/* What the owner needs to know about the computer before acting: nothing when it is ready. */
	const browserReadiness = $derived.by(() => {
		const health = view?.sourceHealth.browser;
		if (health === 'available') return '';
		if (health === 'unavailable') return 'Computer not ready';
		return 'Checking the computer';
	});

	/* Finish the human step from the computer itself: the sign-in is done here, so Mark done is
	 * here too, with the same Undo window as everywhere else. */
	const queryClient = useQueryClient();
	const focusStep = $derived(
		focusItem?.actions.find((action) => action.id === 'complete-human-step') ?? null
	);
	let finishingStep = $state(false);
	async function finishStep() {
		const item = focusItem;
		if (!item || finishingStep) return;
		finishingStep = true;
		try {
			if (await markDoneSoon(queryClient, companyId, item.source.reference)) {
				resolvedHere = item.id;
				sent = { text: 'Marked done. The work resumes.' };
				await closePreparedComputer(null);
			}
		} finally {
			finishingStep = false;
		}
	}

	async function closePreparedComputer(item: AttentionItem | null = focusItem) {
		if (control.owner) await control.release();
		focusItem = null;
		desktopUrl = '';
		focusAttachKey = '';
		error = '';
		await goto(item ? itemHref(item.id) : baseHref, { replaceState: true });
	}

	async function stopConversation(): Promise<StopOutcome> {
		try {
			return { notice: stopNotice(requestingActorName, await leadConversation?.stopTurn()) };
		} catch (cause) {
			return {
				error: failureSentence(cause, 'The reply could not be stopped; it may already have ended.')
			};
		}
	}

	async function sendMessage(event: SubmitEvent) {
		event.preventDefault();
		const files = messageFiles;
		const body =
			messageDraft.trim() ||
			(files.length ? `Attached: ${files.map((file) => file.name).join(', ')}` : '');
		if (!body || sendingMessage || !leadConversation) return;
		sendingMessage = true;
		conversationError = '';
		const sent = messageDraft;
		messageDraft = '';
		try {
			await leadConversation.send(
				body,
				files,
				undefined,
				false,
				conversationWorking && interruptMessage
			);
			messageFiles = [];
		} catch (cause) {
			messageDraft = sent;
			conversationError = failureSentence(cause, 'Your message was not delivered.');
		} finally {
			sendingMessage = false;
		}
	}
	holdQueries(() => [leadConversation, identity, collaboration]);
</script>

<svelte:head
	><title>{view?.company.name ? `Inbox — ${view.company.name}` : 'Inbox'}</title></svelte:head
>
<h1 class="sr-title">Inbox</h1>

{#if focusItem && !previewTarget}
	<div class="browser-focus">
		<header class="handover-rail">
			<div class="handover-context">
				<span class="live-mark" class:owner={control.owner}></span>
				<div>
					<strong>{focusItem.title}</strong>
					<span class="handover-request" title="Exact action requested by the responsible actor"
						>{focusItem.requestedAction}</span
					>
				</div>
			</div>
			<div class="handover-state">
				{#if focusItem.runtimeAttach && desktopUrl}
					{#if browserReadiness}
						<span
							class="handover-readiness"
							title={`The company computer reports “${view?.sourceHealth.browser ?? 'unknown'}”. The view reconnects when it is ready.`}
							>{browserReadiness}</span
						>
					{/if}
					{#if control.idleSeconds}
						<span class="handover-idle" role="status">Handing back in {control.idleSeconds}s</span>
						<button
							class="btn small primary"
							type="button"
							title="Keeps control with you. Quiet control is handed back after a minute without telling the agent to resume."
							onclick={() => control.stillHere()}>Still here</button
						>
					{/if}
					<span
						title="Control changes keyboard and pointer ownership only; it does not complete this handoff."
						>{control.owner ? 'You control' : 'Observe only'}</span
					>
					{#if !control.owner}
						<button
							class="btn small primary"
							type="button"
							disabled={!!control.working}
							title="Pauses company automation and gives this tab sole keyboard and pointer control."
							onclick={() => control.take()}
							>{control.working === 'control' ? 'Taking control…' : 'Take control'}</button
						>
					{:else}
						<button
							class="btn small"
							type="button"
							disabled={!!control.working}
							title="Returns input to the responsible actor. The source condition is checked separately."
							onclick={() => control.release()}
							>{control.working === 'return' ? 'Returning…' : 'Return control'}</button
						>
					{/if}
				{:else}
					<span class="mono">Preview offline · Discussion available</span>
				{/if}
				{#if focusStep && !focusItem.preparing}
					<button
						class="btn small"
						class:primary={control.owner || !desktopUrl}
						type="button"
						disabled={finishingStep}
						title={`${focusItem.ifNoAction ? `${focusItem.ifNoAction}. ` : ''}${focusStep.consequence} Undo stays available for a few seconds.`}
						onclick={() => void finishStep()}
						>{finishingStep ? 'Marking done…' : 'Mark done'}</button
					>
				{/if}
				<button
					class="btn small"
					type="button"
					title="Leaves this work-through view without resolving the handoff."
					onclick={() => closePreparedComputer()}>Leave computer</button
				>
			</div>
		</header>
		{#if error || control.error}<div class="focus-error" role="alert">
				{error || control.error}
			</div>{/if}
		<div
			class="browser-workspace"
			use:resizePane={{
				key: `${companyId}:handover`,
				label: 'Resize handover conversation',
				target: '.handover-conversation',
				variable: '--handover-w',
				side: 'end',
				min: 240,
				minOther: 320,
				defaultSize: 300
			}}
		>
			<aside
				class="handover-conversation"
				aria-label={requestingActor
					? 'Conversation with the responsible team lead'
					: 'Prepared owner context'}
			>
				<div class="handover-conversation-head">
					<div>
						<strong
							title={`Responsible lead · ${leadConversation?.actor?.role ?? focusItem.responsibleActor?.role ?? 'Work owner'}`}
							>{requestingActorName}</strong
						>
					</div>
					<span class="controller-badge" class:owner={control.owner && !!desktopUrl}>
						{desktopUrl
							? control.owner
								? 'You control'
								: `${requestingActorName} observes`
							: 'Chat open'}
					</span>
				</div>

				<div class="handover-thread" aria-live="polite">
					{#each visibleConversationMessages as message (message.id)}
						<ConversationMessage
							sender={message.from === 'you' ? 'owner' : 'agent'}
							author={message.from === 'you' ? 'You' : message.author || requestingActorName}
							text={message.text}
							createdAt={message.createdAt}
							details={message.details}
							intent={message.intent}
							accountUses={message.accountUses}
							attachments={message.attachments}
							hrefFor={attachmentHref}
							messageId={message.id}
							{companyId}
						/>
					{/each}
				</div>

				{#if requestingActor}
					<div class="handover-footer" class:with-turn={!!conversationTurn}>
						{#if conversationTurn}
							<ConversationTurnDock participantName={requestingActorName} turn={conversationTurn} />
						{/if}
						<form class="handover-composer" onsubmit={sendMessage}>
							<Composer
								bind:value={messageDraft}
								bind:files={messageFiles}
								interruptible={owner && conversationWorking}
								onstop={owner ? stopConversation : null}
								bind:interrupt={interruptMessage}
								disabled={sendingMessage}
								placeholder={conversationWorking
									? `Interrupt ${requestingActorName} with new direction…`
									: `Ask or tell ${requestingActorName} anything…`}
								ariaLabel={conversationWorking
									? `Interrupt and message ${requestingActorName}`
									: `Message ${requestingActorName}`}
								minlength={1}
							/>
						</form>
						{#if conversationError}<p class="conversation-error">{conversationError}</p>{/if}
					</div>
				{:else}
					<div class="handover-composer">
						<small
							>This older item has no recorded requesting actor. Restless will not guess who should
							receive a message.</small
						>
					</div>
				{/if}
			</aside>
			<div class="desktop-stage">
				{#snippet offlineDesktop()}
					<div class="outcome-offline">
						<h2 title="The review and the lead conversation are still available beside it.">
							The live outcome is unavailable.
						</h2>
						<Markdown text={focusItem?.whatHappened ?? ''} />
						{#each (focusItem?.evidence ?? []).filter((entry) => entry.uri) as evidence}
							<a class="evidence-link" href={evidence.uri} target="_blank" rel="noreferrer"
								>{evidence.label} ↗</a
							>
						{/each}
					</div>
				{/snippet}
				{#if focusItem.nativeDocument}
					<AttentionDocument
						{companyId}
						request={focusItem.nativeDocument}
						onresolved={() => void source.refresh()}
					/>
				{:else}
					<DesktopViewport
						src={desktopUrl}
						title="Live company browser"
						offline={offlineDesktop}
						interactive={control.owner}
						canResize={control.owner}
						onclaim={() => control.take()}
						ondropfile={(file) => dropComputerFile(companyId, file)}
						onload={() => void desktopConnected()}
						onactivity={() => control.activity()}
					/>
				{/if}
			</div>
		</div>
	</div>
{:else}
	<div
		class="cockpit-screen attention-screen"
		class:queue-clear={queueClear}
		class:settled
		class:mobile-detail={compactScreen && !!(selectedItemId || previewTarget)}
		class:awaiting={!loaded && !source.failure}
		aria-busy={!loaded && !source.failure}
		use:resizePane={{
			key: `${companyId}:inbox`,
			label: 'Resize attention panes',
			target: '.attention-index',
			variable: '--attention-index-w',
			min: 260,
			minOther: 320,
			defaultSize: 340,
			enabled: !queueClear
		}}
	>
		{#if sent}<div class="attention-resolution-notice" role="status">
				{sent.text}{#if sent.result}<a href={sent.result.href}>{sent.result.label}</a
					>{/if}{#if sent.home}<a
						href={sent.home.href}
						title={`Where this lives: ${sent.home.label}`}>{sent.home.label}</a
					>{/if}{#if sent.href}<a href={sent.href}>Open conversation</a>{/if}<button
					aria-label="Dismiss notification"
					onclick={() => (sent = null)}>×</button
				>
			</div>{/if}
		{#if missingNotice}<div class="attention-resolution-notice" role="status">
				{missingNotice}{#if missingDecisionHref}
					<a href={missingDecisionHref}>See it in Activity</a>{/if}<button
					aria-label="Dismiss notification"
					onclick={() => (missingNotice = '')}>×</button
				>
			</div>{/if}
		{#if error}
			<div class="cockpit-error attention-error" role="alert">{error}</div>
		{:else if source.failure && loaded}
			<div class="cockpit-error">
				<FailureNotice error={source.failure} subject="the Inbox" stale onretry={source.reload} />
			</div>
		{/if}
		<aside class="cockpit-pane attention-index" aria-hidden={queueClear} inert={queueClear}>
			<InboxList
				onvisible={(rows) => (visibleItems = rows)}
				{companyId}
				{items}
				canSetAside={owner}
				selectedId={selectedItem?.id}
				blocker={startBlocker}
				notice={handoverStatus}
				{loaded}
				failure={source.failure}
				{coordination}
				onretry={source.reload}
			/>
		</aside>

		<section
			use:revealItemEditor
			onclickcapture={() => (paneClickAt = performance.now())}
			class="cockpit-pane attention-focus"
			class:office-focus={!selectedItem && !previewTarget && loaded}
		>
			{#if queueClear && startBlocker && !compactScreen}
				<a class="attention-start-blocker" href={startFixHref(companyId)}>
					<span class="attention-start-glyph" aria-hidden="true">
						<TriangleAlert size={16} />
					</span>
					<span class="attention-start-copy">
						<strong>Can’t start yet</strong>
						<span>{startBlocker}</span>
					</span>
					<span class="attention-start-go" aria-hidden="true">→</span>
				</a>
			{/if}
			<button
				class="attention-clear-control"
				class:visible={showClear && paneEmpty}
				type="button"
				aria-hidden={!(showClear && paneEmpty)}
				tabindex={showClear && paneEmpty ? 0 : -1}
				title={showClear && paneEmpty ? 'No decision is waiting on you. Check again now.' : undefined}
				onclick={() => void refresh()}
			>
				<span class="attention-clear-glyph" aria-hidden="true">
					<MatrixGlyph rows={GLYPHS.check} size={7} />
				</span>
				<span>No decisions</span>
			</button>
			{#if queueClear && paneEmpty}<div class="attention-set-aside">
					{#if owner}<a href={`/${companyId}?filter=all`} class="attention-archived">All Inbox items</a>{/if}
					{#if forYou}<button
							type="button"
							class="attention-archived"
							title="For you items you archived"
							onclick={() => forYou.setFilter('archived')}>Archived</button
						>{/if}
				</div>{/if}
			{#if compactScreen && (selectedItemId || previewTarget)}<a
					class="attention-mobile-back"
					href={baseHref}>← Inbox</a
				>{/if}
			{#if selectedItem}
				{#if selectedItem.nativeDocument}
					{#key `${companyId}:${selectedItem.id}`}
						<AttentionDocument
							{companyId}
							request={selectedItem.nativeDocument}
							preview={previewTarget ? previewContent : undefined}
							onresolved={() => void source.refresh()}
						/>
					{/key}
				{:else}
					{#key `${companyId}:${selectedItem.id}`}
						{@render attentionDetail(selectedItem)}
					{/key}
				{/if}
			{:else if previewTarget}
				<div class="update-preview">
					{@render previewContent()}
					<AttentionFooter
						><div class="action-row">
							{#if selectedUpdateId}<button
									class="btn small"
									type="button"
									onclick={() => void archiveUpdate()}>Archive</button
								>{/if}
							<a class="btn small" href={inboxPreviewBack(page.url)}>Back to Inbox</a>
						</div></AttentionFooter
					>
				</div>
			{:else if !loaded && source.failure}
				<FailureNotice
					error={source.failure}
					subject="the Inbox"
					variant="page"
					onretry={source.reload}
				/>
			{:else if !loaded}
				<!-- Deliberately nothing until the source answers. An empty pane for
				     one round trip reads as loading; the zero-state hero reads as a
				     verdict, and it was the wrong one about half a second later. -->
			{:else if coordination && compactScreen}
				<!-- No verdict while coordination is down: the list says why. -->
			{:else}
				<!-- The office fills the clear Inbox; Work holds outcomes and next steps. -->
				<div class="clear-surface">
					{#if brandMissing && !startBlocker}
						<!-- Until the brand and voice are set, every site and message the team makes is a guess. -->
						<div class="brand-nudge" role="status">
							<p>
								<strong>Set your brand and voice</strong>
								Exec asks three questions, so what the team makes sounds and looks like you.
							</p>
							<button class="btn small primary" type="button" onclick={setUpBrand}
								>Set up with Exec</button
							>
						</div>
					{/if}
					<div class="clear-office">
						<CompanyOffice
							{companyId}
							{graph}
							collaboration={owner ? null : collaboration.view}
							sourceHealth={view?.sourceHealth ?? {}}
							quietSignal={catalogPending || !!startBlocker}
						/>
					</div>
				</div>
			{/if}
		</section>
	</div>
{/if}

{#snippet previewContent()}
	{#if previewTarget}
		{#key companyId + ':' + previewTarget.kind + ':' + previewTarget.value}
			<InboxPreview
				{companyId}
				target={previewTarget}
				{artifacts}
				backHref={inboxPreviewBack(page.url)}
				onnavigate={openPreview}
			/>
		{/key}
	{/if}
{/snippet}

{#snippet attentionDetail(item: AttentionItem)}
	{#if item.charterProposal}
		<div class="attention-surface">
			<header class="attention-surface-head">
				<h2>{item.title}</h2>
				<a href={`/${companyId}/company/charter`} title="Open Company › Charter"
					>Company › Charter</a
				>
			</header>
			{#if charterCompany.view}
				<CharterPurpose
					{companyId}
					view={charterCompany.view}
					inbox
					preview={previewTarget ? previewContent : undefined}
					onsaved={(company) => {
						charterCompany.accept(company);
						resolvedHere = item.id;
						void source.refresh();
					}}
				/>
			{/if}
		</div>
	{:else if item.directionRequest}
		<DirectionRequestDetail
			{companyId}
			request={item.directionRequest}
			preview={previewTarget ? previewContent : undefined}
			onresolved={(result) => {
				resolvedHere = item.id;
				sent = result;
				void source.refresh();
			}}
		/>
	{:else}
		{#snippet outcomeContent(reviewedItem: AttentionItem)}{#key JSON.stringify(reviewedItem.reviewVersion)}<OutcomePreview
				{companyId}
				item={reviewedItem}
				{artifacts}
				onrefresh={source.refresh}
			/>{/key}{/snippet}
		<InboxDetail
			{companyId}
			{item}
			{artifacts}
			content={isOutcomeReview(item) || (item.category === 'review' && !!item.reviewTarget)
				? outcomeContent
				: undefined}
			preview={previewTarget ? previewContent : undefined}
			workOwnerId={graph?.work.find((work) => work.id === item.workId)?.owner_id}
			canSetAside={owner}
			canPassToOwner={!owner}
			onleave={() => {
				/* Snoozed or set aside here: on to the next item, as the list does. */
				resolvedHere = item.id;
				const rows = visibleItems ?? items;
				const others = rows.filter((row) => row.id !== item.id);
				const index = Math.max(
					0,
					rows.findIndex((row) => row.id === item.id)
				);
				const next = others[Math.min(index, others.length - 1)];
				void goto(next ? itemHref(next.id) : baseHref, { noScroll: true });
			}}
			declining={page.url.searchParams.get('decline') === '1'}
			onopenDocument={async () => {
				const result = await source.reload();
				if (result.error) throw result.error;
			}}
			onresolved={(result) => {
				/* From the answer just given, before the detail closes around it. */
				celebrate(document.activeElement);
				resolvedHere = item.id;
				sent = result;
			}}
		/>{/if}
{/snippet}

<style>
	.clear-surface {
		position: relative;
		width: 100%;
		height: 100%;
		min-height: 0;
		overflow: hidden;
	}
	.clear-office {
		height: 100%;
		overflow: hidden;
	}
	.brand-nudge {
		position: absolute;
		top: 60px;
		left: 16px;
		z-index: 2;
		display: flex;
		align-items: center;
		gap: 12px;
		max-width: min(520px, calc(100% - 24px));
		padding: 10px 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: 0 6px 20px rgb(0 0 0 / 0.08);
	}
	.brand-nudge p {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.brand-nudge strong {
		display: block;
		color: var(--ink);
		font-size: var(--t-body);
	}
	.attention-resolution-notice {
		position: absolute;
		bottom: 16px;
		left: 16px;
		z-index: 4;
		display: flex;
		gap: 16px;
		padding: 12px 16px;
		background: var(--surface-raised);
		border: 1px solid var(--border);
		border-radius: 6px;
		box-shadow: var(--shadow-soft);
		font-size: var(--t-body);
	}
	.attention-resolution-notice button {
		background: none;
		border: 0;
		color: var(--text-secondary);
		cursor: pointer;
	}
	.attention-mobile-back {
		display: block;
		padding: 14px 16px;
		border-bottom: 1px solid var(--border);
		color: var(--text-secondary);
		font-size: var(--t-body);
		text-decoration: none;
	}
	@media (max-width: 760px) {
		:global(.bridge-root .attention-screen:not(.queue-clear)) {
			display: flex;
			flex-direction: column;
		}
		:global(.bridge-root .attention-screen:not(.queue-clear) .attention-index) {
			width: 100%;
			max-height: none;
			height: 100%;
			flex: 1;
			border: 0;
			display: flex;
			flex-direction: column;
		}
		:global(.bridge-root .attention-screen:not(.mobile-detail):not(.queue-clear) .attention-focus) {
			display: none;
		}
		:global(.bridge-root .attention-screen.mobile-detail .attention-index) {
			display: none;
		}
		:global(.bridge-root .attention-screen.mobile-detail .attention-focus) {
			display: flex;
			flex-direction: column;
			overflow: hidden;
			flex: 1;
		}
		:global(.bridge-root .attention-screen.mobile-detail .attention-focus > article),
		:global(.bridge-root .attention-screen.mobile-detail .attention-focus > .attention-surface),
		:global(.bridge-root .attention-screen.mobile-detail .attention-focus > .attention-document) {
			flex: 1;
			min-height: 0;
			height: auto;
		}
	}
	.attention-error,
	.focus-error {
		color: var(--danger);
		font-size: var(--t-body);
	}
	.attention-error,
	.focus-error {
		padding: 9px 14px;
		border: 1px solid color-mix(in srgb, var(--danger) 45%, var(--border));
		background: color-mix(in srgb, var(--danger) 7%, var(--surface));
	}
	.browser-focus {
		flex: 1 1 auto;
		width: 100%;
		min-width: 0;
		height: 100%;
		min-height: 0;
		display: grid;
		grid-template-rows: auto auto minmax(0, 1fr);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		overflow: hidden;
		background: var(--glass-pane);
	}
	:global(.bridge-root.immersive) .browser-focus {
		border: 0;
		border-radius: 0;
	}
	.handover-rail {
		grid-row: 1;
		min-height: 54px;
		padding: 7px 10px;
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 14px;
		border-bottom: 1px solid var(--border-strong);
		background: var(--glass-strong);
	}
	.handover-context,
	.handover-state {
		display: flex;
		align-items: center;
		gap: 10px;
	}
	.handover-context > div {
		display: grid;
		gap: 1px;
		min-width: 0;
	}
	.handover-state > span {
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}
	.handover-state > .handover-idle {
		color: var(--ink);
		font-variant-numeric: tabular-nums;
	}
	.handover-context strong {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.handover-request {
		max-width: min(68vw, 760px);
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.3;
	}
	.live-mark {
		width: 9px;
		height: 28px;
		border: 1px solid var(--border-strong);
		background: repeating-linear-gradient(
			0deg,
			var(--surface),
			var(--surface) 3px,
			transparent 3px,
			transparent 6px
		);
	}
	.live-mark.owner {
		background: var(--accent);
		box-shadow: 0 0 12px color-mix(in srgb, var(--accent) 45%, transparent);
	}
	.focus-error {
		grid-row: 2;
	}
	.browser-workspace {
		grid-row: 3;
		min-width: 0;
		min-height: 0;
		display: grid;
		grid-template-columns: minmax(0, 1fr) var(--handover-w, 274px);
	}
	.handover-conversation {
		grid-column: 2;
		min-height: 0;
		display: grid;
		grid-template-rows: auto minmax(0, 1fr) auto;
		border-left: 1px solid var(--border-strong);
		background: color-mix(in srgb, var(--glass-strong) 94%, #111);
	}
	.handover-conversation-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 10px;
		padding: 13px 14px;
		border-bottom: 1px solid var(--border);
	}
	.handover-conversation-head > div {
		display: grid;
		gap: 3px;
		min-width: 0;
	}
	.handover-conversation-head strong {
		overflow: hidden;
		text-overflow: ellipsis;
		font-size: var(--t-body);
	}
	.controller-badge {
		flex: none;
		padding: 4px 6px;
		border: 1px solid var(--border-strong);
		font: var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
		color: var(--text-tertiary);
	}
	.controller-badge.owner {
		border-color: var(--accent);
		color: var(--accent);
		box-shadow: 0 0 9px color-mix(in srgb, var(--accent) 28%, transparent);
	}
	.handover-thread {
		min-height: 0;
		overflow: auto;
		display: flex;
		flex-direction: column;
		gap: 0;
		padding: 0;
	}
	.handover-footer {
		border-top: 1px solid var(--border);
	}
	.handover-footer.with-turn {
		border-top: 0;
	}
	.handover-composer {
		padding: 10px 12px 9px;
	}
	.conversation-error {
		margin: 0;
		padding: 0 12px 10px;
		color: var(--danger);
		font-size: var(--t-label);
	}
	.desktop-stage {
		grid-column: 1;
		grid-row: 1;
		min-width: 0;
		min-height: 0;
		padding: 10px;
	}
	.desktop-stage :global(iframe) {
		width: 100%;
		height: 100%;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
	}
	.outcome-offline {
		height: 100%;
		display: grid;
		place-content: center;
		justify-items: start;
		gap: 10px;
		padding: clamp(28px, 7vw, 92px);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background:
			radial-gradient(circle at 35% 40%, var(--intent-conversation-soft), transparent 44%),
			var(--glass-pane-strong);
	}
	.outcome-offline h2 {
		max-width: 520px;
		margin: 0;
		font-size: var(--t-title);
		line-height: 1.05;
	}
	@media (max-width: 1040px) {
		.browser-workspace {
			grid-template-columns: minmax(0, 1fr) var(--handover-w, 224px);
		}
		.handover-conversation-head {
			padding-inline: 10px;
		}
		.handover-thread {
			padding-inline: 0;
		}
	}
	@media (max-width: 760px) {
		.handover-rail,
		.handover-state {
			align-items: flex-start;
			flex-wrap: wrap;
		}
		.browser-workspace {
			grid-template-columns: 1fr;
			grid-template-rows: minmax(250px, 1fr) minmax(300px, 1fr);
			overflow: auto;
		}
		.desktop-stage {
			grid-column: 1;
			grid-row: 1;
			min-height: 250px;
		}
		.handover-conversation {
			grid-column: 1;
			grid-row: 2;
			min-height: 300px;
			border-top: 1px solid var(--border-strong);
			border-left: 0;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.live-mark.owner {
			box-shadow: none;
		}
	}
	.sr-title {
		position: absolute;
		width: 1px;
		height: 1px;
		margin: 0;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}
	/* An item that is a real surface (the charter, S82): its own heading and a link to where it
	 * lives, then the surface itself, not a summary of it. */
	.attention-surface {
		display: grid;
		grid-template-rows: auto minmax(0, 1fr);
		height: 100%;
		min-height: 0;
		width: 100%;
		overflow: hidden;
	}
	.attention-surface-head {
		padding: 22px 24px 14px;
		border-bottom: 1px solid var(--border);
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: var(--space-3);
	}
	.attention-surface-head h2 {
		margin: 0;
		font-size: var(--t-head);
		font-weight: 600;
		letter-spacing: -0.01em;
	}
	.attention-surface-head a {
		flex: none;
		white-space: nowrap;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		text-decoration: none;
	}
	.attention-surface-head a:hover {
		color: var(--ink);
	}

	.update-preview {
		display: flex;
		flex-direction: column;
		min-height: 0;
		height: 100%;
		overflow: hidden;
	}
	.update-preview :global(.inbox-preview) {
		flex: 1;
		min-height: 0;
	}
</style>
