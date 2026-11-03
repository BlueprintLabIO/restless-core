/* Which of an agent's questions is still open in a conversation. An answer travels as an ordinary
 * reply quoting the question ("> Exec: <question>"), whether it was tapped, filled in or typed after
 * "Something else…". Only such a reply answers it; a newer question supersedes it. An ordinary
 * message in between leaves it open: observed on Cloud, 7 October, where a chat message saying
 * "not an answer" folded Exec's price-and-booking card to "You answered".
 *
 * A decision the owner made after the question also closes it: once they approved the sending
 * permission, "Approve the test-send permission card" stayed as a suggested reply (3 November).
 * Approving in the Inbox is the answer, and the Inbox's own card for the question has already
 * gone, since a newer message in the conversation answers it there. */

export type ThreadMessage = {
	from: string;
	text: string;
	intent?: { ownerNeed?: string | null; kind?: string | null } | null;
};

/** An owner decision line (approve, decline) in the conversation. */
export function isDecisionLine(message: ThreadMessage): boolean {
	return message.from === 'system' && message.intent?.kind === 'authority';
}

/** The one-line form of a message used in quotes. */
export function excerptOf(text: string): string {
	const plain = text
		.replace(/\[([^\]]+)\]\([^)]*\)/g, '$1')
		.replace(/[*_`>#]/g, '')
		.replace(/\s+/g, ' ')
		.trim();
	return plain.length > 180 ? `${plain.slice(0, 180)}…` : plain;
}

export function quotesQuestion(message: ThreadMessage, author: string, question: string) {
	if (message.from !== 'you') return false;
	if (message.text.startsWith(`> ${author}: ${excerptOf(question)}`)) return true;
	/* A dismissal from the Inbox quotes the question under the asker's full name. */
	const first = message.text.split('\n', 1)[0];
	return first.startsWith('> ') && first.endsWith(`: ${excerptOf(question)}`);
}

/** The index of the newest question still open, or -1. */
export function openQuestionIndex(messages: ThreadMessage[], author: string): number {
	for (let index = messages.length - 1; index >= 0; index -= 1) {
		const message = messages[index];
		const need = message.from !== 'you' ? message.intent?.ownerNeed?.trim() : '';
		if (!need) continue;
		return messages
			.slice(index + 1)
			.some((later) => quotesQuestion(later, author, need) || isDecisionLine(later))
			? -1
			: index;
	}
	return -1;
}

/** The index of the reply that answered the question at `index`, or -1. */
export function answerIndex(messages: ThreadMessage[], author: string, index: number): number {
	const need = messages[index]?.intent?.ownerNeed?.trim();
	if (!need || messages[index].from === 'you') return -1;
	for (let later = index + 1; later < messages.length; later += 1) {
		if (quotesQuestion(messages[later], author, need)) return later;
		if (messages[later].from !== 'you' && messages[later].intent?.ownerNeed) return -1;
	}
	return -1;
}
