/* What an inline credential card shows (S101). No requests here, so tests run without a page. */

type HeldAccount = {
	id: string;
	kind: string;
	label: string;
	provider: string | null;
	frozen: boolean;
	identity: { display: string | null };
	grants: { company: string }[];
};

/** The owner's accounts that can answer a request for `app` (a catalogue key or a link), best
 * first: not yet given to this company, then by name. Frozen accounts are left out. */
export function matchingAccounts<T extends HeldAccount>(accounts: T[], app: string, company: string): T[] {
	const key = app.toLowerCase();
	return accounts
		.filter(
			(account) =>
				!account.frozen &&
				(account.kind === key ||
					(account.provider ?? '').toLowerCase().replace(/^www\./, '').split('.')[0] === key)
		)
		.sort(
			(a, b) =>
				Number(a.grants.some((grant) => grant.company === company)) -
					Number(b.grants.some((grant) => grant.company === company)) ||
				a.label.localeCompare(b.label)
		);
}

/** Where to make a token for an app that asks for one, and what it should allow. */
export function tokenHelp(app: string): { href: string; text: string } | null {
	switch (app) {
		case 'github':
			return {
				href: 'https://github.com/settings/personal-access-tokens/new',
				text: 'Make a fine-grained token on GitHub with access to the repositories this company works on.'
			};
		default:
			return null;
	}
}

export function domainOf(url: string): string {
	try {
		return new URL(url.includes('://') ? url : `https://${url.trim()}`).hostname;
	} catch {
		return '';
	}
}

/** Owners type ebay.com.au/signin; the plane wants a full https:// URL. */
export function fullUrl(url: string): string {
	const trimmed = url.trim();
	return !trimmed || /^[a-z][a-z0-9+.-]*:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
}

/** What the card still needs before it can add an account of `kind`, said plainly, or ''. */
export function missingFor(
	kind: string,
	fields: { site: string; username: string; password: string; secret: string; baseUrl: string }
): string {
	switch (kind) {
		case 'website_login':
			if (!domainOf(fields.site)) return 'Add the sign-in page.';
			if (!fields.username.trim()) return 'Add the username or email.';
			return fields.password ? '' : 'Add the password.';
		case 'browser_profile':
			return domainOf(fields.site) ? '' : 'Add the site to sign in to.';
		case 'github':
			return fields.secret.trim() ? '' : 'Paste the token.';
		case 'api':
			if (!domainOf(fields.baseUrl)) return 'Add the API address.';
			return fields.secret.trim() ? '' : 'Paste the key.';
		default:
			return '';
	}
}

/** A provider's sign-in page in a small window beside the card. */
export function popupFeatures(width = 520, height = 720): string {
	return `popup=yes,width=${width},height=${height}`;
}
