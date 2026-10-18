import { bootFetch } from './boot-fetch.ts';
/* Company skills, `/goal` and `/loop` (Sprint 55).
 *
 * Skills are the company's reusable methods in the open SKILL.md format. The
 * owner surfaces here select, read, write, accept, retire and assign them; the
 * package files live in the company computer, and new know-how the owner writes
 * lands there as a candidate to review like any other. `/goal` and `/loop` are Restless
 * primitives (a Goal and an Exec interval schedule), not harness features. */

import type { GoalRow, ScheduleRow, SkillAssignmentRow, SkillRow } from './generated/orgintel';
import { responseFailure } from './failure.ts';
import { cadenceOf, cadenceText } from './schedule-cadence.ts';
import { closeGoal, reopenGoal } from './work-actions.ts';
export { closeGoal, reopenGoal };

export type { SkillRow, SkillAssignmentRow };

export type SkillLibrary = {
	skills: SkillRow[];
	assignments: SkillAssignmentRow[];
	usage?: Record<string, number>;
	/** Skills each active agent may use now, after actor/team/company overrides. */
	availability?: Record<string, string[]>;
	/** Recorded applications, not an assertion about currently loaded prompts. Owner-only. */
	applications?:
		{ actor_id: string; skill_name: string; digest: string; last_used_at: string }[] | null;
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
	const response = await (init?.method && init.method.toUpperCase() !== 'GET' ? fetch : bootFetch)(
		companyPath(company, path),
		{
			credentials: 'same-origin',
			...init,
			headers: init?.body ? { 'content-type': 'application/json' } : undefined
		}
	);
	if (!response.ok) throw await failure(response, `${response.status} ${response.statusText}`);
	return response.json() as Promise<T>;
}

/** What changes on a schedule besides pausing: an interval (`every`), or a time of day
 * (`at_local`, HH:MM) with its `timezone` and `days`, and what it asks Exec to do (`prompt`). */
export type ScheduleChange = {
	every?: string;
	at_local?: string;
	timezone?: string;
	days?: string;
	prompt?: string;
};

/** Pause, resume, or change a schedule's cadence or words. */
export function updateSchedule(
	company: string,
	schedule: ScheduleRow,
	paused: boolean,
	change: ScheduleChange = {}
): Promise<{ schedule: ScheduleRow }> {
	return call(company, `/schedules/${encodeURIComponent(schedule.id)}`, {
		method: 'PUT',
		body: JSON.stringify({
			expected_fire_at: schedule.fire_at,
			expected_paused: !!schedule.paused_at,
			paused,
			...change
		})
	});
}

/** Run a schedule now; the next run is then set from now. */
export function runScheduleNow(
	company: string,
	schedule: ScheduleRow
): Promise<{ schedule: ScheduleRow }> {
	return call(company, `/schedules/${encodeURIComponent(schedule.id)}/run`, {
		method: 'POST',
		body: JSON.stringify({ expected_fire_at: schedule.fire_at })
	});
}

/** One run a schedule fired or skipped, with any recovery of a skipped one. */
export type ScheduleOccurrence = {
	scheduled_for: string;
	fired_at: string;
	disposition: 'fired' | 'skipped' | string;
	detail: string | null;
	superseded_count: number;
	recovered_at: string | null;
	recovered_by: string | null;
};

/** The last twenty runs of a schedule, newest first. */
export async function scheduleHistory(
	company: string,
	schedule: string
): Promise<ScheduleOccurrence[]> {
	const body = await call<{ occurrences: ScheduleOccurrence[] }>(
		company,
		`/schedules/${encodeURIComponent(schedule)}/history`
	);
	return body.occurrences;
}

/** Recover one skipped run with a single wake; asking twice does not wake twice. */
export function recoverScheduleRun(company: string, schedule: string, scheduledFor: string) {
	return call<unknown>(company, `/schedules/${encodeURIComponent(schedule)}/recover`, {
		method: 'POST',
		body: JSON.stringify({ scheduled_for: scheduledFor })
	});
}

export type MissedPolicy = 'skip' | 'skip_if_late' | 'catch_up_once' | 'coalesce_latest';

/** What happens when a run is missed; every policy but skip needs a lateness window. */
export function setScheduleMissedPolicy(
	company: string,
	schedule: string,
	policy: MissedPolicy,
	graceSeconds: number | null
) {
	return call<unknown>(company, `/schedules/${encodeURIComponent(schedule)}/policy`, {
		method: 'POST',
		body: JSON.stringify({
			missed_policy: policy,
			catch_up_grace_seconds: policy === 'skip' ? null : graceSeconds
		})
	});
}

