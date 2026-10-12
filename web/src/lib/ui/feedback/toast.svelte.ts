/* One way to say a thing happened, with an Undo when it can be taken back. Every surface calls
 * toast() instead of growing its own notice; Toasts.svelte, mounted once at the root, shows them. */

export type ToastTone = 'done' | 'failure';
export type Toast = {
	id: number;
	text: string;
	tone: ToastTone;
	undo?: () => unknown;
};

/* Long enough to reach Undo without hurrying; a plain notice goes sooner. */
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
	const undoable = toasts.list.find((item) => item.id === id)?.undo;
	timers.set(
		id,
		setTimeout(() => dismissToast(id), undoable ? UNDO_LIFETIME_MS : LIFETIME_MS)
	);
}

/** Say what happened. Pass undo when the action can be reversed; it runs once, then the toast goes. */
export function toast(text: string, options: { undo?: () => unknown; tone?: ToastTone } = {}) {
	const item: Toast = { id: next++, text, tone: options.tone ?? 'done', undo: options.undo };
	const kept = toasts.list.slice(-(MAX - 1));
	for (const dropped of toasts.list.filter((entry) => !kept.includes(entry)))
		dismissToast(dropped.id);
	toasts.list = [...kept, item];
	schedule(item.id);
	return item.id;
}

/** Hovering or focusing a toast holds it; leaving starts its clock again. */
export function holdToast(id: number, held: boolean) {
	if (held) {
		clearTimeout(timers.get(id));
		timers.delete(id);
	} else schedule(id);
}
