import { error, redirect } from '@sveltejs/kit';
import { companyPageAlias, companyPageHref } from '$lib/model/company-pages';
import type { PageLoad } from './$types';

/* A Company page named by its label or key (`/company/health`, `/company/brand`) lands on the
 * page itself; anything else is still "This page does not exist". */
export const load: PageLoad = ({ params, url }) => {
	const target = companyPageAlias(params.alias);
	if (!target) error(404, 'Not found');
	redirect(308, `${companyPageHref(encodeURIComponent(params.companyId), target)}${url.search}`);
};
