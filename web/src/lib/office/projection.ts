import type { CockpitPerson, CockpitView } from '$lib/model/cockpit';
import type {
	ArtifactRefRow,
	WorkAttemptRow,
	WorkGateRow,
	WorkGraphSnapshot,
	WorkRow
} from '$lib/model/generated/orgintel';

export type OfficePresence = 'observed' | 'in-motion' | 'waiting' | 'available' | 'unavailable';

export interface OfficeMember {
	actorId: string;
	numericId: number;
	display: string;
	role: string;
	teamId: string | null;
	teamName: string | null;
	isTeamLead: boolean;
	palette: number;
	presence: OfficePresence;
	presenceLabel: string;
	sessionObserved: boolean;
	work: WorkRow | null;
	attempt: WorkAttemptRow | null;
	attemptLabel: string;
	activityDetail: string;
	evidenceLabel: string;
	outputCount: number;
	workHref: string | null;
}

function stableHash(value: string): number {
	let hash = 2166136261;
	for (let index = 0; index < value.length; index += 1) {
		hash ^= value.charCodeAt(index);
		hash = Math.imul(hash, 16777619);
	}
	return hash >>> 0;
}

function latestAttemptFor(work: WorkRow | null, graph: WorkGraphSnapshot): WorkAttemptRow | null {
	if (!work) return null;
	return (
		graph.attempts
			.filter((attempt) => attempt.work_id === work.id)
			.toSorted(
				(a, b) =>
					a.revision - b.revision ||
					a.attempt_no - b.attempt_no ||
					Date.parse(a.started_at) - Date.parse(b.started_at)
			)
			.at(-1) ?? null
	);
}

function workFor(person: CockpitPerson, graph: WorkGraphSnapshot): WorkRow | null {
	const priority = { active: 0, blocked: 1, proposed: 2, completed: 3, abandoned: 4 } as const;
	return (
		graph.work
			.filter((work) => work.owner_id === person.actor_id && work.status !== 'abandoned')
			.toSorted(
				(a, b) =>
					priority[a.status] - priority[b.status] ||
					b.priority - a.priority ||
					Date.parse(b.updated_at) - Date.parse(a.updated_at)
			)
			.at(0) ?? null
	);
}

function evidenceFor(
	work: WorkRow | null,
	attempt: WorkAttemptRow | null,
	artifacts: ArtifactRefRow[],
	gates: WorkGateRow[],
	graph: WorkGraphSnapshot
): string {
	if (!work) return 'No current Work';
	const outputs = artifacts.filter((artifact) => artifact.work_id === work.id).length;
	const workGates = gates.filter((gate) => gate.work_id === work.id);
	const passed = attempt
		? workGates.filter((gate) =>
				graph.gate_runs.some(
					(run) => run.gate_id === gate.id && run.attempt_id === attempt.id && run.passed
				)
			).length
		: 0;
	return `${outputs} output${outputs === 1 ? '' : 's'} · ${passed}/${workGates.length} gates`;
}

function attemptText(attempt: WorkAttemptRow | null): string {
	if (!attempt) return 'No Attempt started';
	return `Attempt ${attempt.attempt_no} · ${attempt.state.replaceAll('_', ' ')}`;
}

function presenceFor(
	person: CockpitPerson,
	work: WorkRow | null,
	runtimeAvailable: boolean
): { presence: OfficePresence; label: string; detail: string } {
	if (!runtimeAvailable) {
		return {
			presence: 'unavailable',
			label: 'Signal unavailable',
			detail: work ? `${work.title} remains ${work.status}.` : 'No activity claim is being made.'
		};
	}
	if (person.session_running) {
		return {
			presence: 'observed',
			label: 'Working now',
			detail: work ? work.title : 'A company session is running without linked Work.'
		};
	}
	if (work?.status === 'blocked') {
		return {
			presence: 'waiting',
			label: 'Waiting',
			detail: work.title
		};
	}
	if (work?.status === 'active') {
		return {
			presence: 'in-motion',
			label: 'Work in motion',
			detail: work.title
		};
	}
	return {
		presence: 'available',
		label: 'Available',
		detail: work?.title ?? 'Ready for the next outcome.'
	};
}

function workHref(companyId: string, work: WorkRow | null): string | null {
	if (!work) return null;
	const query = new URLSearchParams({ lens: 'map' });
	if (work.goal_id) query.set('goal', work.goal_id);
	return `/${encodeURIComponent(companyId)}/work/${encodeURIComponent(work.id)}?${query}`;
}

/**
 * Build the office from the same source-owned People and Work rows used by the
 * cockpit. This projection does not create actors, Work states or liveness.
 */
export function projectOfficeMembers(
	companyId: string,
	cockpit: CockpitView,
	graph: WorkGraphSnapshot
): OfficeMember[] {
	const runtimeAvailable = ['available', 'running'].includes(cockpit.source_health.runtime);
	return cockpit.people
		.filter((person) => person.kind === 'exec' || person.kind === 'staff')
		.toSorted((a, b) => {
			const aTeam = cockpit.teams.find((team) => team.id === a.team_id);
			const bTeam = cockpit.teams.find((team) => team.id === b.team_id);
			return (
				Number(b.kind === 'exec') - Number(a.kind === 'exec') ||
				(a.team_id ?? '').localeCompare(b.team_id ?? '') ||
				Number(bTeam?.lead_actor_id === b.actor_id) - Number(aTeam?.lead_actor_id === a.actor_id) ||
				a.display.localeCompare(b.display)
			);
		})
		.map((person) => {
			const work = workFor(person, graph);
			const attempt = latestAttemptFor(work, graph);
			const projectedPresence = presenceFor(person, work, runtimeAvailable);
			const hash = stableHash(person.actor_id);
			const team = cockpit.teams.find((candidate) => candidate.id === person.team_id);
			const outputCount = work
				? graph.artifacts.filter((artifact) => artifact.work_id === work.id).length
				: 0;
			return {
				actorId: person.actor_id,
				numericId: hash || 1,
				display: person.display,
				role: person.role,
				teamId: person.team_id,
				teamName: team?.name ?? null,
				isTeamLead: team?.lead_actor_id === person.actor_id,
				palette: hash % 6,
				presence: projectedPresence.presence,
				presenceLabel: projectedPresence.label,
				sessionObserved: projectedPresence.presence === 'observed',
				work,
				attempt,
				attemptLabel: attemptText(attempt),
				activityDetail: projectedPresence.detail,
				evidenceLabel: evidenceFor(work, attempt, graph.artifacts, graph.gates, graph),
				outputCount,
				workHref: workHref(companyId, work)
			};
		});
}
