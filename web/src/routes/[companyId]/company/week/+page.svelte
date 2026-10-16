<script lang="ts">
	/* What the company did this week and what it cost, in one place: Work it finished, what was
	 * decided, what reached the outside world, and the week's model spend. Counting these is
	 * deterministic, so this page lists them; the account of what they mean is Exec's, which it
	 * writes up as a document on request. */
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { Page, Section, Item, Empty, Notice, Row } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { page } from '$app/state';
	import { attentionQuery, companySpendQuery } from '$lib/model/queries.svelte';
	import { fetchDecisionHistory, sendActorMessage } from '$lib/model/attention';
	import { getGovernanceActivity, type CompanyExternalAction } from '$lib/model/company';
	import { failureSentence } from '$lib/model/failure';
	import { formatMoment, formatMoney } from '$lib/ui/time';
	import { toast } from '$lib/ui/feedback/toast.svelte';
	import type { DecisionContinuation } from '$lib/model/view';
	import { holdQueries } from '$lib/model/hold-queries.svelte';

	const companyId = $derived(page.params.companyId ?? '');
	const attention = $derived(attentionQuery(companyId, true, true));
	/* Only the spend summary: an admin reads this page too, and the whole company view is the
	 * owner's. */
	const company = $derived(companySpendQuery(companyId));
	const since = Date.now() - 7 * 86_400_000;
	const inWeek = (at: string | number | Date | undefined | null) =>
		!!at && new Date(at).getTime() >= since;

	const finished = $derived(
		(attention.view?.workGraph?.work ?? [])
			.filter((work) => work.status === 'completed' && inWeek(work.updated_at))
			.sort((a, b) => b.updated_at.localeCompare(a.updated_at))
	);

	let decisions = $state<DecisionContinuation[]>([]);
	let sent = $state<CompanyExternalAction[]>([]);
	let loaded = $state(false);
	let failure = $state('');
	$effect(() => {
		const id = companyId;
		loaded = false;
		failure = '';
		void Promise.all([fetchDecisionHistory(id), getGovernanceActivity(id)])
			.then(([history, record]) => {
				decisions = history.decisions.filter((decision) => inWeek(decision.observedAt));
				sent = record.items.filter(
					(item) => item.source !== 'owner_setting' && inWeek(item.observed_at)
				);
			})
			.catch((cause) => (failure = failureSentence(cause, 'This week could not be read.')))
			.finally(() => (loaded = true));
	});

	const spend = $derived(company.view?.spend ?? null);
	const failed = $derived(
		sent.filter((item) => item.state === 'failed' || item.state === 'unknown')
	);

	let asking = $state(false);
	async function writeUp() {
		if (asking) return;
		asking = true;
		try {
			await sendActorMessage(
				companyId,
				'exec',
				'Write up this week’s review as a document in the Library: what the company finished, what was decided, what reached the outside world (and anything that failed), and what it cost, with what you would change next week. Share it with me when it is ready.'
			);
			toast('Exec is writing this week’s review');
		} catch (cause) {
			toast(failureSentence(cause, 'Exec was not asked.'), { tone: 'failure' });
		} finally {
			asking = false;
		}
	}
	holdQueries(() => [company]);
</script>

<CompanyTitle title="Weekly review" {companyId} />

<Page
	title="Weekly review"
	info="The last seven days: Work the company finished, what was decided, what reached the outside world, and what it cost. Exec can write it up as a document."
>
	{#snippet actions()}<button
			class="btn small"
			type="button"
			disabled={asking}
			title="Exec writes the week up as a document in the Library, with what it would change next week"
			onclick={() => void writeUp()}>{asking ? 'Asking…' : 'Write it up with Exec'}</button
		>{/snippet}
	{#if failure}<Notice tone="danger" title={failure} />{/if}
	<Section
		title="Spent"
		info="Model spend over the last seven days, and the limit it counts against."
	>
		{#if spend}
			<Row label="Last 7 days">{formatMoney(spend.last_7_days_usd ?? 0)}</Row>
			<Row label={spend.period === 'monthly' ? 'This month' : 'So far'}
				>{formatMoney(spend.accounted_usd)} of {formatMoney(spend.ceiling_usd)}</Row
			>
		{:else}<Skeleton label="Reading spend…" count={2} />{/if}
	</Section>
	<Section title="Finished" count={finished.length || null}>
		{#each finished as work (work.id)}<Item
				title={work.title}
				meta={`Finished ${formatMoment(work.updated_at)}`}
				href={`/${encodeURIComponent(companyId)}/work/${encodeURIComponent(work.id)}`}
			/>{:else}<Empty compact title="Nothing finished this week" />{/each}
	</Section>
	<Section title="Decided" count={decisions.length || null}>
		{#if !loaded}<Skeleton label="Reading decisions…" count={2} />{:else}
			{#each decisions as decision (decision.id)}<Item
					title={decision.title}
					meta={decision.recordedDecision}
					href={decision.workId
						? `/${encodeURIComponent(companyId)}/work/${encodeURIComponent(decision.workId)}`
						: `/${encodeURIComponent(companyId)}/company/activity?show=decisions&open=${encodeURIComponent(decision.id)}`}
				/>{:else}<Empty compact title="No decisions this week" />{/each}
		{/if}
	</Section>
	<Section
		title="Reached the outside world"
		count={sent.length || null}
		info={failed.length
			? `${failed.length} failed or have no known result; Activity has each receipt.`
			: 'Emails, submissions, payments and other actions with a receipt.'}
	>
		{#if !loaded}<Skeleton label="Reading actions…" count={2} />{:else}
			{#each sent as item (item.id)}<Item
					title={item.title}
					meta={`${item.party ? `${item.party} · ` : ''}${item.state.replaceAll('_', ' ')}`}
					href={`/${encodeURIComponent(companyId)}/company/activity?show=external&open=${encodeURIComponent(item.id)}`}
				/>{:else}<Empty compact title="Nothing reached the outside world this week" />{/each}
		{/if}
	</Section>
</Page>
