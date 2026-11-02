/* A cold company load paints the layout this browser last showed, from the first byte (2 Nov).
 *
 * The cockpit is client-rendered and reveals once its critical reads have answered
 * (first-paint.svelte.ts). Until then app.html showed the brand's dot field, so every full load
 * of a company page was seconds of an empty dotted page before anything appeared. Instead, each
 * settled company page records where its surfaces sit (the top bar, the dock, the panes) and in
 * which colours, and app.html draws those as plain boxes before any script or stylesheet has
 * loaded. The reveal then fills panes that were already there.
 *
 * Only geometry and colours are kept: never text, names or counts. A snapshot is used only at the
 * same window width and theme it was taken at; otherwise app.html draws the top bar alone.
 *
 * app.html reads SHELL_KEY and this shape by name; keep the two in step. Dependency-free for
 * node's tests. */

export const SHELL_KEY = 'restless:shell';
const VERSION = 1;
/* A surface smaller than this is part of a pane, not a pane. */
const MIN_PANE = 120;
const MAX_PANES = 6;
const MAX_SECTIONS = 12;

/** One surface, as insets from the window's edges so a taller or shorter window still fits. */
export type ShellPanel = {
	t: number;
	l: number;
	r: number;
	b: number;
	bg: string;
	radius: string;
	bw: string;
	bs: string;
	bc: string;
	shadow: string;
};

export type ShellSnapshot = {
	w: number;
	theme: string;
	bg: string;
	panels: ShellPanel[];
};

export type ShellStore = {
	v: number;
	last: string;
	sections: Record<string, ShellSnapshot>;
};

type Box = { x: number; y: number; width: number; height: number };
type Paint = {
	backgroundColor: string;
	borderRadius: string;
	borderWidth: string;
	borderStyle: string;
	borderColor: string;
	boxShadow: string;
};

/** The section of a company path (`/acme/work/…` is `work`; the company's root is the Inbox). */
export function shellSection(pathname: string): string {
	return pathname.split('/')[2] ?? '';
}

/** A surface's box and paint as a snapshot panel, or null when it is not on screen. */
export function shellPanel(
	box: Box,
	paint: Paint,
	viewport: { width: number; height: number }
): ShellPanel | null {
	if (box.width < 1 || box.height < 1) return null;
	if (box.x >= viewport.width || box.y >= viewport.height) return null;
	const round = (value: number) => Math.round(value);
	return {
		t: round(box.y),
		l: round(box.x),
		r: round(viewport.width - box.x - box.width),
		b: round(viewport.height - box.y - box.height),
		bg: paint.backgroundColor,
		radius: paint.borderRadius,
		bw: paint.borderWidth,
		bs: paint.borderStyle,
		bc: paint.borderColor,
		shadow: paint.boxShadow
	};
}

/** Whether a surface paints a pane of its own: a filled box at least MIN_PANE on each side. */
export function isPane(box: Box, paint: Pick<Paint, 'backgroundColor'>): boolean {
	const clear = /^(transparent|rgba\(0, 0, 0, 0\))$/.test(paint.backgroundColor.trim());
	return !clear && box.width >= MIN_PANE && box.height >= MIN_PANE;
}

/** The surfaces worth drawing: the top bar and dock, then the outermost panes of the body. */
function surfaces(root: Element, view: Window): Element[] {
	const found: Element[] = [];
	for (const selector of ['.bridge-topbar', '.bridge-dock']) {
		const element = root.querySelector(selector);
		if (element) found.push(element);
	}
	const body = root.querySelector('.bridge-body');
	if (!body) return found;
	/* Breadth first, so a pane's own cards are never taken for panes. */
	let level: Element[] = [...body.children];
	for (let depth = 0; depth < 6 && level.length; depth++) {
		const next: Element[] = [];
		for (const element of level) {
			const box = element.getBoundingClientRect();
			if (isPane(box, view.getComputedStyle(element))) {
				if (found.length < MAX_PANES + 2) found.push(element);
			} else next.push(...element.children);
		}
		level = next;
	}
	return found;
}

/** This page's surfaces as a snapshot, or null when the shell is not on screen. */
export function captureShell(document: Document, view: Window): ShellSnapshot | null {
	const root = document.querySelector('.bridge-root');
	/* The loading screen is fixed to the window; a scrolled page's boxes are not where it starts. */
	if (!root || view.scrollY > 0) return null;
	const viewport = { width: view.innerWidth, height: view.innerHeight };
	const panels: ShellPanel[] = [];
	for (const element of surfaces(root, view)) {
		const style = view.getComputedStyle(element);
		if (style.display === 'none' || style.visibility === 'hidden') continue;
		const panel = shellPanel(element.getBoundingClientRect(), style, viewport);
		if (panel) panels.push(panel);
	}
	if (!panels.length) return null;
	return {
		w: viewport.width,
		theme: document.documentElement.dataset.theme ?? 'light',
		bg: view.getComputedStyle(root).backgroundColor,
		panels
	};
}

/** The stored snapshots with this section's replaced; the oldest sections give way past the cap. */
export function withSection(
	stored: ShellStore | null,
	section: string,
	snapshot: ShellSnapshot
): ShellStore {
	const sections = stored?.v === VERSION ? { ...stored.sections } : {};
	delete sections[section];
	sections[section] = snapshot;
	const names = Object.keys(sections);
	for (const name of names.slice(0, Math.max(0, names.length - MAX_SECTIONS)))
		delete sections[name];
	return { v: VERSION, last: section, sections };
}

/** The snapshot app.html draws for a path: its section's, else the last one taken. Only at the
 * width and theme it was taken at; a different layout is worse than none. */
export function shellFor(
	stored: ShellStore | null,
	pathname: string,
	width: number,
	theme: string
): ShellSnapshot | null {
	if (!stored || stored.v !== VERSION || !stored.sections) return null;
	const snapshot = stored.sections[shellSection(pathname)] ?? stored.sections[stored.last];
	if (!snapshot || snapshot.w !== width || snapshot.theme !== theme) return null;
	return Array.isArray(snapshot.panels) ? snapshot : null;
}

/** Record the settled page's shell for the next cold load. */
export function rememberShell(document: Document, view: Window): void {
	const snapshot = captureShell(document, view);
	if (!snapshot) return;
	try {
		let stored: ShellStore | null = null;
		try {
			stored = JSON.parse(view.localStorage.getItem(SHELL_KEY) ?? 'null');
		} catch {
			/* A damaged record is replaced. */
		}
		const next = withSection(stored, shellSection(view.location.pathname), snapshot);
		view.localStorage.setItem(SHELL_KEY, JSON.stringify(next));
	} catch {
		/* Without storage the next load draws the top bar alone. */
	}
}
