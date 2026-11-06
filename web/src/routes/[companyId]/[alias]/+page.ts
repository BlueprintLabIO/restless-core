import { error, redirect } from '@sveltejs/kit';
import { companyAlias } from '$lib/model/company-pages';
import type { PageLoad } from './$types';

/* A company address typed by a surface's or page's name (`/inbox`, `/settings`, `/health`)
 * lands where that lives; anything else is still "This page does not exist". */
export const load: PageLoad = ({ params, url }) => {
	const target = companyAlias(params.alias);
	if (target === undefined) error(404, 'Not found');
	redirect(308, `/${encodeURIComponent(params.companyId)}${target}${url.search}`);
};
