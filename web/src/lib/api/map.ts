/**
 * Wire shapes → view models.
 *
 * The seam lives here so surfaces stay ignorant of the daemon. Everything below
 * is a pure function of what the API actually returned; nothing invents a field
 * the backend does not have. Where the design wants something that does not
 * exist — reporting lines, a four-level goal tree — the mapping stops rather
 * than fabricating it, and the surface renders the gap.
 */

import type { ApiCommitment, ApiEvent, ApiGoal, ApiPerson, ApiSpend } from './client';
import type { KanbanColumn, Person, PersonDetail, TreeNode, WorkState } from '$lib/model/view';

/**
 * Employee tints. Identity only, and assigned here rather than by the daemon —
 * a colour is a property of this screen, not of the company.
 */
const PALETTE = ['#7A6BA8', '#3F7A75', '#A3651F', '#5B7596', '#62744C', '#B04E72'];

export function tintFor(actorId: string): string {
	let hash = 0;
	for (const char of actorId) hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
	return PALETTE[hash % PALETTE.length];
}

export function initialsFor(name: string): string {
	const cleaned = name.replace(/[^a-zA-Z0-9 ]/g, ' ').trim();
	const words = cleaned.split(/\s+/).filter(Boolean);
	if (words.length >= 2) return (words[0][0] + words[1][0]).toUpperCase();
	return (cleaned.slice(0, 2) || '??').toUpperCase();
}

function ago(iso: string): string {
	const then = new Date(iso).getTime();
	if (Number.isNaN(then)) return '';
	const minutes = Math.floor((Date.now() - then) / 60000);
	if (minutes < 1) return 'now';
	if (minutes < 60) return `${minutes}m`;
	const hours = Math.floor(minutes / 60);
	if (hours < 24) return `${hours}h`;
	return `${Math.floor(hours / 24)}d`;
}

const stateOf = (commitment: ApiCommitment): WorkState => {
	switch (commitment.state) {
		case 'completed':
			return 'done';
		case 'blocked':
			return 'waiting';
		case 'proposed':
			return 'queued';
		default:
			return 'doing';
	}
};

/**
 * Actors that are not employees. `owner` is you — you are the root of the
 * company, not a row in its directory — and `daemon` / `world` are machinery
 * OrgIntel needs a foreign key for. Listing them as staff would tell the owner
 * they have hired themselves.
 */
const NOT_STAFF = new Set(['owner', 'daemon', 'world']);

/**
 * The directory. `session_running` is a probe, so it drives the status dot —
 * "working" here means a session is actually up, not that we assume one is.
 */
export function toPeople(rows: ApiPerson[], commitments: ApiCommitment[]): Person[] {
	return rows
		.filter((row) => !NOT_STAFF.has(row.actor_id) && !NOT_STAFF.has(row.role))
		.map((row) => {
			const theirs = commitments.filter((c) => c.owner_id === row.actor_id);
			const blocked = theirs.find((c) => c.state === 'blocked');
			const open = theirs.find((c) => c.state === 'active');
			return {
				id: row.actor_id,
				name: row.display || row.actor_id,
				role: row.role,
				initials: initialsFor(row.display || row.actor_id),
				tint: tintFor(row.actor_id),
				status: blocked ? 'waiting' : row.session_running ? 'working' : 'offline',
				focus: blocked
					? `Blocked — ${blocked.title}`
					: open
						? open.title
						: row.session_running
							? 'Working'
							: 'Idle',
				when: '',
				// The daemon has no reporting lines yet (docs/api/MISSING.md §4), and
				// this is where that would be filled in. Null is the honest answer.
				reportsTo: null
			};
		});
}

export function toPersonDetail(
	person: Person,
	row: ApiPerson,
	commitments: ApiCommitment[],
	spend: ApiSpend | null
): PersonDetail {
	const theirs = commitments.filter((c) => c.owner_id === person.id);
	const current = theirs.find((c) => c.state === 'active') ?? theirs.find((c) => c.state === 'proposed');
	const mine = spend?.by_actor.find((entry) => entry.actor === person.id);
	const ceiling = spend?.ceiling_usd ?? 0;
	return {
		...person,
		statusLabel: row.session_running ? 'Working' : 'No session running',
		now: {
			runId: row.model ?? 'inherited',
			title: current?.title ?? 'Nothing open',
			note: current?.body ?? ''
		},
		work: theirs.map((c) => ({
			id: c.id,
			title: c.title,
			goal: c.goal_id ? 'goal' : 'unassigned',
			state: stateOf(c),
			note: c.state === 'blocked' ? c.resolution || 'blocked' : c.state
		})),
		// Authority per person is a stub (docs/api/MISSING.md §1).
		mayAlone: [],
		needsYou: [],
		settingsCount: 0,
		spend: {
			spent: `$${(mine?.spent_usd ?? row.spent_usd).toFixed(2)}`,
			ceiling: spend?.poisoned ? 'accounting stopped' : `$${ceiling.toFixed(0)} ceiling`,
			fraction: ceiling > 0 ? Math.min(1, (mine?.spent_usd ?? row.spent_usd) / ceiling) : 0
		},
		// Artifacts are a stub (docs/api/MISSING.md §5).
		madeLately: []
	};
}