export function fetchSkillLibrary(company: string, signal?: AbortSignal): Promise<SkillLibrary> {
	return call(company, '/skills', { signal });
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
export type ExecCommand = 'goal' | 'loop';

/** Which Exec commands a membership role may run: the server lets an admin run loops with the
 * owner, while goals stay the owner's. */
export function execCommandsFor(role: string | null | undefined): readonly ExecCommand[] {
	if (role === 'owner') return ['goal', 'loop'];
	if (role === 'admin') return ['loop'];
	return [];
}

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
	/** The newest run was missed and skipped, and nobody recovered it: when it was due. */
	last_skipped?: string | null;
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

/** A live schedule whose newest run failed or asked for a person, or was skipped: the
 * Company tab carries a dot for it, so the owner hears of it outside Schedules. */
export function scheduleNeedsLook(item: MonitoredSchedule): boolean {
	if (item.schedule.paused_at) return false;
	const newest = item.recent_outcomes[0];
	return !!item.last_skipped || ['blocked', 'needs_human'].includes(newest?.state ?? '');
}

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

/** Wake Exec at `atLocal` (HH:MM) in an IANA time zone, on `days`: `weekdays`, `daily`,
 * `weekly:1,3` (ISO weekdays) or `monthly:15`. */
export function addCalendarSchedule(
	company: string,
	atLocal: string,
	timezone: string,
	days: string,
	prompt: string
): Promise<{ schedule_id: string; next_fire_at: string; created: boolean }> {
	return call(company, '/loops', {
		method: 'POST',
		body: JSON.stringify({ at_local: atLocal, timezone, days, prompt })
	});
}

/** The browser's IANA time zone: where a new time-of-day schedule starts. */
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
	text: string,
	allowed: readonly ExecCommand[] = ['goal', 'loop']
): Promise<{
	notice?: string;
	error?: string;
	send: boolean;
	/** Puts back what the command just stopped or closed; resolves to what it says then. */
	undo?: () => Promise<string>;
	/** The goal this command created before its words go to Exec: a retry must not create it again. */
	createdGoal?: string;
}> {
	const command = parseComposerCommand(text);
	if (!command) return { send: true };
	if (!allowed.length) return { send: true };
	if (command.kind.startsWith('goal') && !allowed.includes('goal'))
		return {
			error: 'Only the owner sets and closes goals. Ask Exec in your own words instead.',
			send: false
		};
	switch (command.kind) {
		case 'invalid':
			return { error: command.message, send: false };
		case 'goal-set': {
			const goal = await addGoal(company, command.objective);
			return { send: true, createdGoal: goal.goal_id };
		}
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
			? cadenceText(cadenceOf(loop, loop.timezone))
			: `Every ${describeInterval(loop.interval_seconds ?? 0)}`;
	return `${when}: ${loop.reason}`;
}

/** A stopped loop started again with the same cadence and words (under a new id). */
function restartLoop(company: string, loop: ScheduleRow): Promise<unknown> {
	return loop.local_time && loop.timezone
		? addCalendarSchedule(
				company,
				loop.local_time.slice(0, 5),
				loop.timezone,
				loop.calendar_days ?? 'weekdays',
				loop.reason
			)
		: addLoop(company, `${loop.interval_seconds ?? 0}s`, loop.reason);
}

/** A SKILL.md as uploaded: kept as written when it names itself, otherwise named from its file
 * and described by its first heading. */
export function skillMdFromUpload(fileName: string, text: string): string {
	if (/^---\s*\n[\s\S]*?\bname:\s*\S/.test(text)) return text;
	const base = fileName.replace(/\.md$/i, '').replace(/[-_ ]?skill$/i, '');
	const heading = text.match(/^#\s+(.+)$/m)?.[1]?.trim() ?? base;
	return composeSkillMd(skillSlug(base), heading, text);
}

/** Install an uploaded Library file (its text, as read) as know-how: a candidate the owner reviews in Apps. */
export async function installUploadAsKnowHow(
	company: string,
	read: () => Promise<string>,
	fileName: string
): Promise<SkillRow> {
	const text = await read();
	if (!text.trim()) throw new Error('That file is empty or too large to install.');
	return createSkill(company, skillMdFromUpload(fileName, text));
}
