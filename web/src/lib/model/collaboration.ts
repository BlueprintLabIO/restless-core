import type {
	ArtifactRefState,
	OwnerHandoffCategory,
	WorkAttemptState,
	WorkEdgeKind,
	WorkStatus
} from './generated/orgintel';
import type { CompanyMembershipRole } from './query-persistence';

export const COLLABORATION_SCHEMA_VERSION = 'member-collaboration.v1';

export interface CollaborationPrincipal {
	actor_id: string;
	membership_role: CompanyMembershipRole;
	cache_partition: string;
}

export interface CollaborationCompany {
	id: string;
	company_id: string | null;
	name: string;
	mission: string;
}

export interface CollaborationPerson {
	actor_id: string;
	kind: string;
	actor_class: string;
	role: string;
	display: string;
	team_id: string | null;
	session_running: boolean;
}

export interface CollaborationTeam {
	id: string;
	name: string;
	brief: string;
	lead_actor_id: string;
	member_count: number;
	in_motion_count: number;
	blocked_count: number;
}

export interface CollaborationGoal {
	id: string;
	title: string;
	body: string;
	closed_at: string | null;
}

export interface CollaborationWork {
	id: string;
	goal_id: string | null;
	owner_id: string;
	title: string;
	outcome: string;
	status: WorkStatus;
	resolution: string;
	priority: number;
	expected_artifact: string;
	revision: number;
	updated_at: string;
}

export interface CollaborationWorkEdge {
	from_work_id: string;
	to_work_id: string;
	kind: WorkEdgeKind;
}

export interface CollaborationAttempt {
	id: string;
	work_id: string;
	revision: number;
	attempt_no: number;
	actor_id: string;
	state: WorkAttemptState;
	source_commit: string | null;
	terminal_source_commit: string | null;
	started_at: string;
	finished_at: string | null;
	summary: string;
}

export interface CollaborationArtifact {
	id: string;
	work_id: string;
	attempt_id: string | null;
	kind: string;
	label: string;
	note: string;
	state: ArtifactRefState;
	digest: string | null;
	source_commit: string | null;
	href: string | null;
	created_at: string;
}

export interface CollaborationHandoff {
	id: string;
	work_id: string;
	attempt_id: string | null;
	requested_by: string;
	category: OwnerHandoffCategory;
	requested_action: string;
	prepared_state: string;
	resume_condition: string;
	created_at: string;
}

export interface CollaborationMention {
	id: string;
	room_id: string;
	room_title: string;
	message_id: number;
	thread_root_message_id: number;
	from_actor: string;
	body: string;
	work_id: string | null;
	why_this_actor: string | null;
	expected_response: string | null;
	recommendation: string | null;
	alternatives: string[];
	evidence: string[];
	uncertainty: string | null;
	affected_scope: string | null;
	deadline_at: string | null;
	fallback: string | null;
	independent_work_can_continue: boolean;
	created_at: string;
}

export interface CollaborationBootstrap {
	schema_version: typeof COLLABORATION_SCHEMA_VERSION;
	principal: CollaborationPrincipal;
	company: CollaborationCompany;
	people: CollaborationPerson[];
	teams: CollaborationTeam[];
	goals: CollaborationGoal[];
	work_graph: {
		work: CollaborationWork[];
		edges: CollaborationWorkEdge[];
		attempts: CollaborationAttempt[];
		artifacts: CollaborationArtifact[];
	};
	attention: {
		handoffs: CollaborationHandoff[];
		mentions: CollaborationMention[];
	};
	source_health: { orgintel: string };
	refreshed_at: string;
}

function invalid(): never {
	throw Object.assign(new Error('The company collaboration response is invalid.'), {
		code: 'invalid_collaboration'
	});
}

function object(value: unknown): Record<string, unknown> {
	if (!value || typeof value !== 'object' || Array.isArray(value)) invalid();
	return value as Record<string, unknown>;
}

function text(value: unknown): string {
	if (typeof value !== 'string') invalid();
	return value;
}

function nullableText(value: unknown): string | null {
	if (value === null) return null;
	return text(value);
}

function number(value: unknown): number {
	if (typeof value !== 'number' || !Number.isFinite(value)) invalid();
	return value;
}

