#!/usr/bin/env node
/* Draws the office's floors, walls, carpets and furniture.
 *
 * Every pixel in these files is produced here, from the code below, so the art has one author and a
 * reproducible source: run `npm run office:art` and the PNGs under static/vendor/pixel-agents/assets
 * are rewritten byte for byte. File names, sizes and sheet layouts are the ones the engine already
 * loads (wall autotile 4x4 of 16x32, carpet marching-squares 4x4 of 16x16, 16x16 grey floors that
 * the engine recolours), so nothing else changes. Characters (MetroCity, CC0) and pets are not drawn
 * here. See web/static/office/README.md.
 *
 * No dependencies: a small PNG writer, a few drawing helpers, then the sprites. */

import { deflateSync, crc32 } from 'node:zlib';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const out = join(dirname(fileURLToPath(import.meta.url)), '../static/vendor/pixel-agents/assets');
const sheetMode = process.argv.includes('--sheet');

/* ── tiny raster toolkit ─────────────────────────────────────────────────── */

const rgb = (hex) => {
	const n = parseInt(hex.slice(1), 16);
	return [(n >> 16) & 255, (n >> 8) & 255, n & 255, 255];
};
const grey = (v) => [v, v, v, 255];

class Img {
	constructor(w, h) {
		this.w = w;
		this.h = h;
		this.d = new Uint8Array(w * h * 4);
	}
	set(x, y, c) {
		x = Math.round(x);
		y = Math.round(y);
		if (!c || x < 0 || y < 0 || x >= this.w || y >= this.h) return;
		const i = (y * this.w + x) * 4;
		this.d[i] = c[0];
		this.d[i + 1] = c[1];
		this.d[i + 2] = c[2];
		this.d[i + 3] = c[3];
	}
	get(x, y) {
		const i = (y * this.w + x) * 4;
		return [this.d[i], this.d[i + 1], this.d[i + 2], this.d[i + 3]];
	}
	rect(x, y, w, h, c) {
		for (let j = 0; j < h; j++) for (let i = 0; i < w; i++) this.set(x + i, y + j, c);
	}
	frame(x, y, w, h, c) {
		this.rect(x, y, w, 1, c);
		this.rect(x, y + h - 1, w, 1, c);
		this.rect(x, y, 1, h, c);
		this.rect(x + w - 1, y, 1, h, c);
	}
	disc(cx, cy, rx, ry, c) {
		for (let y = Math.floor(cy - ry); y <= Math.ceil(cy + ry); y++)
			for (let x = Math.floor(cx - rx); x <= Math.ceil(cx + rx); x++) {
				const dx = (x + 0.5 - cx) / rx;
				const dy = (y + 0.5 - cy) / ry;
				if (dx * dx + dy * dy <= 1) this.set(x, y, c);
			}
	}
	/* Paste another sprite, optionally mirrored. */
	blit(src, ox, oy, mirror = false) {
		for (let y = 0; y < src.h; y++)
			for (let x = 0; x < src.w; x++) {
				const c = src.get(mirror ? src.w - 1 - x : x, y);
				if (c[3]) this.set(ox + x, oy + y, c);
			}
	}
	/* Dark outline round every opaque pixel's empty 4-neighbours: the shared pixel-art edge. */
	outline(c) {
		const edge = [];
		for (let y = 0; y < this.h; y++)
			for (let x = 0; x < this.w; x++) {
				if (this.get(x, y)[3]) continue;
				const near = [
					[x - 1, y],
					[x + 1, y],
					[x, y - 1],
					[x, y + 1]
				].some(([a, b]) => a >= 0 && b >= 0 && a < this.w && b < this.h && this.get(a, b)[3]);
				if (near) edge.push([x, y]);
			}
		for (const [x, y] of edge) this.set(x, y, c);
		return this;
	}
	png() {
		const raw = Buffer.alloc((this.w * 4 + 1) * this.h);
		for (let y = 0; y < this.h; y++) {
			raw[y * (this.w * 4 + 1)] = 0;
			Buffer.from(this.d.buffer, y * this.w * 4, this.w * 4).copy(raw, y * (this.w * 4 + 1) + 1);
		}
		const chunk = (type, data) => {
			const head = Buffer.alloc(8);
			head.writeUInt32BE(data.length, 0);
			head.write(type, 4, 'ascii');
			const tail = Buffer.alloc(4);
			tail.writeUInt32BE(crc32(Buffer.concat([head.subarray(4), data])), 0);
			return Buffer.concat([head, data, tail]);
		};
		const ihdr = Buffer.alloc(13);
		ihdr.writeUInt32BE(this.w, 0);
		ihdr.writeUInt32BE(this.h, 4);
		ihdr[8] = 8;
		ihdr[9] = 6;
		return Buffer.concat([
			Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
			chunk('IHDR', ihdr),
			chunk('IDAT', deflateSync(raw, { level: 9 })),
			chunk('IEND', Buffer.alloc(0))
		]);
	}
}

