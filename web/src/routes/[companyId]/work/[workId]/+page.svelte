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
				</main>

				<aside class="work-detail-aside" aria-label="Work facts">
					<section>
						<span class="detail-label">Accountable owner</span>
						<strong>{ownerName(work.owner_id)}</strong>
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
