<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import DesktopViewport from '$lib/components/DesktopViewport.svelte';
	import Composer from '$lib/primitives/Composer.svelte';
	import ConversationMessage from '$lib/primitives/ConversationMessage.svelte';
	import HoldApprove from '$lib/primitives/HoldApprove.svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import SemanticMark from '$lib/primitives/SemanticMark.svelte';
	import ConversationTurnDock from '$lib/primitives/ConversationTurnDock.svelte';
	import CompanyOffice from '$lib/office/CompanyOffice.svelte';
	import {
		mergeAdjacentAgentMessages,
		type AttentionItem,
		type DecisionContinuation
	} from '$lib/model/view';
	import { attentionSource } from '$lib/model/attentionSource.svelte';
	import { conversationSource } from '$lib/model/conversationSource.svelte';
	import { browserTabClientId } from '$lib/model/browserTab';
	import { getBrowserStatus } from '$lib/model/company';
	import {
		approvalAction,
		browserControl,
		issueDesktopTicket,
		issueReviewTicket,
		resolveHandoffDecision
	} from '$lib/model/attention';

	const companyId = $derived(page.params.companyId ?? 'aris');

	/* The shared source, not a second poller. `loaded` is the distinction this
	 * surface was missing: before it existed, `items` was `[]` both when the
	 * queue was genuinely clear and when nobody had asked yet, so first paint
	 * asserted "Nothing needs your judgement" and took it back a moment later. */
	const source = $derived(attentionSource(companyId));
	$effect(() => source.attach());
	const view = $derived(source.view);
	const loaded = $derived(source.status !== 'unknown');

	let error = $state('');
	let acting = $state(false);
	let focusItem = $state<AttentionItem | null>(null);
	let desktopUrl = $state('');
	let controller = $state<'observer' | 'owner'>('observer');
	let conversationError = $state('');
	let messageDraft = $state('');
	let messageFiles = $state<File[]>([]);
	let sendingMessage = $state(false);
	let clientId = $state('');
	let reviewUrl = $state('');
	let reviewError = $state('');
	let reviewRequestKey = $state('');
	let decisionDraft = $state('');
	let recentDecisionsOpen = $state(false);
	let focusAttachKey = $state('');

	const items = $derived(view?.items ?? []);
	const continuations = $derived(view?.continuations ?? []);
	const graph = $derived(view?.workGraph ?? null);
	const selectedItemId = $derived(page.url.searchParams.get('item'));
	const selectedItem = $derived(
		items.find((item) => item.id === selectedItemId) ?? (selectedItemId ? null : (items[0] ?? null))
	);
	const selectedContinuation = $derived(
		continuations.find((continuation) => continuation.id === selectedItemId) ?? null
	);
	$effect(() => {
		recentDecisionsOpen = Boolean(selectedContinuation);
	});
	const focusedReviewId = $derived(page.url.searchParams.get('review'));
	const focusedReview = $derived(
		items.find((item) => item.id === focusedReviewId && item.category === 'review') ?? null
	);
	const focusedComputerId = $derived(page.url.searchParams.get('computer'));
	const baseHref = $derived(`/${companyId}`);
	const requestingActor = $derived(
		focusItem?.responsibleActor?.id ?? focusItem?.runtimeAttach?.requestingActor ?? ''
	);
	const leadConversation = $derived(
		requestingActor && focusItem
			? conversationSource(companyId, requestingActor, focusItem.workId)
			: null
	);
	$effect(() => leadConversation?.attach());
	const requestingActorName = $derived(
		leadConversation?.actor?.display ||
			focusItem?.responsibleActor?.display ||
			focusItem?.runtimeAttach?.requestingActorDisplay ||
			(requestingActor === 'exec' ? 'Exec' : requestingActor || 'Company context')
	);
	const visibleConversationMessages = $derived(
		mergeAdjacentAgentMessages(leadConversation?.messages ?? [])
	);
	const conversationTurn = $derived(leadConversation?.activeTurn ?? null);
	const attachmentHref = (attachment: { uploadId: string }) =>
		`/api/companies/${encodeURIComponent(companyId)}/attachments/${encodeURIComponent(attachment.uploadId)}`;

	$effect(() => {
		const item = items.find((candidate) => candidate.id === focusedComputerId) ?? null;
		if (!focusedComputerId) {
			focusAttachKey = '';
			focusItem = null;
			desktopUrl = '';
			controller = 'observer';
			return;
		}
		if (!item || !clientId) return;
		const key = `${item.id}:${item.runtimeAttach?.generation ?? 'none'}`;
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
		const key = `${item.id}:${item.reviewTarget?.generation ?? 'none'}:${item.reviewTarget?.status ?? 'none'}`;
		if (reviewRequestKey === key) return;
		reviewRequestKey = key;
		reviewUrl = '';
		if (!item.reviewTarget) {
			reviewError = 'This outcome does not have a directly reviewable website.';
			return;
		}
		if (item.reviewTarget.status !== 'available') {
			reviewError = 'The live website is restarting. This page will reconnect automatically.';
			return;
		}
		reviewError = '';
		void issueReviewTicket(companyId, item.id)
			.then((url) => {
				if (reviewRequestKey === key) reviewUrl = url;
			})
			.catch((cause) => {
				if (reviewRequestKey === key) {
					reviewError = cause instanceof Error ? cause.message : 'The live website is unavailable.';
				}
			});
	});

	onMount(() => {
		void browserTabClientId(companyId).then((id) => (clientId = id));
		const heartbeat = window.setInterval(() => {
			if (controller === 'owner') {
				void browserControl(companyId, 'heartbeat', clientId).catch((cause) => {
					controller = 'observer';
					desktopUrl = observedDesktopUrl();
					error = cause instanceof Error ? cause.message : 'Browser control lease ended.';
				});
			}
		}, 12_000);
		return () => {
			window.clearInterval(heartbeat);
		};
	});

	/* Actions still refresh immediately after a write — the poll is a floor, not
	 * the only path. The source coalesces this with any poll already running. */
	async function refresh() {
		await source.refresh();
		error =
			source.status === 'stale' ? (source.failure?.message ?? 'Attention is unavailable.') : '';
	}

	function itemHref(id: string): string {
		return `${baseHref}?item=${encodeURIComponent(id)}`;
	}

	function when(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		return date.toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function attentionKind(category: string): string {
		return (
			{
				approval: 'Approval',
				review: 'Outcome review',
				decision: 'Decision',
				blocker: 'Blocked',
				opportunity: 'Opportunity',
				contradiction: 'Conflicting evidence',
				human_step: 'Your participation'
			}[category] ?? category.replaceAll('_', ' ')
		);
	}

	function attentionAction(category: string): string {
		return (
			{
				approval: 'Approve',
				review: 'Review',
				decision: 'Decide',
				blocker: 'Resolve',
				opportunity: 'Consider',
				contradiction: 'Review',
				human_step: 'Continue'
			}[category] ?? 'Open'
		);
	}

	function workTitle(workId: string | undefined, fallback: string): string {
		return graph?.work.find((work) => work.id === workId)?.title ?? fallback;
	}

	function partyOf(item: AttentionItem): string {
		return item.id.startsWith('authority:approval:') ? item.id.split(':').slice(3).join(':') : '';
	}

	async function decide(item: AttentionItem, action: 'grant' | 'decline') {
		const party = partyOf(item);
		if (!party || acting) return;
		acting = true;
		error = '';
		try {
			await approvalAction(companyId, action, party);
			await refresh();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The authority action failed.';
		} finally {
			acting = false;
		}
	}

	async function recordDecision(item: AttentionItem) {
		const resolution = decisionDraft.trim();
		if (!resolution || acting) return;
		acting = true;
		error = '';
		try {
			await resolveHandoffDecision(companyId, item.source.reference, resolution);
			decisionDraft = '';
			await refresh();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The decision was not recorded.';
		} finally {
			acting = false;
		}
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
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Browser control state is unavailable.';
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
			desktopUrl = await issueDesktopTicket(companyId, item.id, clientId);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The live browser is unavailable.';
		}
	}

	async function openReview(item: AttentionItem) {
		if (item.category === 'review') {
			await goto(`${baseHref}?review=${encodeURIComponent(item.id)}`);
			return;
		}
		if (!clientId) return;
		await goto(
			`${baseHref}?item=${encodeURIComponent(item.id)}&computer=${encodeURIComponent(item.id)}`
		);
	}

	async function talkToLead(item: AttentionItem) {
		await goto(
			`${baseHref}?item=${encodeURIComponent(item.id)}&conversation=${encodeURIComponent(item.id)}`
		);
	}

	async function takeControl() {
		if (!focusItem || !clientId) return;
		error = '';
		try {
			await browserControl(companyId, 'take', clientId);
			controller = 'owner';
			desktopUrl = controlledDesktopUrl();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Control is held elsewhere.';
		}
	}

	async function returnControl() {
		if (!focusItem || !clientId) return;
		error = '';
		try {
			await browserControl(companyId, 'return', clientId);
			controller = 'observer';
			desktopUrl = observedDesktopUrl();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Control could not be returned.';
		}
	}

	async function closePreparedComputer() {
		const item = focusItem;
		if (controller === 'owner') await returnControl();
		focusItem = null;
		desktopUrl = '';
		focusAttachKey = '';
		await goto(item ? itemHref(item.id) : baseHref);
	}

	async function sendMessage(event: SubmitEvent) {
		event.preventDefault();
		const body = messageDraft.trim();
		if (!body || sendingMessage || !leadConversation) return;
		sendingMessage = true;
		conversationError = '';
		const sent = messageDraft;
		const files = messageFiles;
		messageDraft = '';
		try {
			await leadConversation.send(body, files);
			messageFiles = [];
		} catch (cause) {
			messageDraft = sent;
			conversationError =
				cause instanceof Error ? cause.message : 'Your message was not delivered.';
		} finally {
			sendingMessage = false;
		}
	}
