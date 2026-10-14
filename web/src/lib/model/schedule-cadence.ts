/* When a schedule runs, as the owner chooses it: one control for creating and for changing a
 * schedule. An interval counts from when it is set; a time of day runs in a chosen zone on
 * weekdays, every day, chosen days of the week, or a day of the month. */

import type { ScheduleRow } from './generated/orgintel';

export type Repeat = 'daily' | 'weekdays' | 'weekly' | 'monthly' | `${number}m`;

export type Cadence = {
	repeat: Repeat;
	/** HH:MM, for a time of day. */
	time: string;
	timezone: string;
	/** ISO weekdays, 1 = Monday, for `weekly`. */
	weekdays: number[];
	/** 1–31, for `monthly`; a short month runs on its last day. */
	monthDay: number;
};

export const INTERVALS: { value: Repeat; label: string }[] = [
	{ value: '30m', label: 'Every 30 minutes' },
	{ value: '60m', label: 'Every hour' },
	{ value: '240m', label: 'Every 4 hours' }
];

export const CALENDARS: { value: Repeat; label: string }[] = [
	{ value: 'daily', label: 'Every day at…' },
	{ value: 'weekdays', label: 'Every weekday at…' },
	{ value: 'weekly', label: 'Every week on…' },
	{ value: 'monthly', label: 'Every month on…' }
];

export const WEEKDAY_NAMES = [
	'Monday',
	'Tuesday',
	'Wednesday',
	'Thursday',
	'Friday',
	'Saturday',
	'Sunday'
];

export const isInterval = (repeat: Repeat) => repeat.endsWith('m');

export function newCadence(timezone: string): Cadence {
	return { repeat: '60m', time: '09:00', timezone, weekdays: [1], monthDay: 1 };
}

/** The cadence a schedule has now, in the control's terms. */
export function cadenceOf(schedule: ScheduleRow, timezone: string): Cadence {
	const cadence = newCadence(schedule.timezone ?? timezone);
	if (schedule.interval_seconds) {
		cadence.repeat = `${Math.round(schedule.interval_seconds / 60)}m`;
		return cadence;
	}
	cadence.time = schedule.local_time?.slice(0, 5) ?? cadence.time;
	const days = schedule.calendar_days;
	if (!days) cadence.repeat = 'weekdays';
	else if (days === 'daily') cadence.repeat = 'daily';
	else if (days.startsWith('weekly:')) {
		cadence.repeat = 'weekly';
		cadence.weekdays = days.slice(7).split(',').map(Number);
	} else if (days.startsWith('monthly:')) {
		cadence.repeat = 'monthly';
		cadence.monthDay = Number(days.slice(8));
	}
	return cadence;
}

/** The days as the owner API takes them. */
export function daysOf(cadence: Cadence): string {
	if (cadence.repeat === 'weekly')
		return `weekly:${[...new Set(cadence.weekdays)].sort((a, b) => a - b).join(',')}`;
	if (cadence.repeat === 'monthly') return `monthly:${cadence.monthDay}`;
	return cadence.repeat;
}

/** What to send: an interval, or a time of day with its zone and days. */
export function cadenceRequest(
	cadence: Cadence
): { every: string } | { at_local: string; timezone: string; days: string } {
	if (isInterval(cadence.repeat)) return { every: cadence.repeat };
	return { at_local: cadence.time, timezone: cadence.timezone, days: daysOf(cadence) };
}

export function sameCadence(left: Cadence, right: Cadence): boolean {
	return JSON.stringify(cadenceRequest(left)) === JSON.stringify(cadenceRequest(right));
}

/** A weekly cadence needs at least one day. */
export function cadenceReady(cadence: Cadence): boolean {
	return cadence.repeat !== 'weekly' || cadence.weekdays.length > 0;
}

function intervalText(minutes: number): string {
	if (minutes % 1440 === 0) return minutes === 1440 ? 'Every day' : `Every ${minutes / 1440} days`;
	if (minutes % 60 === 0) return minutes === 60 ? 'Every hour' : `Every ${minutes / 60} hours`;
	return `Every ${minutes} minutes`;
}

function ordinal(day: number): string {
	const tens = day % 100;
	const suffix =
		tens >= 11 && tens <= 13 ? 'th' : ({ 1: 'st', 2: 'nd', 3: 'rd' }[day % 10] ?? 'th');
	return `${day}${suffix}`;
}

function list(words: string[]): string {
	return words.length < 2
		? (words[0] ?? '')
		: `${words.slice(0, -1).join(', ')} and ${words.at(-1)}`;
}

/** The cadence in a few words: "Every hour", "Mondays and Wednesdays at 09:00". */
export function cadenceText(cadence: Cadence): string {
	if (isInterval(cadence.repeat)) return intervalText(Number.parseInt(cadence.repeat, 10));
	const at = `at ${cadence.time}`;
	switch (cadence.repeat) {
		case 'daily':
			return `Every day ${at}`;
		case 'weekdays':
			return `Weekdays ${at}`;
		case 'weekly':
			return `${list(cadence.weekdays.map((day) => `${WEEKDAY_NAMES[day - 1]}s`))} ${at}`;
		default:
			return `Monthly on the ${ordinal(cadence.monthDay)} ${at}`;
	}
}

/** The IANA zones the browser knows, with the given ones first. */
export function timeZones(...first: string[]): string[] {
	let all: string[] = [];
	try {
		all =
			(Intl as unknown as { supportedValuesOf?: (key: string) => string[] }).supportedValuesOf?.(
				'timeZone'
			) ?? [];
	} catch {
		all = [];
	}
	return [...new Set([...first.filter(Boolean), ...all])];
}
