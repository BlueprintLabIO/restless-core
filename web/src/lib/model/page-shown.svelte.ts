/**
 * Whether this page is being shown, for its long-lived streams. A tab hidden for a minute lets
 * its company change stream and its agents' activity go: a stream held open for nobody is a slot
 * in the company's capped set (cell_wake.rs) and, hosted, a connection through the edge. Shown
 * again, they reopen at once and the query client reads what is older than a minute.
 */

/** A quick switch to another tab and back keeps the streams. */
const RELEASE_AFTER_MS = 60_000;

export const pageShown = $state({ current: true });

if (typeof document !== 'undefined') {
	let release: ReturnType<typeof setTimeout> | undefined;
	document.addEventListener('visibilitychange', () => {
		clearTimeout(release);
		if (document.visibilityState === 'visible') pageShown.current = true;
		else release = setTimeout(() => (pageShown.current = false), RELEASE_AFTER_MS);
	});
}
