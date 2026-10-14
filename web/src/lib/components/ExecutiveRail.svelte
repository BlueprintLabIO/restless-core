<script lang="ts" module>
	/* Files, a quote and chosen skills wait with the conversation they were meant for, across topic
	 * switches, as a Room keeps its files per draft. Files live only in memory. */
	const heldExtras = new Map<
		string,
		{
			files: File[];
			skills: string[];
			quoting: { author: string; excerpt: string; answer?: boolean } | null;
		}
	>();
</script>

<script lang="ts">
	import {
		answerIndex,
		excerptOf,
		openQuestionIndex as findOpenQuestion
	} from '$lib/model/open-question';
	import { untrack } from 'svelte';
	import { failureSentence } from '$lib/model/failure';
	import Skeleton from '$lib/ui/feedback/Skeleton.svelte';
	/* One contextual conversation rail. It normally belongs to the Exec; while
	 * the owner focuses a review it belongs to that Work's accountable lead.
	 * The rail stays mounted and takes real space rather than nesting another
	 * chat inside the outcome surface. */

	import { followChat } from '$lib/actions/follow-chat';
	import IntelligenceChip from './IntelligenceChip.svelte';
	import ActionMenu from '$lib/ui/controls/ActionMenu.svelte';
	import { copyText } from '$lib/model/copy-text';
	import { conversationMarkdown, saveConversationToLibrary } from '$lib/model/conversation-export';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import Check from '@lucide/svelte/icons/check';
	import SquarePen from '@lucide/svelte/icons/square-pen';
	import Maximize2 from '@lucide/svelte/icons/maximize-2';
	import PanelRightClose from '@lucide/svelte/icons/panel-right-close';
	import Reply from '@lucide/svelte/icons/reply';
	import X from '@lucide/svelte/icons/x';
	import { SvelteDate } from 'svelte/reactivity';
	import { tick } from 'svelte';
	import Plus from '@lucide/svelte/icons/plus';
	import RotateCw from '@lucide/svelte/icons/rotate-cw';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import Composer from '$lib/primitives/Composer.svelte';
	import ConversationHistoryTools from '$lib/primitives/ConversationHistoryTools.svelte';
	import ConversationMessage from '$lib/primitives/ConversationMessage.svelte';
	import ConversationTurnDock from '$lib/primitives/ConversationTurnDock.svelte';
	import HoldApprove from '$lib/ui/controls/HoldApprove.svelte';
	import MatrixGlyph, { GLYPHS } from '$lib/ui/glyph/MatrixGlyph.svelte';
	import SemanticMark from '$lib/ui/glyph/SemanticMark.svelte';
	import {
		turnWorking,
		type ActiveAgentTurn,
		type QuerySourceStatus
	} from '$lib/model/queries.svelte';
	import type { OutcomeStandard } from '$lib/model/company';
	import type { ThreadMessage } from '$lib/model/view';
	import { execCommandsFor, runExecCommand } from '$lib/model/skills';
	import { PIN, reactionsQuery, roomPinsQuery } from '$lib/model/reactions.svelte';
	import { workDraftFrom, askExec } from '$lib/model/ask-exec';
	import Pin from '@lucide/svelte/icons/pin';
	import { recentDirectConversationsQuery } from '$lib/model/room-queries.svelte';
	import {
		conversationUnread,
		deleteRoomMessage,
		markRoomRead,
		messagePreview,
		searchRoomMessages
	} from '$lib/model/rooms';
	import { goto } from '$app/navigation';
	import {
		appendCommandLog,
		composerOptions,
		readCommandLog,
		type CommandRecord,
		readDraft,
		readDraftExtras,
		writeDraft,
		writeDraftExtras
	} from '$lib/model/composer-options.svelte';
	import { expectRemoval } from '$lib/model/steady';
	import { agentExchangesQuery, type AgentExchange } from '$lib/model/exchanges.svelte';
	import HandoffGroup from '$lib/primitives/HandoffGroup.svelte';
	import AppRequestCards from './AppRequestCards.svelte';
	import { fetchSetAside } from '$lib/model/attention';
	import WorkInFlight from './WorkInFlight.svelte';
	import MyRequests from './MyRequests.svelte';
	import { holdQueries } from '$lib/model/hold-queries.svelte';
	import { MAX_UPLOAD_BYTES, MAX_UPLOAD_FILES } from '$lib/model/attachments';

	let {
		messages = [],
		participantName = 'Exec',
		participantId = 'exec',
		participantRole = 'Executive',
		turn = null,
		companyId,
		membershipRole,
		connected = false,
		connectionStatus = 'unknown',
		conversationStatus = 'unknown',
		conversationFailed = false,
		onrefreshConversation = null,
		needsProvider = false,
		providerLabel = 'Connect intelligence',
		contextLabel = 'Current screen',
		contextLabelFor = null,
		contextKind = '',
		focusAfterMessageId = 0,
		focusStartedAt = null,
		newFocusAvailable = false,
		open = true,
		onask = null,
		review = null,
		workContext = null,
		topicLabel = '',
		topics = [],
		currentTopicKey = 'general',
		ontopic = null,
		viewerActorId = 'owner',
		references = [],
		onclose = null,
		focusRequest = 0,
		draftRequest = null,
		wide = false,
		focusMessage = 0,
		uploadProgress = null,
		onstopturn = null,
		onretryturn = null,
		ambientTurn = null,
		hasEarlier = false,
		onloadearlier = null
	}: {
		messages?: ThreadMessage[];
		participantName?: string;
		participantId?: string;
		participantRole?: string;
		turn?: ActiveAgentTurn | null;
		companyId: string;
		membershipRole: string;
		/** Whether this actor has a usable conversation route in the latest cockpit view. */
		connected?: boolean;
		/** Separates confirmed unavailability from an unknown or failed status read. */
		connectionStatus?: 'unknown' | 'error' | 'unavailable' | 'available';
		conversationStatus?: QuerySourceStatus;
		conversationFailed?: boolean;
		onrefreshConversation?: (() => void) | null;
		needsProvider?: boolean;
		providerLabel?: string;
		/** The thing on screen a message will be linked to, by name. */
		contextLabel?: string;
		/** What the screen a sent message links to is called, so the message can name it. */
		contextLabelFor?: ((path: string) => string) | null;
		/** Which surface it is on: work, library, people, company or inbox. */
		contextKind?: string;
		focusAfterMessageId?: number;
		focusStartedAt?: string | null;
		newFocusAvailable?: boolean;
		open?: boolean;
		/** Returns delivery feedback. Unwired ordinary chat is inert. */
		onask?:
			| ((
					text: string,
					files: File[],
					includeContext: boolean,
					newFocus: boolean,
					interrupt: boolean,
					outcomeStandard?: OutcomeStandard,
					skills?: string[],
					retryCommandId?: string
			  ) => Promise<{ error?: string; notice?: string; retryCommandId?: string }>)
			| null;
		review?: {
			/** The Inbox item under review; a change clears the previous review's note. */
			id: string;
			onback: () => void;
			ondecide: (
				decision: 'accept' | 'request_changes',
				feedback: string
			) => Promise<string | null>;
		} | null;
		workContext?: { onback: () => void } | null;
		/** What this conversation is about: General, or one piece of Work. */
		topicLabel?: string;
		topics?: {
			key: string;
			label: string;
			hint: string;
			topic: { actorId: string; workId?: string } | null;
		}[];
		currentTopicKey?: string;
		ontopic?: ((topic: { actorId: string; workId?: string } | null) => void) | null;
		viewerActorId?: string;
		/** Work, Goals and people the composer offers after `#` and `@`. */
		references?: import('$lib/model/skills').ComposerOption[];
		/** Hides the rail; the topbar's Exec button brings it back. */
		onclose?: (() => void) | null;
		/** Bumped by a page that hands the owner to this conversation. */
		focusRequest?: number;
		/** A draft another surface prepared for this conversation. */
		draftRequest?: { text: string; key: number } | null;
		/** The same conversation at full width, in People. */
		wide?: boolean;
		/** A message a link points at; the conversation scrolls to it. */
		focusMessage?: number;
		/** How much of a sending message's files has gone, 0 to 1, while they go. */
		uploadProgress?: number | null;
		/** Stops the reply to your message without sending anything; resolves to an error. */
		onstopturn?: (() => Promise<string | null>) | null;
		/** Answers your message again after its reply failed; resolves to an error. */
		onretryturn?: (() => Promise<string | null>) | null;
		/** What the agent is doing when no person's message started it (a schedule, a result). */
		ambientTurn?: ActiveAgentTurn | null;
		/** The server holds messages older than the ones loaded. */
		hasEarlier?: boolean;
		/** Pages in earlier messages; resolves to how many arrived. */
		onloadearlier?: (() => Promise<number>) | null;
	} = $props();
	let focusedFor = 0;
	$effect(() => {
		if (!focusMessage || focusMessage === focusedFor) return;
		const target = messages.find((message) => messageNumericId(message.id) === focusMessage);
		if (!target) {
			/* Older than the recent end this conversation holds: the room view pages back through
			 * the whole history to it, so a link or search hit never lands on nothing. */
			if (conversationStatus === 'live' && directRoom) {
				focusedFor = focusMessage;
				void goto(
					`/${encodeURIComponent(companyId)}/people?room=${encodeURIComponent(directRoom)}&focus=${focusMessage}`,
					{ replaceState: true, noScroll: true, keepFocus: true }
				);
			}
			return;
		}
		focusedFor = focusMessage;
		requestAnimationFrame(() => jumpToMessage(target.id));
	});
	/* Untracked: reading the key it bumps would re-run this forever. */
	$effect(() => {
		if (focusRequest) untrack(() => (composerFocusKey += 1));
	});
	const reactions = reactionsQuery(
		() => companyId,
		() =>
			messages
				.map((message) => messageNumericId(message.id))
				.filter((id) => id > 0)
				.slice(-60),
		() => viewerActorId
	);

	/* Every person in the company talks to Exec and the team, and decides a review handed to
	 * them; the server checks the review is theirs to decide. */
	const canOperate = $derived(['owner', 'admin', 'member'].includes(membershipRole ?? ''));
	const canDecide = $derived(canOperate);

	/* Pinned messages stay in reach above the conversation, however old: the direct conversation's
	 * pins come as their own list; elsewhere they are found among the messages held. */
	const pinnedMessages = $derived.by(
		(): { id: string; from: ThreadMessage['from']; author: string; text: string }[] => {
			const listed = roomPins?.pins;
			if (listed)
				return listed.map((pin) => ({
					id: String(pin.id),
					from: pin.from_actor === viewerActorId ? 'you' : 'agent',
					author: pin.from_actor === viewerActorId ? 'You' : participantName,
					text: pin.body
				}));
			const ids = reactions.pinned();
			return messages.filter((message) => ids.has(messageNumericId(message.id)));
		}
	);
	function isPinned(id: number): boolean {
		return reactions.pinned().has(id) || pinnedMessages.some((pin) => Number(pin.id) === id);
	}
	let pinsOpen = $state(false);
	function togglePin(message: ThreadMessage) {
		const id = messageNumericId(message.id);
		if (id > 0) void reactions.react(id, PIN, !isPinned(id));
	}
	async function copyLink(message: ThreadMessage) {
		const href = new URL(
			`/${encodeURIComponent(companyId)}/people?person=${encodeURIComponent(participantId)}&focus=${messageNumericId(message.id)}`,
			window.location.origin
		).href;
		try {
			await navigator.clipboard.writeText(href);
			askNotice = 'Link to that message copied.';
		} catch {
			askError = 'The link could not be copied.';
		}
	}
	async function copyConversation() {
		const copied = await copyText(conversationMarkdown(messages, participantName));
		if (copied) askNotice = 'Conversation copied.';
		else askError = 'The conversation could not be copied.';
	}
	let savingConversation = $state(false);
	async function keepConversation() {
		if (savingConversation) return;
		savingConversation = true;
		try {
			await goto(await saveConversationToLibrary(companyId, messages, participantName));
		} catch (cause) {
			askError = failureSentence(cause, 'The conversation was not saved to the Library.');
		} finally {
			savingConversation = false;
		}
	}
	function turnIntoWork(message: ThreadMessage) {
		const draft = workDraftFrom(
			message.from === 'you' ? 'You' : message.author || participantName,
			message.text
		);
		if (participantId === 'exec' && !review && !workContext) handIn(draft);
		else askExec(draft);
	}

	/* Handoffs between agents appear where they happened in time, for the owner and an admin who
	 * runs the company with them; a member sees what is handed to them under "Needs you". */
	const exchanges = agentExchangesQuery(
		() => companyId,
		() => participantId,
		() => open && ['owner', 'admin'].includes(membershipRole)
	);
	/* A finished turn stays for its dock; only one still in progress can be interrupted. */
	const working = $derived(turnWorking(turn));
	/* The message the turn answers, when it is yours: you may retry it after a failure, and the
	 * owner or an admin may stop its reply (a member's message waits its turn). */
	const turnMessage = $derived(
		turn
			? (messages.find(
					(message) =>
						message.from === 'you' && messageNumericId(message.id) === turn.triggerMessageId
				) ?? null)
			: null
	);
	const stopTurn = $derived(
		turnMessage && ['owner', 'admin'].includes(membershipRole) ? onstopturn : null
	);
	const retryTurn = $derived(turnMessage && !turnMessage.readAt ? onretryturn : null);
	let lastTurn: unknown = null;
	$effect(() => {
		const current = turn?.triggerMessageId ?? null;
		if (lastTurn !== null && current === null) void exchanges.refresh();
		lastTurn = current;
	});
	function at(value: Date | string): number {
		return new Date(value).getTime();
	}
	function handoffsBefore(index: number): AgentExchange[] {
		/* Indexes the shown window; the message before it may be hidden. */
		const previous = messages[windowStart + index - 1];
		if (!previous && windowStart + index > 0) return [];
		const after = previous ? at(previous.createdAt) : -Infinity;
		const until = at(messages[windowStart + index].createdAt);
		return exchanges.exchanges.filter(
			(exchange) => at(exchange.created_at) > after && at(exchange.created_at) <= until
		);
	}
	/* Command lines sit where they happened in time, like handoffs; only the Exec conversation has them. */
	function commandsBefore(index: number): CommandRecord[] {
		if (!execConversation) return [];
		const previous = messages[windowStart + index - 1];
		if (!previous && windowStart + index > 0) return [];
		const after = previous ? at(previous.createdAt) : -Infinity;
		const until = at(messages[windowStart + index].createdAt);
		return commandLog.filter((line) => at(line.at) > after && at(line.at) <= until);
	}
	const commandsSinceLast = $derived.by(() => {
		if (!execConversation) return [];
		const last = visibleMessages.at(-1);
		return last ? commandLog.filter((line) => at(line.at) > at(last.createdAt)) : commandLog;
	});
	const handoffsSinceLast = $derived.by(() => {
		const last = visibleMessages.at(-1);
		return last
			? exchanges.exchanges.filter((exchange) => at(exchange.created_at) > at(last.createdAt))
			: exchanges.exchanges;
	});

	/* `$skill` selection and the `/goal` and `/loop` commands. Skills are the
	 * company library; the commands are Restless primitives offered only in the
	 * Exec conversation, where they have one accountable meaning. */
	let composerSkills = $state<string[]>([]);
	const execConversation = $derived(participantId === 'exec' && !review && !workContext);
	/* Each command is offered to whoever the server lets run it: an admin runs loops with the owner. */
	const allowedCommands = $derived(execConversation ? execCommandsFor(membershipRole) : []);
	const options = composerOptions(
		() => companyId,
		() => open && canOperate,
		() => allowedCommands
	);

	/* Returns a notice for commands handled without a message, or send: true. */
	async function runCommand(text: string): ReturnType<typeof runExecCommand> {
		return runExecCommand(companyId, text, allowedCommands);
	}

	/* Day separators, same as the thread — computed from the record, so the
	 * rail and the page group the same way. */
	function dayOf(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		return Number.isNaN(date.getTime()) ? '' : date.toDateString();
	}

	function dayLabel(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		if (Number.isNaN(date.getTime())) return '';
		const today = new SvelteDate();
		if (date.toDateString() === today.toDateString()) return 'Today';
		const yesterday = new SvelteDate();
		yesterday.setDate(today.getDate() - 1);
		if (date.toDateString() === yesterday.toDateString()) return 'Yesterday';
		return date.toLocaleDateString(undefined, { month: 'long', day: 'numeric' });
	}

	function messageDomId(messageId: string): string {
		return `rail-message-${companyId}-${messageId.replaceAll(':', '-')}`;
	}

	async function jumpToMessage(messageId: string) {
		/* A search hit older than what is loaded: page back to it (a bounded number of pages). */
		for (
			let pages = 0;
			pages < 20 && hasEarlier && onloadearlier && !messages.some((m) => m.id === messageId);
			pages++
		) {
			if (!(await onloadearlier())) break;
			await tick();
		}
		const index = messages.findIndex((entry) => entry.id === messageId);
		if (index >= 0 && index < windowStart) await showEarlier(index);
		const message = document.getElementById(messageDomId(messageId));
		if (!message) return;
		scrollEl?.dispatchEvent(new Event('chat-scroll-pause'));
		const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
		message.scrollIntoView({ behavior: reduceMotion ? 'auto' : 'smooth', block: 'center' });
		if (!reduceMotion) {
			message.animate(
				[
					{
						boxShadow:
							'inset 2px 0 0 color-mix(in srgb, var(--intent-conversation) 0%, transparent)'
					},
					{ boxShadow: 'inset 2px 0 0 var(--intent-conversation)' },
					{
						boxShadow:
							'inset 2px 0 0 color-mix(in srgb, var(--intent-conversation) 0%, transparent)'
					}
				],
				{ duration: 900, easing: 'cubic-bezier(0.23, 1, 0.32, 1)' }
			);
		}
	}

	let composer = $state('');
	/* The draft belongs to this company and this recipient, and survives a reload. */
	const draftKey = $derived(`restless:${companyId}:rail-draft:${participantId}:${currentTopicKey}`);
	let loadedDraftKey = '';
	$effect(() => {
		const key = draftKey;
		if (key === loadedDraftKey) return;
		loadedDraftKey = key;
		composer = readDraft(key);
	});
	/* Files, a quote and chosen skills belong to one recipient (and to the conversation before or
	 * after an Inbox focus): each waits with its own conversation and comes back with it. A pending
	 * new conversation or a tapped answer never travels. A reload keeps the quote, the skills and
	 * an uncertain send's key; files cannot survive one, and the rail says so. */
	const carryKey = $derived(`${draftKey}:${review || workContext ? 'focus' : ''}`);
	let carriedFor = $state('');
	function holdExtras(key: string) {
		if (!key) return;
		if (composerFiles.length || composerSkills.length || quoting)
			heldExtras.set(key, {
				files: [...composerFiles],
				skills: [...composerSkills],
				quoting
			});
		else heldExtras.delete(key);
	}
	$effect(() => {
		const key = carryKey;
		if (key === carriedFor) return;
		untrack(() => {
			holdExtras(carriedFor);
			if (pendingAnswer) window.clearTimeout(pendingAnswer.timer);
			pendingAnswer = null;
			newFocusPending = false;
			fieldValues = {};
			const held = heldExtras.get(key);
			const stored = readDraftExtras(key);
			composerFiles = held?.files ?? [];
			composerSkills = held?.skills ?? stored.skills;
			quoting = held ? held.quoting : stored.quoting;
			retryCommandId = stored.commandId;
			retryBody = stored.commandBody;
			if (!held && stored.files.length)
				askNotice = `Attached files do not survive a reload: attach ${stored.files.join(', ')} again.`;
		});
		carriedFor = key;
	});
	$effect(() => () => holdExtras(untrack(() => carriedFor)));
	$effect(() => {
		const key = carryKey;
		const extras = {
			files: composerFiles.map((file) => file.name),
			skills: [...composerSkills],
			quoting,
			commandId: retryCommandId,
			commandBody: retryBody
		};
		if (key !== carriedFor) return;
		writeDraftExtras(key, extras);
	});
	/* An uncertain send keeps its key with the exact text, so resending it is idempotent. */
	let retryCommandId = $state<string | null>(null);
	let retryBody = $state('');
	/* Text handed in from elsewhere (Ask Exec, Turn into Work, Edit) joins what the owner was
	 * writing instead of replacing it; an unsent draft is never discarded. */
	function handIn(text: string) {
		const current = untrack(() => composer);
		const handed = text.trim();
		if (!handed) return;
		if (!current.trim() || current.includes(handed)) composer = current.trim() ? current : text;
		else {
			composer = `${current.trimEnd()}\n\n${text}`;
			askNotice = 'Added below what you were writing.';
		}
		untrack(() => (composerFocusKey += 1));
	}
	/* After the saved draft loads, so a requested draft is not overwritten by it. */
	let appliedDraft = 0;
	$effect(() => {
		if (!draftRequest || draftRequest.key === appliedDraft) return;
		appliedDraft = draftRequest.key;
		const text = draftRequest.text;
		untrack(() => handIn(text));
	});
	$effect(() => {
		const key = draftKey,
			body = composer;
		if (key !== loadedDraftKey) return;
		const timer = setTimeout(() => writeDraft(key, body), 150);
		return () => clearTimeout(timer);
	});
	let composerFiles = $state<File[]>([]);
	/* While the agent is replying, Send queues and Interrupt stops the reply. */
	let interruptNext = $state(false);
	let includeContext = $state(true);
	let contextFlare = $state(0);
	let askError = $state('');
	let askNotice = $state('');
	/* The words of a `/goal` whose goal exists but whose message did not reach Exec: sending the
	 * same words again only delivers the message, so a retry never makes a second goal. */
	let goalCreatedFor = $state('');
	/* A command that stopped a loop or closed a goal can be put back from its notice. */
	let noticeUndo = $state<{ notice: string; run: () => Promise<string> } | null>(null);
	/* A notice is a moment, not a record: it leaves by itself (later when it offers Undo). What a
	 * command did is kept as a line in the transcript instead. The note that a message waits for
	 * Exec stays while that is true. */
	$effect(() => {
		const notice = askNotice;
		if (!notice || notice === leftForLater) return;
		const timer = setTimeout(
			() => {
				if (askNotice === notice) askNotice = '';
			},
			noticeUndo?.notice === notice ? 20_000 : 8_000
		);
		return () => clearTimeout(timer);
	});
	const commandLogKey = $derived(`restless:exec-commands:${companyId}`);
	let commandLog = $state<CommandRecord[]>([]);
	$effect(() => {
		commandLog = readCommandLog(commandLogKey);
	});
	function recordCommand(command: string, result: string) {
		commandLog = appendCommandLog(commandLogKey, commandLog, command, result);
	}
	async function undoCommand() {
		const undo = noticeUndo;
		if (!undo || undo.notice !== askNotice) return;
		noticeUndo = null;
		try {
			askNotice = await undo.run();
			recordCommand('Undo', askNotice);
		} catch (cause) {
			askError = failureSentence(cause, 'That could not be undone.');
		}
	}
	let reviewError = $state('');
	let reviewFeedback = $state('');
	let deciding = $state(false);
	/* A note belongs to one review: opening a different one starts blank, and
	 * an error never outlives the review it was about. Leaving and coming back
	 * to the same review keeps an unsent note. */
	let noteFor = '';
	let reviewShown = '';
	$effect(() => {
		const id = review?.id ?? '';
		if (id === reviewShown) return;
		reviewShown = id;
		reviewError = '';
		if (id && id !== noteFor) {
			noteFor = id;
			reviewFeedback = '';
		}
	});
	let scrollEl = $state<HTMLDivElement | undefined>();
	let scrollReset = $state(0);
	/* How many messages there were when the reader scrolled up; null while
	 * the transcript follows its end. */
	let awayFrom = $state<number | null>(null);
	let newFocusPending = $state(false);
	let pendingFocusAfterMessageId = $state(0);
	let composerFocusKey = $state(0);

	/* What the owner has already seen here, per company and recipient, so new
	 * replies start under a "New" rule. Read once per conversation; marked as
	 * seen while the rail is open. */
	const seenKey = $derived(`restless:${companyId}:rail-seen:${participantId}:${currentTopicKey}`);
	let seenThrough = $state<number | null>(null);
	let seenLoadedFor = '';
	$effect(() => {
		const key = seenKey;
		if (key === seenLoadedFor || !messages.length) return;
		seenLoadedFor = key;
		let stored = 0;
		try {
			stored = Number(localStorage.getItem(key) ?? 0) || 0;
		} catch {
			/* Without storage there is no rule; the conversation still reads. */
		}
		seenThrough = stored;
	});
	$effect(() => {
		const key = seenKey,
			latest = messages.reduce((max, m) => Math.max(max, messageNumericId(m.id)), 0);
		if (!open || !latest || key !== seenLoadedFor) return;
		try {
			localStorage.setItem(key, String(latest));
		} catch {
			/* See above. */
		}
	});
	function unreadRuleBefore(index: number): boolean {
		if (!seenThrough) return false;
		const current = messages[windowStart + index];
		if (!current || current.from === 'you' || messageNumericId(current.id) <= seenThrough)
			return false;
		const previous = messages[windowStart + index - 1];
		return !previous || messageNumericId(previous.id) <= seenThrough;
	}
	function shortTime(value: Date | string): string {
		const date = value instanceof Date ? value : new Date(value);
		return Number.isNaN(date.getTime())
			? ''
			: date.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
	}
	/* The agent's open question, when its latest word asks for one. Replying
	 * answers it; the Inbox lists the same question until then. */
	/* A refresh that fails while the last messages are still on screen is usually a moment: a
	 * restart or a network blip. Say nothing for a few seconds, then one quiet line, and keep
	 * retrying on its own; it clears as soon as a refresh succeeds. */
	const RECONNECT_GRACE_MS = 8_000;
	const RECONNECT_RETRY_MS = 10_000;
	const troubled = $derived(
		(!needsProvider && connectionStatus === 'error') ||
			(conversationFailed && conversationStatus === 'stale')
	);
	let reconnecting = $state(false);
	$effect(() => {
		if (!troubled) {
			reconnecting = false;
			return;
		}
		const grace = window.setTimeout(() => (reconnecting = true), RECONNECT_GRACE_MS);
		const retry = window.setInterval(() => onrefreshConversation?.(), RECONNECT_RETRY_MS);
		return () => {
			window.clearTimeout(grace);
			window.clearInterval(retry);
		};
	});
	function retryNow() {
		onrefreshConversation?.();
	}

	/* The newest ask still unanswered: only a reply quoting it, or a newer ask, closes it. */
	/* A question the owner set aside from the Inbox is closed here too, without a message. */
	let setAsideIds = $state<Set<string>>(new Set());
	$effect(() => {
		void messages.length;
		if (membershipRole !== 'owner') return;
		void fetchSetAside(companyId)
			.then((items) => (setAsideIds = new Set(items.map((item) => item.item_id))))
			.catch(() => {});
	});
	const openQuestionIndex = $derived.by(() => {
		const index = findOpenQuestion(visibleMessages, participantName);
		const message = index >= 0 ? visibleMessages[index] : null;
		return message && setAsideIds.has(`conversation:${participantId}:${message.id}`) ? -1 : index;
	});
	const openQuestion = $derived.by(() =>
		openQuestionIndex >= 0
			? (visibleMessages[openQuestionIndex].intent?.ownerNeed?.trim() ?? '')
			: ''
	);
	/* A receipt under your latest message: sent, or seen when the agent's turn
	 * consumed it without replying. While it works, the turn dock says so;
	 * once it replies, the reply is the receipt. */
	const directRooms = $derived(recentDirectConversationsQuery(companyId, () => open && canOperate));
	const directRow = $derived(
		directRooms.conversations.find((entry) => entry.person_actor_id === participantId) ?? null
	);
	const directRoom = $derived(directRow?.room_id ?? '');
	/* Reading here is reading on every device: an open rail on the direct conversation advances
	 * the server read cursor through the room's latest message. A reply newer than the list row
	 * first refreshes the row, so the cursor only ever names a message the server listed. */
	const isDirectTopic = $derived(
		!review && !workContext && (!currentTopicKey.includes(':') || currentTopicKey.endsWith(':'))
	);
	const roomPins = roomPinsQuery(
		() => companyId,
		() => directRoom,
		() => open && isDirectTopic
	);
	let readMarking = '';
	$effect(() => {
		const row = directRow;
		const latest = messages.reduce((max, m) => Math.max(max, messageNumericId(m.id)), 0);
		if (!open || !row || !isDirectTopic || !viewerActorId) return;
		if (latest > row.last_message_id) {
			void directRooms.refresh();
			return;
		}
		if (!conversationUnread(row, viewerActorId)) return;
		const key = `${companyId}:${row.room_id}:${row.last_message_id}`;
		if (readMarking === key) return;
		readMarking = key;
		void markRoomRead(companyId, row.room_id, row.last_message_id)
			.then(() => directRooms.refresh())
			.catch(() => {
				/* Read state is recoverable; the next change tries again. */
				if (readMarking === key) readMarking = '';
			});
	});
	/* Search reaches the whole conversation on the server, not only what is loaded. */
	async function searchAll(text: string, signal: AbortSignal) {
		const page = await searchRoomMessages(companyId, text, null, 20, signal, directRoom);
		return page.messages.map((hit) => ({
			id: String(hit.id),
			snippet: hit.snippet,
			fromViewer: hit.from_actor === viewerActorId
		}));
	}
	/* Another conversation in the switcher with something new since you read it. */
	function topicHasNew(entry: { key: string; topic: { actorId: string; workId?: string } | null }) {
		if (entry.key === currentTopicKey || entry.topic?.workId) return false;
		const actor = entry.topic?.actorId ?? 'exec';
		const conversation = directRooms.conversations.find((c) => c.person_actor_id === actor);
		return !!conversation && conversationUnread(conversation, viewerActorId);
	}
	const anyTopicNew = $derived(topics.some((entry) => topicHasNew(entry)));
	/* Until the agent picks it up, your last message can be taken back: retract
	 * removes it, edit returns its words to the composer. */
	const retractable = $derived.by(() => {
		const last = visibleMessages.at(-1);
		return !!last &&
			last.from === 'you' &&
			!last.readAt &&
			!working &&
			!!directRoom &&
			messageNumericId(last.id) > 0
			? last
			: null;
	});
	let retracting = $state(false);
	async function retract(restore: boolean) {
		const last = retractable;
		if (!last || retracting) return;
		retracting = true;
		try {
			expectRemoval(last.id);
			await deleteRoomMessage(
				companyId,
				directRoom,
				messageNumericId(last.id),
				crypto.randomUUID()
			);
			if (restore) handIn(last.text);
			onrefreshConversation?.();
		} catch (cause) {
			askError = failureSentence(
				cause,
				'That message could not be taken back; it may already be in progress.'
			);
		} finally {
			retracting = false;
		}
	}
	/* Once it has been read (or work on it started), the last message can no longer be taken
	 * back, but it can still be corrected: the composer opens with its words to edit, under a
	 * line saying this replaces it, so the agent reads the correction as superseding it. */
	function correct() {
		const last = visibleMessages.at(-1);
		if (!last || last.from !== 'you') return;
		composer = `Correction, replacing my last message:\n\n${last.text}`;
		composerFocusKey += 1;
	}
	const receipt = $derived.by(() => {
		const last = visibleMessages.at(-1);
		if (!last || last.from !== 'you' || working || last.id.startsWith('optimistic:')) return '';
		return last.readAt ? `Seen by ${participantName}` : 'Sent';
	});
	const openReplies = $derived.by(() =>
		openQuestion ? (visibleMessages[openQuestionIndex]?.intent?.ownerReplies ?? []).slice(0, 3) : []
	);
	/* A likely answer sends on tap, as a message would, after a moment in which it can be taken
	 * back. The composer stays the owner's: a draft there is never replaced. */
	const UNDO_MS = 2_000;
	let pendingAnswer = $state<{ text: string; timer: number } | null>(null);
	function tapReply(text: string) {
		if (pendingAnswer) window.clearTimeout(pendingAnswer.timer);
		const timer = window.setTimeout(() => {
			pendingAnswer = null;
			void sendAnswer(text);
		}, UNDO_MS);
		pendingAnswer = { text, timer };
	}
	function undoReply() {
		if (!pendingAnswer) return;
		window.clearTimeout(pendingAnswer.timer);
		pendingAnswer = null;
	}
	$effect(() => () => {
		if (pendingAnswer) window.clearTimeout(pendingAnswer.timer);
	});
	/* An answer travels as an ordinary reply quoting the question, so the thread reads the same
	 * whether it was tapped, filled in or typed. */
	async function sendAnswer(text: string) {
		const answer = text.trim();
		if (!answer || sending || deciding || !onask) return;
		sending = true;
		askError = '';
		askNotice = '';
		const outgoing = `> ${participantName}: ${excerptOf(openQuestion)}\n\n${answer}`;
		try {
			const outcome = await onask(
				outgoing,
				[],
				includeContext,
				newFocusPending,
				working && interruptNext,
				undefined,
				[]
			);
			if (outcome.error) {
				askError = outcome.error;
			} else {
				fieldValues = {};
				newFocusPending = false;
				askNotice = outcome.notice ?? leftForLater;
			}
		} catch (cause) {
			askError = failureSentence(cause, 'Your answer was not delivered.');
		} finally {
			sending = false;
		}
	}
	/* The separate facts the question asks for, each with its own field. The
	 * answer travels as ordinary text, one line per fact. */
	const openFields = $derived.by(() =>
		openQuestion ? (visibleMessages[openQuestionIndex]?.intent?.ownerFields ?? []).slice(0, 4) : []
	);
	let fieldValues = $state<Record<string, string>>({});
	let fieldsFor = '';
	$effect(() => {
		const key = `${visibleMessages[openQuestionIndex]?.id ?? ''}`;
		if (key === fieldsFor) return;
		fieldsFor = key;
		fieldValues = {};
	});
	const fieldAnswer = $derived(
		openFields
			.filter((field) => fieldValues[field]?.trim())
			.map((field) => `${field}: ${fieldValues[field].trim()}`)
			.join('\n')
	);
	function answerWithFields(event: SubmitEvent) {
		event.preventDefault();
		if (fieldAnswer) void sendAnswer(fieldAnswer);
	}
	/* When the owner replied to an agent's question, the question folds to a
	 * receipt under it instead of staying open. */
	function answeredAt(index: number): string {
		const answer = answerIndex(visibleMessages, participantName, index);
		return answer >= 0 ? shortTime(visibleMessages[answer].createdAt) : '';
	}

	/* Replying to one message quotes it, so a reply to a long message says
	 * which point it answers. The quote travels as ordinary Markdown. */
	let quoting = $state<{ author: string; excerpt: string; answer?: boolean } | null>(null);
	function quote(author: string, text: string, mode: 'reply' | 'answer' = 'reply') {
		quoting = { author, excerpt: excerptOf(text), answer: mode === 'answer' };
		composerFocusKey += 1;
	}

	const activeFocusAfterMessageId = $derived(
		newFocusPending ? pendingFocusAfterMessageId : focusAfterMessageId
	);
	const focusActive = $derived(newFocusPending || focusStartedAt !== null);
	/* The rail sits beside every company page, so it renders only the recent end
	 * of the conversation; each message is Markdown, and a long history cost
	 * thousands of nodes on pages that never looked at it. Earlier messages load
	 * on request, and a closed rail renders no transcript until first opened. */
	const MESSAGE_WINDOW = 30;
	let messageWindow = $state(MESSAGE_WINDOW);
	let windowFor = '';
	$effect(() => {
		const key = `${companyId}:${participantId}:${currentTopicKey}`;
		if (key === windowFor) return;
		windowFor = key;
		messageWindow = MESSAGE_WINDOW;
	});
	let openedOnce = $state(false);
	$effect(() => {
		if (open) openedOnce = true;
	});
	const everOpened = $derived(open || openedOnce);
	const windowStart = $derived(Math.max(0, messages.length - messageWindow));
	// Each durable reply keeps its own timestamp, intent, and navigation target.
	const visibleMessages = $derived(windowStart ? messages.slice(windowStart) : messages);

	/* Prepending earlier messages keeps the reader's place rather than jumping
	 * them to the top of what was just loaded. */
	let loadingEarlier = $state(false);
	let earlierError = $state('');
	async function showEarlier(through = windowStart) {
		const el = scrollEl;
		const before = el ? el.scrollHeight - el.scrollTop : 0;
		el?.dispatchEvent(new Event('chat-scroll-pause'));
		/* Everything loaded is shown: ask the server for the page before it. */
		if (through === 0 && windowStart === 0 && hasEarlier && onloadearlier) {
			if (loadingEarlier) return;
			loadingEarlier = true;
			earlierError = '';
			try {
				const arrived = await onloadearlier();
				messageWindow += arrived;
			} catch (cause) {
				earlierError = failureSentence(cause, 'Earlier messages could not be loaded.');
			} finally {
				loadingEarlier = false;
			}
			await tick();
			if (el) el.scrollTop = el.scrollHeight - before;
			return;
		}
		messageWindow = Math.max(messageWindow + MESSAGE_WINDOW, messages.length - through);
		await tick();
		if (el) el.scrollTop = el.scrollHeight - before;
	}
	/* Mirrors the transcript's empty-state branch: only there does the
	 * connect action appear beside its explanation. */
	const providerPromptInline = $derived(
		visibleMessages.length === 0 &&
			conversationStatus !== 'unknown' &&
			!focusActive &&
			!review &&
			!workContext
	);
	const hasMessagesAfterFocus = $derived(
		focusActive &&
			messages.some((message) => messageNumericId(message.id) > activeFocusAfterMessageId)
	);
	const capabilityHint = $derived.by(() => {
		if (participantId !== 'exec')
			return `Ask ${participantName} about the work they own; the page you are on is linked.`;
		if (contextKind === 'work') return 'Exec can inspect current Work before answering.';
		if (contextKind === 'people') return 'Exec can route a question through the accountable lead.';
		if (contextKind === 'company')
			return 'Exec can compare a proposal with current company direction.';
		return 'Exec can inspect Work, compare options, or bring in a fresh critic.';
	});

	function messageNumericId(messageId: string): number {
		const value = Number(messageId.split(':').at(-1));
		return Number.isFinite(value) ? value : 0;
	}

	/* A screen reader hears each new reply as it lands, not only that one arrived. History that
	 * loads, or a conversation switched to, is not read out. */
	let announcement = $state('');
	let announcedThrough = -1;
	let announcedKey = '';
	$effect(() => {
		const key = `${companyId}:${participantId}:${currentTopicKey}`;
		const last = messages.at(-1);
		const id = last ? messageNumericId(last.id) : 0;
		if (key !== announcedKey || conversationStatus === 'unknown') {
			announcedKey = key;
			announcedThrough = conversationStatus === 'unknown' ? -1 : id;
			announcement = '';
			return;
		}
		if (announcedThrough < 0) {
			announcedThrough = id;
			return;
		}
		if (!last || id <= announcedThrough) return;
		announcedThrough = id;
		if (last.from !== 'you')
			announcement = `${last.author || participantName}: ${excerptOf(last.intent?.summary || last.text)}`;
	});

	function firstMessageNumericId(messageId: string): number {
		const value = Number(messageId.split(':')[0]);
		return Number.isFinite(value) ? value : 0;
	}

	/* One header for a run: same author, same day, within five minutes, and
	 * no focus boundary between them. */
	function continuesRun(index: number): boolean {
		if (windowStart + index === 0 || focusDividerBefore(index)) return false;
		const previous = messages[windowStart + index - 1];
		const current = messages[windowStart + index];
		return (
			previous.from === current.from &&
			dayOf(previous.createdAt) === dayOf(current.createdAt) &&
			new Date(current.createdAt).getTime() - new Date(previous.createdAt).getTime() < 5 * 60_000
		);
	}

	function focusDividerBefore(index: number): boolean {
		if (!focusActive) return false;
		const current = messages[windowStart + index];
		if (!current || firstMessageNumericId(current.id) <= activeFocusAfterMessageId) return false;
		const previous = messages[windowStart + index - 1];
		return !previous || messageNumericId(previous.id) <= activeFocusAfterMessageId;
	}

	/* Where earlier conversations began. The server keeps only the current focus, so each one
	 * this device sees is remembered here, and the transcript keeps a mark where it started. */
	const focusLogKey = $derived(`restless:focus-starts:${companyId}:${participantId}`);
	let pastFocusStarts = $state<number[]>([]);
	$effect(() => {
		try {
			const stored = JSON.parse(localStorage.getItem(focusLogKey) ?? '[]') as unknown;
			pastFocusStarts = Array.isArray(stored)
				? stored.filter((value): value is number => Number.isInteger(value) && value > 0)
				: [];
		} catch {
			pastFocusStarts = [];
		}
	});
	$effect(() => {
		const after = focusAfterMessageId;
		if (!after || focusStartedAt === null || untrack(() => pastFocusStarts).includes(after)) return;
		pastFocusStarts = [...untrack(() => pastFocusStarts), after].slice(-50);
		try {
			localStorage.setItem(focusLogKey, JSON.stringify(pastFocusStarts));
		} catch {
			/* A convenience: the current focus is still marked from the server. */
		}
	});
	function earlierFocusBefore(index: number): boolean {
		const current = messages[windowStart + index];
		const previous = messages[windowStart + index - 1];
		if (!current || !previous) return false;
		const id = firstMessageNumericId(current.id);
		const before = messageNumericId(previous.id);
		return pastFocusStarts.some(
			(after) =>
				!(focusActive && after === activeFocusAfterMessageId) && before <= after && id > after
		);
	}

	function cancelNewFocus() {
		newFocusPending = false;
		composerFocusKey += 1;
	}

	function beginNewFocus() {
		if (!newFocusAvailable || !connected || working || sending || review || workContext) return;
		pendingFocusAfterMessageId = messages.reduce(
			(maximum, message) => Math.max(maximum, messageNumericId(message.id)),
			0
		);
		newFocusPending = true;
		askError = '';
		askNotice = '';
		scrollReset += 1;
		composerFocusKey += 1;
	}

	/* Text selected on the main surface travels with the next message as a quote, so "explain
	 * this" names exactly what the owner means. The selection is kept while the composer has
	 * focus (which would clear it on the page), until it is sent, dropped, or the page changes. */
	let railEl = $state<HTMLElement | null>(null);
	let selection = $state<{ excerpt: string; label: string } | null>(null);
	$effect(() => {
		void contextLabel;
		selection = null;
	});
	$effect(() => {
		if (!open || review || workContext) return;
		const capture = () => {
			const current = window.getSelection();
			if (!current || current.isCollapsed) return;
			const node = current.anchorNode;
			const element = node instanceof Element ? node : (node?.parentElement ?? null);
			if (!element || railEl?.contains(element)) return;
			const text = current.toString().replace(/\s+/g, ' ').trim();
			if (text.length < 2) return;
			selection = {
				excerpt: text.length > 1_200 ? `${text.slice(0, 1_200)}…` : text,
				label: contextLabel
			};
		};
		document.addEventListener('selectionchange', capture);
		return () => document.removeEventListener('selectionchange', capture);
	});

	function sentFrom(path: string | null | undefined): { href: string; label: string } | null {
		if (!path || !path.startsWith(`/${encodeURIComponent(companyId)}`)) return null;
		const label = contextLabelFor?.(path);
		return label ? { href: path, label } : null;
	}

	function toggleContext() {
		includeContext = !includeContext;
		if (includeContext) contextFlare += 1;
	}

	const attachmentHref = (attachment: { uploadId: string }) =>
		`/api/companies/${encodeURIComponent(companyId)}/attachments/${encodeURIComponent(attachment.uploadId)}`;

	/* While Exec cannot be reached a message is still saved, and read when it is back. The turn dock
	 * says so when intelligence needs connecting; otherwise this receipt does. */
	const leftForLater = $derived(
		!needsProvider && (connectionStatus === 'unavailable' || connectionStatus === 'error')
			? `Saved. ${participantName} reads it when it is back.`
			: ''
	);
	let sending = $state(false);
	async function submitAsk(event: SubmitEvent) {
		event.preventDefault();
		await deliver();
	}

	async function deliver() {
		const typed = composer.trim();
		const files = composerFiles;
		/* A file on its own is a message: it says what it carries, so Exec reads a sentence. */
		const text =
			typed || (files.length ? `Attached: ${files.map((file) => file.name).join(', ')}` : '');
		if (!text || sending || deciding || !onask) return;
		/* The same limits the server holds, said before anything is sent. */
		const tooLarge = files.find((file) => file.size > MAX_UPLOAD_BYTES);
		if (tooLarge || files.length > MAX_UPLOAD_FILES) {
			askError = tooLarge
				? `${tooLarge.name} is larger than ${MAX_UPLOAD_BYTES / (1024 * 1024)} MB, the most one file can be.`
				: `Attach at most ${MAX_UPLOAD_FILES} files to one message.`;
			return;
		}
		scrollReset += 1;
		sending = true;
		askError = '';
		askNotice = '';
		const sent = composer;
		const quoted = quoting;
		const selected = includeContext && !review && !workContext ? selection : null;
		const quotedText = quoted ? `> ${quoted.author}: ${quoted.excerpt}\n\n${text}` : text;
		const outgoing = selected
			? `> Selected in ${selected.label}: ${selected.excerpt}\n\n${quotedText}`
			: quotedText;
		const skills = composerSkills;
		composer = '';
		try {
			const command: Awaited<ReturnType<typeof runCommand>> =
				typed && typed !== goalCreatedFor ? await runCommand(typed) : { send: true };
			if (command.createdGoal) goalCreatedFor = typed;
			if (!command.send) {
				if (command.error) {
					composer = sent;
					askError = command.error;
				} else {
					askNotice = command.notice ?? '';
					noticeUndo = command.undo ? { notice: askNotice, run: command.undo } : null;
					if (askNotice) recordCommand(typed, askNotice);
				}
				return;
			}
			const delivery = onask(
				outgoing,
				files,
				includeContext,
				newFocusPending,
				working && interruptNext,
				undefined,
				skills,
				retryCommandId && retryBody === outgoing ? retryCommandId : undefined
			);
			// Your message rises to the top of the transcript and the reply grows beneath it.
			await tick();
			scrollEl?.dispatchEvent(new Event('chat-scroll-anchor'));
			const outcome = await delivery;
			if (outcome.error) {
				composer = sent;
				askError = outcome.error;
				retryCommandId = outcome.retryCommandId ?? null;
				retryBody = outcome.retryCommandId ? outgoing : '';
			} else {
				composerFiles = [];
				composerSkills = [];
				quoting = null;
				if (selected && selection === selected) selection = null;
				fieldValues = {};
				newFocusPending = false;
				retryCommandId = null;
				retryBody = '';
				goalCreatedFor = '';
				askNotice = outcome.notice ?? leftForLater;
			}
		} catch (cause) {
			composer = sent;
			retryCommandId = null;
			retryBody = '';
			askError = failureSentence(cause, 'Your message was not delivered.');
		} finally {
			sending = false;
		}
	}

	async function requestChanges() {
		const feedback = reviewFeedback.trim();
		if (!review || !feedback || deciding) return;
		deciding = true;
		reviewError = '';
		try {
			const failure = await review.ondecide('request_changes', feedback);
			if (failure) reviewError = failure;
			else reviewFeedback = '';
		} finally {
			deciding = false;
		}
	}

	async function acceptReview() {
		if (!review || deciding) return;
		deciding = true;
		reviewError = '';
		try {
			const failure = await review.ondecide('accept', '');
			if (failure) reviewError = failure;
			else reviewFeedback = '';
		} finally {
			deciding = false;
		}
	}

	/* A company that never had a model and one whose model connection stopped are different
	 * situations: say which, in a few words, and name the one action that fixes it. */
	const neverConnected = $derived(!providerLabel.startsWith('Reconnect'));
	/* Only the owner connects models. A teammate is told who can fix it, with no button to a page
	 * they cannot open. */
	const ownerFixes = $derived(membershipRole === 'owner');
	const providerHeading = $derived(
		!ownerFixes
			? `${participantName} can't reply right now`
			: neverConnected
				? 'Connect a model'
				: `${participantName} can't reply`
	);
	const providerReason = $derived(
		!ownerFixes
			? 'Its model connection isn’t working. The company owner can fix it in Company → Intelligence.'
			: neverConnected
				? `${participantName} needs one to think and act. Use an API key or a ChatGPT or Claude sign-in.`
				: 'Its model connection stopped working.'
	);
	const providerAction = $derived(neverConnected ? 'Connect a model' : providerLabel);
	holdQueries(() => [directRooms]);
