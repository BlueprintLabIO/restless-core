import type { CatalogEntry } from '$lib/vendor/pixel-agents/core/src/assets/types.js';
import type {
	Direction,
	SpriteData
} from '$lib/vendor/pixel-agents/webview-ui/src/office/types.js';

export const CANOPY_TREE_TYPE = 'RESTLESS_CANOPY_TREE';
export const PANTRY_TYPE = 'RESTLESS_PANTRY';
export const FOCUS_NOOK_TYPE = 'RESTLESS_FOCUS_NOOK';
export const WELLNESS_NOOK_TYPE = 'RESTLESS_WELLNESS_NOOK';
export const PET_CORNER_TYPE = 'RESTLESS_PET_CORNER';
export const STORAGE_TYPE = 'RESTLESS_CARE_STORAGE';
export const UNICORN_TYPE = 'RESTLESS_UNICORN';
export const POOL_TABLE_TYPE = 'RESTLESS_POOL_TABLE';
export const GAME_TABLE_TYPE = 'RESTLESS_GAME_TABLE';
export const AQUARIUM_TYPE = 'RESTLESS_AQUARIUM';
export const ART_TABLE_TYPE = 'RESTLESS_ART_TABLE';
export const ARCADE_DUO_TYPE = 'RESTLESS_ARCADE_DUO';
export const RECORD_PLAYER_TYPE = 'RESTLESS_RECORD_PLAYER';
export const PICNIC_TABLE_TYPE = 'RESTLESS_PICNIC_TABLE';
export const HAMMOCK_TYPE = 'RESTLESS_HAMMOCK';
export const ROBOT_VACUUM_TYPE = 'RESTLESS_ROBOT_VACUUM';
export const GREENHOUSE_TYPE = 'RESTLESS_GREENHOUSE';
export const PROJECT_TABLE_TYPE = 'RESTLESS_PROJECT_TABLE';
export const LAKESIDE_DOCK_TYPE = 'RESTLESS_LAKESIDE_DOCK';

export type AmenityKind =
	| 'nourishment'
	| 'focus'
	| 'recovery'
	| 'reading'
	| 'social'
	| 'practical'
	| 'belonging'
	| 'garden'
	| 'play'
	| 'creative'
	| 'outdoors';

export interface AmenityDefinition {
	type: string;
	label: string;
	kind: AmenityKind;
	width: number;
	height: number;
	footprintW: number;
	footprintH: number;
	interactive: boolean;
	interactionOffsets: Array<{ dc: number; dr: number; facing: Direction; exclusive: boolean }>;
	frames: SpriteData[];
}

const transparent = '';

function sprite(width: number, height: number): SpriteData {
	return Array.from({ length: height }, () => new Array<string>(width).fill(transparent));
}

function rect(
	frame: SpriteData,
	x: number,
	y: number,
	width: number,
	height: number,
	color: string
): void {
	for (let row = Math.max(0, y); row < Math.min(frame.length, y + height); row += 1) {
		for (let col = Math.max(0, x); col < Math.min(frame[0].length, x + width); col += 1) {
			frame[row][col] = color;
		}
	}
}

function pixels(frame: SpriteData, points: Array<[number, number]>, color: string): void {
	for (const [x, y] of points) {
		if (frame[y]?.[x] !== undefined) frame[y][x] = color;
	}
}

function oval(
	frame: SpriteData,
	centerX: number,
	centerY: number,
	radiusX: number,
	radiusY: number,
	color: string
): void {
	for (let y = centerY - radiusY; y <= centerY + radiusY; y += 1) {
		for (let x = centerX - radiusX; x <= centerX + radiusX; x += 1) {
			const dx = (x - centerX) / radiusX;
			const dy = (y - centerY) / radiusY;
			if (dx * dx + dy * dy <= 1 && frame[y]?.[x] !== undefined) frame[y][x] = color;
		}
	}
}

// Keep the authored catalogue recognisably warm and handmade without letting
// old brown furniture dominate the cool Restless campus. This material pass is
// deliberately finite: it changes only known structural colours, preserving
// skin, food, flowers, game pieces and other small expressive accents.
const RESTLESS_MATERIAL_RECOLOR: Readonly<Record<string, string>> = {
	'#5f493c55': '#20263333',
	'#5f493c44': '#2026332e',
	'#76543f55': '#2026332e',
	'#80583e': '#405568',
	'#6e4632': '#405568',
	'#7b5139': '#506477',
	'#694432': '#405568',
	'#754934': '#405568',
	'#754d38': '#465e72',
	'#76513b': '#465e72',
	'#72513b': '#506477',
	'#7d543d': '#465e72',
	'#81533e': '#405568',
	'#895439': '#35495a',
	'#8d5c41': '#506477',
	'#9a603d': '#405568',
	'#9c6847': '#506a7e',
	'#9b6948': '#506a7e',
	'#a86f4b': '#61798b',
	'#a56f48': '#61798b',
	'#aa7149': '#4e8079',
	'#b5794f': '#647e91',
	'#b97b54': '#5f7e90',
	'#b77a49': '#61798b',
	'#c18a55': '#5f9b90',
	'#c28957': '#6f8998',
	'#c79255': '#8ba9a5',
	'#d0a06a': '#8fb2af',
	'#d3a367': '#7fb0a4',
	'#d59a64': '#7fa49f',
	'#d6aa6b': '#8ca8af',
	'#d7b06d': '#8ea9b7',
	'#d89b69': '#7f9faa',
	'#d8ae70': '#aac8bc',
	'#e0ad75': '#8fb2af',
	'#e0b06d': '#9ebfb3',
	'#e6ba7b': '#a7c8bd'
};

