import type { CompanyPrincipal } from './query-persistence';

export type CompanyMembershipRole = CompanyPrincipal['membership_role'];

export type CompanyShellTab = {
	key: string;
	label: string;
	href: string;
	on: boolean;
	badge?: number;
	/** Something here needs the owner; a dot, never a count (the Inbox counts). */
	dot?: string;
};

export function hasOwnerSurfaceAccess(
	principal: CompanyPrincipal | null | undefined
): principal is CompanyPrincipal & { membership_role: 'owner' } {
	return principal?.membership_role === 'owner';
}

/** Owners and admins run the company's day to day: members, identity and schedules. */
export function mayManageCompanySetup(principal: CompanyPrincipal | null | undefined): boolean {
	return principal?.membership_role === 'owner' || principal?.membership_role === 'admin';
}

/** The Company sections every person in the company can open; the owner sees them all. */
/* Accounts are shared too (S98-08): members direct agents to use what the company may use. */
const SHARED_COMPANY_SECTIONS = [
	'',
	'/charter',
	'/identity',
	'/members',
	'/schedules',
	'/accounts'
];
/** What an admin also reads, to run the company with the owner: what it sent and decided,
 * what it spends, and the apps it uses. Changing them stays the owner's. */
const ADMIN_READ_SECTIONS = [
	'/activity',
	'/spend',
	'/tokens',
	'/accounting',
	'/resources',
	'/week',
	'/party'
];

/** An admin reads Activity, spend and Apps beside the owner. */
export function mayReadCompanyRecord(principal: CompanyPrincipal | null | undefined): boolean {
	return principal?.membership_role === 'owner' || principal?.membership_role === 'admin';
}

/** Where a member lands: their Inbox, with For you and what the team handed them. */
export function collaboratorHome(companyId: string): string {
	return `/${encodeURIComponent(companyId)}`;
}

/** Why a member was brought home from a page that is the owner's, in a sentence. */
export function refusedRouteReason(companyId: string, pathname: string): string {
	const root = `/${encodeURIComponent(companyId)}`;
	if (pathname === `${root}/apps` || pathname.startsWith(`${root}/apps/`))
		return 'Apps are managed by the owner.';
	if (pathname === `${root}/company/computer`) return 'The company computer is the owner’s.';
	if (pathname.startsWith(`${root}/company/`))
		return 'That part of Company is managed by the owner.';
	return 'That page is the owner’s.';
}

/**
 * Every person in the company works in it: Work, Library, People, Exec, and the
 * Company sections that describe it. Approvals, money, keys, apps and the company
 * computer stay with the owner; the server enforces the same split.
 */
export function mayOpenCompanyRoute(
	companyId: string,
	pathname: string,
	principal: CompanyPrincipal | null | undefined
): boolean {
	if (!principal) return false;
	if (hasOwnerSurfaceAccess(principal)) return true;
	const root = `/${encodeURIComponent(companyId)}`;
	if (SHARED_COMPANY_SECTIONS.some((section) => pathname === `${root}/company${section}`))
		return true;
	if (
		mayReadCompanyRecord(principal) &&
		(ADMIN_READ_SECTIONS.some((section) => pathname === `${root}/company${section}`) ||
			pathname === `${root}/apps` ||
			pathname.startsWith(`${root}/apps/`))
	)
		return true;
	/* Their own Inbox: what the team handed them and where they were mentioned. */
	if (pathname === root) return true;
	return (
		pathname === `${root}/people` ||
		pathname.startsWith(`${root}/people/`) ||
		pathname === `${root}/work` ||
		pathname.startsWith(`${root}/work/`) ||
		pathname === `${root}/library` ||
		pathname.startsWith(`${root}/library/`)
	);
}

export function companyShellTabs(
	companyId: string,
	pathname: string,
	principal: CompanyPrincipal | null | undefined,
	attentionCount?: number
): CompanyShellTab[] {
	const root = `/${encodeURIComponent(companyId)}`;
	if (!principal) return [];
	if (!hasOwnerSurfaceAccess(principal)) {
		return [
			{
				key: 'attention',
				label: 'Inbox',
				href: root,
				on: pathname === root,
				badge: attentionCount || undefined
			},
			{
				key: 'work',
				label: 'Work',
				href: `${root}/work`,
				on: pathname === `${root}/work` || pathname.startsWith(`${root}/work/`)
			},
			{
				key: 'library',
				label: 'Library',
				href: `${root}/library`,
				on: pathname === `${root}/library` || pathname.startsWith(`${root}/library/`)
			},
			{
				key: 'people',
				label: 'People',
				href: `${root}/people`,
				on: pathname === `${root}/people` || pathname.startsWith(`${root}/people/`)
			},
			...(mayReadCompanyRecord(principal)
				? [
						{
							key: 'apps',
							label: 'Apps',
							href: `${root}/apps`,
							on: pathname === `${root}/apps` || pathname.startsWith(`${root}/apps/`)
						}
					]
				: []),
			{
				key: 'company',
				label: 'Company',
				href: `${root}/company`,
				on: pathname === `${root}/company` || pathname.startsWith(`${root}/company/`)
			}
		];
	}
	return [
		{
			key: 'attention',
			label: 'Inbox',
			href: root,
			on: pathname === root,
			badge: attentionCount || undefined
		},
		{
			key: 'work',
			label: 'Work',
			href: `${root}/work`,
			on: pathname === `${root}/work` || pathname.startsWith(`${root}/work/`)
		},
		{
			key: 'library',
			label: 'Library',
			href: `${root}/library`,
			on: pathname === `${root}/library` || pathname.startsWith(`${root}/library/`)
		},
		{
			key: 'people',
			label: 'People',
			href: `${root}/people`,
			on: pathname === `${root}/people` || pathname.startsWith(`${root}/people/`)
		},
		{
			key: 'apps',
			label: 'Apps',
			href: `${root}/apps`,
			on: pathname === `${root}/apps` || pathname.startsWith(`${root}/apps/`)
		},
		{
			key: 'company',
			label: 'Company',
			href: `${root}/company`,
			on: pathname === `${root}/company` || pathname.startsWith(`${root}/company/`)
		}
	];
}
