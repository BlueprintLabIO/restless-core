/* One lightweight client projection for a conversation.
 *
 * OrgIntel remains the record: completed messages always come back through
 * getActorConversation. This source only owns the ephemeral bridge between a
 * durable owner message and its durable reply. It is shared by the Exec rail,
 * People and prepared handoffs so those surfaces cannot disagree about whether
 * a reply is queued, live, reconnecting or complete. */

import {
	getActorConversation,
	openActorConversationStream,
	sendActorMessage,
	type ActorConversation,
	type ConversationLiveState,
	type MessageSendResult
} from './attention';
import type { ThreadMessage } from './view';

export type ConversationSourceStatus = 'unknown' | 'live' | 'stale';
export type ConversationTransport = 'idle' | 'connecting' | 'live' | 'reconnecting';

export interface ActiveConversationTurn {
	triggerMessageId: number;
	since: Date | string;
	live: ConversationLiveState | null;
	transport: ConversationTransport;
}

const POLL_MS = 8_000;

export class ConversationSource {
	readonly companyId: string;
	readonly actorId: string;
	readonly workId?: string;

	actor = $state<ActorConversation['actor'] | null>(null);
	messages = $state<ThreadMessage[]>([]);
	live = $state<ConversationLiveState | null>(null);
	status = $state<ConversationSourceStatus>('unknown');
	transport = $state<ConversationTransport>('idle');
	failure = $state<(Error & { status?: number }) | null>(null);
	focusAfterMessageId = $state(0);
	focusStartedAt = $state<string | null>(null);
	triggerMessageId = $state<number | null>(null);
	pendingSince = $state<Date | string | null>(null);

	#consumers = 0;
	#timer: ReturnType<typeof setInterval> | undefined;
	#inFlight: Promise<void> | null = null;
	#stopStream: (() => void) | null = null;

	constructor(companyId: string, actorId: string, workId?: string) {
		this.companyId = companyId;
		this.actorId = actorId;
		this.workId = workId;
	}

	get activeTurn(): ActiveConversationTurn | null {
		if (this.triggerMessageId === null || this.pendingSince === null) return null;
		return {
			triggerMessageId: this.triggerMessageId,
			since: this.pendingSince,
			live: this.live,
			transport: this.transport
		};
	}

	refresh(showFailure = false): Promise<void> {
		this.#inFlight ??= this.#load(showFailure).finally(() => {
			this.#inFlight = null;
		});
		return this.#inFlight;
	}

	async #load(showFailure: boolean): Promise<void> {
		try {
			const conversation = await getActorConversation(this.companyId, this.actorId, this.workId);
			const actorDisplay = conversation.actor.id === 'exec' ? 'Exec' : conversation.actor.display;
			this.actor = { ...conversation.actor, display: actorDisplay };
			this.focusAfterMessageId = conversation.focus?.after_message_id ?? 0;
			this.focusStartedAt = conversation.focus?.started_at ?? null;
			this.messages = conversation.messages.map((message) => ({
				id: String(message.id),
				from: message.from_actor === 'owner' ? 'you' : 'agent',
				author: message.from_actor === 'owner' ? 'You' : actorDisplay,
				text: message.body,
				createdAt: message.created_at,
				replyToMessageId: null,
				assetId: null,
				runId: null,
				attachments: message.attachments ?? [],
				details: message.details ?? null,
				intent: message.intent ?? null,
				contextPath: message.context_path ?? null
			}));
			this.status = 'live';
			this.failure = null;
			this.#recoverTurn();
		} catch (cause) {
			if (showFailure || this.messages.length === 0) {
				this.failure = cause as Error & { status?: number };
			}
			this.status = this.messages.length ? 'stale' : 'unknown';
		}
	}

	async send(
		body: string,
		files: File[] = [],
		contextPath?: string,
		newFocus = false
	): Promise<MessageSendResult> {
		const result = await sendActorMessage(
			this.companyId,
			this.actorId,
			body,
			this.workId,
			files,
			contextPath,
			newFocus
		);
		if (result.focus) {
			this.focusAfterMessageId = result.focus.afterMessageId;
			this.focusStartedAt = result.focus.startedAt;
		}
		const sentAt = new Date();
		if (!this.messages.some((message) => message.id === String(result.messageId))) {
			this.messages = [
				...this.messages,
				{
					id: String(result.messageId),
					from: 'you',
					author: 'You',
					text: body,
					createdAt: sentAt,
					replyToMessageId: null,
					assetId: null,
					runId: null,
					attachments: [],
					contextPath: contextPath ?? null
				}
			];
		}
		this.#follow(result.messageId, sentAt);
		void this.refresh();
		return result;
	}

	attach(): () => void {
		this.#consumers += 1;
		if (this.#consumers === 1) {
			void this.refresh();
			this.#timer = setInterval(() => void this.refresh(), POLL_MS);
		}
		let released = false;
		return () => {
			if (released) return;
			released = true;
			this.#consumers -= 1;
			if (this.#consumers === 0) {
				clearInterval(this.#timer);
				this.#timer = undefined;
				this.#stopStream?.();
				this.#stopStream = null;
				if (this.triggerMessageId !== null) this.transport = 'idle';
			}
		};
	}

	#recoverTurn(): void {
		const last = this.messages.at(-1);
		if (last?.from === 'you') {
			const messageId = Number(last.id);
			if (Number.isFinite(messageId)) this.#follow(messageId, last.createdAt);
			return;
		}
		if (this.triggerMessageId !== null) this.#clearTurn();
	}

	#follow(messageId: number, since: Date | string): void {
		if (this.triggerMessageId === messageId && this.#stopStream) return;
		const changedTurn = this.triggerMessageId !== messageId;
		this.#stopStream?.();
		this.triggerMessageId = messageId;
		this.pendingSince = since;
		if (changedTurn) this.live = null;
		this.transport = 'connecting';
		this.#stopStream = openActorConversationStream(
			this.companyId,
			this.actorId,
			messageId,
			(state) => {
				if (this.triggerMessageId !== messageId) return;
				this.live = state;
				this.transport = 'live';
				if (state.phase === 'failed') {
					this.#stopTerminalStream();
				} else if (state.phase === 'complete') {
					this.#stopTerminalStream();
					this.#refreshAfterCurrent(messageId);
				}
			},
			() => {
				if (this.triggerMessageId === messageId) this.transport = 'reconnecting';
			}
		);
	}

	#stopTerminalStream(): void {
		this.#stopStream?.();
		this.#stopStream = null;
		this.transport = 'idle';
	}

	/* The optimistic post-send refresh may still be in flight when the terminal
	 * SSE event arrives. Queue one read behind it so a fast reply cannot wait for
	 * the next polling tick before replacing the dock with its durable message. */
	#refreshAfterCurrent(messageId: number): void {
		const current = this.#inFlight;
		if (!current) {
			void this.refresh();
			return;
		}
		void current.then(() => {
			if (this.triggerMessageId === messageId && this.live?.phase === 'complete') {
				void this.refresh();
			}
		});
	}

	#clearTurn(): void {
		this.#stopStream?.();
		this.#stopStream = null;
		this.triggerMessageId = null;
		this.pendingSince = null;
		this.live = null;
		this.transport = 'idle';
	}
}

const sources = new Map<string, ConversationSource>();

export function conversationSource(
	companyId: string,
	actorId: string,
	workId?: string
): ConversationSource {
	const key = `${companyId}:${actorId}:${workId ?? ''}`;
	let source = sources.get(key);
	if (!source) {
		source = new ConversationSource(companyId, actorId, workId);
		sources.set(key, source);
	}
	return source;
}
