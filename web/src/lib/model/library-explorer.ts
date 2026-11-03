/* The Library's All files explorer and its sidebar collections (owner redesign, 3 November).
 *
 * The explorer is a view of where files already are: folders are the directories files sit in
 * under the company home, shown with human names, and nothing is ever moved, renamed or deleted
 * to make it. Collections are saved queries over the same files: a file can be in several, and a
 * collection leaves the sidebar when its goal closes. Pure functions, so the page stays a view. */

export const OUTPUTS = '/company/outputs';

/** The folder a file sits in, keyed by its directory in the company home. The outputs folder's
 * own top level and anything without a path (a document, a sheet, a kept file, a live site) are
 * the top of All files, keyed ''. */
export function folderOf(uri: string): string {
	const path = uri.split(/[?#]/)[0];
	if (!path.startsWith('/company/')) return '';
	const dir = path.slice(0, path.lastIndexOf('/'));
	return dir === OUTPUTS || dir === '/company' ? '' : dir;
}

/** The folders from the top down to this one, this one last. Outputs' folders sit at the top of
 * All files; any other place in the company home (Downloads, a repository) is a folder there too. */
export function folderChain(folder: string): string[] {
	if (!folder) return [];
	const base = folder.startsWith(`${OUTPUTS}/`) ? OUTPUTS : '/company';
	const rest = folder
		.slice(base.length + 1)
		.split('/')
		.filter(Boolean);
	return rest.map((_, index) => `${base}/${rest.slice(0, index + 1).join('/')}`);
}

/** The folder above this one; '' is the top. */
export function parentFolder(folder: string): string {
	return folderChain(folder).at(-2) ?? '';
}

/** Whether a file in `dir` sits in `folder` or anywhere beneath it ('' holds everything). */
export function within(dir: string, folder: string): boolean {
	return !folder || dir === folder || dir.startsWith(`${folder}/`);
}

const EXTENSION =
	/\.(html?|md|markdown|txt|pdf|png|jpe?g|gif|webp|avif|svg|ico|csv|tsv|json|log|docx?|xlsx?|pptx?|mp4|m4v|webm|ogv|mp3|m4a|wav|oga|ogg|flac|zip)$/i;
const SMALL_WORD = /^(a|an|and|as|at|by|for|in|of|on|or|the|to|vs|via|with)$/i;

function decoded(name: string): string {
	try {
		return decodeURIComponent(name);
	} catch {
		return name;
	}
}

/** A name a person would say for a file or folder that only has a slug: `aleon-electrical` is
 * "Aleon Electrical", `mobile-390.png` is "Mobile 390", `v3` is "Version 3". A name that is
 * already words (it has a space) keeps its words and loses only a file extension. Folders are in
 * title case, as a place is named; files in sentence case, as a title is written. */
export function humanName(name: string, kind: 'file' | 'folder' = 'file'): string {
	const raw = decoded(name).trim();
	const base = (kind === 'file' ? raw.replace(EXTENSION, '') : raw).replace(/^[._]+/, '') || raw;
	if (/\s/.test(base)) return base;
	const version = base.match(/^v(?:ersion)?[-_]?(\d+)$/i);
	if (version) return `Version ${version[1]}`;
	const words = base.split(/[-_]+/).filter(Boolean);
	if (!words.length) return raw;
	/* An all-capitals or mixed-case word (README, iPhone, A1000) is left as written. */
	const keep = (word: string) => /[A-Z]/.test(word.slice(1)) || /^[A-Z0-9]+$/.test(word);
	return words
		.map((word, index) => {
			if (keep(word)) return word;
			const lower = word.toLowerCase();
			if (index === 0) return lower.charAt(0).toUpperCase() + lower.slice(1);
			if (kind === 'folder' && !SMALL_WORD.test(lower))
				return lower.charAt(0).toUpperCase() + lower.slice(1);
			return lower;
		})
		.join(' ');
}

/** What a folder is called: its own name made human ("Aleon Electrical", "Version 4"), unless
 * that is still a machine's name (long, or numbered at its end, as an agent's run folder
 * `bakehouse-concept-website-version-3-quiet-luxury-27` is), then the title of the site it holds
 * when its page was given one. */
export function folderTitle(folder: string, titles: ReadonlyMap<string, string>): string {
	if (folder === '/company/downloads') return 'Downloads';
	const name = folder.split('/').filter(Boolean).at(-1) ?? folder;
	const human = humanName(name, 'folder');
	const given = titles.get(folder);
	const machine = human.length > 32 || (/\s\d+$/.test(human) && !/^Version \d+$/.test(human));
	return given && machine ? given : human;
}

/** The breadcrumb from All files down to this folder. */
export function crumbs(
	folder: string,
	titles: ReadonlyMap<string, string>
): { folder: string; title: string }[] {
	return [
		{ folder: '', title: 'All files' },
		...folderChain(folder).map((key) => ({ folder: key, title: folderTitle(key, titles) }))
	];
}

export interface ExplorerItem {
	/** The directory it sits in ('' for the top). */
	dir: string;
	updatedAt: string;
}

export interface ExplorerFolder {
	folder: string;
	title: string;
	/** The newest change to anything inside it. */
	updatedAt: string;
}

/** One folder's contents: the folders directly inside it that hold anything shown, newest change
 * first, and the items sitting in it. Only the given items are considered, so a folder holding
 * nothing but hidden working notes does not appear. */
export function explore<T extends ExplorerItem>(
	items: readonly T[],
	folder: string,
	titles: ReadonlyMap<string, string>
): { folders: ExplorerFolder[]; items: T[] } {
	const depth = folderChain(folder).length;
	const folders = new Map<string, number>();
	const here: T[] = [];
	for (const item of items) {
		if (item.dir === folder) {
			here.push(item);
			continue;
		}
		if (!within(item.dir, folder)) continue;
		const child = folderChain(item.dir)[depth];
		if (!child) continue;
		const at = Date.parse(item.updatedAt);
		const newest = folders.get(child);
		folders.set(
			child,
			newest === undefined
				? Number.isNaN(at)
					? 0
					: at
				: Math.max(newest, Number.isNaN(at) ? 0 : at)
		);
	}
	return {
		folders: [...folders]
			.map(([key, at]) => ({
				folder: key,
				title: folderTitle(key, titles),
				updatedAt: at ? new Date(at).toISOString() : ''
			}))
			.sort((a, b) => Date.parse(b.updatedAt || '0') - Date.parse(a.updatedAt || '0')),
		items: here
	};
}

/** Every folder that holds at least one of the items, with its title, for search. */
export function allFolders(
	items: readonly ExplorerItem[],
	titles: ReadonlyMap<string, string>
): ExplorerFolder[] {
	const newest = new Map<string, number>();
	for (const item of items)
		for (const key of folderChain(item.dir)) {
			const at = Date.parse(item.updatedAt);
			newest.set(key, Math.max(newest.get(key) ?? 0, Number.isNaN(at) ? 0 : at));
		}
	return [...newest].map(([folder, at]) => ({
		folder,
		title: folderTitle(folder, titles),
		updatedAt: at ? new Date(at).toISOString() : ''
	}));
}

/* Pins name an item by `kind:id`, and the server takes only letters, digits, `.`, `_`, `:` and
 * `-` in the id: a folder's path and a saved filter go as base64url. */
function toBase64Url(text: string): string {
	const bytes = new TextEncoder().encode(text);
	let binary = '';
	for (const byte of bytes) binary += String.fromCharCode(byte);
	return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}
function fromBase64Url(text: string): string | null {
	try {
		const binary = atob(text.replace(/-/g, '+').replace(/_/g, '/'));
		return new TextDecoder().decode(Uint8Array.from(binary, (char) => char.charCodeAt(0)));
	} catch {
		return null;
	}
}
/** The longest pin id the server keeps. */
const PIN_ID_MAX = 200;

/** A pin for a folder, or null when its path is too long to pin. */
export function folderPin(folder: string): string | null {
	const id = toBase64Url(folder);
	return id && id.length <= PIN_ID_MAX ? `folder:${id}` : null;
}

/** A pin for a saved filter: its name and the Library address it shows. */
export function filterPin(name: string, search: string): string | null {
	const id = toBase64Url(`${name}\n${search.replace(/^\?/, '')}`);
	return id.length <= PIN_ID_MAX ? `filter:${id}` : null;
}

export type DecodedPin =
	| { kind: 'folder'; folder: string }
	| { kind: 'filter'; name: string; search: string }
	| { kind: 'item'; key: string };

export function decodePin(key: string): DecodedPin | null {
	const [kind, id] = [key.slice(0, key.indexOf(':')), key.slice(key.indexOf(':') + 1)];
	if (kind === 'folder') {
		const folder = fromBase64Url(id);
		return folder?.startsWith('/company/') ? { kind: 'folder', folder } : null;
	}
	if (kind === 'filter') {
		const text = fromBase64Url(id);
		if (!text?.includes('\n')) return null;
		const at = text.indexOf('\n');
		return { kind: 'filter', name: text.slice(0, at), search: text.slice(at + 1) };
	}
	return { kind: 'item', key };
}

/* Live now: the sidebar's dynamic collections, mirroring open goals, the clients Work is busy
 * with, and collections Exec or the owner named. */

export interface LiveItem {
	/** The row's pin key (`doc:…`, `site:…`, `path:…`). */
	key: string;
	kind: string;
	dir: string;
	updatedAt: string;
	workId: string | null;
	/** Waiting on the owner. */
	review: boolean;
	/** A document under review. */
	inReview: boolean;
	/** A site served live from the company computer. */
	live: boolean;
	/** A named collection it was filed in. */
	collection?: string;
}

export interface LiveCollection {
	/** What the address names: `goal:<id>`, `folder:<path>` or `collection:<name>`. */
	key: string;
	title: string;
	/** Why it is here, in a few words. */
	reason: string;
	tone: 'wait' | 'work' | '';
	/** The newest change in it, for ordering. */
	activity: number;
}

/* Folder names that say what kind of thing is inside, not whose it is. */
const KIND_FOLDER =
	/^(sites?|pages?|images?|img|photos?|screenshots?|docs?|documents?|decks?|slides|reports?|reviews?|notes?|drafts?|exports?|assets?|static|public|dist|build|out|outputs?|files?|downloads?|media|videos?|pdfs?|data)$/i;

/** The folders that are about one client or project: the deepest folders whose files came from
 * two or more pieces of Work (a client's versions and its reviews), unless the folder's name is
 * only a kind of file. */
export function clientFolders(items: readonly LiveItem[]): Set<string> {
	const works = new Map<string, Set<string>>();
	for (const item of items) {
		if (!item.workId) continue;
		for (const folder of folderChain(item.dir)) {
			const set = works.get(folder) ?? new Set<string>();
			set.add(item.workId);
			works.set(folder, set);
		}
	}
	const qualifying = [...works]
		.filter(([folder, set]) => set.size >= 2 && !KIND_FOLDER.test(folder.split('/').at(-1) ?? ''))
		.map(([folder]) => folder);
	return new Set(
		qualifying.filter(
			(folder) => !qualifying.some((other) => other !== folder && other.startsWith(`${folder}/`))
		)
	);
}

/** Whether an item belongs to a collection. A client folder holds what is in it, and anything else
 * the same Work made (its review, kept elsewhere). */
export function inCollection(
	key: string,
	item: Pick<LiveItem, 'dir' | 'workId' | 'collection'>,
	works: { id: string; goal_id: string | null }[],
	folderWorks?: ReadonlySet<string>
): boolean {
	if (key.startsWith('goal:')) {
		const goal = key.slice(5);
		return !!item.workId && works.some((work) => work.id === item.workId && work.goal_id === goal);
	}
	if (key.startsWith('folder:')) {
		const folder = key.slice(7);
		return within(item.dir, folder) || (!!item.workId && !!folderWorks?.has(item.workId));
	}
	if (key.startsWith('collection:')) return item.collection === key.slice(11);
	return false;
}

/** The Work whose files sit in a folder: a client folder's collection also holds what that Work
 * made elsewhere. */
export function worksIn(items: readonly LiveItem[], folder: string): Set<string> {
	return new Set(
		items.filter((item) => item.workId && within(item.dir, folder)).map((item) => item.workId!)
	);
}

const DAY = 86_400_000;

/** At most six collections, busiest first, each with why it is here: what needs the owner, what is
 * in review, what is new since the last visit, a live site. Open goals are here while they have
 * files; a client folder or a named collection while something in it changed in the past two
 * weeks. A closed goal is not. Computed once per visit or navigation, never under the cursor. */
export function liveCollections(input: {
	goals: readonly { id: string; title: string; closed_at: string | null }[];
	works: readonly { id: string; goal_id: string | null }[];
	items: readonly LiveItem[];
	titles: ReadonlyMap<string, string>;
	/** When the owner last opened the Library, for "new". */
	since: number;
	now: number;
	limit?: number;
}): LiveCollection[] {
	const { goals, works, items, titles, since, now } = input;
	const candidates: { key: string; title: string; members: LiveItem[]; open: boolean }[] = [];
	for (const goal of goals) {
		if (goal.closed_at) continue;
		const key = `goal:${goal.id}`;
		const members = items.filter((item) => inCollection(key, item, [...works]));
		if (members.length) candidates.push({ key, title: goal.title, members, open: true });
	}
	for (const folder of clientFolders(items)) {
		const key = `folder:${folder}`;
		const folderWorks = worksIn(items, folder);
		const members = items.filter((item) => inCollection(key, item, [...works], folderWorks));
		candidates.push({ key, title: folderTitle(folder, titles), members, open: false });
	}
	const named = new Set(
		items.map((item) => item.collection).filter((name): name is string => !!name)
	);
	for (const name of named) {
		const key = `collection:${name}`;
		candidates.push({
			key,
			title: name,
			members: items.filter((item) => item.collection === name),
			open: false
		});
	}
	const seen = new Set<string>();
	return candidates
		.flatMap((candidate) => {
			const activity = Math.max(
				0,
				...candidate.members.map((item) => Date.parse(item.updatedAt) || 0)
			);
			if (!candidate.open && now - activity > 14 * DAY) return [];
			/* Two candidates with one name (a goal and a folder for the same client) are one row. */
			const name = candidate.title.toLowerCase();
			if (seen.has(name)) return [];
			seen.add(name);
			const waiting = candidate.members.filter((item) => item.review).length;
			const fresh = candidate.members.filter((item) => Date.parse(item.updatedAt) > since).length;
			const reason = waiting
				? 'needs you'
				: candidate.members.some((item) => item.inReview)
					? 'in review'
					: since && fresh
						? `${fresh} new`
						: candidate.members.some((item) => item.live)
							? 'live site'
							: quietAge(now - activity);
			return [
				{
					key: candidate.key,
					title: candidate.title,
					reason,
					tone: waiting ? ('wait' as const) : fresh && since ? ('work' as const) : ('' as const),
					activity
				}
			];
		})
		.sort((a, b) => b.activity - a.activity || a.title.localeCompare(b.title))
		.slice(0, input.limit ?? 6);
}

function quietAge(ms: number): string {
	const days = Math.floor(ms / DAY);
	if (days < 1) return 'today';
	if (days === 1) return 'yesterday';
	if (days < 7) return `${days} days ago`;
	const weeks = Math.floor(days / 7);
	return weeks === 1 ? 'last week' : `${weeks} weeks ago`;
}
