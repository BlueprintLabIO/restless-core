<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	/* One timeline of what was decided and what reached the outside world: your
	 * recorded decisions and the company's governed external effects, newest
	 * first, filterable. Internal chats, builds and edits stay with their Work. */
	import { formatMoment, formatDay, formatClock } from '$lib/ui/time';
	import { Page, Section, Item, Notice, Empty, Segmented, Dot } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { page } from '$app/state';
	import { untrack } from 'svelte';
	import { attentionQuery, companyQuery } from '$lib/model/queries.svelte';
	import { fetchDecisionHistory } from '$lib/model/attention';
	import { getGovernanceActivity, type CompanyExternalAction } from '$lib/model/company';
	import { failureSentence } from '$lib/model/failure';
	import type { DecisionContinuation } from '$lib/model/view';
	import CircleCheck from '@lucide/svelte/icons/circle-check';
	import Send from '@lucide/svelte/icons/send';
	import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal';
	import { holdQueries } from '$lib/model/hold-queries.svelte';

	const companyId = $derived(page.params.companyId ?? '');
	const attention = $derived(attentionQuery(companyId));
	const company = $derived(companyQuery(companyId));
	$effect(() => company.attach());
	const graph = $derived(attention.view?.workGraph ?? null);
	/* The Inbox's "Recently decided" arrives with ?show=decisions; a link to one entry (an
	 * outside action that failed, a decision someone else made) with ?q= and ?open=. */
	const shown = page.url.searchParams.get('show');
	let filter = $state<'all' | 'decisions' | 'external' | 'changes'>(
		shown === 'decisions' || shown === 'external' || shown === 'changes' ? shown : 'all'
	);
	let search = $state(page.url.searchParams.get('q') ?? '');
	let person = $state('');
	/* One day, as YYYY-MM-DD from the date field; empty shows every day. */
	let day = $state('');
	let open = $state(page.url.searchParams.get('open') ?? '');

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
	/* The governance record (receipts, email, the owner's setting changes) is paged too, so
	 * nothing older than one page silently drops out. */
	let governance = $state<CompanyExternalAction[]>([]);
	let governanceNext = $state<number | undefined>(undefined);
	let governanceLoaded = $state(false);
	let governanceFailure = $state<unknown>(null);
	let governanceReading = $state(false);
	async function readGovernance(before?: number) {
		const target = companyId;
		governanceReading = true;
		try {
			const result = await getGovernanceActivity(target, before);
			if (target !== companyId) return;
			if (!governanceLoaded || before != null) governanceNext = result.next_before;
			const byId = new Map(governance.map((row) => [row.id, row]));
			for (const row of result.items) byId.set(row.id, row);
			governance = [...byId.values()];
			governanceFailure = null;
			governanceLoaded = true;
		} catch (cause) {
			if (target === companyId) governanceFailure = cause;
		} finally {
			governanceReading = false;
		}
	}
	$effect(() => {
		const target = companyId;
		untrack(() => {
			governance = [];
			governanceNext = undefined;
			governanceLoaded = false;
			governanceFailure = null;
			void readGovernance();
		});
		return () => void target;
	});
	const reading = $derived(historyReading || governanceReading);
	const hasEarlier = $derived(
		(filter !== 'external' && filter !== 'changes' && !!historyNext) ||
			(filter !== 'decisions' && governanceNext != null)
	);
	async function readEarlier() {
		await Promise.all([
			filter !== 'external' && filter !== 'changes' && historyNext
				? readHistory(historyNext)
				: null,
			filter !== 'decisions' && governanceNext != null ? readGovernance(governanceNext) : null
		]);
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
					/** Who to follow up with: whoever holds it now, or Exec for an approval. */
					followUpId: string;
					followUpName: string;
					workId?: string;
				};
		  }
		| {
				kind: 'external' | 'change';
				id: string;
				title: string;
				summary: string;
				at: string;
				state: string;
				evidence: string;
				detail: string | null;
				actor: string;
				workId?: string;
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
				followUpId: decision.responsibleActor?.id ?? 'exec',
				followUpName: decision.responsibleActor?.display ?? 'Exec',
				workId: decision.workId
			}
		}))
	);
	/* Payments and effects still awaiting a result come from the company view, which always holds
	 * all of them; everything else comes from the paged record. */
	const externalAvailable = $derived(governanceLoaded);
	const external = $derived<Entry[]>(
		[
			...(company.view?.external_actions.status === 'available'
				? company.view.external_actions.items.filter(
						(item) => item.id.startsWith('intent:') || item.id.startsWith('payment:')
					)
				: []),
			...governance
		].map((item) => ({
			kind: item.source === 'owner_setting' ? 'change' : 'external',
			id: item.id,
			title: item.title,
			summary:
				item.source === 'owner_setting'
					? 'Setting changed'
					: `${words(item.effect_class)}${item.party ? ` · ${item.party}` : ''}`,
			at: String(item.observed_at),
			state: item.state,
			evidence: item.evidence,
			detail: item.detail ?? null,
			actor: item.actor ?? 'source record',
			workId: item.work_id
		}))
	);
	const who = (entry: Entry) =>
		entry.kind === 'decision' ? entry.detail.followUpName : entry.actor;
	const people = $derived([...new Set([...decisions, ...external].map(who))].sort());
	const localDay = (at: string) => {
		const date = new Date(at);
		return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
	};
	const entries = $derived.by(() => {
		const q = search.trim().toLocaleLowerCase();
		return [
			...(filter === 'all' || filter === 'decisions' ? decisions : []),
			...external.filter(
				(entry) =>
					filter === 'all' ||
					(filter === 'external' && entry.kind === 'external') ||
					(filter === 'changes' && entry.kind === 'change')
			)
		]
			.filter(
				(entry) =>
					(!person || who(entry) === person) &&
					(!day || localDay(entry.at) === day) &&
					(!q || `${entry.title} ${entry.summary} ${who(entry)}`.toLocaleLowerCase().includes(q))
			)
			.sort((a, b) => Date.parse(b.at) - Date.parse(a.at));
	});
	/* Grouped under day headings, so "what happened yesterday" is a heading, not hovering. */
	function dayHeading(key: string): string {
		const today = localDay(new Date().toISOString());
		const yesterday = localDay(new Date(Date.now() - 86_400_000).toISOString());
		if (key === today) return 'Today';
		if (key === yesterday) return 'Yesterday';
		return formatDay(`${key}T12:00:00`);
	}
	const days = $derived.by(() => {
		const out: { key: string; entries: Entry[] }[] = [];
		for (const entry of entries) {
			const key = localDay(entry.at);
			const last = out.at(-1);
			if (last?.key === key) last.entries.push(entry);
			else out.push({ key, entries: [entry] });
		}
		return out;
	});
	/* A chosen day older than what is loaded reads earlier pages until it is reached. */
	let seeking = 0;
	$effect(() => {
		if (!day || reading || !hasEarlier || seeking >= 10) return;
		const oldest = [...decisions, ...external].reduce(
			(min, entry) => (entry.at < min ? entry.at : min),
			'9999'
		);
		if (oldest !== '9999' && localDay(oldest) <= day) return;
		seeking += 1;
		untrack(() => void readEarlier());
	});
	$effect(() => {
		day;
		seeking = 0;
	});
	const filtered = $derived(!!(search.trim() || person || day));
	const loaded = $derived(
		!!company.view &&
			(historyLoaded || !!historyFailure) &&
			(governanceLoaded || !!governanceFailure)
	);

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
	holdQueries(() => [attention, company]);
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
				{ value: 'external', label: 'External' },
				{ value: 'changes', label: 'Changes' }
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
				details={governanceFailure
					? failureSentence(governanceFailure, 'Authority is not answering.')
					: 'Authority is not answering, so external history is not being shown as empty.'}
			>
				{#snippet actions()}<button
						class="btn small"
						disabled={governanceReading}
						onclick={() => void readGovernance()}>Retry</button
					>{/snippet}
			</Notice>{/if}
		<div class="filters" role="search">
			<SlidersHorizontal size={14} strokeWidth={1.8} aria-hidden="true" />
			<input
				type="search"
				class="text"
				placeholder="Search activity"
				aria-label="Search activity"
				bind:value={search}
			/>
			<select aria-label="Person" bind:value={person}>
				<option value="">Anyone</option>
				{#each people as name (name)}<option value={name}>{name}</option>{/each}
			</select>
			<input
				type="date"
				aria-label="Day"
				bind:value={day}
				max={localDay(new Date().toISOString())}
			/>
			{#if filtered}<button
					class="btn small ghost"
					type="button"
					onclick={() => {
						search = '';
						person = '';
						day = '';
					}}>Clear</button
				>{/if}
		</div>
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
		{#each days as group (group.key)}
			<Section title={dayHeading(group.key)} count={group.entries.length}>
				{#each group.entries as entry (entry.kind + entry.id)}
					<Item
						title={entry.title}
						meta={entry.summary}
						onclick={() => (open = open === entry.id ? '' : entry.id)}
						selected={open === entry.id}
					>
						{#snippet leading()}
							{#if entry.kind === 'decision'}<CircleCheck
									size={15}
									strokeWidth={1.8}
								/>{:else if entry.kind === 'change'}<SlidersHorizontal
									size={15}
									strokeWidth={1.8}
								/>{:else}<Send size={15} strokeWidth={1.8} />{/if}
						{/snippet}
						{#snippet trailing()}
							{#if entry.kind === 'external'}<Dot
									show
									tone={stateTone(entry.state)}
									label={words(entry.state)}
								/>{/if}
							<time datetime={entry.at} title={formatMoment(entry.at)}>{formatClock(entry.at)}</time
							>
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
									<div class="follow-up">
										<a
											class="btn small"
											href={`/${encodeURIComponent(companyId)}/people?person=${encodeURIComponent(entry.detail.followUpId)}&view=people`}
											title="Change or add to your answer: a message in your conversation with them"
											>Follow up with {entry.detail.followUpName}</a
										>
										{#if entry.detail.workId}<a
												class="btn small"
												href={`/${companyId}/work/${entry.detail.workId}`}>Open work</a
											>{/if}
									</div>
								{:else if entry.kind === 'change'}
									<div>
										<dt>By</dt>
										<dd>{entry.actor}</dd>
									</div>
									<div>
										<dt>When</dt>
										<dd>{formatMoment(entry.at)}</dd>
									</div>
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
									{#if entry.workId}<div class="follow-up">
											<a
												class="btn small"
												href={`/${encodeURIComponent(companyId)}/work/${encodeURIComponent(entry.workId)}`}
												>Open work</a
											>
										</div>{/if}
								{/if}
							</dl>
						{/if}
					</Item>
				{/each}
			</Section>
		{:else}
			<Section>
				<Empty
					compact
					title={filtered
						? 'Nothing matches'
						: filter === 'external'
							? 'No external activity yet'
							: filter === 'decisions'
								? 'No decisions recorded yet'
								: filter === 'changes'
									? 'No setting changes yet'
									: 'Nothing recorded yet'}
					info="Messages sent, forms submitted, payments made, settings you change and decisions you record appear here."
				/>
			</Section>
		{/each}
		{#if hasEarlier}
			<div class="earlier">
				<button class="btn small" disabled={reading} onclick={() => void readEarlier()}
					>{reading ? 'Reading…' : 'Show earlier'}</button
				>
			</div>
		{/if}
	{/if}
</Page>

<style>
	.follow-up {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}
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
	.filters {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-2);
		margin-bottom: var(--space-3);
		color: var(--text-tertiary);
	}
	.filters .text {
		flex: 1 1 180px;
		min-width: 0;
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
