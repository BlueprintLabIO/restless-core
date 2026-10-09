import { TILE_SIZE } from '$lib/vendor/pixel-agents/webview-ui/src/office/types.js';
import {
	blitVisible,
	type WorldLayer
} from '$lib/vendor/pixel-agents/webview-ui/src/office/engine/blit.js';

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
	duneShade: '#d6ca9b',
	marram: '#8fae6a',
	umbrellas: [
		['#e8594a', '#f7f1e3'],
		['#3f8fd2', '#f7f1e3'],
		['#f2c14e', '#e8594a'],
		['#5fb38a', '#f7f1e3'],
		['#e889b5', '#f7f1e3']
	] as const,
	towels: ['#e8594a', '#3f8fd2', '#f2c14e', '#5fb38a', '#9a7fd1', '#f29c4c'] as const,
	huts: ['#e46a6a', '#6aa9e4', '#f2c94c', '#7cc4a0', '#c99be0'] as const,
	hutRoof: '#5a6470',
	castle: '#d8c48e',
	castleTop: '#e9daa9',
	lifeguard: '#d24a3c',
	court: '#f6f2e6',
	net: '#4a5463',
	stoneRing: '#9a9d96',
	ember: '#f08a3c',
	emberHot: '#f7c95a',
	shell: '#f6ebe2',
	shellShade: '#e3c6b4',
	starfish: '#f08a5d',
	crab: '#d9533f',
	crabDark: '#a93c2e',
	board: ['#3fb3b0', '#f29c4c', '#f7f1e3', '#e8594a'] as const,
	marramDark: '#6f9254',
	plank: '#b48d60',
	plankLight: '#cfab7c',
	plankDark: '#8c6a45',
	post: '#6d5640',
	rope: '#e3d6b4',
	hull: '#7a5a3c',
	hullInside: '#c99c63',
	sail: '#fbfaf6',
	sailShade: '#dfe4e2',
	buoy: '#ec8a4a',
	buoyLight: '#f7c08f',
	skin: '#e2b48f',
	shirt: '#3f7fb3',
	shirtAlt: '#d0644e',
	hat: '#e9d27a',
	fern: '#5d9a5b',
	cliff: '#b3ad9c',
	highMeadow: '#b2d98f',
	highMeadowLight: '#cde8a8',
	highMeadowDark: '#97c47a',
	cliffLight: '#cdc8b8',
	cliffDark: '#8f8a7c',
	fernLight: '#7fb873',
	pine: '#3d6d50',
	pineMid: '#4c8160',
	pineLight: '#63996f',
	birchBark: '#efece2',
	birchMark: '#5a5650',
	birchLeaf: '#9cc77a',
	birchLeafLight: '#bcdb92',
	birchLeafDark: '#7aab64',
	berry: '#c8414b',
	capRed: '#d2533f',
	capWhite: '#f4eee2',
	driftwood: '#a89a85',
	driftwoodDark: '#857865',
	flowerCentre: '#f8e8a0',
	/* Each meadow keeps one pair of colours, as wildflowers do. */
	meadows: [
		['#b49ad8', '#f5f2e8'],
		['#e3604f', '#f2d24b'],
		['#f0a6c0', '#f5f2e8'],
		['#7fa6e0', '#f5f2e8'],
		['#f2d24b', '#f29c4c']
	] as const,
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

