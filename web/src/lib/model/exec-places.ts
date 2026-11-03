/* Kept free of imports so the Inbox order stays testable with plain node. */

/** Exec's place for items it moved: 1 is the top, 0 the bottom. Everything else keeps the
 * Inbox's own order around them. */
export function withExecPlaces<T extends { execPosition?: number | null }>(ordered: T[]): T[] {
	const placed = ordered
		.filter((item) => (item.execPosition ?? null) !== null && item.execPosition! > 0)
		.sort((a, b) => a.execPosition! - b.execPosition!);
	const bottom = ordered.filter((item) => item.execPosition === 0);
	const rest = ordered.filter(
		(item) => item.execPosition === undefined || item.execPosition === null
	);
	const out = [...rest];
	for (const item of placed) out.splice(Math.min(item.execPosition! - 1, out.length), 0, item);
	return [...out, ...bottom];
}
