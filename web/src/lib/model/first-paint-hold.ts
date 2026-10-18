/* Which cached reads hold the cockpit's first reveal (Sprint 82); see first-paint.svelte.ts. */

export const CRITICAL = new Set([
	'companies',
	'appliance',
	'company-principal',
	'attention',
	'cockpit',
	'company-collaboration',
	'company'
]);

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

/** Whether one cached read still holds the reveal: a critical read for this company (or the
 * account-wide ones) fetching its first answer, or fetching past a persisted placeholder. */
export function holdsReveal(read: CachedRead, companyId: string): boolean {
	const key = String(read.queryKey[0]);
	if (!CRITICAL.has(key)) return false;
	if (!['companies', 'appliance'].includes(key) && read.queryKey[1] !== companyId) return false;
	if (read.state.fetchStatus !== 'fetching') return false;
	return (read.state.data === undefined && read.state.status === 'pending') || placeholder(read);
}
