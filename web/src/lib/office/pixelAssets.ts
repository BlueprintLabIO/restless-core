/**
 * Browser-side asset decoder adapted from Pixel Agents' browserMock.ts.
 * Restless keeps the engine and its tiny authored sprites, while the company
 * layout itself is projected from live Teams and Work in officePlan.ts.
 */
import { rgbaToHex } from '$lib/vendor/pixel-agents/core/src/assets/colorUtils.js';
import {
	CARPET_GRID_COLS,
	CARPET_MARCHING_SQUARES_COUNT,
	CARPET_TILE_SIZE,
	CHAR_FRAME_H,
	CHAR_FRAME_W,
	CHAR_FRAMES_PER_ROW,
	CHARACTER_DIRECTIONS,
	FLOOR_TILE_SIZE,
	PET_FRAME_H,
	PET_FRAME_W_LARGE,
	PET_FRAME_W_SMALL,
	WALL_BITMASK_COUNT,
	WALL_GRID_COLS,
	WALL_PIECE_HEIGHT,
	WALL_PIECE_WIDTH
} from '$lib/vendor/pixel-agents/core/src/assets/constants.js';
import type {
	AssetIndex,
	CatalogEntry,
	CharacterDirectionSprites,
	PetSpriteFrames
} from '$lib/vendor/pixel-agents/core/src/assets/types.js';
import { setFloorSprites } from '$lib/vendor/pixel-agents/webview-ui/src/office/floorTiles.js';
import { buildDynamicCatalog } from '$lib/vendor/pixel-agents/webview-ui/src/office/layout/furnitureCatalog.js';
import { setCarpetSprites } from '$lib/vendor/pixel-agents/webview-ui/src/office/sprites/carpetTiles.js';
import { setPetTemplates } from '$lib/vendor/pixel-agents/webview-ui/src/office/sprites/petSpriteData.js';
import { setCharacterTemplates } from '$lib/vendor/pixel-agents/webview-ui/src/office/sprites/spriteData.js';
import { setWallSprites } from '$lib/vendor/pixel-agents/webview-ui/src/office/wallTiles.js';
import { FOUNTAIN_TYPE, UNICORN_TYPE } from './officePlan';

const ASSET_BASE = '/vendor/pixel-agents/assets/';
const FOUNTAIN_PATH = '/office/fountain-atrium-frames.png';
const FOUNTAIN_FRAME_COUNT = 4;
const FOUNTAIN_SIZE = 64;
const UNICORN_PATH = '/office/unicorn-statue-frames.png';
const UNICORN_FRAME_COUNT = 4;
const UNICORN_SIZE = 32;

interface RuntimeCatalog {
	index: AssetIndex;
	catalog: CatalogEntry[];
}

interface DecodedPng {
	width: number;
	height: number;
	data: Uint8ClampedArray;
}

export interface PixelOfficeAssets {
	fountainFrames: string[][][];
	unicornFrames: string[][][];
}

let assetLoad: Promise<PixelOfficeAssets> | null = null;

function getPixel(
	data: Uint8ClampedArray,
	width: number,
	x: number,
	y: number
): [number, number, number, number] {
	const index = (y * width + x) * 4;
	return [data[index], data[index + 1], data[index + 2], data[index + 3]];
}

function readSprite(
	png: DecodedPng,
	width: number,
	height: number,
	offsetX = 0,
	offsetY = 0
): string[][] {
	const sprite: string[][] = [];
	for (let y = 0; y < height; y += 1) {
		const row: string[] = [];
		for (let x = 0; x < width; x += 1) {
			const [red, green, blue, alpha] = getPixel(png.data, png.width, offsetX + x, offsetY + y);
			row.push(rgbaToHex(red, green, blue, alpha));
		}
		sprite.push(row);
	}
	return sprite;
}

