<script lang="ts">
	import type { WorkSurfaceEdge, WorkSurfaceItem } from '../product/contracts';
	import { layoutWorkGraph, WORK_NODE_HEIGHT, WORK_NODE_WIDTH } from './layout';

	let {
		work,
		edges,
		totalCount,
		workHref
	}: {
		work: WorkSurfaceItem[];
		edges: WorkSurfaceEdge[];
		totalCount: number;
		workHref: (workId: string) => string;
	} = $props();

	const focusId = $derived(
		work
			.filter((item) => item.status === 'blocked' || item.status === 'active')
			.toSorted((a, b) => b.priority - a.priority)[0]?.id ?? null
	);
	const layout = $derived.by(() =>
		layoutWorkGraph(work, edges, (item) => ({
			item,
			owner: item.ownerName,
			attemptState: item.attemptState,
			artifactCount: item.artifactCount,
			gateSummary: { passed: item.gatesPassed, total: item.gatesTotal },
			href: workHref(item.id),
			isFocus: item.id === focusId
		}))
	);
	const scopeLabel = $derived(edges.length ? 'Current path' : 'Current Work');

	function stateLabel(item: WorkSurfaceItem): string {
		return item.status === 'proposed'
			? 'Next'
			: item.status === 'active'
				? 'In motion'
				: item.status === 'blocked'
					? 'Waiting'
					: item.status === 'completed'
						? 'Landed'
						: 'Stopped';
	}

	function signal(node: (typeof layout.nodes)[number]): string {
		if (node.data.gateSummary.total) {
			return `${node.data.gateSummary.passed}/${node.data.gateSummary.total} gates`;
		}
		if (node.data.artifactCount) {
			return `${node.data.artifactCount} ${node.data.artifactCount === 1 ? 'output' : 'outputs'}`;
		}
		return node.data.attemptState.replaceAll('_', ' ');
	}
</script>

