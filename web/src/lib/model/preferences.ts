/* Preferences follow the person (S109 F2): sidebar and pane widths, the Work view, the Inbox
 * filter and folded sidebar sections are kept with the signed-in person on the plane, the same in
 * every company and on every device.
 *
 * This browser keeps a copy, read synchronously, so a page draws at its final width on the first
 * frame; the plane's answer then reconciles it. A change lands here at once and reaches the plane
 * a moment later. Before the first answer, choices this browser already had (including the old
 * per-company keys, migrated once) go up; after it, the plane decides, so a choice reset on one
 * device does not come back from another. Dependency-free for node's tests. */

/** The choices kept; the plane refuses any other key (owner_preferences.rs keeps the same list). */
export const PREFERENCE_KEYS = [
	'pane.sidebar.work',
	'pane.sidebar.apps',
	'pane.sidebar.library',
	'pane.sidebar.company',
	'pane.sidebar.people',
	'pane.exec',
	'pane.work-detail',
	'pane.attention-document',
	'pane.handover',
	'pane.inbox',
	'account-rail',
	'work.view',
	'work.sort',
	'inbox.filter',
	'folds',
	'voice.stt',
	'voice.tts'
] as const;
export type PreferenceKey = (typeof PREFERENCE_KEYS)[number];
export type PreferenceValues = Partial<Record<PreferenceKey, unknown>>;

export const PREFERENCES_CACHE = 'restless:preferences';
export const PREFERENCES_URL = '/api/preferences';

const KNOWN = new Set<string>(PREFERENCE_KEYS);
export const isPreferenceKey = (key: string): key is PreferenceKey => KNOWN.has(key);

export interface PreferenceIo {
	/** This browser's storage; may be missing or throw. */
	storage(): Storage | undefined;
	/** The plane's copy, or null when it cannot answer (signed out, offline, an older plane). */
	load(): Promise<PreferenceValues | null>;
	/** Change some keys on the plane (null forgets one); whether it kept them. */
	save(patch: Record<string, unknown>, keepalive: boolean): Promise<boolean>;
	/** Defaults to setTimeout. */
	schedule?(run: () => void, ms: number): () => void;
}

type Cache = { values: PreferenceValues; synced: boolean };

function known(values: unknown): PreferenceValues {
	if (!values || typeof values !== 'object' || Array.isArray(values)) return {};
	const out: PreferenceValues = {};
	for (const [key, value] of Object.entries(values))
		if (isPreferenceKey(key) && value !== null && value !== undefined) out[key] = value;
	return out;
}

const LEGACY_PANE = /^restless:pane:[^:]+:(sidebar:[a-z-]+|[a-z-]+)$/;
const LEGACY_FOLD = 'sidebar-fold:';
const LEGACY_ACCOUNT_RAIL = 'restless:account-rail';

/** Lift the old per-company widths and per-browser folds into one key per area, and drop them.
 * A company's width stands in for every company; the first one found wins. The account rail's
 * old key stays: the design system's own fallback still reads it where no store is provided. */
export function migrateLegacy(storage: Storage): PreferenceValues {
	const values: PreferenceValues = {};
	const folds: Record<string, true> = {};
	const drop: string[] = [];
	for (let index = 0; index < storage.length; index += 1) {
		const key = storage.key(index);
		if (!key) continue;
		const pane = LEGACY_PANE.exec(key);
		if (pane) {
			const target = `pane.${pane[1].replace(':', '.')}`;
			const width = Number(storage.getItem(key));
			if (isPreferenceKey(target) && Number.isFinite(width) && width > 0 && !(target in values))
				values[target] = Math.round(width);
			drop.push(key);
		} else if (key.startsWith(LEGACY_FOLD)) {
			if (storage.getItem(key) === 'closed') folds[key.slice(LEGACY_FOLD.length)] = true;
			drop.push(key);
		}
	}
	if (Object.keys(folds).length) values.folds = folds;
	try {
		const rail = JSON.parse(storage.getItem(LEGACY_ACCOUNT_RAIL) ?? 'null');
		if (rail && typeof rail === 'object') values['account-rail'] = rail;
	} catch {
		/* An unreadable rail starts from its default. */
	}
	for (const key of drop) storage.removeItem(key);
	return values;
}

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

