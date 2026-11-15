<script lang="ts">
	/* Today, at the top of the Inbox list. (With nothing waiting the list is folded away and the
	 * same brief is said by the pill over the office; see the company page.) For the owner it
	 * leads with the brief Exec wrote from the
	 * latest daily review (S117): what moved, what it is starting unless they say no, and what
	 * it needs. One row of fixed height; the whole brief unfolds underneath. With no brief yet
	 * the same row says calmly where the company stands. A reply goes through the rail; this
	 * never holds chat. Under it sits the S80 line: what Exec applied for others and what waits
	 * on this person, each clause opening its list. Everything here is one cached read the
	 * company layout makes with the cockpit, so the row appears with the list, never after it,
	 * and reading it wakes nothing. */
	import { createQuery } from '@tanstack/svelte-query';
	import { briefDay, briefLead, directionDigestQuery } from '$lib/model/direction';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import AppliedChanges from './AppliedChanges.svelte';

	let {
		companyId,
		requestsOnly = false,
		onrequests
	}: {
		companyId: string;
		/** The Inbox is showing only the waiting requests. */
		requestsOnly?: boolean;
		/** Filter the Inbox to the waiting requests, or back to everything. */
		onrequests?: (only: boolean) => void;
	} = $props();

	const digest = createQuery(() => directionDigestQuery(companyId));
	const applied = $derived(digest.data?.applied ?? []);
	const waiting = $derived(digest.data?.requests_waiting.length ?? 0);
	const brief = $derived(digest.data?.brief ?? null);
	/* Only the owner is sent a standing line, so its presence is what shows the brief row. */
	const standing = $derived(digest.data?.standing ?? '');
	const plural = (count: number, one: string, many: string) =>
		`${count} ${count === 1 ? one : many}`;
	let showApplied = $state(false);
	let showBrief = $state(false);

	/* The row shows one line of the brief; an older brief is shown dated. */
	const lead = $derived(briefLead(brief?.text ?? ''));
	const when = $derived(brief ? briefDay(brief.at) : 'Today');
</script>

{#if standing || applied.length || waiting}
	<div class="direction-today">
		{#if standing}
			<div class="brief-row" role="status">
				<span class="label">{when}</span>
				{#if brief && lead}<button
						type="button"
						class="brief-lead"
						aria-expanded={showBrief}
						title={showBrief ? 'Fold the brief away' : 'Read the whole brief'}
						onclick={() => (showBrief = !showBrief)}>{lead}</button
					>{:else}<span class="brief-calm">{standing}</span>{/if}
			</div>
		{/if}
		{#if applied.length || waiting}
			<p class:sub={!!standing} role="status">
				{#if !standing}<span class="label">Today</span>{/if}
				{#if applied.length}<button
						type="button"
						class="clause"
						aria-expanded={showApplied}
						title="See each change, open what it touched, or undo it"
						onclick={() => (showApplied = !showApplied)}
						>Exec applied {plural(applied.length, 'change', 'changes')} for others</button
					>{/if}
				{#if applied.length && waiting}<span aria-hidden="true">·</span>{/if}
				{#if waiting}<button
						type="button"
						class="clause"
						aria-pressed={requestsOnly}
						title={requestsOnly ? 'Show everything in the Inbox' : 'Show only these requests'}
						onclick={() => onrequests?.(!requestsOnly)}
						>{plural(waiting, 'request', 'requests')} waiting</button
					>{/if}
			</p>
		{/if}
		{#if showBrief && brief}
			<div class="brief-full">
				<Markdown text={brief.text} />
				<small>To answer, write to Exec in the chat beside this page.</small>
			</div>
		{/if}
		{#if showApplied && applied.length}<AppliedChanges
				{companyId}
				changes={applied}
				onchanged={() => void digest.refetch()}
			/>{/if}
	</div>
{/if}

<style>
	.direction-today {
		border-bottom: 1px solid var(--border);
	}
	/* One row, one line, one height: with a brief, without one, and whatever the brief says. */
	.brief-row {
		display: flex;
		align-items: center;
		gap: 8px;
		box-sizing: border-box;
		height: 36px;
		min-width: 0;
		padding: 0 16px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	/* Width 0 and grow: a long brief must not widen the list pane, only fill it. */
	.brief-lead,
	.brief-calm {
		flex: 1 1 0;
		width: 0;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		color: var(--text-secondary);
	}
	.brief-lead {
		padding: 0;
		border: 0;
		background: none;
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.brief-lead:hover,
	.brief-lead[aria-expanded='true'] {
		color: var(--ink);
	}
	.brief-full {
		margin: 0 16px 12px;
		padding: 10px 12px;
		border: 1px solid var(--border);
		border-radius: 8px;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.brief-full small {
		display: block;
		margin-top: 8px;
		color: var(--text-tertiary);
	}
	p {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: 4px 8px;
		min-width: 0;
		margin: 0;
		padding: 8px 16px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	p.sub {
		padding-top: 0;
	}
	.label {
		flex: none;
		font-weight: 600;
	}
	.clause {
		padding: 0;
		border: 0;
		background: none;
		color: var(--text-secondary);
		font: inherit;
		text-decoration: underline;
		text-decoration-color: var(--border-strong);
		text-underline-offset: 3px;
		cursor: pointer;
	}
	.clause:hover,
	.clause[aria-expanded='true'],
	.clause[aria-pressed='true'] {
		color: var(--ink);
		text-decoration-color: currentColor;
	}
	.direction-today :global(.applied-changes) {
		margin: 0 8px 8px;
	}
</style>
