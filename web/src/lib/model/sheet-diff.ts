/* What changed between two saved versions of a sheet, cell by cell: the content as typed (a
 * formula, not its result), for every tab either version has. Tabs are matched by the grid's id,
 * so a renamed tab is the same tab. Dependency-free, so node's test runner imports it directly. */

export interface SheetCellChange {
	/** The tab's name now, or as it was if the tab is gone. */
	tab: string;
	cell: string;
	before: string;
	after: string;
}

export interface SheetTabChange {
	tab: string;
	kind: 'added' | 'removed' | 'renamed';
	/** For a renamed tab, its old name. */
	from?: string;
}

export interface SheetChanges {
	tabs: SheetTabChange[];
	cells: SheetCellChange[];
	/** Changed cells beyond the ones listed. */
	more: number;
}

type Workbook = { sheets?: { id?: string; name?: string; cells?: Record<string, unknown> }[] };

function content(value: unknown): string {
	if (value === null || value === undefined) return '';
	if (typeof value === 'string') return value;
	if (typeof value === 'number' || typeof value === 'boolean') return String(value);
	if (typeof value === 'object' && 'content' in value)
		return content((value as { content: unknown }).content);
	return '';
}

/* A1 order: row first, then column, so a list reads down the sheet. */
function position(cell: string): [number, number] {
	const match = /^([A-Z]+)(\d+)$/.exec(cell);
	if (!match) return [Number.MAX_SAFE_INTEGER, 0];
	let col = 0;
	for (const letter of match[1]) col = col * 26 + (letter.charCodeAt(0) - 64);
	return [Number(match[2]), col];
}

export function sheetChanges(
	before: Workbook | null | undefined,
	after: Workbook | null | undefined,
	limit = 200
): SheetChanges {
	const old = before?.sheets ?? [];
	const now = after?.sheets ?? [];
	const tabs: SheetTabChange[] = [];
	const cells: SheetCellChange[] = [];
	const ids = [...new Set([...now, ...old].map((sheet, index) => sheet.id ?? `#${index}`))];
	for (const id of ids) {
		const was = old.find((sheet) => sheet.id === id);
		const is = now.find((sheet) => sheet.id === id);
		const tab = is?.name ?? was?.name ?? id;
		if (!was) tabs.push({ tab, kind: 'added' });
		else if (!is) tabs.push({ tab, kind: 'removed' });
		else if (was.name && is.name && was.name !== is.name)
			tabs.push({ tab, kind: 'renamed', from: was.name });
		const names = new Set([...Object.keys(was?.cells ?? {}), ...Object.keys(is?.cells ?? {})]);
		const changed = [...names]
			.map((cell) => ({
				tab,
				cell,
				before: content(was?.cells?.[cell]),
				after: content(is?.cells?.[cell])
			}))
			.filter((change) => change.before !== change.after)
			.toSorted((left, right) => {
				const [lr, lc] = position(left.cell);
				const [rr, rc] = position(right.cell);
				return lr - rr || lc - rc;
			});
		cells.push(...changed);
	}
	return { tabs, cells: cells.slice(0, limit), more: Math.max(0, cells.length - limit) };
}
