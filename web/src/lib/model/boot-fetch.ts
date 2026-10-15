/**
 * Hand a cold-load preload to its first reader. Every startup read has a deadline, including
 * prefetched reads, so a stalled connection cannot keep Home or a company skeleton forever.
 */
const BOOT_WINDOW_MS = 10_000;
const READ_TIMEOUT_MS = 15_000;
type BootReads = Record<string, Promise<Response> | undefined>;

declare global {
	interface Window {
		__restlessBoot?: BootReads;
	}
}

export function bootFetch(url: string, init: RequestInit): Promise<Response> {
	const deadline = AbortSignal.timeout(READ_TIMEOUT_MS);
	const signal = init.signal ? AbortSignal.any([init.signal, deadline]) : deadline;
	signal.throwIfAborted();
	const reads = typeof window === 'undefined' ? undefined : window.__restlessBoot;
	const started = reads?.[url];
	if (reads) delete reads[url];
	if (!started || performance.now() > BOOT_WINDOW_MS) return fetch(url, { ...init, signal });
	return new Promise<Response>((resolve, reject) => {
		const aborted = () => reject(signal.reason);
		signal.addEventListener('abort', aborted, { once: true });
		started
			.catch(() => {
				if (signal.aborted) throw signal.reason;
				return fetch(url, { ...init, signal });
			})
			.then(resolve, reject)
			.finally(() => signal.removeEventListener('abort', aborted));
	});
}
