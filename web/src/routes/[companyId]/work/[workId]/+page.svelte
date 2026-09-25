<script lang="ts">
	import Skeleton from '$lib/primitives/Skeleton.svelte';
	import { runStateLabel, workStatusLabel } from '$lib/work/status';
	import { resizePane } from '$lib/actions/resize-pane';
	import { page } from '$app/state';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import ConversationTurnDock from '$lib/primitives/ConversationTurnDock.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import type { CollaborationArtifact, CollaborationWork } from '$lib/model/collaboration';
	import {
		attentionQuery,
		cockpitQuery,
		collaborationBootstrapQuery,
		companyPrincipalQuery,
		workActivityStream
	} from '$lib/model/queries.svelte';
	import type { ArtifactRefRow, WorkGateRow, WorkRow } from '$lib/model/generated/orgintel';

	/* The authoring contract deliberately separates a human opening from the
	 * exact actor contract with one blank line. Respect that declared boundary;
	 * never infer a summary from arbitrary prose or rewrite either side. */
	const OUTCOME_CLAMP_CHARS = 460;
	let outcomeExpanded = $state(false);

	const companyId = $derived(page.params.companyId ?? 'aris');
	const workId = $derived(page.params.workId ?? '');
	const principalProjection = $derived(companyPrincipalQuery(companyId));
	const ownerAccess = $derived(principalProjection.view?.membership_role === 'owner');
	const attentionProjection = $derived(attentionQuery(companyId, () => ownerAccess));
	const cockpitProjection = $derived(cockpitQuery(companyId, () => ownerAccess));
	const collaborationProjection = $derived(
		collaborationBootstrapQuery(companyId, () => principalProjection.view)
	);
	const attention = $derived(attentionProjection.view);
	const cockpit = $derived(cockpitProjection.view);
	const collaboration = $derived(collaborationProjection.view);
	const loaded = $derived(
		principalProjection.status !== 'unknown' &&
			(ownerAccess
				? attentionProjection.status !== 'unknown' || cockpitProjection.status !== 'unknown'
				: collaborationProjection.status !== 'unknown')
	);
	const error = $derived(
		principalProjection.failure?.message ??
			(ownerAccess
				? (attentionProjection.failure?.message ?? cockpitProjection.failure?.message)
				: collaborationProjection.failure?.message) ??
			''
	);
	type WorkItem = WorkRow | CollaborationWork;
	type Artifact = ArtifactRefRow | CollaborationArtifact;

	const graph = $derived(
		ownerAccess ? (attention?.workGraph ?? null) : (collaboration?.work_graph ?? null)
	);
	const work = $derived(graph?.work.find((item) => item.id === workId) ?? null);
	const goals = $derived(ownerAccess ? (cockpit?.goals ?? []) : (collaboration?.goals ?? []));
	const people = $derived(ownerAccess ? (cockpit?.people ?? []) : (collaboration?.people ?? []));
	const teams = $derived(ownerAccess ? (cockpit?.teams ?? []) : (collaboration?.teams ?? []));
	const companyName = $derived(
		ownerAccess ? (cockpit?.company.name ?? companyId) : (collaboration?.company.name ?? companyId)
	);
	const goal = $derived(goals.find((item) => item.id === work?.goal_id) ?? null);
	const attempts = $derived(
		(graph?.attempts ?? [])
			.filter((attempt) => attempt.work_id === workId)
			.toSorted((a, b) => a.attempt_no - b.attempt_no)
	);
	const latestAttempt = $derived(attempts.at(-1) ?? null);
	const activity = $derived(
		ownerAccess && work && latestAttempt?.state === 'running'
			? workActivityStream(companyId, work.owner_id, workId)
			: null
	);
	$effect(() => activity?.attach());
	const workTurn = $derived.by(() => {
		if (!activity?.live || !latestAttempt) return null;
		return {
			triggerMessageId: latestAttempt.attempt_no,
			since: activity.live.startedAt ?? activity.live.updatedAt,
			live: activity.live,
			transport: activity.transport
		};
	});
	const artifacts = $derived(
		(graph?.artifacts ?? [])
			.filter((artifact) => artifact.work_id === workId)
			.toSorted((a, b) => Date.parse(b.created_at) - Date.parse(a.created_at))
	);
	const workspace = $derived(
		work && isOwnerWork(work)
			? {
					location: work.worktree || work.repo,
					integrationBranch: work.integration_branch
				}
			: null
	);
	const outcomeParts = $derived(splitOutcome(work?.outcome ?? ''));
	const readerSummary = $derived((work?.resolution || outcomeParts.opening).trim());
	const readerSummaryLabel = $derived(
		work?.resolution ? 'What happened' : 'What this Work delivers'
	);
	const executionContract = $derived(outcomeParts.contract || work?.outcome || '');
	const outcomeIsLong = $derived(executionContract.length > OUTCOME_CLAMP_CHARS);
	$effect(() => {
		/* A different Work is a different contract: never carry one open state
		 * onto the next page. */
		void workId;
		outcomeExpanded = false;
	});
	const gates = $derived(
		graph && 'gates' in graph ? graph.gates.filter((gate) => gate.work_id === workId) : []
	);
	const passedGates = $derived(gates.filter((gate) => gatePassed(gate)).length);
	const unverifiedCompletion = $derived(
		work?.status === 'completed' && artifacts.length === 0 && passedGates === 0
	);
	const prerequisites = $derived(
		(graph?.edges ?? [])
			.filter((edge) => edge.to_work_id === workId && edge.kind === 'requires')
			.flatMap((edge) => graph?.work.filter((item) => item.id === edge.from_work_id) ?? [])
	);
	const dependents = $derived(
		(graph?.edges ?? [])
			.filter((edge) => edge.from_work_id === workId && edge.kind === 'requires')
			.flatMap((edge) => graph?.work.filter((item) => item.id === edge.to_work_id) ?? [])
	);
	const revisions = $derived(
		(graph?.edges ?? [])
			.filter(
				(edge) =>
					edge.kind === 'revises' && (edge.from_work_id === workId || edge.to_work_id === workId)
			)
			.flatMap((edge) => {
				const relatedId = edge.from_work_id === workId ? edge.to_work_id : edge.from_work_id;
				return graph?.work.filter((item) => item.id === relatedId) ?? [];
			})
	);
	const workOwner = $derived(people.find((person) => person.actor_id === work?.owner_id));
	const accountableLeadId = $derived(
		workOwner?.team_id
			? (teams.find((team) => team.id === workOwner.team_id)?.lead_actor_id ?? work?.owner_id)
			: work?.owner_id
	);
	const accountableLead = $derived(
		people.find((person) => person.actor_id === accountableLeadId) ?? null
	);
	const unknownRecovery = $derived(
		ownerAccess &&
			work?.status === 'blocked' &&
			latestAttempt?.state === 'failed' &&
			latestAttempt.summary.includes('productive outcome unknown')
	);
	const recoveryArtifacts = $derived(
		unknownRecovery && latestAttempt
			? artifacts.filter((artifact) => artifact.attempt_id === latestAttempt.id)
			: []
	);
	const preservedCandidate = $derived(
		recoveryArtifacts.find((artifact) => artifact.kind !== 'git_worktree_observation') ??
			recoveryArtifacts.at(0) ??
			null
	);
	const workIsLeadOwned = $derived(!!work?.owner_id && work.owner_id === accountableLeadId);

	function gatePassed(gate: WorkGateRow): boolean {
		if (!latestAttempt || !graph || !('gate_runs' in graph)) return false;
		return (
			graph.gate_runs.some(
				(run) => run.gate_id === gate.id && run.attempt_id === latestAttempt?.id && run.passed
			) ?? false
		);
	}

	function isOwnerWork(item: WorkItem): item is WorkRow {
		return 'owner_review_required' in item;
	}

	function splitOutcome(value: string): { opening: string; contract: string } {
		const boundary = value.indexOf('\n\n');
		if (boundary < 0) return { opening: '', contract: value };
		return {
			opening: value.slice(0, boundary).trim(),
			contract: value.slice(boundary + 2).trim()
		};
	}

	function ownerName(actorId: string): string {
		return (
			people.find((person) => person.actor_id === actorId)?.display ??
			actorId.replaceAll('-', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase())
		);
	}

	function backHref(): string {
		const selectedGoal = page.url.searchParams.get('goal') || work?.goal_id || '';
		const lens = page.url.searchParams.get('lens') === 'board' ? 'board' : 'map';
		const query = new URLSearchParams({ lens });
		if (selectedGoal) query.set('goal', selectedGoal);
		return `/${encodeURIComponent(companyId)}/work?${query}`;
	}

	function relatedHref(item: WorkItem): string {
		const query = new URLSearchParams({
			goal: item.goal_id ?? work?.goal_id ?? '',
			lens: page.url.searchParams.get('lens') === 'board' ? 'board' : 'map'
		});
		return `/${encodeURIComponent(companyId)}/work/${encodeURIComponent(item.id)}?${query}`;
	}

	function artifactState(artifact: Artifact): string {
		return artifact.state === 'available' ? 'Available' : artifact.state.replaceAll('_', ' ');
	}

	/* Older Runtime-created artifacts used the whole expected-output contract as
	 * their label. The equality and source kind identify that exact mechanical
	 * path; never shorten a genuinely authored artifact label. */
	function isLegacyAutomaticArtifact(artifact: Artifact): boolean {
		return (
			!!work &&
			'created_by' in artifact &&
			artifact.label === work.expected_artifact &&
			artifact.created_by === work.owner_id &&
			['file', 'repository_tree'].includes(artifact.kind)
		);
	}

	function artifactLabel(artifact: Artifact): string {
		return isLegacyAutomaticArtifact(artifact)
			? `Output from: ${work?.title ?? 'this work'}`
			: artifact.label || artifact.kind;
	}

	function artifactNote(artifact: Artifact): string {
		if (!isLegacyAutomaticArtifact(artifact)) {
			return artifact.note || 'Linked evidence for this Work';
		}
		return artifact.kind === 'file'
			? 'The exact file produced by this work and observed in the company runtime.'
			: 'The saved result produced by this work; Restless observed it with no uncommitted changes.';
	}

	function artifactLocator(artifact: Artifact): string | null {
		return 'href' in artifact ? artifact.href : artifact.uri;
	}

	function attemptModel(): string | null {
		if (!latestAttempt || !('model' in latestAttempt)) return null;
		return typeof latestAttempt.model === 'string' ? latestAttempt.model : null;
	}

	function canOpenOutsideCompany(uri: string): boolean {
		try {
			const parsed = new URL(uri);
			return (
				parsed.protocol === 'https:' ||
				(parsed.protocol === 'http:' &&
					!['localhost', '127.0.0.1', '::1'].includes(parsed.hostname))
			);
		} catch {
			return false;
		}
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

<svelte:head><title>{work?.title ?? 'Work detail'} — {companyName}</title></svelte:head>

<article class="work-detail-screen cockpit-pane">
	{#if error}<div class="cockpit-error">{error}</div>{/if}

	{#if work}
		<header class="work-detail-head">
			<nav class="work-crumbs" aria-label="Breadcrumb">
				<a href={backHref()}>Work</a>
				<i aria-hidden="true">/</i>
				<span title={goal?.body || undefined}>{goal?.title ?? 'Unassigned'}</span>
			</nav>
			<div class="work-title-row">
				<h1 style:view-transition-name={`work-title-${work.id}`}>{work.title}</h1>
				<div
					class="work-detail-status status-{work.status}"
					class:unverified={unverifiedCompletion}
					title={unverifiedCompletion
						? 'Completion was recorded, but no output or passing check supports it yet.'
						: undefined}
				>
					<i aria-hidden="true"></i>
					<span>{unverifiedCompletion ? 'Done · unverified' : workStatusLabel(work.status)}</span>
				</div>
			</div>
		</header>

		{#if workTurn}
			<section class="work-live-activity" aria-label="Live Work activity">
				<ConversationTurnDock
					participantName={accountableLead?.display ?? ownerName(work.owner_id)}
					turn={workTurn}
				/>
			</section>
		{/if}

		{#if unknownRecovery}
			<section class="recovery-brief" aria-labelledby="recovery-heading">
				<div class="recovery-mark" aria-hidden="true">
					<MatrixGlyph rows={GLYPHS.ring} size={9} />
				</div>
				<div class="recovery-copy">
					<h2 id="recovery-heading">A candidate is preserved. Its outcome is still unknown.</h2>
					<p>
						The cognitive process ended before it reported a trustworthy result. Restless has not
						called this success or failure. {accountableLead?.display ??
							ownerName(accountableLeadId ?? work.owner_id)}
						owns the next judgement: inspect the same candidate, then revise, resume, reassign, or abandon
						it.
					</p>
					{#if preservedCandidate}
						<div class="preserved-candidate">
							<div>
								<h3>Preserved candidate</h3>
								<strong>{preservedCandidate.label || preservedCandidate.kind}</strong>
								<code title="Exact Runtime or external target preserved with this Attempt"
									>{artifactLocator(preservedCandidate)}</code
								>
							</div>
							{#if artifactLocator(preservedCandidate) && canOpenOutsideCompany(artifactLocator(preservedCandidate)!)}
								<a
									class="preserved-link"
									href={artifactLocator(preservedCandidate)!}
									target="_blank"
									rel="noreferrer"
									title="Open this exact preserved target without deciding the Work"
									>Open target ↗</a
								>
							{/if}
						</div>
					{/if}
					<details class="recovery-evidence">
						<summary
							title="Process observations and linked outputs; these support review but do not decide quality"
						>
							Show recovery evidence
						</summary>
						<div>
							<p>{latestAttempt?.summary}</p>
							{#each recoveryArtifacts as artifact (artifact.id)}
								<div class="recovery-artifact">
									<strong>{artifactLabel(artifact)}</strong>
									<code>{artifactLocator(artifact)}</code>
								</div>
							{/each}
						</div>
					</details>
				</div>
			</section>
		{/if}

		<div class="work-detail-scroll">
			<div
				class="work-detail-layout"
				use:resizePane={{
					key: `${companyId}:work-detail`,
					label: 'Resize Work facts pane',
					target: '.work-detail-rail',
					variable: '--work-facts-w',
					side: 'end',
					min: 220,
					minOther: 280,
					defaultSize: 300
				}}
			>
				<main class="work-detail-main">
					<section class="work-description" aria-label={readerSummaryLabel}>
						<div
							class="outcome-body"
							class:clamped={!readerSummary && outcomeIsLong && !outcomeExpanded}
						>
							<Markdown text={readerSummary || executionContract} />
						</div>
						{#if !readerSummary && outcomeIsLong}
							<button
								type="button"
								class="outcome-toggle"
								onclick={() => (outcomeExpanded = !outcomeExpanded)}
								aria-expanded={outcomeExpanded}
							>
								{outcomeExpanded ? 'Show less' : 'Read the full brief'}
							</button>
						{/if}
					</section>

					<section class="work-evidence-section" aria-labelledby="work-evidence-heading">
						<header>
							<h2 id="work-evidence-heading">Evidence</h2>
							<span class="evidence-score"
								>{artifacts.length} output{artifacts.length === 1 ? '' : 's'} · {ownerAccess
									? gates.length
										? `${passedGates}/${gates.length} checks passed`
										: 'no checks'
									: 'checks owner-visible'}</span
							>
						</header>
						{#if work.expected_artifact}
							<p class="expected-artifact" title="The output the accountable lead asked for">
								<span>Expected</span>{work.expected_artifact}
							</p>
						{/if}
						{#if artifacts.length || gates.length || !ownerAccess}
							<div class="evidence-columns">
								{#if artifacts.length}
									<div class="artifact-list">
										<h3 class="detail-sublabel">Outputs</h3>
										{#each artifacts as artifact (artifact.id)}
											<div class="detail-artifact">
												<MatrixGlyph rows={GLYPHS.work} size={7} />
												<span>
													<strong>{artifactLabel(artifact)}</strong>
													<small>{artifactNote(artifact)}</small>
												</span>
												<div class="artifact-actions">
													<em class:available={artifact.state === 'available'}
														>{artifactState(artifact)}</em
													>
													{#if artifactLocator(artifact) && canOpenOutsideCompany(artifactLocator(artifact)!)}
														<a href={artifactLocator(artifact)!} target="_blank" rel="noreferrer"
															>Open ↗</a
														>
													{/if}
												</div>
											</div>
										{/each}
									</div>
								{/if}
								{#if ownerAccess && gates.length}
									<div class="gate-list">
										<h3 class="detail-sublabel">Automated checks</h3>
										{#each gates as gate (gate.id)}
											<div class:passed={gatePassed(gate)} class="detail-gate">
												<MatrixGlyph
													rows={gatePassed(gate) ? GLYPHS.check : GLYPHS.ring}
													size={7}
												/>
												<span
													><strong>{gate.name}</strong><small
														>{gatePassed(gate) ? 'Passed' : 'Not passed'}</small
													></span
												>
											</div>
										{/each}
									</div>
								{:else if !ownerAccess}
									<p class="detail-empty">
										Check definitions and raw run output stay in the owner surface.
									</p>
								{/if}
							</div>
						{:else}
							<p class="detail-empty">
								Nothing is linked yet. Outputs and automated checks appear here as the Work runs.
							</p>
						{/if}
					</section>

					<details class="work-technical-details">
						<summary>Technical execution details</summary>
						<div class="work-technical-body">
							{#if readerSummary}<section class="work-detail-section outcome-contract">
									<span
										class="detail-label"
										title="The exact instructions the accountable actor executes. Shown verbatim; Restless never rewrites them."
									>
										Exact execution contract
									</span>
									<div class="outcome-body" class:clamped={outcomeIsLong && !outcomeExpanded}>
										<Markdown text={executionContract} />
									</div>
									{#if outcomeIsLong}
										<button
											type="button"
											class="outcome-toggle"
											onclick={() => (outcomeExpanded = !outcomeExpanded)}
											aria-expanded={outcomeExpanded}
										>
											{outcomeExpanded ? 'Show less' : 'Read the full contract'}
										</button>
									{/if}
								</section>{/if}

							<section class="work-detail-section">
								<div class="detail-section-head">
									<span class="detail-label">Latest run</span>
									{#if latestAttempt}
										<small
											>Attempt {latestAttempt.attempt_no} · revision {latestAttempt.revision}</small
										>
									{/if}
								</div>
								{#if latestAttempt}
									<strong class="attempt-state state-{latestAttempt.state}"
										>{latestAttempt.state.replaceAll('_', ' ')}</strong
									>
									<p>{latestAttempt.summary || 'This run has not recorded a summary yet.'}</p>
									<div class="attempt-meta">
										<span>Started {displayDate(latestAttempt.started_at)}</span>
										{#if ownerAccess}<span>{attemptModel() || 'Model not recorded'}</span>{/if}
									</div>
								{:else}
									<p class="detail-empty">No run has started yet.</p>
								{/if}
							</section>

							<section class="contribution-trace">
								<h2>{workIsLeadOwned ? 'Accountability record' : 'Contribution record'}</h2>
								{#if workIsLeadOwned}
									<p>
										{accountableLead?.display ?? ownerName(accountableLeadId ?? work.owner_id)} owns the
										final judgement for this outcome. Accepted, revised or stopped contributions retain
										their own execution and observed-output records.
									</p>
								{:else}
									<p>
										{ownerName(work.owner_id)} owns this part of the work for
										{accountableLead?.display ?? ownerName(accountableLeadId ?? work.owner_id)}.
										Finishing this part does not mean the whole outcome was accepted.
									</p>
								{/if}
								{#if work.status === 'abandoned'}
									<p class="contribution-status">
										This Work was stopped and is not presented as accepted output.
									</p>
								{:else if revisions.length}
									<p class="contribution-status">Revisions are shown in the Work graph below.</p>
								{/if}
							</section>
						</div>
					</details>
				</main>

				<div class="work-detail-rail">
					<dl class="work-properties" aria-label="Work properties">
						<dt title="The lead accountable for integrating the whole outcome">Lead</dt>
						<dd>{accountableLead?.display ?? ownerName(accountableLeadId ?? work.owner_id)}</dd>
						{#if !workIsLeadOwned}
							<dt title="The Staff member doing this work under the lead">Staff</dt>
							<dd>{ownerName(work.owner_id)}</dd>
						{/if}
						<dt>Goal</dt>
						<dd>
							{#if goal}<a
									href={`/${encodeURIComponent(companyId)}/work?goal=${encodeURIComponent(goal.id)}`}
									>{goal.title}</a
								>{:else}<span class="muted">Unassigned</span>{/if}
						</dd>
						{#if latestAttempt}
							<dt>Latest run</dt>
							<dd>
								Attempt {latestAttempt.attempt_no} · {runStateLabel(latestAttempt.state)}
							</dd>
						{/if}
						<dt>Updated</dt>
						<dd>{displayDate(work.updated_at)}</dd>
						{#if workspace?.location}
							<dt>Workspace</dt>
							<dd class="mono" title={workspace.integrationBranch || undefined}>
								{workspace.location}
							</dd>
						{/if}
					</dl>

					{#if prerequisites.length || dependents.length || revisions.length}
						<section class="work-relations-section" aria-label="Related Work">
							{#snippet relation(item: WorkItem, revision = false)}
								<a class:revision href={relatedHref(item)}>
									<i class="relation-dot status-{item.status}" aria-hidden="true"></i>
									<span>{item.title}</span>
									<small>{workStatusLabel(item.status)}</small>
								</a>
							{/snippet}
							{#if prerequisites.length}
								<div class="relation-group">
									<h3 class="detail-sublabel">Requires</h3>
									{#each prerequisites as item (item.id)}{@render relation(item)}{/each}
								</div>
							{/if}
							{#if dependents.length}
								<div class="relation-group">
									<h3 class="detail-sublabel">Hands over to</h3>
									{#each dependents as item (item.id)}{@render relation(item)}{/each}
								</div>
							{/if}
							{#if revisions.length}
								<div class="relation-group">
									<h3 class="detail-sublabel">Revision loop</h3>
									{#each revisions as item (item.id)}{@render relation(item, true)}{/each}
								</div>
							{/if}
						</section>
					{/if}
				</div>
			</div>
		</div>
	{:else if loaded && !error}
		<div class="work-detail-missing">
			<MatrixGlyph rows={GLYPHS.ring} size={14} />
			<h1>Work not found</h1>
			<p>This Work no longer exists in this company.</p>
			<a class="btn small" href={backHref()}>Back to Work</a>
		</div>
	{:else if !error}
		<div class="work-detail-loading"><Skeleton label="Loading Work" variant="page" count={5} /></div>
	{/if}
</article>
