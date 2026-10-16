import type {
	CompanyIdentitySnapshot,
	ConstitutionArtifactBindingRow,
	ConstitutionLearningProposalRow,
	CultureCaseRecordRow,
	CultureEvidenceDetailRow,
	CultureReviewRow,
	CultureWorkContractRow,
	IdentityEvidenceRow,
	IdentityDriftFindingRow,
	IdentityMigrationDecisionRow,
	IdentityMigrationDisposition,
	IdentityPillar,
	IdentityProposalRow,
	IdentityReleaseRow,
	IdentityWorkBindingRow,
	VoiceEvidenceDetailRow,
	VoiceRenderEvidenceRow,
	VoiceReviewRow,
	VoiceWorkContractRow,
	VisualEvidenceDetailRow,
	VisualPrimitiveUseRow,
	VisualRenderEvidenceRow,
	VisualReviewRow,
	VisualWorkContractRow
} from './generated/orgintel';
import { ownerJson } from './failure.ts';

export type {
	CompanyIdentitySnapshot,
	ConstitutionArtifactBindingRow,
	ConstitutionLearningProposalRow,
	CultureCaseRecordRow,
	CultureEvidenceDetailRow,
	CultureReviewRow,
	CultureWorkContractRow,
	IdentityEvidenceRow,
	IdentityDriftFindingRow,
	IdentityMigrationDecisionRow,
	IdentityMigrationDisposition,
	IdentityProposalRow,
	IdentityReleaseRow,
	IdentityWorkBindingRow,
	VoiceEvidenceDetailRow,
	VoiceRenderEvidenceRow,
	VoiceReviewRow,
	VoiceWorkContractRow,
	VisualEvidenceDetailRow,
	VisualPrimitiveUseRow,
	VisualRenderEvidenceRow,
	VisualReviewRow,
	VisualWorkContractRow
};

const ownerResponse = ownerJson;

export async function getCompanyIdentity(company: string): Promise<CompanyIdentitySnapshot> {
	return ownerResponse(
		await fetch(`/api/companies/${encodeURIComponent(company)}/company/identity`, {
			credentials: 'same-origin',
			cache: 'no-store'
		})
	);
}

export async function promoteIdentityProposal(
	company: string,
	proposal: string,
	changeAccount: string
): Promise<{ proposal_id: string; release_id: string; authority_record_id: number }> {
	return ownerResponse(
		await fetch(
			`/api/companies/${encodeURIComponent(company)}/company/identity/proposals/${encodeURIComponent(proposal)}/promote`,
			{
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ change_account: changeAccount }),
				credentials: 'same-origin'
			}
		)
	);
}

export async function rejectIdentityProposal(
	company: string,
	proposal: string,
	rationale: string
): Promise<{ proposal_id: string; decision: 'rejected'; authority_record_id: number }> {
	return ownerResponse(
		await fetch(
			`/api/companies/${encodeURIComponent(company)}/company/identity/proposals/${encodeURIComponent(proposal)}/reject`,
			{
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ rationale }),
				credentials: 'same-origin'
			}
		)
	);
}

export async function decideIdentityMigration(
	company: string,
	finding: string,
	disposition: IdentityMigrationDisposition,
	rationale: string
): Promise<{ decision: IdentityMigrationDecisionRow; authority_record_id: number }> {
	return ownerResponse(
		await fetch(
			`/api/companies/${encodeURIComponent(company)}/company/identity/drift/${encodeURIComponent(finding)}/decide`,
			{
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ disposition, rationale }),
				credentials: 'same-origin'
			}
		)
	);
}

/** The four headed pillars of the brand document, in reading order. */
export const IDENTITY_PILLARS = [
	{
		key: 'truth',
		label: 'Truth',
		hint: 'What the company stands for, what it offers, and the claims it can substantiate.'
	},
	{
		key: 'voice',
		label: 'Voice',
		hint: 'How the company sounds, including useful examples and language to avoid.'
	},
	{
		key: 'visual',
		label: 'Visual language',
		hint: 'The logo, colours, typefaces and reference images, then visual direction and design principles.'
	},
	{
		key: 'culture',
		label: 'Culture',
		hint: 'How the company works, handles uncertainty and treats people.'
	}
] as const satisfies readonly { key: IdentityPillar; label: string; hint: string }[];

/** The structured visual settings an owner sets directly. */
export const VISUAL_TOKENS = [
	{ kind: 'semantic_token', label: 'Colours', add: 'Add colour', role: 'Primary' },
	{ kind: 'typography_role', label: 'Typefaces', add: 'Add typeface', role: 'Headings' },
	{ kind: 'imagery_direction', label: 'Reference images', add: 'Add reference', role: 'Example' }
] as const;
export type VisualTokenKind = (typeof VISUAL_TOKENS)[number]['kind'];
export type VisualToken = { kind: VisualTokenKind; role: string; value: string };

/** One statement of the brand document, as read or as drafted. `id` names the
 * evidence it came from; a new statement has none. */
export type BrandStatement = {
	id: string | null;
	pillar: IdentityPillar;
	text: string;
	token: VisualToken | null;
	evidence: IdentityEvidenceRow | null;
};

function linked(links: { evidence_id: string }[], view: CompanyIdentitySnapshot) {
	const ids = new Set(links.map((link) => link.evidence_id));
	return view.evidence.filter((item) => ids.has(item.id));
}
export function releaseEvidence(
	view: CompanyIdentitySnapshot,
	release: string | null | undefined
): IdentityEvidenceRow[] {
	return linked(
		view.release_evidence.filter((link) => link.release_id === release),
		view
	);
}
export function proposalEvidence(
	view: CompanyIdentitySnapshot,
	proposal: string
): IdentityEvidenceRow[] {
	return linked(
		view.proposal_evidence.filter((link) => link.proposal_id === proposal),
		view
	);
}

