/* Company skills, `/goal` and `/loop` (Sprint 55).
 *
 * Skills are the company's reusable methods in the open SKILL.md format. The
 * owner surfaces here select, read, write, accept, retire and assign them; the
 * package files live in the company computer, and new know-how the owner writes
 * lands there as a candidate to review like any other. `/goal` and `/loop` are Restless
 * primitives (a Goal and an Exec interval schedule), not harness features. */

import type { GoalRow, ScheduleRow, SkillAssignmentRow, SkillRow } from './generated/orgintel';
import { responseFailure } from './failure.ts';

export type { SkillRow, SkillAssignmentRow };

export type SkillLibrary = {
	skills: SkillRow[];
	assignments: SkillAssignmentRow[];
	usage?: Record<string, number>;
	/** Names the current viewer may select now. */
	usable: string[];
	scan: { state: 'observed' } | { state: 'unavailable'; message: string };
};

/** One entry in the composer's `/` and `$` menus. */
export type ComposerOption = {
	/** `reference` links Work or a Goal (`#`); `person` mentions someone (`@`). */
	kind: 'skill' | 'command' | 'reference' | 'person';
	name: string;
	label: string;
	description: string;
	/** Text inserted for a command; skills become chips instead. */
	insert?: string;
};

async function failure(response: Response, fallback: string): Promise<Error> {
	const error = await responseFailure(response);
	if (!error.serverMessage) error.message = fallback;
	return error;
}

function companyPath(company: string, path: string): string {
	return `/api/companies/${encodeURIComponent(company)}${path}`;
}

async function call<T>(company: string, path: string, init?: RequestInit): Promise<T> {
	const response = await fetch(companyPath(company, path), {
		credentials: 'same-origin',
		...init,
		headers: init?.body ? { 'content-type': 'application/json' } : undefined
	});
	if (!response.ok) throw await failure(response, `${response.status} ${response.statusText}`);
	return response.json() as Promise<T>;
}

/** Pause, resume, or change an interval (`every`) or a weekday time (`atLocal`, HH:MM). */
export function updateSchedule(
	company: string,
	schedule: ScheduleRow,
	paused: boolean,
	every?: string,
	atLocal?: string
): Promise<{ schedule: ScheduleRow }> {
	return call(company, `/schedules/${encodeURIComponent(schedule.id)}`, {
		method: 'PUT',
		body: JSON.stringify({
			expected_fire_at: schedule.fire_at,
			expected_paused: !!schedule.paused_at,
			paused,
			every,
			at_local: atLocal
		})
	});
}

export function fetchSkillLibrary(company: string): Promise<SkillLibrary> {
	return call(company, '/skills');
}

export function setSkillDisposition(
	company: string,
	skill: string,
	disposition: 'candidate' | 'accepted' | 'retired'
): Promise<SkillRow> {
	return call(company, `/skills/${encodeURIComponent(skill)}/disposition`, {
		method: 'POST',
		body: JSON.stringify({ disposition })
	});
}

/** One file of a skill package, as read live from the company computer. */
export type SkillPackageFile = { path: string; bytes: number; text?: string | null };

export type SkillPackage = {
	skill: SkillRow;
	package: { skill_md: string; digest: string; files: SkillPackageFile[] };
	/** The files differ from the version the library recorded. */
	changed: boolean;
};

/** Read a skill's SKILL.md and the scripts and references beside it. */
export function fetchSkillPackage(company: string, skill: string): Promise<SkillPackage> {
	return call(company, `/skills/${encodeURIComponent(skill)}/package`);
}

/** Write new know-how; it lands as a candidate for review. */
export function createSkill(company: string, skillMd: string): Promise<SkillRow> {
	return call(company, '/skills', { method: 'POST', body: JSON.stringify({ skill_md: skillMd }) });
}

/** Save an edited SKILL.md over the version read at `expectedDigest`. */
export function editSkill(
	company: string,
	skill: string,
	skillMd: string,
	expectedDigest: string
): Promise<SkillRow> {
	return call(company, `/skills/${encodeURIComponent(skill)}/package`, {
		method: 'PUT',
		body: JSON.stringify({ skill_md: skillMd, expected_digest: expectedDigest })
	});
}

/** A skill name from a title: `Reply to refunds` → `reply-to-refunds`. */
export function skillSlug(title: string): string {
	return title
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-+|-+$/g, '')
		.slice(0, 64)
		.replace(/-+$/, '');
}