function integer(value: unknown): number {
	const parsed = number(value);
	if (!Number.isInteger(parsed)) invalid();
	return parsed;
}

function boolean(value: unknown): boolean {
	if (typeof value !== 'boolean') invalid();
	return value;
}

function list(value: unknown): unknown[] {
	if (!Array.isArray(value)) invalid();
	return value;
}

function stringList(value: unknown): string[] {
	return list(value).map(text);
}

function oneOf<T extends string>(value: unknown, values: readonly T[]): T {
	const parsed = text(value);
	if (!values.includes(parsed as T)) invalid();
	return parsed as T;
}

const MEMBERSHIP_ROLES = ['owner', 'admin', 'member'] as const;
const WORK_STATUSES = ['proposed', 'active', 'blocked', 'completed', 'abandoned'] as const;
const EDGE_KINDS = ['requires', 'revises'] as const;
const ATTEMPT_STATES = [
	'running',
	'produced',
	'changes_requested',
	'blocked',
	'failed',
	'abandoned',
	'superseded'
] as const;
const ARTIFACT_STATES = ['available', 'stale', 'missing', 'superseded', 'unknown'] as const;
const HANDOFF_CATEGORIES = [
	'identity',
	'captcha',
	'mfa',
	'legal_attestation',
	'payment_confirmation',
	'owner_judgement'
] as const;

function parsePrincipal(value: unknown): CollaborationPrincipal {
	const row = object(value);
	return {
		actor_id: text(row.actor_id),
		membership_role: oneOf(row.membership_role, MEMBERSHIP_ROLES),
		cache_partition: text(row.cache_partition)
	};
}

function parsePerson(value: unknown): CollaborationPerson {
	const row = object(value);
	return {
		actor_id: text(row.actor_id),
		kind: text(row.kind),
		actor_class: text(row.actor_class),
		role: text(row.role),
		display: text(row.display),
		team_id: nullableText(row.team_id),
		session_running: boolean(row.session_running)
	};
}

function parseTeam(value: unknown): CollaborationTeam {
	const row = object(value);
	return {
		id: text(row.id),
		name: text(row.name),
		brief: text(row.brief),
		lead_actor_id: text(row.lead_actor_id),
		member_count: integer(row.member_count),
		in_motion_count: integer(row.in_motion_count),
		blocked_count: integer(row.blocked_count)
	};
}

function parseGoal(value: unknown): CollaborationGoal {
	const row = object(value);
	return {
		id: text(row.id),
		title: text(row.title),
		body: text(row.body),
		closed_at: nullableText(row.closed_at)
	};
}

function parseWork(value: unknown): CollaborationWork {
	const row = object(value);
	return {
		id: text(row.id),
		goal_id: nullableText(row.goal_id),
		owner_id: text(row.owner_id),
		title: text(row.title),
		outcome: text(row.outcome),
		status: oneOf(row.status, WORK_STATUSES),
		resolution: text(row.resolution),
		priority: integer(row.priority),
		expected_artifact: text(row.expected_artifact),
		revision: integer(row.revision),
		updated_at: text(row.updated_at)
	};
}

function parseEdge(value: unknown): CollaborationWorkEdge {
	const row = object(value);
	return {
		from_work_id: text(row.from_work_id),
		to_work_id: text(row.to_work_id),
		kind: oneOf(row.kind, EDGE_KINDS)
	};
}

function parseAttempt(value: unknown): CollaborationAttempt {
	const row = object(value);
	return {
		id: text(row.id),
		work_id: text(row.work_id),
		revision: integer(row.revision),
		attempt_no: integer(row.attempt_no),
		actor_id: text(row.actor_id),
		state: oneOf(row.state, ATTEMPT_STATES),
		source_commit: nullableText(row.source_commit),
		terminal_source_commit: nullableText(row.terminal_source_commit),
		started_at: text(row.started_at),
		finished_at: nullableText(row.finished_at),
		summary: text(row.summary)
	};
}

