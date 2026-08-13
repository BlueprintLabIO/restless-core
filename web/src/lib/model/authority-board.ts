/* Standing authority, grouped for reading.
 *
 * The Mission pane shows the first 14 grants as chips and counts the rest. At ~74 grants that
 * is a page, not a fold — which is what earns it an expand route (UIR-011).
 *
 * This reads `desk.authority` directly rather than `toMission`'s `standingRules`. That view
 * renders each grant into a sentence (`"member may send email (approval_required)"`) which the
 * Mission surface then reverses with a regex to get the capability back. Round-tripping
 * structured fields through prose loses on any label the regex does not anticipate, and the
 * structured fields are right there. */

export type AuthorityGrant = {
	id: string;
	actorId: string;
	capability: string;
	mode: string;
	version: number;
	active: boolean;
	subject: 'member' | 'staff';
};

export type GrantRow = {
	id: string;
	capability: string;
	/** The readable tail: `email.send.external` → `send external`. */
	action: string;
	mode: string;
	subject: 'member' | 'staff';
	/** True when acting needs a human's word first, rather than being autonomous. */
	needsApproval: boolean;
};

export type GrantGroup = {
	/** The capability's first segment — `email`, `work`, `billing`. */
	domain: string;
	rows: GrantRow[];
	/** How many in this group are gated on approval rather than standing. */
	approvalCount: number;
};

export type AuthorityFilters = {
	subject?: 'member' | 'staff' | null;
	/** Only grants that need a human's word. */
	approvalOnly?: boolean;
	query?: string | null;
};

/** `email.send.external` → domain `email`, action `send external`. */
export function splitCapability(capability: string): { domain: string; action: string } {
	const [domain, ...rest] = capability.split('.');
	return {
		domain: domain || capability,
		action: rest.length > 0 ? rest.join(' ').replaceAll('_', ' ') : capability
	};
}

function toRow(grant: AuthorityGrant): GrantRow {
	const { action } = splitCapability(grant.capability);
	return {
		id: grant.id,
		capability: grant.capability,
		action,
		mode: grant.mode,
		subject: grant.subject,
		needsApproval: grant.mode === 'approval_required'
	};
}

/**
 * Active grants, grouped by capability domain.
 *
 * `access.*` is excluded to match what the Mission pane counts — those are the reader's own
 * view/operate/administer powers, not grants the company made to anyone about doing work, and
 * counting them would inflate the number the pane advertises.
 *
 * Inactive grants are excluded too: a withdrawn grant is history, and the tape is where history
 * is read. This page answers "what may happen right now".
 */
export function composeAuthorityBoard(
	authority: readonly AuthorityGrant[],
	filters: AuthorityFilters = {}
): GrantGroup[] {
	const query = (filters.query ?? '').trim().toLowerCase();
	const byDomain = new Map<string, GrantRow[]>();

	for (const grant of authority) {
		if (!grant.active) continue;
		if (grant.capability.startsWith('access.')) continue;
		if (filters.subject && grant.subject !== filters.subject) continue;
		if (filters.approvalOnly && grant.mode !== 'approval_required') continue;
		if (query && !grant.capability.toLowerCase().includes(query)) continue;
		const { domain } = splitCapability(grant.capability);
		const rows = byDomain.get(domain) ?? [];
		rows.push(toRow(grant));
		byDomain.set(domain, rows);
	}

	return [...byDomain]
		.map(([domain, rows]) => ({
			domain,
			rows: rows.sort((a, b) => a.capability.localeCompare(b.capability)),
			approvalCount: rows.filter((row) => row.needsApproval).length
		}))
		.sort((a, b) => a.domain.localeCompare(b.domain));
}

/** The headline the pane advertises, so the page and the pane cannot disagree about the count. */
export function countGrants(groups: readonly GrantGroup[]): {
	total: number;
	approval: number;
} {
	return groups.reduce(
		(totals, group) => ({
			total: totals.total + group.rows.length,
			approval: totals.approval + group.approvalCount
		}),
		{ total: 0, approval: 0 }
	);
}