const written = [];
function save(rel, img) {
	const path = join(out, rel);
	mkdirSync(dirname(path), { recursive: true });
	writeFileSync(path, img.png());
	written.push([rel, img]);
}

/* A repeatable tiny hash so "noise" is the same on every run. */
const noise = (x, y, seed = 0) => {
	let h = (x * 374761393 + y * 668265263 + seed * 2147483647) | 0;
	h = (h ^ (h >>> 13)) * 1274126177;
	return ((h ^ (h >>> 16)) >>> 0) / 4294967295;
};

/* ── palette ─────────────────────────────────────────────────────────────── */

const K = {
	ink: rgb('#262a37'),
	wood: [rgb('#ecd0a4'), rgb('#d2a974'), rgb('#ae8250'), rgb('#7f5c39'), rgb('#5b4129')],
	metal: [rgb('#eef2f8'), rgb('#bcc6d6'), rgb('#8996ac'), rgb('#586379'), rgb('#363e52')],
	teal: [rgb('#b4efe4'), rgb('#5cc2b4'), rgb('#2f918b'), rgb('#1e6266'), rgb('#143f47')],
	slate: [rgb('#8d9cb8'), rgb('#6a7b9a'), rgb('#4d5d7b'), rgb('#38455f'), rgb('#27324a')],
	green: [rgb('#a6e59d'), rgb('#5cbc6c'), rgb('#36904f'), rgb('#256a40'), rgb('#184830')],
	clay: [rgb('#f0b08a'), rgb('#d37d55'), rgb('#a85a3a'), rgb('#7a3f29')],
	paper: [rgb('#fbf7ec'), rgb('#e8e1cf'), rgb('#c7bea7')],
	glow: [rgb('#8cf0d6'), rgb('#4fc7b0'), rgb('#2e8f87')],
	amber: rgb('#f4c871'),
	rose: rgb('#e5748a'),
	sky: rgb('#7fb7e6'),
	night: rgb('#18202e')
};
const [W1, W2, W3, W4, W5] = K.wood;

/* ── floors: 16x16 greys the engine recolours ────────────────────────────── */

function floor(draw) {
	const img = new Img(16, 16);
	for (let y = 0; y < 16; y++) for (let x = 0; x < 16; x++) img.set(x, y, draw(x, y));
	return img;
}
const g = (base, x, y, seed, amount = 7) =>
	grey(Math.round(base + (noise(x, y, seed) - 0.5) * amount));

const FLOORS = [
	/* 0: smooth microcement */
	(x, y) => g(172, x, y, 1, 5),
	/* 1: wide planks, laid along the room */
	(x, y) => {
		const row = Math.floor(y / 4);
		const offset = [0, 7, 3, 10][row];
		const seam = (x + offset) % 16 === 0;
		if (y % 4 === 3) return grey(138);
		if (seam) return grey(150);
		return g(188 - row * 4 + (x + offset > 8 ? -5 : 3), x, y, 2, 9);
	},
	/* 2: narrow planks */
	(x, y) => {
		const col = Math.floor(x / 4);
		if (x % 4 === 3) return grey(141);
		if ((y + col * 5) % 16 === 0) return grey(152);
		return g(184 + (col % 2) * 8, x, y, 3, 8);
	},
	/* 3: large tile, soft grout */
	(x, y) =>
		x % 8 === 7 || y % 8 === 7 ? grey(138) : g(194 - (((x >> 3) + (y >> 3)) % 2) * 6, x, y, 4, 6),
	/* 4: checker */
	(x, y) => (((x >> 3) + (y >> 3)) % 2 === 0 ? g(205, x, y, 5, 4) : g(150, x, y, 5, 4)),
	/* 5: woven diagonal */
	(x, y) => {
		const d = (x + y) % 8;
		const e = (x - y + 16) % 8;
		if (d === 0 || e === 0) return grey(150);
		return g(d < 4 === e < 4 ? 190 : 178, x, y, 6, 5);
	},
	/* 6: running-bond brick */
	(x, y) => {
		const row = Math.floor(y / 4);
		const shift = row % 2 ? 4 : 0;
		if (y % 4 === 3) return grey(140);
		if ((x + shift) % 8 === 7) return grey(147);
		return g(180 + ((row * 3 + Math.floor((x + shift) / 8)) % 3) * 6, x, y, 7, 8);
	},
	/* 7: concrete with aggregate */
	(x, y) => {
		const n = noise(x, y, 8);
		return n > 0.93 ? grey(214) : n < 0.07 ? grey(134) : g(166, x, y, 9, 10);
	},
	/* 8: fine tile with a bevel */
	(x, y) => {
		if (x % 4 === 3 || y % 4 === 3) return grey(120);
		if (x % 4 === 0 || y % 4 === 0) return grey(222);
		return g(176, x, y, 10, 4);
	}
];
FLOORS.forEach((draw, i) => save(`floors/floor_${i}.png`, floor(draw)));