export function createPreferences(io: PreferenceIo, delay = 800) {
	const schedule =
		io.schedule ??
		((run: () => void, ms: number) => {
			const timer = setTimeout(run, ms);
			return () => clearTimeout(timer);
		});
	const listeners = new Set<() => void>();
	/* Keys changed here that the plane has not yet confirmed, with the change that set them. */
	const dirty = new Map<PreferenceKey, number>();
	let change = 0;
	let cancelFlush: (() => void) | null = null;
	let sending: Promise<void> | null = null;
	let cache: Cache = read();

	function read(): Cache {
		try {
			const storage = io.storage();
			if (!storage) return { values: {}, synced: false };
			const raw = storage.getItem(PREFERENCES_CACHE);
			if (raw !== null) {
				const parsed = JSON.parse(raw) as Partial<Cache>;
				return { values: known(parsed.values), synced: parsed.synced === true };
			}
			const migrated: Cache = { values: known(migrateLegacy(storage)), synced: false };
			storage.setItem(PREFERENCES_CACHE, JSON.stringify(migrated));
			return migrated;
		} catch {
			return { values: {}, synced: false };
		}
	}
	function write() {
		try {
			io.storage()?.setItem(PREFERENCES_CACHE, JSON.stringify(cache));
		} catch {
			/* Kept for this visit and on the plane. */
		}
	}
	function notify() {
		for (const listener of [...listeners]) listener();
	}

	async function send(keepalive: boolean) {
		if (!dirty.size) return;
		const sent = new Map(dirty);
		const patch: Record<string, unknown> = {};
		for (const key of sent.keys()) patch[key] = cache.values[key] ?? null;
		let kept = false;
		try {
			kept = await io.save(patch, keepalive);
		} catch {
			kept = false;
		}
		if (!kept) return;
		for (const [key, at] of sent) if (dirty.get(key) === at) dirty.delete(key);
	}
	/** Send what changed now; a send already on its way is followed by one more. */
	async function flush(keepalive = false): Promise<void> {
		cancelFlush?.();
		cancelFlush = null;
		if (sending) {
			await sending;
			if (!dirty.size) return;
		}
		sending = send(keepalive).finally(() => (sending = null));
		await sending;
	}

	return {
		get(key: PreferenceKey): unknown {
			return cache.values[key];
		},
		/** Keep a choice; null or undefined returns the key to its default. */
		set(key: PreferenceKey, value: unknown) {
			const next = value === null || value === undefined ? undefined : value;
			if (same(cache.values[key], next)) return;
			const values = { ...cache.values };
			if (next === undefined) delete values[key];
			else values[key] = next;
			cache = { ...cache, values };
			dirty.set(key, ++change);
			write();
			notify();
			cancelFlush?.();
			cancelFlush = schedule(() => {
				cancelFlush = null;
				void flush();
			}, delay);
		},
		listen(listener: () => void): () => void {
			listeners.add(listener);
			return () => listeners.delete(listener);
		},
		/** Take the plane's copy, keeping what changed here and is still on its way. */
		async reconcile(): Promise<void> {
			let server: PreferenceValues | null = null;
			try {
				server = await io.load();
			} catch {
				server = null;
			}
			if (!server) return;
			const remote = known(server);
			const values: PreferenceValues = {};
			for (const key of PREFERENCE_KEYS) {
				if (dirty.has(key)) {
					if (cache.values[key] !== undefined) values[key] = cache.values[key];
				} else if (remote[key] !== undefined) {
					values[key] = remote[key];
				} else if (!cache.synced && cache.values[key] !== undefined) {
					/* Never synced: what this browser had goes up rather than being lost. */
					values[key] = cache.values[key];
					dirty.set(key, ++change);
				}
			}
			const changed = PREFERENCE_KEYS.some((key) => !same(values[key], cache.values[key]));
			cache = { values, synced: true };
			write();
			if (changed) notify();
			if (dirty.size) await flush();
		},
		flush
	};
}

export type Preferences = ReturnType<typeof createPreferences>;
