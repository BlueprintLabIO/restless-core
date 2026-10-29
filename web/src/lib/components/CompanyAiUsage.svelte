<script lang="ts">
	import { Section, Row } from '$lib/ui/page';
	import { spendReportQuery } from '$lib/model/spend.svelte';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import { formatMoney } from '$lib/ui/time';
	let { companyId }: { companyId: string } = $props();
	const source = spendReportQuery(
		() => companyId,
		() => ({ days: 7 }),
		() => true,
		() => ['resources']
	);
	holdQueries(() => [source]);
	const totals = $derived(source.view?.totals);
	const count = (value: number) => new Intl.NumberFormat().format(value);
	function tokens(value: number | undefined, missing = 0): string {
		if (value == null || !totals) return '—';
		if (totals.calls > 0 && missing >= totals.calls) return 'Not reported';
		return `${missing > 0 ? 'At least ' : ''}${count(value)}`;
	}
</script>

<Section
	id="ai-usage"
	title="AI usage, last 7 days"
	info="The last seven days, from the existing company usage records. Missing provider counts stay unknown. Spend and native sign-in limits are controlled below."
>
	{#snippet actions()}<a class="btn small" href={`/${companyId}/company/tokens`}>Details</a
		>{/snippet}
	<Row label="Input tokens">{tokens(totals?.input_tokens, totals?.calls_without_split)}</Row>
	<Row label="Output tokens">{tokens(totals?.output_tokens, totals?.calls_without_split)}</Row>
	<Row
		label="Cached input"
		info="Input the provider reported serving from cache. This is a token count, not a claimed dollar saving."
		>{tokens(totals?.cached_input_tokens, totals?.calls_without_cache)}</Row
	>
	<Row
		label="API-equivalent estimate"
		info="An estimate from model prices, including subscription-backed work where available. It is not an invoice or a subscription quota."
	>
		{totals?.estimated_usd == null
			? 'Not reported'
			: `${totals.calls_without_estimate > 0 ? 'At least ' : ''}${formatMoney(totals.estimated_usd)}`}
	</Row>
	<Row
		label="Usage coverage"
		info="A call can lack token counts even when its price was recorded. Unknown usage does not mean free usage."
	>
		{#if source.failure}<span>Could not refresh</span><button
				class="btn small"
				onclick={() => source.refresh()}>Retry</button
			>
		{:else if !totals}Reading…
		{:else if totals.calls_without_tokens > 0}{count(totals.calls_without_tokens)} calls without token
			counts
		{:else}{count(totals.calls)} calls recorded{/if}
	</Row>
</Section>
