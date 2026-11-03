/* Owner-facing copy is written by the platform, in plain words, by state. An agent's or the
 * system's own reason (abandon notes, resolutions, run summaries) can carry ids, hashes, agent
 * handles and message numbers; those belong in Activity's details, never in a notice or a status
 * line. */

import type { DecisionContinuation } from './view';

/* 8+ hex characters with a digit among them (a commit, a short or full uuid), never a word. */
const HEX_ID = /\b(?=[0-9a-f]*\d)[0-9a-f]{8,}\b/i;
const MESSAGE_NUMBER = /\b(?:msg|message)\s*#?\d+/i;
/* An agent's handle: lowercase, hyphenated, ending in a role (`reno-supervisor`, `ops-lead`). */
const AGENT_HANDLE =
	/\b[a-z][a-z0-9]*(?:-[a-z0-9]+)*-(?:supervisor|lead|exec|staff|agent|worker|writer|reviewer|bot|ops)\b/;
/* Engine nouns an owner never needs to read. */
const INTERNAL_NOUN =
	/\b(?:delivery node|work node|digests?|bound to|handoff|attempt_id|work_id|orgintel|authority record)\b/i;

/** True when a line carries ids, hashes, message numbers, agent handles or engine nouns. */
export function leaksInternals(text: string, actorIds: readonly string[] = []): boolean {
	if (HEX_ID.test(text) || MESSAGE_NUMBER.test(text) || AGENT_HANDLE.test(text)) return true;
	if (INTERNAL_NOUN.test(text)) return true;
	return actorIds.some(
		(id) =>
			/[-_]/.test(id) && new RegExp(`\\b${id.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\b`).test(text)
	);
}

/** A recorded line the owner may read, or the platform's own line when it leaks internals. */
export function ownerLine(
	text: string,
	fallback: string,
	actorIds: readonly string[] = []
): string {
	const said = text.trim();
	return said && !leaksInternals(said, actorIds) ? said : fallback;
}

export type DecidedOutcome = NonNullable<DecisionContinuation['outcome']>;

export interface DecidedNotice {
	/** The item's title, when it reads cleanly; the notice still makes sense without it. */
	title: string;
	text: string;
}

type Decided = Pick<
	DecisionContinuation,
	'title' | 'outcome' | 'decidedBy' | 'byOwner' | 'viaChat' | 'replacementItemId'
>;

/* "<who> approved this." in every voice the record allows. */
function decidedBy(
	decision: Decided,
	viewerIsOwner: boolean,
	verbs: { past: string; already: string; passive: string }
): string {
	if (decision.viaChat)
		return `Exec ${verbs.past} this from ${viewerIsOwner ? 'your' : 'the owner’s'} chat.`;
	if (decision.byOwner)
		return viewerIsOwner ? `You already ${verbs.already} this.` : `The owner ${verbs.past} this.`;
	if (decision.decidedBy) return `${decision.decidedBy} ${verbs.past} this.`;
	return `This was already ${verbs.passive}.`;
}

/**
 * What an old link to an Inbox item says once it is settled. Built only from the decision's
 * state; the record's own reason stays behind "See it in Activity".
 */
export function decidedNotice(
	decision: Decided,
	options: { viewerIsOwner: boolean; replacementInInbox?: boolean }
): DecidedNotice {
	const { viewerIsOwner } = options;
	const title = ownerLine(decision.title, '');
	let text: string;
	switch (decision.outcome) {
		case 'replaced':
			text = options.replacementInInbox
				? `This was replaced by an updated version. The new one is in ${viewerIsOwner ? 'your' : 'the'} Inbox.`
				: `This was replaced by an updated version. If it needs ${viewerIsOwner ? 'you' : 'the owner'}, it will come to ${viewerIsOwner ? 'your' : 'the'} Inbox.`;
			break;
		case 'withdrawn':
			text = 'Withdrawn: this is no longer needed.';
			break;
		case 'expired':
			text = 'This expired before it was decided.';
			break;
		case 'declined':
			text = decidedBy(decision, viewerIsOwner, {
				past: 'declined',
				already: 'declined',
				passive: 'declined'
			});
			break;
		case 'changes_requested':
			text = decidedBy(decision, viewerIsOwner, {
				past: 'asked for changes to',
				already: 'asked for changes to',
				passive: 'sent back for changes'
			});
			break;
		case 'completed':
			text = decidedBy(decision, viewerIsOwner, {
				past: 'did',
				already: 'did',
				passive: 'done'
			});
			break;
		case 'revoked':
			text = decidedBy(decision, viewerIsOwner, {
				past: 'took back approval for',
				already: 'took back approval for',
				passive: 'taken back'
			});
			break;
		case 'approved':
			text = decidedBy(decision, viewerIsOwner, {
				past: 'approved',
				already: 'approved',
				passive: 'approved'
			});
			break;
		default:
			text = 'This was already decided.';
	}
	return { title, text };
}

/** When an old link's item has no record yet (or the record could not be read). */
export const RESOLVED_ELSEWHERE = 'That item has been resolved or is no longer in the Inbox.';
