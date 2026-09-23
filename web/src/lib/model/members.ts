// Company access (ADR 0012). Core supplies the verified read side and names
// the issuer; every change is made at that issuer with the account session.

export type MembershipRole = 'owner' | 'admin' | 'member';
export type MembershipStatus = 'active' | 'suspended' | 'removed';

export interface IssuerMetadata {
	contract_version: 1;
	issuer: string;
	admin_api: string;
	account_url: string;
	roles: string[];
	terminal_statuses: string[];
	invitation_delivery: string[];
	role_changes: string;
}

/** A human Core has admitted, as its verified binding records them. */
export interface CoreMember {
	actor_id: string;
	display: string;
	membership_id: string;
	membership_role: MembershipRole;
	membership_status: MembershipStatus;
	membership_version: number;
	first_verified_at: string;
	last_verified_at: string;
}

export interface CoreMembersView {
	mode: 'local' | 'network';
	company_id: string | null;
	issuer: IssuerMetadata | null;
	issuer_unavailable: boolean;
	members: CoreMember[];
}

export interface IssuerMember {
	membership_id: string;
	name: string;
	email: string;
	role: MembershipRole;
	status: MembershipStatus;
	version: number;
	/** A suspension or removal Core has not yet confirmed. */
	ending: boolean;
}

export interface IssuerInvitation {
	id: string;
	email: string;
	role: MembershipRole;
	expires_at: string;
	link: string;
}

export interface IssuerMembersView {
	company_id: string;
	viewer: { membership_id: string; role: MembershipRole };
	members: IssuerMember[];
	invitations: IssuerInvitation[];
}

/** One row on the page: the issuer's membership joined to Core's Actor. */
export interface MemberRow extends IssuerMember {
	actor?: CoreMember;
}

export class IssuerSessionMissing extends Error {}

async function read<T>(response: Response): Promise<T> {
	if (!response.ok) {
		let message = `${response.status} ${response.statusText}`;
		try {
			const body = (await response.json()) as { message?: string; error?: { message?: string } };
			message = body.message ?? body.error?.message ?? message;
		} catch {
			// Keep the transport status when the body is not JSON.
		}
		throw Object.assign(new Error(message), { status: response.status });
	}
	return response.json() as Promise<T>;
}

export async function getCoreMembers(company: string): Promise<CoreMembersView> {
	return read(
		await fetch(`/api/companies/${encodeURIComponent(company)}/members`, {
			credentials: 'same-origin',
			cache: 'no-store'
		})
	);
}

function adminUrl(issuer: IssuerMetadata, companyId: string, rest: string): string {
	return `${issuer.admin_api}/companies/${encodeURIComponent(companyId)}/${rest}`;
}

export async function getIssuerMembers(
	issuer: IssuerMetadata,
	companyId: string
): Promise<IssuerMembersView> {
	const response = await fetch(adminUrl(issuer, companyId, 'members'), {
		credentials: 'include',
		cache: 'no-store'
	});
	if (response.status === 401) throw new IssuerSessionMissing('Sign in to your account.');
	return read(response);
}

export async function changeMembership<T = { ending: boolean }>(
	issuer: IssuerMetadata,
	companyId: string,
	rest: string,
	body: Record<string, unknown> = {}
): Promise<T> {
	const response = await fetch(adminUrl(issuer, companyId, rest), {
		method: 'POST',
		credentials: 'include',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body)
	});
	if (response.status === 401) throw new IssuerSessionMissing('Sign in to your account.');
	return read(response);
}

export function joinMembers(core: CoreMember[], issuer: IssuerMember[]): MemberRow[] {
	const actors = new Map(core.map((member) => [member.membership_id, member]));
	const order: Record<MembershipRole, number> = { owner: 0, admin: 1, member: 2 };
	return issuer
		.map((member) => ({ ...member, actor: actors.get(member.membership_id) }))
		.sort((a, b) => order[a.role] - order[b.role] || a.name.localeCompare(b.name));
}

/** Owners manage everyone but themselves; admins manage members only. */
export function mayManage(viewer: IssuerMembersView['viewer'], target: IssuerMember): boolean {
	if (target.role === 'owner' || target.membership_id === viewer.membership_id) return false;
	return viewer.role === 'owner' || target.role === 'member';
}