</script>

{#snippet commandLines(lines: CommandRecord[])}
	{#each lines as line (line.id)}
		<p class="command-line" title={`${line.command} · ${shortTime(line.at)} · kept on this device`}>
			<span class="command-name">{line.command}</span>
			<span>{line.result}</span>
		</p>
	{/each}
{/snippet}

<aside
	bind:this={railEl}
	id="bridge-exrail"
	class="bridge-exrail"
	class:open
	class:wide
	aria-label={`${participantName} conversation`}
	aria-hidden={!open}
	inert={!open}
>
	<div class="exr-inner">
		<header class="exr-head">
			<div class="exr-head-primary">
				{#if review || workContext}
					<button
						class="rail-back"
						type="button"
						aria-label="Back to the Inbox"
						title="Back to the Inbox"
						onclick={(review ?? workContext)!.onback}
					>
						<ArrowLeft size={18} aria-hidden="true" />
					</button>
				{/if}
				{#if topics.length && ontopic}
					<!-- Who you are talking to, and about what. One menu switches both. -->
					<div class="exr-who topic-switch">
						<ActionMenu label="Switch conversation">
							{#snippet trigger()}<span class="topic-face"
									><SemanticMark
										meaning={participantId === 'exec' ? 'executive' : 'work'}
										size="small"
									/><span class="topic-text"
										><strong class="exr-name">{participantName}</strong>{#if topicLabel}<span
												class="topic-label">{topicLabel}</span
											>{/if}</span
									>{#if anyTopicNew}<i
											class="topic-new"
											title="Something new in another conversation"
										></i>{/if}<ChevronDown size={14} aria-hidden="true" /></span
								>{/snippet}
							{#each topics as entry (entry.key)}
								<button type="button" onclick={() => ontopic(entry.topic)}
									><span class="topic-option"
										><span>{entry.label}</span><small>{entry.hint}</small></span
									>{#if topicHasNew(entry)}<i class="topic-new" aria-label="New"
										></i>{/if}{#if entry.key === currentTopicKey}<Check
											size={14}
											aria-label="Current"
										/>{/if}</button
								>
							{/each}
						</ActionMenu>
					</div>
				{:else}
					<div class="exr-who">
						<SemanticMark meaning={review || workContext ? 'work' : 'executive'} size="small" />
						<strong class="exr-name">{participantName}</strong>
					</div>
				{/if}
				<div class="exr-tools">
					{#if newFocusAvailable}
						<button
							type="button"
							class="exr-tool"
							aria-label="New conversation"
							title={working || sending
								? `New conversation · available once ${participantName} has finished replying`
								: !connected
									? `New conversation · available once ${participantName} can be reached`
									: 'New conversation · fresh working context, company memory kept'}
							disabled={!connected || working || sending}
							onclick={beginNewFocus}><SquarePen size={15} strokeWidth={1.9} /></button
						>
					{/if}
					{#if visibleMessages.length}
						<ConversationHistoryTools
							{messages}
							{participantName}
							onjump={jumpToMessage}
							more={hasEarlier}
							onsearchall={directRoom && isDirectTopic ? searchAll : null}
						/>
					{/if}
					{#if messages.length}
						<ActionMenu label="Conversation actions"
							><button
								type="button"
								title={hasEarlier
									? `Copies the ${messages.length} loaded messages; show earlier messages to include them`
									: 'Copies the whole conversation as text'}
								onclick={() => void copyConversation()}>Copy conversation</button
							>{#if canOperate}<button
									type="button"
									disabled={savingConversation}
									title="Keep the loaded messages as a Library document only you can open until you share it"
									onclick={() => void keepConversation()}
									>{savingConversation ? 'Saving…' : 'Save to Library'}</button
								>{/if}</ActionMenu
						>
					{/if}
					{#if !review && !workContext && !wide}
						<a
							class="exr-tool"
							href={`/${encodeURIComponent(companyId)}/people?person=${encodeURIComponent(participantId)}`}
							aria-label="Open full width"
							title="Open this conversation full width"><Maximize2 size={14} strokeWidth={1.9} /></a
						>
					{/if}
					{#if onclose}
						<button
							type="button"
							class="exr-tool"
							aria-label="Close"
							title="Close · the Exec button brings it back"
							onclick={onclose}><PanelRightClose size={15} strokeWidth={1.9} /></button
						>
					{/if}
				</div>
			</div>
			{#if review}
				<div class="review-controls">
					<HoldApprove
						small
						label={deciding ? 'Recording…' : 'Hold to accept'}
						title="Hold to accept outcome"
						completeLabel="accepted ✓"
						disabled={!canDecide || deciding || sending}
						onapprove={() => void acceptReview()}
					/>
				</div>
			{/if}
		</header>
		{#if reviewError}<p class="review-error" role="alert">{reviewError}</p>{/if}
		{#if review}
			<form
				class="review-feedback"
				onsubmit={(event) => {
					event.preventDefault();
					void requestChanges();
				}}
			>
				<label
					>Request changes<textarea
						bind:value={reviewFeedback}
						rows="3"
						placeholder="Describe the changes needed…"
						disabled={!canDecide || deciding}></textarea></label
				>
				<button class="btn small" disabled={!canDecide || deciding || !reviewFeedback.trim()}
					>{deciding ? 'Recording…' : 'Request changes'}</button
				>
			</form>
		{/if}

		{#if pinnedMessages.length}
			<details class="pinned" bind:open={pinsOpen}>
				<summary
					><Pin size={12} aria-hidden="true" />{pinnedMessages.length} pinned{#if !pinsOpen}<span
							class="pinned-first">{messagePreview(pinnedMessages.at(-1)?.text).slice(0, 80)}</span
						>{/if}</summary
				>
				{#each pinnedMessages as message (message.id)}
					<button
						type="button"
						onclick={() => {
							pinsOpen = false;
							jumpToMessage(message.id);
						}}
						><strong>{message.from === 'you' ? 'You' : message.author || participantName}</strong
						><span>{messagePreview(message.text).slice(0, 140)}</span></button
					>
				{/each}
			</details>
		{/if}
		<div class="exr-panel">
			{#if !needsProvider && connectionStatus === 'unknown'}
				<!-- Unknown is the ordinary first moment; the transcript skeleton below
				     already says so without a sentence about it. -->
				<p class="sr-only" role="status">Checking {participantName}'s conversation status…</p>
			{:else if !needsProvider && connectionStatus === 'unavailable'}
				<p class="exr-connection-notice" role="status">
					A conversation route for {participantName} is not available in the latest company status.
				</p>
			{/if}
			{#if reconnecting}
				<p class="exr-reconnecting" role="status">
					<i aria-hidden="true"></i><span
						title="The last loaded messages stay readable while the connection comes back. It retries by itself."
						>Reconnecting… showing the last loaded messages</span
					>
					<button type="button" class="exr-retry" onclick={retryNow}>Try again</button>
				</p>
			{/if}
			<div class="exr-chat">
				<p class="sr-only" aria-live="polite">{announcement}</p>
				<div
					class="exr-msgs"
					bind:this={scrollEl}
					use:followChat={{
						key: `${companyId}:${participantId}:${scrollReset}`,
						memory: `rail:${companyId}:${participantId}:${currentTopicKey}`,
						onfollow: (following) => {
							awayFrom = following ? null : messages.length;
						}
					}}
				>
					{#if everOpened && (windowStart || hasEarlier)}
						<button
							type="button"
							class="exr-earlier"
							disabled={loadingEarlier}
							onclick={() => void showEarlier()}
							>{loadingEarlier ? 'Loading earlier messages…' : 'Show earlier messages'}</button
						>
						{#if earlierError}<p class="exr-error" role="alert">{earlierError}</p>{/if}
					{/if}
					{#each everOpened ? visibleMessages : [] as message, i (message.clientKey ?? message.id)}
						{#if focusDividerBefore(i)}
							<div class="conversation-focus-boundary">
								<span>New focus</span><i aria-hidden="true"></i><span>Company memory retained</span>
							</div>
						{:else if earlierFocusBefore(i)}
							<div
								class="conversation-focus-boundary earlier"
								title="A new conversation began here; company memory was kept"
							>
								<span>New conversation</span><i aria-hidden="true"></i><span
									>{shortTime(message.createdAt)}</span
								>
							</div>
						{/if}
						{#if unreadRuleBefore(i)}
							<div class="unread-rule" role="separator">
								<span
									>New since {shortTime(
										messages[windowStart + i - 1]?.createdAt ?? message.createdAt
									)}</span
								>
							</div>
						{/if}
						<HandoffGroup
							exchanges={handoffsBefore(i)}
							name={exchanges.name}
							host={participantId}
						/>
						{@render commandLines(commandsBefore(i))}
						{#if i === 0 || dayOf(message.createdAt) !== dayOf(visibleMessages[i - 1].createdAt)}
							<div class="day-sep" aria-hidden="true">
								<span>{dayLabel(message.createdAt)}</span>
							</div>
						{/if}
						<ConversationMessage
							continued={continuesRun(i)}
							domId={messageDomId(message.id)}
							messageId={String(messageNumericId(message.id) || '')}
							{companyId}
							reactions={reactions.summaryFor(messageNumericId(message.id))}
							onreact={canOperate && messageNumericId(message.id) > 0
								? (emoji, on) => void reactions.react(messageNumericId(message.id), emoji, on)
								: undefined}
							sender={message.from === 'you' ? 'owner' : message.from}
							author={message.from === 'you' ? 'You' : message.author || participantName}
							text={message.text}
							createdAt={message.createdAt}
							details={message.details}
							intent={message.intent}
							attachments={message.attachments}
							hrefFor={attachmentHref}
							context={message.from === 'you' ? sentFrom(message.contextPath) : null}
						>
							{#snippet actions()}<!-- Two tools in sight (reply, and copy beside them); the rest wait in one menu. -->{#if message.from !== 'you' && onask}<button
										type="button"
										class="copy-message"
										aria-label="Reply to this message"
										title="Reply to this message"
										onclick={() => quote(message.author || participantName, message.text)}
										><Reply size={12} aria-hidden="true" /></button
									>{/if}{#if canOperate && messageNumericId(message.id) > 0}<ActionMenu label="More"
										><button type="button" onclick={() => togglePin(message)}
											>{isPinned(messageNumericId(message.id))
												? 'Unpin'
												: 'Pin above the conversation'}</button
										><button type="button" onclick={() => void copyLink(message)}>Copy link</button
										><button
											type="button"
											title="Ask the Exec to turn this into Work"
											onclick={() => turnIntoWork(message)}>Turn into Work</button
										></ActionMenu
									>{/if}{/snippet}
						</ConversationMessage>
						{#if openQuestion && i === openQuestionIndex}
							<div class="open-question" role="group" aria-label={`${participantName} is asking`}>
								<p>{openQuestion}</p>
								{#if openReplies.length}
									<!-- Beautiful UI's approval card: each likely answer is a row, and the last row
									     hands the answer to the composer. A tap sends after a moment's Undo. -->
									<div class="open-replies">
										{#each openReplies as reply (reply)}
											<button
												type="button"
												class:chosen={pendingAnswer?.text === reply}
												disabled={!canOperate || sending || !!pendingAnswer}
												title="Send this answer"
												onclick={() => tapReply(reply)}
												><i aria-hidden="true"></i><span>{reply}</span></button
											>
										{/each}
										{#if !pendingAnswer}<button
												type="button"
												class="other"
												disabled={!canOperate}
												onclick={() => quote(participantName, openQuestion, 'answer')}
												><i aria-hidden="true"></i><span>Something else…</span></button
											>{/if}
									</div>
								{/if}
								{#if pendingAnswer}
									<p class="answer-pending" role="status">
										<span>Sending “{pendingAnswer.text}”</span><button
											type="button"
											onclick={undoReply}>Undo</button
										>
									</p>
								{/if}
								{#if openFields.length && !pendingAnswer}
									<form class="open-fields" onsubmit={answerWithFields}>
										{#each openFields as field (field)}
											<label
												><span>{field}</span><input
													bind:value={fieldValues[field]}
													disabled={!canOperate || sending}
												/></label
											>
										{/each}
										<button
											class="btn small primary"
											disabled={!fieldAnswer || sending || !canOperate}
											title="Send these as your answer">Send</button
										>
									</form>
								{/if}
							</div>
						{/if}
						{#if message.from === 'you' && working && turn && !message.readAt && messageNumericId(message.id) > turn.triggerMessageId}<p
								class="queued"
								title={`${participantName} reads it after the current reply`}
							>
								Queued
							</p>{/if}
						{#if answeredAt(i)}<p class="answered" title={message.intent?.ownerNeed ?? ''}>
								<Check size={12} aria-hidden="true" /> You answered · {answeredAt(i)}
							</p>{/if}
					{:else}
						{#if conversationFailed && conversationStatus === 'unknown'}
							<div class="exr-empty" role="alert">
								<p class="exr-empty-h">Conversation unavailable</p>
								<p class="exr-empty-p">Recent messages could not be loaded.</p>
								<button type="button" class="exr-retry" onclick={() => onrefreshConversation?.()}
									>Try again</button
								>
							</div>
						{:else if conversationStatus === 'unknown' || (!needsProvider && connectionStatus === 'unknown')}
							<!-- Whether intelligence is connected is not known yet: neither
							     "Ask anything" nor "Connect intelligence" would be true. -->
							<Skeleton label="Loading conversation" variant="messages" count={3} />
						{:else if focusActive}
							<!-- The focus boundary below is the empty transcript state. -->
						{:else if review || workContext}
							<div class="exr-empty review-empty">
								<div class="review-empty-card">
									<span class="review-empty-mark">
										<MatrixGlyph rows={GLYPHS.work} size={12} />
									</span>
									<div>
										<strong>Talk to the lead</strong>
										<p>Ask questions, discuss evidence, or share revision feedback.</p>
									</div>
								</div>
							</div>
						{:else if needsProvider || !exchanges.exchanges.length}
							<div class="exr-empty">
								{#if needsProvider}<span class="exr-empty-mark" aria-hidden="true"
										><MatrixGlyph rows={GLYPHS.key} size={10} /></span
									>{/if}
								<p class="exr-empty-h">
									{needsProvider ? providerHeading : 'Ask anything.'}
								</p>
								<p class="exr-empty-p">
									{needsProvider ? providerReason : capabilityHint}
								</p>
								{#if needsProvider && ownerFixes}
									<a
										class="btn primary small provider-connect"
										href={`/${companyId}/company/provider`}
										>{#if neverConnected}<Plus
												size={14}
												strokeWidth={2}
												aria-hidden="true"
											/>{:else}<RotateCw size={14} strokeWidth={2} aria-hidden="true" />{/if}<span
											>{providerAction}</span
										></a
									>
								{/if}
							</div>
						{/if}
					{/each}
					{#if focusActive && !hasMessagesAfterFocus}
						<div class="conversation-focus-boundary" role={newFocusPending ? 'status' : undefined}>
							<span>New focus</span><i aria-hidden="true"></i>{#if newFocusPending}<button
									type="button"
									class="focus-cancel"
									title="Keep talking in the current conversation"
									onclick={cancelNewFocus}>Cancel</button
								>{:else}<span>Company memory retained</span>{/if}
						</div>
						{#if !composer.trim() && !working}
							<p class="conversation-capability-hint">{capabilityHint}</p>
						{/if}
					{/if}
					<HandoffGroup exchanges={handoffsSinceLast} name={exchanges.name} host={participantId} />
					{@render commandLines(commandsSinceLast)}
					{#if receipt}<p class="receipt" role="status">
							{#if retractable}<button
									type="button"
									disabled={retracting}
									onclick={() => retract(true)}>Edit</button
								><button type="button" disabled={retracting} onclick={() => retract(false)}
									>Retract</button
								><span aria-hidden="true">·</span>{:else if !composer.trim()}<button
									type="button"
									title={`${participantName} has it already. Send a corrected version that replaces it.`}
									onclick={correct}>Correct</button
								><span aria-hidden="true">·</span>{/if}{receipt}
						</p>{/if}
					{#if turn && needsProvider && (turn.live?.phase ?? 'queued') === 'queued'}
						<p class="exr-setup-pending" role="status">
							Message saved. {participantName} can reply after intelligence access is restored.
						</p>
					{:else if turn}<ConversationTurnDock
							{participantName}
							{turn}
							onstop={stopTurn}
							onretry={retryTurn}
						/>{/if}
					<!-- Work nobody's message started: a quiet line of what it is doing now. -->
					{#if ambientTurn && (!working || (turn?.live?.phase ?? 'queued') === 'queued')}
						<ConversationTurnDock {participantName} turn={ambientTurn} />
					{/if}
				</div>

				{#if awayFrom !== null && visibleMessages.length}
					<div class="jump-anchor">
						<button
							type="button"
							class="jump-latest"
							onclick={() => scrollEl?.dispatchEvent(new Event('chat-scroll-end'))}
							>{messages.length > awayFrom
								? `${messages.length - awayFrom} new ↓`
								: 'Jump to latest ↓'}</button
						>
					</div>
				{/if}
				{#if needsProvider}
					<!-- In the empty state the connect action sits with its explanation
					     above; otherwise it sits above the composer, which stays usable: a message
					     written now is kept and read when the connection is back. -->
					{#if !providerPromptInline && ownerFixes}<a
							class="btn primary small provider-connect-slot"
							href={`/${companyId}/company/provider`}
							>{#if neverConnected}<Plus
									size={14}
									strokeWidth={2}
									aria-hidden="true"
								/>{:else}<RotateCw size={14} strokeWidth={2} aria-hidden="true" />{/if}<span
								>{providerAction}</span
							></a
						>{/if}
				{:else}
					{#if participantId === 'exec'}<WorkInFlight
							{companyId}
							owner={membershipRole === 'owner'}
						/>{/if}
					{#if membershipRole === 'owner' || membershipRole === 'admin'}<AppRequestCards
							{companyId}
							actor={participantId}
							readOnly={membershipRole !== 'owner'}
						/>{/if}
					{#if participantId === 'exec'}<MyRequests {companyId} />{/if}
				{/if}
				<form class="exr-composer" onsubmit={submitAsk}>
					{#if quoting}
						<div class="quoting">
							<Reply size={13} aria-hidden="true" />
							<span
								>{quoting.answer ? 'Answering' : `Replying to ${quoting.author}`}:
								<em>{quoting.excerpt}</em></span
							>
							<button
								type="button"
								aria-label="Stop replying to that message"
								title="Stop replying to that message"
								onclick={() => (quoting = null)}><X size={13} /></button
							>
						</div>
					{/if}
					<Composer
						bind:value={composer}
						bind:files={composerFiles}
						bind:selectedSkills={composerSkills}
						options={[...options.value, ...references]}
						interruptible={working}
						bind:interrupt={interruptNext}
						actionLabel={retryCommandId ? 'Retry send · it cannot be sent twice' : ''}
						disabled={!canOperate || sending || deciding || !onask}
						minlength={1}
						placeholder={needsProvider ||
						connectionStatus === 'unavailable' ||
						connectionStatus === 'error'
							? `Leave a message for ${participantName}…`
							: openQuestion
								? `Answer ${participantName}, or ask anything…`
								: review || workContext || participantId !== 'exec'
									? `Message ${participantName}…`
									: 'Ask, redirect, or make a judgement…'}
						ariaLabel={review || workContext
							? `Message ${participantName}`
							: `Ask ${participantName}`}
						flareKey={contextFlare}
						focusKey={composerFocusKey}
					>
						{#snippet controls()}
							<IntelligenceChip {companyId} actorId={participantId} name={participantName} />
							{#if !review && !workContext}
								<div class="exec-context-line">
									<button
										type="button"
										class="exec-context-chip"
										class:off={!includeContext}
										aria-pressed={includeContext}
										title={includeContext
											? `This message links to ${contextLabel}; click to send it without the link`
											: `Link ${contextLabel} to this message`}
										onclick={toggleContext}
									>
										<MatrixGlyph rows={GLYPHS.work} size={8} />
										<span title={includeContext && selection ? `“${selection.excerpt}”` : undefined}
											>{includeContext
												? selection
													? `Selection in ${selection.label}`
													: contextLabel
												: `Link ${contextLabel}`}</span
										>
									</button>
									{#if includeContext && selection}<button
											type="button"
											class="exec-context-drop"
											aria-label="Send without the selection"
											title="Send without the selection"
											onclick={() => (selection = null)}><X size={11} aria-hidden="true" /></button
										>{/if}
								</div>
							{/if}
						{/snippet}
					</Composer>
					{#if uploadProgress !== null && sending}
						<p class="exr-notice" role="status">
							Uploading… {Math.round(uploadProgress * 100)}%
						</p>
					{/if}
					{#if askError}
						<p class="exr-error" role="alert">{askError}</p>
					{/if}
					{#if askNotice}
						<p class="exr-notice" role="status">
							{askNotice}{#if noticeUndo?.notice === askNotice}
								<button type="button" class="exr-undo" onclick={() => void undoCommand()}
									>Undo</button
								>{/if}
						</p>
					{/if}
				</form>
			</div>
		</div>
	</div>
</aside>

<style>
	.command-line {
		display: flex;
		gap: 6px;
		min-width: 0;
		margin: 4px 0;
		padding: 0 4px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		overflow-wrap: anywhere;
	}
	.command-name {
		flex: none;
		color: var(--text-secondary);
		font-family: var(--font-mono);
	}
	.exr-undo {
		margin-left: 6px;
		padding: 0;
		border: 0;
		background: none;
		color: var(--ink);
		font: inherit;
		font-weight: 600;
		text-decoration: underline;
		cursor: pointer;
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
	.pinned {
		flex: none;
		border-bottom: 1px solid var(--border-soft);
		background: color-mix(in srgb, var(--surface-raised) 70%, transparent);
		font-size: var(--t-label);
	}
	.pinned summary {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		padding: 6px 14px;
		color: var(--text-secondary);
		font-weight: 500;
		cursor: pointer;
		list-style: none;
	}
	.pinned summary::-webkit-details-marker {
		display: none;
	}
	.pinned-first {
		min-width: 0;
		overflow: hidden;
		color: var(--text-tertiary);
		font-weight: 400;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.pinned button {
		display: grid;
		width: 100%;
		gap: 1px;
		padding: 6px 14px;
		border: 0;
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.pinned button:hover {
		background: var(--wash-hover);
	}
	.pinned button strong {
		color: var(--ink);
		font-weight: 500;
	}
	.pinned button span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.unread-rule {
		display: flex;
		align-items: center;
		gap: 8px;
		margin: 10px 14px 2px;
		color: var(--intent-conversation);
		font: 500 var(--t-label) var(--font-ui);
	}
	.unread-rule::after {
		flex: 1;
		height: 1px;
		content: '';
		background: color-mix(in srgb, var(--intent-conversation) 30%, transparent);
	}
	/* The open question sits under the message that asks it, in the conversation's flow: the
	 * composer below stays free, and typing there answers it too. */
	.open-question {
		display: grid;
		gap: 8px;
		margin: 4px 14px 12px 45px;
		padding: 10px 6px 6px;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface-raised);
		box-shadow: var(--shadow-soft);
		animation: question-in var(--motion-disclosure, 180ms) var(--ease-standard, ease-out) both;
	}
	.open-question > p {
		margin: 0;
		padding: 0 6px;
		color: var(--ink);
		font-weight: 500;
		line-height: 1.4;
	}
	@keyframes question-in {
		from {
			opacity: 0;
			transform: translateY(4px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.open-question {
			animation: none;
		}
	}
	.answer-pending {
		display: flex;
		align-items: center;
		gap: 10px;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.open-question > .answer-pending {
		padding: 0 6px 4px;
		font-weight: 400;
	}
	.answer-pending button {
		padding: 0;
		border: 0;
		background: transparent;
		color: var(--intent-conversation);
		font: inherit;
		font-weight: 500;
		cursor: pointer;
	}
	.answer-pending button:hover {
		text-decoration: underline;
	}
	.topic-new {
		width: 7px;
		height: 7px;
		flex: none;
		border-radius: 999px;
		background: var(--intent-conversation);
	}
	.receipt {
		display: flex;
		justify-content: flex-end;
		gap: 6px;
		margin: 2px 16px 8px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.receipt button {
		padding: 0;
		border: 0;
		background: transparent;
		color: var(--text-secondary);
		font: inherit;
		cursor: pointer;
	}
	.receipt button:hover {
		color: var(--ink);
		text-decoration: underline;
	}
	.open-replies {
		display: grid;
		gap: 1px;
	}
	.open-replies button {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		min-height: 30px;
		padding: 4px 6px;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--ink);
		font: inherit;
		font-size: var(--t-body);
		text-align: left;
		cursor: pointer;
		transition: background-color var(--motion-state) var(--ease-standard);
	}
	/* The radio ring: empty until chosen, then filled while the answer is on its way. */
	.open-replies i {
		flex: none;
		width: 14px;
		height: 14px;
		border: 1.5px solid var(--border-strong);
		border-radius: 50%;
		transition:
			border-color var(--motion-state) var(--ease-standard),
			box-shadow var(--motion-state) var(--ease-standard);
	}
	.open-replies button:hover:not(:disabled) {
		background: var(--wash-hover);
	}
	.open-replies button:hover:not(:disabled) i {
		border-color: var(--ink);
	}
	.open-replies button.chosen i {
		border-color: var(--ink);
		box-shadow:
			inset 0 0 0 3px var(--surface-raised),
			inset 0 0 0 7px var(--ink);
	}
	.open-replies button.other {
		color: var(--text-tertiary);
	}
	.open-replies button.other i {
		border-style: dashed;
	}
	.open-replies button:disabled:not(.chosen) {
		opacity: 0.5;
		cursor: default;
	}
	.open-replies button.chosen {
		cursor: default;
	}

	.quoting {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 0 0 6px;
		padding: 6px 8px;
		border-left: 2px solid var(--intent-conversation);
		background: var(--surface-raised);
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.quoting span {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.quoting em {
		color: var(--ink);
		font-style: normal;
	}
	.quoting button {
		display: grid;
		place-items: center;
		width: 22px;
		height: 22px;
		padding: 0;
		border: 0;
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
	}
	.open-fields {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
		gap: 6px;
	}
	.open-fields label {
		display: grid;
		gap: 2px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.open-fields input {
		min-width: 0;
		padding: 5px 7px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface-raised);
		color: var(--ink);
		font: inherit;
		font-size: var(--t-body);
	}
	.open-fields .btn {
		grid-column: 1 / -1;
		justify-self: start;
	}
	/* A message sent while the reply is still being written waits, said on the message itself. */
	.queued {
		margin: -2px 14px 4px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		text-align: right;
	}
	.answered {
		display: flex;
		align-items: center;
		gap: 5px;
		margin: 0 14px 4px 45px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.provider-connect-slot {
		margin: var(--space-3);
	}
	.provider-connect {
		margin-top: var(--space-4);
	}

	.exr-head-primary {
		min-width: 0;
		flex: 1 1 auto;
		display: flex;
		align-items: center;
		gap: 10px;
	}
	/* The switcher is the name, not a bar: it sizes to its words and only
	 * shows a wash on hover or while its menu is open. */
	.topic-switch {
		flex: 0 1 auto !important;
		overflow: hidden;
	}
	.topic-switch :global(.action-menu) {
		min-width: 0;
	}
	.topic-switch :global(summary) {
		display: flex;
		justify-content: flex-start;
		width: auto;
		max-width: 100%;
		min-width: 0;
		height: 30px;
		padding: 0 6px 0 2px;
		border-color: transparent !important;
		overflow: hidden;
	}
	.topic-switch :global(summary:hover),
	.topic-switch :global(.action-menu[open] > summary) {
		background: var(--wash-hover);
	}
	/* The switcher clips what overflows it, so its focus ring is drawn inside. */
	.topic-switch :global(summary:focus-visible) {
		outline-offset: -2px;
	}
	.exr-tools {
		display: flex;
		flex: none;
		align-items: center;
		gap: 2px;
		margin-left: auto;
	}
	.exr-tool {
		display: grid;
		place-items: center;
		width: 28px;
		height: 28px;
		padding: 0;
		border: 0;
		border-radius: var(--radius-control);
		background: transparent;
		color: var(--text-tertiary);
		cursor: pointer;
	}
	.exr-tool:hover:not(:disabled) {
		background: var(--wash-hover);
		color: var(--ink);
	}
	.exr-tool:disabled {
		opacity: 0.4;
		cursor: default;
	}
	.exr-head-primary :global(.history-toggle) {
		width: 28px;
		height: 28px;
		padding: 0;
		justify-content: center;
	}
	.topic-face {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
		color: var(--ink);
	}
	.topic-face :global(svg) {
		flex: none;
		color: var(--text-tertiary);
	}
	/* The name and its topic are two sizes on one line: they share a baseline, while the mark
	 * and chevron stay centred on the row. */
	.topic-text {
		display: flex;
		align-items: baseline;
		gap: 6px;
		min-width: 0;
	}
	.topic-face .exr-name {
		flex: none;
		max-width: 60%;
	}
	.topic-label {
		flex: 1 1 auto;
		min-width: 0;
		overflow: hidden;
		color: var(--text-secondary);
		font-size: var(--t-label);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.topic-label::before {
		content: '· ';
		color: var(--text-tertiary);
	}
	.topic-option {
		display: grid;
		flex: 1;
		min-width: 0;
	}
	.topic-option span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.topic-option small {
		color: var(--text-tertiary);
		font-size: var(--t-label);
	}
	.exr-head-primary .exr-who {
		min-width: 0;
		flex: 1 1 auto;
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.exr-panel {
		position: relative;
		min-height: 0;
		flex: 1 1 auto;
		display: flex;
		flex-direction: column;
	}
	.exr-chat {
		min-height: 0;
		flex: 1 1 auto;
		display: flex;
		flex-direction: column;
	}
	.conversation-focus-boundary {
		width: calc(100% - 28px);
		display: grid;
		grid-template-columns: auto minmax(16px, 1fr) auto;
		align-items: center;
		gap: 8px;
		margin: 18px 14px 8px;
		color: color-mix(in srgb, var(--intent-conversation) 58%, var(--text-tertiary));
		font: 500 var(--t-label) var(--font-ui);
		font-variant-numeric: tabular-nums;
		animation: focus-arrive var(--motion-disclosure) var(--ease-spring) both;
	}
	.conversation-focus-boundary.earlier {
		margin-block: 12px 6px;
		color: var(--text-tertiary);
		animation: none;
	}
	.focus-cancel {
		padding: 0;
		border: 0;
		background: transparent;
		color: inherit;
		font: inherit;
		text-decoration: underline;
		cursor: pointer;
	}
	.conversation-focus-boundary i {
		height: 1px;
		background: linear-gradient(
			90deg,
			color-mix(in srgb, var(--intent-conversation) 35%, transparent),
			color-mix(in srgb, var(--intent-conversation) 10%, transparent)
		);
	}
	.conversation-capability-hint {
		max-width: 270px;
		margin: 3px auto 20px;
		padding-inline: 18px;
		color: var(--text-tertiary);
		font-size: var(--t-label);
		line-height: 1.55;
		text-align: center;
		animation: focus-arrive var(--motion-disclosure) var(--ease-standard) both;
	}
	.exr-setup-pending {
		margin: var(--space-3);
		padding: var(--space-3);
		border: 1px solid var(--border-soft);
		border-radius: var(--radius-control);
		background: var(--surface-pane);
		color: var(--text-secondary);
		font-size: var(--t-label);
		line-height: 1.5;
	}
	@keyframes focus-arrive {
		from {
			opacity: 0;
			transform: translateY(5px);
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.conversation-focus-boundary,
		.conversation-capability-hint {
			animation: none;
		}
	}
	.exr-connection-notice {
		flex: none;
		margin: 0;
		padding: 9px 14px;
		border-bottom: 1px solid var(--border-soft);
		color: var(--text-secondary);
		font-size: var(--t-label);
		line-height: 1.45;
	}
	.exr-reconnecting {
		display: flex;
		flex: none;
		align-items: center;
		gap: 8px;
		margin: 0;
		padding: 6px 14px;
		border-bottom: 1px solid var(--border-soft);
		color: var(--text-tertiary);
		font-size: var(--t-label);
		animation: exr-reconnecting-in var(--motion-disclosure, 180ms) var(--ease-out, ease-out) both;
	}
	.exr-reconnecting i {
		flex: none;
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--state-warning, #b26a00);
		animation: exr-pulse 1.4s ease-in-out infinite;
	}
	.exr-reconnecting span {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.exr-reconnecting .exr-retry {
		margin: 0 0 0 auto;
	}
	@keyframes exr-reconnecting-in {
		from {
			opacity: 0;
		}
	}
	@keyframes exr-pulse {
		50% {
			opacity: 0.35;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.exr-reconnecting,
		.exr-reconnecting i {
			animation: none;
		}
	}
	.exr-retry {
		flex: none;
		display: inline-block;
		margin-top: 8px;
		border: 0;
		padding: 0;
		background: none;
		color: var(--intent-conversation);
		font: inherit;
		font-weight: 600;
		cursor: pointer;
	}
	.exr-earlier {
		align-self: center;
		margin: 6px 0 2px;
		border: 0;
		padding: 4px 8px;
		background: none;
		color: var(--intent-conversation);
		font: inherit;
		font-size: var(--t-label);
		font-weight: 600;
		cursor: pointer;
	}
	.exr-earlier:focus-visible,
	.exr-retry:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 3px;
	}
	.rail-back {
		width: 34px;
		height: 34px;
		flex: none;
		display: grid;
		place-items: center;
		padding: 0;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--highlight) 58%, transparent);
		color: var(--text-secondary);
		cursor: pointer;
	}
	.rail-back:focus-visible {
		outline: 2px solid var(--intent-conversation);
		outline-offset: 2px;
	}
	.review-controls {
		flex: none;
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.review-feedback {
		display: grid;
		gap: 8px;
		margin: 0 16px 12px;
		padding: 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-control);
		background: var(--surface);
	}
	.review-feedback label {
		display: grid;
		gap: 6px;
		font-size: var(--t-label);
		font-weight: 500;
	}
	.review-feedback textarea {
		box-sizing: border-box;
		width: 100%;
		resize: vertical;
		padding: 8px 10px;
		border: 1px solid var(--control-edge);
		border-radius: var(--radius-control);
		background: var(--surface);
		color: var(--ink);
		font: inherit;
	}
	.review-feedback button {
		justify-self: start;
	}
	.review-controls :global(.hold-approve) {
		min-width: 122px;
		white-space: nowrap;
	}
	.exr-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.review-error {
		margin: 0;
		padding: 9px 14px;
		border-bottom: 1px solid color-mix(in srgb, var(--danger) 28%, var(--border));
		background: color-mix(in srgb, var(--danger) 6%, var(--surface-raised));
		font-size: var(--t-label);
		line-height: 1.4;
		color: var(--danger);
	}
	.review-empty {
		width: calc(100% - 36px);
		max-width: 360px;
		padding: 0;
		text-align: left;
	}
	.review-empty-card {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr);
		align-items: center;
		gap: 12px;
		padding: 14px 15px;
		border: 1px solid color-mix(in srgb, var(--intent-feedback) 34%, var(--border));
		border-left: 3px solid var(--intent-feedback);
		border-radius: var(--radius-pane);
		background: linear-gradient(
			145deg,
			color-mix(in srgb, var(--intent-feedback-soft) 82%, var(--surface-raised)),
			color-mix(in srgb, var(--intent-feedback-soft) 58%, var(--surface-alt))
		);
		box-shadow: var(--shadow-soft);
	}
	.review-empty-mark {
		width: 38px;
		height: 38px;
		display: grid;
		place-items: center;
		border: 1px solid color-mix(in srgb, var(--intent-feedback) 28%, var(--border));
		border-radius: var(--radius-control);
		background: color-mix(in srgb, var(--intent-feedback-soft) 76%, var(--surface-raised));
		box-shadow: var(--control-depth);
		color: var(--intent-feedback);
	}
	.review-empty-card strong {
		display: block;
		color: var(--ink);
		font-size: var(--t-body);
		font-weight: 600;
		line-height: 1.3;
	}
	.review-empty-card p {
		margin: 3px 0 0;
		color: var(--text-secondary);
		font-size: var(--t-label);
		line-height: 1.45;
	}
</style>
