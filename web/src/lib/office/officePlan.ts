import type { CockpitTeam } from '$lib/model/cockpit';
import type { CatalogEntry } from '$lib/vendor/pixel-agents/core/src/assets/types.js';
import type { ColorValue } from '$lib/vendor/pixel-agents/webview-ui/src/components/ui/types.js';
import { getCatalogEntry } from '$lib/vendor/pixel-agents/webview-ui/src/office/layout/furnitureCatalog.js';
import {
	Direction,
	TileType,
	type AreaDefinition,
	type CarpetTile,
	type OfficeLayout,
	type PlacedFurniture
} from '$lib/vendor/pixel-agents/webview-ui/src/office/types.js';
import {
	AMENITY_DEFINITIONS,
	AQUARIUM_TYPE,
	ARCADE_DUO_TYPE,
	ART_TABLE_TYPE,
	CANOPY_TREE_TYPE,
	FOCUS_NOOK_TYPE,
	GAME_TABLE_TYPE,
	GREENHOUSE_TYPE,
	HAMMOCK_TYPE,
	LAKESIDE_DOCK_TYPE,
	PANTRY_TYPE,
	PET_CORNER_TYPE,
	PICNIC_TABLE_TYPE,
	POOL_TABLE_TYPE,
	PROJECT_TABLE_TYPE,
	RECORD_PLAYER_TYPE,
	ROBOT_VACUUM_TYPE,
	STORAGE_TYPE,
	UNICORN_TYPE,
	WELLNESS_NOOK_TYPE,
	getAmenityDefinition,
	type AmenityKind
} from './amenities';
import type { OfficeMember } from './projection';

export const OFFICE_PLAN_VERSION = 7;
export const MAX_VISIBLE_OFFICE_MEMBERS = 20;

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

export interface OfficeInteractionPoint {
	id: string;
	kind: AmenityKind;
	label: string;
	col: number;
	row: number;
	facing: Direction;
	exclusive: boolean;
}

export interface OfficeRestingSpot {
	col: number;
	row: number;
	facing: Direction;
	poseCol?: number;
	poseRow?: number;
	posture: 'sit' | 'stand';
	activity:
		| 'conversation'
		| 'reading'
		| 'garden'
		| 'pool'
		| 'table-game'
		| 'sketching'
		| 'headphones'
		| 'arcade'
		| 'aquarium'
		| 'hammock'
		| 'picnic'
		| 'pet-care'
		| 'zen'
		| 'project'
		| 'fishing'
		| 'birdwatching';
	tone: 'quiet' | 'social' | 'playful' | 'whimsical' | 'outdoors';
	groupId: string;
}

export interface OfficeActivityScene {
	id: string;
	label: string;
	tone: OfficeRestingSpot['tone'];
	spots: OfficeRestingSpot[];
}

export interface OfficePlan {
	layout: OfficeLayout;
	zones: OfficeZone[];
	home: { col: number; row: number };
	interactionPoints: OfficeInteractionPoint[];
	activityScenes: OfficeActivityScene[];
	restingSpots: OfficeRestingSpot[];
	waitingSpots: Array<{ col: number; row: number }>;
	protectedPath: Array<{ col: number; row: number }>;
	areaMappings: Record<string, string[]>;
	animatedAmenities: Array<{ type: string; col: number; row: number }>;
	landmark: { type: string; col: number; row: number };
	visibleMemberCount: number;
	signature: string;
}

export interface OfficePlanValidation {
	valid: boolean;
	errors: string[];
	seatCount: number;
	walkableTileCount: number;
	reachableInteractionCount: number;
	cols: number;
	rows: number;
}

export interface DecorationRouteConstraint {
	home: { col: number; row: number };
	requiredPoints: Array<{ col: number; row: number }>;
	ignoreUid?: string;
}

export const DEFAULT_OFFICE_PREFERENCES: OfficePreferences = {
	version: OFFICE_PLAN_VERSION,
	decorDensity: 'lush',
	pets: true,
	decorations: []
};

const DAYLIGHT = {
	stone: { h: 194, s: 18, b: 28, c: -18, colorize: true } satisfies ColorValue,
	deck: { h: 175, s: 24, b: 20, c: -14, colorize: true } satisfies ColorValue,
	zen: { h: 71, s: 12, b: 24, c: -18, colorize: true } satisfies ColorValue,
	furniture: { h: 206, s: 22, b: 17, c: -12, colorize: true } satisfies ColorValue,
	carpet: { h: 162, s: 24, b: 18, c: -10, colorize: true } satisfies ColorValue,
	accent: { h: 258, s: 28, b: 12, c: -8, colorize: true } satisfies ColorValue
};
const AREA_COLORS = ['#4e9a7e', '#5d8fc1', '#7b66b2', '#68b6ba', '#669c76', '#8b78ba'];
const BAY_CAPACITY = 6;
const BAY_WIDTH = 12;
const BAY_HEIGHT = 10;

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

