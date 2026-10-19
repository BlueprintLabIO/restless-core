import { error, redirect } from '@sveltejs/kit';
import { legacyAccountSection } from '$lib/model/account-sections';
import type { PageLoad } from './$types';

/* Every earlier account address, in one place: each lands on the section it became, its query kept
 * (Inbox items stored before the move carry `/account/connections?company=…&request=…`). */
export const load: PageLoad = ({ params, url }) => {
	const section = legacyAccountSection(params.legacy);
	if (!section) error(404, 'Not found');
	redirect(308, `${section.href}${url.search}`);
};
