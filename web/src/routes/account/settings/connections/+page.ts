import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

/* Account settings moved onto the one Account page. */
export const load: PageLoad = ({ url }) => {
	redirect(308, `/account${url.search}#connections`);
};
