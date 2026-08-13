/**
 * The view models every surface renders.
 *
 * In the source system these types were the *output* of a mapper that read a
 * `CompanyDeskView` straight off the database. Here they stand alone, and that
 * is deliberate: the shape the UI needs is a contract in its own right, not a
 * projection of whatever the store happens to hold. When the OrgIntel read API
 * exists (ARCHITECTURE.md §4.4), it fills these in — the components do not move.
 *
 * No framework imports. Pure data and pure functions only.
 */

/* ============ people ============ */

/** The neutral avatar steps. Monochrome by law (design-language §3, D1): identity
 *  is carried by initials, position, and name — never by hue. */
export const PIG_STEPS = 5;

/** Deterministic avatar step for an id. Stable across renders and across surfaces,
 *  which matters: the same person must not change tile between Ops and People. */
export function pigFor(id: string | null | undefined): number {
	if (!id) return 0;
	let hash = 0;
	for (let i = 0; i < id.length; i += 1) hash = (hash * 31 + id.charCodeAt(i)) | 0;
	return Math.abs(hash) % PIG_STEPS;
}

export function initialsOf(name: string): string {
	return name
		.split(/\s+/)
		.filter(Boolean)
		.slice(0, 2)
		.map((word) => word[0]?.toUpperCase() ?? '')
		.join('');
}

/* ============ needs you — the inbox queue ============ */

export type NeedsYouKind = 'decision' | 'email-approval' | 'promotion-approval' | 'escalation';

/** Ids (and expected versions) needed to submit whatever resolves the item. */
export type NeedsYouRef =
	| { decisionId: string }
	| { approvalRequestId: string; version: number }
	| { escalationId: string; version: number };

/** The exact email draft awaiting a signature. */
export interface EmailDraftView {
	fromName: string;
	fromEmail: string;
	toName: string | null;
	toEmail: string;
	subject: string;
	textBody: string;
}

/**
 * Everything the operator needs to act in place: the card carries the ask, the
 * evidence, and the exact thing being signed, so judgement never requires a journey.
 */
export type NeedsYouContext =
	| { kind: 'decision'; proposalFacts: [string, string][] }
	| { kind: 'email-approval'; draft: EmailDraftView | null }
	| { kind: 'promotion-approval'; branch: string; runId: string }
	| {
			kind: 'escalation';
			escalationKind: string;
			what: string | null;
			why: string | null;
			expiresAt: Date | string | null;
			workTitle: string;
	  };

export interface NeedsYouItem {
	id: string;
	kind: NeedsYouKind;
	title: string;
	detail: string;
	/** Null sorts oldest — undated items have by definition been waiting at least as long. */
	createdAt: Date | string | null;
	ref: NeedsYouRef;
	context: NeedsYouContext;
}

/* ============ conversation ============ */

export type ThreadKind = 'executive' | 'agent' | 'goal';

export interface ThreadSummary {
	/** Stable UI key: 'executive', `agent:<agentId>`, or `goal:<goalId>`. */
	key: string;
	kind: ThreadKind;
	/** The persisted conversation, or null when the thread has not been opened yet. */
	conversationId: string | null;
	/** The DM partner (agent id) or the goal id; null for the executive thread. */
	subjectId: string | null;
	title: string;
	subtitle: string;
	pig: number;
	live: boolean;
	preview: string;
	lastAt: Date | string | null;
	messageCount: number;
}

export function threadKey(kind: ThreadKind, subjectId: string | null): string {
	return kind === 'executive' ? 'executive' : `${kind}:${subjectId}`;
}

export interface MessageAttachment {
	uploadId: string;
	name: string;
	mediaType: string;
	sizeBytes: number;
}

export interface ThreadMessage {
	id: string;
	from: 'you' | 'agent' | 'system';
	/** Honest author label — the employee's name for a reply, else a safe role. */
	author: string;
	text: string;
	createdAt: Date | string;
	replyToMessageId: string | null;
	/** The versioned record backing an agent reply, when one was produced. */
	assetId: string | null;
	runId: string | null;
	attachments: MessageAttachment[];
}

/* ============ work ============ */

