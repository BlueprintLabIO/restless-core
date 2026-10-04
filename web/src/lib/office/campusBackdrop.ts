import { TILE_SIZE } from '$lib/vendor/pixel-agents/webview-ui/src/office/types.js';
import { blitVisible } from '$lib/vendor/pixel-agents/webview-ui/src/office/engine/renderer.js';

/* The lakeside campus around the office: a textured meadow, a forest to the
 * north, a lake to the east, and garden beds woven around the work pavilions.
 *
 * Everything static is painted once, in world space, at one art pixel per
 * canvas pixel (a tile is 16), and blitted each frame at the camera's integer
 * zoom with nearest-neighbour scaling: pixel-identical to painting at zoom,
 * and panning or zooming never repaints it. Motion (ripples, foam, glints,
 * koi, leaves, wildlife) is a light per-frame pass that moves one art pixel at
 * a time, so nothing jumps. Coordinates below are office-local art pixels:
 * (0, 0) is the office's top-left tile corner. */

export const CAMPUS_BACKDROP_VERSION = 6;

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

/** Project one normalized campus point through the same world transform as
 * office tiles. The backdrop is part of the world, not a viewport wallpaper,
 * so camera pan and zoom move both identically. */
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

const C = {
	grass: '#9ccb8c',
	grassDark: '#8bbd7d',
	grassDeep: '#7cb072',
	grassLight: '#b1d79d',
	grassPale: '#c4e2ae',
	forestFloor: '#6c9f6c',
	forestDeep: '#5c9064',
	shadow: 'rgba(38, 72, 58, 0.28)',
	shadowSoft: 'rgba(38, 72, 58, 0.14)',
	treeDark: '#3e7356',
	tree: '#4f8b63',
	treeMid: '#63a272',
	treeLight: '#80bc84',
	treeTop: '#a4d49a',
	blossom: '#f1c9d8',
	trunk: '#6d5b4b',
	trunkLight: '#8a7560',
	sand: '#e8dfba',
	sandShade: '#d6cba0',
	sandWet: '#cfc59a',
	pebble: '#bdb59d',
	shallow: '#8ccfc8',
	mid: '#79c0bd',
	deep: '#67aeb2',
	deeper: '#5ca2ab',
	waterLine: '#86c8c3',
	waterDark: '#57929f',
	foam: '#f1f9f5',
	foamDim: '#cfeae4',
	ripple: '#c6e8e6',
	rippleDim: '#9fd3d3',
	glint: '#ffffff',
	glintArm: '#d8f1ef',
	lily: '#5f9b63',
	lilyLight: '#7cb57b',
	lotus: '#f3b9c9',
	reed: '#6f9b55',
	reedDark: '#4f7a40',
	cattail: '#8b6b4a',
	stone: '#d3d6cd',
	stoneShade: '#aeb3a9',
	stoneDark: '#8e948b',
	gravel: '#ddd6bf',
	gravelEdge: '#c9c1a5',
	gravelDot: '#bdb497',
	soil: '#a28c6c',
	soilDark: '#8a7558',
	hedge: '#5d9868',
	hedgeLight: '#7fb581',
	hedgeDark: '#467a57',
	plateShadow: 'rgba(52, 92, 66, 0.26)',
	plateShadowSoft: 'rgba(52, 92, 66, 0.12)',
	koi: '#f08a4b',
	koiWhite: '#fbf4ea',
	koiShadow: 'rgba(40, 90, 100, 0.35)',
	leafGold: '#d9b45a',
	leafGreen: '#9ccc7c',
	flowers: ['#f3e3a0', '#eab6d0', '#f7f3e9', '#bcd0f0', '#f1c89c']
} as const;

/* ---------- deterministic noise ---------- */

function hash(x: number, y: number, salt = 0): number {
	let h = (x * 374761393 + y * 668265263 + salt * 2147483647) | 0;
	h = Math.imul(h ^ (h >>> 13), 1274126177);
	h ^= h >>> 16;
	return (h >>> 0) / 4294967296;
}

