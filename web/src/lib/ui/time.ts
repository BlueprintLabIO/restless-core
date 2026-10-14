/* One way to write a date, a time or an amount for the owner. Moments read as
 * "27 Sept, 23:24" (the year only when it is not this year); days as
 * "27 Sept"; times of day as "23:24"; recent things relatively ("5m ago");
 * spend as "$12.50". The browser's default `toLocaleString()`
 * ("9/28/2026, 10:27:00 AM") never reaches a screen. */

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

/** "27 Sept" or "27 Sept 2025". */
export function formatDay(value: Date | string | number | null | undefined, fallback = ''): string {
	const date = asDate(value);
	if (!date) return fallback;
	return new Intl.DateTimeFormat(undefined, dayPart(date)).format(date);
}

/** "23:24": a time of day, written the same way as in formatMoment. */
export function formatClock(
	value: Date | string | number | null | undefined,
	fallback = ''
): string {
	const date = asDate(value);
	if (!date) return fallback;
	return new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit' }).format(date);
}

/** Spend in US dollars, one way everywhere: "$12.50", never "US$12.50" in one place and "$12.50"
 * in another. */
export function formatMoney(usd: number): string {
	return new Intl.NumberFormat(undefined, {
		style: 'currency',
		currency: 'USD',
		currencyDisplay: 'narrowSymbol',
		maximumFractionDigits: 2
	}).format(usd);
}

/** Compact relative time, including scheduled times in the future: "5m ago", "in 2h". */
export function formatRelative(
	value: Date | string | number | null | undefined,
	fallback = '',
	now = Date.now()
): string {
	const date = asDate(value);
	if (!date) return fallback;
	const seconds = Math.round((now - date.getTime()) / 1000);
	const distance = Math.abs(seconds);
	if (distance < 60) return seconds < 0 ? 'Soon' : 'Just now';
	const label =
		distance < 3600
			? `${Math.floor(distance / 60)}m`
			: distance < 86400
				? `${Math.floor(distance / 3600)}h`
				: distance < 604800
					? `${Math.floor(distance / 86400)}d`
					: distance < 2592000
						? `${Math.floor(distance / 604800)}w`
						: distance < 31536000
							? `${Math.floor(distance / 2592000)}mo`
							: `${Math.floor(distance / 31536000)}y`;
	return seconds < 0 ? `in ${label}` : `${label} ago`;
}
