/* One search for the whole app (S104 O3, owner 23 Oct). Every list, menu, picker and
 * find-in-conversation calls these, never the library underneath, so they all match the same way:
 *
 * - Lists (`rank`, `keep`, `matches`): uFuzzy. Case and accents fold, every typed word matches the
 *   start of a word in any order, and a longer word may carry one typo. "sonnet 5" finds
 *   "Claude Sonnet 5.5"; "guild relief" finds "Relief Guild hook".
 * - Find in a page (`occurrences`, `findRanges`): what was typed, case and accents folded, wherever it
 *   appears, counted one by one, as a browser's own find does. A typo-tolerant match that cannot
 *   be pointed at is no use when the job is to jump to it.
 *
 * uFuzzy was chosen over Fuse.js (O3): Fuse scores by distance from the start of each field, so a
 * match deep in a long message ranks out unless that is switched off, and its loose fuzziness
 * matches scattered letters. */
import uFuzzy from '@leeoniya/ufuzzy';

const engine = new uFuzzy({
	// A word may carry one typo (substitution, transposition, deletion or insertion)...
	intraMode: 1,
	// ...and every typed word starts at a word boundary in the text ("gu" finds "Guild", not "Aguila").
	interLft: 2
});
/* How many typed words may be tried in another order. uFuzzy permutes them, so this stays small. */
const OUT_OF_ORDER = 3;

type Field = string | null | undefined;

function haystackOf<T>(
	items: readonly T[],
	fields: (item: T) => Field | readonly Field[]
): string[] {
	return uFuzzy.latinize(
		items.map((item) => {
			const value = fields(item);
			return typeof value === 'string' || value == null
				? (value ?? '')
				: value.filter(Boolean).join('   ');
		})
	);
}

function search<T>(
	items: readonly T[],
	query: string,
	fields: (item: T) => Field | readonly Field[]
): { idxs: number[]; ranked: number[] } | null {
	const needle = uFuzzy.latinize(query.trim());
	if (!needle) return null;
	const [idxs, info, order] = engine.search(haystackOf(items, fields), needle, OUT_OF_ORDER);
	if (!idxs) return { idxs: [], ranked: [] };
	const ranked = info && order ? order.map((at) => info.idx[at]) : [...idxs];
	return { idxs: [...idxs], ranked };
}

/** The items that match `query`, best first. An empty query keeps every item in its own order. */
export function rank<T>(
	items: readonly T[],
	query: string,
	fields: (item: T) => Field | readonly Field[]
): T[] {
	const found = search(items, query, fields);
	return found ? found.ranked.map((at) => items[at]) : [...items];
}

/** The items that match `query`, in their own order: for lists whose order means something. */
export function keep<T>(
	items: readonly T[],
	query: string,
	fields: (item: T) => Field | readonly Field[]
): T[] {
	const found = search(items, query, fields);
	if (!found) return [...items];
	const hit = new Set(found.idxs);
	return items.filter((_, at) => hit.has(at));
}

/** Whether a row's visible words match `query`. An empty query matches everything. */
export function matches(query: string, fields: readonly Field[]): boolean {
	return keep([fields], query, (row) => row).length > 0;
}

/* Folding one character at a time keeps a map back to the original text, so a folded match can be
 * pointed at in what is on screen ("Café" is four characters either way only by luck). */
function folded(text: string): { text: string; start: number[]; end: number[] } {
	let out = '';
	const start: number[] = [];
	const end: number[] = [];
	let at = 0;
	for (const char of text) {
		// A run of spaces or line breaks reads as one space, as it does on screen.
		if (/\s/.test(char) && out.endsWith(' ')) {
			end[end.length - 1] = at + char.length;
			at += char.length;
			continue;
		}
		const plain = /\s/.test(char)
			? ' '
			: char.normalize('NFD').replace(/\p{M}/gu, '').toLocaleLowerCase();
		for (const piece of plain) {
			out += piece;
			start.push(at);
			end.push(at + char.length);
		}
		at += char.length;
	}
	return { text: out, start, end };
}

function needleOf(query: string): string {
	return folded(query.trim().replace(/\s+/g, ' ')).text;
}

/** Every place `query` appears in `text`, as [start, end) offsets into `text`. */
export function occurrences(text: string, query: string): [number, number][] {
	const needle = needleOf(query);
	if (!needle) return [];
	const hay = folded(text);
	const found: [number, number][] = [];
	for (
		let at = hay.text.indexOf(needle);
		at >= 0;
		at = hay.text.indexOf(needle, at + needle.length)
	)
		found.push([hay.start[at], hay.end[at + needle.length - 1]]);
	return found;
}

/** Markdown as it reads on screen, for searching: no emphasis marks, link targets or fences. */
export function markdownText(markdown: string): string {
	return markdown
		.replace(/```[^\n]*\n?/g, '')
		.replace(/!?\[([^\]]*)\]\([^)]*\)/g, '$1')
		.replace(/(\*\*|__|~~|`)/g, '')
		.replace(/(^|[\s(])[*_]([^*_\n]+)[*_](?=[\s).,:;!?]|$)/gm, '$1$2')
		.replace(/^\s{0,3}(#{1,6}\s+|>\s?|[-*+]\s+|\d+[.)]\s+)/gm, '');
}

const SKIP = 'script, style, textarea, input, select, [aria-hidden="true"], [hidden]';

/** DOM ranges for every place `query` appears in the text shown under `roots`, in document
 * order. A match may cross elements (a bold word, a link), as it does on screen. */
export function findRanges(roots: readonly (Element | null | undefined)[], query: string): Range[] {
	const needle = needleOf(query);
	const ranges: Range[] = [];
	if (!needle) return ranges;
	for (const root of roots) {
		if (!root) continue;
		const nodes: Text[] = [];
		const offsets: number[] = [];
		let text = '';
		const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
			acceptNode: (node) =>
				node.parentElement?.closest(SKIP) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT
		});
		for (let node = walker.nextNode(); node; node = walker.nextNode()) {
			nodes.push(node as Text);
			offsets.push(text.length);
			text += (node as Text).data;
		}
		const locate = (offset: number, isEnd: boolean): [Text, number] => {
			let index = nodes.length - 1;
			while (index > 0 && (isEnd ? offsets[index] >= offset : offsets[index] > offset)) index--;
			return [nodes[index], offset - offsets[index]];
		};
		for (const [start, end] of occurrences(text, query)) {
			const range = document.createRange();
			range.setStart(...locate(start, false));
			range.setEnd(...locate(end, true));
			ranges.push(range);
		}
	}
	return ranges;
}

/* Matches are painted with the CSS Custom Highlight API: nothing in the page is rewrapped, so a
 * search never disturbs a message being streamed or edited. Styles: ::highlight(search-hit) and
 * ::highlight(search-current) in ui/style/primitives.css. */
const highlights = () =>
	(globalThis as { CSS?: { highlights?: Map<string, unknown> } }).CSS?.highlights;

/** Paint `hits`, with `current` marked as the one in hand. */
export function paintMatches(hits: readonly Range[], current: Range | null = null): void {
	const registry = highlights();
	if (!registry || typeof Highlight === 'undefined') return;
	registry.set('search-hit', new Highlight(...hits));
	if (current) registry.set('search-current', new Highlight(current));
	else registry.delete('search-current');
}

export function clearMatches(): void {
	const registry = highlights();
	registry?.delete('search-hit');
	registry?.delete('search-current');
}
