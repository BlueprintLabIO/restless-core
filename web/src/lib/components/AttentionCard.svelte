<script lang="ts">
	import { useQueryClient } from '@tanstack/svelte-query';
	import type { AttentionItem } from '$lib/model/view';
	import { approvalAction, resolveHandoffDecision } from '$lib/model/attention';
	import { refreshAttention } from '$lib/model/queries.svelte';
	import HoldApprove from '$lib/primitives/HoldApprove.svelte';
	import Markdown from '$lib/primitives/Markdown.svelte';

	let {
		companyId,
		item,
		showTitle = true,
		inChat = false
	}: {
		companyId: string;
		item: AttentionItem;
		showTitle?: boolean;
		inChat?: boolean;
	} = $props();
	const client = useQueryClient();
	let acting = $state(false);
	let error = $state('');
	let decision = $state('');
	let attempt = $state(0);
	const base = $derived(`/${encodeURIComponent(companyId)}?item=${encodeURIComponent(item.id)}`);
	const grant = $derived(item.actions.find((a) => a.id === 'grant'));
	const decline = $derived(item.actions.find((a) => a.id === 'decline'));
	const record = $derived(item.actions.find((a) => a.id === 'record-decision'));
	const open = $derived(item.actions.find((a) => a.id === 'open-outcome'));
	const review = $derived(item.actions.some((a) => a.id === 'accept-review'));
	async function act(kind: 'grant' | 'decline' | 'decision') {
		if (acting) return;
		acting = true;
		error = '';
		try {
			if (kind === 'decision')
				await resolveHandoffDecision(companyId, item.source.reference, decision.trim());
			else {
				if (!item.source.party)
					throw new Error('This request has no approval target. Refresh and try again.');
				await approvalAction(companyId, kind, item.source.party);
			}
			decision = '';
			await refreshAttention(client, companyId);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The action was not recorded. Try again.';
		} finally {
			acting = false;
			attempt += 1;
		}
	}
</script>

<section
	class="attention-card"
	data-attention-id={item.id}
	aria-label={item.title}
	aria-busy={acting}
>
	{#if showTitle}<header><strong>{item.title}</strong><span>{item.preparing ? 'Preparing' : 'Needs you'}</span></header>{/if}
	<div class="request"><Markdown text={item.requestedAction} /></div>
	{#if inChat}
		<details>
			<summary>Details</summary>
			<p>{item.whatHappened}</p>
			<p>{item.whyItMatters}</p>
			<Markdown text={item.recommendation} />
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
		{#each item.actions.filter((a) => a.href && !(inChat && a.id === 'continue-conversation')) as action (action.id)}
			<a
				class="btn small primary"
				href={action.href}
				target={action.href?.startsWith('/') ? undefined : '_blank'}
				rel="noreferrer"
				title={`${action.consequence} ${action.nextState}`}>{action.label}</a
			>
		{/each}
		{#if grant}
			{#key `${item.id}:${attempt}`}
				<HoldApprove
					small
					completeLabel="Working…"
					label={acting ? 'Working…' : `Hold to ${grant.label.toLowerCase()}`}
					disabled={acting}
					title={`${grant.consequence} ${grant.nextState}`}
					onapprove={() => void act('grant')}
				/>
			{/key}
		{/if}
		{#if decline}<button
				class="btn small"
				disabled={acting}
				title={`${decline.consequence} ${decline.nextState}`}
				onclick={() => void act('decline')}>{decline.label}</button
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
		{#if item.nativeDocument && !item.actions.some((action) => action.href)}<a
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
	{#if !inChat}<p class="quiet">{item.ifNoAction}</p>{/if}
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
		.attention-card {
			padding: 12px;
		}
		header {
			flex-wrap: wrap;
			gap: 4px 12px;
		}
	}
</style>
