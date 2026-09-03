<script lang="ts">
	import type { Snippet } from 'svelte';
	import Markdown from '../primitives/Markdown.svelte';
	import MatrixGlyph, { GLYPHS } from '../primitives/MatrixGlyph.svelte';
	import type {
		WorkDetailArtifact,
		WorkDetailPlatform,
		WorkDetailView,
		WorkSurfaceStatus
	} from '../product/contracts';

	const OUTCOME_CLAMP_CHARS = 460;
	let {
		view = null,
		loaded,
		error = '',
		companyName,
		platform,
		liveActivity,
		headerActions
	}: {
		view?: WorkDetailView | null;
		loaded: boolean;
		error?: string;
		companyName: string;
		platform: WorkDetailPlatform;
		liveActivity?: Snippet;
		headerActions?: Snippet;
	} = $props();

	let outcomeExpanded = $state(false);
	let previousWorkId = $state('');
	$effect(() => {
		if (!view || view.id === previousWorkId) return;
		previousWorkId = view.id;
		outcomeExpanded = false;
	});

	const passedGates = $derived(view?.gates.filter((gate) => gate.passed).length ?? 0);
	const unverifiedCompletion = $derived(
		view?.status === 'completed' && !view.artifacts.length && passedGates === 0
	);
	const outcomeIsLong = $derived((view?.executionContract.length ?? 0) > OUTCOME_CLAMP_CHARS);

	function statusLabel(status: WorkSurfaceStatus): string {
		return {
			proposed: 'Not started',
			active: 'In progress',
			blocked: 'Waiting on a blocker',
			completed: 'Complete',
			abandoned: 'Stopped'
		}[status];
	}

	function artifactState(artifact: WorkDetailArtifact): string {
		return artifact.state === 'available' ? 'Available' : artifact.state.replaceAll('_', ' ');
	}

	function displayDate(value: string | null | undefined): string {
		if (!value) return 'Not recorded';
		const date = new Date(value);
		if (Number.isNaN(date.getTime())) return value;
		return new Intl.DateTimeFormat(undefined, {
			day: 'numeric',
			month: 'short',
			year: 'numeric',
			hour: 'numeric',
			minute: '2-digit'
		}).format(date);
	}
</script>

<svelte:head><title>{view?.title ?? 'Work detail'} — {companyName}</title></svelte:head>