/* ── walls: 16 autotile pieces (N=1, E=2, S=4, W=8) of 16x32 ─────────────── */

function wallPiece(mask) {
	const img = new Img(16, 32);
	const N = mask & 1;
	const E = mask & 2;
	const S = mask & 4;
	const Wd = mask & 8;
	const capRows = S ? 32 : 8;
	for (let y = 0; y < capRows; y++) {
		const shade = !N && y < 3 ? 214 : !S && y >= 6 ? 190 : 203;
		for (let x = 0; x < 16; x++) img.set(x, y, grey(shade));
	}
	if (!S) {
		for (let y = 8; y < 31; y++) {
			const baseboard = y >= 27;
			for (let x = 0; x < 16; x++) {
				const v = baseboard
					? y === 27
						? 232
						: 214
					: y < 10
						? 246
						: 255 - Math.floor((y - 10) / 5);
				img.set(x, y, grey(v));
			}
		}
	}
	const edge = [48, 42, 40, 255];
	if (!N) img.rect(0, 0, 16, 1, edge);
	if (!S) img.rect(0, 31, 16, 1, edge);
	if (!Wd) img.rect(0, 0, 1, 32, edge);
	if (!E) img.rect(15, 0, 1, 32, edge);
	return img;
}
{
	const sheet = new Img(64, 128);
	for (let m = 0; m < 16; m++) sheet.blit(wallPiece(m), (m % 4) * 16, Math.floor(m / 4) * 32);
	save('walls/wall_0.png', sheet);
}

/* ── carpets: marching squares round a junction, two tones ───────────────── */

const MAIN = grey(100);
const ACCENT = grey(196);
function carpet(variant) {
	const sheet = new Img(64, 64);
	const border = 2;
	for (let m = 0; m < 16; m++) {
		const quads = [m & 1, m & 2, m & 4, m & 8]; // NW NE SE SW
		const [ox, oy] = [(m % 4) * 16, Math.floor(m / 4) * 16];
		const has = (qx, qy) => quads[qy === 0 ? (qx === 0 ? 0 : 1) : qx === 0 ? 3 : 2];
		for (let y = 0; y < 16; y++)
			for (let x = 0; x < 16; x++) {
				const qx = x < 8 ? 0 : 1;
				const qy = y < 8 ? 0 : 1;
				if (!has(qx, qy)) continue;
				const nearV = (qx === 0 ? x >= 8 - border : x < 8 + border) && !has(1 - qx, qy);
				const nearH = (qy === 0 ? y >= 8 - border : y < 8 + border) && !has(qx, 1 - qy);
				let c = MAIN;
				if (nearV || nearH) c = ACCENT;
				else if (variant === 1 && x % 4 === 1 && y % 4 === 1) c = ACCENT;
				else if (variant === 2 && (x + y) % 8 === 0) c = ACCENT;
				sheet.set(ox + x, oy + y, c);
			}
	}
	return sheet;
}
[0, 1, 2].forEach((v) => save(`carpets/carpet_${v}.png`, carpet(v)));

/* ── furniture ───────────────────────────────────────────────────────────── */

const sprite = (w, h, draw) => {
	const img = new Img(w, h);
	draw(img);
	return img;
};
const shadow = (img, x, y, w) => img.rect(x, y, w, 1, [38, 42, 55, 70]);

/* Wooden board seen from the front: lit top, shaded front, dark underside. */
function board(img, x, y, w, topH, frontH) {
	img.rect(x, y, w, topH, W2);
	img.rect(x, y, w, 1, W1);
	img.rect(x, y + topH, w, frontH, W3);
	img.rect(x, y + topH, w, 1, W2);
	img.rect(x, y + topH + frontH - 1, w, 1, W4);
}

const pot = (img, cx, bottom, w = 8, h = 6) => {
	img.rect(cx - w / 2, bottom - h, w, h, K.clay[1]);
	img.rect(cx - w / 2, bottom - h, w, 1, K.clay[0]);
	img.rect(cx - w / 2 + 1, bottom - 1, w - 2, 1, K.clay[3]);
	img.rect(cx - w / 2 + w - 2, bottom - h + 1, 1, h - 2, K.clay[2]);
};
const leaf = (img, cx, cy, rx, ry, tone = 1) => {
	img.disc(cx, cy, rx, ry, K.green[tone]);
	img.disc(cx - 0.5, cy - 0.5, rx * 0.5, ry * 0.45, K.green[tone - 1 < 0 ? 0 : tone - 1]);
};

