/* What this browser last learned about who it is, so the first byte of a page can tell an owner's
 * load from a member's. app.html starts the owner's reads before the app's code arrives, and only
 * when this browser last saw the owner here: a member's browser must not ask for what the owner
 * alone may read, a refusal each page load and a console error each time. The keys are read by
 * app.html by name; keep the two in step. Dependency-free for node's tests. */

export const roleKey = (company: string) => `restless:role:${company}`;
/** Set once this browser has seen its person own a company on this plane. */
export const PLANE_OWNER_KEY = 'restless:plane-owner';

export function rememberRole(company: string, role: string): void {
	try {
		localStorage.setItem(roleKey(company), role);
		if (role === 'owner') localStorage.setItem(PLANE_OWNER_KEY, '1');
	} catch {
		/* Without storage the next load starts only what everyone may read. */
	}
}

/** Whether this browser has seen its person own a company here: the plane's own reads are theirs. */
export function knownPlaneOwner(): boolean {
	try {
		return localStorage.getItem(PLANE_OWNER_KEY) === '1';
	} catch {
		return false;
	}
}
