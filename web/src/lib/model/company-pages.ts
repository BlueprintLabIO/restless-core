/* The Company area's pages, in navigation order. The settings spine and the
 * command menu both read this list, so a page cannot be reachable from one and
 * missing from the other. */
export type CompanyPage = {
	key: string;
	section: 'Setup' | 'Operations' | 'Activity';
	label: string;
	path: string;
	exact?: boolean;
	/** Extra words the owner might type for this page. */
	keywords?: string;
};

export const COMPANY_PAGES: CompanyPage[] = [
	{
		key: 'charter',
		section: 'Setup',
		label: 'Charter',
		path: '',
		exact: true,
		keywords: 'mission name'
	},
	{
		key: 'identity',
		section: 'Setup',
		label: 'Identity',
		path: '/identity',
		keywords: 'voice brand'
	},
	{
		key: 'members',
		section: 'Setup',
		label: 'Members',
		path: '/members',
		keywords: 'invite access'
	},
	{ key: 'skills', section: 'Setup', label: 'Skills', path: '/skills' },
	{
		key: 'provider',
		section: 'Setup',
		label: 'Intelligence',
		path: '/provider',
		keywords: 'model provider connection api key codex claude'
	},
	{
		key: 'vault',
		section: 'Setup',
		label: 'Vault',
		path: '/vault',
		keywords: 'secrets credentials'
	},
	{ key: 'schedules', section: 'Operations', label: 'Schedules', path: '/schedules' },
	{
		key: 'resources',
		section: 'Operations',
		label: 'Access & limits',
		path: '/resources',
		keywords: 'spend budget authority'
	},
	{
		key: 'computer',
		section: 'Operations',
		label: 'Computer',
		path: '/computer',
		keywords: 'desktop runtime'
	},
	{
		key: 'doctor',
		section: 'Activity',
		label: 'Doctor',
		path: '/doctor',
		keywords: 'health diagnose'
	},
	{ key: 'decisions', section: 'Activity', label: 'Decision history', path: '/decisions' },
	{
		key: 'actions',
		section: 'Activity',
		label: 'External activity',
		path: '/actions',
		keywords: 'receipts effects'
	}
];

export function companyPageHref(companyId: string, page: CompanyPage): string {
	return `/${companyId}/company${page.path}`;
}
