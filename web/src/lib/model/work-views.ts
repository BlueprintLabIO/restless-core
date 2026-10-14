/* What an empty Work view says. "Waiting on you", "In progress", "Archived" and "Other work" are
 * filters, not goals, so each says what it found nothing of. Dependency-free for node's tests. */
export function emptyWorkTitle(view: string): string {
	switch (view) {
		case '':
			return 'No work yet';
		case 'waiting':
			return 'Nothing is waiting on you.';
		case 'active':
			return 'Nothing is in progress.';
		case 'archived':
			return 'Nothing is archived.';
		case 'other':
			return 'No other work.';
		case 'stopped':
			return 'Nothing is stopped.';
		default:
			return 'No work under this goal yet';
	}
}