<div class="work-flow">
	{#if layout.nodes.length}
		<div
			class="work-canvas"
			style={`width: max(100%, ${layout.width}px); height: max(100%, ${layout.height}px);`}
		>
			<svg
				class="work-edges"
				width={layout.width}
				height={layout.height}
				viewBox={`0 0 ${layout.width} ${layout.height}`}
				aria-label="Work dependencies. Solid arrows mean requires; dashed return arrows mean revises."
			>
				<defs>
					<marker
						id="work-requires-arrow"
						viewBox="0 0 10 10"
						refX="8"
						refY="5"
						markerWidth="7"
						markerHeight="7"
						orient="auto-start-reverse"
					>
						<path d="M 0 0 L 10 5 L 0 10 z" fill="var(--intent-conversation)"></path>
					</marker>
					<marker
						id="work-revises-arrow"
						viewBox="0 0 10 10"
						refX="8"
						refY="5"
						markerWidth="7"
						markerHeight="7"
						orient="auto-start-reverse"
					>
						<path d="M 0 0 L 10 5 L 0 10 z" fill="var(--intent-feedback)"></path>
					</marker>
				</defs>
				{#each layout.edges as edge (edge.id)}
					<g class:revises={edge.kind === 'revises'}>
						<title>{edge.kind === 'revises' ? 'Revision return' : 'Required handover'}</title>
						<path class="edge-underlay" d={edge.path}></path>
						<path class="edge-line" d={edge.path} marker-end={`url(#work-${edge.kind}-arrow)`}
						></path>
						{#if edge.kind === 'revises'}
							<text x={edge.labelX} y={edge.labelY - 7} text-anchor="middle">revision return</text>
						{/if}
					</g>
				{/each}
			</svg>
			{#each layout.nodes as node (node.id)}
				<a
					class="work-flow-node status-{node.data.item.status}"
					class:is-focus={node.data.isFocus}
					href={node.data.href}
					style={`left: ${node.x}px; top: ${node.y}px; width: ${WORK_NODE_WIDTH}px; height: ${WORK_NODE_HEIGHT}px;`}
					aria-label={`Open Work: ${node.data.item.title}`}
				>
					<header>
						<span class="node-state"><i></i>{stateLabel(node.data.item)}</span>
						<span class="node-revision">R{node.data.item.revision}</span>
					</header>
					<strong>{node.data.item.title}</strong>
					<footer>
						<span>{node.data.owner}</span>
						<small>{signal(node)}</small>
					</footer>
				</a>
			{/each}
		</div>
	{:else}
		<p class="empty-graph">No current Work is available for this scope.</p>
	{/if}
	<div class="work-flow-scope">
		<strong>{scopeLabel}</strong>
		<span>{work.length} of {totalCount} Work items</span>
	</div>
</div>

<style>
	.work-flow {
		position: relative;
		width: 100%;
		height: 100%;
		min-width: 0;
		min-height: 0;
		overflow: auto;
		background:
			linear-gradient(90deg, transparent, rgba(47, 108, 168, 0.018) 50%, transparent),
			var(--surface-pane);
	}
	.work-canvas {
		position: relative;
		min-width: 100%;
		min-height: 100%;
	}
	.work-edges {
		position: absolute;
		top: 0;
		left: 0;
		overflow: visible;
	}
	.edge-underlay,
	.edge-line {
		fill: none;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
	.edge-underlay {
		stroke: var(--surface-pane);
		stroke-width: 6px;
	}
	.edge-line {
		stroke: color-mix(in srgb, var(--intent-conversation) 88%, var(--ink));
		stroke-width: 1.45px;
	}
	.work-edges g.revises .edge-line {
		stroke: var(--intent-feedback);
		stroke-dasharray: 5 5;
	}
	.work-edges text {
		fill: var(--intent-feedback);
		font: 500 var(--t-label) var(--font-mono);
		paint-order: stroke;
		stroke: var(--surface-pane);
		stroke-width: 5px;
	}
	.work-flow-node {
		position: absolute;
		display: grid;
		grid-template-rows: auto minmax(0, 1fr) auto;
		padding: 12px 13px 11px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: rgba(255, 255, 255, 0.96);
		box-shadow:
			var(--bevel-subtle),
			0 1px 2px rgba(43, 51, 66, 0.06),
			0 8px 22px rgba(43, 51, 66, 0.055);
		color: var(--ink);
		text-decoration: none;
		transition:
			border-color var(--motion-state) var(--ease-standard),
			box-shadow var(--motion-state) var(--ease-standard),
			transform var(--motion-press) var(--ease-standard);
	}
	.work-flow-node::before {
		content: '';
		position: absolute;
		top: 10px;
		bottom: 10px;
		left: -1px;
		width: 2px;
		border-radius: 2px;
		background: var(--node-tone, var(--intent-direction));
		opacity: 0.38;
	}
	.work-flow-node:hover {
		border-color: color-mix(in srgb, var(--node-tone, var(--intent-feedback)) 42%, var(--border));
		box-shadow:
			var(--bevel),
			0 12px 30px rgba(43, 51, 66, 0.1);
		transform: translateY(-1px);
	}
	.work-flow-node:focus-visible {
		outline: 3px solid color-mix(in srgb, var(--intent-conversation) 30%, transparent);
		outline-offset: 3px;
	}
	.work-flow-node.is-focus {
		border-color: color-mix(in srgb, var(--node-tone, var(--intent-feedback)) 56%, var(--border));
		box-shadow:
			var(--bevel),
			0 0 0 2px color-mix(in srgb, var(--node-tone, var(--intent-feedback)) 10%, transparent),
			0 12px 30px rgba(43, 51, 66, 0.09);
	}
	.work-flow-node.status-active {
		--node-tone: var(--intent-conversation);
	}
	.work-flow-node.status-blocked {
		--node-tone: var(--intent-authority);
	}
	.work-flow-node.status-completed {
		--node-tone: var(--state-success);
	}
	.work-flow-node.status-abandoned {
		--node-tone: var(--text-tertiary);
	}
	.work-flow-node header,
	.work-flow-node footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
		min-width: 0;
	}
	.node-state,
	.node-revision,
	.work-flow-node footer {
		font: 500 var(--t-label) var(--font-mono);
		color: var(--text-tertiary);
	}
	.node-state {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		color: var(--node-tone, var(--intent-direction));
	}
	.node-state i {
		width: 7px;
		height: 7px;
		border: 1px solid currentColor;
		border-radius: 50%;
		background: currentColor;
	}
	.work-flow-node > strong {
		display: -webkit-box;
		align-self: center;
		overflow: hidden;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		font-size: var(--t-body);
		font-weight: 600;
		line-height: 1.36;
	}
	.work-flow-node footer {
		padding-top: 8px;
		border-top: 1px solid var(--border);
	}
	.work-flow-node footer > span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.work-flow-node footer small {
		flex: none;
		font: inherit;
		color: var(--text-secondary);
	}
	.work-flow-scope {
		position: sticky;
		z-index: 5;
		left: 12px;
		bottom: 12px;
		display: inline-flex;
		align-items: center;
		gap: 7px;
		margin: 0 0 12px 12px;
		padding: 6px 8px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: rgba(255, 255, 255, 0.9);
		box-shadow:
			var(--bevel-subtle),
			0 4px 12px rgba(43, 51, 66, 0.05);
		font: var(--t-label) var(--font-mono);
		color: var(--text-tertiary);
		pointer-events: none;
		backdrop-filter: blur(10px);
	}
	.work-flow-scope strong {
		color: var(--intent-feedback);
		font-weight: 600;
	}
	.work-flow-scope span::before {
		content: '·';
		margin-right: 7px;
	}
	.empty-graph {
		margin: 0;
		padding: 24px;
		color: var(--text-tertiary);
	}
	@media (prefers-reduced-motion: reduce) {
		.work-flow-node {
			transition: none;
		}
	}
</style>
