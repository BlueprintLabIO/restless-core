import type { CockpitTeam } from '$lib/model/cockpit';
import type { ColorValue } from '$lib/vendor/pixel-agents/webview-ui/src/components/ui/types.js';
import { getCatalogEntry } from '$lib/vendor/pixel-agents/webview-ui/src/office/layout/furnitureCatalog.js';
import {
	TileType,
	type AreaDefinition,
	type CarpetTile,
	type OfficeLayout,
	type PlacedFurniture
} from '$lib/vendor/pixel-agents/webview-ui/src/office/types.js';
import type { OfficeMember } from './projection';

export const FOUNTAIN_TYPE = 'RESTLESS_FOUNTAIN';
export const UNICORN_TYPE = 'RESTLESS_UNICORN';
export const OFFICE_PLAN_VERSION = 3;

export type OfficeTheme = 'daylight' | 'garden' | 'midnight';
export type DecorationType =
	'PLANT_2' | 'LARGE_PLANT' | 'SOFA_FRONT' | 'COFFEE_TABLE' | 'DOUBLE_BOOKSHELF';

export interface OfficeDecoration {
	uid: string;
	type: DecorationType;
	col: number;
	row: number;
}

export interface OfficePreferences {
	version: typeof OFFICE_PLAN_VERSION;
	theme: OfficeTheme;
	decorDensity: 'calm' | 'lush';
	pets: boolean;
	decorations: OfficeDecoration[];
}

export interface OfficeZone {
	id: string;
	label: string;
	kind: 'team' | 'shared' | 'executive';
	col: number;
	row: number;
	width: number;
	height: number;
}

export interface OfficePlan {
	layout: OfficeLayout;
	zones: OfficeZone[];
	fountain: { col: number; row: number };
	unicorn: { col: number; row: number };
	restorativeSpots: Array<{ col: number; row: number }>;
	waitingSpots: Array<{ col: number; row: number }>;
	areaMappings: Record<string, string[]>;
	signature: string;
}

export const DEFAULT_OFFICE_PREFERENCES: OfficePreferences = {
	version: OFFICE_PLAN_VERSION,
	theme: 'daylight',
	decorDensity: 'calm',
	pets: true,
	decorations: []
};

const THEMES: Record<
	OfficeTheme,
	{ stone: ColorValue; wood: ColorValue; wall: ColorValue; carpet: ColorValue; accent: ColorValue }
> = {
	daylight: {
		stone: { h: 47, s: 22, b: 22, c: -16, colorize: true },
		wood: { h: 30, s: 28, b: 16, c: -10, colorize: true },
		wall: { h: 198, s: 26, b: 14, c: -12, colorize: true },
		carpet: { h: 176, s: 36, b: 12, c: -5, colorize: true },
		accent: { h: 46, s: 54, b: 10, c: 2, colorize: true }
	},
	midnight: {
		stone: { h: 205, s: 18, b: -30, c: -22 },
		wood: { h: 20, s: 20, b: -30, c: -28 },
		wall: { h: 214, s: 30, b: -100, c: -55 },
		carpet: { h: 185, s: 48, b: -25, c: 8, colorize: true },
		accent: { h: 166, s: 42, b: -13, c: 4, colorize: true }
	},
	garden: {
		stone: { h: 164, s: 19, b: -22, c: -24 },
		wood: { h: 28, s: 24, b: -24, c: -22 },
		wall: { h: 178, s: 28, b: -92, c: -46 },
		carpet: { h: 153, s: 46, b: -24, c: 5, colorize: true },
		accent: { h: 184, s: 52, b: -18, c: 6, colorize: true }
	}
};

function stableHash(value: string): number {
	let hash = 2166136261;
	for (const character of value) {
		hash ^= character.charCodeAt(0);
		hash = Math.imul(hash, 16777619);
	}
	return hash >>> 0;
}

function safeLabel(value: string): string {
	const compact = value.trim().replace(/\s+/g, ' ');
	return compact.length > 22 ? `${compact.slice(0, 20)}…` : compact;
}

function orderedTeams(teams: CockpitTeam[]): CockpitTeam[] {
	return teams.toSorted((a, b) => stableHash(a.id) - stableHash(b.id) || a.id.localeCompare(b.id));
}