</script>

<svelte:head><title>Attention — {view?.company.name ?? companyId}</title></svelte:head>

{#if focusedReview}
	<section class="review-canvas" aria-label={`Review ${focusedReview.title}`}>
		{#if reviewUrl}
			<iframe
				title={focusedReview.reviewTarget?.label ?? focusedReview.title}
				src={reviewUrl}
				sandbox="allow-downloads allow-forms allow-modals allow-popups allow-same-origin allow-scripts"
				referrerpolicy="no-referrer"
			></iframe>
		{:else}
			<div class="review-unavailable" role="status">
				<span class="mono">LIVE OUTCOME</span>
				<h1>{reviewError ? 'The website is not ready yet.' : 'Opening the website…'}</h1>
				{#if reviewError}<p>{reviewError}</p>{/if}
			</div>
		{/if}
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
							onclick={takeControl}>Take control</button
						>
					{:else}
						<button
							class="btn small"
							type="button"
							title="Returns input to the responsible actor. The source condition is checked separately."
							onclick={returnControl}>Return control</button
						>
					{/if}
				{:else}
					<span class="mono">Preview offline · Discussion available</span>
				{/if}
				<button
					class="btn small"
					type="button"
					title="Leaves the prepared computer without resolving the handoff."
					onclick={closePreparedComputer}>Leave computer</button
				>
			</div>
		</header>
		{#if error}<div class="focus-error">{error}</div>{/if}
		<div class="browser-workspace">
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
							attachments={message.attachments}
							hrefFor={attachmentHref}
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
								disabled={sendingMessage}
								placeholder={`Ask or tell ${requestingActorName} anything…`}
								ariaLabel={`Message ${requestingActorName}`}
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
				<DesktopViewport
					src={desktopUrl}
					title="Live company browser"
					offline={offlineDesktop}
					onload={syncFocusedControl}
				/>
			</div>
		</div>
	</div>
{:else}
	<div class="cockpit-screen attention-screen">
		{#if error}<div class="cockpit-error attention-error">{error}</div>{/if}
		<aside class="cockpit-pane attention-index">
			<!-- No mark and no count. The mark rendered GLYPHS.ring, which means
			     "waiting" everywhere else in the vocabulary and read here as a
			     stray letter O; the count is already on the Attention tab, where
			     it is useful from the other three surfaces. What is left is the
			     label, and the queue below says the rest. -->
			<header class="attention-pane-title">
				<h1 class="attention-queue-title">Needs your judgement</h1>
			</header>
			<div class="attention-index-scroll">
				<div class="attention-list" class:clear={loaded && items.length === 0}>
					{#each items as item (item.id)}
						<a
							class="attention-item category-{item.category}"
							class:selected={selectedItem?.id === item.id}
							href={itemHref(item.id)}
							aria-current={selectedItem?.id === item.id ? 'true' : undefined}
						>
							<span class="attention-item-meta">
								<span title={`${attentionKind(item.category)} requiring owner attention`}>
									<MatrixGlyph rows={GLYPHS.rules} size={7} />
									{attentionKind(item.category)}
								</span>
								<time>{when(item.createdAt)}</time>
							</span>
							<strong class="attention-item-title">{workTitle(item.workId, item.title)}</strong>
							<span class="attention-item-action" title={item.requestedAction}>
								{attentionAction(item.category)} <span aria-hidden="true">→</span>
							</span>
						</a>
					{:else}
						{#if loaded}
							<div class="attention-list-clear">
								<span class="attention-clear-mark" title="No owner action is required">
									<MatrixGlyph rows={GLYPHS.check} size={7} />
								</span>
								<strong>All clear</strong>
								<small>Nothing needs your judgement.</small>
							</div>
						{:else}
							<!-- The source has not answered yet. Three placeholder rows
							     hold the shape of the queue without asserting that it is
							     empty — "Queue clear" here would be a claim we cannot
							     make, and would be contradicted a round trip later. -->
							<div class="attention-list-waiting" aria-hidden="true">
								<i></i><i></i><i></i>
							</div>
						{/if}
					{/each}
				</div>
				{#if continuations.length}
					<details class="decision-continuations" bind:open={recentDecisionsOpen}>
						<summary>
							<span>Recent decisions</span>
							<small>{continuations.length}</small>
							<span class="decision-disclosure" aria-hidden="true"></span>
						</summary>
						<div class="recent-decisions-list">
							{#each continuations as continuation (continuation.id)}
								<a
									class:selected={selectedContinuation?.id === continuation.id}
									href={itemHref(continuation.id)}
									aria-current={selectedContinuation?.id === continuation.id ? 'true' : undefined}
								>
									<span class="decision-continuation-meta">
										<span title="The owner decision was recorded">
											<SemanticMark meaning="success" size="small" />
											Decision observed
										</span>
										<time>{when(continuation.observedAt)}</time>
									</span>
									<strong>{workTitle(continuation.workId, continuation.title)}</strong>
									<span class="decision-continuation-action" title={continuation.whatItUnlocked}>
										Inspect <span aria-hidden="true">→</span>
									</span>
								</a>
							{/each}
						</div>
					</details>
				{/if}
			</div>
		</aside>

		<section
			class="cockpit-pane attention-focus"
			class:office-focus={!selectedItem && !selectedContinuation && loaded}
		>
			{#if selectedItem}
				{@render attentionDetail(selectedItem)}
			{:else if selectedContinuation}
				{@render continuationDetail(selectedContinuation)}
			{:else if !loaded}
				<!-- Deliberately nothing until the source answers. An empty pane for
				     one round trip reads as loading; the zero-state hero reads as a
				     verdict, and it was the wrong one about half a second later. -->
			{:else}
				<CompanyOffice {companyId} {graph} sourceHealth={view?.sourceHealth ?? {}} />
			{/if}
		</section>
	</div>
{/if}

{#snippet attentionDetail(item: AttentionItem)}
	<div class="inbox-pane">
		<article class="owner-folio category-{item.category}">
			<div class="folio-edge" aria-hidden="true"></div>
			<header class="folio-opening">
				<div class="folio-meta">
					<span>{attentionKind(item.category)}</span>
					{#if item.deadline}<strong>By {item.deadline}</strong>{/if}
				</div>
				<h1>{item.title}</h1>
				<p class="folio-situation">{item.whatHappened}</p>
				<p class="folio-impact">{item.whyItMatters}</p>
				{#if item.uncertainty}
					<p class="folio-uncertainty">
						<strong>What remains uncertain:</strong>
						{item.uncertainty}
					</p>
				{/if}
			</header>

			<section class="folio-recommendation" aria-label="Recommendation">
				<span>Recommendation</span>
				<p>{item.recommendation}</p>
			</section>

			<section class="folio-move" aria-label="Your next move">
				<div class="folio-move-copy">
					<span>Your next move</span>
					<Markdown text={item.requestedAction} />
					<small>If you wait: {item.ifNoAction}</small>
				</div>
				<div class="nc-actions">
					{#each item.actions.filter((action) => action.href) as action (action.id)}
						<a
							class="btn small primary"
							href={action.href}
							target="_blank"
							rel="noreferrer"
							title={action.consequence}>{action.label} ↗</a
						>
					{/each}
					{#if item.category === 'approval'}
						<form
							onsubmit={(event) => {
								event.preventDefault();
								void decide(item, 'grant');
							}}
						>
							<HoldApprove small label={acting ? 'Working…' : 'Hold to grant'} />
						</form>
						<button
							class="btn small danger"
							type="button"
							onclick={() => decide(item, 'decline')}
							disabled={acting}
						>
							Decline
						</button>
					{/if}
					{#if item.actions.some((action) => action.id === 'record-decision')}
						<form
							class="decision-response"
							onsubmit={(event) => {
								event.preventDefault();
								void recordDecision(item);
							}}
						>
							<input
								bind:value={decisionDraft}
								placeholder="Your decision…"
								aria-label="Your decision"
							/>
							<button
								class="btn small primary"
								type="submit"
								disabled={acting || !decisionDraft.trim()}
							>
								{acting ? 'Recording…' : 'Record decision'}
							</button>
						</form>
					{/if}
					{#if item.category === 'review'}
						<button class="btn small primary" type="button" onclick={() => openReview(item)}>
							Review outcome
						</button>
						{#if item.responsibleActor}
							<button class="btn small" type="button" onclick={() => talkToLead(item)}>
								Talk to {item.responsibleActor.display}
							</button>
						{/if}
					{:else}
						{#if item.actions.some((action) => action.id === 'open-outcome')}
							<button class="btn small" type="button" onclick={() => openReview(item)}>
								{item.actions.find((action) => action.id === 'open-outcome')?.label ??
									'Open outcome'}
							</button>
						{/if}
						{#if item.responsibleActor && item.actions.some((action) => action.id === 'chat-lead')}
							<button class="btn small" type="button" onclick={() => talkToLead(item)}>
								Talk to {item.responsibleActor.display}
							</button>
						{/if}
					{/if}
				</div>
			</section>

			<footer class="folio-provenance">
				<div>
					<span>Prepared by</span>
					<strong
						>{item.briefAuthor?.display ??
							item.responsibleActor?.display ??
							'Source record'}</strong
					>
					{#if item.briefedAt}<time>{when(item.briefedAt)}</time>{/if}
				</div>
				<span class="brief-state">{item.briefStatus.replaceAll('-', ' ')}</span>
			</footer>

			<details class="folio-evidence">
				<summary>
					<span>Evidence and source detail</span>
					<small>{item.evidence.length} item{item.evidence.length === 1 ? '' : 's'}</small>
				</summary>
				<div class="folio-evidence-body">
					{#each item.evidence as evidence (`${evidence.kind}:${evidence.label}`)}
						{#if evidence.content}
							<div class="evidence-entry">
								<div class="evidence-label mono">{evidence.label}</div>
								<blockquote class="ib-quote">{evidence.content}</blockquote>
							</div>
						{:else if evidence.uri}
							<a class="evidence-link" href={evidence.uri} target="_blank" rel="noreferrer">
								{evidence.label} <span aria-hidden="true">↗</span>
							</a>
						{/if}
					{/each}
					<div class="source-ref mono">
						SOURCE {item.source.kind} / {item.source.reference} · {item.canContinue
							? 'work may continue'
							: 'blocking'}
					</div>
				</div>
			</details>
		</article>
	</div>
{/snippet}

{#snippet continuationDetail(continuation: DecisionContinuation)}
	<div class="continuation-pane">
		<div class="continuation-frame">
			<a class="continuation-back" href={baseHref}>
				<span aria-hidden="true">←</span> Back to company floor
			</a>
			<article class="continuation-folio">
				<header>
					<div class="continuation-folio-meta">
						<span title="The owner decision was recorded">
							<SemanticMark meaning="success" size="small" />
							Decision observed
						</span>
						<time>{when(continuation.observedAt)}</time>
					</div>
					<h1>{workTitle(continuation.workId, continuation.title)}</h1>
				</header>
				<dl>
					<div>
						<dt>Recorded decision</dt>
						<dd>{continuation.recordedDecision}</dd>
					</div>
					<div>
						<dt>What it unlocked</dt>
						<dd>{continuation.whatItUnlocked}</dd>
					</div>
					<div>
						<dt>Current observed state</dt>
						<dd>{continuation.currentState}</dd>
					</div>
					<div>
						<dt>Observed outcome</dt>
						<dd>{continuation.observedOutcome}</dd>
					</div>
				</dl>
				<footer>
					<div>
						<span>Responsible now</span>
						<strong>{continuation.responsibleActor?.display ?? 'No further owner'}</strong>
					</div>
					<a class="btn small" href={`/${companyId}/work/${continuation.workId}`}>Inspect Work</a>
				</footer>
			</article>
		</div>
	</div>
{/snippet}

<style>
	.continuation-pane {
		width: 100%;
		min-height: 100%;
		display: grid;
		place-items: center;
		padding: clamp(24px, 4vw, 56px);
	}
	.continuation-frame {
		width: min(720px, 100%);
		display: grid;
		gap: 12px;
	}
	.continuation-back {
		justify-self: start;
		display: inline-flex;
		align-items: center;
		gap: 7px;
		padding: 7px 9px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		font: 600 var(--t-label) var(--font-mono);
		text-decoration: none;
	}
	.continuation-back:hover {
		background: var(--surface-alt);
		color: var(--ink);
	}
	.continuation-back:focus-visible {
		outline: 2px solid var(--state-success);
		outline-offset: 2px;
	}
	.continuation-folio {
		width: 100%;
		border: 1px solid color-mix(in srgb, var(--state-success) 24%, var(--border-strong));
		border-radius: var(--radius-lg);
		background: var(--surface-pane);
		box-shadow:
			var(--bevel),
			0 18px 38px rgba(43, 51, 66, 0.1);
		overflow: hidden;
	}
	.continuation-folio header {
		display: grid;
		gap: 10px;
		padding: clamp(24px, 4vw, 38px);
		background: linear-gradient(135deg, var(--state-success-soft), rgba(255, 255, 255, 0.35));
	}
	.continuation-folio-meta,
	.continuation-folio dt,
	.continuation-folio footer span {
		font: 600 var(--t-label) var(--font-mono);
		letter-spacing: var(--track-label);
		text-transform: uppercase;
		color: var(--text-tertiary);
	}
	.continuation-folio-meta,
	.continuation-folio-meta > span {
		display: flex;
		align-items: center;
	}
	.continuation-folio-meta {
		justify-content: space-between;
		gap: 16px;
	}
	.continuation-folio-meta > span {
		gap: 6px;
	}
	.continuation-folio h1 {
		margin: 0;
		font-size: var(--t-hero);
		line-height: 1.08;
		letter-spacing: -0.035em;
	}
	.continuation-folio time {
		font: var(--t-label) var(--font-mono);
		color: var(--text-secondary);
	}
	.continuation-folio dl {
		margin: 0;
	}
	.continuation-folio dl > div {
		display: grid;
		grid-template-columns: 150px minmax(0, 1fr);
		gap: 18px;
		padding: 17px clamp(24px, 4vw, 38px);
		border-top: 1px solid var(--border);
	}
	.continuation-folio dd {
		margin: 0;
		line-height: 1.5;
		color: var(--ink);
	}
	.continuation-folio footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 18px;
		padding: 14px clamp(24px, 4vw, 38px);
		border-top: 1px solid var(--border);
		background: var(--surface-alt);
	}
	.continuation-folio footer > div {
		display: grid;
		gap: 4px;
	}
	.review-canvas {
		width: 100%;
		height: 100%;
		min-width: 0;
		min-height: 0;
		display: flex;
		overflow: hidden;
		background: #fff;
	}
	.review-canvas iframe {
		width: 100%;
		height: 100%;
		border: 0;
		background: #fff;
	}
	.review-unavailable {
		width: 100%;
		height: 100%;
		display: grid;
		place-content: center;
		gap: 8px;
		padding: 40px;
		background: #fff;
		color: var(--ink);
	}
	.review-unavailable .mono {
		color: var(--intent-feedback);
		letter-spacing: 0.09em;
	}
	.review-unavailable h1,
	.review-unavailable p {
		max-width: 560px;
		margin: 0;
	}
	.review-unavailable p {
		color: var(--text-secondary);
	}
	.evidence-label,
	.source-ref {
		font-size: var(--t-body);
		letter-spacing: 0.09em;
		color: var(--text-tertiary);
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
	.owner-folio {
		--folio-tone: var(--surface-attention);
		position: relative;
		container-type: inline-size;
		width: min(820px, calc(100% - 48px));
		margin: clamp(24px, 5vh, 58px) auto;
		overflow: hidden;
		border: 1px solid color-mix(in srgb, var(--folio-tone) 28%, var(--border-strong));
		border-radius: var(--radius-lg);
		background:
			linear-gradient(145deg, color-mix(in srgb, var(--folio-tone) 5%, white), transparent 42%),
			var(--surface-pane);
		box-shadow:
			var(--bevel),
			0 2px 0 color-mix(in srgb, var(--folio-tone) 10%, transparent),
			0 20px 44px rgba(43, 51, 66, 0.13);
	}
	.owner-folio.category-review {
		--folio-tone: var(--intent-feedback);
	}
	.owner-folio.category-approval,
	.owner-folio.category-human_step {
		--folio-tone: var(--intent-authority);
	}
	.owner-folio.category-blocker {
		--folio-tone: var(--state-danger);
	}
	.owner-folio.category-opportunity {
		--folio-tone: var(--state-success);
	}
	.folio-edge {
		position: absolute;
		z-index: 1;
		inset: 0 auto 0 0;
		width: 5px;
		background: linear-gradient(
			180deg,
			color-mix(in srgb, var(--folio-tone) 72%, white),
			var(--folio-tone) 54%,
			color-mix(in srgb, var(--folio-tone) 68%, var(--ink))
		);
		box-shadow: 2px 0 8px color-mix(in srgb, var(--folio-tone) 18%, transparent);
	}
	.folio-opening {
		padding: clamp(26px, 4vw, 42px) clamp(26px, 4vw, 46px) clamp(22px, 3vw, 32px);
	}
	.folio-meta,
	.folio-provenance,
	.folio-move,
	.folio-recommendation {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 18px;
	}
	.folio-meta {
		align-items: center;
		margin-bottom: 15px;
		font: 600 var(--t-label) var(--font-mono);
		letter-spacing: var(--track-label);
		text-transform: uppercase;
		color: var(--folio-tone);
	}
	.folio-meta strong {
		padding: 4px 7px;
		border: 1px solid color-mix(in srgb, var(--folio-tone) 25%, var(--border));
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--folio-tone) 7%, white);
		font-weight: 500;
	}
	.folio-opening h1 {
		max-width: 720px;
		margin: 0;
		font-size: var(--t-hero);
		font-weight: 600;
		line-height: 1.08;
		letter-spacing: -0.038em;
		text-wrap: balance;
	}
	.folio-situation {
		max-width: 700px;
		margin: 18px 0 0;
		font-size: var(--t-head);
		line-height: 1.45;
		color: var(--text-secondary);
	}
	.folio-impact {
		max-width: 700px;
		margin: 12px 0 0;
		font-size: var(--t-body);
		line-height: 1.58;
		color: var(--ink);
	}
	.folio-uncertainty {
		max-width: 700px;
		margin: 15px 0 0;
		padding: 10px 12px;
		border-left: 3px solid var(--intent-authority);
		background: var(--intent-authority-soft);
		font-size: var(--t-body);
		line-height: 1.5;
		color: var(--text-secondary);
	}
	.folio-recommendation {
		justify-content: flex-start;
		padding: 19px clamp(26px, 4vw, 46px);
		border-block: 1px solid color-mix(in srgb, var(--intent-feedback) 16%, var(--border));
		background: linear-gradient(
			100deg,
			color-mix(in srgb, var(--intent-feedback-soft) 82%, white),
			color-mix(in srgb, var(--intent-feedback-soft) 34%, white)
		);
	}
	.folio-recommendation > span,
	.folio-move-copy > span,
	.folio-provenance span {
		flex: 0 0 118px;
		font: 600 var(--t-label) var(--font-mono);
		letter-spacing: var(--track-label);
		text-transform: uppercase;
		color: var(--text-tertiary);
	}
	.folio-recommendation > span {
		color: var(--intent-feedback);
	}
	.folio-recommendation p {
		max-width: 590px;
		margin: 0;
		font-size: var(--t-head);
		font-weight: 500;
		line-height: 1.45;
		color: var(--ink);
	}
	.folio-move {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(220px, 248px);
		align-items: start;
		padding: 22px clamp(26px, 4vw, 46px);
		background: rgba(255, 255, 255, 0.48);
	}
	.folio-move-copy {
		min-width: 0;
		max-width: 520px;
	}
	.folio-move-copy :global(p) {
		margin: 6px 0 0;
		font-size: var(--t-head);
		font-weight: 600;
		line-height: 1.4;
	}
	.folio-move-copy small {
		display: block;
		margin-top: 9px;
		font-size: var(--t-body);
		line-height: 1.45;
		color: var(--text-secondary);
	}
	.owner-folio .nc-actions {
		width: 100%;
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: 8px;
		margin: 0;
	}
	.owner-folio .nc-actions > :is(button, form),
	.owner-folio .nc-actions > form > button,
	.owner-folio .nc-actions > form > :global(.hold-approve) {
		width: 100%;
	}
	.decision-response {
		display: grid;
		gap: 7px;
	}
	.decision-response input {
		width: 100%;
		min-width: 0;
		padding: 7px 9px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--ink);
		font: var(--t-body) var(--font-ui);
		box-shadow: var(--control-depth-pressed);
	}
	.decision-response input:focus-visible {
		outline: 3px solid color-mix(in srgb, var(--surface-attention) 28%, transparent);
		outline-offset: 2px;
	}
	.folio-provenance {
		align-items: center;
		padding: 11px clamp(26px, 4vw, 46px);
		border-top: 1px solid var(--border);
		background: color-mix(in srgb, var(--surface-alt) 72%, white);
	}
	.folio-provenance > div {
		display: flex;
		align-items: baseline;
		gap: 8px;
	}
	.folio-provenance strong,
	.folio-provenance time,
	.brief-state {
		font-size: var(--t-label);
	}
	.folio-provenance time {
		color: var(--text-tertiary);
	}
	.folio-provenance .brief-state {
		flex: none;
		color: var(--folio-tone);
	}
	.folio-evidence {
		border-top: 1px solid var(--border);
		background: color-mix(in srgb, var(--surface-alt) 62%, white);
	}
	.folio-evidence summary {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 13px clamp(26px, 4vw, 46px);
		cursor: pointer;
		font: 500 var(--t-body) var(--font-mono);
		color: var(--text-secondary);
	}
	.folio-evidence summary:hover {
		background: rgba(255, 255, 255, 0.5);
	}
	.folio-evidence summary:focus-visible {
		outline: 3px solid color-mix(in srgb, var(--folio-tone) 28%, transparent);
		outline-offset: -3px;
	}
	.folio-evidence summary small {
		font: var(--t-label) var(--font-mono);
		color: var(--text-tertiary);
	}
	.folio-evidence-body {
		padding: 2px clamp(26px, 4vw, 46px) 22px;
		border-top: 1px solid var(--border);
	}
	.evidence-entry {
		margin-top: 15px;
	}
	.evidence-link {
		display: block;
		margin-top: 10px;
		padding: 10px 12px;
		border: 1px solid var(--border-strong);
		color: var(--ink);
		text-decoration: none;
	}
	.source-ref {
		margin-top: 14px;
		overflow-wrap: anywhere;
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
		grid-template-columns: minmax(0, 1fr) minmax(238px, 274px);
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
		font: var(--t-label) var(--font-mono);
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
		background: white;
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
	.outcome-offline p {
		max-width: 620px;
		margin: 0;
		color: var(--text-secondary);
		line-height: 1.55;
	}
	@container (max-width: 700px) {
		.folio-move {
			grid-template-columns: minmax(0, 1fr);
		}
	}
	@media (max-width: 1040px) {
		.browser-workspace {
			grid-template-columns: minmax(0, 1fr) 224px;
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
		.owner-folio {
			width: calc(100% - 24px);
			margin-block: 12px;
		}
		.folio-recommendation,
		.folio-provenance,
		.folio-provenance > div {
			flex-direction: column;
		}
		.folio-recommendation > span,
		.folio-move-copy > span,
		.folio-provenance span {
			flex-basis: auto;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.live-mark.owner {
			box-shadow: none;
		}
	}
</style>
