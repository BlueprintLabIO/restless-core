<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import { getAttention, type AttentionView } from '$lib/model/attention';
	import { getCockpit, type CockpitView } from '$lib/model/cockpit';
	import type { WorkRow } from '$lib/model/generated/orgintel';
	import WorkGraph from '$lib/work/WorkGraph.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
	let attention = $state<AttentionView | null>(null);
	let cockpit = $state<CockpitView | null>(null);
	let error = $state('');
	let loaded = $state(false);
	let lens = $state<'map' | 'board'>(
		page.url.searchParams.get('lens') === 'board' ? 'board' : 'map'
	);
	const ALL_WORK_QUERY = 'all';
	const UNASSIGNED_QUERY = 'unassigned';
	let selectedGoal = $state<string>('');
	let goalSelectionInitialized = $state(false);
	let showHistory = $state(false);

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
			loaded = true;
			if (!goalSelectionInitialized) {
				const requestedGoal = page.url.searchParams.get('goal');
				selectedGoal = nextCockpit.goals.find((goal) => goal.id === requestedGoal)?.id ?? '';
				if (!selectedGoal && requestedGoal === UNASSIGNED_QUERY) {
					selectedGoal = UNASSIGNED_QUERY;
				}
				goalSelectionInitialized = true;
			}
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Work is unavailable.';
			if (showError) loaded = true;
		}
	}

	const graph = $derived(attention?.workGraph ?? null);
	const goals = $derived(cockpit?.goals ?? []);
	const unassignedWork = $derived((graph?.work ?? []).filter((item) => item.goal_id === null));
	const goalWork = $derived(
		(graph?.work ?? []).filter(
			(item) =>
				!selectedGoal ||
				(selectedGoal === UNASSIGNED_QUERY ? item.goal_id === null : item.goal_id === selectedGoal)
		)
	);
	const completedWork = $derived(
		goalWork
			.filter((item) => item.status === 'completed')
			.toSorted((a, b) => Date.parse(b.updated_at) - Date.parse(a.updated_at))
	);
	const evidenceBackedCompleted = $derived(
		completedWork.filter((item) => artifactCount(item) > 0 || gateCount(item).passed > 0)
	);
	const recentlyLanded = $derived(evidenceBackedCompleted.slice(0, 3));
	// Map and board consume this exact row set. History expands the same
	// projection; it does not create a board-only source of status or ordering.
	const visibleWork = $derived(
		goalWork.filter(
			(item) =>
				item.status !== 'abandoned' &&
				(item.status !== 'completed' ||
					showHistory ||
					recentlyLanded.some((landed) => landed.id === item.id))
		)
	);
	const totalGraphWork = $derived(goalWork.filter((item) => item.status !== 'abandoned').length);
	const visibleIds = $derived(new Set(visibleWork.map((item) => item.id)));
	const visibleEdges = $derived(
		(graph?.edges ?? []).filter(
			(edge) => visibleIds.has(edge.from_work_id) && visibleIds.has(edge.to_work_id)
		)
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

	function attemptState(work: WorkRow): string {
		return attemptOf(work)?.state ?? 'Not started';
	}

	const boardColumns = $derived([
		{
			key: 'proposed',
			label: 'Next',
			rows: visibleWork.filter((item) => item.status === 'proposed')
		},
		{
			key: 'active',
			label: 'In motion',
			rows: visibleWork.filter((item) => item.status === 'active')
		},
		{
			key: 'blocked',
			label: 'Waiting',
			rows: visibleWork.filter((item) => item.status === 'blocked')
		},
		{
			key: 'completed',
			label: showHistory ? 'Completed history' : 'Recently landed',
			rows: visibleWork.filter((item) => item.status === 'completed')
		}
	]);

	function goalProgress(goalId: string): string {
		const rows = (graph?.work ?? []).filter((item) => item.goal_id === goalId);
		if (!rows.length) return 'No Work';
		const landed = rows.filter((item) => item.status === 'completed').length;
		const inMotion = rows.filter((item) => item.status === 'active').length;
		return `${landed} landed · ${inMotion} in motion`;
	}

	function ownerName(actorId: string): string {
		return (
			cockpit?.people.find((person) => person.actor_id === actorId)?.display ??
			actorId.replaceAll('-', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase())
		);
	}

	function selectGoal(id: string) {
		selectedGoal = id;
		showHistory = false;
	}

	function toggleHistory() {
		showHistory = !showHistory;
		if (showHistory) lens = 'board';
	}

	function showMap() {
		showHistory = false;
		lens = 'map';
	}

	function workHref(workId: string): string {
		const query = new URLSearchParams({
			goal: selectedGoal || ALL_WORK_QUERY,
			lens
		});
		return `/${encodeURIComponent(companyId)}/work/${encodeURIComponent(workId)}?${query}`;
	}
</script>

<svelte:head><title>Work — {cockpit?.company.name ?? companyId}</title></svelte:head>

<div class="cockpit-screen work-screen">
	{#if error}<div class="cockpit-error">{error}</div>{/if}
	<aside class="goal-spine cockpit-pane" aria-label="Company goals">
		<header class="cockpit-pane-head compact">
			<div>
				<h2>Goals</h2>
			</div>
			<span class="pane-count">{goals.length}</span>
		</header>
		{#if loaded && graph}
			<button
				class:current={!selectedGoal}
				type="button"
				aria-pressed={!selectedGoal}
				title="Every Work item across the company"
				onclick={() => selectGoal('')}
			>
				<span class="goal-index"><em>ALL</em><b>{graph?.work.length ?? 0}</b></span>
				<strong>All work</strong>
			</button>
			<button
				class:current={selectedGoal === UNASSIGNED_QUERY}
				type="button"
				aria-pressed={selectedGoal === UNASSIGNED_QUERY}
				title="Work not linked to a company goal"
				onclick={() => selectGoal(UNASSIGNED_QUERY)}
			>
				<span class="goal-index"><em>—</em><b>{unassignedWork.length}</b></span>
				<strong>Unassigned</strong>
			</button>
			{#each goals as goal, index (goal.id)}
				<button
					class:current={selectedGoal === goal.id}
					type="button"
					aria-pressed={selectedGoal === goal.id}
					title={goal.body || `${goal.closed_at ? 'Closed' : 'Open'} company goal`}
					onclick={() => selectGoal(goal.id)}
				>
					<span class="goal-index">
						<em>G–{String(index + 1).padStart(2, '0')}</em><b>{goalProgress(goal.id)}</b>
					</span>
					<strong>{goal.title}</strong>
				</button>
			{:else}
				<p class="empty-state">No company goals are recorded.</p>
			{/each}
		{:else if !loaded}
			<p class="empty-state">Loading goals…</p>
		{:else}
			<p class="empty-state">Goals are unavailable.</p>
		{/if}
	</aside>

	<section class="work-stage cockpit-pane">
		<header class="cockpit-pane-head">
			<div>
				<h1>
					{selectedGoal === UNASSIGNED_QUERY
						? 'Unassigned work'
						: (goals.find((goal) => goal.id === selectedGoal)?.title ?? 'Company work')}
				</h1>
			</div>
			{#if lens === 'map'}
				<div class="map-key" aria-label="Map relationships">
					<span><i></i>Requires</span>
					<span><i class="revision"></i>Revises</span>
				</div>
			{/if}
			{#if completedWork.length}
				<button
					class="history-control"
					class:on={showHistory}
					type="button"
					aria-pressed={showHistory}
					onclick={toggleHistory}
				>
					<MatrixGlyph rows={showHistory ? GLYPHS.check : GLYPHS.ring} size={8} />
					{showHistory ? 'Hide history' : `${completedWork.length} completed`}
				</button>
			{/if}
			<div class="lens-switch" class:board={lens === 'board'} role="group" aria-label="Work view">
				<button type="button" aria-pressed={lens === 'map'} onclick={showMap}>Map</button>
				<button type="button" aria-pressed={lens === 'board'} onclick={() => (lens = 'board')}
					>Board</button
				>
			</div>
		</header>

		{#if !loaded}
			<p class="empty-state">Loading the current Work projection…</p>
		{:else if !graph}
			<p class="empty-state">Work is unavailable. No empty state is being inferred.</p>
		{:else if lens === 'map'}
			<div class="work-map" aria-label="Work dependency map">
				{#if visibleWork.length}
					<WorkGraph
						work={visibleWork}
						edges={visibleEdges}
						totalCount={totalGraphWork}
						{ownerName}
						{attemptState}
						{artifactCount}
						gateSummary={gateCount}
						{workHref}
					/>
				{:else}
					<p class="empty-state">
						{selectedGoal === UNASSIGNED_QUERY
							? 'Every Work item is linked to a goal.'
							: selectedGoal
								? 'No Work is linked to this goal yet.'
								: 'No Work has been recorded yet.'}
					</p>
				{/if}
			</div>
		{:else}
			<div class="work-board" aria-label="Work board">
				{#each boardColumns as column (column.key)}
					<section class="board-column">
						<header><span>{column.label}</span><b>{column.rows.length}</b></header>
						{#each column.rows as item (item.id)}
							<a
								class="board-item status-{item.status}"
								href={workHref(item.id)}
								aria-label={`Open Work: ${item.title}`}
							>
								<span><i></i>R{item.revision} · {item.status}</span>
								<strong>{item.title}</strong>
								<p>{attemptState(item)} · {gateCount(item).passed}/{gateCount(item).total} gates</p>
								<footer>
									<span>{ownerName(item.owner_id)}</span><span>{artifactCount(item)} outputs</span>
								</footer>
							</a>
						{:else}
							<p class="column-empty">
								{column.key === 'completed' && completedWork.length
									? 'No evidence-backed completion yet.'
									: 'Clear'}
							</p>
						{/each}
						{#if column.key === 'completed' && !showHistory && completedWork.length > recentlyLanded.length}
							<button
								class="board-history-toggle"
								type="button"
								onclick={() => (showHistory = true)}
							>
								View {completedWork.length} recorded completions
							</button>
						{/if}
					</section>
				{/each}
			</div>
		{/if}
	</section>
</div>
