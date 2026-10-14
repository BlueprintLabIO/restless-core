<script lang="ts">
	/* A person's Work beside their chat (S88-04): what is in motion with the pages it made, what is
	 * next, and what was finished this week. For a lead it is the team's Work. Nothing is stored;
	 * it reads the Work graph. Details opens the full Work sheet (S85-15). */
	import { reviewVerdicts, workPanel, type PanelWorkItem } from '$lib/model/people-directory';
	import { workStatusLabel } from '$lib/work/status';
	import OutputThumb from './OutputThumb.svelte';
	import Check from '@lucide/svelte/icons/check';

	type Item = PanelWorkItem & { goal_id?: string | null };
	type Artifact = {
		id: string;
		kind: string;
		label: string;
		work_id: string | null;
		state: string;
		superseded_at?: string | null;
		uri?: string;
	};
	let {
		companyId,
		name,
		work,
		edges = [],
		artifacts = [],
		gates = [],
		gateRuns = [],
		attempts = [],
		owners,
		ownerName,
		highlight = '',
		spend = null,
		hrefFor,
		ondetails
	}: {
		companyId: string;
		/** Whose Work this is, for "All of Arlo's work". */
		name: string;
		work: readonly Item[];
		edges?: readonly { from_work_id: string; to_work_id: string; kind: string }[];
		artifacts?: readonly Artifact[];
		gates?: readonly { id: string; work_id: string; retired_at: string | null }[];
		gateRuns?: readonly { gate_id: string; passed: boolean; ran_at: string }[];
		attempts?: readonly { work_id: string; revision: number; attempt_no: number; state: string }[];
		owners: ReadonlySet<string>;
		/** Who does a piece of Work, named when the panel is a team's. */
		ownerName: (actorId: string) => string;
		highlight?: string;
		spend?: ReadonlyMap<string, number> | null;
		/** Where a piece of Work opens beside this chat. */
		hrefFor: (workId: string) => string;
		ondetails: (workId: string) => void;
	} = $props();

	const panel = $derived(workPanel(work, owners, { highlight }));
	const team = $derived(owners.size > 1);
	const now = $derived(panel.now);
	/* An independent reviewer's verdict, from its own outcome (S88-04). */
	const verdicts = $derived(reviewVerdicts(work, edges, attempts));
	const nowReview = $derived(now ? (verdicts.get(now.id) ?? null) : null);
	const outputs = $derived(
		now
			? artifacts
					.filter(
						(artifact): artifact is Artifact & { uri: string } =>
							artifact.work_id === now.id &&
							artifact.state === 'available' &&
							!artifact.superseded_at &&
							typeof artifact.uri === 'string'
					)
					.slice(0, 4)
			: []
	);
	/* Checks passed: each gate's latest run. */
	const checks = $derived.by(() => {
		if (!now) return null;
		const mine = gates.filter((gate) => gate.work_id === now.id && !gate.retired_at);
		if (!mine.length) return null;
		const passed = mine.filter((gate) => {
			const latest = gateRuns
				.filter((run) => run.gate_id === gate.id)
				.toSorted((a, b) => Date.parse(b.ran_at) - Date.parse(a.ran_at))[0];
			return latest?.passed === true;
		}).length;
		return { passed, total: mine.length };
	});
	const spent = $derived(now ? (spend?.get(now.id) ?? null) : null);
	const money = (usd: number) => `$${usd < 10 ? usd.toFixed(2) : usd.toFixed(0)}`;
</script>

