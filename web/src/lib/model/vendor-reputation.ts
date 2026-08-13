/**
 * Vendor reputation — composed from ATTRIBUTABLE facts, never an opaque star score.
 *
 * The item this serves ("build reputation from attributable engagements, verified outcomes, reversals,
 * disputes, and current credentials rather than an opaque star score") is, at heart, a truthfulness
 * rule: a vendor's standing must be traceable to recorded facts, not a black-box number. So this module
 * derives, per vendor, the actual counts — how many engagements reached each outcome, and how its
 * recorded credentials stand — and a `standing` signal computed by TRANSPARENT rules over those visible
 * counts, together with a plain-language `basis` that states the facts it rests on. There is no hidden
 * weighting and no invented score.
 *
 * Honesty rules this module keeps:
 * - Every number is a real count of recorded engagements/credentials for that exact vendor.
 * - `standing` is a rule-based label over the visible counts (documented below), not a rating out of
 *   five; the `basis` names the facts, so the owner can always see WHY.
 * - Reversals and disputes are NOT modelled here — the engagement model records no reversal or dispute
 *   concept, so they are a named, deliberate gap rather than a fabricated input. Credentials come from
 *   the recorded-not-verified vendor-credential view (see vendor-credential-view.ts).
 */

export type VendorStanding = 'unproven' | 'developing' | 'established' | 'attention';

export interface VendorReputation {
	vendorPartyId: string;
	engagements: {
		total: number;
		/** Reached the 'completed' stage — a verified positive outcome. */
		completed: number;
		lost: number;
		cancelled: number;
		/** lead / qualified / proposed / agreed / active. */
		inProgress: number;
	};
	credentials: {
		valid: number;
		expiringSoon: number;
		expired: number;
		withdrawn: number;
	};
	/** Transparent rule-based label over the visible counts — never an opaque score. */
	standing: VendorStanding;
	/** Plain-language statement of the facts the standing rests on. */
	basis: string;
}

export interface VendorReputationInput {
	/** The party ids that carry a 'vendor' role. */
	vendorPartyIds: readonly string[];
	engagements: readonly { partyId: string; stage: string }[];
	/** Credential statuses from the vendor-credential view (valid/expiring_soon/expired/withdrawn/…). */
	credentials: readonly { vendorPartyId: string; status: string }[];
}

const IN_PROGRESS_STAGES: ReadonlySet<string> = new Set([
	'lead',
	'qualified',
	'proposed',
	'agreed',
	'active'
]);

/**
 * Composes one reputation profile per vendor party. Pure and deterministic in its input.
 */
export function composeVendorReputations(input: VendorReputationInput): VendorReputation[] {
	return input.vendorPartyIds.map((vendorPartyId) => {
		const engagements = input.engagements.filter((e) => e.partyId === vendorPartyId);
		const completed = engagements.filter((e) => e.stage === 'completed').length;
		const lost = engagements.filter((e) => e.stage === 'lost').length;
		const cancelled = engagements.filter((e) => e.stage === 'cancelled').length;
		const inProgress = engagements.filter((e) => IN_PROGRESS_STAGES.has(e.stage)).length;

		const credentials = input.credentials.filter((c) => c.vendorPartyId === vendorPartyId);
		const valid = credentials.filter((c) => c.status === 'valid').length;
		const expiringSoon = credentials.filter((c) => c.status === 'expiring_soon').length;
		const expired = credentials.filter((c) => c.status === 'expired').length;
		const withdrawn = credentials.filter((c) => c.status === 'withdrawn').length;

		const abandoned = lost + cancelled;
		let standing: VendorStanding;
		if (expired > 0 || (abandoned > 0 && abandoned > completed)) {
			standing = 'attention';
		} else if (completed >= 2 && valid > 0) {
			standing = 'established';
		} else if (engagements.length > 0 || valid > 0 || expiringSoon > 0) {
			standing = 'developing';
		} else {
			standing = 'unproven';
		}

		const basis = buildBasis({
			completed,
			abandoned,
			inProgress,
			valid,
			expiringSoon,
			expired,
			withdrawn,
			standing
		});

		return {
			vendorPartyId,
			engagements: { total: engagements.length, completed, lost, cancelled, inProgress },
			credentials: { valid, expiringSoon, expired, withdrawn },
			standing,
			basis
		};
	});
}

function buildBasis(parts: {
	completed: number;
	abandoned: number;
	inProgress: number;
	valid: number;
	expiringSoon: number;
	expired: number;
	withdrawn: number;
	standing: VendorStanding;
}): string {
	if (parts.standing === 'unproven') {
		return 'No engagements or credentials recorded yet.';
	}
	const facts: string[] = [];
	facts.push(`${parts.completed} completed, ${parts.abandoned} lost/cancelled`);
	if (parts.inProgress > 0) facts.push(`${parts.inProgress} in progress`);
	const cred: string[] = [];
	if (parts.valid > 0) cred.push(`${parts.valid} valid`);
	if (parts.expiringSoon > 0) cred.push(`${parts.expiringSoon} expiring soon`);
	if (parts.expired > 0) cred.push(`${parts.expired} expired`);
	if (parts.withdrawn > 0) cred.push(`${parts.withdrawn} withdrawn`);
	facts.push(cred.length > 0 ? `credentials: ${cred.join(', ')}` : 'no credentials recorded');
	return `${facts.join('; ')}.`;
}
