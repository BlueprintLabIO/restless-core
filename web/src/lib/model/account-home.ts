/** Resolve an explicit return first; hosted Home belongs to the account issuer.
 * Keep the creation intent and never redirect the current URL to itself. */
export function accountHomeDestination(
	current: URL,
	status: { hosted?: boolean; home_url?: string | null } | null
): string | null {
	const next = current.searchParams.get('next');
	if (next?.startsWith('/') && !next.startsWith('//')) {
		const target = new URL(next, current.origin);
		if (target.href !== current.href) return target.pathname + target.search + target.hash;
	}
	if (!status?.hosted || !status.home_url) return null;
	const target = new URL(status.home_url, current.origin);
	if (current.searchParams.get('start') === '1') target.searchParams.set('start', '1');
	return target.href === current.href ? null : target.href;
}
