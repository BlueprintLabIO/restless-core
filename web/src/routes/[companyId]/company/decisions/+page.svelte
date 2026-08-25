<script lang="ts">
	import { page } from '$app/state';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import { attentionQuery } from '$lib/model/queries.svelte';
	import SemanticMark from '$lib/primitives/SemanticMark.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(attentionQuery(companyId));
	const view = $derived(source.view);
	const decisions = $derived(view?.continuations ?? []);
	const graph = $derived(view?.workGraph ?? null);

	function workTitle(workId: string, fallback: string): string {
		return graph?.work.find((work) => work.id === workId)?.title ?? fallback;
	}

	function when(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		return date.toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			year: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<svelte:head><title>Decision history — {view?.company.name ?? companyId}</title></svelte:head>

<div class="company-page decisions-page">
	<header class="company-page-head">
		<div class="decision-history-heading">
			<h1>Decision history</h1>
			<InfoTip
				text="Recorded owner decisions and the work they actually released. This is a read-only projection of source-owned company state."
			/>
		</div>
		<div class="company-page-freshness">
			<span class="source-lamp status-{source.status}" aria-hidden="true"></span>
			{source.status === 'live'
				? `${decisions.length} recorded`
				: source.status === 'stale'
					? 'Last observation'
					: 'Reading decisions'}
		</div>
	</header>

	{#if view}
		{#if decisions.length}
			<section class="company-decision-ledger" aria-label="Recorded owner decisions">
				{#each decisions as decision (decision.id)}
					<details class="company-decision">
						<summary title="Show what this decision unlocked and what happened next">
							<span class="company-decision-mark" aria-hidden="true">
								<SemanticMark meaning="success" size="small" />
							</span>
							<strong>{workTitle(decision.workId, decision.title)}</strong>
							<time>{when(decision.observedAt)}</time>
							<span class="company-decision-disclosure" aria-hidden="true"></span>
						</summary>
						<div class="company-decision-body">
							<dl>
								<div>
									<dt>Recorded decision</dt>
									<dd>{decision.recordedDecision}</dd>
								</div>
								<div>
									<dt>What it unlocked</dt>
									<dd>{decision.whatItUnlocked}</dd>
								</div>
								<div>
									<dt>Current state</dt>
									<dd>{decision.currentState}</dd>
								</div>
								<div>
									<dt>Observed outcome</dt>
									<dd>{decision.observedOutcome}</dd>
								</div>
							</dl>
							<footer>
								<div>
									<span>Responsible now</span>
									<strong>{decision.responsibleActor?.display ?? 'No further owner'}</strong>
								</div>
								<a class="btn small" href={`/${companyId}/work/${decision.workId}`}>Inspect Work</a>
							</footer>
						</div>
					</details>
				{/each}
			</section>
		{:else}
			<p class="quiet-empty">No owner decisions have been recorded yet.</p>
		{/if}
	{:else if source.failure}
		<div class="company-source-error" role="alert">{source.failure.message}</div>
	{:else}
		<div class="company-page-wait" aria-label="Reading decision history"></div>
	{/if}
</div>
