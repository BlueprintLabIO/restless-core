/* Where an external link opens for this viewer: their own browser (the default), or the company
 * browser the agents use, where shared sign-ins live. A per-viewer convenience, so it stays in
 * this browser; only the owner may choose the company browser. */
const KEY = 'restless:links-in-company-browser';

function read(): boolean {
	try {
		return localStorage.getItem(KEY) === '1';
	} catch {
		return false;
	}
}

export const linkPreference = $state({ company: typeof window === 'undefined' ? false : read() });

export function setCompanyBrowserLinks(on: boolean) {
	linkPreference.company = on;
	try {
		if (on) localStorage.setItem(KEY, '1');
		else localStorage.removeItem(KEY);
	} catch {
		/* Private windows may refuse storage; the choice still holds for this visit. */
	}
}
