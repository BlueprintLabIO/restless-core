/* app.html starts the reads a cold company load always makes while the app's
 * code is still downloading, so the first screen does not wait for the code
 * and then for the network in turn. Each started read goes to its first
 * reader; every later read, and any read after the first moments of the
 * page, fetches fresh. */

const BOOT_WINDOW_MS = 10_000;

type BootReads = Record<string, Promise<Response> | undefined>;

declare global {
	interface Window {
		__restlessBoot?: BootReads;
	}
}

export function bootFetch(url: string, init: RequestInit): Promise<Response> {
	const reads = typeof window === 'undefined' ? undefined : window.__restlessBoot;
	const started = reads?.[url];
	if (!reads || !started) return fetch(url, init);
	delete reads[url];
	if (performance.now() > BOOT_WINDOW_MS || init.signal?.aborted) return fetch(url, init);
	return started.catch(() => fetch(url, init));
}