function recolorMaterials(frame: SpriteData): SpriteData {
	return frame.map((row) => row.map((color) => RESTLESS_MATERIAL_RECOLOR[color] ?? color));
}

function canopyFrame(glint = 0): SpriteData {
	const frame = sprite(64, 64);
	oval(frame, 32, 57, 20, 4, '#76543f55');
	rect(frame, 19, 52, 28, 6, '#9a6844');
	rect(frame, 22, 49, 22, 5, '#c79158');
	rect(frame, 29, 30, 7, 22, '#8d5b3c');
	rect(frame, 33, 31, 3, 20, '#b77b4e');
	rect(frame, 24, 23, 5, 15, '#8d5b3c');
	rect(frame, 38, 24, 5, 13, '#8d5b3c');
	oval(frame, 19, 25, 14, 12, '#2f755a');
	oval(frame, 32, 19, 18, 15, '#3d9068');
	oval(frame, 47, 27, 12, 11, '#32785d');
	oval(frame, 23, 13, 12, 10, '#52a774');
	oval(frame, 39, 9, 10, 8, '#65b47c');
	oval(frame, 9, 30, 7, 7, '#438d69');
	oval(frame, 54, 32, 6, 6, '#438d69');
	oval(frame, 33, 29, 15, 9, '#39835f');
	oval(frame, 29, 13, 8, 6, '#76be83');
	const glints: Array<Array<[number, number]>> = [
		[
			[20, 10],
			[43, 12],
			[13, 24]
		],
		[
			[23, 9],
			[47, 17],
			[15, 27]
		],
		[
			[27, 7],
			[50, 21],
			[18, 30]
		]
	];
	pixels(frame, glints[glint % glints.length], '#d5e98f');
	pixels(
		frame,
		[
			[12, 32],
			[19, 36],
			[43, 36],
			[52, 33],
			[31, 35]
		],
		'#245f4d'
	);
	rect(frame, 13, 55, 6, 2, '#e7c578');
	rect(frame, 47, 55, 5, 2, '#e7c578');
	return frame;
}

function pantryFrame(steam = 0): SpriteData {
	const frame = sprite(64, 32);
	rect(frame, 2, 7, 60, 22, '#80583e');
	rect(frame, 3, 5, 58, 5, '#d6aa6b');
	rect(frame, 4, 11, 17, 16, '#b5794f');
	rect(frame, 23, 11, 17, 16, '#c28957');
	rect(frame, 42, 11, 18, 16, '#a86f4b');
	rect(frame, 5, 13, 15, 2, '#e0ad75');
	rect(frame, 24, 13, 15, 2, '#e0ad75');
	rect(frame, 43, 13, 16, 2, '#d89b69');
	rect(frame, 46, 0, 10, 7, '#76b6bd');
	rect(frame, 48, 2, 6, 4, '#c9eef0');
	rect(frame, 25, 0, 11, 7, '#394d57');
	rect(frame, 27, 2, 7, 4, '#e9e0bf');
	rect(frame, 37, 3, 5, 4, '#fff0cf');
	rect(frame, 38, 2, 3, 2, '#b1d9d0');
	pixels(
		frame,
		[
			[8, 3],
			[11, 2],
			[14, 4]
		],
		'#e5b94f'
	);
	pixels(
		frame,
		[
			[9, 4],
			[12, 4],
			[15, 3]
		],
		'#dc7764'
	);
	if (steam === 0)
		pixels(
			frame,
			[
				[38, 0],
				[39, 1]
			],
			'#f7f1db'
		);
	if (steam === 1)
		pixels(
			frame,
			[
				[39, 0],
				[38, 1]
			],
			'#f7f1db'
		);
	if (steam === 2)
		pixels(
			frame,
			[
				[37, 0],
				[39, 1]
			],
			'#f7f1db'
		);
	return frame;
}

