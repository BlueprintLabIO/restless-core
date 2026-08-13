import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/* Static adapter with an SPA fallback. This app is deliberately unwired: there is no
 * server, no data loading, and no API. Every surface renders from `$lib/fixtures`.
 * When the OrgIntel read API exists (ARCHITECTURE.md §4.4), the fixtures are replaced
 * by a thin client and the adapter choice is revisited then — not before. */

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter({ fallback: 'index.html' }),
		prerender: { entries: [] },
		alias: {
			$lib: 'src/lib'
		}
	}
};

export default config;
