import dagre from '@dagrejs/dagre';
import type { WorkEdgeKind, WorkEdgeRow, WorkRow } from '$lib/model/generated/orgintel';

export const WORK_NODE_WIDTH = 224;
export const WORK_NODE_HEIGHT = 126;

export interface WorkGraphNodeData {
	item: WorkRow;
	owner: string;
	attemptState: string;
	artifactCount: number;
	gateSummary: { passed: number; total: number };
	href: string;
	isFocus: boolean;
}

export interface WorkGraphLayoutNode {
	id: string;
	x: number;
	y: number;
	data: WorkGraphNodeData;
}

export interface WorkGraphLayoutEdge {
	id: string;
	kind: WorkEdgeKind;
	path: string;
	labelX: number;
	labelY: number;
}

export interface WorkGraphLayout {
	nodes: WorkGraphLayoutNode[];
	edges: WorkGraphLayoutEdge[];
	width: number;
	height: number;
}

export function layoutWorkGraph(
	work: WorkRow[],
	edges: WorkEdgeRow[],
	dataFor: (item: WorkRow) => WorkGraphNodeData
): WorkGraphLayout {
	const graph = new dagre.graphlib.Graph({ multigraph: true })
		.setGraph({
			rankdir: 'LR',
			nodesep: 34,
			ranksep: 82,
			edgesep: 16,
			marginx: 28,
			marginy: 28,
			ranker: 'network-simplex'
		})
		.setDefaultEdgeLabel(() => ({}));

	for (const item of work) {
		graph.setNode(item.id, { width: WORK_NODE_WIDTH, height: WORK_NODE_HEIGHT });
	}
	for (const edge of edges) {
		graph.setEdge(
			edge.from_work_id,
			edge.to_work_id,
			{ kind: edge.kind },
			`${edge.from_work_id}:${edge.to_work_id}:${edge.kind}`
		);
	}

	dagre.layout(graph);
	const extent = graph.graph() as { width?: number; height?: number };
	const nodes = graph.nodes().flatMap((id): WorkGraphLayoutNode[] => {
		const item = work.find((candidate) => candidate.id === id);
		const position = graph.node(id);
		if (!item || !position) return [];
		return [
			{
				id,
				x: position.x - WORK_NODE_WIDTH / 2,
				y: position.y - WORK_NODE_HEIGHT / 2,
				data: dataFor(item)
			}
		];
	});
	const laidEdges = graph.edges().map((reference): WorkGraphLayoutEdge => {
		const edge = graph.edge(reference) as {
			kind: WorkEdgeKind;
			points?: Array<{ x: number; y: number }>;
		};
		const points = edge.points ?? [];
		const middle = points[Math.floor(points.length / 2)] ?? { x: 0, y: 0 };
		return {
			id: reference.name ?? `${reference.v}:${reference.w}:${edge.kind}`,
			kind: edge.kind,
			path: roundedPolyline(points, 9),
			labelX: middle.x,
			labelY: middle.y
		};
	});

	return {
		nodes,
		edges: laidEdges,
		width: Math.max(extent.width ?? 0, WORK_NODE_WIDTH + 56),
		height: Math.max(extent.height ?? 0, WORK_NODE_HEIGHT + 56)
	};
}

function roundedPolyline(points: Array<{ x: number; y: number }>, radius: number): string {
	if (!points.length) return '';
	if (points.length === 1) return `M ${points[0].x} ${points[0].y}`;
	let result = `M ${points[0].x} ${points[0].y}`;
	for (let index = 1; index < points.length - 1; index += 1) {
		const previous = points[index - 1];
		const corner = points[index];
		const next = points[index + 1];
		const incoming = Math.hypot(corner.x - previous.x, corner.y - previous.y);
		const outgoing = Math.hypot(next.x - corner.x, next.y - corner.y);
		if (!incoming || !outgoing) {
			result += ` L ${corner.x} ${corner.y}`;
			continue;
		}
		const bend = Math.min(radius, incoming / 2, outgoing / 2);
		const beforeX = corner.x - ((corner.x - previous.x) / incoming) * bend;
		const beforeY = corner.y - ((corner.y - previous.y) / incoming) * bend;
		const afterX = corner.x + ((next.x - corner.x) / outgoing) * bend;
		const afterY = corner.y + ((next.y - corner.y) / outgoing) * bend;
		result += ` L ${beforeX} ${beforeY} Q ${corner.x} ${corner.y} ${afterX} ${afterY}`;
	}
	const last = points.at(-1)!;
	return `${result} L ${last.x} ${last.y}`;
}