function focusNookFrame(): SpriteData {
	const frame = sprite(48, 48);
	rect(frame, 2, 4, 7, 38, '#6f8e8d');
	rect(frame, 39, 4, 7, 38, '#6f8e8d');
	rect(frame, 5, 7, 3, 32, '#a7c2ba');
	rect(frame, 40, 7, 3, 32, '#a7c2ba');
	rect(frame, 9, 24, 30, 7, '#b97b54');
	rect(frame, 11, 29, 3, 13, '#81533e');
	rect(frame, 34, 29, 3, 13, '#81533e');
	rect(frame, 18, 13, 15, 11, '#354b59');
	rect(frame, 20, 15, 11, 7, '#7cb9be');
	rect(frame, 24, 24, 3, 5, '#354b59');
	rect(frame, 18, 39, 12, 6, '#d59a64');
	rect(frame, 20, 37, 8, 3, '#e6ba7b');
	pixels(
		frame,
		[
			[13, 20],
			[14, 18],
			[14, 22]
		],
		'#65a668'
	);
	return frame;
}

function wellnessFrame(light = 0): SpriteData {
	const frame = sprite(48, 32);
	rect(frame, 2, 19, 44, 10, '#8db4a7');
	rect(frame, 5, 21, 38, 6, '#bed6bd');
	rect(frame, 8, 13, 11, 8, '#dca68a');
	rect(frame, 10, 11, 8, 5, '#edc2a5');
	rect(frame, 27, 16, 13, 7, '#809eb2');
	rect(frame, 29, 14, 9, 4, '#adc7d0');
	rect(frame, 22, 8, 5, 12, '#a56f48');
	rect(frame, 20, 6, 9, 4, '#d6a864');
	const glow = ['#f4dc83', '#fff0a8', '#ead272'][light % 3];
	rect(frame, 22, 3, 5, 4, glow);
	pixels(
		frame,
		[
			[23, 1],
			[26, 0],
			[28, 2]
		],
		'#f7eed2'
	);
	return frame;
}

function poolTableFrame(turn = 0): SpriteData {
	const frame = sprite(64, 48);
	oval(frame, 32, 43, 27, 3, '#5f493c55');
	rect(frame, 5, 7, 54, 31, '#6e4632');
	rect(frame, 8, 10, 48, 25, '#2c765f');
	rect(frame, 11, 13, 42, 19, '#3d9772');
	rect(frame, 8, 8, 48, 3, '#c79255');
	rect(frame, 8, 35, 48, 3, '#9a603d');
	rect(frame, 5, 10, 3, 25, '#b77a49');
	rect(frame, 56, 10, 3, 25, '#895439');
	for (const [x, y] of [
		[9, 11],
		[54, 11],
		[9, 33],
		[54, 33]
	] as Array<[number, number]>)
		oval(frame, x, y, 2, 2, '#203e38');
	const ballFrames: Array<Array<[number, number]>> = [
		[
			[23, 19],
			[41, 26],
			[44, 23]
		],
		[
			[25, 20],
			[39, 25],
			[44, 23]
		],
		[
			[28, 21],
			[37, 24],
			[44, 23]
		]
	];
	oval(
		frame,
		ballFrames[turn % ballFrames.length][0][0],
		ballFrames[turn % 3][0][1],
		1,
		1,
		'#fff7dd'
	);
	oval(
		frame,
		ballFrames[turn % ballFrames.length][1][0],
		ballFrames[turn % 3][1][1],
		1,
		1,
		'#f0c343'
	);
	oval(
		frame,
		ballFrames[turn % ballFrames.length][2][0],
		ballFrames[turn % 3][2][1],
		1,
		1,
		'#d76558'
	);
	const cueY = 15 + (turn % 3);
	for (let index = 0; index < 28; index += 1) {
		const x = 4 + index;
		const y = cueY + Math.floor(index / 9);
		if (frame[y]?.[x] !== undefined) frame[y][x] = index < 4 ? '#f5eee0' : '#d2a66c';
	}
	rect(frame, 9, 38, 5, 6, '#754934');
	rect(frame, 50, 38, 5, 6, '#754934');
	return frame;
}

function gameTableFrame(turn = 0): SpriteData {
	const frame = sprite(48, 32);
	oval(frame, 24, 28, 18, 3, '#5f493c55');
	rect(frame, 4, 5, 40, 21, '#7b5139');
	rect(frame, 7, 7, 34, 16, '#d8ae70');
	rect(frame, 9, 9, 30, 12, '#f0d99d');
	rect(frame, 8, 24, 5, 5, '#694432');
	rect(frame, 35, 24, 5, 5, '#694432');
	const tokenColors = ['#5f91be', '#d46f5e', '#62a474', '#e0b84c'];
	const tokenPoints: Array<Array<[number, number]>> = [
		[
			[16, 13],
			[30, 17],
			[22, 18],
			[33, 11]
		],
		[
			[18, 14],
			[29, 16],
			[24, 17],
			[32, 12]
		],
		[
			[20, 15],
			[27, 15],
			[26, 16],
			[31, 13]
		]
	];
	tokenPoints[turn % tokenPoints.length].forEach(([x, y], index) => {
		oval(frame, x, y, 1, 1, tokenColors[index]);
	});
	rect(frame, 11, 10, 6, 4, '#f8f2dc');
	rect(frame, 12, 11, 2, 1, '#5f91be');
	rect(frame, 32, 17, 5, 3, '#f8f2dc');
	return frame;
}

