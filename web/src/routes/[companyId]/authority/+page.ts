import { redirect } from '@sveltejs/kit';

/* Access & limits moved; old links land there before anything renders. */
export function load({ params }) {
	redirect(307, `/${params.companyId}/company/resources#limits`);
}
