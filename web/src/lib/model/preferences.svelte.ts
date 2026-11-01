/* The app's one preferences store (S109 F2); see preferences.ts. Reading a preference inside a
 * component or $derived re-reads it when it changes, here or from the plane. */
import { createSubscriber } from 'svelte/reactivity';
import {
	createPreferences,
	isPreferenceKey,
	PREFERENCES_URL,
	type PreferenceKey,
	type PreferenceValues
} from './preferences';

const store = createPreferences({
	storage: () => (typeof localStorage === 'undefined' ? undefined : localStorage),
	async load() {
		const response = await fetch(PREFERENCES_URL, {
			cache: 'no-store',
			credentials: 'same-origin'
		});
		if (!response.ok) return null;
		const body = (await response.json()) as { preferences?: PreferenceValues };
		return body.preferences ?? null;
	},
	async save(patch, keepalive) {
		const response = await fetch(PREFERENCES_URL, {
			method: 'PUT',
			cache: 'no-store',
			credentials: 'same-origin',
			keepalive,
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({ preferences: patch })
		});
		return response.ok;
	}
});
const subscribe = createSubscriber((update) => store.listen(update));

export const preferences = {
	/** The kept value, or undefined for the default. Reactive. */
	get(key: PreferenceKey): unknown {
		subscribe();
		return store.get(key);
	},
	/** The kept value without subscribing, for actions and event handlers. */
	peek: (key: PreferenceKey): unknown => store.get(key),
	/** One of `allowed`, or `fallback` when nothing (or something unknown) is kept. Reactive. */
	choice<T extends string>(key: PreferenceKey, allowed: readonly T[], fallback: T): T {
		const value = preferences.get(key);
		return allowed.includes(value as T) ? (value as T) : fallback;
	},
	set: (key: PreferenceKey, value: unknown) => store.set(key, value),
	listen: (listener: () => void) => store.listen(listener)
};

export const INBOX_FILTERS = ['open', 'skipped', 'done', 'all'] as const;
export type InboxFilter = (typeof INBOX_FILTERS)[number];
/** The Inbox filter: the address's when it names one, else the person's own choice. Reactive. */
export function inboxFilter(params: URLSearchParams): InboxFilter {
	const asked = params.get('filter') as InboxFilter | null;
	return asked && INBOX_FILTERS.includes(asked)
		? asked
		: preferences.choice('inbox.filter', INBOX_FILTERS, 'open');
}
/** Choose the Inbox filter for every company; returns what the address should say. */
export function chooseInboxFilter(value: string): InboxFilter {
	const chosen = INBOX_FILTERS.includes(value as InboxFilter) ? (value as InboxFilter) : 'open';
	preferences.set('inbox.filter', chosen === 'open' ? null : chosen);
	return chosen;
}

/** The same store as the design system's views see it (src/lib/ui/preferences.svelte.ts). */
export const viewPreferenceStore = {
	get: (key: string) => (isPreferenceKey(key) ? preferences.get(key) : undefined),
	set: (key: string, value: unknown) => {
		if (isPreferenceKey(key)) store.set(key, value);
	}
};

/** Reconcile with the plane now and whenever the page comes back into view (at most once a
 * minute), and send what is still on its way as the page goes. Returns the cleanup. */
export function syncPreferences(): () => void {
	let last = 0;
	const reconcile = () => {
		const now = Date.now();
		if (now - last < 60_000) return;
		last = now;
		void store.reconcile();
	};
	const visible = () => {
		if (document.visibilityState === 'visible') reconcile();
		else void store.flush(true);
	};
	const leaving = () => void store.flush(true);
	reconcile();
	document.addEventListener('visibilitychange', visible);
	window.addEventListener('pagehide', leaving);
	return () => {
		document.removeEventListener('visibilitychange', visible);
		window.removeEventListener('pagehide', leaving);
	};
}
