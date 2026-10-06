import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

/* Connections and Skills are Apps now (Sprint 63). Old links keep working. */
export const load: PageLoad = ({ params }) => {
	redirect(308, `/${encodeURIComponent(params.companyId)}/apps`);
};
