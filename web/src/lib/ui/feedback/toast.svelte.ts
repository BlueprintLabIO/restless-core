import { untrack } from 'svelte';
/* One way to say a thing happened, with an Undo when it can be taken back. Every surface calls
 * toast() instead of growing its own notice; Toasts.svelte, mounted once at the root, shows them. */

export type ToastTone = 'done' | 'failure';
export type Toast = {
	id: number;
	/** Independent durable results may share wording without replacing each other. */
	dedupeKey?: string;
	/** One plain sentence. Raw system text (an error body, a path, a stack) never goes here;
	 * the server logs it, and client-side causes go to the console. */
	text: string;
	tone: ToastTone;
	undo?: () => unknown;
	/** One other thing to do about it ("Sign in", "Reload"); runs once, then the toast goes. */
	action?: { label: string; run: () => unknown };
	/** Stays until dismissed or acted on, as a failure does. */
	persist?: boolean;
	/** A source-owned deadline is never extended by hovering or focus. */
	expiresAt?: number;
	/** Called only for an explicit dismissal, never automatic eviction. */
	onDismiss?: () => unknown;
};

/* Long enough to reach Undo without hurrying; a plain notice goes sooner. A failure stays until it
 * is dismissed: it may be the only record that something did not happen. */
const LIFETIME_MS = 6000;
const UNDO_LIFETIME_MS = 10000;
const MAX = 3;
let next = 1;
const waiting: Toast[] = [];
const timers = new Map<number, ReturnType<typeof setTimeout>>();

export const toasts = $state<{ list: Toast[] }>({ list: [] });

export function dismissToast(id: number) {
	clearTimeout(timers.get(id));
	timers.delete(id);
	const queued = waiting.findIndex((item) => item.id === id);
	if (queued >= 0) waiting.splice(queued, 1);
	toasts.list = toasts.list.filter((item) => item.id !== id);
	while (toasts.list.length < MAX && waiting.length) {
		const item = waiting.shift()!;
		toasts.list = [...toasts.list, item];
		schedule(item.id);
	}
}

export function closeToast(id: number) {
	const item = toasts.list.find((entry) => entry.id === id);
	dismissToast(id);
	void item?.onDismiss?.();
}

function schedule(id: number) {
	clearTimeout(timers.get(id));
	const item = toasts.list.find((entry) => entry.id === id);
	if (!item || item.tone === 'failure' || item.persist) return;
	const undoable = item.undo;
	timers.set(
		id,
		setTimeout(
			() => dismissToast(id),
			item.expiresAt === undefined
				? undoable
					? UNDO_LIFETIME_MS
					: LIFETIME_MS
				: Math.max(0, item.expiresAt - Date.now())
		)
	);
}

/** Say what happened. Pass undo when the action can be reversed; it runs once, then the toast goes. */
export function toast(
	text: string,
	options: Pick<Toast, 'undo' | 'action' | 'persist' | 'expiresAt' | 'onDismiss' | 'dedupeKey'> & {
		tone?: ToastTone;
	} = {}
) {
	const item: Toast = { ...options, id: next++, text, tone: options.tone ?? 'done' };
	/* The same lasting notice said again replaces itself rather than stacking. */
	const lasting = (entry: Toast) => entry.tone === 'failure' || Boolean(entry.persist);
	if (lasting(item))
		for (const same of [...toasts.list, ...waiting].filter(
			(entry) => lasting(entry) && entry.text === text && entry.dedupeKey === item.dedupeKey
		))
			dismissToast(same.id);
	/* Over the limit, plain notices go first, then the oldest failure. A live Undo is never
	 * evicted; it leaves on its own clock. */
	while (toasts.list.length >= MAX) {
		const evictable =
			toasts.list.find((entry) => !entry.undo && entry.tone !== 'failure' && !entry.persist) ??
			toasts.list.find((entry) => !entry.undo && !entry.onDismiss);
		if (!evictable) break;
		dismissToast(evictable.id);
	}
	// Durable failures wait for a visible slot; eviction is never an acknowledgement.
	if (item.onDismiss && toasts.list.length >= MAX) {
		waiting.push(item);
		return item.id;
	}
	toasts.list = [...toasts.list, item];
	schedule(item.id);
	return item.id;
}

