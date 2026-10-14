/* Signing out on a shared or borrowed computer leaves nothing of the person behind: recents carry
 * document and conversation titles, the shell remembers company names, rail topics and drafts, and
 * the safe query cache keeps recent reads. All of it is this browser's memory of the person, so it
 * goes before the sign-out leaves the page. Only the appearance choice stays: it is the device's. */

const KEEP = new Set(['restless:theme']);
const QUERY_CACHE_DATABASE = 'restless-safe-query-cache';

function clear(storage: Storage | undefined) {
	if (!storage) return;
	const keys: string[] = [];
	for (let index = 0; index < storage.length; index += 1) {
		const key = storage.key(index);
		if (key && /^restless[:.-]|^account-reentry:/.test(key) && !KEEP.has(key)) keys.push(key);
	}
	for (const key of keys) storage.removeItem(key);
}

/** Forget everything this browser keeps about the signed-in person. Never throws. */
export function forgetThisBrowser() {
	try {
		clear(window.localStorage);
	} catch {
		/* Storage may be unavailable; nothing was kept there either. */
	}
	try {
		clear(window.sessionStorage);
	} catch {
		/* As above. */
	}
	try {
		window.indexedDB?.deleteDatabase(QUERY_CACHE_DATABASE);
	} catch {
		/* As above. */
	}
}
