<script lang="ts">
	import { page } from '$app/state';
	import ConversationTurnDock from '$lib/primitives/ConversationTurnDock.svelte';
	import { attentionQuery, cockpitQuery, workActivityStream } from '$lib/model/queries.svelte';
	import type { ArtifactRefRow, WorkGateRow, WorkRow } from '$lib/model/generated/orgintel';
	import type {
		WorkDetailArtifact,
		WorkDetailRelation,
		WorkDetailView
	} from '$lib/product/contracts';
	import WorkDetailSurface from '$lib/work/WorkDetailSurface.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const workId = $derived(page.params.workId ?? '');
	const attentionProjection = $derived(attentionQuery(companyId));
	const cockpitProjection = $derived(cockpitQuery(companyId));
	const attention = $derived(attentionProjection.view);
	const cockpit = $derived(cockpitProjection.view);
	const loaded = $derived(
		attentionProjection.status !== 'unknown' || cockpitProjection.status !== 'unknown'
	);
	const error = $derived(
		attentionProjection.failure?.message ?? cockpitProjection.failure?.message ?? ''
	);
	const graph = $derived(attention?.workGraph ?? null);
	const work = $derived(graph?.work.find((item) => item.id === workId) ?? null);
	const goal = $derived(cockpit?.goals.find((item) => item.id === work?.goal_id) ?? null);
	const attempts = $derived(
		(graph?.attempts ?? [])
			.filter((attempt) => attempt.work_id === workId)
			.toSorted((a, b) => a.attempt_no - b.attempt_no)
	);
	const latestAttempt = $derived(attempts.at(-1) ?? null);
	const activity = $derived(
		work && latestAttempt?.state === 'running'
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
	const gates = $derived((graph?.gates ?? []).filter((gate) => gate.work_id === workId));
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
			cockpit?.people.find((person) => person.actor_id === actorId)?.display ??
			actorId.replaceAll('-', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase())
		);
	}

	function gatePassed(gate: WorkGateRow): boolean {
		if (!latestAttempt) return false;
		return (
			graph?.gate_runs.some(
				(run) => run.gate_id === gate.id && run.attempt_id === latestAttempt.id && run.passed
			) ?? false
		);
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

	function isLegacyAutomaticArtifact(artifact: ArtifactRefRow): boolean {
		return (
			!!work &&
			artifact.label === work.expected_artifact &&
			artifact.created_by === work.owner_id &&
			['file', 'repository_tree'].includes(artifact.kind)
		);
	}

	function artifactView(artifact: ArtifactRefRow): WorkDetailArtifact {
		const legacy = isLegacyAutomaticArtifact(artifact);
		return {
			id: artifact.id,
			kind: artifact.kind,
			label: legacy
				? `Output from: ${work?.title ?? 'this work'}`
				: artifact.label || artifact.kind,
			note: legacy
				? artifact.kind === 'file'
					? 'The exact file produced by this work and observed in the company runtime.'
					: 'The saved result produced by this work; Restless observed it with no uncommitted changes.'
				: artifact.note || 'Linked evidence for this Work',
			uri: artifact.uri,
			state: artifact.state,
			...(canOpenOutsideCompany(artifact.uri) ? { openHref: artifact.uri } : {})
		};
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

	function relationView(item: WorkRow): WorkDetailRelation {
		return {
			id: item.id,
			title: item.title,
			revision: item.revision,
			status: item.status,
			href: relatedHref(item)
		};
	}

	const detailView = $derived.by((): WorkDetailView | null => {
		if (!work) return null;
		const outcome = splitOutcome(work.outcome);
		const accountableName =
			accountableLead?.display ?? ownerName(accountableLeadId ?? work.owner_id);
		const mappedArtifacts = artifacts.map(artifactView);
		const recoveryArtifacts =
			unknownRecovery && latestAttempt
				? artifacts.filter((artifact) => artifact.attempt_id === latestAttempt.id).map(artifactView)
				: [];
		const preservedCandidate =
			recoveryArtifacts.find((artifact) => artifact.kind !== 'git_worktree_observation') ??
			recoveryArtifacts.at(0) ??
			null;
		const prerequisites = (graph?.edges ?? [])
			.filter((edge) => edge.to_work_id === workId && edge.kind === 'requires')
			.flatMap((edge) => graph?.work.filter((item) => item.id === edge.from_work_id) ?? [])
			.map(relationView);
		const dependents = (graph?.edges ?? [])
			.filter((edge) => edge.from_work_id === workId && edge.kind === 'requires')
			.flatMap((edge) => graph?.work.filter((item) => item.id === edge.to_work_id) ?? [])
			.map(relationView);
		const revisions = (graph?.edges ?? [])
			.filter(
				(edge) =>
					edge.kind === 'revises' && (edge.from_work_id === workId || edge.to_work_id === workId)
			)
			.flatMap((edge) => {
				const relatedId = edge.from_work_id === workId ? edge.to_work_id : edge.from_work_id;
				return graph?.work.filter((item) => item.id === relatedId) ?? [];
			})
			.map(relationView);

		return {
			id: work.id,
			title: work.title,
			status: work.status,
			goalTitle: goal?.title ?? 'Company work',
			readerSummary: (work.resolution || outcome.opening).trim(),
			readerSummaryLabel: work.resolution ? 'What happened' : 'What this Work delivers',
			executionContract: outcome.contract || work.outcome,
			ownerName: ownerName(work.owner_id),
			accountableLeadName: accountableName,
			staffResponsibilityName:
				work.owner_id === accountableLeadId ? null : ownerName(work.owner_id),
			updatedAt: work.updated_at,
			expectedArtifact: work.expected_artifact,
			workspace: work.worktree || work.repo || '',
			integrationBranch: work.integration_branch || '',
			attempt: latestAttempt
				? {
						attemptNo: latestAttempt.attempt_no,
						revision: latestAttempt.revision,
						state: latestAttempt.state,
						summary: latestAttempt.summary,
						model: latestAttempt.model,
						startedAt: latestAttempt.started_at
					}
				: null,
			artifacts: mappedArtifacts,
			gates: gates.map((gate) => ({ id: gate.id, name: gate.name, passed: gatePassed(gate) })),
			prerequisites,
			dependents,
			revisions,
			recovery:
				unknownRecovery && latestAttempt
					? { summary: latestAttempt.summary, artifacts: recoveryArtifacts, preservedCandidate }
					: null
		};
	});
</script>

{#snippet activityDock()}
	{#if workTurn && detailView}
		<section class="work-live-activity" aria-label="Live Work activity">
			<ConversationTurnDock participantName={detailView.accountableLeadName} turn={workTurn} />
		</section>
	{/if}
{/snippet}

<WorkDetailSurface
	view={detailView}
	{loaded}
	{error}
	companyName={cockpit?.company.name ?? companyId}
	platform={{ backHref: backHref() }}
	liveActivity={workTurn ? activityDock : undefined}
/>
