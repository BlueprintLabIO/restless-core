import dagre from '@dagrejs/dagre';
import type { WorkEdgeKind, WorkStatus } from '$lib/model/generated/orgintel';

export interface WorkGraphItem {
	id: string;
	owner_id: string;
	title: string;
	status: WorkStatus;
	priority: number;
	revision: number;
}

export interface WorkGraphEdge {
	from_work_id: string;
	to_work_id: string;
	kind: WorkEdgeKind;
}

export const WORK_NODE_WIDTH = 224;
export const WORK_NODE_HEIGHT = 126;

export interface WorkGraphNodeData {
	item: WorkGraphItem;
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
	/** The routed line, kept so a relayout can glide from one route to the next. */
	points: Array<{ x: number; y: number }>;
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
	work: WorkGraphItem[],
	edges: WorkGraphEdge[],
	dataFor: (item: WorkGraphItem) => WorkGraphNodeData
): WorkGraphLayout {
	const graph = new dagre.graphlib.Graph({ multigraph: true })
		.setGraph({
			rankdir: 'LR',
			nodesep: 34,
			ranksep: 64,
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
			points,
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

export function roundedPolyline(points: Array<{ x: number; y: number }>, radius: number): string {
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

/* Relayout motion. Routes from the layout engine have different numbers of
 * bends before and after a change, so both are resampled to the same count
 * along their length and interpolated point by point; the final frame then
 * snaps to the exact new route. */
const RESAMPLE = 16;

function resample(points: Array<{ x: number; y: number }>, count = RESAMPLE) {
	if (points.length < 2) return Array.from({ length: count }, () => points[0] ?? { x: 0, y: 0 });
	const lengths = [0];
	for (let index = 1; index < points.length; index += 1)
		lengths.push(
			lengths[index - 1] +
				Math.hypot(points[index].x - points[index - 1].x, points[index].y - points[index - 1].y)
		);
	const total = lengths.at(-1) || 1;
	return Array.from({ length: count }, (_, step) => {
		const at = (step / (count - 1)) * total;
		let segment = 1;
		while (segment < lengths.length - 1 && lengths[segment] < at) segment += 1;
		const span = lengths[segment] - lengths[segment - 1] || 1;
		const t = (at - lengths[segment - 1]) / span;
		const a = points[segment - 1];
		const b = points[segment];
		return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t };
	});
}

const mix = (a: number, b: number, t: number) => a + (b - a) * t;

/** The layout at progress `t` (0..1) between `from` and `to`. */
export function blendLayouts(
	from: WorkGraphLayout,
	to: WorkGraphLayout,
	t: number
): WorkGraphLayout {
	if (t >= 1) return to;
	const before = new Map(from.nodes.map((node) => [node.id, node]));
	const beforeEdges = new Map(from.edges.map((edge) => [edge.id, edge]));
	return {
		...to,
		nodes: to.nodes.map((node) => {
			const start = before.get(node.id);
			return start ? { ...node, x: mix(start.x, node.x, t), y: mix(start.y, node.y, t) } : node;
		}),
		edges: to.edges.map((edge) => {
			const start = beforeEdges.get(edge.id);
			if (!start) return edge;
			const a = resample(start.points);
			const b = resample(edge.points);
			const points = a.map((point, index) => ({
				x: mix(point.x, b[index].x, t),
				y: mix(point.y, b[index].y, t)
			}));
			return {
				...edge,
				path: roundedPolyline(points, 9),
				labelX: mix(start.labelX, edge.labelX, t),
				labelY: mix(start.labelY, edge.labelY, t)
			};
		})
	};
}
