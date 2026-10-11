import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

/* The old Rooms list is part of People now; its links keep working. */
export const load: PageLoad = ({ params, url }) => {
	redirect(307, `/${encodeURIComponent(params.companyId)}/people${url.search}`);
};
