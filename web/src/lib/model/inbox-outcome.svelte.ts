/* An answer the owner gave outside the Inbox's own detail pane (the rail's review controls), held
 * until the Inbox confirms it the way it confirms every other answer: what happened, and where the
 * conversation it went to is. One hand-over, then it is gone. */

export interface InboxOutcome {
	text: string;
	href?: string;
	/** An accepted outcome: where it now lives, to open it from the confirmation. */
	result?: { href: string; label: string };
}

let pending = $state<InboxOutcome | null>(null);

export function handInboxOutcome(outcome: InboxOutcome) {
	pending = outcome;
}

/** The waiting outcome, if any, read once. Reactive, so the Inbox picks it up when it lands. */
export function takeInboxOutcome(): InboxOutcome | null {
	const outcome = pending;
	if (outcome) pending = null;
	return outcome;
}
