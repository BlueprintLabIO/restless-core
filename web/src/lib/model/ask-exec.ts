/* Any page can hand the owner to the Exec: open the rail on the general
 * conversation with the composer ready. The layout listens. */
export const ASK_EXEC_EVENT = 'restless:ask-exec';
/** Reveal the already selected conversation without changing its actor or Work. */
export const SHOW_CONVERSATION_EVENT = 'restless:show-conversation';

/** With a draft, the composer opens holding it for the owner to edit. */
export function askExec(draft?: unknown) {
	window.dispatchEvent(
		new CustomEvent(ASK_EXEC_EVENT, {
			detail: { draft: typeof draft === 'string' ? draft : '' }
		})
	);
}

/** The item is linked context in the existing Exec conversation, never a new thread. */
export function askExecAboutItem(companyId: string, item: { id: string; title: string }) {
	askExec(
		`About [${item.title.replace(/[[\]]/gu, '')}](/${encodeURIComponent(companyId)}?item=${encodeURIComponent(item.id)}): `
	);
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