function aquariumFrame(turn = 0): SpriteData {
	const frame = sprite(48, 48);
	oval(frame, 24, 44, 19, 3, '#5f493c55');
	rect(frame, 4, 5, 40, 31, '#47646b');
	rect(frame, 7, 8, 34, 24, '#74bac0');
	rect(frame, 9, 10, 30, 20, '#a3d6ce');
	rect(frame, 4, 35, 40, 6, '#9b6948');
	rect(frame, 8, 40, 5, 5, '#76513b');
	rect(frame, 35, 40, 5, 5, '#76513b');
	for (const [x, y] of [
		[12, 27],
		[17, 25],
		[29, 28],
		[35, 25]
	] as Array<[number, number]>)
		oval(frame, x, y, 2, 1, '#d8ad6d');
	for (const [x, y] of [
		[14, 18],
		[25, 14],
		[34, 21]
	] as Array<[number, number]>) {
		const swim = turn % 2 ? 1 : -1;
		oval(frame, x + swim, y, 3, 2, x === 25 ? '#e88376' : '#f4cc66');
		pixels(frame, [[x - 3 + swim, y]], '#537a87');
	}
	pixels(
		frame,
		[
			[37, 12],
			[36, 10],
			[37, 8]
		],
		'#e7f5e9'
	);
	return frame;
}

function artTableFrame(turn = 0): SpriteData {
	const frame = sprite(48, 32);
	oval(frame, 24, 29, 18, 2, '#5f493c44');
	rect(frame, 3, 9, 42, 15, '#9c6847');
	rect(frame, 6, 7, 36, 5, '#d3a367');
	rect(frame, 7, 23, 5, 6, '#754d38');
	rect(frame, 36, 23, 5, 6, '#754d38');
	rect(frame, 10, 11, 12, 9, '#f5edd7');
	pixels(
		frame,
		[
			[13, 15],
			[14, 14],
			[15, 15],
			[16, 13],
			[17, 14]
		],
		'#65a47b'
	);
	rect(frame, 26, 11, 8, 6, '#7da2c5');
	rect(frame, 28, 9, 2, 3, '#e6bd4e');
	rect(frame, 34, 13, 4, 5, '#df806f');
	const pencilX = 22 + (turn % 3);
	rect(frame, pencilX, 17, 8, 1, '#f0c454');
	return frame;
}

function arcadeDuoFrame(turn = 0): SpriteData {
	const frame = sprite(48, 48);
	oval(frame, 24, 45, 19, 2, '#5f493c55');
	for (const offset of [3, 25]) {
		rect(frame, offset, 5, 19, 38, offset === 3 ? '#587899' : '#a56582');
		rect(frame, offset + 2, 7, 15, 15, '#243948');
		rect(frame, offset + 4, 9, 11, 10, turn % 2 ? '#6cc0b4' : '#efc65d');
		rect(frame, offset + 3, 25, 13, 7, '#d7b06d');
		pixels(
			frame,
			[
				[offset + 7, 28],
				[offset + 12, 28]
			],
			'#243948'
		);
		rect(frame, offset + 3, 34, 13, 7, '#3d4f5c');
	}
	pixels(
		frame,
		[
			[10, 12],
			[14, 15],
			[32, 13],
			[37, 11]
		],
		'#fff2c7'
	);
	return frame;
}

function recordPlayerFrame(turn = 0): SpriteData {
	const frame = sprite(32, 32);
	oval(frame, 16, 29, 11, 2, '#5f493c55');
	rect(frame, 3, 14, 26, 14, '#8d5c41');
	rect(frame, 5, 11, 22, 5, '#d0a06a');
	oval(frame, 15, 13, 7, 4, '#304653');
	oval(frame, 15, 13, 2, 1, turn % 2 ? '#e1766e' : '#f0c254');
	rect(frame, 23, 7, 2, 8, '#d9c490');
	pixels(
		frame,
		[
			[25, 6],
			[27, 4],
			[29, 6],
			[28, 8]
		],
		'#6a8bb3'
	);
	pixels(
		frame,
		turn % 2
			? [
					[8, 5],
					[6, 3]
				]
			: [
					[9, 4],
					[7, 2]
				],
		'#e1766e'
	);
	return frame;
}

