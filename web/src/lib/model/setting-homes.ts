/* Where each thing an Inbox card settles lives (S101). The card is the fastest way to answer, so
 * nothing has to be found first; it names the page quietly, the receipt says where the change
 * landed, and answering the same card again and again says what to set there once. In the
 * Inbox, a Company page link opens beside the item rather than leaving it. */

export type HomeKey =
	| 'limits'
	| 'intelligence'
	| 'brand'
	| 'computer'
	| 'charter'
	| 'schedules'
	| 'notifications'
	| 'accounts'
	| 'people';

export type SettingHome = { key: HomeKey; label: string; href: string };

const company = (id: string, path: string) => `/${encodeURIComponent(id)}/company${path}`;

export function settingHome(
	companyId: string,
	key: HomeKey,
	detail: { id?: string; name?: string } = {}
): SettingHome {
	switch (key) {
		case 'limits':
			return { key, label: 'Company › Limits', href: company(companyId, '/resources#spend') };
		case 'intelligence':
			return { key, label: 'Company › Intelligence', href: company(companyId, '/provider') };
		case 'brand':
			return {
				key,
				label: 'Company › Brand',
				href: company(
					companyId,
					detail.id ? `/identity?drift=${encodeURIComponent(detail.id)}` : '/identity'
				)
			};
		case 'computer':
			return { key, label: 'Company › Computer', href: company(companyId, '/computer') };
		case 'charter':
			return { key, label: 'Company › Charter', href: company(companyId, '/charter#legal') };
		case 'schedules':
			return { key, label: 'Company › Schedules', href: company(companyId, '/schedules') };
		case 'notifications':
			return { key, label: 'Company › Notifications', href: company(companyId, '/notifications') };
		case 'accounts':
			return { key, label: 'Settings › Accounts', href: '/account/settings/accounts' };
		case 'people':
			return {
				key,
				label: `People › ${detail.name ?? 'Exec'}`,
				href: `/${encodeURIComponent(companyId)}/people?person=${encodeURIComponent(detail.id ?? 'exec')}&view=people`
			};
	}
}

/** The page a proposed setting change lives on. */
export const PROPOSAL_HOMES: Record<string, HomeKey> = {
	spend_limit: 'limits',
	reasoning_effort: 'intelligence',
	model: 'intelligence',
	legal_profile: 'charter',
	schedule: 'schedules',
	notifications: 'notifications'
};

/** After a few answers in a month, what setting it once there would do instead. */
export function habitLine(home: SettingHome, count: number): { before: string; after: string } {
	switch (home.key) {
		case 'limits':
			return {
				before: `Raised here ${count} times this month. A monthly budget that fits in `,
				after: ' would stop these.'
			};
		case 'intelligence':
			return {
				before: `Fixed here ${count} times this month. A fallback model in `,
				after: ' takes over by itself next time.'
			};
		case 'computer':
			return {
				before: `Handled here ${count} times this month. When it sleeps and wakes is set in `,
				after: '.'
			};
		default:
			return { before: `Answered here ${count} times this month. It all lives in `, after: '.' };
	}
}
