<script lang="ts">
	import { Section, Row } from '$lib/ui/page';
	import UsageChart from '$lib/ui/data/UsageChart.svelte';
	import { spendReportQuery, planLine } from '$lib/model/spend.svelte';
	import { companySpendQuery } from '$lib/model/queries.svelte';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import { formatMoney } from '$lib/ui/time';
	let { companyId }: { companyId: string } = $props();
	const DAYS = 30;
	const source = spendReportQuery(
		() => companyId,
		() => ({ days: DAYS }),
		() => true,
		() => ['resources']
	);
	const spendSource = $derived(companySpendQuery(companyId));
	holdQueries(() => [source, spendSource]);
	const report = $derived(source.view);
	const totals = $derived(report?.totals);
	const plan = $derived(report?.plan ?? []);
	const budget = $derived(spendSource.view?.spend);
	const count = (value: number) => new Intl.NumberFormat().format(value);
	function tokens(value: number | undefined, missing = 0): string {
		if (value == null || !totals) return '—';
		if (totals.calls > 0 && missing >= totals.calls) return 'Not reported';
		return `${missing > 0 ? 'At least ' : ''}${count(value)}`;
	}
	/* Cached input as a share of input: the cache is what keeps long conversations cheap. */
	const cacheShare = $derived(
		totals && totals.input_tokens > 0 && totals.calls_without_cache < totals.calls
			? `${Math.round((100 * totals.cached_input_tokens) / totals.input_tokens)}% of input`
			: null
	);
	const days = $derived(
		(report?.by_day ?? []).map((day) => ({ date: day.key, value: day.totals.estimated_usd ?? 0 }))
	);
	const rate = (value: number) =>
		value >= 1 ? `$${value.toFixed(2)}` : `$${value.toFixed(value >= 0.1 ? 2 : 3)}`;
	const prices = $derived((report?.prices ?? []).slice(0, 3));
	const priced = $derived(
		report?.pricing_updated_at
			? new Date(report.pricing_updated_at).toLocaleDateString(undefined, {
					day: 'numeric',
					month: 'short'
				})
			: null
	);
</script>

<Section
	id="ai-usage"
	title="AI usage, last 30 days"
	info="From this company's usage records. Missing provider counts stay unknown. The budget and native sign-in limits are set under Spend."
>
	{#snippet actions()}<a class="btn small" href={`/${companyId}/company/tokens`}>Details</a
		>{/snippet}
	<div class="chart">
		<UsageChart
			{days}
			span={DAYS}
			height={112}
			format={(value) => formatMoney(value)}
			label="API-key AI cost, cumulative over 30 days"
		/>
	</div>
	<Row
		label="On subscription"
		info="Calls through a Claude or ChatGPT sign-in. They count against the plan's usage windows, not dollars; the share is this company's part of each window, as the provider last reported it."
	>
		<span class="numbers plan"
			>{!totals
				? '—'
				: `${count(totals.subscription_tokens ?? 0)} tokens${plan.length ? ` · ${plan.map(planLine).join(' · ')}` : ''}`}</span
		>
	</Row>
	<Row
		label="API-key cost"
		info="Calls billed per token, priced at the current model catalogue rates. It is not an invoice. Subscription calls are not in it."
	>
		<span class="numbers"
			>{!totals
				? '—'
				: totals.calls === totals.subscription_calls
					? formatMoney(0)
					: totals.estimated_usd == null
						? 'Not reported'
						: `${totals.calls_without_estimate > 0 ? 'At least ' : ''}${formatMoney(totals.estimated_usd)}`}</span
		>
	</Row>
	<Row
		label="Budget"
		info="Known paid model spend against this company's model budget. New paid work pauses at the limit; change it under Spend."
	>
		<span class="numbers"
			>{budget
				? `${formatMoney(budget.accounted_usd)} of ${formatMoney(budget.ceiling_usd)} ${budget.period === 'monthly' ? 'this month' : 'in total'}`
				: '—'}</span
		><a class="btn small ghost" href="#spend">Change</a>
	</Row>
	<Row label="Input · output tokens">
		<span class="numbers"
			>{tokens(totals?.input_tokens, totals?.calls_without_split)} · {tokens(
				totals?.output_tokens,
				totals?.calls_without_split
			)}</span
		>
	</Row>
	<Row
		label="Cached input"
		info="Input the provider reported serving from cache, which it charges at its lower cached rate. A token count, not a claimed saving."
	>
		<span class="numbers"
			>{tokens(totals?.cached_input_tokens, totals?.calls_without_cache)}{cacheShare
				? ` · ${cacheShare}`
				: ''}</span
		>
	</Row>
	<Row
		label="Catalogue prices"
		info={`Per million tokens (input · cached input · output), from the public model catalogue the estimate uses${priced ? `, updated ${priced}` : ''}. Before taxes and provider surcharges.`}
	>
		<span class="numbers prices"
			>{#if prices.length}{prices
					.map(
						(price) =>
							`${price.model.split('/').at(-1)} ${rate(price.input_per_million)} · ${price.cached_input_per_million == null ? '—' : rate(price.cached_input_per_million)} · ${rate(price.output_per_million)}`
					)
					.join('; ')}{:else}{report ? 'No priced model use' : '—'}{/if}</span
		>
	</Row>
	<Row
		label="Usage coverage"
		info="A call can lack token counts even when its price was recorded. Unknown usage does not mean free usage."
	>
		{#if source.failure}<span>Could not refresh</span><button
				class="btn small"
				onclick={() => source.refresh()}>Retry</button
			>
		{:else if !totals}—
		{:else if totals.calls_without_tokens > 0}{count(totals.calls_without_tokens)} calls without token
			counts
		{:else}{count(totals.calls)} calls recorded{/if}
	</Row>
</Section>

<style>
	.chart {
		padding: 16px 16px 8px;
	}
	.numbers {
		font-variant-numeric: tabular-nums;
	}
	.prices,
	.plan {
		display: block;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
