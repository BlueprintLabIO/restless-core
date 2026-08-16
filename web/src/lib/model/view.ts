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

/* ============ attention — the owner queue ============ */

/**
 * One common read envelope over source-owned work, produced by
 * `restlessd::attention::project`. Not modelled here as a union of workflow
 * types: `category` is deliberately open, so a new kind of owner moment adds
 * neither a mutation type nor a page component.
 *
 * `source` and `actions` are the load-bearing fields. Attention is a
 * projection and can resolve nothing by itself — Authority owns approvals and
 * OrgIntel owns blocked commitments, so every action names the plane that
 * will actually carry it out.
 *
 * Unlike `InboxItem` below, this is not a shape the frontend invented. It
 * mirrors the Rust struct field for field; the snake_case → camelCase mapping
 * is the only liberty taken, in `$lib/api/attention`.
 */
export interface AttentionItem {
	id: string;
	source: {
		plane: 'authority' | 'orgintel' | 'runtime' | string;
		kind: string;
		reference: string;
	};
	category: 'approval' | 'review' | 'blocker' | string;
	title: string;
	whatHappened: string;
	whyItMatters: string;
	recommendation: string;
	requestedAction: string;
	/** What happens if the owner closes the tab. Never blank — silence is a claim. */
	ifNoAction: string;
	evidence: Array<{
		label: string;
		kind: string;
		uri?: string;
		content?: string;
	}>;
	/** Present when the last mile is a live browser the owner can take over. */
	runtimeAttach?: {
		company: string;
		generation: string;
		requestingActor?: string;
		kind: 'persistent-browser';
	};
	actions: Array<{
		id: string;
		label: string;
		consequence: string;
	}>;
	/** Whether the company can keep working on other things while this waits. */
	canContinue: boolean;
	createdAt: Date | string;
}

/** Whether each plane could answer. A degraded source must read as unknown,
 *  never as an empty queue — "nothing needs you" is the one lie this surface
 *  cannot tell. */
export interface SourceHealth {
	orgintel: string;
	authority: string;
	runtime: string;
	browser: string;
}

export interface AttentionView {
	company: { id: string; name: string; mission: string; model: string };
	sourceHealth: SourceHealth;
	items: AttentionItem[];
	refreshedAt: string;
}
