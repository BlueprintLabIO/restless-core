/* On Cloud's one address the account service answers every /account path (ADR 0007, Cloud ADR
 * 0008): it draws every account page and mounts this plane's own sections into them. A navigation
 * to one of them leaves this app with a full load, which the edge sends to the account service. */

/** Whether a path is an account page, which the account service draws on the one address. */
export function accountServicePath(pathname: string): boolean {
	return pathname === '/account' || pathname.startsWith('/account/');
}
