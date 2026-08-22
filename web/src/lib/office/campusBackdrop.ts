import { TILE_SIZE } from '$lib/vendor/pixel-agents/webview-ui/src/office/types.js';

export const CAMPUS_BACKDROP_VERSION = 5;

/** Three slow, presentation-only channels. None carries company truth. */
export const CAMPUS_MOTION_CHANNELS = ['water', 'leaves', 'wildlife'] as const;

export const CAMPUS_WILDLIFE_CYCLE_MS = 72_000;

export type CampusWildlifeKind = 'birds' | 'butterfly' | 'whale';

export interface CampusWildlifeMoment {
	kind: CampusWildlifeKind;
	progress: number;
}

const WILDLIFE_WINDOWS: Array<{
	kind: CampusWildlifeKind;
	start: number;
	duration: number;
}> = [
	{ kind: 'birds', start: 8_000, duration: 5_000 },
	{ kind: 'butterfly', start: 29_000, duration: 4_000 },
	{ kind: 'whale', start: 58_000, duration: 5_000 }
];

/**
 * Wildlife is intentionally absent for most of the cycle. The deterministic
 * windows keep the campus calm, make reduced-motion straightforward, and
 * prevent several ambient creatures competing for attention at once.
 */
export function campusWildlifeAt(now: number): CampusWildlifeMoment | null {
	if (!Number.isFinite(now)) return null;
	const phase =
		((now % CAMPUS_WILDLIFE_CYCLE_MS) + CAMPUS_WILDLIFE_CYCLE_MS) % CAMPUS_WILDLIFE_CYCLE_MS;
	const window = WILDLIFE_WINDOWS.find(
		(candidate) => phase >= candidate.start && phase < candidate.start + candidate.duration
	);
	return window
		? {
				kind: window.kind,
				progress: (phase - window.start) / window.duration
			}
		: null;
}

export interface CampusWorldProjection {
	officeLeft: number;
	officeTop: number;
	officeCols: number;
	officeRows: number;
	tilePixelSize: number;
}

interface CampusBackdropOptions extends CampusWorldProjection {
	canvasWidth: number;
	canvasHeight: number;
	officeTiles: readonly number[];
	now: number;
	motion: boolean;
}

/** Project one normalized campus point through the same world transform as
 * office tiles. The backdrop is presentation, but it is part of the world—not
 * a viewport wallpaper—so camera pan and zoom must affect both identically. */
export function projectCampusPoint(
	projection: CampusWorldProjection,
	xRatio: number,
	yRatio: number
): { x: number; y: number } {
	return {
		x: projection.officeLeft + xRatio * projection.officeCols * projection.tilePixelSize,
		y: projection.officeTop + yRatio * projection.officeRows * projection.tilePixelSize
	};
}

const COLORS = {
	meadow: '#94c78a',
	meadowLight: '#b8dca4',
	ridge: '#57906b',
	ridgeShade: '#3f795b',
	ridgeLight: '#79aa79',
	lake: '#68b6ba',
	lakeDeep: '#4b929a',
	lakeShallow: '#8acbca',
	wave: '#cfeaec',
	shore: '#d8dcae',
	shoreShade: '#a9b88d',
	stone: '#d9e2e5',
	stoneShade: '#9dacb5',
	stoneLight: '#eef4f3',
	treeDark: '#326a50',
	tree: '#4e9a7e',
	treeLight: '#79b58b',
	trunk: '#65778a',
	attention: '#7b66b2',
	people: '#5d8fc1',
	work: '#4e9a7e',
	sun: '#d2a554',
	flower: '#d8a6cc'
} as const;

function snap(value: number, pixel: number): number {
	return Math.round(value / pixel) * pixel;
}

function block(
	context: CanvasRenderingContext2D,
	x: number,
	y: number,
	width: number,
	height: number,
	color: string,
	pixel: number
): void {
	context.fillStyle = color;
	context.fillRect(
		snap(x, pixel),
		snap(y, pixel),
		Math.max(pixel, snap(width, pixel)),
		Math.max(pixel, snap(height, pixel))
	);
}

function polygon(
	context: CanvasRenderingContext2D,
	points: Array<[number, number]>,
	color: string,
	pixel: number
): void {
	context.fillStyle = color;
	context.beginPath();
	points.forEach(([x, y], index) => {
		const pointX = snap(x, pixel);
		const pointY = snap(y, pixel);
		if (index === 0) context.moveTo(pointX, pointY);
		else context.lineTo(pointX, pointY);
	});
	context.closePath();
	context.fill();
}