function readContainedSprite(
	png: DecodedPng,
	frameStart: number,
	frameEnd: number,
	targetSize: number
): string[][] {
	let minX = frameEnd;
	let minY = png.height;
	let maxX = frameStart;
	let maxY = 0;
	for (let y = 0; y < png.height; y += 1) {
		for (let x = frameStart; x < frameEnd; x += 1) {
			if (getPixel(png.data, png.width, x, y)[3] < 12) continue;
			minX = Math.min(minX, x);
			minY = Math.min(minY, y);
			maxX = Math.max(maxX, x);
			maxY = Math.max(maxY, y);
		}
	}
	if (maxX < minX || maxY < minY) {
		return Array.from({ length: targetSize }, () => new Array<string>(targetSize).fill(''));
	}

	const sourceWidth = maxX - minX + 1;
	const sourceHeight = maxY - minY + 1;
	const scale = Math.min((targetSize - 2) / sourceWidth, (targetSize - 2) / sourceHeight);
	const drawWidth = Math.max(1, Math.round(sourceWidth * scale));
	const drawHeight = Math.max(1, Math.round(sourceHeight * scale));
	const offsetX = Math.floor((targetSize - drawWidth) / 2);
	const offsetY = Math.floor((targetSize - drawHeight) / 2);
	const sprite = Array.from({ length: targetSize }, () => new Array<string>(targetSize).fill(''));
	for (let y = 0; y < drawHeight; y += 1) {
		for (let x = 0; x < drawWidth; x += 1) {
			const sourceX = minX + Math.min(sourceWidth - 1, Math.floor(x / scale));
			const sourceY = minY + Math.min(sourceHeight - 1, Math.floor(y / scale));
			const [red, green, blue, alpha] = getPixel(png.data, png.width, sourceX, sourceY);
			sprite[offsetY + y][offsetX + x] = rgbaToHex(red, green, blue, alpha);
		}
	}
	return sprite;
}

async function decodePng(path: string, base = ASSET_BASE): Promise<DecodedPng> {
	const response = await fetch(`${base}${path}`);
	if (!response.ok) throw new Error(`Office asset unavailable: ${path}`);
	const bitmap = await createImageBitmap(await response.blob());
	const canvas = document.createElement('canvas');
	canvas.width = bitmap.width;
	canvas.height = bitmap.height;
	const context = canvas.getContext('2d');
	if (!context) {
		bitmap.close();
		throw new Error('The office could not create a canvas renderer.');
	}
	context.drawImage(bitmap, 0, 0);
	bitmap.close();
	const image = context.getImageData(0, 0, canvas.width, canvas.height);
	return { width: canvas.width, height: canvas.height, data: image.data };
}

async function decodeCharacters(index: AssetIndex): Promise<CharacterDirectionSprites[]> {
	return Promise.all(
		index.characters.map(async (file) => {
			const png = await decodePng(`characters/${file}`);
			const directions: CharacterDirectionSprites = { down: [], up: [], right: [] };
			for (
				let directionIndex = 0;
				directionIndex < CHARACTER_DIRECTIONS.length;
				directionIndex += 1
			) {
				const direction = CHARACTER_DIRECTIONS[directionIndex];
				for (let frame = 0; frame < CHAR_FRAMES_PER_ROW; frame += 1) {
					directions[direction].push(
						readSprite(
							png,
							CHAR_FRAME_W,
							CHAR_FRAME_H,
							frame * CHAR_FRAME_W,
							directionIndex * CHAR_FRAME_H
						)
					);
				}
			}
			return directions;
		})
	);
}

async function decodeFloors(index: AssetIndex): Promise<string[][][]> {
	return Promise.all(
		index.floors.map(async (file) => {
			const png = await decodePng(`floors/${file}`);
			return readSprite(png, FLOOR_TILE_SIZE, FLOOR_TILE_SIZE);
		})
	);
}

async function decodeWalls(index: AssetIndex): Promise<string[][][][]> {
	return Promise.all(
		index.walls.map(async (file) => {
			const png = await decodePng(`walls/${file}`);
			return Array.from({ length: WALL_BITMASK_COUNT }, (_, mask) =>
				readSprite(
					png,
					WALL_PIECE_WIDTH,
					WALL_PIECE_HEIGHT,
					(mask % WALL_GRID_COLS) * WALL_PIECE_WIDTH,
					Math.floor(mask / WALL_GRID_COLS) * WALL_PIECE_HEIGHT
				)
			);
		})
	);
}

async function decodeFurniture(catalog: CatalogEntry[]): Promise<Record<string, string[][]>> {
	const decoded = await Promise.all(
		catalog.map(async (entry) => {
			const png = await decodePng(entry.furniturePath);
			return [entry.id, readSprite(png, entry.width, entry.height)] as const;
		})
	);
	return Object.fromEntries(decoded);
}

async function decodeCarpets(): Promise<string[][][][]> {
	return Promise.all(
		[0, 1, 2].map(async (variant) => {
			const png = await decodePng(`carpets/carpet_${variant}.png`);
			return Array.from({ length: CARPET_MARCHING_SQUARES_COUNT }, (_, mask) =>
				readSprite(
					png,
					CARPET_TILE_SIZE,
					CARPET_TILE_SIZE,
					(mask % CARPET_GRID_COLS) * CARPET_TILE_SIZE,
					Math.floor(mask / CARPET_GRID_COLS) * CARPET_TILE_SIZE
				)
			);
		})
	);
}

