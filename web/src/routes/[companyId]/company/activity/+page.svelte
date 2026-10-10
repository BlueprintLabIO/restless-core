<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	/* One timeline of what was decided and what reached the outside world: your
	 * recorded decisions and the company's governed external effects, newest
	 * first, filterable. Internal chats, builds and edits stay with their Work. */
	import { formatRelative, formatMoment } from '$lib/ui/time';
	import { Page, Section, Item, Notice, Empty, Segmented, Dot } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { page } from '$app/state';
	import { untrack } from 'svelte';
	import { attentionQuery, companyQuery } from '$lib/model/queries.svelte';
	import { fetchDecisionHistory } from '$lib/model/attention';
	import { failureSentence } from '$lib/model/failure';
	import type { DecisionContinuation } from '$lib/model/view';
	import CircleCheck from '@lucide/svelte/icons/circle-check';
	import Send from '@lucide/svelte/icons/send';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const attention = $derived(attentionQuery(companyId));
	const company = $derived(companyQuery(companyId));
	$effect(() => company.attach());
	const graph = $derived(attention.view?.workGraph ?? null);
	let filter = $state<'all' | 'decisions' | 'external'>('all');
	let open = $state('');

	/* Decision history is read a page at a time, apart from the Inbox poll: every answered
	 * item and every approval or decline, however old, one "Show earlier" away. */
	let history = $state<DecisionContinuation[]>([]);
	let historyNext = $state<string | undefined>(undefined);
	let historyLoaded = $state(false);
	let historyFailure = $state<unknown>(null);
	let historyReading = $state(false);
	let historyCompany = '';
	let waitingSeen: number | undefined;
	function mergeHistory(rows: DecisionContinuation[]) {
		const byId = new Map(history.map((row) => [row.id, row]));
		for (const row of rows) byId.set(row.id, row);
		history = [...byId.values()].sort(
			(a, b) => Date.parse(String(b.observedAt)) - Date.parse(String(a.observedAt))
		);
	}
	async function readHistory(before?: string) {
		const target = companyId;
		historyReading = true;
		try {
			const result = await fetchDecisionHistory(target, before);
			if (target !== companyId) return;
			if (!historyLoaded || before) historyNext = result.nextBefore;
			mergeHistory(result.decisions);
			historyFailure = null;
			historyLoaded = true;
		} catch (cause) {
			if (target === companyId) historyFailure = cause;
		} finally {
			historyReading = false;
		}
	}
	$effect(() => {
		const target = companyId;
		/* An answer leaves the Inbox and lands here; read the newest page again when it does. */
		const waiting = attention.view?.items.length;
		untrack(() => {
			if (target !== historyCompany) {
				historyCompany = target;
				history = [];
				historyNext = undefined;
				historyLoaded = false;
				historyFailure = null;
				waitingSeen = waiting;
				void readHistory();
				return;
			}
			if (waiting !== undefined && waitingSeen !== undefined && waiting !== waitingSeen)
				void readHistory();
			if (waiting !== undefined) waitingSeen = waiting;
		});
	});

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
					responsibleLabel: string;
					workId?: string;
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
		history.map((decision) => ({
			kind: 'decision',
			id: decision.id,
			title:
				(decision.workId && graph?.work.find((work) => work.id === decision.workId)?.title) ||
				decision.title,
			summary: decision.recordedDecision,
			at: String(decision.observedAt),
			detail: {
				unlocked: decision.whatItUnlocked,
				state: decision.currentState,
				outcome: decision.observedOutcome,
				/* An approval names who prepared the draft; answered Work names who holds it now. */
				responsible:
					decision.responsibleActor?.display ?? (decision.workId ? 'No further owner' : ''),
				responsibleLabel: decision.workId ? 'Responsible now' : 'Prepared by',
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
	const loaded = $derived(!!company.view && (historyLoaded || !!historyFailure));

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
		{#if company.failure}<Notice
				tone="danger"
				title="Activity could not be read"
				details={String(company.failure?.message ?? '')}
			>
				{#snippet actions()}<button
						class="btn small"
						onclick={() => {
							void company.refresh();
							void readHistory();
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
		{#if historyFailure && filter !== 'external'}<Notice
				tone="warning"
				title="Decisions could not be read"
				details={failureSentence(historyFailure, 'Your decision history was not read.')}
			>
				{#snippet actions()}<button
						class="btn small"
						disabled={historyReading}
						onclick={() => void readHistory()}>Retry</button
					>{/snippet}
			</Notice>{/if}
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
								{#if entry.detail.unlocked}<div>
										<dt>What it unlocked</dt>
										<dd>{entry.detail.unlocked}</dd>
									</div>{/if}
								{#if entry.detail.state}<div>
										<dt>Where it stands</dt>
										<dd>{entry.detail.state}</dd>
									</div>{/if}
								{#if entry.detail.outcome}<div>
										<dt>What happened</dt>
										<dd>{entry.detail.outcome}</dd>
									</div>{/if}
								{#if entry.detail.responsible}<div>
										<dt>{entry.detail.responsibleLabel}</dt>
										<dd>{entry.detail.responsible}</dd>
									</div>{/if}
								{#if entry.detail.workId}<a
										class="btn small"
										href={`/${companyId}/work/${entry.detail.workId}`}>Open work</a
									>{/if}
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
		{#if historyNext && filter !== 'external'}
			<div class="earlier">
				<button
					class="btn small"
					disabled={historyReading}
					title="Older answers and approvals, a page at a time"
					onclick={() => void readHistory(historyNext)}
					>{historyReading ? 'Reading…' : 'Show earlier decisions'}</button
				>
			</div>
		{/if}
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
	.earlier {
		display: flex;
		justify-content: center;
		padding: var(--space-3) 0;
	}
	@container page (max-width: 560px) {
		.facts div {
			grid-template-columns: 1fr;
			gap: 2px;
		}
	}
</style>
