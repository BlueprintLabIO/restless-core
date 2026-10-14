import { dev } from '$app/environment';
import { error } from '@sveltejs/kit';

/* Design fixtures for development only: an install never serves them. */
export function load() {
	if (!dev) error(404, 'Not found');
}