export interface KanbanCard {
	id: string;
	title: string;
	state: string;
	stateReason: string | null;
	/** The assignee's id — what the People rollup groups by; names are not unique. */
	ownerId: string | null;
	ownerName: string | null;
	ownerPig: number;
	/** Sum of recorded run cost for this work item, in cents. */
	costCents: number;
	doneAt: Date | string | null;
}

export interface HqView {
	/** Recorded treasury cash on hand. */
	cashCents: number;
	currency: string;
	spendCents: number;
	budgetCents: number;
	runway: {
		months: number | null;
		/** Why no runway could be estimated, when months is null. */
		reason: string | null;
		assumption: string;
		disclaimer: string;
	};
	needsYou: number;
	activeRuns: number;
	columns: {
		queued: KanbanCard[];
		inProgress: KanbanCard[];
		/** Finished work awaiting the owner's read. A judgement, not a repair. */
		needsReview: KanbanCard[];
		/** Blocked or failed work. Not waiting on a decision — waiting on a fix. */
		stuck: KanbanCard[];
		doneRecent: KanbanCard[];
	};
	/** Completed work older than the recent window — counted, not hidden. */
	doneOlder: number;
	team: TeamMember[];
	drift: null;
}

export interface TeamMember {
	id: string;
	name: string;
	role: string;
	/** active | proposed | departed — the bench is a state of a person, not a room. */
	status: string;
	pig: number;
	live: boolean;
	working: boolean;
	openWork: number;
	spendCents: number;
	limitCents: number;
}

export interface PersonWork {
	inFlight: number;
	needsReview: number;
	stuck: number;
	landedThisWeek: number;
}

/**
 * Per-person work counts derived from the **same lane columns the Ops kanban renders**
 * rather than re-filtering the raw work list. That is the whole point: if People
 * re-derived "stuck" independently, the two surfaces could disagree about the same word.
 */
export function toPersonWork(view: HqView): Map<string, PersonWork> {
	const out = new Map<string, PersonWork>();
	const bump = (id: string | null, key: keyof PersonWork) => {
		if (!id) return;
		const row = out.get(id) ?? { inFlight: 0, needsReview: 0, stuck: 0, landedThisWeek: 0 };
		row[key] += 1;
		out.set(id, row);
	};
	for (const card of view.columns.queued) bump(card.ownerId, 'inFlight');
	for (const card of view.columns.inProgress) bump(card.ownerId, 'inFlight');
	for (const card of view.columns.needsReview) bump(card.ownerId, 'needsReview');
	for (const card of view.columns.stuck) bump(card.ownerId, 'stuck');
	for (const card of view.columns.doneRecent) bump(card.ownerId, 'landedThisWeek');
	return out;
}

/* ============ the org chart ============ */

export interface OrgNode {
	id: string;
	name: string;
	role: string;
	status: string;
	pig: number;
	/** Proposed people have synthetic ids and no profile to open yet. */
	draft: boolean;
	reports: OrgNode[];
}

/* ============ one employee, expanded ============ */

/**
 * Evidence, not a grade.
 *
 * Deliberately no composite score. A single number invites ranking people against each
 * other on a scale nobody agreed to; these are the countable facts, and the owner reads
 * them. `cost.metered` is load-bearing: a subscription-billed run records zero, which is
 * measured-as-nothing rather than free, and the surface has to say which.
 */
export interface TrackRecord {
	runs: { total: number; completed: number; failed: number; needsReview: number };
	outputs: { accepted: number; pendingReview: number; reversals: number };
	latency: { medianMs: number | null };
	cost: { recordedCents: number; metered: boolean };
	escalationsRaised: number;
}

/** The employee's operating policy — model, memory, and how much they may do alone. */
export interface OperatingPolicy {
	modelPolicy: string;
	memoryPolicy: string;
	approvalEnvelope: string;
	version: number;
}

/** One row of the task trail: every operation this employee took part in. */
export interface TrailEntry {
	sequence: number;
	operationKind: string;
	eventType: string;
	status: string;
	createdAt: Date | string;
}

/** An active emergency stop scoped to one employee. */
export interface EmergencyStop {
	id: string;
	reason: string;
	createdAt: Date | string;
}

