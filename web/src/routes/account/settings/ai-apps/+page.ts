import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

/* Account settings moved to a page per section. */
export const load: PageLoad = ({ url }) => {
	redirect(307, `/account/connections${url.search}`);
};
