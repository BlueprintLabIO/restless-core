/* One way to say a thing happened, with an Undo when it can be taken back. Every surface calls
 * toast() instead of growing its own notice; Toasts.svelte, mounted once at the root, shows them. */

export type ToastTone = 'done' | 'failure';
export type Toast = {
	id: number;
	text: string;
	tone: ToastTone;
	undo?: () => unknown;
};

/* Long enough to reach Undo without hurrying; a plain notice goes sooner. A failure stays until it
 * is dismissed: it may be the only record that something did not happen. */
const LIFETIME_MS = 6000;
const UNDO_LIFETIME_MS = 10000;
const MAX = 3;
let next = 1;
const timers = new Map<number, ReturnType<typeof setTimeout>>();

export const toasts = $state<{ list: Toast[] }>({ list: [] });

export function dismissToast(id: number) {
	clearTimeout(timers.get(id));
	timers.delete(id);
	toasts.list = toasts.list.filter((item) => item.id !== id);
}

function schedule(id: number) {
	clearTimeout(timers.get(id));
	const item = toasts.list.find((entry) => entry.id === id);
	if (!item || item.tone === 'failure') return;
	const undoable = item.undo;
	timers.set(
		id,
		setTimeout(() => dismissToast(id), undoable ? UNDO_LIFETIME_MS : LIFETIME_MS)
	);
}

/** Say what happened. Pass undo when the action can be reversed; it runs once, then the toast goes. */
export function toast(text: string, options: { undo?: () => unknown; tone?: ToastTone } = {}) {
	const item: Toast = { id: next++, text, tone: options.tone ?? 'done', undo: options.undo };
	/* The same failure said again replaces itself rather than stacking. */
	if (item.tone === 'failure')
		for (const same of toasts.list.filter(
			(entry) => entry.tone === 'failure' && entry.text === text
		))
			dismissToast(same.id);
	/* Over the limit, plain notices go first, then the oldest failure. A live Undo is never
	 * evicted; it leaves on its own clock. */
	while (toasts.list.length >= MAX) {
		const evictable =
			toasts.list.find((entry) => !entry.undo && entry.tone !== 'failure') ??
			toasts.list.find((entry) => !entry.undo);
		if (!evictable) break;
		dismissToast(evictable.id);
	}
	toasts.list = [...toasts.list, item];
	schedule(item.id);
	return item.id;
}

/** Take back the newest action that still offers Undo. False when none does. */
export function undoLatest(): boolean {
	const item = toasts.list.findLast((entry) => entry.undo);
	if (!item) return false;
	dismissToast(item.id);
	void item.undo?.();
	return true;
}

/** Hovering or focusing a toast holds it; leaving starts its clock again. */
export function holdToast(id: number, held: boolean) {
	if (held) {
		clearTimeout(timers.get(id));
		timers.delete(id);
	} else schedule(id);
}