export interface StaffProfile {
	id: string;
	name: string;
	role: string;
	/** `ai` or `human`. Operating policy only applies to the first. */
	kind: 'ai' | 'human';
	mandate: string;
	instructions: string | null;
	status: string;
	live: boolean;
	pig: number;
	reportsToName: string | null;
	/** Recorded run spend this month vs the standing monthly envelope. */
	spendCents: number;
	limitCents: number;
	currency: string;
	/** Capabilities granted autonomous. */
	can: string[];
	/** Capabilities granted approval_required — they need your word. */
	needsWord: string[];
	runningNow: Array<{
		id: string;
		workTitle: string | null;
		status: string;
		startedAt: Date | string | null;
	}>;
	planned: Array<{
		id: string;
		title: string;
		status: string;
		nextOccurrenceAt: Date | string | null;
		timezone: string;
	}>;
	recentRuns: Array<{
		id: string;
		workTitle: string | null;
		status: string;
		resultSummary: string | null;
		finishedAt: Date | string | null;
	}>;
	/** Evidence-based tallies — deliberately no composite score. */
	trackRecord: TrackRecord | null;
	artifacts: Array<{
		id: string;
		title: string;
		assetType: string;
		status: string;
		latestVersion: number | null;
	}>;
	goalsServed: Array<{ id: string; title: string; status: string }>;
	stopped: boolean;
	/** Null when nothing has been recorded — the defaults hold, and the surface says so. */
	operatingPolicy: OperatingPolicy | null;
	/** Present only while an emergency stop holds this employee's runs. */
	activeStop: EmergencyStop | null;
	trail: TrailEntry[];
	trailHasMore: boolean;
}

/* ============ the library ============ */

export interface LibraryRow {
	id: string;
	title: string;
	assetType: string;
	status: string;
	byName: string | null;
	byPig: number;
	versionCount: number;
	latestVersion: number | null;
	updatedAt: Date | string | null;
}

/**
 * One recorded version of a record.
 *
 * Nothing here is invented. A version recorded outside a run has a null `runId` and a
 * null `producedBy` — labelled honestly rather than attributed to a guessed author.
 */
export interface AssetVersion {
	id: string;
	version: number;
	/** The run that produced this version, or null when it was recorded outside a run. */
	runId: string | null;
	/** Who produced it: the run's employee, or null when there is no run. */
	producedBy: { actorId: string; kind: string; label: string } | null;
	contentDigest: string;
	mediaType: string;
	recordedAt: Date | string;
}

export interface RecordDetail {
	row: LibraryRow;
	relatedWorkId: string | null;
	relatedWorkTitle: string | null;
	/** Every recorded version, newest first. */
	versions: AssetVersion[];
	latestVersionId: string | null;
	latestDigest: string | null;
	openComments: number;
	content: unknown;
}

/* ============ the tape ============ */

export type TapeCategory = 'money' | 'work' | 'rules' | 'people' | 'other';

export interface TapeEntry {
	id: string;
	at: Date | string;
	summary: string;
	you: boolean;
	actorLabel: string | null;
	kind: 'event' | 'command';
	category: TapeCategory;
	policyOutcome: string | null;
	policyReason: string | null;
	eventCount: number | null;
	effectCount: number | null;
}

/* ============ what happened without you ============ */

export interface InboxUpdate {
	id: string;
	kind: 'run' | 'record';
	title: string;
	detail: string;
	at: Date | string;
	unread: boolean;
	/** Where the row points: an employee's thread or a library record. */
	target: { kind: 'thread'; thread: string } | { kind: 'record'; assetId: string };
}

export interface InboxUpdatesView {
	items: InboxUpdate[];
	/** The full count before the cap — shown so a truncated list is never presented as complete. */
	total: number;
	unreadCount: number;
}

/* ============ the mission and its authority ============ */

export interface MissionView {
	mission: string | null;
	/** Who set it and when, already phrased — the surface renders it verbatim. */
	set: string | null;
	goals: Array<{
		id: string;
		title: string;
		outcome: string;
		status: string;
		targetDate?: Date | string | null;
	}>;
	directives: Array<{ id: string; statement: string; status: string; createdAt: Date | string }>;
	/**
	 * Structured, NOT a rendered sentence.
	 *
	 * This used to be phrased as `<holder> may <capability> (<mode>)` and reversed with a
	 * regex at render time — which silently failed on any holder whose name contained a
	 * space, printing the whole sentence as the chip. Round-tripping structured fields
	 * through prose loses on every case the pattern did not anticipate. The fields are
	 * right here; the surface phrases them, never the other way around.
	 */
	standingRules: Array<{ id: string; holder: string; capability: string; mode: string }>;
}