function catalogEntry(type: string): CatalogEntry | null {
	const entry = getCatalogEntry(type);
	if (entry) {
		return {
			id: type,
			name: entry.label,
			label: entry.label,
			category: entry.category ?? 'decor',
			file: '',
			furniturePath: '',
			width: entry.sprite[0]?.length ?? entry.footprintW * 16,
			height: entry.sprite.length || entry.footprintH * 16,
			footprintW: entry.footprintW,
			footprintH: entry.footprintH,
			isDesk: entry.isDesk,
			canPlaceOnWalls: entry.canPlaceOnWalls ?? false,
			canPlaceOnSurfaces: entry.canPlaceOnSurfaces,
			backgroundTiles: entry.backgroundTiles
		};
	}
	const amenity = getAmenityDefinition(type);
	if (amenity) {
		return {
			id: type,
			name: amenity.label,
			label: amenity.label,
			category: 'decor',
			file: '',
			furniturePath: '',
			width: amenity.width,
			height: amenity.height,
			footprintW: amenity.footprintW,
			footprintH: amenity.footprintH,
			isDesk: false,
			canPlaceOnWalls: false
		};
	}
	return null;
}

function occupancyFor(furniture: PlacedFurniture[], ignoreUid?: string): Set<string> {
	const occupied = new Set<string>();
	for (const item of furniture) {
		if (item.uid === ignoreUid) continue;
		const entry = catalogEntry(item.type);
		if (!entry) continue;
		for (let row = 0; row < entry.footprintH; row += 1) {
			for (let col = 0; col < entry.footprintW; col += 1)
				occupied.add(`${item.col + col},${item.row + row}`);
		}
	}
	return occupied;
}

export function isDecorationPlacementValid(
	layout: OfficeLayout,
	type: string,
	col: number,
	row: number,
	route?: DecorationRouteConstraint
): boolean {
	const entry = catalogEntry(type);
	if (!entry || col < 1 || row < 1) return false;
	if (col + entry.footprintW >= layout.cols || row + entry.footprintH >= layout.rows) return false;
	const occupied = occupancyFor(layout.furniture, route?.ignoreUid);
	for (let dr = 0; dr < entry.footprintH; dr += 1) {
		for (let dc = 0; dc < entry.footprintW; dc += 1) {
			const tile = layout.tiles[(row + dr) * layout.cols + col + dc];
			if (
				tile === TileType.WALL ||
				tile === TileType.VOID ||
				occupied.has(`${col + dc},${row + dr}`)
			)
				return false;
		}
	}
	if (route) {
		const blocked = furnitureBlockedTiles(layout);
		for (let dr = 0; dr < entry.footprintH; dr += 1) {
			for (let dc = 0; dc < entry.footprintW; dc += 1) blocked.add(`${col + dc},${row + dr}`);
		}
		const reached = reachableTiles(layout, route.home, blocked);
		if (
			!reached.has(`${route.home.col},${route.home.row}`) ||
			route.requiredPoints.some((point) => !reached.has(`${point.col},${point.row}`))
		)
			return false;
	}
	return true;
}

interface BaySource {
	id: string;
	label: string;
	seatCount: number;
	bayIndex: number;
	kind: 'team' | 'shared';
}

function buildBays(teams: CockpitTeam[], members: OfficeMember[]): BaySource[] {
	const visibleStaff = members.filter((member) => member.actorId !== 'exec');
	const bays: BaySource[] = [];
	for (const team of orderedTeams(teams)) {
		const count = visibleStaff.filter((member) => member.teamId === team.id).length;
		const bayCount = Math.max(1, Math.ceil(count / BAY_CAPACITY));
		for (let index = 0; index < bayCount; index += 1) {
			bays.push({
				id: team.id,
				label: team.name,
				seatCount: Math.max(0, Math.min(BAY_CAPACITY, count - index * BAY_CAPACITY)),
				bayIndex: index,
				kind: 'team'
			});
		}
	}
	const unassigned = visibleStaff.filter((member) => !member.teamId).length;
	if (unassigned > 0 || teams.length === 0) {
		const bayCount = Math.max(1, Math.ceil(unassigned / BAY_CAPACITY));
		for (let index = 0; index < bayCount; index += 1) {
			bays.push({
				id: '__company__',
				label: teams.length ? 'Shared studio' : 'Company studio',
				seatCount: Math.max(0, Math.min(BAY_CAPACITY, unassigned - index * BAY_CAPACITY)),
				bayIndex: index,
				kind: 'shared'
			});
		}
	}
	return bays;
}

