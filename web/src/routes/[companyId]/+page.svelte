<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import HoldApprove from '$lib/primitives/HoldApprove.svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import SemanticMark from '$lib/primitives/SemanticMark.svelte';
	import type { AttentionItem } from '$lib/model/view';
	import { attentionSource } from '$lib/model/attentionSource.svelte';
	import {
		approvalAction,
		browserControl,
		getActorConversation,
		issueDesktopTicket,
		issueReviewTicket,
		sendActorMessage
	} from '$lib/model/attention';
	import type { ActorConversation } from '$lib/model/attention';

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
	let conversation = $state<ActorConversation | null>(null);
	let conversationError = $state('');
	let messageStatus = $state('');
	let messageDraft = $state('');
	let sendingMessage = $state(false);
	let clientId = $state('');
	let reviewUrl = $state('');
	let reviewError = $state('');
	let reviewRequestKey = $state('');

	const items = $derived(view?.items ?? []);
	const graph = $derived(view?.workGraph ?? null);
	const activeWork = $derived(graph?.work.filter((work) => work.status === 'active') ?? []);
	const blockedWork = $derived(graph?.work.filter((work) => work.status === 'blocked') ?? []);
	const recentAcceptedWork = $derived(
		(graph?.work ?? [])
			.filter(
				(work) =>
					work.status === 'completed' &&
					(graph?.artifacts.some((artifact) => artifact.work_id === work.id) ?? false)
			)
			.toSorted((a, b) => Date.parse(b.updated_at) - Date.parse(a.updated_at))[0] ?? null
	);
	const degradedSources = $derived(
		Object.entries(view?.sourceHealth ?? {}).filter(([, status]) => status !== 'available')
	);
	const selectedItemId = $derived(page.url.searchParams.get('item'));
	const selectedItem = $derived(
		items.find((item) => item.id === selectedItemId) ?? (selectedItemId ? null : (items[0] ?? null))
	);
	const focusedReviewId = $derived(page.url.searchParams.get('review'));
	const focusedReview = $derived(
		items.find((item) => item.id === focusedReviewId && item.category === 'review') ?? null
	);
	const baseHref = $derived(`/${companyId}`);
	const requestingActor = $derived(
		focusItem?.responsibleActor?.id ?? focusItem?.runtimeAttach?.requestingActor ?? ''
	);
	const requestingActorName = $derived(
		conversation?.actor.display ||
			focusItem?.responsibleActor?.display ||
			focusItem?.runtimeAttach?.requestingActorDisplay ||
			(requestingActor === 'exec' ? 'Exec' : requestingActor || 'Company context')
	);
	const lastConversationMessage = $derived(conversation?.messages.at(-1) ?? null);
	const awaitingLeadReply = $derived(lastConversationMessage?.from_actor === 'owner');

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
		clientId = crypto.randomUUID();
		const heartbeat = window.setInterval(() => {
			if (controller === 'owner') {
				void browserControl(companyId, 'heartbeat', clientId).catch((cause) => {
					controller = 'observer';
					error = cause instanceof Error ? cause.message : 'Browser control lease ended.';
				});
			}
		}, 12_000);
		const conversationRefresh = window.setInterval(() => {
			if (focusItem && requestingActor) void refreshConversation(false);
		}, 4_000);
		return () => {
			window.clearInterval(heartbeat);
			window.clearInterval(conversationRefresh);
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

	async function openReview(item: AttentionItem) {
		if (item.category === 'review') {
			await goto(`${baseHref}?review=${encodeURIComponent(item.id)}`);
			return;
		}
		if (!clientId) return;
		error = '';
		focusItem = item;
		controller = 'observer';
		conversation = null;
		conversationError = '';
		messageStatus = '';
		desktopUrl = '';
		void refreshConversation();
		if (!item.runtimeAttach) return;
		try {
			desktopUrl = await issueDesktopTicket(companyId, item.id, clientId);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The live browser is unavailable.';
		}
	}

	async function takeControl() {
		if (!focusItem || !clientId) return;
		error = '';
		try {
			await browserControl(companyId, 'take', clientId);
			controller = 'owner';
			desktopUrl = `/desktop/${encodeURIComponent(companyId)}/control?client_id=${encodeURIComponent(clientId)}`;
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
			desktopUrl = `/desktop/${encodeURIComponent(companyId)}/vnc.html?autoconnect=1&resize=scale&view_only=1&path=desktop/${encodeURIComponent(companyId)}/websockify`;
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Control could not be returned.';
		}
	}

	async function refreshConversation(showError = true) {
		if (!focusItem || !requestingActor) {
			conversation = null;
			return;
		}
		try {
			const refreshed = await getActorConversation(companyId, requestingActor, focusItem.workId);
			conversation = refreshed;
			if (
				messageStatus.startsWith('Delivered to ') &&
				refreshed.messages.at(-1)?.from_actor !== 'owner'
			) {
				messageStatus = `Reply received from ${requestingActorName}.`;
			}
			conversationError = '';
		} catch (cause) {
			if (showError) {
				conversationError =
					cause instanceof Error ? cause.message : 'The requester conversation is unavailable.';
			}
		}
	}

	async function sendMessage(event: SubmitEvent) {
		event.preventDefault();
		const body = messageDraft.trim();
		if (!body || sendingMessage || !focusItem || !requestingActor) return;
		sendingMessage = true;
		conversationError = '';
		messageStatus = '';
		try {
			await sendActorMessage(companyId, requestingActor, body, focusItem.workId);
			messageDraft = '';
			messageStatus = `Delivered to ${requestingActorName}.`;
			await refreshConversation(false);
		} catch (cause) {
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
					<span class="mono">{view?.company.name ?? companyId} COMPANY COMPUTER</span>
					<strong>{focusItem.title}</strong>
				</div>
			</div>
			<div class="handover-state">
				{#if focusItem.runtimeAttach && desktopUrl}
					<span class="mono"
						>{view?.sourceHealth.browser ?? 'unknown'} · {controller === 'owner'
							? 'YOU CONTROL'
							: 'OBSERVE ONLY'}</span
					>
					{#if controller === 'observer'}
						<button class="btn small primary" type="button" onclick={takeControl}
							>Take control</button
						>
					{:else}
						<button class="btn small" type="button" onclick={returnControl}>Return control</button>
					{/if}
				{:else}
					<span class="mono">PREVIEW OFFLINE · DISCUSSION AVAILABLE</span>
				{/if}
				<button class="btn small" type="button" onclick={() => (focusItem = null)}
					>Back to inbox</button
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
						<span class="mono"
							>RESPONSIBLE LEAD / {conversation?.actor.role ??
								focusItem.responsibleActor?.role ??
								'WORK OWNER'}</span
						>
						<strong>{requestingActorName}</strong>
					</div>
					<span class="controller-badge" class:owner={controller === 'owner' && !!desktopUrl}>
						{desktopUrl
							? controller === 'owner'
								? 'YOU CONTROL'
								: `${requestingActorName} OBSERVES`
							: 'CHAT OPEN'}
					</span>
				</div>

				<div class="handover-thread" aria-live="polite">
					<div class="handover-message requester context-message">
						<span>{requestingActorName} · prepared context</span>
						<p>{focusItem.recommendation}</p>
						<p>{focusItem.requestedAction}</p>
					</div>
					{#each conversation?.messages ?? [] as message (message.id)}
						<div
							class="handover-message"
							class:owner-message={message.from_actor === 'owner'}
							class:requester={message.from_actor !== 'owner'}
						>
							<span
								>{message.from_actor === 'owner' ? 'You' : requestingActorName} · {when(
									message.created_at
								)}</span
							>
							<p>{message.body}</p>
						</div>
					{/each}
				</div>

				{#if requestingActor}
					<div class="handover-footer">
						<form class="handover-composer" onsubmit={sendMessage}>
							<textarea
								bind:value={messageDraft}
								placeholder={`Ask or tell ${requestingActorName} anything…`}
								aria-label={`Message ${requestingActorName}`}
								rows="3"></textarea>
							<div>
								<small>Replies appear in this conversation.</small>
								<button
									class="btn small"
									type="submit"
									disabled={sendingMessage || !messageDraft.trim()}
								>
									{sendingMessage ? 'Sending…' : 'Send for discussion'}
								</button>
							</div>
						</form>
						{#if messageStatus}
							<p class="conversation-status" role="status">{messageStatus}</p>
						{:else if awaitingLeadReply}
							<p class="conversation-status" role="status">
								Your last message is delivered. This page is checking for {requestingActorName}’s
								reply.
							</p>
						{/if}
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
				{#if desktopUrl}
					<iframe
						title="Live company browser"
						src={desktopUrl}
						allow="clipboard-read; clipboard-write"
						referrerpolicy="same-origin"
					></iframe>
				{:else}
					<div class="outcome-offline">
						<span class="mono">LIVE OUTCOME UNAVAILABLE</span>
						<h2>The review and lead conversation are still available.</h2>
						<Markdown text={focusItem.whatHappened} />
						{#each focusItem.evidence.filter((entry) => entry.uri) as evidence}
							<a class="evidence-link" href={evidence.uri} target="_blank" rel="noreferrer"
								>{evidence.label} ↗</a
							>
						{/each}
					</div>
				{/if}
			</div>
		</div>
		<footer class="desktop-foot">
			{desktopUrl
				? controller === 'owner'
					? 'You are the sole input controller.'
					: 'The desktop is observe-only.'
				: 'No live desktop is attached.'}
			Conversation does not complete “{focusItem.title}”.
		</footer>
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
				<span class="over-label">Needs your judgement</span>
			</header>
			<div class="attention-index-scroll">
				<div class="attention-list">
					{#each items as item (item.id)}
						<a
							class="attention-item category-{item.category}"
							class:selected={selectedItem?.id === item.id}
							href={itemHref(item.id)}
							aria-current={selectedItem?.id === item.id ? 'true' : undefined}
						>
							<MatrixGlyph rows={GLYPHS.rules} size={8} />
							<span class="attention-item-copy">
								<strong>{item.title}</strong>
								<small>{item.category} · {item.source.plane}</small>
							</span>
							<time>{when(item.createdAt)}</time>
						</a>
					{:else}
						{#if loaded}
							<div class="attention-list-clear">
								<MatrixGlyph rows={GLYPHS.check} size={10} />
								<span
									><strong>Queue clear</strong><small
										>Exec will bring you the next material decision.</small
									></span
								>
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
			</div>
		</aside>

		<section class="cockpit-pane attention-focus">
			{#if selectedItem}
				{@render attentionDetail(selectedItem)}
			{:else if !loaded}
				<!-- Deliberately nothing until the source answers. An empty pane for
				     one round trip reads as loading; the zero-state hero reads as a
				     verdict, and it was the wrong one about half a second later. -->
			{:else}
				<div class="attention-zero">
					<header class="attention-zero-hero">
						<SemanticMark meaning="success" size="large" />
						<div>
							<h1>Nothing needs your judgement.</h1>
							<p>Exec is continuing inside the authority you already set.</p>
						</div>
					</header>
					<div class="attention-zero-grid">
						<section class="zero-focus">
							<span class="zero-label work">Current focus</span>
							<strong>{activeWork[0]?.title ?? 'No Work is currently in motion'}</strong>
							<p>
								{activeWork.length
									? `${activeWork.length} Work item${activeWork.length === 1 ? '' : 's'} in motion.`
									: 'Exec is ready for the next outcome.'}
							</p>
						</section>
						<section>
							<span class="zero-label success">Recent accepted progress</span>
							<strong>{recentAcceptedWork?.title ?? 'No evidence-backed completion yet'}</strong>
							<p>
								{recentAcceptedWork
									? 'Linked to inspectable evidence.'
									: 'Recorded completions remain in history until evidence is linked.'}
							</p>
						</section>
						<section>
							<span class="zero-label direction">Next expected review</span>
							<strong>When judgement changes the plan</strong>
							<p>
								Routine progress stays in Work. Exec will surface an outcome, decision, or prepared
								last mile here.
							</p>
						</section>
						<section>
							<span class="zero-label authority">Operating status</span>
							<strong>{activeWork.length} in motion · {blockedWork.length} waiting</strong>
							{#if degradedSources.length}
								<p>
									{degradedSources.map(([source, status]) => `${source}: ${status}`).join(' · ')}
								</p>
							{:else}
								<p>All company sources are available.</p>
							{/if}
						</section>
					</div>
				</div>
			{/if}
		</section>
	</div>
{/if}

{#snippet attentionDetail(item: AttentionItem)}
	<div class="inbox-pane">
		<div class="needs-card category-{item.category}">
			<div class="nc-kicker">{item.category} · {item.source.plane}</div>
			<div class="nc-title">{item.title}</div>
			<!-- The same author writes to the rail, the Work page and here; structure
			     should survive on all three. `prepared_state` is the owner's primary
			     reading surface for a decision, so it is the last place to render a
			     wall of text. -->
			<div class="nc-detail"><Markdown text={item.whatHappened} /></div>

			<div class="attention-facts">
				<div>
					<span>Why it matters</span>
					<p>{item.whyItMatters}</p>
				</div>
				<div>
					<span>Recommendation</span>
					<p>{item.recommendation}</p>
				</div>
				<div>
					<span>Your move</span>
					<Markdown text={item.requestedAction} />
				</div>
				<div>
					<span>If you do nothing</span>
					<p>{item.ifNoAction}</p>
				</div>
			</div>

			{#each item.evidence as evidence (`${evidence.kind}:${evidence.label}`)}
				{#if evidence.content}
					<div class="evidence-label mono">{evidence.label}</div>
					<blockquote class="ib-quote">{evidence.content}</blockquote>
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

			<div class="nc-actions">
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
				{#if item.category === 'review'}
					<button class="btn small primary" type="button" onclick={() => openReview(item)}>
						Review outcome
					</button>
					{#if item.responsibleActor}
						<button class="btn small" type="button" onclick={() => openReview(item)}>
							Talk to {item.responsibleActor.display}
						</button>
					{/if}
				{:else if item.runtimeAttach}
					<button class="btn small" type="button" onclick={() => openReview(item)}>
						Open prepared browser
					</button>
				{/if}
			</div>
		</div>
	</div>
{/snippet}

<style>
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
	.attention-facts {
		display: grid;
		gap: 9px;
		margin: 16px 0;
	}
	.attention-facts div {
		display: grid;
		grid-template-columns: 110px 1fr;
		gap: 12px;
		padding-top: 8px;
		border-top: 1px solid var(--border);
	}
	.attention-facts span {
		font: var(--t-body) var(--font-mono);
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: var(--text-tertiary);
	}
	.attention-facts p {
		margin: 0;
		font-size: var(--t-body);
		line-height: 1.5;
		color: var(--text-secondary);
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
		grid-template-rows: auto auto minmax(0, 1fr) auto;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		overflow: hidden;
		background: var(--glass-pane);
	}
	.handover-rail {
		grid-row: 1;
		min-height: 58px;
		padding: 9px 14px;
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
		gap: 2px;
	}
	.handover-context .mono,
	.handover-state .mono {
		font-size: var(--t-label);
		letter-spacing: 0.08em;
		color: var(--text-tertiary);
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
	.handover-conversation-head .mono {
		font-size: var(--t-label);
		letter-spacing: 0.08em;
		color: var(--text-tertiary);
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
	.handover-message {
		width: 100%;
		max-width: none;
		padding: 13px 14px;
		border: 0;
		border-bottom: 1px solid var(--border);
		background: rgba(255, 255, 255, 0.36);
	}
	.handover-message.owner-message {
		background: var(--intent-conversation-soft);
		box-shadow: inset 2px 0 0 color-mix(in srgb, var(--intent-conversation) 55%, transparent);
	}
	.handover-message.context-message {
		box-shadow: inset 2px 0 0 var(--intent-feedback);
	}
	.handover-message > span {
		display: block;
		margin-bottom: 5px;
		font: var(--t-label) var(--font-mono);
		letter-spacing: 0.04em;
		color: var(--text-tertiary);
	}
	.handover-message p {
		margin: 0;
		font-size: var(--t-body);
		line-height: 1.5;
		color: var(--text-secondary);
		white-space: pre-wrap;
	}
	.handover-message p + p {
		margin-top: 7px;
		color: var(--ink);
	}
	.handover-footer {
		border-top: 1px solid var(--border);
	}
	.handover-composer {
		padding: 10px 12px 9px;
	}
	.handover-composer textarea {
		width: 100%;
		resize: vertical;
		max-height: 130px;
		min-height: 56px;
		padding: 8px 9px;
		border: 1px solid var(--border-strong);
		background: var(--surface);
		color: var(--ink);
		font: inherit;
		font-size: var(--t-body);
		line-height: 1.4;
	}
	.handover-composer > div {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		margin-top: 7px;
	}
	.handover-composer small {
		font-size: var(--t-label);
		line-height: 1.35;
		color: var(--text-tertiary);
	}
	.conversation-error {
		margin: 0;
		padding: 0 12px 10px;
		color: var(--danger);
		font-size: var(--t-label);
	}
	.conversation-status {
		margin: 0;
		padding: 0 12px 10px;
		color: var(--text-secondary);
		font-size: var(--t-label);
		line-height: 1.45;
	}
	.desktop-stage {
		grid-column: 1;
		grid-row: 1;
		min-width: 0;
		min-height: 0;
		padding: 10px;
	}
	.desktop-stage iframe {
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
	.desktop-foot {
		grid-row: 4;
		padding: 8px 14px;
		font-size: var(--t-body);
		color: var(--text-tertiary);
		border-top: 1px solid var(--border);
		background: var(--glass-strong);
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
		.attention-facts div {
			grid-template-columns: 1fr;
			gap: 4px;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.live-mark.owner {
			box-shadow: none;
		}
	}
</style>
