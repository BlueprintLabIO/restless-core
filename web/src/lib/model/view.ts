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
	actions: Array<{
		id: string;
		label: string;
		consequence: string;
	}>;
	canContinue: boolean;
	createdAt: Date | string;
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
	intent?: MessageIntentReceipt | null;
	contextPath?: string | null;
}