export function createCompanyOfficePlan(
	teams: CockpitTeam[],
	members: OfficeMember[],
	preferences: OfficePreferences
): OfficePlan {
	const visibleMembers = members.slice(0, MAX_VISIBLE_OFFICE_MEMBERS);
	const bays = buildBays(teams, visibleMembers);
	const cols = 72;
	const rows = 52;
	const centerCol = Math.floor(cols / 2);
	const tiles = new Array<TileType>(cols * rows).fill(TileType.VOID);
	const tileColors = new Array<ColorValue | null>(cols * rows).fill(null);
	const carpetTiles = new Array<CarpetTile | null>(cols * rows).fill(null);
	const areaTiles = new Array<string | null>(cols * rows).fill(null);
	const areas: AreaDefinition[] = [];
	const zones: OfficeZone[] = [];
	const furniture: PlacedFurniture[] = [];
	const areaMappings: Record<string, string[]> = {};
	const interactionPoints: OfficeInteractionPoint[] = [];
	const animatedAmenities: OfficePlan['animatedAmenities'] = [];

	const setTile = (col: number, row: number, tile: TileType, color: ColorValue | null) => {
		if (col < 0 || row < 0 || col >= cols || row >= rows) return;
		const index = row * cols + col;
		tiles[index] = tile;
		tileColors[index] = color;
	};
	const fill = (
		left: number,
		top: number,
		width: number,
		height: number,
		tile: TileType,
		color: ColorValue
	) => {
		for (let row = top; row < top + height; row += 1) {
			for (let col = left; col < left + width; col += 1) setTile(col, row, tile, color);
		}
	};
	const add = (uid: string, type: string, col: number, row: number, color?: ColorValue) =>
		furniture.push({ uid, type, col, row, color });
	const rug = (
		left: number,
		top: number,
		width: number,
		height: number,
		variant: number,
		color = DAYLIGHT.carpet,
		accentColor = DAYLIGHT.accent
	) => {
		for (let row = top; row < top + height; row += 1) {
			for (let col = left; col < left + width; col += 1) {
				if (row <= 0 || col <= 0 || row >= rows - 1 || col >= cols - 1) continue;
				if (tiles[row * cols + col] === TileType.VOID) continue;
				carpetTiles[row * cols + col] = {
					variant,
					color,
					accentColor
				};
			}
		}
	};
	const addAmenity = (uid: string, type: string, col: number, row: number) => {
		add(uid, type, col, row);
		const definition = getAmenityDefinition(type);
		if (!definition) return;
		if (definition.frames.length > 1) animatedAmenities.push({ type, col, row });
		definition.interactionOffsets.forEach((point, index) => {
			interactionPoints.push({
				id: `${uid}-${index}`,
				kind: definition.kind,
				label: definition.label,
				col: col + point.dc,
				row: row + point.dr,
				facing: point.facing,
				exclusive: point.exclusive
			});
		});
	};

	// One irregular C-shaped campus: broad open team pavilions form the north,
	// west and south arms; small shared plates bridge a zen garden to the lake.
	// Team plates themselves are added below, so a small company does not inherit
	// a huge empty rectangle and the campus genuinely grows with its real Teams.
	fill(3, 3, 15, 46, TileType.FLOOR_4, DAYLIGHT.stone);
	fill(12, 14, 45, 5, TileType.FLOOR_7, DAYLIGHT.deck);
	fill(12, 31, 47, 8, TileType.FLOOR_7, DAYLIGHT.deck);
	fill(14, 19, 10, 14, TileType.FLOOR_4, DAYLIGHT.stone);
	fill(22, 24, 24, 5, TileType.FLOOR_7, DAYLIGHT.deck);
	fill(25, 20, 17, 12, TileType.FLOOR_6, DAYLIGHT.zen);
	fill(43, 21, 26, 10, TileType.FLOOR_7, DAYLIGHT.deck);

	// Open team neighbourhoods repeat only when real visible membership needs capacity.
	const topBayCount = Math.ceil(bays.length / 2);
	const slotColumns = (count: number): number[] => {
		if (count <= 1) return [27];
		if (count === 2) return [13, 43];
		if (count === 3) return [5, 28, 51];
		return [5, 20, 35, 50];
	};
	const topColumns = slotColumns(topBayCount);
	const bottomColumns = slotColumns(Math.max(0, bays.length - topBayCount));
	bays.forEach((bay, index) => {
		const upper = index < topBayCount;
		const slot = upper ? index : index - topBayCount;
		const col = (upper ? topColumns : bottomColumns)[slot];
		const row = upper ? 4 : 38;
		const suffix = bay.bayIndex ? ` ${bay.bayIndex + 1}` : '';
		const areaLabel = safeLabel(`${bay.label}${suffix}`);
		areas.push({ label: areaLabel, color: AREA_COLORS[index % AREA_COLORS.length] });
		zones.push({
			id: `${bay.id}:${bay.bayIndex}`,
			label: areaLabel,
			kind: bay.kind,
			col,
			row,
			width: BAY_WIDTH,
			height: BAY_HEIGHT
		});
		(areaMappings[bay.id] ??= []).push(areaLabel);
		for (let dr = 0; dr < BAY_HEIGHT; dr += 1) {
			for (let dc = 0; dc < BAY_WIDTH; dc += 1) {
				setTile(col + dc, row + dr, TileType.FLOOR_4, DAYLIGHT.stone);
				areaTiles[(row + dr) * cols + col + dc] = areaLabel;
			}
		}
		rug(col + 1, row + 1, BAY_WIDTH - 2, BAY_HEIGHT - 2, index % 3);
		const seats = Math.max(1, bay.seatCount);
		for (let seat = 0; seat < seats; seat += 1) {
			const column = seat % 3;
			const band = Math.floor(seat / 3);
			const deskCol = col + column * 4;
			const deskRow = row + 1 + band * 4;
			const uid = `bay-${stableHash(`${bay.id}:${bay.bayIndex}`)}-${seat}`;
			add(`${uid}-desk`, 'DESK_FRONT', deskCol, deskRow, DAYLIGHT.furniture);
			add(`${uid}-pc`, 'PC_FRONT_OFF', deskCol + 1, deskRow);
			add(`${uid}-seat`, 'CUSHIONED_CHAIR_BACK', deskCol + 1, deskRow + 2, DAYLIGHT.furniture);
		}
		add(`bay-${index}-board`, 'WHITEBOARD', col + 10, row + 8);
		add(`bay-${index}-plant`, 'PLANT_2', col + 11, row + 4);
	});

	// The singleton Exec remains a real projected person, but its workstation is
	// deliberately unlabelled: the source does not call this actor "Company lead".
	// It is an open desk beside the shared circulation, not a ceremonial office.
	add('exec-desk', 'DESK_FRONT', 5, 20, DAYLIGHT.furniture);
	add('exec-pc', 'PC_FRONT_OFF', 6, 20);
	add('exec-seat', 'CUSHIONED_CHAIR_BACK', 6, 22, DAYLIGHT.furniture);
	add('exec-whiteboard', 'WHITEBOARD', 9, 19);
	add('exec-plant', 'PLANT_2', 4, 24);

	// West pavilion: quiet care spaces open directly onto the garden path.
	rug(17, 19, 7, 13, 1);
	addAmenity('garden-canopy', CANOPY_TREE_TYPE, 18, 20);
	add('canopy-seat', 'SOFA_SIDE', 22, 21, DAYLIGHT.furniture);
	add('library-shelf', 'DOUBLE_BOOKSHELF', 18, 25, DAYLIGHT.furniture);
	add('library-seat', 'SOFA_FRONT', 18, 29, DAYLIGHT.furniture);
	add('library-plant', 'PLANT_2', 21, 25);
	interactionPoints.push({
		id: 'reading-seat',
		kind: 'reading',
		label: 'Garden library',
		col: 20,
		row: 29,
		facing: Direction.LEFT,
		exclusive: true
	});

	addAmenity('care-wellness', WELLNESS_NOOK_TYPE, 7, 24);
	addAmenity('care-storage', STORAGE_TYPE, 4, 28);
	addAmenity('care-focus', FOCUS_NOOK_TYPE, 9, 27);
	addAmenity('care-pantry', PANTRY_TYPE, 5, 32);
	add('care-bin', 'BIN', 4, 34);
	if (preferences.pets) addAmenity('care-pets', PET_CORNER_TYPE, 10, 32);

	// The central void becomes a restorative zen court rather than more office.
	// The larger unicorn is a genuine campus landmark, with calm seating around it.
	const unicorn = { type: UNICORN_TYPE, col: 31, row: 22 };
	addAmenity('company-unicorn', unicorn.type, unicorn.col, unicorn.row);
	add('zen-bench-west', 'CUSHIONED_BENCH', 27, 23, DAYLIGHT.furniture);
	add('zen-bench-east', 'CUSHIONED_BENCH', 38, 23, DAYLIGHT.furniture);
	add('zen-pot-west', 'POT', 27, 21);
	add('zen-pot-east', 'POT', 39, 21);
	interactionPoints.push(
		{
			id: 'unicorn-garden-west',
			kind: 'belonging',
			label: 'Unicorn zen garden',
			col: 30,
			row: 24,
			facing: Direction.RIGHT,
			exclusive: true
		},
		{
			id: 'unicorn-garden-east',
			kind: 'belonging',
			label: 'Unicorn zen garden',
			col: 35,
			row: 24,
			facing: Direction.LEFT,
			exclusive: true
		}
	);

	// East deck: a glasshouse and open project table face the lake and dock.
	rug(43, 21, 26, 10, 0);
	addAmenity('garden-greenhouse', GREENHOUSE_TYPE, 44, 22);
	addAmenity('project-table', PROJECT_TABLE_TYPE, 50, 22);
	addAmenity('care-aquarium', AQUARIUM_TYPE, 56, 22);
	addAmenity('lakeside-dock', LAKESIDE_DOCK_TYPE, 62, 22);
	add('commons-sofa-west', 'SOFA_FRONT', 44, 29, DAYLIGHT.furniture);
	add('commons-table', 'COFFEE_TABLE', 47, 29, DAYLIGHT.furniture);
	add('commons-sofa-east', 'SOFA_BACK', 50, 29, DAYLIGHT.furniture);
	addAmenity('music-player', RECORD_PLAYER_TYPE, 55, 29);
	addAmenity('little-vacuum', ROBOT_VACUUM_TYPE, 59, 29);
	interactionPoints.push(
		{
			id: 'social-deck-west',
			kind: 'social',
			label: 'Lakeside lounge',
			col: 46,
			row: 29,
			facing: Direction.LEFT,
			exclusive: true
		},
		{
			id: 'social-deck-east',
			kind: 'social',
			label: 'Lakeside lounge',
			col: 52,
			row: 29,
			facing: Direction.LEFT,
			exclusive: true
		}
	);

	// South terrace: play and making are visible, but the paired circulation
	// lane below remains furniture-free for the pathfinder and for visual calm.
	rug(17, 31, 30, 5, 2);
	const poolCol = 18;
	const poolRow = 32;
	const gameCol = 24;
	const gameRow = 32;
	const artCol = 29;
	const picnicCol = 34;
	const hammockCol = 39;
	const arcadeCol = 43;
	addAmenity('lounge-pool', POOL_TABLE_TYPE, poolCol, poolRow);
	addAmenity('lounge-games', GAME_TABLE_TYPE, gameCol, gameRow);
	add('games-chair-west', 'CUSHIONED_CHAIR_BACK', gameCol, gameRow + 2, DAYLIGHT.furniture);
	add('games-chair-east', 'CUSHIONED_CHAIR_BACK', gameCol + 2, gameRow + 2, DAYLIGHT.furniture);
	addAmenity('creative-table', ART_TABLE_TYPE, artCol, 32);
	add('creative-chair-west', 'CUSHIONED_CHAIR_BACK', artCol, 34, DAYLIGHT.furniture);
	add('creative-chair-east', 'CUSHIONED_CHAIR_BACK', artCol + 2, 34, DAYLIGHT.furniture);
	addAmenity('terrace-picnic', PICNIC_TABLE_TYPE, picnicCol, 32);
	addAmenity('terrace-hammock', HAMMOCK_TYPE, hammockCol, 32);
	addAmenity('lounge-arcade', ARCADE_DUO_TYPE, arcadeCol, 32);
	add('terrace-pot-west', 'POT', 22, 33);
	add('terrace-plant-east', 'PLANT_2', 27, 33);

	const home = { col: 16, row: 26 };
	const waitingSpots = [
		{ col: 28, row: 16 },
		{ col: 29, row: 16 },
		{ col: 30, row: 16 }
	];
	const pathCandidates = [
		...[16, 17].flatMap((row) =>
			Array.from({ length: 53 }, (_, index) => ({ col: index + 4, row }))
		),
		...[15, 16].flatMap((col) =>
			Array.from({ length: 22 }, (_, index) => ({ col, row: index + 16 }))
		),
		...[27, 28].flatMap((row) =>
			Array.from({ length: 53 }, (_, index) => ({ col: index + 16, row }))
		),
		...[36, 37].flatMap((row) =>
			Array.from({ length: 55 }, (_, index) => ({ col: index + 4, row }))
		)
	];
	const protectedPath = Array.from(
		new Map(pathCandidates.map((point) => [`${point.col},${point.row}`, point])).values()
	);

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
	const routeConstraint = {
		home,
		requiredPoints: [...interactionPoints, ...waitingSpots, ...protectedPath]
	};
	if (preferences.decorDensity === 'lush') {
		[
			[4, 35, 'LARGE_PLANT'],
			[23, 20, 'LARGE_PLANT'],
			[27, 19, 'HANGING_PLANT'],
			[59, 26, 'PLANT_2'],
			[66, 29, 'PLANT_2']
		].forEach(([col, row, type], index) => {
			const candidate = {
				uid: `lush-${index}`,
				type: String(type),
				col: Number(col),
				row: Number(row)
			};
			if (
				isDecorationPlacementValid(
					baseLayout,
					candidate.type,
					candidate.col,
					candidate.row,
					routeConstraint
				)
			)
				baseLayout.furniture.push(candidate);
		});
	}
	for (const decoration of preferences.decorations) {
		if (
			isDecorationPlacementValid(
				baseLayout,
				decoration.type,
				decoration.col,
				decoration.row,
				routeConstraint
			)
		)
			baseLayout.furniture.push({ ...decoration });
	}

	// Availability is source truth; these calm leisure scenes are presentation
	// only. Idle people use real amenities instead of standing in random gaps.
	const restingReserved = new Set(
		[...interactionPoints, ...waitingSpots, ...protectedPath, home].map(
			(point) => `${point.col},${point.row}`
		)
	);
	const restingBlocked = furnitureBlockedTiles(baseLayout);
	const restingReachable = reachableTiles(baseLayout, home, restingBlocked);
	const scene = (
		id: string,
		label: string,
		tone: OfficeActivityScene['tone'],
		spots: Array<Omit<OfficeRestingSpot, 'groupId' | 'tone'>>
	): OfficeActivityScene => ({
		id,
		label,
		tone,
		spots: spots.map((spot) => ({ ...spot, tone, groupId: id }))
	});
	const proposedScenes: OfficeActivityScene[] = [
		scene('library', 'Garden library', 'quiet', [
			...[18, 19].map((col) => ({
				col,
				row: 30,
				poseCol: col,
				poseRow: 29,
				facing: Direction.DOWN,
				posture: 'sit' as const,
				activity: 'reading' as const
			}))
		]),
		scene('pool-game', 'Pool game', 'playful', [
			{
				col: poolCol + 1,
				row: poolRow - 1,
				facing: Direction.DOWN,
				posture: 'stand',
				activity: 'pool'
			},
			{
				col: poolCol + 2,
				row: poolRow + 3,
				facing: Direction.UP,
				posture: 'stand',
				activity: 'pool'
			}
		]),
		scene('table-game', 'Cards and puzzles', 'social', [
			...[gameCol, gameCol + 2].map((col) => ({
				col,
				row: gameRow + 3,
				poseCol: col,
				poseRow: gameRow + 2,
				facing: Direction.UP,
				posture: 'sit' as const,
				activity: 'table-game' as const
			}))
		]),
		scene('arcade-duo', 'Co-op arcade', 'playful', [
			...[arcadeCol, arcadeCol + 2].map((col) => ({
				col,
				row: 35,
				facing: Direction.UP,
				posture: 'stand' as const,
				activity: 'arcade' as const
			}))
		]),
		scene('commons-chat', 'Lakeside sofa conversation', 'social', [
			...[44, 45].map((col) => ({
				col,
				row: 31,
				poseCol: col,
				poseRow: 29,
				facing: Direction.DOWN,
				posture: 'sit' as const,
				activity: 'conversation' as const
			})),
			...[50, 51].map((col) => ({
				col,
				row: 31,
				poseCol: col,
				poseRow: 29,
				facing: Direction.UP,
				posture: 'sit' as const,
				activity: 'conversation' as const
			}))
		]),
		scene('sketch-table', 'Shared sketch table', 'quiet', [
			...[artCol, artCol + 2].map((col) => ({
				col,
				row: 35,
				poseCol: col,
				poseRow: 34,
				facing: Direction.UP,
				posture: 'sit' as const,
				activity: 'sketching' as const
			}))
		]),
		scene('greenhouse', 'Greenhouse tending', 'outdoors', [
			...[44, 48].map((col) => ({
				col,
				row: 26,
				facing: Direction.UP,
				posture: 'stand' as const,
				activity: 'garden' as const
			}))
		]),
		scene('project-table', 'Open project table', 'social', [
			...[50, 54].map((col, index) => ({
				col,
				row: 25,
				poseCol: 51 + index * 2,
				poseRow: 23,
				facing: Direction.UP,
				posture: 'sit' as const,
				activity: 'project' as const
			}))
		]),
		scene('dock', 'Fishing and birdwatching', 'outdoors', [
			{
				col: 67,
				row: 26,
				poseCol: 67,
				poseRow: 25,
				facing: Direction.RIGHT,
				posture: 'sit',
				activity: 'fishing'
			},
			{
				col: 62,
				row: 26,
				facing: Direction.UP,
				posture: 'stand',
				activity: 'birdwatching'
			}
		]),
		scene('zen-garden', 'Unicorn zen garden', 'whimsical', [
			...[27, 38].map((col) => ({
				col,
				row: 24,
				poseCol: col,
				poseRow: 23,
				facing: Direction.UP,
				posture: 'sit' as const,
				activity: 'zen' as const
			}))
		]),
		scene('garden-lunch', 'Garden lunch', 'outdoors', [
			...[35, 36].map((col) => ({
				col,
				row: 35,
				poseCol: col,
				poseRow: 33,
				facing: Direction.UP,
				posture: 'sit' as const,
				activity: 'picnic' as const
			}))
		]),
		scene('hammock-rest', 'Hammock rest', 'quiet', [
			{
				col: 40,
				row: 34,
				poseCol: 40,
				poseRow: 32,
				facing: Direction.RIGHT,
				posture: 'sit',
				activity: 'hammock'
			}
		]),
		scene('music-corner', 'Headphones and records', 'quiet', [
			{
				col: 57,
				row: 30,
				facing: Direction.LEFT,
				posture: 'stand',
				activity: 'headphones'
			}
		]),
		scene('aquarium-pause', 'Aquarium pause', 'whimsical', [
			{
				col: 59,
				row: 23,
				facing: Direction.LEFT,
				posture: 'stand',
				activity: 'aquarium'
			}
		]),
		scene('canopy-chat', 'Canopy garden', 'outdoors', [
			...[21, 22].map((row) => ({
				col: 23,
				row,
				poseCol: 22,
				poseRow: row,
				facing: Direction.LEFT,
				posture: 'sit' as const,
				activity: 'garden' as const
			}))
		]),
		...(preferences.pets
			? [
					scene('pet-care', 'Office pet visit', 'whimsical', [
						{
							col: 12,
							row: 33,
							facing: Direction.LEFT,
							posture: 'stand',
							activity: 'pet-care'
						}
					])
				]
			: [])
	];
	const activityScenes = proposedScenes
		.map((candidate) => ({
			...candidate,
			spots: candidate.spots.filter(
				(spot) =>
					spot.row >= 19 &&
					spot.row <= 35 &&
					!restingBlocked.has(`${spot.col},${spot.row}`) &&
					!restingReserved.has(`${spot.col},${spot.row}`) &&
					restingReachable.has(`${spot.col},${spot.row}`)
			)
		}))
		.filter((candidate) => candidate.spots.length > 0);
	// Interleave one pose from each scene before taking a second pose. Small
	// companies therefore still show variety; large companies fill the groups.
	const authoredRestingSpots = Array.from(
		{ length: Math.max(...activityScenes.map((candidate) => candidate.spots.length), 0) },
		(_, spotIndex) => activityScenes.flatMap((candidate) => candidate.spots[spotIndex] ?? [])
	).flat();
	const authoredApproaches = new Set(authoredRestingSpots.map((spot) => `${spot.col},${spot.row}`));
	const fallbackRestingSpots = Array.from({ length: 17 }, (_, rowOffset) => rowOffset + 19)
		.flatMap((row) => Array.from({ length: 65 }, (_, colOffset) => ({ col: colOffset + 4, row })))
		.filter(
			(spot) =>
				!restingBlocked.has(`${spot.col},${spot.row}`) &&
				!restingReserved.has(`${spot.col},${spot.row}`) &&
				!authoredApproaches.has(`${spot.col},${spot.row}`) &&
				restingReachable.has(`${spot.col},${spot.row}`)
		)
		.toSorted(
			(a, b) =>
				((a.col + a.row) % 2) - ((b.col + b.row) % 2) ||
				stableHash(`rest:${a.col}:${a.row}`) - stableHash(`rest:${b.col}:${b.row}`)
		)
		.map((spot, index) => ({
			...spot,
			facing: [Direction.RIGHT, Direction.LEFT][index % 2],
			posture: 'stand' as const,
			activity: 'conversation' as const,
			tone: 'social' as const,
			groupId: `fallback-chat-${Math.floor(index / 2)}`
		}));
	const restingSpots = [...authoredRestingSpots, ...fallbackRestingSpots];

	const signature = JSON.stringify({
		version: OFFICE_PLAN_VERSION,
		teams: teams.map((team) => [team.id, team.name]),
		members: visibleMembers.map((member) => [member.actorId, member.teamId]),
		preferences
	});
	return {
		layout: baseLayout,
		zones,
		home,
		interactionPoints,
		activityScenes,
		restingSpots,
		waitingSpots,
		protectedPath,
		areaMappings,
		animatedAmenities,
		landmark: unicorn,
		visibleMemberCount: visibleMembers.length,
		signature
	};
}

