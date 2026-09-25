<script lang="ts">
	import { resizePane } from '$lib/actions/resize-pane';
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import AttentionDocument from '$lib/components/AttentionDocument.svelte';
	import DesktopViewport from '$lib/components/DesktopViewport.svelte';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import Composer from '$lib/primitives/Composer.svelte';
	import ConversationMessage from '$lib/primitives/ConversationMessage.svelte';
	import AttentionCard from '$lib/components/AttentionCard.svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import ConversationTurnDock from '$lib/primitives/ConversationTurnDock.svelte';
	import CompanyOffice from '$lib/office/CompanyOffice.svelte';
	import type { AttentionItem } from '$lib/model/view';
	import { attentionQuery, companiesQuery, conversationQuery } from '$lib/model/queries.svelte';
	import { startFixHref, startLinkLabel } from '$lib/model/company-start';
	import { browserTabClientId } from '$lib/model/browserTab';
	import { getBrowserStatus } from '$lib/model/company';
	import { browserControl, issueDesktopTicket, issueReviewTicket } from '$lib/model/attention';

	const companyId = $derived(page.params.companyId ?? 'aris');

	/* The shared source, not a second poller. `loaded` is the distinction this
	 * surface was missing: before it existed, `items` was `[]` both when the
	 * queue was genuinely clear and when nobody had asked yet, so first paint
	 * asserted "Nothing needs your judgement" and took it back a moment later. */
	const source = attentionQuery(() => companyId);
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

	function officeReviewExtension(uri: string | undefined): string | undefined {
		return uri?.split(/[?#]/, 1)[0]?.match(/\.(doc|docx|xls|xlsx|ppt|pptx)$/i)?.[1];
	}

	const items = $derived(view?.items ?? []);
	const graph = $derived(view?.workGraph ?? null);
	const selectedItemId = $derived(page.url.searchParams.get('item'));
	const companyCatalog = companiesQuery();
	/* A company that cannot start is not clear, whatever the queue says: say
	 * the blocker in the portfolio's words and link straight to its fix. */
	const startBlocker = $derived.by(() => {
		const reason = companyCatalog.view.find(
			(company) => company.id === companyId
		)?.unstartable_reason;
		return reason ? startLinkLabel(reason) : '';
	});
	const queueClear = $derived(loaded && items.length === 0);
	const showClear = $derived(queueClear && !startBlocker);
	/* The queue collapses and returns with motion, but its first known state
	 * is placed at once: animating a page into the layout it loaded with is
	 * only movement the owner has to wait out. */
	let paneMotion = $state(false);
	$effect(() => {
		if (!loaded || paneMotion) return;
		const frame = requestAnimationFrame(() =>
			requestAnimationFrame(() => {
				paneMotion = true;
			})
		);
		return () => cancelAnimationFrame(frame);
	});
	const selectedItem = $derived(
		items.find((item) => item.id === selectedItemId) ?? (selectedItemId ? null : (items[0] ?? null))
	);
	const focusedReviewId = $derived(page.url.searchParams.get('review'));
	const focusedReview = $derived(
		items.find((item) => item.id === focusedReviewId && item.category === 'review') ?? null
	);
	const reviewEvidence = $derived(focusedReview?.evidence ?? []);
	const DOCUMENT_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
	function sameCompanyDocumentHref(content: string, company: string): string {
		const canonicalPath = `/${encodeURIComponent(company)}/work/documents`;
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
	const visibleConversationMessages = $derived(leadConversation?.messages ?? []);
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
		if (!discussionOnly && !item.runtimeAttach) {
			// The source can withdraw a prepared step while this URL is open.
			// Return to its current card rather than showing an unrelated desktop.
			void closePreparedComputer(item);
			return;
		}
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
				human_step: 'Your participation',
				collaboration: 'Work together',
				conversation: 'Needs you'
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
			if (!silent) error = cause instanceof Error ? cause.message : 'Control is held elsewhere.';
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
		void browserControl(companyId, 'heartbeat', clientId, controlLeaseId)
			.catch((cause) => {
				controller = 'observer';
				desktopUrl = observedDesktopUrl();
				error = cause instanceof Error ? cause.message : 'Desktop control expired.';
			})
			.finally(() => (activityRenewing = false));
	}

	async function closePreparedComputer(item: AttentionItem | null = focusItem) {
		if (controller === 'owner') await returnControl();
		focusItem = null;
		desktopUrl = '';
		focusAttachKey = '';
		await goto(item ? itemHref(item.id) : baseHref, { replaceState: true });
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
				<InfoTip
					text="The exact candidate selected by Staff and inspected by the accountable lead."
				/>
			</header>
			<div class="review-outcome-stage">
				{#if focusedReview.reviewTarget?.kind === 'runtime-text' && focusedReview.reviewTarget.content}
					<article class="review-document">
						<Markdown text={focusedReview.reviewTarget.content} />
					</article>
				{:else if officeReviewExtension(focusedReview.reviewTarget?.uri) && reviewUrl}
					<div class="review-download">
						<div class="review-download-kind">
							{officeReviewExtension(focusedReview.reviewTarget?.uri)?.toUpperCase()}
						</div>
						<h1>{focusedReview.reviewTarget?.label ?? focusedReview.title}</h1>
						<p>
							This Office file is ready to download. Open it in Word, Excel or PowerPoint to review
							it.
						</p>
						<a href={reviewUrl} target="_blank" rel="noopener noreferrer">Download file</a>
						<small>Restless can’t preview or edit Office files in this review.</small>
					</div>
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
					onclick={() => closePreparedComputer()}
					>Leave {focusedComputerId ? 'computer' : 'discussion'}</button
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
		class:queue-clear={queueClear || !loaded}
		class:pane-motion={paneMotion}
		use:resizePane={{
			key: `${companyId}:attention`,
			label: 'Resize attention panes',
			target: '.attention-index',
			variable: '--attention-index-w',
			min: 180,
			minOther: 280,
			defaultSize: 280,
			enabled: !queueClear
		}}
	>
		{#if error}<div class="cockpit-error attention-error">{error}</div>{/if}
		<aside class="cockpit-pane attention-index" aria-hidden={queueClear} inert={queueClear}>
			<div class="attention-index-scroll">
				{#if startBlocker && !queueClear}
					<a class="attention-start-blocker inline" href={startFixHref(companyId)}>
						<span class="attention-start-glyph" aria-hidden="true">
							<MatrixGlyph rows={GLYPHS.alert} size={7} />
						</span>
						<span class="attention-start-copy">
							<strong>Can’t start yet</strong>
							<span>{startBlocker}</span>
						</span>
						<span class="attention-start-go" aria-hidden="true">→</span>
					</a>
				{/if}
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
								<span>{item.preparing ? 'Preparing:' : 'Needs you:'}</span>
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
			{#if queueClear && startBlocker}
				<a class="attention-start-blocker" href={startFixHref(companyId)}>
					<span class="attention-start-glyph" aria-hidden="true">
						<MatrixGlyph rows={GLYPHS.alert} size={7} />
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
				title={showClear ? 'No owner action is required. Check again now.' : undefined}
				onclick={() => void refresh()}
			>
				<span class="attention-clear-glyph" aria-hidden="true">
					<MatrixGlyph rows={GLYPHS.check} size={7} />
				</span>
				<span>All clear</span>
			</button>
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
					{@render attentionDetail(selectedItem)}
				{/if}
			{:else if !loaded}
				<!-- Deliberately nothing until the source answers. An empty pane for
				     one round trip reads as loading; the zero-state hero reads as a
				     verdict, and it was the wrong one about half a second later. -->
			{:else}
				<CompanyOffice
					{companyId}
					{graph}
					sourceHealth={view?.sourceHealth ?? {}}
					quietSignal={!!startBlocker}
				/>
			{/if}
		</section>
	</div>
{/if}

{#snippet attentionDetail(item: AttentionItem)}
	{#if item.source.kind === 'conversation_owner_need'}
		<article class="conversation-request cockpit-pane">
			<h1>{item.title}</h1>
			<div class="request-message"><Markdown text={item.whatHappened} /></div>
			<p class="request-need">
				<strong>{item.preparing ? 'Preparing' : 'Needs you'}</strong>
				{item.requestedAction}
			</p>
			<a
				class="btn small primary"
				href={item.actions.find((action) => action.id === 'continue-conversation')?.href}
				>Reply →</a
			>
		</article>
	{:else}
		<div class="inbox-pane">
			<article class="owner-folio category-{item.category}">
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
					<div class="folio-context-copy">
						<p>{item.whatHappened}</p>
						<p>{item.whyItMatters}</p>
					</div>
					{#if item.uncertainty}
						<p class="folio-uncertainty"><strong>Uncertain:</strong> {item.uncertainty}</p>
					{/if}
				</header>

				{#if item.recommendation.trim() !== item.whatHappened.trim() && item.recommendation.trim() !== item.whyItMatters.trim()}
					<section class="folio-recommendation" aria-label="Recommendation">
						<strong>Recommended</strong>
						<Markdown text={item.recommendation} />
					</section>
				{/if}

				{#key `${companyId}:${item.id}`}<AttentionCard
						{companyId}
						{item}
						showTitle={false}
						embedded={true}
						onopenDocument={async () => {
							const result = await source.reload();
							if (result.error) throw result.error;
						}}
					/>{/key}

				<details class="folio-details">
					<summary title="Prepared by, supporting evidence, and source references">
						<span class="evidence-chevron" aria-hidden="true">›</span>
						<span>Details</span>
						{#if item.evidence.length}<small
								>· {item.evidence.length} item{item.evidence.length === 1 ? '' : 's'}</small
							>{/if}
					</summary>
					<div class="folio-evidence-body">
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
	{/if}
{/snippet}

<style>
	.conversation-request {
		margin: 16px;
		padding: 24px;
		min-width: 0;
	}
	.conversation-request h1 {
		margin: 0 0 20px;
		font-size: var(--t-title);
	}
	.request-message {
		max-width: 72ch;
		overflow-wrap: anywhere;
	}
	.request-need {
		margin: 20px 0;
		padding-top: 16px;
		border-top: 1px solid var(--border);
		overflow-wrap: anywhere;
	}
	.request-need strong {
		color: var(--intent-authority);
		margin-right: 10px;
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
		font: var(--t-label) var(--font-mono);
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
	.review-download small {
		display: block;
		color: var(--text-tertiary);
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
		container-type: inline-size;
		width: min(760px, calc(100% - 40px));
		margin: 20px auto;
		padding: clamp(20px, 3vw, 32px);
		background: var(--surface-pane);
	}
	.folio-opening {
		padding: 0;
	}
	.folio-heading {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: var(--space-4);
	}
	.folio-context {
		display: flex;
		align-items: center;
		justify-content: flex-start;
		gap: var(--space-2);
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.folio-context time {
		max-width: 18ch;
	}
	.folio-heading h1 {
		max-width: 760px;
		margin: 0;
		font-size: var(--t-title);
		font-weight: 600;
		line-height: 1.16;
		letter-spacing: -0.03em;
		text-wrap: balance;
	}
	.folio-context-copy {
		max-width: 72ch;
		margin-top: var(--space-4);
		color: var(--text-secondary);
		font-size: var(--t-head);
		line-height: 1.5;
	}
	.folio-context-copy p {
		margin: 0;
	}
	.folio-context-copy p + p {
		margin-top: var(--space-2);
	}
	.folio-uncertainty {
		margin: var(--space-3) 0 0;
		font-size: var(--t-body);
		line-height: 1.45;
		color: var(--text-secondary);
	}
	.folio-uncertainty strong {
		color: var(--intent-authority);
		font-weight: 600;
	}
	.folio-recommendation {
		margin: var(--space-4) 0;
		padding: var(--space-3) 0;
		border-block: 1px solid var(--border);
		color: var(--ink);
	}
	.folio-recommendation > strong {
		color: var(--intent-feedback);
		font-size: var(--t-body);
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
	.folio-details {
		margin-top: var(--space-3);
		border-top: 1px solid var(--border);
	}
	.folio-details summary {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		padding: 10px 0;
		cursor: pointer;
		list-style: none;
		font: 500 var(--t-body) var(--font-ui);
		color: var(--text-secondary);
	}
	.folio-details summary::-webkit-details-marker {
		display: none;
	}
	.folio-details summary:hover {
		color: var(--ink);
	}
	.folio-details summary:focus-visible {
		outline: 2px solid var(--intent-feedback);
		outline-offset: 2px;
	}
	.folio-details summary small {
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
	.folio-details[open] .evidence-chevron {
		transform: rotate(90deg);
	}
	.folio-evidence-body {
		padding: 2px 0 12px;
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
		.owner-folio {
			width: calc(100% - 24px);
			margin-block: 12px;
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
