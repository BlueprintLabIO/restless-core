/** The small owner-surface contract. Source-owned reads map into these shapes. */

export interface AttentionItem {
	id: string;
	workId?: string;
	source: {
		plane: 'authority' | 'orgintel' | 'runtime' | string;
		kind: string;
		reference: string;
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
		status: 'available' | 'unavailable';
		kind: 'runtime-web';
		label: string;
	};
	actions: Array<{
		id: string;
		label: string;
		consequence: string;
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
export function mergeAdjacentAgentMessages(messages: ThreadMessage[]): ThreadMessage[] {
	const merged: ThreadMessage[] = [];

	for (const message of messages) {
		const previous = merged.at(-1);
		const sameDay = previous && dayKey(previous.createdAt) === dayKey(message.createdAt);
		if (
			previous?.from === 'agent' &&
			message.from === 'agent' &&
			previous.author === message.author &&
			sameDay
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
