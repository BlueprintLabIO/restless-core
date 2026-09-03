<script lang="ts">
	import type { Snippet } from 'svelte';
	import MatrixGlyph, { GLYPHS } from '../primitives/MatrixGlyph.svelte';
	import type {
		WorkSurfaceItem,
		WorkSurfaceLens,
		WorkSurfacePlatform,
		WorkSurfaceView
	} from '../product/contracts';
	import WorkGraph from './WorkGraph.svelte';

	let {
		view = null,
		loaded,
		error = '',
		companyName,
		initialLens = 'map',
		initialGoal = '',
		platform,
		headerActions
	}: {
		view?: WorkSurfaceView | null;
		loaded: boolean;
		error?: string;
		companyName: string;
		initialLens?: WorkSurfaceLens;
		initialGoal?: string;
		platform: WorkSurfacePlatform;
		headerActions?: Snippet;
	} = $props();

	let lens = $state<WorkSurfaceLens>('map');
	let lensInitialized = $state(false);
	const ALL_WORK_QUERY = 'all';
	const UNASSIGNED_QUERY = 'unassigned';
	let selectedGoal = $state('');
	let goalSelectionInitialized = $state(false);
	let showHistory = $state(false);

	$effect(() => {
		if (lensInitialized) return;
		lens = initialLens;
		lensInitialized = true;
	});

	$effect(() => {
		if (!view || goalSelectionInitialized) return;
		selectedGoal = view.goals.some((goal) => goal.id === initialGoal) ? initialGoal : '';
		if (!selectedGoal && initialGoal === UNASSIGNED_QUERY) selectedGoal = UNASSIGNED_QUERY;
		goalSelectionInitialized = true;
	});

	const goals = $derived(view?.goals ?? []);
	const unassignedWork = $derived((view?.work ?? []).filter((item) => item.goalId === null));
	const goalWork = $derived(
		(view?.work ?? []).filter(
			(item) =>
				!selectedGoal ||
				(selectedGoal === UNASSIGNED_QUERY
					? item.goalId === null
					: item.goalId === selectedGoal)
		)
	);
	const completedWork = $derived(
		goalWork
			.filter((item) => item.status === 'completed')
			.toSorted((a, b) => Date.parse(b.updatedAt) - Date.parse(a.updatedAt))
	);
	const evidenceBackedCompleted = $derived(
		completedWork.filter((item) => item.artifactCount > 0 || item.gatesPassed > 0)
	);
	const recentlyLanded = $derived(evidenceBackedCompleted.slice(0, 3));
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
		(view?.edges ?? []).filter(
			(edge) => visibleIds.has(edge.fromWorkId) && visibleIds.has(edge.toWorkId)
		)
	);
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
		const rows = (view?.work ?? []).filter((item) => item.goalId === goalId);
		if (!rows.length) return 'No Work';
		const landed = rows.filter((item) => item.status === 'completed').length;
		const inMotion = rows.filter((item) => item.status === 'active').length;
		return `${landed} landed · ${inMotion} in motion`;
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
		return platform.workHref(workId, {
			goalId: selectedGoal || ALL_WORK_QUERY,
			lens
		});
	}

	function itemEvidence(item: WorkSurfaceItem): string {
		return `${item.gatesPassed}/${item.gatesTotal} gates`;
	}
</script>

<svelte:head><title>Work — {companyName}</title></svelte:head>

<div class="cockpit-screen work-screen">
	{#if error}<div class="cockpit-error">{error}</div>{/if}
	<aside class="goal-spine cockpit-pane" aria-label="Company goals">
		<header class="cockpit-pane-head compact">
			<div><h2>Goals</h2></div>
			<span class="pane-count">{goals.length}</span>
		</header>
		{#if loaded && view}
			<button
				class:current={!selectedGoal}
				type="button"
				aria-pressed={!selectedGoal}
				title="Every Work item across the company"
				onclick={() => selectGoal('')}
			>
				<span class="goal-index"><em>ALL</em><b>{view.work.length}</b></span>
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
					title={goal.body || `${goal.closedAt ? 'Closed' : 'Open'} company goal`}
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
			{#if headerActions}{@render headerActions()}{/if}
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
		{:else if !view}
			<p class="empty-state">Work is unavailable. No empty state is being inferred.</p>
		{:else if lens === 'map'}
			<div class="work-map" aria-label="Work dependency map">
				{#if visibleWork.length}
					<WorkGraph
						work={visibleWork}
						edges={visibleEdges}
						totalCount={totalGraphWork}
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
								<p>{item.attemptState} · {itemEvidence(item)}</p>
								<footer>
									<span>{item.ownerName}</span><span>{item.artifactCount} outputs</span>
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
