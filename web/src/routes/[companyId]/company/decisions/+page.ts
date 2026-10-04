import { redirect } from '@sveltejs/kit';

/* Decision history and external activity are one Activity timeline now. */
export function load({ params }) {
	redirect(307, `/${params.companyId}/company/activity`);
}
