import { TILE_SIZE, TileType } from '$lib/vendor/pixel-agents/webview-ui/src/office/types.js';
import type { OfficePlan, TileRect } from './officePlan';
import { makeShore } from './campusBackdrop';

/* The ground inside the office, painted into the cached floor in office-local
 * art pixels (a tile is 16). Stone stays stone where people work; the
 * circulation between pavilions becomes a timber boardwalk; the central court
 * becomes a raked-gravel zen garden with moss, stones, stepping stones and a
 * koi pond. Only the koi and the pond's light move (see drawGroundMotion). */

const G = {
	board: ['#cdb48f', '#c7ad87', '#d3bb97', '#c9b18b'],
	boardSeam: '#a88e69',
	boardEnd: '#b39874',
	boardGrain: '#bfa47d',
	gravel: '#ebe6d8',
	gravelDot: '#dfd8c6',
	rake: '#d7cfba',
	rakeLight: '#f4f1e8',
	moss: '#86ad73',
	mossDark: '#6e9861',
	mossLight: '#a3c68c',
	stone: '#cfd1c8',
	stoneShade: '#a9ada3',
	stoneDark: '#8d9189',
	step: '#c9c6ba',
	stepShade: '#aaa697',
	pondRim: '#b9b8ad',
	pondRimDark: '#96968c',
	pondDeep: '#4f9aa5',
	pond: '#63b0b4',
	pondLight: '#8fcdc9',
	lily: '#5f9b63',
	lotus: '#f3b9c9',
	lanternBody: '#b7b3a5',
	lanternShade: '#908c80',
	lanternLight: '#f3dc92',
	koi: '#f08a4b',
	koiWhite: '#fbf4ea',
	koiShadow: 'rgba(30, 80, 90, 0.35)',
	sun: 'rgba(255, 244, 214, 0.2)',
	paver: ['#dfe3dd', '#d8ddd7', '#e3e6df', '#dadfd8'],
	edgeGrass: '#9ccb8c',
	edgeGrassDark: '#86b97a',
	edgeGrassLight: '#b3d99f',
	edgeMoss: '#8db57c',
	wetEdge: 'rgba(60, 110, 110, 0.35)',
	wetLight: 'rgba(240, 250, 246, 0.55)',
	edgeFlowers: ['#f3e3a0', '#eab6d0', '#f7f3e9'],
	paverJoint: '#c2c9c3',
	paverSpeck: '#cfd5cf'
} as const;

function hash(x: number, y: number, salt = 0): number {
	let h = (x * 374761393 + y * 668265263 + salt * 2147483647) | 0;
	h = Math.imul(h ^ (h >>> 13), 1274126177);
	h ^= h >>> 16;
	return (h >>> 0) / 4294967296;
}

type Paint = CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D;

function px(ctx: Paint, x: number, y: number, w: number, h: number, color: string) {
	ctx.fillStyle = color;
	ctx.fillRect(
		Math.round(x),
		Math.round(y),
		Math.max(1, Math.round(w)),
		Math.max(1, Math.round(h))
	);
}

function disc(ctx: Paint, cx: number, cy: number, rx: number, ry: number, color: string) {
	ctx.fillStyle = color;
	const top = Math.round(-ry);
	for (let dy = top; dy <= -top; dy += 1) {
		const k = 1 - (dy * dy) / (ry * ry + 0.01);
		if (k <= 0) continue;
		const half = Math.round(Math.sqrt(k) * rx);
		ctx.fillRect(Math.round(cx - half), Math.round(cy + dy), half * 2 + 1, 1);
	}
}

function ringPixels(cx: number, cy: number, rx: number, ry: number): Array<[number, number]> {
	const out = new Map<string, [number, number]>();
	const steps = Math.round((rx + ry) * 4);
	for (let i = 0; i < steps; i += 1) {
		const a = (i / steps) * Math.PI * 2;
		const x = Math.round(cx + Math.cos(a) * rx);
		const y = Math.round(cy + Math.sin(a) * ry);
		out.set(`${x},${y}`, [x, y]);
	}
	return [...out.values()];
}

const rectPx = (rect: TileRect) => ({
	x: rect.col * TILE_SIZE,
	y: rect.row * TILE_SIZE,
	w: rect.width * TILE_SIZE,
	h: rect.height * TILE_SIZE
});

