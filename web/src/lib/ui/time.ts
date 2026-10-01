/* One way to write a date or time for the owner. Moments read as
 * "27 Sept, 23:24" (the year only when it is not this year); days as
 * "27 Sept"; recent things relatively ("5 min ago"). The browser's default
 * `toLocaleString()` ("9/28/2026, 10:27:00 AM") never reaches a screen. */

function asDate(value: Date | string | number | null | undefined): Date | null {
	if (value === null || value === undefined || value === '') return null;
	const date = value instanceof Date ? value : new Date(value);
	return Number.isNaN(date.getTime()) ? null : date;
}

function dayPart(date: Date): Intl.DateTimeFormatOptions {
	return date.getFullYear() === new Date().getFullYear()
		? { day: 'numeric', month: 'short' }
		: { day: 'numeric', month: 'short', year: 'numeric' };
}

/** "27 Sept" or "27 Sept 2025". */
export function formatDay(value: Date | string | number | null | undefined, fallback = ''): string {
	const date = asDate(value);
	return date ? new Intl.DateTimeFormat(undefined, dayPart(date)).format(date) : fallback;
}

/** "27 Sept, 23:24" or "27 Sept 2025, 23:24". */
export function formatMoment(
	value: Date | string | number | null | undefined,
	fallback = ''
): string {
	const date = asDate(value);
	if (!date) return fallback;
	return new Intl.DateTimeFormat(undefined, {
		...dayPart(date),
		hour: '2-digit',
		minute: '2-digit'
	}).format(date);
}

/** "Just now", "5 min ago", "3 hours ago", "Yesterday", then the day. */
export function formatRelative(
	value: Date | string | number | null | undefined,
	fallback = '',
	now = Date.now()
): string {
	const date = asDate(value);
	if (!date) return fallback;
	const seconds = Math.round((now - date.getTime()) / 1000);
	if (seconds < 45) return 'Just now';
	const minutes = Math.round(seconds / 60);
	if (minutes < 60) return `${minutes} min ago`;
	const hours = Math.round(minutes / 60);
	if (hours < 24) return `${hours} ${hours === 1 ? 'hour' : 'hours'} ago`;
	if (hours < 48) return 'Yesterday';
	return formatDay(date);
}
