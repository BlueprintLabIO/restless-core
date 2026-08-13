/* The full work board — the Ops pane expanded.
 *
 * The Ops pane truncates: `HqView.doneOlder` is literally a count of rows the lanes cannot
 * show, which is what earns "The work" its expand affordance. This module turns the same
 * `HqView` into an untruncated, filterable board so a card reads identically in both places
 * and neither surface can invent a lane the other does not have.
 *
 * Pure and deterministic, like `desk-map.ts` — the route composes, it does not compute. */

import type { HqView, KanbanCard } from '$lib/model/view';

export type LaneKey = 'queued' | 'inProgress' | 'needsReview' | 'stuck' | 'doneRecent';

export type WorkLane = {
	key: LaneKey;
	label: string;
	cards: KanbanCard[];
	/** Completed work outside the recent window. Counted rather than hidden — only on `doneRecent`. */
	olderCount: number;
};

export const LANE_LABELS: Record<LaneKey, string> = {
	queued: 'Queued',
	inProgress: 'In progress',
	needsReview: 'Needs review',
	stuck: 'Stuck',
	doneRecent: 'Done this week'
};

export const LANE_ORDER: readonly LaneKey[] = [
	'queued',
	'inProgress',
	'needsReview',
	'stuck',
	'doneRecent'
];

export type WorkFilters = {
	/** An assignee's agent id. Names are not unique, so the filter is by id. */
	ownerId?: string | null;
	/** Restrict to one lane; absent means every lane. */
	lane?: LaneKey | null;
	/** Case-insensitive substring of the work item's title. */
	query?: string | null;
};

/**
 * Every lane, untruncated, after filters.
 *
 * A filtered lane still appears when it empties. A board whose columns come and go as you type
 * makes you re-find the one you were reading, and "nothing queued matches" is itself an answer.
 */
export function composeWorkBoard(hq: HqView, filters: WorkFilters = {}): WorkLane[] {
	const owner = filters.ownerId ?? null;
	const query = (filters.query ?? '').trim().toLowerCase();
	const only = filters.lane ?? null;

	const keep = (card: KanbanCard) => {
		if (owner && card.ownerId !== owner) return false;
		if (query && !card.title.toLowerCase().includes(query)) return false;
		return true;
	};

	return LANE_ORDER.filter((key) => only === null || key === only).map((key) => ({
		key,
		label: LANE_LABELS[key],
		cards: hq.columns[key].filter(keep),
		/* The older-done count belongs to the unfiltered record: it is a tally the view never
		 * held cards for, so no filter can honestly narrow it. Suppressed when a filter is on
		 * rather than shown against a filtered lane it does not describe. */
		olderCount: key === 'doneRecent' && !owner && !query ? hq.doneOlder : 0
	}));
}

/** Assignees who actually hold work, for the owner filter. Sorted by name for a stable menu. */
export function boardOwners(hq: HqView): Array<{ id: string; name: string }> {
	const owners = new Map<string, string>();
	for (const key of LANE_ORDER) {
		for (const card of hq.columns[key]) {
			if (card.ownerId && card.ownerName) owners.set(card.ownerId, card.ownerName);
		}
	}
	return [...owners]
		.map(([id, name]) => ({ id, name }))
		.sort((a, b) => a.name.localeCompare(b.name));
}

/** How many cards the board is showing, and how many exist — so truncation is never silent. */
export function boardCount(lanes: readonly WorkLane[]): number {
	return lanes.reduce((total, lane) => total + lane.cards.length, 0);
}
