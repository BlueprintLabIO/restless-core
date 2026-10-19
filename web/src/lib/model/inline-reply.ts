/* Answering an agent's question inside its Inbox card (S101): the recent messages, the question,
 * suggested answers, and a reply that goes to the agent as a message. No requests here. */

/** Answers the question itself offers: "quote S&I at A$1,000 for five pages, or A$500 for the
 * single page?" gives the two halves. At most three, each short enough to be a button. */
export function quickAnswers(question: string): string[] {
	const text = question
		.replace(/^\s*(?:>\s*)?(?:needs you:?\s*)?/i, '')
		.replace(/\?\s*$/, '')
		.trim();
	if (!/\bor\b/i.test(text)) return [];
	const lead = text.match(/^(?:do you want me to|should i|shall i|would you like me to)\s+/i);
	const body = lead ? text.slice(lead[0].length) : text;
	const parts = body
		.split(/,\s*or\s+|\s+or\s+/i)
		.map((part) => part.replace(/^(?:either\s+)/i, '').trim())
		.filter((part) => part.length > 1);
	if (parts.length < 2 || parts.length > 3 || parts.some((part) => part.length > 90)) return [];
	return balanced(parts).map((part) => part.charAt(0).toUpperCase() + part.slice(1));
}

const AMOUNT = /^(?:[A-Z]{0,3}\$|£|€)?\d/;

/** "announce 3 November or 10 November": the words before the first choice belong to every
 * choice, so the buttons read "3 November" and "10 November". Only when the choices are
 * amounts or dates, where the shared lead-in is unmistakable. */
function balanced(parts: string[]): string[] {
	if (!parts.slice(1).every((part) => AMOUNT.test(part))) return parts;
	const words = parts[0].split(/\s+/);
	const start = words.findIndex((word, index) => index > 0 && AMOUNT.test(word));
	if (start < 1 || start > 3) return parts;
	return [words.slice(start).join(' '), ...parts.slice(1)];
}

/** The reply as sent: the question quoted, so the agent knows what it answers, then the answer. */
export function replyBody(question: string, answer: string): string {
	const asked = question.trim().replace(/^>\s*/, '');
	return asked ? `> ${asked}\n\n${answer.trim()}` : answer.trim();
}