function picnicTableFrame(turn = 0): SpriteData {
	const frame = sprite(64, 48);
	oval(frame, 32, 44, 27, 3, '#5f493c44');
	rect(frame, 8, 13, 48, 12, '#c18a55');
	rect(frame, 11, 10, 42, 5, '#e0b06d');
	rect(frame, 13, 24, 6, 18, '#7d543d');
	rect(frame, 45, 24, 6, 18, '#7d543d');
	rect(frame, 3, 31, 58, 5, '#aa7149');
	rect(frame, 18, 12, 11, 7, '#f7edd2');
	pixels(
		frame,
		[
			[20, 14],
			[23, 15],
			[26, 14]
		],
		'#de796d'
	);
	rect(frame, 36, 11, 9, 7, '#7eb08b');
	pixels(
		frame,
		turn % 2
			? [
					[40, 8],
					[43, 7]
				]
			: [
					[39, 7],
					[42, 8]
				],
		'#f7efd2'
	);
	return frame;
}

function hammockFrame(turn = 0): SpriteData {
	const frame = sprite(48, 32);
	rect(frame, 3, 5, 4, 25, '#72513b');
	rect(frame, 41, 5, 4, 25, '#72513b');
	pixels(
		frame,
		[
			[2, 6],
			[5, 2],
			[8, 7],
			[39, 7],
			[42, 2],
			[45, 6]
		],
		'#4b865d'
	);
	for (let index = 0; index < 32; index += 1) {
		const dip = Math.round(Math.sin((index / 31) * Math.PI) * (turn % 2 ? 7 : 8));
		const y = 13 + dip;
		if (frame[y]?.[8 + index] !== undefined) frame[y][8 + index] = '#e5a887';
		if (frame[y + 1]?.[8 + index] !== undefined) frame[y + 1][8 + index] = '#f1c3a1';
	}
	return frame;
}

function robotVacuumFrame(turn = 0): SpriteData {
	const frame = sprite(16, 16);
	oval(frame, 8, 11, 6, 4, '#42535c');
	oval(frame, 8, 10, 5, 3, '#83999b');
	pixels(frame, [[8, 8]], turn % 2 ? '#e5796e' : '#74c29a');
	pixels(
		frame,
		turn % 3 === 0
			? [
					[14, 11],
					[15, 10]
				]
			: [
					[1, 12],
					[0, 11]
				],
		'#d6bc77'
	);
	return frame;
}

function petCornerFrame(): SpriteData {
	const frame = sprite(32, 32);
	rect(frame, 2, 17, 20, 11, '#d28669');
	rect(frame, 5, 19, 14, 6, '#f0b99d');
	rect(frame, 23, 21, 7, 6, '#75aeba');
	rect(frame, 24, 22, 5, 2, '#d8edf0');
	rect(frame, 4, 8, 12, 8, '#caa05f');
	pixels(
		frame,
		[
			[7, 10],
			[13, 10],
			[10, 12],
			[8, 14],
			[12, 14]
		],
		'#fff0d3'
	);
	pixels(
		frame,
		[
			[27, 10],
			[29, 12],
			[25, 13],
			[28, 15]
		],
		'#e2a547'
	);
	return frame;
}

function storageFrame(): SpriteData {
	const frame = sprite(48, 32);
	rect(frame, 2, 3, 28, 27, '#72939b');
	rect(frame, 4, 5, 11, 22, '#9bb7b5');
	rect(frame, 17, 5, 11, 22, '#a9c3bc');
	rect(frame, 8, 8, 3, 2, '#e7d6a8');
	rect(frame, 21, 8, 3, 2, '#e7d6a8');
	rect(frame, 33, 12, 12, 17, '#477f69');
	rect(frame, 35, 15, 8, 11, '#6fad82');
	pixels(
		frame,
		[
			[37, 18],
			[40, 18],
			[38, 21],
			[41, 22]
		],
		'#e8f0d5'
	);
	return frame;
}

