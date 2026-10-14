import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

/* A piece of Work lands on its agent's page in People, highlighted in their Work panel (S88-06).
 * This address stays so links already sent keep working; "talk" named the agent to open. */
export const load: PageLoad = ({ params, url }) => {
	const query = new URLSearchParams({ work: params.workId });
	const talk = url.searchParams.get('talk');
	if (talk) query.set('person', talk);
	redirect(307, `/${encodeURIComponent(params.companyId)}/people?${query}`);
};
