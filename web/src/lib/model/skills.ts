/* Company skills, `/goal` and `/loop` (Sprint 55).
 *
 * Skills are the company's reusable methods in the open SKILL.md format. The
 * owner surfaces here only select, accept, retire and assign them; the package
 * files live in the company computer. `/goal` and `/loop` are Restless
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

export function updateSchedule(
	company: string,
	schedule: ScheduleRow,
	paused: boolean,
	every?: string
): Promise<{ schedule: ScheduleRow }> {
	return call(company, `/schedules/${encodeURIComponent(schedule.id)}`, {
		method: 'PUT',
		body: JSON.stringify({
			expected_fire_at: schedule.fire_at,
			expected_paused: !!schedule.paused_at,
			paused,
			every
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
		description: 'Set a durable objective. Exec routes it to one accountable team.',
		insert: '/goal '
	},
	{
		kind: 'command',
		name: 'loop',
		label: '/loop',
		description: 'Have Exec check in on an interval, e.g. /loop 30m review inbound leads.',
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
	| { kind: 'goal-clear' }
	| { kind: 'loop-set'; every: string; prompt: string }
	| { kind: 'loop-show' }
	| { kind: 'loop-clear' }
	| { kind: 'invalid'; message: string };

/** Interpret one Exec composer command, or null for ordinary text. */
export function parseComposerCommand(text: string): ParsedCommand | null {
	const trimmed = text.trim();
	const match = trimmed.match(/^\/(goal|loop)(?:\s+([\s\S]*))?$/);
	if (!match) return null;
	const rest = (match[2] ?? '').trim();
	if (match[1] === 'goal') {
		if (!rest) return { kind: 'goal-show' };
		if (rest === 'clear') return { kind: 'goal-clear' };
		return { kind: 'goal-set', objective: rest };
	}
	if (!rest) return { kind: 'loop-show' };
	if (rest === 'clear' || rest === 'stop') return { kind: 'loop-clear' };
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
	test_company?: string;
	test_schedule_id?: string;
	occurrence?: unknown;
	opportunity?: { state: string; outcome?: unknown; outcome_reason?: string | null };
	clone_retained: boolean;
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
): Promise<{ notice?: string; error?: string; send: boolean }> {
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
					? `Open goals: ${open.map((goal) => goal.title).join('; ')}`
					: 'No open goals. Set one with /goal <objective>.',
				send: false
			};
		}
		case 'goal-clear': {
			const open = (await listGoals(company)).filter((goal) => !goal.closed_at);
			const latest = open.at(-1);
			if (!latest) return { notice: 'There is no open goal to clear.', send: false };
			await closeGoal(company, latest.id);
			return { notice: `Closed the goal “${latest.title}”. Its work is unchanged.`, send: false };
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
					? loops
							.map(
								(loop) => `Every ${describeInterval(loop.interval_seconds ?? 0)}: ${loop.reason}`
							)
							.join(' · ')
					: 'No loops are running. Start one with /loop 30m <what to check>.',
				send: false
			};
		}
		case 'loop-clear': {
			const loops = await listLoops(company);
			await Promise.all(loops.map((loop) => cancelLoop(company, loop.id)));
			return {
				notice: loops.length
					? `Stopped ${loops.length} loop${loops.length === 1 ? '' : 's'}.`
					: 'No loops were running.',
				send: false
			};
		}
	}
}
