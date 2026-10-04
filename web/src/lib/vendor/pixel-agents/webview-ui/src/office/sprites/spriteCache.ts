import type { SpriteData } from '../types.js';

const zoomCaches = new Map<number, WeakMap<SpriteData, HTMLCanvasElement>>();

// ── Outline sprite generation ─────────────────────────────────

const outlineCache = new WeakMap<SpriteData, SpriteData>();

/** Generate a 1px white outline SpriteData (2px larger in each dimension) */
export function getOutlineSprite(sprite: SpriteData): SpriteData {
	const cached = outlineCache.get(sprite);
	if (cached) return cached;

	const rows = sprite.length;
	const cols = sprite[0].length;
	// Expanded grid: +2 in each dimension for 1px border
	const outline: string[][] = [];
	for (let r = 0; r < rows + 2; r++) {
		outline.push(new Array<string>(cols + 2).fill(''));
	}

	// For each opaque pixel, mark its 4 cardinal neighbors as white
	for (let r = 0; r < rows; r++) {
		for (let c = 0; c < cols; c++) {
			if (sprite[r][c] === '') continue;
			const er = r + 1;
			const ec = c + 1;
			if (outline[er - 1][ec] === '') outline[er - 1][ec] = '#FFFFFF';
			if (outline[er + 1][ec] === '') outline[er + 1][ec] = '#FFFFFF';
			if (outline[er][ec - 1] === '') outline[er][ec - 1] = '#FFFFFF';
			if (outline[er][ec + 1] === '') outline[er][ec + 1] = '#FFFFFF';
		}
	}

	// Clear pixels that overlap with original opaque pixels
	for (let r = 0; r < rows; r++) {
		for (let c = 0; c < cols; c++) {
			if (sprite[r][c] !== '') {
				outline[r + 1][c + 1] = '';
			}
		}
	}

	outlineCache.set(sprite, outline);
	return outline;
}

const baseCache = new WeakMap<SpriteData, HTMLCanvasElement | null>();
const packed = new Map<string, number>();
const littleEndian = new Uint8Array(new Uint32Array([1]).buffer)[0] === 1;
const HEX_COLOR = /^#[0-9a-f]{6}(?:[0-9a-f]{2})?$/i;

/** '#RRGGBB' or '#RRGGBBAA' as one RGBA pixel in ImageData order, or null. */
function packColor(color: string): number | null {
	let value = packed.get(color);
	if (value !== undefined) return value;
	if (!HEX_COLOR.test(color)) return null;
	const r = parseInt(color.slice(1, 3), 16);
	const g = parseInt(color.slice(3, 5), 16);
	const b = parseInt(color.slice(5, 7), 16);
	const a = color.length === 9 ? parseInt(color.slice(7, 9), 16) : 255;
	value = littleEndian
		? ((a << 24) | (b << 16) | (g << 8) | r) >>> 0
		: ((r << 24) | (g << 16) | (b << 8) | a) >>> 0;
	packed.set(color, value);
	return value;
}

/** The sprite at one art pixel per canvas pixel, written as ImageData. */
function baseSprite(sprite: SpriteData): HTMLCanvasElement | null {
	const known = baseCache.get(sprite);
	if (known !== undefined) return known;
	const rows = sprite.length;
	const cols = sprite[0].length;
	let result: HTMLCanvasElement | null = null;
	const image = new ImageData(cols, rows);
	const pixels = new Uint32Array(image.data.buffer);
	let parsed = true;
	for (let r = 0; r < rows && parsed; r++) {
		const row = sprite[r];
		for (let c = 0; c < cols; c++) {
			const color = row[c];
			if (!color) continue;
			const value = packColor(color);
			if (value === null) {
				parsed = false;
				break;
			}
			pixels[r * cols + c] = value;
		}
	}
	if (parsed) {
		const canvas = document.createElement('canvas');
		canvas.width = cols;
		canvas.height = rows;
		const context = canvas.getContext('2d');
		if (context) {
			context.putImageData(image, 0, 0);
			result = canvas;
		}
	}
	baseCache.set(sprite, result);
	return result;
}

export function getCachedSprite(sprite: SpriteData, zoom: number): HTMLCanvasElement {
	let cache = zoomCaches.get(zoom);
	if (!cache) {
		cache = new WeakMap();
		zoomCaches.set(zoom, cache);
	}

	const cached = cache.get(sprite);
	if (cached) return cached;

	const rows = sprite.length;
	const cols = sprite[0].length;
	const canvas = document.createElement('canvas');
	canvas.width = cols * zoom;
	canvas.height = rows * zoom;
	const ctx = canvas.getContext('2d')!;
	ctx.imageSmoothingEnabled = false;

	/* Restless: write the pixels once at one art pixel per canvas pixel and
	 * scale with a single nearest-neighbour blit, instead of a fillRect per
	 * pixel per zoom. Falls back to fillRect for colours it cannot parse. */
	const base = baseSprite(sprite);
	if (base) {
		ctx.drawImage(base, 0, 0, canvas.width, canvas.height);
	} else {
		for (let r = 0; r < rows; r++) {
			for (let c = 0; c < cols; c++) {
				const color = sprite[r][c];
				if (color === '') continue;
				ctx.fillStyle = color;
				ctx.fillRect(c * zoom, r * zoom, zoom, zoom);
			}
		}
	}

	cache.set(sprite, canvas);
	return canvas;
}
