/* The market, projected.
 *
 * The market is a two-sided outsourcing surface, not a catalogue: you post a brief, offers come
 * back, and an engagement follows (AGENTS.md — "work items are done internally and externally…
 * externally by vendors in the marketplace"; domain-concepts.txt — a marketplace listing is "a
 * discoverable offering from a vendor or capability provider, including its terms, credentials,
 * and assurances"). This module holds the projections that shape reads, kept pure so the route
 * composes rather than computes.
 *
 * The honest limit this module is written against: `ProfileClass` includes `'service'`
 * (`server/port/model.ts`) but `server/port/profiles/` implements only web-read, email-send,
 * browser-session, and computer-session. **No service profile exists**, so nothing here can yet
 * cross the Helm Port boundary to a human vendor. The market records a request and stops. */

/**
 * How a delegated outcome reconciled.
 *
 * Deliberately four states, matching helm-port.txt's RECONCILIATION: "It concludes that the
 * business outcome is confirmed, failed, unknown, or inconsistent." `unknown` is the one most
 * systems lack and the one that matters most — Helm is allowed to say it does not know whether
 * a vendor delivered, and the UI must not quietly round that up to success.
 */
export type Reconciliation = 'confirmed' | 'failed' | 'unknown' | 'inconsistent';

export type StatusTone = 'good' | 'bad' | 'pending' | 'wary';

export type StatusMark = {
	tone: StatusTone;
	glyph: 'check' | 'cross' | 'ring' | 'dots';
	label: string;
};

/**
 * Classify a recorded status into a reconciliation state.
 *
 * `inconsistent` and `unknown` are distinct on purpose: "the evidence disagrees with itself" is a
 * different call to action from "no evidence arrived". Anything unrecognised lands on `unknown`
 * rather than on a hopeful default — an unmatched status is precisely a thing we do not know.
 */
export function reconcileStatus(status: string): Reconciliation {
	const value = status.trim().toLowerCase();
	if (/inconsistent|disputed|mismatch|conflict/.test(value)) return 'inconsistent';
	if (/failed|rejected|revoked|blocked|cancelled|departed|expired/.test(value)) return 'failed';
	if (/unknown|pending|awaiting|open|proposed|not_sourced|in_progress/.test(value))
		return 'unknown';
	/* Word-anchored, and last. Without `\b` the substring `sourced` matches inside
	 * `not_sourced` — a vendor sitting on the bench would have rendered as delivered, which is
	 * the exact direction an honest reconciliation must never fail in. (`_` is a word
	 * character, so `\bsourced\b` correctly declines `not_sourced`.) */
	if (/\b(confirmed|accepted|delivered|complete|settled|active|sourced|available)\b/.test(value))
		return 'confirmed';
	return 'unknown';
}

const MARKS: Record<Reconciliation, StatusMark> = {
	confirmed: { tone: 'good', glyph: 'check', label: 'confirmed' },
	failed: { tone: 'bad', glyph: 'cross', label: 'failed' },
	// Not a spinner and not a tick: an unknown outcome is a standing question, not progress.
	unknown: { tone: 'pending', glyph: 'ring', label: 'unknown' },
	inconsistent: { tone: 'wary', glyph: 'dots', label: 'inconsistent' }
};

export function statusMark(status: string): StatusMark {
	return MARKS[reconcileStatus(status)];
}

export type PartyLike = {
	id: string;
	name: string;
	status: string;
	roles: unknown;
	serviceAreas?: unknown;
	jurisdictions?: unknown;
	availabilityNote?: string | null;
	website?: string | null;
	email?: string | null;
};

export type OfferingLike = {
	id: string;
	name: string;
	kind: string;
	description?: string | null;
	priceCents: number | null;
	currency: string;
	billing?: string | null;
	providerPartyId: string | null;
};

export type VendorWorkerLike = {
	id: string;
	name: string;
	role?: string | null;
	vendorPartyId: string;
};

/** Free-text list fields arrive as unknown JSON; render only the strings actually recorded. */
export function textList(value: unknown): string[] {
	if (Array.isArray(value))
		return value.filter((entry): entry is string => typeof entry === 'string');
	return typeof value === 'string' && value.length > 0 ? [value] : [];
}

export function hasRole(party: PartyLike, role: string): boolean {
	return textList(party.roles).includes(role);
}

export type VendorDetail = {
	party: PartyLike;
	serviceAreas: string[];
	jurisdictions: string[];
	offerings: OfferingLike[];
	workers: VendorWorkerLike[];
	mark: StatusMark;
};

/** Everything recorded about one vendor — the "app detail page" of a two-sided marketplace. */
export function composeVendorDetail(input: {
	parties: readonly PartyLike[];
	offerings: readonly OfferingLike[];
	vendorWorkers: readonly VendorWorkerLike[];
	partyId: string;
}): VendorDetail | null {
	const party = input.parties.find((entry) => entry.id === input.partyId);
	if (!party) return null;
	return {
		party,
		serviceAreas: textList(party.serviceAreas),
		jurisdictions: textList(party.jurisdictions),
		offerings: input.offerings
			.filter((offering) => offering.providerPartyId === party.id)
			.sort((a, b) => a.name.localeCompare(b.name)),
		workers: input.vendorWorkers
			.filter((worker) => worker.vendorPartyId === party.id)
			.sort((a, b) => a.name.localeCompare(b.name)),
		mark: statusMark(party.status)
	};
}

export type OfferRow = {
	partyId: string;
	partyName: string;
	note: string | null;
	selected: boolean;
};

export type OfferBoard = {
	requestId: string;
	need: string;
	category: string;
	status: string;
	mark: StatusMark;
	budgetCapCents: number | null;
	deadline: Date | string | null;
	requirements: string[];
	offers: OfferRow[];
	/** True while the request is still taking offers — the state that wants the operator's eye. */
	open: boolean;
};

export type SourcingRequestLike = {
	id: string;
	need: string;
	category: string;
	status: string;
	budgetCapCents?: number | null;
	deadline?: Date | string | null;
	requirements?: string[];
	selectedPartyId: string | null;
	candidates: ReadonlyArray<{ partyId: string; partyName: string; note: string | null }>;
};

/**
 * One board per sourcing request, its shortlist as rows.
 *
 * The market page used to flatten `candidates` into a comma-joined string, which threw away the
 * only genuinely two-sided content on the surface. Open requests sort first: a request still
 * taking offers is the one that wants a decision.
 */
export function composeOfferBoards(requests: readonly SourcingRequestLike[]): OfferBoard[] {
	return requests
		.map((request) => ({
			requestId: request.id,
			need: request.need,
			category: request.category,
			status: request.status,
			mark: statusMark(request.status),
			budgetCapCents: request.budgetCapCents ?? null,
			deadline: request.deadline ?? null,
			requirements: request.requirements ?? [],
			open: request.status === 'open',
			offers: request.candidates
				.map((candidate) => ({
					partyId: candidate.partyId,
					partyName: candidate.partyName,
					note: candidate.note,
					selected: candidate.partyId === request.selectedPartyId
				}))
				.sort((a, b) => {
					if (a.selected !== b.selected) return a.selected ? -1 : 1;
					return a.partyName.localeCompare(b.partyName);
				})
		}))
		.sort((a, b) => (a.open === b.open ? 0 : a.open ? -1 : 1));
}
