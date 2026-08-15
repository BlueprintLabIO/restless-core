/**
 * View models for the four surfaces.
 *
 * These are what components take — already mapped, never a raw projection. The
 * seam is deliberate: when the OrgIntel read API exists, a client returns these
 * shapes and no component changes. `$lib/api/map` is that mapping today.
 *
 * Writes are not modelled here. Every write in this product is a governed change
 * that has to travel an authority path, so components receive callbacks and the
 * caller owns the authority — a component that fetched for itself would be a
 * second write path.
 */

export type StatusKind = 'working' | 'waiting' | 'blocked' | 'offline';

/** How a piece of work stands. Drives the glyph, never colour alone. */
export type WorkState = 'done' | 'doing' | 'queued' | 'waiting';

export interface Person {
	id: string;
	name: string;
	role: string;
	initials: string;
	/** Employee tint. Identity only — it never encodes state. */
	tint: string;
	status: StatusKind;
	/** What they are doing now, not what they last said. */
	focus: string;
	when: string;
	/** Who they answer to. null means they answer to you. */
	reportsTo: string | null;
}

/** A person placed in the reporting tree. */
export interface ReportingNode {
	person: Person | null;
	/** The root is you — the one node in the chart that is not an employee. */
	you: boolean;
	depth: number;
	reports: number;
}

export interface WorkItem {
	id: string;
	title: string;
	goal: string;
	state: WorkState;
	note: string;
}

export interface PersonDetail extends Person {
	statusLabel: string;
	now: { runId: string; title: string; note: string };
	work: WorkItem[];
	mayAlone: string[];
	needsYou: string[];
	settingsCount: number;
	spend: { spent: string; ceiling: string; fraction: number };
	madeLately: { path: string; when: string }[];
}

export interface InboxItem {
	id: string;
	title: string;
	kind: string;
	who: { name: string; initials: string; tint: string };
	ago: string;
	quote: { label: string; to: string; body: string } | null;
	/** The standing setting that made this a question instead of an action. */
	why: string;
	primaryAction: string;
	secondaryAction: string;
}

export interface TreeNode {
	id: string;
	depth: number;
	label: string;
	kind: 'objective' | 'goal' | 'step' | 'task';
	state: WorkState;
	meta: string;
	owner: { initials: string; tint: string } | null;
	/** null = a leaf, no disclosure control. */
	expanded: boolean | null;
	selected: boolean;
}

export interface KanbanCard {
	id: string;
	title: string;
	goal: string;
	owner: { initials: string; tint: string };
	taskId: string;
	cost: string;
}

export interface KanbanColumn {
	id: string;
	name: string;
	count: number;
	/** Waiting-on-you is the only column that annunciates. */
	waiting: boolean;
	/**
	 * Yields its width when the executive is expanded. The dock costs 380px and
	 * on this surface that is a whole column, so one column is nominated to pay
	 * rather than letting all four go unreadably narrow.
	 */
	secondary: boolean;
	note: string | null;
	cards: KanbanCard[];
}

export interface AuthorityRow {
	id: string;
	subject: { name: string; initials: string; tint: string } | null;
	setting: string;
	standing: { label: string; tone: 'ok' | 'ask' | 'no' };
	setBy: string;
	/** Invariants cannot be changed — not by you, and not by the company. */
	invariant: boolean;
}

export interface AuthorityGroup {
	id: string;
	name: string;
	icon: string;
	rows: AuthorityRow[];
}

export interface ChatMessage {
	from: 'agent' | 'you';
	text: string;
	/** The receipt under a claim: what actually moved. */
	did: string | null;
	didState: WorkState | null;
}

/** What the executive dock shows on a given surface. */
export interface DockView {
	name: string;
	role: string;
	initials: string;
	tint: string;
	status: StatusKind;
	/** What she can see and act on here, stated rather than implied. */
	context: string;
	messages: ChatMessage[];
	placeholder: string;
	foot: string;
	/** How many things are waiting on you — shown when collapsed. */
	waiting: number;
}

export interface Company {
	id: string;
	name: string;
	mark: string;
	inboxCount: number;
}
