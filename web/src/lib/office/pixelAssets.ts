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
import { AMENITY_CATALOG, AMENITY_FRAMES, AMENITY_SPRITES } from './amenities';

const ASSET_BASE = '/vendor/pixel-agents/assets/';

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
	amenityFrames: Record<string, string[][][]>;
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

async function initializePixelAssets(): Promise<PixelOfficeAssets> {
	const catalogResponse = await fetch(`${ASSET_BASE}runtime-catalog.json`);
	if (!catalogResponse.ok) throw new Error('The office asset catalogue is unavailable.');
	const { index, catalog } = (await catalogResponse.json()) as RuntimeCatalog;
	const [characters, floors, walls, furniture, carpets, pets] = await Promise.all([
		decodeCharacters(index),
		decodeFloors(index),
		decodeWalls(index),
		decodeFurniture(catalog),
		decodeCarpets(),
		decodePets()
	]);

	setCharacterTemplates(characters);
	setFloorSprites(floors);
	setWallSprites(walls);
	setCarpetSprites(carpets);
	setPetTemplates(pets.frames, pets.names);
	if (
		!buildDynamicCatalog({
			catalog: [...catalog, ...AMENITY_CATALOG],
			sprites: {
				...furniture,
				...AMENITY_SPRITES
			}
		})
	) {
		throw new Error('The office furniture catalogue could not be loaded.');
	}
	return { amenityFrames: AMENITY_FRAMES };
}

/** Decode and register the upstream pack once per browser session. */
export function loadPixelOfficeAssets(): Promise<PixelOfficeAssets> {
	assetLoad ??= initializePixelAssets();
	return assetLoad;
}