const TOKEN_KINDS = new Set<string>(VISUAL_TOKENS.map((token) => token.kind));
/** A colour, typeface or reference image recorded with its structured value. */
export function visualToken(
	view: CompanyIdentitySnapshot,
	evidence: IdentityEvidenceRow
): VisualToken | null {
	const detail = view.visual_evidence_details.find((row) => row.evidence_id === evidence.id);
	if (!detail || !TOKEN_KINDS.has(detail.kind) || !detail.value?.trim()) return null;
	return {
		kind: detail.kind as VisualTokenKind,
		role: (detail.semantic_role ?? detail.purpose).trim(),
		value: detail.value.trim()
	};
}

export function brandStatements(
	view: CompanyIdentitySnapshot,
	evidence: IdentityEvidenceRow[]
): BrandStatement[] {
	return evidence.map((item) => ({
		id: item.id,
		pillar: item.pillar,
		text: item.statement,
		token: visualToken(view, item),
		evidence: item
	}));
}

/** Who a statement came from, for its hover. */
export function attribution(item: IdentityEvidenceRow | null): string {
	if (!item) return 'New';
	const kind = item.statement_kind === 'guidance' ? '' : ` · ${item.statement_kind}`;
	if (item.source === 'owner_identity_editor')
		return `Written in Brand by ${item.author_id}${kind}`;
	return `${item.author_id} · ${item.source.replaceAll('_', ' ')}${kind} · ${item.evidence_locator}`;
}

/** What a pending proposal would change against the current version. */
export function proposalChanges(view: CompanyIdentitySnapshot, proposal: IdentityProposalRow) {
	const current = releaseEvidence(view, view.current_release?.id);
	const proposed = proposalEvidence(view, proposal.id);
	const currentIds = new Set(current.map((item) => item.id));
	const proposedIds = new Set(proposed.map((item) => item.id));
	return {
		added: proposed.filter((item) => !currentIds.has(item.id)),
		removed: current.filter((item) => !proposedIds.has(item.id)),
		/* Promotion refuses a proposal drafted against an older version. */
		stale: (proposal.expected_predecessor ?? null) !== (view.current_release?.id ?? null)
	};
}

/** A statement while the owner edits: `original` is the structured value it
 * started from, so an untouched colour or typeface keeps its evidence. */
export type DraftStatement = BrandStatement & { key: string; original: VisualToken | null };

export function draftStatement(statement: BrandStatement, key: string): DraftStatement {
	return {
		...statement,
		token: statement.token ? { ...statement.token } : null,
		key,
		original: statement.token ? { ...statement.token } : null
	};
}

const sameVisual = (a: VisualToken, b: VisualToken) =>
	a.kind === b.kind && a.role.trim() === b.role.trim() && a.value.trim() === b.value.trim();

/** The save body for a drafted document. Unchanged statements keep their
 * evidence; edits name what they replace; anything left out is retired. */
export function identityStatementsPayload(draft: DraftStatement[]): IdentityStatementInput[] {
	return draft.flatMap((statement): IdentityStatementInput[] => {
		const token = statement.token && {
			kind: statement.token.kind,
			role: statement.token.role.trim(),
			value: statement.token.value.trim()
		};
		if (token ? !token.role || !token.value : !statement.text.trim()) return [];
		if (token && !(statement.id && statement.original && sameVisual(statement.original, token)))
			return [{ id: statement.id, pillar: 'visual', text: '', visual: token }];
		return [{ id: statement.id, pillar: statement.pillar, text: statement.text, visual: null }];
	});
}

/** The brand guide as Markdown, for a designer, printer or another tool. */
export function brandMarkdown(statements: BrandStatement[], title = 'Brand'): string {
	const out = [`# ${title.trim() || 'Brand'}`];
	for (const pillar of IDENTITY_PILLARS) {
		const items = statements.filter((statement) => statement.pillar === pillar.key);
		out.push('', `## ${pillar.label}`);
		if (!items.length) {
			out.push('', '_Not written._');
			continue;
		}
		for (const group of VISUAL_TOKENS) {
			const tokens = items.filter((item) => item.token?.kind === group.kind);
			if (!tokens.length) continue;
			out.push('', `### ${group.label}`, '');
			for (const item of tokens) out.push(`- ${item.token!.role}: ${item.token!.value}`);
		}
		for (const item of items.filter((item) => !item.token)) {
			const avoid = item.evidence?.polarity === 'negative' ? '**Avoid:** ' : '';
			out.push('', `${avoid}${item.text.trim()}`);
		}
	}
	return `${out.join('\n')}\n`;
}

export type IdentityStatementInput = {
	id: string | null;
	pillar: IdentityPillar;
	text: string;
	visual: VisualToken | null;
};

export async function saveCompanyIdentity(
	company: string,
	input: {
		expected_release: string | null;
		statements: IdentityStatementInput[];
		change_account?: string;
		amends_proposal?: string | null;
	}
): Promise<{ release_id: string }> {
	return ownerResponse(
		await fetch(`/api/companies/${encodeURIComponent(company)}/company/identity`, {
			method: 'PUT',
			credentials: 'same-origin',
			headers: { 'content-type': 'application/json' },
			body: JSON.stringify(input)
		})
	);
}