function unicornFrame(glint = 0): SpriteData {
	const frame = sprite(64, 64);
	oval(frame, 32, 59, 24, 4, '#20263344');
	// A substantial cool-stone garden plinth makes this the campus landmark.
	rect(frame, 7, 51, 50, 8, '#65778a');
	rect(frame, 10, 47, 44, 6, '#aebdca');
	rect(frame, 13, 44, 38, 5, '#dde5ea');
	rect(frame, 16, 42, 32, 3, '#f5f8f7');
	// Body, legs, chest and lifted head.
	oval(frame, 30, 33, 15, 10, '#f8fbf8');
	oval(frame, 40, 29, 9, 11, '#f8fbf8');
	rect(frame, 39, 16, 9, 17, '#f8fbf8');
	oval(frame, 47, 14, 10, 7, '#f8fbf8');
	rect(frame, 49, 13, 11, 5, '#d8e3e7');
	rect(frame, 20, 38, 6, 8, '#d8e3e7');
	rect(frame, 36, 38, 6, 8, '#d8e3e7');
	// Ear, horn and eye.
	pixels(
		frame,
		[
			[43, 6],
			[44, 7],
			[55, 7],
			[54, 8],
			[53, 9]
		],
		'#d8a6cc'
	);
	pixels(
		frame,
		[
			[56, 2],
			[55, 4],
			[54, 6],
			[53, 8]
		],
		'#d2a554'
	);
	pixels(frame, [[50, 13]], '#202633');
	// A Restless violet/blue/teal mane and tail replace the warmer rainbow.
	for (const [x, y, color] of [
		[40, 16, '#7b66b2'],
		[38, 19, '#5d8fc1'],
		[37, 23, '#4e9a7e'],
		[36, 27, '#7b66b2'],
		[15, 29, '#7b66b2'],
		[12, 31, '#5d8fc1'],
		[10, 34, '#4e9a7e']
	] as Array<[number, number, string]>) {
		oval(frame, x, y, 3, 3, color);
	}
	// Flowers bind the plinth to the zen garden.
	for (const [x, color] of [
		[12, '#7b66b2'],
		[20, '#5d8fc1'],
		[44, '#4e9a7e'],
		[52, '#d2a554']
	] as Array<[number, string]>) {
		pixels(
			frame,
			[
				[x, 48],
				[x - 1, 49],
				[x + 1, 49]
			],
			color
		);
		pixels(frame, [[x, 50]], '#4e9a7e');
	}
	const sparkles: Array<Array<[number, number]>> = [
		[[8, 16]],
		[
			[11, 12],
			[8, 15],
			[11, 18]
		],
		[[14, 14]],
		[
			[11, 12],
			[9, 14],
			[13, 14],
			[11, 16]
		]
	];
	pixels(frame, sparkles[glint % sparkles.length], '#d2a554');
	return frame;
}

function greenhouseFrame(turn = 0): SpriteData {
	const frame = sprite(80, 64);
	oval(frame, 40, 60, 34, 3, '#20263333');
	// Cool blue glass, white framing and a shallow planted base.
	rect(frame, 5, 19, 70, 37, '#506a7e');
	rect(frame, 8, 21, 64, 31, '#9fd6d5cc');
	rect(frame, 10, 23, 60, 27, '#c9ece5aa');
	for (const x of [8, 24, 40, 56, 72]) rect(frame, x, 18, 3, 37, '#dde5ea');
	rect(frame, 5, 52, 70, 6, '#65778a');
	for (let index = 0; index < 34; index += 1) {
		const roofY = 18 - Math.round(Math.abs(index - 17) * 0.42);
		if (frame[roofY]?.[6 + index * 2] !== undefined) frame[roofY][6 + index * 2] = '#eef6f4';
	}
	for (const [x, y, color] of [
		[16, 43, '#4e9a7e'],
		[28, 39, '#72b477'],
		[51, 42, '#3f795b'],
		[63, 38, '#78b786']
	] as Array<[number, number, string]>) {
		rect(frame, x - 3, y, 7, 8, '#7b66b2');
		oval(frame, x, y - 4, 8, 7, color);
	}
	const glintX = [18, 34, 50][turn % 3];
	pixels(
		frame,
		[
			[glintX, 25],
			[glintX + 1, 24],
			[glintX + 2, 23]
		],
		'#f8fbf8'
	);
	return frame;
}

function projectTableFrame(turn = 0): SpriteData {
	const frame = sprite(80, 48);
	oval(frame, 40, 44, 32, 3, '#20263333');
	rect(frame, 6, 12, 68, 22, '#647e91');
	rect(frame, 9, 9, 62, 8, '#d8e3e7');
	rect(frame, 12, 11, 56, 4, '#eef4f3');
	for (const x of [12, 62]) rect(frame, x, 31, 6, 12, '#506477');
	rect(frame, 18, 15, 16, 11, '#5d8fc1');
	rect(frame, 20, 17, 12, 7, turn % 2 ? '#8dc9c3' : '#7b66b2');
	rect(frame, 43, 16, 15, 10, '#202633');
	rect(frame, 45, 18, 11, 6, '#68b6ba');
	rect(frame, 37, 13, 3, 12, '#65778a');
	for (const [x, y, color] of [
		[25, 29, '#d2a554'],
		[39, 28, '#7b66b2'],
		[53, 30, '#4e9a7e']
	] as Array<[number, number, string]>)
		oval(frame, x, y, 2, 2, color);
	return frame;
}

