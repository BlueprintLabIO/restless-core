/// <reference lib="webworker" />

import { version } from '$service-worker';

const worker = self as unknown as ServiceWorkerGlobalScope;
const SHELL_CACHE = `restless-shell-${version}`;
const SHELL_FALLBACK = '/index.html';
const PRECACHE = [SHELL_FALLBACK, '/manifest.webmanifest', '/app-icon.svg', '/favicon.svg'];
const NAVIGATION_TIMEOUT_MS = 6_000;

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

function navigationErrorResponse(): Response {
	return new Response(
		`<!doctype html>
<html lang="en">
	<head>
		<meta charset="utf-8">
		<meta name="viewport" content="width=device-width, initial-scale=1">
		<meta name="theme-color" content="#eef1f5">
		<title>Page unavailable · Restless</title>
		<style>
			:root { color-scheme: light; font: 13px/1.5 'IBM Plex Sans', system-ui, sans-serif; }
			* { box-sizing: border-box; }
			body { min-height: 100svh; margin: 0; display: grid; place-items: center; padding: 24px;
				color: #17181c; background: #eef1f5; position: relative; isolation: isolate; }
			body::before { content: ''; position: fixed; inset: -30%; z-index: -1; pointer-events: none;
				background: radial-gradient(ellipse at 28% 32%, #dfe7f2 0, transparent 45%),
					radial-gradient(ellipse at 75% 72%, #e6e2f1 0, transparent 42%); filter: blur(34px); }
			main { width: min(100%, 400px); padding: 24px; border: 1px solid rgba(76, 88, 117, .14);
				border-radius: 6px; background: rgba(255, 255, 255, .92); backdrop-filter: blur(12px);
				-webkit-backdrop-filter: blur(12px); box-shadow: 0 8px 28px rgba(65, 76, 104, .12); }
			p { margin: 0; color: rgba(23, 27, 36, .66); }
			h1 { margin: 0 0 8px; color: #293244; font-size: clamp(22px, 6vw, 26px); letter-spacing: -.03em; }
			a { display: inline-flex; margin-top: 20px; padding: 8px 12px; border-radius: 4px;
				color: #f8fafc; background: #37435b; font-weight: 600; text-decoration: none; }
			a:focus-visible { outline: 2px solid #48546c; outline-offset: 3px; }
		</style>
	</head>
	<body>
		<main role="alert">
			<h1>This page didn’t load</h1>
			<p>Restless couldn’t reach this page in time. Check your connection and try again.</p>
			<a href="">Try again</a>
		</main>
	</body>
</html>`,
		{
			status: 503,
			statusText: 'Service Unavailable',
			headers: { 'Content-Type': 'text/html; charset=utf-8', 'Cache-Control': 'no-store' }
		}
	);
}

async function boundedNavigationFetch(request: Request): Promise<Response> {
	const controller = new AbortController();
	const timeout = setTimeout(() => controller.abort(), NAVIGATION_TIMEOUT_MS);
	try {
		// A previous shell response can remain in the browser's HTTP cache even
		// after the server begins sending no-store. Ask the network on every
		// navigation so a deployed cockpit is visible on the next reload.
		const response = await fetch(new Request(request, { signal: controller.signal, cache: 'no-store' }));
		// Fetch resolves at the headers. Read a clone so a response whose body stalls is
		// bounded too, while leaving the original available to the browser and cache.
		if (response.body) await response.clone().arrayBuffer();
		return response;
	} finally {
		clearTimeout(timeout);
	}
}

async function navigationFallback(): Promise<Response> {
	try {
		const shell = await caches.match(SHELL_FALLBACK);
		return shell ?? navigationErrorResponse();
	} catch {
		return navigationErrorResponse();
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
		const network = boundedNavigationFetch(request);
		event.waitUntil(network.then(cacheNavigationFallback, () => undefined));
		event.respondWith(network.catch(() => navigationFallback()));
		return;
	}
	if (url.pathname.startsWith('/_app/immutable/')) {
		event.respondWith(immutableAsset(request));
	}
});

export {};
