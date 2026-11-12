/* Company lessons: a correction the owner made, kept as a standing rule for the whole company,
 * one team or one agent. The owner sees each rule, how often the same problem came back, and
 * can retire one that no longer stands. */
import { responseFailure } from './failure.ts';

export type Lesson = {
	id: string;
	rule: string;
	scope_kind: 'company' | 'team' | 'actor';
	team_id: string | null;
	team_name: string | null;
	actor_id: string | null;
	actor_display: string | null;
	source_message_id: number | null;
	recorded_by: string;
	created_at: string;
	status: 'active' | 'retired';
	retired_at: string | null;
	retired_by: string | null;
	retirement_reason: string;
	/** How many times the same problem came back after the rule was recorded. */
	recurrence_count: number;
	last_recurred_at: string | null;
	last_recurrence_message_id: number | null;
};

const base = (companyId: string) => `/api/companies/${encodeURIComponent(companyId)}/lessons`;

/** Active lessons, newest first. */
export async function fetchLessons(companyId: string): Promise<Lesson[]> {
	const response = await fetch(base(companyId), { credentials: 'same-origin', cache: 'no-store' });
	if (!response.ok) throw await responseFailure(response);
	return ((await response.json()) as { items: Lesson[] }).items;
}

async function change(companyId: string, lessonId: string, verb: 'retire' | 'restore') {
	const response = await fetch(`${base(companyId)}/${encodeURIComponent(lessonId)}/${verb}`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		credentials: 'same-origin',
		keepalive: true,
		body: '{}'
	});
	if (!response.ok) throw await responseFailure(response);
}

/** Stop applying a lesson. It keeps its record. */
export const retireLesson = (companyId: string, lessonId: string) =>
	change(companyId, lessonId, 'retire');

/** Bring a retired lesson back: the Undo for a retire. */
export const restoreLesson = (companyId: string, lessonId: string) =>
	change(companyId, lessonId, 'restore');

/** Who a lesson is for, as the owner reads it. */
export function lessonScope(lesson: Lesson): string {
	if (lesson.scope_kind === 'team')
		return lesson.team_name ? `Team ${lesson.team_name}` : 'One team';
	if (lesson.scope_kind === 'actor') return lesson.actor_display || lesson.actor_id || 'One agent';
	return 'Whole company';
}

/** How often the problem came back, always said so a row never changes shape. */
export function lessonRecurrence(lesson: Lesson): string {
	if (lesson.recurrence_count <= 0) return 'Has not come back';
	if (lesson.recurrence_count === 1) return 'Came back once';
	return `Came back ${lesson.recurrence_count} times`;
}