function lakesideDockFrame(turn = 0): SpriteData {
	const frame = sprite(96, 64);
	// Painted boardwalk: warm enough to read as timber without dominating brown.
	rect(frame, 3, 18, 78, 34, '#61798b');
	for (let y = 20; y < 50; y += 6) rect(frame, 6, y, 72, 3, '#aebdca');
	for (let x = 8; x < 80; x += 12) rect(frame, x, 19, 2, 32, '#d8e3e7');
	rect(frame, 7, 48, 5, 12, '#405568');
	rect(frame, 71, 48, 5, 12, '#405568');
	// Telescope and a tiny moored boat.
	rect(frame, 25, 8, 4, 25, '#202633');
	rect(frame, 18, 8, 18, 6, '#5d8fc1');
	rect(frame, 15, 10, 7, 3, '#68b6ba');
	oval(frame, 85, 42, 10, 15, '#7b66b2');
	oval(frame, 85, 41, 7, 12, '#d8e3e7');
	rect(frame, 84, 28, 2, 24, '#65778a');
	const wave = turn % 2 ? '#cfeaec' : '#9bd4d2';
	for (const [x, y, width] of [
		[80, 16, 12],
		[76, 56, 17],
		[2, 58, 14]
	] as Array<[number, number, number]>)
		rect(frame, x, y, width, 2, wave);
	return frame;
}