<article class="work-detail-screen cockpit-pane">
	{#if error}<div class="cockpit-error">{error}</div>{/if}
	{#if view}
		<header class="work-detail-head">
			<div class="work-detail-heading">
				<a class="work-back" href={platform.backHref} aria-label="Back to Work">
					<span aria-hidden="true">←</span> Work
				</a>
				<div class="work-breadcrumb">
					<span>{view.goalTitle || 'Company work'}</span><i aria-hidden="true">/</i><b>Details</b>
				</div>
				<h1>{view.title}</h1>
			</div>
			<div class="work-detail-head-actions">
				{#if headerActions}{@render headerActions()}{/if}
				<div
					class="work-detail-status status-{view.status}"
					class:unverified={unverifiedCompletion}
				>
					<MatrixGlyph
						rows={view.status === 'completed'
							? GLYPHS.check
							: view.status === 'blocked'
								? GLYPHS.ring
								: GLYPHS.dots}
						size={9}
					/>
					<span
						>{unverifiedCompletion
							? 'Completion recorded · evidence unavailable'
							: statusLabel(view.status)}</span
					>
				</div>
			</div>
		</header>

		{#if liveActivity}{@render liveActivity()}{/if}

		{#if view.recovery}
			<section class="recovery-brief" aria-labelledby="recovery-heading">
				<div class="recovery-mark" aria-hidden="true">
					<MatrixGlyph rows={GLYPHS.ring} size={9} />
				</div>
				<div class="recovery-copy">
					<h2 id="recovery-heading">A candidate is preserved. Its outcome is still unknown.</h2>
					<p>
						The cognitive process ended before it reported a trustworthy result. Restless has not
						called this success or failure.
						{view.accountableLeadName} owns the next judgement: inspect the same candidate, then revise,
						resume, reassign, or abandon it.
					</p>
					{#if view.recovery.preservedCandidate}
						<div class="preserved-candidate">
							<div>
								<h3>Preserved candidate</h3>
								<strong
									>{view.recovery.preservedCandidate.label ||
										view.recovery.preservedCandidate.kind}</strong
								>
								<code>{view.recovery.preservedCandidate.uri}</code>
							</div>
							{#if view.recovery.preservedCandidate.openHref}
								<a
									class="preserved-link"
									href={view.recovery.preservedCandidate.openHref}
									target="_blank"
									rel="noreferrer">Open target ↗</a
								>
							{/if}
						</div>
					{/if}
					<details class="recovery-evidence">
						<summary>Show recovery evidence</summary>
						<div>
							<p>{view.recovery.summary}</p>
							{#each view.recovery.artifacts as artifact (artifact.id)}
								<div class="recovery-artifact">
									<strong>{artifact.label}</strong><code>{artifact.uri}</code>
								</div>
							{/each}
						</div>
					</details>
				</div>
			</section>
		{/if}

		<div class="work-detail-scroll">
			<div class="work-detail-layout">
				<main class="work-detail-main">
					{#if view.readerSummary}
						<section class="work-reader-summary" aria-label={view.readerSummaryLabel}>
							<span class="detail-label">{view.readerSummaryLabel}</span>
							<Markdown text={view.readerSummary} />
						</section>
					{/if}
					<details class="work-technical-details">
						<summary>Technical execution details</summary>
						<div class="work-technical-body">
							<section class="work-detail-section outcome-contract">
								<span class="detail-label">Exact execution contract</span>
								<div class="outcome-body" class:clamped={outcomeIsLong && !outcomeExpanded}>
									<Markdown text={view.executionContract} />
								</div>
								{#if outcomeIsLong}
									<button
										class="outcome-toggle"
										type="button"
										onclick={() => (outcomeExpanded = !outcomeExpanded)}
										aria-expanded={outcomeExpanded}
									>
										{outcomeExpanded ? 'Show less' : 'Read the full contract'}
									</button>
								{/if}
							</section>
							<section class="work-detail-section">
								<div class="detail-section-head">
									<span class="detail-label">Latest run</span>
									{#if view.attempt}<small
											>Attempt {view.attempt.attemptNo} · revision {view.attempt.revision}</small
										>{/if}
								</div>
								{#if view.attempt}
									<strong class="attempt-state state-{view.attempt.state}"
										>{view.attempt.state.replaceAll('_', ' ')}</strong
									>
									<p>{view.attempt.summary || 'This run has not recorded a summary yet.'}</p>
									<div class="attempt-meta">
										<span>Started {displayDate(view.attempt.startedAt)}</span><span
											>{view.attempt.model || 'Model not recorded'}</span
										>
									</div>
								{:else}<p class="detail-empty">No run has started yet.</p>{/if}
							</section>
							<section class="contribution-trace">
								<h2>
									{view.staffResponsibilityName ? 'Contribution record' : 'Accountability record'}
								</h2>
								{#if view.staffResponsibilityName}
									<p>
										{view.staffResponsibilityName} owns this bounded contribution for {view.accountableLeadName}.
										Its run and outputs are evidence of the contribution, not proof that the whole
										outcome was accepted.
									</p>
								{:else}
									<p>
										{view.accountableLeadName} owns the final judgement for this outcome. Accepted, revised
										or stopped contributions retain their own execution and observed-output records.
									</p>
								{/if}
								{#if view.status === 'abandoned'}
									<p class="contribution-status">
										This Work was stopped and is not presented as accepted output.
									</p>
								{:else if view.revisions.length}
									<p class="contribution-status">
										A source-observed revision route is shown in the Work graph below.
									</p>
								{/if}
							</section>
						</div>
					</details>
				</main>

				<div class="work-detail-rail">
					<aside class="work-detail-aside" aria-label="Work facts">
						<section>
							<span class="detail-label">Accountable lead</span><strong
								>{view.accountableLeadName}</strong
							>{#if view.staffResponsibilityName}<small
									>Staff responsibility: {view.staffResponsibilityName}</small
								>{/if}
						</section>
						<section>
							<span class="detail-label">Evidence</span><strong
								>{view.artifacts.length} linked output{view.artifacts.length === 1
									? ''
									: 's'}</strong
							><small
								>{view.gates.length
									? `${passedGates}/${view.gates.length} automated checks passed`
									: 'No automated checks recorded'}</small
							>
						</section>
						<section>
							<span class="detail-label">Updated</span><strong>{displayDate(view.updatedAt)}</strong
							>
						</section>
						{#if view.workspace}<section>
								<span class="detail-label">Workspace</span><strong>{view.workspace}</strong
								>{#if view.integrationBranch}<small>{view.integrationBranch}</small>{/if}
							</section>{/if}
					</aside>

					<section class="work-evidence-section">
						<header>
							<div>
								<span class="detail-label">Evidence</span>
								<h2>What supports this outcome</h2>
							</div>
							<span class="evidence-score"
								>{view.artifacts.length} linked output{view.artifacts.length === 1 ? '' : 's'} · {view
									.gates.length
									? `${passedGates}/${view.gates.length} checks passed`
									: 'no automated checks'}</span
							>
						</header>
						{#if view.expectedArtifact}<p class="expected-artifact">
								<span>Expected output</span>{view.expectedArtifact}
							</p>{/if}
						<div class="evidence-columns">
							<div class="artifact-list">
								<span class="detail-sublabel">Linked outputs</span>
								{#each view.artifacts as artifact (artifact.id)}
									<div class="detail-artifact">
										<MatrixGlyph rows={GLYPHS.work} size={7} /><span
											><strong>{artifact.label || artifact.kind}</strong><small
												>{artifact.note || 'Linked evidence for this Work'}</small
											></span
										>
										<div class="artifact-actions">
											<em class:available={artifact.state === 'available'}
												>{artifactState(artifact)}</em
											>{#if artifact.openHref}<a
													href={artifact.openHref}
													target="_blank"
													rel="noreferrer">Open ↗</a
												>{/if}
										</div>
									</div>
								{:else}<p class="detail-empty">No linked outputs are recorded.</p>{/each}
							</div>
							<div class="gate-list">
								<span class="detail-sublabel">Automated checks</span>
								{#each view.gates as gate (gate.id)}<div
										class:passed={gate.passed}
										class="detail-gate"
									>
										<MatrixGlyph rows={gate.passed ? GLYPHS.check : GLYPHS.ring} size={7} /><span
											><strong>{gate.name}</strong><small
												>{gate.passed ? 'Passed' : 'Not passed'}</small
											></span
										>
									</div>{:else}<p class="detail-empty">No automated checks are recorded.</p>{/each}
							</div>
						</div>
					</section>

					{#if view.prerequisites.length || view.dependents.length || view.revisions.length}
						<section class="work-relations-section">
							<header>
								<span class="detail-label">Work graph</span>
								<h2>Handovers and review loops</h2>
							</header>
							<div class="relation-groups">
								{#if view.prerequisites.length}<div>
										<span class="detail-sublabel">Requires</span
										>{#each view.prerequisites as item (item.id)}<a href={item.href}
												>{item.title}<small>R{item.revision} · {item.status}</small></a
											>{/each}
									</div>{/if}
								{#if view.dependents.length}<div>
										<span class="detail-sublabel">Hands over to</span
										>{#each view.dependents as item (item.id)}<a href={item.href}
												>{item.title}<small>R{item.revision} · {item.status}</small></a
											>{/each}
									</div>{/if}
								{#if view.revisions.length}<div>
										<span class="detail-sublabel">Revision loop</span
										>{#each view.revisions as item (item.id)}<a class="revision" href={item.href}
												>{item.title}<small>R{item.revision} · {item.status}</small></a
											>{/each}
									</div>{/if}
							</div>
						</section>
					{/if}
				</div>
			</div>
		</div>
	{:else if loaded && !error}
		<div class="work-detail-missing">
			<MatrixGlyph rows={GLYPHS.ring} size={14} />
			<h1>Work not found</h1>
			<p>This Work is no longer present in the current company projection.</p>
			<a class="work-back" href={platform.backHref}
				><span aria-hidden="true">←</span> Return to Work</a
			>
		</div>
	{:else if !error}<div class="work-detail-loading">Loading Work…</div>{/if}
</article>

<style>
	.work-detail-head-actions {
		display: flex;
		align-items: center;
		gap: 10px;
	}
	@media (max-width: 760px) {
		.work-detail-head-actions {
			width: 100%;
			justify-content: space-between;
		}
	}
</style>
