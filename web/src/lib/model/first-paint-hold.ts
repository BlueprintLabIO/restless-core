/* Which cached reads hold the cockpit's first reveal (Sprint 82); see first-paint.svelte.ts. */

export const CRITICAL = new Set([
	'companies',
	'appliance',
	'company-principal',
	'attention',
	'cockpit',
	'company-collaboration',
	'company',
	/* A company without a model shows the connect card first; it appears with its form, never as
	 * a status line that grows into one (S109 C7). */
	'reusable-connections',
	/* The Inbox's Today line sits above its rows. */
	'direction-digest',
	/* Work's goals and its phone view picker come from the Work graph; arriving after the reveal,
	 * the picker pushed the list down (zero-jank gate, owner/slow/phone/warm/work). */
	'work-graph'
]);

/* Reads the shell starts on every page that only one surface shows: they hold the reveal there
 * and nowhere else (3 Nov walkthrough: a direct load of Resources or the Library waited on the
 * Inbox's own reads). A route's own reads hold it by being read only where they are shown. */
const ONLY_ON: Record<string, string> = {
	'direction-digest': 'inbox',
	'work-graph': 'work'
};

/* Reads whose answer can take seconds, where they may stop holding the reveal, and after how long.
 * The company read carries Health's live checks: on Cloud it took 8–10 s, and the page waited
 * blank all that time (6 Nov). The Company pages have a skeleton in its place, so there the shell
 * appears and the read fills the page in. Elsewhere it keeps holding: the Inbox detail lays out
 * from it, and released early its folds moved (zero-jank gate, slow Inbox). */
export const HOLDS_AT_MOST: Record<string, { surface: string; ms: number }> = {
	company: { surface: 'company', ms: 2000 }
};

type CachedRead = {
	queryKey: readonly unknown[];
	state: { data: unknown; status: string; fetchStatus: string };
};

/* A persisted Attention summary is a placeholder, not an answer: it omits live-only sources, so
 * the page treats it as not yet loaded and adds the clear pill and the floor when the live read
 * lands. Showing it would reveal the page one step early. */
function placeholder(read: CachedRead): boolean {
	if (read.queryKey[0] !== 'attention') return false;
	const health = (read.state.data as { sourceHealth?: { authority?: string } } | undefined)
		?.sourceHealth;
	return health?.authority === 'unknown';
}

/** The surface a company path shows: `inbox` for the company's root, else its first segment. */
export function surfaceOf(pathname: string, companyId: string): string {
	const rest = pathname.slice(companyId.length + 2).split('/')[0];
	return rest || 'inbox';
}

/** Whether one cached read still holds the reveal: a critical read for this company (or the
 * account-wide ones) fetching its first answer, or fetching past a persisted placeholder. A read
 * that only one surface shows holds it only on that surface, and a slow one (HOLDS_AT_MOST) only
 * until the reveal has waited `waitedMs` that long. */
export function holdsReveal(
	read: CachedRead,
	companyId: string,
	surface = 'inbox',
	waitedMs = 0
): boolean {
	const key = String(read.queryKey[0]);
	if (!CRITICAL.has(key)) return false;
	if (ONLY_ON[key] && ONLY_ON[key] !== surface) return false;
	const limit = HOLDS_AT_MOST[key];
	if (limit && limit.surface === surface && waitedMs >= limit.ms) return false;
	if (!['companies', 'appliance'].includes(key) && read.queryKey[1] !== companyId) return false;
	// A failed read has a recovery state to show. A retry of its persisted
	// placeholder must not hide the whole app again while the source is down.
	if (read.state.status === 'error') return false;
	if (read.state.fetchStatus !== 'fetching') return false;
	return (read.state.data === undefined && read.state.status === 'pending') || placeholder(read);
}
