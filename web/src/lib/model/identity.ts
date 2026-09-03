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

async function ownerResponse<T>(response: Response): Promise<T> {
	if (!response.ok) {
		let message = `${response.status} ${response.statusText}`;
		try {
			const body = await response.json();
			message = body.message ?? message;
		} catch {
			// Preserve the transport failure when an intermediary returns non-JSON.
		}
		throw Object.assign(new Error(message), { status: response.status });
	}
	return response.json() as Promise<T>;
}

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
