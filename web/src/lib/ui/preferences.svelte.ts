/* Where a view keeps a person's layout choices, such as a folded sidebar section or the account
 * rail's width. The app hands its views a store through context, which keeps them with the
 * person on every device; a view without one keeps them in this browser, as before. `get` is
 * read inside a component, so a change re-renders what depends on it. */
import { getContext, setContext } from 'svelte';

export interface ViewPreferences {
	get(key: string): unknown;
	/** null or undefined returns the key to its default. */
	set(key: string, value: unknown): void;
}

const CONTEXT = Symbol('view-preferences');

export function provideViewPreferences(store: ViewPreferences): void {
	setContext(CONTEXT, store);
}

/* This browser alone: `restless:<key>`, read once per key and redrawn on change. */
const kept = new Map<string, unknown>();
let version = $state(0);
const browserPreferences: ViewPreferences = {
	get(key) {
		void version;
		if (!kept.has(key)) {
			let value: unknown;
			try {
				value =
					JSON.parse(globalThis.localStorage?.getItem(`restless:${key}`) ?? 'null') ?? undefined;
			} catch {
				value = undefined;
			}
			kept.set(key, value);
		}
		return kept.get(key);
	},
	set(key, value) {
		const next = value ?? undefined;
		kept.set(key, next);
		version += 1;
		try {
			if (next === undefined) globalThis.localStorage?.removeItem(`restless:${key}`);
			else globalThis.localStorage?.setItem(`restless:${key}`, JSON.stringify(next));
		} catch {
			/* Kept for this visit. */
		}
	}
};

/** The store this view keeps its choices in. Call while the component initialises. */
export function viewPreferences(): ViewPreferences {
	return getContext<ViewPreferences | undefined>(CONTEXT) ?? browserPreferences;
}
