/* How often this person has answered the same kind of card in the Inbox this month (S101), kept
 * in this browser only. It decides whether a card says what to set once on its page instead; it
 * is a hint, so a browser that keeps nothing simply never shows it. */

const KEY = 'restless.inline-habits';
const MONTH = 30 * 24 * 60 * 60 * 1000;

type Habits = Record<string, number[]>;

function read(): Habits {
	try {
		const parsed = JSON.parse(localStorage.getItem(KEY) ?? '{}');
		return parsed && typeof parsed === 'object' ? (parsed as Habits) : {};
	} catch {
		return {};
	}
}

function recent(times: unknown, now: number): number[] {
	return Array.isArray(times)
		? times.filter((at) => typeof at === 'number' && now - at < MONTH)
		: [];
}

/** Answers of this kind in this company over the last 30 days. */
export function inlineHabit(company: string, kind: string, now = Date.now()): number {
	return recent(read()[`${company}:${kind}`], now).length;
}

/** Count one more answer of this kind; returns the new count. */
export function noteInlineAnswer(company: string, kind: string, now = Date.now()): number {
	const habits = read();
	const key = `${company}:${kind}`;
	const times = [...recent(habits[key], now), now];
	habits[key] = times;
	try {
		localStorage.setItem(KEY, JSON.stringify(habits));
	} catch {
		/* Storage is unavailable: the hint just never shows. */
	}
	return times.length;
}