function oval(
	context: CanvasRenderingContext2D,
	centerX: number,
	centerY: number,
	radiusX: number,
	radiusY: number,
	color: string,
	pixel: number
): void {
	for (let y = -radiusY; y <= radiusY; y += pixel) {
		const normalized = 1 - (y * y) / (radiusY * radiusY);
		const halfWidth = Math.sqrt(Math.max(0, normalized)) * radiusX;
		block(context, centerX - halfWidth, centerY + y, halfWidth * 2, pixel, color, pixel);
	}
}

function treeCrown(
	context: CanvasRenderingContext2D,
	x: number,
	y: number,
	scale: number,
	pixel: number,
	variant = 0
): void {
	oval(context, x + 3 * scale, y + 12 * scale, 15 * scale, 8 * scale, '#2026332e', pixel);
	block(context, x, y + 5 * scale, 4 * scale, 12 * scale, COLORS.trunk, pixel);
	oval(context, x - 5 * scale, y, 11 * scale, 9 * scale, COLORS.treeDark, pixel);
	oval(context, x + 5 * scale, y - 4 * scale, 13 * scale, 11 * scale, COLORS.tree, pixel);
	oval(context, x + 12 * scale, y + 2 * scale, 9 * scale, 8 * scale, COLORS.treeLight, pixel);
	if (variant % 2 === 0)
		block(context, x + 5 * scale, y - 9 * scale, 4 * scale, 2 * scale, COLORS.meadowLight, pixel);
}

function rock(
	context: CanvasRenderingContext2D,
	x: number,
	y: number,
	scale: number,
	pixel: number
): void {
	polygon(
		context,
		[
			[x, y + 7 * scale],
			[x + 3 * scale, y + 2 * scale],
			[x + 10 * scale, y],
			[x + 15 * scale, y + 5 * scale],
			[x + 13 * scale, y + 10 * scale],
			[x + 3 * scale, y + 11 * scale]
		],
		COLORS.stoneShade,
		pixel
	);
	block(context, x + 4 * scale, y + 3 * scale, 7 * scale, 2 * scale, COLORS.stone, pixel);
}

function flowerPatch(
	context: CanvasRenderingContext2D,
	x: number,
	y: number,
	pixel: number,
	variant: number
): void {
	const colors = [COLORS.attention, COLORS.people, COLORS.sun, COLORS.flower];
	for (let index = 0; index < 7; index += 1) {
		const dx = ((index * 13 + variant * 9) % 43) * pixel;
		const dy = ((index * 7 + variant * 5) % 19) * pixel;
		block(context, x + dx, y + dy, 2 * pixel, 2 * pixel, colors[(index + variant) % 4], pixel);
		block(context, x + dx, y + dy + 2 * pixel, pixel, 2 * pixel, COLORS.work, pixel);
	}
}

function drawBirdFlock(
	context: CanvasRenderingContext2D,
	worldX: (ratio: number) => number,
	worldY: (ratio: number) => number,
	pixel: number,
	progress: number
): void {
	const wingFrame = Math.floor(progress * 18) % 2;
	const flock: Array<[number, number, number]> = [
		[0, 0, 0],
		[-0.035, 0.026, 1],
		[-0.072, -0.018, 0]
	];
	for (const [dx, dy, variant] of flock) {
		const x = worldX(0.08 + progress * 0.46 + dx);
		const y = worldY(0.055 + progress * 0.055 + dy);
		block(context, x, y + 4 * pixel, 4 * pixel, 2 * pixel, '#20263324', pixel);
		block(context, x + pixel, y, 2 * pixel, 5 * pixel, '#405568', pixel);
		const lift = (wingFrame + variant) % 2;
		block(context, x - 4 * pixel, y + lift * pixel, 5 * pixel, 2 * pixel, '#526673', pixel);
		block(context, x + 3 * pixel, y + lift * pixel, 5 * pixel, 2 * pixel, '#526673', pixel);
		block(context, x + pixel, y - 2 * pixel, 2 * pixel, 2 * pixel, '#eef4f3', pixel);
	}
}

function drawButterfly(
	context: CanvasRenderingContext2D,
	worldX: (ratio: number) => number,
	worldY: (ratio: number) => number,
	pixel: number,
	progress: number
): void {
	const flutter = Math.floor(progress * 20) % 2;
	const x = worldX(0.5 + progress * 0.16);
	const y = worldY(1.03) + Math.sin(progress * Math.PI * 4) * 3 * pixel;
	block(context, x, y, pixel, 2 * pixel, '#526673', pixel);
	block(
		context,
		x - (1 + flutter) * pixel,
		y - pixel,
		(1 + flutter) * pixel,
		pixel,
		COLORS.attention,
		pixel
	);
	block(context, x + pixel, y - pixel, (1 + flutter) * pixel, pixel, COLORS.sun, pixel);
}

