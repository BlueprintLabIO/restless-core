import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		port: 5180,
		// The daemon's cockpit listener is loopback-only and does not do CORS,
		// deliberately — it is the owner's machine talking to itself. In dev the
		// proxy makes the browser same-origin with it; in a packaged build the
		// SPA is served by the daemon and the question does not arise.
		proxy: {
			'/v1': {
				target: 'http://127.0.0.1:7792',
				changeOrigin: false
			}
		}
	}
});
