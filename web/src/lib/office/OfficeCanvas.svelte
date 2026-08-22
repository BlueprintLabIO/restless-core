<script lang="ts">
	import { onMount } from 'svelte';
	import Check from '@lucide/svelte/icons/check';
	import Eraser from '@lucide/svelte/icons/eraser';
	import Focus from '@lucide/svelte/icons/focus';
	import Minus from '@lucide/svelte/icons/minus';
	import Paintbrush from '@lucide/svelte/icons/paintbrush';
	import PawPrint from '@lucide/svelte/icons/paw-print';
	import Plus from '@lucide/svelte/icons/plus';
	import Sparkles from '@lucide/svelte/icons/sparkles';
	import Undo2 from '@lucide/svelte/icons/undo-2';
	import type { CockpitTeam } from '$lib/model/cockpit';
	import { startGameLoop } from '$lib/vendor/pixel-agents/webview-ui/src/office/engine/gameLoop.js';
	import { OfficeState } from '$lib/vendor/pixel-agents/webview-ui/src/office/engine/officeState.js';
	import { renderFrame } from '$lib/vendor/pixel-agents/webview-ui/src/office/engine/renderer.js';
	import { getCatalogEntry } from '$lib/vendor/pixel-agents/webview-ui/src/office/layout/furnitureCatalog.js';
	import { mapOffset } from '$lib/vendor/pixel-agents/webview-ui/src/office/projection.js';
	import {
		CharacterState,
		TILE_SIZE
	} from '$lib/vendor/pixel-agents/webview-ui/src/office/types.js';
	import {
		createCompanyOfficePlan,
		isDecorationPlacementValid,
		type DecorationType,
		type OfficePlan,
		type OfficePreferences,
		type OfficeRestingSpot
	} from './officePlan';
	import type { OfficeMember } from './projection';
	import { chooseBubblePlacement, type BubbleRect } from './bubblePlacement';
	import { CAMPUS_MOTION_CHANNELS, campusWildlifeAt, drawCampusBackdrop } from './campusBackdrop';
	import {
		MAX_AMBIENT_VISITORS,
		MAX_ANIMATED_ACTIVITY_SCENES,
		shouldCelebrateWorkCompletion
	} from './officeBehaviour';
	import { loadPixelOfficeAssets, type PixelOfficeAssets } from './pixelAssets';

	let {
		members,
		teams,
		preferences,
		selectedActorId = $bindable(null),
		onopen,
		onpreferenceschange
	}: {
		members: OfficeMember[];
		teams: CockpitTeam[];
		preferences: OfficePreferences;
		selectedActorId?: string | null;
		onopen?: (member: OfficeMember) => void;
		onpreferenceschange?: (preferences: OfficePreferences) => void;
	} = $props();

	const decorationOptions: Array<{
		type: DecorationType;
		label: string;
		src: string;
	}> = [
		{
			type: 'PLANT_2',
			label: 'Plant',
			src: '/vendor/pixel-agents/assets/furniture/PLANT_2/PLANT_2.png'
		},
		{
			type: 'LARGE_PLANT',
			label: 'Large plant',
			src: '/vendor/pixel-agents/assets/furniture/LARGE_PLANT/LARGE_PLANT.png'
		},
		{
			type: 'SOFA_FRONT',
			label: 'Sofa',
			src: '/vendor/pixel-agents/assets/furniture/SOFA/SOFA_FRONT.png'
		},
		{
			type: 'COFFEE_TABLE',
			label: 'Coffee table',
			src: '/vendor/pixel-agents/assets/furniture/COFFEE_TABLE/COFFEE_TABLE.png'
		},
		{
			type: 'DOUBLE_BOOKSHELF',
			label: 'Bookshelf',
			src: '/vendor/pixel-agents/assets/furniture/DOUBLE_BOOKSHELF/DOUBLE_BOOKSHELF.png'
		}
	];
	const ZOOM_STEP = 0.25;
	const MAX_CLOSE_ZOOM = 3;
	const ZOOM_EPSILON = 0.001;
	let shell: HTMLDivElement;
	let canvas: HTMLCanvasElement;
	let office = $state<OfficeState | null>(null);
	let plan = $state<OfficePlan | null>(null);
	let assets = $state<PixelOfficeAssets | null>(null);
	let ready = $state(false);
	let error = $state('');
	let hoveredActorId = $state<string | null>(null);
	let decorating = $state(false);
	let selectedDecoration = $state<DecorationType | 'erase'>('PLANT_2');
	let decorationMessage = $state('');
	let documentVisible = $state(true);
	let reducedMotion = $state(false);
	let devicePixelRatio = 2;
	let zoomCss = $state(2.5);
	let minZoomCss = $state(0.5);
	let maxZoomCss = $state(1);
	let cameraPan = { x: 0, y: 0 };
	let lastOffset = { x: 0, y: 0 };
	let lastZoom = 5;
	let lastPresence = new Map<string, OfficeMember['presence']>();
	let lastSemantic = new Map<string, boolean>();
	let simulationClock = 0;
	let behaviourTimer = 0;
	let ambientCursor = 0;
	let ambientReservations = new Map<string, string>();
	let reservedPoints = new Map<string, string>();
	let ambientDwell = new Map<string, number>();
	let ambientCooldown = new Map<string, number>();
	let lastAmbientKind = new Map<string, string>();
	let lastChatBeat = -1;
	let chatSpeakerId: number | null = null;
	let chatBubbleUntil = 0;
	let planSignature = '';
	let worldShape = '';
	let layoutRebuildCount = 0;
	let renderedFrameCount = 0;
	let longFrameCount = 0;
	let longestFrameMs = 0;
	let lastWorkStatus = new Map<string, string | null>();
	let completionCelebrations = new Map<number, number>();
	let pointer = $state<{
		id: number;
		startX: number;
		startY: number;
		lastX: number;
		lastY: number;
		moved: boolean;
	} | null>(null);

	const selectedMember = $derived(
		members.find((member) => member.actorId === selectedActorId) ?? null
	);
	const memberByNumericId = $derived(new Map(members.map((member) => [member.numericId, member])));
	const canZoomOut = $derived(zoomCss > minZoomCss + ZOOM_EPSILON);
	const canZoomIn = $derived(zoomCss < maxZoomCss - ZOOM_EPSILON);

	$effect(() => {
		if (!assets) return;
		rebuildOffice(teams, members, preferences);
	});

	onMount(() => {
		let destroyed = false;
		let stopLoop: (() => void) | undefined;
		const media = window.matchMedia('(prefers-reduced-motion: reduce)');
		const readMotion = () => {
			reducedMotion = media.matches;
			if (reducedMotion) {
				chatSpeakerId = null;
				chatBubbleUntil = 0;
			}
			if (shell) shell.dataset.motion = reducedMotion ? 'reduced' : 'full';
			if (reducedMotion && office && plan) synchronizeMembers(office, members, plan);
		};
		const readVisibility = () => {
			documentVisible = document.visibilityState === 'visible';
			if (shell) shell.dataset.documentVisible = documentVisible ? 'true' : 'false';
		};
		const wheel = (event: WheelEvent) => handleWheel(event);
		readMotion();
		readVisibility();
		media.addEventListener('change', readMotion);
		document.addEventListener('visibilitychange', readVisibility);
		canvas.addEventListener('wheel', wheel, { passive: false });

		const resize = new ResizeObserver(() => sizeCanvas());
		resize.observe(shell);

		void loadPixelOfficeAssets()
			.then((loadedAssets) => {
				if (destroyed) return;
				assets = loadedAssets;
				rebuildOffice(teams, members, preferences);
				sizeCanvas();
				homeCamera();
				stopLoop = startGameLoop(canvas, {
					update: (delta) => {
						if (documentVisible && !reducedMotion) updateOffice(delta);
					},
					render: (context) => render(context, performance.now())
				});
				ready = true;
			})
			.catch((cause) => {
				if (destroyed) return;
				error = cause instanceof Error ? cause.message : 'The company floor could not be opened.';
			});

		return () => {
			destroyed = true;
			stopLoop?.();
			resize.disconnect();
			canvas.removeEventListener('wheel', wheel);
			media.removeEventListener('change', readMotion);
			document.removeEventListener('visibilitychange', readVisibility);
		};
	});

	function rebuildOffice(
		nextTeams: CockpitTeam[],
		nextMembers: OfficeMember[],
		nextPreferences: OfficePreferences
	) {
		if (!assets) return;
		const nextPlan = createCompanyOfficePlan(nextTeams, nextMembers, nextPreferences);
		if (nextPlan.signature === planSignature && office) {
			synchronizeMembers(office, nextMembers, nextPlan);
			return;
		}
		const nextShape = `${nextPlan.layout.cols}x${nextPlan.layout.rows}:${nextTeams
			.map((team) => team.id)
			.sort()
			.join(',')}`;
		if (office) {
			ambientReservations.clear();
			reservedPoints.clear();
			ambientDwell.clear();
			lastChatBeat = -1;
			chatSpeakerId = null;
			chatBubbleUntil = 0;
			office.setAreaMappings(nextPlan.areaMappings);
			office.rebuildFromLayout(nextPlan.layout);
		} else {
			office = new OfficeState(nextPlan.layout);
			office.setAreaMappings(nextPlan.areaMappings);
		}
		layoutRebuildCount += 1;
		if (shell) shell.dataset.layoutRebuilds = String(layoutRebuildCount);
		plan = nextPlan;
		planSignature = nextPlan.signature;
		synchronizeMembers(office, nextMembers, nextPlan);
		if (worldShape !== nextShape) {
			worldShape = nextShape;
			homeCamera();
		}
	}

	function updateOffice(delta: number) {
		if (!office || !plan) return;
		office.update(delta);
		simulationClock += delta;
		behaviourTimer += delta;
		settleAvailableMembers();
		animateLoungeChat();

		for (const [actorId, pointId] of [...ambientReservations]) {
			const member = members.find((candidate) => candidate.actorId === actorId);
			const character = member ? office.characters.get(member.numericId) : null;
			const point = plan.interactionPoints.find((candidate) => candidate.id === pointId);
			if (!member || member.presence !== 'available' || !character || !point) {
				releaseAmbient(actorId, false);
				continue;
			}
			if (character.path.length > 0) continue;
			if (!ambientDwell.has(actorId)) {
				ambientDwell.set(actorId, 4 + (member.numericId % 4));
				character.dir = point.facing;
				continue;
			}
			const remaining = (ambientDwell.get(actorId) ?? 0) - delta;
			if (remaining > 0) ambientDwell.set(actorId, remaining);
			else releaseAmbient(actorId, true);
		}

		if (behaviourTimer < 3) return;
		behaviourTimer = 0;
		const maximumAmbient = Math.min(
			MAX_AMBIENT_VISITORS,
			Math.max(1, Math.ceil(members.length / 8))
		);
		if (ambientReservations.size >= maximumAmbient || !plan.interactionPoints.length) return;
		const eligible = members.filter((member) => {
			const character = office?.characters.get(member.numericId);
			return (
				member.presence === 'available' &&
				!ambientReservations.has(member.actorId) &&
				(ambientCooldown.get(member.actorId) ?? 0) <= simulationClock &&
				character?.path.length === 0
			);
		});
		if (!eligible.length) return;
		const member = eligible[ambientCursor % eligible.length];
		const occupied = new Set(
			[...office.characters.values()].map((character) => {
				const destination = character.path.at(-1);
				return `${destination?.col ?? character.tileCol},${destination?.row ?? character.tileRow}`;
			})
		);
		const points = plan.interactionPoints.filter(
			(point) =>
				(!point.exclusive || !reservedPoints.has(point.id)) &&
				!occupied.has(`${point.col},${point.row}`)
		);
		if (!points.length) return;
		const freshPoints = points.filter(
			(point) => point.kind !== lastAmbientKind.get(member.actorId)
		);
		const candidates = freshPoints.length ? freshPoints : points;
		const point = candidates[(ambientCursor + member.numericId) % candidates.length];
		if (office.walkToTile(member.numericId, point.col, point.row)) {
			if (chatSpeakerId === member.numericId) chatSpeakerId = null;
			ambientReservations.set(member.actorId, point.id);
			if (point.exclusive) reservedPoints.set(point.id, member.actorId);
			ambientCursor += 1;
		}
	}

	function releaseAmbient(actorId: string, returnToCommons: boolean) {
		const pointId = ambientReservations.get(actorId);
		const point = plan?.interactionPoints.find((candidate) => candidate.id === pointId);
		if (point) lastAmbientKind.set(actorId, point.kind);
		if (pointId) reservedPoints.delete(pointId);
		ambientReservations.delete(actorId);
		ambientDwell.delete(actorId);
		ambientCooldown.set(actorId, simulationClock + 16);
		const member = members.find((candidate) => candidate.actorId === actorId);
		if (returnToCommons && member && office && plan)
			sendToRestingSpot(office, member, members, plan);
	}

	function sendToRestingSpot(
		state: OfficeState,
		member: OfficeMember,
		roster: OfficeMember[],
		nextPlan: OfficePlan
	) {
		const spot = restingSpotFor(member, roster, nextPlan);
		const character = state.characters.get(member.numericId);
		if (!spot || !character) return;
		if (reducedMotion) {
			poseAtRestingSpot(character, spot);
			return;
		}
		const destination = character.path.at(-1);
		const atPose =
			character.tileCol === (spot.poseCol ?? spot.col) &&
			character.tileRow === (spot.poseRow ?? spot.row) &&
			!character.path.length;
		if (
			atPose ||
			(character.tileCol === spot.col &&
				character.tileRow === spot.row &&
				!character.path.length) ||
			(destination?.col === spot.col && destination.row === spot.row)
		) {
			if (!character.path.length) poseAtRestingSpot(character, spot);
			return;
		}
		state.walkToTile(member.numericId, spot.col, spot.row);
	}

	function restingSpotFor(
		member: OfficeMember,
		roster: OfficeMember[],
		nextPlan: OfficePlan
	): OfficeRestingSpot | null {
		const memberIndex = roster.findIndex((candidate) => candidate.actorId === member.actorId);
		return memberIndex >= 0 ? (nextPlan.restingSpots[memberIndex] ?? null) : null;
	}

	function isAtRestingSpot(
		character: NonNullable<ReturnType<OfficeState['characters']['get']>>,
		spot: OfficeRestingSpot
	): boolean {
		return (
			!character.path.length &&
			character.tileCol === (spot.poseCol ?? spot.col) &&
			character.tileRow === (spot.poseRow ?? spot.row)
		);
	}

	function poseAtRestingSpot(
		character: NonNullable<ReturnType<OfficeState['characters']['get']>>,
		spot: OfficeRestingSpot
	) {
		const col = spot.poseCol ?? spot.col;
		const row = spot.poseRow ?? spot.row;
		const state = spot.posture === 'sit' ? CharacterState.TYPE : CharacterState.IDLE;
		if (character.tileCol !== col || character.tileRow !== row) {
			snapCharacter(character, col, row, spot.facing, state);
		} else {
			character.dir = spot.facing;
			if (character.state !== state) {
				character.state = state;
				character.frame = 0;
				character.frameTimer = 0;
			}
		}
		if (spot.posture === 'sit') character.seatTimer = 60;
		character.currentTool =
			spot.activity === 'reading' || spot.activity === 'sketching' ? 'Read' : null;
	}

	function settleAvailableMembers() {
		if (!office || !plan) return;
		for (const member of members) {
			if (member.presence !== 'available' || ambientReservations.has(member.actorId)) continue;
			const character = office.characters.get(member.numericId);
			const spot = restingSpotFor(member, members, plan);
			if (!character || !spot || character.path.length) continue;
			const atApproach = character.tileCol === spot.col && character.tileRow === spot.row;
			if (atApproach || isAtRestingSpot(character, spot)) poseAtRestingSpot(character, spot);
		}
	}

	function animateLoungeChat() {
		if (!office || !plan || reducedMotion) return;
		if (simulationClock >= chatBubbleUntil) chatSpeakerId = null;
		const beat = Math.floor(simulationClock / 2.8);
		if (beat === lastChatBeat) return;
		lastChatBeat = beat;
		const settled = members.flatMap((member) => {
			if (member.presence !== 'available' || ambientReservations.has(member.actorId)) return [];
			const character = office?.characters.get(member.numericId);
			const spot = plan?.restingSpots[members.indexOf(member)];
			return character && spot && isAtRestingSpot(character, spot) ? [{ member, spot }] : [];
		});
		const groups = [...new Set(settled.map(({ spot }) => spot.groupId))].filter(
			(groupId) => settled.filter(({ spot }) => spot.groupId === groupId).length > 1
		);
		if (!groups.length) return;
		const groupId = groups[beat % groups.length];
		const group = settled.filter(({ spot }) => spot.groupId === groupId);
		const speaker = group[beat % group.length];
		chatSpeakerId = speaker.member.numericId;
		chatBubbleUntil = simulationClock + 2.4;
	}

	function sendToWaitingSpot(
		state: OfficeState,
		member: OfficeMember,
		spot: { col: number; row: number }
	) {
		const character = state.characters.get(member.numericId);
		if (!character) return;
		if (reducedMotion) {
			snapCharacter(character, spot.col, spot.row, character.dir, CharacterState.IDLE);
			return;
		}
		state.walkToTile(member.numericId, spot.col, spot.row);
	}

	function sendToWorkstation(state: OfficeState, member: OfficeMember) {
		const character = state.characters.get(member.numericId);
		const seat = character?.seatId ? state.seats.get(character.seatId) : null;
		if (!character || !seat) return;
		if (reducedMotion) {
			snapCharacter(
				character,
				seat.seatCol,
				seat.seatRow,
				seat.facingDir,
				member.semanticActivity ? CharacterState.TYPE : CharacterState.IDLE
			);
			return;
		}
		state.sendToSeat(member.numericId);
	}

	function snapCharacter(
		character: NonNullable<ReturnType<OfficeState['characters']['get']>>,
		col: number,
		row: number,
		facing: (typeof character)['dir'],
		state: CharacterState
	) {
		character.tileCol = col;
		character.tileRow = row;
		character.x = col * TILE_SIZE + TILE_SIZE / 2;
		character.y = row * TILE_SIZE + TILE_SIZE / 2;
		character.path = [];
		character.moveProgress = 0;
		character.state = state;
		character.dir = facing;
		character.frame = 0;
		character.frameTimer = 0;
	}

	function sizeCanvas() {
		if (!canvas || !shell) return;
		const rectangle = shell.getBoundingClientRect();
		devicePixelRatio = Math.max(2, Math.min(window.devicePixelRatio || 1, 2));
		canvas.width = Math.max(1, Math.round(rectangle.width * devicePixelRatio));
		canvas.height = Math.max(1, Math.round(rectangle.height * devicePixelRatio));
		updateZoomBounds();
		lastZoom = Math.max(1, Math.round(zoomCss * devicePixelRatio));
	}

	function synchronizeMembers(
		state: OfficeState,
		nextMembers: OfficeMember[],
		nextPlan: OfficePlan
	) {
		state.setAreaMappings(nextPlan.areaMappings);
		const nextIds = new Set(nextMembers.map((member) => member.numericId));
		for (const character of state.characters.values()) {
			if (!nextIds.has(character.id)) {
				const member = members.find((candidate) => candidate.numericId === character.id);
				if (member) releaseAmbient(member.actorId, false);
				state.removeAgent(character.id);
			}
		}

		for (const member of nextMembers) {
			const areaKey = member.actorId === 'exec' ? '__exec__' : (member.teamId ?? '__company__');
			if (!state.characters.has(member.numericId)) {
				state.addAgent(member.numericId, member.palette, 0, undefined, false, areaKey);
			}
		}

		for (const member of nextMembers) {
			const lead = member.teamId
				? nextMembers.find(
						(candidate) => candidate.teamId === member.teamId && candidate.isTeamLead
					)
				: null;
			state.setTeamInfo(
				member.numericId,
				member.teamName ?? undefined,
				member.role,
				member.isTeamLead,
				lead && lead.actorId !== member.actorId ? lead.numericId : undefined
			);
			const previousSemantic = lastSemantic.get(member.actorId);
			if (previousSemantic !== member.semanticActivity) {
				state.setAgentActive(member.numericId, member.semanticActivity);
				lastSemantic.set(member.actorId, member.semanticActivity);
			}
			state.setAgentAmbient(member.numericId, false);
			state.setAgentTool(member.numericId, null);
			if (member.presence === 'waiting') state.showPermissionBubble(member.numericId);
			else state.clearPermissionBubble(member.numericId);
			if (member.presence !== 'available' && chatSpeakerId === member.numericId)
				chatSpeakerId = null;

			const previous = lastPresence.get(member.actorId);
			const workStatus = member.work?.status ?? null;
			const previousWorkStatus = lastWorkStatus.get(member.actorId);
			if (
				!reducedMotion &&
				shouldCelebrateWorkCompletion({
					previousStatus: previousWorkStatus,
					currentStatus: workStatus,
					updatedAt: member.work?.updated_at ?? null,
					now: Date.now()
				})
			)
				completionCelebrations.set(member.numericId, performance.now() + 5_000);
			lastWorkStatus.set(member.actorId, workStatus);
			if (previous && previous !== member.presence) releaseAmbient(member.actorId, false);
			if (member.presence === 'waiting' && previous !== 'waiting') {
				const waitingSpot =
					nextPlan.waitingSpots[
						Math.abs(member.numericId) % Math.max(1, nextPlan.waitingSpots.length)
					];
				if (waitingSpot) sendToWaitingSpot(state, member, waitingSpot);
			} else if (
				(member.semanticActivity || member.presence === 'in-motion') &&
				previous !== member.presence
			) {
				sendToWorkstation(state, member);
			} else if (member.presence === 'available' && !ambientReservations.has(member.actorId)) {
				sendToRestingSpot(state, member, nextMembers, nextPlan);
			}
			lastPresence.set(member.actorId, member.presence);
		}
		for (const actorId of [...lastPresence.keys()]) {
			if (!nextMembers.some((member) => member.actorId === actorId)) {
				lastPresence.delete(actorId);
				lastSemantic.delete(actorId);
				lastWorkStatus.delete(actorId);
				releaseAmbient(actorId, false);
			}
		}
		if (selectedActorId && !nextMembers.some((member) => member.actorId === selectedActorId))
			selectedActorId = null;
	}

	function render(context: CanvasRenderingContext2D, now: number) {
		if (!office || !plan || !canvas.width || !canvas.height) return;
		const frameStartedAt = performance.now();
		const hoveredMember = hoveredActorId
			? (members.find((member) => member.actorId === hoveredActorId) ?? null)
			: null;
		const bubbleMember = hoveredMember;
		if (!bubbleMember && shell) {
			delete shell.dataset.bubbleTail;
			delete shell.dataset.bubbleOverlap;
		}
		office.selectedAgentId = selectedMember?.numericId ?? null;
		context.imageSmoothingEnabled = false;
		lastZoom = Math.max(1, Math.round(zoomCss * devicePixelRatio));
		animateRestorativeDecor(now);
		const frame = renderFrame(
			context,
			canvas.width,
			canvas.height,
			office.tileMap,
			office.furniture,
			office.getCharacters(),
			lastZoom,
			cameraPan.x,
			cameraPan.y,
			{
				selectedAgentId: office.selectedAgentId,
				hoveredAgentId: office.hoveredAgentId,
				hoveredTile: null,
				seats: office.seats,
				characters: office.characters
			},
			undefined,
			office.layout.tileColors,
			office.layout.cols,
			office.layout.rows,
			office.layout.carpetTiles,
			office.layout.areas,
			office.layout.areaTiles,
			false,
			null,
			office.pets
		);
		lastOffset = { x: frame.offsetX, y: frame.offsetY };
		drawCampusBackdrop(context, {
			canvasWidth: canvas.width,
			canvasHeight: canvas.height,
			officeLeft: frame.offsetX,
			officeTop: frame.offsetY,
			officeTiles: plan.layout.tiles,
			officeCols: plan.layout.cols,
			officeRows: plan.layout.rows,
			tilePixelSize: TILE_SIZE * lastZoom,
			now,
			motion: !reducedMotion && documentVisible
		});
		drawFishingActivities(context, now);
		drawZonePlaques(context);
		drawChatBubble(context);
		drawPresence(context, now, bubbleMember);
		drawCompletionCelebrations(context, now);
		const frameMs = performance.now() - frameStartedAt;
		renderedFrameCount += 1;
		longestFrameMs = Math.max(longestFrameMs, frameMs);
		if (frameMs > 32) longFrameCount += 1;
		if (renderedFrameCount % 60 === 0 && shell) {
			const currentPlan = plan;
			const availableAtDesks = members.filter((member) => {
				if (member.presence !== 'available') return false;
				const character = office?.characters.get(member.numericId);
				const seat = character?.seatId ? office?.seats.get(character.seatId) : null;
				return !!(
					character &&
					seat &&
					character.tileCol === seat.seatCol &&
					character.tileRow === seat.seatRow
				);
			}).length;
			const leisure = members.flatMap((member) => {
				if (member.presence !== 'available') return [];
				const character = office?.characters.get(member.numericId);
				const spot = restingSpotFor(member, members, currentPlan);
				return character && spot && isAtRestingSpot(character, spot) ? [spot] : [];
			});
			shell.dataset.renderedFrames = String(renderedFrameCount);
			shell.dataset.longFrames = String(longFrameCount);
			shell.dataset.longestFrameMs = longestFrameMs.toFixed(2);
			shell.dataset.renderZoom = String(lastZoom);
			shell.dataset.worldShape = `${plan.layout.cols}x${plan.layout.rows}`;
			shell.dataset.ambientCount = String(ambientReservations.size);
			shell.dataset.availableAtDesks = String(availableAtDesks);
			shell.dataset.leisureSeated = String(leisure.filter((spot) => spot.posture === 'sit').length);
			shell.dataset.leisurePlaying = String(
				leisure.filter((spot) => spot.activity === 'pool' || spot.activity === 'table-game').length
			);
			shell.dataset.chatBubbles = String(
				chatSpeakerId !== null && simulationClock < chatBubbleUntil ? 1 : 0
			);
			shell.dataset.activityKinds = String(new Set(leisure.map((spot) => spot.activity)).size);
			shell.dataset.activityTones = String(new Set(leisure.map((spot) => spot.tone)).size);
			shell.dataset.campus = 'lakeside-courtyard';
			shell.dataset.environmentMotion = String(CAMPUS_MOTION_CHANNELS.length);
			shell.dataset.wildlife =
				!reducedMotion && documentVisible
					? (campusWildlifeAt(Date.now())?.kind ?? 'none')
					: 'paused';
			shell.dataset.celebrations = String(
				[...completionCelebrations.values()].filter((until) => now < until).length
			);
		}
	}

	function drawFishingActivities(context: CanvasRenderingContext2D, now: number) {
		if (!office || !plan) return;
		let visibleFishers = 0;
		for (const member of members) {
			if (member.presence !== 'available' || ambientReservations.has(member.actorId)) continue;
			const character = office.characters.get(member.numericId);
			const spot = restingSpotFor(member, members, plan);
			if (!character || spot?.activity !== 'fishing' || !isAtRestingSpot(character, spot)) continue;
			visibleFishers += 1;
			const pixel = lastZoom;
			const bob = reducedMotion || !documentVisible ? 0 : Math.floor(now / 620) % 2;
			const anchorX = lastOffset.x + (character.x + 4) * lastZoom;
			const anchorY = lastOffset.y + (character.y - 8) * lastZoom;
			const tipX = anchorX + 12 * pixel;
			const tipY = anchorY - 10 * pixel;
			const floatX = anchorX + 20 * pixel;
			const floatY = anchorY + (8 + bob) * pixel;

			context.save();
			context.imageSmoothingEnabled = false;
			context.fillStyle = '#7b5139';
			for (let step = 0; step < 7; step += 1)
				context.fillRect(
					Math.round(anchorX + step * 2 * pixel),
					Math.round(anchorY - step * 1.5 * pixel),
					pixel,
					2 * pixel
				);
			context.strokeStyle = '#d8e3e7';
			context.lineWidth = Math.max(1, pixel);
			context.beginPath();
			context.moveTo(Math.round(tipX), Math.round(tipY));
			context.lineTo(Math.round(floatX), Math.round(floatY));
			context.stroke();
			context.fillStyle = '#f4efe2';
			context.fillRect(Math.round(floatX - pixel), Math.round(floatY), 3 * pixel, pixel);
			context.fillStyle = '#d76558';
			context.fillRect(Math.round(floatX), Math.round(floatY + pixel), pixel, 2 * pixel);
			context.fillStyle = '#cfeaec';
			context.fillRect(
				Math.round(floatX - 5 * pixel),
				Math.round(floatY + 4 * pixel),
				4 * pixel,
				pixel
			);
			context.fillRect(
				Math.round(floatX + 3 * pixel),
				Math.round(floatY + 4 * pixel),
				5 * pixel,
				pixel
			);
			context.restore();
		}
		if (shell) shell.dataset.fishing = String(visibleFishers);
	}

	function animateRestorativeDecor(now: number) {
		if (!assets || !plan || !office) return;
		const loadedAssets = assets;
		const currentPlan = plan;
		const currentOffice = office;
		const paused = reducedMotion || !documentVisible;
		const animationBudget = Math.min(
			MAX_ANIMATED_ACTIVITY_SCENES,
			currentPlan.animatedAmenities.length
		);
		const activeStart = paused
			? 0
			: Math.floor(now / 8_000) % Math.max(1, currentPlan.animatedAmenities.length);
		currentPlan.animatedAmenities.forEach((amenity, index) => {
			const frames = loadedAssets.amenityFrames[amenity.type];
			if (!frames?.length) return;
			const activeDistance =
				(index - activeStart + currentPlan.animatedAmenities.length) %
				currentPlan.animatedAmenities.length;
			const frameIndex =
				!paused && activeDistance < animationBudget ? Math.floor(now / 720) % frames.length : 0;
			const instance = currentOffice.furniture.find(
				(candidate) =>
					candidate.x === amenity.col * TILE_SIZE && candidate.y === amenity.row * TILE_SIZE
			);
			if (instance) instance.sprite = frames[frameIndex];
		});
		if (shell) shell.dataset.animatedScenes = String(paused ? 0 : animationBudget);
	}

	function drawCompletionCelebrations(context: CanvasRenderingContext2D, now: number) {
		if (!office || reducedMotion) return;
		const colors = ['#efc653', '#e77f76', '#70ad83', '#79a7cc'];
		for (const [numericId, until] of [...completionCelebrations]) {
			if (now >= until) {
				completionCelebrations.delete(numericId);
				continue;
			}
			const character = office.characters.get(numericId);
			if (!character) continue;
			const elapsed = 5_000 - (until - now);
			const beat = Math.floor(elapsed / 180);
			context.save();
			for (let index = 0; index < 8; index += 1) {
				const direction = index % 2 ? -1 : 1;
				const x =
					lastOffset.x +
					character.x * lastZoom +
					direction * (5 + ((index * 5 + beat * 2) % 17)) * devicePixelRatio;
				const y =
					lastOffset.y +
					(character.y - 18) * lastZoom +
					(((index * 7 + beat * 3) % 22) - 11) * devicePixelRatio;
				context.fillStyle = colors[index % colors.length];
				context.fillRect(Math.round(x), Math.round(y), 2 * devicePixelRatio, 2 * devicePixelRatio);
			}
			context.restore();
		}
	}

	function drawZonePlaques(context: CanvasRenderingContext2D) {
		if (!plan) return;
		const scale = devicePixelRatio;
		context.save();
		context.textBaseline = 'middle';
		context.font = `400 ${9 * scale}px Silkscreen, monospace`;
		for (const zone of plan.zones) {
			const x = lastOffset.x + (zone.col + 1) * TILE_SIZE * lastZoom;
			const y = lastOffset.y + (zone.row + 0.58) * TILE_SIZE * lastZoom;
			if (x < -220 * scale || y < -30 * scale || x > canvas.width || y > canvas.height) {
				continue;
			}
			const label = zone.label;
			const width = Math.ceil(context.measureText(label).width) + 10 * scale;
			const height = 16 * scale;
			context.fillStyle = 'rgba(239, 248, 244, 0.92)';
			context.fillRect(Math.round(x), Math.round(y - height / 2), width, height);
			context.fillStyle = zone.kind === 'team' ? '#2b6660' : '#6d5da8';
			context.fillText(label, Math.round(x + 5 * scale), Math.round(y));
		}
		context.restore();
	}

	function drawChatBubble(context: CanvasRenderingContext2D) {
		if (!office || chatSpeakerId === null || reducedMotion || simulationClock >= chatBubbleUntil)
			return;
		const character = office.characters.get(chatSpeakerId);
		const member = memberByNumericId.get(chatSpeakerId);
		if (!character || member?.presence !== 'available') return;
		const pixel = lastZoom;
		const width = 11 * pixel;
		const height = 10 * pixel;
		const x = Math.round(lastOffset.x + character.x * lastZoom - width / 2);
		const y = Math.round(lastOffset.y + (character.y - 24) * lastZoom - height);
		const remaining = chatBubbleUntil - simulationClock;
		context.save();
		context.globalAlpha = remaining < 0.35 ? remaining / 0.35 : 1;
		context.fillStyle = '#4c5f61';
		context.fillRect(x, y, width, height);
		context.fillStyle = '#fff9ea';
		context.fillRect(x + pixel, y + pixel, width - 2 * pixel, height - 2 * pixel);
		context.fillStyle = '#4c5f61';
		context.fillRect(x + 4 * pixel, y + height, 3 * pixel, pixel);
		context.fillRect(x + 5 * pixel, y + height + pixel, pixel, pixel);
		context.fillStyle = '#4c9b91';
		for (const offset of [3, 5, 7])
			context.fillRect(x + offset * pixel, y + 5 * pixel, pixel, pixel);
		context.restore();
	}

	function drawPresence(
		context: CanvasRenderingContext2D,
		now: number,
		bubbleMember: OfficeMember | null
	) {
		if (!office) return;
		for (const character of office.characters.values()) {
			const member = memberByNumericId.get(character.id);
			if (!member) continue;
			const x = lastOffset.x + character.x * lastZoom;
			const y = lastOffset.y + (character.y + 8) * lastZoom;
			if (member.sessionObserved) {
				const pulse = reducedMotion || !documentVisible ? 0.78 : 0.62 + Math.sin(now / 280) * 0.16;
				context.save();
				context.globalAlpha = pulse;
				context.strokeStyle = '#d8f2cf';
				context.lineWidth = Math.max(2, devicePixelRatio);
				context.beginPath();
				context.ellipse(x, y, 7 * lastZoom, 3 * lastZoom, 0, 0, Math.PI * 2);
				context.stroke();
				context.restore();
			} else if (member.presence !== 'available') {
				const cue =
					member.presence === 'waiting'
						? '!'
						: member.presence === 'stale'
							? '~'
							: member.presence === 'unknown'
								? '?'
								: member.presence === 'unavailable'
									? '×'
									: '·';
				context.save();
				context.fillStyle = member.presence === 'waiting' ? '#a56820' : '#526673';
				context.font = `400 ${8 * devicePixelRatio}px Silkscreen, monospace`;
				context.textAlign = 'center';
				context.fillText(cue, Math.round(x), Math.round(y + 3 * lastZoom));
				context.restore();
			}
		}

		const character = bubbleMember ? office.characters.get(bubbleMember.numericId) : null;
		if (character && bubbleMember) {
			drawSpeechBubble(context, character.x, character.y, bubbleMember);
		}
	}

	function drawSpeechBubble(
		context: CanvasRenderingContext2D,
		worldX: number,
		worldY: number,
		member: OfficeMember
	) {
		if (!office) return;
		const scale = devicePixelRatio;
		const name = member.display;
		const detail = member.presenceLabel;
		context.save();
		context.font = `400 ${10 * scale}px Silkscreen, monospace`;
		const width = Math.max(
			1,
			Math.min(
				canvas.width - 20 * scale,
				Math.max(context.measureText(name).width, context.measureText(detail).width) + 20 * scale
			)
		);
		const height = 40 * scale;
		const anchorX = lastOffset.x + worldX * lastZoom;
		const anchorY = lastOffset.y + (worldY - 29) * lastZoom;
		const obstaclePadding = 2 * scale;
		const obstacles: BubbleRect[] = office.furniture.map((item) => ({
			left: lastOffset.x + item.x * lastZoom - obstaclePadding,
			top: lastOffset.y + item.y * lastZoom - obstaclePadding,
			width: (item.sprite[0]?.length ?? 1) * lastZoom + obstaclePadding * 2,
			height: item.sprite.length * lastZoom + obstaclePadding * 2
		}));
		for (const other of office.characters.values()) {
			if (other.id === member.numericId) continue;
			obstacles.push({
				left: lastOffset.x + (other.x - 8) * lastZoom,
				top: lastOffset.y + (other.y - 24) * lastZoom,
				width: 16 * lastZoom,
				height: 24 * lastZoom
			});
		}
		const placement = chooseBubblePlacement({
			canvasWidth: canvas.width,
			canvasHeight: canvas.height,
			anchorX,
			anchorY,
			width,
			height,
			scale,
			obstacles
		});
		const { left, top } = placement;
		if (shell) {
			shell.dataset.bubbleTail = placement.tail;
			shell.dataset.bubbleOverlap = placement.overlapArea.toFixed(0);
		}

		context.shadowColor = 'rgba(23, 36, 51, 0.3)';
		context.shadowOffsetY = 3 * scale;
		context.shadowBlur = 0;
		context.fillStyle = '#fff8e8';
		context.fillRect(left, top, width, height);
		context.shadowColor = 'transparent';
		context.strokeStyle = '#172433';
		context.lineWidth = scale;
		context.strokeRect(left + scale / 2, top + scale / 2, width - scale, height - scale);

		context.fillStyle = '#fff8e8';
		context.strokeStyle = '#172433';
		context.beginPath();
		if (placement.tail === 'bottom') {
			const tailX = Math.max(left + 12 * scale, Math.min(left + width - 12 * scale, anchorX));
			context.fillRect(tailX - 4 * scale, top + height - scale, 8 * scale, 5 * scale);
			context.fillRect(tailX - 2 * scale, top + height + 4 * scale, 4 * scale, 4 * scale);
			context.moveTo(tailX - 4 * scale, top + height);
			context.lineTo(tailX - 4 * scale, top + height + 4 * scale);
			context.lineTo(tailX - 2 * scale, top + height + 4 * scale);
			context.lineTo(tailX - 2 * scale, top + height + 8 * scale);
			context.lineTo(tailX + 2 * scale, top + height + 8 * scale);
			context.lineTo(tailX + 2 * scale, top + height + 4 * scale);
			context.lineTo(tailX + 4 * scale, top + height + 4 * scale);
			context.lineTo(tailX + 4 * scale, top + height);
		} else {
			const tailY = Math.max(top + 12 * scale, Math.min(top + height - 12 * scale, anchorY));
			const direction = placement.tail === 'right' ? 1 : -1;
			const edgeX = placement.tail === 'right' ? left + width : left;
			context.fillRect(
				placement.tail === 'right' ? edgeX - scale : edgeX - 4 * scale,
				tailY - 4 * scale,
				5 * scale,
				8 * scale
			);
			context.fillRect(
				edgeX + direction * 4 * scale - (direction < 0 ? 4 * scale : 0),
				tailY - 2 * scale,
				4 * scale,
				4 * scale
			);
			context.moveTo(edgeX, tailY - 4 * scale);
			context.lineTo(edgeX + direction * 4 * scale, tailY - 4 * scale);
			context.lineTo(edgeX + direction * 4 * scale, tailY - 2 * scale);
			context.lineTo(edgeX + direction * 8 * scale, tailY - 2 * scale);
			context.lineTo(edgeX + direction * 8 * scale, tailY + 2 * scale);
			context.lineTo(edgeX + direction * 4 * scale, tailY + 2 * scale);
			context.lineTo(edgeX + direction * 4 * scale, tailY + 4 * scale);
			context.lineTo(edgeX, tailY + 4 * scale);
		}
		context.stroke();

		context.fillStyle = '#172433';
		context.font = `400 ${10 * scale}px Silkscreen, monospace`;
		context.textBaseline = 'top';
		context.fillText(name, left + 9 * scale, top + 6 * scale, width - 18 * scale);
		context.fillStyle = member.sessionObserved
			? '#2f7752'
			: member.presence === 'waiting'
				? '#a56820'
				: '#526673';
		context.font = `400 ${8 * scale}px Silkscreen, monospace`;
		context.fillText(detail, left + 9 * scale, top + 23 * scale, width - 18 * scale);
		context.restore();
	}

	function eventPoint(event: { clientX: number; clientY: number }) {
		const rectangle = canvas.getBoundingClientRect();
		return {
			x: (event.clientX - rectangle.left) * devicePixelRatio,
			y: (event.clientY - rectangle.top) * devicePixelRatio
		};
	}

	function eventWorld(event: { clientX: number; clientY: number }): {
		x: number;
		y: number;
	} {
		const point = eventPoint(event);
		return {
			x: (point.x - lastOffset.x) / lastZoom,
			y: (point.y - lastOffset.y) / lastZoom
		};
	}

	function handlePointerDown(event: PointerEvent) {
		if (!office) return;
		canvas.focus({ preventScroll: true });
		canvas.setPointerCapture(event.pointerId);
		pointer = {
			id: event.pointerId,
			startX: event.clientX,
			startY: event.clientY,
			lastX: event.clientX,
			lastY: event.clientY,
			moved: false
		};
	}

	function handlePointerMove(event: PointerEvent) {
		if (!office) return;
		if (pointer?.id === event.pointerId) {
			const dx = event.clientX - pointer.lastX;
			const dy = event.clientY - pointer.lastY;
			if (
				pointer.moved ||
				Math.hypot(event.clientX - pointer.startX, event.clientY - pointer.startY) > 3
			) {
				pointer.moved = true;
				cameraPan = {
					x: cameraPan.x + dx * devicePixelRatio,
					y: cameraPan.y + dy * devicePixelRatio
				};
				hoveredActorId = null;
				office.hoveredAgentId = null;
			}
			pointer.lastX = event.clientX;
			pointer.lastY = event.clientY;
			return;
		}
		const world = eventWorld(event);
		const numericId = office.getCharacterAt(world.x, world.y);
		const member = numericId == null ? null : memberByNumericId.get(numericId);
		hoveredActorId = member?.actorId ?? null;
		office.hoveredAgentId = member?.numericId ?? null;
	}

	function handlePointerUp(event: PointerEvent) {
		if (!office || pointer?.id !== event.pointerId) return;
		const moved = pointer.moved;
		pointer = null;
		canvas.releasePointerCapture(event.pointerId);
		if (moved) return;
		const world = eventWorld(event);
		if (decorating) {
			editAt(Math.floor(world.x / TILE_SIZE), Math.floor(world.y / TILE_SIZE));
			return;
		}
		const numericId = office.getCharacterAt(world.x, world.y);
		const member = numericId == null ? null : memberByNumericId.get(numericId);
		if (member) {
			selectedActorId = member.actorId;
			return;
		}
		const petId = office.getPetAt(world.x, world.y);
		if (petId) office.showPetBubble(petId);
		else selectedActorId = null;
	}

	function handlePointerCancel(event: PointerEvent) {
		if (pointer?.id !== event.pointerId) return;
		pointer = null;
	}

	function handlePointerLeave() {
		if (pointer) return;
		hoveredActorId = null;
		if (office) office.hoveredAgentId = null;
	}

	function handleWheel(event: WheelEvent) {
		if (!office || !plan) return;
		event.preventDefault();
		const next = clampZoom(zoomCss + (event.deltaY < 0 ? ZOOM_STEP : -ZOOM_STEP));
		if (Math.abs(next - zoomCss) < ZOOM_EPSILON) return;
		const point = eventPoint(event);
		const oldZoom = lastZoom;
		const worldX = (point.x - lastOffset.x) / oldZoom;
		const worldY = (point.y - lastOffset.y) / oldZoom;
		const nextZoom = Math.max(1, Math.round(next * devicePixelRatio));
		const centered = mapOffset(
			canvas.width,
			canvas.height,
			plan.layout.cols,
			plan.layout.rows,
			nextZoom,
			0,
			0
		);
		zoomCss = next;
		cameraPan = {
			x: point.x - worldX * nextZoom - centered.offsetX,
			y: point.y - worldY * nextZoom - centered.offsetY
		};
	}

	function changeZoom(delta: number) {
		if (!plan) return;
		const next = clampZoom(zoomCss + delta);
		if (Math.abs(next - zoomCss) < ZOOM_EPSILON) return;
		const center = {
			clientX: canvas.getBoundingClientRect().left + canvas.clientWidth / 2,
			clientY: canvas.getBoundingClientRect().top + canvas.clientHeight / 2
		};
		handleWheel({
			...center,
			deltaY: delta > 0 ? -1 : 1,
			preventDefault: () => {}
		} as WheelEvent);
	}

	function fittedZoom(): number {
		if (!shell || !plan) return 1;
		const campusMargin = plan.layout.cols >= 60 ? 40 : 88;
		const fit = Math.min(
			(shell.clientWidth - campusMargin) / (plan.layout.cols * TILE_SIZE),
			(shell.clientHeight - campusMargin) / (plan.layout.rows * TILE_SIZE)
		);
		return Math.max(0.5, Math.min(2.5, fit));
	}

	function updateZoomBounds(clampCurrent = true) {
		const fit = fittedZoom();
		minZoomCss = fit;
		maxZoomCss = Math.max(fit, Math.min(MAX_CLOSE_ZOOM, fit * 2));
		if (clampCurrent) zoomCss = clampZoom(zoomCss);
	}

	function clampZoom(value: number): number {
		return Math.max(minZoomCss, Math.min(maxZoomCss, value));
	}

	function homeCamera() {
		cameraPan = { x: 0, y: 0 };
		if (!shell || !plan) {
			zoomCss = 1;
			return;
		}
		updateZoomBounds(false);
		zoomCss = minZoomCss;
	}

	function editAt(col: number, row: number) {
		if (!plan) return;
		if (selectedDecoration === 'erase') {
			const decoration = preferences.decorations.find((candidate) => {
				const entry = getCatalogEntry(candidate.type);
				return (
					entry &&
					col >= candidate.col &&
					col < candidate.col + entry.footprintW &&
					row >= candidate.row &&
					row < candidate.row + entry.footprintH
				);
			});
			if (!decoration) {
				decorationMessage = 'Choose a decoration you placed.';
				return;
			}
			updatePreferences({
				decorations: preferences.decorations.filter((candidate) => candidate.uid !== decoration.uid)
			});
			decorationMessage = 'Decoration removed.';
			return;
		}
		if (
			!isDecorationPlacementValid(plan.layout, selectedDecoration, col, row, {
				home: plan.home,
				requiredPoints: [
					...plan.interactionPoints,
					...plan.restingSpots,
					...plan.waitingSpots,
					...plan.protectedPath
				]
			})
		) {
			decorationMessage = 'That space needs to stay clear for people.';
			return;
		}
		updatePreferences({
			decorations: [
				...preferences.decorations,
				{
					uid: `owner-${Date.now()}-${preferences.decorations.length}`,
					type: selectedDecoration,
					col,
					row
				}
			]
		});
		decorationMessage = 'Decoration placed.';
	}

	function updatePreferences(patch: Partial<OfficePreferences>) {
		onpreferenceschange?.({ ...preferences, ...patch });
	}

	function undoDecoration() {
		if (!preferences.decorations.length) return;
		updatePreferences({ decorations: preferences.decorations.slice(0, -1) });
		decorationMessage = 'Last decoration removed.';
	}

	function selectOffset(offset: number) {
		if (!members.length) return;
		if (!selectedActorId) {
			selectedActorId = offset < 0 ? members.at(-1)!.actorId : members[0].actorId;
			return;
		}
		const index = Math.max(
			0,
			members.findIndex((member) => member.actorId === selectedActorId)
		);
		selectedActorId = members[(index + offset + members.length) % members.length].actorId;
	}

	function observationLabel(value: string | null): string | null {
		if (!value) return null;
		const date = new Date(value);
		if (Number.isNaN(date.getTime())) return null;
		return `Observed ${date.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })}`;
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			if (selectedActorId) selectedActorId = null;
			else decorating = false;
		} else if (event.key === '+' || event.key === '=') {
			event.preventDefault();
			changeZoom(ZOOM_STEP);
		} else if (event.key === '-') {
			event.preventDefault();
			changeZoom(-ZOOM_STEP);
		} else if (event.key === '0') {
			event.preventDefault();
			homeCamera();
		} else if (
			event.shiftKey &&
			['ArrowRight', 'ArrowDown', 'ArrowLeft', 'ArrowUp'].includes(event.key)
		) {
			event.preventDefault();
			const step = 80 * devicePixelRatio;
			cameraPan = {
				x:
					cameraPan.x + (event.key === 'ArrowRight' ? -step : event.key === 'ArrowLeft' ? step : 0),
				y: cameraPan.y + (event.key === 'ArrowDown' ? -step : event.key === 'ArrowUp' ? step : 0)
			};
		} else if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
			event.preventDefault();
			selectOffset(1);
		} else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
			event.preventDefault();
			selectOffset(-1);
		} else if (event.key === 'Home' && members.length) {
			event.preventDefault();
			selectedActorId = members[0].actorId;
		} else if (event.key === 'Enter' && selectedMember) {
			event.preventDefault();
			onopen?.(selectedMember);
		}
	}
