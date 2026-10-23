import { matches } from '../search/search.ts';

/* What an empty Work view says. "Needs you", "In progress", "Archived" and "No goal" are
 * filters, not goals, so each says what it found nothing of. A search or filter that found nothing
 * says that, never that the company has no work. Runs under node's own tests. */
export function emptyWorkTitle(view: string, search = '', filtered = false): string {
	const query = search.trim();
	if (query) return `No work matches “${query}”`;
	if (filtered) return 'No work matches these filters';
	switch (view) {
		case '':
			return 'No work yet';
		case 'waiting':
			return 'Nothing needs you.';
		case 'active':
			return 'Nothing is in progress.';
		case 'archived':
			return 'Nothing is archived.';
		case 'other':
			return 'All work serves a goal.';
		case 'stopped':
			return 'Nothing is stopped.';
		default:
			return 'No work under this goal yet';
	}
}

/* What a search finds: the words the owner sees on a row (title, brief, owner, goal and output
 * names), matched as every search in the app matches (lib/search). */
export function matchesSearch(query: string, fields: readonly (string | null | undefined)[]) {
	return matches(query, fields);
}

/** Ready for an explicit archive choice: every task is complete, with at least one task.
 * A progress count alone never changes a goal's lifecycle. */
export function goalsWithFinishedWork(
	goals: readonly { id: string; closed_at?: string | null }[],
	work: readonly { goal_id?: string | null; status: string }[]
): Set<string> {
	const byGoal = new Map<string, { total: number; unfinished: boolean }>();
	for (const item of work) {
		if (!item.goal_id) continue;
		const state = byGoal.get(item.goal_id) ?? { total: 0, unfinished: false };
		state.total++;
		state.unfinished ||= item.status !== 'completed';
		byGoal.set(item.goal_id, state);
	}
	return new Set(
		goals
			.filter(
				(goal) =>
					!goal.closed_at &&
					(byGoal.get(goal.id)?.total ?? 0) > 0 &&
					!byGoal.get(goal.id)?.unfinished
			)
			.map((goal) => goal.id)
	);
}
