// Company access (ADR 0012). Core supplies the verified read side and names
// the issuer; every change is made at that issuer with the account session.

import { ownerJson } from './failure.ts';

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

const read = ownerJson;

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

/** Leave the company: your own removal at the issuer. The owner cannot. */
export function leaveCompany(
	issuer: IssuerMetadata,
	companyId: string
): Promise<{ ending: boolean }> {
	return changeMembership(issuer, companyId, 'members/self/leave');
}

/** Inviting found someone already in the company: share with them by name instead. */
export class AlreadyMember extends Error {
	constructor(readonly member: CoreMember) {
		super(`${member.display} is already in the company.`);
	}
}

/** Whether this company can send email invitations now: only once sharing is set up does it have
 * an account issuer to send them. */
export async function emailInvitesReady(company: string): Promise<boolean> {
	const core = await getCoreMembers(company);
	return core.mode === 'network' && !!core.issuer && !!core.company_id;
}

/** Invite someone by email from anywhere in the company, Share included: Core names the issuer
 * and the issuer decides whether this person may invite. An email that belongs to an active member
 * throws AlreadyMember with their Actor, so Share adds them rather than inviting them again. */
export async function inviteToCompany(company: string, email: string): Promise<IssuerInvitation> {
	const core = await getCoreMembers(company);
	if (core.mode !== 'network' || !core.issuer || !core.company_id)
		throw new Error('Turn on sharing under Company, Members before inviting people.');
	const known = await getIssuerMembers(core.issuer, core.company_id).catch(() => null);
	const wanted = email.trim().toLowerCase();
	const found = known?.members.find(
		(member) => member.status === 'active' && member.email.toLowerCase() === wanted
	);
	const actor =
		found && core.members.find((member) => member.membership_id === found.membership_id);
	if (actor) throw new AlreadyMember(actor);
	const result = await changeMembership<{ invitation: IssuerInvitation }>(
		core.issuer,
		core.company_id,
		'invitations',
		{ email, role: 'member' }
	);
	return result.invitation;
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

/** Who holds root Authority now: approvals, money, keys and the charter answer to them. */
export interface AuthorityOwner {
	actor_id: string;
	source: 'explicit_transfer' | 'bootstrap_membership_owner' | 'local_default';
}

export async function getAuthorityOwner(company: string): Promise<AuthorityOwner> {
	return read(
		await fetch(`/api/companies/${encodeURIComponent(company)}/company/authority-owner`, {
			credentials: 'same-origin',
			cache: 'no-store'
		})
	);
}

/**
 * Hand the company to an active admin or member, both halves in a safe order. Core first moves root
 * Authority to them and records it; then the issuer makes them the owner and you an admin in one
 * step. Core answers a repeat of the first half without writing, so if the second half fails the
 * same call finishes the handover. Until it does, approvals wait.
 */
export async function makeOwner(
	company: string,
	issuer: IssuerMetadata,
	companyId: string,
	actorId: string,
	name: string
): Promise<void> {
	const moved = await read<{ membership_id: string }>(
		await fetch(`/api/companies/${encodeURIComponent(company)}/company/authority-owner/transfer`, {
			method: 'POST',
			credentials: 'same-origin',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify({
				to_actor_id: actorId,
				rationale: `The owner made ${name} the owner from Company, Members.`
			})
		})
	);
	await changeMembership(issuer, companyId, `members/${moved.membership_id}/make-owner`);
}
