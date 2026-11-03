/** Standing permissions: what the owner allowed the company to do on its own, within limits,
 * until a date. Email is one kind; ordering from a vendor is another. */
import { ownerRequest } from './setting.svelte';

export type MandateKind = 'email' | 'vendor_order';

export type Money = { minor: number; currency: string };

export type Mandate = {
	id: string;
	version: number;
	kind: MandateKind;
	title: string;
	scope: string;
	limits: {
		max_total?: number;
		max_per_day?: number;
		timezone?: string;
		max_per_action?: Money;
		max_spend_total?: Money;
		counterparties?: string[];
	};
	expires_at: string;
	owner_approves_each: boolean;
	rationale: string;
	evidence: string[];
	email?: {
		purpose: string;
		audience_guidance: string;
		sender: string;
		sender_name?: string | null;
	};
	proposed_by?: string | null;
	created_at: string;
	active: boolean;
	/** Each limit as the owner reads it. */
	limits_in_words: string[];
	/** Exactly what was granted, line by line, as the Inbox card showed it. */
	terms_in_words: { label: string; value: string }[];
	/** What is used so far, in a few words. */
	usage_in_words: string;
	recent_sends?: {
		permit_id: string;
		recipient: string;
		issued_at: string;
		rationale: string;
		evidence_refs: string[];
		outcome: string;
		provider_ref?: string | null;
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

export const mandatesKey = (company: string) => ['mandates', company] as const;

export async function readMandates(company: string, signal?: AbortSignal) {
	const body = await ownerRequest<{ mandates?: unknown }>(`${base(company)}/mandates`, { signal });
	if (!Array.isArray(body.mandates))
		throw new Error('The standing permissions response was incomplete.');
	return body.mandates as Mandate[];
}

/** The owner proposing an email permission from Company › Authority. `replaces` names the current
 * one a change is for; approving the change in the Inbox replaces it. */
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

export function revokeMandate(company: string, id: string, reason: string) {
	return ownerRequest<unknown>(`${base(company)}/mandates/${encodeURIComponent(id)}/revoke`, {
		method: 'POST',
		json: { reason }
	});
}
