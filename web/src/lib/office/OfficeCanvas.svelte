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
	import { TILE_SIZE } from '$lib/vendor/pixel-agents/webview-ui/src/office/types.js';
	import {
		createCompanyOfficePlan,
		isDecorationPlacementValid,
		type DecorationType,
		type OfficePlan,
		type OfficePreferences,
		type OfficeTheme
	} from './officePlan';
	import type { OfficeMember } from './projection';
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
	const themes: Array<{ key: OfficeTheme; label: string; color: string }> = [
		{ key: 'daylight', label: 'Sunlit company floor', color: '#b8d6d1' },
		{ key: 'garden', label: 'Garden office', color: '#86b99e' },
		{ key: 'midnight', label: 'Midnight field office', color: '#456f91' }
	];

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
	let documentVisible = true;
	let reducedMotion = false;
	let devicePixelRatio = 2;
	let zoomCss = 2.5;
	let cameraPan = { x: 0, y: 0 };
	let lastOffset = { x: 0, y: 0 };
	let lastZoom = 5;
	let lastPresence = new Map<string, OfficeMember['presence']>();
	let restorativeTimer = 0;
	let restorativeCursor = 0;
	let planSignature = '';
	let worldShape = '';
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

	$effect(() => {
		if (!assets) return;
		rebuildOffice(teams, members, preferences);
	});

	onMount(() => {
		let destroyed = false;
		let stopLoop: (() => void) | undefined;
		const media = window.matchMedia('(prefers-reduced-motion: reduce)');
		const readMotion = () => (reducedMotion = media.matches);
		const readVisibility = () => (documentVisible = document.visibilityState === 'visible');
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
			office.setAreaMappings(nextPlan.areaMappings);
			office.rebuildFromLayout(nextPlan.layout);
		} else {
			office = new OfficeState(nextPlan.layout);
			office.setAreaMappings(nextPlan.areaMappings);
		}
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
		restorativeTimer += delta;
		if (restorativeTimer < 14) return;
		restorativeTimer = 0;

		const availableMembers = members.filter((member) => {
			if (member.presence !== 'available') return false;
			const character = office?.characters.get(member.numericId);
			return character && character.path.length === 0;
		});
		if (!availableMembers.length || !plan.restorativeSpots.length) return;

		const member = availableMembers[restorativeCursor % availableMembers.length];
		const spot = plan.restorativeSpots[restorativeCursor % plan.restorativeSpots.length];
		if (office.walkToTile(member.numericId, spot.col, spot.row)) restorativeCursor += 1;
	}

	function sizeCanvas() {
		if (!canvas || !shell) return;
		const rectangle = shell.getBoundingClientRect();
		devicePixelRatio = Math.max(2, Math.min(window.devicePixelRatio || 1, 2));
		canvas.width = Math.max(1, Math.round(rectangle.width * devicePixelRatio));
		canvas.height = Math.max(1, Math.round(rectangle.height * devicePixelRatio));
		lastZoom = Math.max(devicePixelRatio, Math.round(zoomCss * devicePixelRatio));
	}

	function synchronizeMembers(
		state: OfficeState,
		nextMembers: OfficeMember[],
		nextPlan: OfficePlan
	) {
		state.setAreaMappings(nextPlan.areaMappings);
		const nextIds = new Set(nextMembers.map((member) => member.numericId));
		for (const character of state.characters.values()) {
			if (!nextIds.has(character.id)) state.removeAgent(character.id);
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
			state.setAgentActive(member.numericId, member.sessionObserved);
			state.setAgentTool(member.numericId, null);
			if (member.presence === 'waiting') state.showPermissionBubble(member.numericId);
			else state.clearPermissionBubble(member.numericId);

			const previous = lastPresence.get(member.actorId);
			if (previous === 'observed' && member.presence !== 'observed') {
				state.showWaitingBubble(member.numericId);
			}
			if (member.presence === 'waiting' && previous !== 'waiting') {
				const waitingSpot =
					nextPlan.waitingSpots[
						Math.abs(member.numericId) % Math.max(1, nextPlan.waitingSpots.length)
					];
				if (waitingSpot) state.walkToTile(member.numericId, waitingSpot.col, waitingSpot.row);
			} else if (
				previous === 'waiting' &&
				(member.sessionObserved || member.presence === 'in-motion')
			) {
				state.sendToSeat(member.numericId);
			}
			lastPresence.set(member.actorId, member.presence);
		}
		for (const actorId of [...lastPresence.keys()]) {
			if (!nextMembers.some((member) => member.actorId === actorId)) {
				lastPresence.delete(actorId);
			}
		}

		if (!selectedActorId && nextMembers.length) {
			selectedActorId =
				nextMembers.find((member) => member.sessionObserved)?.actorId ?? nextMembers[0].actorId;
		}
	}

	function render(context: CanvasRenderingContext2D, now: number) {
		if (!office || !plan || !canvas.width || !canvas.height) return;
		const hoveredMember = hoveredActorId
			? (members.find((member) => member.actorId === hoveredActorId) ?? null)
			: null;
		const bubbleMember =
			hoveredMember ?? (document.activeElement === canvas ? selectedMember : null);
		office.selectedAgentId = bubbleMember?.numericId ?? null;
		context.imageSmoothingEnabled = false;
		lastZoom = Math.max(devicePixelRatio, Math.round(zoomCss * devicePixelRatio));
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
		drawZonePlaques(context);
		drawPresence(context, now, bubbleMember);
	}

	function animateRestorativeDecor(now: number) {
		if (!assets || !plan || !office) return;
		const fountainFrame = reducedMotion ? 0 : Math.floor(now / 520) % assets.fountainFrames.length;
		const fountain = office.furniture.find(
			(instance) =>
				instance.x === plan!.fountain.col * TILE_SIZE &&
				instance.y === plan!.fountain.row * TILE_SIZE &&
				instance.sprite.length === TILE_SIZE * 4
		);
		if (fountain) fountain.sprite = assets.fountainFrames[fountainFrame];

		const unicornFrame = reducedMotion ? 0 : Math.floor(now / 680) % assets.unicornFrames.length;
		const unicorn = office.furniture.find(
			(instance) =>
				instance.x === plan!.unicorn.col * TILE_SIZE &&
				instance.y === plan!.unicorn.row * TILE_SIZE &&
				instance.sprite.length === TILE_SIZE * 2
		);
		if (unicorn) unicorn.sprite = assets.unicornFrames[unicornFrame];
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
			context.fillStyle = 'rgba(255, 248, 232, 0.9)';
			context.fillRect(Math.round(x), Math.round(y - height / 2), width, height);
			context.fillStyle = zone.kind === 'team' ? '#2b6660' : '#765d35';
			context.fillText(label, Math.round(x + 5 * scale), Math.round(y));
		}
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
				const pulse = reducedMotion ? 0.78 : 0.62 + Math.sin(now / 280) * 0.16;
				context.save();
				context.globalAlpha = pulse;
				context.strokeStyle = '#d8f2cf';
				context.lineWidth = Math.max(2, devicePixelRatio);
				context.beginPath();
				context.ellipse(x, y, 7 * lastZoom, 3 * lastZoom, 0, 0, Math.PI * 2);
				context.stroke();
				context.restore();
			} else if (member.presence === 'in-motion') {
				context.fillStyle = '#78b8c3';
				context.fillRect(
					Math.round(x - 2 * devicePixelRatio),
					Math.round(y + 2 * lastZoom),
					4 * devicePixelRatio,
					2 * devicePixelRatio
				);
			} else if (member.presence === 'waiting') {
				context.fillStyle = '#e0a43b';
				context.fillRect(
					Math.round(x - 2 * devicePixelRatio),
					Math.round(y + 2 * lastZoom),
					4 * devicePixelRatio,
					2 * devicePixelRatio
				);
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
		const scale = devicePixelRatio;
		const name = member.display;
		const detail = member.presenceLabel;
		context.save();
		context.font = `400 ${10 * scale}px Silkscreen, monospace`;
		const width =
			Math.min(
				canvas.width - 20 * scale,
				Math.max(context.measureText(name).width, context.measureText(detail).width) + 20 * scale
			) || 1;
		const height = 40 * scale;
		const anchorX = lastOffset.x + worldX * lastZoom;
		const anchorY = lastOffset.y + (worldY - 29) * lastZoom;
		const left = Math.max(
			8 * scale,
			Math.min(canvas.width - width - 8 * scale, anchorX - width / 2)
		);
		const top = Math.max(8 * scale, anchorY - height);

		context.shadowColor = 'rgba(23, 36, 51, 0.3)';
		context.shadowOffsetY = 3 * scale;
		context.shadowBlur = 0;
		context.fillStyle = '#fff8e8';
		context.fillRect(left, top, width, height);
		context.shadowColor = 'transparent';
		context.strokeStyle = '#172433';
		context.lineWidth = scale;
		context.strokeRect(left + scale / 2, top + scale / 2, width - scale, height - scale);

		const tailX = Math.max(left + 12 * scale, Math.min(left + width - 12 * scale, anchorX));
		context.fillStyle = '#fff8e8';
		context.fillRect(tailX - 4 * scale, top + height - scale, 8 * scale, 5 * scale);
		context.fillRect(tailX - 2 * scale, top + height + 4 * scale, 4 * scale, 4 * scale);
		context.strokeStyle = '#172433';
		context.beginPath();
		context.moveTo(tailX - 4 * scale, top + height);
		context.lineTo(tailX - 4 * scale, top + height + 4 * scale);
		context.lineTo(tailX - 2 * scale, top + height + 4 * scale);
		context.lineTo(tailX - 2 * scale, top + height + 8 * scale);
		context.lineTo(tailX + 2 * scale, top + height + 8 * scale);
		context.lineTo(tailX + 2 * scale, top + height + 4 * scale);
		context.lineTo(tailX + 4 * scale, top + height + 4 * scale);
		context.lineTo(tailX + 4 * scale, top + height);
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
		if (member) selectedActorId = member.actorId;
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
			onopen?.(member);
			return;
		}
		const petId = office.getPetAt(world.x, world.y);
		if (petId) office.showPetBubble(petId);
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
		const next = Math.max(1.25, Math.min(5, zoomCss + (event.deltaY < 0 ? 0.25 : -0.25)));
		if (next === zoomCss) return;
		const point = eventPoint(event);
		const oldZoom = lastZoom;
		const worldX = (point.x - lastOffset.x) / oldZoom;
		const worldY = (point.y - lastOffset.y) / oldZoom;
		const nextZoom = Math.max(devicePixelRatio, Math.round(next * devicePixelRatio));
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
		const next = Math.max(1.25, Math.min(5, zoomCss + delta));
		if (next === zoomCss) return;
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

	function homeCamera() {
		cameraPan = { x: 0, y: 0 };
		zoomCss = shell?.clientWidth && shell.clientWidth < 700 ? 2 : 2.5;
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
		if (!isDecorationPlacementValid(plan.layout, selectedDecoration, col, row)) {
			decorationMessage = 'That space needs a clear floor.';
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
		const index = Math.max(
			0,
			members.findIndex((member) => member.actorId === selectedActorId)
		);
		selectedActorId = members[(index + offset + members.length) % members.length].actorId;
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape' && decorating) {
			event.preventDefault();
			decorating = false;
		} else if (event.key === '+' || event.key === '=') {
			event.preventDefault();
			changeZoom(0.25);
		} else if (event.key === '-') {
			event.preventDefault();
			changeZoom(-0.25);
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

	<div class="office-camera-controls" aria-label="Office camera">
		<button type="button" title="Zoom out" aria-label="Zoom out" onclick={() => changeZoom(-0.25)}>
			<Minus size={14} strokeWidth={2.25} />
		</button>
		<button
			type="button"
			title="Return to the fountain"
			aria-label="Return to the fountain"
			onclick={homeCamera}
		>
			<Focus size={14} strokeWidth={2.25} />
		</button>
		<button type="button" title="Zoom in" aria-label="Zoom in" onclick={() => changeZoom(0.25)}>
			<Plus size={14} strokeWidth={2.25} />
		</button>
	</div>

	<div class="office-decorate">
		{#if decorating}
			<div class="decor-tray" aria-label="Decorate the office">
				<div class="decor-section decor-themes" aria-label="Office atmosphere">
					{#each themes as theme (theme.key)}
						<button
							type="button"
							class:active={preferences.theme === theme.key}
							title={theme.label}
							aria-label={theme.label}
							aria-pressed={preferences.theme === theme.key}
							style:--theme-color={theme.color}
							onclick={() => updatePreferences({ theme: theme.key })}><span></span></button
						>
					{/each}
				</div>
				<div class="decor-rule"></div>
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
		background:
			linear-gradient(rgba(79, 137, 132, 0.13) 1px, transparent 1px),
			linear-gradient(90deg, rgba(79, 137, 132, 0.13) 1px, transparent 1px), #dcebea;
		background-size: 32px 32px;
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

	.office-camera-controls,
	.decorate-trigger,
	.decor-tray {
		border: 1px solid rgba(23, 36, 51, 0.76);
		background: rgba(255, 248, 232, 0.94);
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

	.office-camera-controls button:hover,
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

	.decor-tray button.done {
		color: #2f7752;
	}

	.decor-tray img {
		width: 24px;
		height: 24px;
		object-fit: contain;
		image-rendering: pixelated;
	}

	.decor-themes button span {
		width: 15px;
		height: 15px;
		border: 2px solid #fff8e8;
		outline: 1px solid rgba(23, 36, 51, 0.42);
		background: var(--theme-color);
	}

	.decor-themes button.active span {
		outline: 2px solid #172433;
	}

	.decor-message {
		max-width: 180px;
		padding: 4px 6px;
		color: rgba(255, 248, 232, 0.86);
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
		background:
			linear-gradient(rgba(79, 137, 132, 0.13) 1px, transparent 1px),
			linear-gradient(90deg, rgba(79, 137, 132, 0.13) 1px, transparent 1px),
			rgba(220, 235, 234, 0.96);
		background-size: 32px 32px;
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
