<script lang="ts">
	/* A request someone asked Exec for and that waits on this person (S80): what they asked, the
	 * exact change Exec will make, and why, with the answer one click or key away. Approving
	 * carries the change out; "always allow" also stops this kind of change asking again. */
	import { onMount } from 'svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import RelativeTime from '$lib/ui/RelativeTime.svelte';
	import { Notice } from '$lib/ui/page';
	import { celebrate } from '$lib/ui/celebrate';
	import { failureSentence } from '$lib/model/failure';
	import { refreshAttention } from '$lib/model/queries.svelte';
	import { decideDirectionRequest, targetPhrase } from '$lib/model/direction';
	import type { DirectionDecisionInput, DirectionRequest } from '$lib/model/generated/direction';

	let {
		companyId,
		request,
		onresolved
	}: {
		companyId: string;
		request: DirectionRequest;
		onresolved?: (sent: { text: string; href?: string }) => void;
	} = $props();

	const client = useQueryClient();
	let busy = $state(false);
	let declining = $state(false);
	let reason = $state('');
	let error = $state('');
	let answered = $state<DirectionRequest | null>(null);
	const current = $derived(answered ?? request);
	const open = $derived(current.state === 'open');
	const always = $derived(current.offers.always_allow);
	const makeOwner = $derived(current.offers.make_team_owner);

	async function answer(input: DirectionDecisionInput, from?: EventTarget | null) {
		if (busy || !open) return;
		busy = true;
		error = '';
		try {
			answered = await decideDirectionRequest(companyId, current.id, input);
			if (input.decision === 'approve') celebrate(from instanceof Element ? from : null);
			const who = current.requester.name;
			onresolved?.({
				text:
					input.decision === 'approve'
						? input.make_team_owner && makeOwner
							? `Approved, and ${makeOwner.name} now owns ${current.team?.name ?? 'the team'}. Exec is carrying it out.`
							: input.always_allow
								? `Approved, and ${who} won't need to ask for this again. Exec is carrying it out.`
								: `Approved. Exec is carrying it out and will tell ${who}.`
						: `Declined. Exec will tell ${who}${input.reason ? ' why' : ''}.`
			});
			declining = false;
			reason = '';
		} catch (cause) {
			// Someone else answered first: say who, then show their answer.
			error = failureSentence(cause, 'That answer was not recorded.');
		} finally {
			busy = false;
			/* The Today line counts what still waits. */
			void client.invalidateQueries({ queryKey: ['direction-digest', companyId] });
			await refreshAttention(client, companyId);
		}
	}

	/* Y approves, A approves and always allows, N declines; never while typing. */
	onMount(() => {
		const keys = (event: KeyboardEvent) => {
			if (event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey) return;
			const target = event.target as HTMLElement | null;
			if (target?.closest('input, textarea, select, [contenteditable], dialog')) return;
			const key = event.key.toLowerCase();
			if (key === 'y') void answer({ decision: 'approve' }, document.activeElement);
			else if (key === 'a' && always)
				void answer({ decision: 'approve', always_allow: true }, document.activeElement);
			else if (key === 'n') declining = true;
			else return;
			event.preventDefault();
		};
		window.addEventListener('keydown', keys);
		return () => window.removeEventListener('keydown', keys);
	});

	const outcome = $derived.by(() => {
		const decision = current.decision;
		switch (current.state) {
			case 'approved':
				return `${decision?.by.name ?? 'Someone'} approved this. Exec is carrying it out.`;
			case 'done':
				return `Done${current.applied ? `: ${targetPhrase(current.applied)} changed` : ''}.`;
			case 'declined':
				return `${decision?.by.name ?? 'Someone'} declined this${decision?.reason ? `: ${decision.reason}` : '.'}`;
			case 'failed':
				return `Approved, but carrying it out failed: ${current.failure ?? 'Exec could not make the change.'}`;
			default:
				return '';
		}
	});
</script>

