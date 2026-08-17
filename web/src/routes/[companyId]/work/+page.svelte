<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import MatrixGlyph, { GLYPHS } from '$lib/primitives/MatrixGlyph.svelte';
	import { getAttention, type AttentionView } from '$lib/model/attention';
	import { getCockpit, type CockpitView } from '$lib/model/cockpit';
	import type { WorkEdgeRow, WorkRow } from '$lib/model/generated/orgintel';

	const companyId = $derived(page.params.companyId ?? 'aris');
	let attention = $state<AttentionView | null>(null);
	let cockpit = $state<CockpitView | null>(null);
	let error = $state('');
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
			if (!goalSelectionInitialized) {
				const requestedGoal = page.url.searchParams.get('goal');
				selectedGoal = nextCockpit.goals.find((goal) => goal.id === requestedGoal)?.id ?? '';
				if (!selectedGoal && requestedGoal === UNASSIGNED_QUERY) {
					selectedGoal = UNASSIGNED_QUERY;
				}
				goalSelectionInitialized = true;
			}
		} catch (cause) {
			if (showError) error = cause instanceof Error ? cause.message : 'Work is unavailable.';
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
	const visibleWork = $derived([
		...goalWork.filter((item) => item.status !== 'completed' && item.status !== 'abandoned'),
		...(showHistory ? completedWork : recentlyLanded)
	]);
	const visibleIds = $derived(new Set(visibleWork.map((item) => item.id)));
	const visibleEdges = $derived(
		(graph?.edges ?? []).filter(
			(edge) => visibleIds.has(edge.from_work_id) && visibleIds.has(edge.to_work_id)
		)
	);
	function attemptOf(work: WorkRow) {
		return graph?.attempts.filter((attempt) => attempt.work_id === work.id).at(-1) ?? null;
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

	function prerequisites(work: WorkRow): WorkEdgeRow[] {
		return visibleEdges.filter((edge) => edge.to_work_id === work.id && edge.kind === 'requires');
	}

	function depthOf(work: WorkRow, seen = new Set<string>()): number {
		if (seen.has(work.id)) return 0;
		seen.add(work.id);
		const parents = prerequisites(work);
		if (!parents.length) return 0;
		return (
			1 +
			Math.max(
				...parents.map((edge) => {
					const parent = visibleWork.find((candidate) => candidate.id === edge.from_work_id);
					return parent ? depthOf(parent, new Set(seen)) : 0;
				})
			)
		);
	}

	const depthGroups = $derived.by(() => {
		const groups = new Map<number, WorkRow[]>();
		for (const item of visibleWork) {
			const depth = depthOf(item);
			groups.set(depth, [...(groups.get(depth) ?? []), item]);
		}
		return [...groups.entries()].sort(([a], [b]) => a - b);
	});

	type MapNode = { item: WorkRow; depth: number; x: number; y: number };
	type MapConnector = {
		key: string;
		kind: WorkEdgeRow['kind'];
		x: number;
		y: number;
		length: number;
		angle: number;
	};

	const NODE_WIDTH = 208;
	const NODE_HEIGHT = 132;
	const COLUMN_STEP = 300;
	const ROW_STEP = 160;
	const MAP_INSET = 24;

	const mapNodes = $derived.by((): MapNode[] => {
		const largestColumn = Math.max(1, ...depthGroups.map(([, rows]) => rows.length));
		return depthGroups.flatMap(([depth, rows]) => {
			const columnOffset = ((largestColumn - rows.length) * ROW_STEP) / 2;
			return rows.map((item, row) => ({
				item,
				depth,
				x: MAP_INSET + depth * COLUMN_STEP,
				y: 48 + columnOffset + row * ROW_STEP
			}));
		});
	});

	const mapConnectors = $derived.by((): MapConnector[] => {
		const positions = new Map(mapNodes.map((node) => [node.item.id, node]));
		return visibleEdges.flatMap((edge) => {
			const from = positions.get(edge.from_work_id);
			const to = positions.get(edge.to_work_id);
			if (!from || !to) return [];
			const forwards = to.x >= from.x;
			const startX = forwards ? from.x + NODE_WIDTH : from.x;
			const endX = forwards ? to.x : to.x + NODE_WIDTH;
			const startY = from.y + NODE_HEIGHT / 2;
			const endY = to.y + NODE_HEIGHT / 2;
			const dx = endX - startX;
			const dy = endY - startY;
			return [
				{
					key: `${edge.from_work_id}:${edge.to_work_id}:${edge.kind}`,
					kind: edge.kind,
					x: startX,
					y: startY,
					length: Math.hypot(dx, dy),
					angle: Math.atan2(dy, dx) * (180 / Math.PI)
				}
			];
		});
	});

	const mapWidth = $derived(
		Math.max(
			760,
			MAP_INSET * 2 + Math.max(0, ...depthGroups.map(([depth]) => depth)) * COLUMN_STEP + NODE_WIDTH
		)
	);
	const mapHeight = $derived(
		Math.max(570, 96 + Math.max(1, ...depthGroups.map(([, rows]) => rows.length)) * ROW_STEP)
	);

	const boardColumns = $derived([
		{
			key: 'proposed',
			label: 'Next',
			rows: goalWork.filter((item) => item.status === 'proposed')
		},
		{
			key: 'active',
			label: 'In motion',
			rows: goalWork.filter((item) => item.status === 'active')
		},
		{
			key: 'blocked',
			label: 'Waiting',
			rows: goalWork.filter((item) => item.status === 'blocked')
		},
		{
			key: 'completed',
			label: showHistory ? 'Completed history' : 'Recently landed',
			rows: showHistory ? completedWork : recentlyLanded
		}
	]);

	function goalProgress(goalId: string): number {
		const rows = (graph?.work ?? []).filter((item) => item.goal_id === goalId);
		if (!rows.length) return 0;
		return Math.round(
			(rows.filter((item) => item.status === 'completed').length / rows.length) * 100
		);
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
		<button class:current={!selectedGoal} type="button" onclick={() => selectGoal('')}>
			<span class="goal-index"><em>ALL</em><b>{graph?.work.length ?? 0}</b></span>
			<strong>All work</strong>
			<small>Every Work item across the company.</small>
		</button>
		<button
			class:current={selectedGoal === UNASSIGNED_QUERY}
			type="button"
			onclick={() => selectGoal(UNASSIGNED_QUERY)}
		>
			<span class="goal-index"><em>—</em><b>{unassignedWork.length}</b></span>
			<strong>Unassigned</strong>
			<small>Work not linked to a company goal.</small>
		</button>
		{#each goals as goal, index (goal.id)}
			<button
				class:current={selectedGoal === goal.id}
				type="button"
				onclick={() => selectGoal(goal.id)}
			>
				<span class="goal-index">
					<em>G–{String(index + 1).padStart(2, '0')}</em><b>{goalProgress(goal.id)}%</b>
				</span>
				<strong>{goal.title}</strong>
				<small>{goal.body || `${goal.closed_at ? 'Closed' : 'Open'} company goal.`}</small>
				<i class="goal-progress" aria-hidden="true"
					><b style={`width: ${goalProgress(goal.id)}%`}></b></i
				>
			</button>
		{:else}
			<p class="empty-state">No company goals are recorded.</p>
		{/each}
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
					onclick={() => (showHistory = !showHistory)}
				>
					<MatrixGlyph rows={showHistory ? GLYPHS.check : GLYPHS.ring} size={8} />
					{showHistory ? 'Hide history' : `${completedWork.length} completed`}
				</button>
			{/if}
			<div class="lens-switch" role="tablist" aria-label="Work view">
				<button
					type="button"
					role="tab"
					aria-selected={lens === 'map'}
					onclick={() => (lens = 'map')}>Map</button
				>
				<button
					type="button"
					role="tab"
					aria-selected={lens === 'board'}
					onclick={() => (lens = 'board')}>Board</button
				>
			</div>
		</header>

		{#if lens === 'map'}
			<div class="work-map" aria-label="Work dependency map">
				<div class="work-map-inner" style={`width: ${mapWidth}px; height: ${mapHeight}px`}>
					{#each depthGroups as [depth] (depth)}
						<span class="map-depth-label" style={`left: ${MAP_INSET + depth * COLUMN_STEP}px`}>
							{depth === 0 ? 'Starts here' : `Handover ${depth}`}
						</span>
					{/each}
					{#each mapConnectors as connector (connector.key)}
						<span
							class="map-connector {connector.kind}"
							style={`left: ${connector.x}px; top: ${connector.y}px; width: ${connector.length}px; transform: rotate(${connector.angle}deg)`}
							aria-hidden="true"
						></span>
					{/each}
					{#each mapNodes as node (node.item.id)}
						<a
							class="work-map-node status-{node.item.status}"
							href={workHref(node.item.id)}
							aria-label={`Open Work: ${node.item.title}`}
							style={`left: ${node.x}px; top: ${node.y}px`}
						>
							<span class="node-state"><i></i>R{node.item.revision} · {node.item.status}</span>
							<strong>{node.item.title}</strong>
							<p>{node.item.outcome}</p>
							<small
								>{ownerName(node.item.owner_id)} · {attemptOf(node.item)?.state ??
									'not started'}</small
							>
						</a>
					{/each}
				</div>
				{#if !visibleWork.length}
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
								<p>{item.outcome}</p>
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
