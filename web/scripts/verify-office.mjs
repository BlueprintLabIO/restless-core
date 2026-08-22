import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { createServer } from 'vite';

const root = resolve(import.meta.dirname, '..');
const server = await createServer({ root, appType: 'custom', server: { middlewareMode: true } });

try {
	const catalogJson = JSON.parse(
		await readFile(resolve(root, 'static/vendor/pixel-agents/assets/runtime-catalog.json'), 'utf8')
	);
	const catalogModule = await server.ssrLoadModule(
		'/src/lib/vendor/pixel-agents/webview-ui/src/office/layout/furnitureCatalog.ts'
	);
	const amenities = await server.ssrLoadModule('/src/lib/office/amenities.ts');
	const planModule = await server.ssrLoadModule('/src/lib/office/officePlan.ts');
	const officeTypes = await server.ssrLoadModule(
		'/src/lib/vendor/pixel-agents/webview-ui/src/office/types.ts'
	);
	const projectionModule = await server.ssrLoadModule('/src/lib/office/projection.ts');
	const bubbleModule = await server.ssrLoadModule('/src/lib/office/bubblePlacement.ts');
	const campusModule = await server.ssrLoadModule('/src/lib/office/campusBackdrop.ts');
	const behaviourModule = await server.ssrLoadModule('/src/lib/office/officeBehaviour.ts');
	const runtimeCatalog = [...catalogJson.catalog, ...amenities.AMENITY_CATALOG];
	const sprites = Object.fromEntries(runtimeCatalog.map((entry) => [entry.id, [['#000000']]]));
	if (!catalogModule.buildDynamicCatalog({ catalog: runtimeCatalog, sprites })) {
		throw new Error('Could not initialize the deterministic office catalogue.');
	}

	const shapes = [];
	const preferenceCases = [
		['default-lush-pets', planModule.DEFAULT_OFFICE_PREFERENCES],
		['calm-pets', { ...planModule.DEFAULT_OFFICE_PREFERENCES, decorDensity: 'calm' }],
		[
			'calm-no-pets',
			{ ...planModule.DEFAULT_OFFICE_PREFERENCES, decorDensity: 'calm', pets: false }
		]
	];
	for (const [preferenceCase, preferences] of preferenceCases) {
		for (const teamCount of [0, 1, 2, 4, 8]) {
			for (const peopleCount of [1, 6, 20]) {
				const teams = Array.from({ length: teamCount }, (_, index) => ({
					id: `team-${index + 1}`,
					name: `Team ${index + 1}`,
					brief: '',
					lead_actor_id: `staff-${index + 1}`,
					created_by: 'exec',
					created_at: '2026-08-20T00:00:00Z',
					member_count: 0,
					in_motion_count: 0,
					blocked_count: 0
				}));
				const members = Array.from({ length: peopleCount }, (_, index) => ({
					actorId: index === 0 ? 'exec' : `staff-${index}`,
					teamId: index === 0 || teamCount === 0 ? null : teams[(index - 1) % teamCount].id
				}));
				for (const team of teams) {
					team.member_count = members.filter((member) => member.teamId === team.id).length;
				}
				const plan = planModule.createCompanyOfficePlan(teams, members, preferences);
				const result = planModule.validateOfficePlan(plan);
				const floorTileCount = plan.layout.tiles.filter(
					(tile) => tile !== officeTypes.TileType.VOID
				).length;
				shapes.push({ preferenceCase, teamCount, peopleCount, floorTileCount, ...result });
				const tileAt = (col, row) => plan.layout.tiles[row * plan.layout.cols + col];
				const protectedKeys = new Set(
					plan.protectedPath.map((point) => `${point.col},${point.row}`)
				);
				const requiredLanes = [
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
				if (
					plan.layout.cols !== 72 ||
					plan.layout.rows !== 52 ||
					plan.layout.tiles.includes(officeTypes.TileType.WALL) ||
					tileAt(24, 21) !== officeTypes.TileType.VOID ||
					tileAt(42, 21) !== officeTypes.TileType.VOID ||
					tileAt(65, 24) === officeTypes.TileType.VOID ||
					requiredLanes.some(
						(point) =>
							!protectedKeys.has(`${point.col},${point.row}`) ||
							tileAt(point.col, point.row) === officeTypes.TileType.VOID
					)
				)
					throw new Error(
						`${preferenceCase}: ${teamCount} Teams / ${peopleCount} people lost the open C-campus or its paired circulation.`
					);
				for (const type of [
					amenities.GREENHOUSE_TYPE,
					amenities.PROJECT_TABLE_TYPE,
					amenities.LAKESIDE_DOCK_TYPE
				]) {
					if (!plan.layout.furniture.some((item) => item.type === type))
						throw new Error(`${type} is missing from the lakeside campus.`);
				}
				for (const activity of ['zen', 'project', 'fishing', 'birdwatching']) {
					if (!plan.restingSpots.some((spot) => spot.activity === activity))
						throw new Error(`${activity} is missing from the authored campus scenes.`);
				}
				if (JSON.stringify(plan).toLowerCase().includes('company lead'))
					throw new Error('The planner reintroduced the invented Company lead label.');
				const assignedRestingSpots = plan.restingSpots.slice(0, peopleCount);
				const restingKeys = new Set(assignedRestingSpots.map((spot) => `${spot.col},${spot.row}`));
				const poseKeys = new Set(
					assignedRestingSpots.map(
						(spot) => `${spot.poseCol ?? spot.col},${spot.poseRow ?? spot.row}`
					)
				);
				if (
					plan.restingSpots.length < peopleCount ||
					restingKeys.size !== assignedRestingSpots.length ||
					poseKeys.size !== assignedRestingSpots.length ||
					assignedRestingSpots.some((spot) => spot.row < 19 || spot.row > 35)
				)
					throw new Error(
						`${preferenceCase}: ${teamCount} Teams / ${peopleCount} people did not receive unique commons resting spots.`
					);
				if (
					peopleCount === 20 &&
					(assignedRestingSpots.filter((spot) => spot.posture === 'sit').length < 10 ||
						assignedRestingSpots.filter((spot) => spot.activity === 'pool').length < 2 ||
						assignedRestingSpots.filter((spot) => spot.activity === 'table-game').length < 2 ||
						assignedRestingSpots.filter((spot) => spot.activity === 'arcade').length < 2 ||
						assignedRestingSpots.filter((spot) => spot.activity === 'fishing').length !== 1 ||
						new Set(assignedRestingSpots.map((spot) => spot.activity)).size < 9 ||
						new Set(assignedRestingSpots.map((spot) => spot.tone)).size < 5)
				)
					throw new Error(
						`${preferenceCase}: ${teamCount} Teams did not retain the diverse 20-person scene grammar: ` +
							`${assignedRestingSpots.filter((spot) => spot.posture === 'sit').length} seated, ` +
							`${new Set(assignedRestingSpots.map((spot) => spot.activity)).size} activities, ` +
							`${new Set(assignedRestingSpots.map((spot) => spot.tone)).size} tones.`
					);
				const hasLushDecor = plan.layout.furniture.some((item) => item.uid.startsWith('lush-'));
				const petCount = plan.layout.pets?.length ?? 0;
				if (
					(preferences.decorDensity === 'lush' && !hasLushDecor) ||
					(preferences.decorDensity === 'calm' && hasLushDecor) ||
					(preferences.pets && petCount === 0) ||
					(!preferences.pets && petCount !== 0)
				)
					throw new Error(`${preferenceCase} did not retain its presentation preference.`);
				if (!result.valid) {
					throw new Error(
						`${preferenceCase}: ${teamCount} Teams / ${peopleCount} people failed:\n` +
							result.errors.join('\n')
					);
				}
			}
		}
	}

	const widest = Math.max(...shapes.map((shape) => shape.cols));
	const tallest = Math.max(...shapes.map((shape) => shape.rows));
	if (widest > 72 || tallest > 52) throw new Error(`Bounded view exceeded: ${widest}x${tallest}`);
	if (new Set(shapes.map((shape) => shape.floorTileCount)).size < 4)
		throw new Error('Real company size no longer changes the generated pavilion footprint.');
	const unicorn = amenities.AMENITY_DEFINITIONS.find(
		(definition) => definition.type === amenities.UNICORN_TYPE
	);
	if (!unicorn || unicorn.footprintW !== 4 || unicorn.footprintH !== 4)
		throw new Error('The campus landmark is no longer the enlarged 4x4 unicorn.');
	if (
		campusModule.CAMPUS_BACKDROP_VERSION !== 5 ||
		campusModule.CAMPUS_MOTION_CHANNELS.join(',') !== 'water,leaves,wildlife'
	)
		throw new Error(
			'The retained campus background exceeded or lost its three-channel motion budget.'
		);
	const wildlifeSamples = Array.from(
		{ length: campusModule.CAMPUS_WILDLIFE_CYCLE_MS / 1_000 },
		(_, index) => campusModule.campusWildlifeAt(index * 1_000)?.kind ?? null
	);
	const visibleWildlifeSamples = wildlifeSamples.filter(Boolean);
	if (
		campusModule.CAMPUS_WILDLIFE_CYCLE_MS !== 72_000 ||
		new Set(visibleWildlifeSamples).size !== 3 ||
		visibleWildlifeSamples.length > wildlifeSamples.length / 4 ||
		wildlifeSamples.filter((kind) => kind === 'whale').length > 5 ||
		campusModule.campusWildlifeAt(0) !== null ||
		campusModule.campusWildlifeAt(8_000)?.kind !== 'birds' ||
		campusModule.campusWildlifeAt(29_000)?.kind !== 'butterfly' ||
		campusModule.campusWildlifeAt(58_000)?.kind !== 'whale'
	)
		throw new Error(
			'The wildlife cycle no longer keeps birds, butterfly and whale sparse and mutually exclusive.'
		);
	const firstCamera = {
		officeLeft: 100,
		officeTop: 60,
		officeCols: 72,
		officeRows: 52,
		tilePixelSize: 16
	};
	const secondCamera = {
		...firstCamera,
		officeLeft: -40,
		officeTop: 120,
		tilePixelSize: 32
	};
	const firstShore = campusModule.projectCampusPoint(firstCamera, 0.61, 0.46);
	const secondShore = campusModule.projectCampusPoint(secondCamera, 0.61, 0.46);
	const expectedSecondShore = {
		x: secondCamera.officeLeft + (firstShore.x - firstCamera.officeLeft) * 2,
		y: secondCamera.officeTop + (firstShore.y - firstCamera.officeTop) * 2
	};
	if (
		Math.abs(secondShore.x - expectedSecondShore.x) > Number.EPSILON ||
		Math.abs(secondShore.y - expectedSecondShore.y) > Number.EPSILON
	)
		throw new Error('The campus backdrop no longer shares the office camera projection.');
	if (
		behaviourModule.MAX_ANIMATED_ACTIVITY_SCENES !== 4 ||
		behaviourModule.MAX_AMBIENT_VISITORS !== 3 ||
		behaviourModule.MAX_AMBIENT_CHAT_BUBBLES !== 1
	)
		throw new Error(
			'The office calmness budget changed without updating its deterministic contract.'
		);
	const completionNow = Date.parse('2026-08-20T12:00:00Z');
	const celebrationCases = [
		[
			true,
			{
				previousStatus: 'active',
				currentStatus: 'completed',
				updatedAt: '2026-08-20T10:00:00Z',
				now: completionNow
			}
		],
		[
			true,
			{
				previousStatus: undefined,
				currentStatus: 'completed',
				updatedAt: '2026-08-20T11:59:20Z',
				now: completionNow
			}
		],
		[
			false,
			{
				previousStatus: undefined,
				currentStatus: 'completed',
				updatedAt: '2026-08-20T11:50:00Z',
				now: completionNow
			}
		],
		[
			false,
			{
				previousStatus: 'completed',
				currentStatus: 'completed',
				updatedAt: '2026-08-20T11:59:59Z',
				now: completionNow
			}
		],
		[
			false,
			{
				previousStatus: 'active',
				currentStatus: 'blocked',
				updatedAt: '2026-08-20T11:59:59Z',
				now: completionNow
			}
		]
	];
	for (const [expected, input] of celebrationCases) {
		if (behaviourModule.shouldCelebrateWorkCompletion(input) !== expected)
			throw new Error('Completion celebration escaped source-owned Work truth.');
	}
	const overflowPlan = planModule.createCompanyOfficePlan(
		[],
		Array.from({ length: 31 }, (_, index) => ({
			actorId: index === 0 ? 'exec' : `overflow-${index}`,
			teamId: null
		})),
		planModule.DEFAULT_OFFICE_PREFERENCES
	);
	const overflowResult = planModule.validateOfficePlan(overflowPlan);
	if (overflowPlan.visibleMemberCount !== 20 || !overflowResult.valid) {
		throw new Error('Oversized company did not degrade to the proved 20-person view.');
	}
	const decorationPlan = planModule.createCompanyOfficePlan(
		[],
		[{ actorId: 'exec', teamId: null }],
		planModule.DEFAULT_OFFICE_PREFERENCES
	);
	const decorationRoutes = {
		home: decorationPlan.home,
		requiredPoints: [
			...decorationPlan.interactionPoints,
			...decorationPlan.restingSpots,
			...decorationPlan.waitingSpots,
			...decorationPlan.protectedPath
		]
	};
	const protectedPoint = decorationPlan.interactionPoints.find((point) =>
		planModule.isDecorationPlacementValid(decorationPlan.layout, 'PLANT_2', point.col, point.row)
	);
	if (
		!protectedPoint ||
		planModule.isDecorationPlacementValid(
			decorationPlan.layout,
			'PLANT_2',
			protectedPoint.col,
			protectedPoint.row,
			decorationRoutes
		)
	)
		throw new Error('Decoration validation did not protect a required amenity approach.');
	const protectedRibbonPoint = decorationPlan.protectedPath[1];
	if (
		!planModule.isDecorationPlacementValid(
			decorationPlan.layout,
			'PLANT_2',
			protectedRibbonPoint.col,
			protectedRibbonPoint.row
		) ||
		planModule.isDecorationPlacementValid(
			decorationPlan.layout,
			'PLANT_2',
			protectedRibbonPoint.col,
			protectedRibbonPoint.row,
			decorationRoutes
		)
	)
		throw new Error('Decoration validation did not protect the paired campus circulation.');
	let validDecoration = null;
	for (let row = 1; row < decorationPlan.layout.rows - 1 && !validDecoration; row += 1) {
		for (let col = 1; col < decorationPlan.layout.cols - 1; col += 1) {
			if (
				planModule.isDecorationPlacementValid(
					decorationPlan.layout,
					'PLANT_2',
					col,
					row,
					decorationRoutes
				)
			) {
				validDecoration = { uid: 'verified-plant', type: 'PLANT_2', col, row };
				break;
			}
		}
	}
	if (!validDecoration)
		throw new Error('The retained office offered no safe decoration placement.');
	const decoratedPlan = planModule.createCompanyOfficePlan(
		[],
		[{ actorId: 'exec', teamId: null }],
		{ ...planModule.DEFAULT_OFFICE_PREFERENCES, decorations: [validDecoration] }
	);
	if (
		!decoratedPlan.layout.furniture.some((item) => item.uid === validDecoration.uid) ||
		!planModule.validateOfficePlan(decoratedPlan).valid
	)
		throw new Error('A safe decoration was not retained as a walkable office plan.');

	const now = new Date('2026-08-20T12:00:00Z');
	const person = (sessionRunning, observedAt) => ({
		actor_id: 'staff-1',
		kind: 'staff',
		role: 'maker',
		display: 'Mira',
		model: null,
		team_id: null,
		spent_usd: 0,
		session_running: sessionRunning,
		session_observed_at: observedAt,
		model_cooldown: null
	});
	const work = (status) => ({
		id: 'work-1',
		goal_id: 'goal-1',
		owner_id: 'staff-1',
		title: 'Prepare the launch review',
		outcome: 'A prepared review',
		status,
		resolution: '',
		priority: 1,
		expected_artifact: '',
		repo: null,
		base_ref: null,
		integration_branch: null,
		worktree: null,
		revision: 1,
		attempt_limit: null,
		created_at: '2026-08-20T10:00:00Z',
		updated_at: '2026-08-20T11:59:00Z'
	});
	const attempt = {
		id: 'attempt-1',
		work_id: 'work-1',
		revision: 1,
		attempt_no: 1,
		actor_id: 'staff-1',
		session_id: 'session-1',
		state: 'running',
		trigger: 'wake',
		input_fingerprint: '',
		feedback_cursor: 0,
		model: null,
		started_at: '2026-08-20T11:58:00Z',
		finished_at: null,
		summary: 'Rendering the final candidate'
	};
	const projectTruth = ({
		sessionRunning = false,
		observedAt = null,
		status = null,
		attempts = [],
		runtimeStatus = 'running',
		orgintelStatus = 'available'
	}) => {
		const cockpit = {
			people: [person(sessionRunning, observedAt)],
			teams: [],
			source_health: { runtime: runtimeStatus, orgintel: orgintelStatus }
		};
		const graph = {
			work: status ? [work(status)] : [],
			attempts,
			artifacts: [],
			gates: [],
			gate_runs: []
		};
		return projectionModule.projectOfficeMembers('company_test', cockpit, graph, { now })[0];
	};
	const truthCases = [
		[
			'observed',
			projectTruth({
				sessionRunning: true,
				observedAt: '2026-08-20T11:59:55Z',
				status: 'active',
				attempts: [attempt]
			})
		],
		['in-motion', projectTruth({ status: 'active' })],
		['waiting', projectTruth({ status: 'blocked' })],
		[
			'stale',
			projectTruth({
				sessionRunning: true,
				observedAt: '2026-08-20T11:59:20Z',
				status: 'active',
				attempts: [attempt]
			})
		],
		[
			'unknown',
			projectTruth({ sessionRunning: true, observedAt: '2026-08-20T11:59:55Z', status: 'active' })
		],
		['unavailable', projectTruth({ status: 'active', runtimeStatus: 'unavailable' })],
		['unavailable', projectTruth({ status: 'active', orgintelStatus: 'unavailable' })],
		['available', projectTruth({})]
	];
	for (const [expected, member] of truthCases) {
		if (member.presence !== expected)
			throw new Error(`Expected ${expected}, received ${member.presence}`);
		if (expected !== 'observed' && member.semanticActivity)
			throw new Error(`${expected} must not claim semantic activity`);
	}
	const observed = truthCases[0][1];
	if (!observed.semanticActivity || observed.currentStep !== attempt.summary)
		throw new Error('Fresh associated Work did not retain its observed current step.');
	const waiting = truthCases[2][1];
	if (!waiting.workHref?.includes('/work/work-1'))
		throw new Error('Waiting did not link to canonical Work.');
	const clearBubble = bubbleModule.chooseBubblePlacement({
		canvasWidth: 800,
		canvasHeight: 600,
		anchorX: 400,
		anchorY: 300,
		width: 240,
		height: 80,
		scale: 2,
		obstacles: [{ left: 360, top: 204, width: 80, height: 40 }]
	});
	if (clearBubble.overlapArea !== 0)
		throw new Error('Bubble placement did not move away from nearby furniture.');
	const edgeBubble = bubbleModule.chooseBubblePlacement({
		canvasWidth: 160,
		canvasHeight: 120,
		anchorX: 2,
		anchorY: 2,
		width: 100,
		height: 80,
		scale: 1,
		obstacles: []
	});
	if (
		edgeBubble.left < 8 ||
		edgeBubble.top < 8 ||
		edgeBubble.left + edgeBubble.width > 152 ||
		edgeBubble.top + edgeBubble.height > 112
	)
		throw new Error('Bubble placement escaped its viewport margin.');
	console.log(
		`Office geometry verified: ${shapes.length} shapes, max ${widest}x${tallest}, ` +
			`${Math.min(...shapes.map((shape) => shape.reachableInteractionCount))}-` +
			`${Math.max(...shapes.map((shape) => shape.reachableInteractionCount))} reachable amenities. ` +
			`Truth grammar verified across ${truthCases.length} source states. ` +
			`Every visible person has a unique reachable authored scene pose; 20-person plans retain at least nine quiet, social, playful, whimsical and outdoor activities. ` +
			`The top-down lakeside campus retains exactly three non-semantic motion channels, with wildlife absent for more than three quarters of its cycle. ` +
			`The calmness budget caps four animated scenes, three ambient visitors and one bubble; completion confetti is source-gated. ` +
			`Decoration routes, bubble avoidance and viewport clamping verified.`
	);
} finally {
	await server.close();
}
