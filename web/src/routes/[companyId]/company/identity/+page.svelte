<script lang="ts">
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { formatRelative, formatMoment } from '$lib/ui/time';
	import { Page, Section, Row, Item, Notice, Segmented, Fold } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { failureSentence } from '$lib/model/failure';
	import IdentityEditor from '$lib/components/IdentityEditor.svelte';
	import { page } from '$app/state';
	import { identityQuery } from '$lib/model/queries.svelte';
	import FileQuestion from '@lucide/svelte/icons/file-question';
	import GitBranch from '@lucide/svelte/icons/git-branch';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import {
		decideIdentityMigration,
		promoteIdentityProposal,
		rejectIdentityProposal,
		type IdentityDriftFindingRow,
		type IdentityEvidenceRow,
		type IdentityMigrationDisposition,
		type IdentityProposalRow
	} from '$lib/model/identity';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const source = $derived(identityQuery(companyId));
	const view = $derived(source.view);
	const currentEvidence = $derived.by(() => {
		if (!view) return [];
		const ids = new Set(
			view.release_evidence
				.filter((link) => link.release_id === view.current_release?.id)
				.map((link) => link.evidence_id)
		);
		return view.evidence.filter((item) => ids.has(item.id));
	});
	const staleBindings = $derived(view?.bindings.filter((binding) => binding.stale_at).length ?? 0);
	const openDrift = $derived.by(() => {
		if (!view) return [];
		const decided = new Set(view.identity_migration_decisions.map((d) => d.drift_finding_id));
		return view.identity_drift_findings.filter((finding) => !decided.has(finding.id));
	});
	/* Usage records: how bound Work used the identity and what reviewers found.
	 * Detail for anyone who asks, never a wall on the page. */
	const usage = $derived.by(() => {
		if (!view) return [];
		const reviews = [
			...view.voice_reviews.map((r) => ({
				id: r.id,
				pillar: 'Voice',
				verdict: r.verdict,
				finding: firstOf(
					r.factual_findings,
					r.abstraction_findings,
					r.repetition_findings,
					r.channel_findings,
					r.authorship_findings
				),
				by: r.reviewer,
				at: r.created_at
			})),
			...view.visual_reviews.map((r) => ({
				id: r.id,
				pillar: 'Visual',
				verdict: r.verdict,
				finding: firstOf(
					r.identity_findings,
					r.hierarchy_findings,
					r.proof_findings,
					r.product_fidelity_findings,
					r.motion_findings,
					r.defect_findings
				),
				by: '',
				at: ''
			})),
			...view.culture_reviews.map((r) => ({
				id: r.id,
				pillar: 'Culture',
				verdict: r.verdict,
				finding: firstOf(
					r.conduct_findings,
					r.dissent_findings,
					r.uncertainty_findings,
					r.correction_findings,
					r.authority_findings,
					r.customer_or_hiring_findings
				),
				by: '',
				at: ''
			}))
		];
		return reviews;
	});
	const PILLARS = [
		['truth', 'Truth'],
		['voice', 'Voice'],
		['visual', 'Visual language'],
		['culture', 'Culture']
	] as const;
	const DISPOSITIONS: { value: IdentityMigrationDisposition; label: string; title: string }[] = [
		{ value: 'retain', label: 'Keep', title: 'Keep the artifact as it is.' },
		{ value: 'revise', label: 'Revise', title: 'Have the artifact revised to the new identity.' },
		{ value: 'retire', label: 'Retire', title: 'Stop using the artifact.' }
	];

	let deciding = $state<string | null>(null);
	let decisionText = $state('');
	let migrationFinding = $state<string | null>(null);
	let migrationDisposition = $state<IdentityMigrationDisposition>('revise');
	let migrationRationale = $state('');
	let saving = $state(false);
	let notice = $state('');
	let failure = $state('');

	function firstOf(...findings: string[]): string {
		return findings.find((finding) => finding.trim()) ?? '';
	}
	function evidenceFor(proposal: IdentityProposalRow): IdentityEvidenceRow[] {
		if (!view) return [];
		const ids = new Set(
			view.proposal_evidence
				.filter((link) => link.proposal_id === proposal.id)
				.map((link) => link.evidence_id)
		);
		return view.evidence.filter((item) => ids.has(item.id));
	}

	async function decide(proposal: IdentityProposalRow, decision: 'promote' | 'reject') {
		if (!decisionText.trim() || saving) return;
		saving = true;
		failure = '';
		notice = '';
		try {
			if (decision === 'promote') {
				await promoteIdentityProposal(companyId, proposal.id, decisionText);
				notice = 'Promoted. New work will use this version; earlier work keeps its own.';
			} else {
				await rejectIdentityProposal(companyId, proposal.id, decisionText);
				notice = 'Rejected. Its evidence stays available.';
			}
			deciding = null;
			decisionText = '';
			await source.refresh();
		} catch (cause) {
			failure = failureSentence(cause, 'The identity decision was not recorded.');
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
			notice = 'Decision recorded. The artifact itself was not changed.';
			migrationFinding = null;
			migrationRationale = '';
			await source.refresh();
		} catch (cause) {
			failure = failureSentence(cause, 'The decision was not recorded.');
		} finally {
			saving = false;
		}
	}

	const when = (value?: Date | string) => formatRelative(value, '');
	const words = (value: string) => value.replaceAll('_', ' ');
	const sentence = (value: string) => {
		const text = words(value);
		return text.charAt(0).toUpperCase() + text.slice(1);
	};