const BIN = sprite(16, 16, (i) => {
	i.rect(5, 6, 6, 8, K.metal[2]);
	i.rect(4, 5, 8, 2, K.metal[1]);
	i.rect(5, 7, 1, 6, K.metal[1]);
	i.rect(10, 7, 1, 6, K.metal[3]);
	i.rect(6, 9, 4, 1, K.metal[3]);
	i.rect(6, 11, 4, 1, K.metal[3]);
	i.outline(K.ink);
	shadow(i, 4, 15, 8);
});

const shelfRow = (img, x, y, w, seed) => {
	img.rect(x, y + 6, w, 1, W3);
	let cx = x + 1;
	let n = 0;
	const tones = [K.teal[2], K.rose, K.amber, K.slate[1], K.green[2], K.paper[1], K.sky];
	while (cx < x + w - 2) {
		const bw = 1 + Math.floor(noise(cx, y, seed) * 2.2);
		const bh = 4 + Math.floor(noise(cx, y + 7, seed) * 3);
		const tone = tones[(n + seed) % tones.length];
		img.rect(cx, y + 6 - bh, bw, bh, tone);
		img.rect(cx, y + 6 - bh, bw, 1, [255, 255, 255, 70]);
		cx += bw;
		n++;
	}
};
const BOOKSHELF = sprite(32, 16, (i) => {
	i.rect(0, 1, 32, 14, W4);
	i.rect(1, 2, 30, 12, W5);
	shelfRow(i, 1, 1, 30, 1);
	shelfRow(i, 1, 8, 30, 4);
	i.rect(0, 14, 32, 1, W2);
	i.rect(0, 15, 32, 1, W3);
	i.outline(K.ink);
});
const DOUBLE_BOOKSHELF = sprite(32, 32, (i) => {
	i.rect(0, 2, 32, 28, W4);
	i.rect(1, 3, 30, 26, W5);
	for (const [n, y] of [
		[1, 2],
		[5, 9],
		[3, 16],
		[7, 23]
	])
		shelfRow(i, 1, y, 30, n);
	i.rect(0, 29, 32, 2, W3);
	i.rect(0, 29, 32, 1, W2);
	i.rect(15, 3, 2, 26, W4);
	i.outline(K.ink);
});

const CACTUS = sprite(16, 32, (i) => {
	i.rect(7, 8, 4, 16, K.green[2]);
	i.rect(7, 8, 1, 16, K.green[1]);
	i.rect(10, 9, 1, 15, K.green[3]);
	i.rect(3, 13, 4, 3, K.green[2]);
	i.rect(3, 10, 3, 4, K.green[2]);
	i.rect(11, 15, 3, 3, K.green[2]);
	i.rect(11, 12, 3, 4, K.green[2]);
	for (const [x, y] of [
		[8, 10],
		[9, 14],
		[8, 18],
		[4, 11],
		[12, 13]
	])
		i.set(x, y, K.paper[0]);
	i.rect(8, 6, 2, 1, K.rose);
	pot(i, 9, 31, 10, 8);
	i.outline(K.ink);
});

const CLOCK = sprite(16, 32, (i) => {
	i.disc(8, 14, 6.5, 6.5, K.paper[0]);
	i.disc(8, 14, 5.2, 5.2, K.paper[1]);
	i.disc(8, 14, 4.6, 4.6, K.paper[0]);
	i.rect(8, 10, 1, 5, K.ink);
	i.rect(8, 14, 3, 1, K.ink);
	i.set(8, 14, K.rose);
	for (const [x, y] of [
		[8, 9],
		[8, 19],
		[3, 14],
		[13, 14]
	])
		i.set(x, y, K.metal[3]);
	i.outline(K.ink);
});

const COFFEE = sprite(16, 16, (i) => {
	i.rect(5, 8, 6, 5, K.paper[0]);
	i.rect(10, 9, 2, 3, K.paper[1]);
	i.rect(11, 10, 1, 1, K.paper[0]);
	i.rect(6, 8, 4, 1, K.clay[3]);
	i.rect(5, 12, 6, 1, K.paper[2]);
	i.set(7, 5, [255, 255, 255, 140]);
	i.set(8, 4, [255, 255, 255, 110]);
	i.set(8, 6, [255, 255, 255, 140]);
	i.outline(K.ink);
});

const COFFEE_TABLE = sprite(32, 32, (i) => {
	i.rect(3, 29, 3, 2, W5);
	i.rect(26, 29, 3, 2, W5);
	i.rect(3, 22, 2, 8, W4);
	i.rect(27, 22, 2, 8, W4);
	board(i, 2, 6, 28, 16, 4);
	i.rect(8, 11, 16, 1, [255, 255, 255, 60]);
	i.outline(K.ink);
	shadow(i, 4, 31, 24);
});

