<script lang="ts">
	import { askExec } from '$lib/model/ask-exec';
	import { failureSentence } from '$lib/model/failure';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { listFlip, listIn, listOut } from '$lib/ui/motion';
	import { resizePane } from '$lib/actions/resize-pane';
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import AttentionDocument from '$lib/components/AttentionDocument.svelte';
	import DesktopViewport from '$lib/components/DesktopViewport.svelte';
	import InfoTip from '$lib/ui/controls/InfoTip.svelte';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';
	import Paperclip from '@lucide/svelte/icons/paperclip';
	import Composer from '$lib/primitives/Composer.svelte';
	import ConversationMessage from '$lib/primitives/ConversationMessage.svelte';
	import InboxList from '$lib/components/InboxList.svelte';
	import { useForYou } from '$lib/model/for-you.svelte';
	import InboxDetail from '$lib/components/InboxDetail.svelte';
	import SetAsideList from '$lib/components/SetAside.svelte';
	import { celebrate } from '$lib/ui/celebrate';
	import DirectionRequestDetail from '$lib/components/DirectionRequestDetail.svelte';
	import CharterPurpose from '$lib/components/CharterPurpose.svelte';
	import {
		attentionTitle,
		coordinationFailure,
		isOutcomeReview
	} from '$lib/model/attention-presentation';
	import { formatRelative, formatMoment } from '$lib/ui/time';
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
	import { browserTabClientId } from '$lib/model/browserTab';
	import { getBrowserStatus } from '$lib/model/company';
	import { browserControl, issueDesktopTicket, issueReviewTicket } from '$lib/model/attention';
	import { takeInboxOutcome } from '$lib/model/inbox-outcome.svelte';
	import { companyFileLinks, workFileLinks } from '$lib/model/inbox-files';
	import {
		downloadHref,
		downloads,
		libraryFileBytes,
		officeFormat,
		type LibraryArtifact
	} from '$lib/model/library-files';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import OfficePreview from '$lib/components/OfficePreview.svelte';
	import Download from '@lucide/svelte/icons/download';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import { holdQueries } from '$lib/model/hold-queries.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');

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
		() => !!principal
	);
	const collaboration = $derived(
		collaborationBootstrapQuery(companyId, () => (owner ? null : principal))
	);
	const view = $derived(source.view);
	const loaded = $derived(source.status !== 'unknown');

	let error = $state('');
	let focusItem = $state<AttentionItem | null>(null);
	let desktopUrl = $state('');
	let controller = $state<'observer' | 'owner'>('observer');
	let autoClaimPending = $state(false);
	let lastDesktopActivity = $state(0);
	let lastLeaseRenewal = $state(0);
	let activityRenewing = $state(false);
	let conversationError = $state('');
	let messageDraft = $state('');
	let messageFiles = $state<File[]>([]);
	let sendingMessage = $state(false);
	let clientId = $state('');
	let controlLeaseId = $state('');
	let reviewUrl = $state('');
	let reviewError = $state('');
	let reviewRequestKey = $state('');
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
						standards: [],
						...collaboration.view.work_graph
					} as unknown as NonNullable<typeof view>['workGraph'])
				: null
	);
	/* The Work graph's file references, so files named by an item open in the Library viewer. */
	const artifacts = $derived((graph?.artifacts ?? []) as unknown as LibraryArtifact[]);
	const selectedItemId = $derived(page.url.searchParams.get('item'));
	/* Brand & voice not set yet: the clear Inbox offers to set it up with Exec. */
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
	const queueClear = $derived(attentionClear && !forYou?.items.length && forYou?.filter === 'all');
	/* Phones never mount the office canvas (see the clear state below). */
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
	const showClear = $derived(attentionClear && !catalogPending && !startBlocker);
	/* The queue starts collapsed until its source answers, so an idle
	 * company's office is never squeezed and released; when items do arrive,
	 * the queue opens with the same motion as any later change. */
	const selectedItem = $derived(
		items.find((item) => item.id === selectedItemId) ??
			(selectedItemId ? null : ((visibleItems ?? items)[0] ?? null))
	);
	let missingNotice = $state('');
	let missingItemRedirect = $state('');
	let resolvedHere = '';
	let sent = $state<{ text: string; href?: string } | null>(null);
	$effect(() => {
		if (!sent) return;
		const timer = setTimeout(() => (sent = null), 6000);
		return () => clearTimeout(timer);
	});
	/* A review decided from the rail lands here: confirm it like any other answer. */
	$effect(() => {
		const outcome = takeInboxOutcome();
		if (outcome) sent = outcome;
	});
	let previousItemOrder: string[] = [];
	$effect(() => {
		if (
			loaded &&
			selectedItemId &&
			missingItemRedirect !== selectedItemId &&
			!items.some((item) => item.id === selectedItemId)
		) {
			missingItemRedirect = selectedItemId;
			missingNotice =
				resolvedHere === selectedItemId
					? ''
					: 'That item has been resolved or is no longer in the Inbox.';
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
	const focusedReviewId = $derived(page.url.searchParams.get('review'));
	const focusedReview = $derived(
		items.find((item) => item.id === focusedReviewId && isOutcomeReview(item)) ?? null
	);
	/* An older Telegram link names a document request with `?review=`: open it as its item. */
	$effect(() => {
		const named = items.find((item) => item.id === focusedReviewId);
		if (named && !isOutcomeReview(named))
			void goto(itemHref(named.id), { replaceState: true, noScroll: true });
	});
	const reviewEvidence = $derived(focusedReview?.evidence ?? []);
	const reviewFileLink = $derived(companyFileLinks(companyId, artifacts));
	/* The Work's other files, one click from the outcome under review. */
	const reviewOtherFiles = $derived(
		workFileLinks(companyId, artifacts, focusedReview?.workId).filter(
			(file) => file.path !== focusedReview?.reviewTarget?.uri
		)
	);
	const DOCUMENT_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
	function sameCompanyDocumentHref(content: string, company: string): string {
		const canonicalPath = `/${encodeURIComponent(company)}/library/documents`;
		for (const line of content.split(/\r?\n/)) {
			const value = line.trim();
			const legacy = /^\/documents\/([^/?#\s]+)\/([^/?#\s]+)$/.exec(value);
			if (legacy?.[1] === company && DOCUMENT_ID.test(legacy[2]))
				return `${canonicalPath}?document=${encodeURIComponent(legacy[2])}`;
			if (!value.startsWith('/')) continue;
			let url: URL;
			try {
				url = new URL(value, 'https://restless.local');
			} catch {
				continue;
			}
			const documentId = url.searchParams.get('document');
			if (
				url.pathname === canonicalPath &&
				url.searchParams.size === 1 &&
				!url.hash &&
				!!documentId &&
				DOCUMENT_ID.test(documentId)
			)
				return `${canonicalPath}?document=${encodeURIComponent(documentId)}`;
		}
		return '';
	}
	const nativeDocumentHref = $derived.by(() => {
		for (const evidence of reviewEvidence) {
			if (!evidence.content) continue;
			const href = sameCompanyDocumentHref(evidence.content, companyId);
			if (href) return href;
		}
		return '';
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
			controller = 'observer';
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
				controller = 'observer';
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

	$effect(() => {
		const item = focusedReview;
		if (!item) {
			reviewUrl = '';
			reviewError = '';
			reviewRequestKey = '';
			return;
		}
		/* A computer that fell asleep is not a broken outcome: asking for the ticket wakes it. The
		 * open frame then survives the projection catching up from asleep to available. */
		const status = item.reviewTarget?.status ?? 'none';
		const key = `${item.id}:${item.reviewTarget?.generation ?? 'none'}:${status === 'asleep' ? 'available' : status}:${item.reviewTarget?.kind === 'runtime-text' ? status : ''}`;
		if (reviewRequestKey === key) return;
		reviewRequestKey = key;
		reviewUrl = '';
		if (item.reviewTarget?.status === 'asleep') {
			reviewError = '';
			void issueReviewTicket(companyId, item.id)
				.then((url) => {
					if (reviewRequestKey !== key) return;
					if (url) reviewUrl = url;
					else void source.refresh();
				})
				.catch((cause) => {
					if (reviewRequestKey === key)
						reviewError = failureSentence(cause, 'The company computer could not be woken.');
				});
			return;
		}
		if (!item.reviewTarget) {
			/* The accountable actor did not link a target this cockpit can open.
			 * Say what is actually true and show the evidence it did prepare —
			 * a heading claiming the outcome "is not ready" was wrong whenever
			 * the outcome was ready and merely in a format we could not frame. */
			reviewError =
				'The company did not prepare a target this cockpit can open. Its recorded evidence is below.';
			return;
		}
		if (item.reviewTarget.status !== 'available') {
			reviewError =
				item.reviewTarget.unavailableReason ??
				'The prepared outcome is unavailable. This page will reconnect automatically.';
			return;
		}
		reviewError = '';
		if (item.reviewTarget.kind === 'runtime-text') return;
		void issueReviewTicket(companyId, item.id)
			.then((url) => {
				if (reviewRequestKey === key) reviewUrl = url ?? '';
			})
			.catch((cause) => {
				if (reviewRequestKey === key) {
					reviewError = failureSentence(cause, 'The live website is unavailable.');
				}
			});
	});
	const reviewPath = $derived(focusedReview?.reviewTarget?.uri?.split(/[?#]/)[0] ?? '');
	const reviewIsFile = $derived(focusedReview?.reviewTarget?.kind === 'runtime-file');
	const reviewIsPdf = $derived(reviewIsFile && /\.pdf$/i.test(reviewPath));
	const reviewOffice = $derived(reviewIsFile ? officeFormat(reviewPath) : null);
	const reviewIsPowerPoint = $derived(/\.pptx?$/i.test(reviewPath));
	/* A file the browser cannot show (another Office format, an archive) is offered to download. */
	const reviewDownloadOnly = $derived(
		reviewIsFile && !reviewOffice && downloads({ uri: reviewPath })
	);
	const reviewSaveHref = $derived(reviewUrl && reviewIsFile ? downloadHref(reviewUrl) : '');

	onMount(() => {
		void browserTabClientId(companyId).then((id) => (clientId = id));
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

	/* Actions still refresh immediately after a write — the poll is a floor, not
	 * the only path. The source coalesces this with any poll already running. */
	async function refresh() {
		await source.refresh();
	}

	function itemHref(id: string): string {
		return `${baseHref}?item=${encodeURIComponent(id)}`;
	}

	const when = (value: Date | string) => formatRelative(value);

	function attentionKind(category: string): string {
		return (
			{
				approval: 'Approval',
				review: 'Outcome review',
				decision: 'Decision',
				blocker: 'Blocked',
				opportunity: 'Opportunity',
				contradiction: 'Conflicting evidence',
				human_step: 'Your participation',
				collaboration: 'Work together',
				conversation: 'Question'
			}[category] ?? category.replaceAll('_', ' ')
		);
	}

	function observedDesktopUrl(): string {
		return `/desktop/${encodeURIComponent(companyId)}/observe`;
	}

	function controlledDesktopUrl(): string {
		return `/desktop/${encodeURIComponent(companyId)}/control?client_id=${encodeURIComponent(clientId)}`;
	}

	async function syncFocusedControl() {
		if (!focusItem || !desktopUrl || !clientId) return;
		try {
			const status = await getBrowserStatus(companyId);
			if (status.control?.controller === 'owner' && status.control.client_id === clientId) {
				controller = 'owner';
				const controlled = controlledDesktopUrl();
				if (desktopUrl !== controlled) desktopUrl = controlled;
			} else if (controller === 'owner') {
				controller = 'observer';
				desktopUrl = observedDesktopUrl();
			}
			if (autoClaimPending) {
				autoClaimPending = false;
				await takeControl(true);
			}
		} catch (cause) {
			if (focusItem) error = failureSentence(cause, 'Browser control state is unavailable.');
		}
	}

	async function attachPreparedComputer(item: AttentionItem) {
		error = '';
		focusItem = item;
		controller = 'observer';
		conversationError = '';
		messageDraft = '';
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

	async function takeControl(silent = false) {
		if (!focusItem || !clientId) return;
		error = '';
		try {
			const control = await browserControl(companyId, 'take', clientId);
			controlLeaseId = control.lease_id ?? '';
			controller = 'owner';
			desktopUrl = controlledDesktopUrl();
			lastDesktopActivity = Date.now();
			lastLeaseRenewal = Date.now();
		} catch (cause) {
			if (!silent) error = failureSentence(cause, 'Control is held elsewhere.');
		}
	}

	async function returnControl(automatic = false) {
		if (!focusItem || !clientId) return;
		error = '';
		try {
			await browserControl(companyId, 'return', clientId, controlLeaseId);
			controlLeaseId = '';
			controller = 'observer';
			desktopUrl = observedDesktopUrl();
			lastDesktopActivity = 0;
		} catch (cause) {
			if (!automatic) error = failureSentence(cause, 'Control could not be returned.');
		}
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
		void browserControl(companyId, 'heartbeat', clientId, controlLeaseId)
			.catch((cause) => {
				if (!focusItem) return;
				controller = 'observer';
				desktopUrl = observedDesktopUrl();
				error = failureSentence(cause, 'Desktop control expired.');
			})
			.finally(() => (activityRenewing = false));
	}

	async function closePreparedComputer(item: AttentionItem | null = focusItem) {
		if (controller === 'owner') await returnControl();
		focusItem = null;
		desktopUrl = '';
		focusAttachKey = '';
		error = '';
		await goto(item ? itemHref(item.id) : baseHref, { replaceState: true });
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
			await leadConversation.send(body, files, undefined, false, leadConversation.working);
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

{#if focusedReview}
	<section
		class="review-canvas"
		use:resizePane={{
			key: `${companyId}:review`,
			label: 'Resize review source pane',
			target: '.review-source',
			variable: '--review-source-w',
			min: 220,
			minOther: 280,
			defaultSize: (width) => width * 0.42,
			enabled: focusedReview.reviewSources.length > 0
		}}
		class:with-source={focusedReview.reviewSources.length > 0}
		aria-label={`Review ${focusedReview.title}`}
	>
		{#if focusedReview.reviewSources.length > 0}
			<aside class="review-source" aria-label="Source material">
				<header class="review-source-head">
					<h2>Source</h2>
					<InfoTip
						text="External inputs already linked to this exact Work. Each source preserves its observed verification state."
					/>
				</header>
				<div class="review-source-scroll">
					{#each focusedReview.reviewSources as source (source.reference)}
						<article class="review-source-card">
							<header>
								<strong>{source.label}</strong>
								<div class="review-source-meta">
									<span title="Observed source verification">{source.verification}</span>
									<time>{when(source.observedAt)}</time>
								</div>
							</header>
							<pre>{source.content}</pre>
							{#if source.uri}
								<a href={source.uri} target="_blank" rel="noreferrer"
									>Open original in {source.provider} ↗</a
								>
							{/if}
						</article>
					{/each}
				</div>
			</aside>
		{/if}
		<div class="review-outcome" aria-label="Prepared outcome">
			<header class="review-outcome-head">
				<strong title={focusedReview.reviewTarget?.uri}
					>{focusedReview.reviewTarget?.label ?? focusedReview.title}</strong
				>
				{#if reviewOtherFiles.length}
					<span class="review-other-files">
						<ActionMenu
							label={`${reviewOtherFiles.length} other ${reviewOtherFiles.length === 1 ? 'file' : 'files'} from this work`}
						>
							{#snippet trigger()}<Paperclip
									size={15}
									strokeWidth={1.8}
									aria-hidden="true"
								/>{/snippet}
							{#each reviewOtherFiles as file (file.href)}
								<a href={file.href} title={file.path}>{file.label}</a>
							{/each}
						</ActionMenu>
					</span>
				{/if}
				<InfoTip
					text="The exact candidate selected by Staff and inspected by the accountable lead."
				/>
				<span class="review-outcome-spacer"></span>
				{#if reviewUrl && reviewIsFile && !reviewDownloadOnly && !reviewOffice}
					<a
						class="review-outcome-action"
						href={reviewUrl}
						target="_blank"
						rel="noopener noreferrer"
						title="Open in a new tab"
						aria-label="Open in a new tab"><ExternalLink size={15} strokeWidth={1.9} /></a
					>
				{:else if reviewUrl && !reviewIsFile}
					<a
						class="review-outcome-action"
						href={reviewUrl}
						target="_blank"
						rel="noopener noreferrer"
						title="Open the site in a new tab"
						aria-label="Open the site in a new tab"><ExternalLink size={15} strokeWidth={1.9} /></a
					>
				{/if}
				{#if reviewSaveHref}
					<a
						class="review-outcome-action"
						href={reviewSaveHref}
						download
						title="Download the file"
						aria-label="Download the file"><Download size={15} strokeWidth={1.9} /></a
					>
				{:else if focusedReview.reviewTarget?.kind === 'runtime-text' && reviewPath}
					<a
						class="review-outcome-action"
						href={`/${encodeURIComponent(companyId)}/library?path=${encodeURIComponent(reviewPath)}`}
						title="Open in the Library to download or link it"
						aria-label="Open in the Library"><ExternalLink size={15} strokeWidth={1.9} /></a
					>
				{/if}
			</header>
			<div class="review-outcome-stage">
				{#if focusedReview.reviewTarget?.kind === 'runtime-text' && focusedReview.reviewTarget.content}
					<article class="review-document">
						<Markdown text={focusedReview.reviewTarget.content} />
					</article>
				{:else if reviewOffice && reviewUrl}
					<OfficePreview
						format={reviewOffice}
						name={reviewPath.split('/').pop() ?? reviewPath}
						source={reviewPath}
						load={() => libraryFileBytes(companyId, reviewPath)}
						downloadHref={reviewSaveHref}
					/>
				{:else if reviewDownloadOnly && reviewUrl}
					<div class="review-download">
						<div class="review-download-kind">
							{reviewPath.match(/\.([a-z0-9]+)$/i)?.[1]?.toUpperCase() ?? 'FILE'}
						</div>
						<h1>{focusedReview.reviewTarget?.label ?? focusedReview.title}</h1>
						<p>
							{reviewIsPowerPoint
								? 'Browsers cannot show PowerPoint files. Download it to review it in PowerPoint or Keynote.'
								: 'Browsers cannot show this kind of file. Download it to review it on your computer.'}
						</p>
						<a href={reviewSaveHref} download>Download file</a>
					</div>
				{:else if reviewUrl && reviewIsPdf && compactScreen}
					<div class="review-download">
						<div class="review-download-kind">PDF</div>
						<h1>{focusedReview.reviewTarget?.label ?? focusedReview.title}</h1>
						<p>This PDF reads best on its own on a phone.</p>
						<a href={reviewUrl} target="_blank" rel="noopener noreferrer">Open the PDF</a>
					</div>
				{:else if reviewUrl && reviewIsPdf}
					<!-- The browser's PDF reader will not run in a sandboxed frame; the PDF is already on
					     its own read-only review origin and runs no page script (S64). -->
					<iframe
						title={focusedReview.reviewTarget?.label ?? focusedReview.title}
						src={reviewUrl}
						referrerpolicy="no-referrer"
					></iframe>
				{:else if reviewUrl}
					<iframe
						title={focusedReview.reviewTarget?.label ?? focusedReview.title}
						src={reviewUrl}
						sandbox="allow-downloads allow-forms allow-modals allow-popups allow-same-origin allow-scripts"
						referrerpolicy="no-referrer"
					></iframe>
				{:else}
					<div class="review-unavailable" role="status">
						<h1>{reviewError ? 'This outcome cannot be opened here.' : 'Opening the outcome…'}</h1>
						{#if reviewError}
							<p>{reviewError}</p>
							{#if focusedReview.reviewTarget?.uri}
								<code class="review-unavailable-uri">{focusedReview.reviewTarget.uri}</code>
							{/if}
							{#if reviewEvidence.length}
								<ul class="review-unavailable-evidence">
									{#each reviewEvidence as evidence (evidence.label)}
										{@const file =
											evidence.kind === 'artifact' && evidence.content
												? reviewFileLink(evidence.content, evidence.label)
												: null}
										<li>
											{#if evidence.uri}
												<a href={evidence.uri} target="_blank" rel="noreferrer"
													>{evidence.label} <span aria-hidden="true">↗</span></a
												>
											{:else if file}
												<a href={file.href} title={`Open in the Library · ${file.path}`}
													>{file.label}</a
												>
											{:else}
												<strong>{evidence.label}</strong>
											{/if}
										</li>
									{/each}
								</ul>
							{/if}
							{#if nativeDocumentHref}
								<a class="btn small primary" href={nativeDocumentHref}>Open document</a>
							{/if}
						{/if}
					</div>
				{/if}
			</div>
		</div>
	</section>
{:else if focusItem}
	<div class="browser-focus">
		<header class="handover-rail">
			<div class="handover-context">
				<span class="live-mark" class:owner={controller === 'owner'}></span>
				<div>
					<strong>{focusItem.title}</strong>
					<span class="handover-request" title="Exact action requested by the responsible actor"
						>{focusItem.requestedAction}</span
					>
				</div>
			</div>
			<div class="handover-state">
				{#if focusItem.runtimeAttach && desktopUrl}
					<span
						class="mono"
						title="Control changes keyboard and pointer ownership only; it does not complete this handoff."
						>{view?.sourceHealth.browser ?? 'unknown'} · {controller === 'owner'
							? 'You control'
							: 'Observe only'}</span
					>
					{#if controller === 'observer'}
						<button
							class="btn small primary"
							type="button"
							title="Pauses company automation and gives this tab sole keyboard and pointer control."
							onclick={() => takeControl()}>Take control</button
						>
					{:else}
						<button
							class="btn small"
							type="button"
							title="Returns input to the responsible actor. The source condition is checked separately."
							onclick={() => returnControl()}>Return control</button
						>
					{/if}
				{:else}
					<span class="mono">Preview offline · Discussion available</span>
				{/if}
				<button
					class="btn small"
					type="button"
					title="Leaves this work-through view without resolving the handoff."
					onclick={() => closePreparedComputer()}>Leave computer</button
				>
			</div>
		</header>
		{#if error}<div class="focus-error">{error}</div>{/if}
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
					<span class="controller-badge" class:owner={controller === 'owner' && !!desktopUrl}>
						{desktopUrl
							? controller === 'owner'
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
								actionLabel={conversationWorking ? 'Queue direction' : 'Send'}
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
						<span class="mono">LIVE OUTCOME UNAVAILABLE</span>
						<h2>The review and lead conversation are still available.</h2>
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
						onload={syncFocusedControl}
						onactivity={desktopActivity}
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
		class:mobile-detail={compactScreen && !!selectedItemId}
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
				{sent.text}{#if sent.href}<a href={sent.href}>Open conversation</a>{/if}<button
					aria-label="Dismiss notification"
					onclick={() => (sent = null)}>×</button
				>
			</div>{/if}
		{#if missingNotice}<div class="attention-resolution-notice" role="status">
				{missingNotice}<button
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
				{loaded}
				failure={source.failure}
				{coordination}
				onretry={source.reload}
			/>
		</aside>

		<section
			class="cockpit-pane attention-focus"
			class:office-focus={!selectedItem && loaded && !compactScreen}
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
				class:visible={showClear}
				type="button"
				aria-hidden={!showClear}
				tabindex={showClear ? 0 : -1}
				title={showClear ? 'No decision is waiting on you. Check again now.' : undefined}
				onclick={() => void refresh()}
			>
				<span class="attention-clear-glyph" aria-hidden="true">
					<MatrixGlyph rows={GLYPHS.check} size={7} />
				</span>
				<span>No decisions</span>
			</button>
			{#if queueClear}<div class="attention-set-aside">
					{#if owner}<SetAsideList {companyId} />{/if}
					{#if forYou}<button
							type="button"
							class="attention-archived"
							title="For you items you archived"
							onclick={() => forYou.setFilter('archived')}>Archived</button
						>{/if}
				</div>{/if}
			{#if compactScreen && selectedItemId}<a class="attention-mobile-back" href={baseHref}
					>← Inbox</a
				>{/if}
			{#if selectedItem}
				{#if selectedItem.nativeDocument}
					{#key `${companyId}:${selectedItem.id}`}
						<AttentionDocument
							{companyId}
							request={selectedItem.nativeDocument}
							onresolved={() => void source.refresh()}
						/>
					{/key}
				{:else}
					{#key `${companyId}:${selectedItem.id}`}
						{@render attentionDetail(selectedItem)}
					{/key}
				{/if}
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
			{:else if compactScreen}
				<!-- A phone gets the verdict, not the company floor: the pixel
				     campus is a large canvas that costs battery for little use at
				     this size. -->
				<div class="attention-clear-compact" role="status">
					{#if catalogPending}<p>Checking company…</p>
					{:else if startBlocker}<TriangleAlert size={22} />
						<p>Exec is waiting for intelligence.</p>
						<a class="btn primary small" href={startFixHref(companyId)}>{startBlocker}</a>
					{:else}
						<span class="attention-clear-compact-mark" aria-hidden="true">
							<MatrixGlyph rows={GLYPHS.check} size={10} />
						</span>
						<p>Nothing needs you right now.</p>
						<button class="btn small primary" type="button" onclick={askExec}
							>Tell Exec what you want</button
						>
						<a class="btn small" href={`/${companyId}/work`}>Open Work</a>
					{/if}
				</div>
			{:else}
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
				<CompanyOffice
					{companyId}
					{graph}
					collaboration={owner ? null : collaboration.view}
					sourceHealth={view?.sourceHealth ?? {}}
					quietSignal={catalogPending || !!startBlocker}
				/>
			{/if}
		</section>
	</div>
{/if}

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
			onresolved={(result) => {
				resolvedHere = item.id;
				sent = result;
				void source.refresh();
			}}
		/>
	{:else}<InboxDetail
			{companyId}
			{item}
			{artifacts}
			workOwnerId={graph?.work.find((work) => work.id === item.workId)?.owner_id}
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
	.brand-nudge {
		position: absolute;
		z-index: 2;
		/* Below the "No decisions" pill that shares this corner. */
		top: 52px;
		left: 12px;
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
			display: block;
			flex: 1;
		}
	}

	.review-canvas {
		width: 100%;
		height: 100%;
		min-width: 0;
		min-height: 0;
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		overflow: hidden;
		background: var(--surface-alt);
	}
	.review-canvas.with-source {
		grid-template-columns: minmax(0, var(--review-source-w, 42%)) minmax(0, 1fr);
	}
	.review-source,
	.review-outcome {
		min-width: 0;
		min-height: 0;
		display: grid;
		grid-template-rows: auto minmax(0, 1fr);
	}
	.review-source {
		border-right: 1px solid var(--border-strong);
		background: color-mix(in srgb, var(--surface-alt) 82%, var(--surface-raised));
	}
	.review-source-head,
	.review-outcome-head {
		min-height: 48px;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2);
		padding: 10px 14px;
		border-bottom: 1px solid var(--border);
		background: color-mix(in srgb, var(--surface-pane) 86%, transparent);
	}
	.review-source-head h2,
	.review-outcome-head strong {
		min-width: 0;
		margin: 0;
		overflow: hidden;
		font-size: var(--t-head);
		font-weight: 600;
		line-height: 1.35;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.review-other-files {
		margin-left: auto;
	}
	.review-source-scroll,
	.review-outcome-stage {
		min-width: 0;
		min-height: 0;
		overflow: auto;
	}
	.review-source-scroll {
		padding: 12px;
	}
	.review-source-card {
		overflow: hidden;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface-pane);
		box-shadow: var(--bevel);
	}
	.review-source-card + .review-source-card {
		margin-top: 10px;
	}
	.review-source-card header {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: var(--space-2);
		padding: 10px 11px;
		border-bottom: 1px solid var(--border);
	}
	.review-source-card header strong {
		font-size: var(--t-body);
		font-weight: 600;
	}
	.review-source-card time {
		flex: none;
		font: var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
		color: var(--text-tertiary);
	}
	.review-source-meta {
		flex: none;
		display: grid;
		justify-items: end;
		gap: 2px;
	}
	.review-source-meta span {
		font-size: var(--t-label);
		color: var(--intent-feedback);
	}
	.review-source-card pre {
		max-height: min(58vh, 620px);
		margin: 0;
		overflow: auto;
		padding: 12px;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		font: var(--t-body)/1.55 var(--font-mono);
		color: var(--text-secondary);
	}
	.review-source-card a {
		display: block;
		padding: 9px 11px;
		border-top: 1px solid var(--border);
		color: var(--ink);
		font-size: var(--t-body);
		text-decoration: none;
	}
	.review-source-card a:hover {
		background: var(--surface-alt);
	}
	.review-outcome {
		background: var(--surface-raised);
	}
	.review-outcome-stage iframe {
		width: 100%;
		height: 100%;
		border: 0;
		background: var(--surface-raised);
	}
	.review-document {
		width: min(820px, calc(100% - 48px));
		min-height: calc(100% - 48px);
		margin: 24px auto;
		padding: clamp(28px, 5vw, 58px);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-lift);
	}
	.review-document :global(.md) {
		font-size: var(--t-head);
		line-height: 1.65;
		color: var(--ink);
	}
	.review-unavailable {
		width: 100%;
		height: 100%;
		display: grid;
		place-content: center;
		gap: 8px;
		padding: 40px;
		background: var(--surface-raised);
		color: var(--ink);
	}
	.review-download {
		width: min(480px, calc(100% - 48px));
		margin: auto;
		padding: 28px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		color: var(--ink);
	}
	.review-download-kind {
		margin-bottom: 20px;
		color: var(--text-tertiary);
		font: var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
	}
	.review-download h1 {
		margin: 0;
		font-size: var(--t-head);
	}
	.review-download p {
		color: var(--text-secondary);
		line-height: 1.5;
	}
	.review-download a {
		display: inline-block;
		margin: 8px 0 16px;
		padding: 9px 14px;
		border-radius: var(--radius-md);
		background: var(--ink);
		color: var(--surface);
		text-decoration: none;
	}
	.review-outcome-spacer {
		flex: 1;
	}
	.review-outcome-action {
		display: inline-grid;
		flex: none;
		place-items: center;
		width: 30px;
		height: 30px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
	}
	.review-outcome-action:hover {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.review-unavailable h1,
	.review-unavailable p {
		max-width: 560px;
		margin: 0;
	}
	.review-unavailable p {
		color: var(--text-secondary);
	}
	.attention-clear-compact {
		display: grid;
		justify-items: center;
		align-content: center;
		gap: var(--space-3);
		height: 100%;
		padding: var(--space-6);
		text-align: center;
	}
	.attention-clear-compact p {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-head);
		font-weight: 600;
	}
	.attention-clear-compact-mark {
		display: grid;
		place-items: center;
		width: 40px;
		height: 40px;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface-raised);
		color: var(--state-success);
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
	.handover-state .mono {
		font-size: var(--t-label);
		color: var(--text-tertiary);
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
	.outcome-offline > .mono {
		font-size: var(--t-label);
		letter-spacing: 0.1em;
		color: var(--intent-authority);
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
		.review-canvas.with-source {
			grid-template-columns: minmax(0, 1fr);
			grid-template-rows: minmax(240px, 0.72fr) minmax(360px, 1fr);
			overflow: auto;
		}
		.review-source {
			border-right: 0;
			border-bottom: 1px solid var(--border-strong);
		}
		.review-source-card pre {
			max-height: 260px;
		}
		.review-document {
			width: calc(100% - 24px);
			min-height: calc(100% - 24px);
			margin: 12px auto;
			padding: 22px;
		}
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
		gap: var(--space-4);
		max-width: 760px;
		width: 100%;
		margin: 0 auto;
		padding: var(--space-5) var(--space-5) var(--space-6);
	}
	.attention-surface-head {
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
</style>
