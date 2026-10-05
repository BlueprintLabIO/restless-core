/** Authority and transport stay in the adapter; these are observed display values. */
export type PortfolioAction =
	| { href: string; label?: string }
	| { formAction: string; fields: Record<string, string>; label?: string };

/**
 * A company's own content-free card: the counts its plane signs for Fleet (company projection v2)
 * and hands unsigned to the local root page. Counts and times only, never titles or amounts, so
 * every host can show exactly the same thing.
 */
export interface PortfolioCard {
	decisionsWaiting: number;
	peopleWorking: number;
	outcomesLastDay: number;
	execReady: boolean;
	/** ISO time work last moved. */
	lastActivityAt?: string | null;
	/** ISO time the plane computed the card, when it travelled (Cloud). */
	asOf?: string | null;
	/** Older than the host's staleness bound: shown with its age, never as current. */
	stale?: boolean;
}

export interface CompanyPortfolioEntry {
	id: string;
	name: string;
	/** Run state in the owner's words: Running, Asleep, Setting up, Can't start, Archived. */
	status: string;
	tone?: 'presence' | 'waiting' | 'unavailable';
	/** Absent when the company has not shared its counts (or they could not be read). */
	card?: PortfolioCard | null;
	/** A setup problem the host observed itself, as one sentence. */
	issue?: string;
	/** The fix's button label, e.g. "Give access". Defaults to "Fix setup". */
	issueAction?: string;
	/** Where the card goes: the company, or its fix when `issue` is set. */
	entry?: PortfolioAction | null;
	/** Kept but not running, such as an archived company: shown quietly, last. */
	dormant?: boolean;
}

/** Needs you first, then working, then quiet, then dormant; by name within each. */
export function portfolioRank(company: CompanyPortfolioEntry): number {
	if (company.dormant) return 3;
	if (company.issue || (company.card?.decisionsWaiting ?? 0) > 0) return 0;
	if ((company.card?.peopleWorking ?? 0) > 0) return 1;
	return 2;
}

export function orderPortfolio(companies: CompanyPortfolioEntry[]): CompanyPortfolioEntry[] {
	return [...companies].sort(
		(a, b) => portfolioRank(a) - portfolioRank(b) || a.name.localeCompare(b.name)
	);
}

function plural(count: number, one: string, many = one + 's'): string {
	return `${count} ${count === 1 ? one : many}`;
}

/** "3 hours ago", "just now": coarse on purpose; a card is a glance, not a log. */
export function ago(iso: string | null | undefined, now = Date.now()): string {
	if (!iso) return '';
	const minutes = Math.max(0, Math.round((now - Date.parse(iso)) / 60_000));
	if (Number.isNaN(minutes)) return '';
	if (minutes < 2) return 'just now';
	if (minutes < 60) return `${minutes} minutes ago`;
	const hours = Math.round(minutes / 60);
	if (hours < 24) return plural(hours, 'hour') + ' ago';
	return plural(Math.round(hours / 24), 'day') + ' ago';
}

/** The one action a card leads with, or nothing when nothing needs the owner. */
export function portfolioPrimary(company: CompanyPortfolioEntry): string | null {
	if (company.dormant) return null;
	if (company.issue) return company.issueAction || 'Fix setup';
	const waiting = company.card?.decisionsWaiting ?? 0;
	return waiting > 0 ? plural(waiting, 'decision') + ' waiting' : null;
}

/** The quiet line under the name: what the company is doing, from its own counts. */
export function portfolioSignal(company: CompanyPortfolioEntry, now = Date.now()): string {
	if (company.dormant) return 'Kept, not running';
	if (company.issue) return company.issue;
	const card = company.card;
	if (!card) return '';
	const parts: string[] = [];
	if (card.peopleWorking > 0)
		parts.push(plural(card.peopleWorking, 'person', 'people') + ' working');
	else {
		const since = ago(card.lastActivityAt, now);
		parts.push(since ? `Quiet · last activity ${since}` : 'Quiet');
	}
	if (card.outcomesLastDay > 0)
		parts.push(plural(card.outcomesLastDay, 'outcome') + ' in the last day');
	if (card.stale && card.asOf) parts.push(`as of ${ago(card.asOf, now)}`);
	return parts.join(' · ');
}

/**
 * A content-free identity mark: a symmetric 5×7 pixel figure seeded from the company id, in the
 * same matrix the product's glyphs are drawn in. Stable for a company, distinct across companies,
 * and derived from nothing a company says.
 */
export function companyMarkRows(id: string): string[] {
	let hash = 2166136261;
	for (let index = 0; index < id.length; index++) {
		hash ^= id.charCodeAt(index);
		hash = Math.imul(hash, 16777619) >>> 0;
	}
	const rows: string[] = [];
	for (let y = 0; y < 7; y++) {
		const left = [0, 1, 2].map((x) => ((hash >>> ((y * 3 + x) % 31)) & 1 ? '1' : '0'));
		rows.push([left[0], left[1], left[2], left[1], left[0]].join(''));
	}
	// Never an empty or near-empty mark: light the centre column where a row went dark.
	return rows.map((row, y) => (row === '00000' && y % 2 === 1 ? '00100' : row));
}
