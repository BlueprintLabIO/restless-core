import type { CockpitPerson, CockpitView } from '$lib/model/cockpit';
import type {
	ArtifactRefRow,
	WorkAttemptRow,
	WorkGateRow,
	WorkGraphSnapshot,
	WorkRow
} from '$lib/model/generated/orgintel';

export type OfficePresence =
	'observed' | 'in-motion' | 'waiting' | 'available' | 'stale' | 'unknown' | 'unavailable';

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
	semanticActivity: boolean;
	sessionObserved: boolean;
	presenceObservedAt: string | null;
	currentStep: string | null;
	work: WorkRow | null;
	attempt: WorkAttemptRow | null;
	attemptLabel: string;
	activityDetail: string;
	evidenceLabel: string;
	outputCount: number;
	workHref: string | null;
	personHref: string;
}

export interface OfficeProjectionObservation {
	runtimeStatus?: string;
	orgintelStatus?: string;
	now?: Date;
}

export const FRESH_OBSERVATION_MS = 20_000;

function stableHash(value: string): number {
	let hash = 2166136261;
	for (let index = 0; index < value.length; index += 1) {
		hash ^= value.charCodeAt(index);
		hash = Math.imul(hash, 16777619);
	}
	return hash >>> 0;
}

function attemptsFor(work: WorkRow | null, graph: WorkGraphSnapshot): WorkAttemptRow[] {
	if (!work) return [];
	return graph.attempts
		.filter((attempt) => attempt.work_id === work.id)
		.toSorted(
			(a, b) =>
				a.revision - b.revision ||
				a.attempt_no - b.attempt_no ||
				Date.parse(a.started_at) - Date.parse(b.started_at)
		);
}

function latestAttemptFor(work: WorkRow | null, graph: WorkGraphSnapshot): WorkAttemptRow | null {
	return attemptsFor(work, graph).at(-1) ?? null;
}

function observedAttemptFor(
	person: CockpitPerson,
	graph: WorkGraphSnapshot
): WorkAttemptRow | null {
	return (
		graph.attempts
			.filter((attempt) => attempt.actor_id === person.actor_id && attempt.state === 'running')
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
	const observedAttempt = observedAttemptFor(person, graph);
	if (observedAttempt) {
		const observedWork = graph.work.find((work) => work.id === observedAttempt.work_id);
		if (observedWork) return observedWork;
	}
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

function validObservationTime(value: string | null): number | null {
	if (!value) return null;
	const parsed = Date.parse(value);
	return Number.isFinite(parsed) ? parsed : null;
}

function presenceFor(
	person: CockpitPerson,
	work: WorkRow | null,
	observedAttempt: WorkAttemptRow | null,
	runtimeAvailable: boolean,
	orgintelAvailable: boolean,
	now: Date
): {
	presence: OfficePresence;
	label: string;
	detail: string;
	semantic: boolean;
	observedAt: string | null;
	currentStep: string | null;
} {
	if (!runtimeAvailable || !orgintelAvailable) {
		return {
			presence: 'unavailable',
			label: 'Signal unavailable',
			detail: work ? `${work.title} remains ${work.status}.` : 'No activity claim is being made.',
			semantic: false,
			observedAt: null,
			currentStep: null
		};
	}
	if (person.session_running) {
		const observedTime = validObservationTime(person.session_observed_at);
		if (observedTime === null) {
			return {
				presence: 'unknown',
				label: 'Session time unknown',
				detail: work?.title ?? 'A runtime session was reported without a usable observation time.',
				semantic: false,
				observedAt: null,
				currentStep: null
			};
		}
		if (now.getTime() - observedTime > FRESH_OBSERVATION_MS) {
			return {
				presence: 'stale',
				label: 'Observation stale',
				detail: work?.title ?? 'The last runtime observation is no longer current.',
				semantic: false,
				observedAt: person.session_observed_at,
				currentStep: null
			};
		}
		if (!observedAttempt || !work || observedAttempt.work_id !== work.id) {
			return {
				presence: 'unknown',
				label: 'Session observed · Work unknown',
				detail: work?.title ?? 'A fresh session is visible without associated Work and Attempt.',
				semantic: false,
				observedAt: person.session_observed_at,
				currentStep: null
			};
		}
		return {
			presence: 'observed',
			label: 'Working now',
			detail: work.title,
			semantic: true,
			observedAt: person.session_observed_at,
			currentStep: observedAttempt.summary.trim() || null
		};
	}
	if (work?.status === 'blocked' || observedAttempt?.state === 'blocked') {
		return {
			presence: 'waiting',
			label: 'Waiting',
			detail: work?.title ?? 'Source-owned Work is blocked.',
			semantic: false,
			observedAt: null,
			currentStep: null
		};
	}
	if (work?.status === 'active') {
		return {
			presence: 'in-motion',
			label: 'Work in motion',
			detail: work.title,
			semantic: false,
			observedAt: null,
			currentStep: null
		};
	}
	return {
		presence: 'available',
		label: 'Available',
		detail: work?.title ?? 'Ready for the next outcome.',
		semantic: false,
		observedAt: null,
		currentStep: null
	};
}

function workHref(companyId: string, work: WorkRow | null): string | null {
	if (!work) return null;
	const query = new URLSearchParams({ lens: 'map' });
	if (work.goal_id) query.set('goal', work.goal_id);
	return `/${encodeURIComponent(companyId)}/work/${encodeURIComponent(work.id)}?${query}`;
}

function personHref(companyId: string, actorId: string): string {
	const query = new URLSearchParams({ person: actorId });
	return `/${encodeURIComponent(companyId)}/people?${query}`;
}

/** Project source-owned People, Work and observations without inventing liveness. */
export function projectOfficeMembers(
	companyId: string,
	cockpit: CockpitView,
	graph: WorkGraphSnapshot,
	observation: OfficeProjectionObservation = {}
): OfficeMember[] {
	const runtimeStatus = observation.runtimeStatus ?? cockpit.source_health.runtime;
	const orgintelStatus = observation.orgintelStatus ?? cockpit.source_health.orgintel;
	const runtimeAvailable = ['available', 'running'].includes(runtimeStatus);
	const orgintelAvailable = orgintelStatus === 'available';
	const now = observation.now ?? new Date();
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
			const runningAttempt = observedAttemptFor(person, graph);
			const attempt = runningAttempt ?? latestAttemptFor(work, graph);
			const projected = presenceFor(
				person,
				work,
				runningAttempt,
				runtimeAvailable,
				orgintelAvailable,
				now
			);
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
				presence: projected.presence,
				presenceLabel: projected.label,
				semanticActivity: projected.semantic,
				sessionObserved: projected.semantic,
				presenceObservedAt: projected.observedAt,
				currentStep: projected.currentStep,
				work,
				attempt,
				attemptLabel: attemptText(attempt),
				activityDetail: projected.detail,
				evidenceLabel: evidenceFor(work, attempt, graph.artifacts, graph.gates, graph),
				outputCount,
				workHref: workHref(companyId, work),
				personHref: personHref(companyId, person.actor_id)
			};
		});
}
