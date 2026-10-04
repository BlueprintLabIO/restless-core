import { redirect } from '@sveltejs/kit';

/* Documents and sheets live in the Library now. */
export function load({ params, url }) {
	redirect(307, `/${params.companyId}/library/sheets${url.search}`);
}
