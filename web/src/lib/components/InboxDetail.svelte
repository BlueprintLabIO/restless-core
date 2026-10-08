<script lang="ts">
	/* One Inbox item, read as a message from whoever is asking and answered
	 * from one action bar at the bottom.
	 *
	 * A reply or a "Not doing this" goes to the responsible lead as a message in
	 * their conversation (the handoff decision endpoint posts it), so the owner
	 * is told where it went. Approvals keep hold-to-confirm. Context and
	 * evidence are folded away until asked for. */
	import { failureSentence } from '$lib/model/failure';
	import { useQueryClient } from '@tanstack/svelte-query';
	import type { AttentionItem } from '$lib/model/view';
	import {
		approvalAction,
		completeHandoffHumanStep,
		resolveHandoffDecision
	} from '$lib/model/attention';
	import { refreshAttention, removeConfirmedAttention } from '$lib/model/queries.svelte';
	import {
		attentionAsker,
		attentionKindLabel,
		attentionTitle
	} from '$lib/model/attention-presentation';
	import HoldApprove from '$lib/ui/controls/HoldApprove.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import { Fold, Notice } from '$lib/ui/page';
	import { formatMoment } from '$lib/ui/time';
	import { fetchAppRequests, type AppRequest } from '$lib/model/app-requests';
	import { dismissAttention } from '$lib/model/attention';
	import { initials } from '$lib/model/initials';

	let {
		companyId,
		item,
		declining: startDeclining = false,
		onopenDocument,
		onresolved
	}: {
		companyId: string;
		item: AttentionItem;
		/** Open straight into "Not doing this", as the list's quick action does. */
		declining?: boolean;
		onopenDocument?: () => Promise<unknown>;
		/** Where the answer went, for the page to confirm after the item leaves. */
		onresolved?: (sent: { text: string; href?: string }) => void;
	} = $props();

	const client = useQueryClient();
	const asker = $derived(attentionAsker(item));
	const askerId = $derived(item.responsibleActor?.id ?? item.briefAuthor?.id ?? 'exec');
	const conversationHref = $derived(
		`/${encodeURIComponent(companyId)}/people?person=${encodeURIComponent(askerId)}&view=people`
	);
	const base = $derived(`/${encodeURIComponent(companyId)}?item=${encodeURIComponent(item.id)}`);
	const action = (id: string) => item.actions.find((candidate) => candidate.id === id);
	const grant = $derived(action('grant'));
	/* Permission at first use: approve this call and let the tool act from now on. */
	const grantAlways = $derived(action('grant-always'));
	const decline = $derived(action('decline'));
	const approveMandate = $derived(action('approve-email-mandate'));
	const declineMandate = $derived(action('decline-email-mandate'));
	const record = $derived(action('record-decision'));
	const open = $derived(action('open-outcome'));
	const review = $derived(item.actions.some((candidate) => candidate.id === 'accept-review'));
	const humanStep = $derived(action('complete-human-step'));
	const discuss = $derived(item.actions.some((candidate) => candidate.id === 'chat-lead'));
	const documentRequest = $derived(
		item.source.kind === 'document_collaboration' || item.source.kind === 'document_review'
	);
	const navigation = $derived(
		item.actions.filter((candidate) => candidate.href && candidate.role !== 'conversation')
	);
	/* An app request (Sprint 63): a sign-in handoff that names the app to add.
	 * Adding and allowing it is the resume condition, so there is no "Mark done". */
	let appRequest = $state<AppRequest | null>(null);
	$effect(() => {
		appRequest = null;
		if (item.source.kind !== 'owner_handoff') return;
		const reference = item.source.reference;
		let current = true;
		void fetchAppRequests(companyId)
			.then((requests) => {
				if (current) appRequest = requests.find((row) => row.handoff_id === reference) ?? null;
			})
			.catch(() => {});
		return () => {
			current = false;
		};
	});
	/* Older daemons put a verification link only in the instructions. */
	const instructionLink = $derived.by(() => {
		if (item.preparing || item.category !== 'human_step' || item.actions.some((a) => a.href))
			return undefined;
		const match = item.requestedAction.match(/https?:\/\/[^\s<>"']+/);
		if (!match) return undefined;
		try {
			const url = new URL(match[0].replace(/[).,;!?]+$/, ''));
			if (['localhost', '127.0.0.1', '[::1]'].includes(url.hostname)) return undefined;
			return { href: url.href, label: `Open ${url.hostname}` };
		} catch {
			return undefined;
		}
	});
	const showRecommendation = $derived(
		!!item.recommendation.trim() &&
			item.recommendation.trim() !== item.whatHappened.trim() &&
			item.recommendation.trim() !== item.whyItMatters.trim() &&
			item.recommendation.trim() !== item.requestedAction.trim()
	);
	const fields = $derived(item.requestFields ?? []);
	const optionsFor = (field: { type: string; options?: string[] }) =>
		field.type === 'boolean' ? ['Yes', 'No'] : (field.options ?? []);

	let acting = $state(false);
	let error = $state('');
	let reply = $state('');
	let reason = $state('');
	let declining = $state(false);
	let fieldValues = $state<Record<string, string>>({});
	let attempt = $state(0);
	$effect(() => {
		declining = startDeclining && !!(record || humanStep);
	});

	const replyReady = $derived(
		fields.length
			? fields.every((field) => !field.required || (fieldValues[field.id] ?? '').trim())
			: !!reply.trim()
	);
	const answer = () =>
		fields.length
			? [
					...fields.map((field) => `${field.label}: ${fieldValues[field.id] ?? ''}`),
					...(reply.trim() ? [reply.trim()] : [])
				].join('\n')
			: reply.trim();

	async function run(work: () => Promise<unknown>, sent: string, fallback: string) {
		if (acting) return;
		acting = true;
		error = '';
		try {
			await work();
			onresolved?.({ text: sent, href: conversationHref });
		} catch (cause) {
			error = failureSentence(cause, fallback);
		} finally {
			acting = false;
			attempt += 1;
		}
	}

	/* Not needed: off the Inbox until it is raised again, and whoever asked is told so in their
	 * conversation. It grants and declines nothing. */
	const dismiss = () =>
		run(
			async () => {
				await dismissAttention(companyId, item.id);
				await refreshAttention(client, companyId);
			},
			'Set aside. It is under Set aside in your Inbox if you want it back.',
			'That was not dismissed. Try again.'
		);
	const sendReply = () =>
		replyReady &&
		run(
			async () => {
				await resolveHandoffDecision(companyId, item.source.reference, answer());
				await refreshAttention(client, companyId);
			},
			`Sent to ${asker}`,
			'Your answer was not sent. Try again.'
		);
	const sendDecline = () =>
		reason.trim() &&
		run(
			async () => {
				await resolveHandoffDecision(companyId, item.source.reference, reason.trim(), true);
				await refreshAttention(client, companyId);
			},
			`Declined. ${asker} has your reason.`,
			'That was not recorded. Try again.'
		);
	const markDone = () =>
		run(
			async () => {
				await completeHandoffHumanStep(companyId, item.source.reference);
				await refreshAttention(client, companyId);
			},
			'Marked done',
			'That was not recorded. Try again.'
		);
	const approve = (kind: 'grant' | 'decline', always = false) =>
		run(
			async () => {
				const target = item.source.call_key
					? { call_key: item.source.call_key, ...(always ? { always: true } : {}) }
					: item.source.party;
				if (!target) throw new Error('This request has no approval target. Refresh and try again.');
				await approvalAction(companyId, kind, target);
				await removeConfirmedAttention(client, companyId, item.id);
			},
			kind === 'grant' ? (always ? 'Approved. It won’t ask again.' : 'Approved') : 'Declined',
			'The approval was not recorded. Try again.'
		);
	const decideMandate = (decision: 'approve' | 'decline') =>
		run(
			async () => {
				const response = await fetch(
					`/api/companies/${encodeURIComponent(companyId)}/email-mandates/proposals/${encodeURIComponent(item.source.reference)}/decision`,
					{
						method: 'POST',
						headers: { 'content-type': 'application/json' },
						credentials: 'same-origin',
						body: JSON.stringify({ decision })
					}
				);
				if (!response.ok) {
					const body = await response.json().catch(() => null);
					throw new Error(body?.message ?? 'The mandate decision was not recorded.');
				}
				await refreshAttention(client, companyId);
			},
			decision === 'approve' ? 'Mandate approved' : 'Mandate declined',
			'The mandate decision was not recorded. Try again.'
		);
	async function openDocument() {
		if (acting || !onopenDocument) return;
		acting = true;
		error = '';
		try {
			await onopenDocument();
		} catch {
			error = 'Could not open the document. Try again.';
		} finally {
			acting = false;
		}
	}
	async function copyReference() {
		try {
			await navigator.clipboard.writeText(
				`${item.source.plane}:${item.source.kind}:${item.source.reference}`
			);
		} catch {
			/* The reference stays visible in Evidence. */
		}
	}
	function keys(event: KeyboardEvent, submit: () => unknown) {
		if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
			event.preventDefault();
			void submit();
		}
		if (event.key === 'Escape') (event.currentTarget as HTMLElement).blur();
	}
	const initial = (name: string) => initials(name) || '?';
