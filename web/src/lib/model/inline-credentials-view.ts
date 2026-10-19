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
export function matchingAccounts<T extends HeldAccount>(
	accounts: T[],
	app: string,
	company: string
): T[] {
	const key = app.toLowerCase();
	return accounts
		.filter(
			(account) =>
				!account.frozen &&
				(account.kind === key ||
					(account.provider ?? '')
						.toLowerCase()
						.replace(/^www\./, '')
						.split('.')[0] === key)
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

export { domainOf, fullUrl } from './account-forms.ts';

/** A provider's sign-in page in a small window beside the card. */
export function popupFeatures(width = 520, height = 720): string {
	return `popup=yes,width=${width},height=${height}`;
}
