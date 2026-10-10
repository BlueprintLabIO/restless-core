/* On Cloud's one address (ADR 0007) the account service answers every /account path except the
 * sections this plane owns: Home, Profile, Security, Plan and Support are its pages. The cockpit's
 * own legacy redirects for /account and /account/settings would otherwise swallow a click on them
 * and land the owner back on Connections, so a navigation to one of them leaves this app with a
 * full load, which the edge sends to the account service. */
const PLANE_ACCOUNT_PAGES = [
	'/account/connections',
	'/account/ai-apps',
	'/account/appearance',
	'/account/entry',
	'/account/settings/connections',
	'/account/settings/ai-apps',
	'/account/settings/appearance'
];

/** Whether the account service, not this plane, answers this path on the one address. */
export function accountServicePath(pathname: string): boolean {
	if (pathname !== '/account' && !pathname.startsWith('/account/')) return false;
	return !PLANE_ACCOUNT_PAGES.some((page) => pathname === page || pathname.startsWith(`${page}/`));
}
