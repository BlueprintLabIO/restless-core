/* Driver kinds billed by subscription rather than by token. A run on one of these
 * records a cost of 0, which is measured-as-nothing — NOT free. Every surface that
 * renders these numbers has to carry that distinction or it reports a falsehood. */
export const UNMETERED_SUBSCRIPTION_DRIVER_KINDS = ['codex-acp', 'claude-acp'] as const;

/**
 * Attribute run effort and cost to the outcome that caused it.
 *
 * Every AI run is caused by a work item, and most work items serve a goal. This module rolls the
 * runs — their count and their recorded cost — up to the goal that ultimately caused them, so the
 * owner can see where the company's agent effort is going. It derives everything from records the
 * desk already fetched and invents nothing.
 *
 * The honesty this module keeps is the same seam as the per-turn provenance: the local subscription
 * drivers (codex-acp / claude-acp) meter no tokens or per-run cost, so recorded cost is 0 by design
 * — measured-as-nothing, not free. `metered` says whether any run actually metered a cost, so a row
 * of zeros is never read as "this outcome was free"; the run **counts** are always real effort. A
 * metered production provider is the seam where real model/tool cost would flow in per run, and it
 * would roll up here unchanged.
 */

const UNMETERED_KINDS: ReadonlySet<string> = new Set(UNMETERED_SUBSCRIPTION_DRIVER_KINDS);

/** The label for runs whose work serves no goal (or whose work is not in view). */
export const UNATTRIBUTED_GOAL_LABEL = 'Not tied to a goal';

export interface GoalCostAttribution {
	/** The goal these runs roll up to, or null when the work serves no goal. */
	goalId: string | null;
	goalTitle: string;
	runCount: number;
	recordedCents: number;
}

export interface CostAttribution {
	byGoal: GoalCostAttribution[];
	/** Total runs and recorded cost across all goals (equals the company's run activity). */
	totalRuns: number;
	totalRecordedCents: number;
	/** True only if any run actually metered a cost; false for the unmetered local drivers. */
	metered: boolean;
	note: string;
}

export interface CostAttributionInput {
	runs: readonly { workItemId: string; costCents: number; driverProbe: unknown }[];
	work: readonly { id: string; goalId?: string | null }[];
	goals: readonly { id: string; title: string }[];
}

function probeKind(probe: unknown): string | null {
	if (probe && typeof probe === 'object' && 'kind' in probe) {
		const value = (probe as Record<string, unknown>).kind;
		if (typeof value === 'string' && value.length > 0) return value;
	}
	return null;
}

/**
 * Composes run-effort-and-cost attribution per goal. Pure and deterministic; goals with no runs do
 * not appear (only outcomes that actually caused effort), and the list is ordered by run count so
 * the biggest consumer is first.
 */
export function composeCostAttribution(input: CostAttributionInput): CostAttribution {
	const goalByWork = new Map(input.work.map((item) => [item.id, item.goalId ?? null]));
	const goalTitleById = new Map(input.goals.map((goal) => [goal.id, goal.title]));

	const tallies = new Map<string | null, { runCount: number; recordedCents: number }>();
	let totalRuns = 0;
	let totalRecordedCents = 0;
	let metered = false;
	for (const run of input.runs) {
		const goalId = goalByWork.has(run.workItemId) ? goalByWork.get(run.workItemId)! : null;
		const tally = tallies.get(goalId) ?? { runCount: 0, recordedCents: 0 };
		tally.runCount += 1;
		tally.recordedCents += run.costCents;
		tallies.set(goalId, tally);
		totalRuns += 1;
		totalRecordedCents += run.costCents;
		const kind = probeKind(run.driverProbe);
		if (kind !== null && !UNMETERED_KINDS.has(kind) && run.costCents > 0) metered = true;
	}

	const byGoal: GoalCostAttribution[] = [...tallies.entries()]
		.map(([goalId, tally]) => ({
			goalId,
			goalTitle:
				goalId === null
					? UNATTRIBUTED_GOAL_LABEL
					: (goalTitleById.get(goalId) ?? UNATTRIBUTED_GOAL_LABEL),
			runCount: tally.runCount,
			recordedCents: tally.recordedCents
		}))
		.sort((left, right) => right.runCount - left.runCount);

	return {
		byGoal,
		totalRuns,
		totalRecordedCents,
		metered,
		note: metered
			? 'Recorded cost was metered for at least one run.'
			: 'Runs used personal-subscription drivers, which meter no tokens or per-run cost, so recorded cost is 0 by design; the run counts are real effort.'
	};
}
