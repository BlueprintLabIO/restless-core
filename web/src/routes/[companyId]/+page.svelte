<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import DesktopViewport from '$lib/components/DesktopViewport.svelte';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import Composer from '$lib/primitives/Composer.svelte';
	import ConversationMessage from '$lib/primitives/ConversationMessage.svelte';
	import HoldApprove from '$lib/primitives/HoldApprove.svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import ConversationTurnDock from '$lib/primitives/ConversationTurnDock.svelte';
	import CompanyOffice from '$lib/office/CompanyOffice.svelte';
	import { mergeAdjacentAgentMessages, type AttentionItem } from '$lib/model/view';
	import { attentionQuery, conversationQuery } from '$lib/model/queries.svelte';
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
	const source = $derived(attentionQuery(companyId));
	const view = $derived(source.view);
	const loaded = $derived(source.status !== 'unknown');

	let error = $state('');
	let acting = $state(false);
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
	let reviewUrl = $state('');
	let reviewError = $state('');
	let reviewRequestKey = $state('');
	let decisionDraft = $state('');
	let focusAttachKey = $state('');

	const items = $derived(view?.items ?? []);
	const graph = $derived(view?.workGraph ?? null);
	const selectedItemId = $derived(page.url.searchParams.get('item'));
	const queueClear = $derived(loaded && items.length === 0);
	const selectedItem = $derived(
		items.find((item) => item.id === selectedItemId) ?? (selectedItemId ? null : (items[0] ?? null))
	);
	const focusedReviewId = $derived(page.url.searchParams.get('review'));
	const focusedReview = $derived(
		items.find((item) => item.id === focusedReviewId && item.category === 'review') ?? null
	);
	const reviewEvidence = $derived(focusedReview?.evidence ?? []);
	const focusedComputerId = $derived(page.url.searchParams.get('computer'));
	const focusedConversationId = $derived(page.url.searchParams.get('conversation'));
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
	const visibleConversationMessages = $derived(
		mergeAdjacentAgentMessages(leadConversation?.messages ?? [])
	);
	const conversationTurn = $derived(leadConversation?.activeTurn ?? null);
	const attachmentHref = (attachment: { uploadId: string }) =>
		`/api/companies/${encodeURIComponent(companyId)}/attachments/${encodeURIComponent(attachment.uploadId)}`;

	$effect(() => {
		const focusId = focusedComputerId ?? focusedConversationId;
		if (!focusId) {
			focusAttachKey = '';
			focusItem = null;
			desktopUrl = '';
			controller = 'observer';
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
		const discussionOnly = !focusedComputerId;
		if (!discussionOnly && !clientId) return;
		const key = discussionOnly
			? `conversation:${item.id}`
			: `computer:${item.id}:${item.runtimeAttach?.generation ?? 'none'}`;
		if (focusAttachKey === key) return;
		focusAttachKey = key;
		if (discussionOnly) {
			focusItem = item;
			controller = 'observer';
			conversationError = '';
			messageDraft = '';
			messageFiles = [];
			desktopUrl = '';
			return;
		}
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

	function partyOf(item: AttentionItem): string {
		return item.source.party ?? '';
	}

	function actionFor(item: AttentionItem, id: string) {
		return item.actions.find((action) => action.id === id);
	}

	function actionHeading(item: AttentionItem): string {
		if (item.category === 'review') return 'Review, then choose';
		if (item.category === 'human_step') return 'Complete this step';
		if (item.category === 'approval') return 'Choose what happens';
		return 'Your decision';
	}

	function holdLabel(label: string | undefined): string {
		if (!label) return 'Hold to approve';
		return `Hold to ${label.charAt(0).toLocaleLowerCase()}${label.slice(1)}`;
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
			if (autoClaimPending) {
				autoClaimPending = false;
				await takeControl(true);
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
			autoClaimPending = true;
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

	async function takeControl(silent = false) {
		if (!focusItem || !clientId) return;
		error = '';
		try {
			await browserControl(companyId, 'take', clientId);
			controller = 'owner';
			desktopUrl = controlledDesktopUrl();
			lastDesktopActivity = Date.now();
			lastLeaseRenewal = Date.now();
		} catch (cause) {
			if (!silent) error = cause instanceof Error ? cause.message : 'Control is held elsewhere.';
		}
	}

	async function returnControl(automatic = false) {
		if (!focusItem || !clientId) return;
		error = '';
		try {
			await browserControl(companyId, 'return', clientId);
			controller = 'observer';
			desktopUrl = observedDesktopUrl();
			lastDesktopActivity = 0;
		} catch (cause) {
			if (!automatic)
				error = cause instanceof Error ? cause.message : 'Control could not be returned.';
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
		void browserControl(companyId, 'heartbeat', clientId)
			.catch((cause) => {
				controller = 'observer';
				desktopUrl = observedDesktopUrl();
				error = cause instanceof Error ? cause.message : 'Desktop control expired.';
			})
			.finally(() => (activityRenewing = false));
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
			await leadConversation.send(body, files, undefined, false, !!conversationTurn);
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
	<section
		class="review-canvas"
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
				<InfoTip
					text="The exact candidate selected by Staff and inspected by the accountable lead."
				/>
			</header>
			<div class="review-outcome-stage">
				{#if focusedReview.reviewTarget?.kind === 'runtime-text' && focusedReview.reviewTarget.content}
					<article class="review-document">
						<Markdown text={focusedReview.reviewTarget.content} />
					</article>
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
										<li>
											{#if evidence.uri}
												<a href={evidence.uri} target="_blank" rel="noreferrer"
													>{evidence.label} <span aria-hidden="true">↗</span></a
												>
											{:else}
												<strong>{evidence.label}</strong>
											{/if}
										</li>
									{/each}
								</ul>
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
					onclick={closePreparedComputer}
					>Leave {focusedComputerId ? 'computer' : 'discussion'}</button
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
							intent={message.intent}
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
								actionLabel={conversationTurn ? 'Queue direction' : 'Send'}
								disabled={sendingMessage}
								placeholder={conversationTurn
									? `Interrupt ${requestingActorName} with new direction…`
									: `Ask or tell ${requestingActorName} anything…`}
								ariaLabel={conversationTurn
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
				<DesktopViewport
					src={desktopUrl}
					title="Live company browser"
					offline={offlineDesktop}
					onload={syncFocusedControl}
					onactivity={desktopActivity}
				/>
			</div>
		</div>
	</div>
{:else}
	<div class="cockpit-screen attention-screen" class:queue-clear={queueClear}>
		{#if error}<div class="cockpit-error attention-error">{error}</div>{/if}
		<aside class="cockpit-pane attention-index" aria-hidden={queueClear} inert={queueClear}>
			<div class="attention-index-scroll">
				<div class="attention-list">
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
							<strong class="attention-item-title">{item.title}</strong>
							<span class="attention-item-action">
								<span>Needs you:</span>
								{item.requestedAction} <span aria-hidden="true">→</span>
							</span>
						</a>
					{:else}
						{#if !loaded}
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
			</div>
		</aside>

		<section class="cockpit-pane attention-focus" class:office-focus={!selectedItem && loaded}>
			<button
				class="attention-clear-control"
				class:visible={queueClear}
				type="button"
				aria-hidden={!queueClear}
				tabindex={queueClear ? 0 : -1}
				title="No owner action is required. Check again now."
				onclick={() => void refresh()}
			>
				<span class="attention-clear-glyph" aria-hidden="true">
					<MatrixGlyph rows={GLYPHS.check} size={7} />
				</span>
				<span>All clear</span>
			</button>
			{#if selectedItem}
				{@render attentionDetail(selectedItem)}
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
				<div class="folio-heading">
					<h1>{item.title}</h1>
					<div class="folio-context">
						<InfoTip
							text={`${attentionKind(item.category)} from ${item.source.plane.replaceAll('_', ' ')}. Supporting source detail is available below.`}
						/>
						{#if item.deadline}<time>Decision needed by {item.deadline}</time>{/if}
					</div>
				</div>
				<div class="folio-facts" aria-label="Decision context">
					<section class="folio-fact">
						<h2>What changed</h2>
						<p>{item.whatHappened}</p>
					</section>
					<section class="folio-fact important">
						<h2>Why it matters</h2>
						<p>{item.whyItMatters}</p>
					</section>
				</div>
				{#if item.uncertainty}
					<div class="folio-uncertainty">
						<strong>What is still uncertain</strong>
						<p>{item.uncertainty}</p>
						<InfoTip text="This unknown could change the recommendation." />
					</div>
				{/if}
			</header>

			<section class="folio-recommendation" aria-label="Recommendation">
				<h2>The company recommends</h2>
				<Markdown text={item.recommendation} />
			</section>

			<section class="folio-move" aria-label="Your next move">
				<div class="folio-move-copy">
					<h2>{actionHeading(item)}</h2>
					<Markdown text={item.requestedAction} />
				</div>
				<div class="nc-actions" aria-label="Available actions">
					{#each item.actions.filter((action) => action.href) as action (action.id)}
						<div class="action-choice primary-choice">
							<a class="btn small primary" href={action.href} target="_blank" rel="noreferrer"
								>{action.label} ↗</a
							>
							{@render actionMeaning(action)}
						</div>
					{/each}
					{#if item.category === 'approval'}
						{@const grantAction = actionFor(item, 'grant')}
						{@const declineAction = actionFor(item, 'decline')}
						{#if grantAction}
							<div class="action-choice primary-choice">
								<form
									onsubmit={(event) => {
										event.preventDefault();
										void decide(item, 'grant');
									}}
								>
									<HoldApprove small label={acting ? 'Working…' : holdLabel(grantAction.label)} />
								</form>
								{@render actionMeaning(grantAction)}
							</div>
						{/if}
						{#if declineAction}
							<div class="action-choice">
								<button
									class="btn small danger"
									type="button"
									onclick={() => decide(item, 'decline')}
									disabled={acting}
								>
									{declineAction.label}
								</button>
								{@render actionMeaning(declineAction)}
							</div>
						{/if}
					{/if}
					{#if item.actions.some((action) => action.id === 'record-decision')}
						{@const recordAction = actionFor(item, 'record-decision')}
						<div class="action-choice primary-choice">
							<form
								class="decision-response"
								onsubmit={(event) => {
									event.preventDefault();
									void recordDecision(item);
								}}
							>
								<label for="owner-decision">Write the decision in plain language</label>
								<input
									id="owner-decision"
									bind:value={decisionDraft}
									placeholder="For example: proceed with option A"
									aria-label="Your decision"
								/>
								<button
									class="btn small primary"
									type="submit"
									disabled={acting || !decisionDraft.trim()}
								>
									{acting ? 'Recording…' : 'Record this decision'}
								</button>
							</form>
							{#if recordAction}{@render actionMeaning(recordAction)}{/if}
						</div>
					{/if}
					{#if item.category === 'review'}
						{@const openAction = actionFor(item, 'open-outcome')}
						<div class="action-choice primary-choice">
							<button class="btn small primary" type="button" onclick={() => openReview(item)}>
								{openAction?.label ?? 'Review outcome'}
							</button>
							{#if openAction}{@render actionMeaning(openAction)}{/if}
						</div>
						<div class="review-choices" aria-label="Choices available after review">
							<strong>After reviewing, choose one:</strong>
							{#each item.actions.filter((action) => action.id === 'accept-review' || action.id === 'request-revision') as action (action.id)}
								<div><span>{action.label}</span><small>{action.nextState}</small></div>
							{/each}
						</div>
						{#if item.responsibleActor}
							{@const chatAction = actionFor(item, 'chat-lead')}
							<div class="action-choice quiet-choice">
								<button class="btn small" type="button" onclick={() => talkToLead(item)}>
									{chatAction?.label ?? `Work through this with ${item.responsibleActor.display}`}
								</button>
								{#if chatAction}{@render actionMeaning(chatAction)}{/if}
							</div>
						{/if}
					{:else}
						{#if item.actions.some((action) => action.id === 'open-outcome')}
							{@const openAction = actionFor(item, 'open-outcome')}
							<div class="action-choice">
								<button class="btn small" type="button" onclick={() => openReview(item)}>
									{openAction?.label ?? 'Open outcome'}
								</button>
								{#if openAction}{@render actionMeaning(openAction)}{/if}
							</div>
						{/if}
						{#if item.responsibleActor && item.actions.some((action) => action.id === 'chat-lead')}
							{@const chatAction = actionFor(item, 'chat-lead')}
							<div class="action-choice quiet-choice">
								<button class="btn small" type="button" onclick={() => talkToLead(item)}>
									{chatAction?.label ?? `Work through this with ${item.responsibleActor.display}`}
								</button>
								{#if chatAction}{@render actionMeaning(chatAction)}{/if}
							</div>
						{/if}
					{/if}
				</div>
				<div class="folio-wait">
					<strong>If you do nothing</strong>
					<p>{item.ifNoAction}</p>
				</div>
			</section>

			<footer class="folio-provenance">
				<div class="folio-credit">
					<span>Prepared by</span>
					<strong
						>{item.briefAuthor?.display ??
							item.responsibleActor?.display ??
							'Source record'}</strong
					>
					{#if item.briefedAt}
						<span class="folio-credit-separator" aria-hidden="true">·</span>
						<time>{when(item.briefedAt)}</time>
					{/if}
				</div>
				<InfoTip
					text={`Brief status: ${item.briefStatus.replaceAll('-', ' ')}. The wording was prepared by the named accountable actor.`}
				/>
			</footer>

			<details class="folio-evidence">
				<summary title="Supporting evidence and source references">
					<span class="evidence-chevron" aria-hidden="true">›</span>
					<span>Evidence</span>
					<small>· {item.evidence.length} item{item.evidence.length === 1 ? '' : 's'}</small>
				</summary>
				<div class="folio-evidence-body">
					{#each item.evidence as evidence, evidenceIndex (`${evidence.kind}:${evidence.label}:${evidenceIndex}`)}
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

{#snippet actionMeaning(action: AttentionItem['actions'][number])}
	<div class="action-meaning">
		<p>{action.consequence}</p>
		<small><span aria-hidden="true">Then:</span> {action.nextState}</small>
	</div>
{/snippet}

<style>
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
		grid-template-columns: minmax(320px, 0.42fr) minmax(0, 1fr);
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
		background: color-mix(in srgb, var(--surface-alt) 82%, white);
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
		font: var(--t-label) var(--font-mono);
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
		background: #fff;
	}
	.review-outcome-stage iframe {
		width: 100%;
		height: 100%;
		border: 0;
		background: #fff;
	}
	.review-document {
		width: min(820px, calc(100% - 48px));
		min-height: calc(100% - 48px);
		margin: 24px auto;
		padding: clamp(28px, 5vw, 58px);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: white;
		box-shadow: 0 18px 48px rgba(43, 51, 66, 0.11);
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
		background: #fff;
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
	.folio-provenance,
	.folio-move {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 18px;
	}
	.folio-heading {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: start;
		gap: var(--space-4);
	}
	.folio-context {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: var(--space-2);
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.folio-context time {
		white-space: nowrap;
	}
	.folio-heading h1 {
		max-width: 720px;
		margin: 0;
		font-size: var(--t-title);
		font-weight: 600;
		line-height: 1.16;
		letter-spacing: -0.03em;
		text-wrap: balance;
	}
	.folio-facts {
		max-width: 700px;
		margin-top: var(--space-5);
		border-block: 1px solid var(--border);
	}
	.folio-fact {
		display: grid;
		grid-template-columns: 124px minmax(0, 1fr);
		gap: var(--space-4);
		padding: 13px 0;
	}
	.folio-fact + .folio-fact {
		border-top: 1px solid var(--border);
	}
	.folio-fact h2 {
		margin: 2px 0 0;
		color: var(--text-tertiary);
		font: 600 var(--t-body) var(--font-ui);
	}
	.folio-fact p {
		margin: 0;
		font-size: var(--t-head);
		font-weight: 400;
		line-height: 1.55;
		color: var(--text-secondary);
	}
	.folio-fact.important p {
		color: var(--ink);
	}
	.folio-uncertainty {
		max-width: 700px;
		display: grid;
		grid-template-columns: 154px minmax(0, 1fr) auto;
		align-items: start;
		gap: var(--space-2);
		margin-top: var(--space-4);
		padding: var(--space-3);
		border-left: 3px solid var(--intent-authority);
		background: var(--intent-authority-soft);
		font-size: var(--t-head);
		line-height: 1.55;
		color: var(--text-secondary);
	}
	.folio-uncertainty strong {
		color: var(--ink);
		font-size: var(--t-body);
		line-height: 1.55;
	}
	.folio-uncertainty p {
		margin: 0;
	}
	.folio-recommendation {
		display: grid;
		gap: var(--space-2);
		padding: var(--space-5) clamp(26px, 4vw, 46px) var(--space-6);
		border-block: 1px solid color-mix(in srgb, var(--intent-feedback) 16%, var(--border));
		background: linear-gradient(
			100deg,
			color-mix(in srgb, var(--intent-feedback-soft) 82%, white),
			color-mix(in srgb, var(--intent-feedback-soft) 34%, white)
		);
	}
	.folio-recommendation h2,
	.folio-move-copy h2 {
		margin: 0;
		font-size: var(--t-head);
		font-weight: 600;
		line-height: 1.4;
	}
	.folio-recommendation h2 {
		color: var(--intent-feedback);
	}
	.folio-recommendation :global(.md),
	.folio-move-copy :global(.md) {
		max-width: 700px;
		font-size: var(--t-head);
		font-weight: 400;
		line-height: 1.55;
		color: var(--ink);
	}
	.folio-recommendation :global(.md strong),
	.folio-move-copy :global(.md strong) {
		font-weight: inherit;
	}
	.folio-recommendation :global(.md h1),
	.folio-recommendation :global(.md h2),
	.folio-recommendation :global(.md h3),
	.folio-recommendation :global(.md h4),
	.folio-recommendation :global(.md h5),
	.folio-recommendation :global(.md h6),
	.folio-move-copy :global(.md h1),
	.folio-move-copy :global(.md h2),
	.folio-move-copy :global(.md h3),
	.folio-move-copy :global(.md h4),
	.folio-move-copy :global(.md h5),
	.folio-move-copy :global(.md h6) {
		font-size: inherit;
		font-weight: 600;
	}
	.folio-recommendation :global(.md p),
	.folio-move-copy :global(.md p) {
		margin: 0;
	}
	.folio-recommendation :global(.md p + p),
	.folio-recommendation :global(.md :is(ul, ol)),
	.folio-move-copy :global(.md p + p),
	.folio-move-copy :global(.md :is(ul, ol)) {
		margin-top: var(--space-2);
	}
	.folio-move {
		display: grid;
		grid-template-areas:
			'intro actions'
			'wait actions';
		grid-template-columns: minmax(220px, 0.72fr) minmax(320px, 1.28fr);
		align-items: start;
		gap: var(--space-4) clamp(24px, 4vw, 44px);
		padding: 24px clamp(26px, 4vw, 46px) 28px;
		background: rgba(255, 255, 255, 0.48);
	}
	.folio-move-copy {
		grid-area: intro;
		min-width: 0;
		max-width: 620px;
	}
	.folio-move-copy :global(.md) {
		margin-top: var(--space-2);
	}
	.folio-wait {
		grid-area: wait;
		margin: 0;
		padding-top: var(--space-3);
		border-top: 1px solid var(--border);
		font-size: var(--t-body);
		font-weight: 400;
		line-height: 1.5;
		color: var(--text-secondary);
	}
	.folio-wait strong {
		display: block;
		margin-bottom: 3px;
		color: var(--ink);
		font-weight: 600;
	}
	.folio-wait p {
		margin: 0;
	}
	.owner-folio .nc-actions {
		grid-area: actions;
		width: 100%;
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: 10px;
		margin: 0;
	}
	.action-choice {
		min-width: 0;
		display: grid;
		gap: 8px;
		padding: 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--surface) 88%, transparent);
		box-shadow: var(--bevel);
	}
	.action-choice.primary-choice {
		border-color: color-mix(in srgb, var(--folio-tone) 34%, var(--border-strong));
		background: color-mix(in srgb, var(--folio-tone) 5%, var(--surface));
	}
	.action-choice.quiet-choice {
		background: color-mix(in srgb, var(--surface-alt) 72%, transparent);
	}
	.action-choice > :is(button, a, form),
	.action-choice > form > button,
	.action-choice > form > :global(.hold-approve) {
		width: 100%;
	}
	.action-meaning {
		display: grid;
		gap: 4px;
	}
	.action-meaning p,
	.action-meaning small {
		margin: 0;
		font-size: var(--t-body);
		font-weight: 400;
		line-height: 1.42;
		color: var(--text-secondary);
	}
	.action-meaning small {
		color: var(--text-tertiary);
	}
	.action-meaning small span {
		color: var(--ink);
		font-weight: 600;
	}
	.review-choices {
		display: grid;
		gap: 7px;
		padding: 10px 12px;
		border-left: 2px solid var(--folio-tone);
		background: var(--surface-alt);
	}
	.review-choices > strong {
		font-size: var(--t-body);
	}
	.review-choices > div {
		display: grid;
		grid-template-columns: 116px minmax(0, 1fr);
		gap: var(--space-2);
	}
	.review-choices span {
		font-size: var(--t-body);
		font-weight: 600;
		color: var(--ink);
	}
	.review-choices small {
		font-size: var(--t-body);
		line-height: 1.4;
		color: var(--text-secondary);
	}
	.decision-response {
		display: grid;
		gap: 7px;
	}
	.decision-response label {
		font-size: var(--t-body);
		font-weight: 600;
		color: var(--ink);
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
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		padding: 11px clamp(26px, 4vw, 46px);
		border-top: 1px solid var(--border);
		background: color-mix(in srgb, var(--surface-alt) 72%, white);
	}
	.folio-credit {
		min-width: 0;
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 0 var(--space-1);
		font-size: var(--t-body);
		font-weight: 400;
		line-height: 1.45;
		color: var(--text-secondary);
	}
	.folio-credit strong {
		font-weight: 500;
		color: var(--ink);
	}
	.folio-credit time {
		white-space: nowrap;
		color: var(--text-tertiary);
	}
	.folio-credit-separator {
		color: var(--border-strong);
	}
	.folio-evidence {
		border-top: 1px solid var(--border);
		background: color-mix(in srgb, var(--surface-alt) 62%, white);
	}
	.folio-evidence summary {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		padding: 11px clamp(26px, 4vw, 46px);
		cursor: pointer;
		list-style: none;
		font: 500 var(--t-body) var(--font-ui);
		color: var(--text-secondary);
	}
	.folio-evidence summary::-webkit-details-marker {
		display: none;
	}
	.folio-evidence summary:hover {
		background: rgba(255, 255, 255, 0.5);
	}
	.folio-evidence summary:focus-visible {
		outline: 3px solid color-mix(in srgb, var(--folio-tone) 28%, transparent);
		outline-offset: -3px;
	}
	.folio-evidence summary small {
		font: inherit;
		font-weight: 400;
		color: var(--text-tertiary);
	}
	.evidence-chevron {
		width: var(--space-3);
		flex: 0 0 var(--space-3);
		font-size: var(--t-head);
		line-height: 1;
		color: var(--text-tertiary);
		transform-origin: center;
		transition: transform 120ms ease;
	}
	.folio-evidence[open] .evidence-chevron {
		transform: rotate(90deg);
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
			grid-template-areas:
				'intro'
				'actions'
				'wait';
			grid-template-columns: minmax(0, 1fr);
		}
		.folio-fact,
		.folio-uncertainty {
			grid-template-columns: minmax(0, 1fr);
			gap: 5px;
		}
		.folio-uncertainty :global(.info-tip) {
			display: none;
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
		.owner-folio {
			width: calc(100% - 24px);
			margin-block: 12px;
		}
		.folio-heading {
			grid-template-columns: 1fr;
		}
		.folio-context {
			justify-content: flex-start;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.evidence-chevron {
			transition: none;
		}
		.live-mark.owner {
			box-shadow: none;
		}
	}
</style>
