import { PRODUCT_NAME } from '$lib/brand/brand';

/* The install manifest takes its name from the brand config, written once at build time. */
export const prerender = true;

export function GET() {
	return new Response(
		JSON.stringify({
			name: PRODUCT_NAME,
			short_name: PRODUCT_NAME,
			description: 'A judgment console for accountable company work.',
			start_url: '/',
			scope: '/',
			display: 'standalone',
			background_color: '#eef1f5',
			theme_color: '#eef1f5',
			icons: [
				{ src: '/app-icon.svg', sizes: 'any', type: 'image/svg+xml', purpose: 'any' },
				{ src: '/icon-192.png', sizes: '192x192', type: 'image/png', purpose: 'any' },
				{ src: '/icon-512.png', sizes: '512x512', type: 'image/png', purpose: 'any' },
				{ src: '/icon-maskable-512.png', sizes: '512x512', type: 'image/png', purpose: 'maskable' }
			]
		}),
		{ headers: { 'content-type': 'application/manifest+json' } }
	);
}
