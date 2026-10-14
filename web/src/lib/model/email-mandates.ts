/** Email authority: the mandates that let Exec email an approved audience within limits. */
import { ownerRequest } from './setting.svelte';

export type EmailMandate = {
	id: string;
	purpose: string;
	audience_guidance: string;
	sender: string;
	sender_name?: string | null;
	max_per_day: number;
	max_total: number;
	timezone: string;
	expires_at: string;
	created_at: string;
	revoked_at?: string | null;
	usage?: {
		mandate_id: string;
		usage_day: string;
		used_today: number;
		used_total: number;
		limit_per_day: number;
		limit_total: number;
	};
	recent_decisions?: {
		permit_id: string;
		recipient: string;
		effect_key: string;
		issued_at: string;
		rationale: string;
		evidence_refs: string[];
		outcome: string;
		provider_ref?: string | null;
		provider_detail?: string | null;
	}[];
};

export type EmailMandateProposal = {
	purpose: string;
	audience_guidance: string;
	sender: string;
	sender_name: string | null;
	max_per_day: number;
	max_total: number;
	timezone: string;
	expires_at: string;
};

const base = (company: string) => `/api/companies/${encodeURIComponent(company)}`;

export const emailMandatesKey = (company: string) => ['email-mandates', company] as const;

export async function readEmailMandates(company: string, signal?: AbortSignal) {
	const body = await ownerRequest<{ mandates?: unknown }>(`${base(company)}/mandates/email`, {
		signal
	});
	if (!Array.isArray(body.mandates)) throw new Error('The email mandate response was incomplete.');
	return body.mandates as EmailMandate[];
}

/** `replaces` names the current mandate a change is for; approving the change replaces it. */
export function proposeEmailMandate(
	company: string,
	proposal: EmailMandateProposal,
	replaces: string | null = null
) {
	return ownerRequest<unknown>(`${base(company)}/email-mandates/proposals`, {
		method: 'POST',
		json: { proposal, ...(replaces ? { replaces } : {}) }
	});
}

export function revokeEmailMandate(company: string, id: string, reason: string) {
	return ownerRequest<unknown>(`${base(company)}/mandates/email/${encodeURIComponent(id)}/revoke`, {
		method: 'POST',
		json: { reason }
	});
}