</script>

<div
	class="office-canvas-shell"
	class:interactive={hoveredActorId}
	class:panning={pointer?.moved}
	class:decorating
	bind:this={shell}
	data-office-ready={ready ? 'true' : 'false'}
	data-member-count={members.length}
	data-selected-actor={selectedActorId ?? ''}
	data-presences={members.map((member) => `${member.actorId}:${member.presence}`).join(',')}
	data-motion={reducedMotion ? 'reduced' : 'full'}
	data-document-visible={documentVisible ? 'true' : 'false'}
	data-zoom={zoomCss.toFixed(3)}
	data-zoom-min={minZoomCss.toFixed(3)}
	data-zoom-max={maxZoomCss.toFixed(3)}
>
	<canvas
		bind:this={canvas}
		tabindex="0"
		aria-label="Explorable company office. Drag to move, scroll to zoom, use arrow keys to choose a colleague, Shift plus arrow keys to move the camera, and Enter to inspect Work."
		onpointerdown={handlePointerDown}
		onpointermove={handlePointerMove}
		onpointerup={handlePointerUp}
		onpointercancel={handlePointerCancel}
		onpointerleave={handlePointerLeave}
		onkeydown={handleKeydown}
	></canvas>

	{#if selectedMember}
		<aside
			class="person-detail"
			aria-label={`${selectedMember.display} office detail`}
			data-presence={selectedMember.presence}
		>
			<button
				type="button"
				class="person-detail-close"
				aria-label="Close person detail"
				onclick={() => (selectedActorId = null)}>×</button
			>
			<div class="person-detail-heading">
				<strong>{selectedMember.display}</strong>
				<span>{selectedMember.teamName ?? selectedMember.role}</span>
			</div>
			<div class="person-detail-state">
				<i class={`presence-${selectedMember.presence}`}></i>
				<span>{selectedMember.presenceLabel}</span>
				{#if observationLabel(selectedMember.presenceObservedAt)}
					<time datetime={selectedMember.presenceObservedAt ?? undefined}
						>{observationLabel(selectedMember.presenceObservedAt)}</time
					>
				{/if}
			</div>
			{#if selectedMember.work}
				<p class="person-detail-outcome">{selectedMember.work.title}</p>
			{/if}
			{#if selectedMember.currentStep}
				<p class="person-detail-step">{selectedMember.currentStep}</p>
			{:else if !selectedMember.work}
				<p class="person-detail-step">{selectedMember.activityDetail}</p>
			{/if}
			{#if onopen}
				<button type="button" class="person-detail-action" onclick={() => onopen(selectedMember)}>
					{selectedMember.workHref ? 'Open Work' : 'Open person'} <span>→</span>
				</button>
			{/if}
		</aside>
	{/if}

	<div class="office-camera-controls" aria-label="Office camera">
		<button
			type="button"
			title={canZoomOut ? 'Zoom out' : 'The whole office is already in view'}
			aria-label={canZoomOut ? 'Zoom out' : 'Zoom out unavailable; the whole office is in view'}
			disabled={!canZoomOut}
			onclick={() => changeZoom(-ZOOM_STEP)}
		>
			<Minus size={14} strokeWidth={2.25} />
		</button>
		<button
			type="button"
			title="Centre the office"
			aria-label="Centre the office"
			onclick={homeCamera}
		>
			<Focus size={14} strokeWidth={2.25} />
		</button>
		<button
			type="button"
			title={canZoomIn ? 'Zoom in' : 'Maximum detail reached'}
			aria-label={canZoomIn ? 'Zoom in' : 'Zoom in unavailable; maximum detail reached'}
			disabled={!canZoomIn}
			onclick={() => changeZoom(ZOOM_STEP)}
		>
			<Plus size={14} strokeWidth={2.25} />
		</button>
	</div>

	<div class="office-decorate">
		{#if decorating}
			<div class="decor-tray" aria-label="Decorate the office">
				<div class="decor-section decor-props" aria-label="Furniture">
					{#each decorationOptions as decoration (decoration.type)}
						<button
							type="button"
							class:active={selectedDecoration === decoration.type}
							title={decoration.label}
							aria-label={decoration.label}
							aria-pressed={selectedDecoration === decoration.type}
							onclick={() => (selectedDecoration = decoration.type)}
							><img src={decoration.src} alt="" /></button
						>
					{/each}
					<button
						type="button"
						class:active={selectedDecoration === 'erase'}
						title="Remove a decoration"
						aria-label="Remove a decoration"
						aria-pressed={selectedDecoration === 'erase'}
						onclick={() => (selectedDecoration = 'erase')}><Eraser size={15} /></button
					>
				</div>
				<div class="decor-rule"></div>
				<div class="decor-section">
					<button
						type="button"
						class:active={preferences.decorDensity === 'lush'}
						title="More plants around the campus"
						aria-label="More plants around the campus"
						aria-pressed={preferences.decorDensity === 'lush'}
						onclick={() =>
							updatePreferences({
								decorDensity: preferences.decorDensity === 'lush' ? 'calm' : 'lush'
							})}><Sparkles size={15} /></button
					>
					<button
						type="button"
						class:active={preferences.pets}
						title="Office pets"
						aria-label="Office pets"
						aria-pressed={preferences.pets}
						onclick={() => updatePreferences({ pets: !preferences.pets })}
						><PawPrint size={15} /></button
					>
					<button
						type="button"
						title="Undo the last decoration"
						aria-label="Undo the last decoration"
						disabled={!preferences.decorations.length}
						onclick={undoDecoration}><Undo2 size={15} /></button
					>
					<button
						type="button"
						class="done"
						title="Finish decorating"
						aria-label="Finish decorating"
						onclick={() => (decorating = false)}><Check size={15} /></button
					>
				</div>
			</div>
		{:else}
			<button
				type="button"
				class="decorate-trigger"
				title="Decorate the office"
				aria-label="Decorate the office"
				onclick={() => (decorating = true)}><Paintbrush size={15} strokeWidth={2.2} /></button
			>
		{/if}
		<span class="decor-message" aria-live="polite">{decorationMessage}</span>
	</div>

	{#if !ready && !error}
		<div class="office-loading" role="status">
			<span></span>
			Opening the company floor…
		</div>
	{:else if error}
		<div class="office-error" role="status">
			<strong>The company floor is unavailable.</strong>
			<span>{error}</span>
		</div>
	{/if}
</div>

<style>
	.office-canvas-shell {
		position: relative;
		height: 100%;
		min-height: 0;
		overflow: hidden;
		isolation: isolate;
		background: #94c78a;
		border: 0;
		border-radius: 0;
	}

	canvas {
		display: block;
		width: 100%;
		height: 100%;
		min-height: 0;
		image-rendering: pixelated;
		outline: none;
		cursor: grab;
		touch-action: none;
		user-select: none;
	}

	canvas:focus-visible {
		box-shadow: inset 0 0 0 3px rgba(118, 197, 209, 0.72);
	}

	.panning canvas {
		cursor: grabbing;
	}

	.interactive:not(.panning) canvas {
		cursor: pointer;
	}

	.decorating:not(.panning) canvas {
		cursor: crosshair;
	}

	.person-detail {
		position: absolute;
		right: 12px;
		bottom: 58px;
		z-index: 4;
		display: grid;
		width: min(292px, calc(100% - 24px));
		gap: 9px;
		padding: 15px;
		border: 1px solid rgba(23, 36, 51, 0.76);
		background: rgba(239, 248, 244, 0.96);
		box-shadow: 0 4px 0 rgba(23, 36, 51, 0.28);
		color: #172433;
		backdrop-filter: blur(6px);
	}

	.person-detail-close {
		position: absolute;
		top: 5px;
		right: 6px;
		width: 28px;
		height: 28px;
		padding: 0;
		border: 0;
		background: transparent;
		color: #526673;
		font: 600 var(--t-head)/1 var(--font-sans);
		cursor: pointer;
	}

	.person-detail-heading {
		display: grid;
		gap: 2px;
		padding-right: 24px;
	}

	.person-detail-heading strong {
		font: 700 var(--t-head)/1.2 var(--font-sans);
	}

	.person-detail-heading span,
	.person-detail-state time {
		color: #69747a;
		font: 500 var(--t-label)/1.35 var(--font-mono);
	}

	.person-detail-state {
		display: flex;
		align-items: center;
		gap: 7px;
		font: 600 var(--t-label)/1.2 var(--font-mono);
	}

	.person-detail-state i {
		width: 8px;
		height: 8px;
		background: #748790;
	}

	.person-detail-state i.presence-observed {
		border-radius: 50%;
		background: #4a9b66;
		box-shadow: 0 0 0 3px rgba(74, 155, 102, 0.17);
	}

	.person-detail-state i.presence-waiting {
		background: #d99a35;
		transform: rotate(45deg);
	}

	.person-detail-state time {
		margin-left: auto;
	}

	.person-detail-outcome,
	.person-detail-step {
		margin: 0;
	}

	.person-detail-outcome {
		font: 650 var(--t-body)/1.35 var(--font-sans);
	}

	.person-detail-step {
		color: #53636a;
		font: 500 var(--t-body)/1.4 var(--font-sans);
	}

	.person-detail-action {
		display: flex;
		align-items: center;
		justify-content: space-between;
		min-height: 34px;
		padding: 7px 10px;
		border: 1px solid rgba(23, 36, 51, 0.5);
		background: #173d3b;
		color: #eff8f4;
		font: 650 var(--t-body)/1 var(--font-sans);
		cursor: pointer;
	}

	.person-detail button:focus-visible {
		outline: 2px solid #4fa6a0;
		outline-offset: 2px;
	}

	.office-camera-controls,
	.decorate-trigger,
	.decor-tray {
		border: 1px solid rgba(23, 36, 51, 0.76);
		background: rgba(239, 248, 244, 0.94);
		box-shadow:
			0 3px 0 rgba(23, 36, 51, 0.34),
			inset 0 1px rgba(255, 255, 255, 0.8);
		backdrop-filter: blur(5px);
	}

	.office-camera-controls {
		position: absolute;
		right: 12px;
		bottom: 12px;
		z-index: 3;
		display: flex;
	}

	.office-camera-controls button,
	.decor-tray button,
	.decorate-trigger {
		width: 34px;
		height: 32px;
		display: grid;
		place-items: center;
		padding: 0;
		border: 0;
		border-right: 1px solid rgba(23, 36, 51, 0.2);
		background: transparent;
		color: #172433;
		cursor: pointer;
	}

	.office-camera-controls button:last-child {
		border-right: 0;
	}

	.office-camera-controls button:not(:disabled):hover,
	.decor-tray button:hover,
	.decorate-trigger:hover,
	.decor-tray button.active {
		background: rgba(79, 166, 160, 0.18);
	}

	.office-camera-controls button:focus-visible,
	.decor-tray button:focus-visible,
	.decorate-trigger:focus-visible {
		position: relative;
		z-index: 1;
		outline: 2px solid #4fa6a0;
		outline-offset: -3px;
	}

	.office-decorate {
		position: absolute;
		left: 12px;
		bottom: 12px;
		z-index: 3;
		display: flex;
		align-items: flex-end;
		gap: 8px;
	}

	.decorate-trigger {
		border-right: 1px solid rgba(23, 36, 51, 0.76);
	}

	.decor-tray {
		display: flex;
		align-items: center;
		min-height: 34px;
		padding: 3px;
	}

	.decor-section {
		display: flex;
		align-items: center;
	}

	.decor-rule {
		width: 1px;
		height: 24px;
		margin: 0 3px;
		background: rgba(23, 36, 51, 0.18);
	}

	.decor-tray button {
		width: 31px;
		height: 29px;
		border-right: 0;
	}

	.decor-tray button:disabled {
		opacity: 0.32;
		cursor: default;
	}

	.office-camera-controls button:disabled {
		opacity: 0.38;
		cursor: default;
	}

	.decor-tray button.done {
		color: #2f7752;
	}

	.decor-tray img {
		width: 24px;
		height: 24px;
		object-fit: contain;
		image-rendering: pixelated;
	}

	.decor-message {
		max-width: 180px;
		padding: 4px 6px;
		color: rgba(239, 248, 244, 0.9);
		font:
			400 var(--t-label)/1.35 Silkscreen,
			var(--font-mono);
		text-shadow: 1px 1px #172433;
	}

	.office-loading,
	.office-error {
		position: absolute;
		inset: 0;
		z-index: 4;
		display: grid;
		place-content: center;
		justify-items: center;
		gap: var(--space-3);
		padding: var(--space-6);
		text-align: center;
		color: #172433;
		background: rgba(223, 241, 235, 0.96);
	}

	.office-loading span {
		width: 26px;
		height: 26px;
		border: 4px solid rgba(23, 36, 51, 0.14);
		border-top-color: #63b879;
		border-radius: 50%;
		animation: office-turn var(--motion-working) linear infinite;
	}

	.office-error span {
		max-width: 36ch;
		color: rgba(23, 36, 51, 0.68);
	}

	@keyframes office-turn {
		to {
			transform: rotate(360deg);
		}
	}

	@media (max-width: 760px) {
		.office-decorate {
			max-width: calc(100% - 72px);
		}
		.decorating .office-camera-controls {
			bottom: 56px;
		}
		.decor-tray {
			max-width: 100%;
			overflow-x: auto;
		}
		.decor-message {
			display: none;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.office-loading span {
			animation: none;
		}
	}
</style>
