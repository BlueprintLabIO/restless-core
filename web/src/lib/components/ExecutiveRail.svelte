<script lang="ts">
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
	import type { ActiveAgentTurn, QuerySourceStatus } from '$lib/model/queries.svelte';
	import type { OutcomeStandard } from '$lib/model/company';
	import type { ThreadMessage } from '$lib/model/view';
	import { runExecCommand } from '$lib/model/skills';
	import { PIN, reactionsQuery } from '$lib/model/reactions.svelte';
	import { workDraftFrom, askExec } from '$lib/model/ask-exec';
	import Pin from '@lucide/svelte/icons/pin';
	import Link from '@lucide/svelte/icons/link';
	import ListPlus from '@lucide/svelte/icons/list-plus';
	import { recentDirectConversationsQuery } from '$lib/model/room-queries.svelte';
	import { deleteRoomMessage } from '$lib/model/rooms';
	import { markSeen, seenThrough as roomSeenThrough } from '$lib/model/conversation-seen';
	import { composerOptions, readDraft, writeDraft } from '$lib/model/composer-options.svelte';
	import { agentExchangesQuery, type AgentExchange } from '$lib/model/exchanges.svelte';
	import HandoffGroup from '$lib/primitives/HandoffGroup.svelte';

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
		focusMessage = 0
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
					skills?: string[]
			  ) => Promise<{ error?: string; notice?: string }>)
			| null;
		review?: {
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
	} = $props();
	let focusedFor = 0;
	$effect(() => {
		if (!focusMessage || focusMessage === focusedFor) return;
		const target = messages.find((message) => messageNumericId(message.id) === focusMessage);
		if (!target) return;
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

	const canOperate = $derived(['owner', 'operator'].includes(membershipRole ?? ''));

	/* Pinned messages stay in reach above the conversation. */
	const pinnedMessages = $derived.by(() => {
		const ids = reactions.pinned();
		return messages.filter((message) => ids.has(messageNumericId(message.id)));
	});
	let pinsOpen = $state(false);
	function togglePin(message: ThreadMessage) {
		const id = messageNumericId(message.id);
		if (id > 0) void reactions.react(id, PIN, !reactions.pinned().has(id));
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
	function turnIntoWork(message: ThreadMessage) {
		const draft = workDraftFrom(
			message.from === 'you' ? 'You' : message.author || participantName,
			message.text
		);
		if (participantId === 'exec' && !review && !workContext) {
			composer = draft;
			composerFocusKey += 1;
		} else askExec(draft);
	}

	/* Handoffs between agents appear where they happened in time. */
	const exchanges = agentExchangesQuery(
		() => companyId,
		() => participantId,
		() => open && membershipRole === 'owner'
	);
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
		if (!previous) return [];
		const after = at(previous.createdAt);
		const until = at(messages[windowStart + index].createdAt);
		return exchanges.exchanges.filter(
			(exchange) => at(exchange.created_at) > after && at(exchange.created_at) <= until
		);
	}
	const handoffsSinceLast = $derived.by(() => {
		const last = visibleMessages.at(-1);
		return last
			? exchanges.exchanges.filter((exchange) => at(exchange.created_at) > at(last.createdAt))
			: [];
	});

	/* `$skill` selection and the `/goal` and `/loop` commands. Skills are the
	 * company library; the commands are Restless primitives offered only in the
	 * Exec conversation, where they have one accountable meaning. */
	let composerSkills = $state<string[]>([]);
	const execConversation = $derived(participantId === 'exec' && !review && !workContext);
	const commandsAllowed = $derived(execConversation && membershipRole === 'owner');
	const options = composerOptions(
		() => companyId,
		() => open && canOperate,
		() => commandsAllowed
	);

	/* Returns a notice for commands handled without a message, or send: true. */
	async function runCommand(
		text: string
	): Promise<{ notice?: string; error?: string; send: boolean }> {
		return commandsAllowed ? runExecCommand(companyId, text) : { send: true };
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
	/* After the saved draft loads, so a requested draft is not overwritten by it. */
	let appliedDraft = 0;
	$effect(() => {
		if (!draftRequest || draftRequest.key === appliedDraft) return;
		appliedDraft = draftRequest.key;
		composer = draftRequest.text;
		untrack(() => (composerFocusKey += 1));
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
	let reviewError = $state('');
	let reviewFeedback = $state('');
	let deciding = $state(false);
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
	const openQuestion = $derived.by(() => {
		const last = visibleMessages.at(-1);
		return last && last.from !== 'you' ? (last.intent?.ownerNeed?.trim() ?? '') : '';
	});
	/* A receipt under your latest message: sent, or seen when the agent's turn
	 * consumed it without replying. While it works, the turn dock says so;
	 * once it replies, the reply is the receipt. */
	const directRooms = $derived(recentDirectConversationsQuery(companyId, () => open && canOperate));
	const directRoom = $derived(
		directRooms.conversations.find((entry) => entry.person_actor_id === participantId)?.room_id ??
			''
	);
	$effect(() => {
		const latest = messages.reduce((max, m) => Math.max(max, messageNumericId(m.id)), 0);
		if (open && directRoom && latest) markSeen(companyId, directRoom, latest);
	});
	let seenVersion = $state(0);
	$effect(() => {
		const bump = () => (seenVersion += 1);
		window.addEventListener('restless:seen', bump);
		return () => window.removeEventListener('restless:seen', bump);
	});
	/* Another conversation in the switcher with something new since you looked. */
	function topicHasNew(entry: { key: string; topic: { actorId: string; workId?: string } | null }) {
		void seenVersion;
		if (entry.key === currentTopicKey || entry.topic?.workId) return false;
		const actor = entry.topic?.actorId ?? 'exec';
		const conversation = directRooms.conversations.find((c) => c.person_actor_id === actor);
		return (
			!!conversation &&
			conversation.last_message_id > roomSeenThrough(companyId, conversation.room_id)
		);
	}
	const anyTopicNew = $derived(topics.some((entry) => topicHasNew(entry)));
	/* Until the agent picks it up, your last message can be taken back: retract
	 * removes it, edit returns its words to the composer. */
	const retractable = $derived.by(() => {
		const last = visibleMessages.at(-1);
		return !!last &&
			last.from === 'you' &&
			!last.readAt &&
			!turn &&
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
			await deleteRoomMessage(
				companyId,
				directRoom,
				messageNumericId(last.id),
				crypto.randomUUID()
			);
			if (restore) {
				composer = last.text;
				composerFocusKey += 1;
			}
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
	const receipt = $derived.by(() => {
		const last = visibleMessages.at(-1);
		if (!last || last.from !== 'you' || turn || last.id.startsWith('optimistic:')) return '';
		return last.readAt ? `Seen by ${participantName}` : 'Sent';
	});
	const openReplies = $derived.by(() =>
		openQuestion ? (visibleMessages.at(-1)?.intent?.ownerReplies ?? []).slice(0, 3) : []
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
		if (
			!answer ||
			sending ||
			deciding ||
			!onask ||
			needsProvider ||
			connectionStatus !== 'available'
		)
			return;
		scrollReset += 1;
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
				!!turn && interruptNext,
				undefined,
				[]
			);
			if (outcome.error) {
				askError = outcome.error;
			} else {
				fieldValues = {};
				newFocusPending = false;
				askNotice = outcome.notice ?? '';
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
		openQuestion ? (visibleMessages.at(-1)?.intent?.ownerFields ?? []).slice(0, 4) : []
	);
	let fieldValues = $state<Record<string, string>>({});
	let fieldsFor = '';
	$effect(() => {
		const key = `${visibleMessages.at(-1)?.id ?? ''}`;
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
		const message = visibleMessages[index];
		if (!message?.intent?.ownerNeed || message.from === 'you') return '';
		/* A later ask supersedes this one; only your next word before it answers. */
		for (const later of visibleMessages.slice(index + 1)) {
			if (later.from === 'you') return shortTime(later.createdAt);
			if (later.intent?.ownerNeed) return '';
		}
		return '';
	}

	/* Replying to one message quotes it, so a reply to a long message says
	 * which point it answers. The quote travels as ordinary Markdown. */
	let quoting = $state<{ author: string; excerpt: string; answer?: boolean } | null>(null);
	function excerptOf(text: string): string {
		const plain = text
			.replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
			.replace(/[*_`>#]/g, '')
			.replace(/\s+/g, ' ')
			.trim();
		return plain.length > 180 ? `${plain.slice(0, 180)}…` : plain;
	}
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
	async function showEarlier(through = windowStart) {
		const el = scrollEl;
		const before = el ? el.scrollHeight - el.scrollTop : 0;
		el?.dispatchEvent(new Event('chat-scroll-pause'));
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

	function beginNewFocus() {
		if (!newFocusAvailable || !connected || turn || sending || review || workContext) return;
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

	function toggleContext() {
		includeContext = !includeContext;
		if (includeContext) contextFlare += 1;
	}

	const attachmentHref = (attachment: { uploadId: string }) =>
		`/api/companies/${encodeURIComponent(companyId)}/attachments/${encodeURIComponent(attachment.uploadId)}`;

	let sending = $state(false);
	async function submitAsk(event: SubmitEvent) {
		event.preventDefault();
		await deliver();
	}

	async function deliver() {
		const text = composer.trim();
		if (!text || sending || deciding || !onask || needsProvider || connectionStatus !== 'available')
			return;
		scrollReset += 1;
		sending = true;
		askError = '';
		askNotice = '';
		const sent = composer;
		const quoted = quoting;
		const outgoing = quoted ? `> ${quoted.author}: ${quoted.excerpt}\n\n${text}` : text;
		const files = composerFiles;
		const skills = composerSkills;
		composer = '';
		try {
			const command = await runCommand(text);
			if (!command.send) {
				if (command.error) {
					composer = sent;
					askError = command.error;
				} else {
					askNotice = command.notice ?? '';
				}
				return;
			}
			const outcome = await onask(
				outgoing,
				files,
				includeContext,
				newFocusPending,
				!!turn && interruptNext,
				undefined,
				skills
			);
			if (outcome.error) {
				composer = sent;
				askError = outcome.error;
			} else {
				composerFiles = [];
				composerSkills = [];
				quoting = null;
				fieldValues = {};
				newFocusPending = false;
				askNotice = outcome.notice ?? '';
			}
		} catch (cause) {
			composer = sent;
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
			else composer = '';
		} finally {
			deciding = false;
		}
	}
</script>

<aside
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
									/><strong class="exr-name">{participantName}</strong>{#if topicLabel}<span
											class="topic-label">{topicLabel}</span
										>{/if}{#if anyTopicNew}<i
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
							title="New conversation · fresh working context, company memory kept"
							disabled={!connected || !!turn || sending}
							onclick={beginNewFocus}><SquarePen size={15} strokeWidth={1.9} /></button
						>
					{/if}
					{#if visibleMessages.length}
						<ConversationHistoryTools {messages} {participantName} onjump={jumpToMessage} />
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
						disabled={deciding || sending}
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
						disabled={!canOperate || deciding}></textarea></label
				>
				<button class="btn small" disabled={!canOperate || deciding || !reviewFeedback.trim()}
					>{deciding ? 'Recording…' : 'Request changes'}</button
				>
			</form>
		{/if}

		{#if pinnedMessages.length}
			<details class="pinned" bind:open={pinsOpen}>
				<summary
					><Pin size={12} aria-hidden="true" />{pinnedMessages.length} pinned{#if !pinsOpen}<span
							class="pinned-first">{pinnedMessages.at(-1)?.text.slice(0, 80)}</span
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
						><span>{message.text.replace(/\s+/g, ' ').slice(0, 140)}</span></button
					>
				{/each}
			</details>
		{/if}
		<div class="exr-panel">
			{#if !needsProvider && connectionStatus === 'unknown'}
				<!-- Unknown is the ordinary first moment; the transcript skeleton below
				     already says so without a sentence about it. -->
				<p class="sr-only" role="status">Checking {participantName}'s conversation status…</p>
			{:else if !needsProvider && connectionStatus === 'error'}
				<p class="exr-connection-notice" role="status">
					Connection status could not be refreshed. Try again shortly.
				</p>
			{:else if !needsProvider && connectionStatus === 'unavailable'}
				<p class="exr-connection-notice" role="status">
					A conversation route for {participantName} is not available in the latest company status.
				</p>
			{/if}
			{#if conversationFailed && conversationStatus === 'stale'}
				<p class="exr-connection-notice" role="status">
					Recent messages could not be refreshed. Showing the last loaded conversation.
					<button type="button" class="exr-retry" onclick={() => onrefreshConversation?.()}
						>Try again</button
					>
				</p>
			{/if}
			<div class="exr-chat">
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
					{#if everOpened && windowStart}
						<button type="button" class="exr-earlier" onclick={() => void showEarlier()}
							>Show earlier messages</button
						>
					{/if}
					{#each everOpened ? visibleMessages : [] as message, i (message.id)}
						{#if focusDividerBefore(i)}
							<div class="conversation-focus-boundary">
								<span>New focus</span><i aria-hidden="true"></i><span>Company memory retained</span>
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
						>
							{#snippet actions()}{#if canOperate && messageNumericId(message.id) > 0}<button
										type="button"
										class="copy-message"
										class:confirmed={reactions.pinned().has(messageNumericId(message.id))}
										aria-label={reactions.pinned().has(messageNumericId(message.id))
											? 'Unpin this message'
											: 'Pin this message'}
										title={reactions.pinned().has(messageNumericId(message.id))
											? 'Unpin'
											: 'Pin above the conversation'}
										onclick={() => togglePin(message)}><Pin size={12} aria-hidden="true" /></button
									><button
										type="button"
										class="copy-message"
										aria-label="Copy link to this message"
										title="Copy link"
										onclick={() => void copyLink(message)}
										><Link size={12} aria-hidden="true" /></button
									><button
										type="button"
										class="copy-message"
										aria-label="Turn this into Work"
										title="Ask the Exec to turn this into Work"
										onclick={() => turnIntoWork(message)}
										><ListPlus size={12} aria-hidden="true" /></button
									>{/if}{#if message.from !== 'you' && onask}<button
										type="button"
										class="copy-message"
										aria-label="Reply to this message"
										title="Reply to this message"
										onclick={() => quote(message.author || participantName, message.text)}
										><Reply size={12} aria-hidden="true" /></button
									>{/if}{/snippet}
						</ConversationMessage>
						{#if openQuestion && i === visibleMessages.length - 1}
							<div class="open-question" role="group" aria-label={`${participantName} is asking`}>
								<p>{openQuestion}</p>
								{#if pendingAnswer}
									<p class="answer-pending" role="status">
										<span>Sending “{pendingAnswer.text}”</span><button
											type="button"
											onclick={undoReply}>Undo</button
										>
									</p>
								{:else if openReplies.length}
									<div class="open-replies">
										{#each openReplies as reply (reply)}
											<button
												type="button"
												disabled={!canOperate || sending}
												title="Send this answer"
												onclick={() => tapReply(reply)}>{reply}</button
											>
										{/each}
									</div>
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
						{:else}
							<div class="exr-empty">
								<p class="exr-empty-h">
									{needsProvider ? providerLabel : 'Ask anything.'}
								</p>
								<p class="exr-empty-p">
									{needsProvider
										? `Restore intelligence access so ${participantName} can reply.`
										: capabilityHint}
								</p>
								{#if needsProvider}
									<a
										class="btn primary small provider-connect"
										href={`/${companyId}/company/provider`}
										><Plus size={14} strokeWidth={2} aria-hidden="true" /><span
											>{providerLabel}</span
										></a
									>
								{/if}
							</div>
						{/if}
					{/each}
					{#if focusActive && !hasMessagesAfterFocus}
						<div class="conversation-focus-boundary" role={newFocusPending ? 'status' : undefined}>
							<span>New focus</span><i aria-hidden="true"></i><span>Company memory retained</span>
						</div>
						{#if !composer.trim() && !turn}
							<p class="conversation-capability-hint">{capabilityHint}</p>
						{/if}
					{/if}
					<HandoffGroup exchanges={handoffsSinceLast} name={exchanges.name} host={participantId} />
					{#if receipt}<p class="receipt" role="status">
							{#if retractable}<button
									type="button"
									disabled={retracting}
									onclick={() => retract(true)}>Edit</button
								><button type="button" disabled={retracting} onclick={() => retract(false)}
									>Retract</button
								><span aria-hidden="true">·</span>{/if}{receipt}
						</p>{/if}
					{#if turn && needsProvider && (turn.live?.phase ?? 'queued') === 'queued'}
						<p class="exr-setup-pending" role="status">
							Message saved. {participantName} can reply after intelligence access is restored.
						</p>
					{:else if turn}<ConversationTurnDock {participantName} {turn} />{/if}
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
					     above; otherwise it takes the composer's place. -->
					{#if !providerPromptInline}
						<a
							class="btn primary small provider-connect-slot"
							href={`/${companyId}/company/provider`}
							>{#if providerLabel.startsWith('Reconnect')}<RotateCw
									size={14}
									strokeWidth={2}
									aria-hidden="true"
								/>{:else}<Plus size={14} strokeWidth={2} aria-hidden="true" />{/if}<span
								>{providerLabel}</span
							></a
						>
					{/if}
				{:else}
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
							interruptible={!!turn}
							bind:interrupt={interruptNext}
							disabled={!canOperate ||
								sending ||
								deciding ||
								!onask ||
								connectionStatus !== 'available'}
							minlength={1}
							placeholder={openQuestion
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
											<span>{includeContext ? contextLabel : `Link ${contextLabel}`}</span>
										</button>
									</div>
								{/if}
							{/snippet}
						</Composer>
						{#if askError}
							<p class="exr-error" role="alert">{askError}</p>
						{/if}
						{#if askNotice}
							<p class="exr-notice" role="status">{askNotice}</p>
						{/if}
					</form>
				{/if}
			</div>
		</div>
	</div>
</aside>

<style>
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
		margin: 2px 14px 10px 45px;
		padding: 8px 0 0 12px;
		border-left: 2px solid color-mix(in srgb, var(--surface-attention) 70%, transparent);
	}
	.open-question > p {
		margin: 0;
		color: var(--ink);
		font-weight: 500;
		line-height: 1.4;
	}
	.answer-pending {
		display: flex;
		align-items: center;
		gap: 10px;
		color: var(--text-secondary);
		font-size: var(--t-label);
	}
	.open-question > .answer-pending {
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
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
	}
	.open-replies button {
		padding: 3px 10px;
		border: 1px solid var(--border-strong);
		border-radius: 999px;
		background: var(--surface-raised);
		color: var(--ink);
		font: inherit;
		font-size: var(--t-label);
		cursor: pointer;
	}
	.open-replies button:hover:not(:disabled) {
		border-color: var(--ink);
		background: var(--surface-hover);
	}
	.open-replies button:disabled {
		opacity: 0.5;
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
	.exr-retry {
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