/** Static ground painting, cached with the floor. */
export function paintOfficeGround(ctx: Paint, plan: OfficePlan): void {
	const { tiles, cols, rows } = plan.layout;
	// Timber boardwalk wherever the plan lays deck.
	for (let row = 0; row < rows; row += 1) {
		for (let col = 0; col < cols; col += 1) {
			if (tiles[row * cols + col] !== TileType.FLOOR_7) continue;
			const x = col * TILE_SIZE;
			const y = row * TILE_SIZE;
			for (let board = 0; board < 4; board += 1) {
				const by = y + board * 4;
				// Boards run east–west in long lengths; their colour changes per
				// plank, not per tile, so the deck reads as continuous timber.
				const plank = Math.floor((col + (row * 4 + board) * 3) / 3);
				px(ctx, x, by, TILE_SIZE, 4, G.board[Math.floor(hash(plank, row * 4 + board, 1) * 4)]);
				px(ctx, x, by + 3, TILE_SIZE, 1, G.boardSeam);
				// Plank ends fall at random along each run, never on a diagonal.
				if (hash(col, row * 4 + board, 8) < 0.28)
					px(ctx, x + 2 + Math.floor(hash(row * 4 + board, col, 9) * 12), by, 1, 3, G.boardEnd);
				if (hash(col, row * 4 + board, 2) < 0.35)
					px(ctx, x + Math.floor(hash(row, col + board, 3) * 12), by + 1, 3, 1, G.boardGrain);
			}
		}
	}
	// Pale travertine pavers where people work: stone, but warm and quiet.
	for (let row = 0; row < rows; row += 1) {
		for (let col = 0; col < cols; col += 1) {
			if (tiles[row * cols + col] !== TileType.FLOOR_4) continue;
			const x = col * TILE_SIZE;
			const y = row * TILE_SIZE;
			for (let course = 0; course < 2; course += 1) {
				const cy = y + course * 8;
				const shift = (row * 2 + course) % 2 === 0 ? 0 : 8;
				for (let half = -1; half < 2; half += 1) {
					const left = Math.max(x, x + shift + half * 16);
					const right = Math.min(x + TILE_SIZE, x + shift + half * 16 + 16);
					if (right <= left) continue;
					const paverId = Math.floor((col * 16 + shift + half * 16) / 16);
					px(
						ctx,
						left,
						cy,
						right - left,
						8,
						G.paver[Math.floor(hash(paverId, row * 2 + course, 20) * 4)]
					);
				}
				px(ctx, x, cy + 7, TILE_SIZE, 1, G.paverJoint);
				const joint = x + shift;
				if (joint >= x && joint < x + TILE_SIZE) px(ctx, joint, cy, 1, 7, G.paverJoint);
				if (hash(col, row * 2 + course, 21) < 0.3)
					px(
						ctx,
						x + 2 + hash(row, col + course, 22) * 12,
						cy + 2 + hash(col, row, 23) * 4,
						1,
						1,
						G.paverSpeck
					);
			}
		}
	}
	paintZenGarden(ctx, plan);
	paintGroundEdges(ctx, plan);
}

/* Where a plate meets the meadow, the meadow grows onto it: an irregular
 * fringe of grass and moss a few pixels deep, tufts reaching further in, and
 * the odd flower. Where it meets the lake, the timber edge is wet and dark
 * instead. Painted inside the floor so it sits under furniture and people. */
