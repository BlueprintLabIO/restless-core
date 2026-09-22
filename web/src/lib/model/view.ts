/** The small owner-surface contract. Source-owned reads map into these shapes. */

export interface NativeDocumentAttention {
	id: string;
	document_id: string;
	title: string;
	summary: string;
	requested_by_actor_id: string;
	created_at: string;
	kind: 'collaboration' | 'review';
	named_version_id: string | null;
}

export interface AttentionItem {
	preparing?: boolean;
	nativeDocument?: NativeDocumentAttention;
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
