/* Shares that wait for someone invited to the company but not in it yet (S66). Each is kept under
 * their email and given, by whoever shared it, when they first enter. */
import { responseFailure } from './failure.ts';

export type ShareTarget = { document: string } | { sheet: string };

export interface PendingShare {
	email: string;
	access: string;
	invited_by_actor_id: string;
	invited_at: string;
}

function path(companyId: string, rest = ''): string {
	return `/api/companies/${encodeURIComponent(companyId)}/pending-shares${rest}`;
}

/** Who it waits for; empty for anyone who may not share it. */
export async function getPendingShares(
	companyId: string,
	target: ShareTarget
): Promise<PendingShare[]> {
	const response = await fetch(path(companyId, `?${new URLSearchParams(target)}`), {
		credentials: 'same-origin',
		cache: 'no-store'
	});
	if (!response.ok) throw await responseFailure(response);
	return ((await response.json()) as { items: PendingShare[] }).items;
}

async function send(companyId: string, rest: string, body: unknown): Promise<void> {
	const response = await fetch(path(companyId, rest), {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		credentials: 'same-origin',
		body: JSON.stringify(body)
	});
	if (!response.ok) throw await responseFailure(response);
}

/** Keeps a share for an invitee; sharing again with the same address changes its access. */
export const shareWithInvitee = (
	companyId: string,
	target: ShareTarget,
	email: string,
	access: string
) => send(companyId, '', { ...target, email, access });

/** Stops waiting for an invitee. */
export const cancelPendingShare = (companyId: string, target: ShareTarget, email: string) =>
	send(companyId, '/cancel', { ...target, email });