<article class="direction-request" aria-labelledby="direction-request-title">
	<header>
		<span class="kind">Request</span>
		<h2 id="direction-request-title">{current.summary}</h2>
		<p class="meta">
			<strong>{current.requester.name}</strong> asked{#if current.team}&nbsp;for {current.team
					.name}{/if}
			<span aria-hidden="true">·</span>
			<RelativeTime value={current.filed_at} />
		</p>
	</header>

	<dl class="facts">
		<dt>The change</dt>
		<dd>{current.change}</dd>
		{#if current.reason}<dt>Why</dt>
			<dd>{current.reason}</dd>{/if}
	</dl>

	{#if error}<Notice tone="danger" title={error} />{/if}
	{#if outcome}<p class="outcome" data-state={current.state} role="status">{outcome}</p>{/if}

	{#if open}
		{#if declining}
			<form
				class="decline"
				onsubmit={(event) => {
					event.preventDefault();
					void answer({ decision: 'decline', ...(reason.trim() ? { reason: reason.trim() } : {}) });
				}}
			>
				<!-- svelte-ignore a11y_autofocus -->
				<input
					bind:value={reason}
					autofocus
					maxlength="240"
					aria-label="Why not (optional)"
					placeholder={`Why not, for ${current.requester.name} (optional)`}
				/>
				<button class="btn small danger" disabled={busy}>{busy ? 'Recording…' : 'Decline'}</button
				><button class="btn small ghost" type="button" onclick={() => (declining = false)}
					>Cancel</button
				>
			</form>
		{:else}
			<div class="answers">
				<button
					class="btn small primary"
					type="button"
					disabled={busy}
					onclick={(event) => void answer({ decision: 'approve' }, event.currentTarget)}
					>Approve <kbd>Y</kbd></button
				>
				{#if always}<button
						class="btn small"
						type="button"
						disabled={busy}
						title="Approve this, and let this kind of change apply without asking. You can undo each one and revoke this in Company → Members."
						onclick={(event) =>
							void answer({ decision: 'approve', always_allow: true }, event.currentTarget)}
						>Approve, and {always.label.charAt(0).toLowerCase() + always.label.slice(1)}
						<kbd>A</kbd></button
					>{/if}
				{#if makeOwner && current.team}<button
						class="btn small"
						type="button"
						disabled={busy}
						title={`${makeOwner.name}'s direction for ${current.team.name} then applies directly.`}
						onclick={(event) =>
							void answer({ decision: 'approve', make_team_owner: true }, event.currentTarget)}
						>Approve, and make {makeOwner.name} owner of {current.team.name}</button
					>{/if}
				<button
					class="btn small ghost"
					type="button"
					disabled={busy}
					onclick={() => (declining = true)}>Decline… <kbd>N</kbd></button
				>
			</div>
		{/if}
	{/if}
</article>

<style>
	.direction-request {
		display: grid;
		gap: 16px;
		max-width: 680px;
		padding: 24px 28px;
	}
	header {
		display: grid;
		gap: 6px;
	}
	.kind {
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 600;
		letter-spacing: 0.02em;
	}
	h2 {
		margin: 0;
		color: var(--ink);
		font-size: var(--t-title);
		font-weight: 600;
		line-height: 1.3;
	}
	.meta {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		margin: 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.facts {
		display: grid;
		gap: 4px;
		margin: 0;
	}
	dt {
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 600;
	}
	dd {
		margin: 0 0 10px;
		color: var(--ink);
		font-size: var(--t-body);
		line-height: 1.5;
		white-space: pre-wrap;
	}
	.outcome {
		margin: 0;
		padding: 10px 12px;
		border-radius: var(--radius-control);
		background: var(--surface-alt);
		color: var(--text-secondary);
		font-size: var(--t-body);
	}
	.outcome[data-state='done'],
	.outcome[data-state='approved'] {
		color: var(--state-success);
	}
	.outcome[data-state='failed'] {
		color: var(--state-danger);
	}
	.answers,
	.decline {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		align-items: center;
	}
	/* A grant's label is a sentence; it wraps inside the pane instead of running past it. */
	.answers .btn {
		max-width: 100%;
		height: auto;
		min-height: 28px;
		white-space: normal;
		text-align: left;
	}
	.decline input {
		flex: 1 1 260px;
		min-width: 0;
		padding: 6px 10px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: inherit;
	}
	kbd {
		margin-left: 6px;
		padding: 0 4px;
		border: 1px solid currentColor;
		border-radius: 3px;
		font-size: var(--t-label);
		opacity: 0.6;
	}
</style>
