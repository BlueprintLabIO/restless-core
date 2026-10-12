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
const SHARED_COMPANY_SECTIONS = ['', '/charter', '/identity', '/members', '/schedules'];

export function collaboratorHome(companyId: string): string {
	return `/${encodeURIComponent(companyId)}/work`;
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