function furnitureBlockedTiles(layout: OfficeLayout): Set<string> {
	const blocked = new Set<string>();
	for (const item of layout.furniture) {
		const entry = catalogEntry(item.type);
		if (!entry) continue;
		for (let dr = 0; dr < entry.footprintH; dr += 1) {
			for (let dc = 0; dc < entry.footprintW; dc += 1)
				blocked.add(`${item.col + dc},${item.row + dr}`);
		}
	}
	return blocked;
}

function reachableTiles(
	layout: OfficeLayout,
	start: { col: number; row: number },
	blocked: Set<string>
): Set<string> {
	const reached = new Set<string>();
	const queue = [start];
	while (queue.length) {
		const current = queue.shift()!;
		const key = `${current.col},${current.row}`;
		if (reached.has(key) || blocked.has(key)) continue;
		if (
			current.col < 0 ||
			current.row < 0 ||
			current.col >= layout.cols ||
			current.row >= layout.rows
		)
			continue;
		const tile = layout.tiles[current.row * layout.cols + current.col];
		if (tile === TileType.WALL || tile === TileType.VOID) continue;
		reached.add(key);
		queue.push(
			{ col: current.col + 1, row: current.row },
			{ col: current.col - 1, row: current.row },
			{ col: current.col, row: current.row + 1 },
			{ col: current.col, row: current.row - 1 }
		);
	}
	return reached;
}

