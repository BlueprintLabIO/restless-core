/* Any page can hand the owner to the Exec: open the rail on the general
 * conversation with the composer ready. The layout listens. */
export const ASK_EXEC_EVENT = 'restless:ask-exec';
/** Reveal the already selected conversation without changing its actor or Work. */
export const SHOW_CONVERSATION_EVENT = 'restless:show-conversation';

/** An answer typed in the Exec conversation that also settles the item it links. */
export type ExecReply = {
	/** The draft's opening link; the answer is what follows it, sent while it is still there. */
	prefix: string;
	record: (answer: string) => Promise<void>;
};

/** With a draft, the composer opens holding it for the owner to edit. */
export function askExec(draft?: unknown, reply?: ExecReply) {
	window.dispatchEvent(
		new CustomEvent(ASK_EXEC_EVENT, {
			detail: { draft: typeof draft === 'string' ? draft : '', reply }
		})
	);
}

/** The draft that links one thing on screen into the Exec conversation. */
export function linkDraft(title: string, href: string, lead = 'About'): string {
	return `${lead} [${title.replace(/[[\]]/gu, '')}](${href}): `;
}

/** Something on screen (an item, Work, a document) is linked context in the one Exec
 * conversation, never a new thread about it. */
export function askExecAbout(title: string, href: string) {
	askExec(linkDraft(title, href));
}

const itemHref = (companyId: string, itemId: string) =>
	`/${encodeURIComponent(companyId)}?item=${encodeURIComponent(itemId)}`;

export function askExecAboutItem(companyId: string, item: { id: string; title: string }) {
	askExecAbout(item.title, itemHref(companyId, item.id));
}

/** Replying to an item happens in the one Exec conversation too: the draft links the item, and
 * sending it with that link also records the answer on the item. */
export function replyInExec(
	companyId: string,
	item: { id: string; title: string },
	record: ExecReply['record']
) {
	const prefix = linkDraft(item.title, itemHref(companyId, item.id), 'Reply to');
	askExec(prefix, { prefix, record });
}

/** A message as a draft asking the Exec to make it a piece of Work. */
export function workDraftFrom(author: string, text: string): string {
	const plain = text
		.replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
		.replace(/\s+/g, ' ')
		.trim();
	const excerpt = plain.length > 400 ? `${plain.slice(0, 400)}…` : plain;
	return `Turn this into Work:\n\n> ${author}: ${excerpt}`;
}

/** What the owner can ask the Exec to do about someone on the team. */
export type StaffAsk = 'rebrief' | 'pause' | 'retire' | 'hire-like' | 'move';

/** A draft for the Exec about one person: the Exec hires, briefs, pauses and retires Staff, so
 * People starts the conversation rather than a form. The owner finishes the sentence. */
export function staffDraft(
	ask: StaffAsk,
	person: { display: string; role: string },
	team = ''
): string {
	const who = `${person.display}${person.role || team ? ` (${[person.role, team].filter(Boolean).join(', ')})` : ''}`;
	switch (ask) {
		case 'rebrief':
			return `Re-brief ${who}. What should change: `;
		case 'pause':
			return `Pause ${who} until I say otherwise, and tell me what that leaves waiting. Why: `;
		case 'retire':
			return `Retire ${who}. Hand their open Work to someone suitable first. Why: `;
		case 'move':
			return `Move ${who} to another team. Which team, and why: `;
		case 'hire-like':
			return `Add someone like ${who}${team ? ` to ${team}` : ''}. What they should own: `;
	}
}