function paintGroundEdges(ctx: Paint, plan: OfficePlan): void {
	const { tiles, cols, rows } = plan.layout;
	const T = TILE_SIZE;
	// The shore is the land's; a headland to the north moves it down by its rows. Headland
	// plates stand on rock, never at the water's edge.
	const oy = (plan.landOffsetRows ?? 0) * T;
	const landShore = makeShore(cols * T, rows * T - oy);
	const shore = (y: number) => (y < oy ? Infinity : landShore(y - oy));
	const court = plan.garden.court;
	const solid = (col: number, row: number) =>
		col >= 0 && row >= 0 && col < cols && row < rows && tiles[row * cols + col] !== TileType.VOID;
	const inCourt = (col: number, row: number) =>
		col >= court.col &&
		row >= court.row &&
		col < court.col + court.width &&
		row < court.row + court.height;
	const pond = plan.garden.pond;
	const isPond = (col: number, row: number) =>
		col >= pond.col &&
		row >= pond.row &&
		col < pond.col + pond.width &&
		row < pond.row + pond.height;
	// One pass per open side: `along` walks the edge, `depth` goes inward.
	const sides: Array<{
		dc: number;
		dr: number;
		point: (x: number, y: number, along: number, depth: number) => [number, number];
	}> = [
		{ dc: 0, dr: -1, point: (x, y, a, d) => [x + a, y + d] },
		{ dc: 0, dr: 1, point: (x, y, a, d) => [x + a, y + T - 1 - d] },
		{ dc: -1, dr: 0, point: (x, y, a, d) => [x + d, y + a] },
		{ dc: 1, dr: 0, point: (x, y, a, d) => [x + T - 1 - d, y + a] }
	];
	for (let row = 0; row < rows; row += 1) {
		for (let col = 0; col < cols; col += 1) {
			if (!solid(col, row) || inCourt(col, row)) continue;
			const x = col * T;
			const y = row * T;
			for (const side of sides) {
				const ncol = col + side.dc;
				const nrow = row + side.dr;
				if (solid(ncol, nrow) || isPond(ncol, nrow)) continue;
				const [mx, my] = side.point(x, y, T / 2, 0);
				const water = mx > shore(my) - 6;
				for (let along = 0; along < T; along += 1) {
					const seed = (side.dc * 3 + side.dr * 7) * 1000;
					const h = hash(x + along * (side.dr ? 1 : 0), y + along * (side.dc ? 1 : 0), 30 + seed);
					if (water) {
						const [wx, wy] = side.point(x, y, along, 0);
						px(ctx, wx, wy, 1, 1, G.wetEdge);
						if (h < 0.18) {
							const [lx, ly] = side.point(x, y, along, 1);
							px(ctx, lx, ly, 1, 1, G.wetLight);
						}
						continue;
					}
					// A wavy depth of 1–3 pixels, with longer tufts now and then.
					const wave = Math.sin((along + (x + y) * 0.37) * 0.8) * 0.8;
					const depth = Math.max(1, Math.round(1.6 + wave + h * 1.4));
					for (let d = 0; d < depth; d += 1) {
						const [gx, gy] = side.point(x, y, along, d);
						px(ctx, gx, gy, 1, 1, d === depth - 1 ? G.edgeGrassDark : G.edgeGrass);
					}
					if (h > 0.86) {
						const [tx, ty] = side.point(x, y, along, depth);
						px(ctx, tx, ty, 1, 1, G.edgeGrassLight);
						const [ux, uy] = side.point(x, y, along, depth + 1);
						px(ctx, ux, uy, 1, 1, G.edgeMoss);
					} else if (h < 0.035) {
						const [fx, fy] = side.point(x, y, along, depth);
						px(ctx, fx, fy, 1, 1, G.edgeFlowers[Math.floor(h * 1000) % 3]);
					}
				}
			}
		}
	}
}

