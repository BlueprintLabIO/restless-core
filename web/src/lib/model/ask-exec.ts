/* Any page can hand the owner to the Exec: open the rail on the general
 * conversation with the composer ready. The layout listens. */
export const ASK_EXEC_EVENT = 'restless:ask-exec';

/** With a draft, the composer opens holding it for the owner to edit. */
export function askExec(draft?: unknown) {
	window.dispatchEvent(
		new CustomEvent(ASK_EXEC_EVENT, {
			detail: { draft: typeof draft === 'string' ? draft : '' }
		})
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