async function decodePets(): Promise<{ frames: PetSpriteFrames[]; names: string[] }> {
	const pets = [
		{ id: 'claudio', name: 'Claudio' },
		{ id: 'gitcat', name: 'Gitcat' }
	];
	const frames = await Promise.all(
		pets.map(async (pet): Promise<PetSpriteFrames> => {
			const png = await decodePng(`pets/${pet.id}/pet.png`);
			const smallRow = (y: number, offset: number) =>
				Array.from({ length: 3 }, (_, frame) =>
					readSprite(png, PET_FRAME_W_SMALL, PET_FRAME_H, offset + frame * PET_FRAME_W_SMALL, y)
				);
			return {
				walkDown: smallRow(0, 0),
				idleDown: smallRow(0, PET_FRAME_W_SMALL * 3),
				walkUp: smallRow(PET_FRAME_H, 0),
				idleUp: smallRow(PET_FRAME_H, PET_FRAME_W_SMALL * 3),
				walkRight: Array.from({ length: 3 }, (_, frame) =>
					readSprite(
						png,
						PET_FRAME_W_LARGE,
						PET_FRAME_H,
						frame * PET_FRAME_W_LARGE,
						PET_FRAME_H * 2
					)
				)
			};
		})
	);
	return { frames, names: pets.map((pet) => pet.name) };
}

async function decodeFountain(): Promise<string[][][]> {
	const png = await decodePng(FOUNTAIN_PATH, '');
	return Array.from({ length: FOUNTAIN_FRAME_COUNT }, (_, frame) => {
		const start = Math.floor((frame * png.width) / FOUNTAIN_FRAME_COUNT);
		const end = Math.floor(((frame + 1) * png.width) / FOUNTAIN_FRAME_COUNT);
		return readContainedSprite(png, start, end, FOUNTAIN_SIZE);
	});
}

async function decodeUnicorn(): Promise<string[][][]> {
	const png = await decodePng(UNICORN_PATH, '');
	return Array.from({ length: UNICORN_FRAME_COUNT }, (_, frame) => {
		const start = Math.floor((frame * png.width) / UNICORN_FRAME_COUNT);
		const end = Math.floor(((frame + 1) * png.width) / UNICORN_FRAME_COUNT);
		return readContainedSprite(png, start, end, UNICORN_SIZE);
	});
}

async function initializePixelAssets(): Promise<PixelOfficeAssets> {
	const catalogResponse = await fetch(`${ASSET_BASE}runtime-catalog.json`);
	if (!catalogResponse.ok) throw new Error('The office asset catalogue is unavailable.');
	const { index, catalog } = (await catalogResponse.json()) as RuntimeCatalog;
	const [characters, floors, walls, furniture, carpets, pets, fountainFrames, unicornFrames] =
		await Promise.all([
			decodeCharacters(index),
			decodeFloors(index),
			decodeWalls(index),
			decodeFurniture(catalog),
			decodeCarpets(),
			decodePets(),
			decodeFountain(),
			decodeUnicorn()
		]);

	const fountainEntry: CatalogEntry = {
		id: FOUNTAIN_TYPE,
		name: 'Atrium fountain',
		label: 'Atrium fountain',
		category: 'decor',
		file: 'fountain-atrium-frames.png',
		furniturePath: FOUNTAIN_PATH,
		width: FOUNTAIN_SIZE,
		height: FOUNTAIN_SIZE,
		footprintW: 4,
		footprintH: 4,
		isDesk: false,
		canPlaceOnWalls: false,
		backgroundTiles: 0,
		groupId: FOUNTAIN_TYPE
	};
	const unicornEntry: CatalogEntry = {
		id: UNICORN_TYPE,
		name: 'Unicorn statue',
		label: 'Unicorn statue',
		category: 'decor',
		file: 'unicorn-statue-frames.png',
		furniturePath: UNICORN_PATH,
		width: UNICORN_SIZE,
		height: UNICORN_SIZE,
		footprintW: 2,
		footprintH: 2,
		isDesk: false,
		canPlaceOnWalls: false,
		backgroundTiles: 0,
		groupId: UNICORN_TYPE
	};

	setCharacterTemplates(characters);
	setFloorSprites(floors);
	setWallSprites(walls);
	setCarpetSprites(carpets);
	setPetTemplates(pets.frames, pets.names);
	if (
		!buildDynamicCatalog({
			catalog: [...catalog, fountainEntry, unicornEntry],
			sprites: {
				...furniture,
				[FOUNTAIN_TYPE]: fountainFrames[0],
				[UNICORN_TYPE]: unicornFrames[0]
			}
		})
	) {
		throw new Error('The office furniture catalogue could not be loaded.');
	}
	return { fountainFrames, unicornFrames };
}

/** Decode and register the upstream pack once per browser session. */
export function loadPixelOfficeAssets(): Promise<PixelOfficeAssets> {
	assetLoad ??= initializePixelAssets();
	return assetLoad;
}