const seatBench = (i, tone) => {
	i.rect(2, 7, 12, 5, tone[1]);
	i.rect(2, 7, 12, 1, tone[0]);
	i.rect(2, 11, 12, 1, tone[3]);
	i.rect(3, 12, 2, 3, W4);
	i.rect(11, 12, 2, 3, W4);
	i.outline(K.ink);
};
const CUSHIONED_BENCH = sprite(16, 16, (i) => seatBench(i, K.teal));
const WOODEN_BENCH = sprite(16, 16, (i) => {
	i.rect(2, 7, 12, 3, W2);
	i.rect(2, 7, 12, 1, W1);
	i.rect(2, 10, 12, 2, W3);
	i.rect(3, 12, 2, 3, W4);
	i.rect(11, 12, 2, 3, W4);
	i.outline(K.ink);
});

const stool = (view) =>
	sprite(16, 16, (i) => {
		const t = K.teal;
		if (view === 'back') {
			i.rect(3, 2, 10, 8, t[2]);
			i.rect(3, 2, 10, 1, t[1]);
			i.rect(4, 10, 8, 2, t[3]);
			i.rect(4, 12, 2, 3, K.metal[3]);
			i.rect(10, 12, 2, 3, K.metal[3]);
		} else if (view === 'side') {
			i.rect(8, 1, 5, 9, t[2]);
			i.rect(8, 1, 5, 1, t[1]);
			i.rect(3, 8, 9, 3, t[1]);
			i.rect(3, 8, 9, 1, t[0]);
			i.rect(4, 11, 7, 1, t[3]);
			i.rect(4, 12, 2, 3, K.metal[3]);
			i.rect(10, 12, 2, 3, K.metal[3]);
		} else {
			i.rect(3, 8, 10, 3, t[1]);
			i.rect(3, 8, 10, 1, t[0]);
			i.rect(3, 11, 10, 1, t[3]);
			i.rect(4, 12, 2, 3, K.metal[3]);
			i.rect(10, 12, 2, 3, K.metal[3]);
		}
		i.outline(K.ink);
	});

const woodChair = (view) =>
	sprite(16, 32, (i) => {
		const seatY = 20;
		if (view === 'back') {
			i.rect(3, 10, 10, 11, W3);
			i.rect(3, 10, 10, 1, W2);
			i.rect(5, 12, 1, 8, W4);
			i.rect(8, 12, 1, 8, W4);
			i.rect(11, 12, 1, 8, W4);
			i.rect(4, 22, 8, 2, W4);
			i.rect(4, 24, 2, 6, W4);
			i.rect(10, 24, 2, 6, W4);
		} else if (view === 'side') {
			i.rect(10, 10, 3, 14, W3);
			i.rect(10, 10, 3, 1, W2);
			i.rect(3, seatY, 10, 3, W2);
			i.rect(3, seatY, 10, 1, W1);
			i.rect(3, seatY + 3, 10, 1, W4);
			i.rect(3, 24, 2, 6, W4);
			i.rect(11, 24, 2, 6, W4);
		} else {
			i.rect(4, 22, 8, 3, W2);
			i.rect(4, 22, 8, 1, W1);
			i.rect(4, 25, 8, 1, W4);
			i.rect(4, 26, 2, 4, W4);
			i.rect(10, 26, 2, 4, W4);
			i.rect(3, 10, 1, 12, W4);
			i.rect(12, 10, 1, 12, W4);
		}
		i.outline(K.ink);
	});

const DESK_FRONT = sprite(48, 32, (i) => {
	i.rect(2, 26, 3, 5, K.metal[3]);
	i.rect(43, 26, 3, 5, K.metal[3]);
	board(i, 1, 5, 46, 14, 5);
	i.rect(1, 24, 46, 1, W4);
	i.rect(30, 20, 14, 5, W4);
	i.rect(31, 21, 12, 3, W3);
	i.rect(36, 22, 2, 1, K.amber);
	i.rect(3, 8, 10, 1, [255, 255, 255, 60]);
	i.outline(K.ink);
	shadow(i, 4, 31, 40);
});
const DESK_SIDE = sprite(16, 64, (i) => {
	i.rect(2, 2, 12, 56, W2);
	i.rect(2, 2, 12, 1, W1);
	i.rect(2, 58, 12, 4, W3);
	i.rect(2, 61, 12, 1, W4);
	i.rect(3, 62, 2, 2, K.metal[3]);
	i.rect(11, 62, 2, 2, K.metal[3]);
	i.outline(K.ink);
});

