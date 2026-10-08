/* A hosted plane signs the owner in through the account issuer. When the session ends mid-use, its
 * APIs answer 401 `no_session` or `stale_membership`; reloading lets the plane send the page Home
 * on the issuer, which signs the owner in again. One observer covers every read, so no section ever
 * shows the refusal itself. Locally there is no session and these codes never occur.
 *
 * A Cloud update restarts the plane for about a minute, and its sessions with it. Reloading during
 * that minute lands on the account's "Updating" page, which reads as being thrown out. So the page
 * stays put (the rail says it is reconnecting) and reloads only once the plane answers again, when
 * signing back in is immediate. A page whose code changed with the update waits the same way. */

const CODES = new Set(['no_session', 'stale_membership']);
const GUARD_KEY = 'restless:reentry-at';
const GUARD_MS = 15_000;
const WAIT_MS = 3 * 60_000;
const POLL_MS = 2_000;

let waiting = false;

/** Resolve once the plane serves its health check again, or after the longest wait. */
async function planeAnswers(fetcher: typeof fetch): Promise<void> {
	const until = Date.now() + WAIT_MS;
	while (Date.now() < until) {
		try {
			const response = await fetcher('/health', { cache: 'no-store' });
			if (response.ok) return;
		} catch {
			/* Still restarting. */
		}
		await new Promise((resolve) => setTimeout(resolve, POLL_MS));
	}
}

async function reenter(fetcher: typeof fetch) {
	if (waiting) return;
	waiting = true;
	await planeAnswers(fetcher);
	try {
		const last = Number(sessionStorage.getItem(GUARD_KEY) ?? 0);
		// A reload that did not re-enter would loop; leave the page as it is instead.
		if (Date.now() - last < GUARD_MS) {
			waiting = false;
			return;
		}
		sessionStorage.setItem(GUARD_KEY, String(Date.now()));
	} catch {
		// Without storage there is no loop guard, so do not reload.
		waiting = false;
		return;
	}
	window.location.reload();
}

export function observeSessionEnd(): () => void {
	const original = window.fetch;
	window.fetch = async (input, init) => {
		const response = await original(input, init);
		if (response.status === 401) {
			const url = new URL(
				typeof input === 'string' || input instanceof URL ? input : input.url,
				window.location.href
			);
			if (url.origin === window.location.origin && url.pathname.startsWith('/api/')) {
				void response
					.clone()
					.json()
					.then((body: { error?: unknown; code?: unknown }) => {
						const code = typeof body.code === 'string' ? body.code : body.error;
						if (typeof code === 'string' && CODES.has(code)) void reenter(original);
					})
					.catch(() => {});
			}
		}
		return response;
	};
	// A route whose code moved with an update fails to load; wait for the plane, then load it fresh.
	const preloadFailed = (event: Event) => {
		event.preventDefault();
		void reenter(original);
	};
	window.addEventListener('vite:preloadError', preloadFailed);
	return () => {
		window.fetch = original;
		window.removeEventListener('vite:preloadError', preloadFailed);
	};
}
