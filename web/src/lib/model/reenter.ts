/* On a hosted plane a company's session hangs off the account's. When the company session lapses
 * (the plane restarted for an update, or its own session expired) the account is usually still
 * signed in, so the way back is the account's own door: it enters the company again and lands on
 * the same page, as opening it from Home would. A local appliance has no account to ask, and a
 * person whose account session has also ended sees Home's sign-in, so this never loops: one try a
 * minute, then the page says what happened. */

const LAST_TRY = 'restless:reenter-at';
const RETRY_AFTER_MS = 60_000;

type Place = { hostname: string; pathname: string; search: string };
type Memory = Pick<Storage, 'getItem' | 'setItem'>;

export function isLocalHost(hostname: string): boolean {
	return (
		hostname === 'localhost' ||
		hostname === '127.0.0.1' ||
		hostname === '[::1]' ||
		hostname.endsWith('.localhost')
	);
}

/** Where to go to get back in, or null to show the failure instead. */
export function reenterHref(place: Place, now: number, memory?: Memory | null): string | null {
	if (isLocalHost(place.hostname)) return null;
	try {
		const stored = memory?.getItem(LAST_TRY);
		const last = stored ? Number(stored) : NaN;
		if (Number.isFinite(last) && now - last < RETRY_AFTER_MS) return null;
		memory?.setItem(LAST_TRY, String(now));
	} catch {
		/* Storage can be unavailable (private windows); one redirect is still safe. */
	}
	return `/account?return=${encodeURIComponent(`${place.pathname}${place.search}`)}`;
}

/** Leave for the account's door when a hosted company session has lapsed. True when leaving. */
export function reenterIfLapsed(status: number | undefined): boolean {
	if (status !== 401 || typeof window === 'undefined') return false;
	let memory: Storage | null = null;
	try {
		memory = window.sessionStorage;
	} catch {
		memory = null;
	}
	const href = reenterHref(window.location, Date.now(), memory);
	if (!href) return false;
	window.location.replace(href);
	return true;
}
