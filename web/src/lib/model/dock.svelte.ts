/**
 * Whether the executive is expanded, per surface.
 *
 * Per-surface rather than global on purpose: the dock costs 380px, and what
 * that buys differs by screen. On People she is most of the point; on the Board
 * she is competing with a kanban column. One global flag would force the same
 * answer on both.
 */

const KEY = 'dock-collapsed';

type State = Record<string, boolean>;

function read(): State {
	if (typeof localStorage === 'undefined') return {};
	try {
		const raw = localStorage.getItem(KEY);
		return raw ? (JSON.parse(raw) as State) : {};
	} catch {
		return {};
	}
}

const collapsed = $state<State>(read());

export function isCollapsed(surface: string): boolean {
	return collapsed[surface] ?? false;
}

export function toggleDock(surface: string): void {
	collapsed[surface] = !isCollapsed(surface);
	try {
		localStorage?.setItem(KEY, JSON.stringify(collapsed));
	} catch {
		/* a preference that cannot be stored is still a working toggle */
	}
}