const frameArt = (i, x, y, w, h, scene) => {
	i.rect(x, y, w, h, W4);
	i.rect(x + 1, y + 1, w - 2, h - 2, W2);
	i.rect(x + 2, y + 2, w - 4, h - 4, K.paper[0]);
	scene(x + 2, y + 2, w - 4, h - 4);
};
const LARGE_PAINTING = sprite(32, 32, (i) => {
	frameArt(i, 1, 4, 30, 22, (x, y, w, h) => {
		i.rect(x, y, w, h, K.sky);
		i.rect(x, y + h * 0.55, w, h * 0.45, K.teal[2]);
		for (let k = 0; k < w; k++)
			i.rect(x + k, y + Math.round(h * 0.45 + Math.sin(k / 3) * 2), 1, 3, K.teal[3]);
		i.disc(x + w * 0.72, y + h * 0.28, 3, 3, K.amber);
		i.rect(x, y + h - 3, w, 3, K.green[3]);
	});
	i.outline(K.ink);
});
const smallPainting = (variant) =>
	sprite(16, 32, (i) => {
		frameArt(i, 2, 8, 12, 15, (x, y, w, h) => {
			if (variant === 1) {
				i.rect(x, y, w, h, K.slate[2]);
				i.disc(x + w / 2, y + h * 0.4, 3, 3, K.paper[0]);
				i.rect(x + 1, y + h * 0.7, w - 2, h * 0.3, K.paper[1]);
			} else {
				i.rect(x, y, w, h, K.clay[0]);
				i.rect(x, y + 1, w, 2, K.rose);
				i.rect(x, y + 5, w, 2, K.teal[1]);
				i.rect(x, y + 9, w, 2, K.slate[1]);
			}
		});
		i.outline(K.ink);
	});

const HANGING_PLANT = sprite(16, 32, (i) => {
	i.rect(8, 0, 1, 8, K.metal[3]);
	i.rect(4, 8, 8, 4, K.clay[1]);
	i.rect(4, 8, 8, 1, K.clay[0]);
	i.rect(5, 11, 6, 1, K.clay[3]);
	for (const [x, y, len] of [
		[4, 11, 14],
		[6, 11, 18],
		[8, 11, 12],
		[10, 11, 16],
		[12, 11, 10],
		[5, 11, 9]
	]) {
		for (let k = 0; k < len; k++)
			i.set(x + Math.round(Math.sin(k / 3 + x) * 1.2), y + k, K.green[k % 4 === 0 ? 1 : 2]);
		i.disc(x, y + len, 1.6, 1.6, K.green[1]);
	}
	leaf(i, 8, 9, 5.5, 2.6, 2);
	i.outline(K.ink);
});

const LARGE_PLANT = sprite(32, 48, (i) => {
	for (const [cx, cy, rx, ry, t] of [
		[16, 20, 11, 8, 3],
		[9, 24, 8, 7, 2],
		[23, 24, 8, 7, 2],
		[16, 14, 8, 8, 1],
		[16, 26, 10, 7, 2],
		[11, 14, 6, 5, 2],
		[21, 15, 6, 5, 2]
	])
		leaf(i, cx, cy, rx, ry, t);
	i.rect(15, 28, 2, 8, W4);
	pot(i, 16, 47, 16, 13);
	i.rect(7, 35, 18, 2, K.clay[0]);
	i.outline(K.ink);
});
const smallPlant = (kind) =>
	sprite(16, 32, (i) => {
		if (kind === 1) {
			for (const [dx, h] of [
				[-4, 11],
				[-2, 16],
				[0, 19],
				[2, 15],
				[4, 10]
			]) {
				for (let k = 0; k < h; k++) {
					const w = Math.max(1, Math.round(2.4 - (k / h) * 1.8));
					i.rect(
						8 + dx - Math.floor(w / 2) + Math.round((k / h) * dx * 0.6),
						23 - k,
						w,
						1,
						K.green[k % 5 === 0 ? 1 : 2]
					);
				}
			}
		} else {
			leaf(i, 8, 14, 6, 6, 2);
			leaf(i, 5, 18, 4, 4, 3);
			leaf(i, 11, 18, 4, 4, 3);
			leaf(i, 8, 10, 3.5, 3.5, 1);
		}
		pot(i, 8, 31, 9, 8);
		i.outline(K.ink);
	});
const POT = sprite(16, 16, (i) => {
	pot(i, 8, 14, 12, 9);
	i.rect(3, 6, 10, 1, K.clay[3]);
	i.rect(4, 5, 8, 1, W5);
	i.outline(K.ink);
	shadow(i, 3, 15, 10);
});