/** A SKILL.md in the open format from what the owner wrote. */
export function composeSkillMd(name: string, description: string, instructions: string): string {
	const oneLine = description.replace(/\s+/g, ' ').trim().replace(/"/g, '\\"');
	return `---\nname: ${name}\ndescription: "${oneLine}"\n---\n\n${instructions.trim()}\n`;
}

/** `sha256:0123…` → `0123456789ab`, a version the owner can compare. */
export function shortDigest(digest: string): string {
	return digest.replace(/^sha256:/, '').slice(0, 12);
}

export function assignSkill(
	company: string,
	skill: string,
	scope: 'company' | 'team' | 'actor',
	scopeId: string,
	enabled: boolean | null
): Promise<unknown> {
	return call(company, `/skills/${encodeURIComponent(skill)}/assignment`, {
		method: 'POST',
		body: JSON.stringify({ scope, scope_id: scopeId, enabled })
	});
}

/** `frontend-design` → `Frontend design`, for chips and activity. */
export function skillLabel(name: string): string {
	const words = name.split('-').filter(Boolean).join(' ');
	return words ? words[0].toUpperCase() + words.slice(1) : name;
}

export function skillOptions(library: SkillLibrary | null): ComposerOption[] {
	if (!library) return [];
	const usable = new Set(library.usable);
	return library.skills
		.filter((skill) => usable.has(skill.name))
		.map((skill) => ({
			kind: 'skill' as const,
			name: skill.name,
			label: skillLabel(skill.name),
			description: skill.description
		}));
}

/** Restless commands offered by the Exec composer's `/` menu. */
export const EXEC_COMMANDS: ComposerOption[] = [
	{
		kind: 'command',
		name: 'goal',
		label: '/goal',
		description:
			'Set a durable objective; Exec routes it to one team. /goal lists open goals, /goal clear <number> closes one.',
		insert: '/goal '
	},
	{
		kind: 'command',
		name: 'loop',
		label: '/loop',
		description:
			'Have Exec check in on an interval, e.g. /loop 30m review leads. /loop lists them, /loop stop <number> stops one.',
		insert: '/loop '
	}
];

/** `restless skill use frontend-design` → `frontend-design`, for activity labels. */
export function skillInUse(title: string | null | undefined): string | null {
	const match = title?.match(/\brestless\s+skill\s+use\s+([a-z0-9][a-z0-9-]{0,63})\b/);
	return match ? match[1] : null;
}

export type ParsedCommand =
	| { kind: 'goal-set'; objective: string }
	| { kind: 'goal-show' }
	| { kind: 'goal-clear'; target: string }
	| { kind: 'loop-set'; every: string; prompt: string }
	| { kind: 'loop-show' }
	| { kind: 'loop-clear'; target: string }
	| { kind: 'invalid'; message: string };

/** Interpret one Exec composer command, or null for ordinary text. */
export function parseComposerCommand(text: string): ParsedCommand | null {
	const trimmed = text.trim();
	const match = trimmed.match(/^\/(goal|loop)(?:\s+([\s\S]*))?$/);
	if (!match) return null;
	const rest = (match[2] ?? '').trim();
	if (match[1] === 'goal') {
		if (!rest) return { kind: 'goal-show' };
		const clear = rest.match(/^(?:clear|close)(?:\s+([\s\S]*))?$/i);
		if (clear) return { kind: 'goal-clear', target: (clear[1] ?? '').trim() };
		return { kind: 'goal-set', objective: rest };
	}
	if (!rest) return { kind: 'loop-show' };
	const stop = rest.match(/^(?:clear|stop)(?:\s+([\s\S]*))?$/i);
	if (stop) return { kind: 'loop-clear', target: (stop[1] ?? '').trim() };
	const loop = rest.match(/^(\d+\s*(?:s|m|h|d|min|mins|minutes?|hours?|days?)?)\s+([\s\S]+)$/i);
	if (!loop) {
		return {
			kind: 'invalid',
			message: 'Use /loop <interval> <what to check>, for example /loop 30m review inbound leads.'
		};
	}
	return { kind: 'loop-set', every: loop[1].replace(/\s+/g, ''), prompt: loop[2].trim() };
}

export async function listGoals(company: string): Promise<GoalRow[]> {
	return (await call<{ goals: GoalRow[] }>(company, '/goals')).goals;
}

export function addGoal(
	company: string,
	objective: string
): Promise<{ goal_id: string; title: string }> {
	return call(company, '/goals', { method: 'POST', body: JSON.stringify({ objective }) });
}

export function closeGoal(company: string, goal: string): Promise<{ closed: boolean }> {
	return call(company, `/goals/${encodeURIComponent(goal)}/close`, { method: 'POST' });
}

export function reopenGoal(company: string, goal: string): Promise<{ reopened: boolean }> {
	return call(company, `/goals/${encodeURIComponent(goal)}/reopen`, { method: 'POST' });
}

/** Which of `rows` a `/goal clear` or `/loop stop` target names: its number in the listing,
 * words from its title, or `all`. Nothing named chooses the only row; otherwise null asks. */
export function pickTargets<T>(
	rows: readonly T[],
	target: string,
	title: (row: T) => string,
	allowAll: boolean
): T[] | null {
	const wanted = target.trim().toLowerCase();
	if (!wanted) return rows.length === 1 ? [rows[0]] : null;
	if (wanted === 'all') return allowAll ? [...rows] : null;
	if (/^\d+$/.test(wanted)) {
		const row = rows[Number(wanted) - 1];
		return row ? [row] : null;
	}
	const matches = rows.filter((row) => title(row).toLowerCase().includes(wanted));
	return matches.length === 1 ? matches : null;
}

function numbered(titles: string[]): string {
	return titles.map((title, index) => `${index + 1}. ${title}`).join(' · ');
}

export async function listLoops(company: string): Promise<ScheduleRow[]> {
	return (await call<{ loops: ScheduleRow[] }>(company, '/loops')).loops;
}

export type MonitoredSchedule = {
	schedule: ScheduleRow & {
		responsibility_id: string | null;
		responsibility_version: number | null;
	};
	testable: boolean;
	recent_outcomes: Array<{
		scheduled_for: string;
		admission: string | null;
		opportunity_id: string;
		state: string;
		outcome: unknown;
		outcome_reason: string | null;
		created_at: string;
		settled_at: string | null;
	}>;
	prior_responsibility_outcomes: Array<{
		opportunity_id: string;
		state: string;
		outcome: unknown;
		outcome_reason: string | null;
		created_at: string;
		settled_at: string | null;
	}>;
};

export type ScheduleTestReport = {
	status: 'admitted' | 'timed_out_before_admission';
	scope: 'scheduler_only';
	actor_run: 'not_started';
	effect_boundary?: string;
	source_company: string;
	source_schedule_id: string;
	test_schedule_id?: string;
	occurrence?: unknown;
	opportunity?: { state: string; outcome?: unknown; outcome_reason?: string | null };
	work_outcomes?: Array<{
		work: { id: string; title: string; status: string; outcome: string } | null;
		attempts: Array<{ state: string; summary?: string | null }>;
	}>;
	scheduled_for?: string;
};

export async function monitorSchedules(company: string): Promise<MonitoredSchedule[]> {
	return (await call<{ schedules: MonitoredSchedule[] }>(company, '/schedules')).schedules;
}

export function testScheduleTrigger(
	company: string,
	schedule: string
): Promise<ScheduleTestReport> {
	return call(company, `/schedules/${encodeURIComponent(schedule)}/test`, {
		method: 'POST',
		body: '{}'
	});
}

export function addLoop(
	company: string,
	every: string,
	prompt: string
): Promise<{
	schedule_id: string;
	interval_seconds: number;
	next_fire_at: string;
	created: boolean;
}> {
	return call(company, '/loops', { method: 'POST', body: JSON.stringify({ every, prompt }) });
}

/** Wake Exec Monday to Friday at `atLocal` (HH:MM) in an IANA time zone. */
export function addWeekdaySchedule(
	company: string,
	atLocal: string,
	timezone: string,
	prompt: string
): Promise<{ schedule_id: string; next_fire_at: string; created: boolean }> {
	return call(company, '/loops', {
		method: 'POST',
		body: JSON.stringify({ at_local: atLocal, timezone, prompt })
	});
}

/** The browser's IANA time zone, which a weekday schedule runs in. */
export function browserTimeZone(): string {
	try {
		return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
	} catch {
		return 'UTC';
	}
}

export function cancelLoop(company: string, schedule: string): Promise<{ cancelled: boolean }> {
	return call(company, `/loops/${encodeURIComponent(schedule)}/cancel`, { method: 'POST' });
}

export function describeInterval(seconds: number): string {
	if (seconds % 86_400 === 0) return seconds === 86_400 ? 'day' : `${seconds / 86_400} days`;
	if (seconds % 3_600 === 0) return seconds === 3_600 ? 'hour' : `${seconds / 3_600} hours`;
	return `${Math.round(seconds / 60)} minutes`;
}

/** Runs a `/goal` or `/loop` command typed to the Exec. Returns a notice for
 * commands handled without a message, or `send: true` to deliver the text. */
export async function runExecCommand(
	company: string,
	text: string
): Promise<{
	notice?: string;
	error?: string;
	send: boolean;
	/** Puts back what the command just stopped or closed; resolves to what it says then. */
	undo?: () => Promise<string>;
}> {
	const command = parseComposerCommand(text);
	if (!command) return { send: true };
	switch (command.kind) {
		case 'invalid':
			return { error: command.message, send: false };
		case 'goal-set':
			await addGoal(company, command.objective);
			return { send: true };
		case 'goal-show': {
			const open = (await listGoals(company)).filter((goal) => !goal.closed_at);
			return {
				notice: open.length
					? `Open goals: ${numbered(open.map((goal) => goal.title))}`
					: 'No open goals. Set one with /goal <objective>.',
				send: false
			};
		}
		case 'goal-clear': {
			const open = (await listGoals(company)).filter((goal) => !goal.closed_at);
			if (!open.length) return { notice: 'There is no open goal to clear.', send: false };
			const chosen = pickTargets(open, command.target, (goal) => goal.title, false);
			if (!chosen)
				return {
					notice: `Which goal? ${numbered(open.map((goal) => goal.title))}. Close one with /goal clear <number>.`,
					send: false
				};
			const goal = chosen[0];
			await closeGoal(company, goal.id);
			return {
				notice: `Closed the goal “${goal.title}”. Its work is unchanged.`,
				send: false,
				undo: async () => {
					await reopenGoal(company, goal.id);
					return `Reopened the goal “${goal.title}”.`;
				}
			};
		}
		case 'loop-set': {
			const loop = await addLoop(company, command.every, command.prompt);
			return {
				notice: loop.created
					? `Exec will check in every ${describeInterval(loop.interval_seconds)}: ${command.prompt}`
					: `That loop is already running every ${describeInterval(loop.interval_seconds)}.`,
				send: false
			};
		}
		case 'loop-show': {
			const loops = await listLoops(company);
			return {
				notice: loops.length
					? numbered(loops.map(loopTitle))
					: 'No loops are running. Start one with /loop 30m <what to check>.',
				send: false
			};
		}
		case 'loop-clear': {
			const loops = await listLoops(company);
			if (!loops.length) return { notice: 'No loops are running.', send: false };
			const chosen = pickTargets(loops, command.target, loopTitle, true);
			if (!chosen)
				return {
					notice: `Which loop? ${numbered(loops.map(loopTitle))}. Stop one with /loop stop <number>, or every one with /loop stop all.`,
					send: false
				};
			await Promise.all(chosen.map((loop) => cancelLoop(company, loop.id)));
			return {
				notice:
					chosen.length === 1
						? `Stopped the loop “${loopTitle(chosen[0])}”.`
						: `Stopped all ${chosen.length} loops.`,
				send: false,
				undo: async () => {
					await Promise.all(chosen.map((loop) => restartLoop(company, loop)));
					return chosen.length === 1
						? 'The loop is running again.'
						: 'The loops are running again.';
				}
			};
		}
	}
}

function loopTitle(loop: ScheduleRow): string {
	const when =
		loop.local_time && loop.timezone
			? `Weekdays at ${loop.local_time}`
			: `Every ${describeInterval(loop.interval_seconds ?? 0)}`;
	return `${when}: ${loop.reason}`;
}

/** A stopped loop started again with the same cadence and words (under a new id). */
function restartLoop(company: string, loop: ScheduleRow): Promise<unknown> {
	return loop.local_time && loop.timezone
		? addWeekdaySchedule(company, loop.local_time.slice(0, 5), loop.timezone, loop.reason)
		: addLoop(company, `${loop.interval_seconds ?? 0}s`, loop.reason);
}
