<script lang="ts">
	import { formatRelative, formatMoment } from '$lib/ui/time';
	import SettingsHeader from '$lib/ui/views/SettingsHeader.svelte';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { page } from '$app/state';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import { attentionQuery } from '$lib/model/queries.svelte';
	import SemanticMark from '$lib/ui/glyph/SemanticMark.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(attentionQuery(companyId));
	const view = $derived(source.view);
	const decisions = $derived(view?.continuations ?? []);
	const graph = $derived(view?.workGraph ?? null);

	function workTitle(workId: string, fallback: string): string {
		return graph?.work.find((work) => work.id === workId)?.title ?? fallback;
	}
	function decisionOwner(id: string): string {
		const handoff = graph?.handoffs.find((row) => `orgintel:handoff:${row.id}` === id);
		return handoff?.category === 'owner_judgement' && handoff.state === 'resolved' ? 'Owner' : '';
	}

	const when = (value?: Date | string) => formatRelative(value, 'Not yet');
</script>

<svelte:head><title>Decision history — {view?.company.name ?? companyId}</title></svelte:head>

<div class="company-page decisions-page">
	<SettingsHeader
		title="Decision history"
		explanation="Your past approvals and decisions. Pending ones are in Attention."
		>{#snippet actions()}
			{#if view?.items.length}<a class="attention-crosslink" href={`/${companyId}`}
					>{view.items.length} pending →</a
				>{/if}
			<div class="company-page-freshness">
				<span class="source-lamp status-{source.status}" aria-hidden="true"></span>
				{source.status === 'live'
					? decisions.length
						? `${decisions.length} recorded`
						: 'Live'
					: source.status === 'stale'
						? 'Out of date'
						: 'Checking…'}
			</div>
		{/snippet}</SettingsHeader
	>

	{#if view}
		{#if decisions.length}
			<section class="company-decision-ledger" aria-label="Recorded owner decisions">
				{#each decisions as decision (decision.id)}
					<details class="company-decision">
						<summary title="Show what this decision unlocked and what happened next">
							<span class="company-decision-mark" aria-hidden="true">
								<SemanticMark meaning="success" size="small" />
							</span>
							<div class="decision-summary">
								<strong>{workTitle(decision.workId, decision.title)}</strong><span
									title={decision.recordedDecision}
									>{decision.recordedDecision}{#if decisionOwner(decision.id)}{' · '}{decisionOwner(
											decision.id
										)}{/if}</span
								>
							</div>
							<time title={formatMoment(decision.observedAt)}>{when(decision.observedAt)}</time>
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
								<a class="btn small" href={`/${companyId}/work/${decision.workId}`}
									>Open related work</a
								>
							</footer>
						</div>
					</details>
				{/each}
			</section>
		{:else}
			<p class="quiet-empty">No owner decisions have been recorded yet.</p>
		{/if}
	{:else if source.failure}
		<FailureNotice
			error={source.failure}
			subject="decision history"
			variant="block"
			onretry={source.refresh}
		/>
	{:else}
		<Skeleton label="Reading decision history…" variant="page" count={4} />
	{/if}
</div>

<style>
	.decision-summary {
		display: grid;
		gap: 3px;
		min-width: 0;
		flex: 1;
	}
	.decision-summary strong,
	.decision-summary span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.decision-summary span {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
</style>