export const AMENITY_DEFINITIONS: AmenityDefinition[] = (
	[
		{
			type: CANOPY_TREE_TYPE,
			label: 'Garden canopy',
			kind: 'garden',
			width: 64,
			height: 64,
			footprintW: 4,
			footprintH: 4,
			interactive: true,
			// Approach from the paired campus path below the crown. Keeping this point
			// outside the four-tile footprint also makes it robust when the compact
			// layout places a sofa to the tree's left.
			interactionOffsets: [{ dc: 1, dr: 4, facing: 3, exclusive: true }],
			frames: [canopyFrame(0), canopyFrame(1), canopyFrame(2)]
		},
		{
			type: PANTRY_TYPE,
			label: 'Tea, snacks and water',
			kind: 'nourishment',
			width: 64,
			height: 32,
			footprintW: 4,
			footprintH: 2,
			interactive: true,
			interactionOffsets: [{ dc: 2, dr: 2, facing: 3, exclusive: true }],
			frames: [pantryFrame(0), pantryFrame(1), pantryFrame(2)]
		},
		{
			type: FOCUS_NOOK_TYPE,
			label: 'Quiet focus nook',
			kind: 'focus',
			width: 48,
			height: 48,
			footprintW: 3,
			footprintH: 3,
			interactive: true,
			interactionOffsets: [{ dc: 1, dr: 3, facing: 3, exclusive: true }],
			frames: [focusNookFrame()]
		},
		{
			type: WELLNESS_NOOK_TYPE,
			label: 'Stretch and recovery nook',
			kind: 'recovery',
			width: 48,
			height: 32,
			footprintW: 3,
			footprintH: 2,
			interactive: true,
			interactionOffsets: [{ dc: 1, dr: 2, facing: 3, exclusive: true }],
			frames: [wellnessFrame(0), wellnessFrame(1), wellnessFrame(2)]
		},
		{
			type: PET_CORNER_TYPE,
			label: 'Pet corner',
			kind: 'belonging',
			width: 32,
			height: 32,
			footprintW: 2,
			footprintH: 2,
			interactive: true,
			interactionOffsets: [{ dc: 1, dr: 2, facing: 3, exclusive: true }],
			frames: [petCornerFrame()]
		},
		{
			type: STORAGE_TYPE,
			label: 'Coats and recycling',
			kind: 'practical',
			width: 48,
			height: 32,
			footprintW: 3,
			footprintH: 2,
			interactive: false,
			interactionOffsets: [],
			frames: [storageFrame()]
		},
		{
			type: UNICORN_TYPE,
			label: 'Unicorn statue',
			kind: 'belonging',
			width: 64,
			height: 64,
			footprintW: 4,
			footprintH: 4,
			interactive: false,
			interactionOffsets: [],
			frames: [unicornFrame(0), unicornFrame(1), unicornFrame(2), unicornFrame(3)]
		},
		{
			type: POOL_TABLE_TYPE,
			label: 'Pool table',
			kind: 'play',
			width: 64,
			height: 48,
			footprintW: 4,
			footprintH: 3,
			interactive: false,
			interactionOffsets: [],
			frames: [poolTableFrame(0), poolTableFrame(1), poolTableFrame(2)]
		},
		{
			type: GAME_TABLE_TYPE,
			label: 'Table games',
			kind: 'play',
			width: 48,
			height: 32,
			footprintW: 3,
			footprintH: 2,
			interactive: false,
			interactionOffsets: [],
			frames: [gameTableFrame(0), gameTableFrame(1), gameTableFrame(2)]
		},
		{
			type: AQUARIUM_TYPE,
			label: 'Aquarium',
			kind: 'belonging',
			width: 48,
			height: 48,
			footprintW: 3,
			footprintH: 3,
			interactive: true,
			interactionOffsets: [{ dc: 1, dr: 3, facing: 3, exclusive: true }],
			frames: [aquariumFrame(0), aquariumFrame(1)]
		},
		{
			type: ART_TABLE_TYPE,
			label: 'Sketch table',
			kind: 'creative',
			width: 48,
			height: 32,
			footprintW: 3,
			footprintH: 2,
			interactive: false,
			interactionOffsets: [],
			frames: [artTableFrame(0), artTableFrame(1), artTableFrame(2)]
		},
		{
			type: ARCADE_DUO_TYPE,
			label: 'Co-op arcade',
			kind: 'play',
			width: 48,
			height: 48,
			footprintW: 3,
			footprintH: 3,
			interactive: false,
			interactionOffsets: [],
			frames: [arcadeDuoFrame(0), arcadeDuoFrame(1)]
		},
		{
			type: RECORD_PLAYER_TYPE,
			label: 'Record player',
			kind: 'recovery',
			width: 32,
			height: 32,
			footprintW: 2,
			footprintH: 2,
			interactive: true,
			interactionOffsets: [{ dc: 1, dr: 2, facing: 3, exclusive: true }],
			frames: [recordPlayerFrame(0), recordPlayerFrame(1)]
		},
		{
			type: PICNIC_TABLE_TYPE,
			label: 'Garden lunch table',
			kind: 'outdoors',
			width: 64,
			height: 48,
			footprintW: 4,
			footprintH: 3,
			interactive: false,
			interactionOffsets: [],
			frames: [picnicTableFrame(0), picnicTableFrame(1)]
		},
		{
			type: HAMMOCK_TYPE,
			label: 'Garden hammock',
			kind: 'outdoors',
			width: 48,
			height: 32,
			footprintW: 3,
			footprintH: 2,
			interactive: false,
			interactionOffsets: [],
			frames: [hammockFrame(0), hammockFrame(1)]
		},
		{
			type: ROBOT_VACUUM_TYPE,
			label: 'Determined little vacuum',
			kind: 'practical',
			width: 16,
			height: 16,
			footprintW: 1,
			footprintH: 1,
			interactive: false,
			interactionOffsets: [],
			frames: [robotVacuumFrame(0), robotVacuumFrame(1), robotVacuumFrame(2)]
		},
		{
			type: GREENHOUSE_TYPE,
			label: 'Glass conservatory',
			kind: 'garden',
			width: 80,
			height: 64,
			footprintW: 5,
			footprintH: 4,
			interactive: true,
			interactionOffsets: [{ dc: 2, dr: 4, facing: 3, exclusive: true }],
			frames: [greenhouseFrame(0), greenhouseFrame(1), greenhouseFrame(2)]
		},
		{
			type: PROJECT_TABLE_TYPE,
			label: 'Open project table',
			kind: 'creative',
			width: 80,
			height: 48,
			footprintW: 5,
			footprintH: 3,
			interactive: true,
			interactionOffsets: [
				{ dc: 1, dr: 3, facing: 3, exclusive: true },
				{ dc: 3, dr: 3, facing: 3, exclusive: true }
			],
			frames: [projectTableFrame(0), projectTableFrame(1)]
		},
		{
			type: LAKESIDE_DOCK_TYPE,
			label: 'Lakeside dock',
			kind: 'outdoors',
			width: 96,
			height: 64,
			footprintW: 6,
			footprintH: 4,
			interactive: true,
			interactionOffsets: [{ dc: 2, dr: 4, facing: 3, exclusive: true }],
			frames: [lakesideDockFrame(0), lakesideDockFrame(1)]
		}
	] satisfies AmenityDefinition[]
).map((definition) => ({
	...definition,
	frames: definition.frames.map(recolorMaterials)
}));

export const AMENITY_CATALOG: CatalogEntry[] = AMENITY_DEFINITIONS.map((definition) => ({
	id: definition.type,
	name: definition.label,
	label: definition.label,
	category: 'decor',
	file: 'Restless-authored in amenities.ts',
	furniturePath: '',
	width: definition.width,
	height: definition.height,
	footprintW: definition.footprintW,
	footprintH: definition.footprintH,
	isDesk: false,
	canPlaceOnWalls: false,
	backgroundTiles: 0,
	groupId: definition.type
}));

export const AMENITY_SPRITES: Record<string, SpriteData> = Object.fromEntries(
	AMENITY_DEFINITIONS.map((definition) => [definition.type, definition.frames[0]])
);

export const AMENITY_FRAMES: Record<string, SpriteData[]> = Object.fromEntries(
	AMENITY_DEFINITIONS.filter((definition) => definition.frames.length > 1).map((definition) => [
		definition.type,
		definition.frames
	])
);

export function getAmenityDefinition(type: string): AmenityDefinition | null {
	return AMENITY_DEFINITIONS.find((definition) => definition.type === type) ?? null;
}