<section class="work-panel" aria-label={`${name}'s Work`}>
	<h3>Now</h3>
	{#if now}
		<article class="work-now" class:highlighted={highlight === now.id}>
			<a class="work-now-title" href={hrefFor(now.id)}>{now.title}</a>
			<p class="work-now-meta">
				<span class="work-pill status-{now.status}"
					>{panel.nowDelivered ? 'Last delivered' : workStatusLabel(now.status)}</span
				>{#if team}<span>{ownerName(now.owner_id)}</span
					>{/if}{#if nowReview}{#if nowReview.verdict === 'accepted'}<span class="review-badge"
							>Reviewed · accepted</span
						>{:else if nowReview.verdict === 'changes'}<span class="review-changes"
							>{ownerName(nowReview.reviewer.owner_id)} asked for changes</span
						>{:else}<span>review by {ownerName(nowReview.reviewer.owner_id)} next</span>{/if}{/if}
			</p>
			{#if outputs.length}
				<div class="work-outputs">
					{#each outputs as artifact (artifact.id)}
						<OutputThumb {companyId} {artifact} />
					{/each}
				</div>
			{/if}
			<p class="work-now-facts">
				{#if checks}<span>Checks {checks.passed}/{checks.total}</span>{/if}{#if spent !== null}<span
						>{money(spent)}</span
					>{/if}<button type="button" onclick={() => ondetails(now.id)}>Details</button>
			</p>
		</article>
	{:else}
		<p class="work-empty">Nothing yet. Work {name} takes on shows here.</p>
	{/if}

	{#if panel.next.length}
		<h3>Next</h3>
		<ul>
			{#each panel.next as item (item.id)}
				<li>
					<a href={hrefFor(item.id)} title={item.title}>{item.title}</a>{#if team}<small
							>{ownerName(item.owner_id)}</small
						>{/if}
				</li>
			{/each}
		</ul>
	{/if}

	{#if panel.done.length}
		<h3>Done this week</h3>
		<ul>
			{#each panel.done as item (item.id)}
				<li class="done">
					<Check size={13} strokeWidth={2.2} aria-hidden="true" /><a
						href={hrefFor(item.id)}
						title={item.title}>{item.title}</a
					>{#if verdicts.get(item.id)?.verdict === 'accepted'}<span
							class="review-badge"
							title="An independent reviewer accepted it">Reviewed · accepted</span
						>{/if}{#if team}<small>{ownerName(item.owner_id)}</small>{/if}
				</li>
			{/each}
		</ul>
	{/if}

	<a class="work-all" href={`/${encodeURIComponent(companyId)}/work?q=${encodeURIComponent(name)}`}
		>All of {name}'s work</a
	>
</section>

<style>
	.work-panel {
		display: grid;
		align-content: start;
		gap: 8px;
		min-width: 0;
		padding: 14px 16px 20px;
		overflow-y: auto;
	}
	h3 {
		margin: 8px 0 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		font-weight: 600;
		letter-spacing: 0.04em;
		text-transform: uppercase;
	}
	h3:first-child {
		margin-top: 0;
	}
	.work-now {
		display: grid;
		gap: 8px;
		padding: 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
	}
	.work-now.highlighted {
		box-shadow: 0 0 0 2px color-mix(in srgb, var(--intent-conversation) 45%, transparent);
	}
	.work-now-title {
		color: var(--ink);
		font-weight: 600;
		text-decoration: none;
	}
	.work-now-title:hover {
		text-decoration: underline;
	}
	.work-now-meta,
	.work-now-facts {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 4px 10px;
		margin: 0;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.work-now-meta > span:not(.work-pill) {
		min-width: 0;
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.work-pill {
		padding: 1px 8px;
		border-radius: 999px;
		background: var(--wash-hover);
		color: var(--text-secondary);
		font-weight: 600;
	}
	.work-pill.status-active {
		color: var(--intent-feedback);
	}
	.work-pill.status-blocked {
		color: var(--state-danger);
	}
	/* What is safe to send to a client: an independent reviewer accepted it. */
	.review-badge {
		flex: none;
		padding: 0 6px;
		border-radius: 999px;
		background: color-mix(in srgb, var(--intent-feedback) 14%, transparent);
		color: var(--intent-feedback);
		font-size: var(--t-label);
		font-weight: 600;
		white-space: nowrap;
	}
	.review-changes {
		color: var(--intent-authority);
	}
	.work-outputs {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 8px;
	}
	.work-now-facts button {
		margin-left: auto;
		padding: 0;
		border: 0;
		background: none;
		color: var(--intent-conversation);
		font: inherit;
		font-weight: 600;
		cursor: pointer;
	}
	ul {
		display: grid;
		gap: 2px;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	li {
		display: flex;
		align-items: baseline;
		gap: 6px;
		min-width: 0;
		padding: 4px 0;
	}
	li a {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		color: var(--ink);
		text-decoration: none;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	li a:hover {
		text-decoration: underline;
	}
	li small {
		flex: none;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	li.done :global(svg) {
		flex: none;
		align-self: center;
		color: var(--intent-feedback);
	}
	.work-empty {
		margin: 0;
		color: var(--text-tertiary);
	}
	.work-all {
		justify-self: start;
		margin-top: 8px;
		color: var(--intent-conversation);
		font-size: var(--t-label);
		font-weight: 600;
		text-decoration: none;
	}
</style>
