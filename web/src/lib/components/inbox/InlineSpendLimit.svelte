<script lang="ts">
	/* The spend limit, raised inside its Inbox card (S101): what is spent, the limit, and a new one
	 * with quick steps. Saving answers the warning or restarts stopped work. */
	import { companyQuery } from '$lib/model/queries.svelte';
	import { saveSpendLimit } from '$lib/model/company';
	import { failureSentence } from '$lib/model/failure';

	let {
		companyId,
		owner,
		ondone
	}: { companyId: string; owner: boolean; ondone: (receipt: string) => void } = $props();

	const source = $derived(companyQuery(companyId, () => owner));
	$effect(() => source.attach());
	const spend = $derived(source.view?.limits.spend ?? null);
	let limit = $state('');
	let saving = $state(false);
	let error = $state('');
	const money = (usd: number) => `US$${usd.toFixed(usd < 100 ? 2 : 0)}`;
	const steps = $derived(
		spend ? [1.5, 2, 3].map((factor) => Math.ceil((spend.ceiling_usd * factor) / 10) * 10).filter((value, i, all) => all.indexOf(value) === i) : []
	);

	async function save(value: string) {
		if (!spend || saving) return;
		const ceiling = Number(value);
		if (!Number.isFinite(ceiling) || ceiling <= spend.accounted_usd) {
			error = `Enter more than ${money(spend.accounted_usd)}, what is already spent${spend.period === 'monthly' ? ' this month' : ''}.`;
			return;
		}
		saving = true;
		error = '';
		try {
			const result = await saveSpendLimit(companyId, {
				ceiling: String(ceiling),
				expected_ceiling: String(spend.ceiling_usd),
				period: spend.period,
				warning_percent: spend.warning_percent
			});
			source.accept(result);
			ondone(`Spend limit raised to ${money(ceiling)}${spend.period === 'monthly' ? ' a month' : ''}. Work carries on.`);
		} catch (cause) {
			error = failureSentence(cause, 'The limit was not saved. Try again.');
		} finally {
			saving = false;
		}
	}
</script>

<section class="inline-limit" aria-label="Spend limit">
	{#if !owner}
		<p class="quiet">The owner sets the spend limit.</p>
	{:else if spend}
		{@const share = Math.min(100, (spend.accounted_usd / Math.max(spend.ceiling_usd, 0.01)) * 100)}
		<div class="line">
			<span>{money(spend.accounted_usd)} of {money(spend.ceiling_usd)}{spend.period === 'monthly' ? ' this month' : ''}</span>
			<span class="figure" class:full={share >= 100}>{Math.round(share)}%</span>
		</div>
		<div class="meter" class:full={share >= 100} role="meter" aria-label="Spend used" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(share)}>
			<span style:width={`${share}%`}></span>
		</div>
		<form
			class="raise"
			onsubmit={(event) => {
				event.preventDefault();
				void save(limit);
			}}
		>
			<span class="quiet">Raise to</span>
			{#each steps as step (step)}
				<button class="btn small" type="button" disabled={saving} onclick={() => void save(String(step))}>{money(step)}</button>
			{/each}
			<label class="custom"
				><span class="sr-only">Another limit in US dollars</span><input
					inputmode="decimal"
					placeholder="Other"
					bind:value={limit}
					disabled={saving}
				/></label
			>
			<button class="btn small primary" disabled={saving || !limit.trim()}>{saving ? 'Saving…' : 'Save'}</button>
		</form>
		{#if error}<p class="error" role="alert">{error}</p>{/if}
	{/if}
</section>

<style>
	.inline-limit {
		display: grid;
		gap: 8px;
		margin-top: 12px;
		padding: 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
	}
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
	.raise {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
	}
	.custom input {
		width: 7em;
	}
	.quiet {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.quiet,
	.error {
		margin: 0;
	}
	.error {
		color: var(--state-danger);
		font-size: var(--t-label);
	}
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
	}
</style>