export type ToastMessage = Pick<Toast, 'text' | 'action' | 'persist' | 'dedupeKey'> & {
	tone?: ToastTone;
};

/** A component's outcome said as a toast instead of a banner that pushes the page about. Call it
 * while the component initialises, with what to say now (or nothing). Each time `read` gives a
 * message it is toasted once; one still showing with the same words is left alone. A failure or
 * lasting toast goes when the message clears or the component goes; a plain notice keeps its own
 * clock. `show()` says the current message again, for a fixed "!" that reopens a dismissed
 * warning; `active` says whether there is one. */
export function toastWhile(read: () => ToastMessage | null | undefined | false | '') {
	let id: number | undefined;
	let shown = '';
	const message = $derived(read() || null);
	const keyOf = (item: ToastMessage) =>
		JSON.stringify([item.text, item.tone ?? 'done', item.action?.label ?? '']);
	function say(item: ToastMessage) {
		if (id !== undefined) dismissToast(id);
		const { text, ...options } = item;
		id = toast(text, options);
		shown = keyOf(item);
	}
	function clear() {
		const item = id === undefined ? undefined : toasts.list.find((entry) => entry.id === id);
		if (item && (item.tone === 'failure' || item.persist)) dismissToast(item.id);
		id = undefined;
		shown = '';
	}
	$effect(() => {
		const item = message;
		untrack(() => {
			if (!item) return clear();
			const showing = id !== undefined && toasts.list.some((entry) => entry.id === id);
			if (!showing || shown !== keyOf(item)) say(item);
		});
	});
	$effect(() => () => untrack(clear));
	return {
		show() {
			const item = untrack(() => message);
			if (item) say(item);
		},
		get active() {
			return Boolean(message);
		}
	};
}

/** Take back the newest action that still offers Undo. False when none does. */
export function undoLatest(): boolean {
	const item = toasts.list.findLast(
		(entry) => entry.undo && (entry.expiresAt === undefined || entry.expiresAt > Date.now())
	);
	if (!item) return false;
	dismissToast(item.id);
	void item.undo?.();
	return true;
}

/** Hovering or focusing a toast holds it; leaving starts its clock again. */
export function holdToast(id: number, held: boolean) {
	if (toasts.list.find((entry) => entry.id === id)?.expiresAt !== undefined) return;
	if (held) {
		clearTimeout(timers.get(id));
		timers.delete(id);
	} else schedule(id);
}

/* A bar pinned to the bottom of the view (the Inbox detail's actions) keeps toasts above it: attach
 * this to it and the stack rises by the bar's height while it is mounted. */
const clearances = $state<{
	list: {
		id: number;
		px: number;
	}[];
}>({ list: [] });
export const toastClearance = () => Math.max(0, ...clearances.list.map((entry) => entry.px));
export function clearOfToasts(node: HTMLElement) {
	const id = next++;
	/* Untracked: the attachment runs as an effect, so reading the list it writes would make it
	 * depend on its own write and loop (effect_update_depth_exceeded on the Inbox). */
	const measure = () =>
		untrack(() => {
			const rect = node.getBoundingClientRect();
			// A detail pane kept mounted but hidden at phone width must not push notices offscreen.
			const px =
				rect.width > 0 && rect.height > 0 && rect.top >= 0 && rect.top < window.innerHeight
					? Math.round(window.innerHeight - rect.top)
					: 0;
			clearances.list = [...clearances.list.filter((entry) => entry.id !== id), { id, px }];
		});
	measure();
	const observer = new ResizeObserver(measure);
	observer.observe(node);
	window.addEventListener('resize', measure);
	return () => {
		observer.disconnect();
		window.removeEventListener('resize', measure);
		untrack(() => {
			clearances.list = clearances.list.filter((entry) => entry.id !== id);
		});
	};
}

/** Keep notices above a phone composer without moving desktop toasts. */
export function clearOfToastsOnPhone(node: HTMLElement) {
	const phone = window.matchMedia('(max-width: 760px)');
	let clear: (() => void) | undefined;
	const update = () => {
		clear?.();
		clear = phone.matches ? clearOfToasts(node) : undefined;
	};
	update();
	phone.addEventListener('change', update);
	return () => {
		phone.removeEventListener('change', update);
		clear?.();
	};
}
