<script lang="ts">
	/* Where the money goes, expanded.
	 *
	 * The Ops pane shows per-employee bars against a share of the month's run spend. This adds
	 * the two projections behind it: cost attribution (which outcomes the effort actually
	 * served) and a runway forecast (how long the cash lasts, under whose assumptions).
	 *
	 * The unmetered-runs caveat travels with the numbers. A subscription-billed driver records
	 * a cost of 0, which is measured-as-nothing, not free — the pane says so and so must this. */

	import { page } from '$app/state';
	import PaneHeader from '$lib/primitives/PaneHeader.svelte';
	import { composeCostAttribution } from '$lib/model/cost-attribution';
	import { composeRunwayForecast } from '$lib/model/runway-forecast';
	import { cosmon, spendInputs } from '$lib/fixtures/cosmon';

	const desk = cosmon;
	const companyId = $derived(page.params.companyId ?? desk.company.id);
	const hq = desk.hq;
	const currency = desk.company.currency;

	const attribution = composeCostAttribution({
		runs: spendInputs.runs,
		work: spendInputs.work,
		goals: spendInputs.goals
	});

	const forecast = composeRunwayForecast({
		cashOnHandCents: hq.cashCents,
		currency,
		monthlyBudgetCents: hq.budgetCents,
		monthlyRecordedBurnCents: hq.spendCents,
		openCommitmentsCents: spendInputs.openCommitmentsCents
	});

	function money(cents: number): string {
		return new Intl.NumberFormat(undefined, { style: 'currency', currency }).format(cents / 100);
	}

	const spendShare = hq.team.reduce((total, m) => total + m.spendCents, 0) || 1;
</script>

<svelte:head><title>Spend — {desk.company.name}</title></svelte:head>

<div class="bridge-page bridge-bleed bridge-ops">
	<div class="page-head">
		<div style="display: flex; align-items: center; gap: 10px">
			<a class="btn small" href="/{companyId}/ops">‹ Ops</a>
			<h1>Where the money goes</h1>
		</div>
	</div>

	<div class="pane-frame">
		<div class="metric-row">
			<div class="metric">
				<div class="m-label">Cash on hand</div>
				<div class="m-value">{money(hq.cashCents)}</div>
				<div class="m-sub">recorded treasury balance</div>
			</div>
			<div class="metric">
				<div class="m-label">Spend this month</div>
				<div class="m-value">{money(hq.spendCents)}</div>
				<div class="m-sub">of {money(hq.budgetCents)} budget</div>
			</div>
			<div class="metric">
				<div class="m-label">Open commitments</div>
				<div class="m-value">{money(spendInputs.openCommitmentsCents)}</div>
				<div class="m-sub">future obligations recorded</div>
			</div>
		</div>

		<div class="pane-row op-body">
			<section class="pane op-pane">
				<PaneHeader
					title="By outcome"
					hint={attribution.note}
					hintLabel="How run cost is attributed to goals"
				/>
				{#each attribution.byGoal as row (row.goalId ?? 'unattributed')}
					<div style="margin-bottom: 10px">
						<div class="kv" style="padding-bottom: 2px">
							<span>{row.goalTitle}</span>
							<b>
								{attribution.metered ? money(row.recordedCents) : '—'}
								<span class="caption">{row.runCount} run{row.runCount === 1 ? '' : 's'}</span>
							</b>
						</div>
						<div class="bar">
							<span
								style={`width: ${Math.min(100, Math.round((row.runCount / Math.max(1, attribution.totalRuns)) * 100))}%`}
							></span>
						</div>
					</div>
				{:else}
					<p class="caption">No runs recorded yet, so nothing has cost anything.</p>
				{/each}
				{#if !attribution.metered && attribution.totalRuns > 0}
					<!-- The bars are run COUNTS here, not money: with nothing metered, drawing them as
					     spend would render zero as a fact about cost rather than about measurement. -->
					<p class="caption" style="margin-top: 6px">
						Bars show share of runs, not money — no run has metered a cost.
					</p>
				{/if}
			</section>

			<div class="pane-rail">
				<section class="pane op-pane">
					<PaneHeader
						title="By employee"
						hint="Unmetered subscription runs record 0 — measured-as-nothing, not free."
						hintLabel="How run spend is recorded"
					/>
					{#each hq.team as member (member.id)}
						<div style="margin-bottom: 10px">
							<div class="kv" style="padding-bottom: 2px">
								<span>{member.name}</span>
								<b
									>{money(member.spendCents)}
									<span class="caption">/ {money(member.limitCents)}</span></b
								>
							</div>
							<div class="bar">
								<span
									style={`width: ${Math.min(100, Math.round((member.spendCents / spendShare) * 100))}%; background: var(--pig-${member.pig})`}
								></span>
							</div>
						</div>
					{:else}
						<p class="caption">Nothing recorded yet.</p>
					{/each}
				</section>

				<section class="pane op-pane">
					<PaneHeader
						title="Runway"
						hint={forecast.disclaimer}
						hintLabel="What a runway forecast is and is not"
					/>
					{#each forecast.scenarios as scenario (scenario.key)}
						<div class="kv">
							<span>{scenario.label}</span>
							<b>{scenario.runwayMonths} mo</b>
						</div>
						<p class="caption" style="margin: 0 0 8px">
							at {money(scenario.monthlyOutflowCents)} / month · {scenario.confidence} confidence
						</p>
					{:else}
						<p class="caption">
							No runway can be estimated — nothing is recorded to estimate it from.
						</p>
					{/each}
					{#each forecast.assumptions as assumption (assumption)}
						<p class="caption" style="margin: 0">{assumption}</p>
					{/each}
				</section>
			</div>
		</div>

		<section class="pane op-pane">
			<PaneHeader title="The general ledger" />
			<p class="caption">
				Not shown here, and deliberately not approximated. A double-entry view needs account
				postings; the projection behind this page carries treasury movements instead, so drawing
				a ledger from it would mean inventing the postings. It stays unbuilt and named.
			</p>
		</section>
	</div>
</div>