/** Deterministic geometry evidence. The browser engine remains authoritative for actual movement. */
export function validateOfficePlan(plan: OfficePlan): OfficePlanValidation {
	const { layout } = plan;
	const errors: string[] = [];
	let voidCount = 0;
	let floorCount = 0;
	for (let row = 0; row < layout.rows; row += 1) {
		for (let col = 0; col < layout.cols; col += 1) {
			const edge = row === 0 || col === 0 || row === layout.rows - 1 || col === layout.cols - 1;
			const tile = layout.tiles[row * layout.cols + col];
			if (tile === TileType.VOID) voidCount += 1;
			else floorCount += 1;
			if (edge && tile !== TileType.VOID)
				errors.push(`The campus touches the world edge at ${col},${row}`);
			if (tile === TileType.WALL) errors.push(`The open campus contains a wall at ${col},${row}`);
		}
	}
	const voidRatio = voidCount / layout.tiles.length;
	if (voidRatio < 0.3 || voidRatio > 0.7)
		errors.push(`The irregular campus void ratio ${voidRatio.toFixed(2)} escaped its useful range`);
	const connectedFloor = reachableTiles(layout, plan.home, new Set());
	if (connectedFloor.size !== floorCount)
		errors.push(`Only ${connectedFloor.size} of ${floorCount} campus floor tiles are connected`);
	const occupied = new Map<string, { item: PlacedFurniture; entry: CatalogEntry }>();
	for (const item of layout.furniture) {
		const entry = catalogEntry(item.type);
		if (!entry) {
			errors.push(`Unknown furniture ${item.type}`);
			continue;
		}
		if (
			item.col < 0 ||
			item.row < 0 ||
			item.col + entry.footprintW > layout.cols ||
			item.row + entry.footprintH > layout.rows
		) {
			errors.push(`${item.uid} is outside the ${layout.cols}x${layout.rows} floor`);
			continue;
		}
		for (let dr = 0; dr < entry.footprintH; dr += 1) {
			for (let dc = 0; dc < entry.footprintW; dc += 1) {
				const key = `${item.col + dc},${item.row + dr}`;
				const tile = layout.tiles[(item.row + dr) * layout.cols + item.col + dc];
				if (tile === TileType.WALL || tile === TileType.VOID)
					errors.push(`${item.uid} sits outside the campus floor at ${key}`);
				const prior = occupied.get(key);
				const surfacePair = entry.canPlaceOnSurfaces && prior?.entry.isDesk;
				const reverseSurfacePair = entry.isDesk && prior?.entry.canPlaceOnSurfaces;
				if (prior && !surfacePair && !reverseSurfacePair)
					errors.push(`${item.uid} overlaps ${prior.item.uid} at ${key}`);
				else occupied.set(key, { item, entry });
			}
		}
	}
	const blocked = furnitureBlockedTiles(layout);
	const reached = reachableTiles(layout, plan.home, blocked);
	if (!reached.has(`${plan.home.col},${plan.home.row}`))
		errors.push('The office home point is blocked');
	for (const point of [
		...plan.interactionPoints,
		...plan.restingSpots.map((spot, index) => ({ ...spot, id: `resting-${index}` })),
		...plan.waitingSpots.map((spot, index) => ({ ...spot, id: `waiting-${index}` })),
		...plan.protectedPath.map((spot, index) => ({ ...spot, id: `circulation-${index}` }))
	]) {
		if (!reached.has(`${point.col},${point.row}`))
			errors.push(`${point.id} is not reachable from the campus circulation`);
	}
	for (const zone of plan.zones) {
		let hasReachableEdge = false;
		for (let col = zone.col; col < zone.col + zone.width; col += 1) {
			if (reached.has(`${col},${zone.row + zone.height - 1}`) || reached.has(`${col},${zone.row}`))
				hasReachableEdge = true;
		}
		if (!hasReachableEdge) errors.push(`${zone.label} has no reachable circulation edge`);
	}
	const seatCount = layout.furniture.filter((item) => item.type.includes('CHAIR')).length;
	if (seatCount < plan.visibleMemberCount)
		errors.push(`Only ${seatCount} seats exist for ${plan.visibleMemberCount} visible people`);
	if (plan.restingSpots.length < plan.visibleMemberCount)
		errors.push(
			`Only ${plan.restingSpots.length} commons spots exist for ${plan.visibleMemberCount} visible people`
		);
	for (const definition of AMENITY_DEFINITIONS) {
		if (definition.type === PET_CORNER_TYPE && !(layout.pets?.length ?? 0)) continue;
		if (!layout.furniture.some((item) => item.type === definition.type))
			errors.push(`${definition.label} is missing`);
	}
	const interactionKinds = new Set(plan.interactionPoints.map((point) => point.kind));
	for (const kind of [
		'nourishment',
		'focus',
		'recovery',
		'reading',
		'social',
		'belonging',
		'garden'
	] satisfies AmenityKind[]) {
		if (!interactionKinds.has(kind))
			errors.push(`The ${kind} care category has no interaction point`);
	}
	if (!layout.furniture.some((item) => item.type === UNICORN_TYPE))
		errors.push('The whimsical landmark is missing');
	return {
		valid: errors.length === 0,
		errors,
		seatCount,
		walkableTileCount: reached.size,
		reachableInteractionCount: plan.interactionPoints.filter((point) =>
			reached.has(`${point.col},${point.row}`)
		).length,
		cols: layout.cols,
		rows: layout.rows
	};
}
