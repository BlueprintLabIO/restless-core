/* A hosted plane signs the owner in through the account issuer. When the session ends mid-use, its
 * APIs answer 401 `no_session` or `stale_membership`; reloading lets the plane send the page Home
 * on the issuer, which signs the owner in again. A company page whose company the browser's
 * sessions on that plane no longer cover gets 403 `company_out_of_scope`, and its reload goes Home
 * to enter that company the same way. A company the plane does not hold (renamed, removed, a typo)
 * is refused the same way after that reload, so each company is retried once until it answers
 * again. One observer covers every read, so no section ever shows the refusal itself. Locally
 * there is no session and these codes never occur.
 *
 * A Cloud update restarts the plane for about a minute, and its sessions with it. Reloading during
 * that minute lands on the account's "Updating" page, which reads as being thrown out. So the page
 * stays put (the rail says it is reconnecting) and reloads only once the plane answers again, when
 * signing back in is immediate. A page whose code changed with the update waits the same way.
 *
 * Only a refusal from the plane that served this page is its own lapse. On Cloud's one address a
 * company page's plane answers that company's paths; every other API call goes to the viewer's own
 * plane, whose session is not this page's (a member who also owns a plane). */

const CODES = new Set(['no_session', 'stale_membership']);
/* Only a company page's reload is sent Home to enter its company; elsewhere it would just reload. */
const COMPANY_CODE = 'company_out_of_scope';
const GUARD_KEY = 'restless:reentry-at';
const companyRetryKey = (company: string) => `restless:reentry-company:${company}`;
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

/** `once` names a retry this tab makes only once, until it is forgotten. */
async function reenter(fetcher: typeof fetch, once?: string) {
	if (waiting) return;
	waiting = true;
	await planeAnswers(fetcher);
	try {
		const last = Number(sessionStorage.getItem(GUARD_KEY) ?? 0);
		// A reload that did not re-enter would loop; leave the page as it is instead.
		if (Date.now() - last < GUARD_MS || (once && sessionStorage.getItem(once))) {
			waiting = false;
			return;
		}
		sessionStorage.setItem(GUARD_KEY, String(Date.now()));
		if (once) sessionStorage.setItem(once, '1');
	} catch {
		// Without storage there is no loop guard, so do not reload.
		waiting = false;
		return;
	}
	window.location.reload();
}

/** Whether an API path reaches the plane that served a page of `company` (none off company pages). */
function servedPagePlane(pathname: string, company: string | undefined): boolean {
	if (!company) return pathname.startsWith('/api/');
	const scope = `/api/companies/${encodeURIComponent(company)}`;
	return pathname === scope || pathname.startsWith(`${scope}/`);
}

/** `pageCompany` names the company of the page on screen, if it is a company page. */
export function observeSessionEnd(pageCompany: () => string | undefined): () => void {
	const original = window.fetch;
	window.fetch = async (input, init) => {
		const response = await original(input, init);
		const company = pageCompany();
		if (response.status === 401 || response.status === 403 || (company && response.ok)) {
			const url = new URL(
				typeof input === 'string' || input instanceof URL ? input : input.url,
				window.location.href
			);
			if (url.origin === window.location.origin && servedPagePlane(url.pathname, company)) {
				if (response.ok) {
					// The company answers, so a later refusal of it may retry again.
					try {
						if (company) sessionStorage.removeItem(companyRetryKey(company));
					} catch {
						/* Without storage there is no retry to forget. */
					}
					return response;
				}
				const lapsed = (code: string) =>
					response.status === 401 ? CODES.has(code) : Boolean(company) && code === COMPANY_CODE;
				const once = response.status === 403 && company ? companyRetryKey(company) : undefined;
				void response
					.clone()
					.json()
					.then((body: { error?: unknown; code?: unknown }) => {
						const code = typeof body.code === 'string' ? body.code : body.error;
						if (typeof code === 'string' && lapsed(code)) void reenter(original, once);
					})
					.catch(() => {});
			}
		}
		return response;
	};
	// A route whose code moved with an update fails to load; wait for the plane, then load it fresh.
	// Offline, every load fails and nothing moved: the page stays as it is, drafts and all, and
	// whatever failed to load tries again when it is next reached.
	const preloadFailed = (event: Event) => {
		event.preventDefault();
		if (navigator.onLine === false) return;
		void reenter(original);
	};
	window.addEventListener('vite:preloadError', preloadFailed);
	return () => {
		window.fetch = original;
		window.removeEventListener('vite:preloadError', preloadFailed);
	};
}
