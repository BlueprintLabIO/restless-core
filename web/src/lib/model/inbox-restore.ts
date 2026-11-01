/* Undo of a skipped Inbox card (Sprint 109 C1). Skip is saved before the card leaves, so its Undo
 * is the only step that runs ahead of the source: the card goes back where it left at once, and
 * stays there while the restore travels. A read that lands before the source has restored it must
 * not take it away again. Nothing here decides anything; it only keeps a queue's order. */

type Row = { id: string };
export type QueueView<T extends Row> = { items: T[]; pendingDecisionItems?: string[] };
/** Where a card sat: its neighbour above (null when it led) and its index, for when that
 * neighbour has gone too. */
export type Place<T extends Row> = { item: T; after: string | null; index: number };

export function placeOf<T extends Row>(items: readonly T[], item: T): Place<T> {
	const index = items.findIndex((row) => row.id === item.id);
	if (index < 0) return { item, after: null, index: 0 };
	return { item, after: index ? items[index - 1].id : null, index };
}

/** The queue with the card back in its place. A queue that already holds it is left alone: the
 * source's own copy is newer than the one kept here. */
export function reinstate<T extends Row>(items: readonly T[], place: Place<T>): T[] {
	if (items.some((row) => row.id === place.item.id)) return [...items];
	const anchor = place.after === null ? -1 : items.findIndex((row) => row.id === place.after);
	const at =
		place.after === null ? 0 : anchor >= 0 ? anchor + 1 : Math.min(place.index, items.length);
	return [...items.slice(0, at), place.item, ...items.slice(at)];
}

/** Cards on their way back, per company, laid over every read until their source agrees. */
export class RestoringCards {
	#kept = new Map<string, Map<string, Place<Row>>>();

	/** Keep the card in every read from now; the returned release stops that. */
	keep<T extends Row>(company: string, place: Place<T>): () => void {
		const kept = this.#kept.get(company) ?? new Map<string, Place<Row>>();
		this.#kept.set(company, kept);
		kept.set(place.item.id, place);
		return () => {
			if (kept.get(place.item.id) !== place) return;
			kept.delete(place.item.id);
			if (!kept.size) this.#kept.delete(company);
		};
	}

	/** The view as it will be once the restores land. The same object when nothing is kept. */
	over<V extends QueueView<Row>>(company: string, view: V): V {
		const kept = this.#kept.get(company);
		if (!kept?.size) return view;
		let items = view.items;
		for (const place of kept.values()) items = reinstate(items, place);
		return {
			...view,
			items,
			...(view.pendingDecisionItems
				? { pendingDecisionItems: view.pendingDecisionItems.filter((id) => !kept.has(id)) }
				: {})
		};
	}
}

export const restoringCards = new RestoringCards();