/**
 * The company's own facts.
 *
 * The identity descriptors are RECORDED, not verified. Nothing here has been checked
 * against a registry, and the settings surface says so rather than implying otherwise.
 */
export interface CompanyProfile {
	id: string;
	name: string;
	currency: string;
	monthlyBudgetCents: number;
	legalName: string | null;
	tradingNames: string[];
	jurisdictions: string[];
	domains: string[];
	ownership: string | null;
	stage: string;
	/** Whether a scheduled heartbeat may drive the company between the owner's visits. */
	autonomyEnabled: boolean;
	/** Whether message text may cross to a connected model provider. */
	providerDisclosureEnabled: boolean;
}

/** A company- or employee-scoped stop that halts runs and outranks autonomy. */
export interface ScopedStop {
	id: string;
	scope: string;
	agentId: string | null;
	reason: string;
	createdAt: Date | string;
}

export interface BudgetLine {
	id: string;
	name: string;
	status: string;
	amountCents: number;
	currency: string;
}

/** Who serves what: one employee, the goals their work points at, and their mandate. */
export interface ServesRow {
	id: string;
	name: string;
	serves: string | undefined;
	line: string;
}

/* Standing authority, expanded, is typed by `$lib/model/authority-board` — it reads the
 * structured grant, not the rendered sentence in `MissionView.standingRules`. Round-tripping
 * structured fields through prose and back out with a regex loses on any label the regex
 * does not anticipate, and the structured fields are right there. */
export type { AuthorityGrant } from './authority-board';
import type { AuthorityGrant } from './authority-board';

/* ============ what is actually wired ============ */

/**
 * One row of the connections pane.
 *
 * "Probe, never guess" (CLAUDE.md) is the whole reason this is a view model and not a
 * derivation from config: `status` is the raw word the real system returned, `ok` and
 * `failed` are its classification, and `when` says plainly when the check happened —
 * including "never checked", which is a different claim from "working".
 */
export interface ConnectionRow {
	key: string;
	name: string;
	/** e.g. `runtime · acp` or `connector · mcp`. */
	kind: string;
	/** The status verbatim from the live check. Never paraphrased. */
	status: string;
	ok: boolean;
	failed: boolean;
	/** `checked 4 Aug`, or `never checked`. */
	when: string;
}

/* The marketplace is typed by `$lib/model/market-view` — parties, offerings, vendor
 * workers, and sourcing requests, in the shapes its pure composers read. */

/* ============ the whole desk ============ */

/** Everything a company surface renders, in one shape. */
export interface DeskView {
	company: CompanyProfile;
	/** The queue that needs the operator's word. The inbox is the landing surface. */
	needsYou: NeedsYouItem[];
	/** What happened without you. */
	updates: InboxUpdatesView;
	threads: ThreadSummary[];
	/** Keyed by `ThreadSummary.key`. */
	messages: Record<string, ThreadMessage[]>;
	hq: HqView;
	org: OrgNode[];
	/** Keyed by employee id. */
	staff: Record<string, StaffProfile>;
	library: LibraryRow[];
	/** Keyed by asset id. */
	records: Record<string, RecordDetail>;
	tape: TapeEntry[];
	mission: MissionView;
	budgets: BudgetLine[];
	serves: ServesRow[];
	authority: AuthorityGrant[];
	connections: ConnectionRow[];
	/** Active emergency stops, any scope. A stop outranks autonomy. */
	stops: ScopedStop[];
	/**
	 * Providers actually bound to this company's runtimes — read from what is bound, never
	 * from a list of what the product supports. Probed, never guessed.
	 */
	boundProviders: string[];
	/** Whether the executive has a live ACP runtime. Probed, never assumed. */
	executiveConnected: boolean;
	execName: string;
	membershipRole: string;
	providerDisclosureEnabled: boolean;
}
