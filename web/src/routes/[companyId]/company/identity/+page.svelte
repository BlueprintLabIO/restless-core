<script lang="ts">
	import { companyPrincipalQuery } from '$lib/model/queries.svelte';
	import { mayManageCompanySetup } from '$lib/model/company-access';
	import CompanyTitle from '$lib/primitives/CompanyTitle.svelte';
	import { formatRelative, formatMoment } from '$lib/ui/time';
	import { Page, Section, Row, Item, Notice, Segmented, Fold } from '$lib/ui/page';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { failureSentence } from '$lib/model/failure';
	import BrandDocument from '$lib/components/BrandDocument.svelte';
	import { confirmAction } from '$lib/ui/feedback/confirm.svelte';
	import { page } from '$app/state';
	import { identityQuery } from '$lib/model/queries.svelte';
	import GitBranch from '@lucide/svelte/icons/git-branch';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import {
		IDENTITY_PILLARS,
		decideIdentityMigration,
		releaseEvidence,
		saveCompanyIdentity,
		type IdentityDriftFindingRow,
		type IdentityMigrationDisposition,
		type IdentityReleaseRow
	} from '$lib/model/identity';
	import { holdQueries } from '$lib/model/hold-queries.svelte';

	const companyId = $derived(page.params.companyId ?? '');
	/* Owners and admins decide the company identity; everyone else reads it. */
	const principal = $derived(companyPrincipalQuery(companyId));
	const canManage = $derived(mayManageCompanySetup(principal.view));
	const source = $derived(identityQuery(companyId));
	const view = $derived(source.view);
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
	const DISPOSITIONS: { value: IdentityMigrationDisposition; label: string; title: string }[] = [
		{ value: 'retain', label: 'Keep', title: 'Keep the artifact as it is.' },
		{ value: 'revise', label: 'Revise', title: 'Have the artifact revised to the new identity.' },
		{ value: 'retire', label: 'Retire', title: 'Stop using the artifact.' }
	];

	let migrationFinding = $state<string | null>(null);
	let migrationDisposition = $state<IdentityMigrationDisposition>('revise');
	let migrationRationale = $state('');
	let saving = $state(false);
	let notice = $state('');
	let failure = $state('');

	function firstOf(...findings: string[]): string {
		return findings.find((finding) => finding.trim()) ?? '';
	}
	/* Restoring makes an earlier version current again as a new, attributed
	 * version; every statement keeps its original evidence and attribution. */
	async function restore(release: IdentityReleaseRow) {
		if (!view || saving) return;
		const label = formatMoment(release.effective_from);
		if (
			!(await confirmAction({
				title: 'Restore this version?',
				body: `New work will use the version from ${label}. The current version stays in the history.`,
				action: 'Restore'
			}))
		)
			return;
		saving = true;
		failure = '';
		notice = '';
		try {
			await saveCompanyIdentity(companyId, {
				expected_release: view.current_release?.id ?? null,
				statements: releaseEvidence(view, release.id).map((item) => ({
					id: item.id,
					pillar: item.pillar,
					text: item.statement,
					visual: null
				})),
				change_account: `Restored the version from ${label}`
			});
			notice = 'Restored. New work will use it; earlier work keeps its own version.';
			await source.refresh();
		} catch (cause) {
			failure = failureSentence(cause, 'The version was not restored.');
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
	holdQueries(() => [principal, source]);
</script>

<CompanyTitle title="Brand & voice" {companyId} />

<Page
	title="Brand & voice"
	info="How the company looks and sounds. Every site, deck and message the team makes starts from it. Exec proposes changes from your feedback; only you approve them."
>
	{#if view}
		{#if failure}<Notice tone="danger" title="That was not recorded" details={failure} />{/if}
		{#if notice}<Notice tone="success" title={notice} />{/if}

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
						{#if canManage && migrationFinding === finding.id}
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

		<BrandDocument {companyId} {view} {canManage} onSaved={() => source.refresh()} />

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
						{@const statements = releaseEvidence(view, release.id)}
						<ul class="statements">
							{#each IDENTITY_PILLARS as pillar (pillar.key)}
								{#each statements.filter((item) => item.pillar === pillar.key) as evidence (evidence.id)}<li
									>
										<span class="kind">{pillar.label}</span>{evidence.statement}
									</li>{/each}
							{/each}
						</ul>
						{#if canManage && release.id !== view.current_release?.id}
							<div class="restore">
								<button
									class="btn small"
									type="button"
									disabled={saving}
									title="Make this version current again, as a new version"
									onclick={() => restore(release)}>Restore this version</button
								>
							</div>
						{/if}
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
	.statements li {
		display: grid;
		gap: 2px;
		overflow-wrap: anywhere;
		white-space: pre-wrap;
	}
	.kind {
		color: var(--text-tertiary);
		font-size: var(--t-label);
		white-space: normal;
	}
	.restore {
		padding: 0 16px 14px 40px;
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
