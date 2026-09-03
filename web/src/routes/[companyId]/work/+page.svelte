<script lang="ts">
	import { page } from '$app/state';
	import { attentionQuery, cockpitQuery } from '$lib/model/queries.svelte';
	import type { WorkRow } from '$lib/model/generated/orgintel';
	import type { WorkSurfaceView } from '$lib/product/contracts';
	import WorkSurface from '$lib/work/WorkSurface.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	const attentionProjection = $derived(attentionQuery(companyId));
	const cockpitProjection = $derived(cockpitQuery(companyId));
	const attention = $derived(attentionProjection.view);
	const cockpit = $derived(cockpitProjection.view);
	const graph = $derived(attention?.workGraph ?? null);
	const error = $derived(
		attentionProjection.failure?.message ?? cockpitProjection.failure?.message ?? ''
	);
	const loaded = $derived(
		attentionProjection.status !== 'unknown' || cockpitProjection.status !== 'unknown'
	);

	function attemptOf(work: WorkRow) {
		return (
			graph?.attempts
				.filter((attempt) => attempt.work_id === work.id)
				.toSorted(
					(a, b) =>
						a.revision - b.revision ||
						a.attempt_no - b.attempt_no ||
						Date.parse(a.started_at) - Date.parse(b.started_at)
				)
				.at(-1) ?? null
		);
	}

	function artifactCount(work: WorkRow): number {
		return graph?.artifacts.filter((artifact) => artifact.work_id === work.id).length ?? 0;
	}

	function gateCount(work: WorkRow): { passed: number; total: number } {
		const gates = graph?.gates.filter((gate) => gate.work_id === work.id) ?? [];
		const latest = attemptOf(work);
		const passed = latest
			? gates.filter((gate) =>
					graph?.gate_runs.some(
						(run) => run.gate_id === gate.id && run.attempt_id === latest.id && run.passed
					)
				).length
			: 0;
		return { passed, total: gates.length };
	}

	function ownerName(actorId: string): string {
		return (
			cockpit?.people.find((person) => person.actor_id === actorId)?.display ??
			actorId.replaceAll('-', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase())
		);
	}

	const surfaceView = $derived.by((): WorkSurfaceView | null => {
		if (!graph || !cockpit) return null;
		return {
			goals: cockpit.goals.map((goal) => ({
				id: goal.id,
				title: goal.title,
				body: goal.body,
				closedAt: goal.closed_at
			})),
			work: graph.work.map((work) => {
				const gates = gateCount(work);
				return {
					id: work.id,
					title: work.title,
					status: work.status,
					revision: work.revision,
					priority: work.priority,
					goalId: work.goal_id,
					ownerId: work.owner_id,
					ownerName: ownerName(work.owner_id),
					updatedAt: work.updated_at,
					attemptState: attemptOf(work)?.state ?? 'Not started',
					artifactCount: artifactCount(work),
					gatesPassed: gates.passed,
					gatesTotal: gates.total
				};
			}),
			edges: graph.edges.map((edge, index) => ({
				id: `${edge.from_work_id}:${edge.to_work_id}:${edge.kind}:${index}`,
				fromWorkId: edge.from_work_id,
				toWorkId: edge.to_work_id,
				kind: edge.kind
			}))
		};
	});

	const platform = {
		workHref(workId: string, selection: { goalId: string; lens: 'map' | 'board' }): string {
			const query = new URLSearchParams({
				goal: selection.goalId,
				lens: selection.lens
			});
			return `/${encodeURIComponent(companyId)}/work/${encodeURIComponent(workId)}?${query}`;
		}
	};
</script>

<WorkSurface
	view={surfaceView}
	{loaded}
	{error}
	companyName={cockpit?.company.name ?? companyId}
	initialLens={page.url.searchParams.get('lens') === 'board' ? 'board' : 'map'}
	initialGoal={page.url.searchParams.get('goal') ?? ''}
	{platform}
/>
