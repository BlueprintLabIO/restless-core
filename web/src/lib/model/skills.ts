/* Company skills, `/goal` and `/loop` (Sprint 55).
 *
 * Skills are the company's reusable methods in the open SKILL.md format. The
 * owner surfaces here only select, accept, retire and assign them; the package
 * files live in the company computer. `/goal` and `/loop` are Restless
 * primitives (a Goal and an Exec interval schedule), not harness features. */

import type { GoalRow, ScheduleRow, SkillAssignmentRow, SkillRow } from './generated/orgintel';

export type { SkillRow, SkillAssignmentRow };

export type SkillLibrary = {
	skills: SkillRow[];
	assignments: SkillAssignmentRow[];
	/** Names the current viewer may select now. */
	usable: string[];
	scan: { state: 'observed' } | { state: 'unavailable'; message: string };
};

/** One entry in the composer's `/` and `$` menus. */
export type ComposerOption = {
	kind: 'skill' | 'command';
	name: string;
	label: string;
	description: string;
	/** Text inserted for a command; skills become chips instead. */
	insert?: string;
};

async function failure(response: Response, fallback: string): Promise<Error> {
	let message = fallback;
	try {
		message = (await response.json()).message ?? message;
	} catch {
		/* Keep the transport status when the body is not JSON. */
	}
	return Object.assign(new Error(message), { status: response.status });
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
	status: string;
	scope: string;
	actor_run?: string;
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
	schedule: string,
	runActor = false
): Promise<ScheduleTestReport> {
	return call(company, `/schedules/${encodeURIComponent(schedule)}/test`, {
		method: 'POST',
		body: JSON.stringify({ run_actor: runActor })
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