/* A desktop monitor and keyboard: the screen is the animated part. */
function monitor(view, screen) {
	return sprite(16, 32, (i) => {
		if (view === 'back') {
			i.rect(2, 6, 12, 12, K.metal[3]);
			i.rect(3, 7, 10, 10, K.metal[2]);
			i.rect(7, 18, 2, 3, K.metal[3]);
			i.rect(4, 21, 8, 2, K.metal[3]);
			i.rect(6, 10, 4, 4, K.metal[4]);
		} else if (view === 'side') {
			i.rect(5, 6, 5, 12, K.metal[2]);
			i.rect(5, 6, 1, 12, K.metal[1]);
			i.rect(10, 8, 2, 8, K.metal[3]);
			i.rect(7, 18, 2, 3, K.metal[3]);
			i.rect(4, 21, 8, 2, K.metal[3]);
			i.rect(3, 25, 10, 3, K.metal[1]);
		} else {
			i.rect(1, 5, 14, 12, K.metal[1]);
			i.rect(2, 6, 12, 9, K.night);
			i.rect(1, 15, 14, 2, K.metal[2]);
			i.set(13, 16, K.glow[0]);
			i.rect(7, 17, 2, 3, K.metal[3]);
			i.rect(4, 20, 8, 2, K.metal[2]);
			screen?.(i);
			i.rect(2, 25, 12, 4, K.metal[1]);
			i.rect(3, 26, 10, 1, K.metal[3]);
			i.rect(3, 27, 10, 1, K.metal[2]);
			i.rect(4, 28, 8, 0, K.metal[2]);
		}
		i.outline(K.ink);
	});
}
const screenFrame = (n) => (i) => {
	i.rect(3, 7, 10, 7, K.night);
	const lines = [
		[2, 6],
		[4, 8],
		[3, 5]
	];
	for (let k = 0; k < 3; k++) {
		const row = (k + n) % 3;
		i.rect(3 + 1, 8 + k * 2, lines[row][1], 1, k === n % 3 ? K.glow[0] : K.glow[2]);
	}
	i.set(11, 7 + (n % 3) * 2, K.amber);
};

const SMALL_TABLE_FRONT = sprite(32, 32, (i) => {
	i.rect(3, 24, 2, 7, K.metal[3]);
	i.rect(27, 24, 2, 7, K.metal[3]);
	board(i, 2, 6, 28, 14, 4);
	i.rect(2, 22, 28, 1, K.metal[3]);
	i.outline(K.ink);
	shadow(i, 4, 31, 24);
});
const SMALL_TABLE_SIDE = sprite(16, 48, (i) => {
	i.rect(2, 2, 12, 36, W2);
	i.rect(2, 2, 12, 1, W1);
	i.rect(2, 38, 12, 3, W3);
	i.rect(3, 41, 2, 6, K.metal[3]);
	i.rect(11, 41, 2, 6, K.metal[3]);
	i.outline(K.ink);
});

const sofa = (view) => {
	const s = K.slate;
	const tone = [K.teal[1], K.teal[2], K.teal[3]];
	if (view === 'side')
		return sprite(16, 32, (i) => {
			i.rect(8, 4, 6, 24, tone[1]);
			i.rect(8, 4, 6, 1, tone[0]);
			i.rect(3, 12, 6, 14, tone[0]);
			i.rect(3, 12, 6, 1, K.teal[0]);
			i.rect(3, 24, 11, 3, tone[2]);
			i.rect(3, 27, 2, 3, W5);
			i.rect(11, 27, 2, 3, W5);
			i.outline(K.ink);
		});
	return sprite(32, 16, (i) => {
		if (view === 'back') {
			i.rect(1, 2, 30, 11, tone[1]);
			i.rect(1, 2, 30, 1, tone[0]);
			i.rect(1, 11, 30, 2, tone[2]);
			i.rect(15, 3, 1, 8, tone[2]);
		} else {
			i.rect(1, 1, 30, 6, tone[1]);
			i.rect(1, 1, 30, 1, tone[0]);
			i.rect(3, 7, 26, 4, tone[0]);
			i.rect(3, 7, 26, 1, K.teal[0]);
			i.rect(15, 7, 1, 4, tone[2]);
			i.rect(0, 5, 3, 7, tone[2]);
			i.rect(29, 5, 3, 7, tone[2]);
			i.rect(1, 11, 30, 2, tone[2]);
		}
		i.rect(3, 13, 2, 2, W5);
		i.rect(27, 13, 2, 2, W5);
		i.outline(K.ink);
		void s;
	});
};

const TABLE_FRONT = sprite(48, 64, (i) => {
	i.rect(3, 55, 4, 8, K.metal[3]);
	i.rect(41, 55, 4, 8, K.metal[3]);
	board(i, 1, 4, 46, 48, 6);
	i.rect(3, 6, 42, 1, [255, 255, 255, 50]);
	i.rect(1, 52, 46, 1, W4);
	i.rect(8, 18, 12, 8, K.paper[0]);
	i.rect(9, 20, 8, 1, K.metal[2]);
	i.rect(9, 22, 6, 1, K.metal[2]);
	i.disc(34, 24, 3, 3, K.teal[1]);
	i.outline(K.ink);
	shadow(i, 4, 63, 40);
});

