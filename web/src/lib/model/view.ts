/** The small owner-surface contract. Source-owned reads map into these shapes. */

export interface AttentionItem {
	id: string;
	workId?: string;
	source: {
		plane: 'authority' | 'orgintel' | 'runtime' | string;
		kind: string;
		reference: string;
		/** Authority-only: the exact party the typed approval controls. */
		party?: string;
	};
	category: 'approval' | 'review' | 'blocker' | string;
	title: string;
	whatHappened: string;
	whyItMatters: string;
	recommendation: string;
	requestedAction: string;
	ifNoAction: string;
	uncertainty?: string;
	deadline?: string;
	briefStatus: 'current' | 'source-authored' | 'human-fallback' | string;
	briefAuthor?: {
		id: string;
		display: string;
		role: string;
	};
	briefedAt?: Date | string;
	evidence: Array<{
		label: string;
		kind: string;
		uri?: string;
		content?: string;
	}>;
	reviewSources: Array<{
		label: string;
		provider: string;
		reference: string;
		verification: string;
		uri?: string;
		content: string;
		observedAt: Date | string;
	}>;
	responsibleActor?: {
		id: string;
		display: string;
		role: string;
	};
	runtimeAttach?: {
		company: string;
		generation: string;
		requestingActor?: string;
		requestingActorDisplay?: string;
		kind: 'persistent-browser';
	};
	reviewTarget?: {
		company: string;
		generation: string;
		uri: string;
		status: 'available' | 'unavailable';
		kind: 'runtime-web' | 'runtime-text' | 'runtime-file';
		label: string;
		content?: string;
		unavailableReason?: string;
	};
	actions: Array<{
		id: string;
		label: string;
		role: 'decision' | 'inspect' | 'conversation' | 'human_step' | string;
		consequence: string;
		nextState: string;
		href?: string;
	}>;
	canContinue: boolean;
	createdAt: Date | string;
}

export interface DecisionContinuation {
	id: string;
	workId: string;
	title: string;
	recordedDecision: string;
	whatItUnlocked: string;
	currentState: string;
	observedOutcome: string;
	responsibleActor?: {
		id: string;
		display: string;
		role: string;
	};
	observedAt: Date | string;
}

export type NeedsYouItem = AttentionItem;

export interface ThreadSummary {
	key: string;
	kind: 'executive' | 'agent' | 'goal';
	conversationId: string | null;
	subjectId: string | null;
	title: string;
	subtitle: string;
	pig: number;
	live: boolean;
	preview: string;
	lastAt: Date | string | null;
	messageCount: number;
}

export interface MessageAttachment {
	uploadId: string;
	name: string;
	mediaType: string;
	sizeBytes: number;
}

export interface MessageIntentReceipt {
	kind: 'conversation' | 'work_feedback' | 'direction' | 'authority';
	summary: string;
	outcome?: string | null;
	nextStep?: string | null;
	ownerNeed?: string | null;
}

export interface ThreadMessage {
	id: string;
	from: 'you' | 'agent' | 'system';
	author: string;
	text: string;
	createdAt: Date | string;
	replyToMessageId: string | null;
	assetId: string | null;
	runId: string | null;
	attachments: MessageAttachment[];
	details?: string | null;
	intent?: MessageIntentReceipt | null;
	contextPath?: string | null;
}

/** Collapse an uninterrupted run from the same company actor into one reading
 * block. This is a presentation projection only: the source messages remain
 * separate in the company record. Day boundaries stay visible. */
export function mergeAdjacentAgentMessages(
	messages: ThreadMessage[],
	breakAfterMessageId?: number
): ThreadMessage[] {
	const merged: ThreadMessage[] = [];

	for (const message of messages) {
		const previous = merged.at(-1);
		const sameDay = previous && dayKey(previous.createdAt) === dayKey(message.createdAt);
		const previousLastId = Number(previous?.id.split(':').at(-1));
		const messageFirstId = Number(message.id.split(':')[0]);
		const crossesBreak =
			breakAfterMessageId !== undefined &&
			Number.isFinite(previousLastId) &&
			Number.isFinite(messageFirstId) &&
			previousLastId <= breakAfterMessageId &&
			messageFirstId > breakAfterMessageId;
		if (
			previous?.from === 'agent' &&
			message.from === 'agent' &&
			previous.author === message.author &&
			sameDay &&
			!crossesBreak
		) {
			previous.id = `${previous.id}:${message.id}`;
			previous.text = [previous.text, message.text].filter(Boolean).join('\n\n');
			previous.attachments.push(...message.attachments);
			previous.details = [previous.details, message.details].filter(Boolean).join('\n\n') || null;
			continue;
		}

		merged.push({ ...message, attachments: [...message.attachments] });
	}

	return merged;
}

function dayKey(value: Date | string): string {
	const date = value instanceof Date ? value : new Date(value);
	return Number.isNaN(date.getTime()) ? String(value) : date.toDateString();
}