function occupancyFor(furniture: PlacedFurniture[], ignoreUid?: string): Set<string> {
	const occupied = new Set<string>();
	for (const item of furniture) {
		if (item.uid === ignoreUid) continue;
		const entry = getCatalogEntry(item.type);
		if (!entry) continue;
		for (let row = 0; row < entry.footprintH; row += 1) {
			for (let col = 0; col < entry.footprintW; col += 1) {
				occupied.add(`${item.col + col},${item.row + row}`);
			}
		}
	}
	return occupied;
}

export function isDecorationPlacementValid(
	layout: OfficeLayout,
	type: DecorationType,
	col: number,
	row: number,
	ignoreUid?: string
): boolean {
	const entry = getCatalogEntry(type);
	if (!entry || col < 1 || row < 1) return false;
	if (col + entry.footprintW >= layout.cols || row + entry.footprintH >= layout.rows) return false;
	const occupied = occupancyFor(layout.furniture, ignoreUid);
	for (let dr = 0; dr < entry.footprintH; dr += 1) {
		for (let dc = 0; dc < entry.footprintW; dc += 1) {
			const tile = layout.tiles[(row + dr) * layout.cols + col + dc];
			if (tile === TileType.WALL || tile === TileType.VOID) return false;
			if (occupied.has(`${col + dc},${row + dr}`)) return false;
		}
	}
	return true;
}

