import { redirect } from '@sveltejs/kit';

// Existing links lead to the remaining account integrations surface.
export const load = () => redirect(307, '/account/connections');