function mulberry(seed: number): () => number {
	let a = seed >>> 0;
	return () => {
		a = (a + 0x6d2b79f5) | 0;
		let t = Math.imul(a ^ (a >>> 15), 1 | a);
		t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

/* ---------- pixel primitives (art pixels, integers) ---------- */

type Paint = CanvasRenderingContext2D;

function px(ctx: Paint, x: number, y: number, w: number, h: number, color: string): void {
	ctx.fillStyle = color;
	ctx.fillRect(
		Math.round(x),
		Math.round(y),
		Math.max(1, Math.round(w)),
		Math.max(1, Math.round(h))
	);
}

function disc(ctx: Paint, cx: number, cy: number, rx: number, ry: number, color: string): void {
	ctx.fillStyle = color;
	const top = Math.round(-ry);
	for (let dy = top; dy <= -top; dy += 1) {
		const k = 1 - (dy * dy) / (ry * ry + 0.01);
		if (k <= 0) continue;
		const half = Math.round(Math.sqrt(k) * rx);
		ctx.fillRect(Math.round(cx - half), Math.round(cy + dy), half * 2 + 1, 1);
	}
}

/* ---------- art pieces ---------- */

/** A layered pixel tree seen from above: soft shadow, dark underside, lit crown. */
export function paintTree(ctx: Paint, x: number, y: number, r: number, seed: number, trunk = true) {
	const rand = mulberry(seed);
	disc(ctx, x + r * 0.35, y + r * 0.55, r * 1.05, r * 0.62, C.shadow);
	if (trunk) {
		px(ctx, x - 1, y + r * 0.35, 3, r * 0.55, C.trunk);
		px(ctx, x - 1, y + r * 0.35, 1, r * 0.55, C.trunkLight);
	}
	disc(ctx, x, y, r, r * 0.92, C.treeDark);
	disc(ctx, x - r * 0.12, y - r * 0.14, r * 0.86, r * 0.78, C.tree);
	disc(ctx, x - r * 0.26, y - r * 0.3, r * 0.6, r * 0.52, C.treeMid);
	disc(ctx, x - r * 0.38, y - r * 0.42, r * 0.32, r * 0.26, C.treeLight);
	// Leaf clusters break the silhouette so a crown never reads as a circle.
	for (let i = 0; i < Math.round(r * 0.9); i += 1) {
		const a = rand() * Math.PI * 2;
		const d = r * (0.55 + rand() * 0.4);
		const lx = x + Math.cos(a) * d;
		const ly = y + Math.sin(a) * d * 0.9;
		const light = Math.sin(a) < -0.2 && Math.cos(a) < 0.3;
		disc(ctx, lx, ly, 2 + rand() * 2, 2 + rand() * 1.5, light ? C.treeMid : C.treeDark);
	}
	for (let i = 0; i < 3; i += 1)
		px(ctx, x - r * 0.45 + rand() * r * 0.4, y - r * 0.55 + rand() * r * 0.3, 1, 1, C.treeTop);
	if (seed % 7 === 0)
		for (let i = 0; i < 5; i += 1)
			px(ctx, x - r * 0.5 + rand() * r, y - r * 0.5 + rand() * r * 0.8, 1, 1, C.blossom);
}

/* A few dozen tree looks cover a whole forest: each (size, variant, trunk)
 * is painted once into a small sprite and then stamped. */
const treeSprites = new Map<string, { canvas: HTMLCanvasElement; ox: number; oy: number }>();

function stampTree(ctx: Paint, x: number, y: number, r: number, seed: number, trunk: boolean) {
	const size = Math.round(r);
	const variant = seed % 7;
	const key = `${size}:${variant}:${trunk}`;
	let sprite = treeSprites.get(key);
	if (!sprite) {
		const pad = Math.ceil(size * 1.6) + 4;
		const canvas = document.createElement('canvas');
		canvas.width = pad * 2;
		canvas.height = pad * 2;
		const layer = canvas.getContext('2d');
		if (!layer) return paintTree(ctx, x, y, r, seed, trunk);
		paintTree(layer, pad, pad, size, variant * 1_009 + size * 7, trunk);
		sprite = { canvas, ox: pad, oy: pad };
		treeSprites.set(key, sprite);
	}
	ctx.drawImage(sprite.canvas, Math.round(x) - sprite.ox, Math.round(y) - sprite.oy);
}

function paintRock(ctx: Paint, x: number, y: number, r: number, moss: boolean) {
	disc(ctx, x + 1, y + r * 0.5, r * 1.1, r * 0.55, C.shadowSoft);
	disc(ctx, x, y, r, r * 0.72, C.stoneDark);
	disc(ctx, x - 0.5, y - 1, r * 0.86, r * 0.6, C.stoneShade);
	disc(ctx, x - r * 0.25, y - r * 0.3, r * 0.45, r * 0.3, C.stone);
	if (moss) {
		disc(ctx, x + r * 0.2, y + r * 0.15, r * 0.45, r * 0.25, C.hedge);
		px(ctx, x + r * 0.1, y, 2, 1, C.hedgeLight);
	}
}

function paintFlowers(ctx: Paint, x: number, y: number, seed: number, count = 7) {
	const rand = mulberry(seed);
	for (let i = 0; i < count; i += 1) {
		const fx = x + Math.round((rand() - 0.5) * 22);
		const fy = y + Math.round((rand() - 0.5) * 12);
		px(ctx, fx, fy + 1, 1, 2, C.grassDeep);
		px(ctx, fx, fy, 1, 1, C.flowers[Math.floor(rand() * C.flowers.length)]);
	}
}

function paintShrub(ctx: Paint, x: number, y: number, r: number) {
	disc(ctx, x + 1, y + 2, r + 1, r * 0.7, C.shadowSoft);
	disc(ctx, x, y, r, r * 0.8, C.hedgeDark);
	disc(ctx, x - 1, y - 1, r * 0.75, r * 0.6, C.hedge);
	px(ctx, x - r * 0.4, y - r * 0.5, 2, 1, C.hedgeLight);
}

function paintReeds(ctx: Paint, x: number, y: number, seed: number) {
	const rand = mulberry(seed);
	for (let i = 0; i < 7; i += 1) {
		const rx = x + Math.round((rand() - 0.5) * 9);
		const h = 4 + Math.round(rand() * 5);
		px(ctx, rx, y - h, 1, h, rand() < 0.5 ? C.reed : C.reedDark);
		if (rand() < 0.35) px(ctx, rx, y - h - 2, 1, 2, C.cattail);
	}
}

function paintLily(ctx: Paint, x: number, y: number, seed: number) {
	const rand = mulberry(seed);
	disc(ctx, x, y, 3, 2, C.lily);
	px(ctx, x - 1, y - 1, 2, 1, C.lilyLight);
	px(ctx, x + 1, y, 2, 1, C.deep); // the notch
	if (rand() < 0.4) px(ctx, x - 1, y - 1, 1, 1, C.lotus);
}

/* ---------- the shoreline ---------- */

const SHORE: Array<[number, number]> = [
	[-2, 0.61],
	[0.17, 0.69],
	[0.31, 0.65],
	[0.46, 0.72],
	[0.61, 0.67],
	[0.75, 0.73],
	[0.87, 0.68],
	[3, 0.72]
];

/** The shoreline x (office-local art pixels) at each row. */
export function makeShore(width: number, height: number) {
	return (y: number): number => {
		const r = y / height;
		let i = 0;
		while (i < SHORE.length - 2 && r > SHORE[i + 1][0]) i += 1;
		const [r0, x0] = SHORE[i];
		const [r1, x1] = SHORE[i + 1];
		const t = Math.min(1, Math.max(0, (r - r0) / (r1 - r0)));
		const eased = t * t * (3 - 2 * t);
		const wobble = Math.sin(y * 0.09) * 1.6 + Math.sin(y * 0.031 + 1.7) * 2.4;
		return Math.round((x0 + (x1 - x0) * eased) * width + wobble);
	};
}

/* ---------- the world ---------- */

export interface CampusWorld {
	canvas: HTMLCanvasElement;
	/** Office-local art pixel (0,0) sits at this canvas pixel. */
	originX: number;
	originY: number;
	width: number;
	height: number;
	shore: (y: number) => number;
	ripples: Array<{
		x: number;
		y: number;
		len: number;
		speed: number;
		period: number;
		phase: number;
	}>;
	glints: Array<{ x: number; y: number; period: number; phase: number }>;
	leaves: Array<{ x: number; y: number; speed: number; drift: number; phase: number }>;
	/** Molehills in the meadow; a mole peeks out of each now and then. */
	molehills: Array<{ x: number; y: number; phase: number }>;
	/** A heron standing in the shallows. */
	heron: { x: number; y: number } | null;
	/** Where ducks paddle and fish leap: a band of open water near the shore. */
	shoreRows: { top: number; bottom: number };
	/** Office-local width and height, for placing things over the campus. */
	W: number;
	H: number;
}

const MARGIN_X = 640;
const MARGIN_Y = 480;

export interface CampusSource {
	tiles: readonly number[];
	cols: number;
	rows: number;
}

/** Paint the static campus once for this layout. */
export function buildCampusWorld(source: CampusSource): CampusWorld | null {
	if (typeof document === 'undefined') return null;
	const W = source.cols * TILE_SIZE;
	const H = source.rows * TILE_SIZE;
	const canvas = document.createElement('canvas');
	canvas.width = W + MARGIN_X * 2;
	canvas.height = H + MARGIN_Y * 2;
	const ctx = canvas.getContext('2d');
	if (!ctx) return null;
	ctx.imageSmoothingEnabled = false;
	ctx.translate(MARGIN_X, MARGIN_Y);
	const left = -MARGIN_X;
	const top = -MARGIN_Y;
	const right = W + MARGIN_X;
	const bottom = H + MARGIN_Y;
	const shore = makeShore(W, H);
	const solid = (col: number, row: number) =>
		col >= 0 &&
		row >= 0 &&
		col < source.cols &&
		row < source.rows &&
		source.tiles[row * source.cols + col] !== 255;
	// Solid-tile counts as a 2D prefix sum, so any tile rectangle is one lookup.
	const stride = source.cols + 1;
	const solidSum = new Int32Array(stride * (source.rows + 1));
	for (let row = 0; row < source.rows; row += 1)
		for (let col = 0; col < source.cols; col += 1)
			solidSum[(row + 1) * stride + col + 1] =
				(solid(col, row) ? 1 : 0) +
				solidSum[row * stride + col + 1] +
				solidSum[(row + 1) * stride + col] -
				solidSum[row * stride + col];
	const solidIn = (col0: number, row0: number, col1: number, row1: number) => {
		col0 = Math.max(0, col0);
		row0 = Math.max(0, row0);
		col1 = Math.min(source.cols - 1, col1);
		row1 = Math.min(source.rows - 1, row1);
		if (col0 > col1 || row0 > row1) return false;
		return (
			solidSum[(row1 + 1) * stride + col1 + 1] -
				solidSum[row0 * stride + col1 + 1] -
				solidSum[(row1 + 1) * stride + col0] +
				solidSum[row0 * stride + col0] >
			0
		);
	};
	// Any solid tile under a grid of samples 8px apart within `pad`. Samples are
	// closer than a tile, so together they cover every tile in their span.
	const onPlate = (x: number, y: number, pad = 0) => {
		const reach = -pad + Math.floor((2 * pad) / 8) * 8;
		return solidIn(
			Math.floor((x - pad) / TILE_SIZE),
			Math.floor((y - pad) / TILE_SIZE),
			Math.floor((x + reach) / TILE_SIZE),
			Math.floor((y + reach) / TILE_SIZE)
		);
	};
	const land = (x: number, y: number) => x < shore(y) - 14;

	// Meadow: a base green with tufts, pale specks and the odd deeper patch,
	// painted once as a 96-pixel tile and laid as a pattern (fast to build;
	// the tufts are too fine for the repeat to show).
	const grassTile = document.createElement('canvas');
	grassTile.width = 96;
	grassTile.height = 96;
	const grass = grassTile.getContext('2d');
	if (grass) {
		px(grass, 0, 0, 96, 96, C.grass);
		for (let y = 0; y < 96; y += 3) {
			for (let x = 0; x < 96; x += 3) {
				const h = hash(x, y, 1);
				if (h < 0.07) px(grass, x, y, 1, 2, C.grassDark);
				else if (h < 0.11) px(grass, x + 1, y, 1, 2, C.grassLight);
				else if (h < 0.118) px(grass, x, y, 1, 1, C.grassPale);
				else if (h < 0.124) px(grass, x, y + 1, 2, 1, C.grassDeep);
			}
		}
	}
	ctx.fillStyle = (grass && ctx.createPattern(grassTile, 'repeat')) || C.grass;
	ctx.fillRect(left, top, right - left, bottom - top);

	// Forest to the north and north-west, thinning into the meadow.
	const forestEdge = (x: number) => {
		const r = x / W;
		const base = r < 0 ? 0.36 + r * 0.1 : r < 0.62 ? 0.02 - r * 0.2 : -0.14;
		return base * H + Math.sin(x * 0.045) * 10 + Math.sin(x * 0.013 + 2) * 14;
	};
	ctx.fillStyle = C.forestFloor;
	ctx.beginPath();
	ctx.moveTo(left, top);
	for (let x = left; x <= W * 0.64; x += 2) ctx.lineTo(x, Math.max(top, Math.round(forestEdge(x))));
	ctx.lineTo(W * 0.64, top);
	ctx.closePath();
	ctx.fill();
	const forestRand = mulberry(11);
	const trees: Array<[number, number, number, number]> = [];
	for (let y = top - 8; y < H * 0.5; y += 15) {
		for (let x = left - 8; x < W * 0.66; x += 17) {
			const tx = x + (forestRand() - 0.5) * 12;
			const ty = y + (forestRand() - 0.5) * 10;
			const edge = forestEdge(tx);
			const inside = ty < edge - 4;
			const fringe = !inside && ty < edge + 18 && forestRand() < 0.35;
			if (!(inside || fringe) || onPlate(tx, ty, 14) || !land(tx, ty)) continue;
			trees.push([tx, ty, 9 + forestRand() * 6, Math.floor(forestRand() * 10_000)]);
		}
	}
	trees.sort((a, b) => a[1] - b[1]);
	for (const [tx, ty, r, seed] of trees) stampTree(ctx, tx, ty, r, seed, false);

	// Lake: sand, wet sand, then three depths offset from the shore. Each band
	// is one filled shape following the shoreline; only the seams are per row.
	const band = (from: number, to: number | null, color: string) => {
		ctx.fillStyle = color;
		ctx.beginPath();
		ctx.moveTo(shore(top) + from, top);
		for (let y = top; y <= bottom; y += 1) ctx.lineTo(shore(y) + from, y);
		if (to === null) {
			ctx.lineTo(right, bottom);
			ctx.lineTo(right, top);
		} else for (let y = bottom; y >= top; y -= 1) ctx.lineTo(shore(y) + to, y);
		ctx.closePath();
		ctx.fill();
	};
	band(-13, null, C.sand);
	band(0, null, C.sandWet);
	band(3, null, C.shallow);
	band(22, null, C.mid);
	band(50, null, C.deep);
	band(120, null, C.deeper);
	for (let y = top; y < bottom; y += 1) {
		const s = shore(y);
		if (hash(s, y, 3) < 0.25)
			px(ctx, s - 13 + Math.floor(hash(y, s, 4) * 10), y, 1, 1, C.sandShade);
		// Soft transitions: dithered seams between depth bands.
		if (y % 2 === 0) {
			px(ctx, s + 21, y, 1, 1, C.mid);
			px(ctx, s + 49, y, 1, 1, C.deep);
			px(ctx, s + 119, y, 1, 1, C.deeper);
		} else {
			px(ctx, s + 22, y, 1, 1, C.shallow);
			px(ctx, s + 50, y, 1, 1, C.mid);
			px(ctx, s + 120, y, 1, 1, C.deep);
		}
	}
	// Still-water texture: faint long strokes in the deep, darker in the deepest.
	for (let y = top; y < bottom; y += 5) {
		for (let x = W * 0.6; x < right; x += 9) {
			const h = hash(x, y, 5);
			if (x < shore(y) + 30 || h > 0.18) continue;
			px(ctx, x, y, 4 + Math.floor(h * 30), 1, x > shore(y) + 120 ? C.deep : C.waterLine);
		}
	}
	// Pebbles on the beach, lilies in the shallows, reeds at the waterline.
	const lakeRand = mulberry(29);
	for (let y = top; y < bottom; y += 6) {
		const s = shore(y);
		if (lakeRand() < 0.5) px(ctx, s - 12 + lakeRand() * 9, y, 2, 1, C.pebble);
	}
	for (const [ry, count] of [
		[0.24, 5],
		[0.55, 4],
		[0.9, 6],
		[1.12, 4]
	] as Array<[number, number]>) {
		const y = ry * H;
		for (let i = 0; i < count; i += 1) {
			const ly = y + (lakeRand() - 0.5) * 30;
			const lx = shore(ly) + 6 + lakeRand() * 16;
			if (!onPlate(lx, ly, 6)) paintLily(ctx, lx, ly, i * 31 + count);
		}
	}
	for (let y = top + 20; y < bottom; y += 38) {
		if (lakeRand() < 0.55 && !onPlate(shore(y), y, 12)) paintReeds(ctx, shore(y) + 1, y, y);
	}
	for (const [ry, dx] of [
		[0.08, 46],
		[0.66, 58],
		[1.04, 38]
	] as Array<[number, number]>)
		disc(ctx, shore(ry * H) + dx, ry * H, 6, 3, C.waterDark);

	// A small hill in the north meadow between the two top pavilions: terraced
	// greens lit from the north-west, a winding path and a lone tree on top.
	// It goes wherever the layout leaves the most open meadow up north.
	let hill = { x: 0, y: 0, rx: 0, ry: 0 };
	for (let y = H * 0.1; y < H * 0.3; y += 8) {
		for (let x = W * 0.12; x < W * 0.66; x += 8) {
			if (!land(x + 80, y)) continue;
			let clearance = 0;
			while (clearance < 96 && !onPlate(x, y, clearance + 8)) clearance += 8;
			if (clearance > hill.rx + 14) hill = { x, y, rx: clearance - 14, ry: 0 };
		}
	}
	hill.rx = Math.min(64, hill.rx);
	hill.ry = hill.rx * 0.64;
	if (hill.rx >= 30) {
		disc(ctx, hill.x + 8, hill.y + 10, hill.rx + 4, hill.ry * 0.9, C.shadowSoft);
		disc(ctx, hill.x, hill.y, hill.rx, hill.ry, C.grassDeep);
		disc(ctx, hill.x - 4, hill.y - 4, hill.rx * 0.82, hill.ry * 0.78, C.grassDark);
		disc(ctx, hill.x - 8, hill.y - 8, hill.rx * 0.6, hill.ry * 0.56, C.grass);
		disc(ctx, hill.x - 12, hill.y - 12, hill.rx * 0.36, hill.ry * 0.32, C.grassLight);
		disc(ctx, hill.x - 16, hill.y - 15, hill.rx * 0.14, hill.ry * 0.12, C.grassPale);
		// Contour dither where each terrace steps down.
		for (let a = 0; a < Math.PI * 2; a += 0.05) {
			for (const k of [0.82, 0.6]) {
				const cx = hill.x - (k === 0.82 ? 4 : 8) + Math.cos(a) * hill.rx * k;
				const cy = hill.y - (k === 0.82 ? 4 : 8) + Math.sin(a) * hill.ry * k;
				if (Math.sin(a) > -0.2 && hash(Math.round(cx), Math.round(cy), 40) < 0.6)
					px(ctx, cx, cy, 1, 1, C.grassDeep);
			}
		}
		for (let t = 0; t < 1; t += 0.01) {
			const pxX = hill.x + 40 - t * 50 + Math.sin(t * 9) * 8;
			const pxY = hill.y + 34 - t * 44;
			px(ctx, pxX, pxY, 2, 1, C.gravel);
		}
		stampTree(ctx, hill.x + 6, hill.y - 20, 10, 3_311, true);
		paintFlowers(ctx, hill.x - 30, hill.y + 6, 91, 6);
		paintFlowers(ctx, hill.x + 26, hill.y + 16, 92, 5);
	}
	const molehills: CampusWorld['molehills'] = [];
	const moleRand = mulberry(59);
	for (let i = 0; i < 400 && molehills.length < 6; i += 1) {
		const x = W * 0.05 + moleRand() * W * 0.62;
		const y = moleRand() * H * 1.08;
		if (!land(x, y) || onPlate(x, y, 22)) continue;
		if (Math.hypot((x - hill.x) / hill.rx, (y - hill.y) / hill.ry) < 1.2) continue;
		if (molehills.some((m) => Math.hypot(m.x - x, m.y - y) < 60)) continue;
		disc(ctx, x + 1, y + 2, 5, 2, C.shadowSoft);
		disc(ctx, x, y, 4, 2.5, C.soilDark);
		disc(ctx, x - 0.5, y - 0.5, 3, 1.8, C.soil);
		px(ctx, x - 1, y - 2, 2, 1, '#b39c7a');
		molehills.push({ x: Math.round(x), y: Math.round(y), phase: moleRand() });
	}
	// The longest stretch of shoreline with no deck over its water: ducks and
	// leaping fish stay there, where they can be seen.
	const openShore = () => {
		let best = { top: 0, bottom: 0 };
		let start: number | null = null;
		for (let y = 0; y <= H * 1.1; y += 4) {
			const open = !onPlate(world_shore(y) + 30, y, 16) && !onPlate(world_shore(y) + 70, y, 16);
			if (open && start === null) start = y;
			if ((!open || y + 4 > H * 1.1) && start !== null) {
				if (y - start > best.bottom - best.top) best = { top: start + 8, bottom: y - 8 };
				start = null;
			}
		}
		return best;
	};
	const world_shore = shore;
	let heron: CampusWorld['heron'] = null;
	for (const ry of [0.3, 0.55, 0.8]) {
		const y = Math.round(H * ry);
		const x = shore(y) + 7;
		if (!onPlate(x, y, 18)) {
			heron = { x, y };
			paintReeds(ctx, x - 6, y + 3, 71);
			break;
		}
	}

	// Meadow life: trees, rocks and flowers where no plate or water is.
	const meadowRand = mulberry(47);
	const meadowTrees: Array<[number, number, number, number]> = [];
	for (let i = 0; i < 70; i += 1) {
		const x = left + meadowRand() * (right - left);
		const y = H * 0.45 + meadowRand() * (bottom - H * 0.45);
		if (!land(x, y) || onPlate(x, y, 30) || y < forestEdge(x) + 20) continue;
		meadowTrees.push([x, y, 8 + meadowRand() * 5, Math.floor(meadowRand() * 10_000)]);
	}
	for (let i = 0; i < 26; i += 1) {
		const x = left + meadowRand() * (W * 0.1 - left);
		const y = top + meadowRand() * (bottom - top);
		if (!land(x, y) || onPlate(x, y, 30) || y < forestEdge(x) + 16) continue;
		meadowTrees.push([x, y, 8 + meadowRand() * 5, Math.floor(meadowRand() * 10_000)]);
	}
	// The meadow pockets between pavilions: a few trees and flowers, so
	// nature runs through the campus rather than only around it.
	for (let i = 0; i < 120; i += 1) {
		const x = meadowRand() * W;
		const y = meadowRand() * H;
		if (!land(x, y) || onPlate(x, y, 22)) continue;
		if (i % 4 === 0)
			meadowTrees.push([x, y, 7 + meadowRand() * 3, Math.floor(meadowRand() * 10_000)]);
		else if (i % 4 === 1) paintFlowers(ctx, x, y, i * 17 + 3, 6);
	}
	for (let i = 0; i < 60; i += 1) {
		const x = left + meadowRand() * (right - left);
		const y = top + meadowRand() * (bottom - top);
		if (!land(x, y) || onPlate(x, y, 16) || y < forestEdge(x) + 10) continue;
		if (i % 3 === 0) paintRock(ctx, x, y, 3 + meadowRand() * 3, i % 2 === 0);
		else paintFlowers(ctx, x, y, i * 13 + 5, 5 + Math.floor(meadowRand() * 5));
	}

	// A gravel path leaves the south terrace and wanders to the beach.
	const path = (t: number): [number, number] => {
		const x0 = W * 0.44;
		const y0 = H + 2;
		const x1 = W * 0.5;
		const y1 = H * 1.22;
		const x2 = shore(H * 1.28) - 8;
		const y2 = H * 1.28;
		const u = 1 - t;
		return [u * u * x0 + 2 * u * t * x1 + t * t * x2, u * u * y0 + 2 * u * t * y1 + t * t * y2];
	};
	for (let t = 0; t <= 1; t += 0.004) {
		const [x, y] = path(t);
		disc(ctx, x, y, 5, 4, C.gravelEdge);
	}
	for (let t = 0; t <= 1; t += 0.004) {
		const [x, y] = path(t);
		disc(ctx, x, y, 4, 3, C.gravel);
		if (hash(Math.round(x), Math.round(y), 7) < 0.3) px(ctx, x + 2, y - 1, 1, 1, C.gravelDot);
	}

	meadowTrees.sort((a, b) => a[1] - b[1]);
	for (const [x, y, r, seed] of meadowTrees) stampTree(ctx, x, y, r, seed, true);

	// Where the pavilions meet the ground: a soft cast shadow, a light lip,
	// and garden beds along the land-facing edges so work sits in nature.
	for (let row = 0; row < source.rows; row += 1) {
		for (let col = 0; col < source.cols; col += 1) {
			if (!solid(col, row)) continue;
			const x = col * TILE_SIZE;
			const y = row * TILE_SIZE;
			const T = TILE_SIZE;
			// A soft contact shadow, tinted by the ground it falls on; no outline,
			// so the paving settles into the meadow instead of sitting on it.
			if (!solid(col + 1, row)) {
				px(ctx, x + T, y + 2, 1, T, C.plateShadow);
				px(ctx, x + T + 1, y + 3, 2, T, C.plateShadowSoft);
			}
			if (!solid(col, row + 1)) {
				px(ctx, x + 2, y + T, T, 1, C.plateShadow);
				px(ctx, x + 3, y + T + 1, T, 2, C.plateShadowSoft);
			}
			// Planting along the land-facing edges: clumps of shrubs and
			// flowers at irregular intervals, and now and then a clipped hedge.
			if (!solid(col, row + 1) && !solid(col, row + 2) && land(x + T / 2, y + T + 8)) {
				const kind = hash(col, row, 10);
				if (kind < 0.18) {
					disc(ctx, x + T / 2 + 1, y + T + 10, 10, 3, C.shadowSoft);
					px(ctx, x - 2, y + T + 5, T + 4, 5, C.hedgeDark);
					px(ctx, x - 1, y + T + 4, T + 2, 4, C.hedge);
					px(ctx, x + 1, y + T + 4, T - 4, 1, C.hedgeLight);
				} else if (kind < 0.42)
					paintShrub(ctx, x + 4 + hash(col, row, 11) * 8, y + T + 7, 3.5 + hash(col, row, 12) * 2);
				else if (kind < 0.58) paintFlowers(ctx, x + 8, y + T + 8, col * 7 + row, 5);
			}
			if (!solid(col - 1, row) && !solid(col - 2, row) && land(x - 10, y + 8)) {
				if (hash(col, row, 12) < 0.22) paintShrub(ctx, x - 7, y + 8, 3 + hash(col, row, 13) * 2);
				else if (hash(col, row, 14) < 0.12) paintFlowers(ctx, x - 8, y + 8, col + row * 5, 4);
			}
		}
	}

	// Motion seeds.
	const motionRand = mulberry(83);
	const ripples: CampusWorld['ripples'] = [];
	for (let i = 0; i < 90; i += 1) {
		const y = Math.round(top + motionRand() * (bottom - top));
		const x = Math.round(shore(y) + 28 + motionRand() * (right - shore(y) - 60));
		if (onPlate(x, y, 10)) continue;
		ripples.push({
			x,
			y,
			len: 4 + Math.round(motionRand() * 9),
			speed: 2 + motionRand() * 3,
			period: 6 + motionRand() * 6,
			phase: motionRand()
		});
	}
	const glints: CampusWorld['glints'] = [];
	for (let i = 0; i < 14; i += 1) {
		const y = Math.round(top + motionRand() * (bottom - top));
		const x = Math.round(shore(y) + 40 + motionRand() * (right - shore(y) - 60));
		if (onPlate(x, y, 10)) continue;
		glints.push({ x, y, period: 3 + motionRand() * 5, phase: motionRand() });
	}
	const leaves: CampusWorld['leaves'] = [];
	for (let i = 0; i < 7; i += 1)
		leaves.push({
			x: left + motionRand() * W,
			y: forestEdge(W * 0.3) + 10 + motionRand() * 60,
			speed: 8 + motionRand() * 6,
			drift: 3 + motionRand() * 4,
			phase: motionRand()
		});

	return {
		molehills,
		heron,
		shoreRows: openShore(),
		W,
		H,
		canvas,
		originX: MARGIN_X,
		originY: MARGIN_Y,
		width: canvas.width,
		height: canvas.height,
		shore,
		ripples,
		glints,
		leaves
	};
}

/* The campus depends only on the plates' shape. Keep the last one painted
 * across mounts, so returning to Attention does not repaint it. */
let lastWorld: { key: string; world: CampusWorld | null } | null = null;

export function campusWorldFor(plan: { layout: CampusSource }): CampusWorld | null {
	const key = `${plan.layout.cols}x${plan.layout.rows}:${plan.layout.tiles.join('')}`;
	if (lastWorld?.key === key) return lastWorld.world;
	lastWorld = { key, world: buildCampusWorld(plan.layout) };
	return lastWorld.world;
}

/** Identity of everything the cached floor is painted from. */
export function layoutKey(plan: { signature: string }): string {
	return plan.signature;
}

export interface CampusView {
	offsetX: number;
	offsetY: number;
	zoom: number;
	canvasWidth: number;
	canvasHeight: number;
	now: number;
	motion: boolean;
}

/** Map one office-local art pixel rect to the screen. */
function screen(
	ctx: Paint,
	view: CampusView,
	x: number,
	y: number,
	w: number,
	h: number,
	color: string
) {
	const z = view.zoom;
	const sx = view.offsetX + Math.round(x) * z;
	const sy = view.offsetY + Math.round(y) * z;
	if (sx > view.canvasWidth || sy > view.canvasHeight || sx + w * z < 0 || sy + h * z < 0) return;
	ctx.fillStyle = color;
	ctx.fillRect(sx, sy, Math.max(1, Math.round(w)) * z, Math.max(1, Math.round(h)) * z);
}

/** Everything beneath the office: the painted world plus living water. */
export function drawCampusBackground(ctx: Paint, world: CampusWorld, view: CampusView): void {
	const z = view.zoom;
	const worldLeft = view.offsetX - world.originX * z;
	const worldTop = view.offsetY - world.originY * z;
	// Beyond the painted world: meadow to the west, lake to the east.
	ctx.fillStyle = C.grass;
	ctx.fillRect(0, 0, view.canvasWidth, view.canvasHeight);
	const lakeFrom = worldLeft + (world.originX + world.shore(0) + 60) * z;
	ctx.fillStyle = C.deeper;
	ctx.fillRect(lakeFrom, 0, view.canvasWidth - lakeFrom, view.canvasHeight);
	const smoothing = ctx.imageSmoothingEnabled;
	ctx.imageSmoothingEnabled = false;
	blitVisible(ctx, world.canvas, worldLeft, worldTop, z);
	ctx.imageSmoothingEnabled = smoothing;
	drawWater(ctx, world, view);
	drawGroundLife(ctx, world, view);
}

/* Creatures that live at ground and water level: moles, ducks, a heron and
 * leaping fish. Each is a few pixels, deterministic in time, and rare enough
 * to stay calm. */
function drawGroundLife(ctx: Paint, world: CampusWorld, view: CampusView): void {
	if (!view.motion) {
		if (world.heron) drawHeron(ctx, view, world.heron.x, world.heron.y, false);
		return;
	}
	const t = view.now / 1000;
	// Moles: every 16s or so each one peeks out for two seconds.
	for (const hill of world.molehills) {
		const cycle = (t / 16 + hill.phase) % 1;
		if (cycle > 0.13) continue;
		const rise = Math.min(1, cycle * 40, (0.13 - cycle) * 40);
		const up = Math.round(rise * 3);
		if (up <= 0) continue;
		screen(ctx, view, hill.x - 2, hill.y - 1 - up, 4, up + 1, '#4a4038');
		screen(ctx, view, hill.x - 1, hill.y - 1 - up, 2, 1, '#6a5a4e');
		if (up >= 2) {
			screen(ctx, view, hill.x + 2, hill.y - up, 1, 1, '#e8a0a8');
			screen(ctx, view, hill.x, hill.y - up, 1, 1, '#1d1a18');
		}
		screen(ctx, view, hill.x - 3, hill.y, 6, 1, C.soil);
	}
	// Ducks paddle slowly along the shore and turn back, leaving a small wake.
	for (let i = 0; i < 3; i += 1) {
		const span = world.shoreRows.bottom - world.shoreRows.top;
		const travel = (t * 3 + i * 170) % (span * 2);
		const along = travel < span ? travel : span * 2 - travel;
		const heading = travel < span ? 1 : -1;
		const y = world.shoreRows.top + along;
		const x = world.shore(y) + 30 + i * 9 + Math.sin(t * 0.7 + i) * 3;
		const bob = Math.floor(t * 2 + i) % 2;
		screen(ctx, view, x - 3, y + 2 - heading * 3, 1, 1, C.ripple);
		screen(ctx, view, x + 3, y + 2 - heading * 3, 1, 1, C.ripple);
		screen(ctx, view, x - 2, y - 1 + bob, 5, 3, i === 0 ? '#7b5c3e' : '#f1ede3');
		screen(
			ctx,
			view,
			x - 1,
			y - 2 + bob + (heading > 0 ? 3 : -1),
			3,
			2,
			i === 0 ? '#2f6b4f' : '#e8e2d5'
		);
		screen(ctx, view, x, y - 2 + bob + (heading > 0 ? 5 : -2), 1, 1, '#e6a23c');
	}
	// A fish leaps somewhere near the shore every nine seconds.
	const leap = Math.floor(t / 9);
	const phase = (t % 9) / 1.4;
	if (phase <= 1.6) {
		const span = world.shoreRows.bottom - world.shoreRows.top;
		const y = world.shoreRows.top + Math.floor(hash(leap, 1, 50) * span);
		const x = world.shore(y) + 40 + Math.floor(hash(leap, 2, 50) * 140);
		if (phase <= 1) {
			const arcX = x + Math.round(phase * 10);
			const arcY = y - Math.round(Math.sin(phase * Math.PI) * 7);
			screen(ctx, view, arcX, arcY, 3, 1, '#d9e4e6');
			screen(ctx, view, arcX + 1, arcY - 1, 1, 1, '#9fb4bb');
			if (phase < 0.2) screen(ctx, view, x - 1, y, 3, 1, C.foam);
		} else {
			const r = Math.round((phase - 1) * 10) + 2;
			const cx = x + 10;
			screen(ctx, view, cx - r, y, 2, 1, C.foamDim);
			screen(ctx, view, cx + r - 1, y, 2, 1, C.foamDim);
			screen(ctx, view, cx - 1, y - Math.ceil(r / 2), 2, 1, C.foamDim);
			screen(ctx, view, cx - 1, y + Math.ceil(r / 2), 2, 1, C.foamDim);
		}
	}
	if (world.heron) {
		const dip = t % 11 < 1.2;
		drawHeron(ctx, view, world.heron.x, world.heron.y, dip);
	}
}

function drawHeron(ctx: Paint, view: CampusView, x: number, y: number, dip: boolean): void {
	screen(ctx, view, x - 1, y + 3, 5, 1, C.waterDark);
	screen(ctx, view, x, y - 4, 1, 7, '#7c8a92');
	screen(ctx, view, x + 2, y - 4, 1, 7, '#7c8a92');
	screen(ctx, view, x - 1, y - 9, 5, 5, '#b9c3c8');
	screen(ctx, view, x, y - 9, 3, 1, '#dde4e6');
	if (dip) {
		screen(ctx, view, x + 4, y - 7, 1, 4, '#b9c3c8');
		screen(ctx, view, x + 5, y - 3, 2, 1, '#d8a44a');
	} else {
		screen(ctx, view, x + 3, y - 14, 1, 5, '#b9c3c8');
		screen(ctx, view, x + 3, y - 16, 2, 2, '#dde4e6');
		screen(ctx, view, x + 5, y - 15, 3, 1, '#d8a44a');
		screen(ctx, view, x + 3, y - 16, 1, 1, '#1d1a18');
	}
}

function drawWater(ctx: Paint, world: CampusWorld, view: CampusView): void {
	const t = view.motion ? view.now / 1000 : 0;
	const z = view.zoom;
	const firstRow = Math.floor(-view.offsetY / z) - 2;
	const lastRow = Math.ceil((view.canvasHeight - view.offsetY) / z) + 2;
	// Foam: a continuous line on the waterline. A slow swell travels along the
	// shore and moves it one pixel in or out; nothing blinks.
	for (let y = firstRow; y <= lastRow; y += 1) {
		const s = world.shore(y);
		const swell = Math.sin(t * 0.45 - y * 0.035);
		const reach = swell > 0.55 ? 1 : swell < -0.55 ? -1 : 0;
		screen(ctx, view, s + reach, y, 2, 1, C.foamDim);
		if (reach >= 0) screen(ctx, view, s + reach + 1, y, 1, 1, C.foam);
	}
	// Ripples drift slowly east, always visible, one art pixel at a time.
	for (const ripple of world.ripples) {
		if (ripple.y < firstRow || ripple.y > lastRow) continue;
		const x = ripple.x + Math.floor((t * ripple.speed * 0.4 + ripple.phase * 40) % 40);
		if (x < world.shore(ripple.y) + 26) continue;
		screen(ctx, view, x, ripple.y, ripple.len, 1, C.rippleDim);
	}
	// Sun glints: rare, soft single pixels.
	for (const glint of world.glints) {
		if (glint.y < firstRow || glint.y > lastRow) continue;
		const life = (t / (glint.period * 2) + glint.phase) % 1;
		if (life > 0.05) continue;
		screen(ctx, view, glint.x, glint.y, 1, 1, C.glintArm);
	}
}

/** Over the office: drifting leaves and the occasional wildlife moment. */
export function drawCampusOverlay(
	ctx: Paint,
	world: CampusWorld,
	view: CampusView,
	officeCols: number,
	officeRows: number
): void {
	if (!view.motion) return;
	const t = view.now / 1000;
	const W = officeCols * TILE_SIZE;
	const H = officeRows * TILE_SIZE;
	for (const leaf of world.leaves) {
		const run = (t * leaf.speed + leaf.phase * 900) % (W * 1.4);
		const x = leaf.x + run;
		const y = leaf.y + run * 0.18 + Math.sin(t * 1.3 + leaf.phase * 9) * leaf.drift;
		const flip = Math.sin(t * 2.1 + leaf.phase * 7) > 0;
		screen(ctx, view, x, y, flip ? 2 : 1, 1, leaf.phase > 0.5 ? C.leafGold : C.leafGreen);
	}
	// Two gulls wheel slowly over the lake; their shadows fall on the water.
	for (let i = 0; i < 2; i += 1) {
		const a = t * (0.16 + i * 0.05) + i * 2.4;
		const cx = world.shore(H * 0.4) + 150 + i * 70;
		const cy = H * (0.35 + i * 0.35);
		const x = cx + Math.cos(a) * (60 + i * 20);
		const y = cy + Math.sin(a) * (34 + i * 10);
		const wing = Math.floor(t * 3 + i) % 3 === 0 ? 1 : 0;
		screen(ctx, view, x + 6, y + 14, 3, 1, C.shadowSoft);
		screen(ctx, view, x, y, 1, 1, '#f5f7f7');
		screen(ctx, view, x - 3, y - wing, 3, 1, '#e6ecee');
		screen(ctx, view, x + 1, y - wing, 3, 1, '#e6ecee');
		screen(ctx, view, x - 3, y - wing, 1, 1, '#5b6770');
		screen(ctx, view, x + 3, y - wing, 1, 1, '#5b6770');
	}
	// Two butterflies drift between flowers in the meadow pockets.
	for (let i = 0; i < 2; i += 1) {
		const bx = W * (0.5 + i * 0.12) + Math.sin(t * 0.45 + i * 3) * 40 + Math.sin(t * 1.7 + i) * 6;
		const by = H * (i === 0 ? 0.2 : 0.9) + Math.cos(t * 0.38 + i * 2) * 20;
		const flutter = Math.floor(t * 9 + i) % 2;
		screen(ctx, view, bx, by, 1, 2, '#4a5463');
		screen(ctx, view, bx - 1 - flutter, by, 1 + flutter, 1, i === 0 ? '#e7a6c4' : '#b9cdf0');
		screen(ctx, view, bx + 1, by, 1 + flutter, 1, i === 0 ? '#f1cf7c' : '#f7f3e9');
	}
	const wildlife = campusWildlifeAt(Date.now());
	if (!wildlife) return;
	const p = wildlife.progress;
	if (wildlife.kind === 'birds') {
		const wing = Math.floor(t * 6) % 2;
		for (const [dx, dy] of [
			[0, 0],
			[-14, 9],
			[-26, -6]
		]) {
			const x = W * 0.06 + p * W * 0.5 + dx;
			const y = -H * 0.02 + p * H * 0.08 + dy;
			screen(ctx, view, x + 1, y + 6, 3, 1, C.shadowSoft);
			screen(ctx, view, x, y, 2, 2, '#44566a');
			screen(ctx, view, x - 3, y + wing, 3, 1, '#566a7c');
			screen(ctx, view, x + 2, y + wing, 3, 1, '#566a7c');
		}
	} else if (wildlife.kind === 'butterfly') {
		const x = W * 0.46 + p * W * 0.16;
		const y = H * 1.03 + Math.sin(p * Math.PI * 4) * 4;
		const flutter = Math.floor(t * 8) % 2;
		screen(ctx, view, x, y, 1, 2, '#4a5463');
		screen(ctx, view, x - 1 - flutter, y, 1 + flutter, 1, '#e7a6c4');
		screen(ctx, view, x + 1, y, 1 + flutter, 1, '#f1cf7c');
	} else {
		// The whale surfaces far out on the lake, breathes once and sinks.
		const x = world.shore(H * 0.2) + 190 + p * 12;
		const y = H * 0.2 + p * 20;
		const rise = Math.min(1, p * 5, (1 - p) * 5);
		if (rise > 0.25) {
			screen(ctx, view, x - 9, y - 3, 20, 7, C.waterDark);
			screen(ctx, view, x - 7, y - 4, 15, 2, '#6e9ea8');
			screen(ctx, view, x - 12, y - 1, 3, 1, C.foam);
			screen(ctx, view, x + 12, y + 1, 3, 1, C.foam);
		}
		if (p > 0.3 && p < 0.6) {
			const spray = Math.floor((p - 0.3) * 30);
			screen(ctx, view, x, y - 6 - spray, 1, 3, C.foam);
			screen(ctx, view, x - 2, y - 8 - spray, 1, 1, C.foamDim);
			screen(ctx, view, x + 2, y - 8 - spray, 1, 1, C.foamDim);
		}
	}
}
