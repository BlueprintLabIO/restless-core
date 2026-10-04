<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	/* One timeline of what was decided and what reached the outside world: your
	 * recorded decisions and the company's governed external effects, newest
	 * first, filterable. Internal chats, builds and edits stay with their Work. */
	import { formatRelative, formatMoment } from '$lib/ui/time';
	import { Page, Section, Item, Notice, Empty, Segmented, Dot } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { page } from '$app/state';
	import { attentionQuery, companyQuery } from '$lib/model/queries.svelte';
	import CircleCheck from '@lucide/svelte/icons/circle-check';
	import Send from '@lucide/svelte/icons/send';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const attention = $derived(attentionQuery(companyId));
	const company = $derived(companyQuery(companyId));
	$effect(() => company.attach());
	const graph = $derived(attention.view?.workGraph ?? null);
	let filter = $state<'all' | 'decisions' | 'external'>('all');
	let open = $state('');

	type Entry =
		| {
				kind: 'decision';
				id: string;
				title: string;
				summary: string;
				at: string;
				detail: {
					unlocked: string;
					state: string;
					outcome: string;
					responsible: string;
					workId: string;
				};
		  }
		| {
				kind: 'external';
				id: string;
				title: string;
				summary: string;
				at: string;
				state: string;
				evidence: string;
				detail: string | null;
				actor: string;
		  };

	const decisions = $derived<Entry[]>(
		(attention.view?.continuations ?? []).map((decision) => ({
			kind: 'decision',
			id: decision.id,
			title: graph?.work.find((work) => work.id === decision.workId)?.title ?? decision.title,
			summary: decision.recordedDecision,
			at: String(decision.observedAt),
			detail: {
				unlocked: decision.whatItUnlocked,
				state: decision.currentState,
				outcome: decision.observedOutcome,
				responsible: decision.responsibleActor?.display ?? 'No further owner',
				workId: decision.workId
			}
		}))
	);
	const externalAvailable = $derived(company.view?.external_actions.status === 'available');
	const external = $derived<Entry[]>(
		externalAvailable
			? (company.view?.external_actions.items ?? []).map((item) => ({
					kind: 'external',
					id: item.id,
					title: item.title,
					summary: `${words(item.effect_class)}${item.party ? ` · ${item.party}` : ''}`,
					at: String(item.observed_at),
					state: item.state,
					evidence: item.evidence,
					detail: item.detail ?? null,
					actor: item.actor ?? 'source record'
				}))
			: []
	);
	const entries = $derived(
		[...(filter !== 'external' ? decisions : []), ...(filter !== 'decisions' ? external : [])].sort(
			(a, b) => Date.parse(b.at) - Date.parse(a.at)
		)
	);
	const loaded = $derived(!!attention.view && !!company.view);

	function words(value: string): string {
		return value.replaceAll('_', ' ');
	}
	function evidenceExplanation(value: string): string {
		return (
			{
				provider_confirmed: 'Confirmed by the provider.',
				self_attested: 'Recorded locally; the provider has not independently confirmed it.',
				reconciled: 'A later status check closed an earlier unknown outcome.',
				legacy_unverified: 'An older record from before receipts were checked.',
				unknown: 'Started, but no result has been recorded yet.',
				authority_recorded: 'Submitted; no provider result yet.'
			}[value] ?? 'Evidence as reported by its source.'
		);
	}
	const stateTone = (state: string) =>
		state === 'succeeded' || state === 'completed'
			? 'success'
			: state === 'failed' || state === 'rejected'
				? 'danger'
				: state === 'unknown'
					? 'warning'
					: 'neutral';
	const when = (value: string) => formatRelative(value, '');
</script>

<CompanyTitle title="Activity" {companyId} />

<Page
	title="Activity"
	info="Decisions you recorded and actions the company took in the outside world: messages, submissions, payments. Internal chats, builds and edits stay with their Work."
