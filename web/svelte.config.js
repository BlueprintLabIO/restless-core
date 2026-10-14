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
		// Registered by the root layout only on a local appliance: on Cloud's one address a cockpit
		// service worker would control Fleet's pages too (ADR 0007).
		serviceWorker: { register: false },
		// The web manifest is generated from the brand config (routes/manifest.webmanifest).
		prerender: { entries: ['/manifest.webmanifest'] },
		// An open tab learns that a new version was deployed and offers a calm reload (root layout).
		version: { pollInterval: 5 * 60_000 },
		alias: {
			$lib: 'src/lib'
		}
	}
};

export default config;
