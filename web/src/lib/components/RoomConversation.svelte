<script lang="ts">
	import FailureNotice from '$lib/primitives/FailureNotice.svelte';
	import { failureSentence } from '$lib/model/failure';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	import { onDestroy, tick, untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import Search from '@lucide/svelte/icons/search';
	import Users from '@lucide/svelte/icons/users';
	import Wifi from '@lucide/svelte/icons/wifi';
	import WifiOff from '@lucide/svelte/icons/wifi-off';
	import X from '@lucide/svelte/icons/x';
	import { initials } from '$lib/model/initials';
	import HandoffReceipt from '$lib/primitives/HandoffReceipt.svelte';
	import { agentExchangesQuery, type AgentExchange } from '$lib/model/exchanges.svelte';
	import IntelligenceChip from '$lib/components/IntelligenceChip.svelte';
	import { composerOptions, referenceOptions } from '$lib/model/composer-options.svelte';
	import { runExecCommand } from '$lib/model/skills';
	import { PIN, reactionsQuery } from '$lib/model/reactions.svelte';
	import { documentsQuery } from '$lib/model/document-queries.svelte';
	import { askExec, workDraftFrom } from '$lib/model/ask-exec';
	import Pin from '@lucide/svelte/icons/pin';
	import ConversationTurnDock from '$lib/primitives/ConversationTurnDock.svelte';
	import RoomMessage from '$lib/components/RoomMessage.svelte';
	import RoomManager from '$lib/components/RoomManager.svelte';
	import {
		parsePositiveRoomMessageId,
		parseRoomMessageTarget,
		type RoomMessageTargetUnavailableReason
	} from '$lib/model/room-deep-link';
	import {
		attentionQuery,
		collaborationBootstrapQuery,
		companyPrincipalQuery,
		cockpitQuery
	} from '$lib/model/queries.svelte';
	import { conversationQuery } from '$lib/model/queries.svelte';
	import { cockpitContextPath } from '$lib/model/attention';
	import {
		roomActivityStream,
		roomMessageRevisionsQuery,
		roomMessageSearchQuery,
		roomMessagesQuery,
		roomParticipantsQuery,
		roomReadCursorQuery,
		roomsQuery,
		roomThreadQuery
	} from '$lib/model/room-queries.svelte';
	import {
		deleteRoomMessage,
		editRoomMessage,
		markRoomRead,
		readRoomDraft,
		roomDraftKey,
		sendRoomMessage,
		writeRoomDraft,
		createRoom,
		type NewRoomMention,
		type Room,
		type RoomMessage as RoomMessageRecord,
		type RoomMessageSearchResult
	} from '$lib/model/rooms';
	import type { AttentionItem, ThreadMessage } from '$lib/model/view';
	import Composer from '$lib/primitives/Composer.svelte';
	import { followChat } from '$lib/actions/follow-chat';

	const SEARCH_DEBOUNCE_MS = 250;

	const companyId = $derived(page.params.companyId ?? 'aris');
	const principalProjection = $derived(companyPrincipalQuery(companyId));
	const cockpitProjection = $derived(
		cockpitQuery(companyId, () => principalProjection.view?.membership_role === 'owner')
	);
	const ownerAccess = $derived(principalProjection.view?.membership_role === 'owner');
	const principalActorId = $derived(principalProjection.view?.actor_id ?? '');
	const attention = $derived(attentionQuery(companyId, () => ownerAccess));
	const collaboration = $derived(
		collaborationBootstrapQuery(companyId, () => (ownerAccess ? null : principalProjection.view))
	);
	const roomList = $derived(roomsQuery(companyId));
	const requestedRoomId = $derived(page.url.searchParams.get('room') ?? '');
	const requestedPersonId = $derived(page.url.searchParams.get('person') ?? '');
	let resolvedPersonRoom = $state<{ key: string; room: Room } | null>(null);
	let resolvingPersonKey = $state('');
	let attemptedPersonKey = $state('');
	let personResolutionFailure = $state('');
	let personResolutionRetry = $state(0);
	const pendingDirectCommands = new Map<string, string>();
	const volatileFileDrafts = new Map<string, File[]>();
	const selectedRoomId = $derived(
		requestedRoomId ||
			(resolvedPersonRoom?.key === `${companyId}:${principalActorId}:${requestedPersonId}`
				? resolvedPersonRoom.room.id
				: '')
	);
	const threadRootId = $derived(parsePositiveRoomMessageId(page.url.searchParams.get('thread')));
	const focusedMessageId = $derived(parsePositiveRoomMessageId(page.url.searchParams.get('focus')));
	let focusStatus = $state('');
	const requestedMessageTarget = $derived(parseRoomMessageTarget(page.url.searchParams));
	const exactMessageTarget = $derived(
		requestedMessageTarget.kind === 'target' ? requestedMessageTarget.target : null
	);

	const roomProjection = $derived(
		selectedRoomId ? roomMessagesQuery(companyId, selectedRoomId) : null
	);
	const participantProjection = $derived(
		selectedRoomId ? roomParticipantsQuery(companyId, selectedRoomId) : null
	);
	const readProjection = $derived(
		selectedRoomId ? roomReadCursorQuery(companyId, selectedRoomId) : null
	);
	const activity = $derived(selectedRoomId ? roomActivityStream(companyId, selectedRoomId) : null);
	const threadProjection = $derived(
		selectedRoomId && threadRootId ? roomThreadQuery(companyId, selectedRoomId, threadRootId) : null
	);
	$effect(() => activity?.attach());
	$effect(() => {
		const id = focusedMessageId;
		const projection = threadRootId ? threadProjection : roomProjection;
		const status = projection?.status;
		if (!id || !projection || status === 'unknown') {
			focusStatus = '';
			return;
		}
		let current = true;
		focusStatus = 'Finding message…';
		untrack(() => {
			void (async () => {
				for (
					let page = 0;
					current &&
					!projection.messages.some((m) => m.id === id) &&
					projection.hasMore &&
					page < 20;
					page++
				) {
					const count = projection.messages.length;
					await projection.loadMore();
					if (projection.messages.length === count) break;
				}
				if (current)
					focusStatus = projection.messages.some((m) => m.id === id && !m.deleted_at)
						? ''
						: 'This message is no longer available.';
			})().catch(() => {
				if (current) focusStatus = 'Message could not be loaded.';
			});
		});
		return () => {
			current = false;
		};
	});

	let online = $state(true);
	let composer = $state('');
	let composerFiles = $state<File[]>([]);
	let composerSkills = $state<string[]>([]);
	/* While the agent replies, Send queues and Interrupt stops the reply. */
	let interruptNext = $state(false);
	/* Messages in view when the reader scrolled up; null while following. */
	let awayFrom = $state<number | null>(null);
	let activeDraftKey = $state('');
	let retryCommandId = $state<string | null>(null);
	let retryBody = $state('');
	let sending = $state(false);
	let sendError = $state('');
	let sendNotice = $state('');
	let pendingMessage = $state<RoomMessageRecord | null>(null);
	let lastMarkedRoom = $state('');
	let lastMarkedMessage = $state(0);
	// Not reactive: only paces retries of a failed read marker.
	let readMarkFailures = 0;
	let locallyReadRoom = $state('');
	let locallyReadThrough = $state(0);
	let draftTimer: number | undefined;
	let roomScrollEl = $state<HTMLDivElement | undefined>();
	let threadScrollEl = $state<HTMLDivElement | undefined>();
	let messageSearch = $state('');
	let debouncedMessageSearch = $state('');
	let messageSearchOpen = $state(false);
	let historyMessageId = $state<number | null>(null);
	let exactTargetKey = $state('');
	let exactTargetState = $state<'none' | 'invalid' | 'locating' | 'found' | 'unavailable'>('none');
	let exactTargetUnavailableReason = $state<RoomMessageTargetUnavailableReason | null>(null);
	let messageSearchTimer: ReturnType<typeof setTimeout> | undefined;

	$effect(() => {
		const search = messageSearch.trim();
		clearTimeout(messageSearchTimer);
		if (!search) {
			debouncedMessageSearch = '';
			return;
		}
		debouncedMessageSearch = '';
		messageSearchTimer = setTimeout(() => {
			debouncedMessageSearch = search;
		}, SEARCH_DEBOUNCE_MS);
		return () => clearTimeout(messageSearchTimer);
	});

	const hasMessageSearch = $derived(messageSearch.trim().length > 0);
	const messageSearchPending = $derived(messageSearch.trim() !== debouncedMessageSearch);
	const knownRooms = $derived(new Map(roomList.rooms.map((room) => [room.id, room])));
	const selectedRoom = $derived(
		knownRooms.get(selectedRoomId) ??
			(resolvedPersonRoom?.key === `${companyId}:${principalActorId}:${requestedPersonId}`
				? resolvedPersonRoom.room
				: null)
	);
	$effect(() => {
		const id = selectedRoomId,
			status = roomList.status;
		if (!id || status === 'unknown') return;
		let current = true;
		untrack(() => {
			void (async () => {
				while (current && !roomList.rooms.some((room) => room.id === id) && roomList.hasMore) {
					const count = roomList.rooms.length;
					await roomList.loadMore();
					if (roomList.rooms.length === count) break;
				}
			})().catch(() => {});
		});
		return () => {
			current = false;
		};
	});
	const messageSearchProjection = $derived(
		roomMessageSearchQuery(companyId, debouncedMessageSearch, selectedRoomId)
	);
	const revisionProjection = $derived(
		selectedRoomId && historyMessageId
			? roomMessageRevisionsQuery(companyId, selectedRoomId, historyMessageId)
			: null
	);
	$effect(() => {
		selectedRoomId;
		historyMessageId = null;
	});

	const currentActorId = $derived(
		principalProjection.view?.actor_id ?? readProjection?.actorId ?? ''
	);
	// Everyone who can author or be named here, the owner included: a member
	// reads the owner's name, not the `owner` Actor id.
	const roster = $derived(
		ownerAccess ? (cockpitProjection.view?.people ?? []) : (collaboration.view?.people ?? [])
	);
	const people = $derived(
		roster.filter((person) => person.kind !== 'owner' && person.kind !== 'system')
	);
	const teams = $derived(
		ownerAccess ? (cockpitProjection.view?.teams ?? []) : (collaboration.view?.teams ?? [])
	);
	/* app.html starts a remembered person's room on the next cold load. */
	$effect(() => {
		const room = resolvedPersonRoom;
		if (!room || room.key !== `${companyId}:${principalActorId}:${requestedPersonId}`) return;
		try {
			localStorage.setItem(`restless:${companyId}:person-room:${requestedPersonId}`, room.room.id);
		} catch {
			/* Without storage the room is found the ordinary way. */
		}
	});
	function directCanonicalKey(a: string, b: string): string {
		const [first, second] = [a, b].sort();
		return `direct:${new TextEncoder().encode(first).length}:${first}:${new TextEncoder().encode(second).length}:${second}`;
	}
	$effect(() => {
		void personResolutionRetry;
		const personId = requestedPersonId;
		const actorId = principalActorId;
		const status = roomList.status;
		if (requestedRoomId) return;
		const person = people.find((candidate) => candidate.actor_id === personId);
		if (!personId || !actorId || !person || status === 'unknown') return;
		const key = `${companyId}:${actorId}:${personId}`;
		const currentRooms = roomList.rooms;
		const targetRoomList = roomList;
		const guarded = untrack(() => ({
			resolved: resolvedPersonRoom?.key === key,
			resolving: resolvingPersonKey === key,
			attempted: attemptedPersonKey === key,
			existing: currentRooms.find(
				(room) =>
					room.kind === 'direct' && room.canonical_key === directCanonicalKey(actorId, personId)
			)
		}));
		if (guarded.resolved || guarded.resolving) return;
		if (guarded.existing) {
			resolvedPersonRoom = { key, room: guarded.existing };
			personResolutionFailure = '';
			return;
		}
		if (guarded.attempted) return;
		resolvingPersonKey = key;
		attemptedPersonKey = key;
		personResolutionFailure = '';
		const commandId = pendingDirectCommands.get(key) ?? crypto.randomUUID();
		pendingDirectCommands.set(key, commandId);
		untrack(() => {
			void createRoom(companyId, {
				kind: 'direct',
				title: person.display,
				participant_actor_ids: [personId],
				command_id: commandId
			})
				.then((room) => {
					pendingDirectCommands.delete(key);
					void targetRoomList.refresh();
					if (key !== `${companyId}:${principalActorId}:${requestedPersonId}`) return;
					resolvedPersonRoom = { key, room };
					personResolutionFailure = '';
				})
				.catch((cause) => {
					if (key === `${companyId}:${principalActorId}:${requestedPersonId}`)
						personResolutionFailure = failureSentence(cause, 'Could not open this conversation.');
				})
				.finally(() => {
					if (resolvingPersonKey === key) resolvingPersonKey = '';
				});
		});
	});
	const draftScopeKey = $derived(
		selectedRoomId && currentActorId
			? roomDraftKey(companyId, currentActorId, selectedRoomId, threadRootId)
			: ''
	);

	$effect(() => {
		const nextKey = draftScopeKey;
		if (nextKey === activeDraftKey) return;
		if (!nextKey) {
			/* A Room or authenticated principal changed before the new identity
			 * projection arrived. Retire the old scope immediately so fresh input
			 * can never be written into another principal's draft. */
			if (activeDraftKey) volatileFileDrafts.set(activeDraftKey, [...composerFiles]);
			activeDraftKey = '';
			composer = '';
			composerFiles = [];
			composerSkills = [];
			retryCommandId = null;
			retryBody = '';
			sendError = '';
			pendingMessage = null;
			return;
		}
		if (activeDraftKey) volatileFileDrafts.set(activeDraftKey, [...composerFiles]);
		const hadUnscopedInput = !activeDraftKey && (composer.length > 0 || composerFiles.length > 0);
		activeDraftKey = nextKey;
		const stored = readRoomDraft(nextKey);
		const oldPersonKey = requestedPersonId
			? roomDraftKey(companyId, currentActorId, `person:${requestedPersonId}`, null)
			: '';
		const legacyDraft =
			oldPersonKey && oldPersonKey !== nextKey ? readRoomDraft(oldPersonKey) : null;
		/* Identity can arrive after the owner starts typing. Keep that ephemeral
		 * input rather than replacing it with a persisted principal-scoped draft. */
		if (!hadUnscopedInput) {
			const draft =
				stored.body || stored.commandId ? stored : legacyDraft?.body ? legacyDraft : stored;
			composer = draft.body;
			composerFiles = volatileFileDrafts.get(nextKey) ?? [];
			retryCommandId = draft.commandId;
			retryBody = draft.commandId ? draft.body.trim() : '';
			if (draft === legacyDraft && oldPersonKey) {
				writeRoomDraft(nextKey, draft);
				writeRoomDraft(oldPersonKey, { body: '', commandId: null, updatedAt: '' });
			}
		}
		sendError = '';
		sendNotice = '';
		pendingMessage = null;
	});

	$effect(() => {
		const key = activeDraftKey;
		const body = composer;
		const commandId = retryBody === body.trim() ? retryCommandId : null;
		if (!key || typeof window === 'undefined') return;
		const draft = { body, commandId, updatedAt: new Date().toISOString() };
		window.clearTimeout(draftTimer);
		draftTimer = window.setTimeout(() => {
			writeRoomDraft(key, draft);
		}, 120);
		return () => {
			window.clearTimeout(draftTimer);
			/* Switching Room or Thread must not strand the final keystroke inside a
			 * cancelled debounce. Flush the captured scope before it disappears. */
			writeRoomDraft(key, draft);
		};
	});

	function flushDraft() {
		if (activeDraftKey) {
			writeRoomDraft(activeDraftKey, {
				body: composer,
				commandId: retryBody === composer.trim() ? retryCommandId : null,
				updatedAt: new Date().toISOString()
			});
		}
	}

	onDestroy(() => {
		if (typeof window === 'undefined') return;
		window.clearTimeout(draftTimer);
		clearTimeout(messageSearchTimer);
		flushDraft();
	});

	const roomMessages = $derived(roomProjection?.messages ?? []);
	const roots = $derived(roomMessages.filter((message) => message.parent_message_id === null));
	const visibleRoots = $derived(
		pendingMessage && pendingMessage.parent_message_id === null ? [...roots, pendingMessage] : roots
	);
	/* A page of history is fifty messages, each rendered as Markdown, which cost
	 * a phone over a second before the newest showed. The newest fifteen render
	 * first; older loaded ones come back with "Load older messages", before it
	 * fetches another page. A linked or searched message is always in view. */
	const ROOT_WINDOW = 15;
	let rootWindow = $state(ROOT_WINDOW);
	let rootWindowFor = '';
	$effect(() => {
		if (selectedRoomId === rootWindowFor) return;
		rootWindowFor = selectedRoomId;
		rootWindow = ROOT_WINDOW;
	});
	const rootStart = $derived.by(() => {
		const start = Math.max(0, visibleRoots.length - rootWindow);
		const pinned = [
			focusedMessageId,
			exactMessageTarget?.messageId,
			exactMessageTarget?.threadRootMessageId
		];
		const index = visibleRoots.findIndex((message) => pinned.includes(message.id));
		return index >= 0 ? Math.min(start, index) : start;
	});
	const shownRoots = $derived(rootStart ? visibleRoots.slice(rootStart) : visibleRoots);
	const libraryDocuments = $derived(documentsQuery(ownerAccess ? companyId : ''));
	const pinnedRoots = $derived.by(() => {
		const ids = reactions.pinned();
		return visibleRoots.filter((message) => ids.has(message.id) && !message.deleted_at);
	});
	const threadMessages = $derived(threadProjection?.messages ?? []);
	const visibleThread = $derived(
		pendingMessage && pendingMessage.parent_message_id !== null
			? [...threadMessages, pendingMessage]
			: threadMessages
	);
	$effect(() => {
		const parsed = requestedMessageTarget;
		const projection = threadProjection;
		const projectionStatus = projection?.status;
		const projectionFailure = projection?.failure;
		if (parsed.kind === 'none') {
			exactTargetKey = '';
			exactTargetState = 'none';
			exactTargetUnavailableReason = null;
			return;
		}
		if (parsed.kind === 'invalid') {
			exactTargetKey = '';
			exactTargetState = 'invalid';
			exactTargetUnavailableReason = null;
			return;
		}

		const target = parsed.target;
		const key = [
			companyId,
			target.roomId,
			target.threadRootMessageId,
			target.messageId,
			target.mentionId
		].join(':');
		if (
			selectedRoomId.toLowerCase() !== target.roomId ||
			threadRootId !== target.threadRootMessageId
		) {
			exactTargetKey = '';
			exactTargetState = 'invalid';
			exactTargetUnavailableReason = null;
			return;
		}
		if (threadMessages.find((message) => message.id === target.messageId)?.deleted_at) {
			exactTargetKey = key;
			exactTargetState = 'unavailable';
			exactTargetUnavailableReason = 'deleted';
			return;
		}
		if (exactTargetKey === key) return;
		if (!projection || projectionStatus === 'unknown') {
			exactTargetState = projectionFailure ? 'unavailable' : 'locating';
			exactTargetUnavailableReason = projectionFailure ? 'paging-unavailable' : null;
			return;
		}

		exactTargetKey = key;
		exactTargetState = 'locating';
		exactTargetUnavailableReason = null;
		void projection.locateTarget(target).then((location) => {
			if (exactTargetKey !== key) return;
			if (location.state === 'found') {
				exactTargetState = 'found';
				exactTargetUnavailableReason = null;
				return;
			}
			exactTargetState = 'unavailable';
			exactTargetUnavailableReason = location.reason;
		});
	});
	const lastLoadedMessageId = $derived.by(() => {
		const loaded = [roomMessages.at(-1)?.id, threadMessages.at(-1)?.id].filter(
			(value): value is number => typeof value === 'number'
		);
		return loaded.length ? Math.max(...loaded) : null;
	});
	const unreadFrom = $derived.by(() => {
		const server = readProjection?.cursor?.last_read_message_id ?? null;
		const local = locallyReadRoom === selectedRoomId ? locallyReadThrough : 0;
		return Math.max(server ?? 0, local) || server;
	});
	const participants = $derived(
		(participantProjection?.participants ?? []).filter((p) => !p.left_at)
	);
	const directPartner = $derived(
		selectedRoom?.kind === 'direct' &&
			(!requestedPersonId ||
				requestedPersonId === participants.find((p) => p.actor_id !== currentActorId)?.actor_id)
			? participants.find((p) => p.actor_id !== currentActorId)?.actor_id
			: undefined
	);
	const directPerson = $derived(people.find((person) => person.actor_id === directPartner) ?? null);
	/* Handoffs between this agent and others appear where they happened. */
	const exchanges = agentExchangesQuery(
		() => companyId,
		() => directPartner ?? '',
		() => ownerAccess && !!directPartner && actorIsAgent(directPartner)
	);
	const at = (value: string) => new Date(value).getTime();
	function handoffsBefore(index: number): AgentExchange[] {
		if (index === 0) return [];
		const after = at(visibleRoots[index - 1].created_at);
		const until = at(visibleRoots[index].created_at);
		return exchanges.exchanges.filter(
			(exchange) => at(exchange.created_at) > after && at(exchange.created_at) <= until
		);
	}
	const handoffsSinceLast = $derived.by(() => {
		const last = visibleRoots.at(-1);
		return last
			? exchanges.exchanges.filter((exchange) => at(exchange.created_at) > at(last.created_at))
			: [];
	});
	const directTeam = $derived(
		directPerson
			? (teams.find((team) => team.lead_actor_id === directPerson.actor_id) ??
					(directPerson.team_id
						? (teams.find((team) => team.id === directPerson.team_id) ?? null)
						: null))
			: null
	);
	const accountableDirect = $derived(
		ownerAccess &&
			!!directPerson &&
			(directPerson.kind === 'exec' ||
				teams.some((team) => team.lead_actor_id === directPerson.actor_id))
	);
	const actorConversation = $derived(
		directPartner && actorIsAgent(directPartner)
			? conversationQuery(
					companyId,
					directPartner,
					undefined,
					undefined,
					true,
					currentActorId,
					accountableDirect
				)
			: null
	);
	$effect(() => actorConversation?.attach());
	const actorMessages = $derived(actorConversation?.messages ?? []);
	const actorMessagesById = $derived(
		new Map(
			actorMessages
				.filter((message) => !message.id.startsWith('optimistic:'))
				.map((message) => [message.id, message])
		)
	);
	const options = composerOptions(
		() => companyId,
		() => accountableDirect,
		() => ownerAccess && directPartner === 'exec'
	);
	const reactions = reactionsQuery(
		() => companyId,
		() =>
			[...roomMessages, ...threadMessages]
				.map((message) => message.id)
				.filter((id) => id > 0)
				.slice(-120),
		() => currentActorId
	);
	const leadTurn = $derived(accountableDirect ? (actorConversation?.activeTurn ?? null) : null);
	function attentionFor(actorId: string): AttentionItem[] {
		if (!ownerAccess) return [];
		const items = attention.view?.items ?? [];
		if (actorId === 'exec') return items;
		const team = teams.find((candidate) => candidate.lead_actor_id === actorId);
		const members = new Set([
			actorId,
			...people
				.filter((person) => team && person.team_id === team.id)
				.map((person) => person.actor_id)
		]);
		return items.filter((item) => {
			const sourceActor =
				item.responsibleActor?.id ?? item.runtimeAttach?.requestingActor ?? item.briefAuthor?.id;
			const workOwner = attention.view?.workGraph?.work.find(
				(work) => work.id === item.workId
			)?.owner_id;
			return (sourceActor && members.has(sourceActor)) || (workOwner && members.has(workOwner));
		});
	}
	const leadAttention = $derived(
		ownerAccess && !!directPartner && actorIsAgent(directPartner)
			? attentionFor(directPartner ?? '').filter(
					(item) => item.source.kind !== 'conversation_owner_need'
				)
			: []
	);
	const actorNameForHeader = $derived(directPerson?.display ?? directPartner ?? '');
	const actorDisplay = $derived(directPerson?.display ?? directPartner ?? '');
	const canExtendDirect = $derived(
		participantProjection?.status === 'live' &&
			participants.some((p) => p.actor_id === currentActorId) &&
			participants.some((p) => p.actor_id !== currentActorId)
	);
	const participantSummary = $derived.by(() => {
		const names = participants.map((participant) => actorName(participant.actor_id));
		if (names.length <= 3) return names.join(', ');
		return `${names.slice(0, 2).join(', ')} and ${names.length - 2} more`;
	});

	$effect(() => {
		const room = selectedRoomId;
		const company = companyId;
		const projection = readProjection;
		const through = lastLoadedMessageId;
		if (!online || !room || through === null) return;
		if (room === lastMarkedRoom && through <= lastMarkedMessage) return;
		if (readProjection?.cursor?.last_read_message_id === through) return;
		lastMarkedRoom = room;
		lastMarkedMessage = through;
		void markRoomRead(company, room, through)
			.then((cursor) => {
				projection?.accept(cursor);
				if (company !== companyId || room !== selectedRoomId) return;
				locallyReadRoom = room;
				locallyReadThrough = cursor.last_read_message_id ?? through;
				readMarkFailures = 0;
			})
			.catch(() => {
				// Read state is a recoverable projection. Re-arming the marker at once
				// re-ran this effect immediately and retried in a tight loop, so a
				// failing endpoint received hundreds of writes a second; back off.
				readMarkFailures += 1;
				const delay = Math.min(60_000, 1_000 * 2 ** readMarkFailures);
				setTimeout(() => {
					if (lastMarkedRoom === room && lastMarkedMessage === through) lastMarkedMessage = 0;
				}, delay);
			});
	});

	function displayName(actorId: string): string {
		return roster.find((person) => person.actor_id === actorId)?.display ?? actorId;
	}

	function actorName(actorId: string): string {
		return actorId === currentActorId ? 'You' : displayName(actorId);
	}

	function actorIsAgent(actorId: string): boolean {
		const kind = people.find((person) => person.actor_id === actorId)?.kind;
		return actorId === 'exec' || kind === 'exec' || kind === 'staff';
	}

	// An open mention of the viewer in the open Thread: a reply there may
	// answer it, which clears it from their attention.
	const answerableMention = $derived(
		threadRootId === null
			? null
			: ([...(roomProjection?.mentions ?? []), ...(threadProjection?.mentions ?? [])].find(
					(mention) =>
						mention.thread_root_message_id === threadRootId &&
						mention.mentioned_actor_id === currentActorId &&
						mention.resolution_message_id === null &&
						mention.cancelled_event_id === null
				) ?? null)
	);
	let resolvesMention = $state(true);
	$effect(() => {
		void answerableMention?.id;
		resolvesMention = true;
	});

	function mentionsFor(messageId: number) {
		const mentions = [...(roomProjection?.mentions ?? []), ...(threadProjection?.mentions ?? [])];
		return [...new Map(mentions.map((mention) => [mention.id, mention])).values()].filter(
			(mention) => mention.message_id === messageId
		);
	}

	function roomLabel(roomId: string): string {
		const room = knownRooms.get(roomId);
		if (room?.kind === 'direct' && roomId === selectedRoomId) {
			const other = participants.filter((p) => p.actor_id !== currentActorId);
			if (other.length) return other.map((p) => actorName(p.actor_id)).join(', ');
		}
		return room?.title ?? `Conversation ${roomId.slice(0, 8)}`;
	}

	function roomHref(roomId: string): string {
		const params = new URLSearchParams({ room: roomId });
		const view = page.url.searchParams.get('view');
		if (view) params.set('view', view);
		const documentId = page.url.searchParams.get('document');
		if (documentId !== null) params.set('document', documentId);
		return `/${encodeURIComponent(companyId)}/people?${params}`;
	}

	function openSearchResult(message: RoomMessageSearchResult) {
		messageSearch = '';
		messageSearchOpen = false;
		const root = message.thread_root_message_id;
		const suffix = root ? `&thread=${encodeURIComponent(root)}` : '';
		void goto(`${roomHref(message.room_id)}${suffix}&focus=${message.id}`, {
			keepFocus: true,
			noScroll: true
		});
	}

	function toggleHistory(messageId: number) {
		historyMessageId = historyMessageId === messageId ? null : messageId;
	}

	function mayDelete(message: RoomMessageRecord): boolean {
		if (message.id <= 0 || message.deleted_at) return false;
		if (message.from_actor === currentActorId) return true;
		return (
			selectedRoom !== null &&
			selectedRoom.kind !== 'direct' &&
			participants.some(
				(participant) => participant.actor_id === currentActorId && participant.role === 'owner'
			)
		);
	}

	async function saveMessageEdit(message: RoomMessageRecord, body: string, commandId: string) {
		const targetCompanyId = companyId;
		const targetRoomId = message.room_id;
		const targetRoomProjection = selectedRoomId === targetRoomId ? roomProjection : null;
		const targetThreadProjection = selectedRoomId === targetRoomId ? threadProjection : null;
		const targetRevisionProjection =
			selectedRoomId === targetRoomId && historyMessageId === message.id
				? revisionProjection
				: null;
		try {
			const result = await editRoomMessage(
				targetCompanyId,
				targetRoomId,
				message.id,
				body,
				message.revision_number,
				commandId
			);
			targetRoomProjection?.acceptEdit(message.id, result);
			await targetRevisionProjection?.refresh();
		} catch (cause) {
			if ((cause as { status?: unknown }).status === 409) {
				await Promise.all([targetRoomProjection?.refresh(), targetThreadProjection?.refresh()]);
			}
			throw cause;
		}
	}

	async function removeMessage(message: RoomMessageRecord, commandId: string) {
		const targetCompanyId = companyId;
		const targetRoomId = message.room_id;
		const targetRoomProjection = selectedRoomId === targetRoomId ? roomProjection : null;
		const result = await deleteRoomMessage(targetCompanyId, targetRoomId, message.id, commandId);
		targetRoomProjection?.acceptDelete(message.id, result.tombstone.created_at);
		if (
			companyId === targetCompanyId &&
			selectedRoomId === targetRoomId &&
			historyMessageId === message.id
		) {
			historyMessageId = null;
		}
	}

	function threadHref(messageId: number): string {
		return `${roomHref(selectedRoomId)}&thread=${encodeURIComponent(messageId)}`;
	}

	function threadSummary(messageId: number) {
		const summary = roomProjection?.threads.get(messageId);
		return summary
			? {
					count: summary.reply_count,
					lastAt: summary.last_reply_at,
					repliers: summary.repliers
				}
			: null;
	}

	async function copyMessageLink(messageId: number) {
		const link = new URL(`${roomHref(selectedRoomId)}&focus=${messageId}`, window.location.origin);
		try {
			await navigator.clipboard.writeText(link.href);
			sendNotice = 'Link to that message copied.';
		} catch {
			sendError = 'The link could not be copied.';
		}
	}

	function openThread(messageId: number) {
		void goto(threadHref(messageId), { keepFocus: true, noScroll: true });
	}

	function closeThread() {
		void goto(roomHref(selectedRoomId), { keepFocus: true, noScroll: true });
	}

	async function loadOlderRoomMessages() {
		if (!roomProjection || !roomScrollEl) return;
		const before = roomScrollEl.scrollHeight;
		if (rootStart) rootWindow = visibleRoots.length - rootStart + ROOT_WINDOW;
		else await roomProjection.loadMore();
		await tick();
		roomScrollEl.scrollTop += roomScrollEl.scrollHeight - before;
	}

	async function loadOlderThreadMessages() {
		if (!threadProjection || !threadScrollEl) return;
		const before = threadScrollEl.scrollHeight;
		await threadProjection.loadMore();
		await tick();
		threadScrollEl.scrollTop += threadScrollEl.scrollHeight - before;
	}

	function knownMentions(body: string): NewRoomMention[] {
		const tokens = new Set(
			body
				.split(/\s+/)
				.filter((token) => token.startsWith('@'))
				.map((token) => token.slice(1).replace(/[.,!?;]+$/u, ''))
				.filter(Boolean)
		);
		const known = new Set(participants.map((participant) => participant.actor_id));
		if (tokens.has('exec')) known.add('exec');
		// A direct chat addresses its agent counterpart without requiring an @ token.
		if (directPartner && actorIsAgent(directPartner)) tokens.add(directPartner);
		return [...tokens].filter((actorId) => known.has(actorId)).map((actor_id) => ({ actor_id }));
	}

	async function submitMessage(event: SubmitEvent) {
		event.preventDefault();
		const body = composer.trim();
		if (
			!body ||
			!selectedRoomId ||
			!currentActorId ||
			sending ||
			!online ||
			(selectedRoom?.kind === 'direct' && !canExtendDirect)
		)
			return;

		const commandId = retryCommandId && retryBody === body ? retryCommandId : crypto.randomUUID();
		const parent = threadRootId;
		const targetCompany = companyId,
			targetRoom = selectedRoomId,
			targetActor = currentActorId,
			targetDraft = activeDraftKey;
		const recipientActor = directPartner;
		const sendAsAccountableLead = accountableDirect && parent === null;
		const targetConversation = actorConversation;
		const files = [...composerFiles];
		const skills = [...composerSkills];
		if (sendAsAccountableLead && recipientActor === 'exec' && ownerAccess) {
			const command = await runExecCommand(targetCompany, body).catch((cause) => ({
				error: failureSentence(cause, 'That command did not run.'),
				send: false,
				notice: undefined
			}));
			if (!command.send) {
				if (command.error) sendError = command.error;
				else {
					sendNotice = command.notice ?? '';
					composer = '';
				}
				return;
			}
		}
		const targetProjection = parent === null ? roomProjection : threadProjection;
		const stillCurrent = () =>
			companyId === targetCompany &&
			selectedRoomId === targetRoom &&
			currentActorId === targetActor &&
			threadRootId === parent;
		const mentions = knownMentions(body);
		const resolvesMentionId =
			parent !== null && resolvesMention ? (answerableMention?.id ?? null) : null;
		const followDirectReply =
			!sendAsAccountableLead && parent === null && !!directPartner && actorIsAgent(directPartner);
		writeRoomDraft(targetDraft, { body, commandId, updatedAt: new Date().toISOString() });
		retryCommandId = commandId;
		retryBody = body;
		sending = true;
		sendError = '';
		pendingMessage = {
			id: -Date.now(),
			room_id: selectedRoomId,
			from_actor: currentActorId,
			to_actor: null,
			body,
			outcome_standard: null,
			parent_message_id: parent,
			thread_root_message_id: parent,
			client_command_id: commandId,
			created_at: new Date().toISOString(),
			revision_number: 0,
			edited_at: null,
			deleted_at: null,
			legacy_read_at: null
		};
		let sentMessageId: number | null = null;
		if (parent === null) {
			// Your message rises to the top of the room and replies grow beneath it.
			await tick();
			roomScrollEl?.dispatchEvent(
				new CustomEvent('chat-scroll-anchor', { detail: { selector: '.room-message.you' } })
			);
		}
		try {
			if (sendAsAccountableLead && targetConversation && recipientActor) {
				const contextPath = cockpitContextPath(targetCompany, page.url);
				const result = await targetConversation.send(
					body,
					files,
					contextPath,
					false,
					!!leadTurn && interruptNext,
					undefined,
					skills
				);
				if (result.interrupted && stillCurrent())
					sendNotice = `${actorName(recipientActor)} was interrupted and your new direction is queued.`;
				void targetProjection?.refresh().catch(() => {});
			} else {
				const result = await sendRoomMessage(
					targetCompany,
					targetRoom,
					body,
					commandId,
					parent,
					mentions,
					resolvesMentionId
				);
				sentMessageId = result.message.id;
				targetProjection?.accept(result);
			}
			writeRoomDraft(targetDraft, {
				body: '',
				commandId: null,
				updatedAt: new Date().toISOString()
			});
			volatileFileDrafts.delete(targetDraft);
			if (!stillCurrent()) return;
			composer = '';
			composerFiles = [];
			retryCommandId = null;
			retryBody = '';
			pendingMessage = null;
			if (activeDraftKey) {
				writeRoomDraft(activeDraftKey, { body: '', commandId: null, updatedAt: '' });
			}
			if (followDirectReply && sentMessageId !== null) {
				await goto(threadHref(sentMessageId), { keepFocus: true, noScroll: true });
			}
			await tick();
			const scroller = parent === null ? roomScrollEl : threadScrollEl;
			scroller?.scrollTo({
				top: scroller.scrollHeight,
				behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth'
			});
		} catch (cause) {
			const status = (cause as { status?: unknown }).status;
			const retryable =
				typeof status !== 'number' ||
				status === 408 ||
				status === 425 ||
				status === 429 ||
				status >= 500;
			if (!stillCurrent()) {
				writeRoomDraft(targetDraft, {
					body,
					commandId: retryable ? commandId : null,
					updatedAt: new Date().toISOString()
				});
				if (files.length) volatileFileDrafts.set(targetDraft, files);
				return;
			}
			retryCommandId = retryable ? commandId : null;
			retryBody = retryable ? body : '';
			pendingMessage = null;
			sendError = failureSentence(cause, 'This message was not delivered.');
			if (files.length) volatileFileDrafts.set(targetDraft, files);
		} finally {
			sending = false;
		}
	}

	function dayLabel(value: string): string {
		const date = new Date(value);
		if (Number.isNaN(date.getTime())) return '';
		const today = new Date();
		if (date.toDateString() === today.toDateString()) return 'Today';
		const yesterday = new Date(today);
		yesterday.setDate(today.getDate() - 1);
		if (date.toDateString() === yesterday.toDateString()) return 'Yesterday';
		return date.toLocaleDateString(undefined, { month: 'long', day: 'numeric' });
	}

	/* One header for a run: same author, same day, within five minutes. */
	function continuesRun(previous: RoomMessageRecord, current: RoomMessageRecord): boolean {
		return (
			previous.from_actor === current.from_actor &&
			dayKey(previous.created_at) === dayKey(current.created_at) &&
			new Date(current.created_at).getTime() - new Date(previous.created_at).getTime() < 5 * 60_000
		);
	}

	function dayKey(value: string): string {
		const date = new Date(value);
		return Number.isNaN(date.getTime()) ? value : date.toDateString();
	}

	function exactTargetUnavailableCopy(reason: RoomMessageTargetUnavailableReason | null): string {
		if (reason === 'deleted') return 'The linked message was deleted.';
		if (reason === 'page-limit') {
			return 'The linked message is too far back to load from this link.';
		}
		if (reason === 'paging-unavailable') {
			return 'The linked message is unavailable while older messages cannot be loaded.';
		}
		return 'The linked message is no longer available in this Thread.';
	}
</script>

<svelte:window
	bind:online
	onpagehide={flushDraft}
	onkeydown={(event) => {
		if (!selectedRoomId || !(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== 'f')
			return;
		if (document.activeElement?.closest('.bridge-exrail')) return;
		event.preventDefault();
		messageSearchOpen = true;
	}}
/>
<div
	class="cockpit-screen rooms-screen"
	class:room-selected={selectedRoomId !== '' || requestedPersonId !== ''}
	class:thread-selected={threadRootId !== null}
>
	<section class="room-conversation cockpit-pane">
		{#if focusStatus}<p class="exact-target-state" role="status">{focusStatus}</p>{/if}
		{#if selectedRoomId}
			<header class="room-head">
				<div class="room-head-copy">
					{#if directPerson}
						<span class="room-contact"
							><span class="person-avatar">{initials(directPerson.display)}</span><strong
								>{directPerson.display}</strong
							></span
						>
					{:else}<strong>{roomLabel(selectedRoomId)}</strong>{/if}
					{#if participantSummary && !directPerson}<small>{participantSummary}</small>{/if}
				</div>
				<button
					type="button"
					class="message-search-toggle"
					class:active={messageSearchOpen}
					aria-expanded={messageSearchOpen}
					aria-label={messageSearchOpen ? 'Close search' : 'Search this conversation (Ctrl+F)'}
					title={messageSearchOpen ? 'Close search' : 'Search this conversation (Ctrl+F)'}
					onclick={() => {
						messageSearchOpen = !messageSearchOpen;
						if (!messageSearchOpen) messageSearch = '';
					}}
				>
					{#if messageSearchOpen}
						<X size={15} strokeWidth={2} aria-hidden="true" />
					{:else}
						<Search size={15} strokeWidth={2} aria-hidden="true" />
					{/if}
				</button>
				{#if selectedRoom?.kind === 'direct' && canExtendDirect}
					<RoomManager
						{companyId}
						actorId={currentActorId}
						{people}
						label="Add people"
						initialParticipants={participants
							.filter((p) => p.actor_id !== currentActorId)
							.map((p) => p.actor_id)}
						initialContext={roomMessages.at(-1)?.body ?? ''}
						oncreated={(room) => goto(roomHref(room.id))}
					/>
				{:else if selectedRoom?.kind === 'direct'}
					<button
						type="button"
						class="btn small"
						disabled
						title="Participants must finish loading before starting a group.">Add people</button
					>
				{:else if selectedRoom}
					<RoomManager
						{companyId}
						actorId={currentActorId}
						{people}
						room={selectedRoom}
						{participants}
					/>
				{/if}
				<!-- Quiet while live: the stream only earns a chip when it is not. -->
				{#if !online || activity?.transport !== 'live'}
					<div
						class="room-transport"
						class:degraded={!online || activity?.transport === 'reconnecting'}
						role="status"
						aria-live="polite"
						title={!online
							? 'This device is offline. Messages will send when it reconnects.'
							: 'New messages appear when the live connection is restored; nothing is lost.'}
					>
						{#if !online}
							<WifiOff size={13} strokeWidth={2} aria-hidden="true" /> Offline
						{:else if activity?.transport === 'reconnecting'}
							<WifiOff size={13} strokeWidth={2} aria-hidden="true" /> Reconnecting
						{:else}
							<Wifi size={13} strokeWidth={2} aria-hidden="true" /> Connecting
						{/if}
					</div>
				{:else}
					<span class="sr-only" role="status">Live</span>
				{/if}
			</header>
			{#if exactTargetState === 'invalid' && threadRootId === null}
				<div class="exact-target-state unavailable" role="status">
					This linked message address is invalid.
				</div>
			{/if}
			{#if messageSearchOpen}
				<label class="message-search-field">
					<Search size={14} strokeWidth={1.9} aria-hidden="true" />
					<span class="sr-only">Search this conversation</span>
					<input
						bind:value={messageSearch}
						type="search"
						maxlength={256}
						placeholder="Search this conversation"
						{@attach (input) => input.focus()}
						onkeydown={(event) => {
							if (event.key === 'Escape') {
								messageSearch = '';
								messageSearchOpen = false;
							}
						}}
					/>
				</label>
			{/if}

			{#if (!online || activity?.transport === 'reconnecting') && roomMessages.length}
				<div class="degraded-strip" role="status">
					Showing the last loaded messages. Your draft stays on this device.
				</div>
			{/if}

			{#if pinnedRoots.length && !hasMessageSearch}
				<details class="room-pinned">
					<summary><Pin size={12} aria-hidden="true" />{pinnedRoots.length} pinned</summary>
					{#each pinnedRoots as message (message.id)}
						<a href={`${roomHref(selectedRoomId)}&focus=${message.id}`}
							><strong>{actorName(message.from_actor)}</strong><span
								>{message.body.replace(/\s+/g, ' ').slice(0, 140)}</span
							></a
						>
					{/each}
				</details>
			{/if}
			<div
				class="room-message-list"
				bind:this={roomScrollEl}
				use:followChat={{
					key: selectedRoomId,
					enabled: !hasMessageSearch && !focusedMessageId,
					memory: `room:${companyId}:${selectedRoomId}`,
					onfollow: (following) => {
						awayFrom = following ? null : visibleRoots.length;
					}
				}}
			>
				{#if hasMessageSearch}
					<div class="message-search-results" aria-live="polite">
						{#if messageSearchPending || messageSearchProjection.status === 'unknown'}
							<p class="room-empty">Searching messages…</p>
						{:else if messageSearchProjection.failure}
							<FailureNotice
								error={messageSearchProjection.failure}
								subject="search results"
								onretry={messageSearchProjection.refresh}
							/>
						{:else}
							{#each messageSearchProjection.messages as result (result.id)}
								<button
									type="button"
									class="message-search-result"
									onclick={() => openSearchResult(result)}
								>
									<span>
										<strong>{roomLabel(result.room_id)}</strong>
										<small>{actorName(result.from_actor)}</small>
									</span>
									<p>{result.snippet}</p>
								</button>
							{:else}
								<p class="room-empty">No matching messages.</p>
							{/each}
							{#if messageSearchProjection.hasMore}
								<button
									type="button"
									class="load-message-page"
									disabled={messageSearchProjection.loadingMore}
									onclick={() => void messageSearchProjection.loadMore()}
								>
									{messageSearchProjection.loadingMore ? 'Loading…' : 'More results'}
								</button>
							{/if}
						{/if}
					</div>
				{:else}
					{#if roomProjection && (rootStart || roomProjection.hasMore)}
						<button
							type="button"
							class="load-message-page"
							disabled={roomProjection.loadingMore}
							onclick={() => void loadOlderRoomMessages()}
						>
							{roomProjection.loadingMore ? 'Loading…' : 'Load older messages'}
						</button>
					{/if}
					{#each shownRoots as message, index (message.id)}
						{#each handoffsBefore(rootStart + index) as exchange (exchange.id)}<HandoffReceipt
								{exchange}
								name={exchanges.name}
							/>{/each}
						{#if index === 0 || dayKey(message.created_at) !== dayKey(shownRoots[index - 1].created_at)}
							<div class="room-day"><span>{dayLabel(message.created_at)}</span></div>
						{/if}
						{#if unreadFrom !== null && message.id > unreadFrom && (rootStart + index === 0 || visibleRoots[rootStart + index - 1].id <= unreadFrom)}
							<div class="unread-rule"><span>New since your last read</span></div>
						{/if}
						<RoomMessage
							{message}
							{companyId}
							reactions={reactions.summaryFor(message.id)}
							onreact={ownerAccess && message.id > 0
								? (emoji, on) => void reactions.react(message.id, emoji, on)
								: undefined}
							continued={index > 0 &&
								continuesRun(shownRoots[index - 1], message) &&
								!(
									unreadFrom !== null &&
									message.id > unreadFrom &&
									shownRoots[index - 1].id <= unreadFrom
								)}
							presentation={actorMessagesById.get(String(message.id))}
							hrefFor={(attachment) =>
								`/api/companies/${encodeURIComponent(companyId)}/attachments/${encodeURIComponent(attachment.uploadId)}`}
							author={message.id < 0 ? 'You' : actorName(message.from_actor)}
							isYou={message.id < 0 || message.from_actor === currentActorId}
							isAgent={actorIsAgent(message.from_actor)}
							targeted={(exactTargetState === 'found' &&
								exactMessageTarget?.messageId === message.id) ||
								focusedMessageId === message.id}
							focusKey={focusedMessageId
								? `search:${selectedRoomId}:${focusedMessageId}`
								: exactTargetKey}
							mentions={mentionsFor(message.id)}
							nameFor={actorName}
							replies={threadSummary(message.id)}
							onjump={message.id > 0 ? () => void copyMessageLink(message.id) : null}
							pinned={reactions.pinned().has(message.id)}
							onpin={ownerAccess && message.id > 0
								? () => void reactions.react(message.id, PIN, !reactions.pinned().has(message.id))
								: null}
							onwork={ownerAccess && message.id > 0 && !message.deleted_at
								? () => askExec(workDraftFrom(actorName(message.from_actor), message.body))
								: null}
							onthread={message.id > 0 ? () => openThread(message.id) : null}
							canEdit={message.id > 0 &&
								!message.deleted_at &&
								message.from_actor === currentActorId}
							canDelete={mayDelete(message)}
							historyOpen={historyMessageId === message.id}
							revisions={historyMessageId === message.id
								? (revisionProjection?.revisions ?? [])
								: []}
							historyStatus={historyMessageId === message.id
								? (revisionProjection?.status ?? 'unknown')
								: 'live'}
							historyFailure={historyMessageId === message.id
								? revisionProjection?.failure
									? failureSentence(revisionProjection.failure, 'Edit history couldn’t load.')
									: ''
								: ''}
							historyHasMore={historyMessageId === message.id &&
								Boolean(revisionProjection?.hasMore)}
							historyLoadingMore={historyMessageId === message.id &&
								Boolean(revisionProjection?.loadingMore)}
							onhistory={message.id > 0 && message.edited_at
								? () => toggleHistory(message.id)
								: null}
							onloadhistory={() => void revisionProjection?.loadMore()}
							onedit={(body, commandId) => saveMessageEdit(message, body, commandId)}
							ondelete={(commandId) => removeMessage(message, commandId)}
						/>
						{#if index === visibleRoots.length - 1}{#each handoffsSinceLast as exchange (exchange.id)}<HandoffReceipt
									{exchange}
									name={exchanges.name}
								/>{/each}{/if}
					{:else}
						{#if roomProjection?.failure && !roomMessages.length}
							<FailureNotice
								error={roomProjection.failure}
								subject="this conversation"
								variant="page"
								onretry={() => roomProjection?.refresh()}
							/>
						{:else if roomProjection?.status === 'unknown'}
							<Skeleton label="Loading conversation" variant="messages" count={3} />
						{:else}
							<div class="conversation-empty">
								<strong>Nothing said yet.</strong>
								<p>Start the conversation.</p>
							</div>
						{/if}
					{/each}
					{#if leadTurn}
						<ConversationTurnDock participantName={actorDisplay} turn={leadTurn} />
					{/if}
				{/if}
			</div>

			{#if awayFrom !== null && visibleRoots.length && !hasMessageSearch}
				<div class="jump-anchor">
					<button
						type="button"
						class="jump-latest"
						onclick={() => roomScrollEl?.dispatchEvent(new Event('chat-scroll-end'))}
						>{visibleRoots.length > awayFrom
							? `${visibleRoots.length - awayFrom} new ↓`
							: 'Jump to latest ↓'}</button
					>
				</div>
			{/if}
			{#if threadRootId === null}
				{#if leadAttention.length}<a
						class="chat-attention-strip"
						href={`/${companyId}?item=${encodeURIComponent(leadAttention[0].id)}`}
						><span>{leadAttention.length} {leadAttention.length === 1 ? 'needs' : 'need'} you</span
						><strong>{leadAttention[0].title}</strong><span aria-hidden="true">→</span></a
					>{/if}
				{@render messageComposer()}
			{/if}
		{:else}
			<div class="conversation-empty choose-room">
				{#if exactTargetState === 'invalid'}
					<strong>This linked message address is invalid.</strong>
				{:else if requestedPersonId && personResolutionFailure}
					<strong>Could not open this conversation.</strong>
					<p>{personResolutionFailure}</p>
					<button
						type="button"
						onclick={() => {
							attemptedPersonKey = '';
							resolvingPersonKey = '';
							personResolutionFailure = '';
							personResolutionRetry += 1;
						}}>Try again</button
					>
				{:else if requestedPersonId}
					<strong
						>{resolvingPersonKey ? 'Opening conversation…' : 'Loading your conversations…'}</strong
					>
				{:else}
					<Users size={24} strokeWidth={1.6} aria-hidden="true" />
					<strong>Choose a conversation.</strong>
				{/if}
			</div>
		{/if}
	</section>

	{#if threadRootId}
		<aside class="thread-pane cockpit-pane" aria-label="Thread">
			<header class="thread-head">
				<button type="button" onclick={closeThread} aria-label="Close Thread">
					<ArrowLeft size={15} strokeWidth={2} aria-hidden="true" />
				</button>
				<div>
					<strong>Thread</strong><small>{Math.max(0, visibleThread.length - 1)} replies</small>
				</div>
			</header>
			{#if exactTargetState === 'locating'}
				<div class="exact-target-state" role="status" aria-live="polite">
					Finding the linked message…
				</div>
			{:else if exactTargetState === 'invalid'}
				<div class="exact-target-state unavailable" role="status">
					This linked message address is invalid.
				</div>
			{:else if exactTargetState === 'unavailable'}
				<div class="exact-target-state unavailable" role="status">
					{exactTargetUnavailableCopy(exactTargetUnavailableReason)}
				</div>
			{/if}
			<div
				class="thread-messages"
				bind:this={threadScrollEl}
				use:followChat={{
					key: `${selectedRoomId}:${threadRootId ?? ''}`,
					enabled:
						!focusedMessageId &&
						!(
							exactMessageTarget &&
							exactMessageTarget.roomId === selectedRoomId.toLowerCase() &&
							exactMessageTarget.threadRootMessageId === threadRootId
						)
				}}
			>
				{#if threadProjection?.hasMore}
					<button
						type="button"
						class="load-message-page"
						disabled={threadProjection.loadingMore}
						onclick={() => void loadOlderThreadMessages()}
					>
						{threadProjection.loadingMore ? 'Loading…' : 'Load older replies'}
					</button>
				{/if}
				{#each visibleThread as message (message.id)}
					<RoomMessage
						{message}
						{companyId}
						reactions={reactions.summaryFor(message.id)}
						onreact={ownerAccess && message.id > 0
							? (emoji, on) => void reactions.react(message.id, emoji, on)
							: undefined}
						presentation={actorMessagesById.get(String(message.id))}
						hrefFor={(attachment) =>
							`/api/companies/${encodeURIComponent(companyId)}/attachments/${encodeURIComponent(attachment.uploadId)}`}
						author={message.id < 0 ? 'You' : actorName(message.from_actor)}
						isYou={message.id < 0 || message.from_actor === currentActorId}
						isAgent={actorIsAgent(message.from_actor)}
						targeted={(exactTargetState === 'found' &&
							exactMessageTarget?.messageId === message.id) ||
							focusedMessageId === message.id}
						focusKey={focusedMessageId
							? `search:${selectedRoomId}:${focusedMessageId}`
							: exactTargetKey}
						mentions={mentionsFor(message.id)}
						nameFor={actorName}
						thread
						canEdit={message.id > 0 && !message.deleted_at && message.from_actor === currentActorId}
						canDelete={mayDelete(message)}
						historyOpen={historyMessageId === message.id}
						revisions={historyMessageId === message.id ? (revisionProjection?.revisions ?? []) : []}
						historyStatus={historyMessageId === message.id
							? (revisionProjection?.status ?? 'unknown')
							: 'live'}
						historyFailure={historyMessageId === message.id
							? revisionProjection?.failure
								? failureSentence(revisionProjection.failure, 'Edit history couldn’t load.')
								: ''
							: ''}
						historyHasMore={historyMessageId === message.id && Boolean(revisionProjection?.hasMore)}
						historyLoadingMore={historyMessageId === message.id &&
							Boolean(revisionProjection?.loadingMore)}
						onhistory={message.id > 0 && message.edited_at ? () => toggleHistory(message.id) : null}
						onloadhistory={() => void revisionProjection?.loadMore()}
						onedit={(body, commandId) => saveMessageEdit(message, body, commandId)}
						ondelete={(commandId) => removeMessage(message, commandId)}
					/>
				{:else}
					{#if threadProjection?.status === 'unknown'}
						<Skeleton label="Loading thread" variant="messages" count={2} />
					{:else}
						<FailureNotice
							error={threadProjection?.failure ?? new Error('This thread couldn’t be loaded.')}
							subject="this thread"
							variant="block"
							onretry={threadProjection ? () => threadProjection?.refresh() : undefined}
						/>
					{/if}
				{/each}
			</div>
			{@render messageComposer()}
		</aside>
	{/if}
</div>

{#snippet messageComposer()}
	<form class="room-composer" onsubmit={submitMessage}>
		<Composer
			bind:value={composer}
			bind:files={composerFiles}
			bind:selectedSkills={composerSkills}
			options={[
				...(threadRootId === null ? options.value : []),
				...(ownerAccess
					? referenceOptions(
							companyId,
							attention.view?.workGraph?.work ?? [],
							cockpitProjection.view?.goals ?? [],
							people,
							libraryDocuments.documents
						)
					: [])
			]}
			interruptible={!!leadTurn && accountableDirect && threadRootId === null}
			bind:interrupt={interruptNext}
			actionLabel={retryCommandId && retryBody === composer.trim() ? 'Retry send' : 'Send'}
			disabled={sending || !online || (selectedRoom?.kind === 'direct' && !canExtendDirect)}
			minlength={1}
			allowAttachments={accountableDirect && threadRootId === null}
			placeholder={threadRootId
				? 'Reply in this Thread…'
				: directPartner && actorIsAgent(directPartner)
					? `Message ${roomLabel(selectedRoomId)}…`
					: `Message ${roomLabel(selectedRoomId)}… @ asks someone to reply`}
			ariaLabel={threadRootId ? 'Thread reply' : 'Conversation message'}
		>
			{#snippet controls()}
				{#if directPerson && actorIsAgent(directPerson.actor_id) && threadRootId === null}
					<IntelligenceChip
						{companyId}
						actorId={directPerson.actor_id}
						name={directPerson.display}
					/>
				{/if}
			{/snippet}
		</Composer>
		{#if answerableMention}
			<label
				class="resolve-mention"
				title={answerableMention.expected_response
					? `Your reply answers the request: ${answerableMention.expected_response}`
					: 'Your reply answers the request addressed to you in this Thread.'}
			>
				<input type="checkbox" bind:checked={resolvesMention} />
				Answers the request
			</label>
		{/if}
		{#if sendError}<p class="room-send-error" role="alert">{sendError}</p>{/if}
		{#if sendNotice}<p class="room-send-notice" role="status">{sendNotice}</p>{/if}
		{#if accountableDirect && composerFiles.length}<p class="room-draft-state">
				{composerFiles.length} attachment{composerFiles.length === 1 ? '' : 's'} ready to send.
			</p>{/if}
		{#if !online}<p class="room-draft-state">Draft kept locally until you reconnect.</p>{/if}
	</form>
{/snippet}

<style>
	.room-pinned {
		flex: none;
		border-bottom: 1px solid var(--border-soft);
		font-size: var(--t-label);
	}
	.room-pinned summary {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 16px;
		color: var(--text-secondary);
		font-weight: 500;
		cursor: pointer;
	}
	.room-pinned a {
		display: grid;
		gap: 1px;
		padding: 6px 16px;
		color: var(--text-secondary);
		text-decoration: none;
	}
	.room-pinned a:hover {
		background: var(--wash-hover);
	}
	.room-pinned strong {
		color: var(--ink);
		font-weight: 500;
	}
	.room-pinned span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.jump-anchor {
		position: relative;
		height: 0;
	}
	.jump-latest {
		position: absolute;
		z-index: 2;
		left: 50%;
		bottom: 8px;
		transform: translateX(-50%);
		padding: 4px 12px;
		border: 1px solid var(--border-strong);
		border-radius: 999px;
		background: var(--surface-raised);
		box-shadow: var(--shadow-soft);
		color: var(--ink);
		font: 500 var(--t-label) var(--font-ui);
		cursor: pointer;
	}
	.jump-latest:hover {
		background: var(--surface-hover);
	}
	.chat-attention-strip {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 16px;
		border-top: 1px solid var(--border);
		color: var(--text-secondary);
		text-decoration: none;
		font-size: var(--t-label);
		flex: none;
	}
	.chat-attention-strip strong {
		flex: 1;
		font-weight: 500;
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		color: var(--ink);
	}
	.chat-attention-strip:hover {
		background: var(--surface-alt);
	}

	.room-contact {
		display: inline-flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
		padding: 0;
		border: 0;
		background: transparent;
		color: var(--ink);
		font: inherit;
	}
	.person-avatar {
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
		flex: none;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-alt);
		font-size: var(--t-label);
	}
	.room-head :global(.room-manage-trigger) {
		width: 30px;
		height: 30px;
		padding: 0;
		justify-content: center;
	}
	.room-head :global(.room-manage-trigger span) {
		position: absolute;
		width: 1px;
		height: 1px;
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}
	.room-send-notice {
		margin: 6px 0 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}

	.resolve-mention {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		margin: 6px 2px 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
		white-space: nowrap;
		cursor: pointer;
	}
	.rooms-screen {
		width: 100%;
		height: 100%;
		display: grid;
		grid-template-columns: minmax(0, 1fr);
		gap: var(--pane-gap);
		overflow: hidden;
	}

	.rooms-screen.thread-selected {
		grid-template-columns: minmax(280px, 1fr) minmax(280px, 360px);
	}

	.room-conversation,
	.thread-pane {
		min-width: 0;
		min-height: 0;
		overflow: hidden;
	}

	.room-conversation,
	.thread-pane {
		display: flex;
		flex-direction: column;
	}

	.room-head,
	.thread-head {
		display: flex;
		align-items: center;
	}

	.thread-head button {
		width: 28px;
		height: 28px;
		display: grid;
		place-items: center;
		flex: 0 0 auto;
		padding: 0;
		border: 1px solid transparent;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		cursor: pointer;
	}

	.thread-head button:hover {
		border-color: var(--border-strong);
		background: var(--surface-alt);
		color: var(--ink);
	}

	.message-search-field {
		display: flex;
		align-items: center;
		gap: 7px;
		flex: 0 0 auto;
		border-bottom: 1px solid var(--border);
		background: var(--surface-alt);
		color: var(--text-tertiary);
	}

	.message-search-field {
		padding: 7px 14px;
	}

	.message-search-field input {
		min-width: 0;
		width: 100%;
		padding: 0;
		border: 0;
		outline: 0;
		background: transparent;
		font: inherit;
		font-size: var(--t-label);
		color: var(--ink);
	}

	.message-search-field:focus-within {
		box-shadow: inset 0 -2px 0 color-mix(in srgb, var(--intent-conversation) 42%, transparent);
		color: var(--intent-conversation);
	}

	.load-message-page {
		flex: 0 0 auto;
		padding: 8px 12px;
		border: 0;
		border-top: 1px solid var(--border);
		background: var(--surface-alt);
		font-size: var(--t-label);
		font-weight: 600;
		color: var(--intent-conversation);
		cursor: pointer;
	}

	.load-message-page {
		width: 100%;
		border-bottom: 1px solid var(--border);
	}

	.room-empty {
		margin: 0;
		padding: 14px 12px;
		font-size: var(--t-body);
		color: var(--text-secondary);
	}

	.room-head,
	.thread-head {
		min-height: var(--pane-head-h);
		gap: 9px;
		padding: 9px 14px;
		border-bottom: 1px solid var(--border);
		background: color-mix(in srgb, var(--highlight) 48%, transparent);
		box-shadow: var(--bevel-subtle);
	}

	.message-search-toggle {
		width: 30px;
		height: 30px;
		display: grid;
		place-items: center;
		flex: 0 0 auto;
		padding: 0;
		border: 1px solid transparent;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-secondary);
		cursor: pointer;
	}

	.message-search-toggle:hover,
	.message-search-toggle:focus-visible,
	.message-search-toggle.active {
		border-color: color-mix(in srgb, var(--intent-conversation) 24%, var(--border));
		background: var(--intent-conversation-soft);
		color: var(--intent-conversation);
	}

	.message-search-toggle:focus-visible {
		outline: 2px solid color-mix(in srgb, var(--intent-conversation) 30%, transparent);
		outline-offset: 2px;
	}

	.room-head-copy,
	.thread-head > div {
		min-width: 0;
		flex: 1;
	}

	.room-head-copy strong,
	.room-head-copy small,
	.thread-head strong,
	.thread-head small {
		display: block;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.room-head-copy strong,
	.thread-head strong {
		font-size: var(--t-head);
		font-weight: 600;
	}

	.room-head-copy small,
	.thread-head small {
		margin-top: 2px;
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}

	.room-transport {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		flex: 0 0 auto;
		padding: 3px 7px;
		border: 1px solid var(--border);
		border-radius: var(--radius-control);
		background: var(--surface-alt);
		font: 500 var(--t-label) var(--font-ui);
		color: var(--text-tertiary);
		animation: bridge-disclosure-in var(--motion-disclosure) var(--ease-out) both;
	}

	.room-transport.degraded {
		border-color: color-mix(in srgb, var(--intent-authority) 24%, var(--border));
		background: var(--intent-authority-soft);
		color: var(--intent-authority);
	}

	.degraded-strip {
		flex: 0 0 auto;
		padding: 6px 14px;
		border-bottom: 1px solid color-mix(in srgb, var(--intent-authority) 20%, var(--border));
		background: color-mix(in srgb, var(--intent-authority-soft) 58%, var(--surface));
		font-size: var(--t-label);
		color: var(--intent-authority);
	}

	.exact-target-state {
		flex: 0 0 auto;
		padding: 7px 13px;
		border-bottom: 1px solid color-mix(in srgb, var(--intent-direction) 22%, var(--border));
		background: color-mix(in srgb, var(--intent-direction-soft) 66%, var(--surface));
		font-size: var(--t-label);
		color: var(--intent-direction);
	}

	.exact-target-state.unavailable {
		border-bottom-color: color-mix(in srgb, var(--intent-authority) 22%, var(--border));
		background: color-mix(in srgb, var(--intent-authority-soft) 56%, var(--surface));
		color: var(--intent-authority);
	}

	.room-message-list,
	.thread-messages {
		min-height: 0;
		flex: 1;
		overflow: auto;
		overflow-anchor: none;
		overscroll-behavior: contain;
		background: var(--surface-pane);
	}

	/* Room for replies to grow into beneath a message that has just been sent (follow-chat). */
	.room-message-list::after {
		content: '';
		display: block;
		height: var(--chat-anchor-room, 0px);
	}

	.message-search-results {
		min-height: 100%;
	}

	.message-search-result {
		width: 100%;
		display: grid;
		grid-template-columns: minmax(110px, 0.28fr) minmax(0, 1fr);
		gap: 14px;
		padding: 11px 14px;
		border: 0;
		border-bottom: 1px solid var(--border);
		background: transparent;
		text-align: left;
		color: var(--ink);
		cursor: pointer;
	}

	.message-search-result:hover,
	.message-search-result:focus-visible {
		background: var(--intent-conversation-soft);
	}

	.message-search-result:focus-visible {
		outline: 2px solid color-mix(in srgb, var(--intent-conversation) 32%, transparent);
		outline-offset: -2px;
	}

	.message-search-result span,
	.message-search-result strong,
	.message-search-result small {
		display: block;
		min-width: 0;
	}

	.message-search-result strong,
	.message-search-result small {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.message-search-result strong {
		font-size: var(--t-label);
	}

	.message-search-result small {
		margin-top: 3px;
		font-size: var(--t-label);
		color: var(--text-tertiary);
	}

	.message-search-result p {
		margin: 0;
		display: -webkit-box;
		overflow: hidden;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		font-size: var(--t-body);
		line-height: 1.45;
	}

	.room-day,
	.unread-rule {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 9px 14px 5px;
		font: 500 var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
		color: var(--text-tertiary);
	}

	.room-day::before,
	.room-day::after,
	.unread-rule::before,
	.unread-rule::after {
		height: 1px;
		flex: 1;
		background: var(--border);
		content: '';
	}

	/* The same day divider as the Exec rail (design/chat.css): an outlined label on a fading rule. */
	.room-day::before {
		background: linear-gradient(90deg, transparent, var(--border-strong));
	}
	.room-day::after {
		background: linear-gradient(90deg, var(--border-strong), transparent);
	}
	.room-day span {
		padding: 1px 9px;
		border: 1px solid var(--border);
		border-radius: 999px;
		background: var(--surface-pane);
	}

	.unread-rule {
		color: var(--intent-conversation);
	}

	.unread-rule::before,
	.unread-rule::after {
		background: color-mix(in srgb, var(--intent-conversation) 30%, transparent);
	}

	.conversation-empty {
		min-height: 100%;
		display: grid;
		place-content: center;
		justify-items: center;
		padding: 28px;
		text-align: center;
		color: var(--text-secondary);
	}

	.conversation-empty strong {
		font-size: var(--t-head);
		color: var(--ink);
	}

	.conversation-empty p {
		max-width: 420px;
		margin: 6px 0 0;
		font-size: var(--t-body);
	}

	.conversation-empty button {
		margin-top: 12px;
		padding: 6px 10px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface);
		box-shadow: var(--control-depth);
		cursor: pointer;
	}

	.room-composer {
		flex: 0 0 auto;
		padding: 9px 16px max(11px, env(safe-area-inset-bottom));
		background: color-mix(in srgb, var(--highlight) 66%, transparent);
		box-shadow: 0 -10px 30px rgba(43, 51, 66, 0.035);
	}

	.room-send-error,
	.room-draft-state {
		margin: 6px 2px 0;
		font-size: var(--t-label);
		color: var(--state-danger);
	}

	.room-draft-state {
		color: var(--intent-authority);
	}

	.thread-pane {
		border-left: 1px solid var(--border);
		background: var(--surface-rail);
	}

	@media (max-width: 1180px) {
		.rooms-screen.thread-selected {
			grid-template-columns: minmax(250px, 1fr) minmax(250px, 330px);
		}
		.rooms-screen {
			grid-template-columns: minmax(0, 1fr);
		}
	}

	@container conversation (max-width: 650px) {
		.room-head {
			flex-wrap: wrap;
		}
		/* The title truncates; its actions stay on the same row. */
		.room-head-copy {
			flex: 1 1 0;
		}
		.room-contact {
			max-width: 100%;
		}
		.room-transport {
			margin-left: auto;
		}

		.rooms-screen.thread-selected {
			grid-template-columns: minmax(0, 1fr);
		}
		.rooms-screen.thread-selected .room-conversation {
			display: none;
		}
	}

	@media (max-width: 760px) {
		.rooms-screen,
		.rooms-screen.thread-selected {
			grid-template-columns: 1fr;
			grid-template-rows: minmax(0, 1fr);
		}

		.room-conversation,
		.thread-pane {
			display: none;
		}

		.rooms-screen.room-selected .room-conversation {
			display: flex;
		}

		.rooms-screen.thread-selected .room-conversation {
			display: none;
		}

		.rooms-screen.thread-selected .thread-pane {
			display: flex;
		}

		.room-head,
		.thread-head {
			padding-inline: 9px;
		}

		.message-search-toggle {
			width: 40px;
			height: 40px;
		}

		.message-search-result {
			grid-template-columns: 1fr;
			gap: 5px;
			padding: 12px;
		}

		.room-composer {
			padding-inline: 8px;
		}

		.room-message-list,
		.thread-messages {
			scroll-padding-bottom: 92px;
		}
	}
</style>
