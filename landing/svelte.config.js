import { fileURLToPath } from 'node:url';
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/* The public Core page renders the product's own components, so `$lib` is the owner
 * workspace's library, not a copy of it. `$site` is this page's own code. */

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter({ fallback: undefined }),
		/* Absolute: kit hands this straight to Vite, which would read a ../ path as importer-relative. */
		files: { lib: fileURLToPath(new URL('../web/src/lib', import.meta.url)) },
		alias: {
			$site: 'src/site'
		}
	}
};

export default config;
