export interface BubbleRect {
	left: number;
	top: number;
	width: number;
	height: number;
}

export type BubbleTail = 'bottom' | 'left' | 'right';

export interface BubblePlacement extends BubbleRect {
	tail: BubbleTail;
	overlapArea: number;
}

export interface BubblePlacementInput {
	canvasWidth: number;
	canvasHeight: number;
	anchorX: number;
	anchorY: number;
	width: number;
	height: number;
	scale: number;
	obstacles: BubbleRect[];
}

function intersectionArea(a: BubbleRect, b: BubbleRect): number {
	const width = Math.max(
		0,
		Math.min(a.left + a.width, b.left + b.width) - Math.max(a.left, b.left)
	);
	const height = Math.max(0, Math.min(a.top + a.height, b.top + b.height) - Math.max(a.top, b.top));
	return width * height;
}

/**
 * Keep a sprite bubble attached while preferring clear screen space. This is
 * presentation-only geometry: the office engine remains responsible for world
 * collision and movement.
 */
export function chooseBubblePlacement(input: BubblePlacementInput): BubblePlacement {
	const margin = 8 * input.scale;
	const gap = 8 * input.scale;
	const tailInset = 12 * input.scale;
	const maximumLeft = Math.max(margin, input.canvasWidth - input.width - margin);
	const maximumTop = Math.max(margin, input.canvasHeight - input.height - margin);
	const candidates: Array<Omit<BubblePlacement, 'overlapArea'>> = [
		{
			left: input.anchorX - input.width / 2,
			top: input.anchorY - input.height - gap,
			width: input.width,
			height: input.height,
			tail: 'bottom'
		},
		{
			left: input.anchorX - input.width + tailInset,
			top: input.anchorY - input.height - gap,
			width: input.width,
			height: input.height,
			tail: 'bottom'
		},
		{
			left: input.anchorX - tailInset,
			top: input.anchorY - input.height - gap,
			width: input.width,
			height: input.height,
			tail: 'bottom'
		},
		{
			left: input.anchorX - input.width - gap,
			top: input.anchorY - input.height / 2,
			width: input.width,
			height: input.height,
			tail: 'right'
		},
		{
			left: input.anchorX + gap,
			top: input.anchorY - input.height / 2,
			width: input.width,
			height: input.height,
			tail: 'left'
		}
	];

	return candidates
		.map((candidate, index) => {
			const left = Math.max(margin, Math.min(maximumLeft, candidate.left));
			const top = Math.max(margin, Math.min(maximumTop, candidate.top));
			const rect = { ...candidate, left, top };
			const overlapArea = input.obstacles.reduce(
				(total, obstacle) => total + intersectionArea(rect, obstacle),
				0
			);
			const clampDistance = Math.abs(left - candidate.left) + Math.abs(top - candidate.top);
			return { rect: { ...rect, overlapArea }, score: overlapArea * 100 + clampDistance + index };
		})
		.toSorted((a, b) => a.score - b.score)[0].rect;
}
