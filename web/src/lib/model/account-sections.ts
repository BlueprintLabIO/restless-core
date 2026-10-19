/* The account settings this plane owns, at one address each (Cloud ADR 0008). On a computer of
 * its own the plane draws them in its own frame; on Cloud's one address the account service draws
 * the frame and mounts the same sections from this plane's `/plane-ui/` bundle, so the addresses,
 * names and content are the same on both. */

export type AccountSectionId = 'accounts' | 'computers' | 'appearance' | 'this-computer';

export interface AccountSectionInfo {
	id: AccountSectionId;
	title: string;
	tooltip: string;
	href: string;
}

const section = (id: AccountSectionId, title: string, tooltip: string): AccountSectionInfo => ({
	id,
	title,
	tooltip,
	href: `/account/settings/${id}`
});

export const ACCOUNT_SECTIONS: Record<AccountSectionId, AccountSectionInfo> = {
	accounts: section(
		'accounts',
		'Accounts',
		'Accounts you hold once and let companies use: AI models, website logins, API keys and browser sign-ins. Each company gets only what you give it, and never sees a password or key.'
	),
	computers: section(
		'computers',
		'Computers',
		"Where your companies' computers run: this plane, and any computer of your own you add. Restless runs only its own sealed containers on a computer you add."
	),
	appearance: section(
		'appearance',
		'Appearance',
		'Light, dark, or follow this device. Saved in this browser on this device only.'
	),
	'this-computer': section(
		'this-computer',
		'Release and backup',
		'The appliance this computer runs: its release, rolling back, and backing up every company on it.'
	)
};

/** Where an account link goes: the AI accounts section, with a deep link's query kept. */
export function accountsHref(query?: Record<string, string>): string {
	const search = query ? new URLSearchParams(query).toString() : '';
	return `${ACCOUNT_SECTIONS['accounts'].href}${search ? `?${search}` : ''}`;
}

/** Earlier account addresses, and the section each became. Stored Inbox links still use them. */
const LEGACY: Record<string, AccountSectionId> = {
	'': 'accounts',
	settings: 'accounts',
	connections: 'accounts',
	'ai-apps': 'accounts',
	'settings/connections': 'accounts',
	'settings/ai-apps': 'accounts',
	appearance: 'appearance',
	'settings/appearance': 'appearance',
	computers: 'computers',
	'settings/computers': 'computers',
	appliance: 'this-computer'
};

/** The section an earlier `/account/<rest>` address became, or null when it never existed. */
export function legacyAccountSection(rest: string): AccountSectionInfo | null {
	const id = LEGACY[rest.replace(/^\/+|\/+$/g, '')];
	return id ? ACCOUNT_SECTIONS[id] : null;
}
