/* What the owner, or the person who owns a Work's team, does to a Work or a Goal from the Work
 * surface. Each call is one owner-API route; OrgIntel decides who may. */
import { ownerJson } from './failure';
import { failureLine } from './turn-failure';

const base = (company: string) => `/api/companies/${encodeURIComponent(company)}`;

function post<T = unknown>(url: string, body: unknown = {}): Promise<T> {
	return fetch(url, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body),
		credentials: 'same-origin'
	}).then((response) => ownerJson<T>(response));
}

const workUrl = (company: string, work: string, leaf: string) =>
	`${base(company)}/work/${encodeURIComponent(work)}/${leaf}`;
const goalUrl = (company: string, goal: string, leaf: string) =>
	`${base(company)}/goals/${encodeURIComponent(goal)}/${leaf}`;

/** Resume blocked or paused Work; pause it (Resume undoes that); stop it for good; restore it. */
export type WorkSteer = 'resume' | 'pause' | 'abandon' | 'restore';

export function steerWork(company: string, work: string, action: WorkSteer, reason: string) {
	return post(workUrl(company, work, action), { reason });
}

/** Send done Work back: the feedback becomes input to its next run. */
export function reopenWork(company: string, work: string, feedback: string) {
	return post(workUrl(company, work, 'reopen'), { feedback });
}

export type WorkDetails = {
	title?: string;
	outcome?: string;
	expected_artifact?: string;
	/** 1 high, 0 normal, -1 low. */
	priority?: number;
	/** null takes the Work out of its Goal. */
	goal_id?: string | null;
};

export function setWorkDetails(company: string, work: string, details: WorkDetails) {
	return post(workUrl(company, work, 'details'), details);
}

export function reassignWork(company: string, work: string, person: string, reason: string) {
	return post(workUrl(company, work, 'owner'), { person, reason });
}

export type WorkFeedback = {
	message_id: number;
	from_actor: string;
	to_actor: string | null;
	body: string;
	created_at: string;
	attempt_id: string | null;
};

export async function fetchWorkFeedback(company: string, work: string): Promise<WorkFeedback[]> {
	const response = await fetch(workUrl(company, work, 'feedback'), { credentials: 'same-origin' });
	return (await ownerJson<{ feedback: WorkFeedback[] }>(response)).feedback;
}

/** Metered model spend per Work, in dollars. Owner only. */
export async function fetchWorkSpend(company: string): Promise<Map<string, number>> {
	const response = await fetch(`${base(company)}/work-spend`, { credentials: 'same-origin' });
	const body = await ownerJson<{ work: { work_id: string; spent_usd: number }[] }>(response);
	return new Map(body.work.map((row) => [row.work_id, row.spent_usd]));
}

export type GoalDetails = { title?: string; done_when?: string; due_on?: string | null };

export function setGoalDetails(company: string, goal: string, details: GoalDetails) {
	return post(goalUrl(company, goal, 'details'), details);
}

export function closeGoal(company: string, goal: string) {
	return post(goalUrl(company, goal, 'close'));
}

export function reopenGoal(company: string, goal: string) {
	return post(goalUrl(company, goal, 'reopen'));
}

/** null hands the Goal back to the owner. */
export function setGoalTeam(company: string, goal: string, team: string | null) {
	return post(goalUrl(company, goal, 'team'), { team_id: team });
}

/** Priority in words; normal says nothing. */
export function priorityLabel(priority: number): string {
	return priority > 0 ? 'High priority' : priority < 0 ? 'Low priority' : '';
}

export const PRIORITIES = [
	{ value: 1, label: 'High' },
	{ value: 0, label: 'Normal' },
	{ value: -1, label: 'Low' }
] as const;

/* Why Work is waiting, in one sentence: its recorded resolution, without the "changes requested
 * by lead-id:" prefix the record keeps for attribution. Empty when nothing was recorded. */
/* A steer the server records as `<verb> by <actor>: <reason>`, in owner words. */
const STEER_VERB: Record<string, string> = {
	'changes requested': 'Changes requested',
	paused: 'Paused',
	resumed: 'Resumed',
	restored: 'Restored',
	abandoned: 'Stopped'
};
export function steerResolution(resolution: string): string {
	return resolution
		.trim()
		.replace(
			/^(changes requested|paused|resumed|restored|abandoned) by [\w-]+:\s*/i,
			(_, verb: string) => `${STEER_VERB[verb.toLowerCase()]}: `
		);
}

export function blockerSentence(resolution: string, summary = ''): string {
	/* A run that failed is said as a sentence, never the runtime's "[transport] …" (S85-06). */
	const said = failureLine(resolution.trim() ? steerResolution(resolution) : summary.trim());
	const first = said.split('\n')[0];
	return first.length > 180 ? `${first.slice(0, 177).trimEnd()}…` : first;
}