export function createCompanyOfficePlan(
	teams: CockpitTeam[],
	members: OfficeMember[],
	preferences: OfficePreferences
): OfficePlan {
	const theme = THEMES[preferences.theme] ?? THEMES.daylight;
	const actualTeams = orderedTeams(teams);
	const unassignedCount = members.filter(
		(member) => member.actorId !== 'exec' && !member.teamId
	).length;
	const roomSources = [
		...actualTeams.map((team) => ({ id: team.id, label: team.name, count: team.member_count })),
		...(unassignedCount || actualTeams.length === 0
			? [
					{
						id: '__company__',
						label: actualTeams.length ? 'Shared studio' : 'Company studio',
						count: unassignedCount
					}
				]
			: [])
	];
	const roomsPerRow = Math.max(2, Math.ceil(roomSources.length / 2));
	const cols = Math.max(50, roomsPerRow * 16 + 4);
	const rows = 40;
	const centerCol = Math.floor(cols / 2);
	const centerRow = Math.floor(rows / 2);
	const tiles = new Array<TileType>(cols * rows).fill(TileType.FLOOR_4);
	const tileColors = new Array<ColorValue | null>(cols * rows).fill(null);
	const carpetTiles = new Array<CarpetTile | null>(cols * rows).fill(null);
	const areaTiles = new Array<string | null>(cols * rows).fill(null);
	const areas: AreaDefinition[] = [];
	const zones: OfficeZone[] = [];
	const furniture: PlacedFurniture[] = [];
	const areaMappings: Record<string, string[]> = {};

	const setTile = (col: number, row: number, tile: TileType, color: ColorValue | null) => {
		if (col < 0 || row < 0 || col >= cols || row >= rows) return;
		const index = row * cols + col;
		tiles[index] = tile;
		tileColors[index] = color;
	};
	const add = (uid: string, type: string, col: number, row: number) => {
		furniture.push({ uid, type, col, row });
	};

	for (let row = 0; row < rows; row += 1) {
		for (let col = 0; col < cols; col += 1) {
			const edge = row === 0 || col === 0 || row === rows - 1 || col === cols - 1;
			setTile(col, row, edge ? TileType.WALL : TileType.FLOOR_4, edge ? theme.wall : theme.stone);
		}
	}

	const roomWidth = 14;
	const roomHeight = 11;
	roomSources.forEach((room, index) => {
		const upper = index < roomsPerRow;
		const slot = upper ? index : index - roomsPerRow;
		const col = 3 + slot * 16;
		const row = upper ? 3 : rows - roomHeight - 3;
		const areaLabel = safeLabel(room.label || `Team ${index + 1}`);
		areas.push({
			label: areaLabel,
			color: ['#4fa6a0', '#5b84b1', '#8f77b5', '#be825f'][index % 4]
		});
		zones.push({
			id: room.id,
			label: areaLabel,
			kind: room.id === '__company__' ? 'shared' : 'team',
			col,
			row,
			width: roomWidth,
			height: roomHeight
		});
		areaMappings[room.id] = [areaLabel];

		// Teams occupy open material-defined neighborhoods. A single presentation
		// wall carries the board; the other three sides remain shared circulation.
		for (let dr = 0; dr < roomHeight; dr += 1) {
			for (let dc = 0; dc < roomWidth; dc += 1) {
				const tileCol = col + dc;
				const tileRow = row + dr;
				const presentationWall = upper ? dr === 0 : dr === roomHeight - 1;
				setTile(
					tileCol,
					tileRow,
					presentationWall ? TileType.WALL : TileType.FLOOR_7,
					presentationWall ? theme.wall : theme.wood
				);
				if (!presentationWall) areaTiles[tileRow * cols + tileCol] = areaLabel;
			}
		}

		const seatsNeeded = Math.max(2, Math.min(6, room.count || 2));
		for (let seat = 0; seat < seatsNeeded; seat += 1) {
			const column = seat % 3;
			const band = Math.floor(seat / 3);
			const deskCol = col + 1 + column * 4;
			const deskRow = row + 1 + band * 5;
			const uid = `team-${stableHash(room.id)}-${seat}`;
			add(`${uid}-desk`, 'DESK_FRONT', deskCol, deskRow);
			add(`${uid}-pc`, 'PC_FRONT_OFF', deskCol + 1, deskRow);
			add(`${uid}-seat`, 'CUSHIONED_CHAIR_BACK', deskCol + 1, deskRow + 2);
		}
		add(
			`team-${stableHash(room.id)}-board`,
			'WHITEBOARD',
			col + roomWidth - 4,
			upper ? row : row + roomHeight - 1
		);
		if (index % 2 === 0)
			add(
				`team-${stableHash(room.id)}-plant`,
				'PLANT_2',
				col + roomWidth - 2,
				row + roomHeight - 3
			);
	});

	// The company centre remains shared space, even as rooms grow horizontally.
	// A north gallery wall gives completed work a home; the south side stays open
	// so the fountain reads as a courtyard rather than another boxed room.
	const atriumLeft = centerCol - 8;
	const atriumTop = centerRow - 6;
	const atriumBottom = atriumTop + 11;
	for (let row = atriumTop; row <= atriumBottom; row += 1) {
		for (let col = atriumLeft; col <= atriumLeft + 16; col += 1) {
			setTile(col, row, TileType.FLOOR_4, theme.stone);
		}
	}
	for (let col = atriumLeft; col <= atriumLeft + 16; col += 1) {
		const doorway = col === centerCol || col === centerCol - 1;
		setTile(
			col,
			atriumTop,
			doorway ? TileType.FLOOR_4 : TileType.WALL,
			doorway ? theme.stone : theme.wall
		);
	}

	const fountain = { col: centerCol - 2, row: centerRow - 2 };
	const unicorn = { col: atriumLeft + 2, row: atriumTop + 9 };
	add('company-fountain', FOUNTAIN_TYPE, fountain.col, fountain.row);
	add('company-unicorn', UNICORN_TYPE, unicorn.col, unicorn.row);
	add('atrium-bench-west', 'SOFA_SIDE', fountain.col - 2, fountain.row + 1);
	add('atrium-bench-east', 'SOFA_SIDE:left', fountain.col + 5, fountain.row + 1);
	add('atrium-plant-nw', 'LARGE_PLANT', fountain.col - 3, fountain.row - 3);
	add('atrium-plant-se', 'PLANT_2', fountain.col + 6, fountain.row + 4);
	for (let row = fountain.row - 1; row <= fountain.row + 4; row += 1) {
		for (let col = fountain.col - 1; col <= fountain.col + 4; col += 1) {
			if (
				col >= fountain.col &&
				col < fountain.col + 4 &&
				row >= fountain.row &&
				row < fountain.row + 4
			)
				continue;
			carpetTiles[row * cols + col] = {
				variant: 2,
				color: theme.carpet,
				accentColor: theme.accent
			};
		}
	}

	const executiveLabel = 'Executive overlook';
	areas.push({ label: executiveLabel, color: '#63b879' });
	zones.push({
		id: '__exec__',
		label: executiveLabel,
		kind: 'executive',
		col: atriumLeft + 1,
		row: atriumTop + 2,
		width: 6,
		height: 5
	});
	areaMappings.__exec__ = [executiveLabel];
	for (let row = atriumTop + 1; row <= atriumTop + 5; row += 1) {
		for (let col = atriumLeft + 1; col <= atriumLeft + 6; col += 1)
			areaTiles[row * cols + col] = executiveLabel;
	}
	add('exec-desk', 'DESK_FRONT', atriumLeft + 1, atriumTop + 2);
	add('exec-pc', 'PC_FRONT_OFF', atriumLeft + 2, atriumTop + 2);
	add('exec-seat', 'CUSHIONED_CHAIR_BACK', atriumLeft + 2, atriumTop + 4);

	const outputCount = members.reduce((count, member) => count + member.outputCount, 0);
	const galleryFrames = Math.max(2, Math.min(4, outputCount || 2));
	const galleryColumns = [atriumLeft + 1, atriumLeft + 4, atriumLeft + 11, atriumLeft + 14];
	for (let index = 0; index < galleryFrames; index += 1) {
		add(
			`gallery-${index}`,
			index % 3 === 0 ? 'LARGE_PAINTING' : 'SMALL_PAINTING_2',
			galleryColumns[index],
			atriumTop - 1
		);
	}

	// An open coffee corner supplies ambient destinations without becoming a
	// dashboard tile. Furniture stays sparse enough to preserve wide paths.
	const cafeCol = atriumLeft + 12;
	const cafeRow = atriumTop + 8;
	add('cafe-table', 'SMALL_TABLE_FRONT', cafeCol, cafeRow);
	add('cafe-chair', 'CUSHIONED_CHAIR_FRONT', cafeCol, cafeRow + 2);
	add('cafe-coffee', 'COFFEE', cafeCol + 1, cafeRow);
	add('cafe-sofa', 'SOFA_FRONT', cafeCol - 3, cafeRow + 3);
	const restorativeSpots = [
		{ col: fountain.col - 1, row: fountain.row + 4 },
		{ col: fountain.col + 4, row: fountain.row - 1 },
		{ col: unicorn.col + 2, row: unicorn.row },
		{ col: cafeCol + 2, row: cafeRow + 2 }
	];

	// A small company gets a quiet reading garden instead of an unexplained
	// empty lower floor. Once real teams need the lower rooms, their space wins.
	if (roomSources.length <= roomsPerRow) {
		const quietCol = atriumLeft + 1;
		const quietRow = atriumBottom + 1;
		add('quiet-books', 'DOUBLE_BOOKSHELF', quietCol, quietRow);
		add('quiet-table', 'COFFEE_TABLE', quietCol + 2, quietRow + 1);
		add('quiet-sofa', 'SOFA_FRONT', quietCol + 2, quietRow + 4);
		add('quiet-plant', 'LARGE_PLANT', quietCol + 6, quietRow + 1);
		for (let row = quietRow; row <= quietRow + 5; row += 1) {
			for (let col = quietCol; col <= quietCol + 7; col += 1) {
				carpetTiles[row * cols + col] = {
					variant: 1,
					color: theme.carpet,
					accentColor: theme.accent
				};
			}
		}
		restorativeSpots.push({ col: quietCol + 5, row: quietRow + 4 });
	}

	if (preferences.decorDensity === 'lush') {
		[
			[2, centerRow - 2, 'LARGE_PLANT'],
			[cols - 4, centerRow + 1, 'LARGE_PLANT'],
			[centerCol - 10, 1, 'HANGING_PLANT'],
			[centerCol + 9, rows - 3, 'PLANT_2']
		].forEach(([col, row, type], index) =>
			add(`lush-${index}`, String(type), Number(col), Number(row))
		);
	}

	const baseLayout: OfficeLayout = {
		version: 1,
		layoutRevision: OFFICE_PLAN_VERSION,
		cols,
		rows,
		tiles,
		tileColors,
		carpetTiles,
		areas,
		areaTiles,
		furniture,
		pets: preferences.pets
			? [
					{ id: 'company-pet-claudio', petType: 0 },
					{ id: 'company-pet-gitcat', petType: 1 }
				]
			: []
	};

	for (const decoration of preferences.decorations) {
		if (!isDecorationPlacementValid(baseLayout, decoration.type, decoration.col, decoration.row))
			continue;
		baseLayout.furniture.push({ ...decoration });
	}

	const signature = JSON.stringify({
		version: OFFICE_PLAN_VERSION,
		teams: roomSources.map((room) => [room.id, room.count]),
		outputs: outputCount,
		preferences
	});
	return {
		layout: baseLayout,
		zones,
		fountain,
		unicorn,
		restorativeSpots,
		waitingSpots: [
			{ col: centerCol - 1, row: atriumTop + 2 },
			{ col: centerCol, row: atriumTop + 2 },
			{ col: centerCol + 1, row: atriumTop + 2 }
		],
		areaMappings,
		signature
	};
}