</script>

<CompanyTitle title="Identity" {companyId} />

<Page
	title="Identity"
	info="What the company is and how it sounds. Agents use it in all company work. Drafts can suggest changes; only you approve them."
>
	{#if view}
		{#if failure}<Notice
				tone="danger"
				title="That decision was not recorded"
				details={failure}
			/>{/if}
		{#if notice}<Notice tone="success" title={notice} />{/if}

		{#if view.pending_proposals.length}
			<Section
				title="Waiting for your decision"
				count={view.pending_proposals.length}
				info="Promoting makes a proposal the current version. Rejecting keeps its evidence without banning it."
			>
				{#each view.pending_proposals as proposal (proposal.id)}
					{@const evidence = evidenceFor(proposal)}
					<Item
						title={proposal.rationale}
						meta={`${proposal.created_by} · ${evidence.length} statement${evidence.length === 1 ? '' : 's'}`}
						onclick={() => {
							deciding = deciding === proposal.id ? null : proposal.id;
							decisionText = '';
						}}
						selected={deciding === proposal.id}
						unread
					>
						{#snippet leading()}<FileQuestion size={15} strokeWidth={1.8} />{/snippet}
						{#snippet trailing()}<time title={formatMoment(proposal.created_at)}
								>{when(proposal.created_at)}</time
							>{/snippet}
						{#if deciding === proposal.id}
							<div class="decision">
								<ul class="statements">
									{#each evidence as item (item.id)}<li>
											<span class="kind">{sentence(item.pillar)}</span>{item.statement}
										</li>{/each}
								</ul>
								<textarea
									bind:value={decisionText}
									aria-label="Your reason"
									placeholder="Your reason, sent with the decision"></textarea>
								<div class="decision-bar">
									<button
										class="btn primary small"
										type="button"
										disabled={saving || !decisionText.trim()}
										onclick={() => decide(proposal, 'promote')}
										>{saving ? 'Recording…' : 'Promote'}</button
									>
									<button
										class="btn small"
										type="button"
										disabled={saving || !decisionText.trim()}
										onclick={() => decide(proposal, 'reject')}>Reject</button
									>
									<button
										class="btn small ghost"
										type="button"
										disabled={saving}
										onclick={() => (deciding = null)}>Cancel</button
									>
								</div>
							</div>
						{/if}
					</Item>
				{/each}
			</Section>
		{/if}

		{#if openDrift.length}
			<Section
				title="Artifacts affected by a change"
				count={openDrift.length}
				info="A newer identity changed something an existing artifact depends on. Decide what happens to each one."
			>
				{#each openDrift as finding (finding.id)}
					<Item
						title={finding.dependency}
						meta={sentence(finding.kind)}
						onclick={() => {
							migrationFinding = migrationFinding === finding.id ? null : finding.id;
							migrationDisposition = 'revise';
							migrationRationale = '';
						}}
						selected={migrationFinding === finding.id}
					>
						{#snippet leading()}<GitBranch size={15} strokeWidth={1.8} />{/snippet}
						{#snippet trailing()}<ChevronRight
								size={14}
								strokeWidth={1.8}
								aria-hidden="true"
							/>{/snippet}
						{#if migrationFinding === finding.id}
							<div class="decision">
								<p class="consequence">{finding.consequence}</p>
								<Segmented
									label="What should happen to this artifact"
									options={DISPOSITIONS}
									value={migrationDisposition}
									onchange={(value) => (migrationDisposition = value)}
								/>
								<textarea
									bind:value={migrationRationale}
									aria-label="Your reason"
									placeholder="Why this is right for this artifact"></textarea>
								<div class="decision-bar">
									<button
										class="btn primary small"
										type="button"
										disabled={saving || !migrationRationale.trim()}
										onclick={() => decideMigration(finding)}
										>{saving ? 'Recording…' : 'Record decision'}</button
									>
									<button
										class="btn small ghost"
										type="button"
										onclick={() => (migrationFinding = null)}>Cancel</button
									>
								</div>
							</div>
						{/if}
					</Item>
				{/each}
			</Section>
		{/if}

		<IdentityEditor {companyId} {view} onSaved={() => source.refresh()} />

		{#if view.current_release}
			<Section
				title="Current version"
				info="New work uses this version. A later version never changes finished work."
			>
				<Row label="Effective">
					<span title={formatMoment(view.current_release.effective_from)}
						>{when(view.current_release.effective_from)}</span
					>
				</Row>
				<Row label="Promoted by">{view.current_release.promoted_by}</Row>
				<Row
					label="Bound artifacts"
					info="Artifacts that were produced against this exact identity."
				>
					{view.constitution_artifact_bindings.length}{#if staleBindings}<span class="warn"
							>{' · '}{staleBindings} need review</span
						>{/if}
				</Row>
				<Row label="What changed" stack>
					<span class="change">{view.current_release.change_account}</span>
				</Row>
			</Section>
		{/if}

		{#if currentEvidence.length}
			<Section
				title="Released statements"
				count={currentEvidence.length}
				info="Every statement in the current version, with its kind and source."
			>
				{#each PILLARS as [key, label] (key)}
					{@const items = currentEvidence.filter((item) => item.pillar === key)}
					<Fold {label} count={items.length}>
						{#if items.length}
							<ul class="statements">
								{#each items as item (item.id)}
									<li class:negative={item.polarity === 'negative'}>
										<span
											class="kind"
											title={item.source === 'owner_identity_editor'
												? 'Your direction'
												: `${item.author_id} · ${item.evidence_locator}`}
											>{sentence(item.statement_kind)}{item.polarity === 'negative'
												? ' · avoid'
												: ''}</span
										>{item.statement}
									</li>
								{/each}
							</ul>
						{:else}<p class="fold-empty">Nothing released for {label.toLowerCase()}.</p>{/if}
					</Fold>
				{/each}
			</Section>
		{/if}

		{#if usage.length}
			<Section
				title="Reviews"
				count={usage.length}
				info="What independent reviewers found when work used the identity."
			>
				<Fold label="Show reviews">
					<ul class="statements">
						{#each usage.slice(-20).reverse() as review (review.id)}
							<li>
								<span class="kind">{review.pillar} · {words(review.verdict)}</span
								>{review.finding || 'No finding recorded.'}
							</li>
						{/each}
					</ul>
				</Fold>
			</Section>
		{/if}

		{#if view.releases.length > 1}
			<Section title="Version history" count={view.releases.length}>
				{#each view.releases as release (release.id)}
					<Fold
						label={release.change_account}
						hint={`${release.id === view.current_release?.id ? 'Current · ' : ''}${when(release.effective_from)}`}
					>
						<ul class="statements">
							{#each view.release_evidence.filter((link) => link.release_id === release.id) as link (link.evidence_id)}
								{@const evidence = view.evidence.find((item) => item.id === link.evidence_id)}
								{#if evidence}<li>
										<span class="kind">{sentence(evidence.pillar)}</span>{evidence.statement}
									</li>{/if}
							{/each}
						</ul>
					</Fold>
				{/each}
			</Section>
		{/if}
	{:else if source.failure}
		<FailureNotice
			error={source.failure}
			subject="identity"
			variant="block"
			onretry={source.refresh}
		/>
	{:else}
		<Skeleton label="Reading company identity…" variant="page" count={4} />
	{/if}
</Page>

<style>
	.decision {
		display: grid;
		gap: 12px;
		padding-top: 4px;
	}
	.decision textarea {
		width: 100%;
		min-height: 72px;
	}
	.decision-bar {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.consequence {
		margin: 0;
		color: var(--text-secondary);
		line-height: 1.55;
	}
	.statements {
		display: grid;
		gap: 10px;
		margin: 0;
		padding: 0 16px 14px 40px;
		list-style: none;
		color: var(--ink);
		font-size: var(--t-body);
		line-height: 1.55;
	}
	.decision .statements {
		padding: 0;
	}
	.statements li {
		display: grid;
		gap: 2px;
		overflow-wrap: anywhere;
		white-space: pre-wrap;
	}
	.statements li.negative {
		color: var(--text-secondary);
	}
	.kind {
		color: var(--text-tertiary);
		font-size: var(--t-label);
		white-space: normal;
	}
	.fold-empty {
		margin: 0;
		padding: 0 16px 14px 40px;
		color: var(--text-tertiary);
	}
	.change {
		color: var(--text-secondary);
		line-height: 1.55;
		white-space: pre-wrap;
	}
	.warn {
		color: var(--intent-authority);
	}
</style>
