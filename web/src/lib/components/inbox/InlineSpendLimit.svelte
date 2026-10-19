<script lang="ts">
	/* The spend limit, raised in its card (S101): what is spent against the limit on top, the new
	 * limit chosen below. The footer saves it, which answers the warning or restarts work. */
	import { companyQuery } from '$lib/model/queries.svelte';
	import { saveSpendLimit } from '$lib/model/company';
	import { failureSentence } from '$lib/model/failure';
	import InlinePanel from './InlinePanel.svelte';
	import InlineChoice from './InlineChoice.svelte';
	import { cardActions } from './card-actions.svelte';

	let {
		companyId,
		owner,
		ondone
	}: { companyId: string; owner: boolean; ondone: (receipt: string) => void } = $props();

	const source = $derived(companyQuery(companyId, () => owner));
	$effect(() => source.attach());
	const spend = $derived(source.view?.limits.spend ?? null);
	const money = (usd: number) => `US$${usd.toFixed(usd < 100 ? 2 : 0)}`;
	const period = $derived(spend?.period === 'monthly' ? ' a month' : '');
	const steps = $derived(
		spend
			? [1.5, 2, 3]
					.map((factor) => Math.ceil((spend.ceiling_usd * factor) / 10) * 10)
					.filter((value, index, all) => all.indexOf(value) === index)
			: []
	);
	let choice = $state<string | null>(null);
	let other = $state('');
	let error = $state('');
	$effect(() => {
		if (choice === null && steps.length) choice = String(steps[1] ?? steps[0]);
	});
	const ceiling = $derived(choice === 'other' ? Number(other) : Number(choice));
	const valid = $derived(!!spend && Number.isFinite(ceiling) && ceiling > spend.accounted_usd);

	cardActions(() => ({
		primary:
			owner && spend
				? {
						label: valid ? `Raise to ${money(ceiling)}${period}` : 'Raise the limit',
						busyLabel: 'Saving…',
						disabled: !valid,
						run: async () => {
							error = '';
							try {
								source.accept(
									await saveSpendLimit(companyId, {
										ceiling: String(ceiling),
										expected_ceiling: String(spend.ceiling_usd),
										period: spend.period,
										warning_percent: spend.warning_percent
									})
								);
								ondone(`Spend limit raised to ${money(ceiling)}${period}. Work carries on.`);
							} catch (cause) {
								error = failureSentence(cause, 'The limit was not saved. Try again.');
							}
						}
					}
				: owner && source.failure
					? { label: 'Try again', busyLabel: 'Reading…', run: () => source.refresh() }
					: null
	}));
</script>

{#if !owner}
	<InlinePanel label="Spend limit" note="The owner sets the spend limit." />
{:else if spend}
	{@const share = Math.min(100, (spend.accounted_usd / Math.max(spend.ceiling_usd, 0.01)) * 100)}
	<InlinePanel
		label="Spend limit"
		{error}
		note={choice === 'other' && other && !valid
			? `Enter more than ${money(spend.accounted_usd)}, what is already spent.`
			: ''}
	>
		{#snippet status()}
			<div class="line">
				<span>{money(spend.accounted_usd)} of {money(spend.ceiling_usd)}{spend.period === 'monthly' ? ' this month' : ''}</span>
				<span class="figure" class:full={share >= 100}>{Math.round(share)}% used</span>
			</div>
			<div class="meter" class:full={share >= 100} role="meter" aria-label="Spend used" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(share)}>
				<span style:width={`${share}%`}></span>
			</div>
		{/snippet}
		<InlineChoice
			label="New limit"
			options={[
				...steps.map((step) => ({ value: String(step), title: `${money(step)}${period}`, detail: `${Math.round((step / spend.ceiling_usd) * 10) / 10}× the current limit` })),
				{ value: 'other', title: 'Another amount' }
			]}
			bind:value={choice}
		/>
		{#if choice === 'other'}<input
				inputmode="decimal"
				bind:value={other}
				aria-label="New limit in US dollars"
				placeholder="Limit in US dollars"
			/>{/if}
	</InlinePanel>
{:else if source.failure}
	<InlinePanel label="Spend limit" note="The spend limit could not be read." />
{:else}
	<InlinePanel label="Spend limit" note="Reading the spend limit…" />
{/if}

<style>
	.line {
		display: flex;
		justify-content: space-between;
		color: var(--text-secondary);
		font-size: var(--t-label);
		font-variant-numeric: tabular-nums;
	}
	.figure.full {
		color: var(--state-danger);
	}
	.meter {
		height: 6px;
		border-radius: 99px;
		background: var(--surface-alt);
		overflow: hidden;
	}
	.meter span {
		display: block;
		height: 100%;
		background: var(--state-warning, var(--text-secondary));
	}
	.meter.full span {
		background: var(--state-danger);
	}
</style>