>
	{#snippet actions()}
		{#if attention.view?.items.length}<a class="btn small" href={`/${companyId}`}
				>{attention.view.items.length} waiting in Inbox</a
			>{/if}
		<Segmented
			label="Show"
			value={filter}
			options={[
				{ value: 'all', label: 'All' },
				{ value: 'decisions', label: 'Decisions' },
				{ value: 'external', label: 'External' }
			]}
			onchange={(value) => (filter = value)}
		/>
	{/snippet}

	{#if !loaded}
		{#if attention.failure || company.failure}<Notice
				tone="danger"
				title="Activity could not be read"
				details={String((attention.failure ?? company.failure)?.message ?? '')}
			>
				{#snippet actions()}<button
						class="btn small"
						onclick={() => {
							void attention.refresh();
							void company.refresh();
						}}>Retry</button
					>{/snippet}
			</Notice>
		{:else}<Skeleton label="Reading activity…" variant="page" count={4} />{/if}
	{:else}
		{#if !externalAvailable && filter !== 'decisions'}<Notice
				tone="warning"
				title="External activity is unavailable right now"
				details="Authority is not answering, so external history is not being shown as empty."
			/>{/if}
		<Section count={entries.length || null}>
			{#each entries as entry (entry.kind + entry.id)}
				<Item
					title={entry.title}
					meta={entry.summary}
					onclick={() => (open = open === entry.id ? '' : entry.id)}
					selected={open === entry.id}
				>
					{#snippet leading()}
						{#if entry.kind === 'decision'}<CircleCheck size={15} strokeWidth={1.8} />{:else}<Send
								size={15}
								strokeWidth={1.8}
							/>{/if}
					{/snippet}
					{#snippet trailing()}
						{#if entry.kind === 'external'}<Dot
								show
								tone={stateTone(entry.state)}
								label={words(entry.state)}
							/>{/if}
						<time title={formatMoment(entry.at)}>{when(entry.at)}</time>
					{/snippet}
					{#if open === entry.id}
						<dl class="facts">
							{#if entry.kind === 'decision'}
								<div>
									<dt>Your decision</dt>
									<dd>{entry.summary}</dd>
								</div>
								<div>
									<dt>What it unlocked</dt>
									<dd>{entry.detail.unlocked}</dd>
								</div>
								<div>
									<dt>Where it stands</dt>
									<dd>{entry.detail.state}</dd>
								</div>
								<div>
									<dt>What happened</dt>
									<dd>{entry.detail.outcome}</dd>
								</div>
								<div>
									<dt>Responsible now</dt>
									<dd>{entry.detail.responsible}</dd>
								</div>
								<a class="btn small" href={`/${companyId}/work/${entry.detail.workId}`}>Open work</a
								>
							{:else}
								<div>
									<dt>Evidence</dt>
									<dd title={evidenceExplanation(entry.evidence)}>
										{words(entry.evidence)} · {evidenceExplanation(entry.evidence)}
									</dd>
								</div>
								<div>
									<dt>By</dt>
									<dd>{entry.actor}</dd>
								</div>
								{#if entry.detail}<div>
										<dt>Detail</dt>
										<dd>{entry.detail}</dd>
									</div>{/if}
							{/if}
						</dl>
					{/if}
				</Item>
			{:else}
				<Empty
					compact
					title={filter === 'external'
						? 'No external activity yet'
						: filter === 'decisions'
							? 'No decisions recorded yet'
							: 'Nothing recorded yet'}
					info="Messages sent, forms submitted, payments made and decisions you record appear here."
				/>
			{/each}
		</Section>
	{/if}
</Page>

<style>
	.facts {
		display: grid;
		gap: 10px;
		margin: 0;
		padding-top: 4px;
	}
	.facts div {
		display: grid;
		grid-template-columns: 140px minmax(0, 1fr);
		gap: var(--space-3);
	}
	.facts dt {
		color: var(--text-tertiary);
		font-size: var(--t-body);
	}
	.facts dd {
		margin: 0;
		color: var(--ink);
		font-size: var(--t-body);
		line-height: 1.55;
		overflow-wrap: anywhere;
	}
	.facts .btn {
		justify-self: start;
	}
	@container page (max-width: 560px) {
		.facts div {
			grid-template-columns: 1fr;
			gap: 2px;
		}
	}
</style>