function drawWhale(
	context: CanvasRenderingContext2D,
	worldX: (ratio: number) => number,
	worldY: (ratio: number) => number,
	pixel: number,
	progress: number
): void {
	const visibility = Math.min(1, progress * 5, (1 - progress) * 5);
	const x = worldX(0.82) + progress * 8 * pixel;
	const y = worldY(0.16) + progress * 18 * pixel;
	context.save();
	context.globalAlpha = visibility * 0.92;
	oval(context, x, y, 11 * pixel, 21 * pixel, '#3a7882', pixel);
	oval(context, x - 3 * pixel, y - 8 * pixel, 6 * pixel, 10 * pixel, '#568f98', pixel);
	block(context, x - 3 * pixel, y - 18 * pixel, 6 * pixel, 2 * pixel, '#8acbca', pixel);
	block(context, x - pixel, y - 12 * pixel, 2 * pixel, pixel, '#285d67', pixel);
	polygon(
		context,
		[
			[x - 9 * pixel, y - 2 * pixel],
			[x - 17 * pixel, y + 7 * pixel],
			[x - 8 * pixel, y + 5 * pixel]
		],
		'#356f78',
		pixel
	);
	polygon(
		context,
		[
			[x + 9 * pixel, y - 2 * pixel],
			[x + 17 * pixel, y + 7 * pixel],
			[x + 8 * pixel, y + 5 * pixel]
		],
		'#356f78',
		pixel
	);
	polygon(
		context,
		[
			[x, y + 18 * pixel],
			[x - 12 * pixel, y + 26 * pixel],
			[x - pixel, y + 27 * pixel],
			[x, y + 23 * pixel]
		],
		'#356f78',
		pixel
	);
	polygon(
		context,
		[
			[x, y + 18 * pixel],
			[x + 12 * pixel, y + 26 * pixel],
			[x + pixel, y + 27 * pixel],
			[x, y + 23 * pixel]
		],
		'#356f78',
		pixel
	);
	block(context, x - 13 * pixel, y - 19 * pixel, 9 * pixel, pixel, COLORS.wave, pixel);
	block(context, x + 5 * pixel, y - 21 * pixel, 7 * pixel, pixel, COLORS.wave, pixel);
	block(context, x - 12 * pixel, y + 24 * pixel, 8 * pixel, pixel, COLORS.lakeShallow, pixel);
	block(context, x + 5 * pixel, y + 27 * pixel, 10 * pixel, pixel, COLORS.lakeShallow, pixel);
	if (progress > 0.28 && progress < 0.58) {
		const spray = Math.floor((progress - 0.28) * 18);
		block(
			context,
			x - pixel,
			y - 20 * pixel - spray * pixel,
			2 * pixel,
			3 * pixel,
			COLORS.wave,
			pixel
		);
		block(
			context,
			x - 5 * pixel,
			y - 22 * pixel - spray * pixel,
			3 * pixel,
			pixel,
			COLORS.wave,
			pixel
		);
		block(
			context,
			x + 3 * pixel,
			y - 24 * pixel - spray * pixel,
			3 * pixel,
			pixel,
			COLORS.wave,
			pixel
		);
	}
	context.restore();
}

function drawPlateEdges(
	context: CanvasRenderingContext2D,
	options: CampusBackdropOptions,
	pixel: number
): void {
	const { officeLeft, officeTop, officeTiles, officeCols, officeRows, tilePixelSize } = options;
	const solid = (col: number, row: number) => {
		if (col < 0 || row < 0 || col >= officeCols || row >= officeRows) return false;
		return officeTiles[row * officeCols + col] !== 255;
	};
	for (let row = 0; row < officeRows; row += 1) {
		for (let col = 0; col < officeCols; col += 1) {
			if (!solid(col, row)) continue;
			const x = officeLeft + col * tilePixelSize;
			const y = officeTop + row * tilePixelSize;
			if (!solid(col + 1, row))
				block(
					context,
					x + tilePixelSize,
					y + 2 * pixel,
					3 * pixel,
					tilePixelSize,
					'#52647666',
					pixel
				);
			if (!solid(col, row + 1))
				block(
					context,
					x + 2 * pixel,
					y + tilePixelSize,
					tilePixelSize,
					3 * pixel,
					'#52647666',
					pixel
				);
			if (!solid(col - 1, row))
				block(context, x - 2 * pixel, y, 2 * pixel, tilePixelSize, COLORS.stoneLight, pixel);
			if (!solid(col, row - 1))
				block(context, x, y - 2 * pixel, tilePixelSize, 2 * pixel, COLORS.stoneLight, pixel);
		}
	}
}

