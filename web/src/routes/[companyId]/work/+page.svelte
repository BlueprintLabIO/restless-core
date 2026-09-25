<script lang="ts">
	import { WORK_STATUS_LABEL, runStateLabel, workStatusLabel } from '$lib/work/status';
	import { resizePane } from '$lib/actions/resize-pane';
	import { page } from '$app/state';
	import { replaceState } from '$app/navigation';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import {
		attentionQuery,
		cockpitQuery,
		collaborationBootstrapQuery,
		companyPrincipalQuery
	} from '$lib/model/queries.svelte';
	import type { CollaborationWork } from '$lib/model/collaboration';
	import type { WorkRow } from '$lib/model/generated/orgintel';
	import type { WorkGraphItem } from '$lib/work/layout';
	import WorkGraph from '$lib/work/WorkGraph.svelte';

	const companyId = $derived(page.params.companyId ?? 'aris');
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
	const error = $derived(
		principalProjection.failure?.message ??
			(ownerAccess
				? (attentionProjection.failure?.message ?? cockpitProjection.failure?.message)
				: collaborationProjection.failure?.message) ??
			''
	);
	const loaded = $derived(
		principalProjection.status !== 'unknown' &&
			(ownerAccess
				? attentionProjection.status !== 'unknown' || cockpitProjection.status !== 'unknown'
				: collaborationProjection.status !== 'unknown')
	);
	type WorkItem = WorkRow | CollaborationWork;
	/* A dependency map needs width to be legible; on a phone the board, stacked
	 * as one list, is the useful first view. An explicit lens always wins. */
	const requestedLens = page.url.searchParams.get('lens');
	let lens = $state<'map' | 'board'>(
		requestedLens === 'board' || requestedLens === 'map'
			? requestedLens
			: typeof window !== 'undefined' && window.matchMedia('(max-width: 760px)').matches
				? 'board'
				: 'map'
	);
	const ALL_WORK_QUERY = 'all';
	const UNASSIGNED_QUERY = 'unassigned';
	let selectedGoal = $state<string>('');
	let goalSelectionInitialized = $state(false);
	let showHistory = $state(false);

	$effect(() => {
		if (!loaded || goalSelectionInitialized) return;
		const requestedGoal = page.url.searchParams.get('goal');
		/* A named goal can only be matched once the goals themselves arrive. */
		const goalsLoaded = ownerAccess ? !!cockpit : !!collaboration;
		if (
			requestedGoal &&
			requestedGoal !== UNASSIGNED_QUERY &&
			requestedGoal !== ALL_WORK_QUERY &&
			!goalsLoaded
		)
			return;
		selectedGoal = goals.find((goal) => goal.id === requestedGoal)?.id ?? '';
		if (!selectedGoal && requestedGoal === UNASSIGNED_QUERY) selectedGoal = UNASSIGNED_QUERY;
		goalSelectionInitialized = true;
	});

	/* The view is part of the address: a filtered board can be reloaded,
	 * shared or reached again with Back. Shallow, so nothing reloads. */
	$effect(() => {
		if (!goalSelectionInitialized) return;
		const url = new URL(page.url);
		if (selectedGoal) url.searchParams.set('goal', selectedGoal);
		else url.searchParams.delete('goal');
		url.searchParams.set('lens', lens);
		if (url.search !== page.url.search) replaceState(url, page.state);
	});

	const graph = $derived(
		ownerAccess ? (attention?.workGraph ?? null) : (collaboration?.work_graph ?? null)
	);
	const goals = $derived(ownerAccess ? (cockpit?.goals ?? []) : (collaboration?.goals ?? []));
	const people = $derived(ownerAccess ? (cockpit?.people ?? []) : (collaboration?.people ?? []));
	const companyName = $derived(
		ownerAccess ? (cockpit?.company.name ?? companyId) : (collaboration?.company.name ?? companyId)
	);
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
		ownerAccess
			? completedWork.filter((item) => artifactCount(item) > 0 || gateCount(item).passed > 0)
			: completedWork
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
	function attemptOf(work: WorkGraphItem) {
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

	function artifactCount(work: WorkGraphItem): number {
		return graph?.artifacts.filter((artifact) => artifact.work_id === work.id).length ?? 0;
	}

	function gateCount(work: WorkGraphItem): { passed: number; total: number } {
		if (!graph || !('gates' in graph)) return { passed: 0, total: 0 };
		const gates = graph.gates.filter((gate) => gate.work_id === work.id);
		const latest = attemptOf(work);
		const passed = latest
			? gates.filter((gate) =>
					graph.gate_runs.some(
						(run) => run.gate_id === gate.id && run.attempt_id === latest.id && run.passed
					)
				).length
			: 0;
		return { passed, total: gates.length };
	}

	function attemptState(work: WorkGraphItem): string {
		return attemptOf(work)?.state ?? '';
	}

	const boardColumns = $derived([
		{
			key: 'proposed',
			label: WORK_STATUS_LABEL.proposed,
			rows: visibleWork.filter((item) => item.status === 'proposed')
		},
		{
			key: 'active',
			label: WORK_STATUS_LABEL.active,
			rows: visibleWork.filter((item) => item.status === 'active')
		},
		{
			key: 'blocked',
			label: WORK_STATUS_LABEL.blocked,
			rows: visibleWork.filter((item) => item.status === 'blocked')
		},
		{
			key: 'completed',
			label: WORK_STATUS_LABEL.completed,
			rows: visibleWork.filter((item) => item.status === 'completed')
		}
	]);

	function goalProgress(goalId: string): string {
		const rows = (graph?.work ?? []).filter((item) => item.goal_id === goalId);
		if (!rows.length) return 'No Work';
		const done = rows.filter((item) => item.status === 'completed').length;
		return `${done}/${rows.length}`;
	}

	function goalShare(goalId: string): number {
		const rows = (graph?.work ?? []).filter((item) => item.goal_id === goalId);
		return rows.length
			? rows.filter((item) => item.status === 'completed').length / rows.length
			: 0;
	}

	function ownerName(actorId: string): string {
		return (
			people.find((person) => person.actor_id === actorId)?.display ??
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

	function boardSignal(work: WorkItem): string {
		const gates = gateCount(work);
		const outputs = artifactCount(work);
		return [
			runStateLabel(attemptState(work)),
			gates.total ? `${gates.passed}/${gates.total} gates` : '',
			outputs ? `${outputs} ${outputs === 1 ? 'output' : 'outputs'}` : ''
		]
			.filter(Boolean)
			.join(' · ');
	}
</script>

<svelte:head><title>Work — {companyName}</title></svelte:head>

<div
	class="cockpit-screen work-screen"
	use:resizePane={{
		key: `${companyId}:work`,
		label: 'Resize work panes',
		target: '.goal-spine',
		variable: '--work-index-w',
		min: 170,
		minOther: 280,
		defaultSize: 228,
		enabled: true
	}}
>
	{#if error}<div class="cockpit-error">{error}</div>{/if}
	<aside class="goal-spine cockpit-pane" aria-label="Company goals">
		<header class="cockpit-pane-head compact">
			<div>
				<h2>Goals</h2>
			</div>
			<span class="pane-count">{goals.length}</span>
		</header>
		<div class="goal-list">
			{#if loaded && graph}
				<button
					class:current={!selectedGoal}
					type="button"
					aria-pressed={!selectedGoal}
					title="Every Work item across the company"
					onclick={() => selectGoal('')}
				>
					<strong>All work</strong>
					<b class="goal-count">{graph?.work.length ?? 0}</b>
				</button>
				<button
					class:current={selectedGoal === UNASSIGNED_QUERY}
					type="button"
					aria-pressed={selectedGoal === UNASSIGNED_QUERY}
					title="Work not linked to a company goal"
					onclick={() => selectGoal(UNASSIGNED_QUERY)}
				>
					<strong>Unassigned</strong>
					<b class="goal-count">{unassignedWork.length}</b>
				</button>
				{#each goals as goal (goal.id)}
					<button
						class:current={selectedGoal === goal.id}
						type="button"
						aria-pressed={selectedGoal === goal.id}
						title={goal.body || `${goal.closed_at ? 'Closed' : 'Open'} company goal`}
						onclick={() => selectGoal(goal.id)}
					>
						<strong>{goal.title}</strong>
						<b class="goal-count" title="Done of total Work">{goalProgress(goal.id)}</b>
						<i class="goal-progress" style:--goal-done={goalShare(goal.id)} aria-hidden="true"></i>
					</button>
				{:else}
					<p class="empty-state">No company goals are recorded.</p>
				{/each}
			{:else if !loaded}
				<p class="empty-state">Loading goals…</p>
			{:else}
				<p class="empty-state">Goals are unavailable.</p>
			{/if}
		</div>
	</aside>

	<section class="work-stage cockpit-pane">
		<header class="cockpit-pane-head work-stage-head">
			<div class="work-heading">
				<h1>
					{selectedGoal === UNASSIGNED_QUERY
						? 'Unassigned work'
						: (goals.find((goal) => goal.id === selectedGoal)?.title ?? 'Company work')}
				</h1>
			</div>
			<div class="work-utilities">
				<a class="work-documents-link" href={`/${encodeURIComponent(companyId)}/work/documents`}>
					<MatrixGlyph rows={GLYPHS.rules} size={7} /> Documents
				</a>
				<div class="lens-switch" class:board={lens === 'board'} role="group" aria-label="Work view">
					<button type="button" aria-pressed={lens === 'map'} onclick={showMap}>Map</button>
					<button type="button" aria-pressed={lens === 'board'} onclick={() => (lens = 'board')}
						>Board</button
					>
				</div>
			</div>
		</header>

		{#if !loaded}
			<p class="empty-state">Loading the current Work projection…</p>
		{:else if !graph}
			<p class="empty-state">Work is unavailable. No empty state is being inferred.</p>
		{:else if lens === 'map'}
			<div class="work-map" aria-label="Work dependency map">
				<details class="map-legend">
					<summary title="Explain the map lines">Map key</summary>
					<div class="map-key" aria-label="Map relationships">
						<span><i></i>Requires</span><span><i class="revision"></i>Revises</span>
					</div>
				</details>

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
								<strong>{item.title}</strong>
								{#if boardSignal(item)}<p>{boardSignal(item)}</p>{/if}
								<footer>
									<span>{ownerName(item.owner_id)}</span>
									{#if item.revision > 1}<span title="Revision">R{item.revision}</span>{/if}
								</footer>
							</a>
						{:else}
							<p class="column-empty">
								{column.key === 'completed' && completedWork.length
									? 'No evidence-backed completion yet.'
									: 'None'}
							</p>
						{/each}
						{#if column.key === 'completed' && completedWork.length > recentlyLanded.length}
							<button
								class="board-history-toggle"
								type="button"
								onclick={() => (showHistory = !showHistory)}
							>
								{showHistory ? 'Show recent only' : `View all ${completedWork.length} completed`}
							</button>
						{/if}
					</section>
				{/each}
			</div>
		{/if}
		<nav class="mobile-work-links" aria-label="Work resources">
			{#if loaded && graph}
				<!-- Phones hide the Goals list; the same filter lives here. -->
				<select
					class="mobile-goal"
					aria-label="Goal"
					value={selectedGoal}
					onchange={(event) => selectGoal(event.currentTarget.value)}
				>
					<option value="">All work</option>
					<option value={UNASSIGNED_QUERY}>Unassigned</option>
					{#each goals as goal (goal.id)}<option value={goal.id}>{goal.title}</option>{/each}
				</select>
			{/if}
			<a href={`/${encodeURIComponent(companyId)}/work/documents`}>Documents</a>
			{#if completedWork.length}<button
					type="button"
					aria-pressed={showHistory}
					onclick={toggleHistory}>Completed ({completedWork.length})</button
				>{/if}
		</nav>
	</section>
</div>

<style>
	.mobile-work-links {
		display: none;
	}
	@media (max-width: 760px) {
		:global(.bridge-root) .work-stage {
			grid-template-rows: var(--pane-head-h) minmax(0, 1fr) auto;
		}
		.mobile-work-links {
			display: flex;
			align-items: center;
			gap: 8px;
			padding: 8px 10px;
			border-top: 1px solid var(--border);
		}
		.mobile-goal {
			flex: 1;
			min-width: 0;
			min-height: 44px;
			padding: 0 10px;
			border: 1px solid var(--border-strong);
			border-radius: var(--radius-control);
			background: var(--surface);
			color: var(--ink);
			font: 500 var(--t-body) var(--font-ui);
		}
		.mobile-work-links a,
		.mobile-work-links button {
			min-height: 44px;
			display: inline-flex;
			align-items: center;
			padding: 0 10px;
			border: 0;
			border-radius: var(--radius-control);
			background: none;
			color: var(--ink);
			font: 500 var(--t-body) var(--font-ui);
			text-decoration: none;
			cursor: pointer;
		}
		.work-utilities .lens-switch button {
			min-height: 40px;
		}
		.map-legend {
			bottom: 50px !important;
		}
	}

	.map-legend {
		position: absolute;
		z-index: 2;
		right: 12px;
		bottom: 12px;
		padding: 7px 10px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface);
		font-size: var(--t-label);
	}
	.map-legend summary {
		cursor: pointer;
	}
	.map-legend .map-key {
		display: flex;
		margin-top: 8px;
	}
	.work-map {
		position: relative;
	}
	.work-documents-link {
		display: inline-flex;
		align-items: center;
		gap: 7px;
		padding: 6px 10px;
		border-radius: var(--radius-control);
		color: var(--text-secondary);
		font-weight: 500;
		text-decoration: none;
		transition:
			color var(--motion-state) var(--ease-standard),
			background-color var(--motion-state) var(--ease-standard);
	}
	.work-documents-link:hover {
		background: var(--accent-soft);
		color: var(--ink);
	}
	.work-heading {
		min-width: 0;
	}

	.work-heading h1 {
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
	}

	.work-utilities {
		display: flex;
		min-width: 0;
		align-items: center;
		justify-content: flex-end;
		gap: 8px;
		margin-left: auto;
	}

	.documents-entry {
		display: inline-flex;
		min-height: 30px;
		align-items: center;
		justify-content: center;
		gap: 7px;
		padding: 5px 10px;
		border: 1px solid color-mix(in srgb, var(--surface-work) 32%, var(--control-edge));
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--surface-work) 7%, var(--surface));
		box-shadow: var(--bevel-subtle), var(--control-depth);
		font: 600 var(--t-label) var(--font-sans);
		color: var(--ink);
		text-decoration: none;
		white-space: nowrap;
		transition:
			border-color var(--motion-state) var(--ease-standard),
			background var(--motion-state) var(--ease-standard),
			transform var(--motion-press) var(--ease-out),
			box-shadow var(--motion-state) var(--ease-standard);
	}

	.documents-entry :global(.matrix-glyph) {
		color: var(--surface-work);
	}

	.documents-entry:hover {
		border-color: color-mix(in srgb, var(--surface-work) 52%, var(--control-edge));
		background: color-mix(in srgb, var(--surface-work) 12%, var(--surface));
		box-shadow:
			var(--bevel-subtle),
			0 2px 7px color-mix(in srgb, var(--surface-work) 14%, transparent);
	}

	.documents-entry:active {
		transform: translateY(1px);
		box-shadow: var(--bevel-subtle), var(--control-depth-pressed);
	}

	.documents-entry:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}

	@media (max-width: 760px) {
		:global(.bridge-root) .work-stage {
			grid-template-rows: auto minmax(0, 1fr);
		}

		.work-stage-head {
			min-height: var(--pane-head-h);
			flex-wrap: wrap;
			padding: 8px 10px;
		}

		.work-heading {
			flex: 1 1 160px;
		}

		.work-utilities {
			flex: 0 0 auto;
		}

		.work-documents-link {
			display: none;
		}
	}

	@media (max-width: 520px) {
		.documents-entry {
			padding-inline: 8px;
		}

		.work-utilities :global(.lens-switch button) {
			min-width: 52px;
			padding-inline: 8px;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.documents-entry {
			transition: none;
		}
	}
</style>
