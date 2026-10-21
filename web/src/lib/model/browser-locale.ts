import { ownerRequest } from './setting.svelte';

/** The owner's own timezone and language, as this browser knows them. */
export function browserLocale(): { timezone: string | null; language: string | null } {
	let timezone: string | null = null;
	try {
		timezone = Intl.DateTimeFormat().resolvedOptions().timeZone || null;
	} catch {
		timezone = null;
	}
	return { timezone, language: navigator.language || null };
}

/* The company browsers open sites in the owner's timezone and language (S104 F3).
 * The plane keeps the last report; once per tab session is enough. */
export async function reportBrowserLocale(): Promise<void> {
	const locale = browserLocale();
	const key = 'restless.browser-locale';
	const reported = JSON.stringify(locale);
	try {
		if (sessionStorage.getItem(key) === reported) return;
	} catch {
		/* Storage may be blocked; report anyway. */
	}
	await ownerRequest('/api/browser-locale', { method: 'PUT', json: locale });
	try {
		sessionStorage.setItem(key, reported);
	} catch {
		/* Nothing to remember it in. */
	}
}
