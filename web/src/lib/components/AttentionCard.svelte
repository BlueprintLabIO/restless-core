<script lang="ts">
	import { useQueryClient } from '@tanstack/svelte-query';
	import type { AttentionItem } from '$lib/model/view';
	import {
		approvalAction,
		completeHandoffHumanStep,
		resolveHandoffDecision
	} from '$lib/model/attention';
	import { refreshAttention, removeConfirmedAttention } from '$lib/model/queries.svelte';
	import HoldApprove from '$lib/primitives/HoldApprove.svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';

	let {
		companyId,
		item,
		showTitle = true,
		inChat = false,
		embedded = false,
		onopenDocument
	}: {
		companyId: string;
		item: AttentionItem;
		showTitle?: boolean;
		inChat?: boolean;
		embedded?: boolean;
		onopenDocument?: () => Promise<unknown>;
	} = $props();
	const client = useQueryClient();
	let acting = $state(false);
	let actionStatus = $state('');
	let error = $state('');
	let decision = $state('');
	let attempt = $state(0);
	const documentRequest = $derived(
		item.source.kind === 'document_collaboration' || item.source.kind === 'document_review'
	);
	const hasDocumentAction = $derived(
		documentRequest && Boolean(item.nativeDocument || onopenDocument)
	);
	const documentLabel = $derived(
		item.source.kind === 'document_review' ? 'Review this version' : 'Open and edit together'
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
	const base = $derived(`/${encodeURIComponent(companyId)}?item=${encodeURIComponent(item.id)}`);
	const grant = $derived(item.actions.find((a) => a.id === 'grant'));
	const decline = $derived(item.actions.find((a) => a.id === 'decline'));
	const emailMandateProposal = $derived(item.source.kind === 'email_mandate_proposal');
	const approveEmailMandate = $derived(item.actions.find((a) => a.id === 'approve-email-mandate'));
	const declineEmailMandate = $derived(item.actions.find((a) => a.id === 'decline-email-mandate'));
	const record = $derived(item.actions.find((a) => a.id === 'record-decision'));
	const open = $derived(item.actions.find((a) => a.id === 'open-outcome'));
	const review = $derived(item.actions.some((a) => a.id === 'accept-review'));
	const completeHumanStep = $derived(item.actions.find((a) => a.id === 'complete-human-step'));
	// Older daemons may publish the verification URL in the instructions without
	// a dedicated action. Only link the current instructions, never old evidence.
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
	async function act(
		kind: 'grant' | 'decline' | 'decision' | 'approve-email-mandate' | 'decline-email-mandate'
	) {
		if (acting) return;
		acting = true;
		actionStatus =
			kind === 'grant' || kind === 'approve-email-mandate'
				? 'Saving approval…'
				: 'Saving decision…';
		error = '';
		try {
			if (kind === 'approve-email-mandate' || kind === 'decline-email-mandate') {
				const response = await fetch(
					`/api/companies/${encodeURIComponent(companyId)}/email-mandates/proposals/${encodeURIComponent(item.source.reference)}/decision`,
					{
						method: 'POST',
						headers: { 'content-type': 'application/json' },
						credentials: 'same-origin',
						body: JSON.stringify({
							decision: kind === 'approve-email-mandate' ? 'approve' : 'decline'
						})
					}
				);
				if (!response.ok) {
					let message = 'The mandate decision was not recorded. Try again.';
					try {
						const body = await response.json();
						message = body.message ?? message;
					} catch {
						// Keep the useful fallback when the server response is not JSON.
					}
					throw new Error(message);
				}
				await refreshAttention(client, companyId);
			} else if (kind === 'decision')
				await resolveHandoffDecision(companyId, item.source.reference, decision.trim());
			else {
				if (!item.source.party)
					throw new Error('This request has no approval target. Refresh and try again.');
				await approvalAction(companyId, kind, item.source.party);
			}
			decision = '';
			if (kind === 'decision') await refreshAttention(client, companyId);
			else await removeConfirmedAttention(client, companyId, item.id);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The action was not recorded. Try again.';
		} finally {
			acting = false;
			actionStatus = '';
			attempt += 1;
		}
	}
	async function complete() {
		if (acting) return;
		acting = true;
		error = '';
		try {
			await completeHandoffHumanStep(companyId, item.source.reference);
			await refreshAttention(client, companyId);
		} catch (cause) {
			error =
				cause instanceof Error ? cause.message : 'The completion was not recorded. Try again.';
		} finally {
			acting = false;
		}
	}
</script>

<section
	class="attention-card"
	class:embedded
	data-attention-id={item.id}
	aria-label={item.preparing ? 'Preparing your next step' : item.title}
	aria-busy={acting}
>
	{#if showTitle}<header>
			<strong>{item.preparing ? 'Preparing your next step' : item.title}</strong><span
				>{actionStatus || (item.preparing ? 'Preparing' : 'Needs you')}</span
			>
		</header>{/if}
	{#if !hasDocumentAction}<div class="request"><Markdown text={item.requestedAction} /></div>{/if}
	{#if emailMandateProposal}<p class="mandate-judgement-warning">
			Exec must judge whether each recipient and message fits this mandate, and can get that
			judgment wrong. Approval gives Exec bounded sending authority under the limits below. The
			system checks mechanical limits, but cannot guarantee that a particular send is appropriate.
			Review the exact proposal before approving.
		</p>{/if}
	{#if item.preparing}
		<p class="waiting" role="status">
			Nothing to do yet. The team is preparing this step. Your instructions and any sign-in link
			will appear here when ready.
		</p>
	{:else if item.deadline && inChat}
		<p class="quiet">{item.deadline}</p>
	{/if}
	{#if inChat}
		<details>
			<summary>Details</summary>
			{#if !item.preparing}<p>{item.whatHappened}</p>
				<p>{item.whyItMatters}</p>
				<Markdown text={item.recommendation} />{/if}
			{#each item.evidence as evidence}
				{#if evidence.content}<details>
						<summary>{evidence.label}</summary>
						<pre>{evidence.content}</pre>
					</details>{/if}
			{/each}
			<a href={base}>Open in Attention →</a>
		</details>
	{/if}
	<div class="actions">
		{#if documentRequest}
			{#if item.nativeDocument}
				<a class="btn primary" href={base} title={item.ifNoAction}>{documentLabel}</a>
			{:else if onopenDocument}
				<button
					class="btn primary"
					disabled={acting}
					onclick={openDocument}
					title="Load the current document and access permissions"
					>{acting ? 'Opening document…' : documentLabel}</button
				>
			{/if}
		{/if}
		{#if instructionLink}
			<a class="btn small primary" href={instructionLink.href} target="_blank" rel="noreferrer"
				>{instructionLink.label}</a
			>
		{/if}
		{#each item.actions.filter((a) => a.href && !(inChat && a.id === 'continue-conversation')) as action (action.id)}
			<a
				class="btn small primary"
				href={action.href}
				target={action.href?.startsWith('/') ? undefined : '_blank'}
				rel="noreferrer"
				title={action.role === 'human_step'
					? action.nextState
					: `${action.consequence} ${action.nextState}`}>{action.label}</a
			>
		{/each}
		{#if grant}
			{#key `${item.id}:${attempt}`}
				<HoldApprove
					small
					completeLabel="Saving approval…"
					label={actionStatus || `Hold to ${grant.label.toLowerCase()}`}
					disabled={acting}
					title={`${grant.consequence} ${grant.nextState}`}
					onapprove={() => void act('grant')}
				/>
			{/key}
		{/if}
		{#if approveEmailMandate}
			{#key `${item.id}:${attempt}`}
				<HoldApprove
					small
					completeLabel="Saving approval…"
					label={actionStatus || 'Hold to approve mandate'}
					disabled={acting}
					title={`${approveEmailMandate.consequence} ${approveEmailMandate.nextState}`}
					onapprove={() => void act('approve-email-mandate')}
				/>
			{/key}
		{/if}
		{#if declineEmailMandate}<button
				class="btn small"
				disabled={acting}
				title={`${declineEmailMandate.consequence} ${declineEmailMandate.nextState}`}
				onclick={() => void act('decline-email-mandate')}>{declineEmailMandate.label}</button
			>{/if}
		{#if decline}<button
				class="btn small"
				disabled={acting}
				title={`${decline.consequence} ${decline.nextState}`}
				onclick={() => void act('decline')}>{decline.label}</button
			>{/if}
		{#if completeHumanStep}<button
				class="btn small primary"
				disabled={acting}
				title={`${completeHumanStep.consequence} ${completeHumanStep.nextState}`}
				onclick={() => void complete()}>{acting ? 'Recording…' : completeHumanStep.label}</button
			>{/if}
		{#if open && !open.href}
			<a
				class="btn small primary"
				href={`${base}&${item.category === 'review' ? 'review' : 'computer'}=${encodeURIComponent(item.id)}`}
				title={`${open.consequence} ${open.nextState}`}>{open.label}</a
			>
		{/if}
		{#if review && !inChat}
			<a class="btn small primary" href={`${base}&review=${encodeURIComponent(item.id)}`}
				>Review outcome</a
			>
		{/if}
		{#if item.nativeDocument && !documentRequest && !item.actions.some((action) => action.href)}<a
				class="btn small primary"
				href={base}>Open document</a
			>{/if}
		{#if !inChat && item.actions.some((a) => a.id === 'chat-lead')}
			<a class="btn small" href={`${base}&conversation=${encodeURIComponent(item.id)}`}>Discuss</a>
		{/if}
	</div>
	{#if record}
		<form
			onsubmit={(event) => {
				event.preventDefault();
				if (decision.trim()) void act('decision');
			}}
		>
			<label
				>Your decision<textarea
					bind:value={decision}
					rows="2"
					placeholder="Tell the team how to proceed…"
					disabled={acting}></textarea></label
			>
			<button
				class="btn small primary"
				disabled={acting || !decision.trim()}
				title={`${record.consequence} ${record.nextState}`}
				>{acting ? 'Recording…' : record.label}</button
			>
		</form>
	{/if}
	{#if item.category === 'review'}<p class="quiet">
			Review the outcome to accept it or request changes.
		</p>{/if}
	{#if error}<p class="error" role="alert">{error}</p>{/if}
	{#if !inChat && !hasDocumentAction}<p class="quiet">{item.ifNoAction}</p>{/if}
</section>

<style>
	.attention-card {
		min-width: 0;
		padding: 14px 16px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface);
		overflow-wrap: anywhere;
	}
	.attention-card.embedded {
		padding: 0;
		border: 0;
		border-radius: 0;
		background: transparent;
	}
	header {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 12px;
		margin-bottom: 8px;
	}
	header strong {
		font-size: var(--t-body);
	}
	header span {
		flex-shrink: 0;
		color: var(--intent-authority);
		font-size: var(--t-label);
	}
	.request {
		font-size: var(--t-body);
	}
	.mandate-judgement-warning {
		margin: 12px 0 0;
		padding: 10px 12px;
		border-left: 2px solid var(--intent-authority);
		color: var(--text-muted);
		font-size: var(--t-label);
		line-height: 1.5;
	}
	.actions {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		margin-top: 10px;
	}
	.actions:empty {
		display: none;
	}
	.actions :global(.btn),
	.actions :global(.hold-approve) {
		max-width: 100%;
		white-space: normal;
	}
	.actions :global(.btn.primary) {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-height: 42px;
		padding: 10px 18px;
		border-color: var(--intent-conversation);
		background: var(--intent-conversation);
		color: var(--text-inverse);
		text-decoration: none;
	}
	form {
		display: grid;
		gap: 8px;
		margin-top: 12px;
	}
	form button {
		justify-self: start;
	}
	label {
		display: grid;
		gap: 6px;
		font-size: var(--t-label);
		font-weight: 500;
	}
	textarea {
		box-sizing: border-box;
		width: 100%;
		min-width: 0;
		resize: vertical;
		padding: 8px 10px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--ink);
		font: inherit;
	}
	details {
		margin-top: 10px;
		font-size: var(--t-body);
		color: var(--text-secondary);
	}
	summary {
		cursor: pointer;
		width: fit-content;
	}
	pre {
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		font: var(--t-label) var(--font-mono);
	}
	.waiting {
		font-size: var(--t-body);
		color: var(--text-secondary);
		margin: 10px 0 0;
	}
	.quiet {
		font-size: var(--t-label);
		color: var(--text-secondary);
		margin: 12px 0 0;
	}
	.error {
		color: var(--state-danger);
		font-size: var(--t-body);
	}
	@media (max-width: 520px) {
		.attention-card:not(.embedded) {
			padding: 12px;
		}
		header {
			flex-wrap: wrap;
			gap: 4px 12px;
		}
	}
</style>
