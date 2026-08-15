import { redirect } from '@sveltejs/kit';

/* The inbox is the front door: it is the only surface that can be waiting on
 * you, which is why it is also first in the nav. */
export function load() {
	redirect(307, '/inbox');
}
