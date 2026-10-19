import { svelte, vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vite';

/* The plane's account sections as one ES module (Cloud ADR 0008), built beside the cockpit into
 * build/plane-ui/, which the plane serves as it serves the cockpit. One file with its styles
 * inside: the account service imports it and nothing else, so no chunk or stylesheet can be
 * fetched from the wrong plane. */
export default defineConfig({
	plugins: [svelte({ preprocess: vitePreprocess(), compilerOptions: { css: 'injected' } })],
	resolve: {
		alias: { $lib: fileURLToPath(new URL('./src/lib', import.meta.url)) },
		conditions: ['browser']
	},
	define: { 'process.env.NODE_ENV': JSON.stringify('production') },
	build: {
		outDir: 'build/plane-ui',
		emptyOutDir: true,
		copyPublicDir: false,
		cssCodeSplit: false,
		lib: {
			entry: fileURLToPath(new URL('./src/plane-ui/entry.ts', import.meta.url)),
			formats: ['es'],
			fileName: () => 'account-sections.js'
		},
		rolldownOptions: { output: { codeSplitting: false } }
	}
});
