/* How account headroom reads (S100). No requests here, so tests run without a page. */

/** The connection id inside an account key: `openai-codex@<id>` → `<id>`. */
export function connectionOf(account: string): string {
	return account.split('@')[1] ?? account;
}

/** "as of 4 min ago", "as of just now", "as of 3 h ago". */
export function freshness(observedAt: string, now: number): string {
	const minutes = Math.max(0, Math.round((now - Date.parse(observedAt)) / 60_000));
	if (minutes < 1) return 'as of just now';
	if (minutes < 60) return `as of ${minutes} min ago`;
	const hours = Math.round(minutes / 60);
	if (hours < 48) return `as of ${hours} h ago`;
	return `as of ${Math.round(hours / 24)} days ago`;
}

/** "resets in 2 h 41 min", "resets in 3 days". */
export function resetsIn(resetsAt: string | null, now: number): string | null {
	if (!resetsAt) return null;
	const minutes = Math.round((Date.parse(resetsAt) - now) / 60_000);
	if (minutes <= 0) return 'resetting now';
	if (minutes < 60) return `resets in ${minutes} min`;
	const hours = Math.floor(minutes / 60);
	if (hours < 48) return `resets in ${hours} h${minutes % 60 ? ` ${minutes % 60} min` : ''}`;
	return `resets in ${Math.round(hours / 24)} days`;
}

/** A time in the owner's own time zone: "9:40 pm", or "Tue 9:40 pm" when not today. */
export function timeOfDay(iso: string, now: number, locale?: string): string {
	const at = new Date(iso);
	const time = at.toLocaleTimeString(locale, { hour: 'numeric', minute: '2-digit' });
	const sameDay = new Date(now).toDateString() === at.toDateString();
	return sameDay ? time : `${at.toLocaleDateString(locale, { weekday: 'short' })} ${time}`;
}

export function meterTone(usedPercent: number): 'ok' | 'warn' | 'full' {
	if (usedPercent >= 100) return 'full';
	return usedPercent >= 80 ? 'warn' : 'ok';
}
