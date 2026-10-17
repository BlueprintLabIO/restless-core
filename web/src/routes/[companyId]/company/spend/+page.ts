import { redirect } from '@sveltejs/kit';
export const load = ({ params, url }) =>
	redirect(307, `/${encodeURIComponent(params.companyId)}/company/tokens${url.search}`);