const WHITEBOARD = sprite(32, 32, (i) => {
	i.rect(1, 6, 30, 20, K.metal[1]);
	i.rect(2, 7, 28, 18, K.paper[0]);
	i.rect(2, 24, 28, 1, K.metal[2]);
	i.rect(4, 9, 12, 1, K.teal[2]);
	i.rect(4, 12, 20, 1, K.slate[1]);
	i.rect(4, 15, 16, 1, K.slate[1]);
	i.rect(20, 17, 6, 4, K.rose);
	i.rect(21, 18, 4, 2, K.paper[0]);
	i.rect(6, 26, 8, 2, K.metal[2]);
	i.rect(7, 26, 2, 1, K.rose);
	i.rect(10, 26, 2, 1, K.teal[2]);
	i.outline(K.ink);
});

/* ── write the furniture files ───────────────────────────────────────────── */

const F = (rel, img) => save(`furniture/${rel}.png`, img);
F('BIN/BIN', BIN);
F('BOOKSHELF/BOOKSHELF', BOOKSHELF);
F('CACTUS/CACTUS', CACTUS);
F('CLOCK/CLOCK', CLOCK);
F('COFFEE/COFFEE', COFFEE);
F('COFFEE_TABLE/COFFEE_TABLE', COFFEE_TABLE);
F('CUSHIONED_BENCH/CUSHIONED_BENCH', CUSHIONED_BENCH);
for (const v of ['FRONT', 'BACK', 'SIDE'])
	F(`CUSHIONED_CHAIR/CUSHIONED_CHAIR_${v}`, stool(v.toLowerCase()));
F('DESK/DESK_FRONT', DESK_FRONT);
F('DESK/DESK_SIDE', DESK_SIDE);
F('DOUBLE_BOOKSHELF/DOUBLE_BOOKSHELF', DOUBLE_BOOKSHELF);
F('HANGING_PLANT/HANGING_PLANT', HANGING_PLANT);
F('LARGE_PAINTING/LARGE_PAINTING', LARGE_PAINTING);
F('LARGE_PLANT/LARGE_PLANT', LARGE_PLANT);
F('PC/PC_BACK', monitor('back'));
F('PC/PC_SIDE', monitor('side'));
F('PC/PC_FRONT_OFF', monitor('front'));
[0, 1, 2].forEach((n) => F(`PC/PC_FRONT_ON_${n + 1}`, monitor('front', screenFrame(n))));
F('PLANT/PLANT', smallPlant(0));
F('PLANT_2/PLANT_2', smallPlant(1));
F('POT/POT', POT);
F('SMALL_PAINTING/SMALL_PAINTING', smallPainting(0));
F('SMALL_PAINTING_2/SMALL_PAINTING_2', smallPainting(1));
F('SMALL_TABLE/SMALL_TABLE_FRONT', SMALL_TABLE_FRONT);
F('SMALL_TABLE/SMALL_TABLE_SIDE', SMALL_TABLE_SIDE);
for (const v of ['FRONT', 'BACK', 'SIDE']) F(`SOFA/SOFA_${v}`, sofa(v.toLowerCase()));
F('TABLE_FRONT/TABLE_FRONT', TABLE_FRONT);
F('WHITEBOARD/WHITEBOARD', WHITEBOARD);
F('WOODEN_BENCH/WOODEN_BENCH', WOODEN_BENCH);
for (const v of ['FRONT', 'BACK', 'SIDE'])
	F(`WOODEN_CHAIR/WOODEN_CHAIR_${v}`, woodChair(v.toLowerCase()));

console.log(`wrote ${written.length} files under ${out}`);

/* `--sheet <file>` writes a magnified contact sheet for review. */
if (sheetMode) {
	const target = process.argv[process.argv.indexOf('--sheet') + 1];
	const scale = 3;
	const width = 1500;
	let x = 0;
	let y = 0;
	let rowH = 0;
	const placements = [];
	for (const [, img] of written) {
		const w = img.w * scale;
		const h = img.h * scale;
		if (x + w > width) {
			x = 0;
			y += rowH + 6;
			rowH = 0;
		}
		placements.push([img, x, y]);
		x += w + 6;
		rowH = Math.max(rowH, h);
	}
	const sheet = new Img(width, y + rowH + 4);
	sheet.rect(0, 0, sheet.w, sheet.h, rgb('#78909c'));
	for (const [img, px, py] of placements)
		for (let sy = 0; sy < img.h; sy++)
			for (let sx = 0; sx < img.w; sx++) {
				const c = img.get(sx, sy);
				if (c[3]) sheet.rect(px + sx * scale, py + sy * scale, scale, scale, c);
			}
	writeFileSync(target, sheet.png());
}
