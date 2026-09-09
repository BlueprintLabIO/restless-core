import type { CompanyPrincipal } from './query-persistence';

export type CompanyMembershipRole = CompanyPrincipal['membership_role'];

export type CompanyShellTab = {
	key: string;
	label: string;
	href: string;
	on: boolean;
	badge?: number;
};

export function hasOwnerSurfaceAccess(
	principal: CompanyPrincipal | null | undefined
): principal is CompanyPrincipal & { membership_role: 'owner' } {
	return principal?.membership_role === 'owner';
}

export function collaboratorHome(companyId: string): string {
	return `/${encodeURIComponent(companyId)}/work`;
}

/**
 * Non-owner membership is deliberately smaller than owner Authority. These
 * are the collaboration routes whose server handlers authorize the acting
 * principal and record its Actor attribution themselves.
 */
export function mayOpenCompanyRoute(
	companyId: string,
	pathname: string,
	principal: CompanyPrincipal | null | undefined
): boolean {
	if (!principal) return false;
	if (hasOwnerSurfaceAccess(principal)) return true;
	const root = `/${encodeURIComponent(companyId)}`;
	return (
		pathname === `${root}/people` ||
		pathname.startsWith(`${root}/people/`) ||
		pathname === `${root}/work` ||
		pathname.startsWith(`${root}/work/`)
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
				key: 'people',
				label: 'People',
				href: `${root}/people`,
				on: pathname === `${root}/people` || pathname.startsWith(`${root}/people/`)
			}
		];
	}
	return [
		{
			key: 'attention',
			label: 'Attention',
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
