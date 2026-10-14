/* What an empty Work view says. "Needs you", "In progress", "Archived" and "No goal" are
 * filters, not goals, so each says what it found nothing of. A search or filter that found nothing
 * says that, never that the company has no work. Dependency-free for node's tests. */
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

/* What a search finds: any of the words the owner sees on a row (title, brief, owner, goal and
 * output names), case-folded. */
export function matchesSearch(query: string, fields: readonly (string | null | undefined)[]) {
	const needle = query.trim().toLocaleLowerCase();
	return !needle || fields.some((field) => field?.toLocaleLowerCase().includes(needle));
}
