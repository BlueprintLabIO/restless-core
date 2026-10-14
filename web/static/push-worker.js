/* Shows what needs the person when the plane pushes it, with every cockpit tab closed, and opens
 * the item when they click. It handles no requests: its scope is a path no page lives under, so on
 * a hosted plane, where pages share an address with other services, it never answers a page. */

self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', (event) => event.waitUntil(self.clients.claim()));

self.addEventListener('push', (event) => {
	let notice = {};
	try {
		notice = event.data ? event.data.json() : {};
	} catch {
		notice = { body: event.data ? event.data.text() : '' };
	}
	const url = typeof notice.url === 'string' && notice.url.startsWith('/') ? notice.url : '/';
	event.waitUntil(
		self.registration.showNotification(notice.title || 'Something needs you', {
			body: notice.body || '',
			tag: notice.tag || url,
			icon: '/app-icon.svg',
			data: { url }
		})
	);
});

self.addEventListener('notificationclick', (event) => {
	event.notification.close();
	/* This worker controls no page, so it cannot move an open tab; it opens the item in a new one. */
	const target = new URL(event.notification.data?.url || '/', self.location.origin).href;
	event.waitUntil(self.clients.openWindow(target));
});