/**
 * The goal tree, at the depth the backend actually has: goal → commitment.
 *
 * The design draws four levels. Two exist. Rendering the two that are real is
 * the honest version; inventing the other two would make the tree a drawing
 * again. See docs/api/MISSING.md, "deliberately not proposed".
 */
export function toGoalTree(goals: ApiGoal[], commitments: ApiCommitment[]): TreeNode[] {
	const nodes: TreeNode[] = [];
	const unassigned = commitments.filter((c) => !c.goal_id);

	for (const goal of goals) {
		const children = commitments.filter((c) => c.goal_id === goal.id);
		const done = children.filter((c) => c.state === 'completed').length;
		nodes.push({
			id: goal.id,
			depth: 0,
			label: goal.title,
			kind: 'goal',
			state: goal.closed_at ? 'done' : 'doing',
			meta: children.length > 0 ? `${done}/${children.length}` : '',
			owner: null,
			expanded: children.length > 0 ? true : null,
			selected: false
		});
		for (const child of children) {
			nodes.push({
				id: child.id,
				depth: 1,
				label: child.title,
				kind: 'task',
				state: stateOf(child),
				meta: child.state === 'blocked' ? 'blocked' : '',
				owner: {
					initials: initialsFor(child.owner_id),
					tint: tintFor(child.owner_id)
				},
				expanded: null,
				selected: false
			});
		}
	}

	if (unassigned.length > 0) {
		nodes.push({
			id: 'unassigned',
			depth: 0,
			label: 'Not under a goal',
			kind: 'objective',
			state: 'queued',
			meta: String(unassigned.length),
			owner: null,
			expanded: true,
			selected: false
		});
		for (const child of unassigned) {
			nodes.push({
				id: child.id,
				depth: 1,
				label: child.title,
				kind: 'task',
				state: stateOf(child),
				meta: '',
				owner: { initials: initialsFor(child.owner_id), tint: tintFor(child.owner_id) },
				expanded: null,
				selected: false
			});
		}
	}

	return nodes;
}

/**
 * Three columns, from the three states a commitment actually has. There is no
 * "done this week" window in the data — `updated_at` exists, so it could be
 * derived, but a week is a product decision nobody has made.
 */
export function toColumns(commitments: ApiCommitment[]): KanbanColumn[] {
	const card = (c: ApiCommitment) => ({
		id: c.id,
		title: c.title,
		goal: c.goal_id ? 'goal' : 'unassigned',
		owner: { initials: initialsFor(c.owner_id), tint: tintFor(c.owner_id) },
		taskId: c.id.slice(0, 8),
		cost: ago(c.updated_at)
	});
	const by = (...states: ApiCommitment['state'][]) =>
		commitments.filter((c) => states.includes(c.state));

	// One column per state the data actually has. `abandoned` gets no column on
	// purpose: it is not work in flight and not work done, and giving it a lane
	// would put dead entries permanently in front of the owner.
	return [
		{
			id: 'proposed',
			name: 'Proposed',
			count: by('proposed').length,
			waiting: false,
			secondary: false,
			note: null,
			cards: by('proposed').map(card)
		},
		{
			id: 'active',
			name: 'Active',
			count: by('active').length,
			waiting: false,
			secondary: false,
			note: null,
			cards: by('active').map(card)
		},
		{
			id: 'blocked',
			name: 'Blocked on you',
			count: by('blocked').length,
			waiting: true,
			secondary: false,
			note: by('blocked').length > 0 ? 'these are what the inbox is for ▸' : null,
			cards: by('blocked').map(card)
		},
		{
			id: 'completed',
			name: 'Completed',
			count: by('completed').length,
			waiting: false,
			secondary: true,
			note: null,
			cards: by('completed').map(card)
		}
	];
}

export function eventLine(event: ApiEvent): string {
	const body = event.body as Record<string, unknown> | null;
	const summary =
		(body && (body.summary ?? body.title ?? body.message ?? body.capability)) ?? event.kind;
	return `${event.kind} · ${String(summary)}`;
}
