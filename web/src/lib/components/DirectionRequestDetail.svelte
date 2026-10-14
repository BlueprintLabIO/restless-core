<script lang="ts">
	/* A request someone asked Exec for and that waits on this person (S80): what they asked, the
	 * exact change Exec will make, and why. Approving carries the change out, after a few seconds
	 * in which Undo takes it back; "always allow" is a hold, because it also stops this kind of
	 * change asking again. Y and N answer only while focus is inside the request. */
	import { useQueryClient } from '@tanstack/svelte-query';
	import HoldApprove from '$lib/ui/controls/HoldApprove.svelte';
	import { dismissToast, toast } from '$lib/ui/feedback/toast.svelte';
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
	/* An approval waiting out its Undo window. */
	let pending = $state(false);
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

	/* An approval is held for a few seconds with Undo before it is sent: Exec starts carrying it
	 * out as soon as it lands. */
	const UNDO_WINDOW_MS = 5000;
	function approveSoon(input: DirectionDecisionInput, from?: EventTarget | null) {
		if (busy || pending || !open) return;
		pending = true;
		const timer = setTimeout(() => {
			pending = false;
			dismissToast(id);
			void answer(input, from);
		}, UNDO_WINDOW_MS);
		const id = toast(`Approving “${current.summary}”`, {
			undo: () => {
				clearTimeout(timer);
				pending = false;
			}
		});
	}

	/* Y approves (with Undo) and N declines, only while focus is inside this request and never
	 * while typing: a stray key elsewhere answers nothing. Always allow has no key. */
	function keys(event: KeyboardEvent) {
		if (event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey) return;
		const target = event.target as HTMLElement | null;
		if (target?.closest('input, textarea, select, [contenteditable], dialog, details')) return;
		const key = event.key.toLowerCase();
		if (key === 'y') approveSoon({ decision: 'approve' }, document.activeElement);
		else if (key === 'n' && open && !pending) declining = true;
		else return;
		event.preventDefault();
	}

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

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<article
	class="direction-request"
	aria-labelledby="direction-request-title"
	tabindex="-1"
	onkeydown={keys}
>
	<header>
		<h2 id="direction-request-title" title="A request: nothing changes until you answer">
			{current.summary}
		</h2>
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
					disabled={busy || pending}
					title="Approve. Undo stays available for a few seconds before Exec starts."
					onclick={(event) => approveSoon({ decision: 'approve' }, event.currentTarget)}
					>{pending ? 'Approving…' : 'Approve'} <kbd>Y</kbd></button
				>
				{#if always}{#key pending}<HoldApprove
							small
							label="Hold to always allow"
							armedLabel="press again to always allow"
							completeLabel="Approving…"
							disabled={busy || pending}
							title={`Approve, and ${always.label.charAt(0).toLowerCase() + always.label.slice(1)}: this kind of change stops asking you. Each one can still be undone; revoke it in Company → Members.`}
							onapprove={() => approveSoon({ decision: 'approve', always_allow: true })}
						/>{/key}{/if}
				{#if makeOwner && current.team}<button
						class="btn small"
						type="button"
						disabled={busy || pending}
						title={`Approve, and make ${makeOwner.name} owner of ${current.team.name}: their direction for it then applies directly.`}
						onclick={(event) =>
							approveSoon({ decision: 'approve', make_team_owner: true }, event.currentTarget)}
						>Make {makeOwner.name} owner</button
					>{/if}
				<button
					class="btn small ghost"
					type="button"
					disabled={busy || pending}
					onclick={() => (declining = true)}>Decline… <kbd>N</kbd></button
				>
			</div>
		{/if}
	{/if}
</article>

<style>
	.direction-request {
		display: grid;
		gap: 20px;
		max-width: 680px;
		padding: 28px 32px;
	}
	header {
		display: grid;
		gap: 6px;
	}
	h2 {
		margin: 0;
		color: var(--ink);
		font-size: var(--t-head);
		font-weight: 600;
		line-height: 1.35;
		letter-spacing: -0.005em;
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
	.answers {
		padding-top: 4px;
	}
	.answers .btn {
		white-space: nowrap;
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
