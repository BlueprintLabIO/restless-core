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
		keywords: 'mission name purpose quality legal profile direction'
	},
	{
		key: 'identity',
		group: 'About',
		label: 'Brand & voice',
		path: '/identity',
		keywords: 'identity voice brand tone look style'
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
		label: 'Limits',
		path: '/resources',
		keywords: 'spend budget authority email payments tools mcp telegram notifications'
	},
	{
		key: 'computer',
		group: 'Operations',
		label: 'Computer',
		path: '/computer',
		keywords: 'desktop runtime'
	},
	{
		key: 'activity',
		group: 'Records',
		label: 'Activity',
		path: '/activity',
		keywords: 'decisions history receipts effects external'
	},
	{ key: 'health', group: 'Records', label: 'Health', path: '/doctor', keywords: 'doctor diagnose' }
];

export function companyPageHref(companyId: string, page: CompanyPage): string {
	return `/${companyId}/company${page.path}`;
}
