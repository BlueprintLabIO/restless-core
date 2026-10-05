import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

/* Each account section is its own page; the first is Connections. The grant deep link keeps its query. */
export const load: PageLoad = ({ url }) => {
	redirect(308, `/account/connections${url.search}`);
};
