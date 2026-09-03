<script lang="ts">
	import { page } from '$app/state';
	import InfoTip from '$lib/components/InfoTip.svelte';
	import { identityQuery } from '$lib/model/queries.svelte';
	import {
		decideIdentityMigration,
		promoteIdentityProposal,
		rejectIdentityProposal,
		type CultureCaseRecordRow,
		type CultureEvidenceDetailRow,
		type CultureReviewRow,
		type IdentityEvidenceRow,
		type IdentityDriftFindingRow,
		type IdentityMigrationDisposition,
		type IdentityProposalRow,
		type VoiceEvidenceDetailRow,
		type VoiceRenderEvidenceRow,
		type VoiceReviewRow,
		type VisualEvidenceDetailRow,
		type VisualRenderEvidenceRow,
		type VisualReviewRow
	} from '$lib/model/identity';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(identityQuery(companyId));
	const view = $derived(source.view);
	const currentEvidenceIds = $derived(
		new Set(
			view?.release_evidence
				.filter((link) => link.release_id === view.current_release?.id)
				.map((link) => link.evidence_id) ?? []
		)
	);
	const currentEvidence = $derived(
		view?.evidence.filter((item) => currentEvidenceIds.has(item.id)) ?? []
	);
	const staleBindings = $derived(view?.bindings.filter((binding) => binding.stale_at) ?? []);
	const voiceContracts = $derived(view?.voice_work_contracts ?? []);
	const voiceRenders = $derived(view?.voice_render_evidence ?? []);
	const voiceReviews = $derived(view?.voice_reviews ?? []);
	const visualContracts = $derived(view?.visual_work_contracts ?? []);
	const visualUses = $derived(view?.visual_primitive_uses ?? []);
	const visualRenders = $derived(view?.visual_render_evidence ?? []);
	const visualReviews = $derived(view?.visual_reviews ?? []);
	const cultureContracts = $derived(view?.culture_work_contracts ?? []);
	const cultureCases = $derived(view?.culture_case_records ?? []);
	const cultureReviews = $derived(view?.culture_reviews ?? []);
	const constitutionBindings = $derived(view?.constitution_artifact_bindings ?? []);
	const migrationDecisions = $derived(view?.identity_migration_decisions ?? []);
	const decidedDrift = $derived(
		new Set(migrationDecisions.map((decision) => decision.drift_finding_id))
	);
	const consequentialDrift = $derived(
		(view?.identity_drift_findings ?? []).filter((finding) => !decidedDrift.has(finding.id))
	);
	let deciding = $state<string | null>(null);
	let migrationFinding = $state<string | null>(null);
	let migrationDisposition = $state<IdentityMigrationDisposition>('revise');
	let migrationRationale = $state('');
	let decisionText = $state('');
	let saving = $state(false);
	let notice = $state('');
	let failure = $state('');

	function evidenceFor(proposal: IdentityProposalRow): IdentityEvidenceRow[] {
		const ids = new Set(
			view?.proposal_evidence
				.filter((link) => link.proposal_id === proposal.id)
				.map((link) => link.evidence_id) ?? []
		);
		return view?.evidence.filter((item) => ids.has(item.id)) ?? [];
	}

	async function decide(proposal: IdentityProposalRow, decision: 'promote' | 'reject') {
		if (!decisionText.trim() || saving) return;
		saving = true;
		failure = '';
		notice = '';
		try {
			if (decision === 'promote') {
				await promoteIdentityProposal(companyId, proposal.id, decisionText);
				notice = 'Identity release promoted. New Work will use it; earlier Work keeps its release.';
			} else {
				await rejectIdentityProposal(companyId, proposal.id, decisionText);
				notice = 'Proposal rejected. Its attributed evidence remains available.';
			}
			deciding = null;
			decisionText = '';
			await source.refresh();
		} catch (cause) {
			failure = cause instanceof Error ? cause.message : 'The identity decision was not recorded.';
		} finally {
			saving = false;
		}
	}

	async function decideMigration(finding: IdentityDriftFindingRow) {
		if (!migrationRationale.trim() || saving) return;
		saving = true;
		failure = '';
		notice = '';
		try {
			await decideIdentityMigration(
				companyId,
				finding.id,
				migrationDisposition,
				migrationRationale
			);
			notice = `Migration recorded: ${migrationDisposition}. The artifact itself was not changed.`;
			migrationFinding = null;
			migrationRationale = '';
			await source.refresh();
		} catch (cause) {
			failure = cause instanceof Error ? cause.message : 'The migration decision was not recorded.';
		} finally {
			saving = false;
		}
	}

	function short(id: string): string {
		return id.slice(0, 8);
	}

	function when(value: string): string {
		return new Date(value).toLocaleString(undefined, {
			month: 'short',
			day: 'numeric',
			year: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function words(value: string): string {
		return value.replaceAll('_', ' ');
	}

	function voiceDetail(evidenceId: string): VoiceEvidenceDetailRow | undefined {
		return view?.voice_evidence_details.find((detail) => detail.evidence_id === evidenceId);
	}

	function renderFor(review: VoiceReviewRow): VoiceRenderEvidenceRow | undefined {
		return voiceRenders.find((render) => render.id === review.render_evidence_id);
	}

	function firstFinding(review: VoiceReviewRow): string {
		return (
			[
				review.factual_findings,
				review.abstraction_findings,
				review.repetition_findings,
				review.channel_findings,
				review.authorship_findings
			].find((finding) => finding.trim()) ?? 'No revision finding recorded.'
		);
	}

	function visualDetail(evidenceId: string): VisualEvidenceDetailRow | undefined {
		return view?.visual_evidence_details.find((detail) => detail.evidence_id === evidenceId);
	}

	function visualRenderFor(review: VisualReviewRow): VisualRenderEvidenceRow | undefined {
		return visualRenders.find((render) => render.id === review.render_evidence_id);
	}

	function visualFinding(review: VisualReviewRow): string {
		return (
			[
				review.identity_findings,
				review.hierarchy_findings,
				review.proof_findings,
				review.product_fidelity_findings,
				review.motion_findings,
				review.defect_findings
			].find((finding) => finding.trim()) ?? 'No revision finding recorded.'
		);
	}
	function cultureDetail(evidenceId: string): CultureEvidenceDetailRow | undefined {
		return view?.culture_evidence_details.find((detail) => detail.evidence_id === evidenceId);
	}
	function cultureCaseFor(review: CultureReviewRow): CultureCaseRecordRow | undefined {
		return cultureCases.find((record) => record.id === review.case_record_id);
	}
	function cultureFinding(review: CultureReviewRow): string {
		return (
			[
				review.conduct_findings,
				review.dissent_findings,
				review.uncertainty_findings,
				review.correction_findings,
				review.authority_findings,
				review.customer_or_hiring_findings
			].find((finding) => finding.trim()) ?? 'No conduct finding recorded.'
		);
	}
</script>

<svelte:head><title>Company identity — {companyId}</title></svelte:head>

<div class="company-page identity-page">
	<header class="company-page-head">
		<div class="identity-title">
			<h1>Company identity</h1>
			<InfoTip
				text="The owner-released truth and expression evidence Restless uses when producing company work. Drafts can propose changes; they cannot make themselves authoritative."
			/>
		</div>
		<div class="company-page-freshness">
			<span class="source-lamp status-{source.status}" aria-hidden="true"></span>
			{source.status === 'live'
				? 'Live source'
				: source.status === 'stale'
					? 'Last observation'
					: 'Reading source'}
		</div>
	</header>

	{#if failure}<p class="identity-message failure" role="alert">{failure}</p>{/if}
	{#if notice}<p class="identity-message" role="status">{notice}</p>{/if}

	{#if view}
		{#if view.current_release}
			<section class="release-ledger" aria-labelledby="effective-release">
				<div class="release-mark" aria-hidden="true">
					<span>Effective</span>
					<strong>{short(view.current_release.id)}</strong>
				</div>
				<div class="release-account">
					<div class="section-heading">
						<h2 id="effective-release">Current release</h2>
						<InfoTip
							text="This immutable release is the exact identity new Work binds to. A later release never rewrites old outcomes."
						/>
					</div>
					<p>{view.current_release.change_account}</p>
					<dl>
						<div>
							<dt>Effective</dt>
							<dd>{when(view.current_release.effective_from)}</dd>
						</div>
						<div>
							<dt>Promoted by</dt>
							<dd>{view.current_release.promoted_by}</dd>
						</div>
						<div>
							<dt>Authority</dt>
							<dd>{view.current_release.authority_record_id}</dd>
						</div>
						<div>
							<dt>Evidence</dt>
							<dd>{currentEvidence.length} source-owned statements</dd>
						</div>
					</dl>
				</div>
				<div class="binding-facts">
					<strong>{constitutionBindings.length}</strong>
					<span>exact bound artifacts</span>
					{#if staleBindings.length}<em>{staleBindings.length} need review after a correction</em
						>{/if}
				</div>
			</section>
		{:else}
			<section class="identity-empty">
				<h2>No owner-authored identity yet</h2>
				<p>
					Restless will not pretend generic defaults are this company’s voice, visual language or
					culture. Staff can gather attributed evidence and prepare a proposal for your decision.
				</p>
			</section>
		{/if}

		{#if consequentialDrift.length || view.constitution_learning_proposals.length}
			<section class="identity-impact" aria-labelledby="identity-impact-title">
				<div class="section-heading">
					<h2 id="identity-impact-title">Identity impact</h2>
					<InfoTip
						text="Only consequential evidence changes and owner decisions appear here. Routine compliant production stays quiet."
					/>
				</div>
				{#if consequentialDrift.length}
					<div class="impact-list">
						{#each consequentialDrift as finding (finding.id)}
							<article class="impact-card">
								<header>
									<div>
										<strong>{words(finding.kind)}</strong><span
											>Artifact {short(finding.artifact_ref_id)}</span
										>
									</div>
									<code>{short(finding.from_release_id)} → {short(finding.to_release_id)}</code>
								</header>
								<p>{finding.dependency}</p>
								<small>{finding.consequence}</small>
								{#if migrationFinding === finding.id}
									<div class="migration-decision">
										<fieldset>
											<legend>What should happen to this exact artifact?</legend>
											{#each ['retain', 'revise', 'retire'] as disposition}
												<label
													><input
														type="radio"
														bind:group={migrationDisposition}
														value={disposition}
													/>{disposition}</label
												>
											{/each}
										</fieldset>
										<label
											><span>Decision account</span><textarea
												bind:value={migrationRationale}
												placeholder="Why this disposition is correct for this artifact"
											></textarea></label
										>
										<div class="proposal-actions">
											<button
												class="btn small"
												type="button"
												onclick={() => {
													migrationFinding = null;
													migrationRationale = '';
												}}>Cancel</button
											>
											<button
												class="btn primary small"
												type="button"
												disabled={saving || !migrationRationale.trim()}
												onclick={() => decideMigration(finding)}
												>{saving ? 'Recording…' : 'Record decision'}</button
											>
										</div>
									</div>
								{:else}
									<button
										class="btn small"
										type="button"
										onclick={() => {
											migrationFinding = finding.id;
											migrationDisposition = 'revise';
											migrationRationale = '';
										}}>Decide migration</button
									>
								{/if}
							</article>
						{/each}
					</div>
				{/if}
				{#if view.constitution_learning_proposals.length}
					<p class="learning-account">
						{view.constitution_learning_proposals.length} attributed learning {view
							.constitution_learning_proposals.length === 1
							? 'proposal is'
							: 'proposals are'} linked to exact before-and-after artifacts. Promotion remains an owner
						decision.
					</p>
				{/if}
			</section>
		{/if}

		{#if view.pending_proposals.length}
			<section class="proposal-section">
				<div class="section-heading">
					<h2>Waiting for your decision</h2>
					<InfoTip
						text="Promotion makes a complete evidence set effective. Rejection keeps the evidence and attribution without turning it into a permanent ban."
					/>
				</div>
				<div class="proposal-list">
					{#each view.pending_proposals as proposal (proposal.id)}
						<article class="proposal-card">
							<div>
								<strong>{proposal.rationale}</strong>
								<span>Proposed by {proposal.created_by} · {when(proposal.created_at)}</span>
							</div>
							<div class="proposal-evidence">
								{#each evidenceFor(proposal).slice(0, 4) as item (item.id)}
									<span>{words(item.pillar)} · {words(item.statement_kind)}</span>
								{/each}
								{#if evidenceFor(proposal).length > 4}<span
										>+{evidenceFor(proposal).length - 4} more</span
									>{/if}
							</div>
							{#if deciding === proposal.id}
								<label>
									<span>Your decision account</span>
									<textarea
										bind:value={decisionText}
										placeholder="What changed, or why this should not become current"></textarea>
								</label>
								<div class="proposal-actions">
									<button
										class="btn small"
										type="button"
										disabled={saving}
										onclick={() => {
											deciding = null;
											decisionText = '';
										}}>Cancel</button
									>
									<button
										class="btn small"
										type="button"
										disabled={saving || !decisionText.trim()}
										onclick={() => decide(proposal, 'reject')}>Reject</button
									>
									<button
										class="btn primary small"
										type="button"
										disabled={saving || !decisionText.trim()}
										onclick={() => decide(proposal, 'promote')}
										>{saving ? 'Recording…' : 'Promote release'}</button
									>
								</div>
							{:else}
								<button
									class="btn small"
									type="button"
									onclick={() => {
										deciding = proposal.id;
										decisionText = '';
									}}>Review proposal</button
								>
							{/if}
						</article>
					{/each}
				</div>
			</section>
		{/if}

		{#if currentEvidence.length}
			{#if voiceContracts.length || voiceReviews.length}
				<section class="voice-in-use" aria-labelledby="voice-in-use-title">
					<div class="section-heading">
						<h2 id="voice-in-use-title">Voice in use</h2>
						<InfoTip
							text="Every voice-producing Work keeps one explicit reader, author, proof and consequence contract. Copy-desk decisions attach to the exact native artifact reviewed."
						/>
					</div>
					<div class="voice-ledger">
						<div class="voice-contract-list">
							<h3>Bound situations</h3>
							{#each voiceContracts.slice(-6).reverse() as contract (contract.work_id)}
								<article>
									<div>
										<strong>{words(contract.channel)}</strong>
										<span>{contract.author} → {contract.audience}</span>
									</div>
									<p>{contract.reader_situation}</p>
									<small>Proof: {contract.proof} · bound by {contract.bound_by}</small>
								</article>
							{/each}
						</div>
						<div class="voice-review-list">
							<h3>Native copy desk</h3>
							{#each voiceReviews.slice(-6).reverse() as review (review.id)}
								<article class:accepted={review.verdict === 'accept'}>
									<header>
										<strong>{words(review.verdict)}</strong>
										<span>{renderFor(review) ? words(renderFor(review)!.channel) : 'artifact'}</span
										>
									</header>
									<p>{firstFinding(review)}</p>
									<small>{review.reviewer} · {when(review.created_at)}</small>
								</article>
							{/each}
							{#if !voiceReviews.length}<p class="pillar-empty">
									No native copy-desk decision yet.
								</p>{/if}
						</div>
					</div>
				</section>
			{/if}

			{#if visualContracts.length || visualReviews.length}
				<section class="visual-in-use" aria-labelledby="visual-in-use-title">
					<div class="section-heading">
						<h2 id="visual-in-use-title">Visual language in use</h2>
						<InfoTip
							text="Visual direction binds product truth, one outcome and native review states. Registry primitives are inspectable capabilities, never a component quota."
						/>
					</div>
					<div class="visual-ledger">
						<div class="visual-contract-list">
							<h3>Native targets</h3>
							{#each visualContracts.slice(-6).reverse() as contract (contract.work_id)}
								<article>
									<header>
										<strong>{words(contract.channel)}</strong><span
											>{words(contract.product_representation)}</span
										>
									</header>
									<p>{contract.outcome}</p>
									<small>{contract.information_hierarchy}</small>
									{#if contract.requested_departure}<em
											>Departure: {contract.requested_departure}</em
										>{/if}
								</article>
							{/each}
						</div>
						<div class="visual-review-list">
							<h3>Native art direction</h3>
							{#each visualReviews.slice(-6).reverse() as review (review.id)}
								<article class:accepted={review.verdict === 'accept'}>
									<header>
										<strong>{words(review.verdict)}</strong><span
											>{visualRenderFor(review)
												? `${visualRenderFor(review)!.viewport_width} × ${visualRenderFor(review)!.viewport_height} · ${words(visualRenderFor(review)!.motion_state)}`
												: 'exact artifact'}</span
										>
									</header>
									<p>{visualFinding(review)}</p>
									{#if review.control_render_evidence_id}<small
											>Compared with restrained control</small
										>{/if}
									{#if review.departure_decision}<em>{review.departure_decision}</em>{/if}
								</article>
							{/each}
							{#if !visualReviews.length}<p class="pillar-empty">
									No native art-direction decision yet.
								</p>{/if}
						</div>
					</div>
					{#if visualUses.length}<p class="visual-use-account">
							{visualUses.length} exact primitive {visualUses.length === 1 ? 'version' : 'versions'} used
							across bound Work. Availability does not imply use.
						</p>{/if}
				</section>
			{/if}

			{#if cultureContracts.length || cultureReviews.length}
				<section class="culture-in-use" aria-labelledby="culture-in-use-title">
					<div class="section-heading">
						<h2 id="culture-in-use-title">Operating culture in use</h2>
						<InfoTip
							text="Culture here means observed conduct under consequence. It cannot score people, infer personality, grant authority or suppress disagreement."
						/>
					</div>
					<div class="visual-ledger">
						<div class="visual-contract-list">
							<h3>Bound situations</h3>
							{#each cultureContracts.slice(-6).reverse() as contract (contract.work_id)}<article>
									<header>
										<strong>{words(contract.case_kind)}</strong><span
											>{contract.actor_role} · {contract.team}</span
										>
									</header>
									<p>{contract.consequence}</p>
									<small>{contract.decision_boundary}</small>
								</article>{/each}
						</div>
						<div class="visual-review-list">
							<h3>Independent conduct review</h3>
							{#each cultureReviews.slice(-6).reverse() as review (review.id)}<article
									class:accepted={review.verdict === 'accept'}
								>
									<header>
										<strong>{words(review.verdict)}</strong><span
											>{cultureCaseFor(review)
												? words(cultureCaseFor(review)!.case_kind)
												: 'exact case'}</span
										>
									</header>
									<p>{cultureFinding(review)}</p>
									{#if cultureCaseFor(review)?.correction_of}<small
											>Visible correction · original retained</small
										>{/if}
								</article>{/each}
							{#if !cultureReviews.length}<p class="pillar-empty">
									No independent conduct decision yet.
								</p>{/if}
						</div>
					</div>
				</section>
			{/if}

			<section class="evidence-section">
				<div class="section-heading">
					<h2>Released evidence</h2>
					<InfoTip
						text="Facts, beliefs, guidance, observations, examples and exceptions keep their distinct meaning. Open the locator to inspect the exact source when it is available in the company computer."
					/>
				</div>
				<div class="pillar-grid">
					{#each ['truth', 'voice', 'visual', 'culture'] as pillar}
						<section class="pillar">
							<header>
								<h3>{pillar}</h3>
								<span>{currentEvidence.filter((item) => item.pillar === pillar).length}</span>
							</header>
							{#each currentEvidence.filter((item) => item.pillar === pillar) as item (item.id)}
								<article
									class:negative={item.polarity === 'negative'}
									class:disputed={item.status === 'disputed'}
								>
									<div class="evidence-kind">
										{cultureDetail(item.id)
											? words(cultureDetail(item.id)!.kind)
											: visualDetail(item.id)
												? words(visualDetail(item.id)!.kind)
												: voiceDetail(item.id)
													? words(voiceDetail(item.id)!.kind)
													: words(item.statement_kind)}{item.polarity === 'negative'
											? ' · negative evidence'
											: ''}
									</div>
									<p>{item.statement}</p>
									{#if voiceDetail(item.id)}
										<p class="voice-judgement">Why: {voiceDetail(item.id)!.judgement_reason}</p>
										<div class="voice-scopes">
											{#if voiceDetail(item.id)!.channel}<span
													>{words(voiceDetail(item.id)!.channel!)}</span
												>{/if}
											{#if voiceDetail(item.id)!.named_author}<span
													>{voiceDetail(item.id)!.named_author}</span
												>{/if}
											{#if voiceDetail(item.id)!.audience}<span
													>{voiceDetail(item.id)!.audience}</span
												>{/if}
										</div>
									{/if}
									{#if cultureDetail(item.id)}
										<p class="voice-judgement">Observed: {cultureDetail(item.id)!.conduct}</p>
										<p class="voice-judgement">
											Consequence: {cultureDetail(item.id)!.consequence}
										</p>
										<div class="voice-scopes">
											<span>{words(cultureDetail(item.id)!.confidence)}</span><span
												>{cultureDetail(item.id)!.actor_scope}</span
											>{#if cultureDetail(item.id)!.case_kind}<span
													>{words(cultureDetail(item.id)!.case_kind!)}</span
												>{/if}
										</div>
										<p class="culture-counterexample">
											<strong>Counterexample</strong>
											{cultureDetail(item.id)!.counterexample}
										</p>
										<p class="culture-boundary">
											<strong>Boundary</strong>
											{cultureDetail(item.id)!.boundary_conditions}
										</p>
									{/if}
									{#if visualDetail(item.id)}
										<p class="voice-judgement">Why: {visualDetail(item.id)!.rationale}</p>
										<div class="voice-scopes">
											<span>{visualDetail(item.id)!.purpose}</span>
											{#if visualDetail(item.id)!.channel}<span
													>{words(visualDetail(item.id)!.channel!)}</span
												>{/if}
											{#if visualDetail(item.id)!.licence}<span
													>{visualDetail(item.id)!.licence} · {visualDetail(item.id)!
														.framework}</span
												>{/if}
											{#if visualDetail(item.id)!.reduced_motion_replacement}<span
													>Reduced: {visualDetail(item.id)!.reduced_motion_replacement}</span
												>{/if}
										</div>
									{/if}
									<footer>
										<span>{item.author_id} · {item.source}</span>
										<code title={item.evidence_locator}>{item.evidence_locator}</code>
									</footer>
								</article>
							{/each}
							{#if !currentEvidence.some((item) => item.pillar === pillar)}
								<p class="pillar-empty">No released {pillar} evidence.</p>
							{/if}
						</section>
					{/each}
				</div>
			</section>
		{/if}

		{#if view.releases.length > 1}
			<section class="release-history">
				<h2>Release history</h2>
				{#each view.releases as release (release.id)}
					<article class:current={release.id === view.current_release?.id}>
						<code>{short(release.id)}</code>
						<div>
							<strong>{release.change_account}</strong><span
								>{when(release.effective_from)} · {release.promoted_by}</span
							>
						</div>
					</article>
				{/each}
			</section>
		{/if}
	{:else if source.failure}
		<div class="company-source-error" role="alert">{source.failure.message}</div>
	{:else}
		<div class="company-page-wait" aria-label="Reading company identity"></div>
	{/if}
</div>

<style>
	.identity-page {
		gap: 24px;
		container-type: inline-size;
	}
	.identity-title {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.identity-message {
		padding: 10px 12px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--text-secondary);
	}
	.identity-message.failure {
		border-color: color-mix(in srgb, var(--danger) 35%, var(--border));
		color: var(--danger);
	}
	.release-ledger {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		gap: 24px;
		align-items: stretch;
		padding: 22px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-pane);
		background: var(--surface);
		box-shadow: var(--bevel-subtle);
	}
	.release-mark {
		display: grid;
		place-content: center;
		width: 116px;
		aspect-ratio: 1;
		border: 1px solid var(--border-strong);
		border-radius: 50%;
		text-align: center;
		box-shadow: inset 0 0 0 5px var(--surface-muted);
	}
	.release-mark span {
		color: var(--text-secondary);
		font-size: var(--t-caption);
	}
	.release-mark strong {
		font: 600 var(--t-body) var(--font-mono);
		letter-spacing: 0.04em;
	}
	.release-account > p {
		max-width: 68ch;
		margin: 10px 0 16px;
		color: var(--text-secondary);
	}
	.release-account dl {
		display: flex;
		flex-wrap: wrap;
		gap: 12px 24px;
		margin: 0;
	}
	.release-account dl div {
		display: grid;
		gap: 2px;
	}
	.release-account dt {
		color: var(--text-tertiary);
		font-size: var(--t-caption);
	}
	.release-account dd {
		margin: 0;
		color: var(--text-primary);
		font-size: var(--t-body-small);
	}
	.binding-facts {
		display: grid;
		align-content: center;
		min-width: 150px;
		padding-left: 22px;
		border-left: 1px solid var(--border);
	}
	.binding-facts strong {
		font-size: var(--t-head);
		line-height: 1;
	}
	.binding-facts span {
		margin-top: 5px;
		color: var(--text-secondary);
	}
	.binding-facts em {
		max-width: 20ch;
		margin-top: 12px;
		color: var(--warning, #9a6500);
		font-size: var(--t-caption);
		font-style: normal;
	}
	.identity-empty {
		padding: 36px;
		border: 1px dashed var(--border-strong);
		border-radius: var(--radius-pane);
		background: var(--surface-muted);
	}
	.identity-empty p {
		max-width: 65ch;
		color: var(--text-secondary);
	}
	.proposal-section,
	.voice-in-use,
	.visual-in-use,
	.culture-in-use,
	.evidence-section,
	.release-history {
		display: grid;
		gap: 14px;
	}
	.proposal-list {
		display: grid;
		gap: 10px;
	}
	.proposal-card {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		align-items: center;
		gap: 18px;
		padding: 16px 18px;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface);
	}
	.proposal-card > div:first-child {
		display: grid;
		gap: 4px;
	}
	.proposal-card > div:first-child span {
		color: var(--text-secondary);
		font-size: var(--t-caption);
	}
	.proposal-evidence {
		display: flex;
		flex-wrap: wrap;
		justify-content: flex-end;
		gap: 5px;
	}
	.proposal-evidence span {
		padding: 3px 7px;
		border-radius: 999px;
		background: var(--surface-muted);
		color: var(--text-secondary);
		font-size: var(--t-caption);
	}
	.proposal-card label {
		grid-column: 1 / -1;
		display: grid;
		gap: 6px;
	}
	.proposal-card label span {
		color: var(--text-secondary);
		font-size: var(--t-body-small);
	}
	.proposal-card textarea {
		min-height: 84px;
		resize: vertical;
		padding: 10px 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--text-primary);
		font: inherit;
	}
	.proposal-actions {
		grid-column: 1 / -1;
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
	.voice-ledger {
		display: grid;
		grid-template-columns: minmax(0, 1.2fr) minmax(260px, 0.8fr);
		gap: 12px;
	}
	.visual-ledger {
		display: grid;
		grid-template-columns: minmax(0, 1.2fr) minmax(280px, 0.8fr);
		gap: 12px;
	}
	.visual-contract-list,
	.visual-review-list {
		min-width: 0;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface);
	}
	.visual-ledger h3 {
		margin: 0;
		padding: 12px 14px;
		border-bottom: 1px solid var(--border);
		background: var(--surface-muted);
		font-size: var(--t-body);
	}
	.visual-ledger article {
		display: grid;
		gap: 6px;
		padding: 13px 14px;
		border-bottom: 1px solid var(--border);
	}
	.visual-ledger article:last-child {
		border-bottom: 0;
	}
	.visual-ledger header {
		display: flex;
		justify-content: space-between;
		gap: 12px;
		align-items: baseline;
	}
	.visual-ledger p {
		margin: 0;
		color: var(--text-secondary);
	}
	.visual-ledger span,
	.visual-ledger small {
		color: var(--text-tertiary);
		font-size: var(--t-caption);
	}
	.visual-ledger em {
		color: var(--text-secondary);
		font-size: var(--t-caption);
		font-style: normal;
	}
	.visual-review-list article:not(.accepted) {
		box-shadow: inset 3px 0 var(--warning, #9a6500);
	}
	.visual-review-list article.accepted strong {
		color: var(--success);
	}
	.visual-use-account {
		margin: 0;
		color: var(--text-tertiary);
		font-size: var(--t-caption);
	}
	.culture-counterexample,
	.culture-boundary {
		margin: 0;
		padding: 7px 8px;
		border-left: 2px solid var(--border-strong);
		background: var(--surface-muted);
		color: var(--text-secondary);
		font-size: var(--t-caption);
	}
	.culture-counterexample strong,
	.culture-boundary strong {
		margin-right: 4px;
		color: var(--text-primary);
	}
	.voice-contract-list,
	.voice-review-list {
		min-width: 0;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface);
	}
	.voice-contract-list > h3,
	.voice-review-list > h3 {
		margin: 0;
		padding: 12px 14px;
		border-bottom: 1px solid var(--border);
		background: var(--surface-muted);
		font-size: var(--t-body);
	}
	.voice-ledger article {
		display: grid;
		gap: 6px;
		padding: 13px 14px;
		border-bottom: 1px solid var(--border);
	}
	.voice-ledger article:last-child {
		border-bottom: 0;
	}
	.voice-ledger article > div,
	.voice-ledger article > header {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: 12px;
	}
	.voice-ledger article p {
		margin: 0;
		color: var(--text-secondary);
	}
	.voice-ledger article span,
	.voice-ledger article small {
		color: var(--text-tertiary);
		font-size: var(--t-caption);
	}
	.voice-review-list article:not(.accepted) {
		box-shadow: inset 3px 0 var(--warning, #9a6500);
	}
	.voice-review-list article.accepted strong {
		color: var(--success);
	}
	.voice-judgement {
		color: var(--text-secondary) !important;
		font-size: var(--t-body-small);
	}
	.voice-scopes {
		display: flex;
		flex-wrap: wrap;
		gap: 5px;
	}
	.voice-scopes span {
		padding: 2px 6px;
		border: 1px solid var(--border);
		border-radius: 999px;
		color: var(--text-tertiary);
		font-size: var(--t-caption);
	}
	.pillar-grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 12px;
	}
	.pillar {
		min-width: 0;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: var(--radius-pane);
		background: var(--surface);
	}
	.pillar > header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 12px 14px;
		border-bottom: 1px solid var(--border);
		background: var(--surface-muted);
	}
	.pillar h3 {
		margin: 0;
		text-transform: capitalize;
		font-size: var(--t-body);
	}
	.pillar > header span {
		color: var(--text-tertiary);
		font: var(--t-caption) var(--font-mono);
	}
	.pillar article {
		display: grid;
		gap: 7px;
		padding: 14px;
		border-bottom: 1px solid var(--border);
	}
	.pillar article:last-child {
		border-bottom: 0;
	}
	.pillar article.negative {
		box-shadow: inset 3px 0 var(--danger);
	}
	.pillar article.disputed {
		background: color-mix(in srgb, var(--danger) 5%, var(--surface));
	}
	.evidence-kind {
		color: var(--text-tertiary);
		font-size: var(--t-caption);
		text-transform: capitalize;
	}
	.pillar article p {
		margin: 0;
		color: var(--text-primary);
	}
	.pillar article footer {
		display: grid;
		gap: 3px;
		color: var(--text-secondary);
		font-size: var(--t-caption);
	}
	.pillar article code {
		overflow: hidden;
		color: var(--text-tertiary);
		font: var(--t-caption) var(--font-mono);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.pillar-empty {
		margin: 0;
		padding: 18px 14px;
		color: var(--text-tertiary);
		font-size: var(--t-body-small);
	}
	.identity-impact,
	.impact-list {
		display: grid;
		gap: 12px;
	}
	.impact-card {
		display: grid;
		gap: 10px;
		padding: 16px 18px;
		border: 1px solid color-mix(in srgb, var(--warning, #9a6500) 35%, var(--border));
		border-radius: var(--radius-pane);
		background: var(--surface);
		box-shadow: inset 3px 0 var(--warning, #9a6500);
	}
	.impact-card > header {
		display: flex;
		align-items: start;
		justify-content: space-between;
		gap: 16px;
	}
	.impact-card > header div {
		display: grid;
		gap: 3px;
	}
	.impact-card > header span,
	.impact-card small,
	.learning-account {
		color: var(--text-secondary);
		font-size: var(--t-body-small);
	}
	.impact-card p,
	.learning-account {
		margin: 0;
	}
	.impact-card code {
		color: var(--text-tertiary);
		font: var(--t-caption) var(--font-mono);
	}
	.migration-decision {
		display: grid;
		gap: 12px;
		padding-top: 4px;
		border-top: 1px solid var(--border);
	}
	.migration-decision fieldset {
		display: flex;
		flex-wrap: wrap;
		gap: 8px 16px;
		margin: 0;
		padding: 0;
		border: 0;
	}
	.migration-decision legend,
	.migration-decision > label {
		width: 100%;
		color: var(--text-secondary);
		font-size: var(--t-body-small);
	}
	.migration-decision > label {
		display: grid;
		gap: 6px;
	}
	.migration-decision fieldset label {
		display: flex;
		align-items: center;
		gap: 5px;
		text-transform: capitalize;
	}
	.release-history article {
		display: grid;
		grid-template-columns: 90px 1fr;
		gap: 14px;
		align-items: start;
		padding: 10px 0;
		border-top: 1px solid var(--border);
	}
	.release-history article.current code {
		color: var(--accent);
	}
	.release-history article div {
		display: grid;
		gap: 3px;
	}
	.release-history article span {
		color: var(--text-secondary);
		font-size: var(--t-caption);
	}
	@media (max-width: 820px) {
		.voice-ledger,
		.visual-ledger {
			grid-template-columns: 1fr;
		}
		.release-ledger {
			grid-template-columns: auto 1fr;
		}
		.binding-facts {
			grid-column: 1 / -1;
			padding: 14px 0 0;
			border-top: 1px solid var(--border);
			border-left: 0;
		}
		.proposal-card {
			grid-template-columns: 1fr;
		}
		.proposal-evidence {
			justify-content: flex-start;
		}
		.pillar-grid {
			grid-template-columns: 1fr;
		}
	}
	@container (max-width: 720px) {
		.visual-ledger {
			grid-template-columns: 1fr;
		}
		.release-ledger {
			grid-template-columns: 1fr;
		}
		.release-mark {
			width: 88px;
		}
		.binding-facts {
			grid-column: 1;
			padding: 14px 0 0;
			border-top: 1px solid var(--border);
			border-left: 0;
		}
		.proposal-card {
			grid-template-columns: 1fr;
		}
		.proposal-evidence {
			justify-content: flex-start;
		}
		.proposal-card > .btn {
			justify-self: start;
		}
		.pillar-grid {
			grid-template-columns: 1fr;
		}
	}
	@media (max-width: 540px) {
		.release-ledger {
			grid-template-columns: 1fr;
		}
		.release-mark {
			width: 88px;
		}
	}
</style>