function clipAroundOffice(context: CanvasRenderingContext2D, options: CampusBackdropOptions): void {
	context.beginPath();
	context.rect(0, 0, options.canvasWidth, options.canvasHeight);
	for (let row = 0; row < options.officeRows; row += 1) {
		for (let col = 0; col < options.officeCols; col += 1) {
			if (options.officeTiles[row * options.officeCols + col] === 255) continue;
			context.rect(
				options.officeLeft + col * options.tilePixelSize,
				options.officeTop + row * options.tilePixelSize,
				options.tilePixelSize,
				options.tilePixelSize
			);
		}
	}
	context.clip('evenodd');
}

export function drawCampusBackdrop(
	context: CanvasRenderingContext2D,
	options: CampusBackdropOptions
): void {
	const { canvasWidth, canvasHeight, now, motion } = options;
	const sceneWidth = options.officeCols * options.tilePixelSize;
	const sceneHeight = options.officeRows * options.tilePixelSize;
	const worldX = (ratio: number) => options.officeLeft + sceneWidth * ratio;
	const worldY = (ratio: number) => options.officeTop + sceneHeight * ratio;
	const pixel = Math.max(1, Math.round(options.tilePixelSize / TILE_SIZE));
	const outerLeft = Math.min(-2 * pixel, worldX(-0.85));
	const outerRight = Math.max(canvasWidth + 2 * pixel, worldX(1.85));
	const outerTop = Math.min(-2 * pixel, worldY(-0.85));
	const outerBottom = Math.max(canvasHeight + 2 * pixel, worldY(1.85));
	const waterFrame = motion ? Math.floor(now / 760) % 6 : 0;
	const leafFrame = motion ? Math.floor(now / 540) % 8 : 0;
	// The wildlife clock is wall-time based so reloading the office does not
	// restart the same little performance from the beginning every time.
	const wildlife = motion ? campusWildlifeAt(Date.now()) : null;

	context.save();
	context.imageSmoothingEnabled = false;
	clipAroundOffice(context, options);
	block(context, 0, 0, canvasWidth, canvasHeight, COLORS.meadow, pixel);

	// Nested contour bands create a top-down forested ridge—no horizon or
	// vanishing point competes with the office's overworld projection.
	polygon(
		context,
		[
			[outerLeft, outerTop],
			[worldX(0.48), outerTop],
			[worldX(0.41), worldY(0.12)],
			[worldX(0.32), worldY(0.2)],
			[worldX(0.18), worldY(0.25)],
			[outerLeft, worldY(0.31)]
		],
		COLORS.ridgeShade,
		pixel
	);
	polygon(
		context,
		[
			[outerLeft, outerTop],
			[worldX(0.39), outerTop],
			[worldX(0.33), worldY(0.09)],
			[worldX(0.24), worldY(0.14)],
			[worldX(0.12), worldY(0.2)],
			[outerLeft, worldY(0.22)]
		],
		COLORS.ridge,
		pixel
	);
	polygon(
		context,
		[
			[outerLeft, outerTop],
			[worldX(0.27), outerTop],
			[worldX(0.2), worldY(0.08)],
			[worldX(0.1), worldY(0.12)],
			[outerLeft, worldY(0.14)]
		],
		COLORS.ridgeLight,
		pixel
	);

	// The lake occupies the open side of the C-shaped campus. A broad shore
	// polygon underneath the water creates one irregular, readable contour.
	// Its shoreline is office-world geometry; only its far fill edges extend to
	// the viewport so zooming never reveals an unpainted canvas.
	const shorePoints: Array<[number, number]> = [
		[worldX(0.61), outerTop],
		[outerRight, outerTop],
		[outerRight, outerBottom],
		[worldX(0.72), outerBottom],
		[worldX(0.68), worldY(0.87)],
		[worldX(0.73), worldY(0.75)],
		[worldX(0.67), worldY(0.61)],
		[worldX(0.72), worldY(0.46)],
		[worldX(0.65), worldY(0.31)],
		[worldX(0.69), worldY(0.17)]
	];
	polygon(context, shorePoints, COLORS.shoreShade, pixel);
	const lakePoints = shorePoints.map(([x, y], index): [number, number] =>
		index < 4 ? [x, y] : [x + 7 * pixel, y]
	);
	polygon(context, lakePoints, COLORS.shore, pixel);
	const waterPoints = lakePoints.map(([x, y], index): [number, number] =>
		index < 4 ? [x, y] : [x + 6 * pixel, y]
	);
	polygon(context, waterPoints, COLORS.lakeDeep, pixel);
	const shallowPoints = waterPoints.map(([x, y], index): [number, number] =>
		index < 4 ? [x, y] : [x + 5 * pixel, y]
	);
	polygon(context, shallowPoints, COLORS.lake, pixel);

	for (let row = 0; row < 13; row += 1) {
		for (let col = 0; col < 7; col += 1) {
			const waveX = worldX(0.68) + col * 43 * pixel + ((row + waterFrame) % 3) * 5 * pixel;
			const waveY = worldY(row / 12) + ((col + waterFrame) % 2) * 3 * pixel;
			const width = (5 + ((row + col) % 4) * 3) * pixel;
			block(context, waveX, waveY + pixel, width, pixel, COLORS.lakeDeep, pixel);
			block(context, waveX + 2 * pixel, waveY, width, pixel, COLORS.wave, pixel);
		}
	}
	for (let index = 0; index < 12; index += 1) {
		const x = worldX(0.72) + ((index * 97 + waterFrame * 17) % Math.max(1, sceneWidth * 0.28));
		const y =
			worldY(0) +
			((index * 73 + waterFrame * 11) % Math.max(1, sceneHeight - 10 * pixel)) +
			5 * pixel;
		block(context, x, y, (3 + (index % 3) * 2) * pixel, pixel, COLORS.lakeShallow, pixel);
	}

	// Stepping stones sit just beyond the south campus edge. Their source-pixel
	// size follows the camera, exactly like furniture and character sprites.
	for (let index = 0; index < 9; index += 1) {
		const x = worldX(0.12) + index * 13 * pixel;
		const y = worldY(1.08) - index * 5 * pixel;
		oval(context, x, y, 7 * pixel, 4 * pixel, '#20263322', pixel);
		oval(context, x, y - pixel, 6 * pixel, 3 * pixel, COLORS.stone, pixel);
		block(context, x - 3 * pixel, y - 3 * pixel, 5 * pixel, pixel, COLORS.stoneLight, pixel);
	}

	const trees: Array<[number, number, number, number]> = [
		[0.07, -0.07, 1.1, 0],
		[0.22, -0.03, 0.92, 1],
		[0.4, -0.08, 1.02, 2],
		[0.58, -0.04, 1, 3],
		[0.79, -0.07, 0.86, 4],
		[0.12, 1.06, 1.05, 5],
		[0.43, 1.08, 0.82, 6],
		[0.62, 1.05, 0.9, 7]
	];
	trees.forEach(([x, y, scale, variant]) =>
		treeCrown(context, worldX(x), worldY(y), pixel * scale, pixel, variant)
	);

	for (const [x, y, scale] of [
		[0.32, -0.06, 0.8],
		[0.06, 1.03, 0.72],
		[0.69, 1.04, 0.86]
	] as Array<[number, number, number]>)
		rock(context, worldX(x), worldY(y), pixel * scale, pixel);

	flowerPatch(context, worldX(0.05), worldY(-0.01), pixel, 0);
	flowerPatch(context, worldX(0.36), worldY(1.02), pixel, 1);
	flowerPatch(context, worldX(0.56), worldY(-0.03), pixel, 2);

	// The second ambient channel is a tiny group of wind-carried leaves.
	for (let index = 0; index < 4; index += 1) {
		const x = worldX(0.18) + (leafFrame * 4 + index * 9) * pixel;
		const y = worldY(-0.02) + ((leafFrame + index * 2) % 5) * pixel;
		block(context, x, y, 2 * pixel, pixel, index % 2 ? COLORS.sun : COLORS.treeLight, pixel);
	}

	if (wildlife?.kind === 'birds') drawBirdFlock(context, worldX, worldY, pixel, wildlife.progress);
	else if (wildlife?.kind === 'butterfly')
		drawButterfly(context, worldX, worldY, pixel, wildlife.progress);
	else if (wildlife?.kind === 'whale') drawWhale(context, worldX, worldY, pixel, wildlife.progress);

	drawPlateEdges(context, options, pixel);
	context.restore();
}
