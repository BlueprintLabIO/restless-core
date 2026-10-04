/* What this device last saw in each direct conversation, so a list can say
 * which have something new. Per device, by design: it is a reading aid, not
 * read state. */
const key = (company: string, room: string) => `restless:${company}:seen:${room}`;

export function seenThrough(company: string, room: string): number {
	try {
		return Number(localStorage.getItem(key(company, room)) ?? 0) || 0;
	} catch {
		return 0;
	}
}

export function markSeen(company: string, room: string, messageId: number) {
	if (!messageId || messageId <= seenThrough(company, room)) return;
	try {
		localStorage.setItem(key(company, room), String(messageId));
		window.dispatchEvent(new CustomEvent('restless:seen'));
	} catch {
		/* A reading aid only. */
	}
}
