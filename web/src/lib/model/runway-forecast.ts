/**
 * Runway forecast: how long recorded cash lasts, presented as an ESTIMATE — never as ledger truth.
 *
 * The item this serves ("present forecasts, runway, and scenarios as estimates with assumptions and
 * confidence, distinct from ledger truth") is a truthfulness discipline first and a calculation second.
 * So this module keeps a hard line between what is recorded and what is projected:
 *
 * - `cashOnHandCents` is LEDGER TRUTH — the recorded treasury balance, surfaced verbatim.
 * - Everything else (runway months, per-scenario outflow) is an ESTIMATE derived from an explicit
 *   assumption, carried alongside a stated confidence and a plain-language assumption string.
 * - Confidence is honest, not decorative: a scenario built on the owner's monthly budget is a PLAN
 *   ('low'), a scenario built on this month's recorded run spend is a partial MEASUREMENT ('indicative'),
 *   and a scenario whose outflow is zero or unknown cannot be estimated at all ('none', runway null with
 *   a reason) rather than reporting an infinite or invented runway.
 * - Local run drivers meter zero cost (see the cost-attribution honesty rule), so a zero recorded burn
 *   is reported as "not measured yet", never as "free" or "infinite runway".
 *
 * Nothing here mutates anything; it is a pure projection over figures the desk already derives.
 */

export type ForecastConfidence = 'none' | 'low' | 'indicative';

export interface RunwayScenario {
	key: 'budgeted' | 'recorded_burn';
	label: string;
	/** The monthly cash outflow this scenario assumes, in cents. */
	monthlyOutflowCents: number;
	/** Months of cash at this outflow (one decimal), or null when it cannot be estimated. */
	runwayMonths: number | null;
	/** Present only when runwayMonths is null: why no runway could be estimated. */
	unavailableReason: string | null;
	/** The plain-language assumption this scenario rests on. */
	assumption: string;
	confidence: ForecastConfidence;
}

export interface RunwayForecast {
	/** Ledger truth: recorded cash on hand in the company's primary currency. */
	cashOnHandCents: number;
	currency: string;
	/** Recorded open commitments (future obligations) — shown as context, not subtracted from runway. */
	openCommitmentsCents: number;
	scenarios: RunwayScenario[];
	/** Assumptions shared by every scenario. */
	assumptions: string[];
	/** The estimate-vs-truth disclaimer this whole view carries. */
	disclaimer: string;
}

export interface RunwayForecastInput {
	/** Recorded cash on hand in the primary currency, in cents (ledger truth). */
	cashOnHandCents: number;
	currency: string;
	/** The owner-set monthly budget for the company, in cents. */
	monthlyBudgetCents: number;
	/** Cash outflow actually recorded this month (e.g. run spend), in cents. */
	monthlyRecordedBurnCents: number;
	/** Total recorded open commitments (future obligations), in cents. */
	openCommitmentsCents: number;
}

const DISCLAIMER =
	'A forecast, not ledger truth. Cash on hand is recorded; every runway below is an estimate that ' +
	'changes with real spend and income.';

const COMMON_ASSUMPTIONS = [
	'Assumes a constant monthly outflow and no new income.',
	'Counts only cash movements posted to Helm’s treasury; money held elsewhere is not included.'
];

/** Months of cash at a given monthly outflow, to one decimal. Out-of-cash reads as 0, never negative. */
function runwayMonths(cashOnHandCents: number, monthlyOutflowCents: number): number {
	if (cashOnHandCents <= 0) return 0;
	return Math.round((cashOnHandCents / monthlyOutflowCents) * 10) / 10;
}

/**
 * Composes the runway forecast. Pure and deterministic in its input.
 */
export function composeRunwayForecast(input: RunwayForecastInput): RunwayForecast {
	const scenarios: RunwayScenario[] = [];

	// Scenario 1 — against the owner's monthly budget. This is a PLAN (a ceiling the company set), so
	// even a healthy number is only 'low' confidence: it says what happens IF spend fills the budget.
	if (input.monthlyBudgetCents > 0) {
		scenarios.push({
			key: 'budgeted',
			label: 'At your monthly budget',
			monthlyOutflowCents: input.monthlyBudgetCents,
			runwayMonths: runwayMonths(input.cashOnHandCents, input.monthlyBudgetCents),
			unavailableReason: null,
			assumption: 'Assumes you spend your full monthly budget every month.',
			confidence: 'low'
		});
	} else {
		scenarios.push({
			key: 'budgeted',
			label: 'At your monthly budget',
			monthlyOutflowCents: 0,
			runwayMonths: null,
			unavailableReason: 'No monthly budget is set, so a budget-based runway cannot be estimated.',
			assumption: 'Assumes you spend your full monthly budget every month.',
			confidence: 'none'
		});
	}

	// Scenario 2 — against this month's recorded burn. This is a partial MEASUREMENT ('indicative'). A
	// zero recorded burn is reported as "not measured yet" (local drivers meter zero cost), never as an
	// infinite runway.
	if (input.monthlyRecordedBurnCents > 0) {
		scenarios.push({
			key: 'recorded_burn',
			label: 'At this month’s recorded spend',
			monthlyOutflowCents: input.monthlyRecordedBurnCents,
			runwayMonths: runwayMonths(input.cashOnHandCents, input.monthlyRecordedBurnCents),
			unavailableReason: null,
			assumption: 'Assumes spend continues at the rate recorded so far this month.',
			confidence: 'indicative'
		});
	} else {
		scenarios.push({
			key: 'recorded_burn',
			label: 'At this month’s recorded spend',
			monthlyOutflowCents: 0,
			runwayMonths: null,
			unavailableReason:
				'No spend has been recorded this month (local drivers meter zero cost), so a measured burn rate is not available yet.',
			assumption: 'Assumes spend continues at the rate recorded so far this month.',
			confidence: 'none'
		});
	}

	return {
		cashOnHandCents: input.cashOnHandCents,
		currency: input.currency,
		openCommitmentsCents: input.openCommitmentsCents,
		scenarios,
		assumptions: COMMON_ASSUMPTIONS,
		disclaimer: DISCLAIMER
	};
}
