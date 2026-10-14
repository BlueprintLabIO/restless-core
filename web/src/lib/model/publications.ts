/**
 * What the company put online, under the owner's hand: authorise a prepared publication, invite
 * people to it, revoke an invitation, and stop it. Each call is owner-only on the server.
 */
import { ownerRequest } from './setting.svelte';

export type PublicationInvitation = {
	invitation_id: string;
	subject: string;
	expires_at: string;
	revoked: boolean;
};

export type PublicationDetail = {
	publication_id: string;
	invitations: PublicationInvitation[];
	stopped: boolean;
};

const path = (company: string, publication: string, rest = '') =>
	`/api/companies/${encodeURIComponent(company)}/publications/${encodeURIComponent(publication)}${rest}`;

export function getPublication(company: string, publication: string, signal?: AbortSignal) {
	return ownerRequest<PublicationDetail>(path(company, publication), { signal });
}

export function authorizePublication(company: string, publication: string) {
	return ownerRequest<unknown>(path(company, publication, '/authorize'), {
		method: 'POST',
		json: {}
	});
}

/** Mint an invitation; its access code comes back once and is never shown again. */
export function invitePublication(
	company: string,
	publication: string,
	invitee: string,
	expiresAt: string
) {
	return ownerRequest<PublicationInvitation & { token: string }>(
		path(company, publication, '/invitations'),
		{ method: 'POST', json: { invitee, expires_at: expiresAt } }
	);
}

export function revokePublicationInvitation(
	company: string,
	publication: string,
	invitation: string
) {
	return ownerRequest<unknown>(
		path(company, publication, `/invitations/${encodeURIComponent(invitation)}/revoke`),
		{ method: 'POST', json: {} }
	);
}

/** Stop publishing, or decline a prepared publication before it starts. */
export function stopPublication(company: string, publication: string, reason = '') {
	return ownerRequest<unknown>(path(company, publication, '/stop'), {
		method: 'POST',
		json: { reason }
	});
}

/** The publication id of a published-service resource row. */
export function publicationOf(resourceId: string): string | null {
	return resourceId.startsWith('published-service:')
		? resourceId.slice('published-service:'.length)
		: null;
}

const AUDIENCE: Record<string, string> = {
	'owner-only': 'Only you',
	'named-invitees': 'People you invite',
	public: 'Anyone with the address'
};

export function audienceLabel(value: unknown): string {
	return typeof value === 'string' ? (AUDIENCE[value] ?? value) : 'Not stated';
}
