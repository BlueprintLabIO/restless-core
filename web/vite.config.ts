import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';
import { chunkGroups } from './scripts/shared-chunk.mjs';

const ownerGateway = process.env.RESTLESS_OWNER_URL ?? 'http://127.0.0.1:7788';

export default defineConfig({
	plugins: [sveltekit()],
	build: {
		rolldownOptions: {
			output: {
				// Fewer, larger chunks: see scripts/shared-chunk.mjs.
				codeSplitting: { groups: chunkGroups(import.meta.dirname) }
			}
		}
	},
	server: {
		port: 5173,
		strictPort: true,
		/* The live cockpit uses same-origin API and desktop paths in production.
		 * Keep that contract in development: a standalone Vite shell without these
		 * proxies looks healthy while every source is actually a 404. */
		proxy: {
			// Live documents open a WebSocket under /api.
			'/api': { target: ownerGateway, ws: true },
			'/desktop': { target: ownerGateway, ws: true }
		}
	}
});
