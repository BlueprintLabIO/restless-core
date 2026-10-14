/* People as one directory (S88): each agent's status in one word, and the Work beside their chat.
 * Pure, so the page and its tests read the same rules. Nothing here is stored. */

/** One word that can be read at a glance, coloured by meaning in the directory. */
export type StatusWord = 'Working' | 'Waiting' | 'Blocked' | 'Idle';

/** The directory row's tone, as People already colours it, said as a word. */
export function statusWord(tone: string): StatusWord {
	if (tone === 'block') return 'Blocked';
	if (tone === 'wait') return 'Waiting';
	if (tone === 'work') return 'Working';
	return 'Idle';
}

export type DirectoryFilter = 'everyone' | 'working' | 'needs' | 'archived';
export const DIRECTORY_FILTERS: { key: DirectoryFilter; label: string }[] = [
	{ key: 'everyone', label: 'Everyone' },
	{ key: 'working', label: 'Working' },
	{ key: 'needs', label: 'Needs you' },
	{ key: 'archived', label: 'Archived' }
];
export function directoryFilter(value: string | null): DirectoryFilter {
	return DIRECTORY_FILTERS.some((filter) => filter.key === value)
		? (value as DirectoryFilter)
		: 'everyone';
}

/** The fields of a piece of Work the panel reads. */
export interface PanelWorkItem {
	id: string;
	owner_id: string;
	title: string;
	status: 'proposed' | 'active' | 'blocked' | 'completed' | 'abandoned';
	updated_at: string;
}

export interface WorkPanel<W extends PanelWorkItem> {
	/** The Work in motion, or, when nothing is, the last delivered. */
	now: W | null;
	/** Whether `now` is finished Work shown so the panel is never empty. */
	nowDelivered: boolean;
	/** Proposed Work, and other Work in motion after `now`. */
	next: W[];
	/** Finished in the last seven days, newest first. */
	done: W[];
}

const WEEK_MS = 7 * 24 * 60 * 60 * 1000;
const newest = (a: PanelWorkItem, b: PanelWorkItem) =>
	Date.parse(b.updated_at) - Date.parse(a.updated_at);

/** The Work panel for these people: one agent, or a lead's whole team. A `highlight` (a Work opened
 * by address) leads Now when it belongs to them. */
export function workPanel<W extends PanelWorkItem>(
	work: readonly W[],
	owners: ReadonlySet<string>,
	options: { highlight?: string; now?: number } = {}
): WorkPanel<W> {
	const at = options.now ?? Date.now();
	const theirs = work.filter((item) => owners.has(item.owner_id)).toSorted(newest);
	/* Active before blocked: what is moving is what the owner checks in on. */
	const moving = [
		...theirs.filter((item) => item.status === 'active'),
		...theirs.filter((item) => item.status === 'blocked')
	];
	const highlighted = options.highlight
		? (theirs.find((item) => item.id === options.highlight) ?? null)
		: null;
	const completed = theirs.filter((item) => item.status === 'completed');
	const now = highlighted ?? moving[0] ?? completed[0] ?? null;
	const nowDelivered = !!now && now.status === 'completed' && !highlighted && !moving.length;
	const next = [
		...moving.filter((item) => item !== now),
		...theirs.filter((item) => item.status === 'proposed' && item !== now)
	];
	const done = completed.filter(
		(item) => item !== now && Date.parse(item.updated_at) >= at - WEEK_MS
	);
	return { now, nowDelivered, next, done };
}