</script>

<article class="inbox-detail" aria-label={attentionTitle(item)}>
	<header class="head">
		<div class="title">
			<h1>{attentionTitle(item)}</h1>
			<p class="meta">
				<span title={attentionKindLabel(item)}>{attentionKindLabel(item)}</span>
				<span aria-hidden="true">·</span>
				<span>{asker}</span>
				<span aria-hidden="true">·</span>
				<RelativeTime value={item.createdAt} />
				{#if item.deadline}<span aria-hidden="true">·</span><span class="deadline"
						>Needed by {item.deadline}</span
					>{/if}
			</p>
		</div>
		<button
			type="button"
			class="btn ghost small dismiss"
			disabled={acting}
			title={item.responsibleActor
				? `Not needed. Off your Inbox and kept under Set aside; ${asker} stops waiting on it.`
				: 'Not needed. Off your Inbox and kept under Set aside, until it comes up again.'}
			onclick={() => void dismiss()}>Dismiss</button
		>
		<ActionMenu label="More">
			{#if discuss}<a href={`${base}&conversation=${encodeURIComponent(item.id)}`}>Discuss</a>{/if}
			<a href={conversationHref}>Open conversation with {asker}</a>
			{#if item.workId}<a
					href={`/${encodeURIComponent(companyId)}/work/${encodeURIComponent(item.workId)}`}
					>Open the work</a
				>{/if}
			{#each navigation.slice(1) as extra (extra.id)}<a
					href={extra.href}
					target={extra.href?.startsWith('/') ? undefined : '_blank'}
					rel="noreferrer"
					title={`${extra.consequence} ${extra.nextState}`}>{extra.label}</a
				>{/each}
			<button onclick={copyReference}>Copy reference</button>
		</ActionMenu>
	</header>

	<div class="body">
		<div class="message">
			<span class="avatar" aria-hidden="true">{initial(asker)}</span>
			<div class="message-body">
				<div class="message-head">
					<strong>{asker}</strong>
					{#if item.responsibleActor?.role}<span>{item.responsibleActor.role}</span>{/if}
				</div>
				{#if item.preparing}
					<p class="waiting">
						Nothing to do yet. This step is being prepared; instructions and any sign-in link will
						appear here when ready.
					</p>
				{:else}
					<div class="ask"><Markdown text={item.requestedAction} /></div>
				{/if}
				{#if item.uncertainty}<p class="uncertain" title="What the team is not sure about">
						<span>Not certain</span>{item.uncertainty}
					</p>{/if}
			</div>
		</div>

		<div class="folds">
			{#if item.whatHappened || item.whyItMatters || showRecommendation}
				<Fold label="Context">
					<div class="prose">
						{#if item.whatHappened}<Markdown text={item.whatHappened} />{/if}
						{#if item.whyItMatters && item.whyItMatters !== item.whatHappened}<Markdown
								text={item.whyItMatters}
							/>{/if}
						{#if showRecommendation}<div class="recommended">
								<strong>Recommended</strong>
								<Markdown text={item.recommendation} />
							</div>{/if}
					</div>
				</Fold>
			{/if}
			<Fold label="Evidence" count={item.evidence.length || null}>
				<div class="prose evidence">
					<p class="credit">
						Prepared by <strong>{item.briefAuthor?.display ?? asker}</strong>{#if item.briefedAt}
							· <time title={formatMoment(item.briefedAt)}
								><RelativeTime value={item.briefedAt} /></time
							>{/if}
					</p>
					{#each item.evidence as evidence, index (`${evidence.kind}:${evidence.label}:${index}`)}
						{#if evidence.content}
							<div class="quote">
								<span>{evidence.label}</span>
								<blockquote>{evidence.content}</blockquote>
							</div>
						{:else if evidence.uri}
							<a class="link" href={evidence.uri} target="_blank" rel="noreferrer"
								>{evidence.label} <span aria-hidden="true">↗</span></a
							>
						{/if}
					{/each}
					{#if item.ifNoAction}<p class="credit" title="What happens if nobody acts">
							If nothing happens: {item.ifNoAction}
						</p>{/if}
				</div>
			</Fold>
		</div>
	</div>

	<footer class="inbox-composer">
		{#if error}<Notice tone="danger" title={error} />{/if}
		{#if approveMandate}
			<p
				class="note"
				title="Exec judges whether each recipient and message fits the mandate. The system enforces the mechanical limits."
			>
				Approving gives Exec bounded sending authority.
			</p>
		{/if}

		{#if declining}
			<textarea
				bind:value={reason}
				aria-label="Why not"
				placeholder={`Why not? ${asker} will see this.`}
				disabled={acting}
				onkeydown={(event) => keys(event, sendDecline)}></textarea>
			<div class="bar">
				<button class="btn primary small" disabled={acting || !reason.trim()} onclick={sendDecline}
					>{acting ? 'Sending…' : 'Decline'}</button
				>
				<button class="btn small ghost" disabled={acting} onclick={() => (declining = false)}
					>Back</button
				>
				<span class="hint">⌘ ↵</span>
			</div>
		{:else if record}
			{#if fields.length}
				<div class="fields">
					{#each fields as field (field.id)}
						<label
							><span>{field.label}{field.required ? '' : ' (optional)'}</span>
							{#if field.type === 'choice' || field.type === 'boolean'}<select
									bind:value={fieldValues[field.id]}
									required={field.required}
									disabled={acting}
									><option value="">Choose…</option>{#each optionsFor(field) as option}<option
											value={option}>{option}</option
										>{/each}</select
								>{:else}<input
									type={field.type === 'date'
										? 'date'
										: field.type === 'amount'
											? 'number'
											: 'text'}
									step={field.type === 'amount' ? '0.01' : undefined}
									bind:value={fieldValues[field.id]}
									required={field.required}
									disabled={acting}
								/>{/if}
						</label>
					{/each}
				</div>
			{/if}
			<textarea
				bind:value={reply}
				aria-label={`Reply to ${asker}`}
				placeholder={fields.length ? 'Anything else (optional)' : `Reply to ${asker}…`}
				disabled={acting}
				onkeydown={(event) => keys(event, sendReply)}></textarea>
			<div class="bar">
				<button
					class="btn primary small"
					disabled={acting || !replyReady}
					title={replyReady
						? `${record.consequence} ${record.nextState}`
						: 'Write your answer first'}
					onclick={sendReply}>{acting ? 'Sending…' : 'Send'}</button
				>
				<button class="btn small" disabled={acting} onclick={() => (declining = true)}
					>Not doing this</button
				>
				{#if discuss}<a
						class="btn small ghost"
						href={`${base}&conversation=${encodeURIComponent(item.id)}`}>Discuss</a
					>{/if}
				<span class="hint" title={`Goes to ${asker} as a message in your conversation`}
					>To {asker} · ⌘ ↵</span
				>
			</div>
		{:else}
			<div class="bar">
				{#if grant}
					{#key `${item.id}:${attempt}`}
						<HoldApprove
							small
							completeLabel="Saving approval…"
							label={`Hold to ${grant.label.toLowerCase()}`}
							disabled={acting}
							title={`${grant.consequence} ${grant.nextState}`}
							onapprove={() => void approve('grant')}
						/>
					{/key}
					{#if grantAlways}<button
							class="btn small"
							disabled={acting}
							title={`${grantAlways.consequence} ${grantAlways.nextState}`}
							onclick={() => approve('grant', true)}>{grantAlways.label}</button
						>{/if}
					{#if decline}<button
							class="btn small"
							disabled={acting}
							title={`${decline.consequence} ${decline.nextState}`}
							onclick={() => approve('decline')}>{decline.label}</button
						>{/if}
				{:else if approveMandate}
					{#key `${item.id}:${attempt}`}
						<HoldApprove
							small
							completeLabel="Saving approval…"
							label="Hold to approve mandate"
							disabled={acting}
							title={`${approveMandate.consequence} ${approveMandate.nextState}`}
							onapprove={() => void decideMandate('approve')}
						/>
					{/key}
					{#if declineMandate}<button
							class="btn small"
							disabled={acting}
							onclick={() => decideMandate('decline')}>{declineMandate.label}</button
						>{/if}
				{:else if documentRequest && (item.nativeDocument || onopenDocument)}
					{#if item.nativeDocument}<a class="btn primary small" href={base} title={item.ifNoAction}
							>{item.source.kind === 'document_review'
								? 'Review this version'
								: 'Open and edit together'}</a
						>{:else}<button class="btn primary small" disabled={acting} onclick={openDocument}
							>{acting ? 'Opening…' : 'Open the document'}</button
						>{/if}
				{:else if appRequest && humanStep}
					<a
						class="btn primary small"
						href={appRequest.href(companyId)}
						title="Opens the app with this request attached. Allowing it resumes the work on its own."
						>Add {appRequest.name}</a
					>
					<button class="btn small ghost" disabled={acting} onclick={() => (declining = true)}
						>Not doing this</button
					>
				{:else if humanStep}
					{#if instructionLink}<a
							class="btn primary small"
							href={instructionLink.href}
							target="_blank"
							rel="noreferrer">{instructionLink.label}</a
						>{:else if navigation[0]}<a
							class="btn primary small"
							href={navigation[0].href}
							target={navigation[0].href?.startsWith('/') ? undefined : '_blank'}
							rel="noreferrer">{navigation[0].label}</a
						>{/if}
					<button
						class="btn small"
						class:primary={!instructionLink && !navigation[0]}
						disabled={acting || item.preparing}
						title={`${humanStep.consequence} ${humanStep.nextState}`}
						onclick={markDone}>{acting ? 'Recording…' : 'Mark done'}</button
					>
					<button class="btn small ghost" disabled={acting} onclick={() => (declining = true)}
						>Not doing this</button
					>
				{:else if open && !open.href}
					<a
						class="btn primary small"
						href={`${base}&${item.category === 'review' ? 'review' : 'computer'}=${encodeURIComponent(item.id)}`}
						title={`${open.consequence} ${open.nextState}`}>{open.label}</a
					>
				{:else if review && !navigation.length}
					<a class="btn primary small" href={`${base}&review=${encodeURIComponent(item.id)}`}
						>Review the outcome</a
					>
				{:else if navigation[0]}
					<a
						class="btn primary small"
						href={navigation[0].href}
						target={navigation[0].href?.startsWith('/') ? undefined : '_blank'}
						rel="noreferrer"
						title={`${navigation[0].consequence} ${navigation[0].nextState}`}
						>{navigation[0].label}</a
					>
				{:else if item.source.kind === 'conversation_owner_need'}
					<a
						class="btn primary small"
						href={action('continue-conversation')?.href ?? conversationHref}
						>Reply in conversation</a
					>
				{/if}
				{#if discuss && !humanStep}<a
						class="btn small ghost"
						href={`${base}&conversation=${encodeURIComponent(item.id)}`}>Discuss</a
					>{/if}
			</div>
		{/if}
	</footer>
</article>

<style>
	.inbox-detail {
		display: grid;
		grid-template-rows: auto minmax(0, 1fr) auto;
		height: 100%;
		min-height: 0;
		container: page / inline-size;
	}
	.head {
		display: flex;
		align-items: flex-start;
		gap: var(--space-3);
		padding: 22px 24px 14px;
		border-bottom: 1px solid var(--border);
	}
	.title {
		flex: 1;
		min-width: 0;
	}
	h1 {
		margin: 0;
		color: var(--ink);
		font-size: var(--t-title);
		font-weight: 600;
		line-height: 1.3;
		letter-spacing: -0.015em;
		text-wrap: balance;
	}
	.meta {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
		margin: 6px 0 0;
		color: var(--text-tertiary);
		font-size: var(--t-body);
	}
	.meta :global(time) {
		color: inherit;
	}
	.deadline {
		color: var(--intent-authority);
	}
	.body {
		min-height: 0;
		overflow: auto;
		padding: 20px 24px 8px;
	}
	.message {
		display: grid;
		grid-template-columns: 30px minmax(0, 1fr);
		gap: 12px;
		max-width: 760px;
	}
	.avatar {
		display: grid;
		place-items: center;
		width: 30px;
		height: 30px;
		border: 1px solid var(--border-strong);
		border-radius: 50%;
		background: var(--surface-alt);
		color: var(--text-secondary);
		font-size: var(--t-body);
		font-weight: 600;
	}
	.message-head {
		display: flex;
		align-items: baseline;
		gap: 8px;
		margin-bottom: 4px;
		font-size: var(--t-body);
	}
	.message-head strong {
		color: var(--ink);
	}
	.message-head span {
		color: var(--text-tertiary);
	}
	.ask {
		color: var(--ink);
		font-size: var(--t-body);
		line-height: 1.65;
		overflow-wrap: anywhere;
	}
	.ask :global(.md > :first-child),
	.prose :global(.md > :first-child) {
		margin-top: 0;
	}
	.ask :global(.md > :last-child),
	.prose :global(.md > :last-child) {
		margin-bottom: 0;
	}
	.waiting {
		margin: 0;
		color: var(--text-secondary);
	}
	.uncertain {
		display: grid;
		gap: 2px;
		margin: 12px 0 0;
		padding: 8px 10px;
		border-left: 2px solid var(--intent-authority);
		background: color-mix(in srgb, var(--intent-authority) 5%, transparent);
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.55;
	}
	.uncertain span {
		color: var(--intent-authority);
		font-size: var(--t-label);
		font-weight: 600;
	}
	.folds {
		max-width: 760px;
		margin: 20px 0 0 42px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		overflow: hidden;
	}
	.folds > :global(* + *) {
		border-top: 1px solid var(--border);
	}
	.prose {
		display: grid;
		gap: 12px;
		padding: 14px 16px 16px 38px;
		color: var(--text-secondary);
		font-size: var(--t-body);
		line-height: 1.6;
		overflow-wrap: anywhere;
	}
	.recommended strong {
		display: block;
		margin-bottom: 4px;
		color: var(--intent-feedback);
		font-size: var(--t-label);
	}
	.credit {
		margin: 0;
		color: var(--text-tertiary);
	}
	.credit strong {
		color: var(--ink);
		font-weight: 500;
	}
	.quote span {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.quote blockquote {
		margin: 4px 0 0;
		padding-left: 10px;
		border-left: 2px solid var(--border-strong);
		white-space: pre-wrap;
	}
	.link {
		color: var(--intent-conversation);
		text-decoration: none;
	}
	.link:hover {
		text-decoration: underline;
	}
	.inbox-composer {
		display: grid;
		gap: 10px;
		padding: 12px 24px 16px;
		border-top: 1px solid var(--border);
		background: color-mix(in srgb, var(--surface-pane) 92%, transparent);
	}
	.inbox-composer textarea {
		width: 100%;
		min-height: 64px;
		max-height: 220px;
		field-sizing: content;
		resize: none;
	}
	.fields {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
		gap: 10px;
	}
	.fields label {
		display: grid;
		gap: 4px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.fields :is(input, select) {
		width: 100%;
	}
	.bar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-2);
	}
	.hint {
		margin-left: auto;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.note {
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	/* Choosing another item settles its detail in place: a short fade and a
	 * 4px rise after the selection lands, instead of swapping the pane. */
	.head,
	.body {
		animation: inbox-detail-in 260ms var(--ease-out) 30ms backwards;
	}
	@keyframes inbox-detail-in {
		from {
			opacity: 0;
			transform: translateY(4px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.head,
		.body {
			animation: none;
		}
	}
	@container page (max-width: 560px) {
		.head,
		.body,
		.inbox-composer {
			padding-inline: 14px;
		}
		.folds {
			margin-left: 0;
		}
		.hint {
			display: none;
		}
	}
</style>
