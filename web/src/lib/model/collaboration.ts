import type {
	ArtifactRefState,
	OwnerBrief,
	OwnerHandoffCategory,
	WorkAttemptState,
	WorkEdgeKind,
	WorkStatus
} from './generated/orgintel';
import type { CompanyMembershipRole } from './query-persistence';
import { responseFailure } from './failure.ts';

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
	/** The person who owns the team; null means the owner. */
	owner_person: string | null;
	member_count: number;
	in_motion_count: number;
	blocked_count: number;
}

export interface CollaborationGoal {
	id: string;
	title: string;
	body: string;
	closed_at: string | null;
	/** The team that delivers it; null means the owner holds it. */
	team_id: string | null;
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
	/** Where the Library opens it: a company-home path or a service on the company computer. */
	uri?: string;
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
	/** Who passed it here: the team's lead or Exec. */
	escalated_from: string | null;
	/** Exec's prepared brief when this is an outcome to review. */
	owner_brief: OwnerBrief | null;
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

function oneOf<T extends string>(value: unknown, values: readonly T[]): T {
	const parsed = text(value);
	if (!values.includes(parsed as T)) invalid();
	return parsed as T;
}

const MEMBERSHIP_ROLES = ['owner', 'admin', 'member', 'guest'] as const;
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
		owner_person: nullableText(row.owner_person ?? null),
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
		closed_at: nullableText(row.closed_at),
		team_id: nullableText(row.team_id ?? null)
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
		...(typeof row.uri === 'string' ? { uri: row.uri } : {}),
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
		created_at: text(row.created_at),
		escalated_from: nullableText(row.escalated_from ?? null),
		owner_brief: parseBrief(row.owner_brief)
	};
}

const BRIEF_KINDS = [
	'outcome_review',
	'decision',
	'blocker',
	'opportunity',
	'contradiction',
	'human_step'
] as const;

function parseBrief(value: unknown): OwnerBrief | null {
	if (value === null || value === undefined) return null;
	const row = object(value);
	return {
		kind: oneOf(row.kind, BRIEF_KINDS),
		headline: text(row.headline),
		situation: text(row.situation),
		impact: text(row.impact),
		recommendation: text(row.recommendation),
		no_action: text(row.no_action),
		uncertainty: nullableText(row.uncertainty ?? null),
		deadline: nullableText(row.deadline ?? null)
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
			handoffs: list(attention.handoffs).map(parseHandoff)
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
	if (!response.ok) throw await responseFailure(response);
	const view = parseCollaborationBootstrap(await response.json());
	if (view.company.id !== company) invalid();
	return view;
}