function parseArtifact(value: unknown): CollaborationArtifact {
	const row = object(value);
	return {
		id: text(row.id),
		work_id: text(row.work_id),
		attempt_id: nullableText(row.attempt_id),
		kind: text(row.kind),
		label: text(row.label),
		note: text(row.note),
		state: oneOf(row.state, ARTIFACT_STATES),
		digest: nullableText(row.digest),
		source_commit: nullableText(row.source_commit),
		href: nullableText(row.href),
		created_at: text(row.created_at)
	};
}

function parseHandoff(value: unknown): CollaborationHandoff {
	const row = object(value);
	return {
		id: text(row.id),
		work_id: text(row.work_id),
		attempt_id: nullableText(row.attempt_id),
		requested_by: text(row.requested_by),
		category: oneOf(row.category, HANDOFF_CATEGORIES),
		requested_action: text(row.requested_action),
		prepared_state: text(row.prepared_state),
		resume_condition: text(row.resume_condition),
		created_at: text(row.created_at)
	};
}

function parseMention(value: unknown): CollaborationMention {
	const row = object(value);
	return {
		id: text(row.id),
		room_id: text(row.room_id),
		room_title: text(row.room_title),
		message_id: integer(row.message_id),
		thread_root_message_id: integer(row.thread_root_message_id),
		from_actor: text(row.from_actor),
		body: text(row.body),
		work_id: nullableText(row.work_id),
		why_this_actor: nullableText(row.why_this_actor),
		expected_response: nullableText(row.expected_response),
		recommendation: nullableText(row.recommendation),
		alternatives: stringList(row.alternatives),
		evidence: stringList(row.evidence),
		uncertainty: nullableText(row.uncertainty),
		affected_scope: nullableText(row.affected_scope),
		deadline_at: nullableText(row.deadline_at),
		fallback: nullableText(row.fallback),
		independent_work_can_continue: boolean(row.independent_work_can_continue),
		created_at: text(row.created_at)
	};
}

export function parseCollaborationBootstrap(value: unknown): CollaborationBootstrap {
	const root = object(value);
	if (root.schema_version !== COLLABORATION_SCHEMA_VERSION) invalid();
	const company = object(root.company);
	const workGraph = object(root.work_graph);
	const attention = object(root.attention);
	const sourceHealth = object(root.source_health);
	return {
		schema_version: COLLABORATION_SCHEMA_VERSION,
		principal: parsePrincipal(root.principal),
		company: {
			id: text(company.id),
			company_id: nullableText(company.company_id),
			name: text(company.name),
			mission: text(company.mission)
		},
		people: list(root.people).map(parsePerson),
		teams: list(root.teams).map(parseTeam),
		goals: list(root.goals).map(parseGoal),
		work_graph: {
			work: list(workGraph.work).map(parseWork),
			edges: list(workGraph.edges).map(parseEdge),
			attempts: list(workGraph.attempts).map(parseAttempt),
			artifacts: list(workGraph.artifacts).map(parseArtifact)
		},
		attention: {
			handoffs: list(attention.handoffs).map(parseHandoff),
			mentions: list(attention.mentions).map(parseMention)
		},
		source_health: { orgintel: text(sourceHealth.orgintel) },
		refreshed_at: text(root.refreshed_at)
	};
}

export function collaborationMatchesPrincipal(
	view: CollaborationBootstrap,
	principal: CollaborationPrincipal
): boolean {
	return (
		view.principal.actor_id === principal.actor_id &&
		view.principal.membership_role === principal.membership_role &&
		view.principal.cache_partition === principal.cache_partition
	);
}

export async function getCollaborationBootstrap(
	company: string,
	signal?: AbortSignal
): Promise<CollaborationBootstrap> {
	const response = await fetch(
		`/api/companies/${encodeURIComponent(company)}/collaboration/bootstrap`,
		{ credentials: 'same-origin', cache: 'no-store', signal }
	);
	if (!response.ok) {
		let message = `${response.status} ${response.statusText}`;
		try {
			const body = object(await response.json());
			if (typeof body.message === 'string') message = body.message;
		} catch {
			// Preserve the transport status when an intermediary returns non-JSON.
		}
		throw Object.assign(new Error(message), { status: response.status });
	}
	const view = parseCollaborationBootstrap(await response.json());
	if (view.company.id !== company) invalid();
	return view;
}
