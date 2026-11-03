/* Rough progress on Work (owner decision, 3 November 2026): the agent that created each Work
 * estimates how far it has come at each checkpoint, and that is the source of truth. Done Work is
 * 100%; stopped Work keeps its last estimate; a blocked estimate drops the number and says why.
 * No estimate shows nothing, never 0%. */

/** One recorded estimate, oldest first per Work (the owner graph and member projection). */
export type ProgressRow = {
	id: number;
	work_id: string;
	state: 'estimate' | 'blocked' | 'done' | (string & {});
	percent: number | null;
	next: string;
	estimated_by: string;
	recorded_at: string;
};

type WorkLike = { id: string; status: string; goal_id?: string | null };

export type WorkEstimate = {
	/** What the bar fills to: a blocked Work keeps its last number on the bar. */
	fill: number;
	/** The number said, or null when blocked (the words say why instead). */
	percent: number | null;
	tone: 'progress' | 'blocked' | 'done' | 'stopped';
	/** "~60% · next: export print files", "Blocked: …", "100% · done", "~30% · stopped". */
	text: string;
	/** Who said it and when, for a hover title. */
	title: string;
	/** Every number in the history, oldest first, for a small trend. */
	trend: number[];
	recordedAt: string | null;
};

function rowsFor(workId: string, rows: readonly ProgressRow[] | null | undefined): ProgressRow[] {
	return (rows ?? []).filter((row) => row.work_id === workId).toSorted((a, b) => a.id - b.id);
}

function ago(iso: string, now: number): string {
	const minutes = Math.max(0, Math.round((now - Date.parse(iso)) / 60_000));
	if (!Number.isFinite(minutes)) return '';
	if (minutes < 1) return 'just now';
	if (minutes < 60) return `${minutes} min ago`;
	const hours = Math.round(minutes / 60);
	if (hours < 24) return `${hours} h ago`;
	const days = Math.round(hours / 24);
	return `${days} ${days === 1 ? 'day' : 'days'} ago`;
}

/** A Work's current estimate, or null when nobody has estimated it and it is not done. */
export function workEstimate(
	work: WorkLike,
	rows: readonly ProgressRow[] | null | undefined,
	name: (actorId: string) => string = (id) => id,
	now = Date.now()
): WorkEstimate | null {
	const history = rowsFor(work.id, rows);
	const trend = history.flatMap((row) => (row.percent === null ? [] : [row.percent]));
	const latest = history.at(-1);
	const lastNumber = trend.at(-1) ?? null;
	const said = latest
		? `${latest.state === 'done' ? 'Recorded when it was done' : `Rough estimate by ${name(latest.estimated_by)}`}${
				ago(latest.recorded_at, now) ? `, ${ago(latest.recorded_at, now)}` : ''
			}`
		: '';
	if (work.status === 'completed')
		return {
			fill: 100,
			percent: 100,
			tone: 'done',
			text: '100% · done',
			title: said || 'Done',
			trend: trend.at(-1) === 100 ? trend : [...trend, 100],
			recordedAt: latest?.recorded_at ?? null
		};
	if (!latest) return null;
	if (work.status === 'abandoned') {
		if (lastNumber === null) return null;
		return {
			fill: lastNumber,
			percent: lastNumber,
			tone: 'stopped',
			text: `~${lastNumber}% · stopped`,
			title: `Stopped here. ${said}`,
			trend,
			recordedAt: latest.recorded_at
		};
	}
	if (latest.state === 'blocked' || latest.percent === null)
		return {
			fill: lastNumber ?? 0,
			percent: null,
			tone: 'blocked',
			text: `Blocked: ${latest.next}`,
			title: said,
			trend,
			recordedAt: latest.recorded_at
		};
	return {
		fill: latest.percent,
		percent: latest.percent,
		tone: 'progress',
		text: `~${latest.percent}% · next: ${latest.next}`,
		title: said,
		trend,
		recordedAt: latest.recorded_at
	};
}

export type GoalRollup = {
	/** Equal-weight average over the Goal's live Work, or null when nothing is estimated or done. */
	percent: number | null;
	done: number;
	total: number;
};

/** A Goal across its Work, each part counting equally: done is 100, an estimate counts as given,
 * a blocked part keeps its last number, an unestimated part counts as 0; stopped Work is left out.
 * Matches the engine's `goal_progress`. */
export function goalRollup(
	goalId: string,
	work: readonly WorkLike[],
	rows: readonly ProgressRow[] | null | undefined
): GoalRollup {
	const parts = work.filter((item) => item.goal_id === goalId && item.status !== 'abandoned');
	const done = parts.filter((item) => item.status === 'completed').length;
	const numbers = parts.map((item) => workEstimate(item, rows)?.fill ?? null);
	const percent =
		parts.length && numbers.some((value) => value !== null)
			? Math.round(numbers.reduce<number>((sum, value) => sum + (value ?? 0), 0) / parts.length)
			: null;
	return { percent, done, total: parts.length };
}

/** "3 of 4 parts done", said the same way everywhere a Goal rolls up. */
export function partsDone(rollup: GoalRollup): string {
	return `${rollup.done} of ${rollup.total} ${rollup.total === 1 ? 'part' : 'parts'} done`;
}
