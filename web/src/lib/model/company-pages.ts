/* The Company area's pages. The overview and the command menu both read this
 * list, so a page cannot be reachable from one and missing from the other. */
export type CompanyPage = {
	key: string;
	group: 'About' | 'Capabilities' | 'Operations' | 'Records';
	label: string;
	path: string;
	exact?: boolean;
	/** Extra words the owner might type for this page. */
	keywords?: string;
};

export const COMPANY_PAGES: CompanyPage[] = [
	{
		key: 'overview',
		group: 'About',
		label: 'Overview',
		path: '',
		exact: true,
		keywords: 'settings setup status'
	},
	{
		key: 'charter',
		group: 'About',
		label: 'Charter',
		path: '/charter',
		keywords: 'mission name rename logo purpose quality legal profile direction'
	},
	{
		key: 'identity',
		group: 'About',
		label: 'Brand',
		path: '/identity',
		keywords: 'identity voice brand tone look style logo colours colors'
	},
	{
		key: 'members',
		group: 'About',
		label: 'Members',
		path: '/members',
		keywords: 'invite access'
	},
	{
		key: 'provider',
		group: 'Capabilities',
		label: 'Intelligence',
		path: '/provider',
		keywords: 'model provider connection api key codex claude'
	},
	{
		key: 'accounts',
		group: 'Capabilities',
		label: 'Accounts',
		path: '/accounts',
		keywords: 'logins passwords api keys github ebay facebook shared access grants'
	},
	{
		key: 'integrations',
		group: 'Capabilities',
		label: 'Integrations',
		path: '/integrations',
		keywords: 'email resend send domain dns cloudflare integration'
	},
	{
		key: 'vault',
		group: 'Capabilities',
		label: 'Vault',
		path: '/vault',
		keywords: 'secrets credentials'
	},
	{ key: 'schedules', group: 'Operations', label: 'Schedules', path: '/schedules' },
	{
		key: 'limits',
		group: 'Operations',
		label: 'Resources',
		path: '/resources',
		keywords:
			'limits resources cpu ram memory storage tokens compute spend budget authority email payments allowance pause freeze outside actions tools mcp'
	},
	{
		key: 'notifications',
		group: 'Operations',
		label: 'Notifications',
		path: '/notifications',
		keywords: 'telegram alerts phone chat'
	},
	{
		key: 'computer',
		group: 'Operations',
		label: 'Computer',
		path: '/computer',
		keywords: 'desktop runtime files downloads restart sleep'
	},
	{
		key: 'activity',
		group: 'Records',
		label: 'Activity',
		path: '/activity',
		keywords: 'decisions history receipts effects external'
	},
	{
		key: 'tokens',
		group: 'Records',
		label: 'Tokens',
		path: '/tokens',
		keywords: 'spend cost tokens consumption usage estimates model'
	},
	{
		key: 'accounting',
		group: 'Records',
		label: 'Accounting',
		path: '/accounting',
		keywords: 'money income revenue expenses invoices receipts payments finance bookkeeping'
	},
	{
		key: 'week',
		group: 'Records',
		label: 'Weekly review',
		path: '/week',
		keywords: 'week review summary finished decided spent report'
	},
	{
		key: 'team-results',
		group: 'Records',
		label: 'Team results',
		path: '/team-results',
		keywords: 'teams landed failed blocked sent back scorecard organisation organised reviews leads'
	},
	{ key: 'health', group: 'Records', label: 'Health', path: '/doctor', keywords: 'doctor diagnose' }
];

export function companyPageHref(companyId: string, page: CompanyPage): string {
	return `/${companyId}/company${page.path}`;
}

const slug = (words: string) => words.toLowerCase().trim().replace(/\s+/g, '-');

/** The page an address names by its label or key rather than its path: Health lives at
 * `/doctor`, so `/company/health` lands there instead of on "This page does not exist"
 * (6 Nov). Real paths are routes of their own and never reach this. */
export function companyPageAlias(segment: string): CompanyPage | undefined {
	const wanted = slug(segment);
	return COMPANY_PAGES.find(
		(page) => page.path !== '' && (slug(page.label) === wanted || page.key === wanted)
	);
}

/** The company's own addresses people type without the `/company` part, or by the surface's
 * name: `/<company>/inbox` is the Inbox at the company's root, `/<company>/settings` the Company
 * area, and `/<company>/health` the Health page. Returns the path after `/<company>`. */
export function companyAlias(segment: string): string | undefined {
	const wanted = slug(segment);
	if (wanted === 'inbox' || wanted === 'home') return '';
	if (wanted === 'settings' || wanted === 'overview') return '/company';
	const page =
		companyPageAlias(wanted) ?? COMPANY_PAGES.find((candidate) => candidate.path === `/${wanted}`);
	return page && page.path !== '' ? `/company${page.path}` : undefined;
}
