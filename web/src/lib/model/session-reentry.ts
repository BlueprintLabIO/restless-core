/* A hosted plane signs the owner in through the account issuer. When the session ends mid-use, its
 * APIs answer 401 `no_session` or `stale_membership`; reloading lets the plane send the page Home
 * on the issuer, which signs the owner in again. One observer covers every read, so no section ever
 * shows the refusal itself. Locally there is no session and these codes never occur. */

const CODES = new Set(['no_session', 'stale_membership']);
const GUARD_KEY = 'restless:reentry-at';
const GUARD_MS = 15_000;

function reenter() {
	try {
		const last = Number(sessionStorage.getItem(GUARD_KEY) ?? 0);
		// A reload that did not re-enter would loop; leave the page as it is instead.
		if (Date.now() - last < GUARD_MS) return;
		sessionStorage.setItem(GUARD_KEY, String(Date.now()));
	} catch {
		// Without storage there is no loop guard, so do not reload.
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
						if (typeof code === 'string' && CODES.has(code)) reenter();
					})
					.catch(() => {});
			}
		}
		return response;
	};
	return () => {
		window.fetch = original;
	};
}