function paintZenGarden(ctx: Paint, plan: OfficePlan): void {
	const court = rectPx(plan.garden.court);
	const pond = rectPx(plan.garden.pond);
	const landmark = {
		x: plan.landmark.col * TILE_SIZE,
		y: plan.landmark.row * TILE_SIZE,
		w: 4 * TILE_SIZE,
		h: 4 * TILE_SIZE
	};
	const inCourt = (x: number, y: number) =>
		x >= court.x && y >= court.y && x < court.x + court.w && y < court.y + court.h;
	const inPond = (x: number, y: number, pad = 0) =>
		x >= pond.x - pad &&
		y >= pond.y - pad &&
		x < pond.x + pond.w + pad &&
		y < pond.y + pond.h + pad;

	// Raked gravel: fine stone with straight furrows every third row.
	px(ctx, court.x, court.y, court.w, court.h, G.gravel);
	for (let y = court.y + 2; y < court.y + court.h - 2; y += 3) {
		px(ctx, court.x + 3, y, court.w - 6, 1, G.rake);
	}
	for (let y = court.y; y < court.y + court.h; y += 2)
		for (let x = court.x; x < court.x + court.w; x += 2)
			if (hash(x, y, 4) < 0.05) px(ctx, x, y, 1, 1, G.gravelDot);

	// Stones set in the gravel, each with its own ripples raked around it.
	const stones: Array<[number, number, number]> = [
		[court.x + court.w - 70, court.y + court.h - 38, 7],
		[court.x + court.w - 26, court.y + 22, 5],
		[court.x + 26, court.y + 14, 4]
	];
	for (const [sx, sy, r] of stones) {
		disc(ctx, sx, sy, r + 16, (r + 16) * 0.72, G.gravel);
		for (let k = 1; k <= 4; k += 1) {
			for (const [x, y] of ringPixels(sx, sy, r + 3 + k * 4, (r + 3 + k * 4) * 0.72)) {
				if (inCourt(x, y) && !inPond(x, y, 3)) px(ctx, x, y, 1, 1, G.rake);
			}
		}
		disc(ctx, sx + 1, sy + r * 0.6, r + 2, r * 0.55, 'rgba(90, 96, 82, 0.22)');
		disc(ctx, sx, sy, r, r * 0.75, G.stoneDark);
		disc(ctx, sx - 0.5, sy - 1, r * 0.85, r * 0.6, G.stoneShade);
		disc(ctx, sx - r * 0.3, sy - r * 0.35, r * 0.4, r * 0.28, G.stone);
		disc(ctx, sx + r * 0.3, sy + r * 0.3, r * 0.5, r * 0.3, G.moss);
		px(ctx, sx + r * 0.1, sy + r * 0.2, 2, 1, G.mossLight);
	}

	// A rounded moss plinth grounds the unicorn landmark.
	disc(
		ctx,
		landmark.x + landmark.w / 2,
		landmark.y + landmark.h - 10,
		landmark.w / 2 + 6,
		13,
		G.mossDark
	);
	disc(
		ctx,
		landmark.x + landmark.w / 2,
		landmark.y + landmark.h - 12,
		landmark.w / 2 + 3,
		10,
		G.moss
	);
	for (let i = 0; i < 18; i += 1)
		px(
			ctx,
			landmark.x + 4 + hash(i, 1, 5) * (landmark.w - 8),
			landmark.y + landmark.h - 20 + hash(i, 2, 5) * 14,
			1,
			1,
			G.mossLight
		);

	// Moss softens the court's edge, thicker in the corners.
	for (let x = court.x; x < court.x + court.w; x += 1) {
		const top = 2 + Math.round(hash(x, 1, 6) * 2);
		const bottom = 2 + Math.round(hash(x, 2, 6) * 2);
		px(ctx, x, court.y, 1, top, G.moss);
		px(ctx, x, court.y + court.h - bottom, 1, bottom, G.moss);
		if (hash(x, 3, 6) < 0.3) px(ctx, x, court.y + top, 1, 1, G.mossDark);
	}
	for (let y = court.y; y < court.y + court.h; y += 1) {
		const leftW = 2 + Math.round(hash(y, 4, 6) * 2);
		const rightW = 2 + Math.round(hash(y, 5, 6) * 2);
		px(ctx, court.x, y, leftW, 1, G.moss);
		px(ctx, court.x + court.w - rightW, y, rightW, 1, G.moss);
	}
	for (const [cx, cy] of [
		[court.x, court.y],
		[court.x + court.w, court.y],
		[court.x, court.y + court.h],
		[court.x + court.w, court.y + court.h]
	])
		disc(ctx, cx, cy, 9, 7, G.moss);

	// Stepping stones cross the court along the walking line: irregular in
	// size and spacing, each on its own shadow with moss in the joints.
	const stepY = court.y + 6.5 * TILE_SIZE;
	for (let x = court.x + 8, i = 0; x < court.x + court.w - 10; i += 1) {
		const rx = 5 + hash(i, 1, 10) * 3;
		const ry = rx * (0.55 + hash(i, 2, 10) * 0.15);
		const y = stepY + Math.round((hash(i, 3, 10) - 0.5) * 7);
		if (!inPond(x, y, 6)) {
			disc(ctx, x + 1, y + 2, rx + 1, ry + 0.5, 'rgba(90, 96, 82, 0.22)');
			disc(ctx, x, y + 1, rx, ry, G.stepShade);
			disc(ctx, x, y, rx, ry, G.step);
			px(ctx, x - rx * 0.5, y - ry * 0.5, Math.max(2, rx * 0.6), 1, G.rakeLight);
			px(ctx, x + rx + 2, y + 1, 2, 1, G.mossLight);
		}
		x += rx * 2 + 5 + hash(i, 4, 10) * 5;
	}

	// The koi pond: a stone rim, depth, lily pads and a lantern at its corner.
	disc(
		ctx,
		pond.x + pond.w / 2,
		pond.y + pond.h / 2,
		pond.w / 2 + 3,
		pond.h / 2 + 3,
		G.pondRimDark
	);
	disc(
		ctx,
		pond.x + pond.w / 2,
		pond.y + pond.h / 2 - 1,
		pond.w / 2 + 2,
		pond.h / 2 + 2,
		G.pondRim
	);
	disc(ctx, pond.x + pond.w / 2, pond.y + pond.h / 2, pond.w / 2 - 1, pond.h / 2 - 1, G.pond);
	disc(
		ctx,
		pond.x + pond.w / 2 + 3,
		pond.y + pond.h / 2 + 3,
		pond.w / 2 - 9,
		pond.h / 2 - 8,
		G.pondDeep
	);
	for (let i = 0; i < 14; i += 1) {
		const a = (i / 14) * Math.PI * 2;
		const x = pond.x + pond.w / 2 + Math.cos(a) * (pond.w / 2 + 1);
		const y = pond.y + pond.h / 2 + Math.sin(a) * (pond.h / 2 + 1);
		disc(ctx, x, y, 3, 2, hash(i, 7, 7) < 0.5 ? G.stone : G.stoneShade);
	}
	for (const [dx, dy, flower] of [
		[14, 12, true],
		[22, 16, false],
		[pond.w - 16, pond.h - 12, false]
	] as Array<[number, number, boolean]>) {
		disc(ctx, pond.x + dx, pond.y + dy, 3, 2, G.lily);
		px(ctx, pond.x + dx + 1, pond.y + dy, 2, 1, G.pond);
		if (flower) px(ctx, pond.x + dx - 1, pond.y + dy - 1, 1, 1, G.lotus);
	}
	const lanternX = pond.x + pond.w - 4;
	const lanternY = pond.y - 2;
	disc(ctx, lanternX + 1, lanternY + 11, 6, 2, 'rgba(90, 96, 82, 0.25)');
	px(ctx, lanternX - 4, lanternY + 2, 9, 2, G.lanternShade);
	px(ctx, lanternX - 3, lanternY, 7, 2, G.lanternBody);
	px(ctx, lanternX - 2, lanternY + 4, 5, 4, G.lanternBody);
	px(ctx, lanternX - 1, lanternY + 5, 3, 2, G.lanternLight);
	px(ctx, lanternX - 1, lanternY + 8, 3, 3, G.lanternShade);
}

