import { error, redirect } from '@sveltejs/kit';
import { parsePlatformContext } from '$lib/platform';
import type { LayoutLoad } from './$types';

export const load: LayoutLoad = async ({ fetch, url }) => {
	const response = await fetch('/api/platform', {
		credentials: 'same-origin',
		cache: 'no-store'
	});
	if (response.status === 401) {
		const next = `${url.pathname}${url.search}`;
		throw redirect(307, `/account?next=${encodeURIComponent(next)}`);
	}
	if (!response.ok) {
		throw error(response.status, 'Unable to verify this company access');
	}
	const platform = parsePlatformContext(await response.json());
	if (platform.mode === 'cloud_fleet') throw redirect(307, platform.navigation.portfolioHref);
	return {};
};
