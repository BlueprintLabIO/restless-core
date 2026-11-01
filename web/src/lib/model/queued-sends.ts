/* Queued sends (S109 F7): a message written while the agent is still replying waits for that
 * reply to end. It is not sent yet, so it never draws as a sent message: it waits above the
 * composer, saying when it goes, with Edit and Cancel, until the agent's next turn takes it. */

import type { AgentActivityPhase } from './generated/conversation';

type Phase = AgentActivityPhase | null | undefined;

/** A turn under way: begun and not yet ended. A message sent now without interrupting waits
 * for it. A turn still waiting to begin takes a new message with it instead, so nothing queues. */
export function turnUnderWay(phase: Phase): boolean {
	return phase === 'thinking' || phase === 'acting' || phase === 'responding';
}

export function turnEnded(phase: Phase): boolean {
	return phase === 'complete' || phase === 'failed' || phase === 'unavailable';
}

/** Which message to follow next. While the followed turn is under way, a message queued behind
 * it is not followed (the reply in progress stays in view); once that turn ends, the earliest
 * queued message is, since the agent's next turn takes every queued message together. */
export function nextFollow(input: {
	/** The newest message's id when the viewer wrote it, otherwise null. */
	lastFromViewer: number | null;
	/** Messages sent behind the followed turn, still waiting. */
	queued: readonly number[];
	following: number | null;
	phase: Phase;
}): number | null {
	/* null: keep following what is followed now. */
	const { lastFromViewer, queued, following, phase } = input;
	if (queued.length && (following === null || turnEnded(phase))) return Math.min(...queued);
	if (lastFromViewer === null || queued.includes(lastFromViewer)) return null;
	return lastFromViewer;
}

export type QueuedCandidate = {
	id: string;
	from: string;
	readAt?: Date | string | null;
	/** Set on a message sent behind a turn under way, before its saved id is known. */
	queued?: boolean;
};

function numericId(id: string): number {
	const value = Number(id.split(':').at(-1));
	return Number.isFinite(value) ? value : 0;
}

/** The viewer's messages still waiting behind the reply in progress, oldest first. */
export function queuedMessages<T extends QueuedCandidate>(
	messages: readonly T[],
	turn: { triggerMessageId: number } | null | undefined,
	working: boolean
): T[] {
	if (!working || !turn) return [];
	return messages.filter(
		(message) =>
			message.from === 'you' &&
			!message.readAt &&
			(message.queued === true || numericId(message.id) > turn.triggerMessageId)
	);
}

export type QueuedRowState = 'queuing' | 'queued' | 'cancelling' | 'failed';

/** The row's state: a send still on its way has no saved id to edit or cancel yet. */
export function queuedRowState(
	message: { id: string },
	busy: { has(id: string): boolean },
	failed: { has(id: string): boolean }
): QueuedRowState {
	if (message.id.startsWith('optimistic:') || numericId(message.id) <= 0) return 'queuing';
	if (busy.has(message.id)) return 'cancelling';
	if (failed.has(message.id)) return 'failed';
	return 'queued';
}

/** When it goes, or what is happening to it: one short label in the row's fixed slot. */
export function queuedScheduleLabel(state: QueuedRowState): string {
	switch (state) {
		case 'queuing':
			return 'Queuing…';
		case 'cancelling':
			return 'Cancelling…';
		case 'failed':
			return 'Still queued';
		default:
			return 'After this reply';
	}
}