/** Light that falls through skylights onto the team pavilions (above rugs). */
export function paintOfficeLight(ctx: Paint, plan: OfficePlan): void {
	for (const zone of plan.zones) {
		if (zone.kind !== 'team') continue;
		const x = zone.col * TILE_SIZE;
		const y = zone.row * TILE_SIZE;
		for (const [dx, dy] of [
			[2.5, 3],
			[7, 5.5]
		]) {
			const left = x + dx * TILE_SIZE;
			const top = y + dy * TILE_SIZE;
			ctx.fillStyle = G.sun;
			// A slanted pane of light, stepped one art pixel per row.
			for (let r = 0; r < 20; r += 1) ctx.fillRect(Math.round(left + r * 0.5), top + r, 26, 1);
		}
	}
}

export interface GroundView {
	offsetX: number;
	offsetY: number;
	zoom: number;
	now: number;
	motion: boolean;
}

/** The pond's living part: two koi circling and a slow shimmer. */
export function drawGroundMotion(ctx: Paint, plan: OfficePlan, view: GroundView): void {
	const pond = rectPx(plan.garden.pond);
	const z = view.zoom;
	const t = view.motion ? view.now / 1000 : 0;
	const put = (x: number, y: number, w: number, h: number, color: string) => {
		ctx.fillStyle = color;
		ctx.fillRect(view.offsetX + Math.round(x) * z, view.offsetY + Math.round(y) * z, w * z, h * z);
	};
	const cx = pond.x + pond.w / 2;
	const cy = pond.y + pond.h / 2;
	// Shimmer: a highlight that drifts across the water surface.
	const sx = cx - 16 + ((t * 4) % 32);
	put(sx, cy - 6, 5, 1, G.pondLight);
	put(sx + 9, cy + 5, 3, 1, G.pondLight);
	for (const [radius, speed, offset, spotted] of [
		[14, 0.35, 0, true],
		[9, -0.5, 2.1, false]
	] as Array<[number, number, number, boolean]>) {
		const a = t * speed + offset;
		const x = cx + Math.cos(a) * radius;
		const y = cy + Math.sin(a) * radius * 0.55;
		// Heading follows the circle's tangent; the fish is 5 pixels long.
		const dx = -Math.sin(a) * Math.sign(speed);
		const dy = Math.cos(a) * Math.sign(speed) * 0.55;
		const horizontal = Math.abs(dx) >= Math.abs(dy);
		const hx = horizontal ? Math.sign(dx) : 0;
		const hy = horizontal ? 0 : Math.sign(dy);
		put(x + 1, y + 1, horizontal ? 5 : 2, horizontal ? 2 : 5, G.koiShadow);
		for (let i = 0; i < 4; i += 1) {
			const bx = x - hx * i;
			const by = y - hy * i;
			put(bx, by, horizontal ? 1 : 2, horizontal ? 2 : 1, i === 1 && spotted ? G.koiWhite : G.koi);
		}
		const tail = Math.floor(t * 4) % 2;
		put(x - hx * 4 + (horizontal ? 0 : tail), y - hy * 4 + (horizontal ? tail : 0), 1, 1, G.koi);
	}
}