/** Smooth value noise in [0, 1]: nature clusters where this is high and thins where it is low. */
function field(x: number, y: number, scale: number, salt: number): number {
	const fx = x * scale;
	const fy = y * scale;
	const x0 = Math.floor(fx);
	const y0 = Math.floor(fy);
	const ease = (t: number) => t * t * (3 - 2 * t);
	const tx = ease(fx - x0);
	const ty = ease(fy - y0);
	const a = hash(x0, y0, salt);
	const b = hash(x0 + 1, y0, salt);
	const c = hash(x0, y0 + 1, salt);
	const d = hash(x0 + 1, y0 + 1, salt);
	const top = a + (b - a) * tx;
	const bottom = c + (d - c) * tx;
	const coarse = top + (bottom - top) * ty;
	// A second, finer octave keeps the edges of each patch from looking drawn with a compass.
	const fine = (() => {
		const gx = fx * 2.3 + 17;
		const gy = fy * 2.3 + 31;
		const g0 = Math.floor(gx);
		const h0 = Math.floor(gy);
		const ux = ease(gx - g0);
		const uy = ease(gy - h0);
		const e = hash(g0, h0, salt + 1);
		const f = hash(g0 + 1, h0, salt + 1);
		const g = hash(g0, h0 + 1, salt + 1);
		const h = hash(g0 + 1, h0 + 1, salt + 1);
		return e + (f - e) * ux + (g + (h - g) * ux - (e + (f - e) * ux)) * uy;
	})();
	return coarse * 0.72 + fine * 0.28;
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

type Paint = CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D;
type Surface = HTMLCanvasElement | OffscreenCanvas;

/** A blank canvas: in the page, or off the main thread in the campus worker. */
function makeSurface(width: number, height: number): Surface {
	if (typeof document === 'undefined') return new OffscreenCanvas(width, height);
	const canvas = document.createElement('canvas');
	canvas.width = width;
	canvas.height = height;
	return canvas;
}

function paintOn(surface: Surface): Paint | null {
	return surface.getContext('2d') as Paint | null;
}

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
const treeSprites = new Map<string, { canvas: Surface; ox: number; oy: number }>();

function stampTree(ctx: Paint, x: number, y: number, r: number, seed: number, trunk: boolean) {
	const size = Math.round(r);
	const variant = seed % 7;
	const key = `${size}:${variant}:${trunk}`;
	let sprite = treeSprites.get(key);
	if (!sprite) {
		const pad = Math.ceil(size * 1.6) + 4;
		const canvas = makeSurface(pad * 2, pad * 2);
		const layer = paintOn(canvas);
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

/** A drift of wildflowers in one meadow's colours, denser at its heart. */
function paintWildflowers(
	ctx: Paint,
	x: number,
	y: number,
	seed: number,
	palette: readonly string[]
) {
	const rand = mulberry(seed);
	const a = palette[0];
	const b = palette[1];
	const count = 8 + Math.floor(rand() * 14);
	const rx = 6 + rand() * 9;
	const ry = 3 + rand() * 3;
	for (let i = 0; i < count; i += 1) {
		const r = Math.sqrt(rand());
		const t = rand() * Math.PI * 2;
		const fx = x + Math.cos(t) * r * rx;
		const fy = y + Math.sin(t) * r * ry;
		px(ctx, fx, fy + 1, 1, 2, C.grassDeep);
		const big = rand() < 0.45;
		px(ctx, fx, fy, big ? 2 : 1, big ? 2 : 1, rand() < 0.6 ? a : b);
		if (big) px(ctx, fx, fy, 1, 1, C.flowerCentre);
	}
}

/** A clump of tall grass, a few blades carrying seed heads. */
function paintTallGrass(ctx: Paint, x: number, y: number, seed: number) {
	const rand = mulberry(seed);
	const blades = 8 + Math.floor(rand() * 8);
	disc(ctx, x + 1, y + 1, 5, 1.5, C.shadowSoft);
	for (let i = 0; i < blades; i += 1) {
		const bx = x + Math.round((rand() - 0.5) * 10);
		const h = 4 + Math.round(rand() * 5);
		px(ctx, bx, y - h, 1, h, rand() < 0.5 ? C.grassDeep : C.grassDark);
		if (rand() < 0.3) px(ctx, bx, y - h - 1, 1, 1, C.grassPale);
	}
}

/** A fern: fronds fanning from one root. */
function paintFern(ctx: Paint, x: number, y: number, seed: number) {
	const rand = mulberry(seed);
	disc(ctx, x + 1, y + 2, 6, 2, C.shadowSoft);
	for (let f = 0; f < 5; f += 1) {
		const a = -Math.PI * (0.15 + f * 0.175);
		const len = 5 + rand() * 3;
		for (let k = 1; k <= len; k += 1) {
			const fx = x + Math.cos(a) * k;
			const fy = y + Math.sin(a) * k * 0.7;
			px(ctx, fx, fy, 1, 1, k % 2 ? C.fern : C.fernLight);
			if (k > 1 && k < len) px(ctx, fx + (Math.cos(a) > 0 ? 1 : -1), fy + 1, 1, 1, C.fern);
		}
	}
}

/** A conifer from above-and-south: stacked tiers, lit from the north-west. */
function paintConifer(ctx: Paint, x: number, y: number, r: number) {
	disc(ctx, x + r * 0.4, y + r * 0.9, r * 0.9, r * 0.45, C.shadow);
	px(ctx, x - 1, y + r * 0.6, 2, r * 0.5, C.trunk);
	const tiers = 3;
	for (let t = 0; t < tiers; t += 1) {
		const ty = y - r * 0.9 + t * r * 0.55;
		const w = r * (0.45 + t * 0.28);
		for (let row = 0; row < r * 0.7; row += 1) {
			const half = Math.round((row / (r * 0.7)) * w);
			px(ctx, x - half, ty + row, half * 2 + 1, 1, C.pine);
			px(ctx, x - half, ty + row, Math.max(1, half), 1, C.pineMid);
		}
		px(ctx, x - Math.round(w * 0.5), ty + r * 0.45, 2, 1, C.pineLight);
	}
}

/** A birch: a pale marked trunk under a light, open crown. */
function paintBirch(ctx: Paint, x: number, y: number, r: number, seed: number) {
	const rand = mulberry(seed);
	disc(ctx, x + r * 0.35, y + r * 0.7, r * 0.9, r * 0.45, C.shadowSoft);
	px(ctx, x - 1, y, 2, r * 0.9, C.birchBark);
	for (let k = 1; k < r * 0.9; k += 3) px(ctx, x - (k % 2), y + k, 1, 1, C.birchMark);
	for (let i = 0; i < 6; i += 1) {
		const cx = x + (rand() - 0.5) * r * 1.2;
		const cy = y - r * 0.35 + (rand() - 0.5) * r * 0.8;
		disc(ctx, cx, cy, r * 0.42, r * 0.36, i < 2 ? C.birchLeafDark : C.birchLeaf);
	}
	disc(ctx, x - r * 0.3, y - r * 0.55, r * 0.3, r * 0.22, C.birchLeafLight);
}

/** A low bush heavy with red berries. */
function paintBerryBush(ctx: Paint, x: number, y: number, seed: number) {
	const rand = mulberry(seed);
	paintShrub(ctx, x, y, 4 + rand() * 2);
	for (let i = 0; i < 5; i += 1)
		px(ctx, x + (rand() - 0.5) * 7, y + (rand() - 0.6) * 4, 1, 1, C.berry);
}

/** Two or three toadstools at the forest's edge. */
function paintMushrooms(ctx: Paint, x: number, y: number, seed: number) {
	const rand = mulberry(seed);
	const n = 2 + Math.floor(rand() * 2);
	for (let i = 0; i < n; i += 1) {
		const mx = x + i * 3 + Math.round(rand() * 2);
		const my = y + Math.round(rand() * 2);
		px(ctx, mx, my, 1, 2, C.capWhite);
		px(ctx, mx - 1, my - 1, 3, 1, C.capRed);
		px(ctx, mx, my - 1, 1, 1, C.capWhite);
	}
}

/** A beach umbrella from above: alternating panels, a darker lee rim and a soft shadow. */
function paintUmbrella(ctx: Paint, x: number, y: number, colors: readonly [string, string]) {
	disc(ctx, x + 4, y + 5, 9, 4.5, C.shadowSoft);
	for (let dy = -7; dy <= 7; dy += 1)
		for (let dx = -9; dx <= 9; dx += 1) {
			const k = (dx * dx) / 81 + (dy * dy) / 49;
			if (k >= 1) continue;
			const sector = Math.floor(((Math.atan2(dy, dx) + Math.PI) / (Math.PI / 4)) % 8);
			px(ctx, x + dx, y + dy, 1, 1, colors[sector % 2]);
			if (k > 0.72 && dy + dx * 0.4 > 1) px(ctx, x + dx, y + dy, 1, 1, 'rgba(40, 40, 50, 0.16)');
		}
	px(ctx, x - 1, y - 1, 2, 2, '#ffffff');
}

/** A striped towel laid on the sand, sometimes with someone sunbathing on it. */
function paintTowel(
	ctx: Paint,
	x: number,
	y: number,
	color: string,
	sunbather: boolean,
	seed: number
) {
	const rand = mulberry(seed);
	px(ctx, x + 1, y + 1, 7, 13, 'rgba(120, 100, 60, 0.18)');
	px(ctx, x, y, 7, 13, color);
	px(ctx, x, y + 3, 7, 1, '#f7f1e3');
	px(ctx, x, y + 9, 7, 1, '#f7f1e3');
	if (!sunbather) return;
	const suit = C.towels[Math.floor(rand() * C.towels.length)];
	px(ctx, x + 2, y + 1, 3, 3, C.skin); // head
	px(ctx, x + 2, y + 1, 3, 1, rand() < 0.5 ? '#5a4636' : '#e0c27a');
	px(ctx, x + 1, y + 4, 5, 4, C.skin);
	px(ctx, x + 2, y + 6, 3, 2, suit);
	px(ctx, x + 2, y + 8, 1, 4, C.skin);
	px(ctx, x + 4, y + 8, 1, 4, C.skin);
}

/** A sandcastle in its moat, with a bucket and spade beside it. */
function paintSandcastle(ctx: Paint, x: number, y: number) {
	disc(ctx, x, y + 2, 11, 6, C.sandWet);
	disc(ctx, x, y + 1, 8, 4, C.sand);
	px(ctx, x - 6, y - 2, 12, 5, C.castle);
	px(ctx, x - 6, y - 2, 12, 1, C.castleTop);
	for (const tx of [-7, -1, 5]) {
		px(ctx, x + tx, y - 6, 3, 6, C.castle);
		px(ctx, x + tx, y - 6, 3, 1, C.castleTop);
		px(ctx, x + tx, y - 7, 1, 1, C.castle);
		px(ctx, x + tx + 2, y - 7, 1, 1, C.castle);
	}
	px(ctx, x - 1, y, 2, 3, C.sandShade); // gate
	px(ctx, x, y - 10, 1, 4, C.trunk);
	px(ctx, x + 1, y - 10, 3, 2, C.lifeguard);
	px(ctx, x + 11, y + 1, 4, 4, '#e3604f');
	px(ctx, x + 11, y + 1, 4, 1, '#f08a7e');
	px(ctx, x + 16, y - 2, 1, 6, '#4a90c8');
	px(ctx, x + 15, y + 3, 3, 2, '#4a90c8');
}

/** A lifeguard's tower facing the water: stilts, a white cabin with a red band, a flag. */
function paintLifeguardTower(ctx: Paint, x: number, y: number) {
	disc(ctx, x + 6, y + 9, 11, 4, C.shadowSoft);
	for (const [dx, dy] of [
		[-5, 4],
		[5, 4],
		[-5, 9],
		[5, 9]
	])
		px(ctx, x + dx, y + dy - 5, 1, 6, C.post);
	px(ctx, x - 6, y - 3, 13, 9, '#f1ede4');
	px(ctx, x - 6, y + 1, 13, 2, C.lifeguard);
	px(ctx, x - 7, y - 6, 15, 4, C.lifeguard);
	px(ctx, x - 7, y - 6, 15, 1, '#e8705f');
	px(ctx, x + 6, y + 6, 1, 6, C.plankDark); // ladder
	px(ctx, x + 8, y + 6, 1, 6, C.plankDark);
	for (let k = 7; k < 12; k += 2) px(ctx, x + 6, y + k, 3, 1, C.plankDark);
	px(ctx, x - 9, y - 15, 1, 12, C.trunk);
	px(ctx, x - 8, y - 15, 5, 2, C.lifeguard);
	px(ctx, x - 8, y - 13, 5, 2, '#f2c14e');
}

/** A volleyball court marked out in rope, the net across its middle. */
function paintVolleyballCourt(ctx: Paint, x: number, y: number, w: number, h: number) {
	const x0 = Math.round(x - w / 2);
	const y0 = Math.round(y - h / 2);
	px(ctx, x0, y0, w, 1, C.court);
	px(ctx, x0, y0 + h - 1, w, 1, C.court);
	px(ctx, x0, y0, 1, h, C.court);
	px(ctx, x0 + w - 1, y0, 1, h, C.court);
	px(ctx, x0 - 2, y - 1, 2, 3, C.post);
	px(ctx, x0 + w, y - 1, 2, 3, C.post);
	for (let k = 0; k < w; k += 2) px(ctx, x0 + k, y, 1, 1, C.net);
	px(ctx, x0, y - 1, w, 1, 'rgba(74, 84, 99, 0.35)');
}

/** A rack of surfboards stood in the sand. */
function paintSurfRack(ctx: Paint, x: number, y: number) {
	disc(ctx, x + 3, y + 8, 12, 3, C.shadowSoft);
	px(ctx, x - 10, y + 4, 22, 2, C.plankDark);
	C.board.forEach((color, i) => {
		const bx = x - 8 + i * 6;
		disc(ctx, bx, y - 2, 2.2, 8, color);
		px(ctx, bx, y - 9, 1, 15, i === 2 ? '#3fb3b0' : 'rgba(255, 255, 255, 0.55)');
	});
}

/** A ring of stones round a driftwood fire, logs drawn up as seats. */
function paintFirePit(ctx: Paint, x: number, y: number) {
	for (let i = 0; i < 9; i += 1) {
		const a = (i / 9) * Math.PI * 2;
		px(ctx, x + Math.cos(a) * 5, y + Math.sin(a) * 3.5, 2, 2, C.stoneRing);
	}
	px(ctx, x - 3, y - 1, 6, 2, C.trunk);
	px(ctx, x - 1, y - 2, 2, 4, C.trunkLight);
	px(ctx, x - 1, y - 1, 3, 2, C.ember);
	px(ctx, x, y - 1, 1, 1, C.emberHot);
	paintDriftwood(ctx, x - 16, y - 2, 9);
	paintDriftwood(ctx, x + 8, y + 3, 9);
	paintDriftwood(ctx, x - 4, y + 9, 9);
}

/** A row of little beach huts seen from the south: a gabled roof, a painted front with a
 * white-framed door, gaps of sand between them. */
function paintBeachHuts(ctx: Paint, x: number, y: number, count: number, seed: number) {
	const rand = mulberry(seed);
	for (let i = 0; i < count; i += 1) {
		const hy = y + i * 17;
		const hx = x + Math.round((rand() - 0.5) * 4);
		const color = C.huts[(i + Math.floor(rand() * C.huts.length)) % C.huts.length];
		disc(ctx, hx + 8, hy + 12, 9, 2.5, C.shadowSoft);
		// Roof: two slopes meeting at a ridge, the near slope lit.
		for (let r = 0; r < 5; r += 1) {
			const inset = 4 - r;
			px(ctx, hx + inset, hy + r, 13 - inset * 2, 1, r < 2 ? '#76808c' : C.hutRoof);
		}
		px(ctx, hx + 6, hy - 1, 1, 1, '#76808c');
		// Front wall with vertical boards, and a white-framed door.
		px(ctx, hx + 1, hy + 5, 11, 7, color);
		for (let k = 2; k < 12; k += 3) px(ctx, hx + k, hy + 5, 1, 7, 'rgba(255, 255, 255, 0.28)');
		px(ctx, hx + 4, hy + 6, 5, 6, '#f7f1e3');
		px(ctx, hx + 5, hy + 7, 3, 5, 'rgba(40, 40, 50, 0.55)');
		px(ctx, hx + 1, hy + 12, 11, 1, C.plankDark);
	}
}

/** Shells and the odd starfish scattered near the tideline. */
function paintShells(ctx: Paint, x: number, y: number, seed: number) {
	const rand = mulberry(seed);
	const n = 2 + Math.floor(rand() * 4);
	for (let i = 0; i < n; i += 1) {
		const sx = x + (rand() - 0.5) * 14;
		const sy = y + (rand() - 0.5) * 10;
		px(ctx, sx, sy, 2, 1, C.shell);
		px(ctx, sx, sy + 1, 2, 1, C.shellShade);
	}
	if (rand() < 0.45) {
		const sx = x + (rand() - 0.5) * 10;
		const sy = y + (rand() - 0.5) * 8;
		px(ctx, sx, sy - 2, 1, 5, C.starfish);
		px(ctx, sx - 2, sy, 5, 1, C.starfish);
		px(ctx, sx - 1, sy + 1, 1, 1, C.starfish);
		px(ctx, sx + 1, sy + 1, 1, 1, C.starfish);
	}
}

/** A bleached log washed up on the sand. */
function paintDriftwood(ctx: Paint, x: number, y: number, len: number) {
	px(ctx, x + 1, y + 2, len, 1, C.duneShade);
	px(ctx, x, y, len, 2, C.driftwood);
	px(ctx, x, y + 1, len, 1, C.driftwoodDark);
	px(ctx, x + len - 2, y - 1, 2, 1, C.driftwood);
}

/** People and boats are drawn at the office characters' scale: two art pixels per sprite pixel. */
const SPRITE = 2;

/** A small wooden rowboat, bow east (dir 1) or west (-1). */
function paintRowboat(ctx: Paint, x: number, y: number, dir: 1 | -1): void {
	const p = (dx: number, dy: number, w: number, h: number, c: string) =>
		px(ctx, x + dx * SPRITE, y + dy * SPRITE, w * SPRITE, h * SPRITE, c);
	p(1, 5, 11, 1, C.waterDark);
	p(1, 0, 10, 5, C.hull);
	p(dir > 0 ? 11 : 0, 1, 1, 3, C.hull);
	p(2, 1, 8, 3, C.hullInside);
	p(4, 1, 1, 3, C.hull);
	p(7, 1, 1, 3, C.hull);
	p(2, 1, 8, 1, '#b5895a');
}

/* ---------- the shoreline ---------- */

/** Sand between the meadow and the waterline, in art pixels; dunes sit on its landward half. */
const BEACH = 64;

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

/** The campus's land is always this many tiles wide; pavilions beyond it float on the lake. */
export const LAND_COLS = 72;

/** The shoreline x (office-local art pixels) at each row. A plan widened by floating
 * pavilions keeps the land's shore. */
export function makeShore(planWidth: number, height: number) {
	const width = Math.min(planWidth, LAND_COLS * TILE_SIZE);
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
	canvas: WorldLayer;
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
	/** Rods over open water: the bobber floats `reach` sprite pixels out; now and then a fish bites. */
	fishing: Array<{ x: number; y: number; reach: number }>;
	/** Someone fishing off the far end of a deck, drawn above the office. */
	deckFisher: { x: number; y: number } | null;
	/** A swimmer inside the roped area by the jetty. */
	swim: { x: number; y: number };
	/** The volleyball court's centre, where two players keep a ball in the air. */
	volleyball: { x: number; y: number } | null;
	/** Crabs scuttling sideways along the tideline. */
	crabs: Array<{ y: number; phase: number }>;
	/** Open water south of the office where a kayaker paddles. */
	kayakRows: { top: number; bottom: number };
	/** Office-local width and height, for placing things over the campus. */
	W: number;
	H: number;
	/** Art pixels from the plan's west edge to the land's: scenery coordinates start here. */
	landOffsetX: number;
}

const MARGIN_X = 640;
const MARGIN_Y = 480;

export interface CampusSource {
	tiles: readonly number[];
	cols: number;
	rows: number;
	/** Columns of overlook ridge west of the land; the scenery is painted from the land's edge. */
	landOffsetCols?: number;
}

/** How many plan columns lie west of the land (the overlook ridge). */
export function landOffsetOf(source: { landOffsetCols?: number }): number {
	return source.landOffsetCols ?? 0;
}

/** Paint the static campus once for this layout. */
export function buildCampusWorld(source: CampusSource): CampusWorld | null {
	if (typeof document === 'undefined' && typeof OffscreenCanvas === 'undefined') return null;
	// Scenery is painted in land coordinates: x = 0 is the land's west edge, and an overlook
	// ridge, when the company has one, lies at negative x. Plan columns convert through `ox`.
	const oxCols = landOffsetOf(source);
	const ox = oxCols * TILE_SIZE;
	const W = source.cols * TILE_SIZE - ox;
	const H = source.rows * TILE_SIZE;
	const canvas = makeSurface(W + ox + MARGIN_X * 2, H + MARGIN_Y * 2);
	const ctx = paintOn(canvas);
	if (!ctx) return null;
	ctx.imageSmoothingEnabled = false;
	ctx.translate(MARGIN_X + ox, MARGIN_Y);
	const left = -MARGIN_X - ox;
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
			Math.floor((x + ox - pad) / TILE_SIZE),
			Math.floor((y - pad) / TILE_SIZE),
			Math.floor((x + ox + reach) / TILE_SIZE),
			Math.floor((y + reach) / TILE_SIZE)
		);
	};
	// Where the meadow and forest give way to sand: it wanders rather than following the shore.
	const beachEdge = (y: number) =>
		shore(y) -
		BEACH +
		Math.round((field(0, y, 1 / 45, 51) - 0.5) * 20 + (field(3, y, 1 / 11, 52) - 0.5) * 5);
	const land = (x: number, y: number) => x < beachEdge(y) - 2;
	const jettyY = Math.round(Math.min(H * 1.28, H + 360));

	// Meadow: a base green with tufts, pale specks and the odd deeper patch,
	// painted once as a 96-pixel tile and laid as a pattern (fast to build;
	// the tufts are too fine for the repeat to show).
	const grassTile = makeSurface(96, 96);
	const grass = paintOn(grassTile);
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
		return (
			base * H +
			Math.sin(x * 0.045) * 10 +
			Math.sin(x * 0.013 + 2) * 14 +
			(field(x, 0, 1 / 140, 61) - 0.5) * 110
		);
	};
	ctx.fillStyle = C.forestFloor;
	ctx.beginPath();
	ctx.moveTo(left, top);
	for (let x = left; x <= W * 0.64; x += 2) ctx.lineTo(x, Math.max(top, Math.round(forestEdge(x))));
	ctx.lineTo(W * 0.64, top);
	ctx.closePath();
	ctx.fill();
	// The overlook ridge: a terraced hill rising from the forest, a pale cliff face on its south
	// and east where it drops away, and the pavilions on its crest (the plates are drawn above).
	let ridge: { x: number; y: number; rx: number; ry: number } | null = null;
	if (oxCols > 0) {
		let minX = Infinity;
		let maxX = -Infinity;
		let minY = Infinity;
		let maxY = -Infinity;
		for (let row = 0; row < source.rows; row += 1)
			for (let col = 0; col < oxCols; col += 1) {
				if (!solid(col, row)) continue;
				minX = Math.min(minX, col * TILE_SIZE - ox);
				maxX = Math.max(maxX, (col + 1) * TILE_SIZE - ox);
				minY = Math.min(minY, row * TILE_SIZE);
				maxY = Math.max(maxY, (row + 1) * TILE_SIZE);
			}
		if (Number.isFinite(minX)) {
			ridge = {
				x: (minX + maxX) / 2,
				y: (minY + maxY) / 2,
				rx: (maxX - minX) / 2 + 92,
				ry: (maxY - minY) / 2 + 64
			};
			const { x, y, rx, ry } = ridge;
			// Seen from the south, a raised plateau shows its cliff on the south and east only:
			// the face is the plateau shape pushed down and right, and the top covers the rest.
			const drop = 30;
			disc(ctx, x + 26, y + drop + 22, rx + 6, ry + 4, C.shadow);
			disc(ctx, x + 8, y + drop, rx, ry, C.cliffDark);
			const cliffRand = mulberry(71);
			for (let i = 0; i < 900; i += 1) {
				// Vertical strata on the face: short strokes between the top's rim and the foot.
				const a = cliffRand() * Math.PI * 2;
				const k = 0.86 + cliffRand() * 0.14;
				const cx = x + 8 + Math.cos(a) * rx * k;
				const cy = y + drop + Math.sin(a) * ry * k;
				if (((cx - x) / rx) ** 2 + ((cy - y) / ry) ** 2 < 1) continue;
				px(ctx, cx, cy - 2, 1, 2 + cliffRand() * 4, cliffRand() < 0.5 ? C.cliff : C.cliffLight);
			}
			// The plateau top: one sunlit meadow, brighter towards the north-west.
			disc(ctx, x, y, rx, ry, C.highMeadowDark);
			disc(ctx, x - 3, y - 3, rx - 3, ry - 3, C.highMeadow);
			for (let i = 0; i < 160; i += 1) {
				const a = cliffRand() * Math.PI * 2;
				const r = Math.sqrt(cliffRand()) * 0.9;
				const tx = x + Math.cos(a) * rx * r;
				const ty = y + Math.sin(a) * ry * r;
				px(
					ctx,
					tx,
					ty,
					1,
					2,
					Math.cos(a) + Math.sin(a) < -0.4 ? C.highMeadowLight : C.highMeadowDark
				);
			}
			// The rim catches the light: bright along the north-west edge, a turf lip over the cliff.
			for (let t = 0; t < Math.PI * 2; t += 0.012) {
				const ex = x + Math.cos(t) * (rx - 1);
				const ey = y + Math.sin(t) * (ry - 1);
				const lit = Math.cos(t) + Math.sin(t) < -0.3;
				px(ctx, ex, ey, 2, 1, lit ? C.highMeadowLight : C.highMeadowDark);
			}
			for (let i = 0; i < 9; i += 1) {
				const a = 0.2 + cliffRand() * 2.4;
				paintRock(
					ctx,
					x + 8 + Math.cos(a) * (rx + 6),
					y + drop + Math.sin(a) * (ry + 4),
					3 + cliffRand() * 3,
					cliffRand() < 0.5
				);
			}
		}
	}
	const onRidge = (x: number, y: number) =>
		ridge !== null && ((x - ridge.x) / ridge.rx) ** 2 + ((y - ridge.y) / ridge.ry) ** 2 < 0.95;
	const forestRand = mulberry(11);
	const trees: Array<[number, number, number, number]> = [];
	for (let y = top - 8; y < H * 0.5; y += 15) {
		for (let x = left - 8; x < W * 0.66; x += 17) {
			const tx = x + (forestRand() - 0.5) * 12;
			const ty = y + (forestRand() - 0.5) * 10;
			const edge = forestEdge(tx);
			const inside = ty < edge - 4;
			const fringe = !inside && ty < edge + 18 && forestRand() < 0.35;
			if (!(inside || fringe) || onPlate(tx, ty, 14) || tx > beachEdge(ty) + 4 || onRidge(tx, ty))
				continue;
			trees.push([tx, ty, 9 + forestRand() * 6, Math.floor(forestRand() * 10_000)]);
		}
	}

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
	ctx.fillStyle = C.sand;
	ctx.beginPath();
	ctx.moveTo(beachEdge(top), top);
	for (let y = top; y <= bottom; y += 1) ctx.lineTo(beachEdge(y), y);
	ctx.lineTo(right, bottom);
	ctx.lineTo(right, top);
	ctx.closePath();
	ctx.fill();
	band(-4, null, C.sandWet);
	band(3, null, C.shallow);
	band(22, null, C.mid);
	band(50, null, C.deep);
	band(120, null, C.deeper);
	for (let y = top; y < bottom; y += 1) {
		const s = shore(y);
		// Wind-ripple specks across the sand, and a ragged edge where the meadow gives way to it.
		for (let k = 0; k < 2; k += 1)
			if (hash(s, y, 3 + k * 11) < 0.12)
				px(
					ctx,
					beachEdge(y) + Math.floor(hash(y, s, 4 + k * 11) * (s - beachEdge(y) - 6)),
					y,
					2,
					1,
					C.sandShade
				);
		const edge = Math.floor(hash(y, s, 9) * 4);
		if (edge) px(ctx, beachEdge(y) - edge, y, edge, 1, C.sand);
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
	// The forest's edge: crowns overhang the sand unevenly, and a fringe of beach grass and low
	// scrub blurs the line where the meadow ends.
	trees.sort((a, b) => a[1] - b[1]);
	for (const [tx, ty, r, seed] of trees) stampTree(ctx, tx, ty, r, seed, false);
	if (ridge) {
		const slopeRand = mulberry(73);
		for (let a = 0; a < Math.PI * 2; a += 0.11 + slopeRand() * 0.12) {
			const k = 0.97 + slopeRand() * 0.12;
			const sx = ridge.x + Math.cos(a) * ridge.rx * k;
			const sy = ridge.y + Math.sin(a) * ridge.ry * k;
			if (onPlate(sx, sy, 12) || !land(sx, sy) || slopeRand() < 0.35) continue;
			paintConifer(ctx, sx, sy, 7 + slopeRand() * 4);
		}
	}
	const fringeRand = mulberry(43);
	for (let y = top; y < bottom; y += 2 + Math.floor(fringeRand() * 6)) {
		const e = beachEdge(y);
		if (field(5, y, 1 / 34, 53) < 0.4 || onPlate(e, y, 8)) continue;
		const x = e - 4 + fringeRand() * 12;
		if (fringeRand() < 0.7) {
			px(ctx, x, y - 2, 1, 3, C.marramDark);
			px(ctx, x + 1, y - 3, 1, 3, C.marram);
			if (fringeRand() < 0.5) px(ctx, x - 1, y - 1, 1, 2, C.marram);
		} else if (x < e) paintShrub(ctx, x - 3, y, 2.5 + fringeRand() * 2);
	}

	// Beach life, in scenes spaced irregularly along the sand: umbrellas and towels, a row of
	// huts, a lifeguard's tower, a volleyball game, a sandcastle, surfboards and a fire pit.
	const beachRand = mulberry(61);
	const sandFree = (y: number, reach: number) => {
		for (let dy = -reach; dy <= reach; dy += 6)
			if (onPlate(shore(y + dy) - BEACH / 2, y + dy, BEACH / 2 + 4)) return false;
		return Math.abs(y - jettyY) > reach + 18;
	};
	const scenes = [
		'umbrellas',
		'huts',
		'volleyball',
		'umbrellas',
		'sandcastle',
		'lifeguard',
		'umbrellas',
		'surf',
		'umbrellas',
		'firepit',
		'umbrellas',
		'sandcastle',
		'umbrellas'
	];
	for (let i = scenes.length - 1; i > 0; i -= 1) {
		const j = Math.floor(beachRand() * (i + 1));
		[scenes[i], scenes[j]] = [scenes[j], scenes[i]];
	}
	let volleyball: CampusWorld['volleyball'] = null;
	let scene = 0;
	for (let y = top + 50; y < bottom - 50 && scene < scenes.length;) {
		const kind = scenes[scene];
		const reach = kind === 'volleyball' || kind === 'huts' ? 30 : 16;
		if (!sandFree(y, reach)) {
			y += 8;
			continue;
		}
		const s0 = shore(y);
		const mid = Math.round((beachEdge(y) + s0) / 2);
		if (kind === 'umbrellas') {
			const n = 1 + Math.floor(beachRand() * 3);
			for (let k = 0; k < n; k += 1) {
				const uy = y - 12 + k * 15;
				const ux = mid - 6 + Math.round((beachRand() - 0.5) * 10);
				paintTowel(
					ctx,
					ux + 6,
					uy - 2,
					C.towels[Math.floor(beachRand() * C.towels.length)],
					beachRand() < 0.6,
					Math.round(ux + uy)
				);
				paintUmbrella(ctx, ux, uy, C.umbrellas[Math.floor(beachRand() * C.umbrellas.length)]);
			}
		} else if (kind === 'huts') paintBeachHuts(ctx, beachEdge(y) + 4, y - 30, 4, Math.round(y));
		else if (kind === 'volleyball') {
			paintVolleyballCourt(ctx, mid, y, 22, 46);
			volleyball = { x: mid, y };
		} else if (kind === 'sandcastle') paintSandcastle(ctx, s0 - 22, y);
		else if (kind === 'lifeguard') paintLifeguardTower(ctx, s0 - 24, y);
		else if (kind === 'surf') paintSurfRack(ctx, mid, y);
		else if (kind === 'firepit') paintFirePit(ctx, mid, y);
		scene += 1;
		y += reach * 2 + 16 + Math.floor(beachRand() * 60);
	}
	// Shells along the tideline, gathered where the waves leave them.
	for (let y = top; y < bottom; y += 18 + Math.floor(beachRand() * 40)) {
		if (field(9, y, 1 / 60, 55) < 0.5 || onPlate(shore(y) - 10, y, 10)) continue;
		paintShells(ctx, shore(y) - 9, y, Math.round(y));
	}
	const crabs: CampusWorld['crabs'] = [];
	for (let i = 0; i < 40 && crabs.length < 3; i += 1) {
		const y = Math.round(top + beachRand() * (bottom - top));
		if (onPlate(shore(y) - 8, y, 14) || Math.abs(y - jettyY) < 30) continue;
		crabs.push({ y, phase: beachRand() });
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

	// Meadow life. Low-frequency fields decide where things gather: groves of one kind of tree,
	// a few wildflower meadows in their own colours, patches of long grass, and open lawn
	// between them and around the buildings.
	const meadowRand = mulberry(47);
	const grove = (x: number, y: number) => field(x, y, 1 / 170, 31);
	const flowerField = (x: number, y: number) => field(x, y, 1 / 130, 33);
	const wildGrass = (x: number, y: number) => field(x, y, 1 / 90, 35);
	const standKind = (x: number, y: number) => {
		const k = field(x, y, 1 / 280, 37);
		return k > 0.6 ? 'conifer' : k < 0.36 ? 'birch' : 'broadleaf';
	};
	const meadowTrees: Array<[number, number, number, number, string]> = [];
	const span = (step: number, visit: (x: number, y: number) => void) => {
		for (let gy = top; gy < bottom; gy += step)
			for (let gx = left; gx < W * 0.8; gx += step)
				visit(gx + (meadowRand() - 0.5) * step * 0.9, gy + (meadowRand() - 0.5) * step * 0.9);
	};
	span(22, (x, y) => {
		if (!land(x, y) || onPlate(x, y, 28) || y < forestEdge(x) + 16 || onRidge(x, y)) return;
		const g = grove(x, y);
		const inGrove = g > 0.6 && meadowRand() < (g - 0.6) * 4;
		const lone = meadowRand() < 0.012;
		if (!inGrove && !lone) return;
		const kind = meadowRand() < 0.12 ? 'broadleaf' : standKind(x, y);
		const r = (inGrove ? 8 : 10) + meadowRand() * 5;
		meadowTrees.push([x, y, r, Math.floor(meadowRand() * 10_000), kind]);
	});

	// A gravel path leaves the southernmost paving and wanders to the jetty. It starts under the
	// plate's south edge, so the floor runs straight onto it.
	let pathStart = { x: W * 0.44, y: H };
	for (let row = 0; row < source.rows; row += 1)
		for (let col = 0; col < source.cols; col += 1) {
			if (!solid(col, row) || solid(col, row + 1)) continue;
			const x = col * TILE_SIZE + TILE_SIZE / 2 - ox;
			const y = (row + 1) * TILE_SIZE;
			if (!land(x, y + 12)) continue;
			const better =
				y > pathStart.y + 0.5 ||
				(Math.abs(y - pathStart.y) < 0.5 &&
					Math.abs(x - W * 0.44) < Math.abs(pathStart.x - W * 0.44));
			if (better || pathStart.y === H) pathStart = { x, y };
		}
	const path = (t: number): [number, number] => {
		const x0 = pathStart.x;
		const y0 = pathStart.y - 4;
		const x3 = shore(H * 1.28) - 8;
		const y3 = H * 1.28;
		// A cubic with an easy S: it drops away from the building, then leans to the shore.
		const x1 = x0 + 6;
		const y1 = y0 + (y3 - y0) * 0.6;
		const x2 = x0 + (x3 - x0) * 0.55;
		const y2 = y3 + 4;
		const u = 1 - t;
		return [
			u * u * u * x0 + 3 * u * u * t * x1 + 3 * u * t * t * x2 + t * t * t * x3,
			u * u * u * y0 + 3 * u * u * t * y1 + 3 * u * t * t * y2 + t * t * t * y3
		];
	};
	const pathPoints: Array<[number, number]> = [];
	for (let t = 0; t <= 1; t += 0.02) pathPoints.push(path(t));
	const nearPath = (x: number, y: number, reach: number) =>
		pathPoints.some(([px0, py0]) => Math.hypot(px0 - x, py0 - y) < reach);
	// Worn, not ruled: the width wanders and the edge frays into the grass.
	const pathWidth = (t: number) => 3 + field(t * 400, 0, 1 / 60, 41) * 2.2;
	for (let t = 0; t <= 1; t += 0.003) {
		const [x, y] = path(t);
		const w = pathWidth(t);
		disc(ctx, x, y, w + 1, w * 0.8 + 1, C.gravelEdge);
	}
	for (let t = 0; t <= 1; t += 0.003) {
		const [x, y] = path(t);
		const w = pathWidth(t);
		disc(ctx, x, y, w, w * 0.8, C.gravel);
		const h = hash(Math.round(x), Math.round(y), 7);
		if (h < 0.12) px(ctx, x + (h * 40 - 2), y - 1, 1, 1, C.gravelDot);
		else if (h > 0.96) px(ctx, x + (h > 0.98 ? w : -w), y, 1, 2, C.grassDark);
	}

	// Ground cover, painted before the trees so crowns overlap it. Long grass and stones come
	// in patches, shrubs skirt the groves, and wildflowers fill a few meadows; the lawn by the
	// buildings stays clear.
	const open = (x: number, y: number, pad: number) =>
		land(x, y) && !onPlate(x, y, pad) && !nearPath(x, y, pad + 3);
	const groundRand = mulberry(97);
	span(18, (x, y) => {
		if (y < forestEdge(x) - 4 || !open(x, y, 24)) return;
		const f = flowerField(x, y);
		if (f > 0.6 && groundRand() < (f - 0.6) * 5) {
			const palette = C.meadows[Math.floor(field(x, y, 1 / 420, 39) * C.meadows.length)];
			paintWildflowers(ctx, x, y, Math.round(x * 7 + y), palette);
			return;
		}
		const g = grove(x, y);
		if (g > 0.5 && g < 0.62 && groundRand() < 0.28) {
			if (groundRand() < 0.4) paintBerryBush(ctx, x, y, Math.round(x + y * 3));
			else paintShrub(ctx, x, y, 3 + groundRand() * 2.5);
			return;
		}
		const w = wildGrass(x, y);
		if (w > 0.6 && groundRand() < (w - 0.6) * 3) {
			paintTallGrass(ctx, x, y, Math.round(x * 3 + y));
			return;
		}
		if (field(x, y, 1 / 210, 43) > 0.78 && groundRand() < 0.25)
			paintRock(ctx, x, y, 2 + groundRand() * 3, groundRand() < 0.5);
	});
	// Where the forest meets the meadow: ferns in the damp stretches, the odd toadstool ring.
	for (let x = left; x < W * 0.66; x += 6 + Math.floor(groundRand() * 18)) {
		const y = forestEdge(x) + 4 + groundRand() * 12;
		if (!open(x, y, 8)) continue;
		const damp = field(x, 0, 1 / 70, 45);
		if (damp > 0.55 && groundRand() < 0.7) paintFern(ctx, x, y, Math.round(x));
		else if (damp < 0.25 && groundRand() < 0.15) paintMushrooms(ctx, x, y, Math.round(x) + 5);
	}
	// Driftwood on the sand between the dunes and the water.
	for (const ry of [0.12, 0.47, 0.83, 1.18, 1.45]) {
		const y = Math.round(H * ry);
		const x = shore(y) - 22 + Math.round(hash(y, 3, 77) * 8);
		if (!onPlate(x, y, 10) && Math.abs(y - H * 1.28) > 20)
			paintDriftwood(ctx, x, y, 8 + Math.round(hash(y, 4, 77) * 6));
	}

	meadowTrees.sort((a, b) => a[1] - b[1]);
	for (const [x, y, r, seed, kind] of meadowTrees) {
		if (nearPath(x, y, r + 6)) continue;
		if (kind === 'conifer') paintConifer(ctx, x, y, r + 1);
		else if (kind === 'birch') paintBirch(ctx, x, y, r, seed);
		else stampTree(ctx, x, y, r, seed, true);
	}

	// Where the pavilions meet the ground: a soft cast shadow, a light lip,
	// and garden beds along the land-facing edges so work sits in nature.
	for (let row = 0; row < source.rows; row += 1) {
		for (let col = 0; col < source.cols; col += 1) {
			if (!solid(col, row)) continue;
			const x = col * TILE_SIZE - ox;
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

	// A wooden jetty where the gravel path meets the beach, a rowboat tied up beside it, and a
	// roped swimming area to the south.
	const jettyFrom = shore(jettyY) - 14;
	const jettyTo = shore(jettyY) + 84;
	const deck = (x: number, w: number) => {
		px(ctx, x, jettyY - 6, w, 13, C.plank);
		px(ctx, x, jettyY - 6, w, 1, C.plankLight);
		px(ctx, x, jettyY + 6, w, 2, C.plankDark);
	};
	px(ctx, shore(jettyY) + 2, jettyY + 8, jettyTo - shore(jettyY) - 1, 2, C.waterDark);
	deck(jettyFrom, jettyTo - jettyFrom);
	for (let x = jettyFrom + 6; x < jettyTo; x += 6) px(ctx, x, jettyY - 5, 1, 11, C.plankDark);
	for (let x = shore(jettyY) + 8; x < jettyTo; x += 18) px(ctx, x, jettyY + 8, 3, 3, C.post);
	paintRowboat(ctx, jettyTo - 30, jettyY - 20, 1);
	px(ctx, jettyTo - 8, jettyY - 10, 1, 5, C.rope);
	for (let y = jettyY + 40; y < jettyY + 150; y += 10) {
		px(ctx, shore(y) + 70, y, 3, 3, C.buoy);
		px(ctx, shore(y) + 70, y, 2, 1, C.buoyLight);
	}
	const fishing: CampusWorld['fishing'] = [{ x: jettyTo - 6, y: jettyY - 2, reach: 16 }];
	// A second rod off the middle of the far edge of the deck that reaches furthest out.
	let deckX = 0;
	const deckRows: number[] = [];
	for (let row = 0; row < source.rows; row += 1)
		for (let col = 0; col < source.cols; col += 1) {
			if (!solid(col, row) || solid(col + 1, row)) continue;
			const x = (col + 1) * TILE_SIZE - ox;
			if (x <= shore(row * TILE_SIZE) + 40) continue;
			if (x > deckX) {
				deckX = x;
				deckRows.length = 0;
			}
			if (x === deckX) deckRows.push(row);
		}
	const deckRow = deckRows[Math.floor(deckRows.length / 2)];
	const deckFisher =
		deckRow === undefined ? null : { x: deckX - 6, y: deckRow * TILE_SIZE + TILE_SIZE / 2 };

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
		fishing,
		deckFisher,
		volleyball,
		crabs,
		swim: { x: shore(jettyY + 90) + 36, y: jettyY + 90 },
		kayakRows: { top: jettyY + 170, bottom: Math.max(jettyY + 260, bottom - 40) },
		shoreRows: openShore(),
		W,
		H,
		canvas,
		originX: MARGIN_X + ox,
		landOffsetX: ox,
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

export function campusKey(layout: CampusSource): string {
	return `${layout.cols}x${layout.rows}+${landOffsetOf(layout)}:${layout.tiles.join('')}`;
}

/** The painted campus for this key, if it is the one kept. */
export function keptCampusWorld(key: string): CampusWorld | null | undefined {
	return lastWorld?.key === key ? lastWorld.world : undefined;
}

/** Keep a campus painted elsewhere (see offThread.ts). */
export function keepCampusWorld(key: string, world: CampusWorld | null): void {
	lastWorld = { key, world };
}

/** The campus source for a plan: its layout and how far west of it the land begins. */
export function campusSourceOf(plan: {
	layout: CampusSource;
	landOffsetCols?: number;
}): CampusSource {
	return { ...plan.layout, landOffsetCols: plan.landOffsetCols ?? landOffsetOf(plan.layout) };
}

export function campusWorldFor(plan: {
	layout: CampusSource;
	landOffsetCols?: number;
}): CampusWorld | null {
	const source = campusSourceOf(plan);
	const key = campusKey(source);
	if (lastWorld?.key === key) return lastWorld.world;
	lastWorld = { key, world: buildCampusWorld(source) };
	return lastWorld.world;
}

/** The campus as the paint worker returns it: a bitmap and the plain motion seeds. */
export type PaintedCampus = Omit<CampusWorld, 'canvas' | 'shore'> & { bitmap: ImageBitmap };

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
/** The view shifted so world positions, which are in land coordinates, land on the plan. */
function landView(world: CampusWorld, view: CampusView): CampusView {
	return world.landOffsetX
		? { ...view, offsetX: view.offsetX + world.landOffsetX * view.zoom }
		: view;
}

export function drawCampusBackground(ctx: Paint, world: CampusWorld, planView: CampusView): void {
	const view = landView(world, planView);
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
		for (const rod of world.fishing) drawFisher(ctx, view, rod.x, rod.y, rod.reach, 0, false);
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
	drawWaterActivities(ctx, world, view, t);
}

/* People on the water: a sailboat tacking far out, a rowboat, a kayaker, a swimmer, and rods
 * with bobbers. Drawn at the office characters' scale, deterministic in time and slow enough to
 * stay calm. */
function drawWaterActivities(ctx: Paint, world: CampusWorld, view: CampusView, t: number): void {
	const W = world.W;
	const H = world.H;
	const at = (x: number, y: number) => (dx: number, dy: number, w: number, h: number, c: string) =>
		screen(ctx, view, x + dx * SPRITE, y + dy * SPRITE, w * SPRITE, h * SPRITE, c);
	// Sailboat: tacks back and forth well beyond every deck.
	{
		const span = 260;
		const travel = (t * 6) % (span * 2);
		const heading = travel < span ? 1 : -1;
		const x = W + 60 + (travel < span ? travel : span * 2 - travel);
		const y = H * 0.55 + Math.sin(t * 0.05) * 24;
		const p = at(x, y);
		p(-heading * 9, 2, 4, 1, C.foamDim);
		p(-heading * 14, 2, 3, 1, C.rippleDim);
		p(-5, 3, 12, 1, C.waterDark);
		p(-5, 0, 12, 3, '#f4efe4');
		p(-5, 2, 12, 1, '#c9c1b0');
		p(heading > 0 ? 7 : -6, 1, 1, 1, '#f4efe4');
		// The mast stands forward of centre; the mainsail trails aft of it to the boom.
		const mast = heading * 2;
		p(mast, -12, 1, 12, C.trunk);
		for (let r = 0; r < 11; r += 1) {
			const w = Math.max(1, Math.round((r + 1) * 0.65));
			p(heading > 0 ? mast - w : mast + 1, -11 + r, w, 1, r % 4 === 3 ? C.sailShade : C.sail);
		}
		p(heading > 0 ? mast - 7 : mast + 1, -1, 7, 1, C.trunk);
		p(mast, -13, 2, 1, C.buoy);
	}
	// Rowboat: one rower pulls slowly along open water, oars dipping in turn.
	{
		const span = 170;
		const travel = (t * 3 + 80) % (span * 2);
		const heading = travel < span ? 1 : -1;
		const x = W + 30 + (travel < span ? travel : span * 2 - travel);
		const y = H * 0.18;
		const stroke = Math.floor(t * 1.4) % 2;
		const p = at(x, y);
		p(heading > 0 ? -3 : 13, 3, 2, 1, C.rippleDim);
		p(1, 5, 11, 1, C.waterDark);
		p(1, 0, 10, 5, C.hull);
		p(heading > 0 ? 11 : 0, 1, 1, 3, C.hull);
		p(2, 1, 8, 3, C.hullInside);
		p(5, 1, 2, 3, C.shirtAlt);
		p(5, 1, 2, 1, C.skin);
		const oar = stroke ? 1 : -1;
		p(5 + oar, -3, 1, 3, C.plankDark);
		p(5 + oar, 5, 1, 3, C.plankDark);
		if (stroke) {
			p(5 + oar, -4, 1, 1, C.foamDim);
			p(5 + oar, 8, 1, 1, C.foamDim);
		}
	}
	// Kayak: paddles up and down the open water south of the office.
	{
		const top = world.kayakRows.top;
		const span = Math.max(60, world.kayakRows.bottom - top);
		const travel = (t * 7) % (span * 2);
		const heading = travel < span ? 1 : -1;
		const y = top + (travel < span ? travel : span * 2 - travel);
		const x = world.shore(y) + 110;
		const side = Math.floor(t * 2) % 2 ? 1 : -1;
		const p = at(x, y);
		p(0, -heading * 9, 1, 2, C.foamDim);
		p(-1, -6, 3, 12, '#e8b23c');
		p(0, -7, 1, 14, '#e8b23c');
		p(-1, -1, 3, 3, C.shirt);
		p(0, -2, 1, 1, C.skin);
		p(-4, side, 9, 1, '#4a5463');
		p(side * 5, side, 1, 1, C.foam);
	}
	// A swimmer in the roped area: a head, an arm over now and then, a ring of ripples.
	{
		const x = world.swim.x + Math.round(Math.sin(t * 0.3) * 14);
		const y = world.swim.y + Math.round(Math.cos(t * 0.21) * 18);
		const ring = Math.floor(t * 1.5) % 3;
		const p = at(x, y);
		p(-2 - ring, 1, 1, 1, C.ripple);
		p(2 + ring, 1, 1, 1, C.ripple);
		p(-1, -1, 2, 2, C.skin);
		p(-1, -1, 2, 1, '#5a4636');
		if (Math.floor(t * 1.2) % 3 === 0) p(2, -2, 1, 2, C.skin);
	}
	for (const rod of world.fishing) drawFisher(ctx, view, rod.x, rod.y, rod.reach, t, true);
	drawBeachLife(ctx, world, view, t);
}

/* Crabs scuttle sideways along the tideline, stopping now and then; two players keep a
 * volleyball in the air over the net. */
function drawBeachLife(ctx: Paint, world: CampusWorld, view: CampusView, t: number): void {
	for (const crab of world.crabs) {
		const cycle = (t / 14 + crab.phase) % 1;
		const moving = cycle < 0.6;
		const along = Math.sin(cycle * Math.PI * 2) * 10;
		const y = crab.y + Math.round(along);
		const x = world.shore(y) - 8;
		const step = moving ? Math.floor(t * 6) % 2 : 0;
		screen(ctx, view, x - 2, y, 5, 3, C.crab);
		screen(ctx, view, x - 1, y, 3, 1, '#ee7a62');
		screen(ctx, view, x - 4, y - 1 + step, 2, 2, C.crabDark);
		screen(ctx, view, x + 3, y - 1 + (1 - step), 2, 2, C.crabDark);
		screen(ctx, view, x - 1, y - 1, 1, 1, '#1d1a18');
		screen(ctx, view, x + 1, y - 1, 1, 1, '#1d1a18');
	}
	const court = world.volleyball;
	if (!court) return;
	const p = (x: number, y: number) => (dx: number, dy: number, w: number, h: number, c: string) =>
		screen(ctx, view, x + dx * SPRITE, y + dy * SPRITE, w * SPRITE, h * SPRITE, c);
	const rally = (t / 2.4) % 2;
	const toSouth = rally < 1;
	const u = toSouth ? rally : rally - 1;
	const north = { x: court.x - 3 + Math.round(Math.sin(t * 0.7) * 3), y: court.y - 16 };
	const south = { x: court.x + 1 + Math.round(Math.cos(t * 0.6) * 3), y: court.y + 16 };
	for (const [player, shirt, hair] of [
		[north, C.shirt, '#5a4636'],
		[south, C.shirtAlt, '#e0c27a']
	] as const) {
		const hit = (player === north) === (u > 0.92 || u < 0.08) ? 0 : 1;
		const q = p(player.x, player.y);
		q(-1, -3, 3, 3, C.skin);
		q(-1, -3, 3, 1, hair);
		q(-2, 0, 5, 4, shirt);
		q(-2, -hit, 1, 2, C.skin);
		q(2, -hit, 1, 2, C.skin);
		q(-1, 4, 1, 3, C.skin);
		q(1, 4, 1, 3, C.skin);
	}
	const from = toSouth ? north : south;
	const to = toSouth ? south : north;
	const bx = from.x + (to.x - from.x) * u;
	const by = from.y + (to.y - from.y) * u - 8;
	const lift = Math.sin(u * Math.PI) * 18;
	screen(ctx, view, bx + 2, by + 10, 4, 2, C.shadowSoft);
	screen(ctx, view, bx, by - lift, 4, 4, '#f7f1e3');
	screen(ctx, view, bx + 1, by - lift, 2, 1, '#f2c14e');
	screen(ctx, view, bx, by - lift + 2, 2, 1, '#3f8fd2');
}

/** Someone sitting at the end of a jetty or deck with a rod, a line and a bobber. A fish bites
 * every twenty seconds or so: the bobber goes under and the rod tip bends. */
function drawFisher(
	ctx: Paint,
	view: CampusView,
	x: number,
	y: number,
	reach: number,
	t: number,
	moving: boolean
): void {
	const p = (dx: number, dy: number, w: number, h: number, c: string) =>
		screen(ctx, view, x + dx * SPRITE, y + dy * SPRITE, w * SPRITE, h * SPRITE, c);
	p(-2, -1, 3, 3, C.shirt);
	p(-2, -3, 2, 2, C.skin);
	p(-3, -4, 4, 1, C.hat);
	p(1, 2, 2, 1, '#4a5463');
	const cycle = moving ? (t + x * 0.37) % 20 : 0;
	const bite = cycle > 17 && cycle < 18.6;
	for (let i = 1; i <= 7; i += 1)
		p(i, -2 - Math.round(i * 0.6) + (bite && i > 5 ? 1 : 0), 1, 1, C.trunk);
	// The line and bobber are thin, at art scale, out past the rod tip.
	const tipX = x + 8 * SPRITE;
	const tipY = y - 6 * SPRITE + (bite ? SPRITE : 0);
	const bob = moving && !bite ? Math.round(Math.sin(t * 2.4 + x)) : 0;
	const bx = x + reach * SPRITE;
	const by = y + 3 * SPRITE + bob + (bite ? 2 : 0);
	for (let i = 0; i <= 12; i += 1)
		screen(
			ctx,
			view,
			tipX + ((bx - tipX) * i) / 12,
			tipY + ((by - tipY) * i * i) / 144,
			1,
			1,
			C.rope
		);
	if (!bite) {
		screen(ctx, view, bx - 1, by - 2, 3, 2, C.buoyLight);
		screen(ctx, view, bx - 1, by, 3, 2, C.buoy);
	}
	const ring = Math.floor(t * 2) % 3;
	const spread = bite ? 3 + ring * 2 : 3 + (ring % 2);
	screen(ctx, view, bx - spread, by + 2, 2, 1, C.ripple);
	screen(ctx, view, bx + spread, by + 2, 2, 1, C.ripple);
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
	planView: CampusView,
	_officeCols: number,
	_officeRows: number
): void {
	const view = landView(world, planView);
	if (world.deckFisher)
		drawFisher(ctx, view, world.deckFisher.x, world.deckFisher.y, 16, view.now / 1000, view.motion);
	if (!view.motion) return;
	const t = view.now / 1000;
	const W = world.W;
	const H = world.H;
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
