import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

/** `owner::serve`'s default bind. Override with RESTLESS_OWNER_ADDR on both sides. */
const OWNER_GATEWAY = `http://${process.env.RESTLESS_OWNER_ADDR ?? '127.0.0.1:7788'}`;

export default defineConfig({
	plugins: [sveltekit()],
	server: {
		port: 5180,
		// The owner gateway is loopback-only and does not do CORS, deliberately —
		// it is the owner's machine talking to itself. In dev the proxy makes the
		// browser same-origin with it; in a packaged build the daemon serves this
		// SPA itself and the question does not arise.
		//
		// Same-origin is not cosmetic here: `/api` authenticates with an HttpOnly
		// SameSite=Strict cookie, which a cross-origin fetch would never send.
		//
		// One target, because the daemon has one owner-facing port. `/api` is the
		// authenticated BFF (attention, approvals, browser handover) and `/v1` is
		// the cockpit's read/write transport; both sit behind the same credential.
		proxy: {
			'/v1': { target: OWNER_GATEWAY, changeOrigin: false },
			'/api': { target: OWNER_GATEWAY, changeOrigin: false }
		}
	}
});
