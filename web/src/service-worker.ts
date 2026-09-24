/// <reference lib="webworker" />

import { version } from '$service-worker';

const worker = self as unknown as ServiceWorkerGlobalScope;
const SHELL_CACHE = `restless-shell-${version}`;
const SHELL_FALLBACK = '/index.html';
const PRECACHE = [SHELL_FALLBACK, '/manifest.webmanifest', '/app-icon.svg', '/favicon.svg'];

worker.addEventListener('install', (event) => {
	event.waitUntil(
		caches
			.open(SHELL_CACHE)
			.then((cache) => cache.addAll(PRECACHE))
			.then(() => worker.skipWaiting())
	);
});

worker.addEventListener('activate', (event) => {
	event.waitUntil(
		caches
			.keys()
			.then((keys) =>
				Promise.all(
					keys
						.filter((key) => key.startsWith('restless-shell-') && key !== SHELL_CACHE)
						.map((key) => caches.delete(key))
				)
			)
			.then(() => worker.clients.claim())
	);
});

async function cacheNavigationFallback(response: Response): Promise<void> {
	if (!response.ok || !response.headers.get('content-type')?.includes('text/html')) return;

	try {
		const copy = response.clone();
		const cache = await caches.open(SHELL_CACHE);
		await cache.put(SHELL_FALLBACK, copy);
	} catch {
		// A cache failure must not delay or fail the navigation response.
	}
}

async function immutableAsset(request: Request): Promise<Response> {
	const cached = await caches.match(request);
	if (cached) return cached;
	const response = await fetch(request);
	if (response.ok) {
		const cache = await caches.open(SHELL_CACHE);
		await cache.put(request, response.clone()).catch(() => undefined);
	}
	return response;
}

worker.addEventListener('fetch', (event) => {
	const { request } = event;
	if (request.method !== 'GET') return;
	const url = new URL(request.url);
	if (url.origin !== worker.location.origin || url.pathname.startsWith('/api/')) return;

	if (request.mode === 'navigate') {
		const network = fetch(request);
		event.waitUntil(network.then(cacheNavigationFallback, () => undefined));
		event.respondWith(
			network.catch(async (error) => {
				const shell = await caches.match(SHELL_FALLBACK);
				if (shell) return shell;
				throw error;
			})
		);
		return;
	}
	if (url.pathname.startsWith('/_app/immutable/')) {
		event.respondWith(immutableAsset(request));
	}
});

export {};
