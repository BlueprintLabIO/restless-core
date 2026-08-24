<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import Markdown from '$lib/primitives/Markdown.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import { getAttention, type AttentionView } from '$lib/model/attention';
	import { getCockpit, type CockpitView } from '$lib/model/cockpit';
	import type { ArtifactRefRow, WorkGateRow, WorkRow } from '$lib/model/generated/orgintel';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const workId = $derived(page.params.workId ?? '');
	let attention = $state<AttentionView | null>(null);
	let cockpit = $state<CockpitView | null>(null);
	let loaded = $state(false);
	let error = $state('');

	onMount(() => {
		void refresh();
		const timer = window.setInterval(() => void refresh(false), 8_000);
		return () => window.clearInterval(timer);
	});

	async function refresh(showError = true) {
		try {
			const [nextAttention, nextCockpit] = await Promise.all([
				getAttention(companyId),
				getCockpit(companyId)
			]);
			attention = nextAttention;
			cockpit = nextCockpit;
			error = '';
		} catch (cause) {
			if (showError) error = cause instanceof Error ? cause.message : 'Work is unavailable.';
		} finally {
			loaded = true;
		}
	}

	const graph = $derived(attention?.workGraph ?? null);
	const work = $derived(graph?.work.find((item) => item.id === workId) ?? null);
	const goal = $derived(cockpit?.goals.find((item) => item.id === work?.goal_id) ?? null);
	const attempts = $derived(
		(graph?.attempts ?? [])
			.filter((attempt) => attempt.work_id === workId)
			.toSorted((a, b) => a.attempt_no - b.attempt_no)
	);
	const latestAttempt = $derived(attempts.at(-1) ?? null);
	const artifacts = $derived(
		(graph?.artifacts ?? [])
			.filter((artifact) => artifact.work_id === workId)
			.toSorted((a, b) => Date.parse(b.created_at) - Date.parse(a.created_at))
	);
	const gates = $derived((graph?.gates ?? []).filter((gate) => gate.work_id === workId));
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
	const workOwner = $derived(cockpit?.people.find((person) => person.actor_id === work?.owner_id));
	const accountableLeadId = $derived(
		workOwner?.team_id
			? (cockpit?.teams.find((team) => team.id === workOwner.team_id)?.lead_actor_id ??
					work?.owner_id)
			: work?.owner_id
	);
	const accountableLead = $derived(
		cockpit?.people.find((person) => person.actor_id === accountableLeadId) ?? null
	);
	const unknownRecovery = $derived(
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
		if (!latestAttempt) return false;
		return (
			graph?.gate_runs.some(
				(run) => run.gate_id === gate.id && run.attempt_id === latestAttempt?.id && run.passed
			) ?? false
		);
	}

	function ownerName(actorId: string): string {
		return (
			cockpit?.people.find((person) => person.actor_id === actorId)?.display ??
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

	function relatedHref(item: WorkRow): string {
		const query = new URLSearchParams({
			goal: item.goal_id ?? work?.goal_id ?? '',
			lens: page.url.searchParams.get('lens') === 'board' ? 'board' : 'map'
		});
		return `/${encodeURIComponent(companyId)}/work/${encodeURIComponent(item.id)}?${query}`;
	}

	function artifactState(artifact: ArtifactRefRow): string {
		return artifact.state === 'available' ? 'Available' : artifact.state.replaceAll('_', ' ');
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

<svelte:head
	><title>{work?.title ?? 'Work detail'} — {cockpit?.company.name ?? companyId}</title></svelte:head
>

<article class="work-detail-screen cockpit-pane">
	{#if error}<div class="cockpit-error">{error}</div>{/if}

	{#if work}
		<header class="work-detail-head">
			<div class="work-detail-heading">
				<a class="work-back" href={backHref()} aria-label="Back to Work">
					<span aria-hidden="true">←</span> Work
				</a>
				<div class="work-breadcrumb">
					<span>{goal?.title ?? 'Company goal'}</span><i aria-hidden="true">/</i><b>Work detail</b>
				</div>
				<h1>{work.title}</h1>
			</div>
			<div class="work-detail-status status-{work.status}" class:unverified={unverifiedCompletion}>
				<MatrixGlyph
					rows={work.status === 'completed'
						? GLYPHS.check
						: work.status === 'blocked'
							? GLYPHS.ring
							: GLYPHS.dots}
					size={9}
				/>
				<span>
					{unverifiedCompletion
						? 'Completion recorded · evidence unavailable'
						: `${work.status} · revision ${work.revision}`}
				</span>
			</div>
		</header>

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
									>{preservedCandidate.uri}</code
								>
							</div>
							{#if canOpenOutsideCompany(preservedCandidate.uri)}
								<a
									class="preserved-link"
									href={preservedCandidate.uri}
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
									<strong>{artifact.label || artifact.kind}</strong>
									<code>{artifact.uri}</code>
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
					<section class="work-detail-section outcome-contract">
						<span class="detail-label">Outcome contract</span>
						<Markdown text={work.outcome} />
					</section>

					<section class="work-detail-section">
						<div class="detail-section-head">
							<span class="detail-label">Latest Attempt</span>
							{#if latestAttempt}
								<small>Attempt {latestAttempt.attempt_no} · revision {latestAttempt.revision}</small
								>
							{/if}
						</div>
						{#if latestAttempt}
							<strong class="attempt-state state-{latestAttempt.state}"
								>{latestAttempt.state.replaceAll('_', ' ')}</strong
							>
							<p>{latestAttempt.summary || 'This Attempt has not recorded a summary yet.'}</p>
							<div class="attempt-meta">
								<span>Started {displayDate(latestAttempt.started_at)}</span>
								<span>{latestAttempt.model || 'Model not recorded'}</span>
							</div>
						{:else}
							<p class="detail-empty">No Attempt has started yet.</p>
						{/if}
					</section>

					{#if work.resolution}
						<section class="work-detail-section">
							<span class="detail-label">Current explanation</span>
							<Markdown text={work.resolution} />
						</section>
					{/if}

					<section class="contribution-trace">
						<h2>{workIsLeadOwned ? 'Lead-owned outcome' : 'Attributable Staff contribution'}</h2>
						{#if workIsLeadOwned}
							<p>
								{accountableLead?.display ?? ownerName(accountableLeadId ?? work.owner_id)} owns the integration
								judgement for this outcome. Restless does not infer a Staff contribution from chat or
								prose; any accepted, revised, or abandoned contribution has its own Work, Attempt, and
								observed output in the linked record.
							</p>
						{:else}
							<p>
								{ownerName(work.owner_id)} owns this bounded Staff responsibility for
								{accountableLead?.display ?? ownerName(accountableLeadId ?? work.owner_id)}. Its
								actual Attempt and outputs appear below. A completed Attempt is evidence of this
								contribution, not a claim that the lead accepted the whole outcome.
							</p>
						{/if}
						{#if work.status === 'abandoned'}
							<p class="contribution-status">
								This Work was abandoned and is not presented as accepted output.
							</p>
						{:else if revisions.length}
							<p class="contribution-status">
								A source-observed revision route is shown in the Work graph below.
							</p>
						{/if}
					</section>
				</main>

				<aside class="work-detail-aside" aria-label="Work facts">
					<section>
						<span
							class="detail-label"
							title="The lead accountable for integrating the whole outcome"
						>
							Accountable lead
						</span>
						<strong
							>{accountableLead?.display ?? ownerName(accountableLeadId ?? work.owner_id)}</strong
						>
						{#if !workIsLeadOwned}
							<small
								title="This bounded Work is performed by a Staff member under the accountable lead."
								>Staff responsibility: {ownerName(work.owner_id)}</small
							>
						{/if}
					</section>
					<section>
						<span class="detail-label">Evidence</span>
						<strong>{artifacts.length} linked output{artifacts.length === 1 ? '' : 's'}</strong>
						<small>{passedGates}/{gates.length} gates passed on the latest Attempt</small>
					</section>
					<section>
						<span class="detail-label">Updated</span>
						<strong>{displayDate(work.updated_at)}</strong>
					</section>
					{#if work.worktree || work.repo}
						<section>
							<span class="detail-label">Workspace</span>
							<strong>{work.worktree || work.repo}</strong>
							{#if work.integration_branch}<small>{work.integration_branch}</small>{/if}
						</section>
					{/if}
				</aside>
			</div>

			<section class="work-evidence-section">
				<header>
					<div>
						<span class="detail-label">Evidence and acceptance</span>
						<h2>What supports this Work</h2>
					</div>
					<span class="evidence-score"
						>{artifacts.length} outputs · {passedGates}/{gates.length} gates</span
					>
				</header>
				{#if work.expected_artifact}
					<p class="expected-artifact"><span>Expected output</span>{work.expected_artifact}</p>
				{/if}

				<div class="evidence-columns">
					<div class="artifact-list">
						<span class="detail-sublabel">Linked outputs</span>
						{#each artifacts as artifact (artifact.id)}
							<div class="detail-artifact">
								<MatrixGlyph rows={GLYPHS.work} size={7} />
								<span>
									<strong>{artifact.label || artifact.kind}</strong>
									<small>{artifact.uri}</small>
								</span>
								<em class:available={artifact.state === 'available'}>{artifactState(artifact)}</em>
							</div>
						{:else}
							<p class="detail-empty">No linked outputs are recorded.</p>
						{/each}
					</div>

					<div class="gate-list">
						<span class="detail-sublabel">Acceptance gates</span>
						{#each gates as gate (gate.id)}
							<div class:passed={gatePassed(gate)} class="detail-gate">
								<MatrixGlyph rows={gatePassed(gate) ? GLYPHS.check : GLYPHS.ring} size={7} />
								<span
									><strong>{gate.name}</strong><small
										>{gatePassed(gate) ? 'Passed' : 'Not passed'}</small
									></span
								>
							</div>
						{:else}
							<p class="detail-empty">No acceptance gates are recorded.</p>
						{/each}
					</div>
				</div>
			</section>

			{#if prerequisites.length || dependents.length || revisions.length}
				<section class="work-relations-section">
					<header>
						<span class="detail-label">Work graph</span>
						<h2>Handovers and review loops</h2>
					</header>
					<div class="relation-groups">
						{#if prerequisites.length}
							<div>
								<span class="detail-sublabel">Requires</span
								>{#each prerequisites as item (item.id)}<a href={relatedHref(item)}
										>{item.title}<small>R{item.revision} · {item.status}</small></a
									>{/each}
							</div>
						{/if}
						{#if dependents.length}
							<div>
								<span class="detail-sublabel">Hands over to</span
								>{#each dependents as item (item.id)}<a href={relatedHref(item)}
										>{item.title}<small>R{item.revision} · {item.status}</small></a
									>{/each}
							</div>
						{/if}
						{#if revisions.length}
							<div>
								<span class="detail-sublabel">Revision loop</span
								>{#each revisions as item (item.id)}<a class="revision" href={relatedHref(item)}
										>{item.title}<small>R{item.revision} · {item.status}</small></a
									>{/each}
							</div>
						{/if}
					</div>
				</section>
			{/if}
		</div>
	{:else if loaded && !error}
		<div class="work-detail-missing">
			<MatrixGlyph rows={GLYPHS.ring} size={14} />
			<h1>Work not found</h1>
			<p>This Work is no longer present in the current company projection.</p>
			<a class="work-back" href={backHref()}><span aria-hidden="true">←</span> Return to Work</a>
		</div>
	{:else if !error}
		<div class="work-detail-loading">Loading Work…</div>
	{/if}
</article>
